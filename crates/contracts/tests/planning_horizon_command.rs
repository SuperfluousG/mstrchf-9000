use mstrchf_contracts::*;
use serde_json::json;

fn command() -> serde_json::Value {
    json!({"schema_version":1,"command_id":"cm_41000000000000000000000000000001","expected_snapshot_revision":"1","command":{"kind":"suggest_planning_horizon","data":{"ordered_week_ids":["wk_40000000000000000000000000000000","wk_20000000000000000000000000000000","wk_30000000000000000000000000000000","wk_10000000000000000000000000000000"],"mode":"fill_open","allow_repeats":false}}})
}

#[test]
fn ordered_horizon_ids_are_not_mistaken_for_a_lexicographic_set() {
    let raw=command();let parsed:AppCommandV1=parse_document(&serde_json::to_vec(&raw).unwrap()).unwrap();
    assert_eq!(serde_json::to_value(&parsed).unwrap(),raw);
    assert_eq!(parsed.command.kind(),CommandKind::SuggestPlanningHorizon);
    assert!(authorize_command(parsed.command.kind(),CallerClass::LocalUi).is_ok());
    for caller in [CallerClass::ExchangeImport,CallerClass::QualifiedAdapter] {assert_eq!(authorize_command(parsed.command.kind(),caller).unwrap_err().code,ErrorCode::AuthorityDenied);}
}

#[test]
fn horizon_wire_is_exactly_four_distinct_ids_with_no_old_set_field_alias() {
    let source=command();
    for length in [0,1,3,5] {let mut raw=source.clone();raw["command"]["data"]["ordered_week_ids"]=json!((0..length).map(|n|format!("wk_{n:032x}")).collect::<Vec<_>>());assert!(parse_document::<AppCommandV1>(&serde_json::to_vec(&raw).unwrap()).is_err());}
    let mut raw=source.clone();raw["command"]["data"]["ordered_week_ids"][1]=raw["command"]["data"]["ordered_week_ids"][0].clone();assert!(parse_document::<AppCommandV1>(&serde_json::to_vec(&raw).unwrap()).is_err());
    let mut raw=source;let ids=raw["command"]["data"].as_object_mut().unwrap().remove("ordered_week_ids").unwrap();raw["command"]["data"]["week_ids"]=ids;assert!(parse_document::<AppCommandV1>(&serde_json::to_vec(&raw).unwrap()).is_err());
}
