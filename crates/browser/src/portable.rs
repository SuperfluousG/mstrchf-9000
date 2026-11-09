//! Manual browser input for the exact qualified portable pair. No declaration
//! can be inferred from a household default or an AI return.
use crate::{catalogue::RuntimeCatalogue, planner_view::ChoiceOptions};
use mstrchf_contracts::*;
use mstrchf_core::{food::PhaseKey, recipes::ReviewedRecipe, selection::*};

pub(crate) const SOURCE_ID: &str = "rc_8000000000000000000000000000000d";
pub(crate) const COLD_ID: &str = "rc_8000000000000000000000000000000e";

/// Suggestions may fill other cells, but cannot rewrite an owner's declared
/// journey or its exact original source. The core rechecks full custody too.
pub(crate) fn owner_bound_slot(plan: &WeekPlan, slot: &SlotId) -> bool {
    plan.selections.iter().filter(|selection|selection.portable_cold.is_some()).any(|selection| {
        if selection.slot==*slot {return true;}
        let BatchUse::Leftover{batch_id,..}=&selection.batch_use else {return false;};
        plan.selections.iter().any(|source|source.slot==*slot
            && matches!(&source.batch_use,BatchUse::NewBatch{batch_id:source_batch,..} if source_batch==batch_id))
    })
}
fn error(code: ErrorCode, path: &str) -> DomainError { DomainError::new(code, path) }
fn unresolved(slot: &SlotId, recipe: &RecipeRef) -> ChoiceAssessment {
    ChoiceAssessment::Unresolved(vec![Conflict {code:ErrorCode::UnsupportedCapability,
        week_ids:BoundedVec(vec![slot.week_id.clone()]),slot_ids:BoundedVec(vec![slot.clone()]),
        recipe_refs:BoundedVec(vec![recipe.clone()]),evidence_ids:BoundedVec(vec![]),blocking:true}])
}

pub(crate) fn assess_saved_choice(view: &RuntimeCatalogue, context: &PlanningContext,
    slot: &SlotId, choice: &ChoiceSpec, origin: SelectionOrigin, repeat: bool,
    reviewed: &[ReviewedRecipe<'_>]) -> Result<ChoiceAssessment, DomainError>
{
    if choice.portable_cold.is_none() {
        return assess_choice(context,slot,choice,origin,repeat,reviewed);
    }
    assess_choice_with_food(context,slot,choice,origin,repeat,reviewed,
        &view.food_evidence(&context.stamp.recipe_refs.0)?)
}

pub(crate) fn assess_manual_choice(view: &RuntimeCatalogue, context: &PlanningContext,
    slot: &SlotId, choice: &ChoiceSpec, options: &ChoiceOptions,
    reviewed: &[ReviewedRecipe<'_>]) -> Result<ChoiceAssessment, DomainError>
{
    if choice.recipe.recipe_id.0 != COLD_ID {
        return assess_choice(context,slot,choice,SelectionOrigin::Fixed,false,reviewed);
    }
    let Some(input) = options.portable_cold.as_ref().filter(|input| input.chilling_confirmed) else {
        return Ok(unresolved(slot,&choice.recipe));
    };
    if !choice.extras.is_empty() || options.make_extra || options.leftover_source.is_none() {
        return Ok(unresolved(slot,&choice.recipe));
    }
    let recipe = view.resolve_recipe(&choice.recipe)?;
    let base = reviewed.iter().find(|base| base.recipe() == recipe)
        .ok_or_else(||error(ErrorCode::MissingReference,"/portable_cold/recipe"))?;
    let meal = context.requirements.iter().find(|meal| meal.slot == *slot)
        .ok_or_else(||error(ErrorCode::MissingReference,"/portable_cold/meal"))?;
    if meal.location != MealLocation::OnTheMove {
        return Ok(unresolved(slot,&choice.recipe));
    }
    let servings = match mstrchf_core::portions::resolve_servings(base,meal,&choice.meal_overrides) {
        Ok(servings) => servings,
        Err(conflicts) => return Ok(ChoiceAssessment::Unresolved(conflicts)),
    };
    let mut selected = MealSelection {slot:slot.clone(),recipe:choice.recipe.clone(),
        origin:SelectionOrigin::Fixed,repeat_authorised:false,servings:BoundedVec(servings),
        extras:BoundedVec(vec![]),batch_use:choice.batch_use.clone(),
        meal_overrides:choice.meal_overrides.clone(),portable_cold:None};
    let BatchUse::Leftover {batch_id,component_ids} = &selected.batch_use else {
        return Ok(unresolved(slot,&choice.recipe));
    };
    if component_ids.len()!=1 || recipe.reused_inputs.len()!=1 {
        return Err(error(ErrorCode::UnsupportedCapability,"/portable_cold/component"));
    }
    let source = context.weeks.iter().flat_map(|week|week.plan.selections.iter())
        .find(|source| options.leftover_source.as_ref()==Some(&source.slot)
            && source.recipe==recipe.reused_inputs[0].source_recipe
            && matches!(&source.batch_use,BatchUse::NewBatch{batch_id:id,..} if id==batch_id))
        .ok_or_else(||error(ErrorCode::MissingReference,"/portable_cold/source"))?;
    let total=selected.servings.iter().try_fold(Rational::new(0,1)?,
        |total,serving|total.checked_add(&serving.servings.0))?;
    let scaled=mstrchf_core::portions::scale_recipe(base,PositiveRational(total))?;
    let source_input=&recipe.reused_inputs[0];
    let link=BatchLink {batch_id:batch_id.clone(),source_slot:source.slot.clone(),target_slot:slot.clone(),
        components:BoundedVec(vec![ComponentAllocation {component_id:source_input.source_component_id.clone(),
            quantity:Quantity {amount:PositiveRational(source_input.quantity_per_recipe_yield.amount.0
                .checked_mul(&scaled.multiplier.0)?),..source_input.quantity_per_recipe_yield.clone()}}])};
    // Declare against the actual candidate requirements, including a changed
    // lunch time, rather than the old target cell's earlier requirement record.
    let mut candidate=context.clone();
    let week=candidate.weeks.iter_mut().find(|week|week.range.week_id==slot.week_id)
        .ok_or_else(||error(ErrorCode::MissingReference,"/portable_cold/week"))?;
    week.plan.selections.0.retain(|old|old.slot!=*slot);
    week.plan.selections.0.push(selected.clone());
    week.plan.batch_links.0.retain(|old|old.target_slot!=*slot);
    week.plan.batch_links.0.push(link);
    let resolutions=context.requirements.iter().filter(|meal|meal.enabled
        && meal.slot.week_id==slot.week_id
        && !week.plan.selections.iter().any(|selection|selection.slot==meal.slot&&selection.meal_overrides.eat_by.0.is_some()))
        .filter_map(|meal|meal.eat_by.0.as_ref().map(|at|mstrchf_core::availability::SlotResolution{
            slot:meal.slot.clone(),resolution:at.resolution})).collect::<Vec<_>>();
    let requirements=mstrchf_core::availability::meal_requirements(&candidate.household,week,&resolutions)?;
    let zone=week.range.zone.clone();
    for requirement in requirements {
        let old=candidate.requirements.iter_mut().find(|old|old.slot==requirement.slot)
            .ok_or_else(||error(ErrorCode::MissingReference,"/portable_cold/requirements"))?;
        *old=requirement;
    }
    let packing=mstrchf_core::availability::resolve_meal_time(&slot.date,&input.packing_complete_time,
        &zone,Some(input.packing_resolution))?;
    let departure=mstrchf_core::availability::resolve_meal_time(&slot.date,&input.depart_time,
        &zone,Some(input.departure_resolution))?;
    selected.portable_cold=Some(mstrchf_core::portable::declare_journey(&candidate,&selected,packing,
        departure,crate::development::now()?)?);
    let mut declared=choice.clone();
    declared.portable_cold=selected.portable_cold;
    assess_choice_with_food(&candidate,slot,&declared,SelectionOrigin::Fixed,false,reviewed,
        &view.food_evidence(&candidate.stamp.recipe_refs.0)?)
}

pub(crate) fn phase_anchor<'a>(view: &RuntimeCatalogue, selected: &'a MealSelection,
    phase: &PhaseKey, meal: &'a MealRequirements) -> Result<&'a ZonedMoment,DomainError>
{
    if let Some(journey)=&selected.portable_cold {
        let recipe=view.resolve_recipe(&selected.recipe)?;
        let evidence=view.food_evidence(std::slice::from_ref(&selected.recipe))?;
        let method=mstrchf_core::portable::method(recipe,&evidence)?
            .ok_or_else(||error(ErrorCode::MissingReference,"/portable_cold/method"))?;
        if phase.occurrence.extra_id.is_none() && phase.prep_id==method.packing_prep_id {
            return Ok(&journey.packing_complete_at);
        }
    }
    selected.meal_overrides.eat_by.0.as_ref().or(meal.eat_by.0.as_ref())
        .ok_or_else(||error(ErrorCode::MissingReference,"/eat_by"))
}
