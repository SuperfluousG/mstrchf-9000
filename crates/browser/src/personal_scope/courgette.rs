//! Operational scope for the separately pinned hot source and home cold service.
//! Native derivation still owns quantities, cooling, resources and original age.
use super::*;

fn next_day(source: &SlotId, target: &SlotId) -> Result<(), DomainError> {
    let date = chrono::NaiveDate::parse_from_str(&source.date.0, "%Y-%m-%d")
        .map_err(|_| refusal(ErrorCode::InvalidValue, "source_date"))?;
    if date
        .succ_opt()
        .map(|day| day.format("%Y-%m-%d").to_string())
        .as_ref()
        != Some(&target.date.0)
    {
        return Err(unsupported("next_day_home_lunch"));
    }
    Ok(())
}

pub(super) fn check(
    recipe: &RecipeRevision,
    reference: &RecipeRef,
    meal: &MealRequirements,
    selection: &MealSelection,
    plan: &WeekPlan,
    options: &BoardOptions,
    source_reference: &RecipeRef,
    cold_reference: &RecipeRef,
) -> Result<(), DomainError> {
    let hot = reference == source_reference;
    if !hot && reference != cold_reference {
        return Err(refusal(ErrorCode::IdentityMismatch, "recipe_reference"));
    }
    let meal_type = if hot {
        MealType::Dinner
    } else {
        MealType::Lunch
    };
    if meal.location != MealLocation::AtHome || !meal.equipment.contains(&EquipmentCode::Fridge) {
        return Err(unsupported("at_home_with_fridge"));
    }
    if hot
        && (!meal.equipment.contains(&EquipmentCode::Hob)
            || !meal.equipment.contains(&EquipmentCode::Oven)
            || meal.mobility.temperature == MobilityTemperature::Cold)
    {
        return Err(unsupported("hot_home_cooking"));
    }
    if meal.slot.meal_type != meal_type
        || !meal.enabled
        || meal.attendees.is_empty()
        || meal.attendees.len() > if hot { 4 } else { 3 }
        || !meal.shared_hard_requirements.is_empty()
    {
        return Err(unsupported("home_adult_share_cohort"));
    }
    if options.make_extra == Some(true)
        || !selection.extras.is_empty()
        || options
            .extra_recipients
            .as_ref()
            .is_some_and(|people| !people.is_empty())
    {
        return Err(unsupported("single_original_batch_no_extras"));
    }
    for attendee in meal.attendees.iter() {
        let overrides = attendee
            .portion
            .meal_overrides
            .iter()
            .filter(|value| value.meal_type == meal_type)
            .collect::<Vec<_>>();
        if attendee.age_band != AgeBand::Adult
            || !attendee.hard_requirements.is_empty()
            || attendee.portion.preset != PortionPresetName::Regular
            || !one(attendee.portion.adjustment)
            || overrides.len() > 1
            || overrides
                .iter()
                .any(|value| value.preset != PortionPresetName::Regular || !one(value.adjustment))
        {
            return Err(unsupported("unchanged_adult_regular_share"));
        }
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
    if selection.servings.len() != meal.attendees.len()
        || selection.servings.iter().any(|serving| {
            !meal
                .attendees
                .iter()
                .any(|attendee| attendee.person == serving.person)
                || serving.preset_ref.recipe != *reference
                || serving.preset_ref.age_band != AgeBand::Adult
                || serving.preset_ref.meal_type != meal_type
                || serving.preset_ref.preset != PortionPresetName::Regular
                || !one(serving.servings)
        })
        || meal.attendees.iter().any(|attendee| {
            selection
                .servings
                .iter()
                .filter(|serving| serving.person == attendee.person)
                .count()
                != 1
        })
    {
        return Err(refusal(ErrorCode::IdentityMismatch, "serving_identity"));
    }
    let original_component = if hot {
        if recipe.components.len() != 1 || !recipe.reused_inputs.is_empty() {
            return Err(refusal(ErrorCode::IdentityMismatch, "source_component"));
        }
        &recipe.components[0].component_id
    } else {
        if !recipe.ingredients.is_empty()
            || recipe.reused_inputs.len() != 1
            || recipe.reused_inputs[0].source_recipe != *source_reference
        {
            return Err(refusal(ErrorCode::IdentityMismatch, "original_input"));
        }
        &recipe.reused_inputs[0].source_component_id
    };
    let selected_batch = batch_id(selection);
    let sources = plan
        .selections
        .iter()
        .filter(|other| {
            matches!(&other.batch_use,
        BatchUse::NewBatch{batch_id,..} if batch_id==selected_batch)
        })
        .collect::<Vec<_>>();
    if sources.len() != 1
        || sources[0].recipe != *source_reference
        || sources[0].slot.meal_type != MealType::Dinner
        || !sources[0].extras.is_empty()
    {
        return Err(refusal(
            ErrorCode::IdentityMismatch,
            "original_batch_source",
        ));
    }
    let source = sources[0];
    if !matches!(&source.batch_use,BatchUse::NewBatch{production_servings,..}
        if *production_servings==PositiveRational::new(4,1).expect("fixed four-share batch"))
    {
        return Err(unsupported("full_four_share_batch"));
    }
    if hot {
        if selection.slot != source.slot
            || options.leftover_source.is_some()
            || !matches!(selection.batch_use, BatchUse::NewBatch { .. })
        {
            return Err(unsupported("original_hot_source"));
        }
    } else {
        if !matches!(&selection.batch_use,BatchUse::Leftover{component_ids,..}
            if component_ids.0==vec![original_component.clone()])
            || options
                .leftover_source
                .as_ref()
                .is_some_and(|slot| *slot != source.slot)
        {
            return Err(unsupported("exact_original_cold_share"));
        }
        next_day(&source.slot, &selection.slot)?;
    }
    for other in plan
        .selections
        .iter()
        .filter(|other| batch_id(other) == selected_batch && other.slot != source.slot)
    {
        if other.recipe != *cold_reference
            || other.slot.meal_type != MealType::Lunch
            || !matches!(&other.batch_use,BatchUse::Leftover{component_ids,..}
                if component_ids.0==vec![original_component.clone()])
        {
            return Err(unsupported("only_declared_home_cold_service"));
        }
        next_day(&source.slot, &other.slot)?;
    }
    for link in plan.batch_links.iter().filter(|link| {
        &link.batch_id == selected_batch
            || link.source_slot == selection.slot
            || link.target_slot == selection.slot
    }) {
        if &link.batch_id != selected_batch
            || link.source_slot != source.slot
            || link.components.len() != 1
            || link.components[0].component_id != *original_component
            || !plan.selections.iter().any(|target| {
                target.slot == link.target_slot
                    && target.recipe == *cold_reference
                    && batch_id(target) == selected_batch
            })
        {
            return Err(refusal(
                ErrorCode::IdentityMismatch,
                "original_component_link",
            ));
        }
    }
    Ok(())
}
