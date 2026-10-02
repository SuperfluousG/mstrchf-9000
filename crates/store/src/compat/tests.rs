use super::*;
use serde_json::{Value, json};
const HH: &str = "hh_00000000000000000000000000000001";
#[test]
fn validation_errors_do_not_retain_unknown_stored_key_text() {
    let mut v = full_snapshot(false);
    v["private-household-secret"] = json!(true);
    let error = b1_v1::snapshot(&canonical(&v)).unwrap_err();
    assert!(!format!("{error:?}").contains("private-household-secret"));
}
#[test]
fn unsupported_content_reports_unknown_closure_before_decoding_it() {
    for new in [false, true] {
        for field in ["weeks", "conflicts"] {
            let mut v = full_snapshot(new);
            v[field] = json!([{}]);
            let b = canonical(&v);
            let e = if new {
                draft005::snapshot(&b).unwrap_err()
            } else {
                b1_v1::snapshot(&b).unwrap_err()
            };
            assert_eq!(e.code, Code::UnknownClosure);
        }
    }
}

#[test]
fn collection_maxima_refuse_before_decoding_excess_elements() {
    for (root, path, max) in [
        ("Snapshot", "/jobs", 1024),
        ("HouseholdDraft", "/diners", 64),
        ("HouseholdDraft", "/completed_stages", 4),
        ("HouseholdProfile", "/diners", 64),
        ("HouseholdProfile", "/diners/0/dislikes", 256),
        ("HouseholdProfile", "/diners/0/portions/meal_overrides", 3),
        (
            "HouseholdProfile",
            "/diners/0/restrictions/data/requirements",
            64,
        ),
        ("HouseholdProfile", "/preferences/equipment", 9),
        ("HouseholdProfile", "/preferences/meal_times", 3),
        ("HouseholdProfile", "/usual_week/cells", 21),
        ("HouseholdProfile", "/usual_week/day_enabled", 7),
        ("HouseholdProfile", "/usual_week/enabled_meal_types", 3),
        ("HouseholdProfile", "/usual_week/cells/0/attendee_ids", 64),
        ("HouseholdProfile", "/connections/delivery_preferences", 7),
        ("HouseholdProfile", "/connections/reminders/channels", 2),
        ("HouseholdProfile", "/connections/reminders/prompts", 2),
        ("HouseholdDraft", "/unresolved_restrictions", 64),
        ("AppCommandV1", "/command/data/restriction_issue_edits", 64),
    ] {
        let full = full_snapshot(true);
        let mut v = match root {
            "Snapshot" => full.clone(),
            "HouseholdDraft" => full["household_draft"].clone(),
            "HouseholdProfile" => full["profile"].clone(),
            _ => value(fixture("fixtures/new-issue-command.json")),
        };
        *v.pointer_mut(path).unwrap() = json!(vec![Value::Null; max + 1]);
        let bytes = canonical(&v);
        let e =
            codec::decode::<Value>(&bytes, include_str!("draft005.schema.json"), root).unwrap_err();
        assert_eq!(e.code, Code::LimitExceeded, "{root} {path}");
        // Malformed body after the schema's bound must still be refused at the
        // limit, without descending into or allocating that excess element.
        let s = String::from_utf8(bytes).unwrap();
        let pattern = format!("{}null", "null,".repeat(max));
        let malformed = s.replacen(&pattern, &format!("{}{{broken", "null,".repeat(max)), 1);
        assert_ne!(s, malformed);
        assert_eq!(
            codec::decode::<Value>(
                malformed.as_bytes(),
                include_str!("draft005.schema.json"),
                root
            )
            .unwrap_err()
            .code,
            Code::LimitExceeded,
            "early {path}"
        );
    }
}
// A synthetic full draft assembled from the independently supplied profile.
// Originals are never rewritten; this is solely a test input constructor.
fn full_snapshot(new: bool) -> Value {
    let mut v = value(fixture("fixtures/old-profile-readable-snapshot.json"));
    v["household_draft"] = v["profile"].clone();
    v["household_draft"]["completed_stages"] =
        json!(["connections", "food_preferences", "household", "usual_week"]);
    if new {
        v["household_draft"]["unresolved_restrictions"] = json!([]);
    }
    v
}
fn accepts(v: &Value, new: bool) -> bool {
    let b = canonical(v);
    if new {
        draft005::snapshot(&b).is_ok()
    } else {
        b1_v1::snapshot(&b).is_ok()
    }
}

#[test]
fn typed_canonical_equivalence_to_accepted_005() {
    for name in [
        "new-transition-snapshot",
        "new-issue-response",
        "new-resolve-response",
    ] {
        let b = fixture(&format!("fixtures/{name}.json"));
        let frozen = draft005::snapshot(b).unwrap();
        let current = mstrchf_contracts::parse_document::<mstrchf_contracts::Snapshot>(b).unwrap();
        assert_eq!(codec::canonical(&frozen).unwrap(), b);
        assert_eq!(mstrchf_contracts::canonical_bytes(&current).unwrap(), b);
    }
    for name in [
        "new-save-same-id-command",
        "new-issue-command",
        "new-resolve-command",
        "new-identical-privacy-command",
        "new-identical-cancel-command",
    ] {
        let b = fixture(&format!("fixtures/{name}.json"));
        let frozen = draft005::command(b).unwrap();
        let current =
            mstrchf_contracts::parse_document::<mstrchf_contracts::AppCommandV1>(b).unwrap();
        assert_eq!(codec::canonical(&frozen).unwrap(), b);
        assert_eq!(mstrchf_contracts::canonical_bytes(&current).unwrap(), b);
    }
    let mut full = full_snapshot(true);
    full["household_draft"]["diners"][0]["display_name"] = json!("Zoë 🦊 / \"quote\" \\ slash");
    let b = canonical(&full);
    let frozen = draft005::snapshot(&b).unwrap();
    assert_eq!(codec::canonical(&frozen).unwrap(), b);
    assert!(mstrchf_contracts::parse_document::<mstrchf_contracts::Snapshot>(&b).is_ok());
}

// Catch omitted recursive required/unknown-field checks, including nullable
// fields and tagged payload data. Every object in a fully populated graph.
#[test]
fn all_populated_object_fields_are_required_and_closed() {
    fn paths(v: &Value, p: &str, out: &mut Vec<String>) {
        match v {
            Value::Object(m) => {
                out.push(p.into());
                for (k, x) in m {
                    paths(x, &format!("{p}/{k}"), out);
                }
            }
            Value::Array(a) => {
                for (i, x) in a.iter().enumerate() {
                    paths(x, &format!("{p}/{i}"), out);
                }
            }
            _ => {}
        }
    }
    for new in [false, true] {
        let base = full_snapshot(new);
        assert!(accepts(&base, new));
        let mut objects = Vec::new();
        paths(&base, "", &mut objects);
        for p in objects {
            let keys: Vec<_> = base
                .pointer(&p)
                .unwrap()
                .as_object()
                .unwrap()
                .keys()
                .cloned()
                .collect();
            for k in keys {
                let mut v = base.clone();
                v.pointer_mut(&p)
                    .unwrap()
                    .as_object_mut()
                    .unwrap()
                    .remove(&k);
                assert!(!accepts(&v, new), "missing {p}/{k}, new={new}");
            }
            let mut v = base.clone();
            v.pointer_mut(&p).unwrap()["unknown"] = json!(null);
            assert!(!accepts(&v, new), "unknown {p}, new={new}");
        }
    }
}

#[test]
fn complete_household_semantics_and_reference_refusals() {
    for new in [false, true] {
        let base = full_snapshot(new);
        let cases = [
            (
                "/household_draft/completed_stages",
                json!(["household", "connections", "food_preferences", "usual_week"]),
            ),
            (
                "/household_draft/completed_stages",
                json!(["connections", "food_preferences", "usual_week"]),
            ),
            ("/household_draft/connections/privacy/revision", json!("2")),
            ("/profile/connections/privacy/use_ai_features", json!(true)),
            (
                "/profile/household_id",
                json!("hh_00000000000000000000000000000002"),
            ),
            (
                "/household_draft/usual_week/cells/0/attendee_ids",
                json!(["dn_00000000000000000000000000000003"]),
            ),
            (
                "/profile/usual_week/cells/0/attendee_ids",
                json!(["dn_00000000000000000000000000000003"]),
            ),
            (
                "/profile/diners/0/restrictions",
                json!({"kind":"unconfirmed","data":{}}),
            ),
            (
                "/household_draft/preferences/preferred_hands_on_minutes",
                json!(1441),
            ),
            ("/profile/connections/reminders/enabled", json!(true)),
            (
                "/profile/connections/delivery_preferences",
                json!([{"weekday":"monday","earliest":"12:00:00","latest":"12:00:00"}]),
            ),
            (
                "/profile/diners/1/diner_id",
                base["profile"]["diners"][0]["diner_id"].clone(),
            ),
            (
                "/profile/usual_week/cells/1",
                base["profile"]["usual_week"]["cells"][0].clone(),
            ),
            (
                "/profile/preferences/meal_times/1/meal_type",
                json!("breakfast"),
            ),
            ("/profile/preferences/equipment", json!(["oven", "hob"])),
            (
                "/profile/diners/0/dislikes",
                json!([
                    "ig_00000000000000000000000000000001",
                    "ig_00000000000000000000000000000001"
                ]),
            ),
            (
                "/profile/diners/0/restrictions/data/requirements",
                json!([{"kind":"diet","data":{"code":"vegan"}},{"kind":"diet","data":{"code":"vegan"}}]),
            ),
        ];
        for (p, bad) in cases {
            let mut v = base.clone();
            *v.pointer_mut(p).unwrap() = bad;
            assert!(!accepts(&v, new), "{p}, new={new}");
        }
        // These sequences are not sets with required sorting.
        let mut v = base.clone();
        for p in [
            "/profile/diners",
            "/profile/usual_week/cells",
            "/profile/preferences/meal_times",
        ] {
            v.pointer_mut(p).unwrap().as_array_mut().unwrap().reverse();
        }
        v["profile"]["preferences"]["preferred_hands_on_minutes"] = json!(1440);
        v["profile"]["connections"]["delivery_preferences"] =
            json!([{"weekday":"monday","earliest":"00:00:00","latest":"23:59:59"}]);
        assert!(accepts(&v, new));
        // No invented privacy-confirmation dependency or freshness constraint.
        let mut v = base.clone();
        for p in [
            "/privacy",
            "/household_draft/connections/privacy",
            "/profile/connections/privacy",
        ] {
            v.pointer_mut(p).unwrap()["use_ai_features"] = json!(true);
        }
        assert!(accepts(&v, new));
    }
}

#[test]
fn primitive_language_and_current_equivalence() {
    let cases = [
        (
            "/profile/diners/0/display_name",
            json!("a".repeat(80)),
            true,
        ),
        (
            "/profile/diners/0/display_name",
            json!("a".repeat(81)),
            false,
        ),
        ("/profile/diners/0/display_name", json!(" leading"), false),
        ("/profile/diners/0/display_name", json!("line\nname"), false),
        (
            "/profile/diners/0/display_name",
            json!("é".repeat(80)),
            true,
        ),
        (
            "/profile/diners/0/restrictions/data/confirmed_at",
            json!("0000-02-29T00:00:00Z"),
            true,
        ),
        (
            "/profile/diners/0/restrictions/data/confirmed_at",
            json!("1899-12-31T23:59:59Z"),
            true,
        ),
        (
            "/profile/diners/0/restrictions/data/confirmed_at",
            json!("9999-12-31T23:59:59Z"),
            true,
        ),
        (
            "/profile/diners/0/restrictions/data/confirmed_at",
            json!("1900-02-29T00:00:00Z"),
            false,
        ),
        (
            "/profile/diners/0/restrictions/data/confirmed_at",
            json!("2000-02-29T00:00:00Z"),
            true,
        ),
        (
            "/profile/diners/0/restrictions/data/confirmed_at",
            json!("2026-01-01T00:00:60Z"),
            false,
        ),
        (
            "/profile/diners/0/restrictions/data/confirmed_at",
            json!("2026-01-01T00:00:00.0Z"),
            false,
        ),
        (
            "/profile/preferences/meal_times/0/time",
            json!("24:00:00"),
            false,
        ),
        (
            "/profile/preferences/meal_times/0/time",
            json!("23:59:59"),
            true,
        ),
        ("/profile/usual_week/zone", json!("Europe/London"), true),
        ("/profile/usual_week/zone", json!("europe/london"), false),
        ("/profile/usual_week/zone", json!("Not/AZone"), false),
        ("/snapshot_revision", json!("18446744073709551615"), true),
        ("/snapshot_revision", json!(18446744073709551615u64), false),
        (
            "/profile/diners/0/portions/adjustment",
            json!({"numerator":"1000000000000","denominator":"1"}),
            true,
        ),
        (
            "/profile/diners/0/portions/adjustment",
            json!({"numerator":"1","denominator":"1000000"}),
            true,
        ),
        (
            "/profile/diners/0/portions/adjustment",
            json!({"numerator":"1000000000001","denominator":"1"}),
            false,
        ),
        (
            "/profile/diners/0/portions/adjustment",
            json!({"numerator":"1","denominator":"1000001"}),
            false,
        ),
        (
            "/profile/diners/0/portions/adjustment",
            json!({"numerator":"2","denominator":"2"}),
            false,
        ),
        (
            "/profile/diners/0/portions/adjustment",
            json!({"numerator":"0","denominator":"1"}),
            false,
        ),
        (
            "/profile/diners/0/portions/adjustment",
            json!({"numerator":"01","denominator":"1"}),
            false,
        ),
        (
            "/profile/diners/0/portions/adjustment",
            json!({"numerator":1,"denominator":"1"}),
            false,
        ),
    ];
    for (p, x, want) in cases {
        for new in [false, true] {
            let mut v = full_snapshot(new);
            *v.pointer_mut(p).unwrap() = x.clone();
            assert_eq!(accepts(&v, new), want, "{p} {x}, new={new}");
        }
        let mut v = full_snapshot(true);
        *v.pointer_mut(p).unwrap() = x;
        assert_eq!(
            mstrchf_contracts::parse_document::<mstrchf_contracts::Snapshot>(&canonical(&v))
                .is_ok(),
            want,
            "oracle {p}"
        );
    }
    // The pinned recognised-name set is checked against the actual dependency.
    for zone in include_str!("zones.txt").lines() {
        assert!(
            mstrchf_contracts::parse_document::<mstrchf_contracts::ZoneId>(&canonical(&json!(
                zone
            )))
            .is_ok(),
            "{zone}"
        );
    }
}

#[test]
fn draft005_issue_and_edit_rules_remain_strict() {
    let base = value(fixture("fixtures/new-issue-command.json"));
    assert!(draft005::command(&canonical(&base)).is_ok());
    for bad in [
        json!(null),
        json!([]),
        json!([{"diner_id":"dn_00000000000000000000000000000002","supplied_text":"unknown"}]),
    ] {
        let mut v = base.clone();
        v["command"]["data"]["draft"]["unresolved_restrictions"] = bad;
        // Empty is valid structure with this unconfirmed diner; removal itself
        // needs pre-state and is deliberately not inferred by a receipt reader.
        let want = v["command"]["data"]["draft"]["unresolved_restrictions"] == json!([]);
        assert_eq!(draft005::command(&canonical(&v)).is_ok(), want);
    }
    for text in ["", " trim", "bad\u{0000}control"] {
        let mut v = base.clone();
        v["command"]["data"]["draft"]["unresolved_restrictions"][0]["supplied_text"] = json!(text);
        assert!(draft005::command(&canonical(&v)).is_err());
    }
    for (text, want) in [
        ("a".repeat(4000), true),
        ("a".repeat(4001), false),
        ("internal\nline\ttab".into(), true),
    ] {
        let mut v = base.clone();
        v["command"]["data"]["draft"]["unresolved_restrictions"][0]["supplied_text"] = json!(text);
        assert_eq!(draft005::command(&canonical(&v)).is_ok(), want);
        assert_eq!(
            mstrchf_contracts::parse_document::<mstrchf_contracts::AppCommandV1>(&canonical(&v))
                .is_ok(),
            want
        );
    }
    let mut v = base.clone();
    v["command"]["data"]["restriction_issue_edits"] = json!([
        {"kind":"resolve","data":{"diner_id":"dn_00000000000000000000000000000001"}},
        {"kind":"remove_diner","data":{"diner_id":"dn_00000000000000000000000000000001"}}]);
    assert!(draft005::command(&canonical(&v)).is_err());
    v["command"]["data"]["restriction_issue_edits"][0]["data"]["diner_id"] =
        json!("dn_00000000000000000000000000000002");
    assert!(draft005::command(&canonical(&v)).is_err());
    v["command"]["data"]["restriction_issue_edits"]
        .as_array_mut()
        .unwrap()
        .reverse();
    assert!(draft005::command(&canonical(&v)).is_ok());
    let mut v = base.clone();
    v["command"]["data"]["draft"]["diners"][0]["restrictions"] =
        json!({"kind":"confirmed_none","data":{"confirmed_at":"2026-12-21T12:00:00Z"}});
    assert!(draft005::command(&canonical(&v)).is_err());
}

#[test]
fn receipt_projection_digest_and_checked_counter_refusals() {
    let cb = fixture("fixtures/old-save-command.json");
    let rb = fixture("fixtures/old-save-response.json");
    let c = value(cb);
    let r = value(rb);
    let cd = digest(cb);
    let rd = digest(rb);
    for field in [
        "id",
        "household",
        "column",
        "live",
        "command_digest",
        "response_digest",
    ] {
        let mut b = binding(&c, &r, &cd, &rd);
        match field {
            "id" => b.command_id = "cm_00000000000000000000000000000002",
            "household" => b.household_id = "hh_00000000000000000000000000000002",
            "column" => b.response_revision = "3",
            "live" => b.live_revision = "1",
            "command_digest" => {
                b.command_digest =
                    "0000000000000000000000000000000000000000000000000000000000000000"
            }
            _ => {
                b.response_digest =
                    "0000000000000000000000000000000000000000000000000000000000000000"
            }
        }
        assert!(b1_v1::receipt(cb, rb, &b).is_err(), "{field}");
    }
    for (s, d, returned, want) in [
        ("1", "1", "1", true),
        ("1", "1", "2", true),
        ("1", "1", "3", false),
        ("1", "18446744073709551615", "18446744073709551615", true),
        ("18446744073709551615", "1", "1", false),
    ] {
        let mut c = c.clone();
        let mut r = r.clone();
        c["expected_snapshot_revision"] = json!(s);
        c["command"]["data"]["draft"]["revision"] = json!(d);
        r["household_draft"]["revision"] = json!(returned);
        let cb = canonical(&c);
        let rb = canonical(&r);
        let cd = digest(&cb);
        let rd = digest(&rb);
        assert_eq!(
            b1_v1::receipt(&cb, &rb, &binding(&c, &r, &cd, &rd)).is_ok(),
            want,
            "{s}/{d}/{returned}"
        );
    }
    let mut altered = r.clone();
    altered["household_draft"]["diners"] =
        value(fixture("fixtures/old-unconfirmed-draft.json"))["diners"].clone();
    let altered_bytes = canonical(&altered);
    let rd = digest(&altered_bytes);
    assert!(b1_v1::receipt(cb, &altered_bytes, &binding(&c, &altered, &cd, &rd)).is_err());
}

#[test]
fn terminal_metadata_matrix_preserves_all_84_combinations_per_profile() {
    for new in [false, true] {
        for command_name in ["old-cancel-command", "old-privacy-command"] {
            let cb = fixture(&format!("fixtures/{command_name}.json"));
            let c = value(cb);
            let cd = digest(cb);
            for (state, want) in [
                ("prepared", false),
                ("awaiting_user", false),
                ("running", false),
                ("needs_user", true),
                ("succeeded", true),
                ("failed", true),
                ("cancelled", true),
            ] {
                for reason in [Value::Null, json!("privacy_denied")] {
                    for action in [Value::Null, json!("stop_worker"), json!("reconcile")] {
                        let mut r = value(fixture(if command_name == "old-cancel-command" {
                            "fixtures/old-cancel-response.json"
                        } else {
                            "fixtures/old-privacy-response.json"
                        }));
                        if new {
                            r["household_draft"]["unresolved_restrictions"] = json!([]);
                        }
                        r["jobs"] = json!([{"job_id":"jb_00000000000000000000000000000001","action":"setup","state":state,"reason":reason,"next_action":action}]);
                        let rb = canonical(&r);
                        let rd = digest(&rb);
                        let bind = binding(&c, &r, &cd, &rd);
                        let ok = if new {
                            draft005::receipt(cb, &rb, &bind).is_ok()
                        } else {
                            b1_v1::receipt(cb, &rb, &bind).is_ok()
                        };
                        assert_eq!(ok, want, "{command_name} {state} new={new}");
                    }
                }
            }
        }
    }
}
fn value(bytes: &[u8]) -> Value {
    serde_json::from_slice(bytes).unwrap()
}
fn canonical(v: &Value) -> Vec<u8> {
    mstrchf_contracts::canonical_bytes(v).unwrap()
}
fn digest(b: &[u8]) -> String {
    mstrchf_contracts::bytes_digest(b).0
}
fn binding<'a>(c: &'a Value, r: &'a Value, cd: &'a str, rd: &'a str) -> ReceiptBinding<'a> {
    ReceiptBinding {
        command_id: c["command_id"].as_str().unwrap(),
        household_id: HH,
        response_revision: r["snapshot_revision"].as_str().unwrap(),
        live_revision: "100",
        command_digest: cd,
        response_digest: rd,
    }
}
// Removing original-byte validation or widening either frozen grammar breaks this table.
#[test]
fn original_26_literals_remain_exact_and_profile_specific() {
    let manifest = value(fixture("fixtures/known-answers.json"));
    let rows = manifest["vectors"].as_array().unwrap();
    assert_eq!(rows.len(), 26);
    for row in rows {
        let name = row["file"].as_str().unwrap();
        let b = fixture(name);
        assert_eq!(b.len() as u64, row["bytes"].as_u64().unwrap());
        assert_eq!(digest(b), row["sha256"].as_str().unwrap());
    }
    for name in [
        "old-initial-snapshot",
        "old-save-response",
        "old-privacy-response",
        "old-cancel-response",
        "old-profile-readable-snapshot",
    ] {
        let b = fixture(&format!("fixtures/{name}.json"));
        assert!(b1_v1::snapshot(b).is_ok(), "{name}");
        assert!(draft005::snapshot(b).is_err(), "{name}");
    }
    for name in [
        "new-transition-snapshot",
        "new-issue-response",
        "new-resolve-response",
    ] {
        let b = fixture(&format!("fixtures/{name}.json"));
        assert!(draft005::snapshot(b).is_ok(), "{name}");
        assert!(b1_v1::snapshot(b).is_err(), "{name}");
    }
    for name in [
        "old-draft",
        "old-unconfirmed-draft",
        "new-missing-issues-draft",
    ] {
        let b = fixture(&format!("fixtures/{name}.json"));
        assert!(b1_v1::draft(b).is_ok(), "{name}");
        assert!(draft005::draft(b).is_err(), "{name}");
    }
    for name in ["new-transition-draft", "new-unconfirmed-transition-draft"] {
        let b = fixture(&format!("fixtures/{name}.json"));
        assert!(draft005::draft(b).is_ok(), "{name}");
        assert!(b1_v1::draft(b).is_err(), "{name}");
    }
    for name in [
        "new-identical-privacy-command",
        "new-identical-cancel-command",
        "same-id-different-command",
    ] {
        let b = fixture(&format!("fixtures/{name}.json"));
        assert!(b1_v1::command(b).is_ok(), "{name}");
        assert!(draft005::command(b).is_ok(), "{name}");
    }
    let old = fixture("fixtures/old-save-command.json");
    assert!(b1_v1::command(old).is_ok());
    assert!(draft005::command(old).is_err());
    for name in [
        "new-save-same-id-command",
        "new-issue-command",
        "new-resolve-command",
    ] {
        let b = fixture(&format!("fixtures/{name}.json"));
        assert!(b1_v1::command(b).is_err());
        assert!(draft005::command(b).is_ok());
    }
    assert!(draft005::draft(fixture("fixtures/new-null-issues-draft.json")).is_err());
    assert!(draft005::command(fixture("fixtures/new-missing-edits-command.json")).is_err());
    assert!(b1_v1::command(fixture("fixtures/unsupported-update-profile-command.json")).is_err());
    assert!(b1_v1::snapshot(fixture("fixtures/unsupported-conflict-snapshot.json")).is_err());
}

// Removing either corrected R2 postcondition must flip the relevant refusals.
#[test]
fn all_28_r2_pairs_and_five_original_receipts() {
    let manifest = value(fixture("fixtures/correction-known-answers-r2.json"));
    let rows = manifest["cases"].as_array().unwrap();
    assert_eq!(rows.len(), 28);
    for row in rows {
        let c = fixture(row["command_file"].as_str().unwrap());
        let r = fixture(row["response_file"].as_str().unwrap());
        assert_eq!(digest(c), row["command_sha256"].as_str().unwrap());
        assert_eq!(digest(r), row["response_sha256"].as_str().unwrap());
        let cv = value(c);
        let rv = value(r);
        let bind = binding(
            &cv,
            &rv,
            row["command_sha256"].as_str().unwrap(),
            row["response_sha256"].as_str().unwrap(),
        );
        let ok = if row["profile"] == "B1RetainedV1" {
            b1_v1::receipt(c, r, &bind).is_ok()
        } else {
            draft005::receipt(c, r, &bind).is_ok()
        };
        assert_eq!(
            ok,
            row["expected_pair_audit"] == "supported",
            "{} {}",
            row["profile"],
            row["case"]
        );
    }
    let originals = value(fixture("fixtures/known-answers.json"));
    for row in originals["independent_receipt_examples"]
        .as_array()
        .unwrap()
    {
        let c = fixture(row["command_file"].as_str().unwrap());
        let r = fixture(row["response_file"].as_str().unwrap());
        let cv = value(c);
        let rv = value(r);
        let bind = binding(
            &cv,
            &rv,
            row["command_digest"].as_str().unwrap(),
            row["response_digest"].as_str().unwrap(),
        );
        if row["proposed_profile"] == "B1RetainedV1" {
            let receipt = b1_v1::receipt(c, r, &bind).unwrap();
            assert_eq!(receipt.original_command_bytes(), c);
            assert_eq!(receipt.original_response_bytes(), r);
        } else {
            let receipt = draft005::receipt(c, r, &bind).unwrap();
            assert_eq!(receipt.original_command_bytes(), c);
            assert_eq!(receipt.original_response_bytes(), r);
        }
    }
}

#[test]
fn both_unchanged_reviewer_counterexamples_refuse() {
    for (cmd, response) in [
        ("old-cancel-command", "cancel-absent-short-response"),
        ("old-privacy-command", "privacy-running-job-response"),
    ] {
        let c = fixture(&format!("fixtures/{cmd}.json"));
        let r = fixture(&format!("fixtures/{response}.json"));
        assert!(b1_v1::snapshot(r).is_ok());
        let cv = value(c);
        let rv = value(r);
        let cd = digest(c);
        let rd = digest(r);
        assert!(b1_v1::receipt(c, r, &binding(&cv, &rv, &cd, &rd)).is_err());
    }
}

#[test]
fn malformed_noncanonical_and_out_of_closure_documents_refuse() {
    let b = fixture("fixtures/old-initial-snapshot.json");
    let mut whitespace = b.to_vec();
    whitespace.push(b' ');
    assert!(b1_v1::snapshot(&whitespace).is_err());
    let duplicate =
        String::from_utf8(b.to_vec())
            .unwrap()
            .replacen("{", "{\"schema_version\":1,", 1);
    assert!(b1_v1::snapshot(duplicate.as_bytes()).is_err());
    for (pointer, bad) in [
        ("/schema_version", json!(2)),
        ("/snapshot_revision", json!("0")),
        ("/snapshot_revision", json!("01")),
        ("/snapshot_revision", json!("18446744073709551616")),
        (
            "/household_draft/household_id",
            json!("hh_00000000000000000000000000000000"),
        ),
        ("/household_draft/completed_stages", json!(["household"])),
        ("/household_draft/connections", json!({})),
        ("/weeks", json!([{}])),
        ("/jobs", json!([{}])),
    ] {
        let mut v = value(b);
        *v.pointer_mut(pointer).unwrap() = bad;
        assert!(b1_v1::snapshot(&canonical(&v)).is_err(), "{pointer}");
    }
    let mut missing = value(b);
    missing.as_object_mut().unwrap().remove("profile");
    assert!(b1_v1::snapshot(&canonical(&missing)).is_err());
    let mut extra = value(b);
    extra["extra"] = json!(true);
    assert!(b1_v1::snapshot(&canonical(&extra)).is_err());
    assert!(b1_v1::snapshot(&vec![b' '; 8 * 1024 * 1024 + 1]).is_err());
    let deep = format!("{}0{}", "[".repeat(34), "]".repeat(34));
    assert!(b1_v1::snapshot(deep.as_bytes()).is_err());
}
fn fixture(name: &str) -> &'static [u8] {
    match name {
        "fixtures/cancel-absent-short-response.json" => {
            include_bytes!("fixtures/cancel-absent-short-response.json")
        }
        "fixtures/correction-known-answers-r2.json" => {
            include_bytes!("fixtures/correction-known-answers-r2.json")
        }
        "fixtures/known-answers.json" => include_bytes!("fixtures/known-answers.json"),
        "fixtures/new-identical-cancel-command.json" => {
            include_bytes!("fixtures/new-identical-cancel-command.json")
        }
        "fixtures/new-identical-privacy-command.json" => {
            include_bytes!("fixtures/new-identical-privacy-command.json")
        }
        "fixtures/new-issue-command.json" => include_bytes!("fixtures/new-issue-command.json"),
        "fixtures/new-issue-response.json" => include_bytes!("fixtures/new-issue-response.json"),
        "fixtures/new-missing-edits-command.json" => {
            include_bytes!("fixtures/new-missing-edits-command.json")
        }
        "fixtures/new-missing-issues-draft.json" => {
            include_bytes!("fixtures/new-missing-issues-draft.json")
        }
        "fixtures/new-null-issues-draft.json" => {
            include_bytes!("fixtures/new-null-issues-draft.json")
        }
        "fixtures/new-resolve-command.json" => include_bytes!("fixtures/new-resolve-command.json"),
        "fixtures/new-resolve-response.json" => {
            include_bytes!("fixtures/new-resolve-response.json")
        }
        "fixtures/new-save-same-id-command.json" => {
            include_bytes!("fixtures/new-save-same-id-command.json")
        }
        "fixtures/new-transition-draft.json" => {
            include_bytes!("fixtures/new-transition-draft.json")
        }
        "fixtures/new-transition-snapshot.json" => {
            include_bytes!("fixtures/new-transition-snapshot.json")
        }
        "fixtures/new-unconfirmed-transition-draft.json" => {
            include_bytes!("fixtures/new-unconfirmed-transition-draft.json")
        }
        "fixtures/old-cancel-command.json" => include_bytes!("fixtures/old-cancel-command.json"),
        "fixtures/old-cancel-response.json" => include_bytes!("fixtures/old-cancel-response.json"),
        "fixtures/old-draft.json" => include_bytes!("fixtures/old-draft.json"),
        "fixtures/old-initial-snapshot.json" => {
            include_bytes!("fixtures/old-initial-snapshot.json")
        }
        "fixtures/old-privacy-command.json" => include_bytes!("fixtures/old-privacy-command.json"),
        "fixtures/old-privacy-response.json" => {
            include_bytes!("fixtures/old-privacy-response.json")
        }
        "fixtures/old-profile-readable-snapshot.json" => {
            include_bytes!("fixtures/old-profile-readable-snapshot.json")
        }
        "fixtures/old-save-command.json" => include_bytes!("fixtures/old-save-command.json"),
        "fixtures/old-save-response.json" => include_bytes!("fixtures/old-save-response.json"),
        "fixtures/old-unconfirmed-draft.json" => {
            include_bytes!("fixtures/old-unconfirmed-draft.json")
        }
        "fixtures/privacy-running-job-response.json" => {
            include_bytes!("fixtures/privacy-running-job-response.json")
        }
        "fixtures/r2/draft005-cancel-absent-1023.json" => {
            include_bytes!("fixtures/r2/draft005-cancel-absent-1023.json")
        }
        "fixtures/r2/draft005-cancel-absent-full.json" => {
            include_bytes!("fixtures/r2/draft005-cancel-absent-full.json")
        }
        "fixtures/r2/draft005-cancel-absent-short.json" => {
            include_bytes!("fixtures/r2/draft005-cancel-absent-short.json")
        }
        "fixtures/r2/draft005-cancel-present-running.json" => {
            include_bytes!("fixtures/r2/draft005-cancel-present-running.json")
        }
        "fixtures/r2/draft005-cancel-present-terminal.json" => {
            include_bytes!("fixtures/r2/draft005-cancel-present-terminal.json")
        }
        "fixtures/r2/draft005-privacy-awaiting-user.json" => {
            include_bytes!("fixtures/r2/draft005-privacy-awaiting-user.json")
        }
        "fixtures/r2/draft005-privacy-converted-needs-user.json" => {
            include_bytes!("fixtures/r2/draft005-privacy-converted-needs-user.json")
        }
        "fixtures/r2/draft005-privacy-empty.json" => {
            include_bytes!("fixtures/r2/draft005-privacy-empty.json")
        }
        "fixtures/r2/draft005-privacy-prepared.json" => {
            include_bytes!("fixtures/r2/draft005-privacy-prepared.json")
        }
        "fixtures/r2/draft005-privacy-retained-cancelled.json" => {
            include_bytes!("fixtures/r2/draft005-privacy-retained-cancelled.json")
        }
        "fixtures/r2/draft005-privacy-retained-failed.json" => {
            include_bytes!("fixtures/r2/draft005-privacy-retained-failed.json")
        }
        "fixtures/r2/draft005-privacy-retained-needs-user.json" => {
            include_bytes!("fixtures/r2/draft005-privacy-retained-needs-user.json")
        }
        "fixtures/r2/draft005-privacy-retained-succeeded.json" => {
            include_bytes!("fixtures/r2/draft005-privacy-retained-succeeded.json")
        }
        "fixtures/r2/draft005-privacy-running.json" => {
            include_bytes!("fixtures/r2/draft005-privacy-running.json")
        }
        "fixtures/r2/old-cancel-absent-1023.json" => {
            include_bytes!("fixtures/r2/old-cancel-absent-1023.json")
        }
        "fixtures/r2/old-cancel-absent-full.json" => {
            include_bytes!("fixtures/r2/old-cancel-absent-full.json")
        }
        "fixtures/r2/old-cancel-absent-short.json" => {
            include_bytes!("fixtures/r2/old-cancel-absent-short.json")
        }
        "fixtures/r2/old-cancel-present-running.json" => {
            include_bytes!("fixtures/r2/old-cancel-present-running.json")
        }
        "fixtures/r2/old-cancel-present-terminal.json" => {
            include_bytes!("fixtures/r2/old-cancel-present-terminal.json")
        }
        "fixtures/r2/old-privacy-awaiting-user.json" => {
            include_bytes!("fixtures/r2/old-privacy-awaiting-user.json")
        }
        "fixtures/r2/old-privacy-converted-needs-user.json" => {
            include_bytes!("fixtures/r2/old-privacy-converted-needs-user.json")
        }
        "fixtures/r2/old-privacy-empty.json" => {
            include_bytes!("fixtures/r2/old-privacy-empty.json")
        }
        "fixtures/r2/old-privacy-prepared.json" => {
            include_bytes!("fixtures/r2/old-privacy-prepared.json")
        }
        "fixtures/r2/old-privacy-retained-cancelled.json" => {
            include_bytes!("fixtures/r2/old-privacy-retained-cancelled.json")
        }
        "fixtures/r2/old-privacy-retained-failed.json" => {
            include_bytes!("fixtures/r2/old-privacy-retained-failed.json")
        }
        "fixtures/r2/old-privacy-retained-needs-user.json" => {
            include_bytes!("fixtures/r2/old-privacy-retained-needs-user.json")
        }
        "fixtures/r2/old-privacy-retained-succeeded.json" => {
            include_bytes!("fixtures/r2/old-privacy-retained-succeeded.json")
        }
        "fixtures/r2/old-privacy-running.json" => {
            include_bytes!("fixtures/r2/old-privacy-running.json")
        }
        "fixtures/same-id-different-command.json" => {
            include_bytes!("fixtures/same-id-different-command.json")
        }
        "fixtures/unsupported-conflict-snapshot.json" => {
            include_bytes!("fixtures/unsupported-conflict-snapshot.json")
        }
        "fixtures/unsupported-update-profile-command.json" => {
            include_bytes!("fixtures/unsupported-update-profile-command.json")
        }
        _ => panic!("unknown fixture {name}"),
    }
}
