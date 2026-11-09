//! Exact closure for the separately reviewed conditional mixed-diet pair.
//! Parent loading first binds every transported byte to Root's compiled pins.
use super::*;
use crate::personal_scope::{THREE_BEAN_GF_CHICKEN_SCOPE, CHILLED_GF_CHICKEN_SCOPE, GF_CHICKEN_EXTRA_ID};

pub(super) fn entry(compiled: &CompiledPacket) -> bool {
    [THREE_BEAN_GF_CHICKEN_SCOPE.recipe_id, CHILLED_GF_CHICKEN_SCOPE.recipe_id]
        .contains(&compiled.scope.recipe_id)
}

pub(super) fn evidence(record: &ReviewedPacketRecord, recipe: &RecipeRevision,
    reference: &RecipeRef, compiled: &CompiledPacket, policy: &Value,
    basis: &EvidenceBasis) -> Result<Vec<(u16,Vec<EquipmentCode>)>,DomainError> {
    let path="/maker/gf_pair/closure";
    let docs=record.evidence_documents.as_array().ok_or_else(||fail(ErrorCode::MissingReference,path))?;
    let diet_id=record.packet["compiled_loader_inputs"]["dietary_review_id"].as_str()
        .ok_or_else(||fail(ErrorCode::MissingReference,path))?;
    let input_id=record.packet["compiled_loader_inputs"]["input_review_id"].as_str()
        .ok_or_else(||fail(ErrorCode::MissingReference,path))?;
    let diet=unique_document(docs,diet_id)?;
    let input=unique_document(docs,input_id)?;
    let method=unique_document(docs,compiled.method_id)?;
    let resource=unique_document(docs,compiled.resource_id)?;
    let table=&record.packet["intrinsic_input_conditions"];
    let table_digest=canonical_digest(table)?;
    let map_digest=canonical_digest(&record.packet["phase_resource_map"])?;
    let dietary_pin=serde_json::json!({"evidence_id":diet_id,"revision":"1","digest":canonical_digest(diet)?});
    let policy_pin=serde_json::json!({"evidence_id":compiled.policy_id,"revision":"1","digest":canonical_digest(policy)?});
    let ids=serde_json::json!(recipe.ingredients.iter().map(|ingredient|&ingredient.ingredient_id).collect::<Vec<_>>());
    let prep_ids=serde_json::json!(recipe.prep.iter().map(|prep|prep.prep_id).collect::<Vec<_>>());
    let step_ids=serde_json::json!(recipe.steps.iter().map(|step|step.step_id).collect::<Vec<_>>());
    let rf=serde_json::json!(reference);
    if docs.iter().any(|doc|doc["actor"]!="automated" || doc["revision"]!="1"
        || doc.get("recipe_reference").is_some_and(|value|value!=&rf))
        || policy["root_exact_review_complete"]!=true
        || table["format"]!="mstrchf-recipe-intrinsic-input-conditions" || table["version"]!=1
        || table["recipe_reference"]!=rf || table["dietary_review_id"]!=diet_id
        || table["maker_policy_id"]!=compiled.policy_id
        || table["full_method_step_ids"]!=step_ids || table["full_method_prep_ids"]!=prep_ids
        || table["applies_even_when_all_consumers_confirmed_none"]!=true
        || table["product_and_operational_compliance_observed"]!=false
        || diet["coverage"]!=ids || diet["full_method"]!=serde_json::json!(recipe.steps)
        || diet["facts"]!=serde_json::json!(recipe.dietary_evidence)
        || diet["conditional_recipe_inference_only"]!=true
        || diet["actual_products_and_processing_observed"]!=false || diet["physical_handling_observed"]!=false
        || method["full_method"]!=serde_json::json!(recipe.steps) || method["phases"]!=serde_json::json!(recipe.prep)
        || method["production_method_prep_ids"]!=prep_ids
        || input["ingredient_ids"]!=ids || input["ingredient_registry_digest"]!=record.packet["ingredient_registry_digest"]
        || input["actual_product_qualification"]!=false
        || [table, diet, input, method, resource, policy].iter().any(|doc|doc["recipe_reference"]!=rf)
        || record.packet["intrinsic_input_conditions_digest"]!=serde_json::json!(table_digest)
        || diet["intrinsic_input_conditions_digest"]!=serde_json::json!(table_digest)
        || input["intrinsic_input_conditions_digest"]!=serde_json::json!(table_digest)
        || policy["accepted_intrinsic_input_conditions_digest"]!=serde_json::json!(table_digest)
        || policy["accepted_dietary_review_pin"]!=dietary_pin
        || policy["accepted_source_captures_digest"]!=record.packet["source_captures_digest"]
        || record.packet["intrinsic_input_custody"]!=serde_json::json!({"recipe_reference":reference,
            "intrinsic_input_conditions_digest":table_digest,"phase_resource_map_digest":map_digest,
            "dietary_review_pin":dietary_pin,"maker_policy_pin":policy_pin}) {
        return Err(fail(ErrorCode::IdentityMismatch,path));
    }
    let extra=compiled.scope.recipe_id==CHILLED_GF_CHICKEN_SCOPE.recipe_id;
    let rows=table["entries"].as_array().ok_or_else(||fail(ErrorCode::MissingReference,path))?;
    let registry=record.ingredient_registry["entries"].as_array().ok_or_else(||fail(ErrorCode::MissingReference,path))?;
    if rows.len()!=recipe.ingredients.len() || registry.len()!=10 {return Err(fail(ErrorCode::IdentityMismatch,path));}
    for (row,ingredient) in rows.iter().zip(recipe.ingredients.iter()) {
        let quantity=ingredient.quantity.0.as_ref().ok_or_else(||fail(ErrorCode::MissingReference,path))?;
        let definition=registry.iter().find(|entry|entry["ingredient_id"]==serde_json::json!(ingredient.ingredient_id))
            .ok_or_else(||fail(ErrorCode::MissingReference,path))?;
        let predicates=row["predicates"].as_array().ok_or_else(||fail(ErrorCode::MissingReference,path))?;
        if row["ingredient_id"]!=serde_json::json!(ingredient.ingredient_id)
            || row["recipe_input_name"]!=serde_json::json!(ingredient.name)
            || row["input_condition"]!=definition["meaning"]
            || row["form"]!=serde_json::json!(quantity.form) || row["unit"]!=serde_json::json!(quantity.unit)
            || !predicates.iter().any(|value|value["kind"]=="gluten_free_compatible_input_and_processing"
                && value["requirement"]==serde_json::json!(HardRequirement::Diet{code:DietCode::GlutenFree})
                && value["finished_food_benchmark_mg_per_kg"]==20)
            || !extra && !predicates.iter().any(|value|value["kind"]=="vegetarian_compatible_input_composition") {
            return Err(fail(ErrorCode::IdentityMismatch,"/maker/gf_pair/input"));
        }
    }
    for fact in recipe.dietary_evidence.iter() {
        let doc=unique_document(docs,&fact.evidence_id.0)?;
        if fact.review_id.0!=diet_id || doc["review_id"]!=diet_id
            || doc["requirement"]!=serde_json::json!(fact.requirement)
            || doc["verdict"]!=serde_json::json!(fact.verdict) || doc["source"]!=serde_json::json!(fact.source) {
            return Err(fail(ErrorCode::IdentityMismatch,"/maker/gf_pair/dietary_fact"));
        }
    }
    let supported=|code| recipe.dietary_evidence.iter().any(|row|
        row.requirement==HardRequirement::Diet{code} && row.verdict==DietaryVerdict::Supported);
    let equipment=phase_map(record,recipe,reference,resource,policy,basis)?;
    if !supported(DietCode::GlutenFree) || !recipe.storage.is_empty() || !recipe.reused_inputs.is_empty()
        || recipe.equipment.0!=vec![EquipmentCode::Fridge] || !recipe.portability.cold_storage_required {
        return Err(fail(ErrorCode::IdentityMismatch,"/maker/gf_pair/capability"));
    }
    if extra {
        if recipe.source.origin!=RecipeSourceOrigin::UserAuthored || recipe.source.url.0.is_some()
            || recipe.text_rights.distribution!=RightsDistribution::PersonalOnly
            || recipe.text_rights.licence_url.0.is_some() || !recipe.extras.is_empty()
            || recipe.ingredients.len()!=1 || recipe.ingredients[0].ingredient_id.0!="ig_80000000000000000000000000000021"
            || recipe.ingredients[0].quantity.0.as_ref()!=Some(&Quantity{amount:PositiveRational::new(50,1)?,unit:Unit::G,form:IngredientForm::Cooked})
            || recipe.dietary_evidence.len()!=3 || equipment!=vec![(1,vec![])]
            || [DietCode::Vegetarian,DietCode::Vegan].into_iter().any(|code|!recipe.dietary_evidence.iter().any(|fact|
                fact.requirement==HardRequirement::Diet{code} && fact.verdict==DietaryVerdict::Conflicting)) {
            return Err(fail(ErrorCode::IdentityMismatch,"/maker/gf_pair/extra"));
        }
    } else {
        let child=serde_json::json!({"recipe_id":CHILLED_GF_CHICKEN_SCOPE.recipe_id,
            "revision":"1","digest":CHILLED_GF_CHICKEN_SCOPE.recipe_digest});
        let dependency=&record.packet["declared_extra_dependency"];
        if recipe.ingredients.len()!=9 || recipe.dietary_evidence.len()!=2 || !supported(DietCode::Vegetarian)
            || recipe.extras.len()!=1 || serde_json::json!(recipe.extras[0].recipe)!=child
            || recipe.extras[0].extra_id.0!=GF_CHICKEN_EXTRA_ID || !recipe.extras[0].eligible_person_requirements.is_empty()
            || recipe.extras[0].supported_with.0!=vec![ComponentId("co_8000000000000000000000000000000f".into())]
            || dependency["admission_order"]!="extra_first_then_base"
            || dependency["base_without_selected_extra_still_requires_dependency"]!=true
            || dependency["extra_reference"]!=child || dependency["extra_packet_digest"]!=CHILLED_GF_CHICKEN_SCOPE.packet_canonical_digest
            || policy["accepted_dependency"]!=*dependency
            || equipment!=vec![(1,vec![EquipmentCode::Fridge]),(2,vec![EquipmentCode::Fridge])] {
            return Err(fail(ErrorCode::IdentityMismatch,"/maker/gf_pair/base"));
        }
    }
    Ok(equipment)
}
