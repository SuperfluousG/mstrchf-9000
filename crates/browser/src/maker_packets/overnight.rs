//! Exact first-service overnight definition; no inferred products or reuse.
use super::*;
use crate::personal_scope::OVERNIGHT_OATS_SCOPE;

pub(super) fn entry(compiled:&CompiledPacket)->bool {
    compiled.scope.recipe_id==OVERNIGHT_OATS_SCOPE.recipe_id
}
pub(super) fn evidence(record:&ReviewedPacketRecord,recipe:&RecipeRevision,reference:&RecipeRef,
    compiled:&CompiledPacket,policy:&Value,basis:&EvidenceBasis)
    ->Result<Vec<(u16,Vec<EquipmentCode>)>,DomainError> {
    let path="/maker/overnight/closure";
    let docs=record.evidence_documents.as_array().ok_or_else(||fail(ErrorCode::MissingReference,path))?;
    let method=unique_document(docs,compiled.method_id)?;
    let resource=unique_document(docs,compiled.resource_id)?;
    let quantity=unique_document(docs,"ev_800000000000000000000000000000e2")?;
    let inputs=unique_document(docs,"ev_800000000000000000000000000000e4")?;
    let storage=unique_document(docs,"ev_800000000000000000000000000000e7")?;
    let one=PositiveRational::new(1,1)?;
    if docs.len()!=8 || docs.iter().any(|doc|doc["actor"]!="automated" || doc["revision"]!="1"
            || doc["recipe_reference"]!=serde_json::json!(reference))
        || policy["root_exact_review_complete"]!=true
        || policy["accepted_source_captures_digest"]!=record.packet["source_captures_digest"]
        || method["full_method"]!=serde_json::json!(recipe.steps) || method["prep"]!=serde_json::json!(recipe.prep)
        || method["method_kind"]!="production" || method["method_prep_ids"]!=serde_json::json!([1,2])
        || method["measured"]!=false || method["physical_compliance_observed"]!=false
        || quantity["full_inputs"]!=serde_json::json!(recipe.ingredients)
        || quantity["full_yield"]!=serde_json::json!(recipe.r#yield)
        || quantity["source_quantity_conversion_claimed"]!=false || quantity["human_portion_calibration"]!=false
        || inputs["entries"]!=record.ingredient_registry["entries"]
        || inputs["ingredient_registry_digest"]!=record.packet["ingredient_registry_digest"]
        || inputs["actual_products_observed"]!=false || inputs["allergen_or_dietary_guarantee"]!=false
        || !recipe.dietary_evidence.is_empty() || !recipe.extras.is_empty() || !recipe.reused_inputs.is_empty()
        || record.packet["component_class_evidence"]!=serde_json::json!([])
        || recipe.components.len()!=1 || recipe.components[0].component_id.0!="co_80000000000000000000000000000014"
        || !matches!(&recipe.components[0].production.0,Some(ComponentProductionSpec::Unheated{first_handling_prep_id:1}))
        || recipe.storage.len()!=1 || recipe.storage[0].max_chilled_seconds!=86400 || recipe.storage[0].max_reheats!=0
        || storage["full_storage_rule"]!=serde_json::json!(recipe.storage[0])
        || storage["minimum_refrigerated_seconds"]!=18000 || storage["reuse_or_component_class_authority"]!=false
        || recipe.equipment.0!=vec![EquipmentCode::Fridge] || !recipe.portability.cold_storage_required
        || !recipe.portability.cold_suitable || recipe.portability.reheating_required
        || recipe.active_minutes!=10 || recipe.elapsed_minutes!=725 || recipe.prep.len()!=2
        || recipe.r#yield.servings!=one || recipe.r#yield.components.len()!=1
        || recipe.r#yield.components[0].quantity!=(Quantity{amount:one,unit:Unit::Each,form:IngredientForm::Prepared})
        || recipe.capabilities.leftovers || recipe.capabilities.starter_distribution
        || recipe.reviews.iter().any(|review|review.actor!=ReviewActor::Automated || review.kind==ReviewKind::CookTest)
        || recipe.nutrition.0.is_some() || recipe.image.0.is_some()
    {return Err(fail(ErrorCode::IdentityMismatch,path));}
    let amounts=[(45,1,Unit::G,IngredientForm::Raw),(120,1,Unit::Ml,IngredientForm::Prepared),
        (150,1,Unit::G,IngredientForm::Prepared),(80,1,Unit::G,IngredientForm::Drained),
        (1,2,Unit::Ml,IngredientForm::Prepared)];
    if recipe.ingredients.len()!=amounts.len(){return Err(fail(ErrorCode::MissingReference,path));}
    for (index,(ingredient,(n,d,unit,form))) in recipe.ingredients.iter().zip(amounts).enumerate() {
        if ingredient.ingredient_id.0!=format!("ig_800000000000000000000000000000{:02x}",0x30+index)
            || ingredient.quantity.0.as_ref()!=Some(&Quantity{amount:PositiveRational::new(n,d)?,unit,form})
            || !ingredient.conversion_evidence.is_empty() {
            return Err(fail(ErrorCode::IdentityMismatch,"/maker/overnight/quantity"));
        }
    }
    for (prep,offset) in recipe.prep.iter().zip([-43500,-300]) {
        if prep.active_minutes!=5 || prep.elapsed_minutes!=5 || prep.earliest_offset_seconds!=offset
            || prep.latest_offset_seconds!=offset || prep.can_attach_to_prior_session {
            return Err(fail(ErrorCode::IdentityMismatch,"/maker/overnight/fixed_phases"));
        }
    }
    let scope=&record.packet["operational_scope"];
    if scope["only_first_service"]!=true || scope["reuse_allowed"]!=false || scope["travel_allowed"]!=false
        || scope["minimum_continuous_chill_seconds"]!=18000 || scope["max_seconds_from_original_preparation"]!=86400
        || scope["service_local_weekday"]!="monday" || scope["attendee_count"]!=1
        || method["timing"]!=scope["timing"] || scope["timing"]["overnight_gap_seconds"]!=42900 {
        return Err(fail(ErrorCode::IdentityMismatch,"/maker/overnight/scope"));
    }
    let equipment=phase_map(record,recipe,reference,resource,policy,basis)?;
    if equipment!=vec![(1,vec![EquipmentCode::Fridge]),(2,vec![EquipmentCode::Fridge])] {
        return Err(fail(ErrorCode::IdentityMismatch,"/maker/overnight/resources"));
    }
    Ok(equipment)
}
