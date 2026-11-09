//! Three real recipe definitions; fictional household, delivery and calendar.
//! These checks observe planning/custody, not physical cooking or handling.
use mstrchf_browser::{calendar_owner, calendar_view::material_at,
    catalogue::RuntimeCatalogue, maker_packets::*, run_json, BrowserState};
use mstrchf_contracts::*;
use serde_json::{json, Value};

const NOW: &str = "2026-10-14T15:00:00Z";
const DEST: &str = "cd_00000000000000000000000000000001";
fn good(input: Value) -> Value {
    let out: Value = serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap();
    assert_eq!(out["ok"], true, "{out}"); out["value"].clone()
}
fn apply(state: &Value, body: Value, id: u32) -> Value {
    good(json!({"op":"dispatch","state":state,"now":NOW,"command":{"schema_version":1,
        "command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],
        "command":body}}))["state"].clone()
}
fn save(state: &Value, week: Value, id: u32) -> Value {
    apply(state,json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{
        "week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],
        "day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],
        "cells":week["cells"],"plan":week["plan"]}}}),id)
}
fn select(state: &Value, index: usize, recipe: RecipeRef, options: Value, id: u32) -> Value {
    let week=&state["snapshot"]["weeks"][0];
    let board=good(json!({"op":"board","state":state,"week_id":week["range"]["week_id"],
        "slot":week["cells"][index]["slot"],"options":options,"now":NOW}));
    let choice=board["choices"].as_array().unwrap().iter().find(|choice|choice["selection"]["recipe"]==json!(recipe))
        .unwrap_or_else(||panic!("Missing exact chilli choice {:?}; count={}; food_error={}",
            recipe.recipe_id,board["choices"].as_array().unwrap().len(),board["details"]["food_error"]));
    let mut week=week.clone(); week["plan"]=choice["plan"].clone(); save(state,week,id)
}
fn moment(hour: u8) -> Value {
    good(json!({"op":"resolve_moment","date":"2026-10-14","time":format!("{hour:02}:00:00"),
        "zone":"Europe/London","resolution":"unambiguous"}))
}
fn fixture(swap: bool) -> Value {
    let mut state=serde_json::from_slice::<Value>(include_bytes!("fixtures/legacy-engineering-receipts.json")).unwrap()["state"].clone();
    let mut profile=state["snapshot"]["profile"].clone();
    profile["diners"][0]["age_band"]=json!("adult");
    profile["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":NOW}});
    profile["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});
    profile["preferences"]["equipment"]=json!(["fridge","hob","oven"]);
    // Home equipment supplies this home recipe; travel settings stay off.
    profile["preferences"]["mobility"]=json!({"temperature":"cold","reheating":false,"cold_storage":false});
    state=apply(&state,json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),0xdc01);
    let mut week=state["snapshot"]["weeks"][0].clone();
    week["plan"]["selections"]=json!([]); week["plan"]["batch_links"]=json!([]);
    week["day_enabled"]=json!([true,true,true,true,true,true,true]);
    week["meal_type_enabled"]=json!({"breakfast":true,"lunch":true,"dinner":true});
    for index in [8,10,13] {
        week["cells"][index]["choice"]=json!("at_home");
        week["cells"][index]["attendee_ids"]=json!([profile["diners"][0]["diner_id"]]);
        week["cells"][index]["occasion"]=Value::Null;
    }
    state=save(&state,week,0xdc02);
    for (index,packet) in [chilli_batch_packet_ref().unwrap(),chilli_wraps_packet_ref().unwrap(),chilli_potato_packet_ref().unwrap()].into_iter().enumerate() {
        state=apply(&state,json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":(index+1).to_string()}}),0xdc03+index as u32);
    }
    state=select(&state,8,chilli_batch_packet_ref().unwrap().recipe,Value::Null,0xdc06);
    let source=state["snapshot"]["weeks"][0]["cells"][8]["slot"].clone();
    let services=if swap {[chilli_potato_packet_ref().unwrap(),chilli_wraps_packet_ref().unwrap()]}
        else {[chilli_wraps_packet_ref().unwrap(),chilli_potato_packet_ref().unwrap()]};
    for ((index,recipe),id) in [10,13].into_iter().zip(services).zip([0xdc07,0xdc08]) {
        state=select(&state,index,recipe.recipe,json!({"leftover_source":source}),id);
    }
    let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();
    let view=RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();
    let (_,food)=material_at(&view,&typed.snapshot,typed.snapshot.profile.0.as_ref().unwrap(),&typed.snapshot.weeks[0],&Instant(NOW.into())).unwrap().unwrap();
    state=apply(&state,json!({"kind":"approve_plan","data":{"week_id":food.plan.week_id,
        "plan_revision":food.plan.plan_revision,"derived_digest":food.plan.digest}}),0xdc09);
    let week=&state["snapshot"]["weeks"][0];
    apply(&state,json!({"kind":"confirm_order","data":{"confirmation":{"week_id":week["range"]["week_id"],
        "approval_id":week["approved_plan"]["approval_id"],"delivery_start":moment(14),"delivery_end":moment(15),
        "confirmed_at":NOW,"source":{"kind":"user_confirmation","data":{}}}}}),0xdc10)
}
fn prepare(state: &Value) -> (calendar_owner::CalendarJournal,String) {
    let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();
    calendar_owner::prepare(&typed,&typed.snapshot.weeks[0].confirmed_order.0.as_ref().unwrap().order_id,
        &CalendarDestinationId(DEST.into()),&Instant(NOW.into())).unwrap()
}
fn persist(state: &Value, id: u32) -> Value {
    let (journal,_)=prepare(state);
    apply(state,json!({"kind":"request_calendar_publication","data":{
        "order_id":state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],
        "destination_id":DEST,"event_batch_digest":canonical_digest(&journal.batch).unwrap()}}),id)
}

#[test]
fn original_dinner_and_two_distinct_remixes_keep_clock_fresh_inputs_and_methods() {
    let state=fixture(false); let before=state.clone(); let (journal,ics)=prepare(&state);
    assert_eq!(journal.renderer_version,11);
    let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();
    let view=RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();
    let (_,food)=material_at(&view,&typed.snapshot,typed.snapshot.profile.0.as_ref().unwrap(),&typed.snapshot.weeks[0],&Instant(NOW.into())).unwrap().unwrap();
    assert_eq!(food.plan.batches.len(),1,"Remixes must not create new storage batches");
    let batch=&food.plan.batches[0]; let component=&batch.components[0];
    assert_eq!(component.produced.amount,PositiveRational::new(4,1).unwrap());
    assert_eq!(component.allocated.amount,PositiveRational::new(3,1).unwrap());
    assert_eq!(component.unallocated.amount,Rational::new(1,1).unwrap());
    assert_eq!(food.plan.allocations.len(),3);
    assert_eq!(food.plan.allocations.iter().filter(|allocation|allocation.planned_reheats==1).count(),2);
    assert_eq!(food.plan.demand.lines.len(),14,"12 source inputs and two distinct fresh additions");
    let all=journal.saved_events.iter().map(|row|row.event.body.0.as_str()).collect::<Vec<_>>().join("\n");
    for reference in [chilli_batch_packet_ref().unwrap(),chilli_wraps_packet_ref().unwrap(),chilli_potato_packet_ref().unwrap()] {
        let recipe=known_packet(&reference).unwrap().recipe();
        for ingredient in recipe.ingredients.iter() {
            let lines=food.plan.demand.lines.iter().filter(|line|line.ingredient_id==ingredient.ingredient_id).collect::<Vec<_>>();
            assert_eq!(lines.len(),1); assert_eq!(lines[0].quantity,ingredient.quantity.0.clone().unwrap());
        }
        for step in recipe.steps.iter() {assert_eq!(all.matches(&step.text.0).count(),1,"Every executed method step exactly once");}
    }
    assert!(all.contains("Original storage clock"));
    assert!(all.contains("original cooking clock begins before weighing"));
    assert!(all.contains("Reheat only that allocated share once"));
    assert!(all.contains("no new storage entitlement"));
    assert!(ics.contains("BEGIN:VCALENDAR")); assert_eq!(state,before);
    let order=typed.snapshot.weeks[0].confirmed_order.0.as_ref().unwrap();
    for version in 1..=10 {assert!(calendar_owner::prepare_for_renderer(&typed,&order.order_id,
        &CalendarDestinationId(DEST.into()),&Instant(NOW.into()),version).is_err());}
    let saved=persist(&state,0xdc11);
    let reloaded=std::thread::spawn(move||good(json!({"op":"validate_state","state":saved,"now":NOW}))).join().unwrap();
    assert_eq!(reloaded["history"],before["history"]);
    assert_eq!(reloaded["snapshot"]["weeks"][0]["confirmed_order"],before["snapshot"]["weeks"][0]["confirmed_order"]);
}

#[test]
fn services_can_swap_days_without_changing_original_quantity_or_clock() {
    let state=fixture(true); let (journal,_)=prepare(&state);
    assert_eq!(journal.renderer_version,11);
    let text=journal.saved_events.iter().map(|row|row.event.body.0.as_str()).collect::<Vec<_>>().join("\n");
    assert!(text.contains("Reheat only that allocated share once"));
    assert_eq!(state["snapshot"]["weeks"][0]["plan"]["batch_links"].as_array().unwrap().len(),2);
}

#[test]
fn corrected_delivery_uses12_and_preserves_food_events_and_historical_journal() {
    let state=persist(&fixture(false),0xdc11);
    let old=state["receipts"].as_array().unwrap().last().unwrap()["prepared_calendar"].clone();
    let changed=apply(&state,json!({"kind":"correct_order_delivery","data":{
        "order_id":state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],
        "delivery_start":moment(13),"delivery_end":moment(14)}}),0xdc12);
    let (journal,_)=prepare(&changed); assert_eq!(journal.renderer_version,12);
    for row in journal.saved_events.iter().filter(|row|row.event.logical_key.kind!=CalendarEventKind::Delivery) {
        let prior=old["saved_events"].as_array().unwrap().iter().find(|prior|prior["event"]["event_id"]==json!(row.event.event_id)).unwrap();
        assert_eq!(json!(row),*prior);
    }
    let saved=persist(&changed,0xdc13);
    assert_eq!(saved["receipts"][state["receipts"].as_array().unwrap().len()-1]["prepared_calendar"],old);
    assert_eq!(good(json!({"op":"validate_state","state":saved,"now":NOW})),saved);
}

#[test]
fn historical_calendars_cannot_claim_new_renderer_without_new_family() {
    let state=serde_json::from_slice::<Value>(include_bytes!("fixtures/calendar-v15-renderer2-prepared.json")).unwrap()["state"].clone();
    let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();
    for version in [11,12] {assert!(calendar_owner::prepare_for_renderer(&typed,
        &typed.snapshot.weeks[0].confirmed_order.0.as_ref().unwrap().order_id,
        &CalendarDestinationId(DEST.into()),&Instant(NOW.into()),version).is_err());}
    assert_eq!(json!(typed),state);
}
