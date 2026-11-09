//! Actual browser/native integration; every order and household is synthetic.
use mstrchf_browser::{catalogue::RuntimeCatalogue, maker_packets::*, run_json, BrowserState};
use mstrchf_contracts::{EquipmentCode, EvidenceId, Instant};
use serde_json::{json, Value};
const NOW: &str = "2026-10-12T21:30:00Z";
const DESTINATION: &str = "cd_10000000000000000000000000000001";
fn call(input: Value) -> Value {
    serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap()
}
fn good(input: Value) -> Value {
    let output = call(input);
    assert_eq!(output["ok"], true, "{output}");
    output["value"].clone()
}
fn dispatch(state: &Value, body: Value, id: u32) -> Value {
    good(json!({"op":"dispatch","state":state,"now":NOW,"command":{
        "schema_version":1,"command_id":format!("cm_{id:032x}"),
        "expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body
    }}))["state"].clone()
}
fn save(state: &Value, week: Value, id: u32) -> Value {
    dispatch(state,json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{
        "week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],
        "day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],
        "cells":week["cells"],"plan":week["plan"]
    }}}),id)
}
fn fixture() -> (Value, Value) { fixture_with_prior(false) }
fn fixture_with_prior(prior_early_meal: bool) -> (Value, Value) {
    let mut state:Value=serde_json::from_slice::<Value>(include_bytes!("fixtures/legacy-engineering-receipts.json")).unwrap()["state"].clone();
    let mut profile=state["snapshot"]["profile"].clone();
    profile["diners"][0]["age_band"]=json!("adult");
    profile["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":NOW}});
    profile["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});
    profile["preferences"]["equipment"]=json!(["fridge","hob"]);
    state=dispatch(&state,json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),0xdb01);
    let mut week=state["snapshot"]["weeks"][0].clone();
    week["plan"]["selections"]=json!([]);week["plan"]["batch_links"]=json!([]);
    week["meal_type_enabled"]["lunch"]=json!(true);
    state=save(&state,week,0xdb02);
    if prior_early_meal {
        let week=&state["snapshot"]["weeks"][0];
        let board=good(json!({"op":"board","state":state,"week_id":week["range"]["week_id"],"slot":week["cells"][4]["slot"],"options":{"eat_by_time":"00:30:00","time_resolution":"unambiguous"},"now":NOW}));
        let choice=board["choices"].as_array().unwrap().first().expect("retained engineering early lunch");
        let mut week=week.clone();week["plan"]=choice["plan"].clone();state=save(&state,week,0xdb09);
    }
    state=dispatch(&state,json!({"kind":"admit_reviewed_recipe","data":{"packet":three_bean_packet_ref().unwrap(),"expected_catalogue_revision":"1"}}),0xdb03);
    let week=&state["snapshot"]["weeks"][0];
    let board=good(json!({"op":"board","state":state,"week_id":week["range"]["week_id"],"slot":week["cells"][4]["slot"],"now":NOW}));
    let reference=serde_json::to_value(three_bean_packet_ref().unwrap().recipe).unwrap();
    let choice=board["choices"].as_array().unwrap().iter().find(|choice|choice["selection"]["recipe"]==reference).expect("exact cold-home choice");
    let mut week=week.clone();week["plan"]=choice["plan"].clone();state=save(&state,week,0xdb04);
    let board=good(json!({"op":"board","state":state,"week_id":state["snapshot"]["weeks"][0]["range"]["week_id"],"now":NOW}));
    (state,board)
}
fn booked(late: bool) -> Value {
    let (mut state,board)=fixture();let week=&state["snapshot"]["weeks"][0];
    state=dispatch(&state,json!({"kind":"approve_plan","data":{"week_id":week["range"]["week_id"],"plan_revision":week["plan"]["revision"],"derived_digest":board["details"]["derived"]["digest"]}}),0xdb05);
    let week=&state["snapshot"]["weeks"][0];let zone=week["range"]["zone"].as_str().unwrap();
    let date=board["details"]["derived"]["prep_sessions"][0]["ingredient_needed_at"]["local_date"].as_str().unwrap();
    let (start,end)=if late {(format!("{date}T11:15:00"),format!("{date}T11:16:00"))} else {("2026-10-12T16:00:00".into(),"2026-10-12T17:00:00".into())};
    let moment=|local:&str| good(json!({"op":"resolve_moment","date":&local[..10],"time":&local[11..],"zone":zone,"resolution":"unambiguous"}));
    state=dispatch(&state,json!({"kind":"confirm_order","data":{"confirmation":{"week_id":week["range"]["week_id"],"approval_id":week["approved_plan"]["approval_id"],"delivery_start":moment(&start),"delivery_end":moment(&end),"confirmed_at":NOW,"source":{"kind":"user_confirmation","data":{}}}}}),0xdb06);
    state
}
#[test]
fn two_phases_share_one_advance_session_with_nine_exact_demands_and_first_handling_clock() {
    let (state,board)=fixture();let derived=&board["details"]["derived"];
    let typed:BrowserState=serde_json::from_value(state).unwrap();
    let view=RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();
    let (basis,_)=mstrchf_browser::calendar_view::material_at(&view,&typed.snapshot,typed.snapshot.profile.0.as_ref().unwrap(),&typed.snapshot.weeks[0],&Instant(NOW.into())).unwrap().unwrap();
    let evidence=basis.evidence;
    assert_eq!(evidence.methods[0].prep_ids,vec![1,2]);
    assert_eq!(evidence.methods[0].evidence_ids,vec![EvidenceId("ev_80000000000000000000000000000024".into())]);
    assert_eq!(evidence.resources.len(),2);
    assert!(evidence.resources.iter().all(|resource|resource.equipment==vec![EquipmentCode::Fridge]&&resource.evidence_ids==vec![EvidenceId("ev_80000000000000000000000000000023".into())]));
    let sessions=derived["prep_sessions"].as_array().unwrap();assert_eq!(sessions.len(),1);
    assert_eq!(sessions[0]["start"]["local_time"],"11:15:00");
    assert_eq!(sessions[0]["end"]["local_time"],"12:30:00");
    assert_eq!(sessions[0]["tasks"].as_array().unwrap().len(),2);
    assert_eq!(derived["demand"]["lines"].as_array().unwrap().len(),9);
    assert_eq!(derived["batches"][0]["components"][0]["original_production"]["kind"],"unheated");
    assert_eq!(derived["batches"][0]["components"][0]["original_production"]["data"]["preparation_started_at"],sessions[0]["start"]);
    assert_eq!(sessions[0]["ingredient_needed_at"],sessions[0]["start"]);
}
#[test]
fn a_new_valid_lunch_deadline_is_checked_in_the_candidate_context() {
    let (_state,board)=fixture_with_prior(true);
    assert_eq!(board["details"]["derived"]["prep_sessions"][0]["start"]["local_time"],"11:15:00");
}
#[test]
fn cold_calendar_retains_two_methods_passive_chill_conditional_service_and_original_clock() {
    let state=booked(false);let order=&state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"];
    let preview=good(json!({"op":"calendar_preview","state":state,"order_id":order,"destination_id":DESTINATION,"now":NOW}));
    assert_eq!(preview["batch"]["events"].as_array().unwrap().len(),2);
    let body=preview["batch"]["events"].as_array().unwrap().iter().map(|event|event["body"].as_str().unwrap()).find(|body|body.contains("Reserve15minutes")).unwrap();
    for expected in ["at least60minutes passive refrigeration","delay or refuse service","Original storage clock — planned first handling","11:15:00","Assemble fresh salad","Covered home fridge","Requires phase 1","80 g","40 g","20 g","60 g","10 ml","15 ml","5 g","joel-maxuel"] { assert!(body.contains(expected),"missing {expected}: {body}"); }
    assert!(!body.contains("75minutes of full attention"));
    assert!(body.contains("FICTIONAL TEST KITCHEN"));
    let prepared=dispatch(&state,json!({"kind":"request_calendar_publication","data":{"order_id":order,"destination_id":preview["batch"]["destination_id"],"event_batch_digest":preview["event_batch_digest"]}}),0xdb07);
    let again=good(json!({"op":"calendar_preview","state":prepared,"order_id":order,"destination_id":DESTINATION,"now":NOW}));
    assert_eq!(again["prepared"],true);assert_eq!(again["ics"],preview["ics"]);
    assert_eq!(prepared["snapshot"]["weeks"][0]["confirmed_order"],state["snapshot"]["weeks"][0]["confirmed_order"]);
}
#[test]
fn cold_late_arrival_is_refused_without_erasing_booked_order() {
    let state=booked(true);let before=state.clone();
    let output=call(json!({"op":"calendar_preview","state":state,"order_id":state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],"destination_id":DESTINATION,"now":NOW}));
    assert_eq!(output["ok"],false);assert_eq!(output["error"]["code"],"domain_conflict");assert!(output["error"]["issues"].to_string().contains("/delivery_end"));assert_eq!(state,before);
    assert!(!state["snapshot"]["weeks"][0]["confirmed_order"].is_null());
}
