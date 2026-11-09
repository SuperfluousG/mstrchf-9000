use mstrchf_contracts::*;
use serde_json::{Value, json};

fn old_selection() -> Value {
    serde_json::from_slice::<Value>(include_bytes!(
        "../../../tests/fixtures/contracts/ruling004-current--week-disabled-retained.json"
    )).unwrap()["plan"]["selections"][0].clone()
}
fn moment(hour: u8) -> ZonedMoment {
    ZonedMoment::new(LocalDate("2026-10-20".into()), LocalTime(format!("{hour:02}:30:00")),
        ZoneId("Europe/London".into()), Resolution::Unambiguous,
        Instant(format!("2026-10-20T{:02}:30:00Z", hour - 1))).unwrap()
}
fn portable() -> Value {
    let mut selection = old_selection();
    selection["slot"]["meal_type"] = json!("lunch");
    selection["slot"]["date"] = json!("2026-10-20");
    selection["origin"] = json!("fixed");
    selection["extras"] = json!([]);
    selection["batch_use"] = json!({"kind":"leftover","data":{
        "batch_id":"ba_00000000000000000000000000000001",
        "component_ids":["co_00000000000000000000000000000001"]
    }});
    selection["portable_cold"] = json!(PortableColdJourney {
        packing_complete_at: moment(7), depart_at: moment(8),
        declaration: PortableChillingDeclaration { schema_version: SchemaVersion,
            confirmed_at: Instant("2026-10-19T12:00:00Z".into()),
            journey_digest: PayloadDigest("a".repeat(64)) }
    });
    selection
}
fn parse(value: &Value) -> Result<MealSelection, DomainError> {
    parse_document(&serde_json::to_vec(value).unwrap())
}

#[test]
fn absent_journey_preserves_exact_historical_selection_projection() {
    let old = old_selection();
    let decoded = parse(&old).unwrap();
    assert!(decoded.portable_cold.is_none());
    assert_eq!(serde_json::to_value(&decoded).unwrap(), old);
    assert_eq!(canonical_bytes(&decoded).unwrap(), canonical_bytes(&old).unwrap());
    assert!(jsonschema::validator_for(&schema::schema_for::<MealSelection>()).unwrap().is_valid(&old));
}

#[test]
fn explicit_journey_roundtrips_and_null_or_partial_journeys_are_refused() {
    let value = portable();
    assert_eq!(serde_json::to_value(parse(&value).unwrap()).unwrap(), value);
    let validator = jsonschema::validator_for(&schema::schema_for::<MealSelection>()).unwrap();
    assert!(validator.is_valid(&value));
    for mutation in 0..6 {
        let mut value = value.clone();
        match mutation {
            0 => value["portable_cold"] = Value::Null,
            1 => {value["portable_cold"].as_object_mut().unwrap().remove("declaration");},
            2 => value["portable_cold"]["declaration"]["schema_version"] = json!(2),
            3 => value["portable_cold"]["declaration"]["journey_digest"] = json!("unknown"),
            4 => {value["portable_cold"].as_object_mut().unwrap().insert("arrival_guess".into(), json!(1));},
            _ => {value["portable_cold"]["declaration"].as_object_mut().unwrap().remove("confirmed_at");},
        }
        assert!(parse(&value).is_err(), "mutation {mutation}");
        assert!(!validator.is_valid(&value), "schema mutation {mutation}");
    }
}

#[test]
fn journey_shape_requires_fixed_lunch_original_component_and_ordered_instants() {
    for mutation in 0..7 {
        let mut value = portable();
        match mutation {
            0 => value["origin"] = json!("suggested"),
            1 => value["slot"]["meal_type"] = json!("dinner"),
            2 => value["batch_use"] = old_selection()["batch_use"].clone(),
            3 => value["batch_use"]["data"]["component_ids"] = json!([]),
            4 => value["portable_cold"]["packing_complete_at"] = json!(moment(9)),
            5 => value["portable_cold"]["declaration"]["confirmed_at"] = json!("2026-10-20T07:31:00Z"),
            _ => value["slot"]["date"] = json!("2026-10-21"),
        }
        assert!(parse(&value).is_err(), "mutation {mutation}");
    }
}
