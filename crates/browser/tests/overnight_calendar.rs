//! Real reviewed breakfast; fictional household, delivery and calendar only.
use mstrchf_browser::{calendar_owner, calendar_view::material_at,
    catalogue::RuntimeCatalogue, maker_packets::*, run_json, BrowserState};
use mstrchf_contracts::*;
use serde_json::{json,Value};

const NOW:&str="2026-10-14T16:00:00Z";
const DEST:&str="cd_00000000000000000000000000000001";
fn good(input:Value)->Value {
    let out:Value=serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap();
    assert_eq!(out["ok"],true,"{out}");out["value"].clone()
}
fn apply(state:&Value,body:Value,id:u32)->Value {
    good(json!({"op":"dispatch","state":state,"now":NOW,"command":{"schema_version":1,
        "command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body}}))["state"].clone()
}
fn save(state:&Value,week:Value,id:u32)->Value {
    apply(state,json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{
        "week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],"day_enabled":week["day_enabled"],
        "meal_type_enabled":week["meal_type_enabled"],"cells":week["cells"],"plan":week["plan"]}}}),id)
}
fn moment(date:&str,time:&str)->Value {
    good(json!({"op":"resolve_moment","date":date,"time":time,"zone":"Europe/London","resolution":"unambiguous"}))
}
fn fixture()->Value {
    let mut state=serde_json::from_slice::<Value>(include_bytes!("fixtures/legacy-engineering-receipts.json")).unwrap()["state"].clone();
    let mut profile=state["snapshot"]["profile"].clone();
    profile["diners"][0]["age_band"]=json!("adult");
    profile["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":NOW}});
    profile["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});
    profile["preferences"]["equipment"]=json!(["fridge"]);
    profile["preferences"]["mobility"]=json!({"temperature":"cold","reheating":false,"cold_storage":false});
    state=apply(&state,json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),0xde01);
    let mut week=state["snapshot"]["weeks"][0].clone();
    week["plan"]["selections"]=json!([]);week["plan"]["batch_links"]=json!([]);
    week["day_enabled"]=json!([true,true,true,true,true,true,true]);
    week["meal_type_enabled"]=json!({"breakfast":true,"lunch":true,"dinner":true});
    week["cells"][0]["choice"]=json!("at_home");week["cells"][0]["attendee_ids"]=json!([profile["diners"][0]["diner_id"]]);
    week["cells"][0]["occasion"]=Value::Null;
    state=save(&state,week,0xde02);
    let packet=overnight_oats_packet_ref().unwrap();
    state=apply(&state,json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":"1"}}),0xde03);
    let week=&state["snapshot"]["weeks"][0];
    let board=good(json!({"op":"board","state":state,"week_id":week["range"]["week_id"],"slot":week["cells"][0]["slot"],
        "options":{"eat_by_time":"08:00:00"},"now":NOW}));
    let choice=board["choices"].as_array().unwrap().iter().find(|choice|choice["selection"]["recipe"]==json!(packet.recipe))
        .unwrap_or_else(||panic!("Missing overnight choice: {}",board["details"]["food_error"]));
    let mut week=week.clone();week["plan"]=choice["plan"].clone();state=save(&state,week,0xde04);
    let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();
    let view=RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();
    let (_,food)=material_at(&view,&typed.snapshot,typed.snapshot.profile.0.as_ref().unwrap(),&typed.snapshot.weeks[0],&Instant(NOW.into())).unwrap().unwrap();
    state=apply(&state,json!({"kind":"approve_plan","data":{"week_id":food.plan.week_id,"plan_revision":food.plan.plan_revision,
        "derived_digest":food.plan.digest}}),0xde05);
    let week=&state["snapshot"]["weeks"][0];
    // Existing fixture week begins Monday19October. Arrival ends exactly when
    // preceding-Sunday mixing starts; this boundary is intentionally allowed.
    apply(&state,json!({"kind":"confirm_order","data":{"confirmation":{"week_id":week["range"]["week_id"],
        "approval_id":week["approved_plan"]["approval_id"],"delivery_start":moment("2026-10-18","19:00:00"),
        "delivery_end":moment("2026-10-18","19:55:00"),"confirmed_at":NOW,"source":{"kind":"user_confirmation","data":{}}}}}),0xde06)
}
fn prepare(state:&Value)->Result<(calendar_owner::CalendarJournal,String),DomainError> {
    let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();
    calendar_owner::prepare(&typed,&typed.snapshot.weeks[0].confirmed_order.0.as_ref().unwrap().order_id,
        &CalendarDestinationId(DEST.into()),&Instant(NOW.into()))
}
fn persist(state:&Value,id:u32)->Value {
    let (journal,_)=prepare(state).unwrap();
    apply(state,json!({"kind":"request_calendar_publication","data":{"order_id":state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],
        "destination_id":DEST,"event_batch_digest":canonical_digest(&journal.batch).unwrap()}}),id)
}
#[test]
fn preceding_sunday_and_monday_have_separate_readable_methods_and_original_clock() {
    let state=fixture();let before=state.clone();let (journal,ics)=prepare(&state).unwrap();
    assert_eq!(journal.renderer_version,13);assert_eq!(journal.saved_events.len(),3);
    let foods=journal.saved_events.iter().filter(|row|row.event.logical_key.kind!=CalendarEventKind::Delivery).collect::<Vec<_>>();
    assert_eq!(foods.len(),2);
    let prep=foods.iter().find(|row|row.event.title.0.starts_with("MasterChef TEST · Mix and refrigerate")).unwrap();
    let service=foods.iter().find(|row|row.event.title.0.starts_with("MasterChef TEST · Check and serve")).unwrap();
    assert_eq!(prep.event.start.local_date.0,"2026-10-18");assert_eq!(prep.event.start.local_time.0,"19:55:00");
    assert_eq!(prep.event.end.local_time.0,"20:00:00");
    assert_eq!(service.event.start.local_date.0,"2026-10-19");assert_eq!(service.event.start.local_time.0,"07:55:00");
    assert_eq!(service.event.end.local_time.0,"08:00:00");
    let recipe=known_packet(&overnight_oats_packet_ref().unwrap()).unwrap();
    let all=foods.iter().map(|row|row.event.body.0.as_str()).collect::<Vec<_>>().join("\n");
    for step in recipe.recipe().steps.iter(){assert_eq!(all.matches(&step.text.0).count(),1);}
    assert!(all.contains("Original storage clock"));assert!(all.contains("not an occupied cooking session"));
    assert!(all.contains("0–5°C"));assert!(ics.contains("BEGIN:VCALENDAR"));assert_eq!(state,before);
    let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();
    for renderer in 1..=12 {assert!(calendar_owner::prepare_for_renderer(&typed,&typed.snapshot.weeks[0].confirmed_order.0.as_ref().unwrap().order_id,
        &CalendarDestinationId(DEST.into()),&Instant(NOW.into()),renderer).is_err());}
    let saved=persist(&state,0xde07);
    assert_eq!(good(json!({"op":"validate_state","state":saved,"now":NOW})),saved);
}
#[test]
fn one_second_late_delivery_blocks_calendar_but_preserves_confirmed_order() {
    let state=fixture();let before=state["snapshot"]["weeks"][0]["confirmed_order"].clone();
    let late=apply(&state,json!({"kind":"correct_order_delivery","data":{"order_id":before["order_id"],
        "delivery_start":moment("2026-10-18","19:00:00"),"delivery_end":moment("2026-10-18","19:55:01")}}),0xde08);
    assert!(prepare(&late).is_err());assert_eq!(late["snapshot"]["weeks"][0]["confirmed_order"],before);
    assert_eq!(good(json!({"op":"validate_state","state":late,"now":NOW})),late);
}
#[test]
fn corrected_delivery_keeps_food_events_and_prior_calendar_history() {
    let state=persist(&fixture(),0xde07);
    let old=state["receipts"].as_array().unwrap().last().unwrap()["prepared_calendar"].clone();
    let changed=apply(&state,json!({"kind":"correct_order_delivery","data":{
        "order_id":state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],
        "delivery_start":moment("2026-10-18","18:00:00"),"delivery_end":moment("2026-10-18","19:00:00")}}),0xde09);
    let (journal,_)=prepare(&changed).unwrap();assert_eq!(journal.renderer_version,14);
    for row in journal.saved_events.iter().filter(|row|row.event.logical_key.kind!=CalendarEventKind::Delivery) {
        let prior=old["saved_events"].as_array().unwrap().iter().find(|prior|prior["event"]["event_id"]==json!(row.event.event_id)).unwrap();
        assert_eq!(json!(row),*prior);
    }
    let saved=persist(&changed,0xde10);
    assert_eq!(saved["receipts"][state["receipts"].as_array().unwrap().len()-1]["prepared_calendar"],old);
    assert_eq!(good(json!({"op":"validate_state","state":saved,"now":NOW})),saved);
}
#[test]
fn historical_plans_cannot_claim_overnight_renderer() {
    let state=serde_json::from_slice::<Value>(include_bytes!("fixtures/calendar-v15-renderer2-prepared.json")).unwrap()["state"].clone();
    let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();
    for renderer in [13,14] {assert!(calendar_owner::prepare_for_renderer(&typed,&typed.snapshot.weeks[0].confirmed_order.0.as_ref().unwrap().order_id,
        &CalendarDestinationId(DEST.into()),&Instant(NOW.into()),renderer).is_err());}
    assert_eq!(json!(typed),state);
}
