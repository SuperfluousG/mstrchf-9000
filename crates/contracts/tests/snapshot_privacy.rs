use mstrchf_contracts::*;
use serde_json::{Value, json};

fn fixture() -> Value {
    serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/contracts/ruling005-current--snapshot.json"),
        )
        .unwrap(),
    )
    .unwrap()
}
fn bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}
fn policy() -> Value {
    json!({"revision":"1","use_ai_features":false,"include_household_requirements":false,"confirmed_at":null})
}
fn initial() -> Value {
    let mut value = fixture();
    value["privacy"] = policy();
    value["profile"] = Value::Null;
    value["weeks"] = json!([]);
    value["jobs"] = json!([]);
    value["conflicts"] = json!([]);
    value
}
fn schema_accepts(value: &Value) -> bool {
    let outputs = schema::render_contracts().unwrap();
    let schema: Value =
        serde_json::from_slice(&outputs["contracts/v1/snapshot.schema.json"]).unwrap();
    jsonschema::draft202012::options()
        .build(&schema)
        .unwrap()
        .is_valid(value)
}
fn accepted(value: &Value) {
    let parsed: Snapshot = parse_document(&bytes(value)).unwrap();
    parsed.validate_shape().unwrap();
    assert_eq!(serde_json::to_value(parsed).unwrap(), *value);
    assert!(schema_accepts(value));
}

#[test]
fn initial_and_partial_snapshots_expose_policy_without_completing_connections() {
    let mut value = initial();
    accepted(&value);
    assert_eq!(value["household_draft"]["completed_stages"], json!([]));
    value["household_draft"]["preferences"] = fixture()["profile"]["preferences"].clone();
    value["household_draft"]["completed_stages"] = json!(["food_preferences"]);
    value["privacy"] = json!({"revision":"2","use_ai_features":true,"include_household_requirements":false,"confirmed_at":"2026-12-28T12:00:00Z"});
    accepted(&value);
    assert!(value["household_draft"]["connections"].is_null());
    assert!(value["profile"].is_null());
    assert_eq!(
        value["household_draft"]["completed_stages"],
        json!(["food_preferences"])
    );
}

#[test]
fn snapshot_policy_is_required_and_non_null() {
    for missing in [true, false] {
        let mut value = initial();
        if missing {
            value.as_object_mut().unwrap().remove("privacy");
        } else {
            value["privacy"] = Value::Null;
        }
        let error = parse_document::<Snapshot>(&bytes(&value)).unwrap_err();
        assert_eq!(
            error.code,
            if missing {
                ErrorCode::MissingField
            } else {
                ErrorCode::InvalidValue
            }
        );
        assert!(error.issues.iter().any(|issue| issue.path == "/privacy"));
        assert!(!schema_accepts(&value));
    }
}

#[test]
fn every_present_policy_copy_must_match_every_authoritative_field() {
    let mut control = fixture();
    control["privacy"] = control["profile"]["connections"]["privacy"].clone();
    control["household_draft"]["connections"] = control["profile"]["connections"].clone();
    control["household_draft"]["completed_stages"] = json!(["connections"]);
    accepted(&control);
    for path in [
        "/household_draft/connections/privacy",
        "/profile/connections/privacy",
    ] {
        for (field, replacement) in [
            ("revision", json!("2")),
            ("use_ai_features", json!(false)),
            ("include_household_requirements", json!(false)),
            ("confirmed_at", Value::Null),
        ] {
            let mut value = control.clone();
            let old = &value.pointer(path).unwrap()[field];
            let replacement = if old == &replacement {
                match field {
                    "use_ai_features" | "include_household_requirements" => json!(true),
                    "confirmed_at" => json!("2026-12-28T12:00:01Z"),
                    _ => json!("3"),
                }
            } else {
                replacement
            };
            value.pointer_mut(path).unwrap()[field] = replacement;
            let parsed: Snapshot = serde_json::from_value(value.clone()).unwrap();
            assert!(
                parsed
                    .validate_shape()
                    .unwrap_err()
                    .iter()
                    .any(|issue| issue.code == ErrorCode::ContextStale && issue.path == path)
            );
            let error = parse_document::<Snapshot>(&bytes(&value)).unwrap_err();
            assert_eq!(error.code, ErrorCode::ContextStale);
            assert_eq!(error.issues[0].path, path);
            assert!(
                schema_accepts(&value),
                "copy equality is semantic, not expressible in ordinary JSON Schema"
            );
        }
    }
}

#[test]
fn snapshot_policy_does_not_create_an_outbound_payload_variant() {
    let value = initial();
    accepted(&value);
    assert!(parse_document::<JobPayload>(&bytes(&value)).is_err());
}
