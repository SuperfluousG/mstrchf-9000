use mstrchf_contracts::*;
use serde_json::json;

fn intent() -> serde_json::Value {
    json!({"schema_version":1,"command_id":format!("cm_{:032x}",91),"expected_snapshot_revision":"1","command":{"kind":"admit_reviewed_recipe","data":{"packet":{"recipe":{"recipe_id":format!("rc_{:032x}",91),"revision":"2","digest":"a".repeat(64)},"packet_digest":"b".repeat(64)},"expected_catalogue_revision":"1"}}})
}

#[test]
fn reviewed_packet_intent_has_exact_identity_and_local_ui_authority_only() {
    let wire=intent();
    let command:AppCommandV1=parse_document(&serde_json::to_vec(&wire).unwrap()).unwrap();
    assert_eq!(command.command.kind(),CommandKind::AdmitReviewedRecipe);
    assert_eq!(serde_json::to_value(&command).unwrap(),wire);
    authorize_command(command.command.kind(),CallerClass::LocalUi).unwrap();
    for caller in [CallerClass::ExchangeImport,CallerClass::QualifiedAdapter] {
        assert_eq!(authorize_command(command.command.kind(),caller).unwrap_err().code,ErrorCode::AuthorityDenied);
    }
}

#[test]
fn admission_intent_cannot_transport_actor_evidence_or_authority_overrides() {
    for key in ["actor","evidence_basis","reviews","trusted","include_household_requirements"] {
        let mut wire=intent();
        wire["command"]["data"]["packet"][key]=json!(true);
        assert!(parse_document::<AppCommandV1>(&serde_json::to_vec(&wire).unwrap()).is_err(),"{key}");
    }
    let mut wire=intent();
    wire["command"]["data"]["packet"]["recipe"]["digest"]=json!("not-a-digest");
    assert!(parse_document::<AppCommandV1>(&serde_json::to_vec(&wire).unwrap()).is_err());
    let mut wire=intent();
    wire["command"]["data"]["expected_catalogue_revision"]=json!(0);
    assert!(parse_document::<AppCommandV1>(&serde_json::to_vec(&wire).unwrap()).is_err());
}
