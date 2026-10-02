//! Synthetic output-validator controls; full derivation is tested separately.
use mstrchf_contracts::*;
use mstrchf_core::{batches::*, portions::scale_recipe, recipes::*};

#[path = "../../../tests/fixtures/food/batches/f11_fixture.rs"]
mod f11_fixture;

#[test]
fn derivation_f11_exact_source_and_addition_only_demand() {
    use mstrchf_core::food::*;
    let saved = f11_fixture::fixture();
    let feasibility = f11_fixture::feasibility();
    let derived = derive_plan(
        &saved.plan,
        &saved.context,
        &FoodInputs {
            evidence: &saved.evidence,
            identities: &saved.identities,
            schedule: &saved.schedule,
            feasibility: &feasibility,
        },
    )
    .unwrap()
    .plan;
    assert_eq!(derived.batches.len(), 1);
    let component = &derived.batches[0].components[0];
    assert_eq!(component.produced.amount, p(1200, 1));
    assert_eq!(component.allocated.amount, p(1200, 1));
    assert_eq!(component.unallocated.amount, Rational::new(0, 1).unwrap());
    assert_eq!(
        derived
            .allocations
            .iter()
            .map(|a| a.quantity.amount)
            .collect::<Vec<_>>(),
        vec![p(600, 1), p(300, 1), p(300, 1)]
    );
    assert_eq!(
        derived
            .allocations
            .iter()
            .map(|a| a.planned_reheats)
            .collect::<Vec<_>>(),
        vec![0, 1, 1]
    );
    assert_eq!(
        derived
            .demand
            .lines
            .iter()
            .map(|l| l.quantity.amount)
            .collect::<Vec<_>>(),
        vec![
            p(480, 1),
            p(400, 1),
            p(160, 1),
            p(20, 1),
            p(1, 1),
            p(250, 1)
        ]
    );
    let original = match &component.original_production {
        ProductionMoment::Cooked { completed_at } => completed_at,
        _ => panic!("cooked source"),
    };
    assert_eq!(original, &f11_fixture::at(5, "18:00:00"));
    assert_eq!(
        derived
            .allocations
            .iter()
            .skip(1)
            .map(|a| a.use_at.instant.timestamp_seconds() - original.instant.timestamp_seconds())
            .collect::<Vec<_>>(),
        vec![64800, 151200]
    );
}

#[test]
fn derivation_f12_rejects_1350g_without_enlarging_1200g_source() {
    use mstrchf_core::food::*;
    let mut saved = f11_fixture::fixture();
    let selection = &mut saved.plan.selections[2];
    selection.servings[0].servings = p(3, 2);
    selection.meal_overrides.portions = BoundedVec(vec![PersonPortionOverride {
        person: selection.servings[0].person.clone(),
        preset: PortionPresetName::Regular,
        adjustment: p(3, 2),
    }]);
    saved.plan.batch_links[1].components[0].quantity.amount = p(450, 1);
    saved.context.weeks[0].plan = saved.plan.clone();
    saved.context.requirements = BoundedVec(
        mstrchf_core::availability::meal_requirements(
            &saved.context.household,
            &saved.context.weeks[0],
            &[],
        )
        .unwrap()
        .into(),
    );
    let before = saved.clone();
    let feasibility = f11_fixture::feasibility();
    let result = derive_plan(
        &saved.plan,
        &saved.context,
        &FoodInputs {
            evidence: &saved.evidence,
            identities: &saved.identities,
            schedule: &saved.schedule,
            feasibility: &feasibility,
        },
    );
    assert!(
        matches!(result, Err(FoodFailure::Blocked { ref conflicts, feasibility: None }) if conflicts.iter().any(|c| c.code == ErrorCode::DomainConflict))
    );
    assert_eq!(saved, before);
}

#[test]
fn derivation_known_rice_clock_conflict_is_blocked() {
    use mstrchf_core::food::*;
    let mut saved = f11_fixture::fixture();
    saved.evidence.classes[0].components[0].class = ComponentClass::Rice;
    saved.evidence.catalogue_digest = food_evidence_digest(&saved.evidence).unwrap();
    let feasibility = f11_fixture::feasibility();
    let result = derive_plan(
        &saved.plan,
        &saved.context,
        &FoodInputs {
            evidence: &saved.evidence,
            identities: &saved.identities,
            schedule: &saved.schedule,
            feasibility: &feasibility,
        },
    );
    assert!(
        matches!(result, Err(FoodFailure::Blocked { ref conflicts, feasibility: None })
        if conflicts.iter().any(|c| c.code == ErrorCode::DomainConflict)),
        "{result:?}"
    );
}

#[test]
fn output_validator_each_component_keeps_its_original_clock() {
    let (mut batch, mut allocations) = fixture();
    let component_id = ComponentId(format!("co_{:032x}", 2));
    let mut second = batch.components[0].clone();
    second.component_id = component_id.clone();
    second.storage_rule.0.as_mut().unwrap().component_id = component_id.clone();
    second.original_production = ProductionMoment::Unheated {
        preparation_started_at: moment("2026-10-05", "17:00:00"),
    };
    batch.components.0.push(second);
    let extra: Vec<_> = allocations
        .iter()
        .enumerate()
        .map(|(i, a)| Allocation {
            allocation_id: AllocationId(format!("al_{:032x}", 4 + i)),
            component_id: component_id.clone(),
            ..a.clone()
        })
        .collect();
    allocations.extend(extra);
    assert!(
        validate_allocations(&[batch.clone()], &allocations).is_err(),
        "earlier unheated source is 49h old despite later cooked component being exactly48h"
    );
    for a in allocations
        .iter_mut()
        .filter(|a| a.target_slot.date.0 == "2026-10-07")
    {
        a.use_at = moment("2026-10-07", "17:00:00");
    }
    assert!(validate_allocations(&[batch], &allocations).is_ok());
}

#[test]
fn reuse_zero_reheat_cap_and_cross_zone_instants() {
    let (batch, _) = fixture();
    let mut rule = batch.components[0].storage_rule.0.clone().unwrap();
    rule.max_reheats = 0;
    let original = moment("2026-10-05", "17:00:00");
    let at_limit = f11_fixture::at(7, "18:00:00");
    assert!(check_reuse(&original, &at_limit, &rule, 172800, 0).is_ok());
    assert!(check_reuse(&original, &at_limit, &rule, 172800, 1).is_err());
    assert!(check_reuse(&original, &f11_fixture::at(7, "18:00:01"), &rule, 172800, 0).is_err());
    assert!(check_reuse(&original, &at_limit, &rule, 172801, 0).is_err());
}

fn p(n: u64, d: u64) -> PositiveRational {
    PositiveRational::new(n, d).unwrap()
}
fn moment(date: &str, time: &str) -> ZonedMoment {
    ZonedMoment::new(
        LocalDate(date.into()),
        LocalTime(time.into()),
        ZoneId("UTC".into()),
        Resolution::Unambiguous,
        Instant(format!("{date}T{time}Z")),
    )
    .unwrap()
}
fn eid(n: u32) -> EvidenceId {
    EvidenceId(format!("ev_{n:032x}"))
}
fn quantity(n: u64) -> Quantity {
    Quantity {
        amount: p(n, 1),
        unit: Unit::G,
        form: IngredientForm::Cooked,
    }
}
fn source_recipe() -> (RecipeRevision, EvidenceBasis, ComponentClassView) {
    let mut recipe: RecipeRevision = parse_document(include_bytes!(
        "../../../tests/fixtures/food/admission/ruling004-current--complete-synthetic.json"
    ))
    .unwrap();
    recipe.r#yield.servings = p(4, 1);
    recipe.r#yield.components[0].quantity = quantity(1200);
    recipe.ingredients[0].quantity = Nullable(Some(quantity(1200)));
    recipe.scaling = ScalingPolicy::Linear {
        minimum: p(1, 100),
        maximum: p(100, 1),
    };
    let reference = RecipeRef {
        recipe_id: recipe.recipe_id.clone(),
        revision: recipe.revision,
        digest: recipe_digest(&recipe).unwrap(),
    };
    let basis = EvidenceBasis::new(
        reference.clone(),
        (1..=32)
            .map(|n| EvidencePin {
                evidence_id: eid(n),
                revision: Revision(1),
                digest: Digest(format!("{n:064x}")),
            })
            .collect(),
        vec![AssetDigest("a".repeat(64))],
    )
    .unwrap();
    let classes = ComponentClassView::new(
        reference.clone(),
        vec![ComponentClassEvidence {
            recipe: reference,
            component_id: recipe.components[0].component_id.clone(),
            class: ComponentClass::General,
            evidence_ids: vec![eid(8)],
        }],
    )
    .unwrap();
    (recipe, basis, classes)
}
fn fixture() -> (DerivedBatch, Vec<Allocation>) {
    let (recipe, basis, classes) = source_recipe();
    let reviewed = assess_revision(&recipe, &basis, &classes).unwrap();
    let scaled = scale_recipe(&reviewed, p(4, 1)).unwrap();
    assert_eq!(scaled.produced_servings, p(4, 1));
    let slot = SlotId {
        week_id: WeekId(format!("wk_{:032x}", 1)),
        date: LocalDate("2026-10-05".into()),
        meal_type: MealType::Dinner,
    };
    let batch_id = BatchId(format!("ba_{:032x}", 1));
    let component = scaled.components[0].component_id.clone();
    let production = moment("2026-10-05", "18:00:00");
    let mut rule = recipe.storage[0].clone();
    rule.max_chilled_seconds = 172800;
    rule.max_reheats = 1;
    let batch = DerivedBatch {
        batch_id: batch_id.clone(),
        recipe: basis.recipe,
        source_slot: slot.clone(),
        components: BoundedVec(vec![DerivedComponent {
            component_id: component.clone(),
            produced: scaled.components[0].quantity.clone(),
            allocated: quantity(1200),
            unallocated: NonnegativeQuantity {
                amount: Rational::new(0, 1).unwrap(),
                unit: Unit::G,
                form: IngredientForm::Cooked,
            },
            original_production: ProductionMoment::Cooked {
                completed_at: production.clone(),
            },
            storage_rule: Nullable(Some(rule)),
        }]),
    };
    let allocations = [600, 300, 300]
        .into_iter()
        .enumerate()
        .map(|(i, n)| Allocation {
            allocation_id: AllocationId(format!("al_{:032x}", i + 1)),
            batch_id: batch_id.clone(),
            component_id: component.clone(),
            target_slot: SlotId {
                date: LocalDate(format!("2026-10-{:02}", 5 + i)),
                ..slot.clone()
            },
            person: PersonRef::Diner {
                diner_id: DinerId(format!("dn_{:032x}", 1)),
            },
            servings: p(n / 300, 1),
            quantity: quantity(n),
            use_at: moment(&format!("2026-10-{:02}", 5 + i), "18:00:00"),
            planned_reheats: u8::from(i != 0),
            remix_recipe: Nullable(None),
        })
        .collect();
    (batch, allocations)
}
#[test]
fn output_validator_exact_conservation_and_no_enlargement() {
    let (batch, mut allocations) = fixture();
    assert!(validate_allocations(std::slice::from_ref(&batch), &allocations).is_ok());
    allocations[2].quantity = quantity(450);
    allocations[2].servings = p(3, 2);
    assert!(validate_allocations(&[batch], &allocations).is_err());
}
#[test]
fn output_validator_rejects_second_reheat_and_duplicate_disjoint_identity() {
    let (batch, mut allocations) = fixture();
    allocations[1].planned_reheats = 2;
    assert!(validate_allocations(std::slice::from_ref(&batch), &allocations).is_err());
    allocations[1].planned_reheats = 1;
    allocations[2].allocation_id = allocations[1].allocation_id.clone();
    assert!(validate_allocations(&[batch], &allocations).is_err());
}
#[test]
fn reuse_boundaries_general_rice_shorter_and_negative_age() {
    let (batch, _) = fixture();
    let rule = batch.components[0].storage_rule.0.as_ref().unwrap();
    let original = moment("2026-10-05", "18:00:00");
    assert!(
        check_reuse(
            &original,
            &moment("2026-10-07", "18:00:00"),
            rule,
            172800,
            1
        )
        .is_ok()
    );
    assert!(
        check_reuse(
            &original,
            &moment("2026-10-07", "18:00:01"),
            rule,
            172800,
            1
        )
        .is_err()
    );
    assert!(check_reuse(&original, &moment("2026-10-06", "18:00:00"), rule, 86400, 1).is_ok());
    assert!(check_reuse(&original, &moment("2026-10-06", "18:00:01"), rule, 86400, 1).is_err());
    let mut shorter = rule.clone();
    shorter.max_chilled_seconds = 64800;
    assert!(
        check_reuse(
            &original,
            &moment("2026-10-06", "12:00:00"),
            &shorter,
            172800,
            0
        )
        .is_ok()
    );
    assert!(
        check_reuse(
            &original,
            &moment("2026-10-06", "12:00:01"),
            &shorter,
            172800,
            0
        )
        .is_err()
    );
    assert!(
        check_reuse(
            &original,
            &moment("2026-10-05", "17:59:59"),
            rule,
            172800,
            0
        )
        .is_err()
    );
}
#[test]
fn output_validator_rejects_foreign_units_missing_storage_and_forged_clock() {
    let (mut batch, mut allocations) = fixture();
    allocations[1].quantity.unit = Unit::Ml;
    assert!(validate_allocations(&[batch.clone()], &allocations).is_err());
    allocations[1].quantity.unit = Unit::G;
    batch.components[0].storage_rule = Nullable(None);
    assert!(validate_allocations(&[batch.clone()], &allocations).is_err());
    allocations.truncate(1);
    batch.components[0].allocated = quantity(600);
    batch.components[0].unallocated.amount = Rational::new(600, 1).unwrap();
    assert!(validate_allocations(&[batch.clone()], &allocations).is_ok());
    if let ProductionMoment::Cooked { completed_at } = &mut batch.components[0].original_production
    {
        completed_at.instant = Instant("invalid".into());
    }
    assert!(validate_allocations(&[batch], &allocations).is_err());
}
