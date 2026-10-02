use mstrchf_contracts::{
    DerivedPlan, ErrorCode, HouseholdProfile, Revision, ValidateShape,
    schema::{ROOT_NAMES, compare_generated_bytes, parse_root, render_contracts, schema_for},
};
use serde_json::{Value, json};
use std::path::Path;

#[test]
fn registry_has_the_nineteen_frozen_public_roots() {
    assert_eq!(ROOT_NAMES.len(), 19);
    assert_eq!(ROOT_NAMES[0], "household-draft");
    assert_eq!(ROOT_NAMES[18], "read-observation");
    let mut sorted = ROOT_NAMES.to_vec();
    sorted.sort_unstable();
    sorted.dedup();
    assert_eq!(sorted.len(), 19);
}

#[test]
fn schemars_uses_draft_2020_12_and_local_references() {
    let schema = schema_for::<HouseholdProfile>();
    assert_eq!(
        schema["$schema"],
        "https://json-schema.org/draft/2020-12/schema"
    );
    let serialized = serde_json::to_string(&schema).unwrap();
    assert!(!serialized.contains("http://example.com"));
    assert!(!serialized.contains("\"$ref\":\"http"));
    let revision = schema_for::<Revision>();
    assert_eq!(revision["type"], "string");
}

#[test]
fn outputs_are_deterministic_and_match_exact_file_set() {
    let first = render_contracts().unwrap();
    let second = render_contracts().unwrap();
    assert_eq!(first, second);
    assert_eq!(first.len(), 21); // 19 schemas, TypeScript, manifest.
    for root in ROOT_NAMES {
        let path = format!("contracts/v1/{root}.schema.json");
        let raw = first.get(&path).unwrap();
        let schema: Value = serde_json::from_slice(raw).unwrap();
        assert_eq!(schema["$id"], format!("urn:mstrchf:contracts:v1:{root}"));
        assert_eq!(
            schema["$schema"],
            "https://json-schema.org/draft/2020-12/schema"
        );
    }
    let ts = std::str::from_utf8(first.get("app/generated/contracts.ts").unwrap()).unwrap();
    assert!(ts.contains("export type WeekPlanDocumentV1"));
    assert!(ts.contains("export type MealRequirementsDocumentV1"));
    assert!(ts.contains("export type EventBatchDocumentV1"));
    let manifest: Value =
        serde_json::from_slice(first.get("contracts/v1/manifest.json").unwrap()).unwrap();
    assert_eq!(manifest["schema_version"], 1);
    assert_eq!(manifest["roots"].as_array().unwrap().len(), 19);
    assert_eq!(manifest["hashes"].as_object().unwrap().len(), 20);
}

#[test]
fn parse_dispatch_rejects_unknown_root_and_invalid_version() {
    assert!(parse_root("unknown-root", b"{}").is_err());
    assert!(parse_root("week-plan", br#"{"schema_version":2,"week_plan":{}}"#).is_err());
}

#[test]
fn generated_drift_checks_fail_without_changing_observed_bytes() {
    let complete = render_contracts().unwrap();
    assert!(compare_generated_bytes(&complete).is_ok());
    let manifest = "contracts/v1/manifest.json";
    let schema = "contracts/v1/week-plan.schema.json";
    let ts = "app/generated/contracts.ts";

    let mut missing = complete.clone();
    missing.remove(manifest);
    let before = missing.clone();
    assert!(compare_generated_bytes(&missing).is_err());
    assert_eq!(missing, before);

    let mut extra = complete.clone();
    extra.insert("contracts/v1/extra.schema.json".into(), b"{}".to_vec());
    let before = extra.clone();
    assert!(compare_generated_bytes(&extra).is_err());
    assert_eq!(extra, before);

    for path in [schema, manifest, ts] {
        let mut edited = complete.clone();
        edited.get_mut(path).unwrap().push(b' ');
        let before = edited.clone();
        assert!(compare_generated_bytes(&edited).is_err(), "edited {path}");
        assert_eq!(edited, before);

        let mut empty = complete.clone();
        empty.insert(path.into(), Vec::new());
        let before = empty.clone();
        assert!(compare_generated_bytes(&empty).is_err(), "empty {path}");
        assert_eq!(empty, before);
    }
}

#[test]
fn standalone_envelopes_validate_versions_and_preserve_embedded_material() {
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .unwrap()
        .parent()
        .unwrap()
        .join("tests/fixtures/contracts");
    for (root, field, file) in [
        ("week-plan", "week_plan", "week-plan.json"),
        (
            "meal-requirements",
            "meal_requirements",
            "meal-requirements.json",
        ),
        ("event-batch", "event_batch", "event-batch.json"),
    ] {
        let raw = std::fs::read(fixture_root.join(file)).unwrap();
        parse_root(root, &raw).unwrap_or_else(|error| panic!("{root}: {error}"));
        let value: Value = serde_json::from_slice(&raw).unwrap();
        assert_eq!(value.as_object().unwrap().len(), 2, "{root}");
        assert_eq!(value["schema_version"], 1, "{root}");
        let embedded_before = serde_jcs::to_vec(&value[field]).unwrap();

        let schemas = render_contracts().unwrap();
        let schema_bytes = schemas
            .get(&format!("contracts/v1/{root}.schema.json"))
            .unwrap();
        let schema: Value = serde_json::from_slice(schema_bytes).unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        assert!(validator.is_valid(&value), "valid schema {root}");

        let mut missing = value.clone();
        missing.as_object_mut().unwrap().remove("schema_version");
        assert!(
            parse_root(root, &serde_json::to_vec(&missing).unwrap()).is_err(),
            "missing version {root}"
        );
        assert!(
            !validator.is_valid(&missing),
            "schema missing version {root}"
        );

        let mut wrong = value.clone();
        wrong["schema_version"] = json!(2);
        assert!(
            parse_root(root, &serde_json::to_vec(&wrong).unwrap()).is_err(),
            "wrong version {root}"
        );
        assert!(!validator.is_valid(&wrong), "schema wrong version {root}");

        let mut embedded_version = value.clone();
        embedded_version[field]["schema_version"] = json!(1);
        assert!(
            parse_root(root, &serde_json::to_vec(&embedded_version).unwrap()).is_err(),
            "embedded version {root}"
        );
        assert!(
            !validator.is_valid(&embedded_version),
            "schema embedded version {root}"
        );

        let roundtrip: Value =
            serde_json::from_slice(&serde_json::to_vec(&value).unwrap()).unwrap();
        assert_eq!(
            embedded_before,
            serde_jcs::to_vec(&roundtrip[field]).unwrap(),
            "embedded bytes {root}"
        );
    }
}

/// Bounds here come from frozen contract-design.md §11, rather than generated schemas.
/// Each case exercises the published root parser and its generated JSON Schema.
#[test]
fn collection_boundaries_on_positive_root_fixtures() {
    struct Case {
        root: &'static str,
        path: &'static str,
        min: usize,
        max: usize,
    }
    let cases = [
        Case {
            root: "household-draft",
            path: "/diners",
            min: 0,
            max: 64,
        },
        Case {
            root: "household-draft",
            path: "/completed_stages",
            min: 0,
            max: 4,
        },
        Case {
            root: "household-profile",
            path: "/diners",
            min: 1,
            max: 64,
        },
        Case {
            root: "household-profile",
            path: "/diners/0/restrictions/data/requirements",
            min: 1,
            max: 64,
        },
        Case {
            root: "household-profile",
            path: "/diners/0/dislikes",
            min: 0,
            max: 256,
        },
        Case {
            root: "household-profile",
            path: "/diners/0/portions/meal_overrides",
            min: 0,
            max: 3,
        },
        Case {
            root: "household-profile",
            path: "/preferences/dislikes",
            min: 0,
            max: 256,
        },
        Case {
            root: "household-profile",
            path: "/preferences/equipment",
            min: 0,
            max: 9,
        },
        Case {
            root: "household-profile",
            path: "/preferences/meal_times",
            min: 3,
            max: 3,
        },
        Case {
            root: "household-profile",
            path: "/usual_week/enabled_meal_types",
            min: 0,
            max: 3,
        },
        Case {
            root: "household-profile",
            path: "/usual_week/day_enabled",
            min: 7,
            max: 7,
        },
        Case {
            root: "household-profile",
            path: "/usual_week/cells",
            min: 21,
            max: 21,
        },
        Case {
            root: "household-profile",
            path: "/usual_week/cells/0/attendee_ids",
            min: 0,
            max: 64,
        },
        Case {
            root: "household-profile",
            path: "/connections/delivery_preferences",
            min: 0,
            max: 7,
        },
        Case {
            root: "household-profile",
            path: "/connections/reminders/channels",
            min: 0,
            max: 2,
        },
        Case {
            root: "household-profile",
            path: "/connections/reminders/prompts",
            min: 0,
            max: 2,
        },
        Case {
            root: "meal-requirements",
            path: "/meal_requirements/attendees",
            min: 0,
            max: 64,
        },
        Case {
            root: "meal-requirements",
            path: "/meal_requirements/attendees/0/hard_requirements",
            min: 0,
            max: 64,
        },
        Case {
            root: "meal-requirements",
            path: "/meal_requirements/attendees/0/portion/meal_overrides",
            min: 0,
            max: 3,
        },
        Case {
            root: "meal-requirements",
            path: "/meal_requirements/shared_hard_requirements",
            min: 0,
            max: 64,
        },
        Case {
            root: "meal-requirements",
            path: "/meal_requirements/equipment",
            min: 0,
            max: 9,
        },
        Case {
            root: "recipe-candidate",
            path: "/ingredients",
            min: 0,
            max: 256,
        },
        Case {
            root: "recipe-candidate",
            path: "/steps",
            min: 0,
            max: 256,
        },
        Case {
            root: "recipe-candidate",
            path: "/unresolved",
            min: 0,
            max: 256,
        },
        Case {
            root: "recipe-revision",
            path: "/source/authors",
            min: 0,
            max: 64,
        },
        Case {
            root: "recipe-revision",
            path: "/text_rights/adaptations",
            min: 0,
            max: 64,
        },
        Case {
            root: "recipe-revision",
            path: "/text_rights/evidence_ids",
            min: 0,
            max: 256,
        },
        Case {
            root: "recipe-revision",
            path: "/ingredients",
            min: 1,
            max: 256,
        },
        Case {
            root: "recipe-revision",
            path: "/ingredients/0/conversion_evidence",
            min: 0,
            max: 64,
        },
        Case {
            root: "recipe-revision",
            path: "/ingredients/0/step_ids",
            min: 1,
            max: 256,
        },
        Case {
            root: "recipe-revision",
            path: "/yield/components",
            min: 1,
            max: 128,
        },
        Case {
            root: "recipe-revision",
            path: "/steps",
            min: 1,
            max: 256,
        },
        Case {
            root: "recipe-revision",
            path: "/steps/0/ingredient_ids",
            min: 0,
            max: 256,
        },
        Case {
            root: "recipe-revision",
            path: "/steps/0/component_ids",
            min: 0,
            max: 128,
        },
        Case {
            root: "recipe-revision",
            path: "/steps/0/depends_on",
            min: 0,
            max: 256,
        },
        Case {
            root: "recipe-revision",
            path: "/equipment",
            min: 0,
            max: 9,
        },
        Case {
            root: "recipe-revision",
            path: "/meal_types",
            min: 1,
            max: 3,
        },
        Case {
            root: "recipe-revision",
            path: "/portability/evidence_ids",
            min: 0,
            max: 256,
        },
        Case {
            root: "recipe-revision",
            path: "/components",
            min: 1,
            max: 128,
        },
        Case {
            root: "recipe-revision",
            path: "/components/0/ingredient_ids",
            min: 0,
            max: 256,
        },
        Case {
            root: "recipe-revision",
            path: "/components/0/storage_rule_ids",
            min: 0,
            max: 256,
        },
        Case {
            root: "recipe-revision",
            path: "/extras",
            min: 0,
            max: 64,
        },
        Case {
            root: "recipe-revision",
            path: "/extras/0/eligible_person_requirements",
            min: 0,
            max: 64,
        },
        Case {
            root: "recipe-revision",
            path: "/extras/0/supported_with",
            min: 1,
            max: 128,
        },
        Case {
            root: "recipe-revision",
            path: "/dietary_evidence",
            min: 0,
            max: 256,
        },
        Case {
            root: "recipe-revision",
            path: "/storage",
            min: 0,
            max: 128,
        },
        Case {
            root: "recipe-revision",
            path: "/prep",
            min: 0,
            max: 128,
        },
        Case {
            root: "recipe-revision",
            path: "/portion_presets",
            min: 1,
            max: 192,
        },
        Case {
            root: "recipe-revision",
            path: "/portion_presets/0/evidence_ids",
            min: 0,
            max: 256,
        },
        Case {
            root: "recipe-revision",
            path: "/reviews",
            min: 0,
            max: 256,
        },
        Case {
            root: "recipe-revision",
            path: "/capabilities/blocking_issues",
            min: 0,
            max: 256,
        },
        Case {
            root: "week",
            path: "/day_enabled",
            min: 7,
            max: 7,
        },
        Case {
            root: "week",
            path: "/cells",
            min: 21,
            max: 21,
        },
        Case {
            root: "week-plan",
            path: "/week_plan/selections",
            min: 0,
            max: 21,
        },
        Case {
            root: "week-plan",
            path: "/week_plan/batch_links",
            min: 0,
            max: 256,
        },
        Case {
            root: "planning-context",
            path: "/requirements",
            min: 0,
            max: 84,
        },
        Case {
            root: "planning-context",
            path: "/weeks",
            min: 0,
            max: 4,
        },
        Case {
            root: "planning-context",
            path: "/recipes",
            min: 0,
            max: 512,
        },
        Case {
            root: "planning-context",
            path: "/stamp/read_weeks",
            min: 0,
            max: 4,
        },
        Case {
            root: "planning-context",
            path: "/stamp/write_weeks",
            min: 0,
            max: 4,
        },
        Case {
            root: "planning-context",
            path: "/stamp/recipe_refs",
            min: 0,
            max: 512,
        },
        Case {
            root: "derived-plan",
            path: "/batches",
            min: 0,
            max: 256,
        },
        Case {
            root: "derived-plan",
            path: "/allocations",
            min: 0,
            max: 4096,
        },
        Case {
            root: "derived-plan",
            path: "/prep_sessions",
            min: 0,
            max: 256,
        },
        Case {
            root: "derived-plan",
            path: "/warnings",
            min: 0,
            max: 100,
        },
        Case {
            root: "derived-plan",
            path: "/demand/lines",
            min: 0,
            max: 4096,
        },
        Case {
            root: "job-request",
            path: "/required_capabilities",
            min: 1,
            max: 8,
        },
        Case {
            root: "job-request",
            path: "/allowed_artefacts",
            min: 0,
            max: 64,
        },
        Case {
            root: "job-result",
            path: "/observations",
            min: 0,
            max: 256,
        },
        Case {
            root: "job-result",
            path: "/unresolved",
            min: 0,
            max: 256,
        },
        Case {
            root: "event-batch",
            path: "/event_batch/events",
            min: 1,
            max: 256,
        },
        Case {
            root: "snapshot",
            path: "/weeks",
            min: 0,
            max: 4,
        },
        Case {
            root: "snapshot",
            path: "/jobs",
            min: 0,
            max: 1024,
        },
        Case {
            root: "snapshot",
            path: "/conflicts",
            min: 0,
            max: 100,
        },
        Case {
            root: "read-observation",
            path: "/body/data/items",
            min: 0,
            max: 256,
        },
        Case {
            root: "read-observation",
            path: "/body/data/items/0/offers",
            min: 0,
            max: 64,
        },
    ];
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/contracts");
    let generated = render_contracts().unwrap();
    let mut checked = 0;
    let mut zero = 0;
    let mut minimum = 0;
    let mut maximum = 0;
    for case in &cases {
        let label = format!("{}{}", case.root, case.path);
        let raw = std::fs::read(current_fixture(&fixture_root, case.root)).unwrap();
        let baseline: Value = serde_json::from_slice(&raw).unwrap();
        parse_root(case.root, &raw).unwrap_or_else(|error| panic!("baseline {label}: {error}"));
        let schema: Value =
            serde_json::from_slice(&generated[&format!("contracts/v1/{}.schema.json", case.root)])
                .unwrap();
        let validator = jsonschema::validator_for(&schema).unwrap();
        assert!(validator.is_valid(&baseline), "schema baseline {label}");
        let original = baseline
            .pointer(case.path)
            .unwrap_or_else(|| panic!("missing {label}"))
            .as_array()
            .unwrap();
        assert!(
            (case.min..=case.max).contains(&original.len()),
            "baseline bounds {label}"
        );

        // An empty optional array in a positive root proves its zero minimum at runtime.
        if original.is_empty() {
            assert_eq!(case.min, 0, "empty positive fixture for mandatory {label}");
            checked += 1;
            zero += 1;
        }
        if case.min > 0 {
            let mut short = baseline.clone();
            short
                .pointer_mut(case.path)
                .unwrap()
                .as_array_mut()
                .unwrap()
                .clear();
            let schema_min = if case.path.ends_with("/restrictions/data/requirements") {
                schema.pointer("/$defs/Array_HardRequirement_1_64/minItems")
                    == Some(&json!(case.min))
                    && !validator.is_valid(&short)
            } else {
                validator.iter_errors(&short).any(|error| {
                    error.instance_path().to_string() == case.path
                        && error.schema_path().to_string().ends_with("/minItems")
                })
            };
            assert!(schema_min, "schema minItems {label}");
            let error = parse_root(case.root, &serde_json::to_vec(&short).unwrap()).unwrap_err();
            assert_eq!(
                error.code,
                ErrorCode::InvalidValue,
                "runtime minimum {label}: {error}"
            );
            checked += 1;
            minimum += 1;
        }
        {
            // Count limits precede element decoding. Null deliberately probes
            // that ordering when the positive fixture's array is empty.
            let exemplar = original.first().cloned().unwrap_or(Value::Null);
            let mut long = baseline.clone();
            let array = long.pointer_mut(case.path).unwrap().as_array_mut().unwrap();
            array.resize(case.max + 1, exemplar);
            // oneOf reports its enclosing branch rather than the inner keyword.
            let schema_max = if case.path.ends_with("/restrictions/data/requirements") {
                schema.pointer("/$defs/Array_HardRequirement_1_64/maxItems")
                    == Some(&json!(case.max))
                    && !validator.is_valid(&long)
            } else if case.root == "read-observation" {
                let definition = if case.path.ends_with("/offers") {
                    "/$defs/Array_OfferObservation_0_64/maxItems"
                } else {
                    "/$defs/Array_ProductObservation_0_256/maxItems"
                };
                schema.pointer(definition) == Some(&json!(case.max)) && !validator.is_valid(&long)
            } else {
                validator.iter_errors(&long).any(|error| {
                    error.instance_path().to_string() == case.path
                        && error.schema_path().to_string().ends_with("/maxItems")
                })
            };
            assert!(schema_max, "schema maxItems {label}");
            let error = parse_root(case.root, &serde_json::to_vec(&long).unwrap()).unwrap_err();
            assert_eq!(
                error.code,
                ErrorCode::LimitExceeded,
                "runtime maximum {label}: {error}"
            );
            if case.max == 4096 {
                // The global JSON sequence cap fires first at 4097; check the
                // typed shape traversal separately for the field-level path.
                assert!(
                    DerivedPlan::validate_wire(&long)
                        .unwrap_err()
                        .iter()
                        .any(|issue| issue.path == case.path
                            && issue.code == ErrorCode::LimitExceeded),
                    "typed maximum path {label}"
                );
            } else {
                assert!(
                    error
                        .issues
                        .iter()
                        .any(|issue| issue.path == case.path
                            && issue.code == ErrorCode::LimitExceeded),
                    "runtime maximum path {label}: {error}"
                );
            }
            checked += 1;
            maximum += 1;
        }
    }
    println!(
        "collection boundary concrete subcases={checked}; named fixture fields={}; zero={zero}; minimum={minimum}; maximum={maximum}",
        cases.len()
    );
    assert_eq!(checked, zero + minimum + maximum);
    assert!(
        zero >= 30 && minimum >= 10 && maximum == cases.len(),
        "collection boundary suite lost meaningful coverage"
    );
}

#[test]
fn keyed_collections_reject_same_identity_with_different_other_fields() {
    // All changes below preserve each item's wire shape. Equality of the whole
    // record would miss these duplicates; the frozen key must decide identity.
    let cases = [
        (
            "household-profile",
            "/diners",
            "/display_name",
            json!("Different name"),
        ),
        (
            "recipe-revision",
            "/ingredients",
            "/name",
            json!("Different ingredient name"),
        ),
        (
            "recipe-revision",
            "/extras",
            "/title",
            json!("Different extra title"),
        ),
        (
            "recipe-revision",
            "/portion_presets",
            "/servings/numerator",
            json!("2"),
        ),
        (
            "recipe-revision",
            "/components",
            "/title",
            json!("Different component title"),
        ),
        (
            "recipe-revision",
            "/steps",
            "/text",
            json!("Different instruction text"),
        ),
        (
            "planning-context",
            "/recipes",
            "/title",
            json!("Different recipe title"),
        ),
        ("snapshot", "/jobs", "/state", json!("failed")),
        (
            "read-observation",
            "/body/data/items",
            "/title",
            json!("Different product title"),
        ),
    ];
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/contracts");
    for (root, path, changed_field, different) in &cases {
        let mut value: Value =
            serde_json::from_slice(&std::fs::read(current_fixture(&fixture_root, root)).unwrap())
                .unwrap();
        parse_root(root, &serde_json::to_vec(&value).unwrap()).unwrap();
        let items = value.pointer_mut(path).unwrap().as_array_mut().unwrap();
        let mut duplicate = items[0].clone();
        assert_ne!(
            duplicate.pointer(changed_field),
            Some(different),
            "mutation is real: {root}{path}"
        );
        *duplicate.pointer_mut(changed_field).unwrap() = different.clone();
        items.push(duplicate);
        let error = parse_root(root, &serde_json::to_vec(&value).unwrap()).unwrap_err();
        assert_eq!(
            error.code,
            ErrorCode::InvalidValue,
            "keyed duplicate {root}{path}: {error}"
        );
        assert!(
            error.issues.iter().any(|issue| issue.path.ends_with(path)),
            "keyed duplicate path {root}{path}: {error:?}"
        );
    }
    println!("keyed collection concrete subcases={}", cases.len());
}

#[test]
fn nested_collection_types_keep_their_own_bounds_and_identity_rules() {
    use mstrchf_contracts::{Fact, ReadScope, Text, parse_document};
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/contracts");
    let request: Value = serde_json::from_slice(
        &std::fs::read(
            fixture_root.join("ruling004-current--job-request-product-observation.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let scope = request.pointer("/scope/data/read_scope").unwrap().clone();
    parse_document::<ReadScope>(&serde_json::to_vec(&scope).unwrap()).unwrap();
    let schema = schema_for::<ReadScope>();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&scope));
    for (path, min, max) in [
        ("/allowed_kinds", 1, 4),
        ("/products", 0, 256),
        ("/event_ids", 0, 256),
    ] {
        let original = scope.pointer(path).unwrap().as_array().unwrap();
        assert!((min..=max).contains(&original.len()), "ReadScope{path}");
        if min == 0 {
            assert!(original.is_empty(), "zero minimum sample ReadScope{path}");
        } else {
            let mut short = scope.clone();
            short
                .pointer_mut(path)
                .unwrap()
                .as_array_mut()
                .unwrap()
                .clear();
            assert!(
                validator
                    .iter_errors(&short)
                    .any(|error| error.instance_path().to_string() == path
                        && error.schema_path().to_string().ends_with("/minItems")),
                "schema minimum ReadScope{path}"
            );
            assert_eq!(
                parse_document::<ReadScope>(&serde_json::to_vec(&short).unwrap())
                    .unwrap_err()
                    .code,
                ErrorCode::InvalidValue,
                "runtime minimum ReadScope{path}"
            );
        }
        if let Some(exemplar) = original.first() {
            let mut long = scope.clone();
            long.pointer_mut(path)
                .unwrap()
                .as_array_mut()
                .unwrap()
                .resize(max + 1, exemplar.clone());
            assert!(
                validator
                    .iter_errors(&long)
                    .any(|error| error.instance_path().to_string() == path
                        && error.schema_path().to_string().ends_with("/maxItems")),
                "schema maximum ReadScope{path}"
            );
            assert_eq!(
                parse_document::<ReadScope>(&serde_json::to_vec(&long).unwrap())
                    .unwrap_err()
                    .code,
                ErrorCode::LimitExceeded,
                "runtime maximum ReadScope{path}"
            );
        }
    }

    let conflicting = json!({"kind":"conflicting","data":{"values":["alpha","beta"]}});
    parse_document::<Fact<Text>>(&serde_json::to_vec(&conflicting).unwrap()).unwrap();
    let fact_schema = schema_for::<Fact<Text>>();
    let fact_validator = jsonschema::validator_for(&fact_schema).unwrap();
    assert!(fact_validator.is_valid(&conflicting));
    let mut too_short = conflicting.clone();
    too_short["data"]["values"] = json!(["alpha"]);
    assert!(
        !fact_validator.is_valid(&too_short),
        "Fact.conflicting schema minimum 2"
    );
    assert!(
        parse_document::<Fact<Text>>(&serde_json::to_vec(&too_short).unwrap()).is_err(),
        "Fact.conflicting minimum 2"
    );
    let mut too_long = conflicting.clone();
    too_long["data"]["values"] = json!(["a", "b", "c", "d", "e", "f", "g", "h", "i"]);
    assert!(
        !fact_validator.is_valid(&too_long),
        "Fact.conflicting schema maximum 8"
    );
    assert_eq!(
        parse_document::<Fact<Text>>(&serde_json::to_vec(&too_long).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::LimitExceeded,
        "Fact.conflicting maximum 8"
    );
    let mut duplicate = conflicting.clone();
    duplicate["data"]["values"] = json!(["alpha", "alpha"]);
    assert_eq!(
        parse_document::<Fact<Text>>(&serde_json::to_vec(&duplicate).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::InvalidValue,
        "Fact.conflicting distinct values"
    );
    println!("nested collection concrete subcases=7 (ReadScope 4, Fact.conflicting 3)");
}

#[test]
fn collection_limits_on_types_without_positive_root_exemplars() {
    use mstrchf_contracts::*;
    fn probe<T: schemars::JsonSchema + ValidateShape>(
        name: &str,
        base: Value,
        path: &str,
        min: usize,
        max: usize,
    ) {
        fn put(value: &mut Value, path: &str, len: usize) {
            let (parent, field) = path.rsplit_once('/').unwrap();
            value
                .pointer_mut(parent)
                .unwrap()
                .as_object_mut()
                .unwrap()
                .insert(field.into(), Value::Array(vec![Value::Null; len]));
        }
        let schema = schema_for::<T>();
        let validator = jsonschema::validator_for(&schema).unwrap();
        let tagged = base.get("kind").is_some();
        let mut long = base.clone();
        put(&mut long, path, max + 1);
        assert!(!validator.is_valid(&long), "schema maximum {name}{path}");
        if !tagged {
            assert!(
                validator
                    .iter_errors(&long)
                    .any(|error| error.instance_path().to_string() == path
                        && error.schema_path().to_string().ends_with("/maxItems")),
                "schema maxItems keyword {name}{path}"
            );
        }
        let issues = T::validate_wire(&long).unwrap_err();
        assert!(
            issues
                .iter()
                .any(|issue| issue.path == path && issue.code == ErrorCode::LimitExceeded),
            "typed maximum {name}{path}: {issues:?}"
        );
        if min > 0 {
            let mut short = base;
            put(&mut short, path, 0);
            assert!(!validator.is_valid(&short), "schema minimum {name}{path}");
            if !tagged {
                assert!(
                    validator
                        .iter_errors(&short)
                        .any(|error| error.instance_path().to_string() == path
                            && error.schema_path().to_string().ends_with("/minItems")),
                    "schema minItems keyword {name}{path}"
                );
            }
            let issues = T::validate_wire(&short).unwrap_err();
            assert!(
                issues
                    .iter()
                    .any(|issue| issue.path == path && issue.code == ErrorCode::InvalidValue),
                "typed minimum {name}{path}: {issues:?}"
            );
        }
    }
    macro_rules! row {
        ($t:ty, $name:literal, $base:expr, $path:literal, $min:literal, $max:literal) => {
            probe::<$t>($name, $base, $path, $min, $max);
        };
    }
    row!(Occasion, "Occasion", json!({}), "/guests", 0, 64);
    row!(
        CalendarSettings,
        "CalendarSettings",
        json!({}),
        "/meal_times",
        3,
        3
    );
    row!(Occasion, "Occasion", json!({}), "/host_diner_ids", 0, 64);
    row!(Cell, "Cell", json!({}), "/attendee_ids", 0, 64);
    row!(BatchLink, "BatchLink", json!({}), "/components", 1, 128);
    row!(
        ExtraSelection,
        "ExtraSelection",
        json!({}),
        "/recipients",
        1,
        64
    );
    row!(
        MealOverrides,
        "MealOverrides",
        json!({}),
        "/portions",
        0,
        64
    );
    row!(
        MealSelection,
        "MealSelection",
        json!({}),
        "/servings",
        1,
        64
    );
    row!(MealSelection, "MealSelection", json!({}), "/extras", 0, 64);
    row!(WeekEdit, "WeekEdit", json!({}), "/day_enabled", 7, 7);
    row!(WeekEdit, "WeekEdit", json!({}), "/cells", 21, 21);
    row!(WeekChange, "WeekChange", json!({}), "/replace_slots", 0, 21);
    row!(PlanProposal, "PlanProposal", json!({}), "/changes", 1, 4);
    row!(
        SelectionHistory,
        "SelectionHistory",
        json!({}),
        "/servings",
        1,
        64
    );
    row!(
        SelectionHistory,
        "SelectionHistory",
        json!({}),
        "/extras",
        0,
        64
    );
    row!(Feedback, "Feedback", json!({}), "/reasons", 0, 4);
    row!(
        PlanningHistory,
        "PlanningHistory",
        json!({}),
        "/selected",
        0,
        4096
    );
    row!(
        PlanningHistory,
        "PlanningHistory",
        json!({}),
        "/ordered",
        0,
        4096
    );
    row!(
        PlanningHistory,
        "PlanningHistory",
        json!({}),
        "/feedback",
        0,
        4096
    );
    row!(
        PlanningHistory,
        "PlanningHistory",
        json!({}),
        "/hidden_recipe_ids",
        0,
        512
    );
    row!(
        DerivedBatch,
        "DerivedBatch",
        json!({}),
        "/components",
        1,
        128
    );
    row!(PrepSpec, "PrepSpec", json!({}), "/step_ids", 1, 256);
    row!(PrepSpec, "PrepSpec", json!({}), "/depends_on", 0, 128);
    row!(PrepSpec, "PrepSpec", json!({}), "/ingredient_ids", 0, 256);
    row!(
        DemandLine,
        "DemandLine",
        json!({}),
        "/origin_batches",
        0,
        256
    );
    row!(DemandLine, "DemandLine", json!({}), "/origin_extras", 0, 64);
    row!(
        DemandLine,
        "DemandLine",
        json!({}),
        "/conversion_evidence_ids",
        0,
        256
    );
    row!(
        PrepSession,
        "PrepSession",
        json!({}),
        "/linked_slots",
        1,
        21
    );
    row!(PrepSession, "PrepSession", json!({}), "/tasks", 1, 256);
    row!(
        PrepSession,
        "PrepSession",
        json!({}),
        "/ingredient_ids",
        0,
        256
    );
    row!(
        ScheduledPrepTask,
        "ScheduledPrepTask",
        json!({}),
        "/depends_on",
        0,
        256
    );
    row!(
        ScheduledPrepTask,
        "ScheduledPrepTask",
        json!({}),
        "/active_intervals",
        0,
        256
    );
    row!(
        ReconciliationEvidence,
        "ReconciliationEvidence",
        json!({}),
        "/unresolved_operation_ids",
        0,
        256
    );
    row!(
        AiHistory,
        "AiHistory",
        json!({}),
        "/selected_recipe_refs",
        0,
        512
    );
    row!(
        AiHistory,
        "AiHistory",
        json!({}),
        "/ordered_recipe_refs",
        0,
        512
    );
    row!(
        AiHistory,
        "AiHistory",
        json!({}),
        "/hidden_recipe_ids",
        0,
        512
    );
    row!(AiHistory, "AiHistory", json!({}), "/feedback", 0, 512);
    row!(AiFeedback, "AiFeedback", json!({}), "/reasons", 0, 4);
    row!(
        OtherWeekSummary,
        "OtherWeekSummary",
        json!({}),
        "/selections",
        0,
        21
    );
    row!(
        RecipeAdaptation,
        "RecipeAdaptation",
        json!({}),
        "/requirements",
        0,
        64
    );
    row!(
        RecipeAdaptation,
        "RecipeAdaptation",
        json!({}),
        "/equipment",
        0,
        9
    );
    row!(
        RecipeAdaptation,
        "RecipeAdaptation",
        json!({}),
        "/portion_requests",
        0,
        192
    );
    row!(
        ApprovedContributionRef,
        "ApprovedContributionRef",
        json!({}),
        "/lines",
        1,
        256
    );
    row!(Conflict, "Conflict", json!({}), "/week_ids", 0, 4);
    row!(Conflict, "Conflict", json!({}), "/slot_ids", 0, 84);
    row!(Conflict, "Conflict", json!({}), "/recipe_refs", 0, 512);
    row!(Conflict, "Conflict", json!({}), "/evidence_ids", 0, 256);
    row!(DomainError, "DomainError", json!({}), "/issues", 0, 100);
    row!(
        ActionScope,
        "ActionScope.recipe",
        json!({"kind":"recipe","data":{}}),
        "/data/source_ids",
        1,
        64
    );
    row!(
        ActionScope,
        "ActionScope.plan",
        json!({"kind":"plan","data":{}}),
        "/data/slots",
        0,
        84
    );
    row!(
        ActionScope,
        "ActionScope.calendar",
        json!({"kind":"calendar","data":{}}),
        "/data/event_ids",
        1,
        256
    );
    row!(
        ActionScope,
        "ActionScope.booking",
        json!({"kind":"booking","data":{}}),
        "/data/week_ids",
        1,
        4
    );
    row!(
        JobPayload,
        "JobPayload.personalised_plan",
        json!({"kind":"personalised_plan","data":{}}),
        "/data/requirements",
        1,
        84
    );
    row!(
        JobPayload,
        "JobPayload.personalised_plan",
        json!({"kind":"personalised_plan","data":{}}),
        "/data/recipe_refs",
        0,
        512
    );
    row!(
        JobPayload,
        "JobPayload.personalised_plan",
        json!({"kind":"personalised_plan","data":{}}),
        "/data/other_weeks",
        0,
        3
    );
    row!(
        JobPayload,
        "JobPayload.generic_catalogue",
        json!({"kind":"generic_catalogue","data":{}}),
        "/data/public_recipes",
        1,
        512
    );
    row!(
        JobPayload,
        "JobPayload.generic_catalogue",
        json!({"kind":"generic_catalogue","data":{}}),
        "/data/meal_types",
        0,
        3
    );
    row!(
        JobPayload,
        "JobPayload.product_observation",
        json!({"kind":"product_observation","data":{}}),
        "/data/products",
        0,
        256
    );
    row!(
        JobPayload,
        "JobPayload.booking_check",
        json!({"kind":"booking_check","data":{}}),
        "/data/week_ids",
        1,
        4
    );
    row!(
        JobPayload,
        "JobPayload.setup_check",
        json!({"kind":"setup_check","data":{}}),
        "/data/requested_capabilities",
        1,
        8
    );
    row!(
        ResultProposal,
        "ResultProposal.setup",
        json!({"kind":"setup","data":{}}),
        "/data/capabilities",
        1,
        8
    );
    row!(
        ResultProposal,
        "ResultProposal.catalogue",
        json!({"kind":"catalogue","data":{}}),
        "/data/recipe_refs",
        1,
        512
    );
    row!(
        AppCommand,
        "AppCommand.confirm_essentials",
        json!({"kind":"confirm_essentials","data":{}}),
        "/data/confirmations",
        1,
        4096
    );
    row!(
        ReadBody,
        "ReadBody.calendar_events",
        json!({"kind":"calendar_events","data":{}}),
        "/data/items",
        0,
        256
    );
    row!(
        ReadBody,
        "ReadBody.slots",
        json!({"kind":"slots","data":{}}),
        "/data/items",
        0,
        256
    );
    row!(
        BasketSnapshot,
        "BasketSnapshot",
        json!({}),
        "/lines",
        0,
        512
    );
    println!("direct typed collection limit rows=66");
}

#[test]
fn outbound_bundle_internal_artefacts_accept_zero_and_reject_over_sixty_four() {
    use mstrchf_contracts::{
        AllowedArtefactBytes, ArtefactId, JobRequestV1, OutboundBundle, PrivacyPolicy,
        parse_document, validate_outbound,
    };
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/contracts");
    let request: JobRequestV1 = parse_document(
        &std::fs::read(fixture_root.join("ruling004-current--job-request.json")).unwrap(),
    )
    .unwrap();
    let context: Value = serde_json::from_slice(
        &std::fs::read(fixture_root.join("ruling004-current--planning-context.json")).unwrap(),
    )
    .unwrap();
    let policy: PrivacyPolicy =
        parse_document(&serde_json::to_vec(&context["privacy"]).unwrap()).unwrap();
    let mut bundle = OutboundBundle {
        request,
        artefacts: vec![],
    };
    validate_outbound(&bundle, &policy).unwrap();
    bundle.artefacts = (0..65)
        .map(|_| AllowedArtefactBytes {
            artefact_id: ArtefactId("ar_00000000000000000000000000000001".into()),
            bytes: vec![],
        })
        .collect();
    assert_eq!(
        validate_outbound(&bundle, &policy).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
    println!("outbound internal artefact count subcases=2");
}

// Ruling004 current synthetic copies are separately named; originals remain immutable.
fn current_fixture(root: &Path, name: &str) -> std::path::PathBuf {
    let current005 = root.join(format!("ruling005-current--{name}.json"));
    if current005.exists() {
        return current005;
    }
    let current = root.join(format!("ruling004-current--{name}.json"));
    if current.exists() {
        current
    } else {
        root.join(format!("{name}.json"))
    }
}
