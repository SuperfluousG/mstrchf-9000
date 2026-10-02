// The exact-case runner reuses the focused targets and their synthetic fixture.
#![allow(clippy::duplicate_mod)]
use mstrchf_contracts::*;
use mstrchf_core::food::*;
#[path = "../../../tests/fixtures/food/native/support.rs"]
mod native;
use native::{read, value};
#[path = "../../../tests/fixtures/food/batches/f11_fixture.rs"]
mod f11;
mod a_cases {
    include!("food_admission.rs");
    pub fn run() {
        food_f52_incomplete_candidate();
        food_f52_strict_import_claims_reject();
        food_f52_automated_cook_test_rejects();
    }
}
mod b_cases {
    include!("food_portions_eligibility.rs");
    pub fn run(n: u8) {
        match n {
            1 => food_f01_exact_rational_recipe_specific_presets(),
            2 => food_f02_same_child_other_recipe(),
            3 => food_f03_missing_age_recipe_mapping(),
            4 => food_f04_meal_override_is_local(),
            5 => food_f05_shared_base_and_separate_extra(),
            6 => food_f06_extra_to_ineligible_person(),
            7 => food_f07_guest_blocks_previous_base(),
            8 => food_f08_portable_cold_control_and_no_fit(),
            9 => food_f09_no_suitable_recipe_both_routes(),
            37 => food_f37_fractional_egg_without_supported_scaling(),
            38 => food_f38_discrete_batch_scaling_control(),
            39 => food_f39_dry_cooked_without_conversion(),
            _ => panic!("not a B-owned oracle case"),
        }
    }
}
#[path = "food_demand.rs"]
mod demand_cases;
#[path = "food_prep.rs"]
mod prep_cases;

#[test]
fn native_slot_order_uses_meal_rank_not_json_field_order() {
    let a = SlotId {
        week_id: WeekId(format!("wk_{:032x}", 1)),
        date: LocalDate("2026-10-05".into()),
        meal_type: MealType::Lunch,
    };
    let mut b = a.clone();
    b.meal_type = MealType::Dinner;
    assert!(slot_cmp(&a, &b).is_lt());
}

#[test]
fn native_v4_independent_full_typed_hashes() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/food/native/v4");
    let manifest_bytes = std::fs::read(dir.join("manifest-v4.json")).unwrap();
    assert_eq!(
        bytes_digest(&manifest_bytes).0,
        "ba41e5af3fab492d243696082f4826b165a5f72fe0dcb1069f71abe4b53ebede"
    );
    let manifest: serde_json::Value = serde_json::from_slice(&manifest_bytes).unwrap();
    let mut count = 0;
    for row in manifest["vectors"].as_array().unwrap() {
        let id = row["id"].as_str().unwrap();
        let v = value(id);
        let expected = row["sha256"].as_str().unwrap();
        let bytes = std::fs::read(dir.join(row["preimage"].as_str().unwrap())).unwrap();
        assert_eq!(bytes_digest(&bytes).0, expected, "preserved preimage {id}");
        let result = match row["type"].as_str().unwrap() {
            "FoodEvidence" => food_evidence_digest(&read(&v)),
            "FoodIdentities" => food_identities_digest(&read(&v)),
            "OccupiedOwner" => occupied_owner_digest(&read(&v)),
            "OccupancySnapshot" => occupancy_snapshot_digest(&read(&v)),
            "ProductUseSnapshot" => digest_product_uses(&read(&v)),
            "FeasibilityInputs" => digest_feasibility_inputs(&read(&v)),
            "FeasibilityReport" => digest_feasibility_report(&read(&v)),
            "SavedFoodBasis" => saved_basis_digest(&read(&v)),
            "FoodInputsMaterial" => food_inputs_digest(
                &read(&v["input_plan_digest"]),
                &read(&v["evidence"]),
                &read(&v["identities"]),
                &read(&v["schedule"]),
            ),
            "LiveRequirements" => live_requirements_digest(&read(&v)),
            "RelevantEvidenceSnapshot" => relevant_evidence_digest(&read(&v)),
            "LiveFoodPolicy" => live_food_policy_digest(&read(&v)),
            "ContinuationInputs" => continuation_inputs_digest(&read(&v)),
            "ContinuationReport" => continuation_report_digest(&read(&v)),
            "LiveSourceMaterial" => {
                let pin = LiveSourcePin {
                    source: read(&v["source"]),
                    revision: read(&v["revision"]),
                    digest: PayloadDigest(expected.into()),
                    valid_until: read(&v["valid_until"]),
                };
                if pin.source == LiveSource::Order {
                    order_source_digest(&pin, &read(&v["values"]))
                } else {
                    essentials_source_digest(
                        &pin,
                        &read::<Vec<EssentialsConfirmation>>(&v["values"]),
                    )
                }
            }
            _ => continue,
        };
        assert_eq!(
            result
                .unwrap_or_else(|e| panic!("native vector {id}: {e:?}"))
                .0,
            expected,
            "typed material {id}"
        );
        count += 1;
    }
    println!(
        "{count} full typed native known-answer hashes; every one of 138 original preimages retained"
    );
    assert!(count > 100);
}

#[test]
fn native_v4_saved_core_replays_with_real_a_b_evidence() {
    for prefix in ["prior", "subject"] {
        let saved = native::basis(&format!("{prefix}-saved-basis"));
        let mut material = value(&format!("{prefix}-derived"));
        let manifest: serde_json::Value = serde_json::from_slice(include_bytes!(
            "../../../tests/fixtures/food/native/v4/manifest-v4.json"
        ))
        .unwrap();
        material["digest"] = manifest["vectors"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["id"] == format!("{prefix}-derived"))
            .unwrap()["sha256"]
            .clone();
        let expected: DerivedPlan = read(&material);
        let input = FeasibilityInputs {
            revision: Revision(1),
            digest: PayloadDigest("0".repeat(64)),
            delivery: DeliveryInput::Unselected,
            products: ProductUseSnapshot {
                revision: Revision(1),
                digest: PayloadDigest("0".repeat(64)),
                valid_until: None,
                evidence: vec![],
                bindings: vec![],
                facts: vec![],
            },
            confirmations: vec![],
            evaluated_at: Instant("2026-12-28T12:00:00Z".into()),
        };
        let mut input = input;
        input.products.digest = digest_product_uses(&input.products).unwrap();
        input.digest = digest_feasibility_inputs(&input).unwrap();
        let result = derive_plan(
            &saved.plan,
            &saved.context,
            &FoodInputs {
                evidence: &saved.evidence,
                identities: &saved.identities,
                schedule: &saved.schedule,
                feasibility: &input,
            },
        )
        .unwrap_or_else(|e| panic!("{prefix}: {e:?}"));
        assert_eq!(result.plan, expected, "native derivation {prefix}");
    }
}

#[test]
fn native_v4_nine_full_continuation_reports_replay_independent_literals() {
    let historical = native::historical();
    let checked = verify_historical(&historical).unwrap();
    for prefix in [
        "known",
        "unknown",
        "lower-exclusive",
        "opening-late",
        "upper-inclusive",
        "upper-exclusive",
        "planned",
        "soft-preference",
        "unrelated-catalogue",
    ] {
        let inputs: ContinuationInputs = read(&value(&format!("{prefix}-continuation-inputs")));
        let expected: ContinuationReport = read(&value(&format!("{prefix}-continuation-report")));
        let result =
            assess_continuation(&checked, &inputs).unwrap_or_else(|e| panic!("{prefix}: {e:?}"));
        assert_eq!(result, expected, "complete continuation {prefix}");
        assert_eq!(historical.order, native::historical().order);
    }
}

#[test]
fn historical_tamper_and_seven_fresh_feed_closure_fail_closed() {
    let mut h = native::historical();
    h.basis.schedule.layout.routes[0].phase.prep_id = 2;
    assert!(verify_historical(&h).is_err());
    let h = native::historical();
    let checked = verify_historical(&h).unwrap();
    let c: ContinuationInputs = read(&value("known-continuation-inputs"));
    let mut bad = c.clone();
    bad.sources.remove(0);
    assert!(assess_continuation(&checked, &bad).is_err());
    let mut bad = c.clone();
    bad.sources.swap(0, 1);
    assert!(assess_continuation(&checked, &bad).is_err());
    let mut bad = c.clone();
    bad.sources[0].revision = Revision(99);
    assert!(assess_continuation(&checked, &bad).is_err());
    let mut bad = c.clone();
    bad.sources[3].digest = PayloadDigest("1".repeat(64));
    bad.digest = continuation_inputs_digest(&bad).unwrap();
    assert!(assess_continuation(&checked, &bad).is_err());
    let mut bad = c.clone();
    bad.sources[1].valid_until = Some(bad.feasibility.evaluated_at.clone());
    assert!(continuation_inputs_digest(&bad).is_err());
}

fn repin(c: &mut ContinuationInputs) {
    c.requirements.digest = live_requirements_digest(&c.requirements).unwrap();
    c.sources[6].digest = c.requirements.digest.clone();
    c.evidence.digest = relevant_evidence_digest(&c.evidence).unwrap();
    c.sources[5].digest = c.evidence.digest.clone();
    c.policy.digest = live_food_policy_digest(&c.policy).unwrap();
    c.sources[1].digest = c.policy.digest.clone();
    c.occupied.digest = occupancy_snapshot_digest(&c.occupied).unwrap();
    c.sources[2].digest = c.occupied.digest.clone();
    c.digest = continuation_inputs_digest(c).unwrap();
}
#[test]
fn continuation_rejects_material_changes_without_rewriting_order() {
    let h = native::historical();
    let checked = verify_historical(&h).unwrap();
    let original = h.clone();
    let base: ContinuationInputs = read(&value("known-continuation-inputs"));
    let mut changes = vec![];
    let mut c = base.clone();
    c.requirements.meals[0].enabled = false;
    c.requirements.meals[0].eat_by = Nullable(None);
    repin(&mut c);
    changes.push(c);
    let mut c = base.clone();
    c.requirements.meals[0].equipment.0.clear();
    repin(&mut c);
    changes.push(c);
    let mut c = base.clone();
    c.policy.maximum_reheats = 0;
    repin(&mut c);
    changes.push(c);
    let mut c = base.clone();
    c.policy.general_seconds = 60;
    c.policy.rice_seconds = 60;
    repin(&mut c);
    changes.push(c);
    let mut c = base.clone();
    if let LiveRecipeState::Admitted { methods, .. } = &mut c.evidence.recipes[0] {
        methods
            .iter_mut()
            .find(|m| m.kind == MethodKind::Reuse)
            .unwrap()
            .direct_reuse_counts[0]
            .planned_reheats = 0;
    }
    repin(&mut c);
    changes.push(c);
    let mut c = base.clone();
    if let LiveRecipeState::Admitted { basis, .. } = &c.evidence.recipes[0] {
        c.evidence.recipes[0] = LiveRecipeState::Withdrawn {
            recipe: basis.recipe.clone(),
            evidence: basis.admitted.clone(),
        };
    }
    repin(&mut c);
    changes.push(c);
    for (i, c) in changes.iter().enumerate() {
        let result = assess_continuation(&checked, c).unwrap();
        assert!(!result.blocking.is_empty(), "material mutation {i}");
        assert_eq!(h, original);
    }
}

#[test]
fn native_canonical_order_and_duplicate_identities_reject_before_hashing() {
    let h = native::historical();
    let mut identities = h.basis.identities.clone();
    identities.allocations.swap(0, 1);
    assert_eq!(
        food_identities_digest(&identities).unwrap_err().code,
        ErrorCode::NoncanonicalValue
    );
    let mut identities = h.basis.identities.clone();
    identities.allocations[1].1 = identities.allocations[0].1.clone();
    assert_eq!(
        food_identities_digest(&identities).unwrap_err().code,
        ErrorCode::InvalidValue
    );
    let mut e = h.basis.evidence.clone();
    e.methods.push(e.methods[1].clone());
    assert_eq!(
        food_evidence_digest(&e).unwrap_err().code,
        ErrorCode::InvalidValue
    );
    let mut e = h.basis.evidence.clone();
    e.methods[0].direct_reuse_counts = e.methods[1].direct_reuse_counts.clone();
    assert_eq!(
        food_evidence_digest(&e).unwrap_err().code,
        ErrorCode::InvalidValue
    );
    let mut e = h.basis.evidence.clone();
    e.methods[1].direct_reuse_counts[0].planned_reheats = 2;
    assert_eq!(
        food_evidence_digest(&e).unwrap_err().code,
        ErrorCode::InvalidValue
    );
}

#[allow(clippy::result_large_err)] // R3 requires the full by-value failure report.
fn run_basis(s: &SavedFoodBasis) -> Result<FoodDerivation, FoodFailure> {
    let f = f11::feasibility();
    derive_plan(
        &s.plan,
        &s.context,
        &FoodInputs {
            evidence: &s.evidence,
            identities: &s.identities,
            schedule: &s.schedule,
            feasibility: &f,
        },
    )
}
fn refresh_basis_context(s: &mut SavedFoodBasis) {
    s.context.weeks[0].plan = s.plan.clone();
    s.context.requirements = BoundedVec(
        mstrchf_core::availability::meal_requirements(
            &s.context.household,
            &s.context.weeks[0],
            &[],
        )
        .unwrap()
        .into(),
    );
}
fn oracle_moment(v: &serde_json::Value) -> ZonedMoment {
    let t = chrono::DateTime::parse_from_rfc3339(v.as_str().unwrap()).unwrap();
    let date = LocalDate(t.format("%Y-%m-%d").to_string());
    let local = LocalTime(t.format("%H:%M:%S").to_string());
    ZonedMoment::new(
        date,
        local,
        ZoneId("Europe/London".into()),
        Resolution::Unambiguous,
        Instant(
            t.with_timezone(&chrono::Utc)
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string(),
        ),
    )
    .unwrap()
}
fn batch_case(n: u8, row: &serde_json::Value) {
    let mut s = f11::fixture();
    let input = &row["input"];
    let expected = &row["expected"];
    match n {
        11 => {
            let d = run_basis(&s).unwrap();
            assert_eq!(
                d.plan.batches.len(),
                expected["original_batch_count"].as_u64().unwrap() as usize
            );
            let c = &d.plan.batches[0].components[0];
            assert_eq!(
                c.allocated.amount.0.numerator.0,
                expected["allocated"]["amount"]
                    .as_str()
                    .unwrap()
                    .parse::<u64>()
                    .unwrap()
            );
            assert_eq!(c.unallocated.amount, Rational::new(0, 1).unwrap());
            assert_eq!(
                d.plan
                    .allocations
                    .iter()
                    .map(|a| a.quantity.amount.0.numerator.0)
                    .collect::<Vec<_>>(),
                vec![600, 300, 300]
            );
            for (i, name) in ["beans", "tomatoes", "onion", "oil", "wrap", "potato"]
                .iter()
                .enumerate()
            {
                let q = &d.plan.demand.lines[i].quantity;
                assert_eq!(serde_json::json!(q.unit), expected["demand"][name]["unit"]);
                assert_eq!(serde_json::json!(q.form), expected["demand"][name]["form"]);
                assert_eq!(
                    q.amount.0.numerator.0,
                    expected["demand"][name]["amount"]
                        .as_str()
                        .unwrap()
                        .parse::<u64>()
                        .unwrap()
                );
            }
            let original = match &c.original_production {
                ProductionMoment::Cooked { completed_at } => completed_at,
                _ => panic!(),
            };
            assert_eq!(original, &f11::at(5, "18:00:00"));
            for (i, a) in d.plan.allocations.iter().skip(1).enumerate() {
                assert_eq!(
                    a.use_at.instant.timestamp_seconds() - original.instant.timestamp_seconds(),
                    expected["leftover_ages_seconds"][i].as_i64().unwrap()
                );
            }
            assert_eq!(run_basis(&s).unwrap(), d);
        }
        12 => {
            s.plan.selections[2].servings[0].servings = PositiveRational::new(3, 2).unwrap();
            s.plan.selections[2].meal_overrides.portions =
                BoundedVec(vec![PersonPortionOverride {
                    person: s.plan.selections[2].servings[0].person.clone(),
                    preset: s.plan.selections[2].servings[0].preset_ref.preset,
                    adjustment: PositiveRational::new(3, 2).unwrap(),
                }]);
            s.plan.batch_links[1].components[0].quantity.amount =
                PositiveRational::new(450, 1).unwrap();
            refresh_basis_context(&mut s);
            assert!(run_basis(&s).is_err());
            assert!(
                matches!(s.plan.selections[0].batch_use,BatchUse::NewBatch{production_servings,..} if production_servings==PositiveRational::new(4,1).unwrap())
            );
        }
        13 => {
            let a = match &s.plan.selections[0].batch_use {
                BatchUse::NewBatch { batch_id, .. } => batch_id.clone(),
                _ => panic!(),
            };
            let b = BatchId(format!("ba_{:032x}", 999));
            let component = s.plan.batch_links[0].components[0].component_id.clone();
            s.plan.selections[0].batch_use = BatchUse::Leftover {
                batch_id: b.clone(),
                component_ids: BoundedVec(vec![component]),
            };
            s.plan.batch_links.0.push(BatchLink {
                batch_id: b,
                source_slot: s.plan.selections[1].slot.clone(),
                target_slot: s.plan.selections[0].slot.clone(),
                components: s.plan.batch_links[0].components.clone(),
            });
            assert!(
                matches!(&s.plan.selections[1].batch_use,BatchUse::Leftover{batch_id,..} if batch_id==&a)
            );
            assert!(run_basis(&s).is_err());
        }
        14 => {
            s.context.weeks[0].day_enabled[0] = false;
            refresh_basis_context(&mut s);
            assert!(
                !s.context
                    .requirements
                    .iter()
                    .find(|m| m.slot == s.plan.selections[0].slot)
                    .unwrap()
                    .enabled
            );
            assert!(run_basis(&s).is_err());
        }
        15..=20 => {
            let mut rule = s.context.recipes[0].storage[0].clone();
            rule.max_chilled_seconds = input["source_limit_seconds"].as_u64().unwrap() as u32;
            let limit = if input["component"] == "rice" {
                86400
            } else {
                172800
            };
            let start = oracle_moment(&input["original_cook"]);
            let end = oracle_moment(&input["use_at"]);
            let result = mstrchf_core::batches::check_reuse(
                &start,
                &end,
                &rule,
                limit,
                input["planned_reheats"].as_u64().unwrap() as u8,
            );
            assert_eq!(result.is_ok(), expected["outcome"] == "valid");
            assert_eq!(
                end.instant.timestamp_seconds() - start.instant.timestamp_seconds(),
                expected["age_seconds"].as_i64().unwrap()
            );
        }
        21 => {
            let rule = &s.context.recipes[0].storage[0];
            let start = oracle_moment(&input["original_cook"]);
            let end = oracle_moment(&input["use_at"]);
            assert!(mstrchf_core::batches::check_reuse(&start, &end, rule, 172800, 1).is_ok());
            assert!(mstrchf_core::batches::check_reuse(&start, &end, rule, 86400, 1).is_err());
        }
        22 => {
            let rule = &s.context.recipes[0].storage[0];
            let start = oracle_moment(&input["original_cook"]);
            let end = oracle_moment(&input["use_at"]);
            assert!(mstrchf_core::batches::check_reuse(&start, &end, rule, 172800, 1).is_err());
            assert_eq!(
                end.instant.timestamp_seconds() - start.instant.timestamp_seconds(),
                expected["true_age_seconds"].as_i64().unwrap()
            );
        }
        23 => {
            let mut d = run_basis(&s).unwrap();
            d.plan.allocations[1].planned_reheats = 2;
            assert!(
                mstrchf_core::batches::validate_allocations(&d.plan.batches, &d.plan.allocations)
                    .is_err()
            );
        }
        24 => {
            let mut rule = s.context.recipes[0].storage[0].clone();
            rule.max_reheats = 0;
            let start = oracle_moment(&input["original_cook"]);
            let end = oracle_moment(&input["use_at"]);
            assert!(mstrchf_core::batches::check_reuse(&start, &end, &rule, 172800, 0).is_ok());
            assert!(mstrchf_core::batches::check_reuse(&start, &end, &rule, 172800, 1).is_err());
        }
        _ => unreachable!(),
    }
}

#[test]
fn food_oracle_exact_38_owned_cases() {
    let bytes = include_bytes!("../../../tests/fixtures/food/oracle-v1.json");
    assert_eq!(
        bytes_digest(bytes).0,
        "ba50ac3ff55ea90aae006d2883cfe21ab1c9667b5ccfdd185f2e967263c8b092"
    );
    let all: serde_json::Value = serde_json::from_slice(bytes).unwrap();
    assert_eq!(all["fixtures"].as_array().unwrap().len(), 55);
    let ids: Vec<u8> = (1..=9)
        .chain(11..=34)
        .chain(37..=39)
        .chain([52, 54])
        .collect();
    assert_eq!(ids.len(), 38);
    for n in ids {
        let prefix = format!("F{n:02}_");
        let row = all["fixtures"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"].as_str().unwrap().starts_with(&prefix))
            .unwrap();
        match n {
            1..=9 | 37..=39 => b_cases::run(n),
            11..=24 => batch_case(n, row),
            25..=29 | 54 => prep_cases::run_oracle_case(n),
            30..=34 => demand_cases::run_oracle_case(n),
            52 => a_cases::run(),
            _ => unreachable!(),
        };
        println!(
            "ORACLE {} {}",
            row["name"].as_str().unwrap(),
            row["expected"]["outcome"].as_str().unwrap()
        );
    }
    println!(
        "38 P04 cases executed. P08/P02/P09-owned end-to-end cases NOT RUN. Derivation F11/F12/F14/F25/F28; timeline/attachment predicates F26/F27; reuse predicates F15-F22/F24; forged-output validator F23; frozen A/B helper cases and shopping projection cases as labelled."
    );
}

#[test]
fn native_v4_ten_negative_fragments_and_two_historical_context_controls() {
    let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/food/native/v4");
    let index: serde_json::Value =
        serde_json::from_slice(&std::fs::read(dir.join("negative-index.json")).unwrap()).unwrap();
    assert_eq!(index.as_array().unwrap().len(), 10);
    let h = native::historical();
    let checked = verify_historical(&h).unwrap();
    for row in index.as_array().unwrap() {
        let id = row["id"].as_str().unwrap();
        let raw = std::fs::read(dir.join(format!("{id}.raw.json"))).unwrap();
        assert_eq!(bytes_digest(&raw).0, row["raw_sha256"].as_str().unwrap());
        let v: serde_json::Value = serde_json::from_slice(&raw).unwrap();
        let result = match id {
            "negative-source-permutation" | "negative-source-duplicate" => {
                let mut c: ContinuationInputs = read(&value("known-continuation-inputs"));
                c.sources = read(&v);
                continuation_inputs_digest(&c)
            }
            "negative-tuple-permutation" | "negative-tuple-duplicate" => {
                let mut i = h.basis.identities.clone();
                i.sessions = read(&v);
                food_identities_digest(&i)
            }
            "negative-phase-permutation" | "negative-phase-duplicate" => {
                let b: Vec<TaskBinding> = read(&v);
                let mut s = h.basis.schedule.clone();
                s.layout.routes = b
                    .into_iter()
                    .map(|b| PhaseRoute {
                        phase: b.phase,
                        session: s.layout.routes[0].session.clone(),
                    })
                    .collect();
                food_inputs_digest(
                    &h.derived.input_plan_digest,
                    &h.basis.evidence,
                    &h.basis.identities,
                    &s,
                )
            }
            "negative-recipe-shared-id-digest" => {
                let refs: Vec<RecipeRef> = read(&v);
                let mut e = h.basis.evidence.clone();
                e.recipes = refs
                    .iter()
                    .map(|r| {
                        let mut b = e.recipes[0].clone();
                        b.recipe = r.clone();
                        b
                    })
                    .collect();
                e.classes = refs
                    .iter()
                    .map(|r| {
                        let mut c = e.classes[0].clone();
                        c.recipe = r.clone();
                        for f in &mut c.components {
                            f.recipe = r.clone();
                        }
                        c
                    })
                    .collect();
                food_evidence_digest(&e)
            }
            "negative-evidence-shared-id-digest" => {
                let mut e = h.basis.evidence.clone();
                e.recipes[0].admitted = read(&v);
                food_evidence_digest(&e)
            }
            "negative-nested-owner-digest" => continuation_inputs_digest(&read(&v)),
            "negative-current-method-substitution" => {
                let mut c: ContinuationInputs = read(&value("known-continuation-inputs"));
                c.evidence.recipes[0] = read(&v);
                repin(&mut c);
                assert!(
                    !assess_continuation(&checked, &c)
                        .unwrap()
                        .blocking
                        .is_empty()
                );
                continue;
            }
            _ => panic!("uncovered negative {id}"),
        };
        let error = result.unwrap_err();
        let expected = match row["expected_rejection_class"].as_str().unwrap() {
            "invalid_value" => ErrorCode::InvalidValue,
            "noncanonical_value" => ErrorCode::NoncanonicalValue,
            "reference_conflict" => ErrorCode::IdentityMismatch,
            "digest_mismatch" => ErrorCode::DigestMismatch,
            x => panic!("{x}"),
        };
        assert_eq!(error.code, expected, "negative {id}");
    }
    for prefix in ["prior", "subject"] {
        let bytes =
            std::fs::read(dir.join(format!("{prefix}-historical-active-only-negative.json")))
                .unwrap();
        let v: serde_json::Value = serde_json::from_slice(&bytes).unwrap();
        let context: PlanningContext = read(&v["value"]);
        assert!(context.validate_shape().is_err());
    }
}
