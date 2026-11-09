//! Narrow owner-declared cold journey. A declaration binds a plan; it does not
//! certify that food was cooked, cooled, transported or stored as planned.
use crate::food::{FoodEvidence, MethodKind, MethodPhases, PortableColdMethod};
use mstrchf_contracts::*;
use serde_json::json;

fn require(ok: bool, code: ErrorCode, path: &str) -> Result<(), DomainError> {
    if ok { Ok(()) } else { Err(DomainError::new(code, path)) }
}
fn shape(value: &impl ValidateShape) -> Result<(), DomainError> {
    value.validate_shape().map_err(|issues| DomainError::new(
        issues.first().map_or(ErrorCode::InvalidValue, |issue| issue.code),
        issues.first().map_or("/portable_cold", |issue| issue.path.as_str()),
    ))
}

/// Validate the complete admitted method/resource closure before granting the
/// home-preparation exception to ordinary OnTheMove equipment rules.
pub fn method<'a>(recipe: &RecipeRevision, evidence: &'a FoodEvidence)
    -> Result<Option<&'a PortableColdMethod>, DomainError>
{
    let reference = RecipeRef { recipe_id: recipe.recipe_id.clone(), revision: recipe.revision,
        digest: recipe_digest(recipe)? };
    let methods: Vec<&MethodPhases> = evidence.methods.iter()
        .filter(|method| method.recipe == reference && method.portable_cold.is_some()).collect();
    if methods.is_empty() { return Ok(None); }
    require(methods.len() == 1, ErrorCode::InvalidValue, "/methods/portable_cold")?;
    let method = methods[0];
    crate::food::validate_method(method)?;
    let portable = method.portable_cold.as_ref().expect("filtered portable method");
    require(method.kind == MethodKind::Production && method.direct_reuse_counts.is_empty()
        && recipe.ingredients.is_empty() && recipe.reused_inputs.len() == 1
        && recipe.reused_inputs[0].planned_reheats == 0 && recipe.extras.is_empty()
        && recipe.prep.len() == 2 && recipe.prep.iter().all(|phase| method.prep_ids.contains(&phase.prep_id))
        && recipe.equipment.0 == vec![EquipmentCode::Fridge]
        && recipe.portability.cold_suitable && recipe.portability.cold_storage_required
        && !recipe.portability.reheating_required,
        ErrorCode::UnsupportedCapability, "/methods/portable_cold/closure")?;
    for (id, equipment) in [(portable.packing_prep_id, vec![EquipmentCode::Fridge]),
        (portable.service_prep_id, vec![])] {
        let resources: Vec<_> = evidence.resources.iter()
            .filter(|resource| resource.recipe == reference && resource.prep_id == id).collect();
        require(resources.len() == 1 && resources[0].equipment == equipment,
            ErrorCode::UnsupportedCapability, "/methods/portable_cold/resources")?;
    }
    Ok(Some(portable))
}

fn requirement<'a>(context: &'a PlanningContext, slot: &SlotId) -> Result<&'a MealRequirements, DomainError> {
    let values: Vec<_> = context.requirements.iter().filter(|meal| &meal.slot == slot).collect();
    require(values.len() == 1, ErrorCode::MissingReference, "/portable_cold/requirements")?;
    Ok(values[0])
}
fn recipe<'a>(context: &'a PlanningContext, reference: &RecipeRef) -> Result<&'a RecipeRevision, DomainError> {
    require(context.stamp.recipe_refs.contains(reference), ErrorCode::MissingReference, "/portable_cold/recipe")?;
    let values: Vec<_> = context.recipes.iter().filter(|recipe| recipe.recipe_id == reference.recipe_id
        && recipe.revision == reference.revision).collect();
    require(values.len() == 1, ErrorCode::MissingReference, "/portable_cold/recipe")?;
    require(recipe_digest(values[0])? == reference.digest, ErrorCode::DigestMismatch, "/portable_cold/recipe")?;
    Ok(values[0])
}
fn use_at<'a>(selection: &'a MealSelection, meal: &'a MealRequirements) -> Result<&'a ZonedMoment, DomainError> {
    selection.meal_overrides.eat_by.0.as_ref().or(meal.eat_by.0.as_ref())
        .ok_or_else(|| DomainError::new(ErrorCode::UnsupportedCapability, "/portable_cold/eat_by"))
}

fn binding(context: &PlanningContext, selection: &MealSelection, journey: &PortableColdJourney)
    -> Result<PayloadDigest, DomainError>
{
    shape(context)?;
    shape(journey)?;
    let mut target = selection.clone();
    target.portable_cold = None;
    shape(&target)?;
    let meal = requirement(context, &target.slot)?;
    let service = use_at(&target, meal)?;
    shape(service)?;
    let target_recipe = recipe(context, &target.recipe)?;
    require(target.origin == SelectionOrigin::Fixed && target.slot.meal_type == MealType::Lunch
        && target.extras.is_empty() && meal.enabled && meal.location == MealLocation::OnTheMove
        && meal.mobility.cold_storage && meal.mobility.temperature != MobilityTemperature::Hot
        && context.household.preferences.equipment.contains(&EquipmentCode::Fridge),
        ErrorCode::UnsupportedCapability, "/portable_cold/scope")?;
    let (batch_id, component_ids) = match &target.batch_use {
        BatchUse::Leftover {batch_id, component_ids} => (batch_id, component_ids),
        _ => return Err(DomainError::new(ErrorCode::UnsupportedCapability, "/portable_cold/batch_use")),
    };
    require(component_ids.len() == 1 && target_recipe.reused_inputs.len() == 1,
        ErrorCode::UnsupportedCapability, "/portable_cold/component")?;
    let input = &target_recipe.reused_inputs[0];
    require(input.source_component_id == component_ids[0] && input.planned_reheats == 0,
        ErrorCode::IdentityMismatch, "/portable_cold/component")?;
    let sources: Vec<_> = context.weeks.iter().flat_map(|week| week.plan.selections.iter())
        .filter(|source| matches!(&source.batch_use, BatchUse::NewBatch {batch_id: source_id, ..} if source_id == batch_id)).collect();
    require(sources.len() == 1, ErrorCode::MissingReference, "/portable_cold/source")?;
    let source = sources[0];
    require(source.recipe == input.source_recipe && source.portable_cold.is_none()
        && source.extras.is_empty() && source.slot != target.slot,
        ErrorCode::IdentityMismatch, "/portable_cold/source")?;
    let source_recipe = recipe(context, &source.recipe)?;
    require(source_recipe.reused_inputs.is_empty(), ErrorCode::UnsupportedCapability, "/portable_cold/source_chain")?;
    let source_meal = requirement(context, &source.slot)?;
    require(source_meal.enabled && source_meal.location == MealLocation::AtHome,
        ErrorCode::UnsupportedCapability, "/portable_cold/source_location")?;
    let original_completion = use_at(source, source_meal)?;
    let component = source_recipe.components.iter().find(|component| component.component_id == component_ids[0])
        .ok_or_else(|| DomainError::new(ErrorCode::MissingReference, "/portable_cold/component"))?;
    // This first route's cooking endpoint is fixed at source service. The
    // scheduler independently proves that endpoint and all cooling dependencies.
    require(matches!(&component.production.0, Some(ComponentProductionSpec::CookedWithCooling {..})),
        ErrorCode::UnsupportedCapability, "/portable_cold/source_cooling")?;
    let Some(ComponentProductionSpec::CookedWithCooling {completion_prep_id, ..}) = &component.production.0 else {unreachable!()};
    let completion = source_recipe.prep.iter().find(|phase| phase.prep_id == *completion_prep_id)
        .ok_or_else(||DomainError::new(ErrorCode::MissingReference,"/portable_cold/source_completion"))?;
    // Portable v1 binds source service as its original endpoint. Preserve those
    // declaration bytes and refuse earlier-endpoint sources on this route.
    require(completion.earliest_offset_seconds == completion.latest_offset_seconds
        && i64::from(completion.earliest_offset_seconds) + i64::from(completion.elapsed_minutes)*60 == 0,
        ErrorCode::UnsupportedCapability,"/portable_cold/source_completion")?;
    require(journey.packing_complete_at.zone == service.zone
        && service.zone == original_completion.zone
        && journey.packing_complete_at.local_date == target.slot.date
        && service.local_date == target.slot.date
        && original_completion.instant <= journey.packing_complete_at.instant
        && journey.depart_at.instant <= service.instant,
        ErrorCode::DomainConflict, "/portable_cold/chronology")?;
    Ok(PayloadDigest(canonical_digest(&json!({
        "purpose":"mstrchf.owner-portable-cold-journey.v1",
        "target":target,"target_requirements":meal,"source":source,"source_requirements":source_meal,
        "original_completion":original_completion,"service_at":service,
        "packing_complete_at":journey.packing_complete_at,"depart_at":journey.depart_at,
        "declaration_version":journey.declaration.schema_version,"confirmed_at":journey.declaration.confirmed_at
    }))?.0))
}

/// Caller must own the explicit human confirmation. This pure helper grants no
/// write authority and cannot infer a declaration from household defaults.
pub fn declare_journey(context: &PlanningContext, selection: &MealSelection,
    packing_complete_at: ZonedMoment, depart_at: ZonedMoment, confirmed_at: Instant)
    -> Result<PortableColdJourney, DomainError>
{
    let mut journey = PortableColdJourney {packing_complete_at, depart_at,
        declaration: PortableChillingDeclaration {schema_version: SchemaVersion, confirmed_at,
            journey_digest: PayloadDigest("0".repeat(64))}};
    journey.declaration.journey_digest = binding(context, selection, &journey)?;
    Ok(journey)
}

pub fn validate_journey(context: &PlanningContext, selection: &MealSelection) -> Result<(), DomainError> {
    let Some(journey) = &selection.portable_cold else { return Ok(()); };
    shape(selection)?;
    require(binding(context, selection, journey)? == journey.declaration.journey_digest,
        ErrorCode::DigestMismatch, "/portable_cold/declaration/journey_digest")
}

/// An AI patch must preserve owner declarations even while their meal is
/// disabled. Looking only at active food occurrences would miss that custody.
pub(crate) fn preserve_declarations(before: &PlanningContext, after: &PlanningContext)
    -> Result<(), DomainError>
{
    let selected = |context: &PlanningContext| context.weeks.iter()
        .flat_map(|week| week.plan.selections.iter()).cloned().collect::<Vec<_>>();
    let original = selected(before);
    let candidate = selected(after);
    for target in original.iter().filter(|selection| selection.portable_cold.is_some()) {
        require(candidate.iter().find(|selection| selection.slot == target.slot) == Some(target),
            ErrorCode::AuthorityDenied, "/proposal/selection/portable_cold")?;
        let BatchUse::Leftover {batch_id, ..} = &target.batch_use else {
            return Err(DomainError::new(ErrorCode::InvalidValue, "/portable_cold/batch_use"));
        };
        let sources = |selections: &[MealSelection]| selections.iter().filter(|selection|
            matches!(&selection.batch_use, BatchUse::NewBatch {batch_id: id, ..} if id == batch_id))
            .cloned().collect::<Vec<_>>();
        let old_sources = sources(&original);
        require(old_sources.len() == 1 && old_sources == sources(&candidate),
            ErrorCode::AuthorityDenied, "/proposal/selection/portable_cold/source")?;
        for slot in [&target.slot, &old_sources[0].slot] {
            require(requirement(before, slot)? == requirement(after, slot)?,
                ErrorCode::AuthorityDenied, "/proposal/selection/portable_cold/requirements")?;
        }
    }
    Ok(())
}
