//! Isolated diagnostics for an unadmitted real-source personal-use packet.
//! Supplying diagnostic pins exercises the core; it never creates caller authority.
use mstrchf_contracts::*;
use mstrchf_core::{eligibility::Eligibility, recipes::*};
use serde_json::Value;

fn resources() -> (Value, Value, Value, Value, RecipeRevision) {
    let packet: Value = serde_json::from_slice(include_bytes!(
        "../../../sites/mstrchf-9000/content/intake/porridge-personal-v1/packet.json"
    )).unwrap();
    let docs: Value = serde_json::from_slice(include_bytes!(
        "../../../sites/mstrchf-9000/content/intake/porridge-personal-v1/evidence-documents.json"
    )).unwrap();
    let registry: Value = serde_json::from_slice(include_bytes!(
        "../../../sites/mstrchf-9000/content/intake/porridge-personal-v1/ingredient-registry.json"
    )).unwrap();
    let captures: Value = serde_json::from_slice(include_bytes!(
        "../../../sites/mstrchf-9000/content/intake/porridge-personal-v1/source-captures.json"
    )).unwrap();
    let recipe = parse_document::<RecipeRevision>(
        &serde_json::to_vec(&packet["proposed_recipe_revision"]).unwrap()
    ).unwrap();
    (packet, docs, registry, captures, recipe)
}
fn reference(recipe: &RecipeRevision) -> RecipeRef {
    RecipeRef {recipe_id: recipe.recipe_id.clone(), revision: recipe.revision,
        digest: recipe_digest(recipe).unwrap()}
}
fn basis(packet: &Value, recipe: &RecipeRevision) -> EvidenceBasis {
    let pins = packet["proposed_evidence_pins"].as_array().unwrap().iter().map(|p| EvidencePin {
        evidence_id: serde_json::from_value(p["evidence_id"].clone()).unwrap(),
        revision: serde_json::from_value(p["revision"].clone()).unwrap(),
        digest: serde_json::from_value(p["digest"].clone()).unwrap(),
    }).collect();
    EvidenceBasis::new(reference(recipe), pins, vec![]).unwrap()
}
fn meal() -> MealRequirements {
    let mut meal = parse_document::<MealRequirementsDocumentV1>(include_bytes!(
        "../../../tests/fixtures/contracts/meal-requirements.json"
    )).unwrap().meal_requirements;
    meal.slot.meal_type = MealType::Breakfast;
    meal.attendees.0.truncate(1);
    meal.attendees[0].hard_requirements = BoundedVec(vec![]);
    meal.shared_hard_requirements = BoundedVec(vec![]);
    meal
}
fn overrides() -> MealOverrides {
    MealOverrides {portions: BoundedVec(vec![]), eat_by: Nullable(None)}
}

#[test]
fn independent_rust_digests_bind_recipe_registry_captures_and_each_evidence_document() {
    let (packet, docs, registry, captures, recipe) = resources();
    assert_eq!(packet["runtime_admission"], false);
    assert_eq!(packet["evidence_authority_attested"], false);
    assert_eq!(packet["human_cook_test_performed"], false);
    assert_eq!(packet["recipe_reference"]["digest"], recipe_digest(&recipe).unwrap().0);
    assert_eq!(packet["ingredient_registry_digest"], canonical_digest(&registry).unwrap().0);
    assert_eq!(packet["source_captures_digest"], canonical_digest(&captures).unwrap().0);
    assert_eq!(packet["evidence_documents_digest"], canonical_digest(&docs).unwrap().0);
    let pins = packet["proposed_evidence_pins"].as_array().unwrap();
    assert_eq!(pins.len(), docs.as_array().unwrap().len());
    for (document, pin) in docs.as_array().unwrap().iter().zip(pins) {
        assert_eq!(document["evidence_id"], pin["evidence_id"]);
        assert_eq!(pin["digest"], canonical_digest(document).unwrap().0);
        let mut edited = document.clone();
        edited["status"] = Value::String("forged-reviewed-authority".into());
        assert_ne!(pin["digest"], canonical_digest(&edited).unwrap().0);
    }
    let source = &captures[0];
    assert_eq!(source["revision"], 4606890);
    let source_text = source["wikitext"].as_str().unwrap();
    assert!(source_text.contains("servings=1|time=15 minutes"));
    assert!(source_text.contains("(45 ") && source_text.contains("(180 "));
    assert_eq!(source["wikitext_utf8_sha256"], bytes_digest(source_text.as_bytes()).0);
    assert_eq!(registry["global_registry_admitted"], false);
    for (ingredient, entry) in recipe.ingredients.iter().zip(registry["entries"].as_array().unwrap()) {
        assert_eq!(serde_json::to_value(&ingredient.ingredient_id).unwrap(), entry["ingredient_id"]);
        assert_eq!(serde_json::to_value(ingredient.quantity.0.as_ref().unwrap().form).unwrap(), entry["form"]);
    }
    assert!(registry["entries"][2]["meaning"].as_str().unwrap().contains("NOT mean unpasteurised"));
}

#[test]
fn no_authority_basis_refuses_and_diagnostic_pins_only_prove_exact_quantity_mechanics() {
    let (packet, _, _, _, recipe) = resources();
    let classes = ComponentClassView::new(reference(&recipe), vec![]).unwrap();
    let empty = EvidenceBasis::new(reference(&recipe), vec![], vec![]).unwrap();
    assert_eq!(assess_revision(&recipe, &empty, &classes).unwrap_err().code, ErrorCode::MissingReference);
    let diagnostic = basis(&packet, &recipe);
    let reviewed = assess_revision(&recipe, &diagnostic, &classes).unwrap();
    assert!(reviewed.assessment().capabilities.shopping);
    assert!(!reviewed.assessment().capabilities.leftovers);
    assert!(!reviewed.assessment().capabilities.starter_distribution);
    let scaled = mstrchf_core::portions::scale_recipe(&reviewed, PositiveRational::new(1, 1).unwrap()).unwrap();
    let amounts: Vec<_> = scaled.ingredients.iter().map(|i| {
        (i.quantity.amount.0.numerator.0, i.quantity.amount.0.denominator.0, i.quantity.unit, i.quantity.form)
    }).collect();
    assert_eq!(amounts, vec![(45,1,Unit::G,IngredientForm::Dry),(180,1,Unit::Ml,IngredientForm::Raw),(180,1,Unit::Ml,IngredientForm::Raw)]);
    assert_eq!(scaled.components[0].quantity.amount, PositiveRational::new(1,1).unwrap());
    assert_eq!(scaled.components[0].quantity.unit, Unit::Each);
    for requested in [PositiveRational::new(1,2).unwrap(), PositiveRational::new(2,1).unwrap()] {
        assert_eq!(mstrchf_core::portions::scale_recipe(&reviewed, requested).unwrap_err().code, ErrorCode::UnsupportedCapability);
    }
    assert_eq!(packet["asserted_current_product_capabilities"]["planning"], false);
    assert_eq!(packet["operational_scope"]["policy_status"], "proposed_pending_root_review");
}

#[test]
fn adult_regular_source_anchor_resolves_and_other_ages_and_presets_refuse() {
    let (packet, _, _, _, recipe) = resources();
    let diagnostic = basis(&packet, &recipe);
    let classes = ComponentClassView::new(reference(&recipe), vec![]).unwrap();
    let reviewed = assess_revision(&recipe, &diagnostic, &classes).unwrap();
    let base = meal();
    let servings = mstrchf_core::portions::resolve_servings(&reviewed, &base, &overrides()).unwrap();
    assert_eq!(servings[0].servings, PositiveRational::new(1,1).unwrap());
    for age in [AgeBand::Teen,AgeBand::Child1To3,AgeBand::Child4To6,AgeBand::Child7To10,AgeBand::Child11To12] {
        let mut unsupported = base.clone(); unsupported.attendees[0].age_band = age;
        assert!(mstrchf_core::portions::resolve_servings(&reviewed, &unsupported, &overrides()).is_err());
    }
    for preset in [PortionPresetName::Smaller, PortionPresetName::Larger] {
        let mut unsupported = base.clone(); unsupported.attendees[0].portion.preset = preset;
        assert!(mstrchf_core::portions::resolve_servings(&reviewed, &unsupported, &overrides()).is_err());
    }
    // Explicitly expose the integration seam: resolving an adjustment is not
    // proof of supported scaling. The caller must reject it before use.
    let mut adjusted = base.clone();
    adjusted.attendees[0].portion.adjustment = PositiveRational::new(2,1).unwrap();
    let resolved = mstrchf_core::portions::resolve_servings(&reviewed, &adjusted, &overrides()).unwrap();
    assert_eq!(resolved[0].servings, PositiveRational::new(2,1).unwrap());
    assert!(mstrchf_core::portions::scale_recipe(&reviewed, resolved[0].servings).is_err());
    let mut group = base.clone();
    let mut second = group.attendees[0].clone();
    second.person = PersonRef::Diner {
        diner_id: serde_json::from_value(serde_json::json!("dn_00000000000000000000000000000002")).unwrap()
    };
    group.attendees.0.push(second);
    let group_servings = mstrchf_core::portions::resolve_servings(&reviewed, &group, &overrides()).unwrap();
    assert_eq!(group_servings.len(), 2, "The core portion resolver does not enforce packet attendee limits");
    assert!(group_servings.iter().all(|s| s.servings == PositiveRational::new(1,1).unwrap()));
    assert!(mstrchf_core::portions::scale_recipe(&reviewed, PositiveRational::new(2,1).unwrap()).is_err());
}

#[test]
fn intentional_milk_oats_and_vegan_conflict_while_unknown_free_from_and_other_restrictions_refuse() {
    let (packet, _, _, _, recipe) = resources();
    let diagnostic = basis(&packet, &recipe);
    let classes = ComponentClassView::new(reference(&recipe), vec![]).unwrap();
    let reviewed = assess_revision(&recipe, &diagnostic, &classes).unwrap();
    assert_eq!(mstrchf_core::eligibility::eligible(&reviewed, &meal()).unwrap(), Eligibility::Eligible);
    for requirement in [
        HardRequirement::AllergenExclusion {code:AllergenCode::Milk},
        HardRequirement::AllergenExclusion {code:AllergenCode::CerealsContainingGluten},
        HardRequirement::Diet {code:DietCode::Vegan},
    ] {
        let mut restricted=meal();restricted.shared_hard_requirements=BoundedVec(vec![requirement.clone()]);
        restricted.attendees[0].hard_requirements=BoundedVec(vec![requirement]);
        assert!(matches!(mstrchf_core::eligibility::eligible(&reviewed,&restricted).unwrap(),Eligibility::Ineligible(_)));
    }
    for requirement in [
        HardRequirement::Diet {code:DietCode::GlutenFree},
        HardRequirement::Diet {code:DietCode::Vegetarian},
        HardRequirement::AllergenExclusion {code:AllergenCode::Peanuts},
    ] {
        let mut restricted=meal();restricted.shared_hard_requirements=BoundedVec(vec![requirement.clone()]);
        restricted.attendees[0].hard_requirements=BoundedVec(vec![requirement]);
        assert!(matches!(mstrchf_core::eligibility::eligible(&reviewed,&restricted).unwrap(),Eligibility::Unresolved(_)));
    }
    let mut portable=meal();portable.location=MealLocation::OnTheMove;
    assert!(matches!(mstrchf_core::eligibility::eligible(&reviewed,&portable).unwrap(),Eligibility::Unresolved(_)));
    let mut dinner=meal();dinner.slot.meal_type=MealType::Dinner;
    assert!(matches!(mstrchf_core::eligibility::eligible(&reviewed,&dinner).unwrap(),Eligibility::Ineligible(_)));
}

#[test]
fn only_automated_factual_reviews_exist_and_estimates_cannot_become_cook_test_or_media_evidence() {
    let (packet, _, _, _, recipe) = resources();
    assert_eq!(recipe.reviews.len(),3);
    assert!(recipe.reviews.iter().all(|r| r.actor==ReviewActor::Automated && r.kind!=ReviewKind::CookTest));
    assert!(recipe.image.0.is_none() && recipe.nutrition.0.is_none());
    assert!(recipe.storage.is_empty() && recipe.reused_inputs.is_empty() && recipe.extras.is_empty());
    assert_eq!(packet["operational_scope"]["timing"]["estimated"], true);
    assert_eq!(packet["operational_scope"]["timing"]["measured_active_minutes"], Value::Null);
    assert_eq!(packet["operational_scope"]["timing"]["measured_elapsed_minutes"], Value::Null);
    assert!(recipe.steps[0].text.0.contains("estimate"));
    let mut forged=recipe.clone();forged.reviews[0].kind=ReviewKind::CookTest;
    assert_eq!(parse_document::<RecipeRevision>(&serde_json::to_vec(&forged).unwrap()).unwrap_err().code,ErrorCode::AuthorityDenied);
}
