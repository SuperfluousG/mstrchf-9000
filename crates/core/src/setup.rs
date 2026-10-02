//! Pure setup transitions and saved-draft binding. No counters are assigned here.
use crate::household::{profile_from_draft, validate_draft_progress, validate_profile};
use mstrchf_contracts::*;
use std::collections::BTreeSet;

const EDITS: &str = "/command/data/restriction_issue_edits";

/// Validate an explicit replacement of the current draft at its existing revision.
///
/// The store must supply authoritative current data, policy and admission time,
/// enforce CAS/authority, and commit the mutation and receipt atomically. This
/// pure check cannot attest what the local user saw or grant a fresh effect.
pub fn validate_setup_draft_transition(
    current: &HouseholdDraft,
    proposed: &HouseholdDraft,
    edits: &[SetupRestrictionEdit],
    policy: &PrivacyPolicy,
    evaluated_at: &Instant,
) -> Result<(), DomainError> {
    if edits.len() > 64 {
        return Err(DomainError::new(ErrorCode::LimitExceeded, EDITS));
    }
    draft_bounds(current)?;
    draft_bounds(proposed)?;
    validate_draft_progress(current, policy, evaluated_at)?;
    validate_draft_progress(proposed, policy, evaluated_at)?;
    same_household(&current.household_id, &proposed.household_id)?;
    same_revision(current.revision, proposed.revision)?;

    let mut owners = BTreeSet::new();
    for edit in edits {
        shape(edit)?;
        if !owners.insert(edit_owner(edit)) {
            return Err(edit_error());
        }
    }
    if !edits
        .windows(2)
        .all(|w| edit_owner(&w[0]) < edit_owner(&w[1]))
    {
        return Err(DomainError::new(ErrorCode::NoncanonicalValue, EDITS));
    }

    // Old issues are already unique and canonically ordered by validated owner.
    // New issues create no obligation to acknowledge an old issue.
    let mut required = Vec::new();
    for old in current.unresolved_restrictions.iter() {
        let retained = proposed
            .unresolved_restrictions
            .iter()
            .find(|issue| issue.diner_id == old.diner_id);
        let edit = if let Some(new) = retained {
            if new.supplied_text == old.supplied_text {
                continue;
            }
            SetupRestrictionEdit::ReplaceText {
                diner_id: old.diner_id.clone(),
            }
        } else if let Some(owner) = proposed.diners.iter().find(|d| d.diner_id == old.diner_id) {
            // Real draft admission above checks the supported declaration and
            // its confirmation time; omission alone cannot confirm anything.
            if matches!(owner.restrictions, RestrictionDeclaration::Unconfirmed {}) {
                return Err(edit_error());
            }
            SetupRestrictionEdit::Resolve {
                diner_id: old.diner_id.clone(),
            }
        } else {
            // Real A1 admission above checks every usual-week attendance ref.
            SetupRestrictionEdit::RemoveDiner {
                diner_id: old.diner_id.clone(),
            }
        };
        required.push(edit);
    }
    if edits != required {
        return Err(edit_error());
    }
    Ok(())
}

/// Exact D(P,r): validated profile content and the independently supplied draft
/// revision. Never invent defaults, confirmations, timestamps or increments.
pub fn draft_from_profile(
    profile: &HouseholdProfile,
    draft_revision: Revision,
    policy: &PrivacyPolicy,
    evaluated_at: &Instant,
) -> Result<HouseholdDraft, DomainError> {
    profile_bounds(profile)?;
    shape(&draft_revision)?;
    validate_profile(profile, policy, evaluated_at)?;
    let draft = HouseholdDraft {
        schema_version: profile.schema_version,
        household_id: profile.household_id.clone(),
        revision: draft_revision,
        completed_stages: BoundedVec(vec![
            HouseholdStage::Connections,
            HouseholdStage::FoodPreferences,
            HouseholdStage::Household,
            HouseholdStage::UsualWeek,
        ]),
        unresolved_restrictions: BoundedVec(vec![]),
        diners: BoundedVec(profile.diners.0.clone()),
        preferences: Nullable(Some(profile.preferences.clone())),
        usual_week: Nullable(Some(profile.usual_week.clone())),
        connections: Nullable(Some(profile.connections.clone())),
    };
    validate_draft_progress(&draft, policy, evaluated_at)?;
    Ok(draft)
}

/// Supplement profile admission with exact saved-draft binding.
///
/// This does NOT replace `prepare_profile_update`, complete retained-week and
/// original-approval audits, food checks, store CAS or authoritative readiness
/// at application/exposure. No fabricated empty week batch is used here.
pub fn validate_profile_draft_binding(
    current: Option<&HouseholdProfile>,
    saved: &HouseholdDraft,
    proposed: &HouseholdProfile,
    policy: &PrivacyPolicy,
    evaluated_at: &Instant,
) -> Result<(), DomainError> {
    draft_bounds(saved)?;
    profile_bounds(proposed)?;
    if let Some(current) = current {
        profile_bounds(current)?;
    }
    validate_draft_progress(saved, policy, evaluated_at)?;
    validate_profile(proposed, policy, evaluated_at)?;
    if let Some(current) = current {
        validate_profile(current, policy, evaluated_at)?;
        same_household(&current.household_id, &saved.household_id)?;
        same_household(&current.household_id, &proposed.household_id)?;
        same_revision(current.revision, proposed.revision)?;
    } else {
        same_household(&saved.household_id, &proposed.household_id)?;
    }
    let Some(current) = current else {
        let promoted = profile_from_draft(saved, policy, evaluated_at)?;
        same_revision(promoted.revision, proposed.revision)?;
        return exact_profile(&promoted, proposed);
    };
    if saved == &draft_from_profile(current, saved.revision, policy, evaluated_at)? {
        return Ok(());
    }

    // A valid partial draft is still pending data. Promotion must really succeed
    // before its content can be applied; no partial merge or refresh is allowed.
    let mut promoted =
        profile_from_draft(saved, policy, evaluated_at).map_err(|_| draft_conflict())?;
    promoted.revision = current.revision;
    exact_profile(&promoted, proposed)
}

fn exact_profile(
    expected: &HouseholdProfile,
    proposed: &HouseholdProfile,
) -> Result<(), DomainError> {
    if expected == proposed {
        Ok(())
    } else {
        Err(draft_conflict())
    }
}
fn draft_conflict() -> DomainError {
    DomainError::new(ErrorCode::DomainConflict, "/household_draft")
}
fn edit_error() -> DomainError {
    DomainError::new(ErrorCode::InvalidValue, EDITS)
}
fn edit_owner(edit: &SetupRestrictionEdit) -> &DinerId {
    match edit {
        SetupRestrictionEdit::ReplaceText { diner_id }
        | SetupRestrictionEdit::Resolve { diner_id }
        | SetupRestrictionEdit::RemoveDiner { diner_id } => diner_id,
    }
}
fn same_household(a: &HouseholdId, b: &HouseholdId) -> Result<(), DomainError> {
    if a == b {
        Ok(())
    } else {
        Err(DomainError::new(
            ErrorCode::IdentityMismatch,
            "/household_id",
        ))
    }
}
fn same_revision(current: Revision, proposed: Revision) -> Result<(), DomainError> {
    if current == proposed {
        Ok(())
    } else {
        Err(DomainError::new(ErrorCode::ContextStale, "/revision"))
    }
}
fn shape(value: &impl ValidateShape) -> Result<(), DomainError> {
    value.validate_shape().map_err(|issues| DomainError {
        code: issues.first().map_or(ErrorCode::InvalidValue, |i| i.code),
        truncated: issues.len() > 100,
        issues: BoundedVec(issues.into_iter().take(100).collect()),
        retry: Retry::Never,
    })
}

// Public Rust fields can bypass deserialisation bounds. Check collection sizes
// before ValidateShape expands the whole object or a projection clones it.
// These checks supplement, never replace, the actual contract/A1 validators.
fn bound<T, const MIN: usize, const MAX: usize>(
    value: &BoundedVec<T, MIN, MAX>,
    path: &str,
) -> Result<(), DomainError> {
    if value.len() > MAX {
        Err(DomainError::new(ErrorCode::LimitExceeded, path))
    } else if value.len() < MIN {
        Err(DomainError::new(ErrorCode::InvalidValue, path))
    } else {
        Ok(())
    }
}
fn draft_bounds(draft: &HouseholdDraft) -> Result<(), DomainError> {
    bound(&draft.completed_stages, "/completed_stages")?;
    bound(&draft.unresolved_restrictions, "/unresolved_restrictions")?;
    bound(&draft.diners, "/diners")?;
    for issue in draft.unresolved_restrictions.iter() {
        if issue.supplied_text.0.chars().take(4001).count() > 4000 {
            return Err(DomainError::new(
                ErrorCode::LimitExceeded,
                "/unresolved_restrictions",
            ));
        }
    }
    household_bounds(
        &draft.diners,
        draft.preferences.0.as_ref(),
        draft.usual_week.0.as_ref(),
        draft.connections.0.as_ref(),
    )
}
fn profile_bounds(profile: &HouseholdProfile) -> Result<(), DomainError> {
    bound(&profile.diners, "/diners")?;
    household_bounds(
        &profile.diners,
        Some(&profile.preferences),
        Some(&profile.usual_week),
        Some(&profile.connections),
    )
}
fn household_bounds(
    diners: &[Diner],
    preferences: Option<&Preferences>,
    usual: Option<&UsualWeek>,
    connections: Option<&Connections>,
) -> Result<(), DomainError> {
    for diner in diners {
        bound(&diner.dislikes, "/diners/dislikes")?;
        bound(
            &diner.portions.meal_overrides,
            "/diners/portions/meal_overrides",
        )?;
        if let RestrictionDeclaration::Confirmed { requirements, .. } = &diner.restrictions {
            bound(requirements, "/diners/restrictions/data/requirements")?;
        }
    }
    if let Some(p) = preferences {
        bound(&p.dislikes, "/preferences/dislikes")?;
        bound(&p.equipment, "/preferences/equipment")?;
        bound(&p.meal_times, "/preferences/meal_times")?;
    }
    if let Some(u) = usual {
        bound(&u.enabled_meal_types, "/usual_week/enabled_meal_types")?;
        bound(&u.cells, "/usual_week/cells")?;
        for cell in u.cells.iter() {
            bound(&cell.attendee_ids, "/usual_week/cells/attendee_ids")?;
        }
    }
    if let Some(c) = connections {
        bound(&c.delivery_preferences, "/connections/delivery_preferences")?;
        bound(&c.reminders.channels, "/connections/reminders/channels")?;
        bound(&c.reminders.prompts, "/connections/reminders/prompts")?;
    }
    Ok(())
}
