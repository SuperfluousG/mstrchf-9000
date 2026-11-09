//! Isolated typed feasibility probe, NOT food admission or a reviewed recipe.
//! Synthetic timings, component units, safe-cooling and chilling remain explicit
//! TEST PREMISES, never human cooking/calibration, transport or product evidence.
use mstrchf_contracts::*;
use mstrchf_core::{
    batches,
    eligibility::{Eligibility, eligible},
    food::*,
    recipes::assess_revision,
};
use serde_json::Value;
#[path = "support/original_component.rs"]
mod original_component;
use original_component::*;
#[test]
fn exact_retained_inputs_and_old_bytes_are_not_new_food_authority() {
    assert_eq!(
        bytes_digest(include_bytes!(
            "../../../tests/fixtures/food/admission/ruling004-current--complete-synthetic.json"
        ))
        .0,
        "5ebcc2c38faa7943a75b58085c244e72f6e81ef9eec0286e255c7dfc2668f43a"
    );
    assert_eq!(
        bytes_digest(include_bytes!(
            "../../../tests/fixtures/food/batches/native-v4-subject-basis.json"
        ))
        .0,
        "95e160dac957dddd2c890f1b35e31d6afb0fbc0abe5203af66263fc3050002f4"
    );
    let (source, cold) = recipes();
    assert_eq!(
        reference(&source).digest.0,
        "40419e27a7e8d393f48cc793d84f69fecc9980ea94d1d35f04432f3542f23822"
    );
    assert_eq!(
        reference(&cold).digest.0,
        "dba9817a48761bc87159a6956fe81d7175e71d999a576689a7bfbf4e09d84568"
    );
    assert!(
        source
            .reviews
            .iter()
            .chain(cold.reviews.iter())
            .all(|review| review.actor == ReviewActor::Automated
                && review.kind != ReviewKind::CookTest)
    );
    let prior: Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/food/batches/native-v4-subject-basis.json"
    ))
    .unwrap();
    let context: PlanningContext =
        serde_json::from_value(prior["value"]["context"].clone()).unwrap();
    for recipe in context.recipes.iter() {
        assert!(
            context.stamp.recipe_refs.contains(&reference(recipe)),
            "Existing recipe JCS pin remains exact after wire extension"
        );
    }
    println!(
        "SYNTHETIC source ref {:?}; cold-service ref {:?}; source intake {INTAKE_HASH}",
        reference(&source),
        reference(&cold)
    );
}
#[test]
fn same_source_home_cold_use_conserves_full_batch_and_executes_only_reuse_phase() {
    let (source, _) = recipes();
    let saved = assemble(source, None);
    let result = derive(&saved).unwrap();
    check_output(&saved, &result.plan);
    assert_eq!(result.plan.prep_sessions[1].tasks[0].prep_id, 2);
    assert_eq!(result.plan.recipe_refs.len(), 1);
    assert_eq!(result.plan.allocations[1].remix_recipe, Nullable(None));
    // Separate single-consumption output-validator control: this does not assert
    // a prep-only source occurrence in WeekPlan, or admit externally made stock.
    let mut batch = result.plan.batches[0].clone();
    batch.components[0].allocated = units(1);
    batch.components[0].unallocated.amount = Rational::new(3, 1).unwrap();
    batches::validate_allocations(&[batch], &[result.plan.allocations[1].clone()]).unwrap();
}
#[test]
fn portable_reuse_only_recipe_retains_home_appliances_and_original_clock_without_new_ingredients() {
    let (source, cold) = recipes();
    let saved = assemble(source, Some(cold));
    let result = derive(&saved).unwrap();
    check_output(&saved, &result.plan);
    assert_eq!(
        saved.context.recipes[0].equipment.0,
        vec![
            EquipmentCode::Fridge,
            EquipmentCode::Hob,
            EquipmentCode::Oven
        ]
    );
    assert!(
        saved.context.recipes[1].equipment.is_empty()
            && saved.context.recipes[1].ingredients.is_empty()
    );
    let service = saved
        .context
        .requirements
        .iter()
        .find(|m| m.slot == saved.plan.selections[1].slot)
        .unwrap();
    assert_eq!(service.location, MealLocation::OnTheMove);
    assert!(service.equipment.is_empty());
    assert!(service.mobility.cold_storage);
    assert_eq!(service.mobility.temperature, MobilityTemperature::Cold);
    assert_eq!(
        result.plan.allocations[1].remix_recipe.0.as_ref(),
        Some(&reference(&saved.context.recipes[1]))
    );
    assert_eq!(
        saved.context.recipes[1].reused_inputs[0].source_recipe,
        reference(&saved.context.recipes[0])
    );
}
#[test]
fn portable_equipment_and_cold_storage_suitability_guards_remain_in_force() {
    let (source, cold) = recipes();
    let saved = assemble(source.clone(), Some(cold));
    let service = saved
        .context
        .requirements
        .iter()
        .find(|m| m.slot == saved.plan.selections[1].slot)
        .unwrap();
    let (basis, classes) = premise_basis(&source);
    let reviewed = assess_revision(&source, &basis, &classes).unwrap();
    assert!(
        matches!(
            eligible(&reviewed, service).unwrap(),
            Eligibility::Ineligible(_)
        ),
        "Removing destination appliances cannot erase source recipe requirements"
    );
    let mut with_equipment = service.clone();
    with_equipment.equipment = source.equipment.clone();
    assert!(
        matches!(
            eligible(&reviewed, &with_equipment).unwrap(),
            Eligibility::Unresolved(_)
        ),
        "OnTheMove with source appliances remains unsupported"
    );
    let mut no_storage = saved.clone();
    no_storage
        .context
        .requirements
        .iter_mut()
        .find(|m| m.slot == saved.plan.selections[1].slot)
        .unwrap()
        .mobility
        .cold_storage = false;
    let failure = derive(&no_storage);
    assert!(
        matches!(&failure,Err(FoodFailure::Blocked{conflicts,..}) if conflicts.iter().any(|c|
        c.code==ErrorCode::UnsupportedCapability && c.slot_ids.contains(&service.slot)
        && c.recipe_refs.contains(&reference(&saved.context.recipes[1])) && c.evidence_ids.contains(&eid(5)))),
        "{failure:?}"
    );
    let mut unsupported = saved.context.recipes[1].clone();
    unsupported.portability.cold_suitable = false;
    let (basis, classes) = premise_basis(&unsupported);
    let reviewed = assess_revision(&unsupported, &basis, &classes).unwrap();
    assert!(matches!(
        eligible(&reviewed, service).unwrap(),
        Eligibility::Ineligible(_)
    ));
}
#[test]
fn original_24_hour_limit_is_inclusive_and_not_reset_by_cold_handling() {
    let (source, cold) = recipes();
    let mut saved = assemble(source, Some(cold));
    for time in ["18:00:00", "18:00:01"] {
        saved.plan.selections[1].meal_overrides.eat_by = Nullable(Some(at(20, time)));
        saved.context.weeks[0].plan = saved.plan.clone();
        saved
            .context
            .requirements
            .iter_mut()
            .find(|m| m.slot == saved.plan.selections[1].slot)
            .unwrap()
            .eat_by = saved.plan.selections[1].meal_overrides.eat_by.clone();
        saved.schedule.layout.fixed_starts[1].start = shifted(
            saved.plan.selections[1]
                .meal_overrides
                .eat_by
                .0
                .as_ref()
                .unwrap(),
            -300,
        );
        exact_fixed_occupancy(&mut saved);
        if time == "18:00:00" {
            check_output(&saved, &derive(&saved).unwrap().plan);
        } else {
            let before = saved.clone();
            let failure = derive(&saved);
            assert!(
                matches!(&failure,Err(FoodFailure::Blocked{conflicts,..}) if conflicts.iter().any(|c|c.code==ErrorCode::DomainConflict)),
                "{failure:?}"
            );
            let reason = batches::check_reuse(
                &at(19, "18:00:00"),
                &at(20, time),
                &saved.context.recipes[0].storage[0],
                172800,
                0,
            )
            .unwrap_err();
            assert_eq!(reason.code, ErrorCode::DomainConflict);
            assert!(reason.issues.iter().any(|issue| issue.path == "/use_at"));
            assert_eq!(saved, before);
        }
    }
}
#[test]
fn missing_or_changed_full_original_recipe_reference_cannot_supply_the_portable_use() {
    let (source, cold) = recipes();
    let mut saved = assemble(source, Some(cold));
    let source_ref = reference(&saved.context.recipes[0]);
    let mut missing = saved.clone();
    missing.plan.selections.0.remove(0);
    missing.plan.batch_links[0].source_slot = saved.plan.selections[0].slot.clone();
    missing.context.weeks[0].plan = missing.plan.clone();
    missing.evidence.recipes.retain(|r| r.recipe != source_ref);
    missing.evidence.classes.retain(|r| r.recipe != source_ref);
    missing.evidence.methods.retain(|r| r.recipe != source_ref);
    missing
        .evidence
        .resources
        .retain(|r| r.recipe != source_ref);
    missing.evidence.catalogue_digest = food_evidence_digest(&missing.evidence).unwrap();
    assert!(
        matches!(derive(&missing),Err(FoodFailure::Invalid(e)) if e.issues.iter().any(|issue|issue.path=="/source_batch"))
    );
    saved.context.recipes[0].revision = Revision(2);
    let changed = saved.context.recipes[0].clone();
    let cold = saved.context.recipes[1].clone();
    // Keep the old exact input pin deliberately. Current context refuses that
    // absent full reference before expansion; do not refresh or bypass this
    // original-source closure merely to reach a later remix check.
    let old_input = cold.reused_inputs[0].source_recipe.clone();
    let mut case = assemble_without_keys(changed, cold);
    assert_ne!(old_input, reference(&case.context.recipes[0]));
    let result = required_identity_keys(&case.plan, &case.context, &case.evidence, &case.schedule);
    assert!(
        matches!(&result,Err(FoodFailure::Invalid(e)) if e.code==ErrorCode::MissingReference && e.issues.iter().any(|issue|issue.path=="/recipe_refs")),
        "{result:?}"
    );
    case.plan.selections[1].recipe.digest = RecipeDigest("a".repeat(64));
    assert!(derive(&case).is_err());
}
#[test]
fn post_service_cooling_is_refused_at_the_existing_phase_deadline() {
    let (source, _) = recipes();
    let mut saved = assemble(source, None);
    let mut source = saved.context.recipes[0].clone();
    let mut cooling = source.prep[1].clone();
    cooling.prep_id = 3;
    cooling.title = Title(
        "SYNTHETIC post-dinner cooling-container handling; deliberately not expressible".into(),
    );
    cooling.step_ids = BoundedVec(vec![3]);
    cooling.depends_on = BoundedVec(vec![1]);
    cooling.active_minutes = 5;
    cooling.elapsed_minutes = 5;
    cooling.earliest_offset_seconds = 0;
    cooling.latest_offset_seconds = 0;
    let mut step = source.steps[1].clone();
    step.step_id = 3;
    step.active_minutes = 5;
    step.elapsed_minutes = 5;
    step.depends_on = BoundedVec(vec![1]);
    source.steps.0.push(step);
    source.prep.0.push(cooling);
    source.active_minutes += 5;
    source.elapsed_minutes += 5;
    refresh_claims(&mut source);
    let rf = reference(&source);
    saved.context.recipes[0] = source;
    saved.context.stamp.recipe_refs = BoundedVec(vec![rf.clone()]);
    for selection in saved.plan.selections.iter_mut() {
        selection.recipe = rf.clone();
        selection.servings[0].preset_ref.recipe = rf.clone();
    }
    saved.context.weeks[0].plan = saved.plan.clone();
    let (basis, classes) = premise_basis(&saved.context.recipes[0]);
    saved.evidence.recipes[0] = basis;
    saved.evidence.classes[0] = classes;
    for method in &mut saved.evidence.methods {
        method.recipe = rf.clone();
        for count in &mut method.direct_reuse_counts {
            count.recipe = rf.clone();
        }
        if method.kind == MethodKind::Production {
            method.prep_ids.push(3);
        }
    }
    for resource in &mut saved.evidence.resources {
        resource.recipe = rf.clone();
    }
    saved.evidence.resources.push(PhaseResources {
        recipe: rf.clone(),
        prep_id: 3,
        equipment: vec![EquipmentCode::Fridge],
        evidence_ids: vec![eid(3)],
    });
    saved.evidence.catalogue_digest = food_evidence_digest(&saved.evidence).unwrap();
    for route in &mut saved.schedule.layout.routes {
        route.phase.occurrence.recipe = rf.clone();
    }
    for fixed in &mut saved.schedule.layout.fixed_starts {
        fixed.phase.occurrence.recipe = rf.clone();
    }
    let phase = PhaseKey {
        occurrence: OccurrenceKey {
            slot: saved.plan.selections[0].slot.clone(),
            extra_id: None,
            recipe: rf,
        },
        prep_id: 3,
    };
    saved.schedule.layout.routes.push(PhaseRoute {
        phase: phase.clone(),
        session: saved.schedule.layout.routes[0].session.clone(),
    });
    saved
        .schedule
        .layout
        .fixed_starts
        .push(PhaseStartConstraint {
            phase,
            start: at(19, "18:00:00"),
        });
    saved
        .schedule
        .layout
        .routes
        .sort_by(|a, b| phase_cmp(&a.phase, &b.phase));
    saved
        .schedule
        .layout
        .fixed_starts
        .sort_by(|a, b| phase_cmp(&a.phase, &b.phase));
    let result = required_identity_keys(
        &saved.plan,
        &saved.context,
        &saved.evidence,
        &saved.schedule,
    );
    assert!(
        matches!(&result,Err(FoodFailure::Invalid(e)) if e.issues.iter().any(|issue|issue.path=="/prep/window")),
        "Every ordinary phase is still required to finish before its meal: {result:?}"
    );
}
#[test]
fn zero_fresh_and_zero_reused_input_remains_invalid() {
    let (_, mut cold) = recipes();
    cold.reused_inputs = BoundedVec(vec![]);
    assert!(parse_document::<RecipeRevision>(&canonical_bytes(&cold).unwrap()).is_err());
}
