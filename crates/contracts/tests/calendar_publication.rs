use mstrchf_contracts::*;
use serde_json::{json, Value};

fn command() -> Value {
    json!({"schema_version":1,"command_id":"cm_10000000000000000000000000000001","expected_snapshot_revision":"1",
        "command":{"kind":"confirm_calendar_publication","data":{"confirmation":{
            "preparation_command_id":"cm_10000000000000000000000000000002",
            "order_id":"or_10000000000000000000000000000001","destination_id":"cd_10000000000000000000000000000001",
            "event_batch_digest":"a".repeat(64),"calendar_id":"synthetic@example.invalid",
            "source":{"kind":"user_reviewed_google_read","data":{"read_id":"rd_10000000000000000000000000000001","read_digest":"b".repeat(64)}},
            "observed_at":"2026-10-14T12:00:00Z","events":[{
                "event_id":"ce_10000000000000000000000000000001","provider_event_id":"synthetic-google-event",
                "title":"Synthetic observed meal","body":"Exact original method.\nServe.",
                "start":"2026-10-20T17:00:00Z","end":"2026-10-20T17:30:00Z","zone":"Europe/London","observed_at":"2026-10-14T12:00:00Z"
            }]
        }}}})
}
fn parse(value: &Value) -> Result<AppCommandV1, DomainError> { parse_document(&serde_json::to_vec(value).unwrap()) }

#[test]
fn confirmation_is_strict_round_trippable_and_local_ui_only() {
    let value = command(); let typed = parse(&value).unwrap();
    assert_eq!(serde_json::to_value(&typed).unwrap(), value);
    assert_eq!(typed.command.kind(), CommandKind::ConfirmCalendarPublication);
    assert!(authorize_command(typed.command.kind(), CallerClass::LocalUi).is_ok());
    for caller in [CallerClass::ExchangeImport, CallerClass::QualifiedAdapter] {
        assert_eq!(authorize_command(typed.command.kind(), caller).unwrap_err().code, ErrorCode::AuthorityDenied);
    }
}

#[test]
fn unknown_fields_unqualified_sources_and_bad_wire_values_are_refused() {
    for case in 0..11 {
        let mut value = command(); let confirmation = &mut value["command"]["data"]["confirmation"];
        match case {
            0 => confirmation["authenticated"] = json!(true),
            1 => confirmation["source"]["kind"] = json!("authorised_read"),
            2 => confirmation["source"]["data"]["verified"] = json!(true),
            3 => confirmation["events"][0]["state"] = json!("present"),
            4 => confirmation["preparation_command_id"] = json!("cm_00000000000000000000000000000000"),
            5 => confirmation["event_batch_digest"] = json!("A".repeat(64)),
            6 => confirmation["observed_at"] = json!("2026-02-30T12:00:00Z"),
            7 => confirmation["events"][0]["start"] = json!("2026-10-20T18:00:00+01:00"),
            8 => confirmation["events"][0]["zone"] = json!("Mars/Base"),
            9 => confirmation["events"][0]["body"] = json!("\nSilently trimmed method"),
            _ => confirmation["calendar_id"] = json!(""),
        }
        assert!(parse(&value).is_err(), "case {case}");
    }
}

#[test]
fn confirmation_event_set_is_bounded_and_required() {
    let mut value = command();value["command"]["data"]["confirmation"]["events"] = json!([]);
    assert!(parse(&value).is_err());
    let mut value = command();let event=value["command"]["data"]["confirmation"]["events"][0].clone();
    value["command"]["data"]["confirmation"]["events"] = Value::Array(vec![event;257]);
    assert!(parse(&value).is_err());
    let mut value = command();value["command"]["data"]["confirmation"].as_object_mut().unwrap().remove("source");
    assert!(parse(&value).is_err());
}
