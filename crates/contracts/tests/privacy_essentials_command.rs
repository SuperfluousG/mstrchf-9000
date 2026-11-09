use mstrchf_contracts::*;
use serde_json::{Value, json};

fn command(kind: &str) -> Value {
    json!({"schema_version":1,"command_id":"cm_54000000000000000000000000000001","expected_snapshot_revision":"1","command":{"kind":kind,"data":{"policy":{"revision":"1","use_ai_features":true,"include_household_requirements":false,"confirmed_at":"2026-10-14T15:00:00Z"},"expected_policy_revision":"1"}}})
}

#[test]
fn distinct_local_only_command_preserves_the_original_privacy_wire() {
    for (kind, expected) in [
        ("set_privacy", CommandKind::SetPrivacy),
        (
            "set_privacy_preserving_essentials",
            CommandKind::SetPrivacyPreservingEssentials,
        ),
    ] {
        let raw = command(kind);
        let parsed: AppCommandV1 = parse_document(&serde_json::to_vec(&raw).unwrap()).unwrap();
        assert_eq!(serde_json::to_value(&parsed).unwrap(), raw);
        assert_eq!(parsed.command.kind(), expected);
        assert!(authorize_command(expected, CallerClass::LocalUi).is_ok());
        for caller in [CallerClass::ExchangeImport, CallerClass::QualifiedAdapter] {
            assert_eq!(
                authorize_command(expected, caller).unwrap_err().code,
                ErrorCode::AuthorityDenied
            );
        }
    }
    assert_ne!(
        canonical_digest(&command("set_privacy")).unwrap(),
        canonical_digest(&command("set_privacy_preserving_essentials")).unwrap()
    );
}

#[test]
fn carry_command_does_not_accept_confirmation_or_effect_authority_fields() {
    let source = command("set_privacy_preserving_essentials");
    for key in [
        "confirmations",
        "demand_digest",
        "carry_authorised",
        "approval",
    ] {
        let mut raw = source.clone();
        raw["command"]["data"][key] = json!(true);
        assert!(parse_document::<AppCommandV1>(&serde_json::to_vec(&raw).unwrap()).is_err());
    }
    let mut raw = source;
    raw["command"]["data"]
        .as_object_mut()
        .unwrap()
        .remove("expected_policy_revision");
    assert!(parse_document::<AppCommandV1>(&serde_json::to_vec(&raw).unwrap()).is_err());
}
