//! Structural closure for the two separately adopted original-share records.
//! The parent first binds every document and recipe to compiled exact digests.
use super::*;
use crate::personal_scope::{COURGETTE_COLD_SCOPE, COURGETTE_HOT_SCOPE};

pub(super) fn entry(compiled: &CompiledPacket) -> bool {
    [
        COURGETTE_HOT_SCOPE.recipe_id,
        COURGETTE_COLD_SCOPE.recipe_id,
    ]
    .contains(&compiled.scope.recipe_id)
}

pub(super) fn evidence(
    record: &ReviewedPacketRecord,
    recipe: &RecipeRevision,
    reference: &RecipeRef,
    compiled: &CompiledPacket,
    policy: &Value,
    basis: &EvidenceBasis,
) -> Result<(ComponentClassView, Vec<(u16, Vec<EquipmentCode>)>), DomainError> {
    let docs = record
        .evidence_documents
        .as_array()
        .ok_or_else(|| fail(ErrorCode::InvalidValue, "/maker/courgette/documents"))?;
    let method = unique_document(docs, compiled.method_id)?;
    let resource = unique_document(docs, compiled.resource_id)?;
    let ids: Vec<_> = recipe.prep.iter().map(|prep| prep.prep_id).collect();
    if method["recipe_reference"] != serde_json::json!(reference)
        || method["full_method"] != serde_json::json!(recipe.steps)
        || method["prep"] != serde_json::json!(recipe.prep)
        || method["method_prep_ids"] != serde_json::json!(ids)
        || policy["root_exact_review_complete"] != true
        || policy["accepted_source_captures_digest"] != record.packet["source_captures_digest"]
    {
        return Err(fail(ErrorCode::IdentityMismatch, "/maker/courgette/method"));
    }
    let equipment = phase_map(record, recipe, reference, resource, policy, basis)?;
    let hot = compiled.scope.recipe_id == COURGETTE_HOT_SCOPE.recipe_id;
    let facts: Vec<ComponentClassEvidence> = if hot {
        let class_id = "ev_8000000000000000000000000000005b";
        let class = unique_document(docs, class_id)?;
        let storage = unique_document(docs, "ev_8000000000000000000000000000005a")?;
        if recipe.components.len() != 1
            || recipe.storage.len() != 1
            || !recipe.reused_inputs.is_empty()
            || !matches!(&recipe.components[0].production.0,
                Some(ComponentProductionSpec::CookedWithCooling { completion_prep_id:3,
                    cooling_prep_ids, max_cooling_seconds:7200 }) if cooling_prep_ids.0==vec![4,5,6])
            || recipe.storage[0].max_chilled_seconds != 86400
            || recipe.storage[0].max_reheats != 0
            || storage["full_storage_rule"] != serde_json::json!(recipe.storage[0])
            || class["recipe_reference"] != serde_json::json!(reference)
            || class["component_id"] != serde_json::json!(recipe.components[0].component_id)
            || class["class"] != "general"
            || !basis
                .admitted
                .iter()
                .any(|pin| pin.evidence_id.0 == class_id)
        {
            return Err(fail(
                ErrorCode::IdentityMismatch,
                "/maker/courgette/original",
            ));
        }
        vec![ComponentClassEvidence {
            recipe: reference.clone(),
            component_id: recipe.components[0].component_id.clone(),
            class: ComponentClass::General,
            evidence_ids: vec![EvidenceId(class_id.into())],
        }]
    } else {
        let original = RecipeRef {
            recipe_id: RecipeId(COURGETTE_HOT_SCOPE.recipe_id.into()),
            revision: Revision(COURGETTE_HOT_SCOPE.recipe_revision),
            digest: RecipeDigest(COURGETTE_HOT_SCOPE.recipe_digest.into()),
        };
        let reuse = unique_document(docs, "ev_80000000000000000000000000000061")?;
        if !recipe.ingredients.is_empty()
            || recipe.reused_inputs.len() != 1
            || !recipe.storage.is_empty()
            || recipe.reused_inputs[0].source_recipe != original
            || recipe.reused_inputs[0].source_component_id.0
                != "co_8000000000000000000000000000000b"
            || recipe.reused_inputs[0].planned_reheats != 0
            || reuse["reused_inputs"] != serde_json::json!(recipe.reused_inputs)
            || reuse["recipe_reference"] != serde_json::json!(reference)
            || reuse["source_recipe"] != serde_json::json!(original)
        {
            return Err(fail(
                ErrorCode::IdentityMismatch,
                "/maker/courgette/cold_input",
            ));
        }
        vec![]
    };
    let declared: Vec<_> = facts.iter().map(|fact| serde_json::json!({
        "recipe":fact.recipe,"component_id":fact.component_id,
        "class":"general","evidence_ids":fact.evidence_ids
    })).collect();
    if record.packet["component_class_evidence"] != serde_json::json!(declared) {
        return Err(fail(
            ErrorCode::IdentityMismatch,
            "/maker/courgette/component_class",
        ));
    }
    Ok((
        ComponentClassView::new(reference.clone(), facts)?,
        equipment,
    ))
}
