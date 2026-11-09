//! Structural closure for the independently reviewed original chilli family.
//! The parent binds all bytes to the exact compiled recipe/document digests.
use super::*;
use crate::personal_scope::{CHILLI_BATCH_SCOPE, CHILLI_WRAPS_SCOPE, CHILLI_POTATO_SCOPE, CHILLI_COMPONENT_ID};

pub(super) fn entry(compiled: &CompiledPacket) -> bool {
    [CHILLI_BATCH_SCOPE.recipe_id, CHILLI_WRAPS_SCOPE.recipe_id, CHILLI_POTATO_SCOPE.recipe_id]
        .contains(&compiled.scope.recipe_id)
}

pub(super) fn evidence(record: &ReviewedPacketRecord, recipe: &RecipeRevision,
    reference: &RecipeRef, compiled: &CompiledPacket, policy: &Value, basis: &EvidenceBasis)
    -> Result<(ComponentClassView, Vec<(u16, Vec<EquipmentCode>)>), DomainError> {
    let path = "/maker/chilli/closure";
    let docs = record.evidence_documents.as_array().ok_or_else(|| fail(ErrorCode::MissingReference,path))?;
    let source = compiled.scope.recipe_id == CHILLI_BATCH_SCOPE.recipe_id;
    let wraps = compiled.scope.recipe_id == CHILLI_WRAPS_SCOPE.recipe_id;
    let rf = serde_json::json!(reference);
    let method = unique_document(docs,compiled.method_id)?;
    let resource = unique_document(docs,compiled.resource_id)?;
    let ingredient_id = if source {"ev_800000000000000000000000000000cb"}
        else if wraps {"ev_800000000000000000000000000000d4"} else {"ev_800000000000000000000000000000dc"};
    let quantity_id = if source {"ev_800000000000000000000000000000c9"}
        else if wraps {"ev_800000000000000000000000000000d2"} else {"ev_800000000000000000000000000000da"};
    let inputs = unique_document(docs,ingredient_id)?;
    let quantities = unique_document(docs,quantity_id)?;
    if docs.iter().any(|doc| doc["actor"] != "automated" || doc["revision"] != "1" || doc["recipe_reference"] != rf)
        || policy["root_exact_review_complete"] != true
        || policy["accepted_source_captures_digest"] != record.packet["source_captures_digest"]
        || method["full_method"] != serde_json::json!(recipe.steps)
        || method["prep"] != serde_json::json!(recipe.prep)
        || method["method_prep_ids"] != serde_json::json!(recipe.prep.iter().map(|prep|prep.prep_id).collect::<Vec<_>>())
        || method["method_kind"] != "production" || method["measured"] != false
        || method["physical_compliance_observed"] != false
        || record.packet["compiled_loader_inputs"]["method_kind"] != "production"
        || quantities["full_inputs"] != serde_json::json!(recipe.ingredients)
        || quantities["full_yield"] != serde_json::json!(recipe.r#yield)
        || quantities["source_servings_inherited"] != false
        || quantities.get("measured_cooked_yield") != Some(&Value::Null) || quantities["human_portion_calibration"] != false
        || inputs["ingredient_registry_digest"] != record.packet["ingredient_registry_digest"]
        || inputs["actual_products_observed"] != false || inputs["allergen_or_dietary_guarantee"] != false
        || !recipe.dietary_evidence.is_empty() || !recipe.extras.is_empty()
    {return Err(fail(ErrorCode::IdentityMismatch,path));}
    let equipment = phase_map(record,recipe,reference,resource,policy,basis)?;
    let facts = if source {
        let class_id = "ev_800000000000000000000000000000cf";
        let class = unique_document(docs,class_id)?;
        let storage = unique_document(docs,"ev_800000000000000000000000000000ce")?;
        if recipe.ingredients.len() != 12 || recipe.components.len() != 1
            || recipe.components[0].component_id.0 != CHILLI_COMPONENT_ID
            || recipe.storage.len() != 1 || !recipe.reused_inputs.is_empty()
            || !matches!(&recipe.components[0].production.0,Some(ComponentProductionSpec::CookedWithCooling {
                completion_prep_id:3,cooling_prep_ids,max_cooling_seconds:7200}) if cooling_prep_ids.0==vec![4,5,6])
            || recipe.storage[0].max_chilled_seconds != 172800 || recipe.storage[0].max_reheats != 1
            || storage["full_storage_rule"] != serde_json::json!(recipe.storage[0])
            || storage["per_distinct_original_allocation_reheat_limit"] != 1
            || storage["full_reserved_batch_reheat_forbidden"] != true
            || class["component_id"] != CHILLI_COMPONENT_ID || class["class"] != "general"
            || !basis.admitted.iter().any(|pin|pin.evidence_id.0==class_id)
            || equipment != vec![(1,vec![EquipmentCode::Hob]),(2,vec![EquipmentCode::Hob]),
                (3,vec![EquipmentCode::Hob]),(4,vec![]),(5,vec![]),(6,vec![EquipmentCode::Fridge])]
        {return Err(fail(ErrorCode::IdentityMismatch,"/maker/chilli/source"));}
        vec![ComponentClassEvidence {recipe:reference.clone(),component_id:recipe.components[0].component_id.clone(),
            class:ComponentClass::General,evidence_ids:vec![EvidenceId(class_id.into())]}]
    } else {
        let source_ref = RecipeRef {recipe_id:RecipeId(CHILLI_BATCH_SCOPE.recipe_id.into()),
            revision:Revision(CHILLI_BATCH_SCOPE.recipe_revision),digest:RecipeDigest(CHILLI_BATCH_SCOPE.recipe_digest.into())};
        let reuse_id = if wraps {"ev_800000000000000000000000000000d7"} else {"ev_800000000000000000000000000000df"};
        let reuse = unique_document(docs,reuse_id)?;
        let input = crate::personal_scope::chilli_service_input(recipe)?;
        let dependency = &record.packet["declared_source_dependency"];
        if input.source_recipe != source_ref
            || reuse["reused_inputs"] != serde_json::json!(recipe.reused_inputs)
            || reuse["original_batch_recipe_reference"] != serde_json::json!(source_ref)
            || reuse["original_component_id"] != CHILLI_COMPONENT_ID
            || reuse["source_inputs_repeated_as_fresh_demand"] != false
            || reuse["per_allocation_reheats"] != 1
            || reuse["local_fresh_inputs"] != serde_json::json!(recipe.ingredients)
            || reuse["output_is_operational_each_not_combined_mass"] != true
            || reuse["output_storage_or_reuse_authority"] != false
            || dependency["admission_order"] != "source_first_then_service"
            || dependency["source_recipe_reference"] != serde_json::json!(source_ref)
            || dependency["source_packet_digest"] != CHILLI_BATCH_SCOPE.packet_canonical_digest
            || dependency["source_component_id"] != CHILLI_COMPONENT_ID
            || policy["accepted_dependency"] != *dependency
        {return Err(fail(ErrorCode::IdentityMismatch,"/maker/chilli/reused_share"));}
        vec![]
    };
    let declared: Vec<_> = facts.iter().map(|fact|serde_json::json!({"recipe":fact.recipe,
        "component_id":fact.component_id,"class":"general","evidence_ids":fact.evidence_ids})).collect();
    if record.packet["component_class_evidence"] != serde_json::json!(declared) {
        return Err(fail(ErrorCode::IdentityMismatch,"/maker/chilli/component_class"));
    }
    Ok((ComponentClassView::new(reference.clone(),facts)?,equipment))
}
