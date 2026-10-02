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
fn url_schema_matches_control_and_unicode_whitespace_admission() {
    let generated = schema::render_contracts().unwrap();
    let candidate: Value =
        serde_json::from_slice(&generated["contracts/v1/recipe-candidate.schema.json"]).unwrap();
    let validator = jsonschema::draft202012::options()
        .build(candidate.pointer("/$defs/HttpUrl").unwrap())
        .unwrap();
    // Explicit C0, DEL/C1 and Unicode White_Space values. Do not use a regex
    // shorthand as the test oracle: ECMAScript whitespace includes FEFF.
    let forbidden = (0..=0x20)
        .chain(0x7f..=0xa0)
        .chain([0x1680])
        .chain(0x2000..=0x200a)
        .chain([0x2028, 0x2029, 0x202f, 0x205f, 0x3000]);
    let permitted = [
        0x21, 0x7e, 0xa1, 0x167f, 0x1681, 0x180e, 0x200b, 0x200d, 0x2027, 0x202a, 0x202e, 0x2030,
        0x205e, 0x2060, 0x3001, 0xfeff, 0xe9, 0x1f980,
    ];
    let mut failures = Vec::new();
    for (scalar, expected) in forbidden
        .map(|c| (c, false))
        .chain(permitted.into_iter().map(|c| (c, true)))
    {
        let character = char::from_u32(scalar).unwrap();
        for scheme in ["http://", "https://"] {
            for suffix in [
                format!("{character}example.test/ab"),
                format!("example.test/a{character}b"),
                format!("example.test/ab{character}"),
            ] {
                let value = json!(format!("{scheme}{suffix}"));
                let runtime = parse_document::<HttpUrl>(&bytes(&value)).is_ok();
                let schema = validator.is_valid(&value);
                if runtime != expected || schema != expected {
                    failures.push(format!("{scheme} U+{scalar:04X} in {suffix:?}: runtime={runtime}, schema={schema}, expected={expected}"));
                }
            }
        }
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn request_job_command_rejects_incompatible_action_and_scope_at_admission() {
    let cases = [
        ("setup", "job-request-setup.json"),
        ("plan", "ruling004-current--job-request.json"),
        ("add_adapt_recipe", "job-request-recipe-candidate.json"),
        (
            "prepare_shop",
            "ruling004-current--job-request-product-observation.json",
        ),
        (
            "finish_week",
            "ruling004-current--job-request-calendar-publication.json",
        ),
        (
            "check_booking",
            "ruling004-current--job-request-booking-check.json",
        ),
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
    let mut grant = fixture("ruling004-current--operation-grant.json");
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
