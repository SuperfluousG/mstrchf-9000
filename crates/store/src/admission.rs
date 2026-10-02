use crate::records::{storage, terminal, unsupported};
use mstrchf_contracts::*;
use rusqlite::{Connection, OptionalExtension, params};

// Private, concrete decisions; never a caller-provided validated flag/trait.
pub(crate) enum PreparedMutation {
    Draft(Box<HouseholdDraft>),
    Privacy(PrivacyPolicy),
    Cancel(JobSummary),
}
fn stale(path: &str) -> DomainError {
    DomainError::new(ErrorCode::ContextStale, path)
}
pub(crate) fn prepare(
    c: &Connection,
    s: &Snapshot,
    command: &AppCommand,
) -> Result<PreparedMutation, DomainError> {
    match command {
        AppCommand::SaveHouseholdDraft {
            draft,
            restriction_issue_edits,
        } => {
            // Temporary ruling005 boundary: native transition/readiness helpers
            // are not yet integrated. Never discard stored text or ignore edits.
            if !s.household_draft.unresolved_restrictions.is_empty()
                || !draft.unresolved_restrictions.is_empty()
                || !restriction_issue_edits.is_empty()
            {
                return Err(unsupported());
            }
            if draft.household_id != s.household_draft.household_id {
                return Err(DomainError::new(
                    ErrorCode::IdentityMismatch,
                    "/command/data/draft/household_id",
                ));
            }
            if draft.revision != s.household_draft.revision {
                return Err(stale("/command/data/draft/revision"));
            }
            if let Some(connections) = &draft.connections.0
                && connections.privacy != s.privacy
            {
                return Err(stale("/command/data/draft/connections/privacy"));
            }
            if let Some(usual) = &draft.usual_week.0 {
                for cell in usual.cells.iter() {
                    for id in cell.attendee_ids.iter() {
                        if !draft.diners.iter().any(|d| &d.diner_id == id) {
                            return Err(DomainError::new(
                                ErrorCode::ForeignReference,
                                "/command/data/draft/usual_week/cells/attendee_ids",
                            ));
                        }
                    }
                }
            }
            let mut next = draft.clone();
            if next != s.household_draft {
                next.revision = next.revision.checked_increment()?;
            }
            Ok(PreparedMutation::Draft(Box::new(next)))
        }
        AppCommand::SetPrivacy {
            policy,
            expected_policy_revision,
        } => {
            if *expected_policy_revision != s.privacy.revision
                || policy.revision != s.privacy.revision
            {
                return Err(stale("/command/data/policy/revision"));
            }
            let mut next = policy.clone();
            next.revision = next.revision.checked_increment()?;
            Ok(PreparedMutation::Privacy(next))
        }
        AppCommand::CancelJob { job_id } => {
            let row: Option<(bool, Vec<u8>)> = c
                .query_row(
                    "SELECT terminal,summary FROM jobs WHERE job_id=?1",
                    [&job_id.0],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )
                .optional()
                .map_err(storage)?;
            let (is_terminal, bytes) = row.ok_or_else(|| {
                DomainError::new(ErrorCode::MissingReference, "/command/data/job_id")
            })?;
            let mut job: JobSummary = parse_document(&bytes)?;
            if job.job_id != *job_id || terminal(job.state) != is_terminal {
                return Err(storage("job identity"));
            }
            if !terminal(job.state) {
                job.state = JobState::Cancelled;
                job.next_action = Nullable(Some(RequiredAction::StopWorker));
            }
            Ok(PreparedMutation::Cancel(job))
        }
        AppCommand::UpdateProfile { .. }
        | AppCommand::SaveWeek { .. }
        | AppCommand::ApplyPlanProposal { .. }
        | AppCommand::ConfirmEssentials { .. }
        | AppCommand::ApprovePlan { .. }
        | AppCommand::ConfirmOrder { .. }
        | AppCommand::RequestJob { .. }
        | AppCommand::RecordUserStop { .. }
        | AppCommand::RequestCalendarPublication { .. } => Err(unsupported()),
    }
}
pub(crate) fn commit_mutation(
    c: &Connection,
    s: &Snapshot,
    mutation: PreparedMutation,
) -> Result<(), DomainError> {
    match mutation {
        PreparedMutation::Draft(draft) => {
            c.execute(
                "UPDATE household SET draft=?1,draft_revision=?2 WHERE singleton=1",
                params![canonical_bytes(&draft)?, draft.revision.0.to_string()],
            )
            .map_err(storage)?;
        }
        PreparedMutation::Privacy(policy) => {
            let settings: String = c
                .query_row(
                    "SELECT settings_revision FROM app_meta WHERE singleton=1",
                    [],
                    |r| r.get(0),
                )
                .map_err(storage)?;
            let settings: Revision = parse_document(&canonical_bytes(&settings)?)?;
            let settings = settings.checked_increment()?;
            let mut draft = s.household_draft.clone();
            if let Some(connections) = &mut draft.connections.0 {
                connections.privacy = policy.clone();
                draft.revision = draft.revision.checked_increment()?;
            }
            let mut profile = s.profile.0.clone();
            if let Some(profile) = &mut profile {
                profile.connections.privacy = policy.clone();
                profile.revision = profile.revision.checked_increment()?;
            }
            c.execute("UPDATE household SET privacy=?1,privacy_revision=?2,draft=?3,draft_revision=?4 WHERE singleton=1",params![canonical_bytes(&policy)?,policy.revision.0.to_string(),canonical_bytes(&draft)?,draft.revision.0.to_string()]).map_err(storage)?;
            c.execute(
                "UPDATE app_meta SET settings_revision=?1 WHERE singleton=1",
                [settings.0.to_string()],
            )
            .map_err(storage)?;
            if let Some(profile) = profile {
                c.execute(
                    "UPDATE household SET profile=?1,profile_revision=?2 WHERE singleton=1",
                    params![canonical_bytes(&profile)?, profile.revision.0.to_string()],
                )
                .map_err(storage)?;
            }
            let mut q = c
                .prepare("SELECT summary FROM jobs WHERE terminal=0")
                .map_err(storage)?;
            let jobs = q
                .query_map([], |r| r.get::<_, Vec<u8>>(0))
                .map_err(storage)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(storage)?;
            for bytes in jobs {
                let mut job: JobSummary = parse_document(&bytes)?;
                job.state = JobState::NeedsUser;
                job.reason = Nullable(Some(ErrorCode::PrivacyDenied));
                job.next_action = Nullable(Some(RequiredAction::StopWorker));
                c.execute(
                    "UPDATE resource_quarantine SET quarantined=1 WHERE job_id=?1",
                    [&job.job_id.0],
                )
                .map_err(storage)?;
                c.execute(
                    "UPDATE jobs SET summary=?1,terminal=1 WHERE job_id=?2",
                    params![canonical_bytes(&job)?, job.job_id.0],
                )
                .map_err(storage)?;
            }
        }
        PreparedMutation::Cancel(job) => {
            // The old persisted terminal bit decides whether resource quarantine changes.
            c.execute("UPDATE resource_quarantine SET quarantined=1 WHERE job_id=?1 AND EXISTS(SELECT 1 FROM jobs WHERE job_id=?1 AND terminal=0)",[&job.job_id.0]).map_err(storage)?;
            c.execute(
                "UPDATE jobs SET summary=?1,terminal=?2 WHERE job_id=?3",
                params![canonical_bytes(&job)?, terminal(job.state), job.job_id.0],
            )
            .map_err(storage)?;
        }
    }
    Ok(())
}
