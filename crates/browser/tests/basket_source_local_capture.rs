//! Real run_json, current native R3 food and read validators. All actors,
//! aliases, returned goods and witness inputs are fictional. These tests do not
//! commission a model, observe Tesco, attest an account or enable basket effects.
use mstrchf_browser::{run_json, maker_packets::porridge_packet_ref,
    basket_source_reads::{PreparedBasketSourceRead, prepared_digest}};
use mstrchf_contracts::*;
use serde_json::{Value, json};

const CREATED: &str = "2026-10-13T07:00:00Z";
const OBSERVED: &str = "2026-10-13T07:00:01Z";
const NOW: &str = "2026-10-13T07:05:00Z";
const DEADLINE: &str = "2026-10-13T08:00:00Z";
fn call(input: Value) -> Value { serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap() }
fn good(input: Value) -> Value { let out = call(input); assert_eq!(out["ok"], true, "{out}"); out["value"].clone() }
fn refuse(input: Value) -> Value { let out = call(input); assert_eq!(out["ok"], false, "{out}"); out["error"].clone() }
fn apply(state: &Value, body: Value, id: u32) -> Value {
    good(json!({"op":"dispatch","state":state,"now":CREATED,"command":{
        "schema_version":1,"command_id":format!("cm_{id:032x}"),
        "expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body}}))["state"].clone()
}
fn week_body(week: &Value) -> Value { json!({"kind":"save_week","data":{
    "expected_week_revision":week["revision"],"week":{"week_id":week["range"]["week_id"],
    "defaults_snapshot":week["defaults_snapshot"],"day_enabled":week["day_enabled"],
    "meal_type_enabled":week["meal_type_enabled"],"cells":week["cells"],"plan":week["plan"]}}}) }
fn privacy(state: &Value, ai: bool, sharing: bool, id: u32) -> Value {
    let mut policy = state["snapshot"]["privacy"].clone();
    policy["use_ai_features"] = json!(ai); policy["include_household_requirements"] = json!(sharing);
    apply(state, json!({"kind":"set_privacy","data":{
        "expected_policy_revision":policy["revision"],"policy":policy}}), id)
}
fn saved(ai: bool) -> Value {
    let mut state = good(json!({"op":"initialize","household_id":"hh_800000000000000000000000000000b0",
        "diner_id":"dn_800000000000000000000000000000b0","zone":"Europe/London",
        "today":"2026-10-13","now":CREATED,"demo":true}));
    let mut profile = state["snapshot"]["profile"].clone();
    profile["diners"][0]["age_band"] = json!("adult");
    profile["diners"][0]["restrictions"] = json!({"kind":"confirmed_none","data":{"confirmed_at":CREATED}});
    profile["diners"][0]["portions"] = json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});
    profile["preferences"]["equipment"] = json!(["hob"]);
    state = apply(&state, json!({"kind":"update_profile","data":{
        "expected_profile_revision":profile["revision"],"profile":profile}}), 0xfb00);
    let mut week = state["snapshot"]["weeks"][0].clone();
    week["meal_type_enabled"]["breakfast"] = json!(true); week["cells"][3]["choice"] = json!("at_home");
    state = apply(&state, week_body(&week), 0xfb01);
    state = apply(&state, json!({"kind":"admit_reviewed_recipe","data":{
        "packet":porridge_packet_ref().unwrap(),"expected_catalogue_revision":"1"}}), 0xfb02);
    let week = &state["snapshot"]["weeks"][0];
    let board = good(json!({"op":"board","state":state,"week_id":week["range"]["week_id"],"slot":week["cells"][3]["slot"],"now":CREATED}));
    let choice = board["choices"].as_array().unwrap().iter().find(|choice|
        choice["selection"]["recipe"] == json!(porridge_packet_ref().unwrap().recipe)).unwrap();
    let mut week = week.clone(); week["plan"] = choice["plan"].clone();
    state = apply(&state, week_body(&week), 0xfb03);
    if ai { state = privacy(&state, true, false, 0xfb04); }
    state
}
fn prepare_input(state: &Value) -> Value { json!({"op":"basket_source_prepare","state":state,
    "week_id":state["snapshot"]["weeks"][0]["range"]["week_id"],
    "reader":{"provider":"codex_local","native_task_ref":"synthetic-separately-commissioned-chat-not-host-attested"},
    "key":{"retailer":"tesco","account_ref":"synthetic-local-account-alias","basket_id":"bk_800000000000000000000000000000b0"},
    "delivery":{"retailer":"tesco","account_ref":"synthetic-local-account-alias","context_ref":"synthetic-local-delivery-alias","delivery_date":null,"slot_ref":null},
    "job_id":"jb_800000000000000000000000000000b0","read_id":"rd_800000000000000000000000000000b0","now":CREATED,"deadline":DEADLINE}) }
fn prepare(state: &Value) -> Value { good(prepare_input(state)) }
fn unknown() -> Value { json!({"kind":"unknown","data":{"reason":"not_exposed"}}) }
fn known(value: Value) -> Value { json!({"kind":"known","data":{"value":value}}) }
fn product(id: &str) -> Value { json!({"retailer":"tesco","product_id":id,"variant_id":null}) }
fn payload(prepared: &Value) -> Value { json!({"format":"mstrchf-basket-source-return","version":1,
    "read_id":prepared["request"]["scope"]["data"]["read_scope"]["read_id"],
    "job_id":prepared["request"]["job_id"],"input_digest":prepared["request"]["input_digest"],
    "observed_at":OBSERVED,"completeness":"complete","observed_lines":[
        {"line_ref":"synthetic-oats","product":product("100001"),"title":"Fictional oats; no Tesco observation",
         "quantity":known(json!(2)),"unit_price":known(json!({"pence":"125","currency":"GBP","label":"Fictional guide price per sold pack"}))},
        {"line_ref":"synthetic-manual-good","product":product("100002"),"title":"Fictional unrelated groceries",
         "quantity":known(json!(3)),"unit_price":unknown()}],"delivery_charge":unknown(),
    "notes":["Fictional external-capture fixture. No source truth, account or execution is verified."]}) }
fn inspect(state: &Value, prepared: &Value, local: bool) -> Value { json!({"op":if local {"basket_source_local_inspect"} else {"basket_source_inspect"},"state":state,"prepared":prepared,"now":NOW}) }
fn check(state: &Value, prepared: &Value, returned: &Value, local: bool) -> Value { json!({"op":if local {"basket_source_local_check_return"} else {"basket_source_check_return"},"state":state,"prepared":prepared,"returned":returned,"observation_id":"ob_800000000000000000000000000000b0","now":NOW}) }
fn accept(state: &Value, prepared: &Value, returned: &Value, observation: &Value, local: bool) -> Value { json!({"op":if local {"basket_source_local_accept"} else {"basket_source_accept"},"state":state,"prepared":prepared,"returned":returned,"observation":observation,"evidence_id":"ev_800000000000000000000000000000b0","now":NOW}) }
fn audit(prepared: &Value, returned: Value, observation: Value, accepted: Value) -> Value { json!({"op":"basket_source_audit","prepared":prepared,"returned":returned,"observation":observation,"accepted":accepted}) }
fn resign(prepared: &mut Value) {
    let mut request: JobRequestV1 = serde_json::from_value(prepared["request"].clone()).unwrap();
    request.input_digest = input_digest(&RequestMaterial::from(&request)).unwrap(); prepared["request"] = json!(request);
    let typed: PreparedBasketSourceRead = serde_json::from_value(prepared.clone()).unwrap();
    prepared["prepared_digest"] = json!(prepared_digest(&typed).unwrap());
}

#[test]
fn complete_external_capture_and_explicit_witness_keep_ai_off_food_jobs_and_unqualified_assessment() {
    let state = saved(false); let before = state.clone(); let prepared = prepare(&state); let returned = payload(&prepared);
    assert_eq!(state["snapshot"]["privacy"]["use_ai_features"], false);
    assert_eq!(state["snapshot"]["privacy"]["include_household_requirements"], false);
    assert_eq!(good(inspect(&state, &prepared, true)), json!({"current":true}));
    let checked = good(check(&state, &prepared, &returned, true)); let observation = &checked["observation"];
    assert_eq!(checked["payload_digest"], json!(canonical_digest(&returned).unwrap()));
    assert_eq!(observation["reader"], prepared["expected_reader"]);
    assert_eq!(observation["reader"]["provider"], "codex_local", "AI-off does not invent a non-AI observer");
    assert_eq!(observation["body"]["data"]["snapshot"]["lines"][1]["quantity"], 3);
    let receipt = good(accept(&state, &prepared, &returned, observation, true));
    assert_eq!(receipt["accepted"]["digest"], checked["observation_digest"]);
    assert_eq!(receipt["assessment"], json!({"source_truth_assessed":false,"account_verified":false,
        "execution_baseline_status":"unqualified","effect_enabled":false,"price_currentness":"unknown"}));
    assert_eq!(good(audit(&prepared, returned.clone(), observation.clone(), receipt["accepted"].clone()))["valid"], true);
    for input in [inspect(&state, &prepared, false), check(&state, &prepared, &returned, false), accept(&state, &prepared, &returned, observation, false)] {
        let error = refuse(input); assert_eq!(error["code"], "authority_denied");
        assert_eq!(error["issues"][0]["path"], "/basket/source/ai_features");
    }
    assert_eq!(good(json!({"op":"validate_state","state":state,"now":NOW})), before);
    assert_eq!(state["snapshot"]["jobs"], json!([]));
    for key in ["grant_token", "intent", "approval", "baseline", "existing_credit", "read_mode", "ai_enabled"] { assert!(receipt.get(key).is_none()); }
}

#[test]
fn incomplete_capture_retains_ambiguity_without_observation_or_witness_and_empty_complete_is_distinct() {
    let state = saved(false); let prepared = prepare(&state); let mut returned = payload(&prepared);
    let complete = good(check(&state, &prepared, &returned, true));
    returned["completeness"] = json!("incomplete");
    for line in returned["observed_lines"].as_array_mut().unwrap() { line["product"] = Value::Null; }
    returned["observed_lines"][0]["quantity"] = unknown();
    returned["observed_lines"][1]["quantity"] = json!({"kind":"conflicting","data":{"values":[2,3]}});
    let checked = good(check(&state, &prepared, &returned, true));
    assert_eq!(checked["observation"], Value::Null); assert_eq!(checked["observation_digest"], Value::Null);
    assert_eq!(checked["payload_digest"], json!(canonical_digest(&returned).unwrap()));
    assert_eq!(good(audit(&prepared, returned.clone(), Value::Null, Value::Null))["payload_digest"], checked["payload_digest"]);
    refuse(accept(&state, &prepared, &returned, &complete["observation"], true));
    returned["completeness"] = json!("complete"); returned["observed_lines"] = json!([]);
    let empty = good(check(&state, &prepared, &returned, true));
    assert_eq!(empty["observation"]["body"]["data"]["snapshot"]["lines"], json!([]));
    assert_eq!(empty["observation"]["body"]["data"]["snapshot"]["delivery_charge"], unknown());
    assert_eq!(good(accept(&state, &prepared, &returned, &empty["observation"], true))["assessment"]["execution_baseline_status"], "unqualified");
}

#[test]
fn local_capture_preserves_exact_read_identity_clock_and_deadline_refusals() {
    let state = saved(false); let prepared = prepare(&state); let returned = payload(&prepared);
    let checked = good(check(&state, &prepared, &returned, true));
    for at in ["2026-10-13T06:59:59Z", DEADLINE] {
        for mut input in [inspect(&state, &prepared, true), check(&state, &prepared, &returned, true), accept(&state, &prepared, &returned, &checked["observation"], true)] {
            input["now"] = json!(at); let error = refuse(input);
            if at == DEADLINE { assert_eq!(error["code"], "context_stale"); }
        }
    }
    for field in ["read_id", "job_id", "input_digest", "observed_at"] {
        let mut wrong = returned.clone(); wrong[field] = match field {
            "read_id" => json!("rd_800000000000000000000000000000b1"),
            "job_id" => json!("jb_800000000000000000000000000000b1"),
            "input_digest" => json!("f".repeat(64)), _ => json!(CREATED),
        };
        refuse(check(&state, &prepared, &wrong, true)); refuse(accept(&state, &prepared, &wrong, &checked["observation"], true));
    }
    let mut future = returned.clone(); future["observed_at"] = json!("2026-10-13T07:05:01Z"); refuse(check(&state, &prepared, &future, true));
    let mut equal = check(&state, &prepared, &returned, true); equal["now"] = json!(OBSERVED); assert!(good(equal)["observation_digest"].is_string());
}

#[test]
fn unchanged_ai_off_value_does_not_bypass_new_privacy_revision_or_changed_native_food() {
    let state = saved(false); let prepared = prepare(&state); let returned = payload(&prepared);
    let checked = good(check(&state, &prepared, &returned, true));
    let same_off = privacy(&state, false, false, 0xfb10); let sharing_changed = privacy(&state, false, true, 0xfb11);
    let mut profile = state["snapshot"]["profile"].clone(); profile["diners"][0]["display_name"] = json!("Changed fictional context");
    let profile_changed = apply(&state, json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}), 0xfb12);
    let week = state["snapshot"]["weeks"][0].clone(); let week_changed = apply(&state, week_body(&week), 0xfb13);
    for changed in [same_off, sharing_changed, profile_changed, week_changed] {
        for input in [inspect(&changed, &prepared, true), check(&changed, &prepared, &returned, true), accept(&changed, &prepared, &returned, &checked["observation"], true)] { refuse(input); }
        assert_eq!(changed["snapshot"]["privacy"]["use_ai_features"], false);
    }
    let mut empty_week = week; empty_week["plan"]["selections"] = json!([]);
    let empty = apply(&state, week_body(&empty_week), 0xfb14); refuse(prepare_input(&empty)); refuse(inspect(&empty, &prepared, true));
    assert_eq!(good(audit(&prepared, returned, checked["observation"].clone(), Value::Null))["valid"], true, "Stale current context does not rewrite historical source custody");
}

#[test]
fn local_route_cannot_resign_read_scope_into_mutation_or_import_another_observer_account_or_authority() {
    let state = saved(false); let prepared = prepare(&state); let returned = payload(&prepared);
    let observation = good(check(&state, &prepared, &returned, true))["observation"].clone();
    for attack in ["capability", "writes", "purpose", "food"] {
        let mut wrong = prepared.clone(); match attack {
            "capability" => wrong["request"]["required_capabilities"] = json!(["browser_read", "browser_mutate"]),
            "writes" => wrong["request"]["context_stamp"]["write_weeks"] = wrong["request"]["context_stamp"]["read_weeks"].clone(),
            "purpose" => { wrong["request"]["scope"]["data"]["read_scope"]["purpose"] = json!("verification"); wrong["request"]["payload"]["data"]["read_scope"] = wrong["request"]["scope"]["data"]["read_scope"].clone(); },
            _ => wrong["food"]["derived_digest"] = json!("f".repeat(64)),
        }
        resign(&mut wrong); refuse(inspect(&state, &wrong, true)); refuse(check(&state, &wrong, &returned, true));
    }
    for attack in ["reader", "resource", "quantity"] {
        let mut wrong = observation.clone(); match attack {
            "reader" => wrong["reader"]["native_task_ref"] = json!("another-fictional-reader"),
            "resource" => wrong["resource"]["data"]["key"]["account_ref"] = json!("another-account"),
            _ => wrong["body"]["data"]["snapshot"]["lines"][0]["quantity"] = json!(9),
        }
        refuse(accept(&state, &prepared, &returned, &wrong, true));
    }
    for (field, value) in [("reader", prepared["expected_reader"].clone()), ("account_verified", json!(true)), ("read_mode", json!("external_capture")), ("approval", json!("self-approved"))] {
        let mut wrong = returned.clone(); wrong[field] = value; refuse(check(&state, &prepared, &wrong, true));
    }
    let mut malformed_state = state; malformed_state["snapshot"]["snapshot_revision"] = json!("999"); refuse(inspect(&malformed_state, &prepared, true));
}

#[test]
fn ai_enabled_legacy_results_and_historical_v1_prepared_bytes_remain_identical() {
    let state = saved(true); let prepared = prepare(&state); let returned = payload(&prepared);
    let typed: PreparedBasketSourceRead = serde_json::from_value(prepared.clone()).unwrap();
    assert_eq!(canonical_bytes(&prepared).unwrap(), canonical_bytes(&typed).unwrap());
    assert_eq!(typed.prepared_digest, prepared_digest(&typed).unwrap());
    assert_eq!(good(inspect(&state, &prepared, true)), good(inspect(&state, &prepared, false)));
    let legacy = good(check(&state, &prepared, &returned, false)); let local = good(check(&state, &prepared, &returned, true));
    assert_eq!(canonical_bytes(&legacy).unwrap(), canonical_bytes(&local).unwrap());
    let old_accept = good(accept(&state, &prepared, &returned, &legacy["observation"], false));
    let local_accept = good(accept(&state, &prepared, &returned, &local["observation"], true));
    assert_eq!(canonical_bytes(&old_accept).unwrap(), canonical_bytes(&local_accept).unwrap());
    let historical = good(audit(&prepared, returned, legacy["observation"].clone(), old_accept["accepted"].clone()));
    assert_eq!(historical["observation_digest"], legacy["observation_digest"]);
    assert_eq!(state["snapshot"]["privacy"]["use_ai_features"], true);
}
