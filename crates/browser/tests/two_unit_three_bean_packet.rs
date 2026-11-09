//! Native JCS derivation/verification of one exact maker-authored two-unit packet.
//! This test attests only the accepted fixture documents. It creates no runtime
//! registry entry, inbound EvidenceBasis seam, household, product or cook proof.
use mstrchf_contracts::*;
use mstrchf_core::{eligibility::{eligible, Eligibility}, portions, recipes::*};
use serde_json::{Value, json};
use std::{fs, io::Write, path::PathBuf};

fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../sites/mstrchf-9000/content/intake/three-bean-two-unit-reviewed-v1")
}
fn read(name: &str) -> Value {
    serde_json::from_slice(&fs::read(folder().join(name)).unwrap()).unwrap()
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
fn final_values() -> (Value,Value,Value,RecipeRevision) {
    let input=read("authoring-input.json");
    assert_eq!(input["native_jcs_derived"],false);
    let mut packet=input["packet"].clone();
    let mut documents=input["evidence_documents"].clone();
    let registry=read("ingredient-registry.json");
    let captures=read("source-captures.json");
    let mut recipe:RecipeRevision=parse_document(&serde_json::to_vec(&packet["maker_recipe_revision"]).unwrap()).unwrap();
    // The core computes diagnostics/capabilities, rather than an authoring
    // script manufacturing them. A second assessment closes the final bytes.
    recipe.capabilities=assess(&recipe,&documents);
    let rf=reference(&recipe);
    packet["maker_recipe_revision"]=json!(recipe);
    packet["recipe_reference"]=json!(rf);
    packet["compiled_loader_inputs"]["recipe_reference"]=json!(rf);
    packet["status"]=json!("maker_reviewed_native_jcs_ready_not_registry_or_household_runtime_admitted");
    packet["native_contracts_and_core_assessed"]=json!(true);
    let policy=documents.as_array_mut().unwrap().last_mut().unwrap();
    assert_eq!(policy["evidence_id"],"ev_8000000000000000000000000000002b");
    policy["recipe_reference"]=json!(rf);
    policy["accepted_ingredient_registry_digest"]=json!(canonical_digest(&registry).unwrap());
    policy["accepted_source_captures_digest"]=json!(canonical_digest(&captures).unwrap());
    assert_eq!(policy["accepts"]["operational_scope"],packet["operational_scope"]);
    let evidence=pins(&documents);
    let pin_values:Vec<Value>=evidence.iter().map(|pin| json!({"evidence_id":pin.evidence_id,
        "revision":pin.revision,"digest":pin.digest})).collect();
    packet["maker_evidence_pins"]=json!(pin_values);
    packet["ingredient_registry_digest"]=json!(canonical_digest(&registry).unwrap());
    packet["source_captures_digest"]=json!(canonical_digest(&captures).unwrap());
    packet["evidence_documents_digest"]=json!(canonical_digest(&documents).unwrap());
    assert_eq!(assess(&recipe,&documents),recipe.capabilities);
    let mut clocks:Vec<Instant>=documents.as_array().unwrap().iter()
        .map(|document|parse_document(&canonical_bytes(&document["at"]).unwrap()).unwrap()).collect();
    clocks.extend(recipe.reviews.iter().map(|review| {
        parse_document::<Instant>(&canonical_bytes(&review.at).unwrap()).unwrap()
    }));
    let not_before=clocks.into_iter().max().unwrap();
    let digests=json!({"format":"mstrchf-rust-jcs-maker-packet-digests","recipe_reference":rf,
        "packet_canonical_digest":canonical_digest(&packet).unwrap(),
        "ingredient_registry_digest":canonical_digest(&registry).unwrap(),
        "source_captures_digest":canonical_digest(&captures).unwrap(),
        "evidence_documents_digest":canonical_digest(&documents).unwrap(),
        "root_policy_evidence_id":"ev_8000000000000000000000000000002b",
        "root_policy_evidence_digest":evidence.last().unwrap().digest,
        "method_evidence_id":"ev_8000000000000000000000000000002a",
        "method_evidence_digest":evidence[4].digest,
        "resource_evidence_id":"ev_80000000000000000000000000000029",
        "resource_evidence_digest":evidence[3].digest,"not_before":not_before,
        "derivation":"mstrchf_contracts::recipe_digest/canonical_digest via Rust serde_jcs; mstrchf_core::recipes::assess_revision",
        "household_runtime_admission":false,"registry_entry_present":false});
    (packet,documents,digests,recipe)
}
fn verify_original_bytes() {
    let preservation=read("previous-packet-preservation.json");
    assert_eq!(preservation["files"].as_array().unwrap().len(),12);
    for directory in ["porridge-personal-reviewed-v1","aglio-personal-reviewed-v1","three-bean-personal-reviewed-v1"] {
        for filename in ["packet.json","evidence-documents.json","ingredient-registry.json","source-captures.json"] {
            let item=preservation["files"].as_array().unwrap().iter()
                .find(|item|item["directory"]==directory&&item["filename"]==filename).unwrap();
            let bytes=fs::read(folder().join("..").join(directory).join(filename)).unwrap();
            assert_eq!(json!(bytes.len()),item["bytes"]);
            assert_eq!(json!(bytes_digest(&bytes)),item["sha256_bytes"],"Original {directory}/{filename}");
        }
    }
}
fn write_new_or_equal(name: &str,value: &Value) {
    let destination=folder().join(name);
    if destination.exists() {assert_eq!(read(name),*value,"Refuse replacing differing immutable output {name}");return;}
    let mut file=fs::OpenOptions::new().create_new(true).write(true).open(destination).unwrap();
    file.write_all(&serde_json::to_vec_pretty(value).unwrap()).unwrap();
    file.write_all(b"\n").unwrap();
}
fn meal() -> MealRequirements {
    let mut meal=parse_document::<MealRequirementsDocumentV1>(include_bytes!("../../../tests/fixtures/contracts/meal-requirements.json")).unwrap().meal_requirements;
    let mut diner=meal.attendees[0].clone();
    diner.age_band=AgeBand::Adult;diner.hard_requirements=BoundedVec(vec![]);
    diner.portion=PortionPreference {preset:PortionPresetName::Regular,adjustment:PositiveRational::new(1,1).unwrap(),meal_overrides:BoundedVec(vec![])};
    let mut guest=diner.clone();guest.person=PersonRef::Guest {occasion_id:OccasionId("oc_80000000000000000000000000000005".into()),guest_id:GuestId("gu_80000000000000000000000000000005".into())};
    meal.attendees=BoundedVec(vec![diner,guest]);meal.shared_hard_requirements=BoundedVec(vec![]);
    meal.slot.meal_type=MealType::Lunch;meal.location=MealLocation::AtHome;meal.enabled=true;
    meal.equipment=BoundedVec(vec![EquipmentCode::Fridge]);
    meal.mobility=Mobility {reheating:false,cold_storage:false,temperature:MobilityTemperature::Either};
    meal
}

#[test]
fn derive_or_verify_fixed_two_unit_three_bean_packet_using_native_jcs_and_core() {
    verify_original_bytes();
    let (packet,documents,digests,recipe)=final_values();
    let rf=reference(&recipe);
    assert_eq!(rf.recipe_id.0,"rc_80000000000000000000000000000005");
    assert_eq!(recipe.components[0].component_id.0,"co_80000000000000000000000000000005");
    assert_eq!(recipe.reviews.len(),2);
    assert!(recipe.reviews.iter().all(|r|r.actor==ReviewActor::Automated&&r.kind!=ReviewKind::CookTest));
    assert_eq!(documents.as_array().unwrap().len(),6);
    for (document,suffix) in documents.as_array().unwrap().iter().zip(["26","27","28","29","2a","2b"]) {
        assert_eq!(document["evidence_id"],format!("ev_800000000000000000000000000000{suffix}"));
        assert_eq!(document["actor"],"automated");
        let _:Instant=parse_document(&canonical_bytes(&document["at"]).unwrap()).unwrap();
    }
    assert_eq!(packet["household_runtime_admission"],false);assert_eq!(packet["runtime_admission"],false);
    assert_eq!(packet["human_cook_test_performed"],false);assert_eq!(packet["human_calibration_complete"],false);
    let scope=&packet["operational_scope"];
    assert_eq!(scope["attendee_count"],2);assert_eq!(scope["resident_count_minimum"],1);
    assert_eq!(scope["temporary_guest_count_maximum"],1);assert_eq!(scope["all_attendees"]["restriction_declaration"],"confirmed_none");
    assert_eq!(scope["timing"]["first_handling_offset_seconds"],-5400);assert_eq!(scope["thermal_completion_guaranteed"],false);
    assert_eq!(documents[5]["accepts"]["fixed_units"],2);
    assert_eq!(documents[5]["accepts"]["first_handling_offset_seconds"],-5400);
    assert_eq!(recipe.r#yield.servings,PositiveRational::new(2,1).unwrap());
    assert_eq!(recipe.r#yield.components[0].quantity.amount,PositiveRational::new(2,1).unwrap());
    assert!(recipe.image.0.is_none()&&recipe.nutrition.0.is_none()&&recipe.storage.is_empty()
        &&recipe.reused_inputs.is_empty()&&recipe.dietary_evidence.is_empty()&&recipe.extras.is_empty());
    assert_eq!(recipe.components[0].production,Nullable(Some(ComponentProductionSpec::Unheated {first_handling_prep_id:1})));
    assert_eq!(recipe.prep.len(),2);assert_eq!(recipe.prep[0].active_minutes,30);assert_eq!(recipe.prep[0].elapsed_minutes,30);
    assert_eq!(recipe.prep[0].latest_offset_seconds,-5400);assert_eq!(recipe.prep[1].latest_offset_seconds,-3600);
    assert_eq!(recipe.prep[1].active_minutes,0);assert_eq!(recipe.prep[1].elapsed_minutes,60);
    assert_eq!(recipe.prep[1].depends_on.0,vec![1]);
    let evidence=basis(&recipe,&documents);let classes=ComponentClassView::new(rf.clone(),vec![]).unwrap();
    let reviewed=assess_revision(&recipe,&evidence,&classes).unwrap();
    assert!(reviewed.assessment().capabilities.local_display&&reviewed.assessment().capabilities.shopping);
    assert!(!reviewed.assessment().capabilities.leftovers&&!reviewed.assessment().capabilities.starter_distribution);
    assert_eq!(recipe.capabilities,reviewed.assessment().capabilities);
    let requirements=meal();assert_eq!(eligible(&reviewed,&requirements).unwrap(),Eligibility::Eligible);
    let servings=portions::resolve_servings(&reviewed,&requirements,&MealOverrides {portions:BoundedVec(vec![]),eat_by:Nullable(None)}).unwrap();
    assert_eq!(servings.len(),2);assert!(servings.iter().all(|s|s.servings==PositiveRational::new(1,1).unwrap()));
    assert_eq!(servings[0].person,requirements.attendees[0].person);assert_eq!(servings[1].person,requirements.attendees[1].person);
    let scaled=portions::scale_recipe(&reviewed,PositiveRational::new(2,1).unwrap()).unwrap();
    assert_eq!(scaled.multiplier,PositiveRational::new(1,1).unwrap());assert_eq!(scaled.components[0].quantity.amount,PositiveRational::new(2,1).unwrap());
    assert_eq!(scaled.ingredients.len(),9);
    for (ingredient,amount,unit,form) in [
        ("07",160,Unit::G,IngredientForm::Drained),("08",160,Unit::G,IngredientForm::Drained),("09",160,Unit::G,IngredientForm::Drained),
        ("0a",80,Unit::G,IngredientForm::Raw),("0b",40,Unit::G,IngredientForm::Raw),("0c",120,Unit::G,IngredientForm::Raw),
        ("0d",20,Unit::Ml,IngredientForm::Prepared),("0e",30,Unit::Ml,IngredientForm::Prepared),("0f",10,Unit::G,IngredientForm::Prepared),
    ] {let actual=scaled.ingredients.iter().find(|i|i.ingredient_id.0==format!("ig_800000000000000000000000000000{ingredient}")).unwrap();
        assert_eq!(actual.quantity,Quantity {amount:PositiveRational::new(amount,1).unwrap(),unit,form});}
    for amount in [1,3] {assert_eq!(portions::scale_recipe(&reviewed,PositiveRational::new(amount,1).unwrap()).unwrap_err().code,ErrorCode::UnsupportedCapability);}
    let mut missing=evidence.clone();missing.admitted.retain(|p|p.evidence_id.0!="ev_80000000000000000000000000000029");
    assert_eq!(assess_revision(&recipe,&missing,&classes).unwrap_err().code,ErrorCode::MissingReference);
    let mut wrong_caps=recipe.clone();wrong_caps.capabilities.shopping=false;
    let wrong_basis=basis(&wrong_caps,&documents);let wrong_classes=ComponentClassView::new(reference(&wrong_caps),vec![]).unwrap();
    assert_eq!(assess_revision(&wrong_caps,&wrong_basis,&wrong_classes).unwrap_err().code,ErrorCode::DomainConflict);
    for capture in read("source-captures.json")["captures"].as_array().unwrap() {
        let filename=capture["retained_file"].as_str().unwrap();
        assert!(["pdr-bean-salad.md","pdr-LICENSE.md","pdr-README.md","fsa-chilling.html","fsa-allergens.html"].contains(&filename));
        let bytes=fs::read(folder().join(filename)).unwrap();assert_eq!(json!(bytes_digest(&bytes)),capture["body_sha256"]);assert_eq!(json!(bytes.len()),capture["body_bytes"]);
    }
    let existing=mstrchf_browser::maker_packets::available_packet_refs().unwrap();
    for directory in ["porridge-personal-reviewed-v1","aglio-personal-reviewed-v1","three-bean-personal-reviewed-v1"] {
        let old:Value=serde_json::from_slice(&fs::read(folder().join("..").join(directory).join("packet.json")).unwrap()).unwrap();
        let old_ref:RecipeRef=parse_document(&canonical_bytes(&old["recipe_reference"]).unwrap()).unwrap();
        assert!(existing.iter().any(|packet|packet.recipe==old_ref));
        assert!(old_ref.recipe_id!=rf.recipe_id);
    }
    if std::env::var("MSTRCHF_DERIVE_TWO_UNIT_PACKET").as_deref()==Ok("1") {
        for (name,value) in [("packet.json",&packet),("evidence-documents.json",&documents),("digests.json",&digests)] {write_new_or_equal(name,value);}
    }
    assert_eq!(read("packet.json"),packet);assert_eq!(read("evidence-documents.json"),documents);assert_eq!(read("digests.json"),digests);
    verify_original_bytes();
    println!("{}",serde_json::to_string(&digests).unwrap());
}
