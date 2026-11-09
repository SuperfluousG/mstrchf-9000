//! Primary-source draft diagnostics only. Nothing is admitted to runtime state.
use mstrchf_contracts::*;
use mstrchf_core::{eligibility::Eligibility, recipes::*};
use serde_json::{Value, json};

#[test]
fn non_admitted_porridge_draft_retains_source_quantities_and_authority_gaps() {
    let package: Value = serde_json::from_slice(include_bytes!(
        "../../../sites/mstrchf-9000/content/intake/2026-10-12-porridge-revision-evidence-draft.json"
    ))
    .unwrap();
    assert_eq!(package["runtime_admission"], false);
    assert_eq!(package["evidence_authority_attested"], false);
    assert_eq!(package["human_calibration_complete"], false);
    let candidate: RecipeCandidate =
        parse_document(&serde_json::to_vec(&package["candidate"]).unwrap()).unwrap();
    let candidate_assessment = validate_recipe(&candidate).unwrap();
    assert!(candidate_assessment.capabilities.local_display);
    assert!(!candidate_assessment.capabilities.shopping);
    assert!(!candidate_assessment.capabilities.leftovers);
    assert!(!candidate_assessment.capabilities.starter_distribution);
    let recipe: RecipeRevision =
        parse_document(&serde_json::to_vec(&package["proposed_recipe_revision"]).unwrap()).unwrap();
    assert!(
        recipe
            .reviews
            .iter()
            .all(|review| review.actor == ReviewActor::Automated)
    );
    assert!(
        recipe
            .reviews
            .iter()
            .all(|review| review.kind != ReviewKind::CookTest)
    );
    assert!(recipe.dietary_evidence.is_empty() && recipe.storage.is_empty());
    assert!(recipe.image.0.is_none() && recipe.nutrition.0.is_none());
    let reference = RecipeRef {
        recipe_id: recipe.recipe_id.clone(),
        revision: recipe.revision,
        digest: recipe_digest(&recipe).unwrap(),
    };
    let classes = ComponentClassView::new(reference.clone(), vec![]).unwrap();
    let empty_basis = EvidenceBasis::new(reference.clone(), vec![], vec![]).unwrap();
    let not_admitted = assess_revision(&recipe, &empty_basis, &classes).unwrap_err();
    assert_eq!(not_admitted.code, ErrorCode::MissingReference);

    // Deliberately supplied diagnostic pins prove the current core mechanics.
    // They do not attest the unresolved policy proposals or create authority.
    let diagnostic_pins: Vec<_> = package["proposed_evidence_documents"]
        .as_array()
        .unwrap()
        .iter()
        .map(|document| EvidencePin {
            evidence_id: serde_json::from_value(document["evidence_id"].clone()).unwrap(),
            revision: Revision(1),
            digest: canonical_digest(document).unwrap(),
        })
        .collect();
    let basis = EvidenceBasis::new(reference.clone(), diagnostic_pins.clone(), vec![]).unwrap();
    let diagnostic = assess_revision(&recipe, &basis, &classes).unwrap();
    let capabilities = &diagnostic.assessment().capabilities;
    assert!(capabilities.local_display && capabilities.shopping);
    assert!(!capabilities.leftovers && !capabilities.starter_distribution);
    let scaled =
        mstrchf_core::portions::scale_recipe(&diagnostic, PositiveRational::new(1, 1).unwrap())
            .unwrap();
    assert_eq!(scaled.components[0].quantity.unit, Unit::Each);
    assert_eq!(scaled.components[0].quantity.form, IngredientForm::Cooked);
    assert_eq!(
        scaled.components[0].quantity.amount,
        PositiveRational::new(1, 1).unwrap()
    );
    assert_eq!(
        scaled
            .ingredients
            .iter()
            .map(|v| (v.quantity.amount.0.numerator.0, v.quantity.unit))
            .collect::<Vec<_>>(),
        vec![(45, Unit::G), (180, Unit::Ml), (180, Unit::Ml)]
    );
    assert_eq!(
        mstrchf_core::portions::scale_recipe(&diagnostic, PositiveRational::new(2, 1).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::UnsupportedCapability
    );

    let mut meal = parse_document::<MealRequirementsDocumentV1>(include_bytes!(
        "../../../tests/fixtures/contracts/meal-requirements.json"
    ))
    .unwrap()
    .meal_requirements;
    meal.slot.meal_type = MealType::Breakfast;
    meal.attendees.0.truncate(1);
    meal.attendees[0].hard_requirements = BoundedVec(vec![]);
    meal.shared_hard_requirements = BoundedVec(vec![]);
    let overrides = MealOverrides {
        portions: BoundedVec(vec![]),
        eat_by: Nullable(None),
    };
    let servings =
        mstrchf_core::portions::resolve_servings(&diagnostic, &meal, &overrides).unwrap();
    assert_eq!(servings[0].servings, PositiveRational::new(1, 1).unwrap());
    assert_eq!(
        mstrchf_core::eligibility::eligible(&diagnostic, &meal).unwrap(),
        Eligibility::Eligible
    );
    meal.attendees[0].portion.preset = PortionPresetName::Smaller;
    assert!(mstrchf_core::portions::resolve_servings(&diagnostic, &meal, &overrides).is_err());
    meal.attendees[0].portion.preset = PortionPresetName::Regular;
    meal.shared_hard_requirements = BoundedVec(vec![HardRequirement::Diet {
        code: DietCode::Vegan,
    }]);
    meal.attendees[0].hard_requirements = meal.shared_hard_requirements.clone();
    assert!(matches!(
        mstrchf_core::eligibility::eligible(&diagnostic, &meal).unwrap(),
        Eligibility::Unresolved(_)
    ));

    let mut forged = recipe.clone();
    forged.reviews[0].kind = ReviewKind::CookTest;
    assert_eq!(
        parse_document::<RecipeRevision>(&serde_json::to_vec(&forged).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::AuthorityDenied
    );
    let mut unknown_time = package["proposed_recipe_revision"].clone();
    unknown_time["steps"][0]["active_minutes"] = Value::Null;
    assert!(parse_document::<RecipeRevision>(&serde_json::to_vec(&unknown_time).unwrap()).is_err());
    let mut no_presets = package["proposed_recipe_revision"].clone();
    no_presets["portion_presets"] = json!([]);
    assert!(parse_document::<RecipeRevision>(&serde_json::to_vec(&no_presets).unwrap()).is_err());

    let report = json!({
        "format":"mstrchf-non-admitted-food-draft-diagnostics",
        "draft":"2026-10-12-porridge-revision-evidence-draft.json",
        "draft_digest":canonical_digest(&package).unwrap(),
        "recipe_reference":reference,
        "runtime_admission":false,
        "evidence_authority_attested":false,
        "candidate_parse":"passed",
        "candidate_capabilities":candidate_assessment.capabilities,
        "recipe_revision_parse":"passed_with_explicit_unadmitted_policy_proposals",
        "empty_authority_basis":not_admitted,
        "diagnostic_evidence_pins":diagnostic_pins.iter().map(|pin| json!({"evidence_id":pin.evidence_id,"revision":pin.revision,"digest":pin.digest})).collect::<Vec<_>>(),
        "diagnostic_core_capabilities":capabilities,
        "source_serving_output":{"amount":"1","unit":"each","form":"cooked","mass_inferred":false},
        "exact_unchanged_ingredients":[{"amount":"45","unit":"g"},{"amount":"180","unit":"ml"},{"amount":"180","unit":"ml"}],
        "adult_regular_1":"core_resolves_diagnostic_anchor_only; semantic_calibration_unresolved",
        "smaller_portion":"refused_unsupported",
        "two_serving_scaling":"refused_unsupported",
        "vegan_requirement":"unresolved_without_dietary_evidence",
        "automated_cook_test":"refused_authority_denied",
        "unknown_numeric_timing":"recipe_revision_schema_refuses_null",
        "absent_portion_presets":"recipe_revision_schema_refuses_empty",
        "production_semantics":"No runtime command, catalogue mutation, production admission or food acceptance performed. Caller evidence authority remains mandatory."
    });
    println!("{}", serde_json::to_string_pretty(&report).unwrap());
    if let Ok(path) = std::env::var("MSTRCHF_DRAFT_DIAGNOSTICS_PATH") {
        std::fs::write(path, serde_json::to_vec_pretty(&report).unwrap()).unwrap();
    }
}
