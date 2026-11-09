//! Separate conditional Vegetarian/gluten-free base and named chicken extra.
//! Scope checks confer no admission, product suitability or observed handling.
use super::*;

// Exact Root-reviewed recipe and packet pins; household admission remains separate.
pub const THREE_BEAN_GF_CHICKEN_SCOPE: PersonalUseScope = PersonalUseScope {
    scope_id: "three-bean-two-unit-vegetarian-gluten-free-optional-chicken-home-lunch-v1",
    recipe_id: "rc_8000000000000000000000000000000f",
    recipe_revision: 1,
    recipe_digest: "6c2a9a021aca72f639d7c5e02deeb44dd7e6ce30565ed4eb687c13a15cbb6fde",
    meal_type: MealType::Lunch,
    source_url: THREE_BEAN_PERSONAL_SCOPE.source_url,
    packet_canonical_digest: "2923a6f342f1b344b9a99e4e7e1387e671a410e405aa7c78d9b168cf1e397a4f",
    attention_reservation_minutes: 30,
    timing_is_estimate: true,
    portion_convention: "Two separate conditional Vegetarian and gluten-free input Adult Regular units; optional separate fixed 50 g chicken only for one named None or gluten-free adult. No actual product, nutrition or human calibration claim.",
};

pub const CHILLED_GF_CHICKEN_SCOPE: PersonalUseScope = PersonalUseScope {
    scope_id: "chilled-gluten-free-chicken-named-none-or-gf-adult-extra-only-v1",
    recipe_id: "rc_80000000000000000000000000000010",
    recipe_revision: 1,
    recipe_digest: "abf7af80931c2d9d85fcec67577c67718ab942c52e7d214805c152bd33173565",
    meal_type: MealType::Lunch,
    source_url: "",
    packet_canonical_digest: "0853be3839bac138de32430237cc0ee6e3b33ba9c2934cb9596ee4995a99bf47",
    attention_reservation_minutes: 5,
    timing_is_estimate: true,
    portion_convention: "One whole fixed 50 g conditional gluten-free cooked chicken extra for one named None or gluten-free Adult Regular recipient; never a standalone meal, measured yield or calibrated portion.",
};

pub const GF_CHICKEN_EXTRA_ID: &str = "ex_80000000000000000000000000000003";

pub(super) fn base_requirements(requirements: &[HardRequirement]) -> bool {
    requirements.len() <= 2
        && requirements.iter().all(|requirement| {
            matches!(
                requirement,
                HardRequirement::Diet {
                    code: DietCode::Vegetarian | DietCode::GlutenFree
                }
            )
        })
        && (requirements.len() != 2 || requirements[0] != requirements[1])
}

fn recipient_requirements(requirements: &[HardRequirement]) -> bool {
    requirements.is_empty()
        || matches!(
            requirements,
            [HardRequirement::Diet {
                code: DietCode::GlutenFree
            }]
        )
}

pub(super) fn notices(scope: &PersonalUseScope) -> Vec<String> {
    let extra = scope.recipe_id == CHILLED_GF_CHICKEN_SCOPE.recipe_id;
    vec![
        scope.portion_convention.into(),
        if extra {
            "Use only as the exact paired base's independently admitted extra for one explicitly named current Adult Regular ×1 person with confirmed None or exactly gluten-free requirements. Vegetarian, Vegan, other/additional requirements and standalone use are refused.".into()
        } else {
            "Exactly two confirmed Adult Regular ×1 attendees, at least one resident and at most one temporary guest, at-home Lunch with Fridge. Each person may have None, exactly Vegetarian, exactly gluten-free, or both only. All intrinsic input conditions apply even to None consumers.".into()
        },
        if extra {
            "Reserve five minutes of full attention at T−5 for separate chicken handling. Establish actual cooked cold-ready-to-eat product suitability, labels, dates and proper prior chilling; keep separate utensils and plates, protecting both base portions from chicken and gluten cross-contact. These conditions are not observations.".into()
        } else {
            "Independently admit the exact chicken definition first, even when no extra is selected. Reserve 30 minutes of attention from T−90 and at least 60 minutes of passive chilling from T−60, on the same local day. Prepare two separately measured covered base portions.".into()
        },
        "Recipe conditions do not qualify an actual pack, manufacturer processing, allergen status, cross-contact or handling. If required suitability or handling conditions cannot be established, do not use the affected food. No other allergen/exclusion claim, automatic scaling, compensation, make-extra, travel, storage life or reuse is supplied.".into(),
        "Changed or departed people retain exact saved food and history as a conflict until explicit removal or eligible replacement. Never redirect the extra. Timing is an unmeasured reservation; no observed temperature, completion, nutrition, CookTest or portion calibration claim.".into(),
    ]
}

pub(crate) fn check_gf_chicken_pair_allocation(
    selection: &MealSelection,
    options: &BoardOptions,
) -> Result<(), DomainError> {
    if !pinned(&selection.recipe, &THREE_BEAN_GF_CHICKEN_SCOPE) || selection.extras.len() > 1 {
        return Err(unsupported("one_declared_gf_chicken_extra"));
    }
    let requested = options.extra_recipients.as_ref();
    if requested.is_some_and(|people| people.len() > 1) {
        return Err(unsupported("one_named_extra_recipient"));
    }
    if let Some(extra) = selection.extras.first() {
        if extra.extra_id.0 != GF_CHICKEN_EXTRA_ID
            || !pinned(&extra.recipe, &CHILLED_GF_CHICKEN_SCOPE)
            || extra.recipients.len() != 1
        {
            return Err(unsupported("one_declared_gf_chicken_extra"));
        }
        let serving = &extra.recipients[0];
        if serving.preset_ref.recipe != extra.recipe
            || serving.preset_ref.age_band != AgeBand::Adult
            || serving.preset_ref.meal_type != MealType::Lunch
            || serving.preset_ref.preset != PortionPresetName::Regular
            || !one(serving.servings)
            || !selection
                .servings
                .iter()
                .any(|base| base.person == serving.person)
        {
            return Err(refusal(
                ErrorCode::IdentityMismatch,
                "extra_serving_identity",
            ));
        }
        if requested.is_some_and(|people| people.0 != vec![serving.person.clone()]) {
            return Err(refusal(
                ErrorCode::IdentityMismatch,
                "request_extra_recipient",
            ));
        }
    } else if requested.is_some_and(|people| !people.is_empty()) {
        return Err(unsupported("requested_extra_not_selected"));
    }
    Ok(())
}

pub(crate) fn check_gf_chicken_extra_scope(
    recipe: &RecipeRevision,
    meal: &MealRequirements,
    selection: &MealSelection,
    plan: &WeekPlan,
    options: &BoardOptions,
) -> Result<(), DomainError> {
    check_gf_chicken_pair_allocation(selection, options)?;
    let Some(extra) = selection.extras.first() else {
        return Ok(());
    };
    if !pinned(&extra.recipe, &CHILLED_GF_CHICKEN_SCOPE)
        || recipe.recipe_id != extra.recipe.recipe_id
        || recipe.revision != extra.recipe.revision
        || recipe_digest(recipe)? != extra.recipe.digest
    {
        return Err(refusal(
            ErrorCode::IdentityMismatch,
            "extra_recipe_reference",
        ));
    }
    if recipe.source.origin != RecipeSourceOrigin::UserAuthored
        || recipe.source.url.0.is_some()
        || recipe.text_rights.licence_url.0.is_some()
    {
        return Err(refusal(
            ErrorCode::IdentityMismatch,
            "extra_original_source",
        ));
    }
    if !meal.enabled
        || meal.location != MealLocation::AtHome
        || meal.slot.meal_type != MealType::Lunch
        || !meal.equipment.contains(&EquipmentCode::Fridge)
        || selection.slot != meal.slot
        || plan.week_id != meal.slot.week_id
        || selection.portable_cold.is_some()
        || options.portable_cold.is_some()
    {
        return Err(unsupported("at_home_chicken_extra"));
    }
    let person = &extra.recipients[0].person;
    let attendee = meal
        .attendees
        .iter()
        .find(|attendee| &attendee.person == person)
        .ok_or_else(|| refusal(ErrorCode::ForeignReference, "extra_current_recipient"))?;
    if attendee.age_band != AgeBand::Adult
        || !recipient_requirements(&attendee.hard_requirements.0)
        || attendee.portion.preset != PortionPresetName::Regular
        || !one(attendee.portion.adjustment)
        || attendee
            .portion
            .meal_overrides
            .iter()
            .filter(|value| value.meal_type == MealType::Lunch)
            .any(|value| value.preset != PortionPresetName::Regular || !one(value.adjustment))
    {
        return Err(unsupported(
            "none_or_gluten_free_adult_unit_extra_recipient",
        ));
    }
    check_cold_home_day(
        meal,
        selection,
        options,
        ColdHomeTiming {
            first_handling_seconds: 300,
            assembly_minutes: 5,
            chilling_minutes: 0,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_requirement_sets_do_not_alias_gluten_free_to_allergen_avoidance() {
        let v = HardRequirement::Diet {
            code: DietCode::Vegetarian,
        };
        let gf = HardRequirement::Diet {
            code: DietCode::GlutenFree,
        };
        for allowed in [
            vec![],
            vec![v.clone()],
            vec![gf.clone()],
            vec![v.clone(), gf.clone()],
            vec![gf.clone(), v.clone()],
        ] {
            assert!(base_requirements(&allowed));
        }
        for forbidden in [
            vec![v.clone(), v.clone()],
            vec![gf.clone(), gf.clone()],
            vec![HardRequirement::Diet {
                code: DietCode::Vegan,
            }],
            vec![HardRequirement::AllergenExclusion {
                code: AllergenCode::CerealsContainingGluten,
            }],
            vec![HardRequirement::IngredientExclusion {
                ingredient_id: IngredientId("ig_80000000000000000000000000000001".into()),
            }],
        ] {
            assert!(!base_requirements(&forbidden));
            assert!(!recipient_requirements(&forbidden));
        }
        assert!(recipient_requirements(&[]));
        assert!(recipient_requirements(&[gf.clone()]));
        assert!(!recipient_requirements(&[v.clone()]));
        assert!(!recipient_requirements(&[v, gf]));
    }
}
