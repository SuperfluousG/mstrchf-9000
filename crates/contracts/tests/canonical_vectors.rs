use mstrchf_contracts::quantity::{
    DecimalU64, NonnegativeQuantity, PositiveRational, Quantity, Rational,
};
use mstrchf_contracts::revision::{Generation, Revision};
use mstrchf_contracts::time::{Instant, LocalDate, LocalTime, ZoneId, ZonedMoment};
use mstrchf_contracts::{
    DemandMaterial, DerivedMaterial, DerivedPlan, PlanInputMaterial, canonical_bytes,
    canonical_digest, demand_digest, parse_document, plan_input_digest,
};
use serde_json::{Value, json};
use ts_rs::{Config, TS};

#[test]
fn positive_derived_demand_known_answers() {
    let plan = r#"{"context_stamp":{"catalogue_revision":"1","history_revision":"1","household_revision":"1","privacy_revision":"1","ranking_policy_version":"1","read_weeks":[{"revision":"1","week_id":"wk_00000000000000000000000000000001"}],"recipe_refs":[],"settings_revision":"1","write_weeks":[{"revision":"1","week_id":"wk_00000000000000000000000000000001"}]},"schema_version":1,"week_plan":{"batch_links":[],"revision":"1","selections":[],"week_id":"wk_00000000000000000000000000000001"}}"#;
    let p: PlanInputMaterial = parse_document(plan.as_bytes()).unwrap();
    assert_eq!(canonical_bytes(&p).unwrap(), plan.as_bytes());
    assert_eq!(
        plan_input_digest(&p).unwrap().0,
        "de39f29ff0302a9f932eabdec43b2587ee14543f89a68ff934f66c9addca271b"
    );
    let demand = r#"{"input_plan_digest":"de39f29ff0302a9f932eabdec43b2587ee14543f89a68ff934f66c9addca271b","lines":[],"schema_version":1}"#;
    let d: DemandMaterial = parse_document(demand.as_bytes()).unwrap();
    assert_eq!(canonical_bytes(&d).unwrap(), demand.as_bytes());
    assert_eq!(
        demand_digest(&d).unwrap().0,
        "67b17eea783378b277278da5ecc5b63d1c6cc907cf2cfc64ac504b926cd808ed"
    );
    let material = r#"{"allocations":[],"batches":[],"context_stamp":{"catalogue_revision":"1","history_revision":"1","household_revision":"1","privacy_revision":"1","ranking_policy_version":"1","read_weeks":[{"revision":"1","week_id":"wk_00000000000000000000000000000001"}],"recipe_refs":[],"settings_revision":"1","write_weeks":[{"revision":"1","week_id":"wk_00000000000000000000000000000001"}]},"demand":{"digest":"67b17eea783378b277278da5ecc5b63d1c6cc907cf2cfc64ac504b926cd808ed","input_plan_digest":"de39f29ff0302a9f932eabdec43b2587ee14543f89a68ff934f66c9addca271b","lines":[]},"derivation_policy_version":"1","input_plan_digest":"de39f29ff0302a9f932eabdec43b2587ee14543f89a68ff934f66c9addca271b","plan_revision":"1","prep_sessions":[],"recipe_refs":[],"schema_version":1,"warnings":[],"week_id":"wk_00000000000000000000000000000001"}"#;
    // Historical pre-ruling004 bytes stay pinned; the current decoder must reject them.
    assert!(parse_document::<DerivedMaterial>(material.as_bytes()).is_err());
    let m: Value = serde_json::from_str(material).unwrap();
    assert_eq!(canonical_bytes(&m).unwrap(), material.as_bytes());
    assert_eq!(
        canonical_digest(&m).unwrap().0,
        "27026cdc9b650240ae5ed268cb63a3ee17591f965e77f082e904aeb4f65deb1f"
    );
    let mut complete: Value = serde_json::from_str(material).unwrap();
    complete["digest"] = json!("27026cdc9b650240ae5ed268cb63a3ee17591f965e77f082e904aeb4f65deb1f");
    assert!(parse_document::<DerivedPlan>(&serde_json::to_vec(&complete).unwrap()).is_err());
}
#[test]
fn positive_jcs_unicode_vector() {
    let input:Value=serde_json::from_str(r#"{"€":"Euro Sign","\r":"Carriage Return","דּ":"Hebrew Letter Dalet With Dagesh","1":"One","😀":"Emoji: Grinning Face","\u0080":"Control","ö":"Latin Small Letter O With Diaeresis"}"#).unwrap();
    let expected = "{\"\\r\":\"Carriage Return\",\"1\":\"One\",\"\u{0080}\":\"Control\",\"ö\":\"Latin Small Letter O With Diaeresis\",\"€\":\"Euro Sign\",\"😀\":\"Emoji: Grinning Face\",\"דּ\":\"Hebrew Letter Dalet With Dagesh\"}";
    assert_eq!(canonical_bytes(&input).unwrap(), expected.as_bytes());
    let reordered = json!({"b":2,"a":1});
    assert_eq!(canonical_bytes(&reordered).unwrap(), br#"{"a":1,"b":2}"#);
    assert_eq!(
        canonical_digest(&reordered).unwrap().0,
        "43258cff783fe7036d8a43033f830adfc60ec037382473548ac742b888292777"
    );
}

#[test]
fn decimal_u64_is_a_canonical_string_even_at_its_limit() {
    let max: DecimalU64 = serde_json::from_str("\"18446744073709551615\"").unwrap();
    assert_eq!(max.0, u64::MAX);
    assert_eq!(
        serde_json::to_string(&max).unwrap(),
        "\"18446744073709551615\""
    );
    assert_eq!(serde_json::from_str::<DecimalU64>("\"0\"").unwrap().0, 0);
    for invalid in ["01", "+1", "1.0", "1e2", "18446744073709551616", "-1", " 1"] {
        assert!(
            serde_json::from_value::<DecimalU64>(json!(invalid)).is_err(),
            "{invalid}"
        );
    }
    assert!(serde_json::from_str::<DecimalU64>("1").is_err());
}

#[test]
fn quantity_records_use_exact_amount_unit_and_form_fields() {
    let positive: Quantity = serde_json::from_value(
        json!({"amount":{"numerator":"2","denominator":"1"},"unit":"g","form":"raw"}),
    )
    .unwrap();
    assert_eq!(positive.amount.0.numerator.0, 2);
    assert!(
        serde_json::from_value::<Quantity>(
            json!({"amount":{"numerator":"0","denominator":"1"},"unit":"g","form":"raw"})
        )
        .is_err()
    );
    let zero: NonnegativeQuantity = serde_json::from_value(
        json!({"amount":{"numerator":"0","denominator":"1"},"unit":"each","form":"whole"}),
    )
    .unwrap();
    assert_eq!(zero.amount.numerator.0, 0);
    assert!(
        serde_json::from_value::<Quantity>(
            json!({"amount":{"numerator":"1","denominator":"1"},"unit":"kg","form":"raw"})
        )
        .is_err()
    );
    assert!(serde_json::from_value::<Quantity>(
        json!({"amount":{"numerator":"1","denominator":"1"},"unit":"g","form":"raw","extra":true})
    )
    .is_err());
}

#[test]
fn numeric_and_time_schema_types_match_the_string_wire() {
    let config = Config::default();
    assert_eq!(DecimalU64::inline(&config), "string");
    assert_eq!(Revision::inline(&config), "string");
    assert_eq!(Generation::inline(&config), "string");
    assert_eq!(PositiveRational::inline(&config), "Rational");
    assert_eq!(LocalDate::inline(&config), "string");
    assert_eq!(Instant::inline(&config), "string");
    let decimal_schema = serde_json::to_value(schemars::schema_for!(DecimalU64)).unwrap();
    assert_eq!(decimal_schema["type"], "string");
    assert!(decimal_schema["pattern"].as_str().unwrap().starts_with('^'));
    let decimal_validator = jsonschema::validator_for(&decimal_schema).unwrap();
    assert!(decimal_validator.is_valid(&json!("18446744073709551615")));
    assert!(!decimal_validator.is_valid(&json!("18446744073709551616")));
    assert!(!decimal_validator.is_valid(&json!("01")));
    let revision_schema = serde_json::to_value(schemars::schema_for!(Revision)).unwrap();
    let revision_validator = jsonschema::validator_for(&revision_schema).unwrap();
    assert!(!revision_validator.is_valid(&json!("0")));
    assert!(!revision_validator.is_valid(&json!("18446744073709551616")));
    let rational_schema = serde_json::to_value(schemars::schema_for!(Rational)).unwrap();
    assert_eq!(rational_schema["additionalProperties"], false);
    assert_eq!(rational_schema["properties"]["numerator"]["type"], "string");
}

#[test]
fn positive_revisions_are_checked_at_exhaustion() {
    for invalid in ["0", "01", "+1", "18446744073709551616"] {
        assert!(serde_json::from_value::<Revision>(json!(invalid)).is_err());
        assert!(serde_json::from_value::<Generation>(json!(invalid)).is_err());
    }
    let one: Revision = serde_json::from_str("\"1\"").unwrap();
    assert_eq!(one.checked_increment().unwrap().0, 2);
    let max = Revision(u64::MAX);
    assert!(max.checked_increment().is_err());
    assert!(Generation(u64::MAX).checked_increment().is_err());
    assert!(Revision(0).checked_increment().is_err());
}

#[test]
fn rationals_reject_noncanonical_and_out_of_range_values() {
    let two_thirds = Rational::new(2, 3).unwrap();
    assert_eq!(
        serde_json::to_value(two_thirds).unwrap(),
        json!({"numerator":"2","denominator":"3"})
    );
    assert_eq!(Rational::new(0, 1).unwrap().numerator.0, 0);
    for (n, d) in [
        (2, 4),
        (0, 2),
        (1, 0),
        (1_000_000_000_001, 1),
        (1, 1_000_001),
    ] {
        assert!(Rational::new(n, d).is_err(), "{n}/{d}");
    }
    assert!(
        serde_json::from_value::<Rational>(json!({"numerator":"2","denominator":"4"})).is_err()
    );
    assert!(
        serde_json::from_value::<PositiveRational>(json!({"numerator":"0","denominator":"1"}))
            .is_err()
    );
    assert!(serde_json::from_value::<Rational>(json!({"numerator":2,"denominator":"3"})).is_err());
    assert!(
        serde_json::from_value::<Rational>(json!({"numerator":"1","denominator":"1","extra":1}))
            .is_err()
    );
}

#[test]
fn checked_rational_arithmetic_reduces_and_rechecks_bounds() {
    assert_eq!(
        Rational::new(1, 2)
            .unwrap()
            .checked_add(&Rational::new(1, 3).unwrap())
            .unwrap(),
        Rational::new(5, 6).unwrap()
    );
    assert_eq!(
        Rational::new(2, 3)
            .unwrap()
            .checked_mul(&Rational::new(3, 4).unwrap())
            .unwrap(),
        Rational::new(1, 2).unwrap()
    );
    assert!(
        Rational::new(1_000_000_000_000, 1)
            .unwrap()
            .checked_add(&Rational::new(1, 1).unwrap())
            .is_err()
    );
    assert!(
        Rational::new(1, 999_999)
            .unwrap()
            .checked_add(&Rational::new(1, 1_000_000).unwrap())
            .is_err()
    );
    let forged_invalid = Rational {
        numerator: DecimalU64(0),
        denominator: DecimalU64(0),
    };
    assert!(forged_invalid.checked_add(&forged_invalid).is_err());
}

#[test]
fn wall_clock_values_use_exact_formats_and_gregorian_dates() {
    let leap: LocalDate = serde_json::from_str("\"2024-02-29\"").unwrap();
    assert_eq!(leap.0, "2024-02-29");
    for invalid in [
        "2023-02-29",
        "1899-12-31",
        "2200-01-01",
        "2024-2-29",
        "2024-02-29Z",
    ] {
        assert!(
            serde_json::from_value::<LocalDate>(json!(invalid)).is_err(),
            "{invalid}"
        );
    }
    let time: LocalTime = serde_json::from_str("\"23:59:59\"").unwrap();
    assert_eq!(time.0, "23:59:59");
    for invalid in [
        "24:00:00",
        "12:60:00",
        "12:00:60",
        "1:02:03",
        "01:02:03+01:00",
    ] {
        assert!(
            serde_json::from_value::<LocalTime>(json!(invalid)).is_err(),
            "{invalid}"
        );
    }
    let instant: Instant = serde_json::from_str("\"2026-10-25T01:30:00Z\"").unwrap();
    assert_eq!(instant.timestamp_seconds(), 1792891800);
    for invalid in [
        "2026-10-25T01:30:00+00:00",
        "2026-10-25t01:30:00z",
        "2026-10-25T01:30:00.000Z",
    ] {
        assert!(
            serde_json::from_value::<Instant>(json!(invalid)).is_err(),
            "{invalid}"
        );
    }
    assert!(serde_json::from_value::<ZoneId>(json!("europe/london")).is_err());
}

#[test]
fn zoned_moment_enforces_dst_gap_fold_and_round_trip() {
    let make = |date: &str, time: &str, resolution: &str, instant: &str| -> Value {
        json!({"local_date":date,"local_time":time,"zone":"Europe/London","resolution":resolution,"instant":instant})
    };
    assert!(
        serde_json::from_value::<ZonedMoment>(make(
            "2026-03-29",
            "01:30:00",
            "unambiguous",
            "2026-03-29T01:30:00Z"
        ))
        .is_err()
    );
    assert!(
        serde_json::from_value::<ZonedMoment>(make(
            "2026-10-25",
            "01:30:00",
            "unambiguous",
            "2026-10-25T00:30:00Z"
        ))
        .is_err()
    );
    let earlier: ZonedMoment = serde_json::from_value(make(
        "2026-10-25",
        "01:30:00",
        "earlier",
        "2026-10-25T00:30:00Z",
    ))
    .unwrap();
    let later: ZonedMoment = serde_json::from_value(make(
        "2026-10-25",
        "01:30:00",
        "later",
        "2026-10-25T01:30:00Z",
    ))
    .unwrap();
    assert!(earlier.instant.timestamp_seconds() < later.instant.timestamp_seconds());
    assert!(
        serde_json::from_value::<ZonedMoment>(make(
            "2026-10-25",
            "01:30:00",
            "earlier",
            "2026-10-25T01:30:00Z"
        ))
        .is_err()
    );
    assert!(
        serde_json::from_value::<ZonedMoment>(make(
            "2026-06-25",
            "12:30:00",
            "later",
            "2026-06-25T11:30:00Z"
        ))
        .is_err()
    );
    assert!(
        serde_json::from_value::<ZonedMoment>(make(
            "2026-06-25",
            "12:30:00",
            "unambiguous",
            "2026-06-25T11:30:00Z"
        ))
        .is_ok()
    );
}
