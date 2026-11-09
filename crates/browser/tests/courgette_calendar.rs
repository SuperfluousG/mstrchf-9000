//! Exact compiled recipes and real native/calendar custody, using fictional
//! household/order inputs. No physical cooling or calendar import is observed.
use mstrchf_browser::{calendar_owner, calendar_view::material_at, catalogue::RuntimeCatalogue,
    maker_packets::*, run_json, BrowserState};
use mstrchf_contracts::*;
use serde_json::{json, Value};

const NOW:&str="2026-10-14T12:00:00Z";
const DEST:&str="cd_00000000000000000000000000000001";
fn good(input:Value)->Value {
    let out:Value=serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap();
    assert_eq!(out["ok"],true,"{out}"); out["value"].clone()
}
fn apply(state:&Value,body:Value,id:u32)->Value {
    good(json!({"op":"dispatch","state":state,"now":NOW,"command":{"schema_version":1,
        "command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body}}))["state"].clone()
}
fn save(state:&Value,week:Value,id:u32)->Value {
    apply(state,json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{
        "week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],
        "day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],
        "cells":week["cells"],"plan":week["plan"]}}}),id)
}
fn select(state:&Value,index:usize,reference:RecipeRef,options:Value,id:u32)->Value {
    let week=&state["snapshot"]["weeks"][0];
    let board=good(json!({"op":"board","state":state,"week_id":week["range"]["week_id"],
        "slot":week["cells"][index]["slot"],"options":options,"now":NOW}));
    let choice=board["choices"].as_array().unwrap().iter().find(|choice|choice["selection"]["recipe"]==json!(reference))
        .unwrap_or_else(||panic!("missing exact choice: {board}"));
    let mut week=week.clone();week["plan"]=choice["plan"].clone();save(state,week,id)
}
fn moment(hour:u8)->Value {
    good(json!({"op":"resolve_moment","date":"2026-10-14","time":format!("{hour:02}:00:00"),"zone":"Europe/London","resolution":"unambiguous"}))
}
fn fixture(hot:usize,cold:usize)->Value {
    let mut state=serde_json::from_slice::<Value>(include_bytes!("fixtures/legacy-engineering-receipts.json")).unwrap()["state"].clone();
    let mut profile=state["snapshot"]["profile"].clone();
    profile["diners"][0]["age_band"]=json!("adult");
    profile["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":NOW}});
    profile["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});
    profile["preferences"]["equipment"]=json!(["fridge","hob","oven"]);
    profile["preferences"]["mobility"]["temperature"]=json!("hot");
    let first=profile["diners"][0].clone();
    for index in 1..hot.max(cold) {
        let mut diner=first.clone();diner["diner_id"]=json!(format!("dn_{:032x}",0xfc00+index));
        profile["diners"].as_array_mut().unwrap().push(diner);
    }
    state=apply(&state,json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),0xfc01);
    let people=profile["diners"].as_array().unwrap().iter().map(|diner|diner["diner_id"].clone()).collect::<Vec<_>>();
    let mut week=state["snapshot"]["weeks"][0].clone();
    week["plan"]["selections"]=json!([]);week["plan"]["batch_links"]=json!([]);
    week["day_enabled"]=json!([true,true,true,true,true,true,true]);
    week["meal_type_enabled"]["lunch"]=json!(true);week["meal_type_enabled"]["dinner"]=json!(true);
    for (index,count) in [(8,hot),(10,cold.max(1))] {
        week["cells"][index]["choice"]=json!("at_home");
        week["cells"][index]["attendee_ids"]=json!(&people[..count]);
    }
    state=save(&state,week,0xfc02);
    for (index,packet) in [courgette_hot_packet_ref().unwrap(),courgette_cold_packet_ref().unwrap()].into_iter().enumerate() {
        state=apply(&state,json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":(index+1).to_string()}}),0xfc03+index as u32);
    }
    state=select(&state,8,courgette_hot_packet_ref().unwrap().recipe,Value::Null,0xfc05);
    if cold>0 {
        let options=json!({"leftover_source":state["snapshot"]["weeks"][0]["cells"][8]["slot"]});
        state=select(&state,10,courgette_cold_packet_ref().unwrap().recipe,options,0xfc06);
    }
    let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();
    let view=RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();
    let (_,food)=material_at(&view,&typed.snapshot,typed.snapshot.profile.0.as_ref().unwrap(),&typed.snapshot.weeks[0],&Instant(NOW.into())).unwrap().unwrap();
    state=apply(&state,json!({"kind":"approve_plan","data":{"week_id":food.plan.week_id,"plan_revision":food.plan.plan_revision,"derived_digest":food.plan.digest}}),0xfc07);
    let week=&state["snapshot"]["weeks"][0];
    apply(&state,json!({"kind":"confirm_order","data":{"confirmation":{"week_id":week["range"]["week_id"],
        "approval_id":week["approved_plan"]["approval_id"],"delivery_start":moment(14),"delivery_end":moment(15),
        "confirmed_at":NOW,"source":{"kind":"user_confirmation","data":{}}}}}),0xfc08)
}
fn prepare(state:&Value)->(calendar_owner::CalendarJournal,String) {
    let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();
    let order=typed.snapshot.weeks[0].confirmed_order.0.as_ref().unwrap();
    calendar_owner::prepare(&typed,&order.order_id,&CalendarDestinationId(DEST.into()),&Instant(NOW.into())).unwrap()
}
fn persist(state:&Value,id:u32)->Value {
    let (journal,_)=prepare(state);
    apply(state,json!({"kind":"request_calendar_publication","data":{
        "order_id":state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],"destination_id":DEST,
        "event_batch_digest":canonical_digest(&journal.batch).unwrap()}}),id)
}

#[test]
fn source_one_cold_two_calendar_has_original_clock_complete_methods_and_zero_fresh_cold_demand() {
    let state=fixture(1,2);let before=state.clone();let (journal,_)=prepare(&state);
    assert_eq!(journal.renderer_version,5);assert_eq!(journal.saved_events.len(),4);
    let cold=journal.saved_events.iter().find(|row|row.event.body.0.contains("This cold service uses original cooked shares only")).unwrap();
    assert!(cold.event.body.0.contains("allocated for this meal: 2"));
    assert!(cold.event.body.0.contains("Original storage clock"));
    assert!(!cold.event.body.0.contains("Ingredients — total for this occurrence"));
    let combined=journal.saved_events.iter().map(|row|row.event.body.0.as_str()).collect::<Vec<_>>().join("\n");
    for text in ["400 g","250 g","4000 ml","50 g","Hot meal service (cooling of remaining shares follows when scheduled)"] {
        assert!(combined.contains(text),"missing {text}");
    }
    assert!(combined.contains("not continuous hands-on work"),"the passive phases must not be relabelled as full attention");
    assert_eq!(state,before);
    let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();let order=typed.snapshot.weeks[0].confirmed_order.0.as_ref().unwrap();
    for renderer in 1..=4 {assert!(calendar_owner::prepare_for_renderer(&typed,&order.order_id,&CalendarDestinationId(DEST.into()),&Instant(NOW.into()),renderer).is_err());}
    let saved=persist(&state,0xfc09);
    assert_eq!(good(json!({"op":"validate_state","state":saved,"now":NOW})),saved);
}

#[test]
fn all_four_hot_calendar_omits_unexecuted_cooling_instructions() {
    let state=fixture(4,0);let (journal,_)=prepare(&state);
    assert_eq!(journal.renderer_version,5);assert_eq!(journal.saved_events.len(),2);
    let cooking=journal.saved_events.iter().find(|row|row.event.logical_key.kind!=CalendarEventKind::Delivery).unwrap();
    assert!(cooking.event.body.0.contains("allocated for this meal: 4"));
    assert!(!cooking.event.body.0.contains("Post-service shallow-container portioning"));
    assert!(!cooking.event.body.0.contains("Initial room-temperature cooling reservation"));
    assert!(!cooking.event.body.0.contains("Cooling check and fridge put-away"));
}

#[test]
fn corrected_original_pair_uses_renderer_six_and_keeps_food_events_and_old_receipt_exact() {
    let state=persist(&fixture(1,1),0xfc09);
    let old=state["receipts"].as_array().unwrap().last().unwrap()["prepared_calendar"].clone();
    let changed=apply(&state,json!({"kind":"correct_order_delivery","data":{
        "order_id":state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],"delivery_start":moment(15),"delivery_end":moment(16)}}),0xfc10);
    let (journal,_)=prepare(&changed);assert_eq!(journal.renderer_version,6);
    for row in journal.saved_events.iter().filter(|row|row.event.logical_key.kind!=CalendarEventKind::Delivery) {
        let prior=old["saved_events"].as_array().unwrap().iter().find(|prior|prior["event"]["event_id"]==json!(row.event.event_id)).unwrap();
        assert_eq!(json!(row),*prior,"food event bytes must remain unchanged");
    }
    let saved=persist(&changed,0xfc11);
    assert_eq!(saved["receipts"][state["receipts"].as_array().unwrap().len()-1]["prepared_calendar"],old);
    assert_eq!(saved["snapshot"]["weeks"][0]["confirmed_order"],state["snapshot"]["weeks"][0]["confirmed_order"]);
    assert_eq!(good(json!({"op":"validate_state","state":saved,"now":NOW})),saved);
}

#[test]
fn new_renderers_do_not_expand_unrelated_historical_calendar_scope() {
    let state=serde_json::from_slice::<Value>(include_bytes!("fixtures/calendar-v15-renderer2-prepared.json")).unwrap()["state"].clone();
    let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();
    let order=typed.snapshot.weeks[0].confirmed_order.0.as_ref().unwrap();
    for renderer in [5,6] {
        assert!(calendar_owner::prepare_for_renderer(&typed,&order.order_id,
            &CalendarDestinationId(DEST.into()),&Instant(NOW.into()),renderer).is_err());
    }
    assert_eq!(json!(typed),state);
}
