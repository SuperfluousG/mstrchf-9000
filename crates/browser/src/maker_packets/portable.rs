//! Structural closure for the exact portable-qualified source and cold service.
//! The parent first binds every document and recipe to compiled exact digests.
use super::*;
use crate::personal_scope::{PORTABLE_COLD_SCOPE, PORTABLE_HOT_SCOPE};

pub(super) fn entry(compiled: &CompiledPacket) -> bool {
    [
        PORTABLE_HOT_SCOPE.recipe_id,
        PORTABLE_COLD_SCOPE.recipe_id,
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
        .ok_or_else(|| fail(ErrorCode::InvalidValue, "/maker/portable/documents"))?;
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
        return Err(fail(ErrorCode::IdentityMismatch, "/maker/portable/method"));
    }
    let equipment = phase_map(record, recipe, reference, resource, policy, basis)?;
    let hot = compiled.scope.recipe_id == PORTABLE_HOT_SCOPE.recipe_id;
    let facts: Vec<ComponentClassEvidence> = if hot {
        let class_id = "ev_8000000000000000000000000000006a";
        let class = unique_document(docs, class_id)?;
        let storage = unique_document(docs, "ev_80000000000000000000000000000069")?;
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
                "/maker/portable/original",
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
            recipe_id: RecipeId(PORTABLE_HOT_SCOPE.recipe_id.into()),
            revision: Revision(PORTABLE_HOT_SCOPE.recipe_revision),
            digest: RecipeDigest(PORTABLE_HOT_SCOPE.recipe_digest.into()),
        };
        let reuse = unique_document(docs, "ev_80000000000000000000000000000070")?;
        let roles=serde_json::json!({"packing_prep_id":1,"service_prep_id":2});
        if method["method_kind"] != "production" || method["portable_cold"] != roles
            || record.packet["compiled_loader_inputs"]["method_kind"] != "production"
            || record.packet["compiled_loader_inputs"]["portable_cold"] != roles
            || policy["accepts"]["portable_cold"] != roles
            || ids != vec![1,2]
            || equipment != vec![(1,vec![EquipmentCode::Fridge]),(2,vec![])]
        {return Err(fail(ErrorCode::IdentityMismatch,"/maker/portable/phase_roles"));}
        if !recipe.ingredients.is_empty()
            || recipe.reused_inputs.len() != 1
            || !recipe.storage.is_empty()
            || recipe.reused_inputs[0].source_recipe != original
            || recipe.reused_inputs[0].source_component_id.0
                != "co_8000000000000000000000000000000d"
            || recipe.reused_inputs[0].planned_reheats != 0
            || reuse["reused_inputs"] != serde_json::json!(recipe.reused_inputs)
            || reuse["recipe_reference"] != serde_json::json!(reference)
            || reuse["source_recipe"] != serde_json::json!(original)
        {
            return Err(fail(
                ErrorCode::IdentityMismatch,
                "/maker/portable/cold_input",
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
            "/maker/portable/component_class",
        ));
    }
    Ok((
        ComponentClassView::new(reference.clone(), facts)?,
        equipment,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn exact_portable_pair_preserves_old_eleven_and_binds_original_clock_and_phases() {
        let refs=available_packet_refs().unwrap();
        assert_eq!(refs.len(),19);
        let old=[
            "edc4239b57366c96ef4b91198e4cc54de62db44d3d6346574e7318a852e2d726",
            "62949a9b43fb50ed7385271a22c09f9b5d703ff08ce865041b66c6d6f2d14dee",
            "ec3894c5071806462224a3ba282467b159b8a84ad05df8ae12995489784802ac",
            "e18d259dfa7941b893997dee7b378616457025ccac92021d54a2a43e25173932",
            "61aa9bc1b586ff82a1ceab2198f5cf88e7113ef82e37bdfa2b60cd4c7915de6d",
            "93df7d839d9076d377fb1c08b201c3048aff4a428093a1cf75fcf6cf919cebfd",
            "a6a189f045a8fe21617efb5fed082a43773071c6250c2b06bdda943e32395407",
            "e9675ef1aec183cf4d540b5c2c23856a1666f02bcf6b57080e8b845f1551209b",
            "8414f28d6683d7d4024911686034166de3de9d278b0759f1acfa29e6a3bed663",
            "b2c3301c26c57e5e0c12e3dfe467acacb5a3a0a87c6c4c4b46e6b427269a404e",
            "1ccb51875f250c48fe01456465f7ee3c5c2150cb0a05e25026d5ee9a232a1e16",
        ];
        for (reference,digest) in refs.iter().zip(old) {
            assert_eq!(reference.packet_digest.0,digest);
            let packet=known_packet(reference).unwrap();
            assert_eq!(canonical_digest(&packet.record().packet).unwrap().0,digest);
            assert_eq!(packet.portable_cold_method(),None);
        }
        let hot=known_packet(&refs[11]).unwrap();
        let cold=known_packet(&refs[12]).unwrap();
        assert_eq!(hot.reference(),&portable_hot_packet_ref().unwrap());
        assert_eq!(cold.reference(),&portable_cold_packet_ref().unwrap());
        assert_eq!(hot.recipe().ingredients.len(),7);
        assert_eq!(hot.recipe().storage[0].max_chilled_seconds,86400);
        assert_eq!(hot.recipe().storage[0].max_reheats,0);
        assert_eq!(hot.recipe().prep.len(),6);
        assert!(cold.recipe().ingredients.is_empty());
        assert_eq!(cold.recipe().reused_inputs[0].source_recipe,hot.reference().recipe);
        assert_eq!(cold.recipe().reused_inputs[0].source_component_id,hot.recipe().components[0].component_id);
        assert_eq!(cold.recipe().reused_inputs[0].planned_reheats,0);
        assert_eq!(cold.portable_cold_method(),Some(mstrchf_core::food::PortableColdMethod {packing_prep_id:1,service_prep_id:2}));
        assert_eq!(cold.phase_equipment(1).unwrap(),&[EquipmentCode::Fridge]);
        assert!(cold.phase_equipment(2).unwrap().is_empty());
        let mut forged=cold.record().clone();
        forged.packet["compiled_loader_inputs"]["portable_cold"]["packing_prep_id"]=serde_json::json!(2);
        assert_eq!(known_record(&forged).unwrap_err().code,ErrorCode::DigestMismatch);
    }
}
