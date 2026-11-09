//! Rust JCS binding and bounded semantic checks for the accepted Aglio packet.
//! File preparation supplies no household admission or EvidenceBasis wire route.
use mstrchf_contracts::*;
use mstrchf_core::recipes::*;
use serde_json::{Value,json};
use std::path::PathBuf;

fn folder()->PathBuf { PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sites/mstrchf-9000/content/intake/aglio-personal-reviewed-v1") }
fn read(name:&str)->Value { serde_json::from_slice(&std::fs::read(folder().join(name)).unwrap()).unwrap() }
fn finalized()->(Value,Value,Value,RecipeRevision) {
    let mut packet=read("packet.json");let mut documents=read("evidence-documents.json");
    let registry=read("ingredient-registry.json");let captures=read("source-captures.json");
    let recipe:RecipeRevision=parse_document(&serde_json::to_vec(&packet["maker_recipe_revision"]).unwrap()).unwrap();
    let reference=RecipeRef{recipe_id:recipe.recipe_id.clone(),revision:recipe.revision,digest:recipe_digest(&recipe).unwrap()};
    packet["recipe_reference"]=serde_json::to_value(&reference).unwrap();
    packet["status"]=json!("maker_reviewed_ready_for_compiled_registry_not_household_runtime_admitted");
    let policy=documents.as_array_mut().unwrap().last_mut().unwrap();
    assert_eq!(policy["evidence_id"],"ev_80000000000000000000000000000019");
    policy["recipe_reference"]=serde_json::to_value(&reference).unwrap();
    policy["accepted_ingredient_registry_digest"]=json!(canonical_digest(&registry).unwrap().0);
    let pins:Vec<Value>=documents.as_array().unwrap().iter().map(|document|json!({"evidence_id":document["evidence_id"],"revision":document["revision"],"digest":canonical_digest(document).unwrap().0})).collect();
    packet["maker_evidence_pins"]=json!(pins);
    packet["ingredient_registry_digest"]=json!(canonical_digest(&registry).unwrap().0);
    packet["source_captures_digest"]=json!(canonical_digest(&captures).unwrap().0);
    packet["evidence_documents_digest"]=json!(canonical_digest(&documents).unwrap().0);
    let mut clocks:Vec<Instant>=documents.as_array().unwrap().iter().map(|document|parse_document(&canonical_bytes(&document["at"]).unwrap()).unwrap()).collect();
    clocks.extend(recipe.reviews.iter().map(|review|review.at.clone()));
    let not_before=clocks.into_iter().max().unwrap();
    let digests=json!({"format":"mstrchf-rust-jcs-maker-packet-digests","recipe_reference":reference,
        "packet_canonical_digest":canonical_digest(&packet).unwrap().0,"ingredient_registry_digest":canonical_digest(&registry).unwrap().0,
        "source_captures_digest":canonical_digest(&captures).unwrap().0,"evidence_documents_digest":canonical_digest(&documents).unwrap().0,
        "root_policy_evidence_id":"ev_80000000000000000000000000000019","root_policy_evidence_digest":pins.last().unwrap()["digest"],
        "not_before":not_before,"derivation":"mstrchf_contracts::recipe_digest and canonical_digest, Rust serde_jcs","household_runtime_admission":false});
    (packet,documents,digests,recipe)
}
fn basis(packet:&Value,recipe:&RecipeRevision)->EvidenceBasis {
    EvidenceBasis::new(RecipeRef{recipe_id:recipe.recipe_id.clone(),revision:recipe.revision,digest:recipe_digest(recipe).unwrap()},
        packet["maker_evidence_pins"].as_array().unwrap().iter().map(|pin|EvidencePin{evidence_id:serde_json::from_value(pin["evidence_id"].clone()).unwrap(),revision:serde_json::from_value(pin["revision"].clone()).unwrap(),digest:serde_json::from_value(pin["digest"].clone()).unwrap()}).collect(),vec![]).unwrap()
}

#[test]
fn derive_or_verify_exact_aglio_pins_in_rust_jcs() {
    let (packet,documents,digests,recipe)=finalized();
    if std::env::var("MSTRCHF_DERIVE_AGLIO_PACKET").as_deref()==Ok("1") {
        for(name,value)in[("packet.json",&packet),("evidence-documents.json",&documents),("digests.json",&digests)]{
            std::fs::write(folder().join(name),serde_json::to_vec_pretty(value).unwrap()).unwrap();
        }
    } else {assert_eq!(read("packet.json"),packet);assert_eq!(read("evidence-documents.json"),documents);assert_eq!(read("digests.json"),digests);}
    assert_eq!(recipe.recipe_id.0,"rc_80000000000000000000000000000003");assert_eq!(recipe.revision,Revision(1));
    assert_eq!(packet["maker_evidence_pins"].as_array().unwrap().len(),10);
    assert_eq!(digests["not_before"],"2026-10-12T19:02:02Z");
    println!("{}",serde_json::to_string_pretty(&digests).unwrap());
}

#[test]
fn exact_four_inputs_produce_only_unit_quantity_mechanics_and_no_extra_water() {
    let(packet,_,_,recipe)=finalized();let basis=basis(&packet,&recipe);
    let reference:RecipeRef=serde_json::from_value(packet["recipe_reference"].clone()).unwrap();
    let classes=ComponentClassView::new(reference.clone(),vec![]).unwrap();
    let reviewed=assess_revision(&recipe,&basis,&classes).unwrap();
    assert!(reviewed.assessment().capabilities.shopping);assert!(!reviewed.assessment().capabilities.leftovers);assert!(!reviewed.assessment().capabilities.starter_distribution);
    let scaled=mstrchf_core::portions::scale_recipe(&reviewed,PositiveRational::new(1,1).unwrap()).unwrap();
    let values:Vec<_>=scaled.ingredients.iter().map(|ingredient|(ingredient.ingredient_id.0.clone(),ingredient.quantity.amount,ingredient.quantity.unit,ingredient.quantity.form)).collect();
    assert_eq!(values,vec![("ig_80000000000000000000000000000002".into(),PositiveRational::new(1500,1).unwrap(),Unit::Ml,IngredientForm::Raw),
        ("ig_80000000000000000000000000000004".into(),PositiveRational::new(125,1).unwrap(),Unit::G,IngredientForm::Dry),
        ("ig_80000000000000000000000000000005".into(),PositiveRational::new(55,2).unwrap(),Unit::G,IngredientForm::Raw),
        ("ig_80000000000000000000000000000006".into(),PositiveRational::new(2,1).unwrap(),Unit::Each,IngredientForm::Whole)]);
    for(n,d)in[(1,2),(2,1),(3,2)]{assert!(mstrchf_core::portions::scale_recipe(&reviewed,PositiveRational::new(n,d).unwrap()).is_err());}
    let empty=EvidenceBasis::new(reference,vec![],vec![]).unwrap();assert_eq!(assess_revision(&recipe,&empty,&classes).unwrap_err().code,ErrorCode::MissingReference);
}

#[test]
fn accepted_root_policy_binds_full_recipe_namespace_scope_and_internal_water_transfer() {
    let(packet,documents,_,recipe)=finalized();let policy=documents.as_array().unwrap().last().unwrap();
    assert_eq!(policy["recipe_reference"],packet["recipe_reference"]);
    assert_eq!(policy["accepted_ingredient_registry_digest"],packet["ingredient_registry_digest"]);
    assert_eq!(policy["accepts"]["operational_scope"],packet["operational_scope"]);
    assert_eq!(policy["accepts"]["relevant_essentials"]["ingredient_ids"],packet["maker_relevant_essential_ingredient_ids"]);
    assert_eq!(packet["maker_relevant_essential_ingredient_ids"],json!(["ig_80000000000000000000000000000004","ig_80000000000000000000000000000005"]));
    assert_eq!(policy["accepts"]["water_lineage"]["input_ml"],1500);
    assert_eq!(policy["accepts"]["water_lineage"]["used_reserved_water_ml"],60);
    assert_eq!(policy["accepts"]["water_lineage"]["additional_new_water_demand_ml"],0);
    assert_eq!(policy["actor"],"automated");
    for key in["human_cook_test","human_calibration","nutrition","image","storage_or_reuse","hard_dietary_requirements_supported","evidence_basis_from_request_permitted"]{assert_eq!(policy["limits"][key],false);}
    let mut altered=recipe.clone();altered.ingredients[2].quantity.0.as_mut().unwrap().amount=PositiveRational::new(55,1).unwrap();
    assert_ne!(recipe_digest(&altered).unwrap().0,policy["recipe_reference"]["digest"]);
    assert!(packet.get("evidence_basis").is_none());
}

#[test]
fn prior_porridge_bytes_water_meaning_and_draft_captures_are_preserved() {
    let preservation=read("porridge-preservation.json");
    for name in["packet.json","evidence-documents.json","ingredient-registry.json","source-captures.json"]{
        let bytes=std::fs::read(folder().join("../porridge-personal-reviewed-v1").join(name)).unwrap();
        assert_eq!(bytes_digest(&bytes).0,preservation["files"][name]);
    }
    let captures:Value=serde_json::from_slice(&std::fs::read(folder().join("../aglio-personal-draft-v1/source-captures.json")).unwrap()).unwrap();
    assert_eq!(read("source-captures.json"),captures);
    let registry=read("ingredient-registry.json");
    let prior:Value=serde_json::from_slice(&std::fs::read(folder().join("../porridge-personal-reviewed-v1/ingredient-registry.json")).unwrap()).unwrap();
    assert_eq!(registry["entries"][0],prior["entries"][1]);
}

#[test]
fn review_status_stays_truthful_about_estimates_products_images_and_hard_dietary_evidence() {
    let(packet,documents,_,recipe)=finalized();
    assert!(recipe.reviews.iter().all(|review|review.actor==ReviewActor::Automated&&review.kind!=ReviewKind::CookTest));
    assert_eq!(recipe.active_minutes,25);assert_eq!(recipe.elapsed_minutes,25);
    assert!(recipe.source.adaptation_notice.0.as_ref().unwrap().0.contains("unmeasured planning reservation estimate"));
    assert!(recipe.dietary_evidence.iter().all(|evidence|evidence.verdict!=DietaryVerdict::Supported));
    assert!(recipe.image.0.is_none()&&recipe.nutrition.0.is_none()&&recipe.storage.is_empty()&&recipe.reused_inputs.is_empty());
    assert_eq!(packet["human_cook_test_performed"],false);assert_eq!(packet["household_runtime_admission"],false);
    assert_eq!(documents[2]["actual_products_observed"],false);
}
