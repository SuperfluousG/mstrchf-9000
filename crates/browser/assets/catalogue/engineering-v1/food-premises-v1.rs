// Frozen host-owned synthetic engineering-v1 premises, captured 12 October 2026.
// This is an include-only trusted fixture asset, never incoming authority.
// No food arithmetic is implemented here; the unchanged core performs derivation.
pub(crate) fn evidence_inputs(r: &RecipeRevision) -> Result<(EvidenceBasis, ComponentClassView), DomainError> {
    let rf = crate::development::recipe_reference(r)?;
    crate::catalogue::legacy_engineering_v1()?.resolve_recipe(&rf)?;
    let pins=(1..=8).map(|n| {let id=EvidenceId(format!("ev_{n:032x}"));Ok(EvidencePin{evidence_id:id.clone(),revision:Revision(1),digest:canonical_digest(&serde_json::json!({"fixture":"m1-host-owned-synthetic-v1","recipe":rf,"evidence_id":id,"statement":"Synthetic engineering premise only; no production food or rights admission."}))?})}).collect::<Result<Vec<_>,DomainError>>()?;
    Ok((
        EvidenceBasis::new(rf.clone(), pins, r.image.0.iter().map(|v|v.digest.clone()).collect())?,
        ComponentClassView::new(
            rf.clone(),
            vec![ComponentClassEvidence {
                recipe: rf,
                component_id: r.components[0].component_id.clone(),
                class: ComponentClass::General,
                evidence_ids: vec![EvidenceId(format!("ev_{:032x}", 8))],
            }],
        )?,
    ))
}

pub(crate) fn evidence_for_refs(refs: &[RecipeRef], recipes: &[RecipeRevision], inputs: &[(EvidenceBasis, ComponentClassView)]) -> Result<FoodEvidence, DomainError> {
    let view = crate::catalogue::legacy_engineering_v1()?;
    for reference in refs { view.resolve_recipe(reference)?; }
    let mut evidence=FoodEvidence{catalogue_revision:Revision(1),catalogue_digest:PayloadDigest("0".repeat(64)),policy_version:Revision(1),recipes:vec![],classes:vec![],methods:vec![],resources:vec![],essential_ingredient_ids:if refs.is_empty(){vec![]}else{vec![IngredientId(format!("ig_{:032x}",1))]}};
    let ev=vec![EvidenceId(format!("ev_{:032x}",8))];
    for rf in refs {
        let index=recipes.iter().position(|r|crate::development::recipe_reference(r).as_ref()==Ok(rf)).ok_or_else(||error(ErrorCode::MissingReference,"/recipes"))?;
        let r=&recipes[index];evidence.recipes.push(inputs[index].0.clone());evidence.classes.push(inputs[index].1.clone());
        evidence.methods.push(MethodPhases{recipe:rf.clone(),kind:MethodKind::Production,prep_ids:r.prep.iter().map(|v|v.prep_id).collect(),direct_reuse_counts:vec![],evidence_ids:ev.clone()});
        evidence.methods.push(MethodPhases{recipe:rf.clone(),kind:MethodKind::Reuse,prep_ids:r.prep.iter().map(|v|v.prep_id).collect(),direct_reuse_counts:r.components.iter().map(|v|ComponentReheatCount{recipe:rf.clone(),component_id:v.component_id.clone(),planned_reheats:1,evidence_ids:ev.clone()}).collect(),evidence_ids:ev.clone()});
        for prep in &r.prep.0 {evidence.resources.push(PhaseResources{recipe:rf.clone(),prep_id:prep.prep_id,equipment:r.equipment.0.clone(),evidence_ids:ev.clone()});}
    }
    evidence.catalogue_digest=food_evidence_digest(&evidence)?;
    Ok(evidence)
}
