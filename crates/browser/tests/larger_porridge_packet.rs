//! Isolated native derivation of the fixed authored Larger porridge packet.
//! This records maker-reviewed bytes, not runtime catalogue/admission authority,
//! human portion/cooking calibration, actual products or a new scaling engine.
use mstrchf_contracts::*;
use mstrchf_core::{eligibility::{eligible, Eligibility}, portions, recipes::*};
use serde_json::{Value, json};
use std::{fs, io::Write, path::PathBuf};

fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../sites/mstrchf-9000/content/intake/porridge-larger-reviewed-v1")
}
fn read(name: &str) -> Value {
    serde_json::from_slice(&fs::read(folder().join(name)).unwrap()).unwrap()
}
fn prior(name: &str) -> Value {
    serde_json::from_slice(&fs::read(folder().join("../porridge-personal-reviewed-v1").join(name)).unwrap()).unwrap()
}
fn reference(recipe: &RecipeRevision) -> RecipeRef {
    RecipeRef { recipe_id:recipe.recipe_id.clone(), revision:recipe.revision,
        digest:recipe_digest(recipe).unwrap() }
}
fn pins(documents: &Value) -> Vec<EvidencePin> {
    documents.as_array().unwrap().iter().map(|document| EvidencePin {
        evidence_id:parse_document(&canonical_bytes(&document["evidence_id"]).unwrap()).unwrap(),
        revision:parse_document(&canonical_bytes(&document["revision"]).unwrap()).unwrap(),
        digest:canonical_digest(document).unwrap(),
    }).collect()
}
fn basis(recipe: &RecipeRevision, documents: &Value) -> EvidenceBasis {
    EvidenceBasis::new(reference(recipe),pins(documents),vec![]).unwrap()
}
fn assess(recipe: &RecipeRevision, documents: &Value) -> RecipeCapabilities {
    let evidence=basis(recipe,documents);
    let classes=ComponentClassView::new(reference(recipe),vec![]).unwrap();
    assess_revision(recipe,&evidence,&classes).unwrap().assessment().capabilities.clone()
}
fn final_values() -> (Value, Value, Value, RecipeRevision) {
    let input=read("authoring-input.json");
    assert_eq!(input["native_jcs_derived"],false);
    assert_eq!(input["capability_seed_unassessed"],true);
    let mut packet=input["packet"].clone();
    let mut documents=input["evidence_documents"].clone();
    let registry=read("ingredient-registry.json");
    let captures=read("source-captures.json");
    let mut recipe:RecipeRevision=parse_document(&canonical_bytes(&packet["maker_recipe_revision"]).unwrap()).unwrap();
    // Imported capability bits are only claims. Native assessment computes the
    // actual diagnostics before deriving the final immutable recipe reference.
    recipe.capabilities=assess(&recipe,&documents);
    let rf=reference(&recipe);
    packet["maker_recipe_revision"]=json!(recipe);
    packet["recipe_reference"]=json!(rf);
    packet["compiled_loader_inputs"]["recipe_reference"]=json!(rf);
    packet["status"]=json!("maker_reviewed_native_jcs_ready_not_registry_or_household_runtime_admitted");
    packet["native_contracts_and_core_assessed"]=json!(true);
    let registry_digest=canonical_digest(&registry).unwrap();
    let captures_digest=canonical_digest(&captures).unwrap();
    for document in documents.as_array_mut().unwrap() {document["recipe_reference"]=json!(rf);}
    documents[1]["ingredient_registry_digest"]=json!(registry_digest);
    documents[2]["ingredient_registry_digest"]=json!(registry_digest);
    documents[3]["registry_digest"]=json!(registry_digest);
    let policy=documents.as_array_mut().unwrap().last_mut().unwrap();
    assert_eq!(policy["evidence_id"],"ev_8000000000000000000000000000003d");
    policy["accepted_ingredient_registry_digest"]=json!(registry_digest);
    policy["accepted_source_captures_digest"]=json!(captures_digest);
    assert_eq!(policy["accepts"]["operational_scope"],packet["operational_scope"]);
    let evidence=pins(&documents);
    packet["maker_evidence_pins"]=json!(evidence.iter().map(|pin|json!({
        "evidence_id":pin.evidence_id,"revision":pin.revision,"digest":pin.digest})).collect::<Vec<_>>());
    packet["ingredient_registry_digest"]=json!(registry_digest);
    packet["source_captures_digest"]=json!(captures_digest);
    packet["evidence_documents_digest"]=json!(canonical_digest(&documents).unwrap());
    assert_eq!(assess(&recipe,&documents),recipe.capabilities);
    let mut clocks:Vec<Instant>=documents.as_array().unwrap().iter()
        .map(|document|parse_document(&canonical_bytes(&document["at"]).unwrap()).unwrap()).collect();
    clocks.extend(recipe.reviews.iter().map(|r|parse_document::<Instant>(&canonical_bytes(&r.at).unwrap()).unwrap()));
    let not_before=clocks.into_iter().max().unwrap();
    let digests=json!({"format":"mstrchf-rust-jcs-maker-packet-digests","recipe_reference":rf,
        "packet_canonical_digest":canonical_digest(&packet).unwrap(),
        "ingredient_registry_digest":registry_digest,"source_captures_digest":captures_digest,
        "evidence_documents_digest":canonical_digest(&documents).unwrap(),
        "root_policy_evidence_id":evidence[10].evidence_id,"root_policy_evidence_digest":evidence[10].digest,
        "method_evidence_id":evidence[4].evidence_id,"method_evidence_digest":evidence[4].digest,
        "resource_evidence_id":evidence[4].evidence_id,"resource_evidence_digest":evidence[4].digest,
        "ingredient_mapping_evidence_id":evidence[3].evidence_id,"ingredient_mapping_evidence_digest":evidence[3].digest,
        "not_before":not_before,"derivation":"Rust mstrchf_contracts recipe_digest/canonical_digest (serde_jcs) and mstrchf_core assess_revision",
        "household_runtime_admission":false,"registry_entry_present":false});
    (packet,documents,digests,recipe)
}
fn verify_previous_bytes() {
    let preservation=read("previous-packet-preservation.json");
    assert!(preservation["files"].as_array().unwrap().len()>=20);
    for item in preservation["files"].as_array().unwrap() {
        let directory=item["directory"].as_str().unwrap();let filename=item["filename"].as_str().unwrap();
        let bytes=fs::read(folder().join("..").join(directory).join(filename)).unwrap();
        assert_eq!(json!(bytes.len()),item["bytes"]);
        assert_eq!(json!(bytes_digest(&bytes)),item["sha256_bytes"],"Prior {directory}/{filename}");
    }
    assert_eq!(fs::read(folder().join("source-captures.json")).unwrap(),
        fs::read(folder().join("../porridge-personal-reviewed-v1/source-captures.json")).unwrap());
    assert_eq!(read("ingredient-registry.json")["entries"],prior("ingredient-registry.json")["entries"]);
}
fn write_new_or_equal(name: &str, value: &Value) {
    let path=folder().join(name);
    if path.exists() {assert_eq!(read(name),*value,"Refuse replacing differing immutable output {name}");return;}
    let mut file=fs::OpenOptions::new().create_new(true).write(true).open(path).unwrap();
    file.write_all(&serde_json::to_vec_pretty(value).unwrap()).unwrap();file.write_all(b"\n").unwrap();
}
fn meal() -> MealRequirements {
    let mut m=parse_document::<MealRequirementsDocumentV1>(include_bytes!("../../../tests/fixtures/contracts/meal-requirements.json")).unwrap().meal_requirements;
    m.slot.meal_type=MealType::Breakfast;m.attendees.0.truncate(1);
    m.attendees[0].age_band=AgeBand::Adult;m.attendees[0].hard_requirements=BoundedVec(vec![]);
    m.attendees[0].portion=PortionPreference {preset:PortionPresetName::Larger,
        adjustment:PositiveRational::new(1,1).unwrap(),meal_overrides:BoundedVec(vec![])};
    m.shared_hard_requirements=BoundedVec(vec![]);m.equipment=BoundedVec(vec![EquipmentCode::Hob]);
    m.location=MealLocation::AtHome;m.mobility.temperature=MobilityTemperature::Either;m.enabled=true;m
}
fn empty_overrides() -> MealOverrides {MealOverrides {portions:BoundedVec(vec![]),eat_by:Nullable(None)}}

#[test]
fn derive_or_verify_larger_packet_exact_native_jcs_capabilities_and_custody() {
    verify_previous_bytes();
    let (packet,documents,digests,recipe)=final_values();let rf=reference(&recipe);
    assert_eq!(rf.recipe_id.0,"rc_80000000000000000000000000000007");assert_eq!(recipe.revision,Revision(1));
    assert_eq!(recipe.components[0].component_id.0,"co_80000000000000000000000000000007");
    assert_eq!(documents.as_array().unwrap().len(),11);
    for (document,suffix) in documents.as_array().unwrap().iter().zip(["33","34","35","36","37","38","39","3a","3b","3c","3d"]) {
        assert_eq!(document["evidence_id"],format!("ev_800000000000000000000000000000{suffix}"));
        assert_eq!(document["actor"],"automated");assert_eq!(document["recipe_reference"],json!(rf));
        assert_eq!(document["at"],"2026-10-13T02:40:31Z");
    }
    assert_eq!(digests["not_before"],"2026-10-13T02:40:31Z");
    assert_eq!(recipe.source.retrieved_at,parse_document::<Instant>(br#""2026-10-12T16:32:09Z""#).unwrap());
    assert!(recipe.reviews.iter().all(|r|r.actor==ReviewActor::Automated&&r.kind!=ReviewKind::CookTest));
    assert_eq!(recipe.reviews.len(),3);assert_eq!(documents[0]["fresh_network_retrieval"],false);
    assert_eq!(packet["household_runtime_admission"],false);assert_eq!(packet["runtime_admission"],false);
    assert_eq!(packet["human_cook_test_performed"],false);assert_eq!(packet["human_calibration_complete"],false);
    assert!(recipe.capabilities.local_display&&recipe.capabilities.shopping);
    assert!(!recipe.capabilities.leftovers&&!recipe.capabilities.starter_distribution);
    assert!(recipe.capabilities.blocking_issues.iter().any(|i|i.code==CandidateIssueCode::MissingStorage));
    assert!(recipe.capabilities.blocking_issues.iter().any(|i|i.code==CandidateIssueCode::MissingImage));
    assert!(recipe.image.0.is_none()&&recipe.nutrition.0.is_none()&&recipe.storage.is_empty()
        &&recipe.reused_inputs.is_empty()&&recipe.extras.is_empty());
    assert_eq!(recipe.active_minutes,20);assert_eq!(recipe.elapsed_minutes,20);assert_eq!(recipe.prep.len(),1);
    assert_eq!(recipe.prep[0].earliest_offset_seconds,-1200);assert_eq!(recipe.prep[0].latest_offset_seconds,-1200);
    assert!(!recipe.prep[0].can_attach_to_prior_session);assert!(recipe.prep[0].depends_on.is_empty());
    assert_eq!(documents[4]["method_production"]["completion_prep_id"],1);
    assert_eq!(documents[4]["phase_resources"],json!([{"prep_id":1,"resources":["hob"]}]));
    assert_eq!(documents[4]["timing_measured"],false);assert_eq!(documents[4]["thermal_completion_guaranteed"],false);
    assert!(recipe.steps[0].text.0.contains("five-minute")&&recipe.steps[0].text.0.contains("delay service"));
    assert_eq!(recipe.text_rights.licence_id.0,"CC-BY-SA-4.0");
    let classes=ComponentClassView::new(rf.clone(),vec![]).unwrap();
    let empty=EvidenceBasis::new(rf.clone(),vec![],vec![]).unwrap();
    assert_eq!(assess_revision(&recipe,&empty,&classes).unwrap_err().code,ErrorCode::MissingReference);
    let mut missing=basis(&recipe,&documents);missing.admitted.retain(|p|p.evidence_id.0!="ev_80000000000000000000000000000035");
    assert_eq!(assess_revision(&recipe,&missing,&classes).unwrap_err().code,ErrorCode::MissingReference);
    let mut false_claim=recipe.clone();false_claim.capabilities.starter_distribution=true;
    assert_eq!(assess_revision(&false_claim,&basis(&false_claim,&documents),&ComponentClassView::new(reference(&false_claim),vec![]).unwrap()).unwrap_err().code,ErrorCode::DomainConflict);
    let compiled=mstrchf_browser::maker_packets::larger_porridge_packet_ref().unwrap();
    assert_eq!(compiled.recipe,rf);assert_eq!(json!(compiled.packet_digest),digests["packet_canonical_digest"]);
    let registered=mstrchf_browser::maker_packets::known_packet(&compiled).unwrap();
    assert_eq!(registered.record().packet,packet);assert_eq!(registered.record().evidence_documents,documents);
    assert_eq!(registered.not_before(),&parse_document::<Instant>(&canonical_bytes(&digests["not_before"]).unwrap()).unwrap());
    if std::env::var("MSTRCHF_DERIVE_LARGER_PORRIDGE_PACKET").as_deref()==Ok("1") {
        for (name,value) in [("packet.json",&packet),("evidence-documents.json",&documents),("digests.json",&digests)] {write_new_or_equal(name,value);}
    }
    assert_eq!(read("packet.json"),packet);assert_eq!(read("evidence-documents.json"),documents);assert_eq!(read("digests.json"),digests);
    verify_previous_bytes();println!("{}",serde_json::to_string(&digests).unwrap());
}

#[test]
fn native_larger_unit_uses_exact_fixed_inputs_and_portion_precedence_without_double_scaling() {
    let (_,documents,_,recipe)=final_values();let evidence=basis(&recipe,&documents);
    let classes=ComponentClassView::new(reference(&recipe),vec![]).unwrap();let reviewed=assess_revision(&recipe,&evidence,&classes).unwrap();
    let mut m=meal();assert_eq!(eligible(&reviewed,&m).unwrap(),Eligibility::Eligible);
    let check=|m:&MealRequirements,o:&MealOverrides| {
        let servings=portions::resolve_servings(&reviewed,m,o).unwrap();assert_eq!(servings.len(),1);
        assert_eq!(servings[0].person,m.attendees[0].person);assert_eq!(servings[0].servings,PositiveRational::new(1,1).unwrap());
        assert_eq!(servings[0].preset_ref.preset,PortionPresetName::Larger);assert_eq!(servings[0].preset_ref.recipe,reference(&recipe));
        let scaled=portions::scale_recipe(&reviewed,servings[0].servings).unwrap();assert_eq!(scaled.multiplier,PositiveRational::new(1,1).unwrap());
        assert_eq!(scaled.produced_servings,PositiveRational::new(1,1).unwrap());assert_eq!(scaled.unallocated_servings,Rational::new(0,1).unwrap());
        assert_eq!(scaled.components[0].quantity.amount,PositiveRational::new(1,1).unwrap());
        assert_eq!(scaled.ingredients.iter().map(|i|i.quantity.clone()).collect::<Vec<_>>(),vec![
            Quantity {amount:PositiveRational::new(135,2).unwrap(),unit:Unit::G,form:IngredientForm::Dry},
            Quantity {amount:PositiveRational::new(270,1).unwrap(),unit:Unit::Ml,form:IngredientForm::Raw},
            Quantity {amount:PositiveRational::new(270,1).unwrap(),unit:Unit::Ml,form:IngredientForm::Raw}]);
    };
    check(&m,&empty_overrides());
    m.attendees[0].portion.preset=PortionPresetName::Regular;
    m.attendees[0].portion.meal_overrides=BoundedVec(vec![MealPortionOverride {meal_type:MealType::Breakfast,
        preset:PortionPresetName::Larger,adjustment:PositiveRational::new(1,1).unwrap()}]);
    check(&m,&empty_overrides());
    m.attendees[0].portion.meal_overrides=BoundedVec(vec![]);
    assert!(portions::resolve_servings(&reviewed,&m,&empty_overrides()).is_err());
    let explicit=MealOverrides {portions:BoundedVec(vec![PersonPortionOverride {person:m.attendees[0].person.clone(),
        preset:PortionPresetName::Larger,adjustment:PositiveRational::new(1,1).unwrap()}]),eat_by:Nullable(None)};
    check(&m,&explicit);
    for amount in [PositiveRational::new(1,2).unwrap(),PositiveRational::new(3,2).unwrap(),PositiveRational::new(2,1).unwrap()] {
        assert_eq!(portions::scale_recipe(&reviewed,amount).unwrap_err().code,ErrorCode::UnsupportedCapability);
    }
    let mut foreign=explicit.clone();foreign.portions[0].person=PersonRef::Diner {diner_id:DinerId("dn_800000000000000000000000000000ff".into())};
    assert!(portions::resolve_servings(&reviewed,&m,&foreign).is_err());
    for age in [AgeBand::Teen,AgeBand::Child7To10] {let mut other=meal();other.attendees[0].age_band=age;assert!(portions::resolve_servings(&reviewed,&other,&empty_overrides()).is_err());}
    for kind in [MealType::Lunch,MealType::Dinner] {let mut other=meal();other.slot.meal_type=kind;assert!(portions::resolve_servings(&reviewed,&other,&empty_overrides()).is_err());}
}

#[test]
fn no_hard_diet_claim_and_native_arithmetic_bypasses_remain_explicit_future_scope_obligations() {
    let (packet,documents,_,recipe)=final_values();let evidence=basis(&recipe,&documents);
    let classes=ComponentClassView::new(reference(&recipe),vec![]).unwrap();let reviewed=assess_revision(&recipe,&evidence,&classes).unwrap();
    for (requirement,known_conflict) in [(HardRequirement::Diet {code:DietCode::Vegetarian},false),
        (HardRequirement::Diet {code:DietCode::GlutenFree},false),(HardRequirement::Diet {code:DietCode::Vegan},true),
        (HardRequirement::AllergenExclusion {code:AllergenCode::Milk},true),
        (HardRequirement::AllergenExclusion {code:AllergenCode::CerealsContainingGluten},true)] {
        let mut m=meal();m.attendees[0].hard_requirements=BoundedVec(vec![requirement.clone()]);m.shared_hard_requirements=BoundedVec(vec![requirement]);
        let verdict=eligible(&reviewed,&m).unwrap();assert!(if known_conflict {matches!(verdict,Eligibility::Ineligible(_))} else {matches!(verdict,Eligibility::Unresolved(_))});
    }
    assert!(recipe.dietary_evidence.iter().all(|e|e.verdict!=DietaryVerdict::Supported));
    assert_eq!(packet["operational_scope"]["hard_requirements"],json!([]));
    // These accepted arithmetic cases are deliberately NOT claimed to satisfy
    // the future resident/count/all-layer scope. The packet carries no guard.
    let mut two=meal();two.attendees[0].portion.adjustment=PositiveRational::new(1,2).unwrap();
    let mut second=two.attendees[0].clone();second.person=PersonRef::Diner {diner_id:DinerId("dn_800000000000000000000000000000fe".into())};two.attendees.0.push(second);
    let resolved=portions::resolve_servings(&reviewed,&two,&empty_overrides()).unwrap();
    let total=PositiveRational(resolved[0].servings.0.checked_add(&resolved[1].servings.0).unwrap());
    assert_eq!(total,PositiveRational::new(1,1).unwrap());assert!(portions::scale_recipe(&reviewed,total).is_ok());
    let mut compensated=meal();compensated.attendees[0].portion.adjustment=PositiveRational::new(2,1).unwrap();
    let explicit=MealOverrides {portions:BoundedVec(vec![PersonPortionOverride {person:compensated.attendees[0].person.clone(),
        preset:PortionPresetName::Larger,adjustment:PositiveRational::new(1,1).unwrap()}]),eat_by:Nullable(None)};
    assert_eq!(portions::resolve_servings(&reviewed,&compensated,&explicit).unwrap()[0].servings,PositiveRational::new(1,1).unwrap());
    let mut guest=meal();guest.attendees[0].person=PersonRef::Guest {occasion_id:OccasionId("oc_80000000000000000000000000000007".into()),guest_id:GuestId("gu_80000000000000000000000000000007".into())};
    assert!(portions::resolve_servings(&reviewed,&guest,&empty_overrides()).is_ok());
    assert_eq!(packet["operational_scope"]["attendee_count"],1);assert_eq!(packet["operational_scope"]["temporary_guest_count"],0);
    assert_eq!(packet["operational_scope"]["every_applicable_adjustment"],json!({"numerator":"1","denominator":"1"}));
}
