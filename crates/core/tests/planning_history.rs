use mstrchf_contracts::*;
use mstrchf_core::history::selected_index_delta;
use mstrchf_core::history::{SelectedIndexDelta, validate_delta};

fn week() -> Week {
    parse_document(include_bytes!(
        "../../../tests/fixtures/households/p03a3/ruling004-current--selected-week.json"
    ))
    .unwrap()
}
fn at() -> Instant {
    Instant("2026-12-21T12:00:00Z".into())
}

#[test]
fn sh01_identical_and_counter_only_do_not_write_history() {
    let before = week();
    let mut after = before.clone();
    for changed in [false, true] {
        if changed {
            after.revision.0 += 1;
            after.plan.revision.0 += 1;
        }
        let delta = selected_index_delta(&[before.clone()], &[after.clone()], &at()).unwrap();
        assert!(delta.remove_slots.is_empty());
        assert!(delta.upsert.is_empty());
    }
}

#[test]
fn sh01_actual_change_upserts_exact_fields_at_commit_time() {
    let before = week();
    let mut after = before.clone();
    after.plan.selections.0[0].repeat_authorised = !before.plan.selections[0].repeat_authorised;
    let delta = selected_index_delta(&[before.clone()], &[after.clone()], &at()).unwrap();
    assert!(delta.remove_slots.is_empty());
    assert_eq!(delta.upsert.len(), 1);
    let s = &after.plan.selections[0];
    assert_eq!(
        delta.upsert[0],
        SelectionHistory {
            week_id: s.slot.week_id.clone(),
            slot: s.slot.clone(),
            recipe: s.recipe.clone(),
            servings: s.servings.clone(),
            extras: s.extras.clone(),
            repeat_authorised: s.repeat_authorised,
            selected_at: at(),
        }
    );
    assert_eq!(before, week());
}

#[test]
fn sh02_clear_removes_while_deactivation_retains_choice_and_reactivation_upserts() {
    let before = week();
    let selected = before.plan.selections[0].clone();
    let mut cleared = before.clone();
    cleared.plan.selections.0.clear();
    let delta = selected_index_delta(&[before.clone()], &[cleared], &at()).unwrap();
    assert_eq!(delta.remove_slots, vec![selected.slot.clone()]);
    assert!(delta.upsert.is_empty());
    let mut disabled = before.clone();
    disabled.meal_type_enabled.lunch = false;
    let delta = selected_index_delta(&[before.clone()], &[disabled.clone()], &at()).unwrap();
    assert_eq!(delta.remove_slots, vec![selected.slot.clone()]);
    assert!(delta.upsert.is_empty());
    assert_eq!(disabled.plan.selections[0], selected);
    let delta = selected_index_delta(&[disabled], &[before], &at()).unwrap();
    assert!(delta.remove_slots.is_empty());
    assert_eq!(delta.upsert.len(), 1);
    assert_eq!(delta.upsert[0].slot, selected.slot);
}

#[test]
fn disabled_retained_change_does_not_become_selected() {
    let mut before = week();
    before.meal_type_enabled.lunch = false;
    let mut after = before.clone();
    after.plan.selections.0[0].repeat_authorised = true;
    let delta = selected_index_delta(&[before], &[after], &at()).unwrap();
    assert!(delta.upsert.is_empty());
    assert!(delta.remove_slots.is_empty());
}

#[test]
fn history_checks_bounds_before_expansion() {
    let w = week();
    for size in [0, 5] {
        let err = selected_index_delta(&vec![w.clone(); size], &vec![w.clone(); size], &at())
            .unwrap_err();
        assert_eq!(err.code, ErrorCode::LimitExceeded);
    }
    assert!(selected_index_delta(&[w.clone()], &[], &at()).is_err());
}

#[test]
fn history_rejects_duplicates_mismatched_ranges_and_malformed_native_values() {
    let w = week();
    assert!(selected_index_delta(&[w.clone(), w.clone()], &[w.clone(), w.clone()], &at()).is_err());
    let mut wrong = w.clone();
    wrong.range.zone = ZoneId("UTC".into());
    assert!(selected_index_delta(&[w.clone()], &[wrong], &at()).is_err());
    let mut duplicate = w.clone();
    duplicate
        .plan
        .selections
        .0
        .push(w.plan.selections[0].clone());
    assert!(selected_index_delta(&[w.clone()], &[duplicate], &at()).is_err());
    assert!(selected_index_delta(&[w.clone()], &[w.clone()], &Instant("not-time".into())).is_err());
    let mut bad = w.clone();
    bad.plan.selections.0[0].servings.0.clear();
    assert!(selected_index_delta(&[w], &[bad], &at()).is_err());
}

#[test]
fn delta_is_disjoint_for_replacements_and_carries_no_order_mutation() {
    let before = week();
    let mut after = before.clone();
    after.plan.selections.0[0].origin = SelectionOrigin::Suggested;
    let snapshots = (before.clone(), after.clone());
    let delta = selected_index_delta(&[before.clone()], &[after.clone()], &at()).unwrap();
    assert!(
        delta
            .remove_slots
            .iter()
            .all(|s| delta.upsert.iter().all(|u| &u.slot != s))
    );
    assert_eq!((before, after), snapshots);
}

#[test]
fn sh07_public_delta_rejects_duplicate_and_overlapping_slots() {
    let w = week();
    let mut empty = w.clone();
    empty.plan.selections.0.clear();
    let delta = selected_index_delta(&[empty], &[w.clone()], &at()).unwrap();
    let record = delta.upsert[0].clone();
    let slot = record.slot.clone();
    validate_delta(&delta).unwrap();
    for bad in [
        SelectedIndexDelta {
            remove_slots: vec![slot.clone(), slot.clone()],
            upsert: vec![],
        },
        SelectedIndexDelta {
            remove_slots: vec![],
            upsert: vec![record.clone(), record.clone()],
        },
        SelectedIndexDelta {
            remove_slots: vec![slot],
            upsert: vec![record],
        },
    ] {
        assert_eq!(
            validate_delta(&bad).unwrap_err().code,
            ErrorCode::InvalidValue
        );
    }
}

#[test]
fn nested_native_bounds_reject_before_structural_expansion() {
    let before = week();
    let mut after = before.clone();
    after.cells.0.resize(22, after.cells[0].clone());
    assert_eq!(
        selected_index_delta(&[before.clone()], &[after], &at())
            .unwrap_err()
            .code,
        ErrorCode::LimitExceeded
    );
    let mut after = before.clone();
    after.plan.selections.0[0]
        .servings
        .0
        .resize(65, before.plan.selections[0].servings[0].clone());
    assert_eq!(
        selected_index_delta(&[before], &[after], &at())
            .unwrap_err()
            .code,
        ErrorCode::LimitExceeded
    );
}

#[test]
fn paired_week_enumeration_does_not_change_delta() {
    let context: PlanningContext = parse_document(include_bytes!(
        "../../../tests/fixtures/households/p03cd/ranking/canonical/context-base.jcs.json"
    ))
    .unwrap();
    let before = context.weeks.0;
    assert_eq!(before.len(), 4);
    let mut after = before.clone();
    after.reverse();
    let delta = selected_index_delta(&before, &after, &at()).unwrap();
    assert!(delta.remove_slots.is_empty());
    assert!(delta.upsert.is_empty());
}
