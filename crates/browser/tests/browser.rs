use mstrchf_browser::run_json;
use serde_json::{Value, json};
const NOW: &str = "2026-10-12T12:00:00Z";
fn call(request: Value) -> Value {
    serde_json::from_slice(&run_json(&serde_json::to_vec(&request).unwrap())).unwrap()
}
fn good(request: Value) -> Value {
    let out = call(request);
    assert_eq!(out["ok"], true, "{out}");
    out["value"].clone()
}
fn init(demo: bool) -> Value {
    good(
        json!({"op":"initialize","household_id":"hh_00000000000000000000000000000001","diner_id":"dn_00000000000000000000000000000001","zone":"Europe/London","today":"2026-10-12","now":NOW,"demo":demo}),
    )
}
fn dispatch(state: &Value, command: Value, n: u32) -> Value {
    let command = json!({"schema_version":1,"command_id":format!("cm_{n:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":command});
    good(
        json!({"op":"dispatch","state":state,"command_json":serde_json::to_string(&command).unwrap(),"command":command,"now":NOW,"today":"2026-10-12"}),
    )
}
fn save_choice(state: &Value, wi: usize, si: usize, n: u32) -> Value {
    let w = &state["snapshot"]["weeks"][wi];
    let board = good(
        json!({"op":"board","state":state,"week_id":w["range"]["week_id"],"slot":w["cells"][si]["slot"],"now":NOW}),
    );
    assert!(board["choices"].as_array().unwrap().len() > 0);
    dispatch(state,json!({"kind":"save_week","data":{"expected_week_revision":w["revision"],"week":{"week_id":w["range"]["week_id"],"defaults_snapshot":w["defaults_snapshot"],"day_enabled":w["day_enabled"],"meal_type_enabled":w["meal_type_enabled"],"cells":w["cells"],"plan":board["choices"][0]["plan"]}}}),n)["state"].clone()
}
fn ai_on(state: &Value, n: u32) -> Value {
    let mut p = state["snapshot"]["privacy"].clone();
    p["use_ai_features"] = json!(true);
    p["confirmed_at"] = json!(NOW);
    dispatch(
        state,
        json!({"kind":"set_privacy","data":{"policy":p,"expected_policy_revision":p["revision"]}}),
        n,
    )["state"]
        .clone()
}

#[test]
fn initial_empty_draft_and_labelled_demo_four_weeks() {
    let empty = init(false);
    assert!(empty["snapshot"]["profile"].is_null());
    assert_eq!(empty["snapshot"]["household_draft"]["diners"], json!([]));
    let demo = init(true);
    assert_eq!(demo["snapshot"]["weeks"].as_array().unwrap().len(), 4);
    assert_eq!(demo["snapshot"]["privacy"]["use_ai_features"], false);
    assert_eq!(
        demo["snapshot"]["privacy"]["include_household_requirements"],
        false
    );
    assert_eq!(demo["engineering_demo"], true);
    good(json!({"op":"validate_state","state":demo,"now":NOW}));
}

#[test]
fn exact_saved_selections_are_independent_and_reloadable() {
    let state = init(true);
    let original = state["snapshot"]["weeks"][1].clone();
    let state = save_choice(&state, 0, 2, 1);
    assert_eq!(
        state["snapshot"]["weeks"][0]["plan"]["selections"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(state["snapshot"]["weeks"][1], original);
    let state = good(json!({"op":"validate_state","state":state,"now":NOW}));
    let board = good(json!({"op":"board","state":state,"now":NOW}));
    assert!(board["details"]["derived"].is_object());
    assert_eq!(
        board["details"]["history"]["selected"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
}

#[test]
fn command_replay_uses_original_ack_and_rejects_stale_or_changed_intent() {
    let state = init(true);
    let command = json!({"schema_version":1,"command_id":"cm_00000000000000000000000000000001","expected_snapshot_revision":"1","command":{"kind":"open_planning_horizon","data":{}}});
    let command_json = serde_json::to_string(&command).unwrap();
    let applied = good(
        json!({"op":"dispatch","state":state,"command":command,"command_json":command_json,"now":NOW}),
    );
    let replay = good(
        json!({"op":"dispatch","state":applied["state"],"command":command,"command_json":command_json,"now":NOW}),
    );
    assert_eq!(applied["state"], replay["state"]);
    assert_eq!(replay["receipt"]["disposition"], "replayed");
    assert_eq!(
        applied["receipt"]["response_digest"],
        replay["receipt"]["response_digest"]
    );
    let mut changed = command.clone();
    changed["command"] = json!({"kind":"restore_hidden_recipe","data":{"recipe_id":"rc_00000000000000000000000000000001"}});
    assert_eq!(
        call(json!({"op":"dispatch","state":applied["state"],"command":changed,"now":NOW}))["error"]
            ["code"],
        "replay_conflict"
    );
    changed["command_id"] = json!("cm_00000000000000000000000000000002");
    assert_eq!(
        call(json!({"op":"dispatch","state":applied["state"],"command":changed,"now":NOW}))["error"]
            ["code"],
        "context_stale"
    );
}

#[test]
fn saved_partial_onboarding_cannot_silently_promote() {
    let state = init(false);
    let demo = init(true);
    let mut draft = state["snapshot"]["household_draft"].clone();
    draft["diners"] = demo["snapshot"]["household_draft"]["diners"].clone();
    draft["completed_stages"] = json!(["household"]);
    let state = dispatch(
        &state,
        json!({"kind":"save_household_draft","data":{"draft":draft,"restriction_issue_edits":[]}}),
        1,
    )["state"]
        .clone();
    assert_eq!(state["snapshot"]["household_draft"]["revision"], "2");
    assert!(state["snapshot"]["profile"].is_null());
    good(json!({"op":"validate_state","state":state,"now":NOW}));
}

#[test]
fn privacy_safe_scoped_return_needs_explicit_apply_and_survives_reload() {
    let state = init(true);
    let w = &state["snapshot"]["weeks"][0];
    let slot = w["cells"][2]["slot"].clone();
    let denied = call(
        json!({"op":"prepare_proposal","state":state,"week_id":w["range"]["week_id"],"slots":[slot],"mode":"fill_open","allow_repeats":false,"now":NOW}),
    );
    assert_eq!(denied["error"]["code"], "privacy_denied");
    let state = ai_on(&state, 1);
    let prepared = good(
        json!({"op":"prepare_proposal","state":state,"week_id":w["range"]["week_id"],"slots":[slot],"mode":"fill_open","allow_repeats":false,"now":NOW}),
    );
    let text = serde_json::to_string(&prepared["request"]).unwrap();
    for forbidden in [
        "household",
        "restrictions",
        "dn_",
        "servings",
        "portions",
        "preferences",
        "confirmed_at",
    ] {
        assert!(!text.contains(forbidden), "outbound leaked {forbidden}");
    }
    let request = &prepared["request"];
    let result = json!({"proposal_id":request["proposal_id"],"context_digest":request["context_digest"],"selections":[{"slot":slot,"recipe":request["slots"][0]["choices"][0]["recipe"]}]});
    let returned =
        good(json!({"op":"return_proposal","state":prepared["state"],"result":result,"now":NOW}));
    assert!(
        returned["state"]["snapshot"]["weeks"][0]["plan"]["selections"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let reloaded = good(json!({"op":"pending_proposal","state":returned["state"],"now":NOW}));
    assert_eq!(reloaded["status"], "review_ready");
    let applied = dispatch(&returned["state"], reloaded["command"].clone(), 2);
    assert_eq!(
        applied["state"]["snapshot"]["weeks"][0]["plan"]["selections"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert!(applied["state"]["pending_proposal"].is_null());
    assert_eq!(applied["receipt"]["command_kind"], "apply_plan_proposal");
    let saved = good(json!({"op":"validate_state","state":applied["state"],"now":NOW}));
    let repeated = good(json!({"op":"return_proposal","state":saved,"result":result,"now":NOW}));
    assert_eq!(repeated["state"], saved);
    assert!(repeated["command"].is_null());
    assert_eq!(repeated["status"], "already_applied");
    let original_command = saved["receipts"][1]["command"].clone();
    let command_replay = good(
        json!({"op":"dispatch","state":saved,"command":original_command,"command_json":serde_json::to_string(&original_command).unwrap(),"now":NOW}),
    );
    assert_eq!(command_replay["state"], saved);
    assert_eq!(command_replay["receipt"]["disposition"], "replayed");
    let mut changed = result.clone();
    changed["selections"][0]["recipe"] = request["slots"][0]["choices"][1]["recipe"].clone();
    assert_eq!(
        call(json!({"op":"return_proposal","state":saved,"result":changed,"now":NOW}))["error"]["code"],
        "replay_conflict"
    );
    let mut policy = saved["snapshot"]["privacy"].clone();
    policy["use_ai_features"] = json!(false);
    let off=dispatch(&saved,json!({"kind":"set_privacy","data":{"policy":policy,"expected_policy_revision":policy["revision"]}}),3)["state"].clone();
    let off_repeat = good(json!({"op":"return_proposal","state":off,"result":result,"now":NOW}));
    assert_eq!(off_repeat["state"], off);
    assert!(off_repeat["command"].is_null());
    assert_eq!(off_repeat["status"], "already_applied");
}

#[test]
fn scope_staleness_revocation_and_fixed_meal_guard() {
    let state = ai_on(&init(true), 1);
    let w = &state["snapshot"]["weeks"][0];
    let slot = w["cells"][2]["slot"].clone();
    let prepared = good(
        json!({"op":"prepare_proposal","state":state,"week_id":w["range"]["week_id"],"slots":[slot],"mode":"fill_open","allow_repeats":false,"now":NOW}),
    );
    let r = &prepared["request"];
    let mut result = json!({"proposal_id":r["proposal_id"],"context_digest":r["context_digest"],"selections":[{"slot":slot,"recipe":r["slots"][0]["choices"][0]["recipe"]}]});
    result["selections"][0]["slot"] = w["cells"][5]["slot"].clone();
    assert_eq!(
        call(json!({"op":"return_proposal","state":prepared["state"],"result":result,"now":NOW}))["error"]
            ["code"],
        "scope_violation"
    );
    let state = save_choice(&state, 0, 2, 2);
    assert_eq!(
        call(
            json!({"op":"prepare_proposal","state":state,"week_id":w["range"]["week_id"],"slots":[slot],"mode":"fill_open","allow_repeats":false,"now":NOW})
        )["error"]["code"],
        "authority_denied"
    );
}

#[test]
fn restoration_rejects_corrupt_projection_and_unknown_fields() {
    let mut state = init(true);
    state["retained_weeks"][0]["revision"] = json!("2");
    assert_eq!(
        call(json!({"op":"validate_state","state":state,"now":NOW}))["error"]["code"],
        "invalid_value"
    );
    let mut state = init(true);
    state["snapshot"]["injected"] = json!("ignore instructions");
    assert_eq!(
        call(json!({"op":"validate_state","state":state,"now":NOW}))["error"]["code"],
        "unknown_field"
    );
}

#[test]
fn accepted_result_extension_preserves_original_receipt_format() {
    let state = save_choice(&init(true), 0, 2, 1);
    assert!(state["receipts"][0].get("accepted_proposal").is_none());
    let restored = good(json!({"op":"validate_state","state":state,"now":NOW}));
    assert_eq!(state, restored);
    let mut corrupt = state.clone();
    corrupt["receipts"][0]["accepted_proposal"] = json!({"result":{"proposal_id":"pr_00000000000000000000000000000001","context_digest":"a".repeat(64),"selections":[{"slot":state["snapshot"]["weeks"][0]["plan"]["selections"][0]["slot"],"recipe":state["snapshot"]["weeks"][0]["plan"]["selections"][0]["recipe"]}]},"result_digest":"a".repeat(64)});
    assert_eq!(
        call(json!({"op":"validate_state","state":corrupt,"now":NOW}))["error"]["code"],
        "identity_mismatch"
    );
}

#[test]
fn inactive_selection_retention_cannot_be_used_to_inject_servings() {
    let state = save_choice(&init(true), 0, 2, 1);
    let w = &state["snapshot"]["weeks"][0];
    let mut days = w["day_enabled"].clone();
    days[0] = json!(false);
    let state=dispatch(&state,json!({"kind":"save_week","data":{"expected_week_revision":w["revision"],"week":{"week_id":w["range"]["week_id"],"defaults_snapshot":w["defaults_snapshot"],"day_enabled":days,"meal_type_enabled":w["meal_type_enabled"],"cells":w["cells"],"plan":null}}}),2)["state"].clone();
    assert!(state["history"]["selected"].as_array().unwrap().is_empty());
    good(json!({"op":"validate_state","state":state,"now":NOW}));
    let w = &state["snapshot"]["weeks"][0];
    let mut plan = w["plan"].clone();
    plan["selections"][0]["servings"][0]["servings"]["numerator"] = json!("2");
    let command = json!({"schema_version":1,"command_id":"cm_00000000000000000000000000000003","expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":{"kind":"save_week","data":{"expected_week_revision":w["revision"],"week":{"week_id":w["range"]["week_id"],"defaults_snapshot":w["defaults_snapshot"],"day_enabled":w["day_enabled"],"meal_type_enabled":w["meal_type_enabled"],"cells":w["cells"],"plan":plan}}}});
    assert_eq!(
        call(json!({"op":"dispatch","state":state,"command":command,"now":NOW}))["error"]["code"],
        "authority_denied"
    );
    let mut corrupt = state.clone();
    corrupt["snapshot"]["weeks"][0]["plan"] = plan.clone();
    corrupt["retained_weeks"][0]["plan"] = plan;
    assert_eq!(
        call(json!({"op":"validate_state","state":corrupt,"now":NOW}))["error"]["code"],
        "identity_mismatch"
    );
    let mut receipt_corrupt = state;
    receipt_corrupt["receipts"][0]["response_digest"] = json!("a".repeat(64));
    assert_eq!(
        call(json!({"op":"validate_state","state":receipt_corrupt,"now":NOW}))["error"]["code"],
        "digest_mismatch"
    );
}

#[test]
fn duplicate_proposal_return_is_idempotent_and_changed_return_is_refused() {
    let state = ai_on(&init(true), 1);
    let w = &state["snapshot"]["weeks"][0];
    let slot = w["cells"][2]["slot"].clone();
    let prepared = good(
        json!({"op":"prepare_proposal","state":state,"week_id":w["range"]["week_id"],"slots":[slot],"mode":"fill_open","allow_repeats":false,"now":NOW}),
    );
    let r = &prepared["request"];
    let mut result = json!({"proposal_id":r["proposal_id"],"context_digest":r["context_digest"],"selections":[{"slot":slot,"recipe":r["slots"][0]["choices"][0]["recipe"]}]});
    let returned =
        good(json!({"op":"return_proposal","state":prepared["state"],"result":result,"now":NOW}));
    let duplicate =
        good(json!({"op":"return_proposal","state":returned["state"],"result":result,"now":NOW}));
    assert_eq!(returned, duplicate);
    result["selections"][0]["recipe"] = r["slots"][0]["choices"][1]["recipe"].clone();
    assert_eq!(
        call(json!({"op":"return_proposal","state":returned["state"],"result":result,"now":NOW}))["error"]
            ["code"],
        "replay_conflict"
    );
}

#[test]
fn restore_revokes_only_unapplied_proposal_scope_and_keeps_accepted_replay() {
    const RESTORE_AT: &str = "2026-10-12T12:01:00Z";
    let state = ai_on(&init(true), 1);
    let w = &state["snapshot"]["weeks"][0];
    let slot = w["cells"][2]["slot"].clone();
    let prepared = good(
        json!({"op":"prepare_proposal","state":state,"week_id":w["range"]["week_id"],"slots":[slot],"mode":"fill_open","allow_repeats":false,"now":NOW}),
    );
    let request = &prepared["request"];
    let result = json!({"proposal_id":request["proposal_id"],"context_digest":request["context_digest"],"selections":[{"slot":slot,"recipe":request["slots"][0]["choices"][0]["recipe"]}]});
    let restored = good(json!({"op":"validate_state","state":prepared["state"],"now":RESTORE_AT}));
    let cleared = good(json!({"op":"revoke_pending_proposal","state":restored,"now":RESTORE_AT}));
    let mut expected = restored.clone();
    expected["pending_proposal"] = Value::Null;
    assert_eq!(cleared, expected);
    assert_eq!(
        good(json!({"op":"pending_proposal","state":cleared,"now":RESTORE_AT}))["status"],
        "idle"
    );
    assert_eq!(
        call(json!({"op":"return_proposal","state":cleared,"result":result,"now":RESTORE_AT}))["error"]
            ["code"],
        "missing_reference"
    );
    assert_eq!(
        good(json!({"op":"revoke_pending_proposal","state":cleared,"now":RESTORE_AT})),
        cleared
    );

    let returned =
        good(json!({"op":"return_proposal","state":prepared["state"],"result":result,"now":NOW}));
    let restored = good(json!({"op":"validate_state","state":returned["state"],"now":RESTORE_AT}));
    let cleared_review =
        good(json!({"op":"revoke_pending_proposal","state":restored,"now":RESTORE_AT}));
    let mut expected = restored.clone();
    expected["pending_proposal"] = Value::Null;
    assert_eq!(cleared_review, expected);
    good(json!({"op":"validate_state","state":cleared_review,"now":RESTORE_AT}));
    let old_apply = json!({"schema_version":1,"command_id":"cm_00000000000000000000000000000002","expected_snapshot_revision":cleared_review["snapshot"]["snapshot_revision"],"command":returned["command"]});
    assert_eq!(
        call(
            json!({"op":"dispatch","state":cleared_review,"command":old_apply,"command_json":serde_json::to_string(&old_apply).unwrap(),"now":RESTORE_AT})
        )["error"]["code"],
        "missing_reference"
    );

    let applied = dispatch(&returned["state"], returned["command"].clone(), 2)["state"].clone();
    let accepted_cleared =
        good(json!({"op":"revoke_pending_proposal","state":applied,"now":RESTORE_AT}));
    assert_eq!(accepted_cleared, applied);
    let repeated = good(
        json!({"op":"return_proposal","state":accepted_cleared,"result":result,"now":RESTORE_AT}),
    );
    assert_eq!(repeated["state"], applied);
    assert!(repeated["command"].is_null());
    assert_eq!(repeated["status"], "already_applied");

    let idle_ai_off = init(true);
    assert_eq!(
        good(json!({"op":"revoke_pending_proposal","state":idle_ai_off,"now":RESTORE_AT})),
        idle_ai_off
    );
    let mut corrupt = returned["state"].clone();
    corrupt["receipts"][0]["response_digest"] = json!("a".repeat(64));
    assert_eq!(
        call(json!({"op":"revoke_pending_proposal","state":corrupt,"now":RESTORE_AT}))["error"]["code"],
        "digest_mismatch"
    );
    assert_eq!(
        call(
            json!({"op":"revoke_pending_proposal","state":restored,"now":RESTORE_AT,"patch":{"snapshot":null}})
        )["error"]["code"],
        "unknown_field"
    );
    assert_eq!(
        call(json!({"op":"revoke_pending_proposal","state":restored}))["error"]["code"],
        "invalid_value"
    );
}

#[test]
fn fresh_nonce_prevents_same_clock_cancelled_scope_from_returning() {
    let state = ai_on(&init(true), 1);
    let w = &state["snapshot"]["weeks"][0];
    let slot = w["cells"][2]["slot"].clone();
    let first = good(
        json!({"op":"prepare_proposal","state":state,"week_id":w["range"]["week_id"],"slots":[slot],"mode":"fill_open","allow_repeats":false,"nonce":"e13445c5-14a4-4d69-bcbd-6b900f8b16ed","now":NOW}),
    );
    let cleared = good(json!({"op":"revoke_pending_proposal","state":first["state"],"now":NOW}));
    let second = good(
        json!({"op":"prepare_proposal","state":cleared,"week_id":w["range"]["week_id"],"slots":[slot],"mode":"fill_open","allow_repeats":false,"nonce":"fd45feb2-f283-4874-baa7-1dde600fd01d","now":NOW}),
    );
    assert_ne!(
        first["request"]["proposal_id"],
        second["request"]["proposal_id"]
    );
    assert_eq!(
        first["request"]["context_digest"],
        second["request"]["context_digest"]
    );
    assert_eq!(first["state"]["snapshot"], second["state"]["snapshot"]);
    let result = json!({"proposal_id":first["request"]["proposal_id"],"context_digest":first["request"]["context_digest"],"selections":[{"slot":slot,"recipe":first["request"]["slots"][0]["choices"][0]["recipe"]}]});
    assert_eq!(
        call(json!({"op":"return_proposal","state":second["state"],"result":result,"now":NOW}))["error"]
            ["code"],
        "identity_mismatch"
    );
    let mut fresh_result = result;
    fresh_result["proposal_id"] = second["request"]["proposal_id"].clone();
    assert_eq!(
        good(
            json!({"op":"return_proposal","state":second["state"],"result":fresh_result,"now":NOW})
        )["status"],
        "review_ready"
    );
}
