//! Bounded original-share chilli family with exact accepted recipe/packet pins.
//! Native derivation remains authoritative for cooling, age and conservation.
use super::*;

pub const CHILLI_BATCH_SCOPE: PersonalUseScope = PersonalUseScope {
    scope_id: "chilli-four-share-hot-home-dinner-v1",
    recipe_id: "rc_80000000000000000000000000000011",
    recipe_revision: 1,
    recipe_digest: "1dfb0d625718a3eb890701e34e69e2f7c3ae75a3958dab6d35636c393eaece3b",
    meal_type: MealType::Dinner,
    source_url: "https://raw.githubusercontent.com/ronaldl29/public-domain-recipes/da84378b36bd5b2e3cb35f610d64630bf1bd899d/content/lenten-chili.md",
    packet_canonical_digest: "99bdfba2526d24f6c6d2ca78751cfc3464e21bd52386b1abc6e7d6436b89cf80",
    attention_reservation_minutes: 67,
    timing_is_estimate: true,
    portion_convention: "Cook the complete batch and divide its actual finished weight into four equal operational shares. One unrestricted Adult Regular diner eats one share; the two distinct reviewed lunches may each use one other share. The fourth share remains unallocated. No measured yield, nutrition or portion adequacy claim.",
};
pub const CHILLI_WRAPS_SCOPE: PersonalUseScope = PersonalUseScope {
    scope_id: "chilli-original-share-wrap-home-lunch-v1",
    recipe_id: "rc_80000000000000000000000000000012",
    recipe_revision: 1,
    recipe_digest: "dafaef4f8f417e2ca7f05643aaa060c76be69e6c0a3ff0b18e5969c1c83ec41f",
    meal_type: MealType::Lunch,
    source_url: CHILLI_BATCH_SCOPE.source_url,
    packet_canonical_digest: "6cd211674048a6860513db53c0146e11aa938394ef9ba5fe88c5ebd842671aa9",
    attention_reservation_minutes: 15,
    timing_is_estimate: true,
    portion_convention: "One original cooked chilli share plus two ready-to-eat wheat tortillas for one unrestricted Adult Regular diner. Reheat only this share once; consume all filling at this meal, including any served alongside. No new batch or further carry-over.",
};
pub const CHILLI_POTATO_SCOPE: PersonalUseScope = PersonalUseScope {
    scope_id: "chilli-original-share-potato-home-lunch-v1",
    recipe_id: "rc_80000000000000000000000000000013",
    recipe_revision: 1,
    recipe_digest: "5ad6ce59c15710de3ce8c629cc352151e97f201af5e95670869c997507d9dec3",
    meal_type: MealType::Lunch,
    source_url: "https://en.wikibooks.org/w/index.php?title=Cookbook:Baked_Potatoes&oldid=4612337",
    packet_canonical_digest: "6f5fa887ae1c57a883d7139d0ad7c01bd37424004623fba5846a57ad17a150af",
    attention_reservation_minutes: 30,
    timing_is_estimate: true,
    portion_convention: "One original cooked chilli share plus a separately prepared 250 g whole raw baking potato for one unrestricted Adult Regular diner. Reheat only this chilli share once; serve immediately. No combined cooked-mass claim, new batch or further carry-over.",
};
pub const CHILLI_COMPONENT_ID: &str = "co_80000000000000000000000000000011";

pub(crate) fn is_chilli_service(reference: &RecipeRef) -> bool {
    [CHILLI_WRAPS_SCOPE.recipe_id, CHILLI_POTATO_SCOPE.recipe_id]
        .contains(&reference.recipe_id.0.as_str())
}

pub(super) fn is_family(scope: &PersonalUseScope) -> bool {
    [
        CHILLI_BATCH_SCOPE.recipe_id,
        CHILLI_WRAPS_SCOPE.recipe_id,
        CHILLI_POTATO_SCOPE.recipe_id,
    ]
    .contains(&scope.recipe_id)
}

pub(crate) fn chilli_service_input(
    recipe: &RecipeRevision,
) -> Result<&ReusedComponentInput, DomainError> {
    let wrap = recipe.recipe_id.0 == CHILLI_WRAPS_SCOPE.recipe_id;
    let potato = recipe.recipe_id.0 == CHILLI_POTATO_SCOPE.recipe_id;
    let expected_fresh = Quantity {
        amount: PositiveRational::new(if wrap { 2 } else { 250 }, 1).expect("fixed fresh input"),
        unit: if wrap { Unit::Each } else { Unit::G },
        form: if wrap {
            IngredientForm::Prepared
        } else {
            IngredientForm::Raw
        },
    };
    if (!wrap && !potato)
        || recipe.ingredients.len() != 1
        || recipe.ingredients[0].quantity.0.as_ref() != Some(&expected_fresh)
        || !recipe.extras.is_empty()
        || !recipe.storage.is_empty()
        || recipe.capabilities.leftovers
        || !one(recipe.r#yield.servings)
        || recipe.reused_inputs.len() != 1
    {
        return Err(unsupported("chilli_service_definition"));
    }
    let input = &recipe.reused_inputs[0];
    if input.source_recipe.recipe_id.0 != CHILLI_BATCH_SCOPE.recipe_id
        || input.source_component_id.0 != CHILLI_COMPONENT_ID
        || !one(input.quantity_per_recipe_yield.amount)
        || input.quantity_per_recipe_yield.unit != Unit::Each
        || input.quantity_per_recipe_yield.form != IngredientForm::Cooked
        || input.planned_reheats != 1
    {
        return Err(unsupported("chilli_original_share"));
    }
    Ok(input)
}

pub(super) fn notices(scope: &PersonalUseScope) -> Vec<String> {
    vec![scope.portion_convention.into(),
        "Exactly one confirmed unrestricted Adult Regular ×1 attendee at home. All meals require a hob and fridge; the potato lunch also requires an oven. No extras, travel, portion compensation or dietary capability is supplied.".into(),
        "Admit the exact original dinner before either lunch, then select its saved original batch. Each distinct lunch may occur once, on either of the two following local days, within 48 hours of original cooking completion or any shorter applicable limit. The original clock starts before weighing and portioning, not at dinner service.".into(),
        "Cool all reserved shares promptly in shallow containers and refrigerate within two hours of cooking, sooner when cool. Keep at a checked 0–5°C. Reheat only the selected, never-reheated share once until thoroughly hot, then serve immediately. Unknown or failed cooking, cooling, chilling, label or reheating conditions mean do not use the affected food.".into(),
        "Times reserve estimated work and equipment; they do not prove readiness, temperature or physical handling. Actual products, equal-share weights, whole-potato suitability and diner adequacy remain unchecked. Preserve the original deadline; do not retain or reuse a remixed output.".into(),
    ]
}

fn later_lunch(source: &SlotId, target: &SlotId) -> Result<(), DomainError> {
    let parse = |date: &LocalDate| {
        chrono::NaiveDate::parse_from_str(&date.0, "%Y-%m-%d")
            .map_err(|_| refusal(ErrorCode::InvalidValue, "chilli_service_date"))
    };
    let days = (parse(&target.date)? - parse(&source.date)?).num_days();
    if source.week_id != target.week_id
        || target.meal_type != MealType::Lunch
        || !(1..=2).contains(&days)
    {
        return Err(unsupported("chilli_following_two_days"));
    }
    Ok(())
}

fn single_unit(selection: &MealSelection, reference: &RecipeRef, meal_type: MealType) -> bool {
    selection.recipe == *reference
        && selection.servings.len() == 1
        && selection.extras.is_empty()
        && selection.portable_cold.is_none()
        && selection.servings.iter().all(|serving| {
            serving.preset_ref.recipe == *reference
                && serving.preset_ref.age_band == AgeBand::Adult
                && serving.preset_ref.meal_type == meal_type
                && serving.preset_ref.preset == PortionPresetName::Regular
                && one(serving.servings)
        })
}

pub(super) fn check(
    recipe: &RecipeRevision,
    reference: &RecipeRef,
    meal: &MealRequirements,
    selection: &MealSelection,
    plan: &WeekPlan,
    options: &BoardOptions,
    source_reference: &RecipeRef,
    wrap_reference: &RecipeRef,
    potato_reference: &RecipeRef,
) -> Result<(), DomainError> {
    let hot = reference == source_reference;
    if !hot && reference != wrap_reference && reference != potato_reference {
        return Err(refusal(ErrorCode::IdentityMismatch, "recipe_reference"));
    }
    let meal_type = if hot {
        MealType::Dinner
    } else {
        MealType::Lunch
    };
    if meal.location != MealLocation::AtHome
        || !meal.equipment.contains(&EquipmentCode::Fridge)
        || !meal.equipment.contains(&EquipmentCode::Hob)
        || (reference == potato_reference && !meal.equipment.contains(&EquipmentCode::Oven))
    {
        return Err(unsupported("chilli_hot_home_equipment"));
    }
    if !meal.enabled
        || meal.slot.meal_type != meal_type
        || meal.attendees.len() != 1
        || !meal.shared_hard_requirements.is_empty()
    {
        return Err(unsupported("chilli_one_unrestricted_adult"));
    }
    let attendee = &meal.attendees[0];
    if attendee.age_band != AgeBand::Adult
        || !attendee.hard_requirements.is_empty()
        || attendee.portion.preset != PortionPresetName::Regular
        || !one(attendee.portion.adjustment)
        || attendee
            .portion
            .meal_overrides
            .iter()
            .filter(|value| value.meal_type == meal_type)
            .any(|value| value.preset != PortionPresetName::Regular || !one(value.adjustment))
    {
        return Err(unsupported("chilli_unchanged_adult_regular_share"));
    }
    if !single_unit(selection, reference, meal_type)
        || selection.servings[0].person != attendee.person
    {
        return Err(refusal(
            ErrorCode::IdentityMismatch,
            "chilli_serving_identity",
        ));
    }
    if options.make_extra == Some(true)
        || options.portable_cold.is_some()
        || options
            .extra_recipients
            .as_ref()
            .is_some_and(|people| !people.is_empty())
    {
        return Err(unsupported("chilli_no_extras_or_travel"));
    }
    overrides_are_unit(
        &selection.meal_overrides,
        &meal.attendees.0,
        "selection_portion_override",
        Some(PortionPresetName::Regular),
    )?;
    if let Some(overrides) = &options.overrides {
        if overrides.validate_shape().is_err() {
            return Err(refusal(ErrorCode::InvalidValue, "request_portion_override"));
        }
        overrides_are_unit(
            overrides,
            &meal.attendees.0,
            "request_portion_override",
            Some(PortionPresetName::Regular),
        )?;
    }
    if hot {
        if recipe.components.len() != 1
            || recipe.components[0].component_id.0 != CHILLI_COMPONENT_ID
            || !recipe.reused_inputs.is_empty()
            || !recipe.extras.is_empty()
            || recipe.r#yield.servings != PositiveRational::new(4, 1).expect("four shares")
            || recipe.r#yield.components.len() != 1
            || recipe.r#yield.components[0].component_id.0 != CHILLI_COMPONENT_ID
            || recipe.r#yield.components[0].quantity
                != (Quantity {
                    amount: PositiveRational::new(4, 1).expect("four shares"),
                    unit: Unit::Each,
                    form: IngredientForm::Cooked,
                })
            || !matches!(&recipe.components[0].production.0, Some(ComponentProductionSpec::CookedWithCooling { max_cooling_seconds, .. }) if *max_cooling_seconds <= 7200)
            || recipe.storage.len() != 1
            || recipe.storage[0].component_id.0 != CHILLI_COMPONENT_ID
            || recipe.storage[0].max_chilled_seconds > 172800
            || recipe.storage[0].max_reheats != 1
        {
            return Err(refusal(
                ErrorCode::IdentityMismatch,
                "chilli_source_definition",
            ));
        }
    } else if chilli_service_input(recipe)?.source_recipe != *source_reference {
        return Err(refusal(
            ErrorCode::IdentityMismatch,
            "chilli_original_reference",
        ));
    }
    let selected_batch = batch_id(selection);
    let sources = plan.selections.iter().filter(|other|
        matches!(&other.batch_use, BatchUse::NewBatch { batch_id, .. } if batch_id == selected_batch))
        .collect::<Vec<_>>();
    if sources.len() != 1
        || !single_unit(sources[0], source_reference, MealType::Dinner)
        || sources[0].slot.meal_type != MealType::Dinner
        || !matches!(&sources[0].batch_use, BatchUse::NewBatch { production_servings, .. }
            if *production_servings == PositiveRational::new(4, 1).expect("four shares"))
    {
        return Err(refusal(
            ErrorCode::IdentityMismatch,
            "chilli_original_batch",
        ));
    }
    let source = sources[0];
    if hot {
        if selection.slot != source.slot || options.leftover_source.is_some() {
            return Err(unsupported("chilli_original_dinner"));
        }
    } else if options
        .leftover_source
        .as_ref()
        .is_some_and(|slot| slot != &source.slot)
    {
        return Err(unsupported("chilli_exact_source_request"));
    }
    let mut seen = Vec::new();
    for other in plan
        .selections
        .iter()
        .filter(|other| batch_id(other) == selected_batch && other.slot != source.slot)
    {
        let expected = if other.recipe == *wrap_reference {
            wrap_reference
        } else if other.recipe == *potato_reference {
            potato_reference
        } else {
            return Err(unsupported("chilli_only_two_distinct_services"));
        };
        if seen.contains(expected)
            || !single_unit(other, expected, MealType::Lunch)
            || !matches!(&other.batch_use, BatchUse::Leftover { component_ids, .. }
                if component_ids.len() == 1 && component_ids[0].0 == CHILLI_COMPONENT_ID)
        {
            return Err(unsupported("chilli_only_two_distinct_services"));
        }
        seen.push(expected.clone());
        later_lunch(&source.slot, &other.slot)?;
        if plan
            .batch_links
            .iter()
            .filter(|link| link.target_slot == other.slot)
            .count()
            != 1
        {
            return Err(refusal(ErrorCode::IdentityMismatch, "chilli_original_link"));
        }
    }
    for link in plan.batch_links.iter().filter(|link| {
        &link.batch_id == selected_batch
            || link.source_slot == selection.slot
            || link.target_slot == selection.slot
    }) {
        if &link.batch_id != selected_batch
            || link.source_slot != source.slot
            || link.components.len() != 1
            || link.components[0].component_id.0 != CHILLI_COMPONENT_ID
            || !one(link.components[0].quantity.amount)
            || link.components[0].quantity.unit != Unit::Each
            || link.components[0].quantity.form != IngredientForm::Cooked
            || !plan.selections.iter().any(|target| {
                target.slot == link.target_slot
                    && (target.recipe == *wrap_reference || target.recipe == *potato_reference)
                    && batch_id(target) == selected_batch
            })
        {
            return Err(refusal(ErrorCode::IdentityMismatch, "chilli_original_link"));
        }
    }
    Ok(())
}
