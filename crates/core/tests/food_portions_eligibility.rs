use mstrchf_contracts::*;
use mstrchf_core::{eligibility::*, portions::*, recipes::*};

fn p(n: u64, d: u64) -> PositiveRational {
    PositiveRational::new(n, d).unwrap()
}
fn eid(n: u32) -> EvidenceId {
    EvidenceId(format!("ev_{n:032x}"))
}
fn reference(r: &RecipeRevision) -> RecipeRef {
    RecipeRef {
        recipe_id: r.recipe_id.clone(),
        revision: r.revision,
        digest: recipe_digest(r).unwrap(),
    }
}
fn recipe() -> RecipeRevision {
    let mut r: RecipeRevision = parse_document(include_bytes!(
        "../../../tests/fixtures/food/admission/complete-synthetic.json"
    ))
    .unwrap();
    r.scaling = ScalingPolicy::Linear {
        minimum: p(1, 100),
        maximum: p(100, 1),
    };
    r.portability.evidence_ids = BoundedVec(vec![eid(8)]);
    r
}
fn inputs(r: &RecipeRevision) -> (EvidenceBasis, ComponentClassView) {
    let rf = reference(r);
    (
        EvidenceBasis::new(
            rf.clone(),
            (1..=32)
                .map(|n| EvidencePin {
                    evidence_id: eid(n),
                    revision: Revision(1),
                    digest: Digest(format!("{n:064x}")),
                })
                .collect(),
            vec![AssetDigest("a".repeat(64))],
        )
        .unwrap(),
        ComponentClassView::new(
            rf.clone(),
            vec![ComponentClassEvidence {
                recipe: rf,
                component_id: r.components[0].component_id.clone(),
                class: ComponentClass::General,
                evidence_ids: vec![eid(8)],
            }],
        )
        .unwrap(),
    )
}
fn meal() -> MealRequirements {
    let mut m = parse_document::<MealRequirementsDocumentV1>(include_bytes!(
        "../../../tests/fixtures/contracts/meal-requirements.json"
    ))
    .unwrap()
    .meal_requirements;
    m.attendees.0.truncate(1);
    m.shared_hard_requirements = m.attendees[0].hard_requirements.clone();
    m
}
fn no_overrides() -> MealOverrides {
    MealOverrides {
        portions: BoundedVec(vec![]),
        eat_by: Nullable(None),
    }
}
fn preset(
    r: &mut RecipeRevision,
    age: AgeBand,
    meal_type: MealType,
    name: PortionPresetName,
    servings: PositiveRational,
) {
    r.portion_presets.0.push(PortionPreset {
        preset: name,
        age_band: age,
        meal_type,
        servings,
        evidence_ids: BoundedVec(vec![eid(5)]),
    });
}
fn person(n: u32) -> PersonRef {
    PersonRef::Diner {
        diner_id: DinerId(format!("dn_{n:032x}")),
    }
}
fn amount(r: &mut RecipeRevision, n: u64, d: u64, unit: Unit, form: IngredientForm) {
    r.ingredients[0].quantity = Nullable(Some(Quantity {
        amount: p(n, d),
        unit,
        form,
    }));
}
fn oracle(prefix: &str) -> serde_json::Value {
    let o: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/food/oracle-v1.json"
    ))
    .unwrap();
    let cases = o["fixtures"].as_array().unwrap();
    assert_eq!(cases.len(), 55);
    let found: Vec<_> = cases
        .iter()
        .filter(|x| x["name"].as_str().unwrap().starts_with(prefix))
        .collect();
    assert_eq!(found.len(), 1);
    let manifest: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/food/portions-eligibility/cases.json"
    ))
    .unwrap();
    assert_eq!(
        manifest["cases"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|x| x["name"] == found[0]["name"])
            .count(),
        1
    );
    println!("{}", found[0]["name"]);
    found[0].clone()
}
#[test]
fn food_oracle_case_inventory() {
    let manifest: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/food/portions-eligibility/cases.json"
    ))
    .unwrap();
    let cases = manifest["cases"].as_array().unwrap();
    assert_eq!(cases.len(), 13);
    let names: std::collections::BTreeSet<_> =
        cases.iter().map(|x| x["name"].as_str().unwrap()).collect();
    assert_eq!(names.len(), 13);
    let tests: std::collections::BTreeSet<_> =
        cases.iter().map(|x| x["test"].as_str().unwrap()).collect();
    assert_eq!(tests.len(), 13);
    for prefix in [
        "F01_", "F02_", "F03_", "F04_", "F05_", "F06_", "F07_", "F08_", "F09_", "F37_", "F38_",
        "F39_", "F40_",
    ] {
        let case = oracle(prefix);
        assert!(names.contains(case["name"].as_str().unwrap()));
    }
    println!("B coverage: 13 unique oracle case groups; F40 is conversion-only, not pack pricing");
}
fn expect_amount(actual: PositiveRational, n: u64, d: u64) {
    assert_eq!(actual, p(n, d));
}

#[test]
fn food_f01_exact_rational_recipe_specific_presets() {
    let o = oracle("F01_");
    assert_eq!(o["expected"]["beans"]["amount"], "210");
    let mut r = recipe();
    r.r#yield.servings = p(3, 1);
    amount(&mut r, 360, 1, Unit::G, IngredientForm::Drained);
    r.scaling = ScalingPolicy::Linear {
        minimum: p(1, 4),
        maximum: p(2, 1),
    };
    preset(
        &mut r,
        AgeBand::Child7To10,
        MealType::Dinner,
        PortionPresetName::Regular,
        p(2, 3),
    );
    let mut m = meal();
    m.attendees[0].portion.adjustment = p(5, 4);
    let mut child = m.attendees[0].clone();
    child.person = person(3);
    child.age_band = AgeBand::Child7To10;
    child.portion.adjustment = p(3, 4);
    m.attendees.0.push(child);
    let before = (r.clone(), m.clone());
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    let s = resolve_servings(&rr, &m, &no_overrides()).unwrap();
    expect_amount(s[0].servings, 5, 4);
    expect_amount(s[1].servings, 1, 2);
    let total = PositiveRational(s[0].servings.0.checked_add(&s[1].servings.0).unwrap());
    expect_amount(total, 7, 4);
    let scaled = scale_recipe(&rr, total).unwrap();
    expect_amount(scaled.components[0].quantity.amount, 700, 3);
    assert_eq!(scaled.unallocated_servings, Rational::new(0, 1).unwrap());
    assert_eq!(scaled.discrete_batches, None);
    expect_amount(scaled.multiplier, 7, 12);
    expect_amount(scaled.ingredients[0].quantity.amount, 210, 1);
    assert_eq!(scaled.ingredients[0].quantity.form, IngredientForm::Drained);
    assert_eq!(s[0].preset_ref.recipe, reference(&r));
    assert_eq!((r, m), before);
}
#[test]
fn food_f02_same_child_other_recipe() {
    let o = oracle("F02_");
    assert_eq!(o["expected"]["servings"], "9/16");
    let mut r = recipe();
    preset(
        &mut r,
        AgeBand::Child7To10,
        MealType::Dinner,
        PortionPresetName::Regular,
        p(3, 4),
    );
    let mut m = meal();
    m.attendees[0].age_band = AgeBand::Child7To10;
    m.attendees[0].portion.adjustment = p(3, 4);
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    expect_amount(
        resolve_servings(&rr, &m, &no_overrides()).unwrap()[0].servings,
        9,
        16,
    );
}
#[test]
fn food_f03_missing_age_recipe_mapping() {
    let o = oracle("F03_");
    assert!(o["expected"]["servings"].is_null());
    let r = recipe();
    let mut m = meal();
    m.attendees[0].age_band = AgeBand::Child4To6;
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    let e = resolve_servings(&rr, &m, &no_overrides()).unwrap_err();
    assert_eq!(e[0].code, ErrorCode::UnsupportedCapability);
}
#[test]
fn food_f04_meal_override_is_local() {
    let o = oracle("F04_");
    assert_eq!(o["expected"]["a_grams"]["breakfast"], "120");
    let mut r = recipe();
    r.r#yield.servings = p(1, 1);
    amount(&mut r, 80, 1, Unit::G, IngredientForm::Dry);
    r.meal_types = BoundedVec(vec![MealType::Breakfast, MealType::Lunch, MealType::Dinner]);
    for mt in [MealType::Breakfast, MealType::Lunch] {
        preset(
            &mut r,
            AgeBand::Adult,
            mt,
            PortionPresetName::Regular,
            p(1, 1),
        );
    }
    preset(
        &mut r,
        AgeBand::Adult,
        MealType::Breakfast,
        PortionPresetName::Larger,
        p(3, 2),
    );
    preset(
        &mut r,
        AgeBand::Adult,
        MealType::Lunch,
        PortionPresetName::Smaller,
        p(3, 4),
    );
    let mut m = meal();
    let mut second = m.attendees[0].clone();
    second.person = person(2);
    m.attendees.0.push(second);
    m.attendees[0].portion.meal_overrides = BoundedVec(vec![
        MealPortionOverride {
            meal_type: MealType::Breakfast,
            preset: PortionPresetName::Larger,
            adjustment: p(1, 1),
        },
        MealPortionOverride {
            meal_type: MealType::Lunch,
            preset: PortionPresetName::Smaller,
            adjustment: p(1, 1),
        },
    ]);
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    for (mt, want) in [
        (MealType::Breakfast, 120),
        (MealType::Lunch, 60),
        (MealType::Dinner, 80),
    ] {
        m.slot.meal_type = mt;
        let s = resolve_servings(&rr, &m, &no_overrides()).unwrap();
        expect_amount(
            scale_recipe(&rr, s[0].servings).unwrap().ingredients[0]
                .quantity
                .amount,
            want,
            1,
        );
        expect_amount(
            scale_recipe(&rr, s[1].servings).unwrap().ingredients[0]
                .quantity
                .amount,
            80,
            1,
        );
    }
}
#[test]
fn food_f37_fractional_egg_without_supported_scaling() {
    let o = oracle("F37_");
    assert_eq!(o["expected"]["naive_eggs"], "3/2");
    let mut r = recipe();
    amount(&mut r, 3, 1, Unit::Each, IngredientForm::Whole);
    r.ingredients[0].discrete = true;
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    assert_eq!(
        scale_recipe(&rr, p(1, 1)).unwrap_err().code,
        ErrorCode::UnsupportedCapability
    );
}
#[test]
fn food_f38_discrete_batch_scaling_control() {
    let o = oracle("F38_");
    assert_eq!(o["expected"]["eggs"]["amount"], "6");
    let mut r = recipe();
    amount(&mut r, 3, 1, Unit::Each, IngredientForm::Whole);
    r.ingredients[0].discrete = true;
    r.scaling = ScalingPolicy::DiscreteBatches {
        batch_yield: p(2, 1),
        maximum_batches: 3,
    };
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    let s = scale_recipe(&rr, p(3, 1)).unwrap();
    assert_eq!(s.discrete_batches, Some(2));
    expect_amount(s.produced_servings, 4, 1);
    expect_amount(s.requested_servings, 3, 1);
    assert_eq!(s.unallocated_servings, Rational::new(1, 1).unwrap());
    expect_amount(s.ingredients[0].quantity.amount, 6, 1);
}
#[test]
fn food_f39_dry_cooked_without_conversion() {
    let o = oracle("F39_");
    assert!(o["expected"]["pack_count"].is_null());
    let r = recipe();
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    let q = Quantity {
        amount: p(300, 1),
        unit: Unit::G,
        form: IngredientForm::Cooked,
    };
    assert_eq!(
        convert_quantity(
            &rr,
            &r.ingredients[0].ingredient_id,
            &q,
            Unit::G,
            IngredientForm::Dry,
            &eid(9)
        )
        .unwrap_err()
        .code,
        ErrorCode::UnsupportedCapability
    );
}
fn conversion(r: &mut RecipeRevision) {
    r.ingredients[0].conversion_evidence = BoundedVec(vec![ConversionEvidence {
        evidence_id: eid(9),
        ingredient_id: r.ingredients[0].ingredient_id.clone(),
        from_form: IngredientForm::Dry,
        to_form: IngredientForm::Cooked,
        from_unit: Unit::G,
        to_unit: Unit::G,
        factor: p(3, 1),
        source: r.source.clone(),
    }]);
}
#[test]
fn food_f40_scoped_conversion_control() {
    let o = oracle("F40_");
    assert_eq!(o["input"]["demand"]["amount"], "100");
    let mut r = recipe();
    conversion(&mut r);
    let before = r.clone();
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    let cooked = Quantity {
        amount: p(300, 1),
        unit: Unit::G,
        form: IngredientForm::Cooked,
    };
    let dry = convert_quantity(
        &rr,
        &r.ingredients[0].ingredient_id,
        &cooked,
        Unit::G,
        IngredientForm::Dry,
        &eid(9),
    )
    .unwrap();
    expect_amount(dry.amount, 100, 1);
    assert_eq!(
        convert_quantity(
            &rr,
            &r.ingredients[0].ingredient_id,
            &dry,
            Unit::G,
            IngredientForm::Cooked,
            &eid(9)
        )
        .unwrap(),
        cooked
    );
    assert_eq!(r, before); // scoped original evidence and reciprocal direction are retained, never rewritten.
}
fn vegetarian() -> HardRequirement {
    HardRequirement::Diet {
        code: DietCode::Vegetarian,
    }
}
fn gf() -> HardRequirement {
    HardRequirement::Diet {
        code: DietCode::GlutenFree,
    }
}
fn sesame() -> HardRequirement {
    HardRequirement::AllergenExclusion {
        code: AllergenCode::Sesame,
    }
}
fn dietary(r: &mut RecipeRevision, requirement: HardRequirement, verdict: DietaryVerdict, n: u32) {
    r.dietary_evidence.0.push(DietaryEvidence {
        evidence_id: eid(n),
        requirement,
        verdict,
        source: r.source.clone(),
        review_id: eid(3),
    });
}
fn extra_case() -> (
    RecipeRevision,
    RecipeRevision,
    MealRequirements,
    ExtraSelection,
) {
    let mut base = recipe();
    base.r#yield.servings = p(1, 1);
    amount(&mut base, 120, 1, Unit::G, IngredientForm::Drained);
    dietary(&mut base, gf(), DietaryVerdict::Supported, 10);
    dietary(&mut base, sesame(), DietaryVerdict::Supported, 11);
    let mut extra = recipe();
    extra.recipe_id = RecipeId(format!("rc_{:032x}", 2));
    extra.r#yield.servings = p(1, 1);
    amount(&mut extra, 100, 1, Unit::G, IngredientForm::Raw);
    extra.dietary_evidence[0].verdict = DietaryVerdict::Conflicting;
    dietary(&mut extra, gf(), DietaryVerdict::Supported, 10);
    dietary(&mut extra, sesame(), DietaryVerdict::Supported, 11);
    let extra_id = ExtraId(format!("ex_{:032x}", 1));
    base.extras = BoundedVec(vec![ExtraSpec {
        extra_id: extra_id.clone(),
        title: Title("Synthetic chicken".into()),
        recipe: reference(&extra),
        eligible_person_requirements: BoundedVec(vec![]),
        supported_with: BoundedVec(vec![base.components[0].component_id.clone()]),
    }]);
    let mut m = meal();
    let mut b = m.attendees[0].clone();
    b.person = person(2);
    b.hard_requirements = BoundedVec(vec![gf()]);
    let mut guest = b.clone();
    guest.person = PersonRef::Guest {
        occasion_id: OccasionId(format!("oc_{:032x}", 1)),
        guest_id: GuestId(format!("gu_{:032x}", 1)),
    };
    guest.hard_requirements = BoundedVec(vec![sesame()]);
    guest.portion.adjustment = p(1, 2);
    m.attendees.0.extend([b, guest]);
    m.slot.date = LocalDate("2026-10-09".into());
    m.eat_by = Nullable(Some(
        ZonedMoment::new(
            LocalDate("2026-10-09".into()),
            LocalTime("18:00:00".into()),
            ZoneId("Europe/London".into()),
            Resolution::Unambiguous,
            Instant("2026-10-09T17:00:00Z".into()),
        )
        .unwrap(),
    ));
    m.shared_hard_requirements = BoundedVec(vec![vegetarian(), gf(), sesame()]);
    let selected = ExtraSelection {
        extra_id,
        recipe: reference(&extra),
        recipients: BoundedVec(vec![DinerServing {
            person: person(2),
            preset_ref: PortionPresetRef {
                recipe: reference(&extra),
                preset: PortionPresetName::Regular,
                age_band: AgeBand::Adult,
                meal_type: MealType::Dinner,
            },
            servings: p(1, 1),
        }]),
    };
    (base, extra, m, selected)
}
#[test]
fn food_f05_shared_base_and_separate_extra() {
    let o = oracle("F05_");
    assert_eq!(o["expected"]["beans"]["amount"], "300");
    assert_eq!(o["expected"]["chicken"]["amount"], "100");
    let (base, extra, m, selected) = extra_case();
    let before = (base.clone(), extra.clone(), m.clone(), selected.clone());
    let (b, c) = inputs(&base);
    let (eb, ec) = inputs(&extra);
    let br = assess_revision(&base, &b, &c).unwrap();
    let er = assess_revision(&extra, &eb, &ec).unwrap();
    assert_eq!(eligible(&br, &m).unwrap(), Eligibility::Eligible);
    assert_eq!(
        validate_extra(&br, &er, &selected, &m, &no_overrides()).unwrap(),
        Eligibility::Eligible
    );
    let s = resolve_servings(&br, &m, &no_overrides()).unwrap();
    let total = s.iter().fold(Rational::new(0, 1).unwrap(), |a, x| {
        a.checked_add(&x.servings.0).unwrap()
    });
    expect_amount(
        scale_recipe(&br, PositiveRational(total))
            .unwrap()
            .ingredients[0]
            .quantity
            .amount,
        300,
        1,
    );
    expect_amount(
        scale_recipe(&er, selected.recipients[0].servings)
            .unwrap()
            .ingredients[0]
            .quantity
            .amount,
        100,
        1,
    );
    assert_eq!(selected.recipients.len(), 1);
    assert_eq!(selected.recipients[0].person, person(2));
    assert_eq!((base, extra, m, selected), before);
}
#[test]
fn food_f06_extra_to_ineligible_person() {
    oracle("F06_");
    let (base, extra, m, mut selected) = extra_case();
    selected.recipients[0].person = person(1);
    let (b, c) = inputs(&base);
    let (eb, ec) = inputs(&extra);
    let br = assess_revision(&base, &b, &c).unwrap();
    let er = assess_revision(&extra, &eb, &ec).unwrap();
    assert!(matches!(
        validate_extra(&br, &er, &selected, &m, &no_overrides()).unwrap(),
        Eligibility::Ineligible(_)
    ));
}
#[test]
fn food_f07_guest_blocks_previous_base() {
    oracle("F07_");
    let (mut base, _, m, _) = extra_case();
    base.dietary_evidence[2].verdict = DietaryVerdict::Conflicting;
    let (b, c) = inputs(&base);
    let br = assess_revision(&base, &b, &c).unwrap();
    assert!(matches!(
        eligible(&br, &m).unwrap(),
        Eligibility::Ineligible(_)
    ));
    let mut without_guest = m.clone();
    without_guest.attendees.0.pop();
    without_guest.shared_hard_requirements = BoundedVec(vec![vegetarian(), gf()]);
    assert_eq!(
        eligible(&br, &without_guest).unwrap(),
        Eligibility::Eligible
    );
}
#[test]
fn food_f08_portable_cold_control_and_no_fit() {
    oracle("F08_");
    let mut cold = recipe();
    cold.equipment = BoundedVec(vec![]);
    cold.portability.cold_suitable = true;
    cold.portability.cold_storage_required = true;
    let mut m = meal();
    m.location = MealLocation::OnTheMove;
    m.equipment = BoundedVec(vec![]);
    m.mobility.temperature = MobilityTemperature::Cold;
    m.mobility.cold_storage = true;
    let (b, c) = inputs(&cold);
    let cr = assess_revision(&cold, &b, &c).unwrap();
    assert_eq!(eligible(&cr, &m).unwrap(), Eligibility::Eligible);
    let mut hot = cold.clone();
    hot.portability.cold_suitable = false;
    hot.portability.reheating_required = true;
    let (hb, hc) = inputs(&hot);
    let hr = assess_revision(&hot, &hb, &hc).unwrap();
    assert!(matches!(
        eligible(&hr, &m).unwrap(),
        Eligibility::Ineligible(_)
    ));
    m.mobility.cold_storage = false;
    assert!(matches!(
        eligible(&cr, &m).unwrap(),
        Eligibility::Unresolved(_)
    ));
}
#[test]
fn food_f09_no_suitable_recipe_both_routes() {
    oracle("F09_");
    let mut m = meal();
    m.attendees[0].hard_requirements.0.push(gf());
    m.shared_hard_requirements = m.attendees[0].hard_requirements.clone();
    for conflicting in [vegetarian(), gf()] {
        let mut r = recipe();
        r.dietary_evidence.0.clear();
        dietary(
            &mut r,
            vegetarian(),
            if conflicting == vegetarian() {
                DietaryVerdict::Conflicting
            } else {
                DietaryVerdict::Supported
            },
            10,
        );
        dietary(
            &mut r,
            gf(),
            if conflicting == gf() {
                DietaryVerdict::Conflicting
            } else {
                DietaryVerdict::Supported
            },
            11,
        );
        let (b, c) = inputs(&r);
        let rr = assess_revision(&r, &b, &c).unwrap();
        for _route in ["manual", "suggested"] {
            assert!(matches!(
                eligible(&rr, &m).unwrap(),
                Eligibility::Ineligible(_)
            ));
        }
    }
}
#[test]
fn food_discrete_nominal_yield_mismatch_is_unresolved() {
    let mut r = recipe();
    r.scaling = ScalingPolicy::DiscreteBatches {
        batch_yield: p(3, 1),
        maximum_batches: 3,
    };
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    assert_eq!(
        scale_recipe(&rr, p(2, 1)).unwrap_err().code,
        ErrorCode::UnsupportedCapability
    );
}
#[test]
fn food_invalid_constructed_primitive() {
    let r = recipe();
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    for (n, d) in [
        (0, 1),
        (1, 0),
        (2, 2),
        (MAX_NUMERATOR + 1, 1),
        (1, MAX_DENOMINATOR + 1),
        (u64::MAX, u64::MAX),
    ] {
        let bad = PositiveRational(Rational {
            numerator: DecimalU64(n),
            denominator: DecimalU64(d),
        });
        assert!(scale_recipe(&rr, bad).is_err());
        let mut m = meal();
        m.attendees[0].portion.adjustment = bad;
        assert!(resolve_servings(&rr, &m, &no_overrides()).is_err());
        assert!(eligible(&rr, &m).is_err());
        let q = Quantity {
            amount: bad,
            unit: Unit::G,
            form: IngredientForm::Raw,
        };
        assert!(
            convert_quantity(
                &rr,
                &r.ingredients[0].ingredient_id,
                &q,
                Unit::G,
                IngredientForm::Dry,
                &eid(9)
            )
            .is_err()
        );
    }
}
#[test]
fn food_rational_bound_and_overflow() {
    let mut r = recipe();
    r.r#yield.servings = p(1, 1);
    amount(&mut r, MAX_NUMERATOR, 1, Unit::G, IngredientForm::Raw);
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    expect_amount(
        scale_recipe(&rr, p(1, 1)).unwrap().ingredients[0]
            .quantity
            .amount,
        MAX_NUMERATOR,
        1,
    );
    assert_eq!(
        scale_recipe(&rr, p(2, 1)).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
    let mut r = recipe();
    r.r#yield.servings = p(999_983, 1);
    r.scaling = ScalingPolicy::Linear {
        minimum: p(1, MAX_DENOMINATOR),
        maximum: p(2, 1),
    };
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    assert_eq!(
        scale_recipe(&rr, p(1, 2)).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
}
#[test]
fn food_scaling_inclusive_range_and_batch_limit() {
    let mut r = recipe();
    r.scaling = ScalingPolicy::Linear {
        minimum: p(1, 2),
        maximum: p(2, 1),
    };
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    for requested in [p(1, 1), p(4, 1)] {
        assert!(scale_recipe(&rr, requested).is_ok());
    }
    for requested in [p(999, 1000), p(4001, 1000)] {
        assert_eq!(
            scale_recipe(&rr, requested).unwrap_err().code,
            ErrorCode::UnsupportedCapability
        );
    }
    r.scaling = ScalingPolicy::DiscreteBatches {
        batch_yield: p(2, 1),
        maximum_batches: 3,
    };
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    assert_eq!(
        scale_recipe(&rr, p(6, 1)).unwrap().discrete_batches,
        Some(3)
    );
    assert!(scale_recipe(&rr, p(6001, 1000)).is_err());
    assert_eq!(
        scale_recipe(&rr, p(1, MAX_DENOMINATOR))
            .unwrap()
            .discrete_batches,
        Some(1)
    );
}
#[test]
fn food_override_pair_replaces_instead_of_multiplies() {
    let mut r = recipe();
    preset(
        &mut r,
        AgeBand::Adult,
        MealType::Dinner,
        PortionPresetName::Larger,
        p(3, 2),
    );
    let mut m = meal();
    m.attendees[0].portion.adjustment = p(2, 1);
    m.attendees[0].portion.meal_overrides = BoundedVec(vec![MealPortionOverride {
        meal_type: MealType::Dinner,
        preset: PortionPresetName::Larger,
        adjustment: p(3, 1),
    }]);
    let o = MealOverrides {
        portions: BoundedVec(vec![PersonPortionOverride {
            person: person(1),
            preset: PortionPresetName::Regular,
            adjustment: p(5, 4),
        }]),
        eat_by: Nullable(None),
    };
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    expect_amount(resolve_servings(&rr, &m, &o).unwrap()[0].servings, 5, 4);
    expect_amount(
        resolve_servings(&rr, &m, &no_overrides()).unwrap()[0].servings,
        9,
        2,
    );
}
#[test]
fn food_missing_portion_evidence_refused() {
    let mut r = recipe();
    r.portion_presets[0].evidence_ids = BoundedVec(vec![]);
    let (b, c) = inputs(&r);
    assert_eq!(
        assess_revision(&r, &b, &c).unwrap_err().code,
        ErrorCode::MissingReference
    );
}
#[test]
fn food_meal_malformed_duplicate_and_inactive_refused() {
    let r = recipe();
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    let original = meal();
    let mut cases = vec![];
    let mut m = original.clone();
    m.enabled = false;
    cases.push(m);
    let mut m = original.clone();
    m.location = MealLocation::Skip;
    cases.push(m);
    let mut m = original.clone();
    m.shared_hard_requirements = BoundedVec(vec![]);
    cases.push(m);
    let mut m = original.clone();
    m.attendees.0.push(m.attendees[0].clone());
    cases.push(m);
    let mut m = original.clone();
    m.attendees = BoundedVec(vec![m.attendees[0].clone(); 65]);
    cases.push(m);
    let mut m = original.clone();
    m.slot.week_id = WeekId("broken".into());
    cases.push(m);
    let mut m = original.clone();
    m.attendees[0].portion.meal_overrides = BoundedVec(vec![
        MealPortionOverride {
            meal_type: MealType::Dinner,
            preset: PortionPresetName::Regular,
            adjustment: p(1, 1)
        };
        2
    ]);
    cases.push(m);
    for m in cases {
        assert!(eligible(&rr, &m).is_err());
        let e = resolve_servings(&rr, &m, &no_overrides()).unwrap_err();
        assert!(e.iter().all(|c| c.validate_shape().is_ok()));
    }
    let mut o = no_overrides();
    o.portions = BoundedVec(vec![PersonPortionOverride {
        person: person(2),
        preset: PortionPresetName::Regular,
        adjustment: p(1, 1),
    }]);
    assert!(resolve_servings(&rr, &original, &o).is_err());
    o.portions[0].person = person(1);
    o.portions.0.push(o.portions[0].clone());
    assert!(resolve_servings(&rr, &original, &o).is_err());
}
#[test]
fn food_dietary_unknown_contradictory_and_direct_exclusion() {
    for verdict in [DietaryVerdict::Unknown, DietaryVerdict::Conflicting] {
        let mut r = recipe();
        dietary(&mut r, vegetarian(), verdict, 10);
        let (b, c) = inputs(&r);
        let rr = assess_revision(&r, &b, &c).unwrap();
        let fit = eligible(&rr, &meal()).unwrap();
        if verdict == DietaryVerdict::Unknown {
            assert!(matches!(fit, Eligibility::Unresolved(_)));
        } else {
            assert!(matches!(fit, Eligibility::Ineligible(_)));
        }
    }
    let mut r = recipe();
    r.dietary_evidence = BoundedVec(vec![]);
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    assert!(matches!(
        eligible(&rr, &meal()).unwrap(),
        Eligibility::Unresolved(_)
    ));
    let mut m = meal();
    let exclusion = HardRequirement::IngredientExclusion {
        ingredient_id: r.ingredients[0].ingredient_id.clone(),
    };
    m.attendees[0].hard_requirements = BoundedVec(vec![exclusion.clone()]);
    m.shared_hard_requirements = m.attendees[0].hard_requirements.clone();
    dietary(&mut r, exclusion, DietaryVerdict::Supported, 10);
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    assert!(matches!(
        eligible(&rr, &m).unwrap(),
        Eligibility::Ineligible(_)
    ));
}
#[test]
fn food_portability_boolean_is_not_admitted_suitability() {
    let mut r = recipe();
    r.equipment = BoundedVec(vec![]);
    r.portability.cold_suitable = true;
    r.portability.evidence_ids = BoundedVec(vec![]);
    let mut m = meal();
    m.location = MealLocation::OnTheMove;
    m.mobility.temperature = MobilityTemperature::Cold;
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    assert!(matches!(
        eligible(&rr, &m).unwrap(),
        Eligibility::Unresolved(_)
    ));
}
#[test]
fn food_equipment_and_meal_type_constraints() {
    let r = recipe();
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    let mut m = meal();
    m.equipment = BoundedVec(vec![]);
    assert!(matches!(
        eligible(&rr, &m).unwrap(),
        Eligibility::Ineligible(_)
    ));
    let mut m = meal();
    m.slot.meal_type = MealType::Breakfast;
    assert!(matches!(
        eligible(&rr, &m).unwrap(),
        Eligibility::Ineligible(_)
    ));
    let mut m = meal();
    m.location = MealLocation::OnTheMove;
    assert!(matches!(
        eligible(&rr, &m).unwrap(),
        Eligibility::Unresolved(_)
    ));
}
#[test]
fn food_extra_duplicate_foreign_wrong_ref_and_amount_refused() {
    let (base, extra, m, selected) = extra_case();
    let (b, c) = inputs(&base);
    let (eb, ec) = inputs(&extra);
    let br = assess_revision(&base, &b, &c).unwrap();
    let er = assess_revision(&extra, &eb, &ec).unwrap();
    let mut duplicate = selected.clone();
    duplicate.recipients.0.push(duplicate.recipients[0].clone());
    assert!(validate_extra(&br, &er, &duplicate, &m, &no_overrides()).is_err());
    let mut foreign = selected.clone();
    foreign.recipients[0].person = person(99);
    assert!(validate_extra(&br, &er, &foreign, &m, &no_overrides()).is_err());
    let mut wrong = selected.clone();
    wrong.recipe.revision = Revision(2);
    assert!(validate_extra(&br, &er, &wrong, &m, &no_overrides()).is_err());
    let mut wrong = selected.clone();
    wrong.recipients[0].servings = p(2, 1);
    assert!(matches!(
        validate_extra(&br, &er, &wrong, &m, &no_overrides()).unwrap(),
        Eligibility::Ineligible(_)
    ));
    let mut wrong = selected.clone();
    wrong.recipients[0].preset_ref.recipe.digest = RecipeDigest("b".repeat(64));
    assert!(matches!(
        validate_extra(&br, &er, &wrong, &m, &no_overrides()).unwrap(),
        Eligibility::Ineligible(_)
    ));
}
#[test]
fn food_extra_applicability_never_weakens_restrictions() {
    let (mut base, extra, m, selected) = extra_case();
    base.extras[0].eligible_person_requirements = BoundedVec(vec![HardRequirement::Diet {
        code: DietCode::Vegan,
    }]);
    let (b, c) = inputs(&base);
    let (eb, ec) = inputs(&extra);
    let br = assess_revision(&base, &b, &c).unwrap();
    let er = assess_revision(&extra, &eb, &ec).unwrap();
    assert!(matches!(
        validate_extra(&br, &er, &selected, &m, &no_overrides()).unwrap(),
        Eligibility::Unresolved(_)
    ));
    base.extras[0].eligible_person_requirements = BoundedVec(vec![gf()]);
    let (b, c) = inputs(&base);
    let br = assess_revision(&base, &b, &c).unwrap();
    assert_eq!(
        validate_extra(&br, &er, &selected, &m, &no_overrides()).unwrap(),
        Eligibility::Eligible
    );
    let mut for_a = selected.clone();
    for_a.recipients[0].person = person(1);
    assert!(matches!(
        validate_extra(&br, &er, &for_a, &m, &no_overrides()).unwrap(),
        Eligibility::Ineligible(_)
    ));
    base.dietary_evidence[0].verdict = DietaryVerdict::Conflicting;
    let (b, c) = inputs(&base);
    let br = assess_revision(&base, &b, &c).unwrap();
    assert!(matches!(
        validate_extra(&br, &er, &selected, &m, &no_overrides()).unwrap(),
        Eligibility::Ineligible(_)
    ));
}
#[test]
fn food_conversion_endpoints_revision_and_conflicting_factors() {
    let mut r = recipe();
    conversion(&mut r);
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    let q = Quantity {
        amount: p(300, 1),
        unit: Unit::G,
        form: IngredientForm::Cooked,
    };
    for (unit, form) in [
        (Unit::Ml, IngredientForm::Dry),
        (Unit::G, IngredientForm::Raw),
    ] {
        assert!(
            convert_quantity(
                &rr,
                &r.ingredients[0].ingredient_id,
                &q,
                unit,
                form,
                &eid(9)
            )
            .is_err()
        );
    }
    assert!(
        convert_quantity(
            &rr,
            &IngredientId(format!("ig_{:032x}", 99)),
            &q,
            Unit::G,
            IngredientForm::Dry,
            &eid(9)
        )
        .is_err()
    );
    r.revision = Revision(2);
    r.ingredients[0].conversion_evidence[0].evidence_id = eid(10);
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    assert!(
        convert_quantity(
            &rr,
            &r.ingredients[0].ingredient_id,
            &q,
            Unit::G,
            IngredientForm::Dry,
            &eid(9)
        )
        .is_err()
    );
    let mut contrary = r.ingredients[0].conversion_evidence[0].clone();
    contrary.evidence_id = eid(11);
    contrary.factor = p(4, 1);
    r.ingredients[0].conversion_evidence.0.push(contrary);
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    assert_eq!(
        convert_quantity(
            &rr,
            &r.ingredients[0].ingredient_id,
            &q,
            Unit::G,
            IngredientForm::Dry,
            &eid(10)
        )
        .unwrap_err()
        .code,
        ErrorCode::DomainConflict
    );
}
#[test]
fn food_unknown_missing_class_and_storage_keep_fresh_paths() {
    for known in [false, true] {
        for absent_storage in [false, true] {
            let mut r = recipe();
            r.capabilities.leftovers = false;
            r.capabilities.blocking_issues = BoundedVec(vec![CandidateIssue {
                code: CandidateIssueCode::MissingStorage,
                path: Text("/components/0/storage_rule_ids".into()),
                evidence_ids: BoundedVec(vec![]),
            }]);
            if absent_storage {
                r.storage = BoundedVec(vec![]);
                r.components[0].storage_rule_ids = BoundedVec(vec![]);
            }
            let (b, mut c) = inputs(&r);
            if known {
                c.components[0].class = ComponentClass::Unknown;
            } else {
                c.components.clear();
            }
            let rr = assess_revision(&r, &b, &c).unwrap();
            assert_eq!(eligible(&rr, &meal()).unwrap(), Eligibility::Eligible);
            assert!(resolve_servings(&rr, &meal(), &no_overrides()).is_ok());
            assert!(scale_recipe(&rr, p(1, 1)).is_ok());
        }
    }
}
#[test]
fn food_shopping_disabled_wrapper_never_grants_derivation() {
    let mut r = recipe();
    r.reviews.0.retain(|r| r.kind != ReviewKind::QuantityReview);
    r.capabilities.shopping = false;
    r.capabilities.leftovers = false;
    r.capabilities.blocking_issues = BoundedVec(vec![CandidateIssue {
        code: CandidateIssueCode::MissingQuantity,
        path: Text("/reviews".into()),
        evidence_ids: BoundedVec(vec![]),
    }]);
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    assert!(matches!(
        eligible(&rr, &meal()).unwrap(),
        Eligibility::Unresolved(_)
    ));
    assert!(resolve_servings(&rr, &meal(), &no_overrides()).is_err());
    assert_eq!(
        scale_recipe(&rr, p(1, 1)).unwrap_err().code,
        ErrorCode::UnsupportedCapability
    );
    let q = r.ingredients[0].quantity.0.as_ref().unwrap();
    assert_eq!(
        convert_quantity(
            &rr,
            &r.ingredients[0].ingredient_id,
            q,
            Unit::G,
            IngredientForm::Dry,
            &eid(9)
        )
        .unwrap_err()
        .code,
        ErrorCode::UnsupportedCapability
    );
}
fn second_component(r: &mut RecipeRevision) {
    let component = ComponentId(format!("co_{:032x}", 2));
    let mut spec = r.components[0].clone();
    spec.component_id = component.clone();
    spec.storage_rule_ids = BoundedVec(vec![eid(20)]);
    r.components.0.push(spec);
    let mut ingredient = r.ingredients[0].clone();
    ingredient.component_id = component.clone();
    r.ingredients.0.push(ingredient);
    let mut storage = r.storage[0].clone();
    storage.component_id = component.clone();
    storage.evidence_id = eid(20);
    r.storage.0.push(storage);
    let mut yield_component = r.r#yield.components[0].clone();
    yield_component.component_id = component.clone();
    r.r#yield.components.0.push(yield_component);
    r.steps[0].component_ids.0.push(component);
}
#[test]
fn food_conversion_same_ingredient_across_components_checks_all_rules() {
    let mut r = recipe();
    conversion(&mut r);
    second_component(&mut r);
    r.ingredients[1].conversion_evidence[0].evidence_id = eid(10);
    r.ingredients[1].conversion_evidence[0].factor = p(4, 1);
    let (b, mut c) = inputs(&r);
    let mut fact = c.components[0].clone();
    fact.component_id = r.components[1].component_id.clone();
    c.components.push(fact);
    let rr = assess_revision(&r, &b, &c).unwrap();
    let q = Quantity {
        amount: p(300, 1),
        unit: Unit::G,
        form: IngredientForm::Cooked,
    };
    for selected in [eid(9), eid(10)] {
        assert_eq!(
            convert_quantity(
                &rr,
                &r.ingredients[0].ingredient_id,
                &q,
                Unit::G,
                IngredientForm::Dry,
                &selected
            )
            .unwrap_err()
            .code,
            ErrorCode::DomainConflict
        );
    }
}
#[test]
fn food_conflicts_are_deterministic_across_requirement_order() {
    let (mut r, _, mut m, _) = extra_case();
    r.dietary_evidence[0].verdict = DietaryVerdict::Conflicting;
    r.dietary_evidence[1].verdict = DietaryVerdict::Conflicting;
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    let first = eligible(&rr, &m).unwrap();
    m.shared_hard_requirements.0.reverse();
    assert_eq!(eligible(&rr, &m).unwrap(), first);
}
#[test]
fn food_conversion_inverse_limits_and_no_multihop() {
    let mut r = recipe();
    conversion(&mut r);
    r.ingredients[0].conversion_evidence[0].factor = p(MAX_NUMERATOR, 1);
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    let q = Quantity {
        amount: p(MAX_NUMERATOR, 1),
        unit: Unit::G,
        form: IngredientForm::Cooked,
    };
    expect_amount(
        convert_quantity(
            &rr,
            &r.ingredients[0].ingredient_id,
            &q,
            Unit::G,
            IngredientForm::Dry,
            &eid(9),
        )
        .unwrap()
        .amount,
        1,
        1,
    );
    let mut r = recipe();
    conversion(&mut r);
    let mut second = r.ingredients[0].conversion_evidence[0].clone();
    second.evidence_id = eid(10);
    second.from_form = IngredientForm::Cooked;
    second.to_form = IngredientForm::Prepared;
    second.factor = p(1, 1);
    r.ingredients[0].conversion_evidence.0.push(second);
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    let q = Quantity {
        amount: p(100, 1),
        unit: Unit::G,
        form: IngredientForm::Dry,
    };
    for evidence in [eid(9), eid(10)] {
        assert_eq!(
            convert_quantity(
                &rr,
                &r.ingredients[0].ingredient_id,
                &q,
                Unit::G,
                IngredientForm::Prepared,
                &evidence
            )
            .unwrap_err()
            .code,
            ErrorCode::UnsupportedCapability
        );
    }
}
#[test]
fn food_conversion_identical_evidence_across_components_coalesces() {
    let mut r = recipe();
    conversion(&mut r);
    second_component(&mut r);
    let (b, mut c) = inputs(&r);
    let mut fact = c.components[0].clone();
    fact.component_id = r.components[1].component_id.clone();
    c.components.push(fact);
    let rr = assess_revision(&r, &b, &c).unwrap();
    let q = Quantity {
        amount: p(300, 1),
        unit: Unit::G,
        form: IngredientForm::Cooked,
    };
    assert_eq!(
        convert_quantity(
            &rr,
            &r.ingredients[0].ingredient_id,
            &q,
            Unit::G,
            IngredientForm::Dry,
            &eid(9)
        )
        .unwrap()
        .amount,
        p(100, 1)
    );
}
include!(
    "../../../tests/fixtures/food/portions-eligibility/r1-original-conversion-counterexample.inc.rs"
);

fn local_override(
    person: PersonRef,
    preset: PortionPresetName,
    adjustment: PositiveRational,
) -> MealOverrides {
    MealOverrides {
        portions: BoundedVec(vec![PersonPortionOverride {
            person,
            preset,
            adjustment,
        }]),
        eat_by: Nullable(None),
    }
}
fn extra_fit(
    base: &RecipeRevision,
    extra: &RecipeRevision,
    selected: &ExtraSelection,
    meal: &MealRequirements,
    overrides: &MealOverrides,
) -> Result<Eligibility, DomainError> {
    let (bb, bc) = inputs(base);
    let br = assess_revision(base, &bb, &bc).unwrap();
    let (eb, ec) = inputs(extra);
    let er = assess_revision(extra, &eb, &ec).unwrap();
    validate_extra(&br, &er, selected, meal, overrides)
}
#[test]
fn food_extra_local_override_replaces_household_and_meal_type_pair() {
    let (mut base, mut extra, mut m, mut selected) = extra_case();
    preset(
        &mut extra,
        AgeBand::Adult,
        MealType::Dinner,
        PortionPresetName::Larger,
        p(3, 2),
    );
    base.extras[0].recipe = reference(&extra);
    selected.recipe = reference(&extra);
    selected.recipients[0].preset_ref.recipe = reference(&extra);
    m.attendees[1].portion.adjustment = p(2, 1);
    m.attendees[1].portion.meal_overrides = BoundedVec(vec![MealPortionOverride {
        meal_type: MealType::Dinner,
        preset: PortionPresetName::Larger,
        adjustment: p(3, 1),
    }]);
    let o = local_override(person(2), PortionPresetName::Regular, p(5, 4));
    selected.recipients[0].servings = p(5, 4);
    let before = (
        base.clone(),
        extra.clone(),
        m.clone(),
        selected.clone(),
        o.clone(),
    );
    assert_eq!(
        extra_fit(&base, &extra, &selected, &m, &o).unwrap(),
        Eligibility::Eligible
    );
    assert!(matches!(
        extra_fit(&base, &extra, &selected, &m, &no_overrides()).unwrap(),
        Eligibility::Ineligible(_)
    ));
    assert_eq!((base, extra, m, selected, o), before);
}
#[test]
fn food_extra_valid_nonrecipient_override_is_irrelevant() {
    let (base, extra, m, selected) = extra_case();
    // This preset need not be offered by this extra to an unrelated attendee.
    let o = local_override(person(1), PortionPresetName::Smaller, p(7, 4));
    let before = (m.clone(), selected.clone(), o.clone());
    assert_eq!(
        extra_fit(&base, &extra, &selected, &m, &o).unwrap(),
        Eligibility::Eligible
    );
    assert_eq!((m, selected, o), before);
}
#[test]
fn food_extra_omitted_override_never_inferred_from_selected_amount() {
    let (base, extra, m, mut selected) = extra_case();
    selected.recipients[0].servings = p(3, 2);
    assert!(matches!(
        extra_fit(&base, &extra, &selected, &m, &no_overrides()).unwrap(),
        Eligibility::Ineligible(_)
    ));
    let o = local_override(person(2), PortionPresetName::Regular, p(3, 2));
    assert_eq!(
        extra_fit(&base, &extra, &selected, &m, &o).unwrap(),
        Eligibility::Eligible
    );
}
#[test]
fn food_extra_duplicate_foreign_overrides_reject_before_projection() {
    let (mut base, extra, m, selected) = extra_case();
    for who in [person(1), person(2)] {
        let mut o = local_override(who, PortionPresetName::Regular, p(1, 1));
        o.portions.0.push(o.portions[0].clone());
        assert!(extra_fit(&base, &extra, &selected, &m, &o).is_err());
    }
    let foreign = local_override(person(99), PortionPresetName::Regular, p(1, 1));
    assert_eq!(
        extra_fit(&base, &extra, &selected, &m, &foreign)
            .unwrap_err()
            .code,
        ErrorCode::ForeignReference
    );
    base.dietary_evidence[0].verdict = DietaryVerdict::Conflicting;
    assert_eq!(
        extra_fit(&base, &extra, &selected, &m, &foreign)
            .unwrap_err()
            .code,
        ErrorCode::ForeignReference
    );
}
#[test]
fn food_extra_full_override_shape_bounds_and_eat_by_checked() {
    let (base, extra, m, selected) = extra_case();
    for (n, d) in [(0, 1), (1, 0), (2, 2), (MAX_NUMERATOR + 1, 1)] {
        let o = local_override(
            person(1),
            PortionPresetName::Regular,
            PositiveRational(Rational {
                numerator: DecimalU64(n),
                denominator: DecimalU64(d),
            }),
        );
        assert!(extra_fit(&base, &extra, &selected, &m, &o).is_err());
    }
    let mut o = local_override(person(1), PortionPresetName::Regular, p(1, 1));
    o.portions = BoundedVec(vec![o.portions[0].clone(); 65]);
    assert_eq!(
        extra_fit(&base, &extra, &selected, &m, &o)
            .unwrap_err()
            .code,
        ErrorCode::LimitExceeded
    );
    let mut o = no_overrides();
    o.eat_by = m.eat_by.clone();
    o.eat_by.0.as_mut().unwrap().instant = Instant("malformed".into());
    assert!(extra_fit(&base, &extra, &selected, &m, &o).is_err());
    o.eat_by = m.eat_by.clone();
    assert_eq!(
        extra_fit(&base, &extra, &selected, &m, &o).unwrap(),
        Eligibility::Eligible
    );
}
#[test]
fn food_extra_local_missing_mapping_remains_unresolved() {
    let (base, extra, mut m, selected) = extra_case();
    let o = local_override(person(2), PortionPresetName::Smaller, p(1, 1));
    assert!(matches!(
        extra_fit(&base, &extra, &selected, &m, &o).unwrap(),
        Eligibility::Unresolved(_)
    ));
    m.attendees[1].age_band = AgeBand::Child4To6;
    let o = local_override(person(2), PortionPresetName::Regular, p(1, 1));
    assert!(matches!(
        extra_fit(&base, &extra, &selected, &m, &o).unwrap(),
        Eligibility::Unresolved(_)
    ));
}
#[test]
fn food_extra_local_override_does_not_relax_recipient_restrictions() {
    let (base, extra, m, mut selected) = extra_case();
    selected.recipients[0].person = person(1);
    selected.recipients[0].servings = p(1, 2);
    let o = local_override(person(1), PortionPresetName::Regular, p(1, 2));
    assert!(matches!(
        extra_fit(&base, &extra, &selected, &m, &o).unwrap(),
        Eligibility::Ineligible(_)
    ));
}
#[test]
fn food_extra_guest_default_and_local_override() {
    let (base, extra, m, mut selected) = extra_case();
    let guest = m.attendees[2].person.clone();
    selected.recipients[0].person = guest.clone();
    selected.recipients[0].servings = p(1, 2);
    assert_eq!(
        extra_fit(&base, &extra, &selected, &m, &no_overrides()).unwrap(),
        Eligibility::Eligible
    );
    let o = local_override(guest, PortionPresetName::Regular, p(3, 4));
    selected.recipients[0].servings = p(3, 4);
    let before = (
        base.clone(),
        extra.clone(),
        m.clone(),
        selected.clone(),
        o.clone(),
    );
    assert_eq!(
        extra_fit(&base, &extra, &selected, &m, &o).unwrap(),
        Eligibility::Eligible
    );
    assert_eq!((base, extra, m, selected, o), before);
}
#[test]
fn food_extra_local_servings_match_by_identity_without_reordering_inputs() {
    let (base, extra, m, mut selected) = extra_case();
    let mut guest = selected.recipients[0].clone();
    guest.person = m.attendees[2].person.clone();
    guest.servings = p(3, 4);
    selected.recipients[0].servings = p(5, 4);
    selected.recipients.0.insert(0, guest);
    let mut o = local_override(
        m.attendees[2].person.clone(),
        PortionPresetName::Regular,
        p(3, 4),
    );
    o.portions.0.push(PersonPortionOverride {
        person: person(2),
        preset: PortionPresetName::Regular,
        adjustment: p(5, 4),
    });
    o.portions.0.push(PersonPortionOverride {
        person: person(1),
        preset: PortionPresetName::Smaller,
        adjustment: p(7, 4),
    });
    let before = (m.clone(), selected.clone(), o.clone());
    assert_eq!(
        extra_fit(&base, &extra, &selected, &m, &o).unwrap(),
        Eligibility::Eligible
    );
    assert_eq!((m.clone(), selected, o), before);
    let mut malformed = m.clone();
    malformed.equipment = BoundedVec(vec![EquipmentCode::Oven, EquipmentCode::Hob]);
    let selected = extra_case().3;
    let unchanged = malformed.clone();
    assert!(extra_fit(&base, &extra, &selected, &malformed, &no_overrides()).is_err());
    assert_eq!(malformed, unchanged);
}
#[test]
fn food_extra_local_selected_binding_and_amount_tampering_refused() {
    let (base, extra, m, mut selected) = extra_case();
    selected.recipients[0].servings = p(3, 2);
    let o = local_override(person(2), PortionPresetName::Regular, p(3, 2));
    for mutation in 0..5 {
        let mut wrong = selected.clone();
        match mutation {
            0 => wrong.recipients[0].servings = p(2, 1),
            1 => wrong.recipients[0].preset_ref.age_band = AgeBand::Teen,
            2 => wrong.recipients[0].preset_ref.preset = PortionPresetName::Larger,
            3 => wrong.recipients[0].preset_ref.meal_type = MealType::Lunch,
            _ => wrong.recipients[0].preset_ref.recipe.revision = Revision(2),
        };
        assert!(matches!(
            extra_fit(&base, &extra, &wrong, &m, &o).unwrap(),
            Eligibility::Ineligible(_)
        ));
    }
}
#[test]
fn food_conversion_same_id_differing_complete_records_still_conflict() {
    for change in 0..3 {
        for reverse in [false, true] {
            let mut r = recipe();
            conversion(&mut r);
            second_component(&mut r);
            match change {
                0 => r.ingredients[1].conversion_evidence[0].factor = p(4, 1),
                1 => {
                    r.ingredients[1].conversion_evidence[0]
                        .source
                        .source_revision = Nullable(Some(Text("different-source".into())))
                }
                _ => r.ingredients[1].conversion_evidence[0].from_unit = Unit::Ml,
            }
            if reverse {
                r.ingredients.0.reverse();
            }
            let (b, c) = reviewed_classes(&r);
            let rr = assess_revision(&r, &b, &c).unwrap();
            assert_eq!(
                dry_result(&rr, &eid(9)).unwrap_err().code,
                ErrorCode::DomainConflict
            );
        }
    }
}
#[test]
fn food_conversion_distinct_consistent_ids_and_order_controls() {
    for identical_id in [false, true] {
        for reverse in [false, true] {
            let mut r = recipe();
            conversion(&mut r);
            second_component(&mut r);
            if !identical_id {
                r.ingredients[1].conversion_evidence[0].evidence_id = eid(10);
            }
            if reverse {
                r.ingredients.0.reverse();
            }
            let (b, c) = reviewed_classes(&r);
            let rr = assess_revision(&r, &b, &c).unwrap();
            expect_amount(dry_result(&rr, &eid(9)).unwrap().amount, 100, 1);
            if !identical_id {
                expect_amount(dry_result(&rr, &eid(10)).unwrap().amount, 100, 1);
            }
        }
    }
}
