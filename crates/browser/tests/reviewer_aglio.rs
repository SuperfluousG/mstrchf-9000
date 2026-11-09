//! Independent adverse controls for the frozen dual-packet integration.
//! All positive owner states come from ordinary public commands. Changed
//! receipt/packet values below are explicit hostile import controls, not proof.
use mstrchf_browser::{catalogue::RuntimeCatalogue, maker_packets::*, run_json};
use mstrchf_contracts::*;
use serde_json::{Value,json};
const BEFORE:&str="2026-10-12T18:00:00Z";
const NOW:&str="2026-10-12T20:00:00Z";
fn call(request:Value)->Value{serde_json::from_slice(&run_json(&serde_json::to_vec(&request).unwrap())).unwrap()}
fn good(request:Value)->Value{let result=call(request);assert_eq!(result["ok"],true,"{result}");result["value"].clone()}
fn envelope(state:&Value,body:Value,id:u32)->Value{json!({"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body})}
fn apply_at(state:&Value,body:Value,id:u32,now:&str)->Value{good(json!({"op":"dispatch","state":state,"command":envelope(state,body,id),"now":now}))["state"].clone()}
fn profile(state:&Value,value:Value,id:u32,now:&str)->Value{apply_at(state,json!({"kind":"update_profile","data":{"expected_profile_revision":value["revision"],"profile":value}}),id,now)}
fn base()->Value{
 let state=good(json!({"op":"initialize","demo":true,"household_id":"hh_00000000000000000000000000007c01","diner_id":"dn_00000000000000000000000000007c01","zone":"Europe/London","today":"2026-10-12","now":BEFORE}));
 let mut value=state["snapshot"]["profile"].clone();value["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":BEFORE}});profile(&state,value,1,BEFORE)
}
fn edit_body(week:&Value)->Value{json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{"week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],"day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],"cells":week["cells"],"plan":week["plan"]}}})}
fn edit(state:&Value,week:&Value,id:u32,now:&str)->Value{apply_at(state,edit_body(week),id,now)}
fn admit_body(state:&Value,packet:&ReviewedRecipePacketRef)->Value{json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":(state["reviewed_packets"].as_array().map_or(0,Vec::len)+1).to_string()}})}
fn admit(state:&Value,packet:&ReviewedRecipePacketRef,id:u32,now:&str)->Value{apply_at(state,admit_body(state,packet),id,now)}
fn board(state:&Value,index:usize,options:Value,now:&str)->Value{let week=&state["snapshot"]["weeks"][0];good(json!({"op":"board","state":state,"week_id":week["range"]["week_id"],"slot":week["cells"][index]["slot"],"options":options,"now":now}))}
fn save(state:&Value,index:usize,reference:&RecipeRef,id:u32,now:&str)->Value{let out=board(state,index,Value::Null,now);let chosen=out["choices"].as_array().unwrap().iter().find(|choice|choice["selection"]["recipe"]==serde_json::to_value(reference).unwrap()).expect("expected actual admitted scoped choice");let mut week=state["snapshot"]["weeks"][0].clone();week["plan"]=chosen["plan"].clone();edit(state,&week,id,now)}
fn cold(state:Value,now:&str)->Value{let now=now.to_owned();std::thread::spawn(move||call(json!({"op":"validate_state","state":state,"now":now}))).join().unwrap()}
fn refused_warm_cold(state:Value){assert_eq!(call(json!({"op":"validate_state","state":state,"now":NOW}))["ok"],false);assert_eq!(cold(state,NOW)["ok"],false);}

#[test]
fn immutable_known_bundles_and_exact_policy_clock_cannot_be_replaced_by_public_checksums(){
 let old=known_packet(&porridge_packet_ref().unwrap()).unwrap();let new=known_packet(&aglio_packet_ref().unwrap()).unwrap();
 assert_eq!(old.not_before().0,"2026-10-12T17:16:56Z");assert_eq!(new.not_before().0,"2026-10-12T19:02:02Z");
 assert_eq!(bytes_digest(include_bytes!("../../../sites/mstrchf-9000/content/intake/porridge-personal-reviewed-v1/packet.json")).0,"ae1345c277bf9df3031f47ff6262f1ec5020d565623e5a910847d8613007bf12");
 assert_eq!(bytes_digest(include_bytes!("../assets/catalogue/engineering-v1/food-premises-v1.rs")).0,"f1520b18aaa63c7ec3fef4fae0ed012ef67531fb3f05a7cd8d990a1d5b446718");
 let records=[old.record().clone(),new.record().clone()];let forward=RuntimeCatalogue::from_records(&records).unwrap();let reverse=RuntimeCatalogue::from_records(&[records[1].clone(),records[0].clone()]).unwrap();
 assert_eq!(forward.pin(),reverse.pin());assert_eq!(forward.resolve_recipe(&old.reference().recipe).unwrap(),old.recipe());assert_eq!(forward.resolve_recipe(&new.reference().recipe).unwrap(),new.recipe());
 let mut forged=new.record().clone();forged.ingredient_registry["entries"][0]["meaning"]=json!("Water becomes a different ingredient");
 forged.packet["ingredient_registry_digest"]=json!(canonical_digest(&forged.ingredient_registry).unwrap().0);
 let policy=forged.evidence_documents.as_array_mut().unwrap().last_mut().unwrap();policy["accepted_ingredient_registry_digest"]=forged.packet["ingredient_registry_digest"].clone();
 policy["accepts"]["relevant_essentials"]["ingredient_ids"]=json!(["ig_80000000000000000000000000000002"]);
 forged.packet["maker_relevant_essential_ingredient_ids"]=json!(["ig_80000000000000000000000000000002"]);
 let pins:Vec<Value>=forged.evidence_documents.as_array().unwrap().iter().map(|document|json!({"evidence_id":document["evidence_id"],"revision":document["revision"],"digest":canonical_digest(document).unwrap().0})).collect();
 forged.packet["maker_evidence_pins"]=json!(pins);forged.packet["evidence_documents_digest"]=json!(canonical_digest(&forged.evidence_documents).unwrap().0);
 assert_eq!(known_record(&forged).unwrap_err().code,ErrorCode::DigestMismatch,"Re-signing a declared policy and registry never creates compiled authority");
 let state=base();let command=envelope(&state,admit_body(&state,new.reference()),2);
 assert_eq!(call(json!({"op":"dispatch","state":state,"command":command,"now":"2026-10-12T19:02:01Z"}))["error"]["code"],"authority_denied");
 let exact=admit(&state,new.reference(),2,"2026-10-12T19:02:02Z");assert_eq!(cold(exact.clone(),NOW)["value"],exact);
 let mut early=exact;early["receipts"][1]["committed_at"]=json!("2026-10-12T19:02:01Z");
 refused_warm_cold(early);
}

#[test]
fn r3_selected_before_aglio_retains_original_food_catalogue_and_pantries_after_restart(){
 let mut state=base();let mut week=state["snapshot"]["weeks"][0].clone();week["meal_type_enabled"]["breakfast"]=json!(true);week["cells"][0]["choice"]=json!("at_home");state=edit(&state,&week,10,BEFORE);
 state=admit(&state,&porridge_packet_ref().unwrap(),11,BEFORE);state=save(&state,0,&porridge_packet_ref().unwrap().recipe,12,BEFORE);
 let prior=state.clone();let prior_board=board(&prior,0,Value::Null,BEFORE);let selected_receipt=prior["receipts"].as_array().unwrap().len()-1;
 state=admit(&state,&aglio_packet_ref().unwrap(),13,NOW);let current=board(&state,0,Value::Null,NOW);
 assert_eq!(current["details"]["derived"]["demand"]["lines"],prior_board["details"]["derived"]["demand"]["lines"],"Adding Aglio cannot reclassify any selected R3 ingredient");
 assert!(current["details"]["shopping_list"]["lines"].as_array().unwrap().iter().all(|row|row["confirmation_status"]=="not_offered"));
 for(index,receipt)in prior["receipts"].as_array().unwrap().iter().enumerate(){assert_eq!(state["receipts"][index],*receipt);}
 assert_eq!(cold(state.clone(),NOW)["value"],state);
 let replay=good(json!({"op":"dispatch","state":state,"command":prior["receipts"][selected_receipt]["command"],"now":NOW}));
 assert_eq!(replay["state"],state);assert_eq!(replay["receipt"]["response_digest"],prior["receipts"][selected_receipt]["response_digest"]);
 let mut rebound=state.clone();rebound["receipts"][selected_receipt]["food_catalogue_pin"]=current["details"]["shopping_list"]["catalogue_pin"].clone();refused_warm_cold(rebound);
 let mut absent=state;absent["receipts"][selected_receipt].as_object_mut().unwrap().remove("food_catalogue_pin");refused_warm_cold(absent);
}

#[test]
fn public_choices_and_dispatch_reject_age_count_compensation_extras_and_reuse(){
 let admitted=admit(&base(),&aglio_packet_ref().unwrap(),20,NOW);let diner=admitted["snapshot"]["profile"]["diners"][0]["diner_id"].clone();
 let person=json!({"kind":"diner","data":{"diner_id":diner}});
 for options in[json!({"make_extra":true}),json!({"extra_recipients":[person]}),json!({"overrides":{"portions":[{"person":person,"preset":"regular","adjustment":{"numerator":"1","denominator":"2"}}],"eat_by":null}})]{assert!(board(&admitted,2,options,NOW)["choices"].as_array().unwrap().is_empty());}
 let mut changed=admitted["snapshot"]["profile"].clone();changed["diners"][0]["age_band"]=json!("teen");let teen=profile(&admitted,changed,21,NOW);assert!(board(&teen,2,Value::Null,NOW)["choices"].as_array().unwrap().is_empty());
 let mut changed=admitted["snapshot"]["profile"].clone();changed["diners"][0]["portions"]["adjustment"]=json!({"numerator":"2","denominator":"1"});let doubled=profile(&admitted,changed,22,NOW);
 assert!(board(&doubled,2,json!({"overrides":{"portions":[{"person":person,"preset":"regular","adjustment":{"numerator":"1","denominator":"2"}}],"eat_by":null}}),NOW)["choices"].as_array().unwrap().is_empty());
 let mut changed=admitted["snapshot"]["profile"].clone();changed["diners"][0]["portions"]["adjustment"]=json!({"numerator":"1","denominator":"2"});let mut second=changed["diners"][0].clone();second["diner_id"]=json!("dn_00000000000000000000000000007c02");changed["diners"].as_array_mut().unwrap().push(second);
 let group=profile(&admitted,changed,23,NOW);let mut week=group["snapshot"]["weeks"][0].clone();week["cells"][2]["attendee_ids"]=json!([diner,"dn_00000000000000000000000000007c02"]);let group=edit(&group,&week,24,NOW);assert!(board(&group,2,Value::Null,NOW)["choices"].as_array().unwrap().is_empty(),"Two half portions cannot become one permitted attendee");
 let saved=save(&admitted,2,&aglio_packet_ref().unwrap().recipe,25,NOW);let source=saved["snapshot"]["weeks"][0]["cells"][2]["slot"].clone();assert!(board(&saved,5,json!({"leftover_source":source}),NOW)["choices"].as_array().unwrap().is_empty());
 let mut week=saved["snapshot"]["weeks"][0].clone();let recipients=week["plan"]["selections"][0]["servings"].clone();week["plan"]["selections"][0]["extras"]=json!([{"extra_id":"ex_00000000000000000000000000007c01","recipe":aglio_packet_ref().unwrap().recipe,"recipients":recipients}]);
 let result=call(json!({"op":"dispatch","state":saved,"command":envelope(&saved,edit_body(&week),26),"now":NOW}));assert_eq!(result["error"]["code"],"unsupported_capability");assert_eq!(result["error"]["issues"][0]["path"],"/personal_use_scope/extra_placement");
}

#[test]
fn excluded_aglio_on_the_move_and_age_change_remain_conflicts_after_archival(){
 let admitted=admit(&base(),&aglio_packet_ref().unwrap(),30,NOW);let saved=save(&admitted,2,&aglio_packet_ref().unwrap().recipe,31,NOW);let original=saved["snapshot"]["weeks"][0]["plan"]["selections"][0].clone();
 let mut week=saved["snapshot"]["weeks"][0].clone();week["meal_type_enabled"]["dinner"]=json!(false);week["cells"][2]["choice"]=json!("on_the_move");let excluded=edit(&saved,&week,32,NOW);
 assert!(!excluded["snapshot"]["conflicts"].as_array().unwrap().is_empty(),"Exclusion ignores Skip only; OnTheMove remains intrinsic incompatibility");
 let mut value=excluded["snapshot"]["profile"].clone();value["diners"][0]["age_band"]=json!("teen");let changed=profile(&excluded,value,33,NOW);
 let archived=apply_at(&changed,json!({"kind":"open_planning_horizon","data":{}}),34,"2026-10-26T20:00:00Z");
 assert!(archived["retained_weeks"].as_array().unwrap().iter().flat_map(|week|week["plan"]["selections"].as_array().unwrap()).any(|selection|selection==&original));
 assert!(archived["snapshot"]["conflicts"].as_array().unwrap().iter().any(|conflict|conflict["slot_ids"].as_array().unwrap().contains(&original["slot"])));
 assert_eq!(cold(archived.clone(),"2026-10-26T20:00:00Z")["value"],archived);
}
