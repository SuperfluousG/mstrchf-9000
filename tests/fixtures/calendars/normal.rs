//! C01: complete SYNTHETIC source history, distinct from renderer-only records.
use super::{Fixture, at, finish, p, seed};
use mstrchf_contracts::*;
use mstrchf_core::{availability, demand, food::*, recipes::*};

pub const BASE_PREP: &str = "At 18:00 turn the fictional electric oven grill to medium (setting 2 of 3). While it preheats, put 2 whole bread slices on a clean grill tray and spread the 60 g prepared grated cheese evenly over the two slices. Place the tray under the grill at 18:03, after the three-minute preheating and assembly phase.";
pub const BASE_GRILL: &str = "With the fictional electric oven grill still at medium (setting 2 of 3), grill the two cheese-topped bread slices unattended for seven minutes until the cheese has melted and the bread is toasted. At 18:10 serve one slice to Diner 1 and one slice to Diner 2. Discard uneaten toast after service because no storage method is admitted.";
pub const TOMATO_EXTRA: &str = "Wash 100 g cherry tomatoes under running water. Halve them on a clean board during the two-minute phase and put the entire 100 g extra on Diner 2's plate for service at 18:10. Diner 1 receives no tomato extra. Discard uneaten tomatoes after the meal.";
pub const OATS_PREP: &str = "Measure 40 g dry oats and 100 ml prepared milk into a clean covered container. Stir thoroughly during the three-minute preparation phase, cover and refrigerate immediately at 5 C or below by 18:08. Keep the container chilled overnight for Diner 1's Tuesday breakfast.";
pub const OATS_FINISH: &str = "At 07:55 remove Diner 1's covered oats container from the refrigerator. Stir the prepared oats and milk thoroughly, transfer the full portion to a clean bowl and serve cold by 08:00. Add no new ingredients and do not repeat Monday's preparation. Discard any remainder after breakfast.";
pub const OATS_STORAGE: &str = "Cover and refrigerate the prepared oats and milk at 5 C or below immediately after mixing, no later than Monday 18:08. Retain the original first-handling clock Monday 18:05 and use within 24 hours of that clock. Keep chilled until Tuesday 07:55; serve cold by 08:00. Discard any remainder.";
pub const OATS_NO_REHEAT: &str = "Serve this fixture's oats cold. No reheating method is admitted and the maximum reheats is zero.";
pub const C03_OATS_PREP: &str = "On Sunday at 19:00 measure 40 g dry oats and 100 ml prepared milk into a clean covered container. Stir thoroughly during the three-minute preparation phase, cover and refrigerate immediately at 5 C or below by 19:03. Keep the container chilled overnight for Diner 1's Monday breakfast.";
pub const C03_OATS_FINISH: &str = "On Monday at 07:55 remove Diner 1's covered oats container from the refrigerator. Stir the prepared oats and milk thoroughly, transfer the full portion to a clean bowl and serve cold by 08:00. Add no new ingredients and do not repeat Sunday's preparation. Discard any remainder after breakfast.";
pub const C03_OATS_STORAGE: &str = "Cover and refrigerate the prepared oats and milk at 5 C or below immediately after mixing, no later than Sunday 19:03. Retain the original first-handling clock Sunday 19:00 and use within 24 hours of that clock. Keep chilled until Monday 07:55; serve cold by 08:00. Discard any remainder.";

fn reference(recipe: &RecipeRevision) -> RecipeRef {
    RecipeRef {
        recipe_id: recipe.recipe_id.clone(),
        revision: recipe.revision,
        digest: recipe_digest(recipe).unwrap(),
    }
}
fn quantity(n: u64, unit: Unit, form: IngredientForm) -> Quantity {
    Quantity {
        amount: p(n, 1),
        unit,
        form,
    }
}

pub fn normal() -> Fixture {
    let mut saved = seed();
    let prototype = saved.context.recipes[0].clone();
    let evidence_id = prototype.reviews[0].evidence_id.clone();
    let ingredient_specs = [
        (
            11,
            "whole bread slices",
            2,
            Unit::Each,
            IngredientForm::Whole,
        ),
        (12, "grated cheese", 60, Unit::G, IngredientForm::Prepared),
        (13, "cherry tomatoes", 100, Unit::G, IngredientForm::Raw),
        (14, "oats", 40, Unit::G, IngredientForm::Dry),
        (15, "milk", 100, Unit::Ml, IngredientForm::Prepared),
    ];
    let mut recipes = Vec::new();
    for index in 0..3 {
        let mut recipe = prototype.clone();
        recipe.recipe_id = RecipeId(format!("rc_{:032x}", 101 + index));
        recipe.title = Title(
            [
                "Synthetic cheese toast",
                "Synthetic tomato extra",
                "Synthetic overnight oats",
            ][index]
                .into(),
        );
        let component = ComponentId(format!("co_{:032x}", 201 + index));
        let selected: &[usize] = match index {
            0 => &[0, 1],
            1 => &[2],
            _ => &[3, 4],
        };
        recipe.ingredients = BoundedVec(
            selected
                .iter()
                .map(|&n| {
                    let (id, name, amount, unit, form) = ingredient_specs[n];
                    let mut ingredient = prototype.ingredients[0].clone();
                    ingredient.ingredient_id = IngredientId(format!("ig_{id:032x}"));
                    ingredient.name = NameText(name.into());
                    ingredient.quantity = Nullable(Some(quantity(amount, unit, form)));
                    ingredient.component_id = component.clone();
                    ingredient.discrete = unit == Unit::Each;
                    ingredient.step_ids = BoundedVec(vec![1]);
                    ingredient
                })
                .collect(),
        );
        recipe.r#yield.servings = p(if index == 0 { 2 } else { 1 }, 1);
        recipe.r#yield.basis =
            Text("Explicit synthetic portion yield, not an inferred mass conversion".into());
        recipe.r#yield.components = BoundedVec(vec![ComponentYield {
            component_id: component.clone(),
            quantity: match index {
                0 => quantity(2, Unit::Each, IngredientForm::Prepared),
                1 => quantity(100, Unit::G, IngredientForm::Raw),
                _ => quantity(1, Unit::Each, IngredientForm::Prepared),
            },
        }]);
        let ids = recipe
            .ingredients
            .iter()
            .map(|i| i.ingredient_id.clone())
            .collect::<Vec<_>>();
        recipe.components[0].component_id = component.clone();
        recipe.components[0].title = recipe.title.clone();
        recipe.components[0].ingredient_ids = BoundedVec(ids.clone());
        recipe.components[0].production = Nullable(Some(if index == 0 {
            ComponentProductionSpec::Cooked {
                completion_prep_id: 2,
            }
        } else {
            ComponentProductionSpec::Unheated {
                first_handling_prep_id: 1,
            }
        }));
        recipe.extras.0.clear();
        recipe.reused_inputs.0.clear();
        recipe.capabilities.leftovers = index == 2;
        recipe.equipment = BoundedVec(match index {
            0 => vec![EquipmentCode::Oven],
            1 => vec![],
            _ => vec![EquipmentCode::Fridge],
        });
        let meal = if index == 2 {
            MealType::Breakfast
        } else {
            MealType::Dinner
        };
        recipe.meal_types = BoundedVec(vec![meal]);
        let mut preset = recipe.portion_presets[0].clone();
        preset.meal_type = meal;
        preset.preset = PortionPresetName::Regular;
        preset.age_band = AgeBand::Adult;
        preset.servings = p(1, 1);
        recipe.portion_presets = BoundedVec(vec![preset]);
        recipe.portability.cold_suitable = index != 0;
        recipe.portability.reheating_required = false;
        recipe.portability.cold_storage_required = index == 2;
        let phases: Vec<(&str, u16, u16, i32)> = match index {
            0 => vec![(BASE_PREP, 3, 3, -600), (BASE_GRILL, 0, 7, -420)],
            1 => vec![(TOMATO_EXTRA, 2, 2, -420)],
            _ => vec![(OATS_PREP, 3, 3, -50100), (OATS_FINISH, 5, 5, -300)],
        };
        recipe.steps = BoundedVec(
            phases
                .iter()
                .enumerate()
                .map(|(n, (text, active, elapsed, _))| InstructionStep {
                    step_id: n as u16 + 1,
                    text: Text((*text).into()),
                    ingredient_ids: BoundedVec(if n == 0 { ids.clone() } else { vec![] }),
                    component_ids: BoundedVec(vec![component.clone()]),
                    active_minutes: *active,
                    elapsed_minutes: *elapsed,
                    depends_on: BoundedVec(if n == 0 { vec![] } else { vec![1] }),
                })
                .collect(),
        );
        recipe.prep = BoundedVec(
            phases
                .iter()
                .enumerate()
                .map(|(n, (_, active, elapsed, offset))| PrepSpec {
                    prep_id: n as u16 + 1,
                    title: Title(format!("Synthetic normal phase {}", n + 1)),
                    step_ids: BoundedVec(vec![n as u16 + 1]),
                    depends_on: BoundedVec(if n == 0 { vec![] } else { vec![1] }),
                    active_minutes: *active,
                    elapsed_minutes: *elapsed,
                    earliest_offset_seconds: *offset,
                    latest_offset_seconds: *offset,
                    ingredient_ids: BoundedVec(if n == 0 { ids.clone() } else { vec![] }),
                    can_attach_to_prior_session: false,
                })
                .collect(),
        );
        recipe.active_minutes = phases.iter().map(|p| p.1).sum();
        recipe.elapsed_minutes = phases.iter().map(|p| p.2).sum();
        if index == 2 {
            recipe.storage[0].component_id = component;
            recipe.storage[0].max_chilled_seconds = 86400;
            recipe.storage[0].max_reheats = 0;
            recipe.storage[0].cooling_instruction = Text(OATS_STORAGE.into());
            recipe.storage[0].reheating_instruction = Text(OATS_NO_REHEAT.into());
        } else {
            recipe.components[0].storage_rule_ids.0.clear();
            recipe.storage.0.clear();
        }
        recipes.push(recipe);
    }
    let extra_id = ExtraId(format!("ex_{:032x}", 101));
    recipes[0].extras = BoundedVec(vec![ExtraSpec {
        extra_id: extra_id.clone(),
        title: recipes[1].title.clone(),
        recipe: reference(&recipes[1]),
        eligible_person_requirements: BoundedVec(vec![]),
        supported_with: BoundedVec(vec![recipes[0].components[0].component_id.clone()]),
    }]);
    let refs = recipes.iter().map(reference).collect::<Vec<_>>();
    let mut evidence = saved.evidence.clone();
    evidence.recipes.clear();
    evidence.classes.clear();
    evidence.methods.clear();
    evidence.resources.clear();
    for (index, recipe) in recipes.iter().enumerate() {
        let mut basis = saved.evidence.recipes[0].clone();
        basis.recipe = refs[index].clone();
        let classes = ComponentClassView::new(
            refs[index].clone(),
            vec![ComponentClassEvidence {
                recipe: refs[index].clone(),
                component_id: recipe.components[0].component_id.clone(),
                class: ComponentClass::General,
                evidence_ids: vec![evidence_id.clone()],
            }],
        )
        .unwrap();
        let reviewed = assess_revision(recipe, &basis, &classes)
            .unwrap_or_else(|error| panic!("normal recipe {index}: {error:?}"));
        assert!(reviewed.assessment().capabilities.shopping);
        evidence.recipes.push(basis);
        evidence.classes.push(classes);
        evidence.methods.push(MethodPhases {
            portable_cold: None,
            recipe: refs[index].clone(),
            kind: MethodKind::Production,
            prep_ids: recipe.prep.iter().map(|p| p.prep_id).collect(),
            direct_reuse_counts: vec![],
            evidence_ids: vec![evidence_id.clone()],
        });
        for prep in &recipe.prep.0 {
            evidence.resources.push(PhaseResources {
                recipe: refs[index].clone(),
                prep_id: prep.prep_id,
                equipment: if index == 0 {
                    vec![EquipmentCode::Oven]
                } else if index == 2 {
                    vec![EquipmentCode::Fridge]
                } else {
                    vec![]
                },
                evidence_ids: vec![evidence_id.clone()],
            });
        }
    }
    saved.evidence = evidence;
    saved.context.recipes = BoundedVec(recipes);
    saved.context.stamp.recipe_refs = BoundedVec(refs.clone());
    let mut second = saved.context.household.diners[0].clone();
    second.diner_id = DinerId(format!("dn_{:032x}", 2));
    second.display_name = Nullable(Some(NameText("Synthetic Diner 2".into())));
    saved.context.household.diners.0.push(second);
    saved.context.household.preferences.equipment =
        BoundedVec(vec![EquipmentCode::Fridge, EquipmentCode::Oven]);
    let week_id = saved.plan.week_id.clone();
    let dinner = SlotId {
        week_id: week_id.clone(),
        date: LocalDate("2026-10-12".into()),
        meal_type: MealType::Dinner,
    };
    let breakfast = SlotId {
        week_id: week_id.clone(),
        date: LocalDate("2026-10-13".into()),
        meal_type: MealType::Breakfast,
    };
    let person = |index: usize| PersonRef::Diner {
        diner_id: saved.context.household.diners[index].diner_id.clone(),
    };
    let serving = |index: usize, recipe: &RecipeRef, meal_type: MealType| DinerServing {
        person: person(index),
        preset_ref: PortionPresetRef {
            recipe: recipe.clone(),
            preset: PortionPresetName::Regular,
            age_band: AgeBand::Adult,
            meal_type,
        },
        servings: p(1, 1),
    };
    let mut selection = saved.plan.selections[0].clone();
    selection.slot = dinner.clone();
    selection.recipe = refs[0].clone();
    selection.servings = BoundedVec(vec![
        serving(0, &refs[0], MealType::Dinner),
        serving(1, &refs[0], MealType::Dinner),
    ]);
    selection.batch_use = BatchUse::NewBatch {
        batch_id: BatchId(format!("ba_{:032x}", 101)),
        production_servings: p(2, 1),
    };
    selection.meal_overrides.portions.0.clear();
    selection.meal_overrides.eat_by = Nullable(Some(at(12, "18:10:00")));
    selection.extras = BoundedVec(vec![ExtraSelection {
        extra_id: extra_id.clone(),
        recipe: refs[1].clone(),
        recipients: BoundedVec(vec![serving(1, &refs[1], MealType::Dinner)]),
    }]);
    let mut morning = selection.clone();
    morning.slot = breakfast.clone();
    morning.recipe = refs[2].clone();
    morning.servings = BoundedVec(vec![serving(0, &refs[2], MealType::Breakfast)]);
    morning.extras.0.clear();
    morning.batch_use = BatchUse::NewBatch {
        batch_id: BatchId(format!("ba_{:032x}", 102)),
        production_servings: p(1, 1),
    };
    morning.meal_overrides.eat_by = Nullable(Some(at(13, "08:00:00")));
    saved.plan.selections = BoundedVec(vec![selection, morning]);
    saved.plan.batch_links.0.clear();
    let week = &mut saved.context.weeks[0];
    week.range.monday = LocalDate("2026-10-12".into());
    week.meal_type_enabled = MealTypeEnabled {
        breakfast: true,
        lunch: false,
        dinner: true,
    };
    week.plan = saved.plan.clone();
    for (i, cell) in week.cells.0.iter_mut().enumerate() {
        cell.slot.date = LocalDate(format!("2026-10-{:02}", 12 + i / 3));
        cell.choice = if cell.slot == dinner || cell.slot == breakfast {
            MealLocation::AtHome
        } else {
            MealLocation::Skip
        };
        cell.attendee_ids = BoundedVec(if cell.slot == dinner {
            saved
                .context
                .household
                .diners
                .iter()
                .map(|d| d.diner_id.clone())
                .collect()
        } else if cell.slot == breakfast {
            vec![saved.context.household.diners[0].diner_id.clone()]
        } else {
            vec![]
        });
    }
    saved.context.requirements = BoundedVec(
        availability::meal_requirements(&saved.context.household, week, &[])
            .unwrap()
            .into(),
    );
    let evening_session = SessionKey {
        anchor_slot: dinner.clone(),
        role: SessionRole::Cooking,
        ordinal: 1,
    };
    let morning_session = SessionKey {
        anchor_slot: breakfast.clone(),
        role: SessionRole::Cooking,
        ordinal: 1,
    };
    let rows = [
        (
            dinner.clone(),
            None,
            refs[0].clone(),
            1,
            evening_session.clone(),
            at(12, "18:00:00"),
        ),
        (
            dinner.clone(),
            None,
            refs[0].clone(),
            2,
            evening_session.clone(),
            at(12, "18:03:00"),
        ),
        (
            dinner,
            Some(extra_id),
            refs[1].clone(),
            1,
            evening_session.clone(),
            at(12, "18:03:00"),
        ),
        (
            breakfast.clone(),
            None,
            refs[2].clone(),
            1,
            evening_session,
            at(12, "18:05:00"),
        ),
        (
            breakfast,
            None,
            refs[2].clone(),
            2,
            morning_session,
            at(13, "07:55:00"),
        ),
    ];
    saved.schedule.layout = ScheduleLayout {
        routes: vec![],
        fixed_starts: vec![],
        attachments: vec![],
    };
    for (slot, extra_id, recipe, prep_id, session, start) in rows {
        let phase = PhaseKey {
            occurrence: OccurrenceKey {
                slot,
                extra_id,
                recipe,
            },
            prep_id,
        };
        saved.schedule.layout.routes.push(PhaseRoute {
            phase: phase.clone(),
            session,
        });
        saved
            .schedule
            .layout
            .fixed_starts
            .push(PhaseStartConstraint { phase, start });
    }
    saved
        .schedule
        .layout
        .routes
        .sort_by(|a, b| phase_cmp(&a.phase, &b.phase));
    saved
        .schedule
        .layout
        .fixed_starts
        .sort_by(|a, b| phase_cmp(&a.phase, &b.phase));
    saved.schedule.occupied.query = Some(OccupancyQuery {
        horizon: TimeInterval {
            start: at(12, "18:00:00"),
            end: at(13, "08:00:00"),
        },
        excluded_subject: None,
    });
    saved.schedule.occupied.owners.clear();
    saved.schedule.occupied.digest = occupancy_snapshot_digest(&saved.schedule.occupied).unwrap();
    saved.evidence.catalogue_digest = food_evidence_digest(&saved.evidence).unwrap();
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
        .map(|(i, k)| (k, AllocationId(format!("al_{:032x}", 101 + i))))
        .collect();
    saved.identities.sessions = keys
        .sessions
        .into_iter()
        .enumerate()
        .map(|(i, k)| (k, PrepSessionId(format!("ps_{:032x}", 1 + i))))
        .collect();
    saved.identities.demand_lines = keys
        .demand_lines
        .into_iter()
        .enumerate()
        .map(|(i, k)| (k, DemandLineId(format!("dl_{:032x}", 11 + i))))
        .collect();
    let mut fixture = finish(saved);
    let h = &mut fixture.historical;
    h.approved.approved_at = Instant("2026-10-11T11:00:00Z".into());
    h.order.approved_plan = h.approved.clone();
    h.order.confirmation.confirmed_at = Instant("2026-10-11T12:00:00Z".into());
    h.order.confirmation.delivery_start = at(12, "16:00:00");
    h.order.confirmation.delivery_end = at(12, "17:00:00");
    let current = &mut fixture.current;
    current.feasibility.delivery = DeliveryInput::Confirmed {
        order: h.order.clone(),
    };
    current.feasibility.evaluated_at = Instant("2026-10-11T12:01:00Z".into());
    let graph = demand::reconstruct_use_graph(&h.derived, &h.basis).unwrap();
    let product_evidence = EvidenceId(format!("ev_{:032x}", 1100));
    current.feasibility.products.evidence = vec![EvidencePin {
        evidence_id: product_evidence.clone(),
        revision: Revision(1),
        digest: Digest("9".repeat(64)),
    }];
    for usage in graph.uses {
        let scope = ProductUseScope {
            line_id: usage.line_id.clone(),
            use_key: usage.use_key,
            product: ProductRef {
                retailer: Retailer::Tesco,
                product_id: Text(format!("synthetic-normal-{}", usage.line_id.0)),
                variant_id: Nullable(None),
            },
        };
        current
            .feasibility
            .products
            .bindings
            .push(ProductUseBinding {
                scope: scope.clone(),
                evidence_ids: vec![product_evidence.clone()],
            });
        current.feasibility.products.facts.push(ProductUseFacts {
            scope,
            use_by: Some(DateFact {
                use_by: LocalDate("2026-10-14".into()),
                evidence_ids: vec![product_evidence.clone()],
            }),
            shorter: vec![],
        });
    }
    current
        .feasibility
        .products
        .bindings
        .sort_by(|a, b| scope_cmp(&a.scope, &b.scope));
    current
        .feasibility
        .products
        .facts
        .sort_by(|a, b| scope_cmp(&a.scope, &b.scope));
    current.feasibility.products.digest =
        digest_product_uses(&current.feasibility.products).unwrap();
    current.sources[3].digest = order_source_digest(&current.sources[3], &h.order).unwrap();
    current.sources[4].digest = current.feasibility.products.digest.clone();
    current.feasibility.digest = digest_feasibility_inputs(&current.feasibility).unwrap();
    current.digest = continuation_inputs_digest(current).unwrap();
    let checked = verify_historical(h).unwrap();
    let report = assess_continuation(&checked, current).unwrap();
    assert!(
        report.blocking.is_empty(),
        "normal current blockers: {report:?}"
    );
    assert!(
        report.warnings.is_empty(),
        "normal current warnings: {report:?}"
    );
    fixture
}

/// C03 is a distinct complete synthetic approval, never an in-place change to
/// the C01 confirmed order. Its revised oats meaning is bound to revision 2.
pub fn normal_standalone() -> Fixture {
    let mut saved = normal().historical.basis;
    let oats_index = saved
        .context
        .recipes
        .iter()
        .position(|r| r.recipe_id == RecipeId(format!("rc_{:032x}", 103)))
        .unwrap();
    let old_ref = saved.context.stamp.recipe_refs[oats_index].clone();
    let selection_index = saved
        .plan
        .selections
        .iter()
        .position(|s| s.recipe == old_ref)
        .unwrap();
    let mut breakfast = saved.plan.selections[selection_index].slot.clone();
    breakfast.date = LocalDate("2026-10-12".into());
    saved.plan.selections[selection_index].slot = breakfast.clone();
    saved.plan.selections[selection_index].meal_overrides.eat_by =
        Nullable(Some(at(12, "08:00:00")));
    saved.plan.revision = Revision(3);
    saved
        .plan
        .selections
        .0
        .sort_by(|a, b| slot_cmp(&a.slot, &b.slot));
    let oats = &mut saved.context.recipes[oats_index];
    oats.revision = Revision(2);
    oats.steps[0].text = Text(C03_OATS_PREP.into());
    oats.steps[1].text = Text(C03_OATS_FINISH.into());
    oats.prep[0].earliest_offset_seconds = -46800;
    oats.prep[0].latest_offset_seconds = -46800;
    oats.storage[0].cooling_instruction = Text(C03_OATS_STORAGE.into());
    // The explicit cold-only reheat text and 86,400-second source rule remain.
    for route in &mut saved.schedule.layout.routes {
        if route.phase.occurrence.recipe == old_ref {
            route.phase.occurrence.slot = breakfast.clone();
            route.session = SessionKey {
                anchor_slot: breakfast.clone(),
                role: if route.phase.prep_id == 1 {
                    SessionRole::AdvancePrep
                } else {
                    SessionRole::Cooking
                },
                ordinal: 1,
            };
        }
    }
    for fixed in &mut saved.schedule.layout.fixed_starts {
        if fixed.phase.occurrence.recipe == old_ref {
            fixed.phase.occurrence.slot = breakfast.clone();
            fixed.start = if fixed.phase.prep_id == 1 {
                at(11, "19:00:00")
            } else {
                at(12, "07:55:00")
            };
        }
    }
    super::refresh_recipe(&mut saved, oats_index);
    saved
        .schedule
        .layout
        .routes
        .sort_by(|a, b| phase_cmp(&a.phase, &b.phase));
    saved
        .schedule
        .layout
        .fixed_starts
        .sort_by(|a, b| phase_cmp(&a.phase, &b.phase));
    assert!(saved.schedule.layout.attachments.is_empty());
    saved.schedule.occupied.query = Some(OccupancyQuery {
        horizon: TimeInterval {
            start: at(11, "19:00:00"),
            end: at(12, "18:10:00"),
        },
        excluded_subject: None,
    });
    saved.schedule.occupied.digest = occupancy_snapshot_digest(&saved.schedule.occupied).unwrap();
    let week = &mut saved.context.weeks[0];
    week.revision = Revision(3);
    week.plan = saved.plan.clone();
    for cell in &mut week.cells.0 {
        let selection = saved.plan.selections.iter().find(|s| s.slot == cell.slot);
        cell.choice = if selection.is_some() {
            MealLocation::AtHome
        } else {
            MealLocation::Skip
        };
        cell.attendee_ids = BoundedVec(
            selection
                .map(|s| {
                    s.servings
                        .iter()
                        .map(|serving| match &serving.person {
                            PersonRef::Diner { diner_id } => diner_id.clone(),
                            PersonRef::Guest { .. } => {
                                panic!("C03 only has named synthetic diners")
                            }
                        })
                        .collect()
                })
                .unwrap_or_default(),
        );
    }
    for pin in &mut saved.context.stamp.read_weeks.0 {
        if pin.week_id == saved.plan.week_id {
            pin.revision = Revision(3);
        }
    }
    for pin in &mut saved.context.stamp.write_weeks.0 {
        if pin.week_id == saved.plan.week_id {
            pin.revision = Revision(3);
        }
    }
    saved.context.requirements = BoundedVec(
        availability::meal_requirements(&saved.context.household, week, &[])
            .unwrap()
            .into(),
    );
    saved.evidence.catalogue_digest = food_evidence_digest(&saved.evidence).unwrap();
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
        .map(|(i, key)| (key, AllocationId(format!("al_{:032x}", 101 + i))))
        .collect();
    saved.identities.sessions = keys
        .sessions
        .into_iter()
        .map(|key| {
            let id = if key.role == SessionRole::AdvancePrep {
                3
            } else if key.anchor_slot.meal_type == MealType::Breakfast {
                2
            } else {
                1
            };
            (key, PrepSessionId(format!("ps_{id:032x}")))
        })
        .collect();
    saved.identities.demand_lines = keys
        .demand_lines
        .into_iter()
        .enumerate()
        .map(|(i, key)| (key, DemandLineId(format!("dl_{:032x}", 11 + i))))
        .collect();
    let mut fixture = finish(saved);
    let h = &mut fixture.historical;
    h.approved.approved_at = Instant("2026-10-10T11:00:00Z".into());
    h.order.approved_plan = h.approved.clone();
    h.order.confirmation.confirmed_at = Instant("2026-10-10T12:00:00Z".into());
    h.order.confirmation.delivery_start = at(10, "16:00:00");
    h.order.confirmation.delivery_end = at(10, "17:00:00");
    let current = &mut fixture.current;
    current.feasibility.delivery = DeliveryInput::Confirmed {
        order: h.order.clone(),
    };
    current.feasibility.evaluated_at = Instant("2026-10-10T12:01:00Z".into());
    let graph = demand::reconstruct_use_graph(&h.derived, &h.basis).unwrap();
    let product_evidence = EvidenceId(format!("ev_{:032x}", 1101));
    current.feasibility.products.evidence = vec![EvidencePin {
        evidence_id: product_evidence.clone(),
        revision: Revision(1),
        digest: Digest("8".repeat(64)),
    }];
    for usage in graph.uses {
        let scope = ProductUseScope {
            line_id: usage.line_id.clone(),
            use_key: usage.use_key,
            product: ProductRef {
                retailer: Retailer::Tesco,
                product_id: Text(format!("synthetic-normal-standalone-{}", usage.line_id.0)),
                variant_id: Nullable(None),
            },
        };
        current
            .feasibility
            .products
            .bindings
            .push(ProductUseBinding {
                scope: scope.clone(),
                evidence_ids: vec![product_evidence.clone()],
            });
        current.feasibility.products.facts.push(ProductUseFacts {
            scope,
            use_by: Some(DateFact {
                use_by: LocalDate("2026-10-14".into()),
                evidence_ids: vec![product_evidence.clone()],
            }),
            shorter: vec![],
        });
    }
    current
        .feasibility
        .products
        .bindings
        .sort_by(|a, b| scope_cmp(&a.scope, &b.scope));
    current
        .feasibility
        .products
        .facts
        .sort_by(|a, b| scope_cmp(&a.scope, &b.scope));
    current.feasibility.products.digest =
        digest_product_uses(&current.feasibility.products).unwrap();
    current.sources[3].digest = order_source_digest(&current.sources[3], &h.order).unwrap();
    current.sources[4].digest = current.feasibility.products.digest.clone();
    current.feasibility.digest = digest_feasibility_inputs(&current.feasibility).unwrap();
    current.digest = continuation_inputs_digest(current).unwrap();
    let checked = verify_historical(h).unwrap();
    let report = assess_continuation(&checked, current).unwrap();
    assert!(
        report.blocking.is_empty(),
        "C03 current blockers: {report:?}"
    );
    assert!(
        report.warnings.is_empty(),
        "C03 current warnings: {report:?}"
    );
    fixture
}
