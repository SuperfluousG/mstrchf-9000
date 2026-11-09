//! Actual domain custody from a frozen synthetic browser export. No retailer,
//! checkout, delivery, household or calendar account observation is asserted.
use mstrchf_browser::{calendar_owner, run_json, BrowserState};
use mstrchf_contracts::*;
use serde_json::{json, Value};

const NOW: &str = "2026-10-12T21:30:00Z";
const DEST: &str = "cd_10000000000000000000000000000001";
fn call(input: Value) -> Value { serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap() }
fn good(input: Value) -> Value { let out=call(input); assert_eq!(out["ok"],true,"{out}"); out["value"].clone() }
fn assert_issue(out: &Value, path: &str) { assert_eq!(out["ok"],false,"{out}"); assert!(out["error"]["issues"].as_array().unwrap().iter().any(|issue|issue["path"]==path),"missing {path}: {out}"); }
fn fixture() -> Value { serde_json::from_slice::<Value>(include_bytes!("fixtures/calendar-v15-renderer2-prepared.json")).unwrap()["state"].clone() }
fn envelope(state: &Value, body: Value, id: u32) -> Value { json!({"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body}) }
fn apply(state: &Value, body: Value, id: u32) -> Value { good(json!({"op":"dispatch","state":state,"command":envelope(state,body,id),"now":NOW}))["state"].clone() }
fn moment(date: &str, hour: u8) -> Value { good(json!({"op":"resolve_moment","date":date,"time":format!("{hour:02}:00:00"),"zone":"Europe/London","resolution":"unambiguous"})) }
fn correction(state: &Value, date: &str, hour: u8) -> Value { json!({"kind":"correct_order_delivery","data":{"order_id":state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],"delivery_start":moment(date,hour),"delivery_end":moment(date,hour+1)}}) }
fn preview(state: &Value) -> Value { good(json!({"op":"calendar_preview","state":state,"order_id":state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],"destination_id":DEST,"now":NOW})) }
fn board(state: &Value) -> Value { good(json!({"op":"board","state":state,"week_id":state["snapshot"]["weeks"][0]["range"]["week_id"],"now":NOW})) }
fn cold(state: Value) -> Value { std::thread::spawn(move||good(json!({"op":"validate_state","state":state,"now":NOW}))).join().unwrap() }
fn resign_last(state: &mut Value) {
    let receipt=state["receipts"].as_array_mut().unwrap().last_mut().unwrap();
    let command: AppCommandV1=serde_json::from_value(receipt["command"].clone()).unwrap();
    let response: Snapshot=serde_json::from_value(receipt["response"].clone()).unwrap();
    receipt["command_digest"]=json!(canonical_digest(&command).unwrap());
    receipt["response_digest"]=json!(canonical_digest(&response).unwrap());
    state["snapshot"]=serde_json::to_value(response).unwrap();
    // Keep the ordinary current/retained mirror consistent so the adversarial
    // fixture reaches the command-specific response guard, not that prior guard.
    let weeks=state["snapshot"]["weeks"].as_array().unwrap().clone();
    for week in weeks { let id=week["range"]["week_id"].clone();
        if let Some(retained)=state["retained_weeks"].as_array_mut().unwrap().iter_mut().find(|retained|retained["range"]["week_id"]==id) { *retained=week; }
    }
}

#[test]
fn late_then_feasible_correction_preserves_order_food_and_history_and_revises_only_delivery_event() {
    let old=fixture(); let original=old["snapshot"]["weeks"][0].clone(); let receipts=old["receipts"].as_array().unwrap();
    let old_events=old["receipts"].as_array().unwrap().last().unwrap()["prepared_calendar"]["saved_events"].as_array().unwrap();
    assert_eq!(receipts.last().unwrap()["prepared_calendar"]["batch"]["destination_id"],DEST);
    let old_derived=board(&old)["details"]["derived"].clone();
    let late=apply(&old,correction(&old,"2026-10-20",9),0xdc01);
    assert_eq!(late["snapshot"]["weeks"][0]["publication"],"not_requested");
    let failure=call(json!({"op":"calendar_preview","state":late,"order_id":original["confirmed_order"]["order_id"],"destination_id":DEST,"now":NOW}));
    assert_issue(&failure,"/delivery_end");
    let body=correction(&late,"2026-10-19",20); let command=envelope(&late,body,0xdc02);
    let next=good(json!({"op":"dispatch","state":late,"command":command,"now":NOW}))["state"].clone();
    for state in [&late,&next] {
        assert_eq!(state["snapshot"]["weeks"][0]["confirmed_order"],original["confirmed_order"]);
        assert_eq!(state["snapshot"]["weeks"][0]["plan"],original["plan"]);
        assert_eq!(state["snapshot"]["weeks"][0]["revision"],original["revision"]);
        assert_eq!(state["history"],old["history"]); assert_eq!(&state["receipts"].as_array().unwrap()[..receipts.len()],receipts);
    }
    let details=board(&next)["details"].clone(); assert_eq!(details["derived"],old_derived);
    assert_eq!(details["effective_delivery"]["correction_count"],2);
    assert_eq!(details["effective_delivery"]["last_corrected_at"],NOW);
    assert_eq!(details["effective_delivery"]["delivery_end"],moment("2026-10-19",21));
    let p=preview(&next); assert_eq!(p["prepared"],false);
    let save=envelope(&next,json!({"kind":"request_calendar_publication","data":{"order_id":original["confirmed_order"]["order_id"],"destination_id":DEST,"event_batch_digest":p["event_batch_digest"]}}),0xdc03);
    let saved=good(json!({"op":"dispatch","state":next,"command":save,"now":NOW}))["state"].clone();
    let journal=&saved["receipts"].as_array().unwrap().last().unwrap()["prepared_calendar"];
    assert_eq!(journal["renderer_version"],4); assert_eq!(preview(&saved)["prepared"],true);
    for event in journal["saved_events"].as_array().unwrap() {
        let before=old_events.iter().find(|old|old["event"]["event_id"]==event["event"]["event_id"]).unwrap();
        if event["event"]["logical_key"]["kind"]=="delivery" { assert_eq!(event["sequence"].as_u64().unwrap(),before["sequence"].as_u64().unwrap()+1); }
        else { assert_eq!(event,before,"Cooking event bytes remain exact"); }
    }
    assert_eq!(saved["snapshot"]["weeks"][0]["confirmed_order"],original["confirmed_order"]);
    assert_eq!(saved["history"],old["history"]); assert_eq!(cold(saved.clone()),saved);
    for cmd in [command,save,receipts.last().unwrap()["command"].clone(),receipts.iter().find(|r|r["command"]["command"]["kind"]=="confirm_order").unwrap()["command"].clone()] {
        let replay=good(json!({"op":"dispatch","state":saved,"command":cmd,"now":NOW})); assert_eq!(replay["state"],saved); assert_eq!(replay["receipt"]["disposition"],"replayed");
    }
    let typed:BrowserState=serde_json::from_value(saved.clone()).unwrap();let order=typed.snapshot.weeks[0].confirmed_order.0.as_ref().unwrap();
    for version in 1..=3 { assert!(calendar_owner::prepare_for_renderer(&typed,&order.order_id,&CalendarDestinationId(DEST.into()),&Instant(NOW.into()),version).unwrap_err().issues.iter().any(|issue|issue.path=="/calendar/delivery_correction_renderer")); }
}

#[test]
fn same_window_foreign_zone_invalid_moment_stale_and_replay_conflicts_are_refused() {
    let state=fixture();let original=&state["snapshot"]["weeks"][0]["confirmed_order"]["confirmation"];
    let unchanged=json!({"kind":"correct_order_delivery","data":{"order_id":state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],"delivery_start":original["delivery_start"],"delivery_end":original["delivery_end"]}});
    let out=call(json!({"op":"dispatch","state":state,"command":envelope(&state,unchanged,0xdc11),"now":NOW})); assert_issue(&out,"/order/delivery_unchanged");
    let mut foreign=correction(&state,"2026-10-19",20);
    for (field,hour) in [("delivery_start","20:00:00"),("delivery_end","21:00:00")] { foreign["data"][field]=good(json!({"op":"resolve_moment","date":"2026-10-19","time":hour,"zone":"America/New_York","resolution":"unambiguous"})); }
    assert_issue(&call(json!({"op":"dispatch","state":state,"command":envelope(&state,foreign,0xdc12),"now":NOW})),"/order/delivery_zone");
    let mut equal=correction(&state,"2026-10-19",20);equal["data"]["delivery_end"]=equal["data"]["delivery_start"].clone();
    let mut reversed=correction(&state,"2026-10-19",20);reversed["data"]["delivery_end"]=moment("2026-10-19",19);
    let mut foreign_order=correction(&state,"2026-10-19",20);foreign_order["data"]["order_id"]=json!("or_00000000000000000000000000000001");
    for invalid in [equal,reversed,foreign_order] {
        assert_eq!(call(json!({"op":"dispatch","state":state,"command":envelope(&state,invalid,0xdc13),"now":NOW}))["ok"],false);
    }
    let mut bad=correction(&state,"2026-10-19",20);bad["data"]["delivery_start"]["instant"]=json!("2026-10-19T18:00:00Z");
    assert_eq!(call(json!({"op":"dispatch","state":state,"command":envelope(&state,bad,0xdc14),"now":NOW}))["ok"],false);
    let command=envelope(&state,correction(&state,"2026-10-19",20),0xdc15);let next=good(json!({"op":"dispatch","state":state,"command":command,"now":NOW}))["state"].clone();
    assert_eq!(good(json!({"op":"dispatch","state":next,"command":command,"now":NOW}))["state"],next);
    let mut changed=command.clone();changed["command"]=correction(&state,"2026-10-18",20);assert_eq!(call(json!({"op":"dispatch","state":next,"command":changed,"now":NOW}))["error"]["code"],"replay_conflict");
    assert_eq!(call(json!({"op":"dispatch","state":next,"command":envelope(&state,correction(&state,"2026-10-18",20),0xdc16),"now":NOW}))["error"]["code"],"context_stale");
    // Returning to an older window cannot make its old saved batch downloadable
    // without an explicit current preparation, even when event bytes coincide.
    let returned=apply(&next,correction(&next,"2026-10-18",21),0xdc17);assert_eq!(preview(&returned)["prepared"],false);
}

#[test]
fn changed_plan_needs_user_survives_correction_without_rewriting_ordered_history() {
    let state=fixture();let mut week=state["snapshot"]["weeks"][0].clone();week["plan"]["selections"]=json!([]);week["plan"]["batch_links"]=json!([]);
    let edited=apply(&state,json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{"week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],"day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],"cells":week["cells"],"plan":week["plan"]}}}),0xdc21);
    let next=apply(&edited,correction(&edited,"2026-10-19",20),0xdc22);
    assert_eq!(next["snapshot"]["weeks"][0]["publication"],"needs_user");assert_eq!(next["snapshot"]["weeks"][0]["confirmed_order"],state["snapshot"]["weeks"][0]["confirmed_order"]);
    assert_eq!(next["history"],edited["history"]);assert_eq!(next["history"]["ordered"],state["history"]["ordered"]);
    assert_issue(&call(json!({"op":"calendar_preview","state":next,"order_id":state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],"destination_id":DEST,"now":NOW})),"/calendar/fixed_order");assert_eq!(cold(next.clone()),next);
}

#[test]
fn resigned_correction_cannot_change_profile_food_order_time_or_zone() {
    let state=fixture();let saved=apply(&state,correction(&state,"2026-10-19",20),0xdc31);
    let mut foreign=saved.clone();let receipt=foreign["receipts"].as_array_mut().unwrap().last_mut().unwrap();
    for (field,time) in [("delivery_start","20:00:00"),("delivery_end","21:00:00")] {receipt["command"]["command"]["data"][field]=good(json!({"op":"resolve_moment","date":"2026-10-19","time":time,"zone":"America/New_York","resolution":"unambiguous"}));}
    resign_last(&mut foreign);assert_issue(&call(json!({"op":"validate_state","state":foreign,"now":NOW})),"/order/delivery_zone");
    let mut earlier=saved.clone();earlier["receipts"].as_array_mut().unwrap().last_mut().unwrap()["committed_at"]=json!("2026-10-12T20:48:16Z");assert_issue(&call(json!({"op":"validate_state","state":earlier,"now":NOW})),"/order/corrected_at");
    for field in ["profile","plan","order"] {
        let mut forged=saved.clone();let response=&mut forged["receipts"].as_array_mut().unwrap().last_mut().unwrap()["response"];
        match field {"profile"=>response["profile"]["diners"][0]["display_name"]=json!("Forged"),"plan"=>{response["weeks"][0]["plan"]["selections"]=json!([]);response["weeks"][0]["plan"]["batch_links"]=json!([]);},_=>response["weeks"][0]["confirmed_order"]["confirmation"]["delivery_start"]=moment("2026-10-18",20)};
        resign_last(&mut forged);
        if field=="plan" { forged["history"]["selected"]=json!([]); }
        assert_issue(&call(json!({"op":"validate_state","state":forged,"now":NOW})),"/receipts/delivery_correction_response");
    }
}
