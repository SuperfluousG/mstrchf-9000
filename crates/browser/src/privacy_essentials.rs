//! Narrow carry of an existing explicit pantry choice across a privacy-only
//! transition. No stock, new confirmation, food admission or AI permission is
//! inferred here. Both complete food derivations use the same supplied clock.
use crate::{calendar_view::material_at, catalogue::RuntimeCatalogue};
use mstrchf_contracts::*;

fn same_food(before: &DerivedPlan, after: &DerivedPlan) -> bool {
    before.week_id == after.week_id
        && before.plan_revision == after.plan_revision
        && before.recipe_refs == after.recipe_refs
        && before.batches == after.batches
        && before.allocations == after.allocations
        && before.prep_sessions == after.prep_sessions
        && before.demand.lines == after.demand.lines
        && before.warnings == after.warnings
        && before.derivation_policy_version == after.derivation_policy_version
}

pub(crate) fn carry(
    view: &RuntimeCatalogue,
    before: &Snapshot,
    after: &Snapshot,
    confirmations: &[EssentialsConfirmation],
    at: &Instant,
) -> Vec<EssentialsConfirmation> {
    let mut result = confirmations.to_vec();
    if confirmations.is_empty() {
        return result;
    }
    let (Some(old_profile), Some(new_profile)) = (&before.profile.0, &after.profile.0) else {
        return result;
    };
    let mut proven = Vec::new();
    for old_week in &before.weeks.0 {
        let Some(new_week) = after
            .weeks
            .iter()
            .find(|week| week.range.week_id == old_week.range.week_id)
        else {
            continue;
        };
        let Ok(Some((_, old_food))) = material_at(view, before, old_profile, old_week, at) else {
            continue;
        };
        let Ok(Some((_, new_food))) = material_at(view, after, new_profile, new_week, at) else {
            continue;
        };
        if same_food(&old_food.plan, &new_food.plan) {
            proven.push((old_food.plan.demand, new_food.plan.demand));
        }
    }
    for confirmation in &mut result {
        if confirmation.confirmed_at > *at {
            continue;
        }
        let matches: Vec<_> = proven
            .iter()
            .filter(|(old, _)| {
                old.lines
                    .iter()
                    .any(|line| line.line_id == confirmation.line_id && line.essential)
                    && validate_essentials_binding(confirmation, old).is_ok()
            })
            .collect();
        // Ambiguous membership, unsupported food, missing evidence and already
        // stale confirmations convey no permission to renew a pantry choice.
        if let [(old, new)] = matches.as_slice() {
            debug_assert_eq!(old.lines, new.lines);
            confirmation.demand_digest = new.digest.clone();
        }
    }
    result
}
