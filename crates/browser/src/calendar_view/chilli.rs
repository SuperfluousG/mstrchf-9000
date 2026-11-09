//! Renderer11/12 add only the separately reviewed original chilli family.
//! Native derivation remains the authority for quantities, clocks and reheats.
use super::*;
use crate::personal_scope::{CHILLI_BATCH_SCOPE, CHILLI_POTATO_SCOPE, CHILLI_WRAPS_SCOPE};

pub(super) fn family_id(reference: &RecipeRef) -> bool {
    [CHILLI_BATCH_SCOPE, CHILLI_WRAPS_SCOPE, CHILLI_POTATO_SCOPE]
        .iter().any(|scope| reference.recipe_id.0 == scope.recipe_id)
}

pub(super) fn exact(reference: &RecipeRef, source: bool) -> bool {
    [CHILLI_BATCH_SCOPE, CHILLI_WRAPS_SCOPE, CHILLI_POTATO_SCOPE]
        .iter().enumerate().any(|(index, scope)| {
            (index == 0) == source && reference.recipe_id.0 == scope.recipe_id
                && reference.revision.0 == scope.recipe_revision
                && reference.digest.0 == scope.recipe_digest
        })
}

fn one_share(quantity: &Quantity) -> bool {
    quantity.amount == PositiveRational::new(1, 1).unwrap()
        && quantity.unit == Unit::Each && quantity.form == IngredientForm::Cooked
}

pub(super) fn link(plan: &WeekPlan, link: &BatchLink) -> bool {
    let source = plan.selections.iter().find(|selected| selected.slot == link.source_slot);
    let target = plan.selections.iter().find(|selected| selected.slot == link.target_slot);
    source.zip(target).is_some_and(|(source, target)| {
        exact(&source.recipe, true) && exact(&target.recipe, false)
            && source.extras.is_empty() && target.extras.is_empty()
            && source.slot != target.slot && link.components.len() == 1
            && link.components[0].component_id.0 == crate::personal_scope::CHILLI_COMPONENT_ID
            && one_share(&link.components[0].quantity)
            && matches!(&source.batch_use, BatchUse::NewBatch { batch_id, .. } if *batch_id == link.batch_id)
            && matches!(&target.batch_use, BatchUse::Leftover { batch_id, component_ids }
                if *batch_id == link.batch_id && component_ids.len() == 1
                    && component_ids[0] == link.components[0].component_id)
    })
}

pub(super) fn allocation(view: &RuntimeCatalogue, plan: &WeekPlan, batch: &DerivedBatch,
    component: &DerivedComponent, allocation: &Allocation) -> Result<(), CalendarFailure> {
    require(exact(&batch.recipe, true)
        && component.component_id.0 == crate::personal_scope::CHILLI_COMPONENT_ID
        && one_share(&allocation.quantity)
        && component.storage_rule.0.as_ref().is_some_and(|rule|
            rule.max_chilled_seconds == 172800 && rule.max_reheats == 1),
        ErrorCode::UnsupportedCapability, "/calendar/chilli/original_component")?;
    let ProductionMoment::Cooked { completed_at } = &component.original_production else {
        return Err(error(ErrorCode::UnsupportedCapability, "/calendar/chilli/original_clock"));
    };
    let cooked = chrono::DateTime::parse_from_rfc3339(&completed_at.instant.0)
        .map_err(|_| error(ErrorCode::InvalidValue, "/calendar/chilli/original_clock"))?;
    let service = chrono::DateTime::parse_from_rfc3339(&allocation.use_at.instant.0)
        .map_err(|_| error(ErrorCode::InvalidValue, "/calendar/chilli/service_clock"))?;
    if allocation.target_slot == batch.source_slot {
        // The exact producer records thermal completion before five minutes of
        // weighing/portioning. Serving does not replace that original clock.
        require(allocation.remix_recipe.0.is_none() && allocation.planned_reheats == 0
            && service.signed_duration_since(cooked).num_seconds() == 300,
            ErrorCode::UnsupportedCapability, "/calendar/chilli/immediate_dinner")?;
    } else {
        let selected = plan.selections.iter().find(|selected| selected.slot == allocation.target_slot)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/calendar/chilli/service"))?;
        require(exact(&selected.recipe, false) && allocation.planned_reheats == 1
            && allocation.remix_recipe.0.as_ref() == Some(&selected.recipe),
            ErrorCode::UnsupportedCapability, "/calendar/chilli/service")?;
        let recipe = packet(view, &selected.recipe)?.recipe();
        require(recipe.ingredients.len() == 1 && recipe.reused_inputs.len() == 1
            && recipe.storage.is_empty()
            && recipe.reused_inputs[0].source_recipe == batch.recipe
            && recipe.reused_inputs[0].source_component_id == component.component_id
            && recipe.reused_inputs[0].planned_reheats == 1
            && one_share(&recipe.reused_inputs[0].quantity_per_recipe_yield)
            && plan.batch_links.iter().any(|candidate| candidate.batch_id == batch.batch_id
                && candidate.target_slot == allocation.target_slot && link(plan, candidate)),
            ErrorCode::IdentityMismatch, "/calendar/chilli/original_input")?;
    }
    // Historical verification/continuation separately enforce original expiry,
    // cooling, current requirements, exact conservation and per-allocation heat.
    Ok(())
}

pub(super) fn presentation(input: &PersonalCalendarInputs<'_>, draft: &mut EventDraft,
    original: &str) -> Result<bool, CalendarFailure> {
    let references = draft.contributors.iter().flat_map(|member| input.derived.prep_sessions.iter()
        .filter(move |session| session.session_id == member.session_id))
        .flat_map(|session| session.tasks.iter().map(|task| &task.recipe)).collect::<Vec<_>>();
    let source = references.iter().any(|reference| exact(reference, true));
    let service = references.iter().any(|reference| exact(reference, false));
    if !source && !service { return Ok(false); }
    require(!(source && service), ErrorCode::UnsupportedCapability, "/calendar/chilli/assembly")?;
    let note = if source {
        "Divide the actual finished batch into four equal shares by weight: one for dinner and three reserved. No finished gram yield or portion adequacy is established. The original cooking clock begins before weighing and portioning; refrigeration does not restart it. All three reserved shares need prompt cooling and checked refrigeration, including the unallocated share."
    } else {
        "This remix consumes one distinct original chilli share plus only the fresh ingredients listed for this occurrence. Reheat only that allocated share once; do not reheat the reserved batch. The original cooking clock and deadline remain in force. Assembly and reheating create no new storage entitlement or reusable output."
    };
    draft.body.0 = draft.body.0.replacen(original, &format!("{original} {note} Timings are unmeasured planning reservations; actual products, temperatures, readiness and handling still need checking."), 1);
    Ok(true)
}
