//! Synthetic F11 construction. Ingredients and portions are oracle literals;
//! native premises are explicit test inputs, not claims about real cooking.
use mstrchf_contracts::*;
use mstrchf_core::{availability, food::*, portions, recipes::*};

fn p(n: u64, d: u64) -> PositiveRational {
    PositiveRational::new(n, d).unwrap()
}
fn q(n: u64, unit: Unit, form: IngredientForm) -> Quantity {
    Quantity {
        amount: p(n, 1),
        unit,
        form,
    }
}
fn rf(r: &RecipeRevision) -> RecipeRef {
    RecipeRef {
        recipe_id: r.recipe_id.clone(),
        revision: r.revision,
        digest: recipe_digest(r).unwrap(),
    }
}
pub fn at(day: u8, time: &str) -> ZonedMoment {
    availability::resolve_meal_time(
        &LocalDate(format!("2026-10-{day:02}")),
        &LocalTime(time.into()),
        &ZoneId("Europe/London".into()),
        None,
    )
    .unwrap()
}

pub fn fixture() -> SavedFoodBasis {
    let v: serde_json::Value =
        serde_json::from_slice(include_bytes!("native-v4-subject-basis.json")).unwrap();
    let mut context: PlanningContext =
        serde_json::from_value(v["value"]["context"].clone()).unwrap();
    let prototype = context.recipes[0].clone();
    let evidence_id = prototype.reviews[0].evidence_id.clone();
    let source_component = ComponentId(format!("co_{:032x}", 401));
    let ingredients = [
        ("beans", 480, Unit::G, IngredientForm::Drained),
        ("tomatoes", 400, Unit::G, IngredientForm::Raw),
        ("onion", 160, Unit::G, IngredientForm::Raw),
        ("oil", 20, Unit::Ml, IngredientForm::Prepared),
        ("wrap", 1, Unit::Each, IngredientForm::Prepared),
        ("potato", 250, Unit::G, IngredientForm::Raw),
    ];
    let make_ingredient = |index: usize, component: &ComponentId| {
        let (name, n, unit, form) = ingredients[index];
        let mut value = prototype.ingredients[0].clone();
        value.ingredient_id = IngredientId(format!("ig_{:032x}", 401 + index));
        value.name = NameText(format!("Synthetic {name}"));
        value.quantity = Nullable(Some(q(n, unit, form)));
        value.component_id = component.clone();
        value.step_ids = BoundedVec(vec![1]);
        value.discrete = index == 4;
        value
    };
    let mut recipes = Vec::new();
    for index in 0..3 {
        let mut recipe = prototype.clone();
        recipe.recipe_id = RecipeId(format!("rc_{:032x}", 401 + index));
        recipe.title = Title(format!(
            "Synthetic F11 {}",
            ["chilli", "wrap remix", "potato remix"][index]
        ));
        let component = ComponentId(format!("co_{:032x}", 401 + index));
        recipe.ingredients = BoundedVec(if index == 0 {
            (0..4).map(|n| make_ingredient(n, &component)).collect()
        } else {
            vec![make_ingredient(index + 3, &component)]
        });
        recipe.r#yield.servings = if index == 0 { p(4, 1) } else { p(1, 1) };
        recipe.r#yield.basis =
            Text("Synthetic declared component yield; no inferred mass balance".into());
        recipe.r#yield.components = BoundedVec(vec![ComponentYield {
            component_id: component.clone(),
            quantity: match index {
                0 => q(1200, Unit::G, IngredientForm::Cooked),
                1 => q(1, Unit::Each, IngredientForm::Prepared),
                _ => q(250, Unit::G, IngredientForm::Raw),
            },
        }]);
        recipe.scaling = ScalingPolicy::Linear {
            minimum: p(1, 4),
            maximum: p(4, 1),
        };
        let ids = recipe
            .ingredients
            .iter()
            .map(|i| i.ingredient_id.clone())
            .collect::<Vec<_>>();
        recipe.components[0].component_id = component.clone();
        recipe.components[0].ingredient_ids = BoundedVec(ids.clone());
        recipe.components[0].production = Nullable(Some(ComponentProductionSpec::Cooked {
            completion_prep_id: 1,
        }));
        recipe.steps.0.truncate(1);
        recipe.steps[0].text = Text("Synthetic reviewed atomic production and assembly".into());
        recipe.steps[0].ingredient_ids = BoundedVec(ids.clone());
        recipe.steps[0].component_ids = BoundedVec(if index == 0 {
            vec![component.clone()]
        } else {
            vec![source_component.clone(), component.clone()]
        });
        recipe.prep.0.truncate(1);
        recipe.prep[0].ingredient_ids = BoundedVec(ids);
        recipe.active_minutes = 10;
        recipe.elapsed_minutes = 10;
        recipe.portability.reheating_required = false;
        recipe.portability.cold_storage_required = index != 0;
        recipe.portability.cold_suitable = false;
        if index == 0 {
            recipe.storage[0].component_id = component;
            recipe.storage[0].max_chilled_seconds = 172800;
            recipe.storage[0].max_reheats = 1;
        } else {
            recipe.components[0].storage_rule_ids.0.clear();
            recipe.storage.0.clear();
            recipe.capabilities.leftovers = false;
            recipe.reused_inputs = BoundedVec(vec![ReusedComponentInput {
                source_recipe: rf(&recipes[0]),
                source_component_id: source_component.clone(),
                quantity_per_recipe_yield: q(300, Unit::G, IngredientForm::Cooked),
                step_ids: BoundedVec(vec![1]),
                planned_reheats: 1,
                evidence_ids: BoundedVec(vec![evidence_id.clone()]),
            }]);
        }
        recipes.push(recipe);
    }
    let refs = recipes.iter().map(rf).collect::<Vec<_>>();
    let mut evidence = FoodEvidence {
        catalogue_revision: context.stamp.catalogue_revision,
        catalogue_digest: PayloadDigest("0".repeat(64)),
        policy_version: Revision(1),
        recipes: vec![],
        classes: vec![],
        methods: vec![],
        resources: vec![],
        essential_ingredient_ids: vec![],
    };
    for (recipe, reference) in recipes.iter().zip(&refs) {
        let basis = EvidenceBasis::new(
            reference.clone(),
            vec![EvidencePin {
                evidence_id: evidence_id.clone(),
                revision: Revision(1),
                digest: Digest("4".repeat(64)),
            }],
            vec![],
        )
        .unwrap();
        let classes = ComponentClassView::new(
            reference.clone(),
            vec![ComponentClassEvidence {
                recipe: reference.clone(),
                component_id: recipe.components[0].component_id.clone(),
                class: ComponentClass::General,
                evidence_ids: vec![evidence_id.clone()],
            }],
        )
        .unwrap();
        assert!(
            assess_revision(recipe, &basis, &classes)
                .unwrap()
                .assessment()
                .capabilities
                .shopping
        );
        evidence.recipes.push(basis);
        evidence.classes.push(classes);
        evidence.methods.push(MethodPhases {
            recipe: reference.clone(),
            kind: MethodKind::Production,
            prep_ids: vec![1],
            direct_reuse_counts: vec![],
            evidence_ids: vec![evidence_id.clone()],
        });
        evidence.resources.push(PhaseResources {
            recipe: reference.clone(),
            prep_id: 1,
            equipment: vec![EquipmentCode::Hob],
            evidence_ids: vec![evidence_id.clone()],
        });
    }
    evidence.catalogue_digest = food_evidence_digest(&evidence).unwrap();
    context.recipes = BoundedVec(recipes);
    context.stamp.recipe_refs = BoundedVec(refs.clone());
    let week_id = context.weeks[0].range.week_id.clone();
    let person = PersonRef::Diner {
        diner_id: context.household.diners[0].diner_id.clone(),
    };
    let batch_id = BatchId(format!("ba_{:032x}", 401));
    let slots = [
        (5, MealType::Dinner),
        (6, MealType::Lunch),
        (7, MealType::Lunch),
    ]
    .map(|(d, t)| SlotId {
        week_id: week_id.clone(),
        date: LocalDate(format!("2026-10-{d:02}")),
        meal_type: t,
    });
    let mut plan = WeekPlan {
        week_id: week_id.clone(),
        revision: Revision(2),
        selections: BoundedVec(vec![]),
        batch_links: BoundedVec(vec![]),
    };
    for index in 0..3 {
        plan.selections.0.push(MealSelection {
            slot: slots[index].clone(),
            recipe: refs[index].clone(),
            origin: SelectionOrigin::Fixed,
            repeat_authorised: true,
            servings: BoundedVec(vec![DinerServing {
                person: person.clone(),
                preset_ref: PortionPresetRef {
                    recipe: refs[index].clone(),
                    preset: PortionPresetName::Regular,
                    age_band: AgeBand::Adult,
                    meal_type: slots[index].meal_type,
                },
                servings: if index == 0 { p(2, 1) } else { p(1, 1) },
            }]),
            extras: BoundedVec(vec![]),
            batch_use: if index == 0 {
                BatchUse::NewBatch {
                    batch_id: batch_id.clone(),
                    production_servings: p(4, 1),
                }
            } else {
                BatchUse::Leftover {
                    batch_id: batch_id.clone(),
                    component_ids: BoundedVec(vec![source_component.clone()]),
                }
            },
            meal_overrides: MealOverrides {
                portions: BoundedVec(if index == 0 {
                    vec![PersonPortionOverride {
                        person: person.clone(),
                        preset: PortionPresetName::Regular,
                        adjustment: p(2, 1),
                    }]
                } else {
                    vec![]
                }),
                eat_by: Nullable(Some(at(
                    5 + index as u8,
                    if index == 0 { "18:00:00" } else { "12:00:00" },
                ))),
            },
        });
        if index != 0 {
            plan.batch_links.0.push(BatchLink {
                batch_id: batch_id.clone(),
                source_slot: slots[0].clone(),
                target_slot: slots[index].clone(),
                components: BoundedVec(vec![ComponentAllocation {
                    component_id: source_component.clone(),
                    quantity: q(300, Unit::G, IngredientForm::Cooked),
                }]),
            });
        }
    }
    let week = &mut context.weeks[0];
    week.range.monday = LocalDate("2026-10-05".into());
    week.defaults_snapshot.suggest_leftovers = true;
    week.day_enabled = BoundedVec(vec![true; 7]);
    week.meal_type_enabled = MealTypeEnabled {
        breakfast: false,
        lunch: true,
        dinner: true,
    };
    for (index, cell) in week.cells.0.iter_mut().enumerate() {
        cell.slot.date = LocalDate(format!("2026-10-{:02}", 5 + index / 3));
        cell.choice = if slots.contains(&cell.slot) {
            MealLocation::AtHome
        } else {
            MealLocation::Skip
        };
        cell.attendee_ids = BoundedVec(if cell.choice == MealLocation::AtHome {
            vec![context.household.diners[0].diner_id.clone()]
        } else {
            vec![]
        });
        cell.occasion = Nullable(None);
    }
    week.plan = plan.clone();
    context.requirements = BoundedVec(
        availability::meal_requirements(&context.household, week, &[])
            .unwrap()
            .into(),
    );
    for selection in &plan.selections.0 {
        let index = refs.iter().position(|r| r == &selection.recipe).unwrap();
        let reviewed = assess_revision(
            &context.recipes[index],
            &evidence.recipes[index],
            &evidence.classes[index],
        )
        .unwrap();
        let meal = context
            .requirements
            .iter()
            .find(|m| m.slot == selection.slot)
            .unwrap();
        assert_eq!(
            portions::resolve_servings(&reviewed, meal, &selection.meal_overrides).unwrap(),
            selection.servings.0
        );
    }
    let mut layout = ScheduleLayout {
        routes: vec![],
        fixed_starts: vec![],
        attachments: vec![],
    };
    let mut identities = FoodIdentities {
        revision: Revision(1),
        digest: PayloadDigest("0".repeat(64)),
        allocations: vec![],
        sessions: vec![],
        demand_lines: vec![],
    };
    for index in 0..3 {
        let occurrence = OccurrenceKey {
            slot: slots[index].clone(),
            extra_id: None,
            recipe: refs[index].clone(),
        };
        let phase = PhaseKey {
            occurrence,
            prep_id: 1,
        };
        let session = SessionKey {
            anchor_slot: slots[index].clone(),
            role: SessionRole::Cooking,
            ordinal: 1,
        };
        layout.routes.push(PhaseRoute {
            phase: phase.clone(),
            session: session.clone(),
        });
        layout.fixed_starts.push(PhaseStartConstraint {
            phase,
            start: at(
                5 + index as u8,
                if index == 0 { "17:50:00" } else { "11:50:00" },
            ),
        });
        identities
            .sessions
            .push((session, PrepSessionId(format!("ps_{:032x}", 401 + index))));
        identities.allocations.push((
            AllocationKey {
                batch_id: batch_id.clone(),
                component_id: source_component.clone(),
                target_slot: slots[index].clone(),
                person: person.clone(),
            },
            AllocationId(format!("al_{:032x}", 401 + index)),
        ));
    }
    for (index, (_, _, unit, form)) in ingredients.iter().enumerate() {
        identities.demand_lines.push((
            DemandKey {
                ingredient_id: IngredientId(format!("ig_{:032x}", 401 + index)),
                unit: *unit,
                form: *form,
            },
            DemandLineId(format!("dl_{:032x}", 401 + index)),
        ));
    }
    identities.digest = food_identities_digest(&identities).unwrap();
    let mut occupied = OccupancySnapshot {
        query: Some(OccupancyQuery {
            horizon: TimeInterval {
                start: at(5, "17:50:00"),
                end: at(7, "12:00:00"),
            },
            excluded_subject: None,
        }),
        occupancy_epoch: Revision(1),
        owners: vec![],
        digest: PayloadDigest("0".repeat(64)),
    };
    occupied.digest = occupancy_snapshot_digest(&occupied).unwrap();
    SavedFoodBasis {
        plan,
        context,
        evidence,
        identities,
        schedule: ScheduleInputs { layout, occupied },
    }
}

pub fn feasibility() -> FeasibilityInputs {
    let mut products = ProductUseSnapshot {
        revision: Revision(1),
        digest: PayloadDigest("0".repeat(64)),
        valid_until: None,
        evidence: vec![],
        bindings: vec![],
        facts: vec![],
    };
    products.digest = digest_product_uses(&products).unwrap();
    let mut inputs = FeasibilityInputs {
        revision: Revision(1),
        digest: PayloadDigest("0".repeat(64)),
        delivery: DeliveryInput::Unselected,
        products,
        confirmations: vec![],
        evaluated_at: Instant("2026-10-04T12:00:00Z".into()),
    };
    inputs.digest = digest_feasibility_inputs(&inputs).unwrap();
    inputs
}
