//! Renderer13/14 retain the exact unheated first-service clock across midnight.
//! Chilling is a handling condition, never exclusive overnight fridge occupancy.
use super::*;

pub(super) fn exact(reference: &RecipeRef) -> bool {
    let scope = &crate::personal_scope::OVERNIGHT_OATS_SCOPE;
    reference.recipe_id.0 == scope.recipe_id && reference.revision.0 == scope.recipe_revision
        && reference.digest.0 == scope.recipe_digest
}

pub(super) fn allocation(view: &RuntimeCatalogue, batch: &DerivedBatch,
    component: &DerivedComponent, allocation: &Allocation) -> Result<(), CalendarFailure> {
    let recipe = packet(view, &batch.recipe)?.recipe();
    require(exact(&batch.recipe) && recipe.components.len() == 1 && recipe.storage.len() == 1
        && component.component_id == recipe.components[0].component_id
        && component.storage_rule.0.as_ref() == Some(&recipe.storage[0])
        && recipe.storage[0].max_chilled_seconds == 86400 && recipe.storage[0].max_reheats == 0
        && allocation.target_slot == batch.source_slot && allocation.planned_reheats == 0
        && allocation.remix_recipe.0.is_none()
        && allocation.quantity == Quantity { amount: PositiveRational::new(1,1)?, unit: Unit::Each, form: IngredientForm::Prepared },
        ErrorCode::UnsupportedCapability,"/calendar/overnight/first_service")?;
    let ProductionMoment::Unheated { preparation_started_at } = &component.original_production else {
        return Err(error(ErrorCode::UnsupportedCapability,"/calendar/overnight/first_handling"));
    };
    let start = chrono::DateTime::parse_from_rfc3339(&preparation_started_at.instant.0)
        .map_err(|_|error(ErrorCode::InvalidValue,"/calendar/overnight/first_handling"))?;
    let service = chrono::DateTime::parse_from_rfc3339(&allocation.use_at.instant.0)
        .map_err(|_|error(ErrorCode::InvalidValue,"/calendar/overnight/service"))?;
    let prior = chrono::NaiveDate::parse_from_str(&preparation_started_at.local_date.0,"%Y-%m-%d")
        .map_err(|_|error(ErrorCode::InvalidValue,"/calendar/overnight/first_date"))?;
    let next = chrono::NaiveDate::parse_from_str(&allocation.use_at.local_date.0,"%Y-%m-%d")
        .map_err(|_|error(ErrorCode::InvalidValue,"/calendar/overnight/service_date"))?;
    require(service.signed_duration_since(start).num_seconds() == 43500
        && prior.succ_opt() == Some(next) && preparation_started_at.zone == allocation.use_at.zone,
        ErrorCode::UnsupportedCapability,"/calendar/overnight/original_clock")?;
    Ok(())
}

pub(super) fn presentation(input: &PersonalCalendarInputs<'_>, draft: &mut EventDraft,
    original: &str) -> Result<bool, CalendarFailure> {
    let sessions = draft.contributors.iter().flat_map(|member| input.derived.prep_sessions.iter()
        .filter(move |session|session.session_id == member.session_id)).collect::<Vec<_>>();
    let tasks = sessions.iter().flat_map(|session|session.tasks.iter()).collect::<Vec<_>>();
    if !tasks.iter().any(|task|exact(&task.recipe)) {return Ok(false);}
    require(sessions.len() == 1 && tasks.len() == 1 && exact(&tasks[0].recipe)
        && matches!(tasks[0].prep_id,1|2),ErrorCode::UnsupportedCapability,"/calendar/overnight/session")?;
    let task = tasks[0];
    let selected = input.basis.plan.selections.iter().filter(|selection|
        selection.recipe == task.recipe && sessions[0].linked_slots.contains(&selection.slot)).collect::<Vec<_>>();
    require(selected.len() == 1,ErrorCode::IdentityMismatch,"/calendar/overnight/selection")?;
    let slot = &selected[0].slot;
    let phases = input.derived.prep_sessions.iter().filter(|session|session.linked_slots.contains(slot))
        .flat_map(|session|session.tasks.iter().filter(move |phase|phase.recipe == task.recipe)
        .map(move |phase|(session,phase))).collect::<Vec<_>>();
    let mixing = phases.iter().find(|(_,phase)|phase.prep_id == 1)
        .ok_or_else(||error(ErrorCode::MissingReference,"/calendar/overnight/mixing"))?.0;
    let serving = phases.iter().find(|(_,phase)|phase.prep_id == 2)
        .ok_or_else(||error(ErrorCode::MissingReference,"/calendar/overnight/serving"))?.0;
    require(phases.len() == 2,ErrorCode::IdentityMismatch,"/calendar/overnight/phases")?;
    let chilled_from = chrono::DateTime::parse_from_rfc3339(&mixing.end.instant.0)
        .map_err(|_|error(ErrorCode::InvalidValue,"/calendar/overnight/chill_start"))?;
    let retrieved_at = chrono::DateTime::parse_from_rfc3339(&serving.start.instant.0)
        .map_err(|_|error(ErrorCode::InvalidValue,"/calendar/overnight/retrieval"))?;
    require(retrieved_at.signed_duration_since(chilled_from).num_seconds() >= 18000,
        ErrorCode::DomainConflict,"/calendar/overnight/minimum_chilling")?;
    let role = if task.prep_id == 1 {"Mix and refrigerate for Monday breakfast"} else {"Check and serve cold breakfast"};
    draft.title.0 = format!("{role}: {}",packet(input.original_catalogue,&task.recipe)?.recipe().title.0);
    let note = format!("{role}. Planned mixing and refrigeration: {} to {}. Planned retrieval and cold service: {} to {}. Keep covered continuously at 0–5°C between these tasks, for at least five hours before retrieval; the gap is not an occupied cooking session. The original age starts when preparation begins, not when served. Use only the following morning within the authored 24-hour maximum; shorter product-label and use-by limits take precedence. Use ingredients suitable for uncooked consumption. No heating, further reuse or freezing is qualified. Execute each event's method once. These are unmeasured five-minute reservations; the plan does not establish actual temperatures, labels, readiness or handling.",
        readable_moment(&mixing.start),readable_moment(&mixing.end),readable_moment(&serving.start),readable_moment(&serving.end));
    draft.body.0 = draft.body.0.replacen(original,&note,1);
    Ok(true)
}
