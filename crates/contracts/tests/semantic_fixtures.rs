use mstrchf_contracts::*;
#[test]
fn negative_context_other_week_changed() {
    let original = ContextStamp {
        household_revision: Revision(1),
        settings_revision: Revision(1),
        privacy_revision: Revision(1),
        history_revision: Revision(1),
        catalogue_revision: Revision(1),
        ranking_policy_version: Revision(1),
        recipe_refs: BoundedVec(vec![]),
        read_weeks: BoundedVec(vec![WeekRevision {
            week_id: WeekId("wk_00000000000000000000000000000004".into()),
            revision: Revision(1),
        }]),
        write_weeks: BoundedVec(vec![]),
    };
    let mut changed = original.clone();
    changed.read_weeks.0[0].revision = Revision(2);
    assert_eq!(
        compare_context(&original, &changed).unwrap_err().code,
        ErrorCode::ContextStale
    );
    assert_eq!(original.read_weeks[0].revision, Revision(1));
}
use serde_json::Value;
fn decode<T: serde::de::DeserializeOwned + ValidateShape>(v: &Value) -> T {
    parse_document(&serde_json::to_vec(v).unwrap()).unwrap()
}
#[test]
fn disabled_selection_projection_preserves_canonical_plan() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/contracts/ruling004-current--week-disabled-retained.json");
    let mut week: Week = parse_document(&std::fs::read(path).unwrap()).unwrap();
    let retained = week.plan.clone();
    assert!(!retained.selections.is_empty());
    assert!(active_selections(&week).is_empty());
    week.meal_type_enabled = MealTypeEnabled {
        breakfast: true,
        lunch: true,
        dinner: true,
    };
    assert!(active_selections(&week).is_empty());
    week.day_enabled = BoundedVec(vec![true; 7]);
    assert_eq!(active_selections(&week).len(), retained.selections.len());
    assert_eq!(week.plan, retained);
    let slot = retained.selections[0].slot.clone();
    week.cells
        .0
        .iter_mut()
        .find(|c| c.slot == slot)
        .unwrap()
        .choice = MealLocation::Skip;
    assert_eq!(
        active_selections(&week).len(),
        retained.selections.len() - 1
    );
    assert_eq!(week.plan, retained);
}
#[test]
fn semantic_manifest_cases() {
    let base =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/contracts");
    let m: Value = serde_json::from_slice(
        &std::fs::read(base.join("ruling005-current-manifest.json")).unwrap(),
    )
    .unwrap();
    let mut executed = 0;
    let mut failures = Vec::new();
    for c in m["cases"].as_array().unwrap() {
        let stage = c["stage"].as_str().unwrap();
        if ![
            "binding",
            "privacy",
            "context_compare",
            "arithmetic",
            "conversion",
            "essentials",
            "command",
            "canonical",
            "primitive",
        ]
        .contains(&stage)
        {
            continue;
        }
        let v: Value =
            serde_json::from_slice(&std::fs::read(base.join(c["file"].as_str().unwrap())).unwrap())
                .unwrap();
        let result = match stage {
            "canonical" => {
                assert_eq!(
                    canonical_bytes(&v["input"]).unwrap(),
                    v["expected_canonical"].as_str().unwrap().as_bytes()
                );
                Ok(())
            }
            "primitive" => match v["type"].as_str().unwrap() {
                "zoned_moment" => {
                    parse_document::<ZonedMoment>(&serde_json::to_vec(&v["value"]).unwrap())
                        .map(|_| ())
                }
                "nonnegative_quantity" => {
                    parse_document::<NonnegativeQuantity>(&serde_json::to_vec(&v["value"]).unwrap())
                        .map(|_| ())
                }
                _ => panic!("unknown primitive"),
            },
            "binding" => {
                let r: JobRequestV1 = decode(&v["request"]);
                let out: JobResultV1 = serde_json::from_value(v["result"].clone()).unwrap();
                let current: ContextStamp = serde_json::from_value(v["current"].clone()).unwrap();
                validate_result_binding(&out, &r, &current)
            }
            "context_compare" => {
                let a: ContextStamp = decode(&v["expected"]);
                let b: ContextStamp = serde_json::from_value(v["current"].clone()).unwrap();
                compare_context(&a, &b)
            }
            "privacy" => {
                let r: JobRequestV1 = serde_json::from_value(v["request"].clone()).unwrap();
                let policy: PrivacyPolicy = decode(&v["policy"]);
                let artefacts = v["artefacts"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|a| AllowedArtefactBytes {
                        artefact_id: serde_json::from_value(a["artefact_id"].clone()).unwrap(),
                        bytes: a["text"].as_str().unwrap().as_bytes().to_vec(),
                    })
                    .collect();
                validate_outbound(
                    &OutboundBundle {
                        request: r,
                        artefacts,
                    },
                    &policy,
                )
            }
            "arithmetic" => {
                let a: Rational = decode(&v["left"]);
                let b: Rational = decode(&v["right"]);
                if v["operator"] == "add" {
                    a.checked_add(&b).map(|_| ())
                } else {
                    a.checked_mul(&b).map(|_| ())
                }
            }
            "conversion" => {
                let evidence: ConversionEvidence = decode(&v["evidence"]);
                validate_conversion(
                    &evidence,
                    &decode(&v["ingredient_id"]),
                    decode(&v["from_unit"]),
                    decode(&v["from_form"]),
                    decode(&v["to_unit"]),
                    decode(&v["to_form"]),
                )
            }
            "essentials" => {
                let demand: DemandSet = serde_json::from_value(v["demand"].clone()).unwrap();
                let confirmation: EssentialsConfirmation = decode(&v["confirmation"]);
                validate_essentials_binding(&confirmation, &demand)
            }
            "command" => {
                let caller = match v["caller"].as_str().unwrap() {
                    "local_ui" => CallerClass::LocalUi,
                    "exchange_import" => CallerClass::ExchangeImport,
                    "qualified_adapter" => CallerClass::QualifiedAdapter,
                    _ => panic!("unknown caller"),
                };
                let cmd: AppCommand = serde_json::from_value(v["command"].clone()).unwrap();
                authorize_command(cmd.kind(), caller)
            }
            _ => unreachable!(),
        };
        let actual = result
            .as_ref()
            .err()
            .map(|e| serde_json::to_value(e.code).unwrap())
            .unwrap_or(Value::String("ok".into()));
        if actual != c["expected"] {
            failures.push(format!(
                "{}/{}: wanted {}, got {result:?}",
                c["id"], c["subcase"], c["expected"]
            ));
        }
        executed += 1;
    }
    println!("semantic manifest subcases={executed}");
    assert!(executed > 0, "empty semantic suite");
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
