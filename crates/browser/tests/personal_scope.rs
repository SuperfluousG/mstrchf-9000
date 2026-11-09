//! Tests the unregistered browser domain guard, without runtime admission.
use mstrchf_browser::BoardOptions;
use mstrchf_contracts::*;
use mstrchf_core::recipes::*;
use serde_json::{Value, json};

use mstrchf_browser::personal_scope;
use personal_scope::*;

struct Input {
    recipe: RecipeRevision,
    reference: RecipeRef,
    meal: MealRequirements,
    selection: MealSelection,
    plan: WeekPlan,
    options: BoardOptions,
}
impl Input {
    fn check(&self) -> Result<(), DomainError> {
        check_personal_use_scope(
            &self.recipe,
            &self.reference,
            &self.meal,
            &self.selection,
            &self.plan,
            &self.options,
        )
    }
}
fn one() -> PositiveRational {
    PositiveRational::new(1, 1).unwrap()
}
fn fixture() -> Input {
    let packet: Value = serde_json::from_slice(include_bytes!(
        "../../../sites/mstrchf-9000/content/intake/porridge-personal-reviewed-v1/packet.json"
    ))
    .unwrap();
    let recipe: RecipeRevision =
        parse_document(&serde_json::to_vec(&packet["maker_recipe_revision"]).unwrap()).unwrap();
    let reference: RecipeRef = serde_json::from_value(packet["recipe_reference"].clone()).unwrap();
    let mut meal = parse_document::<MealRequirementsDocumentV1>(include_bytes!(
        "../../../tests/fixtures/contracts/meal-requirements.json"
    ))
    .unwrap()
    .meal_requirements;
    meal.slot.meal_type = MealType::Breakfast;
    meal.attendees.0.truncate(1);
    meal.attendees[0].hard_requirements = BoundedVec(vec![]);
    meal.shared_hard_requirements = BoundedVec(vec![]);
    let selection = MealSelection {
        portable_cold: None,
        slot: meal.slot.clone(),
        recipe: reference.clone(),
        origin: SelectionOrigin::Fixed,
        repeat_authorised: false,
        servings: BoundedVec(vec![DinerServing {
            person: meal.attendees[0].person.clone(),
            preset_ref: PortionPresetRef {
                recipe: reference.clone(),
                preset: PortionPresetName::Regular,
                age_band: AgeBand::Adult,
                meal_type: MealType::Breakfast,
            },
            servings: one(),
        }]),
        extras: BoundedVec(vec![]),
        batch_use: BatchUse::NewBatch {
            batch_id: serde_json::from_value(json!("ba_80000000000000000000000000000001")).unwrap(),
            production_servings: one(),
        },
        meal_overrides: MealOverrides {
            portions: BoundedVec(vec![]),
            eat_by: Nullable(None),
        },
    };
    let plan = WeekPlan {
        week_id: meal.slot.week_id.clone(),
        revision: Revision(1),
        selections: BoundedVec(vec![]),
        batch_links: BoundedVec(vec![]),
    };
    Input {
        recipe,
        reference,
        meal,
        selection,
        plan,
        options: BoardOptions::default(),
    }
}
#[track_caller]
fn refused(input: &Input, field: &str) {
    let error = input.check().unwrap_err();
    assert_eq!(error.retry, Retry::Never);
    assert!(!error.truncated);
    assert_eq!(error.issues.len(), 1);
    assert!(error.issues[0].entity_id.0.is_none());
    assert_eq!(error.issues[0].path, format!("/personal_use_scope/{field}"));
}
fn diagnostic_basis(input: &Input) -> EvidenceBasis {
    let docs: Value = serde_json::from_slice(include_bytes!(
        "../../../sites/mstrchf-9000/content/intake/porridge-personal-reviewed-v1/evidence-documents.json"
    ))
    .unwrap();
    let pins = docs
        .as_array()
        .unwrap()
        .iter()
        .map(|d| EvidencePin {
            evidence_id: serde_json::from_value(d["evidence_id"].clone()).unwrap(),
            revision: Revision(1),
            digest: canonical_digest(d).unwrap(),
        })
        .collect();
    EvidenceBasis::new(input.reference.clone(), pins, vec![]).unwrap()
}

#[test]
fn exact_personal_use_context_passes_without_creating_authority() {
    let mut input = fixture();
    assert!(input.check().is_ok());
    assert!(requires_personal_scope(&input.reference.recipe_id));
    assert_eq!(
        recipe_digest(&input.recipe).unwrap().0,
        PORRIDGE_PERSONAL_SCOPE.recipe_digest
    );
    assert_eq!(PORRIDGE_PERSONAL_SCOPE.attention_reservation_minutes, 15);
    assert!(PORRIDGE_PERSONAL_SCOPE.timing_is_estimate);
    // These are available facilities/permissions, not an actual reused batch.
    input.meal.leftovers_allowed = true;
    input.meal.mobility.reheating = true;
    input.meal.mobility.cold_storage = true;
    input.meal.attendees[0]
        .portion
        .meal_overrides
        .0
        .push(MealPortionOverride {
            meal_type: MealType::Dinner,
            preset: PortionPresetName::Larger,
            adjustment: PositiveRational::new(2, 1).unwrap(),
        });
    assert!(
        input.check().is_ok(),
        "Unrelated dinner preferences cannot alter the breakfast scope"
    );
    let empty = EvidenceBasis::new(input.reference.clone(), vec![], vec![]).unwrap();
    let classes = ComponentClassView::new(input.reference.clone(), vec![]).unwrap();
    assert_eq!(
        assess_revision(&input.recipe, &empty, &classes)
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    );
}

#[test]
fn altered_references_and_recipe_bytes_cannot_keep_the_approved_scope() {
    let prior: Value = serde_json::from_slice(include_bytes!(
        "../../../sites/mstrchf-9000/content/intake/porridge-personal-v1/packet.json"
    ))
    .unwrap();
    let mut old_revision = fixture();
    old_revision.recipe =
        parse_document(&serde_json::to_vec(&prior["proposed_recipe_revision"]).unwrap()).unwrap();
    old_revision.reference = serde_json::from_value(prior["recipe_reference"].clone()).unwrap();
    old_revision.selection.recipe = old_revision.reference.clone();
    old_revision.selection.servings[0].preset_ref.recipe = old_revision.reference.clone();
    assert!(requires_personal_scope(&old_revision.reference.recipe_id));
    refused(&old_revision, "recipe_reference");
    for field in ["id", "revision", "digest"] {
        let mut input = fixture();
        match field {
            "id" => {
                input.reference.recipe_id =
                    serde_json::from_value(json!("rc_80000000000000000000000000000002")).unwrap()
            }
            "revision" => input.reference.revision = Revision(1),
            _ => input.reference.digest = RecipeDigest("f".repeat(64)),
        }
        if field != "id" {
            assert!(requires_personal_scope(&input.reference.recipe_id));
        }
        refused(&input, "recipe_reference");
    }
    let mut edited = fixture();
    edited.recipe.ingredients[0]
        .quantity
        .0
        .as_mut()
        .unwrap()
        .amount = PositiveRational::new(90, 1).unwrap();
    refused(&edited, "recipe");
    let mut selection = fixture();
    selection.selection.recipe.digest = RecipeDigest("f".repeat(64));
    refused(&selection, "recipe_reference");
    let mut preset = fixture();
    preset.selection.servings[0].preset_ref.recipe.revision = Revision(1);
    refused(&preset, "serving_identity");
}

#[test]
fn meal_context_requires_one_unrestricted_adult_home_breakfast() {
    let mut empty = fixture();
    empty.meal.attendees.0.clear();
    refused(&empty, "context");
    for age in [
        AgeBand::Teen,
        AgeBand::Child1To3,
        AgeBand::Child4To6,
        AgeBand::Child7To10,
        AgeBand::Child11To12,
    ] {
        let mut input = fixture();
        input.meal.attendees[0].age_band = age;
        refused(&input, "adult_source_serving");
    }
    for requirement in [
        HardRequirement::Diet {
            code: DietCode::Vegetarian,
        },
        HardRequirement::Diet {
            code: DietCode::GlutenFree,
        },
        HardRequirement::AllergenExclusion {
            code: AllergenCode::Milk,
        },
    ] {
        let mut input = fixture();
        input.meal.attendees[0].hard_requirements = BoundedVec(vec![requirement.clone()]);
        refused(&input, "context");
        input.meal.shared_hard_requirements = BoundedVec(vec![requirement.clone()]);
        refused(&input, "hard_requirements");
        input.meal.attendees[0].hard_requirements.0.clear();
        input.meal.shared_hard_requirements = BoundedVec(vec![requirement]);
        refused(&input, "context");
    }
    let mut portable = fixture();
    portable.meal.location = MealLocation::OnTheMove;
    refused(&portable, "at_home_fresh_preparation");
    let mut cold = fixture();
    cold.meal.mobility.temperature = MobilityTemperature::Cold;
    refused(&cold, "at_home_fresh_preparation");
    let mut no_hob = fixture();
    no_hob.meal.equipment.0.clear();
    refused(&no_hob, "at_home_fresh_preparation");
    let mut dinner = fixture();
    dinner.meal.slot.meal_type = MealType::Dinner;
    dinner.selection.slot = dinner.meal.slot.clone();
    dinner.selection.servings[0].preset_ref.meal_type = MealType::Dinner;
    refused(&dinner, "breakfast");
    let mut disabled = fixture();
    disabled.meal.enabled = false;
    refused(&disabled, "context");
    let mut foreign = fixture();
    foreign.selection.slot.date = serde_json::from_value(json!("2026-12-29")).unwrap();
    refused(&foreign, "slot");
}

#[test]
fn compensating_multiple_attendees_pass_core_unit_total_but_fail_personal_scope() {
    let mut input = fixture();
    let mut second = input.meal.attendees[0].clone();
    second.person = PersonRef::Diner {
        diner_id: serde_json::from_value(json!("dn_00000000000000000000000000000002")).unwrap(),
    };
    input.meal.attendees[0].portion.adjustment = PositiveRational::new(1, 2).unwrap();
    second.portion.adjustment = PositiveRational::new(1, 2).unwrap();
    input.meal.attendees.0.push(second);
    let basis = diagnostic_basis(&input);
    let classes = ComponentClassView::new(input.reference.clone(), vec![]).unwrap();
    let reviewed = assess_revision(&input.recipe, &basis, &classes).unwrap();
    let resolved = mstrchf_core::portions::resolve_servings(
        &reviewed,
        &input.meal,
        &input.selection.meal_overrides,
    )
    .unwrap();
    assert_eq!(resolved.len(), 2);
    assert!(
        resolved
            .iter()
            .all(|s| s.servings == PositiveRational::new(1, 2).unwrap())
    );
    let total = PositiveRational(
        resolved[0]
            .servings
            .0
            .checked_add(&resolved[1].servings.0)
            .unwrap(),
    );
    assert_eq!(total, one());
    assert!(mstrchf_core::portions::scale_recipe(&reviewed, total).is_ok());
    input.selection.servings = BoundedVec(resolved);
    refused(&input, "single_attendee");
}

#[test]
fn later_unit_override_cannot_launder_an_unsupported_earlier_portion() {
    let mut input = fixture();
    input.meal.attendees[0].portion.adjustment = PositiveRational::new(2, 1).unwrap();
    input.selection.meal_overrides.portions = BoundedVec(vec![PersonPortionOverride {
        person: input.meal.attendees[0].person.clone(),
        preset: PortionPresetName::Regular,
        adjustment: one(),
    }]);
    let basis = diagnostic_basis(&input);
    let classes = ComponentClassView::new(input.reference.clone(), vec![]).unwrap();
    let reviewed = assess_revision(&input.recipe, &basis, &classes).unwrap();
    let resolved = mstrchf_core::portions::resolve_servings(
        &reviewed,
        &input.meal,
        &input.selection.meal_overrides,
    )
    .unwrap();
    assert_eq!(resolved[0].servings, one());
    assert!(mstrchf_core::portions::scale_recipe(&reviewed, one()).is_ok());
    refused(&input, "portion_preference");
    input.meal.attendees[0].portion.adjustment = one();
    input.meal.attendees[0].portion.meal_overrides = BoundedVec(vec![MealPortionOverride {
        meal_type: MealType::Breakfast,
        preset: PortionPresetName::Regular,
        adjustment: PositiveRational::new(1, 2).unwrap(),
    }]);
    refused(&input, "breakfast_portion_override");
    input.meal.attendees[0].portion.meal_overrides.0.clear();
    input.selection.meal_overrides.portions[0].adjustment = PositiveRational::new(1, 2).unwrap();
    refused(&input, "selection_portion_override");
    input.selection.meal_overrides.portions[0].adjustment = one();
    input.options.overrides = Some(input.selection.meal_overrides.clone());
    input.options.overrides.as_mut().unwrap().portions[0].adjustment =
        PositiveRational::new(2, 1).unwrap();
    refused(&input, "request_portion_override");
}

#[test]
fn final_serving_identity_and_production_total_are_independently_checked() {
    let mut quantity = fixture();
    quantity.selection.servings[0].servings = PositiveRational::new(1, 2).unwrap();
    refused(&quantity, "unchanged_source_serving");
    let mut extra = fixture();
    extra.selection.servings[0].servings = PositiveRational::new(1, 2).unwrap();
    let mut second = extra.selection.servings[0].clone();
    second.person = PersonRef::Diner {
        diner_id: serde_json::from_value(json!("dn_00000000000000000000000000000002")).unwrap(),
    };
    extra.selection.servings.0.push(second);
    refused(&extra, "single_serving_allocation");
    let mut person = fixture();
    person.selection.servings[0].person = PersonRef::Diner {
        diner_id: serde_json::from_value(json!("dn_00000000000000000000000000000002")).unwrap(),
    };
    refused(&person, "serving_identity");
    let mut production = fixture();
    if let BatchUse::NewBatch {
        production_servings,
        ..
    } = &mut production.selection.batch_use
    {
        *production_servings = PositiveRational::new(2, 1).unwrap();
    }
    refused(&production, "fresh_unit_batch");
}

#[test]
fn request_extras_leftovers_and_remix_links_cannot_bypass_fresh_only_scope() {
    let mut extra = fixture();
    extra.options.make_extra = Some(true);
    refused(&extra, "extras_or_reuse");
    extra.options.make_extra = Some(false);
    assert!(extra.check().is_ok());
    extra.options.extra_recipients = Some(BoundedVec(vec![extra.meal.attendees[0].person.clone()]));
    refused(&extra, "extras_or_reuse");
    let mut leftover = fixture();
    leftover.options.leftover_source = Some(leftover.selection.slot.clone());
    refused(&leftover, "extras_or_reuse");
    leftover.options.leftover_source = None;
    leftover.selection.batch_use = BatchUse::Leftover {
        batch_id: serde_json::from_value(json!("ba_80000000000000000000000000000001")).unwrap(),
        component_ids: BoundedVec(vec![leftover.recipe.components[0].component_id.clone()]),
    };
    refused(&leftover, "fresh_unit_batch");
    for as_source in [true, false] {
        let mut link = fixture();
        let mut other = link.selection.slot.clone();
        other.date = serde_json::from_value(json!("2026-12-29")).unwrap();
        link.plan.batch_links = BoundedVec(vec![BatchLink {
            batch_id: serde_json::from_value(json!("ba_80000000000000000000000000000002")).unwrap(),
            source_slot: if as_source {
                link.selection.slot.clone()
            } else {
                other.clone()
            },
            target_slot: if as_source {
                other
            } else {
                link.selection.slot.clone()
            },
            components: BoundedVec(vec![ComponentAllocation {
                component_id: link.recipe.components[0].component_id.clone(),
                quantity: link.recipe.r#yield.components[0].quantity.clone(),
            }]),
        }]);
        refused(&link, "batch_reuse_link");
    }
    let mut duplicate = fixture();
    let mut other = duplicate.selection.clone();
    other.slot.date = serde_json::from_value(json!("2026-12-29")).unwrap();
    duplicate.plan.selections = BoundedVec(vec![other]);
    refused(&duplicate, "batch_already_used_elsewhere");
    let mut addition = fixture();
    addition.selection.extras = BoundedVec(vec![ExtraSelection {
        extra_id: serde_json::from_value(json!("ex_80000000000000000000000000000001")).unwrap(),
        recipe: addition.reference.clone(),
        recipients: addition.selection.servings.clone(),
    }]);
    refused(&addition, "extras_or_reuse");
    let mut detached_link = fixture();
    let mut source = detached_link.selection.slot.clone();
    source.date = serde_json::from_value(json!("2026-12-29")).unwrap();
    let mut target = source.clone();
    target.date = serde_json::from_value(json!("2026-12-30")).unwrap();
    detached_link.plan.batch_links = BoundedVec(vec![BatchLink {
        batch_id: serde_json::from_value(json!("ba_80000000000000000000000000000001")).unwrap(),
        source_slot: source,
        target_slot: target,
        components: BoundedVec(vec![ComponentAllocation {
            component_id: detached_link.recipe.components[0].component_id.clone(),
            quantity: detached_link.recipe.r#yield.components[0].quantity.clone(),
        }]),
    }]);
    refused(&detached_link, "batch_reuse_link");
    detached_link.plan.batch_links[0].batch_id =
        serde_json::from_value(json!("ba_80000000000000000000000000000002")).unwrap();
    assert!(
        detached_link.check().is_ok(),
        "Unrelated meals' batch links are preserved"
    );
}
