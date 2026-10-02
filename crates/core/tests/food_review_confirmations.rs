//! FN-R1-05: canonical confirmation identity classification through public wrappers.
use mstrchf_contracts::*;
use mstrchf_core::{demand, food::*};

#[path = "../../../tests/fixtures/food/batches/f11_fixture.rs"]
mod f11_fixture;

fn fixture() -> (SavedFoodBasis, FeasibilityInputs, FoodDerivation) {
    let saved = f11_fixture::fixture();
    let inputs = f11_fixture::feasibility();
    let derived = derive_plan(
        &saved.plan,
        &saved.context,
        &FoodInputs {
            evidence: &saved.evidence,
            identities: &saved.identities,
            schedule: &saved.schedule,
            feasibility: &inputs,
        },
    )
    .unwrap();
    (saved, inputs, derived)
}

fn rows(derived: &DerivedPlan, at: &Instant) -> Vec<EssentialsConfirmation> {
    derived
        .demand
        .lines
        .iter()
        .map(|line| EssentialsConfirmation {
            line_id: line.line_id.clone(),
            choice: EssentialsChoice::HaveEnough,
            displayed_quantity: line.quantity.clone(),
            demand_digest: derived.demand.digest.clone(),
            confirmed_at: at.clone(),
        })
        .collect()
}

fn assert_rejected(indices: &[usize], expected: ErrorCode) {
    let (saved, mut inputs, derived) = fixture();
    let canonical = rows(&derived.plan, &inputs.evaluated_at);
    inputs.confirmations = indices.iter().map(|&i| canonical[i].clone()).collect();
    // Deliberately do not rebuild a hash over invalid input. Both public wrappers
    // must classify invalid identities/order before testing that parent hash.
    let projection = demand::project_essentials(
        &derived.plan.demand,
        &inputs.confirmations,
        &inputs.evaluated_at,
    )
    .unwrap_err();
    assert_eq!(
        projection.code, expected,
        "shopping projection: {indices:?}"
    );
    let assessment = assess_feasibility(&derived.plan, &saved, &inputs).unwrap_err();
    assert_eq!(assessment.code, expected, "full assessment: {indices:?}");
}

#[test]
fn duplicate_adjacent_confirmation_identity_is_invalid_at_both_wrappers() {
    assert_rejected(&[0, 0], ErrorCode::InvalidValue);
}

#[test]
fn duplicate_nonadjacent_confirmation_identity_is_invalid_at_both_wrappers() {
    assert_rejected(&[0, 1, 0], ErrorCode::InvalidValue);
}

#[test]
fn duplicate_identity_precedes_an_earlier_order_inversion() {
    assert_rejected(&[1, 0, 1], ErrorCode::InvalidValue);
}

#[test]
fn conflicting_values_do_not_turn_one_line_identity_into_two_confirmations() {
    let (saved, mut inputs, derived) = fixture();
    let canonical = rows(&derived.plan, &inputs.evaluated_at);
    let mut conflicting = canonical[0].clone();
    conflicting.choice = EssentialsChoice::AddToShop;
    conflicting.displayed_quantity.amount = PositiveRational::new(1, 1).unwrap();
    inputs.confirmations = vec![canonical[0].clone(), canonical[1].clone(), conflicting];
    assert_eq!(
        demand::project_essentials(
            &derived.plan.demand,
            &inputs.confirmations,
            &inputs.evaluated_at,
        )
        .unwrap_err()
        .code,
        ErrorCode::InvalidValue
    );
    assert_eq!(
        assess_feasibility(&derived.plan, &saved, &inputs)
            .unwrap_err()
            .code,
        ErrorCode::InvalidValue
    );
}

#[test]
fn distinct_out_of_order_confirmations_are_noncanonical_at_both_wrappers() {
    assert_rejected(&[1, 0], ErrorCode::NoncanonicalValue);
}

#[test]
fn canonical_confirmations_pass_both_wrappers() {
    let (saved, mut inputs, derived) = fixture();
    inputs.confirmations = rows(&derived.plan, &inputs.evaluated_at);
    inputs.digest = digest_feasibility_inputs(&inputs).unwrap();
    let projection = demand::project_essentials(
        &derived.plan.demand,
        &inputs.confirmations,
        &inputs.evaluated_at,
    )
    .unwrap();
    assert_eq!(projection.required, derived.plan.demand);
    let assessment = assess_feasibility(&derived.plan, &saved, &inputs).unwrap();
    assert!(assessment.blocking.is_empty());
    assert_eq!(assessment.uses.len(), derived.feasibility.uses.len());
}
