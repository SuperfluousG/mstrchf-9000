//! Synthetic admitted premises only; no real recipe or source-safety claim.
use mstrchf_contracts::*;
use mstrchf_core::{eligibility::*, food::*, recipes::*};
mod food_extra_projection_support;
use food_extra_projection_support::*;

fn vegetarian() -> HardRequirement {
    HardRequirement::Diet {
        code: DietCode::Vegetarian,
    }
}

fn extra_eligibility(s: &SavedFoodBasis) -> Eligibility {
    let selection = &s.plan.selections[0];
    let review = |reference: &RecipeRef| {
        assess_revision(
            s.context
                .recipes
                .iter()
                .find(|r| r.recipe_id == reference.recipe_id)
                .unwrap(),
            s.evidence
                .recipes
                .iter()
                .find(|r| r.recipe == *reference)
                .unwrap(),
            s.evidence
                .classes
                .iter()
                .find(|r| r.recipe == *reference)
                .unwrap(),
        )
        .unwrap()
    };
    let base = review(&selection.recipe);
    let extra = review(&selection.extras[0].recipe);
    let meal = s
        .context
        .requirements
        .iter()
        .find(|m| m.slot == selection.slot)
        .unwrap();
    validate_extra(
        &base,
        &extra,
        &selection.extras[0],
        meal,
        &selection.meal_overrides,
    )
    .unwrap()
}

#[test]
fn base_only_restriction_does_not_block_unrestricted_extra_recipient() {
    let v = extra_value();
    let s = rebuild_inputs(v.clone());
    assert_eq!(extra_eligibility(&s), Eligibility::Eligible);
    let meal = s
        .context
        .requirements
        .iter()
        .find(|m| m.slot == s.plan.selections[0].slot)
        .unwrap();
    assert_eq!(meal.attendees.len(), 2);
    assert_eq!(meal.shared_hard_requirements.0, vec![vegetarian()]);
    assert!(meal.attendees[1].hard_requirements.is_empty());
    let (s, d) = rebuild(v);
    // Three base servings at 200 g and one B-only extra serving at 100 g.
    assert_eq!(d.demand.lines.len(), 2);
    let extra_line = d
        .demand
        .lines
        .iter()
        .find(|l| !l.origin_extras.is_empty())
        .unwrap();
    assert_eq!(
        extra_line.quantity.amount,
        PositiveRational::new(100, 1).unwrap()
    );
    assert_eq!(
        extra_line.origin_extras.0,
        vec![s.plan.selections[0].extras[0].extra_id.clone()]
    );
    let base_line = d
        .demand
        .lines
        .iter()
        .find(|l| l.origin_extras.is_empty())
        .unwrap();
    assert_eq!(
        base_line.quantity.amount,
        PositiveRational::new(600, 1).unwrap()
    );
    assert_eq!(s.plan.selections[0].extras[0].recipients.len(), 1);
    assert_eq!(
        s.plan.selections[0].extras[0].recipients[0].servings,
        PositiveRational::new(1, 1).unwrap()
    );
}

#[test]
fn actual_extra_recipient_restriction_still_blocks_missing_evidence() {
    let mut v = extra_value();
    v["context"]["household"]["diners"][1]["restrictions"] =
        v["context"]["household"]["diners"][0]["restrictions"].clone();
    let s = rebuild_inputs(v);
    assert!(matches!(extra_eligibility(&s), Eligibility::Unresolved(_)));
    assert!(matches!(
        required_identity_keys(&s.plan, &s.context, &s.evidence, &s.schedule),
        Err(FoodFailure::Blocked { .. })
    ));
}

#[test]
fn extra_spec_requirement_still_blocks_missing_evidence() {
    let mut v = extra_value();
    v["context"]["recipes"][0]["extras"][0]["eligible_person_requirements"] =
        serde_json::json!([HardRequirement::Diet {
            code: DietCode::GlutenFree
        }]);
    let s = rebuild_inputs(v);
    assert!(matches!(extra_eligibility(&s), Eligibility::Unresolved(_)));
    assert!(matches!(
        required_identity_keys(&s.plan, &s.context, &s.evidence, &s.schedule),
        Err(FoodFailure::Blocked { .. })
    ));
}

#[test]
fn shared_recipient_and_extra_spec_requirement_is_not_lost_or_duplicated() {
    let mut v = extra_value();
    v["context"]["household"]["diners"][1]["restrictions"] =
        v["context"]["household"]["diners"][0]["restrictions"].clone();
    v["context"]["recipes"][1]["dietary_evidence"] =
        v["context"]["recipes"][0]["dietary_evidence"].clone();
    v["context"]["recipes"][0]["extras"][0]["eligible_person_requirements"] =
        serde_json::json!([vegetarian()]);
    let mut a = v["plan"]["selections"][0]["servings"][0].clone();
    a["preset_ref"]["recipe"] = v["plan"]["selections"][0]["extras"][0]["recipe"].clone();
    v["plan"]["selections"][0]["extras"][0]["recipients"]
        .as_array_mut()
        .unwrap()
        .insert(0, a);
    let (s, d) = rebuild(v);
    assert_eq!(extra_eligibility(&s), Eligibility::Eligible);
    assert_eq!(s.plan.selections[0].extras[0].recipients.len(), 2);
    assert_eq!(
        d.demand
            .lines
            .iter()
            .find(|l| !l.origin_extras.is_empty())
            .unwrap()
            .quantity
            .amount,
        PositiveRational::new(200, 1).unwrap()
    );
}
