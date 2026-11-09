//! Synthetic user-reviewed observations only. No Google access, external write,
//! checkout or cooking evidence is claimed by these native boundary tests.
use mstrchf_browser::run_json;
use mstrchf_contracts::*;
use serde_json::{json, Value};

const NOW: &str = "2026-10-12T21:30:00Z";
const DEST: &str = "cd_10000000000000000000000000000001";
fn call(request: Value) -> Value { serde_json::from_slice(&run_json(&serde_json::to_vec(&request).unwrap())).unwrap() }
fn good(request: Value) -> Value { let out=call(request); assert_eq!(out["ok"],true,"{out}");out["value"].clone() }
fn issue(out: Value, path: &str) { assert_eq!(out["ok"],false,"expected refusal at {path}");assert!(out["error"]["issues"].as_array().unwrap().iter().any(|issue|issue["path"]==path),"{out}"); }
fn fixture() -> Value { serde_json::from_slice::<Value>(include_bytes!("fixtures/calendar-v15-renderer2-prepared.json")).unwrap()["state"].clone() }
fn envelope(state: &Value, command: Value, id: u32) -> Value { json!({"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":command}) }
fn apply(state: &Value, command: Value, id: u32) -> Value { good(json!({"op":"dispatch","state":state,"command":envelope(state,command,id),"now":NOW}))["state"].clone() }
fn preview(state: &Value) -> Value { good(json!({"op":"calendar_preview","state":state,"order_id":state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],"destination_id":DEST,"now":NOW})) }
fn prepare(state: &Value, id: u32) -> Value {
    let material=preview(state);
    apply(state,json!({"kind":"request_calendar_publication","data":{"order_id":material["batch"]["order_id"],"destination_id":DEST,"event_batch_digest":material["event_batch_digest"]}}),id)
}
fn confirmation(state: &Value) -> Value {
    let receipt=state["receipts"].as_array().unwrap().iter().rev().find(|receipt|receipt["prepared_calendar"].is_object()).unwrap();
    let batch=&receipt["prepared_calendar"]["batch"];
    let events=batch["events"].as_array().unwrap().iter().enumerate().map(|(index,event)|json!({
        "event_id":event["event_id"],"provider_event_id":format!("synthetic-google-{index}"),
        "title":event["title"],"body":event["body"],"start":event["start"]["instant"],"end":event["end"]["instant"],
        "zone":event["start"]["zone"],"observed_at":NOW
    })).collect::<Vec<_>>();
    json!({"kind":"confirm_calendar_publication","data":{"confirmation":{
        "preparation_command_id":receipt["command"]["command_id"],"order_id":batch["order_id"],"destination_id":DEST,
        "event_batch_digest":receipt["command"]["command"]["data"]["event_batch_digest"],"calendar_id":"synthetic@example.invalid",
        "source":{"kind":"user_reviewed_google_read","data":{"read_id":"synthetic-read","read_digest":"a".repeat(64)}},
        "observed_at":NOW,"events":events
    }}})
}
fn cold(state: Value) -> Value { std::thread::spawn(move||good(json!({"op":"validate_state","state":state,"now":"2026-10-13T21:30:00Z"}))).join().unwrap() }
fn resign_last(state: &mut Value) {
    let receipt=state["receipts"].as_array_mut().unwrap().last_mut().unwrap();
    let command: AppCommandV1=serde_json::from_value(receipt["command"].clone()).unwrap();
    let response: Snapshot=serde_json::from_value(receipt["response"].clone()).unwrap();
    receipt["command_digest"]=json!(canonical_digest(&command).unwrap());receipt["response_digest"]=json!(canonical_digest(&response).unwrap());
    state["snapshot"]=serde_json::to_value(response).unwrap();
    for week in state["snapshot"]["weeks"].as_array().unwrap().clone() {
        let id=week["range"]["week_id"].clone();
        *state["retained_weeks"].as_array_mut().unwrap().iter_mut().find(|saved|saved["range"]["week_id"]==id).unwrap()=week;
    }
}

#[test]
fn complete_confirmation_is_one_receipt_and_survives_replay_cold_validation_and_new_observation() {
    let before=prepare(&fixture(),0xcf01);let command=envelope(&before,confirmation(&before),0xcf02);
    let result=good(json!({"op":"dispatch","state":before,"command":command,"now":NOW}));let next=result["state"].clone();
    assert_eq!(result["receipt"]["disposition"],"applied");
    let count=before["receipts"].as_array().unwrap().len();assert_eq!(next["receipts"].as_array().unwrap().len(),count+1);
    assert_eq!(&next["receipts"].as_array().unwrap()[..count],before["receipts"].as_array().unwrap());
    assert_eq!(next["snapshot"]["weeks"][0]["publication"],"published");
    let mut expected=before["snapshot"].clone();expected["snapshot_revision"]=next["snapshot"]["snapshot_revision"].clone();expected["weeks"][0]["publication"]=json!("published");
    assert_eq!(next["snapshot"],expected);
    for key in ["history","reviewed_packets","essentials","personal_recipes"] {assert_eq!(next[key],before[key],"{key}");}
    assert_eq!(preview(&next)["prepared"],true);assert_eq!(cold(next.clone()),next);
    let replay=good(json!({"op":"dispatch","state":next,"command":command,"now":"2026-10-13T21:30:00Z"}));
    assert_eq!(replay["state"],next);assert_eq!(replay["receipt"]["disposition"],"replayed");
    let observed_again=apply(&next,confirmation(&next),0xcf03);assert_eq!(observed_again["snapshot"]["weeks"][0]["publication"],"published");
    assert_eq!(cold(observed_again.clone()),observed_again);
}

#[test]
fn old_renderer_changed_content_and_resigned_receipts_cannot_claim_publication() {
    // Renderer 1 has genuinely changed text. Renderer 2's frozen batch is
    // identical to current output and must not be rejected for its version alone.
    let old=serde_json::from_slice::<Value>(include_bytes!("fixtures/calendar-v8-fictional-prepared.json")).unwrap()["state"].clone();
    issue(call(json!({"op":"dispatch","state":old,"command":envelope(&old,confirmation(&old),0xcf11),"now":NOW})),"/calendar/publication/current_renderer");
    let state=prepare(&old,0xcf12);let mut changed=confirmation(&state);changed["data"]["confirmation"]["events"][0]["title"]=json!("Different external title");
    issue(call(json!({"op":"dispatch","state":state,"command":envelope(&state,changed,0xcf13),"now":NOW})),"/calendar/publication/events/content");
    assert_eq!(state["snapshot"]["weeks"][0]["publication"],"prepared");assert_eq!(state["snapshot"]["weeks"][0]["confirmed_order"],old["snapshot"]["weeks"][0]["confirmed_order"]);
    let published=apply(&state,confirmation(&state),0xcf14);
    let mut forged=published.clone();forged["receipts"].as_array_mut().unwrap().last_mut().unwrap()["command"]["command"]["data"]["confirmation"]["events"][0]["title"]=json!("Forged content");resign_last(&mut forged);
    issue(call(json!({"op":"validate_state","state":forged,"now":NOW})),"/calendar/publication/events/content");
    let mut forged=published.clone();forged["receipts"].as_array_mut().unwrap().last_mut().unwrap()["response"]["profile"]["diners"][0]["display_name"]=json!("Forged person");resign_last(&mut forged);
    issue(call(json!({"op":"validate_state","state":forged,"now":NOW})),"/receipts/calendar_publication_response");
    let mut forged=published;forged["receipts"].as_array_mut().unwrap().last_mut().unwrap()["command"]["command"]["data"]["confirmation"]["observed_at"]=json!("2026-10-12T21:24:59Z");resign_last(&mut forged);
    issue(call(json!({"op":"validate_state","state":forged,"now":NOW})),"/calendar/publication/observed_at");
}

#[test]
fn delivery_and_profile_changes_invalidate_publication_without_changing_booked_order_or_history() {
    let state=prepare(&fixture(),0xcf21);let published=apply(&state,confirmation(&state),0xcf22);
    let moment=|hour:u8|good(json!({"op":"resolve_moment","date":"2026-10-19","time":format!("{hour:02}:00:00"),"zone":"Europe/London","resolution":"unambiguous"}));
    let corrected=apply(&published,json!({"kind":"correct_order_delivery","data":{"order_id":published["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],"delivery_start":moment(20),"delivery_end":moment(21)}}),0xcf23);
    assert_eq!(corrected["snapshot"]["weeks"][0]["publication"],"not_requested");assert_eq!(preview(&corrected)["prepared"],false);
    let profile=published["snapshot"]["profile"].clone();
    let updated=apply(&published,json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),0xcf24);
    assert_eq!(updated["snapshot"]["weeks"][0]["publication"],"needs_user");
    for next in [corrected,updated] {
        assert_eq!(next["snapshot"]["weeks"][0]["confirmed_order"],published["snapshot"]["weeks"][0]["confirmed_order"]);
        assert_eq!(next["history"],published["history"]);assert_eq!(next["snapshot"]["weeks"][0]["plan"],published["snapshot"]["weeks"][0]["plan"]);
        assert_eq!(cold(next.clone()),next);
        let old_command=published["receipts"].as_array().unwrap().last().unwrap()["command"].clone();
        let replay=good(json!({"op":"dispatch","state":next,"command":old_command,"now":NOW}));
        assert_eq!(replay["state"],next,"Replaying an old confirmation cannot reinstate publication");
        assert_eq!(replay["receipt"]["disposition"],"replayed");
    }
}
