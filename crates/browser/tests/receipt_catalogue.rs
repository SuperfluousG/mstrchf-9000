use mstrchf_browser::{catalogue::*, run_json};
use mstrchf_contracts::*;
use serde_json::{Value, json};

const NOW: &str = "2026-10-12T12:00:00Z";
fn call(request: Value) -> Value {
    serde_json::from_slice(&run_json(&serde_json::to_vec(&request).unwrap())).unwrap()
}
fn good(request: Value) -> Value {
    let response = call(request);
    assert_eq!(response["ok"], true, "{response}");
    response["value"].clone()
}
fn fixture() -> Value {
    serde_json::from_slice(include_bytes!("fixtures/legacy-engineering-receipts.json")).unwrap()
}
fn cold_restore(state: Value) -> Value {
    // Food and audit caches are thread-local: this is genuinely a cold journal.
    std::thread::spawn(move || good(json!({"op":"validate_state","state":state,"now":NOW})))
        .join().unwrap()
}
fn frozen_pin() -> Value {
    serde_json::to_value(legacy_engineering_v1().unwrap().pin()).unwrap()
}
fn replay(state: &Value, index: usize) -> Value {
    let command = &state["receipts"][index]["command"];
    good(json!({"op":"dispatch","state":state,"command":command,"now":NOW}))
}

#[test]
fn cold_legacy_restore_keeps_original_digests_food_essentials_and_accepted_replay() {
    let fixture = fixture();
    let state = &fixture["state"];
    assert!(state["receipts"].as_array().unwrap().iter().all(|r| r.get("food_catalogue_pin").is_none()));
    assert_eq!(cold_restore(state.clone()), *state);
    let board = good(json!({"op":"board","state":state,"now":NOW}));
    assert_eq!(board["details"]["derived"], fixture["derived"]);
    assert_eq!(board["details"]["essentials"], state["essentials"]);
    for index in 0..4 {
        let replayed = replay(state, index);
        assert_eq!(replayed["state"], *state);
        assert_eq!(replayed["receipt"]["disposition"], "replayed");
        for field in ["command_digest", "response_digest", "receipt_snapshot_revision"] {
            assert_eq!(replayed["receipt"][field], fixture["acknowledgements"][index][field]);
        }
    }
    let repeated = good(json!({"op":"return_proposal","state":state,"result":fixture["accepted_result"],"now":NOW}));
    assert_eq!(repeated["status"], "already_applied");
    assert!(repeated["command"].is_null());
    assert_eq!(repeated["state"], *state);
}

#[test]
fn new_receipts_pin_frozen_food_and_restore_alongside_unmodified_legacy_receipts() {
    let original = fixture()["state"].clone();
    let board = good(json!({"op":"board","state":original,"now":NOW}));
    let demand = &board["details"]["derived"]["demand"];
    let line = demand["lines"].as_array().unwrap().iter().find(|v| v["essential"] == true).unwrap();
    let command = json!({"schema_version":1,"command_id":"cm_00000000000000000000000000000005","expected_snapshot_revision":original["snapshot"]["snapshot_revision"],"command":{"kind":"confirm_essentials","data":{"confirmations":[{"line_id":line["line_id"],"choice":"add_to_shop","displayed_quantity":line["quantity"],"demand_digest":demand["digest"],"confirmed_at":NOW}]}}});
    let applied = good(json!({"op":"dispatch","state":original,"command":command,"now":NOW}));
    let state = &applied["state"];
    assert_eq!(state["receipts"][4]["food_catalogue_pin"], frozen_pin());
    for index in 0..4 { assert_eq!(state["receipts"][index], original["receipts"][index]); }
    assert_eq!(cold_restore(state.clone()), *state);
    let parsed: AppCommandV1 = parse_document(&serde_json::to_vec(&command).unwrap()).unwrap();
    assert_eq!(canonical_digest(&parsed).unwrap().0, applied["receipt"]["command_digest"]);
    let snapshot: Snapshot = parse_document(&serde_json::to_vec(&state["snapshot"]).unwrap()).unwrap();
    assert_eq!(canonical_digest(&snapshot).unwrap().0, applied["receipt"]["response_digest"]);
    let repeated = replay(state, 4);
    assert_eq!(repeated["state"], *state);
    assert_eq!(repeated["receipt"]["response_digest"], applied["receipt"]["response_digest"]);
    // Removing the additive pin is the exact supported legacy representation.
    let mut omitted = state.clone();
    omitted["receipts"][4].as_object_mut().unwrap().remove("food_catalogue_pin");
    assert_eq!(cold_restore(omitted.clone()), omitted);
}

#[test]
fn supplied_catalogue_pins_are_checked_before_warm_food_cache_lookup() {
    let mut state = fixture()["state"].clone();
    for receipt in state["receipts"].as_array_mut().unwrap() { receipt["food_catalogue_pin"] = frozen_pin(); }
    assert_eq!(good(json!({"op":"validate_state","state":state,"now":NOW})), state);
    let mutations = [
        ("revision", json!("2"), "context_stale"),
        ("manifest_digest", json!("a".repeat(64)), "digest_mismatch"),
        ("entries", json!([]), "digest_mismatch"),
    ];
    for (field, value, code) in mutations {
        let mut corrupted = state.clone();
        corrupted["receipts"][0]["food_catalogue_pin"][field] = value;
        assert_eq!(call(json!({"op":"validate_state","state":corrupted,"now":NOW}))["error"]["code"], code);
    }
    for field in ["packet_digest", "recipe"] {
        let mut corrupted = state.clone();
        if field == "recipe" { corrupted["receipts"][0]["food_catalogue_pin"]["entries"][0][field]["digest"] = json!("b".repeat(64)); }
        else { corrupted["receipts"][0]["food_catalogue_pin"]["entries"][0][field] = json!("b".repeat(64)); }
        assert_eq!(call(json!({"op":"validate_state","state":corrupted,"now":NOW}))["error"]["code"], "digest_mismatch");
    }
    let mut malformed = state;
    malformed["receipts"][0]["food_catalogue_pin"]["evidence_basis"] = json!({});
    assert_eq!(call(json!({"op":"validate_state","state":malformed,"now":NOW}))["error"]["code"], "unknown_field");
}

#[test]
fn new_self_consistent_declaration_revision_cannot_reinterpret_original_food_receipts() {
    let legacy = legacy_engineering_v1().unwrap();
    let mut changed = legacy.declared_recipes()[0].clone();
    changed.revision = Revision(2);
    changed.title = Title("Deliberate new declaration; no food admission".into());
    changed.ingredients[0].quantity.0.as_mut().unwrap().amount = PositiveRational::new(500, 1).unwrap();
    let future = non_admitted_manifest(Revision(2), vec![NonAdmittedRecipePacket {schema_version:SchemaVersion,status:NonAdmittedStatus::NonAdmitted,recipe:changed}]).unwrap();
    let _declarations = non_admitted_view(&canonical_bytes(&future).unwrap()).unwrap();
    let fixture = fixture();
    let state = fixture["state"].clone();
    // Merely constructing another immutable declaration view changes no food view.
    let board = good(json!({"op":"board","state":state,"now":NOW}));
    assert_eq!(board["details"]["derived"], fixture["derived"]);
    let refs = board["details"]["library"].as_array().unwrap().iter().map(|entry| entry["reference"].clone()).collect::<Vec<_>>();
    assert_eq!(refs, legacy.pin().entries.iter().map(|entry| serde_json::to_value(&entry.recipe).unwrap()).collect::<Vec<_>>());
    let mut spliced = state;
    spliced["receipts"][0]["food_catalogue_pin"] = serde_json::to_value(&future.catalogue).unwrap();
    assert_eq!(call(json!({"op":"validate_state","state":spliced,"now":NOW}))["error"]["code"], "context_stale");
}

#[test]
fn cold_current_device_archive_fixture_retains_all_110_legacy_conflicts() {
    let state: Value = serde_json::from_slice(include_bytes!("fixtures/current-device-conflicts-110.json")).unwrap();
    assert_eq!(cold_restore(state.clone()), state);
    let first = good(json!({"op":"conflicts","state":state,"offset":0,"limit":100,"now":NOW}));
    let last = good(json!({"op":"conflicts","state":state,"offset":100,"limit":100,"now":NOW}));
    assert_eq!(first["total"], 110);
    assert_eq!(first["conflicts"].as_array().unwrap().len(), 100);
    assert_eq!(last["conflicts"].as_array().unwrap().len(), 10);
}

#[test]
fn frozen_engineering_asset_premises_cannot_drift_silently() {
    assert_eq!(bytes_digest(include_bytes!("../assets/catalogue/engineering-v1/food-premises-v1.rs")).0,
        "f1520b18aaa63c7ec3fef4fae0ed012ef67531fb3f05a7cd8d990a1d5b446718");
    let placeholder = bytes_digest(include_bytes!("../assets/catalogue/engineering-v1/engineering-placeholder.png"));
    assert_eq!(placeholder.0, "ea763c6667d13d84e962dfc3e08ea3c75f9f29cb3543543156e4065d1957b9a0");
    for recipe in &legacy_engineering_v1().unwrap().declared_recipes()[3..] {
        assert_eq!(recipe.image.0.as_ref().unwrap().digest.0, placeholder.0);
    }
    assert_eq!(bytes_digest(include_bytes!("fixtures/legacy-engineering-receipts.json")).0,
        "0f36a8c1820b771504c28c1ebfc9295dae24555dc89e5732c8ddf2bded127b49");
}

#[test]
fn an_unregistered_packet_cannot_gain_authority_through_an_admission_command() {
    let mut state = fixture()["state"].clone();
    let packet = legacy_engineering_v1().unwrap().pin().entries[0].clone();
    let command = json!({"schema_version":1,"command_id":"cm_00000000000000000000000000000005","expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":{"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":"1"}}});
    assert_eq!(call(json!({"op":"dispatch","state":state,"command":command,"now":NOW}))["error"]["code"], "missing_reference");
    // Recomputing public checksums cannot smuggle an admission into a backup.
    let original = &mut state["receipts"][3];
    original["command"]["command"] = command["command"].clone();
    original.as_object_mut().unwrap().remove("accepted_proposal");
    let envelope: AppCommandV1 = parse_document(&serde_json::to_vec(&original["command"]).unwrap()).unwrap();
    original["command_digest"] = json!(canonical_digest(&envelope).unwrap());
    assert_eq!(call(json!({"op":"validate_state","state":state,"now":NOW}))["error"]["code"], "missing_reference");
}
