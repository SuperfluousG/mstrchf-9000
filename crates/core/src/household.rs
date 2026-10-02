//! Pure household admission and update candidates.
use mstrchf_contracts::*;
use std::collections::BTreeSet;

/// A validated candidate, not a committed revision or a food-suitability verdict.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProfileChange {
    pub candidate: HouseholdProfile,
    pub changed_hard_diner_ids: Vec<DinerId>,
}

/// Validate exactly the supplied stages; an incomplete draft can be valid.
pub fn validate_draft_progress(
    draft: &HouseholdDraft,
    policy: &PrivacyPolicy,
    evaluated_at: &Instant,
) -> Result<(), DomainError> {
    shape(draft)?;
    shape(policy)?;
    shape(evaluated_at)?;
    confirmation_times(&draft.diners, evaluated_at)?;
    if let Some(usual) = &draft.usual_week.0 {
        usual_references(usual, &draft.diners)?;
    }
    if let Some(connections) = &draft.connections.0 {
        policy_matches(connections, policy)?;
    }
    Ok(())
}

/// Admit a complete profile against the current authoritative privacy policy.
pub fn validate_profile(
    profile: &HouseholdProfile,
    policy: &PrivacyPolicy,
    evaluated_at: &Instant,
) -> Result<(), DomainError> {
    shape(profile)?;
    shape(policy)?;
    shape(evaluated_at)?;
    confirmation_times(&profile.diners, evaluated_at)?;
    usual_references(&profile.usual_week, &profile.diners)?;
    policy_matches(&profile.connections, policy)
}

/// Promote supplied complete data, starting the independent profile stream at one.
pub fn profile_from_draft(
    draft: &HouseholdDraft,
    policy: &PrivacyPolicy,
    evaluated_at: &Instant,
) -> Result<HouseholdProfile, DomainError> {
    validate_draft_progress(draft, policy, evaluated_at)?;
    if draft.diners.is_empty() {
        return Err(DomainError::new(ErrorCode::MissingField, "/diners"));
    }
    if draft
        .diners
        .iter()
        .any(|d| matches!(d.restrictions, RestrictionDeclaration::Unconfirmed {}))
    {
        return Err(DomainError::new(
            ErrorCode::DomainConflict,
            "/diners/restrictions",
        ));
    }
    let profile = HouseholdProfile {
        schema_version: draft.schema_version,
        household_id: draft.household_id.clone(),
        revision: Revision(1),
        diners: BoundedVec(draft.diners.0.clone()),
        preferences: draft
            .preferences
            .0
            .clone()
            .ok_or_else(|| DomainError::new(ErrorCode::MissingField, "/preferences"))?,
        usual_week: draft
            .usual_week
            .0
            .clone()
            .ok_or_else(|| DomainError::new(ErrorCode::MissingField, "/usual_week"))?,
        connections: draft
            .connections
            .0
            .clone()
            .ok_or_else(|| DomainError::new(ErrorCode::MissingField, "/connections"))?,
    };
    validate_profile(&profile, policy, evaluated_at)?;
    Ok(profile)
}

/// Prepare an update at the current revision; the store owns CAS and increments.
///
/// `current_weeks` is a bounded batch, never proof of archive completeness. The
/// store must separately scan all retained relevant future drafts and original
/// approved inputs before accepting diner removal or a restriction update.
pub fn prepare_profile_update(
    current: &HouseholdProfile,
    proposed: &HouseholdProfile,
    expected_revision: Revision,
    policy: &PrivacyPolicy,
    evaluated_at: &Instant,
    current_weeks: &[Week],
) -> Result<ProfileChange, DomainError> {
    shape(&expected_revision)?;
    validate_profile(current, policy, evaluated_at)?;
    validate_profile(proposed, policy, evaluated_at)?;
    if current.household_id != proposed.household_id {
        return Err(DomainError::new(
            ErrorCode::IdentityMismatch,
            "/household_id",
        ));
    }
    if expected_revision != current.revision || proposed.revision != current.revision {
        return Err(DomainError::new(ErrorCode::ContextStale, "/revision"));
    }
    if current_weeks.len() > 4 {
        return Err(DomainError::new(ErrorCode::LimitExceeded, "/current_weeks"));
    }
    let mut week_ids = BTreeSet::new();
    let mut occasion_ids = BTreeSet::new();
    for (index, week) in current_weeks.iter().enumerate() {
        shape(week)?;
        if !week_ids.insert(&week.range.week_id) {
            return Err(DomainError::new(ErrorCode::InvalidValue, "/current_weeks"));
        }
        for (cell_index, cell) in week.cells.iter().enumerate() {
            if let Some(occasion) = &cell.occasion.0
                && !occasion_ids.insert(&occasion.occasion_id)
            {
                return Err(DomainError::new(
                    ErrorCode::IdentityMismatch,
                    &format!("/current_weeks/{index}/cells/{cell_index}/occasion/occasion_id"),
                ));
            }
        }
        week_references(week, &current.diners, index)?;
        week_references(week, &proposed.diners, index)?;
    }
    let ids = current
        .diners
        .iter()
        .chain(proposed.diners.iter())
        .map(|d| &d.diner_id)
        .collect::<BTreeSet<_>>();
    let mut changed_hard_diner_ids = Vec::new();
    for id in ids {
        let before = current.diners.iter().find(|d| &d.diner_id == id);
        let after = proposed.diners.iter().find(|d| &d.diner_id == id);
        if hard_set(before)? != hard_set(after)? {
            changed_hard_diner_ids.push(id.clone());
        }
    }
    if changed_hard_diner_ids.len() > 64 {
        return Err(DomainError::new(
            ErrorCode::LimitExceeded,
            "/changed_hard_diner_ids",
        ));
    }
    Ok(ProfileChange {
        candidate: proposed.clone(),
        changed_hard_diner_ids,
    })
}

fn shape(value: &impl ValidateShape) -> Result<(), DomainError> {
    value.validate_shape().map_err(|issues| DomainError {
        code: issues.first().map_or(ErrorCode::InvalidValue, |i| i.code),
        truncated: issues.len() > 100,
        issues: BoundedVec(issues.into_iter().take(100).collect()),
        retry: Retry::Never,
    })
}

fn confirmation_times(diners: &[Diner], evaluated_at: &Instant) -> Result<(), DomainError> {
    // Both operands passed shape admission before this strict UTC-string comparison.
    for (index, diner) in diners.iter().enumerate() {
        let at = match &diner.restrictions {
            RestrictionDeclaration::Unconfirmed {} => continue,
            RestrictionDeclaration::ConfirmedNone { confirmed_at }
            | RestrictionDeclaration::Confirmed { confirmed_at, .. } => confirmed_at,
        };
        if at > evaluated_at {
            return Err(DomainError::new(
                ErrorCode::InvalidValue,
                &format!("/diners/{index}/restrictions/data/confirmed_at"),
            ));
        }
    }
    Ok(())
}

fn policy_matches(connections: &Connections, policy: &PrivacyPolicy) -> Result<(), DomainError> {
    if &connections.privacy != policy {
        Err(DomainError::new(
            ErrorCode::ContextStale,
            "/connections/privacy",
        ))
    } else {
        Ok(())
    }
}

fn usual_references(usual: &UsualWeek, diners: &[Diner]) -> Result<(), DomainError> {
    for (index, cell) in usual.cells.iter().enumerate() {
        let path = format!("/usual_week/cells/{index}/attendee_ids");
        if cell
            .attendee_ids
            .iter()
            .any(|id| !diners.iter().any(|d| &d.diner_id == id))
        {
            return Err(DomainError::new(ErrorCode::ForeignReference, &path));
        }
        let day = match cell.weekday {
            Weekday::Monday => 0,
            Weekday::Tuesday => 1,
            Weekday::Wednesday => 2,
            Weekday::Thursday => 3,
            Weekday::Friday => 4,
            Weekday::Saturday => 5,
            Weekday::Sunday => 6,
        };
        if usual.day_enabled[day]
            && usual.enabled_meal_types.contains(&cell.meal_type)
            && cell.choice != MealLocation::Skip
            && cell.attendee_ids.is_empty()
        {
            return Err(DomainError::new(ErrorCode::InvalidValue, &path));
        }
    }
    Ok(())
}

fn week_references(week: &Week, diners: &[Diner], week_index: usize) -> Result<(), DomainError> {
    // The immutable defaults_snapshot is shape-checked, but belongs to its creation
    // household. Membership closure applies to current cells and retained choices.
    for (index, cell) in week.cells.iter().enumerate() {
        if cell
            .attendee_ids
            .iter()
            .any(|id| !diners.iter().any(|d| &d.diner_id == id))
        {
            return Err(DomainError::new(
                ErrorCode::ForeignReference,
                &format!("/current_weeks/{week_index}/cells/{index}/attendee_ids"),
            ));
        }
    }
    for (index, selection) in week.plan.selections.iter().enumerate() {
        let path = format!("/current_weeks/{week_index}/plan/selections/{index}");
        let cell = week
            .cells
            .iter()
            .find(|c| c.slot == selection.slot)
            .ok_or_else(|| {
                DomainError::new(ErrorCode::ForeignReference, &format!("{path}/slot"))
            })?;
        for (i, serving) in selection.servings.iter().enumerate() {
            known_person(
                &serving.person,
                cell,
                diners,
                &format!("{path}/servings/{i}/person"),
            )?;
        }
        for (i, extra) in selection.extras.iter().enumerate() {
            for (j, recipient) in extra.recipients.iter().enumerate() {
                known_person(
                    &recipient.person,
                    cell,
                    diners,
                    &format!("{path}/extras/{i}/recipients/{j}/person"),
                )?;
            }
        }
        for (i, portion) in selection.meal_overrides.portions.iter().enumerate() {
            known_person(
                &portion.person,
                cell,
                diners,
                &format!("{path}/meal_overrides/portions/{i}/person"),
            )?;
        }
    }
    Ok(())
}

fn known_person(
    person: &PersonRef,
    cell: &Cell,
    diners: &[Diner],
    path: &str,
) -> Result<(), DomainError> {
    let known = match person {
        PersonRef::Diner { diner_id } => diners.iter().any(|d| &d.diner_id == diner_id),
        PersonRef::Guest {
            occasion_id,
            guest_id,
        } => cell.occasion.0.as_ref().is_some_and(|o| {
            &o.occasion_id == occasion_id && o.guests.iter().any(|g| &g.guest_id == guest_id)
        }),
    };
    if known {
        Ok(())
    } else {
        Err(DomainError::new(ErrorCode::ForeignReference, path))
    }
}

fn hard_set(diner: Option<&Diner>) -> Result<BTreeSet<Vec<u8>>, DomainError> {
    match diner.map(|d| &d.restrictions) {
        Some(RestrictionDeclaration::Confirmed { requirements, .. }) => {
            requirements.iter().map(canonical_bytes).collect()
        }
        // Absence and explicitly confirmed-none both contribute no hard requirements.
        _ => Ok(BTreeSet::new()),
    }
}
