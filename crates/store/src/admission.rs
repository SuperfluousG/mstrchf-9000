use crate::records::{storage, terminal, unsupported};
use mstrchf_contracts::*;
use rusqlite::{Connection, OptionalExtension, params};

// Private, concrete decisions; never a caller-provided validated flag/trait.
pub(crate) enum PreparedMutation {
    Draft(Box<HouseholdDraft>),
    Privacy(PrivacyPolicy),
    Cancel(JobSummary),
    #[cfg(all(debug_assertions, feature = "synthetic-development"))]
    Profile(Box<HouseholdProfile>),
    #[cfg(all(debug_assertions, feature = "synthetic-development"))]
    Week(Box<Week>),
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
            let evaluated_at = writer_time(c)?;
            mstrchf_core::setup::validate_setup_draft_transition(
                &s.household_draft,
                draft,
                restriction_issue_edits,
                &s.privacy,
                &evaluated_at,
            )?;
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
        AppCommand::UpdateProfile {
            profile,
            expected_profile_revision,
        } => {
            #[cfg(all(debug_assertions, feature = "synthetic-development"))]
            if crate::current::active(c)? {
                let at = writer_time(c)?;
                if profile.household_id != s.household_draft.household_id {
                    return Err(DomainError::new(
                        ErrorCode::IdentityMismatch,
                        "/command/data/profile/household_id",
                    ));
                }
                mstrchf_core::household::validate_profile(profile, &s.privacy, &at)?;
                // M1 establishes one fixed engineering input. Household editing belongs to M2.
                if s.profile.0.is_some() || *profile != crate::development::seed_profile(s)? {
                    return Err(unsupported());
                }
                if *expected_profile_revision != Revision(1) {
                    return Err(stale("/command/data/expected_profile_revision"));
                }
                return Ok(PreparedMutation::Profile(Box::new(profile.clone())));
            }
            let _ = (profile, expected_profile_revision);
            Err(unsupported())
        }
        AppCommand::SaveWeek {
            week,
            expected_week_revision,
        } => {
            #[cfg(all(debug_assertions, feature = "synthetic-development"))]
            if crate::current::active(c)? {
                let at = writer_time(c)?;
                let profile = s
                    .profile
                    .0
                    .as_ref()
                    .ok_or_else(|| DomainError::new(ErrorCode::MissingField, "/profile"))?;
                let existing = s.weeks.iter().find(|w| w.range.week_id == week.week_id);
                if existing.is_none() && !s.weeks.is_empty() {
                    return Err(unsupported());
                }
                let seed = crate::development::seed_week(s, &at)?;
                let current = existing.unwrap_or(&seed);
                if week.defaults_snapshot != current.defaults_snapshot
                    || week.cells != current.cells
                    || week.day_enabled != current.day_enabled
                    || week.meal_type_enabled != current.meal_type_enabled
                {
                    return Err(unsupported());
                }
                let mut next = mstrchf_core::availability::prepare_week_edit(
                    current,
                    week,
                    *expected_week_revision,
                    profile,
                    &at,
                )?
                .candidate;
                if let Some(plan) = &week.plan.0 {
                    if plan.revision != current.plan.revision {
                        return Err(stale("/command/data/week/plan/revision"));
                    }
                    next.plan = plan.clone();
                }
                crate::development::validate_saved_plan(s, profile, &next)?;
                if existing.is_some() {
                    next.revision = next.revision.checked_increment()?;
                    next.plan.revision = next.plan.revision.checked_increment()?;
                }
                next.approved_plan = Nullable(None);
                next.confirmed_order = Nullable(None);
                next.publication = current.publication.clone();
                let _: Week = parse_document(&canonical_bytes(&next)?)?;
                return Ok(PreparedMutation::Week(Box::new(next)));
            }
            let _ = (week, expected_week_revision);
            Err(unsupported())
        }
        AppCommand::ApplyPlanProposal { .. }
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
        #[cfg(all(debug_assertions, feature = "synthetic-development"))]
        PreparedMutation::Profile(profile) => {
            c.execute(
                "UPDATE household SET profile=?1,profile_revision=?2 WHERE singleton=1",
                params![canonical_bytes(&profile)?, profile.revision.0.to_string()],
            )
            .map_err(storage)?;
        }
        #[cfg(all(debug_assertions, feature = "synthetic-development"))]
        PreparedMutation::Week(week) => {
            c.execute("INSERT INTO weeks(week_id,revision,body) VALUES(?1,?2,?3) ON CONFLICT(week_id) DO UPDATE SET revision=excluded.revision,body=excluded.body",params![week.range.week_id.0,week.revision.0.to_string(),canonical_bytes(&week)?]).map_err(storage)?;
        }
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

fn writer_time(c: &Connection) -> Result<Instant, DomainError> {
    #[cfg(test)]
    if let Some(t) = crate::compatibility_tests::clock() {
        return Instant::try_from(t);
    }
    let text: String = c
        .query_row("SELECT strftime('%Y-%m-%dT%H:%M:%SZ','now')", [], |r| {
            r.get(0)
        })
        .map_err(storage)?;
    Instant::try_from(text.as_str())
}
