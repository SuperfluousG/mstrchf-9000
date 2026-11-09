//! Fresh derivation reuses only its private core proof; public saved inputs
//! still reconstruct the exact plan, while every feasibility input is checked.
use mstrchf_contracts::*;
use mstrchf_core::{demand::reconstruct_use_graph, food::*};

#[path = "../../../tests/fixtures/food/batches/f11_fixture.rs"]
mod f11;

fn derive(
    saved: &SavedFoodBasis,
    inputs: &FeasibilityInputs,
) -> Result<FoodDerivation, FoodFailure> {
    derive_plan(
        &saved.plan,
        &saved.context,
        &FoodInputs {
            evidence: &saved.evidence,
            identities: &saved.identities,
            schedule: &saved.schedule,
            feasibility: inputs,
        },
    )
}

// Re-enter through the independently checked public saved-state boundary.
// Compare the complete return and error, including retained blocked reports.
fn public_route(
    saved: &SavedFoodBasis,
    plan: &DerivedPlan,
    inputs: &FeasibilityInputs,
) -> Result<FoodDerivation, FoodFailure> {
    let feasibility = assess_feasibility(plan, saved, inputs)?;
    if !feasibility.blocking.is_empty() {
        return Err(FoodFailure::Blocked {
            conflicts: feasibility
                .blocking
                .iter()
                .map(|item| item.conflict.clone())
                .collect(),
            feasibility: Some(feasibility),
        });
    }
    Ok(FoodDerivation {
        plan: plan.clone(),
        shopping: project_essentials(&plan.demand, &inputs.confirmations, &inputs.evaluated_at)?,
        feasibility,
    })
}

#[test]
fn fresh_and_public_saved_paths_have_identical_source_leftover_outputs() {
    let saved = f11::fixture();
    let inputs = f11::feasibility();
    let fresh = derive(&saved, &inputs).unwrap();
    assert_eq!(fresh.plan.batches.len(), 1);
    assert_eq!(fresh.plan.demand.lines.len(), 6);
    assert!(!saved.plan.batch_links.is_empty());
    let expected_graph = reconstruct_use_graph(&fresh.plan, &saved).unwrap();
    for _ in 0..2 {
        assert_eq!(
            derive(&saved, &inputs),
            public_route(&saved, &fresh.plan, &inputs)
        );
        assert_eq!(
            reconstruct_use_graph(&fresh.plan, &saved).unwrap(),
            expected_graph
        );
    }
    let cold_saved = saved.clone();
    let cold_inputs = inputs.clone();
    assert_eq!(
        std::thread::spawn(move || derive(&cold_saved, &cold_inputs))
            .join()
            .unwrap(),
        Ok(fresh)
    );
}

#[test]
fn fresh_and_public_paths_preserve_exact_feasibility_errors_and_blocked_reports() {
    let saved = f11::fixture();
    let inputs = f11::feasibility();
    let plan = derive(&saved, &inputs).unwrap().plan;
    let mut cases = vec![];
    let mut later = inputs.clone();
    later.evaluated_at = f11::at(6, "12:00:00").instant;
    later.digest = digest_feasibility_inputs(&later).unwrap();
    cases.push(("later evaluation", later, false));
    let mut blocked = inputs.clone();
    blocked.delivery = DeliveryInput::Proposed {
        start: f11::at(6, "13:00:00"),
        end: f11::at(6, "14:00:00"),
        revision: Revision(1),
    };
    blocked.digest = digest_feasibility_inputs(&blocked).unwrap();
    cases.push(("late delivery retains blocked report", blocked, true));
    let mut product_digest = inputs.clone();
    product_digest.products.digest = PayloadDigest("f".repeat(64));
    cases.push(("forged product digest", product_digest, true));
    let mut digest = inputs.clone();
    digest.digest = PayloadDigest("e".repeat(64));
    cases.push(("forged feasibility digest", digest, true));
    let mut expired = inputs.clone();
    expired.products.valid_until = Some(expired.evaluated_at.clone());
    expired.products.digest = digest_product_uses(&expired.products).unwrap();
    cases.push(("expiry equality", expired, true));
    let mut future = inputs.clone();
    let line = &plan.demand.lines[0];
    future.confirmations.push(EssentialsConfirmation {
        line_id: line.line_id.clone(),
        choice: EssentialsChoice::HaveEnough,
        displayed_quantity: line.quantity.clone(),
        demand_digest: plan.demand.digest.clone(),
        confirmed_at: f11::at(5, "12:00:00").instant,
    });
    cases.push(("future confirmation", future, true));
    for (label, changed, error) in cases {
        let expected = public_route(&saved, &plan, &changed);
        assert_eq!(expected.is_err(), error, "{label}: {expected:?}");
        if label.starts_with("late delivery") {
            assert!(matches!(
                &expected,
                Err(FoodFailure::Blocked {
                    feasibility: Some(_),
                    ..
                })
            ));
        }
        assert_eq!(derive(&saved, &changed), expected, "{label}");
        assert_eq!(derive(&saved, &changed), expected, "repeated {label}");
    }
}

#[test]
fn saved_public_boundaries_reconstruct_after_fresh_success_even_with_resigned_outputs() {
    let saved = f11::fixture();
    let inputs = f11::feasibility();
    let fresh = derive(&saved, &inputs).unwrap();
    let mut forged = fresh.plan.clone();
    forged.demand.lines[0].quantity.amount = PositiveRational::new(999, 1).unwrap();
    forged.digest = derived_digest(&DerivedMaterial::from(&forged)).unwrap();
    let expected = DomainError::new(ErrorCode::DigestMismatch, "/derived");
    assert_eq!(
        assess_feasibility(&forged, &saved, &inputs).unwrap_err(),
        expected
    );
    assert_eq!(
        reconstruct_use_graph(&forged, &saved).unwrap_err(),
        expected
    );
    // Basis fields cannot borrow authority from a previously derived plan or
    // from unchanged revision/digest claims, on either public entry point.
    let mutations: Vec<(&str, fn(&mut SavedFoodBasis))> = vec![
        ("recipe", |s| {
            s.context.recipes[0].ingredients[0].name = NameText("Changed".into())
        }),
        ("evidence", |s| {
            s.evidence.catalogue_digest = PayloadDigest("a".repeat(64))
        }),
        ("identity", |s| {
            s.identities.digest = PayloadDigest("b".repeat(64))
        }),
        ("schedule", |s| {
            s.schedule.occupied.digest = PayloadDigest("c".repeat(64))
        }),
    ];
    for (label, mutate) in mutations {
        let mut altered = saved.clone();
        mutate(&mut altered);
        let expected = assess_feasibility(&fresh.plan, &altered, &inputs).unwrap_err();
        assert_eq!(
            reconstruct_use_graph(&fresh.plan, &altered).unwrap_err(),
            expected,
            "{label}"
        );
        assert_eq!(
            assess_feasibility(&fresh.plan, &altered, &inputs).unwrap_err(),
            expected,
            "repeat {label}"
        );
    }
    assert_eq!(derive(&saved, &inputs).unwrap(), fresh);
}
