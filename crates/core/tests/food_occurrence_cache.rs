//! Differential checks for the process-local exact-input occurrence proof.
use mstrchf_contracts::*;
use mstrchf_core::food::*;

#[path = "../../../tests/fixtures/food/batches/f11_fixture.rs"]
mod f11;

fn derive(saved: &SavedFoodBasis, feasibility: &FeasibilityInputs) -> Result<FoodDerivation, FoodFailure> {
    derive_plan(&saved.plan, &saved.context, &FoodInputs {
        evidence: &saved.evidence, identities: &saved.identities,
        schedule: &saved.schedule, feasibility,
    })
}

fn cold(saved: &SavedFoodBasis, feasibility: &FeasibilityInputs) -> Result<FoodDerivation, FoodFailure> {
    let saved=saved.clone();let feasibility=feasibility.clone();
    std::thread::spawn(move || derive(&saved,&feasibility)).join().unwrap()
}

#[test]
fn exact_warm_inputs_match_cold_source_leftover_schedule_and_demand() {
    let saved=f11::fixture();let feasibility=f11::feasibility();
    let expected=cold(&saved,&feasibility).unwrap();
    assert_eq!(expected.plan.batches.len(),1);
    assert_eq!(expected.plan.demand.lines.len(),6);
    for _ in 0..3 {assert_eq!(derive(&saved,&feasibility).unwrap(),expected);}
    // A successful different input replaces the single proof. Returning to the
    // earlier value must still reconstruct exactly, without a persistent token.
    let mut later=saved.clone();later.plan.revision=later.plan.revision.checked_increment().unwrap();
    let changed=cold(&later,&feasibility);assert!(changed.is_ok());assert_eq!(derive(&later,&feasibility),changed);
    assert_eq!(derive(&saved,&feasibility).unwrap(),expected);
}

#[test]
fn changed_plan_context_and_evidence_never_reuse_prior_success_or_claimed_digests() {
    let saved=f11::fixture();let feasibility=f11::feasibility();
    let mutations:Vec<(&str,fn(&mut SavedFoodBasis))>=vec![
        ("selection quantity",|s|s.plan.selections[0].servings[0].servings.0.denominator=DecimalU64(0)),
        ("batch link",|s|s.plan.batch_links[0].batch_id=BatchId("ba_ffffffffffffffffffffffffffffffff".into())),
        ("household",|s|s.context.household.diners[0].restrictions=RestrictionDeclaration::Unconfirmed{}),
        ("requirements",|s|{let selected=s.plan.selections[0].slot.clone();s.context.requirements.0.retain(|meal|meal.slot!=selected);}),
        ("recipe body with retained pin",|s|s.context.recipes[0].ingredients[0].name=NameText("Tampered ingredient".into())),
        ("context stamp",|s|s.context.stamp.catalogue_revision=Revision(999)),
        ("evidence body with retained digest",|s|s.evidence.resources[0].prep_id=65535),
        ("evidence digest claim",|s|s.evidence.catalogue_digest=PayloadDigest("f".repeat(64))),
    ];
    for (label,mutate) in mutations {
        assert!(derive(&saved,&feasibility).is_ok());
        let mut changed=saved.clone();mutate(&mut changed);
        let expected=cold(&changed,&feasibility);
        assert!(expected.is_err(),"{label} must not remain valid");
        assert_eq!(derive(&changed,&feasibility),expected,"{label}");
        assert_eq!(derive(&changed,&feasibility),expected,"repeat failure: {label}");
        assert!(derive(&saved,&feasibility).is_ok(),"failure must not poison valid input: {label}");
    }
}

#[test]
fn schedule_and_time_sensitive_feasibility_still_validate_after_occurrence_reuse() {
    let saved=f11::fixture();let feasibility=f11::feasibility();
    assert!(derive(&saved,&feasibility).is_ok());
    let mut schedule=saved.clone();schedule.schedule.layout.fixed_starts.clear();
    let expected=cold(&schedule,&feasibility);
    assert_eq!(derive(&schedule,&feasibility),expected);
    let mut changed=feasibility.clone();changed.evaluated_at=Instant("2026-10-06T12:00:00Z".into());changed.digest=digest_feasibility_inputs(&changed).unwrap();
    assert_eq!(derive(&saved,&changed),cold(&saved,&changed));
    changed.digest=PayloadDigest("e".repeat(64));
    let expected=cold(&saved,&changed);assert!(expected.is_err());assert_eq!(derive(&saved,&changed),expected);
}
