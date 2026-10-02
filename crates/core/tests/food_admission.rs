use mstrchf_contracts::*;
use mstrchf_core::recipes::*;

fn recipe() -> RecipeRevision {
    parse_document(include_bytes!(
        "../../../tests/fixtures/food/admission/complete-synthetic.json"
    ))
    .unwrap()
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
fn inputs(
    r: &RecipeRevision,
    class: Option<ComponentClass>,
) -> (EvidenceBasis, ComponentClassView) {
    let rf = reference(r);
    let pins = (1..=8)
        .map(|n| EvidencePin {
            evidence_id: eid(n),
            revision: Revision(1),
            digest: Digest(format!("{n:064x}")),
        })
        .collect();
    let basis = EvidenceBasis::new(rf.clone(), pins, vec![AssetDigest("a".repeat(64))]).unwrap();
    let components = class
        .map(|class| {
            vec![ComponentClassEvidence {
                recipe: rf.clone(),
                component_id: r.components[0].component_id.clone(),
                class,
                evidence_ids: vec![eid(8)],
            }]
        })
        .unwrap_or_default();
    (basis, ComponentClassView::new(rf, components).unwrap())
}
fn error<T: std::fmt::Debug>(result: Result<T, DomainError>, code: ErrorCode, path: &str) {
    let e = result.unwrap_err();
    assert_eq!(e.code, code, "{e:?}");
    assert!(e.issues.iter().any(|i| i.path == path), "{e:?}");
}
fn oracle(name: &str) -> serde_json::Value {
    let o: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/food/oracle-v1.json"
    ))
    .unwrap();
    let cases = o["fixtures"].as_array().unwrap();
    assert_eq!(cases.len(), 55);
    let found: Vec<_> = cases
        .iter()
        .filter(|c| c["name"].as_str().unwrap().starts_with(name))
        .collect();
    assert_eq!(found.len(), 1);
    found[0].clone()
}

#[test]
fn food_f52_incomplete_candidate() {
    let o = oracle("F52_");
    let mut c: RecipeCandidate = parse_document(include_bytes!(
        "../../../tests/fixtures/contracts/recipe-candidate.json"
    ))
    .unwrap();
    c.ingredients.0.push(CandidateIngredient {
        ingredient_id: Nullable(Some(IngredientId(format!("ig_{:032x}", 1)))),
        name: Text("Synthetic beans".into()),
        quantity: Nullable(None),
        component_id: Nullable(None),
        source_text: Text("SYNTHETIC unknown quantity".into()),
    });
    let v = validate_recipe(&c).unwrap();
    assert!(v.capabilities.local_display);
    assert_eq!(
        v.capabilities.shopping,
        o["expected"]["shopping"].as_bool().unwrap()
    );
    assert!(!v.capabilities.leftovers && !v.capabilities.starter_distribution);
    assert!(
        v.capabilities
            .blocking_issues
            .iter()
            .any(|i| i.code == CandidateIssueCode::MissingQuantity
                && i.path.0 == "/ingredients/0/quantity")
    );
    assert!(v.issues.iter().any(|i| i.path == "/yield"));
    assert!(o["expected"]["nutrition"].is_null());
}
#[test]
fn food_f52_strict_import_claims_reject() {
    let c: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/contracts/recipe-candidate.json"
    ))
    .unwrap();
    for key in ["capabilities", "cook_test"] {
        let mut bad = c.clone();
        bad[key] = serde_json::json!(true);
        assert_eq!(
            parse_document::<RecipeCandidate>(&serde_json::to_vec(&bad).unwrap())
                .unwrap_err()
                .code,
            ErrorCode::UnknownField
        );
    }
}
#[test]
fn food_f52_automated_cook_test_rejects() {
    let mut r = recipe();
    r.reviews[0].kind = ReviewKind::CookTest;
    r.reviews[0].actor = ReviewActor::Automated;
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    assert_eq!(
        assess_revision(&r, &b, &c).unwrap_err().code,
        ErrorCode::AuthorityDenied
    );
}
#[test]
fn food_complete_revision_positive() {
    let r = recipe();
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    let v = assess_revision(&r, &b, &c).unwrap();
    assert!(
        v.assessment().capabilities.shopping
            && v.assessment().capabilities.leftovers
            && v.assessment().capabilities.starter_distribution
    );
    assert!(std::ptr::eq(v.recipe(), &r));
    assert!(std::ptr::eq(v.component_classes(), &c));
    assert!(v.recipe().nutrition.0.is_none());
}
#[test]
fn food_f53_reference_preservation() {
    let o = oracle("F53_");
    let mut r1 = recipe();
    r1.ingredients[0].quantity.0.as_mut().unwrap().amount = PositiveRational::new(480, 1).unwrap();
    r1.ingredients[0].quantity.0.as_mut().unwrap().form = IngredientForm::Drained;
    let original = serde_json::to_vec(&r1).unwrap();
    let approved = reference(&r1);
    let mut r2 = r1.clone();
    r2.revision = Revision(2);
    r2.ingredients[0].quantity.0.as_mut().unwrap().amount = PositiveRational::new(600, 1).unwrap();
    for r in [&r1, &r2] {
        let (b, c) = inputs(r, Some(ComponentClass::General));
        assess_revision(r, &b, &c).unwrap();
    }
    assert_eq!(r1.recipe_id, r2.recipe_id);
    assert_ne!(approved, reference(&r2));
    assert_eq!(approved, reference(&r1));
    assert_eq!(original, serde_json::to_vec(&r1).unwrap());
    assert_eq!(
        r1.ingredients[0].quantity.0.as_ref().unwrap().amount,
        PositiveRational::new(
            o["expected"]["approved_beans"]["amount"]
                .as_str()
                .unwrap()
                .parse()
                .unwrap(),
            1
        )
        .unwrap()
    );
    assert_eq!(
        r2.ingredients[0].quantity.0.as_ref().unwrap().amount,
        PositiveRational::new(600, 1).unwrap()
    );
    assert_eq!(r1.text_rights, r2.text_rights);
    assert_eq!(r1.image, r2.image);
    assert_eq!(
        r1.text_rights.attribution.0,
        o["expected"]["text_attribution"].as_str().unwrap()
    );
    assert_eq!(
        r1.image.0.as_ref().unwrap().rights.attribution.0,
        o["expected"]["image_attribution"].as_str().unwrap()
    );
}
#[test]
fn food_class_general_rice_unknown() {
    for seconds in [172800, 64800] {
        for class in [
            ComponentClass::General,
            ComponentClass::Rice,
            ComponentClass::Unknown,
        ] {
            let mut r = recipe();
            r.storage[0].max_chilled_seconds = seconds;
            r.capabilities.leftovers = class != ComponentClass::Unknown;
            let (b, c) = inputs(&r, Some(class));
            let v = assess_revision(&r, &b, &c).unwrap();
            assert_eq!(
                v.assessment().capabilities.leftovers,
                class != ComponentClass::Unknown
            );
            assert!(v.assessment().capabilities.shopping);
            assert_eq!(v.component_classes().components[0].class, class);
            assert_eq!(
                v.component_classes().components[0].evidence_ids,
                vec![eid(8)]
            );
            // Actual 48h/24h/18h scheduled-use boundary checks belong to C, not A.
        }
    }
}
#[test]
fn food_class_missing_preserves_fresh() {
    for class in [None, Some(ComponentClass::Unknown)] {
        let mut r = recipe();
        r.capabilities.leftovers = false;
        let (b, c) = inputs(&r, class);
        let v = assess_revision(&r, &b, &c).unwrap();
        assert!(v.assessment().capabilities.shopping);
        assert!(!v.assessment().capabilities.leftovers);
        assert_eq!(
            v.component_classes().components.len(),
            usize::from(class.is_some())
        );
    }
    let mut r = recipe();
    r.storage.0.clear();
    r.components[0].storage_rule_ids.0.clear();
    r.capabilities.leftovers = false;
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    let v = assess_revision(&r, &b, &c).unwrap();
    assert!(v.assessment().capabilities.shopping);
    assert!(!v.assessment().capabilities.leftovers);
}
#[test]
fn food_class_capability_mismatch() {
    for class in [None, Some(ComponentClass::Unknown)] {
        let r = recipe();
        let before = serde_json::to_vec(&r).unwrap();
        let (b, c) = inputs(&r, class);
        error(
            assess_revision(&r, &b, &c),
            ErrorCode::DomainConflict,
            "/capabilities",
        );
        assert_eq!(before, serde_json::to_vec(&r).unwrap());
    }
    for flag in 0..4 {
        let mut r = recipe();
        match flag {
            0 => r.capabilities.local_display = false,
            1 => r.capabilities.shopping = false,
            2 => r.capabilities.leftovers = false,
            _ => r.capabilities.starter_distribution = false,
        };
        let (b, c) = inputs(&r, Some(ComponentClass::General));
        error(
            assess_revision(&r, &b, &c),
            ErrorCode::DomainConflict,
            "/capabilities",
        );
    }
}
#[test]
fn food_class_closure_rejects() {
    let r = recipe();
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    assess_revision(&r, &b, &c).unwrap();
    let mut bad = c.clone();
    bad.components = vec![c.components[0].clone(); 129];
    error(
        assess_revision(&r, &b, &bad),
        ErrorCode::LimitExceeded,
        "/classes/components",
    );
    let mut bad = c.clone();
    bad.components.push(c.components[0].clone());
    error(
        ComponentClassView::new(bad.recipe, bad.components),
        ErrorCode::InvalidValue,
        "/classes/components",
    );
    let mut bad = c.clone();
    bad.components[0].component_id = ComponentId(format!("co_{:032x}", 99));
    error(
        assess_revision(&r, &b, &bad),
        ErrorCode::ForeignReference,
        "/classes/components/0/component_id",
    );
    for i in 0..6 {
        let mut bad = c.clone();
        let rf = if i < 3 {
            &mut bad.recipe
        } else {
            &mut bad.components[0].recipe
        };
        match i % 3 {
            0 => rf.recipe_id = RecipeId(format!("rc_{:032x}", 99)),
            1 => rf.revision = Revision(2),
            _ => rf.digest = RecipeDigest("f".repeat(64)),
        };
        assert!(assess_revision(&r, &b, &bad).is_err());
    }
    let mut bad = c.clone();
    bad.components[0].evidence_ids = vec![eid(99)];
    error(
        assess_revision(&r, &b, &bad),
        ErrorCode::MissingReference,
        "/classes/components/0/evidence_ids/0",
    );
    let mut bad = c.clone();
    bad.components[0].evidence_ids.clear();
    error(
        ComponentClassView::new(bad.recipe, bad.components),
        ErrorCode::MissingReference,
        "/classes/components/0/evidence_ids",
    );
}
#[test]
fn food_native_constructor_bounds_order_and_revalidation() {
    let r = recipe();
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    let mut bad = b.clone();
    bad.admitted = vec![b.admitted[0].clone(); 257];
    error(
        EvidenceBasis::new(bad.recipe.clone(), bad.admitted.clone(), vec![]),
        ErrorCode::LimitExceeded,
        "/basis/admitted",
    );
    error(
        assess_revision(&r, &bad, &c),
        ErrorCode::LimitExceeded,
        "/basis/admitted",
    );
    let mut bad = b.clone();
    bad.admitted.swap(0, 1);
    error(
        EvidenceBasis::new(bad.recipe, bad.admitted, vec![]),
        ErrorCode::NoncanonicalValue,
        "/basis/admitted",
    );
    let mut bad = b.clone();
    bad.admitted[1] = bad.admitted[0].clone();
    error(
        assess_revision(&r, &bad, &c),
        ErrorCode::InvalidValue,
        "/basis/admitted",
    );
    error(
        EvidenceBasis::new(
            b.recipe.clone(),
            vec![],
            vec![AssetDigest("a".repeat(64)); 513],
        ),
        ErrorCode::LimitExceeded,
        "/basis/verified_asset_digests",
    );
    error(
        EvidenceBasis::new(
            b.recipe.clone(),
            vec![],
            vec![AssetDigest("a".repeat(64)); 2],
        ),
        ErrorCode::InvalidValue,
        "/basis/verified_asset_digests",
    );
    let mut bad = c.clone();
    bad.components[0].evidence_ids = vec![eid(8); 257];
    error(
        assess_revision(&r, &b, &bad),
        ErrorCode::LimitExceeded,
        "/classes/components/0/evidence_ids",
    );
}
#[test]
fn food_invalid_constructed_primitive() {
    let r = recipe();
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    for i in 0..3 {
        let mut bad = b.clone();
        match i {
            0 => bad.admitted[0].evidence_id = EvidenceId("invalid".into()),
            1 => bad.admitted[0].revision = Revision(0),
            _ => bad.admitted[0].digest = Digest("bad".into()),
        };
        assert!(assess_revision(&r, &bad, &c).is_err());
    }
    let mut bad = r.clone();
    bad.revision = Revision(0);
    assert!(assess_revision(&bad, &b, &c).is_err());
    let mut bad = r.clone();
    bad.ingredients[0].quantity.0.as_mut().unwrap().amount = PositiveRational(Rational {
        numerator: DecimalU64(2),
        denominator: DecimalU64(2),
    });
    assert!(assess_revision(&bad, &b, &c).is_err());
    let mut bad = r.clone();
    bad.ingredients.0 = vec![r.ingredients[0].clone(); 257];
    assert!(assess_revision(&bad, &b, &c).is_err());
}
#[test]
fn food_admission_forged_review() {
    let r = recipe();
    let (mut b, c) = inputs(&r, Some(ComponentClass::General));
    b.admitted.retain(|p| p.evidence_id != eid(2));
    error(
        assess_revision(&r, &b, &c),
        ErrorCode::MissingReference,
        "/reviews/1/evidence_id",
    );
}
#[test]
fn food_admission_evidence_revision_changed() {
    let r = recipe();
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    let mut changed = r.clone();
    changed.revision = Revision(2);
    error(
        assess_revision(&changed, &b, &c),
        ErrorCode::IdentityMismatch,
        "/basis/recipe",
    );
    changed = r.clone();
    changed.ingredients[0].quantity.0.as_mut().unwrap().amount =
        PositiveRational::new(600, 1).unwrap();
    error(
        assess_revision(&changed, &b, &c),
        ErrorCode::DigestMismatch,
        "/basis/recipe/digest",
    );
    // Pin contents are trusted native premises; currentness is the external admission boundary's job.
}
#[test]
fn food_missing_quantity_review_disables_shopping() {
    let mut r = recipe();
    r.reviews.0.retain(|x| x.kind != ReviewKind::QuantityReview);
    r.capabilities.shopping = false;
    r.capabilities.leftovers = false;
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    let v = assess_revision(&r, &b, &c).unwrap();
    assert!(!v.assessment().capabilities.shopping);
    assert!(v.assessment().issues.iter().any(|i| i.path == "/reviews"));
}
#[test]
fn food_missing_quantity_disables_shopping() {
    let mut r = recipe();
    r.ingredients[0].quantity = Nullable(None);
    r.capabilities.shopping = false;
    r.capabilities.leftovers = false;
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    assert!(
        !assess_revision(&r, &b, &c)
            .unwrap()
            .assessment()
            .capabilities
            .shopping
    );
}
#[test]
fn food_missing_image_or_rights_only_disables_distribution() {
    for i in 0..4 {
        let mut r = recipe();
        r.capabilities.starter_distribution = false;
        match i {
            0 => r.image = Nullable(None),
            1 => r.text_rights.distribution = RightsDistribution::PersonalOnly,
            2 => r.image.0.as_mut().unwrap().rights.distribution = RightsDistribution::Unresolved,
            _ => r.reviews.0.retain(|x| x.kind != ReviewKind::SourceReview),
        };
        let (b, c) = inputs(&r, Some(ComponentClass::General));
        let v = assess_revision(&r, &b, &c).unwrap();
        assert!(v.assessment().capabilities.shopping);
        assert!(!v.assessment().capabilities.starter_distribution);
    }
    let mut r = recipe();
    r.capabilities.starter_distribution = false;
    let (mut b, c) = inputs(&r, Some(ComponentClass::General));
    b.verified_asset_digests.clear();
    let v = assess_revision(&r, &b, &c).unwrap();
    assert!(!v.assessment().capabilities.starter_distribution);
}
#[test]
fn food_dietary_review_closure() {
    let mut r = recipe();
    r.dietary_evidence[0].review_id = eid(2);
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    error(
        assess_revision(&r, &b, &c),
        ErrorCode::ForeignReference,
        "/dietary_evidence/0/review_id",
    );
}
#[test]
fn food_no_name_inferred_classification() {
    let mut r = recipe();
    r.title = Title("Rice rice rice".into());
    r.capabilities.leftovers = false;
    let (b, c) = inputs(&r, None);
    assert!(
        !assess_revision(&r, &b, &c)
            .unwrap()
            .assessment()
            .capabilities
            .leftovers
    );
}

#[test]
fn food_native_upper_bounds_and_empty_views() {
    let r = recipe();
    let rf = reference(&r);
    let pins = (1..=256)
        .map(|n| EvidencePin {
            evidence_id: eid(n),
            revision: Revision(1),
            digest: Digest(format!("{n:064x}")),
        })
        .collect();
    let assets = (1..=512)
        .map(|n| AssetDigest(format!("{n:064x}")))
        .collect();
    EvidenceBasis::new(rf.clone(), pins, assets).unwrap();
    EvidenceBasis::new(rf.clone(), vec![], vec![]).unwrap();
    ComponentClassView::new(rf.clone(), vec![]).unwrap();
    let facts = (1..=128)
        .map(|n| ComponentClassEvidence {
            recipe: rf.clone(),
            component_id: ComponentId(format!("co_{n:032x}")),
            class: ComponentClass::General,
            evidence_ids: (1..=256).map(eid).collect(),
        })
        .collect();
    assert_eq!(
        ComponentClassView::new(rf, facts).unwrap().components.len(),
        128
    );
}
#[test]
fn food_class_direct_fields_revalidated() {
    let r = recipe();
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    let mut bad = c.clone();
    bad.components.push(c.components[0].clone());
    error(
        assess_revision(&r, &b, &bad),
        ErrorCode::InvalidValue,
        "/classes/components",
    );
    let mut bad = c.clone();
    bad.components[0].evidence_ids.clear();
    error(
        assess_revision(&r, &b, &bad),
        ErrorCode::MissingReference,
        "/classes/components/0/evidence_ids",
    );
    let mut bad = c.clone();
    bad.components[0].evidence_ids = vec![eid(8), eid(8)];
    error(
        assess_revision(&r, &b, &bad),
        ErrorCode::InvalidValue,
        "/classes/components/0/evidence_ids",
    );
    let mut bad = c.clone();
    bad.components[0].evidence_ids = vec![eid(8), eid(7)];
    error(
        assess_revision(&r, &b, &bad),
        ErrorCode::NoncanonicalValue,
        "/classes/components/0/evidence_ids",
    );
    let mut bad = c.clone();
    bad.components[0].evidence_ids = vec![EvidenceId("bad".into())];
    assert!(assess_revision(&r, &b, &bad).is_err());
    let mut bad = c.clone();
    bad.components[0].class = ComponentClass::Unknown;
    bad.components[0].evidence_ids.clear();
    ComponentClassView::new(bad.recipe, bad.components).unwrap();
}
#[test]
fn food_basis_ref_and_asset_primitives_revalidated() {
    let r = recipe();
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    for i in 0..3 {
        let mut bad = b.clone();
        match i {
            0 => bad.recipe.recipe_id = RecipeId(format!("rc_{:032x}", 99)),
            1 => bad.recipe.revision = Revision(2),
            _ => bad.recipe.digest = RecipeDigest("f".repeat(64)),
        };
        assert!(assess_revision(&r, &bad, &c).is_err());
    }
    let mut bad = b.clone();
    bad.verified_asset_digests = vec![AssetDigest("invalid".into())];
    assert!(assess_revision(&r, &bad, &c).is_err());
    let mut bad = b.clone();
    bad.verified_asset_digests = vec![AssetDigest("b".repeat(64)), AssetDigest("a".repeat(64))];
    error(
        assess_revision(&r, &bad, &c),
        ErrorCode::NoncanonicalValue,
        "/basis/verified_asset_digests",
    );
}
#[test]
fn food_multiple_components_require_every_class() {
    let mut r = recipe();
    let co2 = ComponentId(format!("co_{:032x}", 2));
    let mut component = r.components[0].clone();
    component.component_id = co2.clone();
    component.ingredient_ids.0.clear();
    component.storage_rule_ids = BoundedVec(vec![eid(9)]);
    r.components.0.push(component);
    let mut yielded = r.r#yield.components[0].clone();
    yielded.component_id = co2.clone();
    r.r#yield.components.0.push(yielded);
    let mut storage = r.storage[0].clone();
    storage.component_id = co2.clone();
    storage.evidence_id = eid(9);
    r.storage.0.push(storage);
    r.capabilities.leftovers = false;
    let (mut b, c) = inputs(&r, Some(ComponentClass::General));
    b.admitted.push(EvidencePin {
        evidence_id: eid(9),
        revision: Revision(1),
        digest: Digest("9".repeat(64)),
    });
    let v = assess_revision(&r, &b, &c).unwrap();
    assert!(v.assessment().capabilities.shopping);
    assert!(!v.assessment().capabilities.leftovers);
    r.capabilities.leftovers = true;
    let (mut b, mut c) = inputs(&r, Some(ComponentClass::General));
    b.admitted.push(EvidencePin {
        evidence_id: eid(9),
        revision: Revision(1),
        digest: Digest("9".repeat(64)),
    });
    c.components.push(ComponentClassEvidence {
        recipe: reference(&r),
        component_id: co2,
        class: ComponentClass::Rice,
        evidence_ids: vec![eid(8)],
    });
    assert!(
        assess_revision(&r, &b, &c)
            .unwrap()
            .assessment()
            .capabilities
            .leftovers
    );
    c.components.swap(0, 1);
    error(
        assess_revision(&r, &b, &c),
        ErrorCode::NoncanonicalValue,
        "/classes/components",
    );
}
#[test]
fn food_required_reheat_without_permission_disables_reuse() {
    let mut r = recipe();
    r.storage[0].max_reheats = 0;
    r.portability.reheating_required = true;
    r.capabilities.leftovers = false;
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    let v = assess_revision(&r, &b, &c).unwrap();
    assert!(v.assessment().capabilities.shopping);
    assert!(!v.assessment().capabilities.leftovers);
    r.storage[0].max_reheats = 1;
    r.capabilities.leftovers = true;
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    assert!(
        assess_revision(&r, &b, &c)
            .unwrap()
            .assessment()
            .capabilities
            .leftovers
    );
    r.storage[0].max_reheats = 0;
    r.portability.reheating_required = false;
    r.portability.cold_suitable = true;
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    assert!(
        assess_revision(&r, &b, &c)
            .unwrap()
            .assessment()
            .capabilities
            .leftovers
    );
}
#[test]
fn food_all_missing_quantity_paths_reported() {
    let mut r = recipe();
    let mut ingredient = r.ingredients[0].clone();
    ingredient.ingredient_id = IngredientId(format!("ig_{:032x}", 2));
    r.components[0]
        .ingredient_ids
        .0
        .push(ingredient.ingredient_id.clone());
    r.steps[0]
        .ingredient_ids
        .0
        .push(ingredient.ingredient_id.clone());
    r.ingredients.0.push(ingredient);
    for ingredient in &mut r.ingredients.0 {
        ingredient.quantity = Nullable(None);
    }
    r.capabilities.shopping = false;
    r.capabilities.leftovers = false;
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    let v = assess_revision(&r, &b, &c).unwrap();
    for path in ["/ingredients/0/quantity", "/ingredients/1/quantity"] {
        assert!(
            v.assessment().issues.iter().any(|i| i.path == path),
            "missing {path}"
        );
    }
}
#[test]
fn food_evidence_closure_covers_all_admitted_claims() {
    let r = recipe();
    for (id, path) in [
        (1, "/reviews/0/evidence_id"),
        (3, "/reviews/2/evidence_id"),
        (4, "/dietary_evidence/0/evidence_id"),
        (5, "/portion_presets/0/evidence_ids/0"),
        (6, "/storage/0/evidence_id"),
        (7, "/image/rights/evidence_ids/0"),
        (8, "/classes/components/0/evidence_ids/0"),
    ] {
        let (mut b, c) = inputs(&r, Some(ComponentClass::General));
        b.admitted.retain(|p| p.evidence_id != eid(id));
        error(
            assess_revision(&r, &b, &c),
            ErrorCode::MissingReference,
            path,
        );
    }
}
#[test]
fn food_rational_bounds_and_invalid_scaling() {
    for (n, d) in [
        (0, 1),
        (1, 0),
        (2, 2),
        (1_000_000_000_001, 1),
        (1, 1_000_001),
    ] {
        let mut r = recipe();
        r.ingredients[0].quantity.0.as_mut().unwrap().amount = PositiveRational(Rational {
            numerator: DecimalU64(n),
            denominator: DecimalU64(d),
        });
        let (b, c) = inputs(&r, Some(ComponentClass::General));
        assert!(assess_revision(&r, &b, &c).is_err());
    }
    let mut r = recipe();
    r.scaling = ScalingPolicy::Linear {
        minimum: PositiveRational::new(2, 1).unwrap(),
        maximum: PositiveRational::new(1, 1).unwrap(),
    };
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    assert!(assess_revision(&r, &b, &c).is_err());
    let mut r = recipe();
    r.ingredients[0].quantity.0.as_mut().unwrap().amount =
        PositiveRational::new(1_000_000_000_000, 1).unwrap();
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    assess_revision(&r, &b, &c).unwrap();
}
#[test]
fn food_candidate_diagnostics_do_not_silently_truncate() {
    let mut c: RecipeCandidate = parse_document(include_bytes!(
        "../../../tests/fixtures/contracts/recipe-candidate.json"
    ))
    .unwrap();
    c.unresolved = BoundedVec(
        (0..256)
            .map(|i| CandidateIssue {
                code: CandidateIssueCode::UnsupportedRequirement,
                path: Text(format!("/synthetic/{i}")),
                evidence_ids: BoundedVec(vec![]),
            })
            .collect(),
    );
    error(
        validate_recipe(&c),
        ErrorCode::LimitExceeded,
        "/capabilities/blocking_issues",
    );
}
#[test]
fn food_oracle_bytes_remain_pinned() {
    assert_eq!(
        bytes_digest(include_bytes!(
            "../../../tests/fixtures/food/oracle-v1.json"
        ))
        .0,
        "ba50ac3ff55ea90aae006d2883cfe21ab1c9667b5ccfdd185f2e967263c8b092"
    );
}

#[test]
fn food_imported_diagnostics_are_not_authority() {
    let mut r = recipe();
    r.capabilities.leftovers = false;
    let original = serde_json::to_vec(&r).unwrap();
    let (b, c) = inputs(&r, None);
    let baseline = assess_revision(&r, &b, &c).unwrap().assessment().clone();
    assert!(
        baseline
            .capabilities
            .blocking_issues
            .iter()
            .any(|i| i.code == CandidateIssueCode::MissingStorage)
    );
    assert!(!baseline.issues.is_empty());
    for code in [
        CandidateIssueCode::MissingImage,
        CandidateIssueCode::MissingQuantity,
    ] {
        let mut changed = r.clone();
        changed.capabilities.blocking_issues.0.push(CandidateIssue {
            code,
            path: Text("/synthetic/imported_claim".into()),
            evidence_ids: BoundedVec(vec![]),
        });
        let before = serde_json::to_vec(&changed).unwrap();
        let (b, c) = inputs(&changed, None);
        let assessed = assess_revision(&changed, &b, &c).unwrap();
        assert_eq!(assessed.assessment(), &baseline);
        assert!(assessed.assessment().capabilities.shopping);
        assert_eq!(before, serde_json::to_vec(&changed).unwrap());
    }
    assert_eq!(original, serde_json::to_vec(&r).unwrap());
    let mut full = recipe();
    full.capabilities.blocking_issues.0.push(CandidateIssue {
        code: CandidateIssueCode::MissingQuantity,
        path: Text("/ingredients".into()),
        evidence_ids: BoundedVec(vec![]),
    });
    let (b, c) = inputs(&full, Some(ComponentClass::General));
    let v = assess_revision(&full, &b, &c).unwrap();
    assert!(v.assessment().capabilities.shopping);
    assert!(v.assessment().capabilities.blocking_issues.is_empty());
}
#[test]
fn food_imported_diagnostic_shape_still_rejects() {
    let mut r = recipe();
    r.capabilities.blocking_issues.0.push(CandidateIssue {
        code: CandidateIssueCode::MissingQuantity,
        path: Text("".into()),
        evidence_ids: BoundedVec(vec![]),
    });
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    assert!(assess_revision(&r, &b, &c).is_err());
}
#[test]
fn food_nested_bounds_checked_before_shape_traversal() {
    let mut r = recipe();
    r.title = Title("".into());
    r.source.authors.0 = vec![Text("SYNTHETIC".into()); 65];
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    error(
        assess_revision(&r, &b, &c),
        ErrorCode::LimitExceeded,
        "/source/authors",
    );
    let mut c: RecipeCandidate = parse_document(include_bytes!(
        "../../../tests/fixtures/contracts/recipe-candidate.json"
    ))
    .unwrap();
    c.title = Title("".into());
    c.source.authors.0 = vec![Text("SYNTHETIC".into()); 65];
    error(
        validate_recipe(&c),
        ErrorCode::LimitExceeded,
        "/source/authors",
    );
}

#[test]
fn food_unknown_nutrition_subfields_remain_absent() {
    let mut r = recipe();
    r.nutrition = Nullable(Some(NutritionEvidence {
        source: r.source.clone(),
        per_servings: PositiveRational::new(1, 1).unwrap(),
        protein_g: Nullable(None),
        energy_kcal: Nullable(None),
        estimated: true,
    }));
    let before = serde_json::to_vec(&r).unwrap();
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    let v = assess_revision(&r, &b, &c).unwrap();
    let n = v.recipe().nutrition.0.as_ref().unwrap();
    assert!(n.protein_g.0.is_none() && n.energy_kcal.0.is_none());
    assert_eq!(before, serde_json::to_vec(&r).unwrap());
}
#[test]
fn food_admitted_automated_review_is_not_a_cook_observation() {
    let mut r = recipe();
    r.reviews[1].actor = ReviewActor::Automated;
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    let v = assess_revision(&r, &b, &c).unwrap();
    assert!(v.assessment().capabilities.shopping);
    assert!(
        !v.recipe()
            .reviews
            .iter()
            .any(|r| r.kind == ReviewKind::CookTest)
    );
}
#[test]
fn food_diagnostics_are_sorted_and_bounded() {
    let mut r = recipe();
    r.image = Nullable(None);
    r.capabilities.starter_distribution = false;
    r.capabilities.shopping = false;
    r.capabilities.leftovers = false;
    r.ingredients[0].quantity = Nullable(None);
    let (b, c) = inputs(&r, None);
    let v = assess_revision(&r, &b, &c).unwrap();
    let a = v.assessment();
    assert!(a.issues.windows(2).all(|p| p[0].path <= p[1].path));
    assert!(
        a.capabilities
            .blocking_issues
            .windows(2)
            .all(|p| (&p[0].path, p[0].code) <= (&p[1].path, p[1].code))
    );
    let mut c: RecipeCandidate = parse_document(include_bytes!(
        "../../../tests/fixtures/contracts/recipe-candidate.json"
    ))
    .unwrap();
    c.unresolved = BoundedVec(
        (0..101)
            .map(|i| CandidateIssue {
                code: CandidateIssueCode::UnsupportedRequirement,
                path: Text(format!("/synthetic/{i}")),
                evidence_ids: BoundedVec(vec![]),
            })
            .collect(),
    );
    error(validate_recipe(&c), ErrorCode::LimitExceeded, "/issues");
}
#[test]
fn food_class_missing_rule_links_and_instruction_shape() {
    let mut r = recipe();
    r.components[0].storage_rule_ids.0.clear();
    r.capabilities.leftovers = false;
    let (b, c) = inputs(&r, Some(ComponentClass::General));
    error(
        assess_revision(&r, &b, &c),
        ErrorCode::DomainConflict,
        "/components/0/storage_rule_ids",
    );
    for field in 0..2 {
        let mut r = recipe();
        if field == 0 {
            r.storage[0].cooling_instruction = Text("".into());
        } else {
            r.storage[0].reheating_instruction = Text("".into());
        }
        let (b, c) = inputs(&r, Some(ComponentClass::General));
        assert!(assess_revision(&r, &b, &c).is_err());
    }
}

fn two_storage_rules(reverse: bool) -> RecipeRevision {
    let mut r = recipe();
    r.portability.reheating_required = true;
    r.portability.evidence_ids = BoundedVec(vec![eid(10)]);
    let mut strict = r.storage[0].clone();
    strict.evidence_id = eid(9);
    strict.max_reheats = 0;
    strict.max_chilled_seconds = 64800;
    r.storage.0.push(strict);
    if reverse {
        r.storage.0.reverse();
    }
    r
}

fn two_storage_inputs(
    r: &RecipeRevision,
    class: Option<ComponentClass>,
) -> (EvidenceBasis, ComponentClassView) {
    let (mut basis, classes) = inputs(r, class);
    basis.admitted.extend((9..=10).map(|n| EvidencePin {
        evidence_id: eid(n),
        revision: Revision(1),
        digest: Digest(format!("{n:064x}")),
    }));
    (basis, classes)
}

#[test]
fn food_r1_partial_storage_links_reject_both_orders_and_claims() {
    for reverse in [false, true] {
        for claimed_leftovers in [true, false] {
            for class in [
                Some(ComponentClass::General),
                Some(ComponentClass::Unknown),
                None,
            ] {
                let mut r = two_storage_rules(reverse);
                r.capabilities.leftovers = claimed_leftovers;
                let before = serde_json::to_vec(&r).unwrap();
                let (basis, classes) = two_storage_inputs(&r, class);
                error(
                    assess_revision(&r, &basis, &classes),
                    ErrorCode::DomainConflict,
                    "/components/0/storage_rule_ids",
                );
                assert_eq!(before, serde_json::to_vec(&r).unwrap());
            }
        }
    }
}

#[test]
fn food_r1_complete_storage_links_preserve_reheat_and_cold_controls() {
    for reverse in [false, true] {
        let mut r = two_storage_rules(reverse);
        r.components[0].storage_rule_ids.0.push(eid(9));
        r.capabilities.leftovers = false;
        let before = serde_json::to_vec(&r).unwrap();
        let (basis, classes) = two_storage_inputs(&r, Some(ComponentClass::General));
        let reviewed = assess_revision(&r, &basis, &classes).unwrap();
        assert!(reviewed.assessment().capabilities.shopping);
        assert!(!reviewed.assessment().capabilities.leftovers);
        assert_eq!(before, serde_json::to_vec(&r).unwrap());

        r.capabilities.leftovers = true;
        let (basis, classes) = two_storage_inputs(&r, Some(ComponentClass::General));
        error(
            assess_revision(&r, &basis, &classes),
            ErrorCode::DomainConflict,
            "/capabilities",
        );

        r.portability.reheating_required = false;
        r.portability.cold_suitable = true;
        let (basis, classes) = two_storage_inputs(&r, Some(ComponentClass::General));
        let reviewed = assess_revision(&r, &basis, &classes).unwrap();
        assert!(reviewed.assessment().capabilities.shopping);
        assert!(reviewed.assessment().capabilities.leftovers);
        assert_eq!(reviewed.recipe().storage.len(), 2);
    }
}
