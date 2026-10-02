//! Complete synthetic recipient-scoped extra premises; oracle frozen separately.
#![allow(dead_code)]
use super::native::{self, Fixture};
use mstrchf_contracts::*;
use mstrchf_core::{availability, demand, food::*, recipes::EvidencePin};

pub fn vegetarian() -> HardRequirement {
    HardRequirement::Diet {
        code: DietCode::Vegetarian,
    }
}

fn gluten_free() -> HardRequirement {
    HardRequirement::Diet {
        code: DietCode::GlutenFree,
    }
}

fn add_vegetarian_evidence(saved: &mut SavedFoodBasis, index: usize) {
    let recipe = &mut saved.context.recipes[index];
    let review_id = EvidenceId(format!("ev_{:032x}", 4200 + index));
    recipe.revision = Revision(recipe.revision.0 + 1);
    recipe.dietary_evidence = BoundedVec(vec![DietaryEvidence {
        evidence_id: recipe.reviews[0].evidence_id.clone(),
        requirement: vegetarian(),
        verdict: DietaryVerdict::Supported,
        source: recipe.source.clone(),
        review_id: review_id.clone(),
    }]);
    recipe.reviews.0.push(ReviewRecord {
        evidence_id: review_id.clone(),
        kind: ReviewKind::DietaryReview,
        actor: ReviewActor::Automated,
        at: Instant("2026-10-01T00:00:00Z".into()),
        note: Text("SYNTHETIC Vegetarian admission for this exact calendar fixture revision; no real dietary assurance".into()),
    });
    let evidence = &mut saved.evidence.recipes[index];
    evidence.admitted.push(EvidencePin {
        evidence_id: review_id,
        revision: Revision(1),
        digest: Digest(format!("{:064x}", 4200 + index)),
    });
    evidence
        .admitted
        .sort_by(|a, b| a.evidence_id.cmp(&b.evidence_id));
    native::refresh_recipe(saved, index);
}

fn refresh_requirements(saved: &mut SavedFoodBasis) {
    saved.context.weeks[0].plan = saved.plan.clone();
    saved.context.requirements = BoundedVec(
        availability::meal_requirements(&saved.context.household, &saved.context.weeks[0], &[])
            .unwrap()
            .into(),
    );
    saved.evidence.catalogue_digest = food_evidence_digest(&saved.evidence).unwrap();
}

/// A complete accepted base for two diners, with an evidence-free extra for the
/// unrestricted diner only. Both recipes eaten by Diner 1 explicitly support
/// Vegetarian; the extra's recipe and all food quantities remain unchanged.
pub fn asymmetric_extra() -> Fixture {
    let baseline = native::normal();
    let mut saved = baseline.historical.basis;
    saved.context.household.revision = Revision(saved.context.household.revision.0 + 1);
    saved.context.stamp.household_revision = saved.context.household.revision;
    saved.context.household.diners[0].restrictions = RestrictionDeclaration::Confirmed {
        confirmed_at: Instant("2026-10-01T00:00:00Z".into()),
        requirements: BoundedVec(vec![vegetarian()]),
    };
    assert!(matches!(
        saved.context.household.diners[1].restrictions,
        RestrictionDeclaration::ConfirmedNone { .. }
    ));
    assert!(saved.context.recipes[1].dietary_evidence.is_empty());
    add_vegetarian_evidence(&mut saved, 0);
    add_vegetarian_evidence(&mut saved, 2);
    refresh_requirements(&mut saved);
    let mut fixture = native::finish(saved);

    // Preserve C01's exact order clocks. Rebuild product scopes from the new
    // derived graph because phase/use keys include the revised recipe digest.
    let h = &mut fixture.historical;
    h.approved.approved_at = baseline.historical.approved.approved_at;
    h.order.approved_plan = h.approved.clone();
    h.order.confirmation = baseline.historical.order.confirmation;
    let current = &mut fixture.current;
    current.feasibility.delivery = DeliveryInput::Confirmed {
        order: h.order.clone(),
    };
    current.feasibility.evaluated_at = baseline.current.feasibility.evaluated_at;
    let graph = demand::reconstruct_use_graph(&h.derived, &h.basis).unwrap();
    let evidence_id = EvidenceId(format!("ev_{:032x}", 4300));
    let products = &mut current.feasibility.products;
    products.evidence = vec![EvidencePin {
        evidence_id: evidence_id.clone(),
        revision: Revision(1),
        digest: Digest(format!("{:064x}", 4300)),
    }];
    products.bindings.clear();
    products.facts.clear();
    for usage in graph.uses {
        let scope = ProductUseScope {
            product: ProductRef {
                retailer: Retailer::Tesco,
                product_id: Text(format!("synthetic-extra-projection-{}", usage.line_id.0)),
                variant_id: Nullable(None),
            },
            line_id: usage.line_id,
            use_key: usage.use_key,
        };
        products.bindings.push(ProductUseBinding {
            scope: scope.clone(),
            evidence_ids: vec![evidence_id.clone()],
        });
        products.facts.push(ProductUseFacts {
            scope,
            use_by: Some(DateFact {
                use_by: LocalDate("2026-10-14".into()),
                evidence_ids: vec![evidence_id.clone()],
            }),
            shorter: vec![],
        });
    }
    products
        .bindings
        .sort_by(|a, b| scope_cmp(&a.scope, &b.scope));
    products.facts.sort_by(|a, b| scope_cmp(&a.scope, &b.scope));
    products.digest = digest_product_uses(products).unwrap();
    current.sources[3].digest = order_source_digest(&current.sources[3], &h.order).unwrap();
    current.sources[4].digest = products.digest.clone();
    current.feasibility.digest = digest_feasibility_inputs(&current.feasibility).unwrap();
    current.digest = continuation_inputs_digest(current).unwrap();
    let checked = verify_historical(h).expect("asymmetric extra full historical integrity");
    let report =
        assess_continuation(&checked, current).expect("asymmetric extra full continuation");
    assert!(report.blocking.is_empty(), "positive blockers: {report:?}");
    assert!(report.warnings.is_empty(), "positive warnings: {report:?}");
    fixture
}

fn change_current_requirement(
    mut fixture: Fixture,
    diner: u64,
    requirement: HardRequirement,
) -> Fixture {
    let person = PersonRef::Diner {
        diner_id: DinerId(format!("dn_{diner:032x}")),
    };
    for meal in &mut fixture.current.requirements.meals {
        for attendee in &mut meal.attendees.0 {
            if attendee.person == person {
                attendee.hard_requirements = BoundedVec(vec![requirement.clone()]);
            }
        }
        let mut union = std::collections::BTreeMap::new();
        for attendee in &meal.attendees.0 {
            for requirement in &attendee.hard_requirements.0 {
                union.insert(canonical_bytes(requirement).unwrap(), requirement.clone());
            }
        }
        meal.shared_hard_requirements = BoundedVec(union.into_values().collect());
    }
    let current = &mut fixture.current;
    current.requirements.revision = Revision(current.requirements.revision.0 + 1);
    current.requirements.digest = live_requirements_digest(&current.requirements).unwrap();
    current.sources[6].revision = current.requirements.revision;
    current.sources[6].digest = current.requirements.digest.clone();
    current.digest = continuation_inputs_digest(current).unwrap();
    let checked =
        verify_historical(&fixture.historical).expect("control preserves accepted history");
    let report = assess_continuation(&checked, current).expect("control has valid current inputs");
    assert!(
        !report.blocking.is_empty(),
        "unsupported current dietary requirement must block"
    );
    fixture
}

pub fn actual_recipient_restricted() -> Fixture {
    change_current_requirement(asymmetric_extra(), 2, vegetarian())
}

pub fn base_requirement_unsupported() -> Fixture {
    change_current_requirement(asymmetric_extra(), 1, gluten_free())
}

/// An unapprovable proposal, deliberately not dressed as accepted history.
/// The caller should require FoodFailure::Blocked from required_identity_keys.
pub fn extra_spec_unsupported() -> SavedFoodBasis {
    let mut saved = asymmetric_extra().historical.basis;
    saved.context.recipes[0].revision = Revision(saved.context.recipes[0].revision.0 + 1);
    saved.context.recipes[0].extras[0].eligible_person_requirements =
        BoundedVec(vec![gluten_free()]);
    native::refresh_recipe(&mut saved, 0);
    refresh_requirements(&mut saved);
    saved
}
