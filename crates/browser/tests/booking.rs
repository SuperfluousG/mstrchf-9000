use mstrchf_browser::{maker_packets::porridge_packet_ref,run_json};
use serde_json::{Value,json};
const NOW:&str="2026-10-12T20:00:00Z";
fn call(input:Value)->Value {serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap()}
fn good(input:Value)->Value {let out=call(input);assert_eq!(out["ok"],true,"{out}");out["value"].clone()}
fn envelope(state:&Value,body:Value,id:u32)->Value {json!({"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body})}
fn apply(state:&Value,body:Value,id:u32)->Value {good(json!({"op":"dispatch","state":state,"command":envelope(state,body,id),"now":NOW}))["state"].clone()}
fn edit(state:&Value,week:Value,id:u32)->Value {apply(state,json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{"week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],"day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],"cells":week["cells"],"plan":week["plan"]}}}),id)}
fn base()->Value {
 let mut state:Value=serde_json::from_slice::<Value>(include_bytes!("fixtures/legacy-engineering-receipts.json")).unwrap()["state"].clone();
 let mut p=state["snapshot"]["profile"].clone();p["diners"][0]["age_band"]=json!("adult");p["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":NOW}});p["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});
 state=apply(&state,json!({"kind":"update_profile","data":{"profile":p,"expected_profile_revision":p["revision"]}}),300);
 let mut w=state["snapshot"]["weeks"][0].clone();w["plan"]["selections"]=json!([]);w["plan"]["batch_links"]=json!([]);w["meal_type_enabled"]["breakfast"]=json!(true);state=edit(&state,w,301);
 state=apply(&state,json!({"kind":"admit_reviewed_recipe","data":{"packet":porridge_packet_ref().unwrap(),"expected_catalogue_revision":"1"}}),302);
 let w=&state["snapshot"]["weeks"][0];let board=good(json!({"op":"board","state":state,"week_id":w["range"]["week_id"],"slot":w["cells"][0]["slot"],"now":NOW}));
 let mut week=w.clone();week["plan"]=board["choices"][0]["plan"].clone();edit(&state,week,303)
}
fn approval(state:&Value,id:u32)->Value { approval_week(state,0,id) }
fn approval_week(state:&Value,index:usize,id:u32)->Value {
 let w=&state["snapshot"]["weeks"][index];let b=good(json!({"op":"board","state":state,"week_id":w["range"]["week_id"],"slot":w["cells"][0]["slot"],"now":NOW}));
 apply(state,json!({"kind":"approve_plan","data":{"week_id":w["range"]["week_id"],"plan_revision":w["plan"]["revision"],"derived_digest":b["details"]["derived"]["digest"]}}),id)
}
fn moment(date:&str,hour:u8)->Value {json!({"local_date":date,"local_time":format!("{hour:02}:00:00"),"zone":"Europe/London","resolution":"unambiguous","instant":format!("{date}T{:02}:00:00Z",hour-1)})}
fn confirmation(state:&Value,date:&str)->Value {confirmation_week(state,0,date)}
fn confirmation_week(state:&Value,index:usize,date:&str)->Value {let w=&state["snapshot"]["weeks"][index];json!({"kind":"confirm_order","data":{"confirmation":{"week_id":w["range"]["week_id"],"approval_id":w["approved_plan"]["approval_id"],"delivery_start":moment(date,21),"delivery_end":moment(date,22),"confirmed_at":NOW,"source":{"kind":"user_confirmation","data":{}}}}})}
fn two_weeks()->Value {let state=base();let mut w=state["snapshot"]["weeks"][1].clone();w["plan"]["selections"]=json!([]);w["plan"]["batch_links"]=json!([]);w["meal_type_enabled"]["breakfast"]=json!(true);let state=edit(&state,w,350);let w=&state["snapshot"]["weeks"][1];let b=good(json!({"op":"board","state":state,"week_id":w["range"]["week_id"],"slot":w["cells"][0]["slot"],"now":NOW}));let mut w=w.clone();w["plan"]=b["choices"][0]["plan"].clone();edit(&state,w,351)}

#[test]
fn approving_a_second_independent_week_preserves_the_first_approval() {
 let first=approval_week(&two_weeks(),0,352);let pin=first["snapshot"]["weeks"][0]["approved_plan"].clone();let both=approval_week(&first,1,353);assert_eq!(both["snapshot"]["weeks"][0]["approved_plan"],pin);assert!(both["snapshot"]["weeks"][1]["approved_plan"].is_object());assert_eq!(good(json!({"op":"validate_state","state":both,"now":NOW})),both);
}
#[test]
fn confirming_a_second_week_preserves_unused_first_week_approval_and_both_orders() {
 let first=approval_week(&two_weeks(),0,360);let both=approval_week(&first,1,361);let one=apply(&both,confirmation_week(&both,1,"2026-10-18"),362);assert_eq!(one["snapshot"]["weeks"][0]["approved_plan"],both["snapshot"]["weeks"][0]["approved_plan"]);let two=apply(&one,confirmation_week(&one,0,"2026-10-18"),363);assert!(two["snapshot"]["weeks"][0]["confirmed_order"].is_object());assert!(two["snapshot"]["weeks"][1]["confirmed_order"].is_object());assert_eq!(two["history"]["ordered"].as_array().unwrap().len(),2);assert_eq!(good(json!({"op":"validate_state","state":two,"now":NOW})),two);
}

#[test]
fn approved_food_and_explicit_order_survive_restart_and_identical_receipt_replay() {
 let approved=approval(&base(),310);let body=confirmation(&approved,"2026-10-18");let cmd=envelope(&approved,body,311);let saved=good(json!({"op":"dispatch","state":approved,"command":cmd,"now":NOW}))["state"].clone();
 assert!(saved["snapshot"]["weeks"][0]["confirmed_order"].is_object());assert_eq!(saved["history"]["ordered"].as_array().unwrap().len(),1);
 assert_eq!(good(json!({"op":"validate_state","state":saved,"now":NOW})),saved);
 let repeated=good(json!({"op":"dispatch","state":saved,"command":cmd,"now":NOW}));assert_eq!(repeated["receipt"]["disposition"],"replayed");assert_eq!(repeated["state"],saved);
}
#[test]
fn late_delivery_still_records_the_actual_order_and_plan_edit_preserves_booked_custody() {
 let approved=approval(&base(),320);let saved=apply(&approved,confirmation(&approved,"2026-10-20"),321);let order=saved["snapshot"]["weeks"][0]["confirmed_order"].clone();
 let mut w=saved["snapshot"]["weeks"][0].clone();w["plan"]["selections"]=json!([]);let changed=edit(&saved,w,322);
 assert_eq!(changed["snapshot"]["weeks"][0]["confirmed_order"],order);assert_eq!(changed["snapshot"]["weeks"][0]["publication"],"needs_user");assert_eq!(changed["history"]["ordered"],saved["history"]["ordered"]);
 assert_eq!(good(json!({"op":"validate_state","state":changed,"now":NOW})),changed);
}
#[test]
fn unrelated_change_revokes_unconfirmed_approval_and_blocks_order_without_checkout_proof() {
 let approved=approval(&base(),330);let w=approved["snapshot"]["weeks"][0].clone();let changed=edit(&approved,w,331);assert!(changed["snapshot"]["weeks"][0]["approved_plan"].is_null());
 let out=call(json!({"op":"dispatch","state":changed,"command":envelope(&changed,confirmation(&approved,"2026-10-18"),332),"now":NOW}));assert_eq!(out["ok"],false);assert_eq!(out["error"]["code"],"missing_reference");
}
#[test]
fn engineering_food_stale_digests_and_adapter_authority_are_refused() {
 let engineering=serde_json::from_slice::<Value>(include_bytes!("fixtures/legacy-engineering-receipts.json")).unwrap()["state"].clone();let ew=&engineering["snapshot"]["weeks"][0];let eb=good(json!({"op":"board","state":engineering,"week_id":ew["range"]["week_id"],"now":NOW}));let intent=json!({"kind":"approve_plan","data":{"week_id":ew["range"]["week_id"],"plan_revision":ew["plan"]["revision"],"derived_digest":eb["details"]["derived"]["digest"]}});assert_eq!(call(json!({"op":"dispatch","state":engineering,"command":envelope(&engineering,intent,339),"now":NOW}))["error"]["code"],"unsupported_capability");
 let state=base();let w=&state["snapshot"]["weeks"][0];let body=json!({"kind":"approve_plan","data":{"week_id":w["range"]["week_id"],"plan_revision":w["plan"]["revision"],"derived_digest":"a".repeat(64)}});assert_eq!(call(json!({"op":"dispatch","state":state,"command":envelope(&state,body,340),"now":NOW}))["error"]["code"],"context_stale");
 let approved=approval(&state,341);let mut fake=confirmation(&approved,"2026-10-18");fake["data"]["confirmation"]["source"]=json!({"kind":"authorised_read","data":{"evidence_id":"ev_00000000000000000000000000000009"}});
 assert_eq!(call(json!({"op":"dispatch","state":approved,"command":envelope(&approved,fake,342),"now":NOW}))["error"]["code"],"unsupported_capability");
}

#[test]
fn delivery_resolution_uses_the_saved_zone_and_requires_clock_change_choice() {
 let ordinary=good(json!({"op":"resolve_moment","date":"2026-10-18","time":"21:00:00","zone":"Europe/London","resolution":"unambiguous"}));assert_eq!(ordinary["instant"],"2026-10-18T20:00:00Z");
 assert_eq!(call(json!({"op":"resolve_moment","date":"2026-03-29","time":"01:30:00","zone":"Europe/London","resolution":"unambiguous"}))["ok"],false);
 assert_eq!(call(json!({"op":"resolve_moment","date":"2026-10-25","time":"01:30:00","zone":"Europe/London","resolution":"unambiguous"}))["ok"],false);
 for(resolution,expected)in[("earlier","2026-10-25T00:30:00Z"),("later","2026-10-25T01:30:00Z")]{assert_eq!(good(json!({"op":"resolve_moment","date":"2026-10-25","time":"01:30:00","zone":"Europe/London","resolution":resolution}))["instant"],expected);}
 assert_eq!(good(json!({"op":"resolve_moment","date":"2026-10-18","time":"21:00:00","zone":"America/New_York","resolution":"unambiguous"}))["instant"],"2026-10-19T01:00:00Z");
}
