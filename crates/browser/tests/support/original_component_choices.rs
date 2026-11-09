//! Synthetic browser assembly mechanics only. No new packet admission, physical
//! cooking/cooling observation or real-pair/browser acceptance is claimed here.
use super::*;

#[allow(dead_code)]
#[path = "../../../core/tests/support/original_component.rs"]
mod fixture;

fn pair() -> (RecipeRevision, RecipeRevision) {
    let (mut source, mut cold) = fixture::recipes();
    source.recipe_id = RecipeId(HOME_ORIGINAL_SOURCE_ID.into());
    fixture::refresh_claims(&mut source);
    cold.recipe_id = RecipeId(HOME_ORIGINAL_COLD_ID.into());
    cold.reused_inputs[0].source_recipe = fixture::reference(&source);
    cold.scaling = ScalingPolicy::Linear {
        minimum: fixture::p(1), maximum: fixture::p(3),
    };
    fixture::refresh_claims(&mut cold);
    (source, cold)
}

fn selected_week(source: &RecipeRevision, cold: &RecipeRevision) -> Week {
    let mut saved = fixture::assemble(source.clone(), Some(cold.clone()));
    let mut week = saved.context.weeks.0.remove(0);
    for cell in week.cells.iter_mut().filter(|cell| cell.choice == MealLocation::OnTheMove) {
        cell.choice = MealLocation::AtHome;
    }
    week
}

#[test]
fn cold_source_requires_explicit_full_reference_original_new_batch() {
    let (source, cold) = pair();
    let mut week = selected_week(&source, &cold);
    let target = week.plan.selections[1].slot.clone();
    let reference = fixture::reference(&cold);
    let options = ChoiceOptions {
        leftover_source: Some(week.plan.selections[0].slot.clone()), ..Default::default()
    };
    assert_eq!(choice_source(&cold, &reference, &week, &target, &options).unwrap(), Some(&week.plan.selections[0]));
    assert!(choice_source(&cold, &reference, &week, &target, &ChoiceOptions::default()).unwrap().is_none());
    assert!(choice_source(&cold, &reference, &week, options.leftover_source.as_ref().unwrap(), &options).unwrap().is_none());
    let exact = week.plan.selections[0].recipe.clone();
    week.plan.selections[0].recipe.revision = Revision(exact.revision.0 + 1);
    assert!(choice_source(&cold, &reference, &week, &target, &options).unwrap().is_none());
    week.plan.selections[0].recipe = exact.clone();
    week.plan.selections[0].recipe.digest = RecipeDigest("a".repeat(64));
    assert!(choice_source(&cold, &reference, &week, &target, &options).unwrap().is_none());
    week.plan.selections[0].recipe = exact;
    week.plan.selections[0].batch_use = week.plan.selections[1].batch_use.clone();
    assert!(choice_source(&cold, &reference, &week, &target, &options).unwrap().is_none(), "a reused meal cannot become the original source");
}

#[test]
fn cold_choice_rejects_new_production_extras_and_extra_source() {
    let (source, cold) = pair();
    let mut week = selected_week(&source, &cold);
    let target = week.plan.selections[1].slot.clone();
    let reference = fixture::reference(&cold);
    let mut options = ChoiceOptions {
        leftover_source: Some(week.plan.selections[0].slot.clone()), make_extra: true,
        ..Default::default()
    };
    assert!(choice_source(&cold, &reference, &week, &target, &options).unwrap().is_none());
    options.make_extra = false;
    options.extra_recipients.push(week.plan.selections[0].servings[0].person.clone());
    assert!(choice_source(&cold, &reference, &week, &target, &options).unwrap().is_none());
    options.extra_recipients.clear();
    let recipient = week.plan.selections[0].servings[0].clone();
    week.plan.selections[0].extras.0.push(ExtraSelection {
        extra_id: ExtraId("ex_00000000000000000000000000000001".into()),
        recipe: reference.clone(), recipients: BoundedVec(vec![recipient]),
    });
    assert!(choice_source(&cold, &reference, &week, &target, &options).unwrap().is_none());
}

#[test]
fn cold_definition_rejects_fresh_missing_multiple_wrong_and_reheated_inputs() {
    let (source, cold) = pair();
    assert!(original_cold_input(&cold).is_ok());
    let mut bad = cold.clone();
    bad.ingredients.0.push(source.ingredients[0].clone());
    assert!(original_cold_input(&bad).is_err());
    let mut bad = cold.clone(); bad.reused_inputs.0.clear();
    assert!(original_cold_input(&bad).is_err());
    let mut bad = cold.clone(); bad.reused_inputs.0.push(cold.reused_inputs[0].clone());
    assert!(original_cold_input(&bad).is_err());
    let mut bad = cold.clone(); bad.reused_inputs[0].source_recipe = fixture::reference(&cold);
    assert!(original_cold_input(&bad).is_err());
    let mut bad = cold; bad.reused_inputs[0].planned_reheats = 1;
    assert!(original_cold_input(&bad).is_err());
}

#[test]
fn cold_links_use_original_component_and_exact_native_scaled_input() {
    let (_, mut cold) = pair();
    // Deliberately differ from the target output so an output-component copy
    // cannot accidentally pass. Input units/forms must stay original as well.
    cold.reused_inputs[0].quantity_per_recipe_yield.amount = PositiveRational::new(3, 2).unwrap();
    fixture::refresh_claims(&mut cold);
    let (basis, classes) = fixture::premise_basis(&cold);
    let reviewed = assess_revision(&cold, &basis, &classes).unwrap();
    let scaled = mstrchf_core::portions::scale_recipe(&reviewed, fixture::p(2)).unwrap();
    assert_eq!(scaled.multiplier, fixture::p(2));
    assert!(scaled.ingredients.is_empty());
    assert_ne!(scaled.components[0].component_id, cold.reused_inputs[0].source_component_id);
    assert_eq!(choice_component_ids(&cold).unwrap().0, vec![cold.reused_inputs[0].source_component_id.clone()]);
    assert_eq!(choice_link_components(&cold, &scaled).unwrap().0, vec![ComponentAllocation {
        component_id: cold.reused_inputs[0].source_component_id.clone(),
        quantity: fixture::units(3),
    }]);
}

#[test]
fn original_cooking_at_t_minus_62_and_named_cooling_have_separate_roles() {
    let mut source = fixture::cooling_recipe();
    source.recipe_id = RecipeId(HOME_ORIGINAL_SOURCE_ID.into());
    source.prep[0].latest_offset_seconds = -3720;
    let reference = fixture::reference(&source);
    assert_eq!(phase_session_role(&source, &reference, &source.prep[0]).unwrap(), SessionRole::Cooking);
    for prep in source.prep.iter().filter(|prep| [3, 4].contains(&prep.prep_id)) {
        assert_eq!(phase_session_role(&source, &reference, prep).unwrap(), SessionRole::AdvancePrep);
    }
    source.components[0].production = Nullable(Some(ComponentProductionSpec::Cooked { completion_prep_id: 1 }));
    assert!(phase_session_role(&source, &reference, &source.prep[0]).is_err());
}

#[test]
fn legacy_source_components_quantities_and_role_threshold_are_unchanged() {
    let (source, cold) = fixture::recipes();
    let week = selected_week(&source, &cold);
    let reference = fixture::reference(&source);
    let options = ChoiceOptions {
        leftover_source: Some(week.plan.selections[0].slot.clone()), ..Default::default()
    };
    assert_eq!(choice_source(&source, &reference, &week, &week.plan.selections[1].slot, &options).unwrap(), Some(&week.plan.selections[0]));
    assert_eq!(choice_component_ids(&source).unwrap().0, source.components.iter().map(|component|component.component_id.clone()).collect::<Vec<_>>());
    let (basis, classes) = fixture::premise_basis(&source);
    let reviewed = assess_revision(&source, &basis, &classes).unwrap();
    let scaled = mstrchf_core::portions::scale_recipe(&reviewed, fixture::p(1)).unwrap();
    assert_eq!(choice_link_components(&source, &scaled).unwrap().0, scaled.components.iter().map(|component|ComponentAllocation {
        component_id: component.component_id.clone(), quantity: component.quantity.clone(),
    }).collect::<Vec<_>>());
    let mut prep = source.prep[0].clone();
    prep.latest_offset_seconds = -3600;
    assert_eq!(phase_session_role(&source, &reference, &prep).unwrap(), SessionRole::Cooking);
    prep.latest_offset_seconds = -3601;
    assert_eq!(phase_session_role(&source, &reference, &prep).unwrap(), SessionRole::AdvancePrep);
}
