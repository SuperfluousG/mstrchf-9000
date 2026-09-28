use mstrchf_contracts::{HttpUrl, parse_document, schema};
use serde_json::{Value, json};

fn bytes(value: &Value) -> Vec<u8> {
    serde_json::to_vec(value).unwrap()
}

fn fixture(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/contracts")
        .join(name);
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

#[test]
fn http_and_https_share_the_exact_2048_character_boundary_in_schema_and_runtime() {
    let rendered = schema::render_contracts().unwrap();
    let candidate: Value = serde_json::from_slice(
        rendered
            .get("contracts/v1/recipe-candidate.schema.json")
            .unwrap(),
    )
    .unwrap();
    let validator = jsonschema::draft202012::options()
        .build(candidate.pointer("/$defs/HttpUrl").unwrap())
        .unwrap();

    for scheme in ["http://", "https://"] {
        for character in ["x", "é", "🧁"] {
            let permitted = format!(
                "{scheme}{}",
                character.repeat(2048 - scheme.chars().count())
            );
            let too_long = format!("{permitted}x");
            assert_eq!(permitted.chars().count(), 2048);
            assert!(parse_document::<HttpUrl>(&bytes(&json!(permitted))).is_ok());
            assert!(
                validator.is_valid(&json!(permitted)),
                "schema rejected {scheme} boundary for {character}"
            );
            assert!(parse_document::<HttpUrl>(&bytes(&json!(too_long))).is_err());
            assert!(
                !validator.is_valid(&json!(too_long)),
                "schema accepted {scheme} over-boundary for {character}"
            );
        }
    }

    for control in ["https://bad\nurl", "https://bad\u{0001}url"] {
        assert!(parse_document::<HttpUrl>(&bytes(&json!(control))).is_err());
    }
    assert!(parse_document::<HttpUrl>(br#""https://bad\ud800url""#).is_err());
}

#[test]
fn request_job_command_rejects_incompatible_action_and_scope_at_admission() {
    let cases = [
        ("setup", "job-request-setup.json"),
        ("plan", "job-request.json"),
        ("add_adapt_recipe", "job-request-recipe-candidate.json"),
        ("prepare_shop", "job-request-product-observation.json"),
        ("finish_week", "job-request-calendar-publication.json"),
        ("check_booking", "job-request-booking-check.json"),
    ];
    let scopes = cases
        .iter()
        .map(|(_, file)| fixture(file)["scope"].clone())
        .collect::<Vec<_>>();

    for (action_index, (action, _)) in cases.iter().enumerate() {
        for (scope_index, scope) in scopes.iter().enumerate() {
            let mut command = fixture("app-command.json");
            command["command"] = json!({
                "kind": "request_job",
                "data": {"action": action, "scope": scope}
            });
            let result = schema::parse_root("app-command", &bytes(&command));
            if action_index == scope_index {
                assert!(
                    result.is_ok(),
                    "valid pair {action} with scope {scope_index}: {result:?}"
                );
            } else {
                assert_eq!(
                    result.unwrap_err().code,
                    mstrchf_contracts::ErrorCode::ScopeViolation,
                    "incompatible pair {action} with scope {scope_index}"
                );
            }
        }
    }
}

#[test]
fn hold_slot_grant_action_expiry_must_follow_grant_time() {
    let mut grant = fixture("operation-grant.json");
    grant["action"] = json!({
        "kind": "hold_slot",
        "data": {
            "slot_ref": "synthetic-slot-1",
            "expires_at": "2026-12-28T12:00:59Z",
            "nonfinancial_evidence_digest": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
        }
    });
    let error = schema::parse_root("operation-grant", &bytes(&grant)).unwrap_err();
    assert_eq!(error.code, mstrchf_contracts::ErrorCode::InvalidValue);

    grant["action"]["data"]["expires_at"] = json!("2026-12-28T12:01:00Z");
    let error = schema::parse_root("operation-grant", &bytes(&grant)).unwrap_err();
    assert_eq!(error.code, mstrchf_contracts::ErrorCode::InvalidValue);

    grant["action"]["data"]["expires_at"] = json!("2026-12-28T12:02:00Z");
    assert!(schema::parse_root("operation-grant", &bytes(&grant)).is_ok());
}
