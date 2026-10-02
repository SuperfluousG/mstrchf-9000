//! Builder controls use native constructors and literal independent v4 premise
//! bytes. No native authority-bearing production type acquires Deserialize.
use super::*;
use crate::food::*;
use crate::recipes::*;
use serde_json::Value;

macro_rules! wire {
    ($v:expr, $t:ty) => {
        serde_json::from_value::<$t>($v.clone()).unwrap()
    };
}
fn array(v: &Value) -> &[Value] {
    v.as_array().unwrap()
}
fn occ(v: &Value) -> OccurrenceKey {
    OccurrenceKey {
        slot: wire!(v["slot"], SlotId),
        extra_id: wire!(v["extra_id"], Option<ExtraId>),
        recipe: wire!(v["recipe"], RecipeRef),
    }
}
fn fixture() -> (
    WeekPlan,
    PlanningContext,
    FoodEvidence,
    FoodIdentities,
    DerivedPlan,
    Vec<TaskBinding>,
) {
    let v: Value = serde_json::from_slice(include_bytes!(
        "../../../../tests/fixtures/food/batches/native-v4-subject-basis.json"
    ))
    .unwrap();
    let v = &v["value"];
    let e = &v["evidence"];
    let evidence = FoodEvidence {
        catalogue_revision: wire!(e["catalogue_revision"], Revision),
        catalogue_digest: wire!(e["catalogue_digest"], PayloadDigest),
        policy_version: wire!(e["policy_version"], Revision),
        recipes: array(&e["recipes"])
            .iter()
            .map(|b| {
                EvidenceBasis::new(
                    wire!(b["recipe"], RecipeRef),
                    array(&b["admitted"])
                        .iter()
                        .map(|p| EvidencePin {
                            evidence_id: wire!(p["evidence_id"], EvidenceId),
                            revision: wire!(p["revision"], Revision),
                            digest: wire!(p["digest"], Digest),
                        })
                        .collect(),
                    wire!(b["verified_asset_digests"], Vec<AssetDigest>),
                )
                .unwrap()
            })
            .collect(),
        classes: array(&e["classes"])
            .iter()
            .map(|c| {
                ComponentClassView::new(
                    wire!(c["recipe"], RecipeRef),
                    array(&c["components"])
                        .iter()
                        .map(|r| ComponentClassEvidence {
                            recipe: wire!(r["recipe"], RecipeRef),
                            component_id: wire!(r["component_id"], ComponentId),
                            class: match r["class"]["kind"].as_str().unwrap() {
                                "general" => ComponentClass::General,
                                "rice" => ComponentClass::Rice,
                                "unknown" => ComponentClass::Unknown,
                                _ => panic!("unknown fixture class"),
                            },
                            evidence_ids: wire!(r["evidence_ids"], Vec<EvidenceId>),
                        })
                        .collect(),
                )
                .unwrap()
            })
            .collect(),
        methods: array(&e["methods"])
            .iter()
            .map(|m| MethodPhases {
                recipe: wire!(m["recipe"], RecipeRef),
                kind: match m["kind"]["kind"].as_str().unwrap() {
                    "production" => MethodKind::Production,
                    "reuse" => MethodKind::Reuse,
                    _ => panic!("unknown fixture method"),
                },
                prep_ids: wire!(m["prep_ids"], Vec<u16>),
                evidence_ids: wire!(m["evidence_ids"], Vec<EvidenceId>),
                direct_reuse_counts: array(&m["direct_reuse_counts"])
                    .iter()
                    .map(|c| ComponentReheatCount {
                        recipe: wire!(c["recipe"], RecipeRef),
                        component_id: wire!(c["component_id"], ComponentId),
                        planned_reheats: wire!(c["planned_reheats"], u8),
                        evidence_ids: wire!(c["evidence_ids"], Vec<EvidenceId>),
                    })
                    .collect(),
            })
            .collect(),
        resources: array(&e["resources"])
            .iter()
            .map(|r| PhaseResources {
                recipe: wire!(r["recipe"], RecipeRef),
                prep_id: wire!(r["prep_id"], u16),
                equipment: wire!(r["equipment"], Vec<EquipmentCode>),
                evidence_ids: wire!(r["evidence_ids"], Vec<EvidenceId>),
            })
            .collect(),
        essential_ingredient_ids: wire!(e["essential_ingredient_ids"], Vec<IngredientId>),
    };
    let i = &v["identities"];
    let identities = FoodIdentities {
        revision: wire!(i["revision"], Revision),
        digest: wire!(i["digest"], PayloadDigest),
        allocations: array(&i["allocations"])
            .iter()
            .map(|r| {
                (
                    AllocationKey {
                        batch_id: wire!(r[0]["batch_id"], BatchId),
                        component_id: wire!(r[0]["component_id"], ComponentId),
                        target_slot: wire!(r[0]["target_slot"], SlotId),
                        person: wire!(r[0]["person"], PersonRef),
                    },
                    wire!(r[1], AllocationId),
                )
            })
            .collect(),
        sessions: array(&i["sessions"])
            .iter()
            .map(|r| {
                (
                    SessionKey {
                        anchor_slot: wire!(r[0]["anchor_slot"], SlotId),
                        role: match r[0]["role"]["kind"].as_str().unwrap() {
                            "cooking" => SessionRole::Cooking,
                            "advance_prep" => SessionRole::AdvancePrep,
                            _ => panic!("unknown role"),
                        },
                        ordinal: wire!(r[0]["ordinal"], u16),
                    },
                    wire!(r[1], PrepSessionId),
                )
            })
            .collect(),
        demand_lines: array(&i["demand_lines"])
            .iter()
            .map(|r| {
                (
                    DemandKey {
                        ingredient_id: wire!(r[0]["ingredient_id"], IngredientId),
                        unit: wire!(r[0]["unit"], Unit),
                        form: wire!(r[0]["form"], IngredientForm),
                    },
                    wire!(r[1], DemandLineId),
                )
            })
            .collect(),
    };
    let map: Value = serde_json::from_slice(include_bytes!(
        "../../../../tests/fixtures/food/batches/native-v4-subject-task-map.json"
    ))
    .unwrap();
    let bindings = array(&map["value"])
        .iter()
        .map(|r| TaskBinding {
            phase: PhaseKey {
                occurrence: occ(&r["phase"]["occurrence"]),
                prep_id: wire!(r["phase"]["prep_id"], u16),
            },
            task: wire!(r["task"], PrepTaskRef),
        })
        .collect();
    let derived: DerivedPlan = parse_document(include_bytes!(
        "../../../../tests/fixtures/food/batches/native-v4-subject-derived.json"
    ))
    .unwrap();
    (
        wire!(v["plan"], WeekPlan),
        wire!(v["context"], PlanningContext),
        evidence,
        identities,
        derived,
        bindings,
    )
}
fn refresh(e: &mut FoodEvidence) {
    e.catalogue_digest = food_evidence_digest(e).unwrap();
}

#[test]
fn derivation_matches_independent_v4_original_batch_and_allocations() {
    let (plan, context, evidence, identities, expected, bindings) = fixture();
    let keys = allocation_keys(&plan, &context, &evidence).unwrap();
    assert_eq!(
        keys,
        identities
            .allocations
            .iter()
            .map(|r| r.0.clone())
            .collect::<Vec<_>>()
    );
    let actual = derive_batches(
        &plan,
        &context,
        &evidence,
        &identities,
        &expected.prep_sessions,
        &bindings,
    )
    .unwrap();
    assert_eq!(actual.0, expected.batches.0);
    assert_eq!(actual.1, expected.allocations.0);
    let again = derive_batches(
        &plan,
        &context,
        &evidence,
        &identities,
        &expected.prep_sessions,
        &bindings,
    )
    .unwrap();
    assert_eq!(actual, again);
}
#[test]
fn derivation_cannot_enlarge_committed_source_or_reinterpret_link() {
    let (mut plan, context, evidence, identities, expected, bindings) = fixture();
    if let BatchUse::NewBatch {
        production_servings,
        ..
    } = &mut plan.selections[0].batch_use
    {
        *production_servings = PositiveRational::new(1, 1).unwrap();
    }
    assert!(
        derive_batches(
            &plan,
            &context,
            &evidence,
            &identities,
            &expected.prep_sessions,
            &bindings
        )
        .is_err()
    );
    if let BatchUse::NewBatch {
        production_servings,
        ..
    } = &mut plan.selections[0].batch_use
    {
        *production_servings = PositiveRational::new(2, 1).unwrap();
    }
    plan.batch_links[0].components[0].quantity.amount = PositiveRational::new(201, 1).unwrap();
    assert!(allocation_keys(&plan, &context, &evidence).is_err());
}
#[test]
fn derivation_exact_chosen_cold_count_and_original_clock() {
    let (plan, context, mut evidence, identities, expected, bindings) = fixture();
    let method = evidence
        .methods
        .iter_mut()
        .find(|m| m.kind == MethodKind::Reuse)
        .unwrap();
    method.direct_reuse_counts[0].planned_reheats = 0;
    method.prep_ids.clear();
    refresh(&mut evidence);
    let (batches, allocations) = derive_batches(
        &plan,
        &context,
        &evidence,
        &identities,
        &expected.prep_sessions,
        &bindings,
    )
    .unwrap();
    assert!(allocations.iter().all(|a| a.planned_reheats == 0));
    assert_eq!(
        batches[0].components[0].original_production,
        expected.batches[0].components[0].original_production
    );
    let mut forged = allocations;
    forged[1].planned_reheats = 2;
    assert!(validate_allocations(&batches, &forged).is_err());
}
#[test]
fn derivation_disabled_missing_or_duplicate_original_fails_closed() {
    let (mut plan, mut context, evidence, identities, expected, bindings) = fixture();
    let source_slot = plan.selections[0].slot.clone();
    context
        .requirements
        .iter_mut()
        .find(|m| m.slot == source_slot)
        .unwrap()
        .enabled = false;
    assert!(
        derive_batches(
            &plan,
            &context,
            &evidence,
            &identities,
            &expected.prep_sessions,
            &bindings
        )
        .is_err()
    );
    context
        .requirements
        .iter_mut()
        .find(|m| m.slot == source_slot)
        .unwrap()
        .enabled = true;
    plan.selections[1].batch_use = plan.selections[0].batch_use.clone();
    assert!(allocation_keys(&plan, &context, &evidence).is_err());
}
#[test]
fn derivation_source_binding_and_allocated_identity_are_exact() {
    let (plan, context, evidence, mut identities, expected, mut bindings) = fixture();
    identities.allocations[1].1 = identities.allocations[0].1.clone();
    assert!(
        derive_batches(
            &plan,
            &context,
            &evidence,
            &identities,
            &expected.prep_sessions,
            &bindings
        )
        .is_err()
    );
    let (.., identities, _, _) = fixture();
    bindings[0].task.task_id = bindings[1].task.task_id;
    assert!(
        derive_batches(
            &plan,
            &context,
            &evidence,
            &identities,
            &expected.prep_sessions,
            &bindings
        )
        .is_err()
    );
}

fn remix_fixture() -> (
    WeekPlan,
    PlanningContext,
    FoodEvidence,
    FoodIdentities,
    DerivedPlan,
    Vec<TaskBinding>,
) {
    let (mut plan, mut context, mut evidence, identities, derived, bindings) = fixture();
    let source = context.recipes[0].clone();
    let source_ref = plan.selections[0].recipe.clone();
    let mut remix = source.clone();
    remix.recipe_id = RecipeId(format!("rc_{:032x}", 302));
    let local_component = ComponentId(format!("co_{:032x}", 302));
    let local_ingredient = IngredientId(format!("ig_{:032x}", 302));
    remix.title = Title("Synthetic addition-only remix".into());
    remix.r#yield.servings = PositiveRational::new(1, 1).unwrap();
    remix.r#yield.components[0].component_id = local_component.clone();
    remix.r#yield.components[0].quantity.amount = PositiveRational::new(100, 1).unwrap();
    remix.components[0].component_id = local_component.clone();
    remix.components[0].ingredient_ids = BoundedVec(vec![local_ingredient.clone()]);
    remix.components[0].storage_rule_ids.0.clear();
    remix.ingredients[0].ingredient_id = local_ingredient.clone();
    remix.ingredients[0].component_id = local_component.clone();
    remix.ingredients[0].quantity.0.as_mut().unwrap().amount =
        PositiveRational::new(100, 1).unwrap();
    remix.ingredients[0].step_ids = BoundedVec(vec![1]);
    remix.steps.0.truncate(1);
    remix.steps[0].ingredient_ids = BoundedVec(vec![local_ingredient.clone()]);
    remix.steps[0].component_ids = BoundedVec(vec![
        source.components[0].component_id.clone(),
        local_component.clone(),
    ]);
    remix.prep.0.truncate(1);
    remix.prep[0].ingredient_ids = BoundedVec(vec![local_ingredient]);
    remix.portability.reheating_required = false;
    remix.storage.0.clear();
    remix.capabilities.leftovers = false;
    plan.batch_links[0].components[0].quantity.amount = PositiveRational::new(100, 1).unwrap();
    remix.reused_inputs = BoundedVec(vec![ReusedComponentInput {
        source_recipe: source_ref.clone(),
        source_component_id: source.components[0].component_id.clone(),
        quantity_per_recipe_yield: plan.batch_links[0].components[0].quantity.clone(),
        step_ids: BoundedVec(vec![1]),
        planned_reheats: 1,
        evidence_ids: BoundedVec(vec![evidence.recipes[0].admitted[0].evidence_id.clone()]),
    }]);
    let remix_ref = RecipeRef {
        recipe_id: remix.recipe_id.clone(),
        revision: remix.revision,
        digest: recipe_digest(&remix).unwrap(),
    };
    plan.selections[1].recipe = remix_ref.clone();
    for s in &mut plan.selections[1].servings.0 {
        s.preset_ref.recipe = remix_ref.clone();
    }
    context.recipes.0.push(remix);
    context.stamp.recipe_refs.0.push(remix_ref.clone());
    let mut basis = evidence.recipes[0].clone();
    basis.recipe = remix_ref.clone();
    evidence.recipes.push(basis);
    evidence.classes.push(
        ComponentClassView::new(
            remix_ref.clone(),
            vec![ComponentClassEvidence {
                recipe: remix_ref.clone(),
                component_id: local_component,
                class: ComponentClass::General,
                evidence_ids: vec![evidence.recipes[0].admitted[0].evidence_id.clone()],
            }],
        )
        .unwrap(),
    );
    let mut method = evidence.methods[0].clone();
    method.recipe = remix_ref.clone();
    evidence.methods.push(method);
    let mut resource = evidence.resources[0].clone();
    resource.recipe = remix_ref;
    evidence.resources.push(resource);
    refresh(&mut evidence);
    (plan, context, evidence, identities, derived, bindings)
}

#[test]
fn derivation_addition_only_remix_preserves_original_and_source_equivalent_servings() {
    let (plan, context, evidence, identities, original, bindings) = remix_fixture();
    let (batches, allocations) = derive_batches(
        &plan,
        &context,
        &evidence,
        &identities,
        &original.prep_sessions,
        &bindings,
    )
    .unwrap();
    assert_eq!(
        batches[0].components[0].produced,
        original.batches[0].components[0].produced
    );
    assert_eq!(
        batches[0].components[0].original_production,
        original.batches[0].components[0].original_production
    );
    assert_eq!(
        batches[0].components[0].allocated.amount,
        PositiveRational::new(300, 1).unwrap()
    );
    assert_eq!(
        batches[0].components[0].unallocated.amount,
        Rational::new(100, 1).unwrap()
    );
    assert_eq!(
        plan.selections[1].servings[0].servings,
        PositiveRational::new(1, 1).unwrap()
    );
    assert_eq!(
        allocations[1].servings,
        PositiveRational::new(1, 2).unwrap()
    );
    assert_eq!(
        allocations[1].quantity.amount,
        PositiveRational::new(100, 1).unwrap()
    );
    assert_eq!(
        allocations[1].remix_recipe.0.as_ref(),
        Some(&plan.selections[1].recipe)
    );
    assert_eq!(allocations[1].batch_id, batches[0].batch_id);
    assert_eq!(batches.len(), 1, "additions do not create a source batch");
}

#[test]
fn derivation_rejects_remix_as_fresh_original_and_missing_source_lineage() {
    let (mut plan, context, evidence, identities, original, bindings) = remix_fixture();
    plan.selections[1].batch_use = BatchUse::NewBatch {
        batch_id: BatchId(format!("ba_{:032x}", 302)),
        production_servings: PositiveRational::new(1, 1).unwrap(),
    };
    assert!(
        derive_batches(
            &plan,
            &context,
            &evidence,
            &identities,
            &original.prep_sessions,
            &bindings
        )
        .is_err()
    );
    plan.selections[1].batch_use = BatchUse::Leftover {
        batch_id: BatchId(format!("ba_{:032x}", 302)),
        component_ids: BoundedVec(vec![original.batches[0].components[0].component_id.clone()]),
    };
    assert!(
        derive_batches(
            &plan,
            &context,
            &evidence,
            &identities,
            &original.prep_sessions,
            &bindings
        )
        .is_err()
    );
}

#[test]
fn derivation_mixed_warm_and_cold_components_keep_exact_chosen_counts() {
    let (mut plan, mut context, mut evidence, mut identities, mut expected, mut bindings) =
        fixture();
    let cold_id = ComponentId(format!("co_{:032x}", 302));
    let ingredient_id = IngredientId(format!("ig_{:032x}", 302));
    let evidence_id = EvidenceId(format!("ev_{:032x}", 302));
    let recipe = &mut context.recipes[0];
    let mut cold = recipe.components[0].clone();
    cold.component_id = cold_id.clone();
    cold.ingredient_ids = BoundedVec(vec![ingredient_id.clone()]);
    cold.storage_rule_ids = BoundedVec(vec![evidence_id.clone()]);
    recipe.components.0.push(cold);
    let mut ingredient = recipe.ingredients[0].clone();
    ingredient.ingredient_id = ingredient_id.clone();
    ingredient.component_id = cold_id.clone();
    ingredient.step_ids = BoundedVec(vec![1]);
    recipe.ingredients.0.push(ingredient);
    let mut yield_row = recipe.r#yield.components[0].clone();
    yield_row.component_id = cold_id.clone();
    yield_row.quantity.amount = PositiveRational::new(100, 1).unwrap();
    recipe.r#yield.components.0.push(yield_row.clone());
    recipe.steps[0].ingredient_ids.0.push(ingredient_id.clone());
    recipe.steps[0].component_ids.0.push(cold_id.clone());
    recipe.prep[0].ingredient_ids.0.push(ingredient_id);
    let mut rule = recipe.storage[0].clone();
    rule.evidence_id = evidence_id.clone();
    rule.component_id = cold_id.clone();
    rule.max_reheats = 0;
    recipe.storage.0.push(rule);
    recipe.portability.reheating_required = false;
    let reference = RecipeRef {
        recipe_id: recipe.recipe_id.clone(),
        revision: recipe.revision,
        digest: recipe_digest(recipe).unwrap(),
    };
    context.stamp.recipe_refs[0] = reference.clone();
    for selection in &mut plan.selections.0 {
        selection.recipe = reference.clone();
        for serving in &mut selection.servings.0 {
            serving.preset_ref.recipe = reference.clone();
        }
    }
    if let BatchUse::Leftover { component_ids, .. } = &mut plan.selections[1].batch_use {
        component_ids.0.push(cold_id.clone());
    }
    plan.batch_links[0].components.0.push(ComponentAllocation {
        component_id: cold_id.clone(),
        quantity: yield_row.quantity,
    });
    context.weeks[0].plan = plan.clone();
    evidence.recipes[0].recipe = reference.clone();
    evidence.recipes[0].admitted.push(EvidencePin {
        evidence_id: evidence_id.clone(),
        revision: Revision(1),
        digest: Digest("b".repeat(64)),
    });
    evidence.classes[0].recipe = reference.clone();
    evidence.classes[0].components[0].recipe = reference.clone();
    evidence.classes[0].components.push(ComponentClassEvidence {
        recipe: reference.clone(),
        component_id: cold_id.clone(),
        class: ComponentClass::General,
        evidence_ids: vec![evidence_id.clone()],
    });
    for method in &mut evidence.methods {
        method.recipe = reference.clone();
        for count in &mut method.direct_reuse_counts {
            count.recipe = reference.clone();
        }
        if method.kind == MethodKind::Reuse {
            method.evidence_ids.push(evidence_id.clone());
            method.direct_reuse_counts.push(ComponentReheatCount {
                recipe: reference.clone(),
                component_id: cold_id.clone(),
                planned_reheats: 0,
                evidence_ids: vec![evidence_id.clone()],
            });
        }
    }
    for resource in &mut evidence.resources {
        resource.recipe = reference.clone();
    }
    refresh(&mut evidence);
    let cold_keys = identities
        .allocations
        .clone()
        .into_iter()
        .enumerate()
        .map(|(i, (mut key, _))| {
            key.component_id = cold_id.clone();
            (key, AllocationId(format!("al_{:032x}", 503 + i)))
        })
        .collect::<Vec<_>>();
    identities.allocations.extend(cold_keys);
    identities.digest = food_identities_digest(&identities).unwrap();
    for session in &mut expected.prep_sessions.0 {
        for task in &mut session.tasks.0 {
            task.recipe = reference.clone();
        }
    }
    for binding in &mut bindings {
        binding.phase.occurrence.recipe = reference.clone();
    }
    let (batches, allocations) = derive_batches(
        &plan,
        &context,
        &evidence,
        &identities,
        &expected.prep_sessions,
        &bindings,
    )
    .unwrap();
    assert_eq!(batches[0].components.len(), 2);
    let served = allocations
        .iter()
        .filter(|a| a.target_slot == plan.selections[1].slot)
        .collect::<Vec<_>>();
    assert_eq!(served.len(), 2);
    assert_eq!(served[0].planned_reheats, 1);
    assert_eq!(served[1].planned_reheats, 0);
    assert_eq!(
        batches[0].components[1]
            .storage_rule
            .0
            .as_ref()
            .unwrap()
            .max_reheats,
        0
    );
}
