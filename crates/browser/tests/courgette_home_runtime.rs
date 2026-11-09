//! Exact authored food through the browser owner. Households/orders are synthetic;
//! these checks do not observe physical cooking, cooling or household handling.
use mstrchf_browser::{maker_packets::*, run_json};
use mstrchf_contracts::*;
use serde_json::{Value, json};

const NOW: &str = "2026-10-14T12:00:00Z";
fn call(input: Value) -> Value {
    serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap()
}
fn good(input: Value) -> Value {
    let out = call(input);
    assert_eq!(out["ok"], true, "{out}");
    out["value"].clone()
}
fn command(state: &Value, body: Value, id: u32) -> Value {
    json!({"schema_version":1,"command_id":format!("cm_{id:032x}"),
        "expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body})
}
fn apply(state: &Value, body: Value, id: u32) -> Value {
    good(json!({"op":"dispatch","state":state,"command":command(state,body,id),"now":NOW}))["state"]
        .clone()
}
fn week_body(week: &Value) -> Value {
    json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{
        "week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],
        "day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],
        "cells":week["cells"],"plan":week["plan"]}}})
}
fn setup() -> Value {
    let mut state: Value = serde_json::from_slice::<Value>(include_bytes!(
        "fixtures/legacy-engineering-receipts.json"
    ))
    .unwrap()["state"]
        .clone();
    let mut profile = state["snapshot"]["profile"].clone();
    profile["diners"][0]["age_band"] = json!("adult");
    profile["diners"][0]["restrictions"] =
        json!({"kind":"confirmed_none","data":{"confirmed_at":NOW}});
    profile["diners"][0]["portions"] = json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});
    profile["preferences"]["equipment"] = json!(["fridge", "hob", "oven"]);
    state = apply(
        &state,
        json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),
        0xfb00,
    );
    let mut week = state["snapshot"]["weeks"][0].clone();
    week["plan"]["selections"] = json!([]);
    week["plan"]["batch_links"] = json!([]);
    week["meal_type_enabled"]["lunch"] = json!(true);
    week["meal_type_enabled"]["dinner"] = json!(true);
    apply(&state, week_body(&week), 0xfb01)
}
fn admission(state: &Value, packet: ReviewedRecipePacketRef) -> Value {
    json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,
        "expected_catalogue_revision":(1+state["reviewed_packets"].as_array().map_or(0,Vec::len)).to_string()}})
}
fn ready() -> Value {
    let state = setup();
    let state = apply(
        &state,
        admission(&state, courgette_hot_packet_ref().unwrap()),
        0xfb02,
    );
    apply(
        &state,
        admission(&state, courgette_cold_packet_ref().unwrap()),
        0xfb03,
    )
}
fn board(state: &Value, index: usize, options: Value) -> Value {
    let week = &state["snapshot"]["weeks"][0];
    good(
        json!({"op":"board","state":state,"week_id":week["range"]["week_id"],"slot":week["cells"][index]["slot"],"options":options,"now":NOW}),
    )
}
fn choices<'a>(board: &'a Value, reference: &RecipeRef) -> Vec<&'a Value> {
    board["choices"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|choice| choice["selection"]["recipe"] == json!(reference))
        .collect()
}
fn select(state: &Value, index: usize, reference: &RecipeRef, options: Value, id: u32) -> Value {
    let board = board(state, index, options);
    let found = choices(&board, reference);
    assert_eq!(found.len(), 1, "expected exact choice: {board}");
    let mut week = state["snapshot"]["weeks"][0].clone();
    week["plan"] = found[0]["plan"].clone();
    apply(state, week_body(&week), id)
}
fn source_options(state: &Value) -> Value {
    json!({"leftover_source":state["snapshot"]["weeks"][0]["cells"][5]["slot"]})
}
fn dinner() -> Value {
    select(
        &ready(),
        5,
        &courgette_hot_packet_ref().unwrap().recipe,
        Value::Null,
        0xfb04,
    )
}
fn saved() -> Value {
    let state = dinner();
    select(
        &state,
        7,
        &courgette_cold_packet_ref().unwrap().recipe,
        source_options(&state),
        0xfb05,
    )
}
fn cold_reload(state: Value) -> Value {
    std::thread::spawn(move || good(json!({"op":"validate_state","state":state,"now":NOW})))
        .join()
        .unwrap()
}

#[test]
fn original_source_must_be_admitted_before_its_cold_service() {
    let state = setup();
    let denied = call(json!({"op":"dispatch","state":state,
        "command":command(&state,admission(&state,courgette_cold_packet_ref().unwrap()),0xfb10),"now":NOW}));
    assert_eq!(denied["ok"], false);
    let ready = ready();
    assert_eq!(ready["reviewed_packets"].as_array().unwrap().len(), 2);
    assert_eq!(cold_reload(ready.clone()), ready);
}

#[test]
fn one_dinner_and_one_cold_lunch_use_one_batch_seven_demands_and_original_clock() {
    let before = ready();
    let state = saved();
    let board = board(&state, 7, source_options(&state));
    let derived = &board["details"]["derived"];
    assert!(derived.is_object(), "{board}");
    assert_eq!(derived["demand"]["lines"].as_array().unwrap().len(), 7);
    assert_eq!(derived["batches"].as_array().unwrap().len(), 1);
    let batch = &derived["batches"][0];
    assert_eq!(
        batch["recipe"],
        json!(courgette_hot_packet_ref().unwrap().recipe)
    );
    let allocations = derived["allocations"].as_array().unwrap();
    assert_eq!(allocations.len(), 2);
    assert!(allocations.iter().all(|row| row["component_id"]
        == "co_8000000000000000000000000000000b"
        && row["planned_reheats"] == 0));
    let tasks: Vec<&Value> = derived["prep_sessions"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|session| session["tasks"].as_array().unwrap())
        .collect();
    assert_eq!(tasks.len(), 7);
    let cooking = tasks
        .iter()
        .find(|task| {
            task["recipe"] == json!(courgette_hot_packet_ref().unwrap().recipe)
                && task["prep_id"] == 3
        })
        .unwrap();
    assert_eq!(
        batch["components"][0]["original_production"]["data"]["completed_at"],
        cooking["elapsed"]["end"]
    );
    let plan = &state["snapshot"]["weeks"][0]["plan"];
    assert_eq!(
        plan["selections"][0]["batch_use"]["data"]["production_servings"],
        json!({"numerator":"4","denominator":"1"})
    );
    assert_eq!(plan["batch_links"].as_array().unwrap().len(), 1);
    assert_eq!(
        plan["batch_links"][0]["components"][0]["quantity"],
        json!({"amount":{"numerator":"1","denominator":"1"},"unit":"each","form":"cooked"})
    );
    assert_eq!(
        &state["snapshot"]["weeks"].as_array().unwrap()[1..],
        &before["snapshot"]["weeks"].as_array().unwrap()[1..]
    );
    assert_eq!(state["snapshot"]["profile"], before["snapshot"]["profile"]);
    assert_eq!(cold_reload(state.clone()), state);
}

#[test]
fn cold_service_requires_selected_original_source_next_day_and_no_extra_batch() {
    let state = dinner();
    let cold = courgette_cold_packet_ref().unwrap().recipe;
    assert!(choices(&board(&state, 7, Value::Null), &cold).is_empty());
    assert!(choices(&board(&state, 4, source_options(&state)), &cold).is_empty());
    assert!(choices(&board(&state, 10, source_options(&state)), &cold).is_empty());
    let mut options = source_options(&state);
    options["make_extra"] = json!(true);
    assert!(choices(&board(&state, 7, options), &cold).is_empty());
    let mut options = source_options(&state);
    options["leftover_source"] = state["snapshot"]["weeks"][0]["cells"][2]["slot"].clone();
    assert!(choices(&board(&state, 7, options), &cold).is_empty());
}
