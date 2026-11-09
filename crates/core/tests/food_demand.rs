use mstrchf_contracts::*;
use mstrchf_core::demand::project_essentials;
use mstrchf_core::{food::*, portions::*, recipes::*};

#[path = "../../../tests/fixtures/food/batches/f11_fixture.rs"]
mod f11_fixture;

#[allow(dead_code)]
pub fn run_oracle_case(id: u8) {
    match id {
        30 => food_f30_have_enough_preserves_complete_demand(),
        31 => food_f31_changed_amount_requires_renewal(),
        32 => food_f32_changed_form_requires_renewal(),
        33 => food_f33_changed_digest_requires_renewal(),
        34 => food_f34_past_purchase_is_not_stock(),
        _ => panic!("case F{id:02} is outside demand ownership"),
    }
}

#[test]
fn food_f11_use_graph_preserves_original_source_and_addition_lineage() {
    let saved = f11_fixture::fixture();
    let inputs = f11_fixture::feasibility();
    let derived = derive_plan(
        &saved.plan,
        &saved.context,
        &FoodInputs {
            evidence: &saved.evidence,
            identities: &saved.identities,
            schedule: &saved.schedule,
            feasibility: &inputs,
        },
    )
    .unwrap();
    let graph = mstrchf_core::demand::reconstruct_use_graph(&derived.plan, &saved).unwrap();
    assert_eq!(derived.plan.batches.len(), 1);
    assert_eq!(derived.plan.demand.lines.len(), 6);
    assert_eq!(graph.uses.len(), 28);
    for (n, amount) in [480, 400, 160, 20, 1, 250].into_iter().enumerate() {
        let line = derived
            .plan
            .demand
            .lines
            .iter()
            .find(|l| l.ingredient_id == IngredientId(format!("ig_{:032x}", 401 + n)))
            .unwrap();
        assert_eq!(
            line.quantity.amount,
            PositiveRational::new(amount, 1).unwrap()
        );
        assert_eq!(
            line.origin_batches.0,
            vec![derived.plan.batches[0].batch_id.clone()]
        );
        assert!(line.origin_extras.is_empty());
        let uses: Vec<_> = graph
            .uses
            .iter()
            .filter(|u| u.line_id == line.line_id)
            .collect();
        assert_eq!(uses.len(), if n < 4 { 6 } else { 2 });
        let consumption: Vec<_> = uses
            .iter()
            .filter_map(|u| match &u.use_key {
                UseKey::Consumption {
                    occurrence,
                    component_id,
                    ..
                } => Some((occurrence, component_id)),
                _ => None,
            })
            .collect();
        if n < 4 {
            assert_eq!(consumption.len(), 3);
            assert!(
                consumption
                    .iter()
                    .all(|(_, id)| **id == derived.plan.batches[0].components[0].component_id)
            );
        } else {
            assert_eq!(consumption.len(), 1);
            assert_eq!(
                consumption[0].0.recipe.recipe_id,
                RecipeId(format!("rc_{:032x}", if n == 4 { 402 } else { 403 }))
            );
        }
    }
    let repeated = derive_plan(
        &saved.plan,
        &saved.context,
        &FoodInputs {
            evidence: &saved.evidence,
            identities: &saved.identities,
            schedule: &saved.schedule,
            feasibility: &inputs,
        },
    )
    .unwrap();
    assert_eq!(
        canonical_bytes(&derived.plan).unwrap(),
        canonical_bytes(&repeated.plan).unwrap()
    );
}

fn demand(amount: u64) -> DemandSet {
    let lines = BoundedVec(vec![DemandLine {
        line_id: DemandLineId(format!("dl_{:032x}", 1)),
        ingredient_id: IngredientId(format!("ig_{:032x}", 1)),
        quantity: Quantity {
            amount: PositiveRational::new(amount, 1).unwrap(),
            unit: Unit::G,
            form: IngredientForm::Dry,
        },
        origin_batches: BoundedVec(vec![BatchId(format!("ba_{:032x}", 1))]),
        origin_extras: BoundedVec(vec![]),
        essential: true,
        conversion_evidence_ids: BoundedVec(vec![]),
    }]);
    let input_plan_digest = PlanInputDigest("a".repeat(64));
    let digest = demand_digest(&DemandMaterial {
        schema_version: SchemaVersion,
        input_plan_digest: input_plan_digest.clone(),
        lines: lines.clone(),
    })
    .unwrap();
    DemandSet {
        input_plan_digest,
        lines,
        digest,
    }
}
fn confirmation(d: &DemandSet) -> EssentialsConfirmation {
    EssentialsConfirmation {
        line_id: d.lines[0].line_id.clone(),
        choice: EssentialsChoice::HaveEnough,
        displayed_quantity: d.lines[0].quantity.clone(),
        demand_digest: d.digest.clone(),
        confirmed_at: Instant::try_from("2026-10-02T09:00:00Z").unwrap(),
    }
}
fn now() -> Instant {
    Instant::try_from("2026-10-02T10:00:00Z").unwrap()
}

fn assert_oracle_projection(prefix: &str, projection: &mstrchf_core::demand::ShoppingProjection) {
    let oracle: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/food/oracle-v1.json"
    ))
    .unwrap();
    let cases = oracle["fixtures"].as_array().unwrap();
    assert_eq!(cases.len(), 55);
    let found: Vec<_> = cases
        .iter()
        .filter(|c| c["name"].as_str().unwrap().starts_with(prefix))
        .collect();
    assert_eq!(found.len(), 1);
    let expected = &found[0]["expected"];
    let required = projection.required.lines[0]
        .quantity
        .amount
        .0
        .numerator
        .0
        .to_string();
    let shopping = projection
        .shopping_lines
        .first()
        .map(|l| l.quantity.amount.0.numerator.0.to_string())
        .unwrap_or_else(|| "0".into());
    if prefix == "F34_" {
        assert_eq!(required, expected["demand"]["amount"].as_str().unwrap());
        assert_eq!(shopping, expected["shopping"]["amount"].as_str().unwrap());
        assert!(projection.covered_line_ids.is_empty());
    } else {
        assert_eq!(
            required,
            expected["retained_required_amount"].as_str().unwrap()
        );
        assert_eq!(shopping, expected["shopping_amount"].as_str().unwrap());
        assert_eq!(
            !projection.covered_line_ids.is_empty(),
            expected["confirmation_usable"].as_bool().unwrap()
        );
        assert_eq!(
            !projection.renewal_line_ids.is_empty(),
            expected["renew_confirmation"].as_bool().unwrap()
        );
    }
    println!(
        "{}: literal oracle values checked",
        found[0]["name"].as_str().unwrap()
    );
}

#[test]
fn food_f30_have_enough_preserves_complete_demand() {
    let d = demand(200);
    let p = project_essentials(&d, &[confirmation(&d)], &now()).unwrap();
    assert_eq!(p.required, d);
    assert!(p.shopping_lines.is_empty());
    assert_eq!(p.covered_line_ids, vec![d.lines[0].line_id.clone()]);
    assert_oracle_projection("F30_", &p);
}
#[test]
fn food_f31_changed_amount_requires_renewal() {
    let old = demand(200);
    let new = demand(250);
    let p = project_essentials(&new, &[confirmation(&old)], &now()).unwrap();
    assert_eq!(p.shopping_lines, new.lines.0);
    assert_eq!(p.renewal_line_ids, vec![old.lines[0].line_id.clone()]);
    assert_oracle_projection("F31_", &p);
}
#[test]
fn food_f32_changed_form_requires_renewal() {
    let mut d = demand(200);
    let c = confirmation(&d);
    d.lines[0].quantity.form = IngredientForm::Cooked;
    d.digest = demand_digest(&DemandMaterial {
        schema_version: SchemaVersion,
        input_plan_digest: d.input_plan_digest.clone(),
        lines: d.lines.clone(),
    })
    .unwrap();
    let p = project_essentials(&d, &[c], &now()).unwrap();
    assert_eq!(p.shopping_lines, d.lines.0);
    assert_eq!(p.renewal_line_ids.len(), 1);
    assert_oracle_projection("F32_", &p);
}
#[test]
fn food_f33_changed_digest_requires_renewal() {
    let mut d = demand(200);
    let c = confirmation(&d);
    d.lines[0].origin_batches[0] = BatchId(format!("ba_{:032x}", 2));
    d.digest = demand_digest(&DemandMaterial {
        schema_version: SchemaVersion,
        input_plan_digest: d.input_plan_digest.clone(),
        lines: d.lines.clone(),
    })
    .unwrap();
    let p = project_essentials(&d, &[c], &now()).unwrap();
    assert_eq!(p.shopping_lines, d.lines.0);
    assert_eq!(p.renewal_line_ids.len(), 1);
    assert_oracle_projection("F33_", &p);
}
#[test]
fn food_f34_past_purchase_is_not_stock() {
    let d = demand(200);
    let p = project_essentials(&d, &[], &now()).unwrap();
    assert_eq!(p.shopping_lines, d.lines.0);
    assert_eq!(
        p.required.lines[0].quantity.amount,
        PositiveRational::new(200, 1).unwrap()
    );
    assert_oracle_projection("F34_", &p);
}
#[test]
fn food_projection_rejects_forged_hash_future_and_duplicate_confirmations() {
    let d = demand(200);
    let c = confirmation(&d);
    let mut bad = d.clone();
    bad.lines[0].quantity.amount = PositiveRational::new(250, 1).unwrap();
    assert_eq!(
        project_essentials(&bad, &[], &now()).unwrap_err().code,
        ErrorCode::DigestMismatch
    );
    let mut future = c.clone();
    future.confirmed_at = Instant::try_from("2026-10-02T11:00:00Z").unwrap();
    assert!(project_essentials(&d, &[future], &now()).is_err());
    assert!(project_essentials(&d, &[c.clone(), c], &now()).is_err());
}
#[test]
fn food_projection_add_to_shop_and_nonessential_do_not_suppress() {
    let d = demand(200);
    let mut c = confirmation(&d);
    c.choice = EssentialsChoice::AddToShop;
    assert_eq!(
        project_essentials(&d, &[c], &now()).unwrap().shopping_lines,
        d.lines.0
    );
    let mut n = d;
    n.lines[0].essential = false;
    n.digest = demand_digest(&DemandMaterial {
        schema_version: SchemaVersion,
        input_plan_digest: n.input_plan_digest.clone(),
        lines: n.lines.clone(),
    })
    .unwrap();
    assert_eq!(
        project_essentials(&n, &[confirmation(&n)], &now())
            .unwrap()
            .shopping_lines,
        n.lines.0
    );
}

fn moment(time: &str) -> ZonedMoment {
    ZonedMoment::new(
        LocalDate("2026-12-28".into()),
        LocalTime(time.into()),
        ZoneId("Europe/London".into()),
        Resolution::Unambiguous,
        Instant(format!("2026-12-28T{time}Z")),
    )
    .unwrap()
}
fn pin(n: usize) -> EvidenceId {
    EvidenceId(format!("ev_{n:032x}"))
}
fn zero() -> PayloadDigest {
    PayloadDigest("0".repeat(64))
}

// Genuine frozen A assessment/B servings and exact synthetic recipe facts.
// This is a full derivation fixture, not an eligibility or completeness flag.
fn full_fixture(ingredients: usize) -> (SavedFoodBasis, FeasibilityInputs, FoodDerivation) {
    let mut recipe: RecipeRevision = parse_document(include_bytes!(
        "../../../tests/fixtures/food/admission/ruling004-current--complete-synthetic.json"
    ))
    .unwrap();
    recipe.r#yield.servings = PositiveRational::new(1, 1).unwrap();
    recipe.r#yield.components[0].quantity.amount =
        PositiveRational::new(ingredients as u64, 1).unwrap();
    recipe.r#yield.components[0].quantity.form = IngredientForm::Cooked;
    recipe.active_minutes = 10;
    recipe.elapsed_minutes = 10;
    recipe.steps[0].active_minutes = 10;
    recipe.steps[0].elapsed_minutes = 10;
    recipe.ingredients.0 = (1..=ingredients)
        .map(|n| {
            let mut i = recipe.ingredients[0].clone();
            i.ingredient_id = IngredientId(format!("ig_{n:032x}"));
            i.quantity.0.as_mut().unwrap().amount = PositiveRational::new(1, 1).unwrap();
            i
        })
        .collect();
    let ingredient_ids: Vec<_> = recipe
        .ingredients
        .iter()
        .map(|i| i.ingredient_id.clone())
        .collect();
    recipe.steps[0].ingredient_ids = BoundedVec(ingredient_ids.clone());
    recipe.components[0].ingredient_ids = BoundedVec(ingredient_ids.clone());
    recipe.components[0].production = Nullable(Some(ComponentProductionSpec::Cooked {
        completion_prep_id: 1,
    }));
    recipe.prep.0 = vec![PrepSpec {
        prep_id: 1,
        title: Title("Synthetic atomic cook".into()),
        step_ids: BoundedVec(vec![1]),
        depends_on: BoundedVec(vec![]),
        active_minutes: 10,
        elapsed_minutes: 10,
        earliest_offset_seconds: -3600,
        latest_offset_seconds: -600,
        ingredient_ids: BoundedVec(ingredient_ids.clone()),
        can_attach_to_prior_session: false,
    }];
    let reference = RecipeRef {
        recipe_id: recipe.recipe_id.clone(),
        revision: recipe.revision,
        digest: recipe_digest(&recipe).unwrap(),
    };
    let basis = EvidenceBasis::new(
        reference.clone(),
        (1..=32)
            .map(|n| EvidencePin {
                evidence_id: pin(n),
                revision: Revision(1),
                digest: Digest(format!("{n:064x}")),
            })
            .collect(),
        vec![AssetDigest("a".repeat(64))],
    )
    .unwrap();
    let classes = ComponentClassView::new(
        reference.clone(),
        vec![ComponentClassEvidence {
            recipe: reference.clone(),
            component_id: recipe.components[0].component_id.clone(),
            class: ComponentClass::General,
            evidence_ids: vec![pin(8)],
        }],
    )
    .unwrap();
    let admitted = assess_revision(&recipe, &basis, &classes).unwrap();
    let mut context_json: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/contracts/planning-context.json"
    ))
    .unwrap();
    context_json["recipes"] = serde_json::json!([recipe.clone()]);
    let mut context: PlanningContext = serde_json::from_value(context_json).unwrap();
    context.weeks.0.truncate(1);
    context.stamp.read_weeks.0.truncate(1);
    context.stamp.write_weeks.0.truncate(1);
    let week = context.weeks[0].range.week_id.clone();
    context.requirements.0.retain(|m| m.slot.week_id == week);
    for cell in &mut context.weeks[0].cells.0 {
        cell.attendee_ids.0.truncate(1);
    }
    for m in &mut context.requirements.0 {
        m.attendees.0.truncate(1);
        m.shared_hard_requirements = m.attendees[0].hard_requirements.clone();
    }
    let meal = context
        .requirements
        .iter()
        .find(|m| m.slot.meal_type == MealType::Dinner)
        .unwrap()
        .clone();
    let overrides = MealOverrides {
        portions: BoundedVec(vec![]),
        eat_by: Nullable(None),
    };
    let servings = resolve_servings(&admitted, &meal, &overrides).unwrap();
    let selection = MealSelection {
        portable_cold: None,
        slot: meal.slot.clone(),
        recipe: reference.clone(),
        origin: SelectionOrigin::Fixed,
        repeat_authorised: false,
        servings: BoundedVec(servings),
        extras: BoundedVec(vec![]),
        batch_use: BatchUse::NewBatch {
            batch_id: BatchId(format!("ba_{:032x}", 1)),
            production_servings: PositiveRational::new(1, 1).unwrap(),
        },
        meal_overrides: overrides,
    };
    let plan = WeekPlan {
        week_id: week,
        revision: Revision(1),
        selections: BoundedVec(vec![selection]),
        batch_links: BoundedVec(vec![]),
    };
    context.recipes = BoundedVec(vec![recipe]);
    context.stamp.recipe_refs = BoundedVec(vec![reference.clone()]);
    context.validate_shape().unwrap();
    let mut evidence = FoodEvidence {
        catalogue_revision: Revision(1),
        catalogue_digest: zero(),
        policy_version: Revision(1),
        recipes: vec![basis],
        classes: vec![classes],
        methods: vec![MethodPhases {
            portable_cold: None,
            recipe: reference.clone(),
            kind: MethodKind::Production,
            prep_ids: vec![1],
            direct_reuse_counts: vec![],
            evidence_ids: vec![pin(8)],
        }],
        resources: vec![PhaseResources {
            recipe: reference.clone(),
            prep_id: 1,
            equipment: vec![EquipmentCode::Hob],
            evidence_ids: vec![pin(8)],
        }],
        essential_ingredient_ids: ingredient_ids,
    };
    evidence.catalogue_digest = food_evidence_digest(&evidence).unwrap();
    let phase = PhaseKey {
        occurrence: OccurrenceKey {
            slot: meal.slot.clone(),
            extra_id: None,
            recipe: reference,
        },
        prep_id: 1,
    };
    let session = SessionKey {
        anchor_slot: meal.slot,
        role: SessionRole::Cooking,
        ordinal: 1,
    };
    let mut occupied = OccupancySnapshot {
        query: Some(OccupancyQuery {
            horizon: TimeInterval {
                start: moment("17:00:00"),
                end: moment("17:10:00"),
            },
            excluded_subject: None,
        }),
        occupancy_epoch: Revision(1),
        owners: vec![],
        digest: zero(),
    };
    occupied.digest = occupancy_snapshot_digest(&occupied).unwrap();
    let schedule = ScheduleInputs {
        layout: ScheduleLayout {
            routes: vec![PhaseRoute {
                phase: phase.clone(),
                session,
            }],
            fixed_starts: vec![PhaseStartConstraint {
                phase,
                start: moment("17:00:00"),
            }],
            attachments: vec![],
        },
        occupied,
    };
    let keys = required_identity_keys(&plan, &context, &evidence, &schedule).unwrap();
    let mut identities = FoodIdentities {
        revision: Revision(1),
        digest: zero(),
        allocations: keys
            .allocations
            .into_iter()
            .enumerate()
            .map(|(n, k)| (k, AllocationId(format!("al_{:032x}", n + 1))))
            .collect(),
        sessions: keys
            .sessions
            .into_iter()
            .enumerate()
            .map(|(n, k)| (k, PrepSessionId(format!("ps_{:032x}", n + 1))))
            .collect(),
        demand_lines: keys
            .demand_lines
            .into_iter()
            .enumerate()
            .map(|(n, k)| (k, DemandLineId(format!("dl_{:032x}", n + 1))))
            .collect(),
    };
    identities.digest = food_identities_digest(&identities).unwrap();
    let products = ProductUseSnapshot {
        revision: Revision(1),
        digest: zero(),
        valid_until: None,
        evidence: vec![],
        bindings: vec![],
        facts: vec![],
    };
    let mut inputs = FeasibilityInputs {
        revision: Revision(1),
        digest: zero(),
        delivery: DeliveryInput::Proposed {
            start: moment("15:00:00"),
            end: moment("16:00:00"),
            revision: Revision(1),
        },
        products,
        confirmations: vec![],
        evaluated_at: Instant("2026-12-28T20:00:00Z".into()),
    };
    rehash(&mut inputs);
    let derived = derive_plan(
        &plan,
        &context,
        &FoodInputs {
            evidence: &evidence,
            identities: &identities,
            schedule: &schedule,
            feasibility: &inputs,
        },
    )
    .unwrap();
    (
        SavedFoodBasis {
            plan,
            context,
            evidence,
            identities,
            schedule,
        },
        inputs,
        derived,
    )
}
fn rehash(inputs: &mut FeasibilityInputs) {
    inputs.products.digest = digest_product_uses(&inputs.products).unwrap();
    inputs.digest = digest_feasibility_inputs(inputs).unwrap();
}
fn bind(
    inputs: &mut FeasibilityInputs,
    usage: &DemandUse,
    number: usize,
    date: Option<&str>,
    shorter: Vec<DeadlineFact>,
) {
    let scope = ProductUseScope {
        line_id: usage.line_id.clone(),
        product: ProductRef {
            retailer: Retailer::Tesco,
            product_id: Text(format!("synthetic-{number}")),
            variant_id: Nullable(None),
        },
        use_key: usage.use_key.clone(),
    };
    inputs.products.evidence.push(EvidencePin {
        evidence_id: pin(number),
        revision: Revision(1),
        digest: Digest(format!("{number:064x}")),
    });
    inputs.products.bindings.push(ProductUseBinding {
        scope: scope.clone(),
        evidence_ids: vec![pin(number)],
    });
    inputs.products.facts.push(ProductUseFacts {
        scope,
        use_by: date.map(|d| DateFact {
            use_by: LocalDate(d.into()),
            evidence_ids: vec![pin(number)],
        }),
        shorter,
    });
    inputs
        .products
        .evidence
        .sort_by(|a, b| a.evidence_id.cmp(&b.evidence_id));
    inputs
        .products
        .bindings
        .sort_by(|a, b| scope_cmp(&a.scope, &b.scope));
    inputs
        .products
        .facts
        .sort_by(|a, b| scope_cmp(&a.scope, &b.scope));
}

#[test]
fn food_full_derivation_reconstructs_300_unknown_uses_and_retains_blocking_report() {
    let (saved, mut inputs, derived) = full_fixture(150);
    assert_eq!(derived.plan.demand.lines.len(), 150);
    assert_eq!(derived.feasibility.uses.len(), 300);
    assert_eq!(derived.feasibility.warnings.len(), 1);
    assert!(derived.feasibility.blocking.is_empty());
    let graph = mstrchf_core::demand::reconstruct_use_graph(&derived.plan, &saved).unwrap();
    for (n, usage) in graph.uses.iter().enumerate() {
        bind(&mut inputs, usage, n + 1, None, vec![]);
    }
    rehash(&mut inputs);
    let report = assess_feasibility(&derived.plan, &saved, &inputs).unwrap();
    assert_eq!(report.uses.len(), 300);
    assert_eq!(inputs.products.evidence.len(), 300);
    assert_eq!(report.warnings.len(), 1);
    assert!(report.blocking.is_empty());
    inputs.products.facts[0].use_by = Some(DateFact {
        use_by: LocalDate("2026-12-27".into()),
        evidence_ids: vec![pin(1)],
    });
    rehash(&mut inputs);
    let failure = derive_plan(
        &saved.plan,
        &saved.context,
        &FoodInputs {
            evidence: &saved.evidence,
            identities: &saved.identities,
            schedule: &saved.schedule,
            feasibility: &inputs,
        },
    )
    .unwrap_err();
    match failure {
        FoodFailure::Blocked {
            feasibility: Some(report),
            ..
        } => {
            assert_eq!(report.uses.len(), 300);
            assert_eq!(report.blocking.len(), 1);
            assert!(
                report
                    .uses
                    .iter()
                    .any(|c| c.issues.contains(&UseIssue::UseByConflict))
            );
            assert!(
                report
                    .uses
                    .iter()
                    .any(|c| c.issues.contains(&UseIssue::UnknownProductDate))
            );
        }
        other => panic!("report discarded: {other:?}"),
    }
}

#[test]
fn food_opening_lower_upper_boundaries_keep_unknown_and_conflict_flags() {
    let (saved, initial, derived) = full_fixture(1);
    let graph = mstrchf_core::demand::reconstruct_use_graph(&derived.plan, &saved).unwrap();
    let usage = graph
        .uses
        .iter()
        .find(|u| matches!(u.use_key, UseKey::Consumption { .. }))
        .unwrap();
    for (at, lower, seconds, upper, chronology, expiry) in [
        (
            "17:59:59",
            DeadlineBoundary::Inclusive,
            2,
            DeadlineBoundary::Inclusive,
            false,
            false,
        ),
        (
            "18:00:00",
            DeadlineBoundary::Inclusive,
            1,
            DeadlineBoundary::Inclusive,
            false,
            false,
        ),
        (
            "18:00:01",
            DeadlineBoundary::Inclusive,
            1,
            DeadlineBoundary::Inclusive,
            true,
            false,
        ),
        (
            "18:00:00",
            DeadlineBoundary::Exclusive,
            1,
            DeadlineBoundary::Inclusive,
            true,
            false,
        ),
        (
            "17:59:59",
            DeadlineBoundary::Inclusive,
            1,
            DeadlineBoundary::Exclusive,
            false,
            true,
        ),
        (
            "17:59:59",
            DeadlineBoundary::Inclusive,
            1,
            DeadlineBoundary::Inclusive,
            false,
            false,
        ),
    ] {
        let mut inputs = initial.clone();
        bind(
            &mut inputs,
            usage,
            1,
            None,
            vec![DeadlineFact {
                rule: DeadlineRule::AfterOpening {
                    anchor: OpeningAnchor::Observed {
                        at: moment(at),
                        evidence_id: pin(1),
                    },
                    opening_boundary: lower,
                    maximum_elapsed_seconds: seconds,
                    boundary: upper,
                },
                evidence_ids: vec![pin(1)],
            }],
        );
        rehash(&mut inputs);
        let report = assess_feasibility(&derived.plan, &saved, &inputs).unwrap();
        let check = report
            .uses
            .iter()
            .find(|u| u.use_key == usage.use_key)
            .unwrap();
        assert_eq!(
            check.issues.contains(&UseIssue::OpeningChronologyConflict),
            chronology
        );
        assert_eq!(
            check.issues.contains(&UseIssue::ShorterDeadlineConflict),
            expiry
        );
        assert!(check.issues.contains(&UseIssue::UnknownProductDate));
        assert_eq!(report.blocking.len(), usize::from(chronology || expiry));
    }
}

#[test]
fn food_delivery_uses_preparation_and_exact_confirmation_only() {
    let (saved, mut inputs, derived) = full_fixture(1);
    inputs.delivery = DeliveryInput::Proposed {
        start: moment("16:00:00"),
        end: moment("17:00:01"),
        revision: Revision(1),
    };
    rehash(&mut inputs);
    let blocked = assess_feasibility(&derived.plan, &saved, &inputs).unwrap();
    assert_eq!(blocked.blocking.len(), 1);
    let mut c = confirmation(&derived.plan.demand);
    c.confirmed_at = inputs.evaluated_at.clone();
    inputs.confirmations = vec![c];
    rehash(&mut inputs);
    let covered = assess_feasibility(&derived.plan, &saved, &inputs).unwrap();
    assert!(covered.blocking.is_empty());
    assert_eq!(covered.uses.len(), 2);
    inputs.confirmations[0].displayed_quantity.amount = PositiveRational::new(2, 1).unwrap();
    rehash(&mut inputs);
    assert_eq!(
        assess_feasibility(&derived.plan, &saved, &inputs)
            .unwrap()
            .blocking
            .len(),
        1
    );
    inputs.confirmations.clear();
    inputs.delivery = DeliveryInput::Proposed {
        start: moment("16:00:00"),
        end: moment("17:00:00"),
        revision: Revision(1),
    };
    rehash(&mut inputs);
    assert!(
        assess_feasibility(&derived.plan, &saved, &inputs)
            .unwrap()
            .blocking
            .is_empty()
    );
}

#[test]
fn food_planned_opening_is_exact_phase_and_can_follow_evaluation() {
    let (saved, initial, derived) = full_fixture(1);
    let graph = mstrchf_core::demand::reconstruct_use_graph(&derived.plan, &saved).unwrap();
    let usage = graph
        .uses
        .iter()
        .find(|u| matches!(u.use_key, UseKey::Preparation { .. }))
        .unwrap();
    let phase = match &usage.use_key {
        UseKey::Preparation { phase } => phase.clone(),
        _ => unreachable!(),
    };
    for lower in [DeadlineBoundary::Inclusive, DeadlineBoundary::Exclusive] {
        let mut inputs = initial.clone();
        inputs.evaluated_at = Instant("2026-12-28T16:00:00Z".into());
        bind(
            &mut inputs,
            usage,
            1,
            Some("2026-12-28"),
            vec![DeadlineFact {
                rule: DeadlineRule::AfterOpening {
                    anchor: OpeningAnchor::PlannedPhaseStart {
                        phase: phase.clone(),
                    },
                    opening_boundary: lower,
                    maximum_elapsed_seconds: 3600,
                    boundary: DeadlineBoundary::Inclusive,
                },
                evidence_ids: vec![pin(1)],
            }],
        );
        rehash(&mut inputs);
        let report = assess_feasibility(&derived.plan, &saved, &inputs).unwrap();
        assert_eq!(
            report.blocking.len(),
            usize::from(lower == DeadlineBoundary::Exclusive)
        );
        inputs.products.facts[0].shorter[0].rule = DeadlineRule::AfterOpening {
            anchor: OpeningAnchor::Observed {
                at: moment("17:00:00"),
                evidence_id: pin(1),
            },
            opening_boundary: lower,
            maximum_elapsed_seconds: 3600,
            boundary: DeadlineBoundary::Inclusive,
        };
        rehash(&mut inputs);
        assert_eq!(
            assess_feasibility(&derived.plan, &saved, &inputs)
                .unwrap_err()
                .code,
            ErrorCode::InvalidValue
        );
    }
    let mut inputs = initial;
    let mut foreign = phase;
    foreign.prep_id = 2;
    bind(
        &mut inputs,
        usage,
        1,
        None,
        vec![DeadlineFact {
            rule: DeadlineRule::AfterOpening {
                anchor: OpeningAnchor::PlannedPhaseStart { phase: foreign },
                opening_boundary: DeadlineBoundary::Inclusive,
                maximum_elapsed_seconds: 3600,
                boundary: DeadlineBoundary::Inclusive,
            },
            evidence_ids: vec![pin(1)],
        }],
    );
    rehash(&mut inputs);
    assert_eq!(
        assess_feasibility(&derived.plan, &saved, &inputs)
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    );
}

#[test]
fn food_absolute_deadline_checks_equal_instants_across_zones_and_all_products() {
    let (saved, initial, derived) = full_fixture(1);
    let graph = mstrchf_core::demand::reconstruct_use_graph(&derived.plan, &saved).unwrap();
    let usage = graph
        .uses
        .iter()
        .find(|u| matches!(u.use_key, UseKey::Consumption { .. }))
        .unwrap();
    let paris = ZonedMoment::new(
        LocalDate("2026-12-28".into()),
        LocalTime("19:00:00".into()),
        ZoneId("Europe/Paris".into()),
        Resolution::Unambiguous,
        Instant("2026-12-28T18:00:00Z".into()),
    )
    .unwrap();
    for boundary in [DeadlineBoundary::Inclusive, DeadlineBoundary::Exclusive] {
        let mut inputs = initial.clone();
        bind(
            &mut inputs,
            usage,
            1,
            Some("2026-12-28"),
            vec![DeadlineFact {
                rule: DeadlineRule::Absolute {
                    latest_use: paris.clone(),
                    boundary,
                },
                evidence_ids: vec![pin(1)],
            }],
        );
        bind(&mut inputs, usage, 2, Some("2026-12-29"), vec![]);
        rehash(&mut inputs);
        let report = assess_feasibility(&derived.plan, &saved, &inputs).unwrap();
        assert_eq!(
            report.blocking.len(),
            usize::from(boundary == DeadlineBoundary::Exclusive)
        );
    }
    let mut inputs = initial;
    bind(
        &mut inputs,
        usage,
        1,
        Some("2026-12-27"),
        vec![DeadlineFact {
            rule: DeadlineRule::AfterOpening {
                anchor: OpeningAnchor::Unknown,
                opening_boundary: DeadlineBoundary::Inclusive,
                maximum_elapsed_seconds: 3600,
                boundary: DeadlineBoundary::Inclusive,
            },
            evidence_ids: vec![pin(1)],
        }],
    );
    bind(&mut inputs, usage, 2, Some("2026-12-29"), vec![]);
    rehash(&mut inputs);
    let report = assess_feasibility(&derived.plan, &saved, &inputs).unwrap();
    let check = report
        .uses
        .iter()
        .find(|u| u.use_key == usage.use_key)
        .unwrap();
    assert_eq!(
        check.issues,
        vec![UseIssue::UnknownOpeningAnchor, UseIssue::UseByConflict]
    );
    assert_eq!(
        check.assessment,
        ProductDateAssessment::ConflictingKnownLimit
    );
}

#[test]
fn food_later_opening_conflict_survives_an_earlier_controlling_deadline() {
    let (saved, mut inputs, derived) = full_fixture(1);
    let graph = mstrchf_core::demand::reconstruct_use_graph(&derived.plan, &saved).unwrap();
    let usage = graph
        .uses
        .iter()
        .find(|u| matches!(u.use_key, UseKey::Consumption { .. }))
        .unwrap();
    for (n, at, seconds) in [(1, "17:00:00", 1800), (2, "18:00:01", 3600)] {
        bind(
            &mut inputs,
            usage,
            n,
            Some("2026-12-28"),
            vec![DeadlineFact {
                rule: DeadlineRule::AfterOpening {
                    anchor: OpeningAnchor::Observed {
                        at: moment(at),
                        evidence_id: pin(n),
                    },
                    opening_boundary: DeadlineBoundary::Inclusive,
                    maximum_elapsed_seconds: seconds,
                    boundary: DeadlineBoundary::Inclusive,
                },
                evidence_ids: vec![pin(n)],
            }],
        );
    }
    rehash(&mut inputs);
    let report = assess_feasibility(&derived.plan, &saved, &inputs).unwrap();
    let check = report
        .uses
        .iter()
        .find(|u| u.use_key == usage.use_key)
        .unwrap();
    assert_eq!(
        check.issues,
        vec![
            UseIssue::OpeningChronologyConflict,
            UseIssue::ShorterDeadlineConflict
        ]
    );
    assert_eq!(report.blocking.len(), 1);
}

#[test]
fn food_feasibility_rejects_expired_forged_missing_and_foreign_scopes() {
    let (saved, initial, derived) = full_fixture(1);
    let graph = mstrchf_core::demand::reconstruct_use_graph(&derived.plan, &saved).unwrap();
    let usage = &graph.uses[0];
    let mut inputs = initial.clone();
    bind(&mut inputs, usage, 1, None, vec![]);
    rehash(&mut inputs);
    let mut altered = inputs.clone();
    altered.products.facts[0].use_by = Some(DateFact {
        use_by: LocalDate("2026-12-27".into()),
        evidence_ids: vec![pin(1)],
    });
    assert_eq!(
        assess_feasibility(&derived.plan, &saved, &altered)
            .unwrap_err()
            .code,
        ErrorCode::DigestMismatch
    );
    let mut missing = inputs.products.clone();
    missing.evidence.clear();
    assert!(digest_product_uses(&missing).is_err());
    let mut duplicate = inputs.products.clone();
    duplicate.bindings.push(duplicate.bindings[0].clone());
    duplicate.facts.push(duplicate.facts[0].clone());
    assert!(digest_product_uses(&duplicate).is_err());
    inputs.products.bindings[0].scope.line_id = DemandLineId(format!("dl_{:032x}", 999));
    inputs.products.facts[0].scope = inputs.products.bindings[0].scope.clone();
    rehash(&mut inputs);
    assert_eq!(
        assess_feasibility(&derived.plan, &saved, &inputs)
            .unwrap_err()
            .code,
        ErrorCode::ForeignReference
    );
    let mut expired = initial;
    expired.products.valid_until = Some(expired.evaluated_at.clone());
    expired.products.digest = digest_product_uses(&expired.products).unwrap();
    assert_eq!(
        digest_feasibility_inputs(&expired).unwrap_err().code,
        ErrorCode::ContextStale
    );
    let mut tampered = derived.plan;
    tampered.demand.lines[0].quantity.amount = PositiveRational::new(2, 1).unwrap();
    assert!(mstrchf_core::demand::reconstruct_use_graph(&tampered, &saved).is_err());
}

fn reference(recipe: &RecipeRevision) -> RecipeRef {
    RecipeRef {
        recipe_id: recipe.recipe_id.clone(),
        revision: recipe.revision,
        digest: recipe_digest(recipe).unwrap(),
    }
}

// Rebind edited synthetic recipe material through real A admission and rebuild
// exact native identities rather than patching already-derived quantities.
fn rebind_fixture(saved: &mut SavedFoodBasis) {
    let refs: Vec<_> = saved.context.recipes.iter().map(reference).collect();
    saved.context.stamp.recipe_refs = BoundedVec(refs.clone());
    for s in &mut saved.plan.selections.0 {
        s.recipe = refs
            .iter()
            .find(|r| r.recipe_id == s.recipe.recipe_id)
            .unwrap()
            .clone();
        for serving in &mut s.servings.0 {
            serving.preset_ref.recipe = s.recipe.clone();
        }
        for e in &mut s.extras.0 {
            e.recipe = refs
                .iter()
                .find(|r| r.recipe_id == e.recipe.recipe_id)
                .unwrap()
                .clone();
            for serving in &mut e.recipients.0 {
                serving.preset_ref.recipe = e.recipe.clone();
            }
        }
    }
    let template = saved.evidence.recipes[0].clone();
    saved.evidence.recipes = saved
        .context
        .recipes
        .iter()
        .map(|r| {
            let mut b = template.clone();
            b.recipe = reference(r);
            b
        })
        .collect();
    saved.evidence.classes = saved
        .context
        .recipes
        .iter()
        .map(|r| {
            ComponentClassView::new(
                reference(r),
                r.components
                    .iter()
                    .map(|c| ComponentClassEvidence {
                        recipe: reference(r),
                        component_id: c.component_id.clone(),
                        class: ComponentClass::General,
                        evidence_ids: vec![pin(8)],
                    })
                    .collect(),
            )
            .unwrap()
        })
        .collect();
    saved.evidence.methods = refs
        .iter()
        .map(|r| MethodPhases {
            portable_cold: None,
            recipe: r.clone(),
            kind: MethodKind::Production,
            prep_ids: vec![1],
            direct_reuse_counts: vec![],
            evidence_ids: vec![pin(8)],
        })
        .collect();
    saved.evidence.resources = refs
        .iter()
        .map(|r| PhaseResources {
            recipe: r.clone(),
            prep_id: 1,
            equipment: vec![EquipmentCode::Hob],
            evidence_ids: vec![pin(8)],
        })
        .collect();
    saved.evidence.catalogue_digest = food_evidence_digest(&saved.evidence).unwrap();
    let mut routes = vec![];
    let mut fixed_starts = vec![];
    for s in &saved.plan.selections.0 {
        for (n, (extra_id, recipe)) in std::iter::once((None, s.recipe.clone()))
            .chain(
                s.extras
                    .iter()
                    .map(|e| (Some(e.extra_id.clone()), e.recipe.clone())),
            )
            .enumerate()
        {
            let phase = PhaseKey {
                occurrence: OccurrenceKey {
                    slot: s.slot.clone(),
                    extra_id,
                    recipe,
                },
                prep_id: 1,
            };
            let session = SessionKey {
                anchor_slot: s.slot.clone(),
                role: SessionRole::Cooking,
                ordinal: (n + 1) as u16,
            };
            let time = if n == 0 { "17:00:00" } else { "17:10:00" };
            let start = ZonedMoment::new(
                s.slot.date.clone(),
                LocalTime(time.into()),
                ZoneId("Europe/London".into()),
                Resolution::Unambiguous,
                Instant(format!("{}T{time}Z", s.slot.date.0)),
            )
            .unwrap();
            routes.push(PhaseRoute {
                phase: phase.clone(),
                session,
            });
            fixed_starts.push(PhaseStartConstraint { phase, start });
        }
    }
    routes.sort_by(|a, b| phase_cmp(&a.phase, &b.phase));
    fixed_starts.sort_by(|a, b| phase_cmp(&a.phase, &b.phase));
    let first = fixed_starts
        .iter()
        .min_by_key(|f| f.start.instant.clone())
        .unwrap()
        .start
        .clone();
    let last = fixed_starts
        .iter()
        .max_by_key(|f| f.start.instant.clone())
        .unwrap()
        .start
        .clone();
    let end = ZonedMoment::new(
        last.local_date.clone(),
        LocalTime("17:20:00".into()),
        last.zone.clone(),
        Resolution::Unambiguous,
        Instant(format!("{}T17:20:00Z", last.local_date.0)),
    )
    .unwrap();
    saved.schedule.layout = ScheduleLayout {
        routes,
        fixed_starts,
        attachments: vec![],
    };
    saved.schedule.occupied.query = Some(OccupancyQuery {
        horizon: TimeInterval { start: first, end },
        excluded_subject: None,
    });
    saved.schedule.occupied.digest = occupancy_snapshot_digest(&saved.schedule.occupied).unwrap();
    let keys = required_identity_keys(
        &saved.plan,
        &saved.context,
        &saved.evidence,
        &saved.schedule,
    )
    .unwrap();
    saved.identities.allocations = keys
        .allocations
        .into_iter()
        .enumerate()
        .map(|(n, k)| (k, AllocationId(format!("al_{:032x}", n + 1))))
        .collect();
    saved.identities.sessions = keys
        .sessions
        .into_iter()
        .enumerate()
        .map(|(n, k)| (k, PrepSessionId(format!("ps_{:032x}", n + 1))))
        .collect();
    saved.identities.demand_lines = keys
        .demand_lines
        .into_iter()
        .enumerate()
        .map(|(n, k)| (k, DemandLineId(format!("dl_{:032x}", n + 1))))
        .collect();
    saved.identities.digest = food_identities_digest(&saved.identities).unwrap();
}

#[test]
fn food_repeated_extra_id_expands_each_occurrence_and_preserves_use_provenance() {
    let (mut saved, inputs, _) = full_fixture(1);
    let mut extra = saved.context.recipes[0].clone();
    extra.recipe_id = RecipeId(format!("rc_{:032x}", 2));
    let iid = IngredientId(format!("ig_{:032x}", 2));
    let cid = ComponentId(format!("co_{:032x}", 2));
    extra.ingredients[0].ingredient_id = iid.clone();
    extra.ingredients[0].component_id = cid.clone();
    extra.components[0].component_id = cid.clone();
    extra.components[0].ingredient_ids = BoundedVec(vec![iid.clone()]);
    extra.r#yield.components[0].component_id = cid.clone();
    extra.storage[0].component_id = cid.clone();
    extra.steps[0].ingredient_ids = BoundedVec(vec![iid.clone()]);
    extra.steps[0].component_ids = BoundedVec(vec![cid]);
    extra.prep[0].ingredient_ids = BoundedVec(vec![iid.clone()]);
    let extra_ref = reference(&extra);
    let extra_id = ExtraId(format!("ex_{:032x}", 1));
    let base_component = saved.context.recipes[0].components[0].component_id.clone();
    saved.context.recipes[0].extras.0.push(ExtraSpec {
        extra_id: extra_id.clone(),
        title: Title("Synthetic extra".into()),
        recipe: extra_ref.clone(),
        eligible_person_requirements: BoundedVec(vec![]),
        supported_with: BoundedVec(vec![base_component]),
    });
    saved.context.recipes.0.push(extra);
    let mut recipients = saved.plan.selections[0].servings.clone();
    recipients[0].preset_ref.recipe = extra_ref.clone();
    saved.plan.selections[0].extras.0.push(ExtraSelection {
        extra_id: extra_id.clone(),
        recipe: extra_ref,
        recipients,
    });
    let mut second = saved.plan.selections[0].clone();
    second.slot.date = LocalDate("2026-12-29".into());
    second.batch_use = BatchUse::NewBatch {
        batch_id: BatchId(format!("ba_{:032x}", 2)),
        production_servings: PositiveRational::new(1, 1).unwrap(),
    };
    saved.plan.selections.0.push(second);
    rebind_fixture(&mut saved);
    let result = derive_plan(
        &saved.plan,
        &saved.context,
        &FoodInputs {
            evidence: &saved.evidence,
            identities: &saved.identities,
            schedule: &saved.schedule,
            feasibility: &inputs,
        },
    )
    .unwrap();
    let extra_line = result
        .plan
        .demand
        .lines
        .iter()
        .find(|l| l.ingredient_id == iid)
        .unwrap();
    assert_eq!(
        extra_line.quantity.amount,
        PositiveRational::new(2, 1).unwrap()
    );
    assert_eq!(extra_line.origin_extras.0, vec![extra_id.clone()]);
    assert!(extra_line.origin_batches.is_empty());
    let graph = mstrchf_core::demand::reconstruct_use_graph(&result.plan, &saved).unwrap();
    let extra_uses: Vec<_> = graph
        .uses
        .iter()
        .filter(|u| u.line_id == extra_line.line_id)
        .collect();
    assert_eq!(extra_uses.len(), 4);
    let consumption: Vec<_> = extra_uses
        .iter()
        .filter_map(|u| match &u.use_key {
            UseKey::Consumption { occurrence, .. } => Some(occurrence),
            _ => None,
        })
        .collect();
    assert_eq!(consumption.len(), 2);
    assert_ne!(consumption[0].slot, consumption[1].slot);
    assert!(
        consumption
            .iter()
            .all(|o| o.extra_id.as_ref() == Some(&extra_id))
    );
}

#[test]
fn food_native_bounds_reject_overflow_without_unknown_summary_expansion() {
    // Output/native validators only. A 16384-row report is not claimed as a
    // fully derived cooking schedule or admitted product projection.
    let (_, inputs, derived) = full_fixture(1);
    let mut report = derived.feasibility.clone();
    report.uses = (1..=16384)
        .map(|n| {
            let mut u = report.uses[0].clone();
            u.line_id = DemandLineId(format!("dl_{n:032x}"));
            u
        })
        .collect();
    report.digest = digest_feasibility_report(&report).unwrap();
    assert_eq!(report.warnings.len(), 1);
    assert!(report.blocking.is_empty());
    let mut overflow = report.clone();
    let mut u = overflow.uses[0].clone();
    u.line_id = DemandLineId(format!("dl_{:032x}", 16385));
    overflow.uses.push(u);
    assert_eq!(
        digest_feasibility_report(&overflow).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
    let mut products = inputs.products.clone();
    products.evidence = (1..=4097)
        .map(|n| EvidencePin {
            evidence_id: pin(n),
            revision: Revision(1),
            digest: Digest(format!("{n:064x}")),
        })
        .collect();
    assert_eq!(
        digest_product_uses(&products).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
    products.evidence.truncate(257);
    let scope = ProductUseScope {
        line_id: derived.plan.demand.lines[0].line_id.clone(),
        product: ProductRef {
            retailer: Retailer::Tesco,
            product_id: Text("synthetic".into()),
            variant_id: Nullable(None),
        },
        use_key: derived.feasibility.uses[0].use_key.clone(),
    };
    products.bindings = vec![ProductUseBinding {
        scope: scope.clone(),
        evidence_ids: (1..=257).map(pin).collect(),
    }];
    products.facts = vec![ProductUseFacts {
        scope,
        use_by: None,
        shorter: vec![],
    }];
    assert_eq!(
        digest_product_uses(&products).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
    let mut excessive_details = derived.feasibility;
    excessive_details.other_checks = (1..=101)
        .map(|n| Conflict {
            code: ErrorCode::DomainConflict,
            week_ids: BoundedVec(vec![derived.plan.week_id.clone()]),
            slot_ids: BoundedVec(vec![]),
            recipe_refs: BoundedVec(vec![RecipeRef {
                recipe_id: RecipeId(format!("rc_{n:032x}")),
                revision: Revision(1),
                digest: RecipeDigest("a".repeat(64)),
            }]),
            evidence_ids: BoundedVec(vec![]),
            blocking: true,
        })
        .collect();
    excessive_details
        .other_checks
        .sort_by_key(|c| canonical_bytes(c).unwrap());
    assert_eq!(
        digest_feasibility_report(&excessive_details)
            .unwrap_err()
            .code,
        ErrorCode::LimitExceeded
    );
}
