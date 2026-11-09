//! Regenerates synthetic evidence solely through the actual public browser API.
//! No receipt/projection is fabricated or patched after admission.
use mstrchf_browser::run_json;
use serde_json::{Value, json};
const FIRST: &str = "2026-10-12T12:00:00Z";
const LATER: &str = "2026-10-19T12:00:00Z";
fn call(value: Value) -> Value {
    let v: Value = serde_json::from_slice(&run_json(&serde_json::to_vec(&value).unwrap())).unwrap();
    assert_eq!(v["ok"], true, "{v}");
    v["value"].clone()
}
fn command(state: &Value, body: Value, number: u32, now: &str) -> Value {
    let envelope = json!({"schema_version":1,"command_id":format!("cm_{number:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body});
    call(json!({"op":"dispatch","state":state,"command_json":serde_json::to_string(&envelope).unwrap(),"command":envelope,"now":now}))["state"].clone()
}
fn week_edit(w: &Value, plan: Value) -> Value {
    json!({"kind":"save_week","data":{"expected_week_revision":w["revision"],"week":{"week_id":w["range"]["week_id"],"defaults_snapshot":w["defaults_snapshot"],"day_enabled":[true,true,true,true,true,true,true],"meal_type_enabled":{"breakfast":true,"lunch":true,"dinner":true},"cells":w["cells"],"plan":plan}}})
}
fn fill(mut state: Value, week_id: &str, number: &mut u32, now: &str) -> Value {
    eprintln!("admitting 21 synthetic meals for {week_id}; command starts at {number}");
    let w = state["snapshot"]["weeks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["range"]["week_id"] == week_id)
        .unwrap()
        .clone();
    state = command(&state, week_edit(&w, Value::Null), *number, now);
    *number += 1;
    let w = state["snapshot"]["weeks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| w["range"]["week_id"] == week_id)
        .unwrap()
        .clone();
    let mut plan = w["plan"].clone();
    let mut selections = Vec::new();
    for cell in w["cells"].as_array().unwrap() {
        let board = call(
            json!({"op":"board","state":state,"week_id":week_id,"slot":cell["slot"],"now":now}),
        );
        let choice = board["choices"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| {
                c["selection"]["recipe"]["recipe_id"] == "rc_00000000000000000000000000000005"
            })
            .expect("synthetic general-meal engineering recipe is eligible");
        selections.push(choice["selection"].clone());
    }
    plan["selections"] = json!(selections);
    state = command(&state, week_edit(&w, plan), *number, now);
    *number += 1;
    eprintln!(
        "admitted {week_id}; revision {}",
        state["snapshot"]["snapshot_revision"]
    );
    state
}

#[test]
#[ignore = "regenerate the bounded archived-conflict fixture through actual API commands"]
fn all_110_conflicts_are_readable_after_archive_rollover_and_restore() {
    let mut state = call(
        json!({"op":"initialize","household_id":"hh_00000000000000000000000000000002","diner_id":"dn_00000000000000000000000000000002","zone":"Europe/London","today":"2026-10-12","now":FIRST,"demo":true}),
    );
    let mut number = 1;
    let mut profile = state["snapshot"]["profile"].clone();
    profile["usual_week"]["enabled_meal_types"] = json!(["breakfast", "dinner", "lunch"]);
    state = command(
        &state,
        json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),
        number,
        FIRST,
    );
    number += 1;
    let ids = state["snapshot"]["weeks"]
        .as_array()
        .unwrap()
        .iter()
        .map(|w| w["range"]["week_id"].as_str().unwrap().to_owned())
        .collect::<Vec<_>>();
    for id in &ids {
        state = fill(state, id, &mut number, FIRST);
    }
    state = command(
        &state,
        json!({"kind":"open_planning_horizon","data":{}}),
        number,
        LATER,
    );
    number += 1;
    let new = state["snapshot"]["weeks"]
        .as_array()
        .unwrap()
        .iter()
        .find(|w| !ids.iter().any(|id| w["range"]["week_id"] == id.as_str()))
        .unwrap()["range"]["week_id"]
        .as_str()
        .unwrap()
        .to_owned();
    state = fill(state, &new, &mut number, LATER);
    let mut profile = state["snapshot"]["profile"].clone();
    profile["diners"][0]["restrictions"] = json!({"kind":"confirmed","data":{"confirmed_at":LATER,"requirements":[{"kind":"diet","data":{"code":"vegan"}}]}});
    state = command(
        &state,
        json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),
        number,
        LATER,
    );
    assert_eq!(state["retained_weeks"].as_array().unwrap().len(), 5);
    assert_eq!(state["history"]["selected"].as_array().unwrap().len(), 105);
    state = call(json!({"op":"validate_state","state":state,"now":LATER}));
    let first = call(json!({"op":"conflicts","state":state,"offset":0,"limit":100,"now":LATER}));
    assert_eq!(first["total"], 110);
    assert_eq!(first["conflicts"].as_array().unwrap().len(), 100);
    assert_eq!(first["next_offset"], 100);
    let last = call(json!({"op":"conflicts","state":state,"offset":100,"limit":100,"now":LATER}));
    assert_eq!(last["total"], 110);
    assert_eq!(last["conflicts"].as_array().unwrap().len(), 10);
    assert!(last["next_offset"].is_null());
    let mut all = first["conflicts"].as_array().unwrap().clone();
    all.extend(last["conflicts"].as_array().unwrap().clone());
    assert!(all.iter().any(|c| {
        c["week_ids"]
            .as_array()
            .unwrap()
            .iter()
            .any(|id| id == &json!(ids[0]))
    }));
    assert_eq!(
        state["snapshot"]["conflicts"].as_array().unwrap().len(),
        100
    );
    let folder = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures");
    std::fs::create_dir_all(&folder).unwrap();
    std::fs::write(
        folder.join("archived-conflicts-110.json"),
        serde_json::to_vec(&state).unwrap(),
    )
    .unwrap();
    eprintln!(
        "actual API fixture: 5 retained weeks; 105 admitted meals; 110 conflicts in pages 100+10; {} receipts",
        state["receipts"].as_array().unwrap().len()
    );
}
