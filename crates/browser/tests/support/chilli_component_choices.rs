//! Synthetic assembly mechanics only; these definitions confer no food admission.
use super::*;
#[allow(dead_code)]
#[path = "../../../../tests/fixtures/food/batches/f11_fixture.rs"]
mod f11;

fn reference(recipe: &RecipeRevision) -> RecipeRef {
    RecipeRef {
        recipe_id: recipe.recipe_id.clone(),
        revision: recipe.revision,
        digest: recipe_digest(recipe).unwrap(),
    }
}
fn fixture() -> (RecipeRevision, RecipeRevision, Week) {
    let saved = f11::fixture();
    let mut source = saved.context.recipes[0].clone();
    source.recipe_id = RecipeId(crate::personal_scope::CHILLI_BATCH_SCOPE.recipe_id.into());
    let old_component = source.components[0].component_id.0.clone();
    source = serde_json::from_str(
        &serde_json::to_string(&source)
            .unwrap()
            .replace(&old_component, crate::personal_scope::CHILLI_COMPONENT_ID),
    )
    .unwrap();
    source.r#yield.components[0].quantity = Quantity {
        amount: PositiveRational::new(4, 1).unwrap(),
        unit: Unit::Each,
        form: IngredientForm::Cooked,
    };
    let mut wrap = saved.context.recipes[1].clone();
    wrap.recipe_id = RecipeId(crate::personal_scope::CHILLI_WRAPS_SCOPE.recipe_id.into());
    wrap.ingredients[0].quantity = Nullable(Some(Quantity {
        amount: PositiveRational::new(2, 1).unwrap(),
        unit: Unit::Each,
        form: IngredientForm::Prepared,
    }));
    wrap.reused_inputs[0].source_recipe = reference(&source);
    wrap.reused_inputs[0].source_component_id =
        ComponentId(crate::personal_scope::CHILLI_COMPONENT_ID.into());
    wrap.reused_inputs[0].quantity_per_recipe_yield = Quantity {
        amount: PositiveRational::new(1, 1).unwrap(),
        unit: Unit::Each,
        form: IngredientForm::Cooked,
    };
    let mut week = saved.context.weeks[0].clone();
    week.plan = saved.plan;
    week.plan.selections[0].recipe = reference(&source);
    week.plan.selections[1].recipe = reference(&wrap);
    (source, wrap, week)
}

#[test]
fn fresh_remix_uses_only_the_explicit_full_original_new_batch() {
    let (_, wrap, mut week) = fixture();
    let rf = reference(&wrap);
    let target = week.plan.selections[1].slot.clone();
    let options = ChoiceOptions {
        leftover_source: Some(week.plan.selections[0].slot.clone()),
        ..Default::default()
    };
    assert!(
        original_cold_input(&wrap).is_err(),
        "old cold route stays narrow"
    );
    assert!(choice_source(&wrap, &rf, &week, &target, &options)
        .unwrap()
        .is_some());
    assert!(
        choice_source(&wrap, &rf, &week, &target, &Default::default())
            .unwrap()
            .is_none()
    );
    let exact = week.plan.selections[0].recipe.clone();
    week.plan.selections[0].recipe.digest = RecipeDigest("a".repeat(64));
    assert!(choice_source(&wrap, &rf, &week, &target, &options)
        .unwrap()
        .is_none());
    week.plan.selections[0].recipe = exact.clone();
    week.plan.selections[0].recipe.revision = Revision(exact.revision.0 + 1);
    assert!(choice_source(&wrap, &rf, &week, &target, &options)
        .unwrap()
        .is_none());
    week.plan.selections[0].recipe = exact;
    week.plan.selections[0].batch_use = week.plan.selections[1].batch_use.clone();
    assert!(choice_source(&wrap, &rf, &week, &target, &options)
        .unwrap()
        .is_none());
}

#[test]
fn fresh_remix_input_is_one_original_share_with_one_reheat() {
    let (_, wrap, _) = fixture();
    assert!(original_service_input(&wrap).is_ok());
    assert_eq!(
        choice_component_ids(&wrap).unwrap()[0].0,
        crate::personal_scope::CHILLI_COMPONENT_ID
    );
    assert_ne!(
        choice_component_ids(&wrap).unwrap()[0],
        wrap.components[0].component_id
    );
    let mut bad = wrap.clone();
    bad.ingredients.0.clear();
    assert!(original_service_input(&bad).is_err());
    let mut bad = wrap.clone();
    bad.reused_inputs[0].planned_reheats = 0;
    assert!(original_service_input(&bad).is_err());
    let mut bad = wrap.clone();
    bad.reused_inputs[0].planned_reheats = 2;
    assert!(original_service_input(&bad).is_err());
    let mut bad = wrap.clone();
    bad.reused_inputs[0].quantity_per_recipe_yield.unit = Unit::G;
    assert!(original_service_input(&bad).is_err());
    let mut bad = wrap.clone();
    bad.reused_inputs[0].source_component_id = wrap.components[0].component_id.clone();
    assert!(original_service_input(&bad).is_err());
    let mut bad = wrap.clone();
    bad.ingredients[0].quantity.0.as_mut().unwrap().amount = PositiveRational::new(1, 1).unwrap();
    assert!(original_service_input(&bad).is_err());
}

#[test]
fn chilli_keeps_thermal_cooking_and_reserved_share_cooling_separate() {
    let (mut source, mut wrap, _) = fixture();
    source.components[0].production = Nullable(Some(ComponentProductionSpec::CookedWithCooling {
        completion_prep_id: 3,
        cooling_prep_ids: BoundedVec(vec![4, 5, 6]),
        max_cooling_seconds: 7200,
    }));
    let mut prep = source.prep[0].clone();
    prep.latest_offset_seconds = -5400;
    for id in 1..=6 {
        prep.prep_id = id;
        assert_eq!(
            phase_session_role(&source, &reference(&source), &prep).unwrap(),
            if id <= 3 {
                SessionRole::Cooking
            } else {
                SessionRole::AdvancePrep
            }
        );
    }
    wrap.recipe_id = RecipeId(crate::personal_scope::CHILLI_POTATO_SCOPE.recipe_id.into());
    for id in 1..=5 {
        prep.prep_id = id;
        assert_eq!(
            phase_session_role(&wrap, &reference(&wrap), &prep).unwrap(),
            SessionRole::Cooking
        );
    }
    source.components[0].production = Nullable(Some(ComponentProductionSpec::Cooked {
        completion_prep_id: 3,
    }));
    assert!(phase_session_role(&source, &reference(&source), &prep).is_err());
}
