//! Exact compiled recipes and real native/calendar custody, using fictional
//! household/order inputs. No physical cooling or calendar import is observed.
use mstrchf_browser::{
    BrowserState, calendar_owner, calendar_view::material_at, catalogue::RuntimeCatalogue,
    maker_packets::*, run_json,
};
use mstrchf_contracts::*;
use serde_json::{Value, json};

const NOW: &str = "2026-10-18T12:00:00Z";
const DEST: &str = "cd_00000000000000000000000000000001";
fn portable_packet(hot: bool) -> ReviewedRecipePacketRef {
    let scope = if hot {
        &mstrchf_browser::personal_scope::PORTABLE_HOT_SCOPE
    } else {
        &mstrchf_browser::personal_scope::PORTABLE_COLD_SCOPE
    };
    available_packet_refs()
        .unwrap()
        .into_iter()
        .find(|packet| {
            packet.recipe.recipe_id.0 == scope.recipe_id
                && packet.recipe.revision.0 == scope.recipe_revision
                && packet.recipe.digest.0 == scope.recipe_digest
        })
        .unwrap()
}
fn good(input: Value) -> Value {
    let out: Value =
        serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap();
    assert_eq!(out["ok"], true, "{out}");
    out["value"].clone()
}
fn apply(state: &Value, body: Value, id: u32) -> Value {
    good(json!({"op":"dispatch","state":state,"now":NOW,"command":{"schema_version":1,
        "command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body}}))["state"].clone()
}
fn save(state: &Value, week: Value, id: u32) -> Value {
    apply(
        state,
        json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{
        "week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],
        "day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],
        "cells":week["cells"],"plan":week["plan"]}}}),
        id,
    )
}
fn select(state: &Value, index: usize, reference: RecipeRef, options: Value, id: u32) -> Value {
    let week = &state["snapshot"]["weeks"][0];
    let board = good(
        json!({"op":"board","state":state,"week_id":week["range"]["week_id"],
        "slot":week["cells"][index]["slot"],"options":options,"now":NOW}),
    );
    let choice = board["choices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|choice| choice["selection"]["recipe"] == json!(reference))
        .unwrap_or_else(|| panic!("missing exact choice: {board}"));
    let mut week = week.clone();
    week["plan"] = choice["plan"].clone();
    save(state, week, id)
}
fn moment(hour: u8) -> Value {
    good(
        json!({"op":"resolve_moment","date":"2026-10-18","time":format!("{hour:02}:00:00"),"zone":"Europe/London","resolution":"unambiguous"}),
    )
}
fn fixture(hot: usize, cold: usize) -> Value {
    let mut state = serde_json::from_slice::<Value>(include_bytes!(
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
    profile["preferences"]["mobility"] =
        json!({"temperature":"either","cold_storage":true,"reheating":false});
    let first = profile["diners"][0].clone();
    for index in 1..hot.max(cold) {
        let mut diner = first.clone();
        diner["diner_id"] = json!(format!("dn_{:032x}", 0xfd00 + index));
        profile["diners"].as_array_mut().unwrap().push(diner);
    }
    state = apply(
        &state,
        json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),
        0xfd01,
    );
    let people = profile["diners"]
        .as_array()
        .unwrap()
        .iter()
        .map(|diner| diner["diner_id"].clone())
        .collect::<Vec<_>>();
    let mut week = state["snapshot"]["weeks"][0].clone();
    week["plan"]["selections"] = json!([]);
    week["plan"]["batch_links"] = json!([]);
    week["day_enabled"] = json!([true, true, true, true, true, true, true]);
    week["meal_type_enabled"]["lunch"] = json!(true);
    week["meal_type_enabled"]["dinner"] = json!(true);
    for (index, count) in [(8, hot), (10, cold.max(1))] {
        week["cells"][index]["choice"] = json!(if index == 8 { "at_home" } else { "on_the_move" });
        week["cells"][index]["attendee_ids"] = json!(&people[..count]);
    }
    state = save(&state, week, 0xfd02);
    for (index, packet) in [portable_packet(true), portable_packet(false)]
        .into_iter()
        .enumerate()
    {
        state = apply(
            &state,
            json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":(index+1).to_string()}}),
            0xfd03 + index as u32,
        );
    }
    state = select(&state, 8, portable_packet(true).recipe, Value::Null, 0xfd05);
    if cold > 0 {
        let options = json!({"leftover_source":state["snapshot"]["weeks"][0]["cells"][8]["slot"],"portable_cold":{"packing_complete_time":"07:30:00","packing_resolution":"unambiguous","depart_time":"08:00:00","departure_resolution":"unambiguous","chilling_confirmed":true}});
        state = select(&state, 10, portable_packet(false).recipe, options, 0xfd06);
    }
    let typed: BrowserState = serde_json::from_value(state.clone()).unwrap();
    let view = RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();
    let (_, food) = material_at(
        &view,
        &typed.snapshot,
        typed.snapshot.profile.0.as_ref().unwrap(),
        &typed.snapshot.weeks[0],
        &Instant(NOW.into()),
    )
    .unwrap()
    .unwrap();
    state = apply(
        &state,
        json!({"kind":"approve_plan","data":{"week_id":food.plan.week_id,"plan_revision":food.plan.plan_revision,"derived_digest":food.plan.digest}}),
        0xfd07,
    );
    let week = &state["snapshot"]["weeks"][0];
    apply(
        &state,
        json!({"kind":"confirm_order","data":{"confirmation":{"week_id":week["range"]["week_id"],
        "approval_id":week["approved_plan"]["approval_id"],"delivery_start":moment(14),"delivery_end":moment(15),
        "confirmed_at":NOW,"source":{"kind":"user_confirmation","data":{}}}}}),
        0xfd08,
    )
}
fn prepare(state: &Value) -> (calendar_owner::CalendarJournal, String) {
    let typed: BrowserState = serde_json::from_value(state.clone()).unwrap();
    let order = typed.snapshot.weeks[0].confirmed_order.0.as_ref().unwrap();
    calendar_owner::prepare(
        &typed,
        &order.order_id,
        &CalendarDestinationId(DEST.into()),
        &Instant(NOW.into()),
    )
    .unwrap()
}
fn persist(state: &Value, id: u32) -> Value {
    let (journal, _) = prepare(state);
    apply(
        state,
        json!({"kind":"request_calendar_publication","data":{
        "order_id":state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],"destination_id":DEST,
        "event_batch_digest":canonical_digest(&journal.batch).unwrap()}}),
        id,
    )
}

fn portable_state() -> Value {
    static SAVED: std::sync::OnceLock<Value> = std::sync::OnceLock::new();
    SAVED.get_or_init(|| fixture(1, 2)).clone()
}

#[test]
fn packing_and_service_keep_distinct_times_instructions_portions_and_original_deadline() {
    let state = portable_state();
    let before = state.clone();
    let (journal, ics) = prepare(&state);
    assert_eq!(journal.renderer_version, 7);
    assert_eq!(journal.saved_events.len(), 5);
    let packing = journal
        .saved_events
        .iter()
        .find(|row| row.event.title.0.contains("Home packing"))
        .unwrap();
    let service = journal
        .saved_events
        .iter()
        .find(|row| row.event.title.0.contains("Portable cold service"))
        .unwrap();
    assert_ne!(packing.event.logical_key, service.event.logical_key);
    assert_eq!(packing.event.start.local_time.0, "07:25:00");
    assert_eq!(packing.event.end.local_time.0, "07:30:00");
    assert_eq!(service.event.start.local_time.0, "12:25:00");
    assert_eq!(service.event.end.local_time.0, "12:30:00");
    let recipe = known_packet(&portable_packet(false)).unwrap().recipe();
    let packing_text = &recipe
        .steps
        .iter()
        .find(|step| step.step_id == 1)
        .unwrap()
        .text
        .0;
    let service_text = &recipe
        .steps
        .iter()
        .find(|step| step.step_id == 2)
        .unwrap()
        .text
        .0;
    assert!(packing.event.body.0.contains(packing_text));
    assert!(!packing.event.body.0.contains(service_text));
    assert!(service.event.body.0.contains(service_text));
    assert!(!service.event.body.0.contains(packing_text));
    for row in [packing, service] {
        assert!(
            row.event
                .body
                .0
                .contains("Original cooked shares: 2; allocated for this meal: 2")
        );
        assert!(
            !row.event
                .body
                .0
                .contains("Ingredients — total for this occurrence")
        );
        for expected in [
            "no new ingredient demands",
            "Planned departure: 2026-10-22 08:00:00",
            "Original use-by deadline: 2026-10-22 18:30:00",
            "0–5°C",
            "not confirmation of cooking",
        ] {
            assert!(
                row.event.body.0.contains(expected),
                "missing {expected}: {}",
                row.event.body.0
            );
        }
    }
    assert_eq!(
        ics.matches("BEGIN:VEVENT").count(),
        5,
        "No departure or arrival event is invented"
    );
    let typed: BrowserState = serde_json::from_value(state.clone()).unwrap();
    let view = RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();
    let (_, food) = material_at(
        &view,
        &typed.snapshot,
        typed.snapshot.profile.0.as_ref().unwrap(),
        &typed.snapshot.weeks[0],
        &Instant(NOW.into()),
    )
    .unwrap()
    .unwrap();
    assert_eq!(
        food.plan.demand.lines.len(),
        7,
        "Only the original seven ingredient demands"
    );
    assert_eq!(state, before);
}

#[test]
fn saved_portable_calendar_reloads_and_corrected_renderer_eight_preserves_food_and_prior_journal() {
    let state = persist(&portable_state(), 0xfd09);
    let old = state["receipts"].as_array().unwrap().last().unwrap()["prepared_calendar"].clone();
    assert_eq!(old["renderer_version"], 7);
    assert_eq!(
        good(json!({"op":"validate_state","state":state,"now":NOW})),
        state
    );
    let changed = apply(
        &state,
        json!({"kind":"correct_order_delivery","data":{
        "order_id":state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],
        "delivery_start":moment(15),"delivery_end":moment(16)}}),
        0xfd10,
    );
    let (journal, _) = prepare(&changed);
    assert_eq!(journal.renderer_version, 8);
    for row in journal
        .saved_events
        .iter()
        .filter(|row| row.event.logical_key.kind != CalendarEventKind::Delivery)
    {
        let previous = old["saved_events"]
            .as_array()
            .unwrap()
            .iter()
            .find(|previous| previous["event"]["event_id"] == json!(row.event.event_id))
            .unwrap();
        assert_eq!(
            json!(row),
            *previous,
            "Food bytes and sequence remain exact"
        );
    }
    let saved = persist(&changed, 0xfd11);
    assert_eq!(
        saved["receipts"][state["receipts"].as_array().unwrap().len() - 1]["prepared_calendar"],
        old
    );
    assert_eq!(
        saved["snapshot"]["weeks"][0]["confirmed_order"],
        state["snapshot"]["weeks"][0]["confirmed_order"]
    );
    assert_eq!(
        good(json!({"op":"validate_state","state":saved,"now":NOW})),
        saved
    );
}

#[test]
fn all_four_portable_source_shares_hot_omit_unexecuted_cooling_and_journey_events() {
    let state = fixture(4, 0);
    let (journal, _) = prepare(&state);
    assert_eq!(journal.renderer_version, 7);
    assert_eq!(journal.saved_events.len(), 2);
    let food = journal
        .saved_events
        .iter()
        .find(|row| row.event.logical_key.kind != CalendarEventKind::Delivery)
        .unwrap();
    let source = known_packet(&portable_packet(true)).unwrap().recipe();
    for phase in source
        .prep
        .iter()
        .filter(|phase| [4, 5, 6].contains(&phase.prep_id))
    {
        assert!(!food.event.body.0.contains(&phase.title.0));
    }
    assert!(food.event.body.0.contains("allocated for this meal: 4"));
}

#[test]
fn renderer_versions_do_not_reinterpret_old_or_portable_receipts() {
    let state: BrowserState = serde_json::from_value(portable_state()).unwrap();
    let order = state.snapshot.weeks[0].confirmed_order.0.as_ref().unwrap();
    for renderer in 1..=6 {
        assert!(
            calendar_owner::prepare_for_renderer(
                &state,
                &order.order_id,
                &CalendarDestinationId(DEST.into()),
                &Instant(NOW.into()),
                renderer
            )
            .is_err()
        );
    }
    let old: BrowserState = serde_json::from_value(
        serde_json::from_slice::<Value>(include_bytes!(
            "fixtures/calendar-v15-renderer2-prepared.json"
        ))
        .unwrap()["state"]
            .clone(),
    )
    .unwrap();
    let order = old.snapshot.weeks[0].confirmed_order.0.as_ref().unwrap();
    let before = json!(old);
    for renderer in [7, 8] {
        assert!(
            calendar_owner::prepare_for_renderer(
                &old,
                &order.order_id,
                &CalendarDestinationId(DEST.into()),
                &Instant(NOW.into()),
                renderer
            )
            .is_err()
        );
    }
    assert_eq!(json!(old), before);
}

#[test]
fn calendar_refuses_changed_current_chilling_or_saved_journey_without_mutating_history() {
    let state: BrowserState = serde_json::from_value(portable_state()).unwrap();
    let before = json!(state);
    let order = state.snapshot.weeks[0].confirmed_order.0.as_ref().unwrap();
    for edit in ["chilling", "journey"] {
        let mut changed = state.clone();
        if edit == "chilling" {
            changed
                .snapshot
                .profile
                .0
                .as_mut()
                .unwrap()
                .preferences
                .mobility
                .cold_storage = false;
        } else {
            let selection = changed.snapshot.weeks[0]
                .plan
                .selections
                .iter_mut()
                .find(|selection| selection.portable_cold.is_some())
                .unwrap();
            selection.portable_cold.as_mut().unwrap().depart_at = selection
                .portable_cold
                .as_ref()
                .unwrap()
                .packing_complete_at
                .clone();
        }
        assert!(
            calendar_owner::prepare(
                &changed,
                &order.order_id,
                &CalendarDestinationId(DEST.into()),
                &Instant(NOW.into())
            )
            .is_err(),
            "{edit}"
        );
    }
    assert_eq!(json!(state), before);
}
