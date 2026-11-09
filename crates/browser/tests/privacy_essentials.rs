//! Synthetic kitchen, compiled reviewed food, and exact historical privacy carry.
use mstrchf_browser::{maker_packets::aglio_packet_ref, run_json};
use mstrchf_contracts::canonical_digest;
use serde_json::{Value, json};

const NOW: &str = "2026-10-14T15:00:00Z";
const LATER: &str = "2026-10-14T15:01:00Z";
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
fn apply_at(state: &Value, body: Value, id: u32, at: &str) -> Value {
    good(json!({"op":"dispatch","state":state,"command":envelope(state,body,id),"now":at}))["state"]
        .clone()
}
fn apply(state: &Value, body: Value, id: u32) -> Value {
    apply_at(state, body, id, NOW)
}
fn edit(state: &Value, week: Value, id: u32) -> Value {
    apply(
        state,
        json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{"week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],"day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],"cells":week["cells"],"plan":week["plan"]}}}),
        id,
    )
}
fn board(state: &Value, slot: usize) -> Value {
    let week = &state["snapshot"]["weeks"][0];
    good(
        json!({"op":"board","state":state,"week_id":week["range"]["week_id"],"slot":week["cells"][slot]["slot"],"now":LATER}),
    )
}
fn list(state: &Value) -> Value {
    board(state, 2)["details"]["shopping_list"].clone()
}
fn choose(state: &Value, slot: usize, id: u32) -> Value {
    let out = board(state, slot);
    let recipe = json!(aglio_packet_ref().unwrap().recipe);
    let choice = out["choices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|choice| choice["selection"]["recipe"] == recipe)
        .expect("admitted exact Aglio choice");
    let mut week = state["snapshot"]["weeks"][0].clone();
    week["plan"] = choice["plan"].clone();
    edit(state, week, id)
}
fn cold(state: &Value) -> Value {
    let state = state.clone();
    std::thread::spawn(move || call(json!({"op":"validate_state","state":state,"now":LATER})))
        .join()
        .unwrap()
}
fn privacy(state: &Value, kind: &str, enabled: bool) -> Value {
    let mut policy = state["snapshot"]["privacy"].clone();
    policy["use_ai_features"] = json!(enabled);
    policy["confirmed_at"] = json!(NOW);
    json!({"kind":kind,"data":{"expected_policy_revision":policy["revision"],"policy":policy}})
}
fn base() -> Value {
    let mut state = good(
        json!({"op":"initialize","household_id":"hh_54000000000000000000000000000001","diner_id":"dn_54000000000000000000000000000001","zone":"Europe/London","today":"2026-10-14","now":NOW,"demo":true}),
    );
    let mut profile = state["snapshot"]["profile"].clone();
    profile["diners"][0]["age_band"] = json!("adult");
    profile["diners"][0]["restrictions"] =
        json!({"kind":"confirmed_none","data":{"confirmed_at":NOW}});
    profile["diners"][0]["portions"] = json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});
    profile["preferences"]["equipment"] = json!(["fridge", "hob"]);
    state = apply(
        &state,
        json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),
        0x5401,
    );
    state = apply(
        &state,
        json!({"kind":"admit_reviewed_recipe","data":{"packet":aglio_packet_ref().unwrap(),"expected_catalogue_revision":"1"}}),
        0x5402,
    );
    choose(&state, 2, 0x5403)
}
fn confirmed() -> Value {
    confirm_current(&base(), 0x5404)
}
fn confirm_current(state: &Value, id: u32) -> Value {
    let projected = list(state);
    let confirmations:Vec<_>=projected["lines"].as_array().unwrap().iter().filter(|row|row["demand"]["essential"]==true).enumerate().map(|(index,row)|json!({"line_id":row["demand"]["line_id"],"choice":if index==0{"have_enough"}else{"add_to_shop"},"displayed_quantity":row["demand"]["quantity"],"demand_digest":projected["demand_digest"],"confirmed_at":NOW})).collect();
    assert_eq!(confirmations.len(), 2);
    apply(
        state,
        json!({"kind":"confirm_essentials","data":{"confirmations":confirmations}}),
        id,
    )
}
fn assert_current(state: &Value) {
    let projected = list(state);
    let essentials: Vec<_> = projected["lines"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["demand"]["essential"] == true)
        .collect();
    assert_eq!(essentials.len(), 2);
    for row in essentials {
        assert_eq!(row["confirmation_status"], "current");
        let saved = state["essentials"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["line_id"] == row["demand"]["line_id"])
            .unwrap();
        assert_eq!(row["action"], saved["choice"]);
    }
}
fn assert_stale(state: &Value) {
    for row in list(state)["lines"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|row| row["demand"]["essential"] == true)
    {
        assert_eq!(row["confirmation_status"], "stale");
        assert_eq!(row["action"], "add_to_shop");
    }
}

#[test]
fn current_have_enough_and_add_to_shop_carry_on_off_without_new_intent_or_food() {
    let before = confirmed();
    assert_current(&before);
    let mut state = before.clone();
    for (index, enabled) in [true, false].into_iter().enumerate() {
        let old = state.clone();
        let legacy = apply_at(
            &old,
            privacy(&old, "set_privacy", enabled),
            0x5410 + index as u32,
            LATER,
        );
        state = apply_at(
            &old,
            privacy(&old, "set_privacy_preserving_essentials", enabled),
            0x5420 + index as u32,
            LATER,
        );
        assert_eq!(
            state["snapshot"], legacy["snapshot"],
            "all original privacy effects remain exact"
        );
        for field in [
            "retained_weeks",
            "history",
            "personal_recipes",
            "reviewed_packets",
            "pending_proposal",
            "engineering_demo",
        ] {
            assert_eq!(state[field], legacy[field], "{field}");
        }
        let mut carried = state["essentials"].clone();
        for (got, original) in carried
            .as_array_mut()
            .unwrap()
            .iter_mut()
            .zip(old["essentials"].as_array().unwrap())
        {
            assert_ne!(got["demand_digest"], original["demand_digest"]);
            got["demand_digest"] = original["demand_digest"].clone();
        }
        assert_eq!(carried, old["essentials"]);
        assert_current(&state);
        assert_eq!(cold(&state)["value"], state);
        assert_eq!(
            &state["receipts"].as_array().unwrap()[..old["receipts"].as_array().unwrap().len()],
            old["receipts"].as_array().unwrap()
        );
    }
    assert_eq!(state["snapshot"]["privacy"]["use_ai_features"], false);
}

#[test]
fn legacy_staleness_and_real_quantity_changes_are_not_revived() {
    let before = confirmed();
    let legacy = apply(&before, privacy(&before, "set_privacy", true), 0x5430);
    assert_stale(&legacy);
    let after = apply(
        &legacy,
        privacy(&legacy, "set_privacy_preserving_essentials", false),
        0x5431,
    );
    assert_eq!(after["essentials"], before["essentials"]);
    assert_stale(&after);
    assert_eq!(cold(&after)["value"], after);
    let changed = choose(&before, 5, 0x5432);
    assert_stale(&changed);
    let after = apply(
        &changed,
        privacy(&changed, "set_privacy_preserving_essentials", true),
        0x5433,
    );
    assert_eq!(after["essentials"], before["essentials"]);
    assert_stale(&after);
    assert_eq!(cold(&after)["value"], after);
}

#[test]
fn conflicting_food_and_missing_current_material_never_renew_confirmations() {
    let before = confirmed();
    let mut profile = before["snapshot"]["profile"].clone();
    profile["diners"][0]["restrictions"] = json!({"kind":"confirmed","data":{"confirmed_at":NOW,"requirements":[{"kind":"diet","data":{"code":"vegan"}}]}});
    let blocked = apply(
        &before,
        json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),
        0x5440,
    );
    assert!(board(&blocked, 2)["details"]["derived"].is_null());
    let after = apply(
        &blocked,
        privacy(&blocked, "set_privacy_preserving_essentials", true),
        0x5441,
    );
    assert_eq!(after["essentials"], before["essentials"]);
    assert_eq!(cold(&after)["value"], after);
    let mut week = before["snapshot"]["weeks"][0].clone();
    week["plan"]["selections"] = json!([]);
    week["plan"]["batch_links"] = json!([]);
    let empty = edit(&before, week, 0x5442);
    let after = apply(
        &empty,
        privacy(&empty, "set_privacy_preserving_essentials", true),
        0x5443,
    );
    assert_eq!(after["essentials"], before["essentials"]);
    assert_eq!(cold(&after)["value"], after);
}

#[test]
fn replay_no_flag_change_and_historical_legacy_import_remain_exact() {
    let before = confirmed();
    let body = privacy(&before, "set_privacy_preserving_essentials", false);
    let command = envelope(&before, body, 0x5450);
    let out = good(json!({"op":"dispatch","state":before,"command":command,"now":NOW}));
    let after = &out["state"];
    assert_current(after);
    let replay = good(json!({"op":"dispatch","state":after,"command":command,"now":LATER}));
    assert_eq!(replay["state"], *after);
    assert_eq!(replay["receipt"]["disposition"], "replayed");
    assert_eq!(
        replay["receipt"]["response_digest"],
        out["receipt"]["response_digest"]
    );
    let mut conflict = command.clone();
    conflict["command"]["data"]["policy"]["use_ai_features"] = json!(true);
    assert_eq!(
        call(json!({"op":"dispatch","state":after,"command":conflict,"now":LATER}))["error"]["code"],
        "replay_conflict"
    );
    let mut stale = command;
    stale["command_id"] = json!("cm_54000000000000000000000000000002");
    assert_eq!(
        call(json!({"op":"dispatch","state":after,"command":stale,"now":LATER}))["error"]["code"],
        "context_stale"
    );
    let legacy: Value =
        serde_json::from_slice(include_bytes!("fixtures/legacy-engineering-receipts.json"))
            .unwrap();
    let state = &legacy["state"];
    assert_eq!(cold(state)["value"], *state);
    for receipt in state["receipts"].as_array().unwrap() {
        let replay =
            good(json!({"op":"dispatch","state":state,"command":receipt["command"],"now":LATER}));
        assert_eq!(replay["state"], *state);
        assert_eq!(
            replay["receipt"]["response_digest"],
            receipt["response_digest"]
        );
    }
}

#[test]
fn cold_reconstruction_rejects_relabelled_choice_time_quantity_and_resigned_response() {
    let before = confirmed();
    let after = apply(
        &before,
        privacy(&before, "set_privacy_preserving_essentials", true),
        0x5460,
    );
    for (field, value) in [
        ("choice", json!("add_to_shop")),
        ("confirmed_at", json!("2026-10-14T14:59:00Z")),
        (
            "displayed_quantity",
            json!({"amount":{"numerator":"999","denominator":"1"},"unit":"g","form":"raw"}),
        ),
        (
            "demand_digest",
            before["essentials"][0]["demand_digest"].clone(),
        ),
    ] {
        let mut bad = after.clone();
        bad["essentials"][0][field] = value;
        assert_ne!(bad, after);
        assert_eq!(cold(&bad)["ok"], false, "{field}");
    }
    let mut omitted = after.clone();
    omitted["essentials"] = json!([]);
    assert_eq!(cold(&omitted)["ok"], false);
    let mut bad = after;
    let last = bad["receipts"].as_array().unwrap().len() - 1;
    bad["receipts"][last]["response"]["privacy"]["include_household_requirements"] = json!(true);
    bad["receipts"][last]["response_digest"] = json!(
        canonical_digest(&bad["receipts"][last]["response"])
            .unwrap()
            .0
    );
    bad["snapshot"] = bad["receipts"][last]["response"].clone();
    assert_eq!(cold(&bad)["ok"], false);
}

#[test]
fn legacy_empty_and_subset_inputs_remain_sparse_on_ai_off_and_exact_cold_replay() {
    let before = confirmed();
    let enabled = apply(&before, privacy(&before, "set_privacy", true), 0x5470);
    let current = confirm_current(&enabled, 0x5471);
    assert_current(&current);
    for keep in [0, 1] {
        let mut sparse = current.clone();
        sparse["essentials"] = json!(&current["essentials"].as_array().unwrap()[..keep]);
        assert_eq!(
            cold(&sparse)["value"],
            sparse,
            "frozen legacy admission accepts omitted rows"
        );
        let after = apply(
            &sparse,
            privacy(&sparse, "set_privacy_preserving_essentials", false),
            0x5472 + keep as u32,
        );
        assert_eq!(after["snapshot"]["privacy"]["use_ai_features"], false);
        assert_eq!(after["essentials"].as_array().unwrap().len(), keep);
        assert_eq!(
            after["receipts"].as_array().unwrap().last().unwrap()["privacy_essentials_before"],
            sparse["essentials"]
        );
        for (given, carried) in sparse["essentials"]
            .as_array()
            .unwrap()
            .iter()
            .zip(after["essentials"].as_array().unwrap())
        {
            for field in ["line_id", "choice", "confirmed_at", "displayed_quantity"] {
                assert_eq!(given[field], carried[field]);
            }
        }
        assert_eq!(cold(&after)["value"], after);
        let again = apply(
            &after,
            privacy(&after, "set_privacy_preserving_essentials", true),
            0x5474 + keep as u32,
        );
        assert_eq!(again["essentials"].as_array().unwrap().len(), keep);
        assert_eq!(cold(&again)["value"], again);
    }
}

#[test]
fn receipt_input_requires_exact_prior_proof_and_later_commands_cannot_resurrect_omissions() {
    let before = confirmed();
    let after = apply(
        &before,
        privacy(&before, "set_privacy_preserving_essentials", true),
        0x5480,
    );
    let last = after["receipts"].as_array().unwrap().len() - 1;
    let seed = after["receipts"][last]["privacy_essentials_before"].clone();
    for (field, value) in [
        ("line_id", json!("dl_ffffffffffffffffffffffffffffffff")),
        ("confirmed_at", json!("2026-10-14T15:02:00Z")),
        ("confirmed_at", json!("2026-10-14T14:59:00Z")),
        ("demand_digest", json!("f".repeat(64))),
        (
            "choice",
            json!(if seed[0]["choice"] == "have_enough" {
                "add_to_shop"
            } else {
                "have_enough"
            }),
        ),
    ] {
        let mut bad = after.clone();
        bad["receipts"][last]["privacy_essentials_before"][0][field] = value;
        assert_eq!(cold(&bad)["ok"], false, "altered input {field}");
    }
    let mut missing = after.clone();
    missing["receipts"][last]
        .as_object_mut()
        .unwrap()
        .remove("privacy_essentials_before");
    assert_eq!(cold(&missing)["ok"], false);
    let mut duplicate = after.clone();
    duplicate["receipts"][last]["privacy_essentials_before"] = json!([seed[0], seed[0]]);
    assert_eq!(cold(&duplicate)["ok"], false);
    let mut wrong_command = before.clone();
    wrong_command["receipts"][0]["privacy_essentials_before"] = json!([]);
    assert_eq!(cold(&wrong_command)["ok"], false);
    let mut sparse = before.clone();
    sparse["essentials"] = json!([]);
    let once = apply(
        &sparse,
        privacy(&sparse, "set_privacy_preserving_essentials", true),
        0x5481,
    );
    let mut twice = apply(
        &once,
        privacy(&once, "set_privacy_preserving_essentials", false),
        0x5482,
    );
    let last = twice["receipts"].as_array().unwrap().len() - 1;
    twice["receipts"][last]["privacy_essentials_before"] = before["essentials"].clone();
    assert_eq!(
        cold(&twice)["ok"],
        false,
        "an admitted old proof cannot restore an intentionally absent row after the first new command"
    );
}

#[test]
fn privacy_carry_revokes_live_assistance_without_recreating_it_on_reenable() {
    let before = confirmed();
    let enabled = apply(
        &before,
        privacy(&before, "set_privacy_preserving_essentials", true),
        0x5490,
    );
    let week = &enabled["snapshot"]["weeks"][0];
    let slot = week["cells"][5]["slot"].clone();
    let pending = good(
        json!({"op":"prepare_proposal","state":enabled,"week_id":week["range"]["week_id"],"slots":[slot],"mode":"fill_open","allow_repeats":true,"nonce":"privacy-carry-revocation","now":NOW}),
    );
    let pending = &pending["state"];
    assert!(pending["pending_proposal"].is_object());
    let off = apply(
        pending,
        privacy(pending, "set_privacy_preserving_essentials", false),
        0x5491,
    );
    assert!(off["pending_proposal"].is_null());
    assert_current(&off);
    let on = apply(
        &off,
        privacy(&off, "set_privacy_preserving_essentials", true),
        0x5492,
    );
    assert!(on["pending_proposal"].is_null());
    assert_current(&on);
    assert_eq!(cold(&on)["value"], on);
}
