//! Actual domain/compiled recipe custody with simulated user order confirmation.
//! No payment, delivery, cook test or calendar-client import is asserted.
use mstrchf_browser::{
    calendar_owner, calendar_view::material_at, catalogue::RuntimeCatalogue, maker_packets::*,
    run_json, BrowserState,
};
use mstrchf_contracts::*;
use serde_json::{json, Value};
const NOW: &str = "2026-10-12T20:00:00Z";
const LATER: &str = "2026-10-12T21:00:00Z";
const DEST: &str = "cd_00000000000000000000000000000001";
fn call(request: Value) -> Value {
    serde_json::from_slice(&run_json(&serde_json::to_vec(&request).unwrap())).unwrap()
}
fn good(request: Value) -> Value {
    let out = call(request);
    assert_eq!(out["ok"], true, "{out}");
    out["value"].clone()
}
fn envelope(state: &Value, body: Value, id: u32) -> Value {
    json!({"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body})
}
fn apply(state: &Value, body: Value, id: u32) -> Value {
    good(json!({"op":"dispatch","state":state,"now":NOW,"command":envelope(state,body,id)}))
        ["state"]
        .clone()
}
fn edit(state: &Value, week: Value, id: u32) -> Value {
    apply(
        state,
        json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{"week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],"day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],"cells":week["cells"],"plan":week["plan"]}}}),
        id,
    )
}
fn select(state: &Value, recipe: RecipeRef, index: usize, id: u32) -> Value {
    let week = &state["snapshot"]["weeks"][0];
    let board = good(
        json!({"op":"board","state":state,"week_id":week["range"]["week_id"],"slot":week["cells"][index]["slot"],"now":NOW}),
    );
    let choice = board["choices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["selection"]["recipe"] == serde_json::to_value(&recipe).unwrap())
        .unwrap();
    let mut week = week.clone();
    week["plan"] = choice["plan"].clone();
    edit(state, week, id)
}
fn moment(date: &str, hour: u8) -> Value {
    json!({"local_date":date,"local_time":format!("{hour:02}:00:00"),"zone":"Europe/London","resolution":"unambiguous","instant":format!("{date}T{:02}:00:00Z",hour-1)})
}
fn fixture(both: bool) -> Value {
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
    state = apply(
        &state,
        json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),
        0xe01,
    );
    let mut week = state["snapshot"]["weeks"][0].clone();
    week["plan"]["selections"] = json!([]);
    week["plan"]["batch_links"] = json!([]);
    week["meal_type_enabled"]["breakfast"] = json!(true);
    week["meal_type_enabled"]["dinner"] = json!(true);
    state = edit(&state, week, 0xe02);
    state = apply(
        &state,
        json!({"kind":"admit_reviewed_recipe","data":{"packet":porridge_packet_ref().unwrap(),"expected_catalogue_revision":"1"}}),
        0xe03,
    );
    if both {
        state = apply(
            &state,
            json!({"kind":"admit_reviewed_recipe","data":{"packet":aglio_packet_ref().unwrap(),"expected_catalogue_revision":"2"}}),
            0xe04,
        );
    }
    state = select(&state, porridge_packet_ref().unwrap().recipe, 3, 0xe05);
    if both {
        state = select(&state, aglio_packet_ref().unwrap().recipe, 8, 0xe06);
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
        0xe07,
    );
    let week = &state["snapshot"]["weeks"][0];
    let confirmation = json!({"week_id":week["range"]["week_id"],"approval_id":week["approved_plan"]["approval_id"],"delivery_start":moment("2026-10-12",16),"delivery_end":moment("2026-10-12",17),"confirmed_at":NOW,"source":{"kind":"user_confirmation","data":{}}});
    apply(
        &state,
        json!({"kind":"confirm_order","data":{"confirmation":confirmation}}),
        0xe08,
    )
}
fn preview(state: &Value, at: &str) -> Value {
    good(
        json!({"op":"calendar_preview","state":state,"order_id":state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],"destination_id":DEST,"now":at}),
    )
}
fn publication(state: &Value, preview: &Value) -> Value {
    json!({"kind":"request_calendar_publication","data":{"order_id":state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],"destination_id":DEST,"event_batch_digest":preview["event_batch_digest"]}})
}
fn prepared(state: &Value) -> Value {
    let p = preview(state, NOW);
    apply(state, publication(state, &p), 0xe09)
}

#[test]
fn preview_reconstructs_exact_food_and_has_no_calendar_or_booked_state_effect() {
    let state = fixture(true);
    let before = state.clone();
    let p = preview(&state, NOW);
    assert_eq!(p["batch"]["events"].as_array().unwrap().len(), 3);
    let body = p["batch"]["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|row| row["body"].as_str().unwrap())
        .collect::<Vec<_>>()
        .join("\n");
    for text in [
        "45 g",
        "180 ml",
        "125 g",
        "55/2 g",
        "2 each",
        "1500 ml",
        "60 ml",
        "full attention as an unmeasured estimate",
        "Actual completion is not guaranteed.",
    ] {
        assert!(body.contains(text), "missing {text}");
    }
    assert!(p["notice"]
        .as_str()
        .unwrap()
        .contains("Import this file yourself"));
    assert!(p["notice"]
        .as_str()
        .unwrap()
        .contains("does not synchronise"));
    let ics = p["ics"].as_str().unwrap();
    assert_eq!(ics.matches("BEGIN:VEVENT").count(), 3);
    assert!(!ics.contains("VALARM"));
    assert!(!ics.contains("ATTENDEE"));
    assert!(ics.ends_with("END:VCALENDAR\r\n"));
    for line in ics.split("\r\n") {
        assert!(line.len() <= 75);
    }
    assert_eq!(state, before);
    assert_eq!(
        state["snapshot"]["weeks"][0]["publication"],
        "not_requested"
    );
    assert!(state["snapshot"]["weeks"][0]["confirmed_order"].is_object());
    assert_eq!(p["prepared"], false);
    assert!(p["batch"]["events"].as_array().unwrap().iter().all(|event| event["title"].as_str().unwrap().starts_with("MasterChef TEST · ") && event["body"].as_str().unwrap().starts_with("FICTIONAL TEST KITCHEN.")));
}

#[test]
fn frozen_v8_test_journal_is_audited_before_explicit_labelled_update_with_stable_ids() {
    // Exact synthetic local browser export from v8's 86109572 release. It has
    // no account/session identifiers, real checkout, or household data.
    let old = serde_json::from_slice::<Value>(include_bytes!("fixtures/calendar-v8-fictional-prepared.json")).unwrap()["state"].clone();
    let at = "2026-10-12T21:30:00Z";
    assert_eq!(good(json!({"op":"validate_state","state":old,"now":at})), old);
    let previous = old["receipts"].as_array().unwrap().last().unwrap()["prepared_calendar"].clone();
    assert!(previous.get("renderer_version").is_none());
    let destination=previous["batch"]["destination_id"].clone();
    let observe=|state:&Value|good(json!({"op":"calendar_preview","state":state,"order_id":state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],"destination_id":destination,"now":at}));
    let p = observe(&old);
    assert_eq!(p["prepared"], false, "Updated text must be retained before download");
    assert_eq!(p["batch"]["events"].as_array().unwrap().iter().map(|event|event["event_id"].clone()).collect::<Vec<_>>(), previous["batch"]["events"].as_array().unwrap().iter().map(|event|event["event_id"].clone()).collect::<Vec<_>>());
    let mut request=publication(&old,&p);request["data"]["destination_id"]=destination.clone();
    let command=envelope(&old,request,0xf301);
    let next=good(json!({"op":"dispatch","state":old,"command":command,"now":at}))["state"].clone();
    let updated=&next["receipts"].as_array().unwrap().last().unwrap()["prepared_calendar"];
    assert_eq!(updated["renderer_version"],3);
    assert_eq!(updated["saved_events"].as_array().unwrap().iter().map(|event|event["sequence"].clone()).collect::<Vec<_>>(),vec![json!(1),json!(1),json!(1)]);
    assert_eq!(next["receipts"][old["receipts"].as_array().unwrap().len()-1]["prepared_calendar"],previous,"Original journal bytes remain intact");
    assert_eq!(next["snapshot"]["weeks"][0]["confirmed_order"],old["snapshot"]["weeks"][0]["confirmed_order"]);
    assert_eq!(good(json!({"op":"validate_state","state":next,"now":at})),next);
    let retained=observe(&next);assert_eq!(retained["prepared"],true);
    assert_eq!(retained["batch"],p["batch"]);
    let replay=good(json!({"op":"dispatch","state":next,"command":command,"now":at}));assert_eq!(replay["state"],next);assert_eq!(replay["receipt"]["disposition"],"replayed");
    let mut forged=next.clone();forged["receipts"].as_array_mut().unwrap().last_mut().unwrap()["prepared_calendar"]["renderer_version"]=json!(99);
    assert_eq!(call(json!({"op":"validate_state","state":forged,"now":at}))["ok"],false);
}
#[test]
fn frozen_v8_original_renderer_command_replay_preserves_newest_prepared_journal() {
    let old = serde_json::from_slice::<Value>(include_bytes!(
        "fixtures/calendar-v8-fictional-prepared.json"
    ))
    .unwrap()["state"]
        .clone();
    let original_receipt = old["receipts"].as_array().unwrap().last().unwrap();
    let original_command = original_receipt["command"].clone();
    let original_journal = original_receipt["prepared_calendar"].clone();
    assert!(original_journal.get("renderer_version").is_none());
    assert_eq!(original_command["command"]["kind"], "request_calendar_publication");
    let at = "2026-10-12T21:30:00Z";
    let destination = original_command["command"]["data"]["destination_id"].clone();
    let observe = |state: &Value| {
        good(json!({"op":"calendar_preview","state":state,
            "order_id":original_command["command"]["data"]["order_id"],
            "destination_id":destination,"now":at}))
    };
    let update = observe(&old);
    assert_eq!(update["prepared"], false);
    let mut body = publication(&old, &update);
    body["data"]["destination_id"] = destination.clone();
    let current = good(json!({"op":"dispatch","state":old,
        "command":envelope(&old,body,0xf302),"now":at}))["state"]
        .clone();
    let newest = current["receipts"].as_array().unwrap().last().unwrap()["prepared_calendar"].clone();
    assert_eq!(newest["renderer_version"], 3);
    assert_ne!(newest["batch"], original_journal["batch"]);
    // Replay the exact fixture command, not a new command with old content.
    let replay = good(json!({"op":"dispatch","state":current,
        "command":original_command,"now":at}));
    assert_eq!(replay["receipt"]["disposition"], "replayed");
    assert_eq!(replay["receipt"]["receipt_snapshot_revision"], old["snapshot"]["snapshot_revision"]);
    assert_eq!(replay["state"], current);
    assert_eq!(replay["state"]["receipts"].as_array().unwrap().last().unwrap()["prepared_calendar"], newest);
    assert_eq!(replay["state"]["snapshot"]["weeks"][0]["confirmed_order"], old["snapshot"]["weeks"][0]["confirmed_order"]);
    let retained = observe(&replay["state"]);
    assert_eq!(retained["prepared"], true);
    assert_eq!(retained["batch"], newest["batch"]);
    assert_eq!(good(json!({"op":"validate_state","state":replay["state"],"now":at})), current);
}
#[test]
fn frozen_v15_renderer_two_keeps_exact_bytes_and_replays_after_renderer_three() {
    // Generated before the change by the exact published v15 WASM. The source
    // is the synthetic v8 fixture; no real account, checkout or food is implied.
    let old = serde_json::from_slice::<Value>(include_bytes!("fixtures/calendar-v15-renderer2-prepared.json")).unwrap()["state"].clone();
    let at = "2026-10-12T21:30:00Z";
    assert_eq!(good(json!({"op":"validate_state","state":old,"now":at})), old);
    let receipt = old["receipts"].as_array().unwrap().last().unwrap();
    let historical = receipt["prepared_calendar"].clone();
    assert_eq!(historical["renderer_version"], 2);
    let typed: BrowserState = serde_json::from_value(old.clone()).unwrap();
    let order = typed.snapshot.weeks[0].confirmed_order.0.as_ref().unwrap();
    let destination: CalendarDestinationId = serde_json::from_value(historical["batch"]["destination_id"].clone()).unwrap();
    let (legacy, ics) = calendar_owner::prepare_for_renderer(&typed, &order.order_id, &destination, &Instant(at.into()), 2).unwrap();
    assert_eq!(serde_json::to_value(legacy).unwrap(), historical);
    assert_eq!(ics, include_str!("fixtures/calendar-v15-renderer2.ics"));
    let current = good(json!({"op":"calendar_preview","state":old,"order_id":order.order_id,"destination_id":destination,"now":at}));
    // No changed text means no new event or sequence merely for a renderer bump.
    assert_eq!(current["batch"], historical["batch"]);
    assert_eq!(current["prepared"], true);
    assert_eq!(current["ics"].as_str().unwrap(), ics);
    let replay = good(json!({"op":"dispatch","state":old,"command":receipt["command"],"now":at}));
    assert_eq!(replay["state"], old);
    assert_eq!(replay["receipt"]["disposition"], "replayed");
}

#[test]
fn recorded_preparation_replays_and_cold_audits_without_claiming_publication() {
    let state = fixture(false);
    let p = preview(&state, NOW);
    let typed: BrowserState = serde_json::from_value(state.clone()).unwrap();
    let (journal, _) = calendar_owner::prepare(
        &typed,
        &typed.snapshot.weeks[0]
            .confirmed_order
            .0
            .as_ref()
            .unwrap()
            .order_id,
        &CalendarDestinationId(DEST.into()),
        &Instant(NOW.into()),
    )
    .unwrap();
    assert!(journal.validate_shape().is_ok());
    let mut shape_only = typed;
    shape_only.receipts.0.last_mut().unwrap().prepared_calendar = Some(journal);
    assert!(shape_only.validate_shape().is_ok());
    let command = envelope(&state, publication(&state, &p), 0xe09);
    let first = good(json!({"op":"dispatch","state":state,"command":command,"now":NOW}));
    let saved = &first["state"];
    let week = &saved["snapshot"]["weeks"][0];
    assert_eq!(week["publication"], "prepared");
    assert_eq!(
        week["confirmed_order"],
        state["snapshot"]["weeks"][0]["confirmed_order"]
    );
    let journal =
        saved["receipts"].as_array().unwrap().last().unwrap()["prepared_calendar"].clone();
    assert_eq!(journal["batch"], p["batch"]);
    assert_eq!(journal["prepared_at"], NOW);
    assert_eq!(journal["saved_events"].as_array().unwrap().len(), 2);
    assert!(journal["saved_events"]
        .as_array()
        .unwrap()
        .iter()
        .all(|row| row["sequence"] == 0));
    assert_eq!(
        good(json!({"op":"validate_state","state":saved,"now":LATER})),
        *saved
    );
    let replay = good(json!({"op":"dispatch","state":saved,"command":command,"now":LATER}));
    assert_eq!(replay["state"], *saved);
    assert_eq!(replay["receipt"]["disposition"], "replayed");
    let again = preview(saved, LATER);
    assert_eq!(again["batch"], p["batch"]);
    assert_eq!(again["ics"], p["ics"]);
    let later=good(json!({"op":"dispatch","state":saved,"command":envelope(saved,publication(saved,&again),0xe0a),"now":LATER}))["state"].clone();
    let retained = &later["receipts"].as_array().unwrap().last().unwrap()["prepared_calendar"];
    assert_eq!(retained["prepared_at"], LATER);
    assert_eq!(retained["saved_events"], journal["saved_events"]);
    assert_eq!(
        good(json!({"op":"validate_state","state":later,"now":LATER})),
        later
    );
}
#[test]
fn stale_event_batch_and_imported_journal_edits_cannot_acquire_custody() {
    let state = fixture(false);
    let p = preview(&state, NOW);
    let mut stale = publication(&state, &p);
    stale["data"]["event_batch_digest"] = json!("0".repeat(64));
    let rejected = call(
        json!({"op":"dispatch","state":state,"command":envelope(&state,stale,0xe0b),"now":NOW}),
    );
    assert_eq!(rejected["error"]["code"], "context_stale");
    let saved = prepared(&state);
    let last = saved["receipts"].as_array().unwrap().len() - 1;
    for path in ["body_raw", "body_resigned", "sequence", "basis", "clock"] {
        let mut forged = saved.clone();
        let journal = &mut forged["receipts"][last]["prepared_calendar"];
        match path {
            "body_raw" => {
                journal["saved_events"][0]["event"]["body"] = json!("invented external publication")
            }
            "body_resigned" => {
                let mut event = journal["saved_events"][0]["event"].clone();
                event["body"] = json!("invented external publication");
                let mut material = event.clone();
                material
                    .as_object_mut()
                    .unwrap()
                    .remove("event_payload_digest");
                material["schema_version"] = json!(1);
                event["event_payload_digest"] =
                    serde_json::to_value(canonical_digest(&material).unwrap()).unwrap();
                journal["batch"]["events"][0] = event.clone();
                journal["saved_events"][0]["event"] = event;
            }
            "sequence" => journal["saved_events"][0]["sequence"] = json!(999),
            "basis" => {
                let index = journal["saved_events"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .position(|row| !row["contributors"].as_array().unwrap().is_empty())
                    .unwrap();
                journal["saved_events"][index]["contributors"][0]["owner"]["basis_digest"] =
                    json!("0".repeat(64));
            }
            "clock" => journal["prepared_at"] = json!(LATER),
            _ => unreachable!(),
        };
        let out = call(json!({"op":"validate_state","state":forged,"now":LATER}));
        assert_eq!(out["ok"], false, "accepted forged {path}: {out}");
        assert_eq!(
            out["error"]["code"],
            if path == "body_raw" {
                "digest_mismatch"
            } else {
                "identity_mismatch"
            },
            "{out}"
        );
    }
}
#[test]
fn later_catalogue_and_current_scope_changes_do_not_rewrite_original_journals() {
    let saved = prepared(&fixture(false));
    let original =
        saved["receipts"].as_array().unwrap().last().unwrap()["prepared_calendar"].clone();
    let p = preview(&saved, NOW);
    let admitted = apply(
        &saved,
        json!({"kind":"admit_reviewed_recipe","data":{"packet":aglio_packet_ref().unwrap(),"expected_catalogue_revision":"2"}}),
        0xe0c,
    );
    let later = preview(&admitted, NOW);
    assert_eq!(later["batch"], p["batch"]);
    assert_eq!(later["ics"], p["ics"]);
    assert_eq!(
        admitted["receipts"][saved["receipts"].as_array().unwrap().len() - 1]["prepared_calendar"],
        original
    );
    let mut profile = admitted["snapshot"]["profile"].clone();
    profile["diners"][0]["age_band"] = json!("teen");
    let changed = apply(
        &admitted,
        json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),
        0xe0d,
    );
    assert!(changed["snapshot"]["weeks"][0]["confirmed_order"].is_object());
    assert_eq!(
        call(
            json!({"op":"calendar_preview","state":changed,"order_id":changed["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],"destination_id":DEST,"now":LATER})
        )["ok"],
        false
    );
    assert_eq!(
        good(json!({"op":"validate_state","state":changed,"now":LATER})),
        changed
    );
}
#[test]
fn edited_booked_plan_and_missing_historical_receipt_refuse_without_unbooking() {
    let state = fixture(false);
    let order = state["snapshot"]["weeks"][0]["confirmed_order"].clone();
    let mut week = state["snapshot"]["weeks"][0].clone();
    week["plan"]["selections"] = json!([]);
    let changed = edit(&state, week, 0xe0e);
    assert_eq!(changed["snapshot"]["weeks"][0]["confirmed_order"], order);
    assert_eq!(changed["snapshot"]["weeks"][0]["publication"], "needs_user");
    assert_eq!(
        call(
            json!({"op":"calendar_preview","state":changed,"order_id":order["order_id"],"destination_id":DEST,"now":LATER})
        )["ok"],
        false
    );
    // Typed owner seam assumes the caller has audited state. Even that seam
    // refuses unavailable historical authority instead of inventing an empty pin.
    let mut typed: BrowserState = serde_json::from_value(state).unwrap();
    typed
        .receipts
        .0
        .retain(|row| !matches!(row.command.command, AppCommand::ApprovePlan { .. }));
    let order_id = typed.snapshot.weeks[0]
        .confirmed_order
        .0
        .as_ref()
        .unwrap()
        .order_id
        .clone();
    let out = calendar_owner::prepare(
        &typed,
        &order_id,
        &CalendarDestinationId(DEST.into()),
        &Instant(NOW.into()),
    )
    .unwrap_err();
    assert_eq!(out.issues[0].path, "/order/approval_receipt");
    assert_eq!(
        serde_json::to_value(&typed.snapshot.weeks[0].confirmed_order.0).unwrap(),
        order
    );
}

#[test]
fn nearby_edited_booked_owner_with_unavailable_recipe_cannot_be_treated_as_empty_occupancy() {
    let mut state = fixture(false);
    let week = &state["snapshot"]["weeks"][1];
    let board = good(
        json!({"op":"board","state":state,"week_id":week["range"]["week_id"],"slot":week["cells"][0]["slot"],"now":NOW}),
    );
    let mut week = week.clone();
    week["plan"] = board["choices"][0]["plan"].clone();
    state = edit(&state, week, 0xe20);
    let typed: BrowserState = serde_json::from_value(state.clone()).unwrap();
    let view = RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();
    let (_, food) = material_at(
        &view,
        &typed.snapshot,
        typed.snapshot.profile.0.as_ref().unwrap(),
        &typed.snapshot.weeks[1],
        &Instant(NOW.into()),
    )
    .unwrap()
    .unwrap();
    state = apply(
        &state,
        json!({"kind":"approve_plan","data":{"week_id":food.plan.week_id,"plan_revision":food.plan.plan_revision,"derived_digest":food.plan.digest}}),
        0xe21,
    );
    let week = &state["snapshot"]["weeks"][1];
    let confirmation = json!({"week_id":week["range"]["week_id"],"approval_id":week["approved_plan"]["approval_id"],"delivery_start":moment("2026-10-12",16),"delivery_end":moment("2026-10-12",17),"confirmed_at":NOW,"source":{"kind":"user_confirmation","data":{}}});
    state = apply(
        &state,
        json!({"kind":"confirm_order","data":{"confirmation":confirmation}}),
        0xe22,
    );
    // Both real receipt chains are valid, and their unchanged prep does not
    // overlap. The following controlled typed mutation marks current custody
    // unavailable; it is not passed off as an accepted household-state edit.
    preview(&state, NOW);
    let mut typed: BrowserState = serde_json::from_value(state).unwrap();
    typed.snapshot.weeks[1].plan.selections[0].recipe.recipe_id =
        RecipeId("rc_ffffffffffffffffffffffffffffffff".into());
    let order_id = typed.snapshot.weeks[0]
        .confirmed_order
        .0
        .as_ref()
        .unwrap()
        .order_id
        .clone();
    let failure = calendar_owner::prepare(
        &typed,
        &order_id,
        &CalendarDestinationId(DEST.into()),
        &Instant(NOW.into()),
    )
    .unwrap_err();
    assert_eq!(failure.code, ErrorCode::IdentityMismatch);
    assert_eq!(
        failure.issues[0].path,
        "/calendar/occupancy/contradictory_week"
    );
    let changed_id = typed.snapshot.weeks[1].range.week_id.clone();
    for week in typed
        .retained_weeks
        .iter_mut()
        .filter(|week| week.range.week_id == changed_id)
    {
        week.plan.selections[0].recipe.recipe_id =
            RecipeId("rc_ffffffffffffffffffffffffffffffff".into());
    }
    let failure = calendar_owner::prepare(
        &typed,
        &order_id,
        &CalendarDestinationId(DEST.into()),
        &Instant(NOW.into()),
    )
    .unwrap_err();
    assert_eq!(failure.code, ErrorCode::ReconciliationRequired);
    assert_eq!(failure.issues[0].path, "/calendar/occupancy/unknown_owner");
    assert!(typed
        .snapshot
        .weeks
        .iter()
        .take(2)
        .all(|week| week.confirmed_order.0.is_some()));
}
