//! Exact compiled registry custody only. No real product/handling, human
//! authority, household admission, prepared basket or calendar qualification.
use mstrchf_browser::maker_packets::*;
use mstrchf_contracts::*;
use serde_json::{json,Value};
use std::{fs,path::PathBuf};
fn repo()->PathBuf{PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")}
fn expected_old()->Value{json!([
 {"recipe":{"recipe_id":"rc_80000000000000000000000000000001","revision":"3","digest":"dd867c017a9b591f57546246518cd93c94e9f1ab50c54bfe4f288a1054de6ce0"},"packet_digest":"edc4239b57366c96ef4b91198e4cc54de62db44d3d6346574e7318a852e2d726"},
 {"recipe":{"recipe_id":"rc_80000000000000000000000000000003","revision":"1","digest":"58914b613960257e33ce294636bcd417baccb631f61379a22aa4977b14cc14f3"},"packet_digest":"62949a9b43fb50ed7385271a22c09f9b5d703ff08ce865041b66c6d6f2d14dee"},
 {"recipe":{"recipe_id":"rc_80000000000000000000000000000004","revision":"1","digest":"247de874dc0348b4afdc06fe6b5aab31cdebf2869ace5098393d61dae9a80377"},"packet_digest":"ec3894c5071806462224a3ba282467b159b8a84ad05df8ae12995489784802ac"},
 {"recipe":{"recipe_id":"rc_80000000000000000000000000000005","revision":"1","digest":"bea391c42fe3a7207d4ac04104f5fac6d839a36970a197be490585c89fb0cf7c"},"packet_digest":"e18d259dfa7941b893997dee7b378616457025ccac92021d54a2a43e25173932"},
 {"recipe":{"recipe_id":"rc_80000000000000000000000000000006","revision":"1","digest":"9e5495c21e6da79685364e4069c62485db521652f5b184f955ff5ae27a6f1632"},"packet_digest":"61aa9bc1b586ff82a1ceab2198f5cf88e7113ef82e37bdfa2b60cd4c7915de6d"},
 {"recipe":{"recipe_id":"rc_80000000000000000000000000000007","revision":"1","digest":"8e73018765acca1f0265082f1c95601359274014259a306f4621ed805a133b97"},"packet_digest":"93df7d839d9076d377fb1c08b201c3048aff4a428093a1cf75fcf6cf919cebfd"}])}
fn read(path:&str)->Value{serde_json::from_slice(&fs::read(repo().join(path)).unwrap()).unwrap()}
fn preservation(){let manifest=read("sites/mstrchf-9000/content/intake/three-bean-two-unit-chicken-extra-reviewed-v1/previous-packet-preservation.json");assert_eq!(manifest["files"].as_array().unwrap().len(),142);for row in manifest["files"].as_array().unwrap(){let bytes=fs::read(repo().join(row["path"].as_str().unwrap())).unwrap();assert_eq!(json!(bytes.len()),row["bytes"]);assert_eq!(json!(bytes_digest(&bytes)),row["sha256_bytes"],"{}",row["path"]);}}
#[test]
fn literal_old_prefix_and_every_frozen_body_registry_capture_and_document_are_preserved(){
 preservation();let refs=available_packet_refs().unwrap();assert_eq!(refs.len(),11);assert_eq!(json!(&refs[..6]),expected_old());
 for (index,resource) in [(0,"0b"),(1,"19"),(2,"23"),(3,"29"),(4,"2f"),(5,"37")]{let packet=known_packet(&refs[index]).unwrap();assert!(packet.record().evidence_documents.as_array().unwrap().iter().any(|d|d["evidence_id"]==format!("ev_800000000000000000000000000000{resource}")));assert_eq!(known_record(packet.record()).unwrap().reference(),&refs[index]);}
 let old=known_packet(&refs[4]).unwrap();let base=known_packet(&refs[6]).unwrap();assert_eq!(base.recipe().ingredients.iter().map(|i|(&i.ingredient_id,&i.quantity)).collect::<Vec<_>>(),old.recipe().ingredients.iter().map(|i|(&i.ingredient_id,&i.quantity)).collect::<Vec<_>>());assert_eq!(base.recipe().prep,old.recipe().prep);assert_eq!(base.record().ingredient_registry["entries"].as_array().unwrap()[..9],old.record().ingredient_registry["entries"].as_array().unwrap()[..]);assert_eq!(base.record().source_captures,old.record().source_captures);
}
#[test]
fn exact_new_references_rights_unique_policy_and_phase_custody_are_compiled(){
 let base=three_bean_two_unit_chicken_extra_packet_ref().unwrap();let extra=chilled_chicken_extra_packet_ref().unwrap();
 assert_eq!(json!(base),json!({"recipe":{"recipe_id":"rc_80000000000000000000000000000008","revision":"1","digest":"fa50f509872b19c8b986dff36ec91397e5268a60afbb93a7728badfde8661a71"},"packet_digest":"a6a189f045a8fe21617efb5fed082a43773071c6250c2b06bdda943e32395407"}));
 assert_eq!(json!(extra),json!({"recipe":{"recipe_id":"rc_80000000000000000000000000000009","revision":"1","digest":"ef0cbaba1c1698ccaaa8de4b0f6ee4526d33150645532a3ca822f65ba9cf97c9"},"packet_digest":"e9675ef1aec183cf4d540b5c2c23856a1666f02bcf6b57080e8b845f1551209b"}));
 for(reference,table,map,count,policy)in[(&base,"68d632a4173ce72feec8771e6d7ceafd1c7d7b33523ea12912a57893321cd0a1","8be352df60af8b7a9297f7cbfd176e4c93b40f9509ae18ef7a4c5834fc578837",7,"4c"),(&extra,"9947369c9ceb7640855ce2d0a5483096588597171b9f526489b730b9ab720da0","b3eeed521d11bfa8d4906cbf3b9fd2eeda7d06296216402b0da8848317388ef5",9,"45")]{let packet=known_packet(reference).unwrap();assert_eq!(packet.not_before(),&Instant("2026-10-13T05:20:48Z".into()));assert_eq!(packet.root_policy_evidence_id().0,format!("ev_800000000000000000000000000000{policy}"));assert_eq!(packet.record().evidence_documents.as_array().unwrap().len(),count);assert_eq!(canonical_digest(packet.intrinsic_input_conditions().unwrap()).unwrap().0,table);assert_eq!(canonical_digest(&packet.record().packet["phase_resource_map"]).unwrap().0,map);assert_eq!(known_record(packet.record()).unwrap().reference(),reference);assert!(packet.recipe().capabilities.local_display&&packet.recipe().capabilities.shopping&&!packet.recipe().capabilities.leftovers&&!packet.recipe().capabilities.starter_distribution);}
 let side=known_packet(&extra).unwrap();assert_eq!(side.record().evidence_documents[7]["evidence_id"],"ev_80000000000000000000000000000045");assert_eq!(side.record().evidence_documents[8]["evidence_id"],"ev_8000000000000000000000000000004d");assert!(side.recipe().source.url.0.is_none());assert_eq!(side.recipe().source.origin,RecipeSourceOrigin::UserAuthored);assert_eq!(side.recipe().text_rights.distribution,RightsDistribution::PersonalOnly);assert!(side.recipe().text_rights.licence_url.0.is_none());assert!(side.recipe().capabilities.blocking_issues.iter().any(|i|i.code==CandidateIssueCode::MissingRights));assert_eq!(known_packet(&base).unwrap().recipe().extras[0].recipe,extra.recipe);assert_eq!(side.record().packet["phase_resource_map"]["entries"],json!([{"prep_id":1,"equipment":[]}]));assert_eq!(side.recipe().equipment.0,vec![EquipmentCode::Fridge]);
}
#[test]
fn self_resigned_table_map_rights_scope_dependency_or_source_cannot_gain_compiled_authority(){
 for reference in [three_bean_two_unit_chicken_extra_packet_ref().unwrap(),chilled_chicken_extra_packet_ref().unwrap()]{let original=known_packet(&reference).unwrap();for attack in 0..8{let mut forged=original.record().clone();match attack{
 0=>forged.packet["intrinsic_input_conditions"]["product_and_operational_compliance_observed"]=json!(true),
 1=>forged.packet["phase_resource_map"]["entries"]=json!([]),
 2=>forged.packet["phase_resource_map"]["recipe_reference"]["digest"]=json!("0".repeat(64)),
 3=>forged.packet["maker_recipe_revision"]["source"]["url"]=json!("https://example.invalid/fictional-recipe"),
 4=>{let changed=if forged.packet["maker_recipe_revision"]["text_rights"]["distribution"]=="permitted"{"personal_only"}else{"permitted"};forged.packet["maker_recipe_revision"]["text_rights"]["distribution"]=json!(changed);},
 5=>forged.packet["operational_scope"]["location"]=json!("on_the_move"),
 6=>forged.packet["declared_extra_dependency"]["base_without_selected_extra_still_requires_dependency"]=json!(false),
 _=>{let document=forged.evidence_documents.as_array_mut().unwrap().iter_mut().find(|d|d["evidence_id"]==json!(original.root_policy_evidence_id())).unwrap();document["status"]=json!("accepted_for_compiled_maker_registry_limited_scope");},}
 let recipe:RecipeRevision=parse_document(&canonical_bytes(&forged.packet["maker_recipe_revision"]).unwrap()).unwrap();let rf=RecipeRef{recipe_id:recipe.recipe_id.clone(),revision:recipe.revision,digest:recipe_digest(&recipe).unwrap()};forged.packet["recipe_reference"]=json!(rf);forged.packet["maker_evidence_pins"]=json!(forged.evidence_documents.as_array().unwrap().iter().map(|d|json!({"evidence_id":d["evidence_id"],"revision":d["revision"],"digest":canonical_digest(d).unwrap()})).collect::<Vec<_>>());forged.packet["evidence_documents_digest"]=json!(canonical_digest(&forged.evidence_documents).unwrap());let changed=ReviewedRecipePacketRef{recipe:rf,packet_digest:canonical_digest(&forged.packet).unwrap()};assert_ne!(forged,original.record().clone(),"Every attack must actually alter retained bytes");assert_eq!(known_record(&forged).err().expect("changed record must refuse").code,ErrorCode::DigestMismatch);assert_eq!(known_packet(&changed).err().expect("changed pin must refuse").code,ErrorCode::DigestMismatch);}
 assert_eq!(known_record(original.record()).unwrap().reference(),&reference);}
 preservation();
}
