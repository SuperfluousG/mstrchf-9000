//! Synthetic conditional-trust tests. No real source interpretation or shopping acceptance.
use mstrchf_contracts::*;
use mstrchf_core::{food::*, products::*};
mod shopping_matching_support;
use shopping_matching_support::*;
#[test]
fn admitted_extra_origin_retains_recipient_and_extra_spec_requirements() {
    let mut v = extra_value();
    let vegetarian = HardRequirement::Diet {
        code: DietCode::Vegetarian,
    };
    v["context"]["household"]["diners"][1]["restrictions"] =
        v["context"]["household"]["diners"][0]["restrictions"].clone();
    v["context"]["recipes"][1]["dietary_evidence"] =
        v["context"]["recipes"][0]["dietary_evidence"].clone();
    let mut gf_evidence = v["context"]["recipes"][1]["dietary_evidence"][0].clone();
    gf_evidence["requirement"] = serde_json::json!(gf());
    gf_evidence["evidence_id"] = serde_json::json!(pin(303).evidence_id);
    v["context"]["recipes"][1]["dietary_evidence"]
        .as_array_mut()
        .unwrap()
        .push(gf_evidence);
    v["evidence"]["recipes"][1]["admitted"].as_array_mut().unwrap().push(serde_json::json!({"evidence_id":pin(303).evidence_id,"revision":"1","digest":pin(303).digest}));
    v["context"]["recipes"][0]["extras"][0]["eligible_person_requirements"] =
        serde_json::json!([gf()]);
    let (s, d) = rebuild(v);
    let line = d
        .demand
        .lines
        .iter()
        .find(|l| !l.origin_extras.is_empty())
        .unwrap();
    let mut p = policy();
    p.rules.push(RuleBinding {
        requirement: vegetarian.clone(),
        required_fields: vec![SourceField::Ingredients],
        review: pin(100),
    });
    p.rules
        .sort_by_key(|r| canonical_bytes(&r.requirement).unwrap());
    p.digest = policy_digest(&p).unwrap();
    let f = ReadFixture::new();
    let mut m = current_meaning(&s, &d, &f);
    m.key_claim.supply_key = DemandKey {
        ingredient_id: line.ingredient_id.clone(),
        unit: line.quantity.unit,
        form: line.quantity.form,
    };
    let mut veg_claim = m.requirement_claims[0].clone();
    veg_claim.requirement = vegetarian.clone();
    m.requirement_claims.push(veg_claim);
    m.requirement_claims
        .sort_by_key(|r| canonical_bytes(&r.requirement).unwrap());
    for state in [ClaimState::Supported, ClaimState::Unknown] {
        m.requirement_claims
            .iter_mut()
            .find(|r| r.requirement == gf())
            .unwrap()
            .state = state;
        let mut a = admission();
        a.policy_digest = p.digest.clone();
        a.meaning_digest = meaning_digest(&m).unwrap();
        let c = prepare_match_context(&s.context, &s, &d, &p, now()).unwrap();
        let source = f.source();
        let reviewed = assess_product_meaning(&source, Some(&m), Some(&a), &c).unwrap();
        let result = match_product(&c, &line.line_id, &reviewed).unwrap();
        assert_eq!(
            outcome(&result),
            if state == ClaimState::Supported {
                "Suitable"
            } else {
                "Unresolved"
            }
        );
        if let MatchAssessment::Suitable(candidate) = result {
            assert_eq!(candidate.basis().origins.len(), 1);
            assert!(candidate.basis().origins[0].occurrence.extra_id.is_some());
            for u in candidate
                .basis()
                .uses
                .iter()
                .filter(|u| matches!(u.use_key, UseKey::Consumption { .. }))
            {
                assert!(u.requirements.contains(&gf()));
                assert!(u.requirements.contains(&vegetarian));
            }
        }
    }
}
#[test]
fn m11_stale_extra_projection_is_invalid_while_recipient_projection_is_valid() {
    use mstrchf_core::{
        eligibility::{Eligibility, validate_extra},
        portions::resolve_servings,
        recipes::assess_revision,
    };
    let s = rebuild_inputs(extra_value());
    let sel = &s.plan.selections[0];
    let selected = &sel.extras[0];
    let review = |reference: &RecipeRef| {
        assess_revision(
            s.context
                .recipes
                .iter()
                .find(|r| r.recipe_id == reference.recipe_id)
                .unwrap(),
            s.evidence
                .recipes
                .iter()
                .find(|r| r.recipe == *reference)
                .unwrap(),
            s.evidence
                .classes
                .iter()
                .find(|r| r.recipe == *reference)
                .unwrap(),
        )
        .unwrap()
    };
    let base = review(&sel.recipe);
    let extra = review(&selected.recipe);
    let meal = s
        .context
        .requirements
        .iter()
        .find(|m| m.slot == sel.slot)
        .unwrap();
    assert_eq!(
        validate_extra(&base, &extra, selected, meal, &sel.meal_overrides).unwrap(),
        Eligibility::Eligible
    );
    // Retain the former stale projection as a negative control after the fix.
    let mut filtered = meal.clone();
    filtered
        .attendees
        .0
        .retain(|a| selected.recipients.iter().any(|r| r.person == a.person));
    let failure = resolve_servings(&extra, &filtered, &sel.meal_overrides).unwrap_err();
    assert_eq!(failure[0].code, ErrorCode::DomainConflict);
    // The declared recipient itself has no restrictions; the stale shared union
    // is the upstream failure, not product meaning or the unchanged M11 oracle.
    assert!(filtered.attendees[0].hard_requirements.is_empty());
    assert_eq!(
        filtered.shared_hard_requirements.0,
        vec![HardRequirement::Diet {
            code: DietCode::Vegetarian
        }]
    );
    filtered.shared_hard_requirements.0.clear();
    assert_eq!(
        resolve_servings(&extra, &filtered, &sel.meal_overrides).unwrap(),
        selected.recipients.0
    );
}
#[test]
fn independent_complete_canonical_vectors() {
    let v = vector();
    let p = policy();
    let m = meaning();
    let b = basis();
    assert_eq!(policy_digest(&p).unwrap().0, v["policy"]["digest"]);
    assert_eq!(
        meaning_digest(&m).unwrap().0,
        v["admission"]["meaning_digest"]
    );
    assert_eq!(
        match_basis_digest(&b).unwrap().0,
        "a24f494c7780c9fd3731055c6d8cf9e52d215d48a9a98ff0efbc0b62e20d5e3d"
    );
    let actual = serde_json::json!({"domain":"mstrchf-product-match-v1","material":match_basis_material(&b).unwrap()});
    let expected: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/shopping/matching/match.canonical.json"
    ))
    .unwrap();
    assert_eq!(actual, expected);
}
#[test]
fn m03_positive_full_native_premises_reach_match_boundary() {
    let (s, d) = base();
    let p = policy();
    let f = ReadFixture::new();
    let m = meaning();
    let a = admission();
    // Existing independent native custody and read guards pass before subject boundary.
    saved_basis_digest(&s).unwrap();
    mstrchf_core::demand::reconstruct_use_graph(&d, &s).unwrap();
    validate_read_binding(
        &f.observation,
        &f.request,
        &f.reader,
        f.accepted.admitted_at.clone(),
    )
    .unwrap();
    let c = prepare_match_context(&s.context, &s, &d, &p, now()).unwrap();
    let source = f.source();
    let r = assess_product_meaning(&source, Some(&m), Some(&a), &c).unwrap();
    let result = match_product(&c, &d.demand.lines[0].line_id, &r).unwrap();
    assert_eq!(outcome(&result), "Suitable");
    let MatchAssessment::Suitable(candidate) = result else {
        panic!()
    };
    assert_eq!(candidate.basis().line, d.demand.lines[0]);
    assert_eq!(candidate.basis().uses.len(), 4);
    assert_eq!(candidate.observation(), f.item());
}

#[test]
fn fixture_changed_requirement_and_conversion_premises_are_admitted() {
    let (s, d) = gf_base();
    mstrchf_core::demand::reconstruct_use_graph(&d, &s).unwrap();
    assert_eq!(
        s.context.requirements.iter().filter(|m| m.enabled).count(),
        2
    );
    assert!(
        s.context
            .requirements
            .iter()
            .filter(|m| m.enabled)
            .all(|m| m.shared_hard_requirements.0 == vec![gf()])
    );
    let mut v = saved_value();
    conversion(&mut v, &[3], false);
    let (s, d) = rebuild(v);
    mstrchf_core::demand::reconstruct_use_graph(&d, &s).unwrap();
    assert_eq!(
        d.demand.lines[0].quantity.amount,
        PositiveRational::new(300, 1).unwrap()
    );
    assert_eq!(d.demand.lines[0].quantity.form, IngredientForm::Cooked);
}
#[test]
fn m01_m02_f50_relevant_raw_states_defeat_positive_admission() {
    let (s, d) = gf_base();
    let p = policy();
    for conflicting in [false, true] {
        let mut f = ReadFixture::new();
        f.item_mut().title = Text("SYNTHETIC gluten-free sauce".into());
        let mut m = current_meaning(&s, &d, &f);
        let mut a = admission();
        if conflicting {
            f.item_mut().ingredients = Fact::Conflicting {
                values: BoundedVec(vec![
                    Text("contains wheat".into()),
                    Text("gluten free".into()),
                ]),
            };
            f.item_mut().allergens = f.item().ingredients.clone();
        } else {
            f.item_mut().ingredients = Fact::Unknown {
                reason: UnknownReason::Missing,
            };
            f.item_mut().allergens = Fact::Unknown {
                reason: UnknownReason::Unreadable,
            };
        }
        f.refresh(&mut m, &mut a);
        assert_eq!(
            outcome(&assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).unwrap()),
            "Unresolved"
        );
        assert_eq!(
            outcome(&assess(&s, &d, &p, &f, None, None, now()).unwrap()),
            "Unresolved"
        );
    }
}
#[test]
fn m03_m04_m05_f51_typed_known_label_meanings() {
    let (s, d) = gf_base();
    let p = policy();
    for (state, expected) in [
        (ClaimState::Supported, "Suitable"),
        (ClaimState::Refuted, "Unsuitable"),
        (ClaimState::Contradictory, "Unresolved"),
    ] {
        let mut f = ReadFixture::new();
        f.item_mut().price = serde_json::from_value(
            serde_json::json!({"kind":"observed","data":{"pence":"100","currency":"GBP"}}),
        )
        .unwrap();
        if state == ClaimState::Refuted {
            f.item_mut().ingredients = Fact::Known {
                value: Text("wheat flour, water".into()),
            };
            f.item_mut().allergens = Fact::Known {
                value: Text("contains wheat".into()),
            };
        } else if state == ClaimState::Contradictory {
            f.item_mut().ingredients = Fact::Known {
                value: Text("wheat flour".into()),
            };
            f.item_mut().allergens = Fact::Known {
                value: Text("gluten free".into()),
            };
        }
        let mut m = current_meaning(&s, &d, &f);
        m.requirement_claims[0].state = state;
        let mut a = admission();
        f.refresh(&mut m, &mut a);
        assert_eq!(
            outcome(&assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).unwrap()),
            expected
        );
    }
}
#[test]
fn m06_m07_exact_variant_and_observation_digest_binding() {
    let (s, d) = base();
    let p = policy();
    let f = ReadFixture::new();
    let mut m = meaning();
    let mut a = admission();
    m.product.variant_id = Nullable(Some(Text("V1".into())));
    a.meaning_digest = meaning_digest(&m).unwrap();
    assert_eq!(
        assess(&s, &d, &p, &f, Some(&m), Some(&a), now())
            .unwrap_err()
            .code,
        ErrorCode::IdentityMismatch
    );
    let mut m = meaning();
    m.observation_digest = Digest("a".repeat(64));
    a.meaning_digest = meaning_digest(&m).unwrap();
    assert_eq!(
        assess(&s, &d, &p, &f, Some(&m), Some(&a), now())
            .unwrap_err()
            .code,
        ErrorCode::DigestMismatch
    );
}
#[test]
fn m08_expiry_is_exclusive_and_withdrawn_admission_is_unresolved() {
    let (s, d) = base();
    let p = policy();
    let f = ReadFixture::new();
    let m = meaning();
    let a = admission();
    for at in [
        a.valid_until.clone(),
        Instant("2026-12-28T13:00:01Z".into()),
    ] {
        assert_eq!(
            outcome(&assess(&s, &d, &p, &f, Some(&m), Some(&a), at).unwrap()),
            "Unresolved"
        );
    }
    assert_eq!(
        outcome(&assess(&s, &d, &p, &f, Some(&m), None, now()).unwrap()),
        "Unresolved"
    );
}
#[test]
fn m13_cooking_change_and_missing_later_use_never_support() {
    let (s, d) = base();
    let p = policy();
    let f = ReadFixture::new();
    for missing in [false, true] {
        let mut m = meaning();
        if missing {
            m.cooking_claims.pop();
        } else {
            m.cooking_claims[1].state = ClaimState::Refuted;
        }
        let mut a = admission();
        a.meaning_digest = meaning_digest(&m).unwrap();
        assert_eq!(
            outcome(&assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).unwrap()),
            "Unresolved"
        );
    }
}
#[test]
fn m14_unknown_commercial_facts_survive_suitable_candidate() {
    let (s, d) = base();
    let p = policy();
    let mut f = ReadFixture::new();
    let mut m = meaning();
    let mut a = admission();
    f.item_mut().price = Price::Unknown {
        reason: UnknownReason::Missing,
    };
    f.item_mut().availability = Availability::Unknown;
    f.item_mut().pack.net = Fact::Unknown {
        reason: UnknownReason::Missing,
    };
    f.refresh(&mut m, &mut a);
    let result = assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).unwrap();
    let MatchAssessment::Suitable(c) = result else {
        panic!("{result:?}")
    };
    assert_eq!(c.observation(), f.item());
}
#[test]
fn m15_irrelevant_claim_unknown_does_not_veto() {
    let (s, d) = base();
    let p = policy();
    let f = ReadFixture::new();
    let mut m = meaning();
    m.requirement_claims[0].state = ClaimState::Unknown;
    let mut a = admission();
    a.meaning_digest = meaning_digest(&m).unwrap();
    assert_eq!(
        outcome(&assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).unwrap()),
        "Suitable"
    );
}
#[test]
fn m16_missing_policy_cannot_turn_precautionary_text_into_support() {
    let (s, d) = gf_base();
    let mut p = policy();
    p.rules.clear();
    p.digest = policy_digest(&p).unwrap();
    let mut f = ReadFixture::new();
    f.item_mut().allergens = Fact::Known {
        value: Text("may contain milk".into()),
    };
    let mut m = current_meaning(&s, &d, &f);
    let mut a = admission();
    a.policy_digest = p.digest.clone();
    f.refresh(&mut m, &mut a);
    assert_eq!(
        outcome(&assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).unwrap()),
        "Unresolved"
    );
}
#[test]
fn m17_missing_optional_meaning_vs_malformed_closure() {
    let (s, d) = base();
    let p = policy();
    let f = ReadFixture::new();
    assert_eq!(
        outcome(&assess(&s, &d, &p, &f, None, None, now()).unwrap()),
        "Unresolved"
    );
    let mut bad = d.clone();
    bad.demand.lines[0].origin_batches.0.clear();
    assert!(prepare_match_context(&s.context, &s, &bad, &p, now()).is_err());
    let mut m = meaning();
    m.key_claim.evidence_ids.clear();
    assert!(meaning_digest(&m).is_err());
}
#[test]
fn m18_duplicate_claims_products_and_nested_bounds_reject() {
    let (s, d) = base();
    let p = policy();
    let mut f = ReadFixture::new();
    let mut m = meaning();
    let a = admission();
    m.requirement_claims.push(m.requirement_claims[0].clone());
    assert!(meaning_digest(&m).is_err());
    if let ReadBody::Products { items, .. } = &mut f.observation.body {
        items.0 = (0..257)
            .map(|i| {
                let mut p = items[0].clone();
                p.product.product_id = Text(format!("synthetic-{i:03}"));
                p
            })
            .collect();
    }
    assert_eq!(
        assess(&s, &d, &p, &f, Some(&meaning()), Some(&a), now())
            .unwrap_err()
            .code,
        ErrorCode::LimitExceeded
    );
    let mut p = policy();
    p.rules = vec![p.rules[0].clone(); 257];
    assert_eq!(
        policy_digest(&p).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
    let mut m = meaning();
    m.key_claim.evidence_ids = (0..257).map(|i| pin(i).evidence_id).collect();
    assert_eq!(
        meaning_digest(&m).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
}
#[test]
fn m19_current_restrictions_change_local_result_generic_request_unchanged() {
    let mut initial = saved_value();
    require(&mut initial, &[gf()]);
    add_diet_review(&mut initial);
    initial["context"]["privacy"]["include_household_requirements"] = serde_json::json!(false);
    initial["context"]["household"]["connections"]["privacy"]["include_household_requirements"] =
        serde_json::json!(false);
    let (s, d) = rebuild(initial);
    let p = policy();
    let f = ReadFixture::new();
    let request = canonical_bytes(&f.request).unwrap();
    let m = current_meaning(&s, &d, &f);
    let mut a = admission();
    a.meaning_digest = meaning_digest(&m).unwrap();
    assert_eq!(
        outcome(&assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).unwrap()),
        "Suitable"
    );
    let mut v = saved_value();
    require(&mut v, &[milk()]);
    add_diet_review(&mut v);
    v["context"]["household"]["connections"]["privacy"]["include_household_requirements"] =
        serde_json::json!(false);
    v["context"]["privacy"]["include_household_requirements"] = serde_json::json!(false);
    let (new, nd) = rebuild(v);
    let nm = current_meaning(&new, &nd, &f);
    a.meaning_digest = meaning_digest(&nm).unwrap();
    assert_eq!(
        outcome(&assess(&new, &nd, &p, &f, Some(&nm), Some(&a), now()).unwrap()),
        "Unresolved"
    );
    assert_eq!(canonical_bytes(&f.request).unwrap(), request);
    assert!(prepare_match_context(&new.context, &s, &d, &p, now()).is_err());
}
#[test]
fn m20_independent_refutation_retains_unresolved_reason() {
    let (s, d) = gf_base();
    let p = policy();
    let f = ReadFixture::new();
    let mut m = current_meaning(&s, &d, &f);
    m.key_claim.state = ClaimState::Refuted;
    m.requirement_claims[0].state = ClaimState::Unknown;
    let mut a = admission();
    a.meaning_digest = meaning_digest(&m).unwrap();
    let result = assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).unwrap();
    let MatchAssessment::Unsuitable(issues) = result else {
        panic!("{result:?}")
    };
    assert!(
        issues
            .iter()
            .any(|i| i.reason == MatchReason::RefutedIdentity)
    );
    assert!(
        issues
            .iter()
            .any(|i| i.reason == MatchReason::UnknownRequirement)
    );
}
#[test]
fn m21_m24_m29_m30_f40_direction_and_factor_join_preserve_demand() {
    for (factors, reverse, expected) in [
        (vec![3], false, "Suitable"),
        (vec![], false, "Unresolved"),
        (vec![3], true, "Unresolved"),
        (vec![3, 2], false, "Unresolved"),
        (vec![3, 3], false, "Suitable"),
    ] {
        let mut v = saved_value();
        conversion(&mut v, &factors, reverse);
        let (s, d) = rebuild(v);
        let before = canonical_bytes(&d.demand).unwrap();
        let p = policy();
        let mut f = ReadFixture::new();
        f.item_mut().pack.net = Fact::Known {
            value: Quantity {
                amount: PositiveRational::new(500, 1).unwrap(),
                unit: Unit::G,
                form: IngredientForm::Dry,
            },
        };
        let mut m = current_meaning(&s, &d, &f);
        m.key_claim.supply_key.form = IngredientForm::Dry;
        let mut a = admission();
        f.refresh(&mut m, &mut a);
        let result = assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).unwrap();
        assert_eq!(
            outcome(&result),
            expected,
            "factors {factors:?} reverse {reverse}"
        );
        if let MatchAssessment::Suitable(c) = result {
            assert_eq!(
                c.basis().line.quantity.amount,
                PositiveRational::new(300, 1).unwrap()
            );
            assert_eq!(c.basis().line.quantity.form, IngredientForm::Cooked);
            assert_eq!(c.basis().origins.len(), 1);
            let SupplyRelation::ForwardConversion { evidence } = &c.basis().origins[0].relation
            else {
                panic!()
            };
            assert_eq!(evidence.len(), factors.len());
            assert_eq!(evidence[0].0.factor, PositiveRational::new(3, 1).unwrap());
        }
        assert_eq!(canonical_bytes(&d.demand).unwrap(), before);
    }
}
#[test]
fn m27_selected_freshness_role_and_all_four_times_bind() {
    let b = basis();
    let mut changed = b.clone();
    changed.freshness_rule = pin(103);
    assert_eq!(
        match_basis_digest(&changed).unwrap().0,
        "7d105899b9a44b1e64c98c316e8c782fe4ae386a67ddc6956d97c3cddb74b796"
    );
    assert_ne!(
        match_basis_digest(&b).unwrap(),
        match_basis_digest(&changed).unwrap()
    );
    for n in 0..4 {
        let mut x = b.clone();
        match n {
            0 => x.item_observed_at = Instant("2026-12-28T11:00:01Z".into()),
            1 => x.read_observed_at = Instant("2026-12-28T12:00:01Z".into()),
            2 => x.read_admitted_at = Instant("2026-12-28T12:01:01Z".into()),
            _ => x.meaning_admitted_at = Instant("2026-12-28T12:02:01Z".into()),
        };
        assert_ne!(
            match_basis_digest(&x).unwrap(),
            match_basis_digest(&b).unwrap()
        );
    }
}
#[test]
fn m28_meaning_admission_cannot_precede_read_admission() {
    let (s, d) = base();
    let p = policy();
    let f = ReadFixture::new();
    let m = meaning();
    let mut a = admission();
    a.meaning_admitted_at = Instant("2026-12-28T12:00:30Z".into());
    assert!(assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).is_err());
}
#[test]
fn m31_missing_mandatory_conversion_pin_errors_before_context() {
    let mut v = saved_value();
    conversion(&mut v, &[3], false);
    let (mut s, d) = rebuild(v);
    s.evidence.recipes[0]
        .admitted
        .retain(|p| p.evidence_id != pin(501).evidence_id);
    s.evidence.catalogue_digest = food_evidence_digest(&s.evidence).unwrap();
    saved_basis_digest(&s).unwrap();
    let err = prepare_match_context(&s.context, &s, &d, &policy(), now()).unwrap_err();
    assert_eq!(err.code, ErrorCode::MissingReference);
    assert!(err.issues.iter().any(|i| i.path.contains("conversion")));
}
#[test]
fn m09_m10_downstream_guest_governs_whole_line_and_cannot_be_filtered() {
    let (s, d) = rebuild(guest_value());
    let mut p = policy();
    let f = ReadFixture::new();
    let mut m = current_meaning(&s, &d, &f);
    let mut a = admission();
    a.meaning_digest = meaning_digest(&m).unwrap();
    assert_eq!(
        outcome(&assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).unwrap()),
        "Unresolved"
    );
    p.rules.push(RuleBinding {
        requirement: milk(),
        required_fields: vec![SourceField::Ingredients, SourceField::Allergens],
        review: pin(100),
    });
    p.rules
        .sort_by_key(|r| canonical_bytes(&r.requirement).unwrap());
    p.digest = policy_digest(&p).unwrap();
    a.policy_digest = p.digest.clone();
    let mut claim = m.requirement_claims[0].clone();
    claim.requirement = milk();
    claim.state = ClaimState::Refuted;
    m.requirement_claims.push(claim);
    m.requirement_claims
        .sort_by_key(|c| canonical_bytes(&c.requirement).unwrap());
    a.meaning_digest = meaning_digest(&m).unwrap();
    let result = assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).unwrap();
    assert_eq!(outcome(&result), "Unsuitable");
    let MatchAssessment::Unsuitable(issues) = result else {
        panic!()
    };
    assert!(issues.iter().any(|i| i.requirement == Some(milk())
        && i.uses.iter().any(|u| matches!(
            u,
            UseKey::Consumption {
                person: PersonRef::Guest { .. },
                ..
            }
        ))));
    let mut bad = s.context.clone();
    for r in &mut bad.requirements.0 {
        r.attendees
            .0
            .retain(|p| !matches!(p.person, PersonRef::Guest { .. }));
        r.shared_hard_requirements.0.retain(|r| *r != milk());
    }
    assert!(prepare_match_context(&bad, &s, &d, &p, now()).is_err());
}
#[test]
fn m25_m26_mixed_origins_retain_individual_quantities_factors() {
    for complete in [false, true] {
        let (s, d) = rebuild(mixed_value(complete));
        assert_eq!(d.demand.lines.len(), 1);
        assert_eq!(
            d.demand.lines[0].quantity.amount,
            PositiveRational::new(500, 1).unwrap()
        );
        let p = policy();
        let mut f = ReadFixture::new();
        f.item_mut().pack.net = Fact::Known {
            value: Quantity {
                amount: PositiveRational::new(500, 1).unwrap(),
                unit: Unit::G,
                form: IngredientForm::Dry,
            },
        };
        let mut m = current_meaning(&s, &d, &f);
        m.key_claim.supply_key.form = IngredientForm::Dry;
        let mut a = admission();
        f.refresh(&mut m, &mut a);
        let result = assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).unwrap();
        assert_eq!(
            outcome(&result),
            if complete { "Suitable" } else { "Unresolved" }
        );
        if let MatchAssessment::Suitable(c) = result {
            assert_eq!(c.basis().origins.len(), 2);
            for (o, (amount, factor)) in c.basis().origins.iter().zip([(300, 3), (200, 2)]) {
                assert_eq!(
                    o.original_quantity.amount,
                    PositiveRational::new(amount, 1).unwrap()
                );
                let SupplyRelation::ForwardConversion { evidence } = &o.relation else {
                    panic!()
                };
                assert_eq!(
                    evidence[0].0.factor,
                    PositiveRational::new(factor, 1).unwrap()
                );
            }
        }
    }
}
#[test]
fn m11_extra_only_consumer_avoids_shared_vegetarian_restriction() {
    let (s, d) = rebuild(extra_value());
    let line = d
        .demand
        .lines
        .iter()
        .find(|l| l.ingredient_id == IngredientId("ig_0000000000000000000000000000012e".into()))
        .unwrap();
    let p = policy();
    let f = ReadFixture::new();
    let mut m = current_meaning(&s, &d, &f);
    m.key_claim.supply_key = DemandKey {
        ingredient_id: line.ingredient_id.clone(),
        unit: line.quantity.unit,
        form: line.quantity.form,
    };
    let graph = mstrchf_core::demand::reconstruct_use_graph(&d, &s).unwrap();
    let scopes: Vec<_> = graph
        .uses
        .iter()
        .filter(|u| u.line_id == line.line_id)
        .collect();
    let os: Vec<_> = scopes
        .iter()
        .map(|u| match &u.use_key {
            UseKey::Consumption { occurrence, .. } => occurrence,
            UseKey::Preparation { phase } => &phase.occurrence,
        })
        .collect();
    m.cooking_claims.retain(|c| os.contains(&&c.occurrence));
    let mut a = admission();
    a.meaning_digest = meaning_digest(&m).unwrap();
    let c = prepare_match_context(&s.context, &s, &d, &p, now()).unwrap();
    let source = f.source();
    let r = assess_product_meaning(&source, Some(&m), Some(&a), &c).unwrap();
    let result = match_product(&c, &line.line_id, &r).unwrap();
    let MatchAssessment::Suitable(candidate) = result else {
        panic!("{result:?}")
    };
    assert_eq!(candidate.basis().origins.len(), 1);
    assert!(
        candidate
            .basis()
            .uses
            .iter()
            .all(|u| u.requirements.is_empty())
    );
    assert!(
        candidate
            .basis()
            .uses
            .iter()
            .filter_map(|u| match &u.use_key {
                UseKey::Consumption { person, .. } => Some(person),
                _ => None,
            })
            .all(|p| *p
                == PersonRef::Diner {
                    diner_id: DinerId("dn_00000000000000000000000000000002".into())
                })
    );
}
#[test]
fn m22_foreign_ingredient_declaration_cannot_cover_rice() {
    let mut v = saved_value();
    conversion(&mut v, &[3], false);
    let mut foreign = v["context"]["recipes"][0]["ingredients"][0].clone();
    foreign["ingredient_id"] = serde_json::json!("ig_0000000000000000000000000000012e");
    foreign["conversion_evidence"][0]["ingredient_id"] = foreign["ingredient_id"].clone();
    foreign["quantity"]["form"] = serde_json::json!("raw");
    let id = foreign["ingredient_id"].clone();
    v["context"]["recipes"][0]["ingredients"][0]["conversion_evidence"] = serde_json::json!([]);
    v["context"]["recipes"][0]["ingredients"]
        .as_array_mut()
        .unwrap()
        .push(foreign);
    v["context"]["recipes"][0]["components"][0]["ingredient_ids"]
        .as_array_mut()
        .unwrap()
        .push(id.clone());
    for key in ["steps", "prep"] {
        for row in v["context"]["recipes"][0][key].as_array_mut().unwrap() {
            row["ingredient_ids"]
                .as_array_mut()
                .unwrap()
                .push(id.clone());
        }
    }
    let (s, d) = rebuild(v);
    let p = policy();
    let mut f = ReadFixture::new();
    f.item_mut().pack.net = Fact::Unknown {
        reason: UnknownReason::Missing,
    };
    let mut m = current_meaning(&s, &d, &f);
    m.key_claim.supply_key.form = IngredientForm::Dry;
    let mut a = admission();
    f.refresh(&mut m, &mut a);
    assert_eq!(
        outcome(&assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).unwrap()),
        "Unresolved"
    );
}
#[test]
fn m23_other_recipe_conversion_cannot_cover_first_origin() {
    let mut v = mixed_value(true);
    v["context"]["recipes"][0]["ingredients"][0]["conversion_evidence"] = serde_json::json!([]);
    let (s, d) = rebuild(v);
    let p = policy();
    let mut f = ReadFixture::new();
    f.item_mut().pack.net = Fact::Unknown {
        reason: UnknownReason::Missing,
    };
    let mut m = current_meaning(&s, &d, &f);
    m.key_claim.supply_key.form = IngredientForm::Dry;
    let mut a = admission();
    f.refresh(&mut m, &mut a);
    assert_eq!(
        outcome(&assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).unwrap()),
        "Unresolved"
    );
}
#[test]
fn complete_line_requirement_union_is_bounded_before_hashing() {
    let mut b = basis();
    b.uses[0].requirements = (1..=64)
        .map(|n| HardRequirement::IngredientExclusion {
            ingredient_id: IngredientId(format!("ig_{n:032x}")),
        })
        .collect();
    b.uses[1].requirements = vec![HardRequirement::IngredientExclusion {
        ingredient_id: IngredientId(format!("ig_{:032x}", 65)),
    }];
    assert_eq!(
        match_basis_digest(&b).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
}
#[test]
fn native_bounds_and_canonical_collections_reject_before_favourable_assessment() {
    let (s, d) = base();
    let p = policy();
    let f = ReadFixture::new();
    let m = meaning();
    let mut a = admission();
    a.evidence = (1..=4097).map(pin).collect();
    assert_eq!(
        assess(&s, &d, &p, &f, Some(&m), Some(&a), now())
            .unwrap_err()
            .code,
        ErrorCode::LimitExceeded
    );
    let mut m = meaning();
    m.requirement_claims = vec![m.requirement_claims[0].clone(); 65];
    assert_eq!(
        meaning_digest(&m).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
    let mut m = meaning();
    m.cooking_claims = vec![m.cooking_claims[0].clone(); 513];
    assert_eq!(
        meaning_digest(&m).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
    let mut m = meaning();
    m.key_claim.source_fields = vec![SourceField::Ingredients, SourceField::Ingredients];
    assert!(meaning_digest(&m).is_err());
    let mut m = meaning();
    m.requirement_claims[0].source_fields = vec![SourceField::Allergens, SourceField::Ingredients];
    assert!(meaning_digest(&m).is_err());
    let mut m = meaning();
    m.cooking_claims.reverse();
    assert!(meaning_digest(&m).is_err());
    let mut a = admission();
    a.evidence.reverse();
    assert!(assess(&s, &d, &p, &f, Some(&meaning()), Some(&a), now()).is_err());
    let mut b = basis();
    b.origins = vec![b.origins[0].clone(); 16385];
    assert_eq!(
        match_basis_digest(&b).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
    let mut b = basis();
    b.uses = vec![b.uses[0].clone(); 16385];
    assert_eq!(
        match_basis_digest(&b).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
    let mut bad = d.clone();
    bad.demand.lines.0 = vec![bad.demand.lines[0].clone(); 4097];
    assert_eq!(
        prepare_match_context(&s.context, &s, &bad, &p, now())
            .unwrap_err()
            .code,
        ErrorCode::LimitExceeded
    );
}
#[test]
fn missing_or_changed_named_evidence_pins_cannot_self_admit() {
    let (s, d) = gf_base();
    let p = policy();
    let f = ReadFixture::new();
    let m = current_meaning(&s, &d, &f);
    let mut a = admission();
    a.meaning_digest = meaning_digest(&m).unwrap();
    for n in [101, 102] {
        let mut a = a.clone();
        a.evidence.retain(|q| q.evidence_id != pin(n).evidence_id);
        assert!(assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).is_err());
    }
    let mut no_rule = a.clone();
    no_rule
        .evidence
        .retain(|q| q.evidence_id != pin(100).evidence_id);
    assert_eq!(
        outcome(&assess(&s, &d, &p, &f, Some(&m), Some(&no_rule), now()).unwrap()),
        "Unresolved"
    );
    let mut changed = a.clone();
    changed.evidence[1].digest = Digest("f".repeat(64));
    assert!(assess(&s, &d, &p, &f, Some(&m), Some(&changed), now()).is_err());
    let mut missing_claim = m.clone();
    missing_claim.key_claim.evidence_ids = vec![pin(110).evidence_id];
    a.meaning_digest = meaning_digest(&missing_claim).unwrap();
    assert!(assess(&s, &d, &p, &f, Some(&missing_claim), Some(&a), now()).is_err());
}
#[test]
fn malformed_direct_native_wrappers_and_overflow_do_not_reach_support() {
    let (s, d) = base();
    let p = policy();
    let m = meaning();
    let a = admission();
    for (n, den) in [
        (0, 1),
        (1, 0),
        (2, 2),
        (1_000_000_000_001, 1),
        (1, 1_000_001),
        (u64::MAX, u64::MAX),
    ] {
        let mut f = ReadFixture::new();
        f.item_mut().pack.net = Fact::Known {
            value: Quantity {
                amount: PositiveRational(Rational {
                    numerator: DecimalU64(n),
                    denominator: DecimalU64(den),
                }),
                unit: Unit::G,
                form: IngredientForm::Raw,
            },
        };
        assert!(
            assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).is_err(),
            "{n}/{den}"
        );
        let mut b = basis();
        b.origins[0].original_quantity.amount = PositiveRational(Rational {
            numerator: DecimalU64(n),
            denominator: DecimalU64(den),
        });
        assert!(match_basis_digest(&b).is_err());
    }
    let mut f = ReadFixture::new();
    f.item_mut().pack.count = Fact::Known {
        value: BoundedCount(0),
    };
    assert!(assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).is_err());
    let mut b = basis();
    b.line.quantity.amount = PositiveRational::new(MAX_NUMERATOR, 1).unwrap();
    b.origins[0].original_quantity.amount = b.line.quantity.amount;
    let mut second = b.origins[0].clone();
    second.batch_id = BatchId(format!("ba_{:032x}", 2));
    b.origins.push(second);
    assert!(match_basis_digest(&b).is_err());
    let j: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/shopping/matching/match-forward.canonical.json"
    ))
    .unwrap();
    let mut b: MatchBasis = shopping_matching_support::native::read(&j["material"]);
    let SupplyRelation::ForwardConversion { evidence } = &mut b.origins[0].relation else {
        panic!()
    };
    evidence[0].0.factor = PositiveRational(Rational {
        numerator: DecimalU64(1),
        denominator: DecimalU64(0),
    });
    assert!(match_basis_digest(&b).is_err());
}
#[test]
fn same_ref_changed_recipe_bytes_and_reused_reviewed_context_reject() {
    let (s, d) = base();
    let p = policy();
    let f = ReadFixture::new();
    let m = meaning();
    let a = admission();
    let mut bad = s.clone();
    bad.context.recipes[0].ingredients[0]
        .quantity
        .0
        .as_mut()
        .unwrap()
        .amount = PositiveRational::new(201, 1).unwrap();
    assert!(prepare_match_context(&bad.context, &bad, &d, &p, now()).is_err());
    let c = prepare_match_context(&s.context, &s, &d, &p, now()).unwrap();
    let source = f.source();
    let reviewed = assess_product_meaning(&source, Some(&m), Some(&a), &c).unwrap();
    let newer = prepare_match_context(
        &s.context,
        &s,
        &d,
        &p,
        Instant("2026-12-28T12:06:00Z".into()),
    )
    .unwrap();
    assert_eq!(
        match_product(&newer, &d.demand.lines[0].line_id, &reviewed)
            .unwrap_err()
            .code,
        ErrorCode::ContextStale
    );
}
#[test]
fn m15_whole_egg_ignores_drained_and_unrelated_vegan_unknown() {
    let mut v = saved_value();
    v["context"]["recipes"][0]["ingredients"][0]["quantity"] = serde_json::json!({"amount":{"numerator":"1","denominator":"1"},"unit":"each","form":"whole"});
    v["context"]["recipes"][0]["ingredients"][0]["discrete"] = serde_json::json!(true);
    let (s, d) = rebuild(v);
    let p = policy();
    let mut f = ReadFixture::new();
    f.item_mut().pack.net = Fact::Known {
        value: Quantity {
            amount: PositiveRational::new(6, 1).unwrap(),
            unit: Unit::Each,
            form: IngredientForm::Whole,
        },
    };
    f.item_mut().pack.drained = Fact::Unknown {
        reason: UnknownReason::Missing,
    };
    let mut m = current_meaning(&s, &d, &f);
    m.requirement_claims[0].requirement = HardRequirement::Diet {
        code: DietCode::Vegan,
    };
    m.requirement_claims[0].state = ClaimState::Unknown;
    let mut a = admission();
    f.refresh(&mut m, &mut a);
    assert_eq!(
        outcome(&assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).unwrap()),
        "Suitable"
    );
}
#[test]
fn m16_gf_support_does_not_cover_unruled_milk_precaution() {
    let (s, d) = rebuild(guest_value());
    let p = policy();
    let mut f = ReadFixture::new();
    f.item_mut().allergens = Fact::Known {
        value: Text("may contain milk".into()),
    };
    let mut m = current_meaning(&s, &d, &f);
    let mut a = admission();
    f.refresh(&mut m, &mut a);
    assert_eq!(
        outcome(&assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).unwrap()),
        "Unresolved"
    );
}
#[test]
fn malformed_native_timestamps_return_errors_without_panicking() {
    let (s, d) = base();
    let p = policy();
    let m = meaning();
    let a = admission();
    for n in 0..5 {
        let mut f = ReadFixture::new();
        let mut a = a.clone();
        match n {
            0 => f.item_mut().observed_at = Instant("bad".into()),
            1 => f.observation.observed_at = Instant("bad".into()),
            2 => f.accepted.admitted_at = Instant("bad".into()),
            3 => a.meaning_admitted_at = Instant("bad".into()),
            _ => f.request.created_at = Instant("bad".into()),
        };
        assert!(
            assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).is_err(),
            "timestamp case {n}"
        );
    }
}
#[path = "../../../tests/fixtures/food/batches/f11_fixture.rs"]
mod remix_fixture;
#[test]
fn remix_new_ingredients_have_own_origin_and_base_keeps_all_later_consumers() {
    let s = remix_fixture::fixture();
    let feasibility = remix_fixture::feasibility();
    let d = derive_plan(
        &s.plan,
        &s.context,
        &FoodInputs {
            evidence: &s.evidence,
            identities: &s.identities,
            schedule: &s.schedule,
            feasibility: &feasibility,
        },
    )
    .unwrap()
    .plan;
    let p = policy();
    for (id, expected_uses, expected_consumers) in [(401, 6, 3), (405, 2, 1), (406, 2, 1)] {
        let line = d
            .demand
            .lines
            .iter()
            .find(|l| l.ingredient_id == IngredientId(format!("ig_{id:032x}")))
            .unwrap();
        let mut f = ReadFixture::new();
        f.item_mut().pack.net = Fact::Known {
            value: line.quantity.clone(),
        };
        let mut m = current_meaning(&s, &d, &f);
        m.key_claim.supply_key = DemandKey {
            ingredient_id: line.ingredient_id.clone(),
            unit: line.quantity.unit,
            form: line.quantity.form,
        };
        let mut a = admission();
        f.refresh(&mut m, &mut a);
        let context = prepare_match_context(&s.context, &s, &d, &p, now()).unwrap();
        let source = f.source();
        let reviewed = assess_product_meaning(&source, Some(&m), Some(&a), &context).unwrap();
        let result = match_product(&context, &line.line_id, &reviewed).unwrap();
        let MatchAssessment::Suitable(c) = result else {
            panic!("{result:?}")
        };
        assert_eq!(c.basis().uses.len(), expected_uses);
        assert_eq!(
            c.basis()
                .uses
                .iter()
                .filter(|u| matches!(u.use_key, UseKey::Consumption { .. }))
                .count(),
            expected_consumers
        );
        assert_eq!(c.basis().origins.len(), 1);
        assert_eq!(c.basis().origins[0].original_quantity, line.quantity);
    }
}
#[test]
fn original_oracle_bytes_have_literal_join_expectations() {
    let original: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/food/oracle-v1.json"
    ))
    .unwrap();
    let frozen: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/shopping/matching/expected-outcomes.json"
    ))
    .unwrap();
    assert_eq!(frozen["cases"].as_array().unwrap().len(), 31);
    for expected in frozen["original_oracles"].as_array().unwrap() {
        let current = original["fixtures"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["name"] == expected["name"])
            .unwrap();
        assert_eq!(current, expected);
    }
    assert_eq!(
        frozen["original_oracles"][0]["input"]["conversion"]["original_demand"]["amount"],
        "300"
    );
    assert_eq!(
        frozen["original_oracles"][1]["expected"]["suitable_product_ids"],
        serde_json::json!([])
    );
}
#[test]
fn independent_forward_and_mixed_canonical_encoder_vectors() {
    for (bytes, expected) in [
        (
            include_bytes!(
                "../../../tests/fixtures/shopping/matching/match-forward.canonical.json"
            )
            .as_slice(),
            "c58729f733be047a2f1ae2884d5827bb9451c0cdd133a8207ceab1846d2da667",
        ),
        (
            include_bytes!("../../../tests/fixtures/shopping/matching/match-mixed.canonical.json")
                .as_slice(),
            "d56d1af3cc2dc63c9b6f3439312b7731e9679e82e9e078ccf3b4b7d6b90ef6b2",
        ),
    ] {
        let j: serde_json::Value = serde_json::from_slice(bytes).unwrap();
        let b: MatchBasis = shopping_matching_support::native::read(&j["material"]);
        assert_eq!(match_basis_material(&b).unwrap(), j["material"]);
        assert_eq!(match_basis_digest(&b).unwrap().0, expected);
    }
}
#[test]
fn current_complete_match_matches_independent_corrected_vector() {
    let (s, d) = base();
    let p = policy();
    let f = ReadFixture::new();
    let m = meaning();
    let a = admission();
    let result = assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).unwrap();
    let MatchAssessment::Suitable(c) = result else {
        panic!("{result:?}")
    };
    let expected: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/shopping/matching/match-current.canonical.json"
    ))
    .unwrap();
    assert_eq!(
        match_basis_material(c.basis()).unwrap(),
        expected["material"]
    );
}
#[test]
fn identity_declared_relevant_fields_override_positive_claim_without_dietary_requirements() {
    let (s, d) = base();
    let p = policy();
    for conflicting in [false, true] {
        let mut f = ReadFixture::new();
        f.item_mut().allergens = if conflicting {
            Fact::Conflicting {
                values: BoundedVec(vec![
                    Text("SYNTHETIC identity A".into()),
                    Text("SYNTHETIC identity B".into()),
                ]),
            }
        } else {
            Fact::Unknown {
                reason: UnknownReason::Missing,
            }
        };
        let mut m = meaning();
        m.key_claim.source_fields = vec![SourceField::Ingredients, SourceField::Allergens];
        let mut a = admission();
        f.refresh(&mut m, &mut a);
        assert_eq!(
            outcome(&assess(&s, &d, &p, &f, Some(&m), Some(&a), now()).unwrap()),
            "Unresolved"
        );
    }
}
