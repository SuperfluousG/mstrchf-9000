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
    #[cfg(all(debug_assertions, feature = "synthetic-development"))]
    Horizon(Vec<Week>),
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
        AppCommand::OpenPlanningHorizon {} => {
            #[cfg(all(debug_assertions, feature = "synthetic-development"))]
            if crate::current::m2(c)? {
                let p=s.profile.0.as_ref().ok_or_else(||DomainError::new(ErrorCode::MissingField,"/profile"))?;
                let at=writer_time(c)?;
                let today=mstrchf_core::weeks::local_planning_date(&at,&p.usual_week.zone)?;
                let ids=[1,2,3,4].map(|n|WeekId(format!("wk_{n:032x}")));
                let ranges=mstrchf_core::weeks::planning_horizon(&today,&p.usual_week.zone,&ids)?;
                let retained=crate::current::all_weeks(c)?;
                let mut weeks=Vec::new();
                for mut range in ranges {
                    if let Some(existing)=retained.iter().find(|w|w.range.monday==range.monday && w.range.zone==range.zone) {
                        weeks.push(existing.clone());
                    } else {
                        range.week_id=WeekId(format!("wk_{}",crate::records::random_id()?));
                        weeks.push(mstrchf_core::weeks::new_week(p,&s.privacy,&at,&range)?);
                    }
                }
                if retained.len()+weeks.iter().filter(|w|!retained.iter().any(|old|old.range.week_id==w.range.week_id)).count()>4096 {
                    return Err(DomainError::new(ErrorCode::LimitExceeded,"/weeks/archive"));
                }
                return Ok(PreparedMutation::Horizon(weeks));
            }
            Err(unsupported())
        }
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
                let mut next=profile.clone();
                if let Some(current)=&s.profile.0 {
                    let retained=crate::current::all_weeks(c)?;
                    for batch in retained.chunks(4) {
                        mstrchf_core::household::prepare_profile_update(current,profile,*expected_profile_revision,&s.privacy,&at,batch)?;
                    }
                    if retained.is_empty() {mstrchf_core::household::prepare_profile_update(current,profile,*expected_profile_revision,&s.privacy,&at,&[])?;}
                    next.revision=current.revision.checked_increment()?;
                } else {
                    if *expected_profile_revision!=Revision(1) || profile.revision!=Revision(1) {return Err(stale("/command/data/expected_profile_revision"));}
                    if *profile!=crate::development::seed_profile(s)? {
                        let bound=mstrchf_core::household::profile_from_draft(&s.household_draft,&s.privacy,&at)?;
                        if next!=bound {return Err(DomainError::new(ErrorCode::IdentityMismatch,"/profile/draft"));}
                    }
                }
                return Ok(PreparedMutation::Profile(Box::new(next)));
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
                if existing.is_none() && !s.weeks.is_empty() {return Err(DomainError::new(ErrorCode::ForeignReference,"/week_id"));}
                let seed = crate::development::seed_week(s, &at)?;
                let current = existing.unwrap_or(&seed);
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
                // Retain existing selections during availability/settings changes;
                // newly supplied selections still require native food admission.
                if next.plan!=current.plan {crate::development::validate_saved_plan(s, profile, &next)?;}
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
        PreparedMutation::Horizon(weeks) => {
            c.execute("DELETE FROM active_weeks",[]).map_err(storage)?;
            for (index,week) in weeks.into_iter().enumerate() {
                c.execute("INSERT INTO weeks VALUES(?1,?2,?3) ON CONFLICT(week_id) DO NOTHING",params![week.range.week_id.0,week.revision.0.to_string(),canonical_bytes(&week)?]).map_err(storage)?;
                c.execute("INSERT INTO active_weeks VALUES(?1,?2)",params![index as i64,week.range.week_id.0]).map_err(storage)?;
            }
        }
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
            if crate::current::m2(c)? {c.execute("INSERT INTO active_weeks(position,week_id) SELECT 0,?1 WHERE NOT EXISTS(SELECT 1 FROM active_weeks)",[&week.range.week_id.0]).map_err(storage)?;}
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
