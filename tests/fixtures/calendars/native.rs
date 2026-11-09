//! Complete SYNTHETIC A/B/C/D/E premises. No production recipe admission.
#![allow(dead_code)]
use mstrchf_contracts::*;
use mstrchf_core::{availability, food::*, prep};

#[path = "../food/batches/f11_fixture.rs"]
mod f11;
pub use f11::at;
#[path = "normal.rs"]
pub mod normal_case;
pub use normal_case::{normal, normal_standalone};

pub const SOURCE_TEXT: &str = "Combine 160 g drained beans, 120 g chopped tomatoes, 110 g chopped onion and 10 ml oil in the pan. Stir over medium heat for 10 minutes until the mixture is steaming throughout. Divide the declared 400 g cooked yield into two 200 g portions; serve one immediately.";
pub const COOLING_TEXT: &str = "Transfer the remaining 200 g portion into a clean shallow covered container, cool promptly and refrigerate within one hour at 5 C or below. Keep chilled until Tuesday service and use within 48 hours of the original Monday 18:10 production completion. This fixture assumes that cooling and storage method was independently admitted for these exact quantities and timings; the calendar does not observe it happening.";
pub const COLD_TEXT: &str = "Remove the reserved 200 g portion from the refrigerator immediately before service. Transfer it to a clean plate and serve cold within the one-minute preparation interval; discard any remainder after service. Do not cook the source batch again.";
pub const REHEAT_TEXT: &str = "Remove the reserved 200 g portion from the refrigerator. Heat in a saucepan over medium heat for five minutes, stirring continuously, until steaming throughout. Serve immediately at 13:00; discard any remainder. Reheat this portion once only.";
pub const REMIX_SOURCE_TEXT: &str = "Combine 480 g drained beans, 400 g chopped tomatoes, 160 g chopped onion and 20 ml oil in the pan. Stir over medium heat for 10 minutes until steaming throughout. From the declared 1200 g cooked yield, serve 600 g immediately and reserve two separate 300 g portions.";
pub const REMIX_COOLING_TEXT: &str = "Put each reserved 300 g portion in a separate clean shallow covered container. Cool promptly and refrigerate within one hour at 5 C or below. Keep chilled and use within 48 hours of Monday 18:00 production completion. These are assumed admitted synthetic storage premises, not observed actions.";
pub const WRAP_TEXT: &str = "Take one chilled 300 g source portion and one wrap. Heat the source portion in a saucepan for eight minutes, stirring until steaming throughout. Place it in the wrap, fold and serve during the remaining two minutes. Reheat this portion once only and discard any remainder.";
pub const POTATO_TEXT: &str = "Wash and pierce one 250 g raw potato. Microwave for five minutes, turn it, and microwave for another three minutes until the centre is soft. Meanwhile heat the separate chilled 300 g source portion in a saucepan until steaming throughout. Split the potato, add the hot source portion and serve within the ten-minute phase. Reheat this portion once only and discard any remainder.";
pub const STANDALONE_TEXT: &str = "Combine 160 g drained beans, 120 g chopped tomatoes, 110 g chopped onion and 10 ml oil in the pan. Stir over medium heat for 10 minutes until steaming throughout. Divide the declared 400 g yield into two 200 g portions. Cool promptly in clean shallow covered containers and refrigerate within one hour at 5 C or below. Keep chilled; serve the allocated 200 g portion cold on Monday at 18:10. No later meal is assigned the other portion; discard it within 48 hours of Sunday 19:10 production completion.";
pub const EXTRA_TEXT: &str = "Wash 100 g tomatoes under running water. Cut into bite-sized pieces on a clean board, put on the named diner's plate and serve alongside the main meal at 18:10. Complete the chopping during the one-minute phase; discard any uneaten extra after the meal.";
pub const PRIOR_MIX_TEXT: &str = "Mix 160 g drained beans, 120 g chopped tomatoes, 110 g chopped onion and 10 ml oil in an ovenproof dish during the five-minute active phase. Place the dish in the oven at 180 C.";
pub const PRIOR_BAKE_TEXT: &str = "Bake the dish unattended at 180 C for 20 minutes until steaming throughout. At 19:15 serve the allocated 200 g from the declared 400 g yield immediately. No storage method is admitted for this prior recipe; discard the unallocated 200 g after service.";

pub struct Fixture {
    pub historical: HistoricalApproval,
    pub current: ContinuationInputs,
}

fn p(n: u64, d: u64) -> PositiveRational {
    PositiveRational::new(n, d).unwrap()
}

const SYNTHETIC_METADATA_AT: &str = "2026-10-01T00:00:00Z";

/// The immutable F11 seed inherits December review/confirmation metadata from
/// its earlier native-v4 prototype. Calendar meals occur in October. Shift only
/// those synthetic metadata fields to 1 October; leave meal/schedule clocks,
/// dates, source wording and the original fixture bytes unchanged.
fn normalise_metadata(value: &mut serde_json::Value) -> usize {
    match value {
        serde_json::Value::Object(fields) => {
            let mut changed = 0;
            for (key, child) in fields {
                if matches!(key.as_str(), "retrieved_at" | "confirmed_at" | "at")
                    && child
                        .as_str()
                        .is_some_and(|text| text.starts_with("2026-12-"))
                {
                    *child = serde_json::Value::String(SYNTHETIC_METADATA_AT.into());
                    changed += 1;
                } else {
                    changed += normalise_metadata(child);
                }
            }
            changed
        }
        serde_json::Value::Array(items) => items.iter_mut().map(normalise_metadata).sum(),
        _ => 0,
    }
}

fn seed() -> SavedFoodBasis {
    let mut saved = f11::fixture();
    // Test-only construction from an already explicit synthetic seed, never a
    // production importer or evidence admission route.
    let mut context = serde_json::to_value(&saved.context).unwrap();
    assert!(normalise_metadata(&mut context) > 0);
    saved.context = serde_json::from_value(context).unwrap();
    // Source first, then its dependent remixes: refreshing the source replaces
    // the exact reused-input ref before each dependent recipe is hashed.
    for index in 0..saved.context.recipes.len() {
        refresh_recipe(&mut saved, index);
    }
    saved.context.weeks[0].plan = saved.plan.clone();
    saved.evidence.catalogue_digest = food_evidence_digest(&saved.evidence).unwrap();
    saved
}

/// Rebind every identity referring to a changed recipe. This is synthetic fixture
/// construction, not a production admission path: the exact revised semantics
/// and evidence are explicit assumptions in expectations.md.
pub fn refresh_recipe(saved: &mut SavedFoodBasis, index: usize) {
    let old = saved.context.stamp.recipe_refs[index].clone();
    let new = RecipeRef {
        recipe_id: saved.context.recipes[index].recipe_id.clone(),
        revision: saved.context.recipes[index].revision,
        digest: recipe_digest(&saved.context.recipes[index]).unwrap(),
    };
    for selection in &mut saved.plan.selections.0 {
        if selection.recipe == old {
            selection.recipe = new.clone();
            for serving in &mut selection.servings.0 {
                serving.preset_ref.recipe = new.clone();
            }
        }
    }
    saved.context.stamp.recipe_refs[index] = new.clone();
    for recipe in &mut saved.context.recipes.0 {
        for reused in &mut recipe.reused_inputs.0 {
            if reused.source_recipe == old {
                reused.source_recipe = new.clone();
            }
        }
    }
    for basis in &mut saved.evidence.recipes {
        if basis.recipe == old {
            basis.recipe = new.clone();
        }
    }
    for classes in &mut saved.evidence.classes {
        if classes.recipe == old {
            classes.recipe = new.clone();
            for c in &mut classes.components {
                c.recipe = new.clone();
            }
        }
    }
    for method in &mut saved.evidence.methods {
        if method.recipe == old {
            method.recipe = new.clone();
            for count in &mut method.direct_reuse_counts {
                count.recipe = new.clone();
            }
        }
    }
    for resource in &mut saved.evidence.resources {
        if resource.recipe == old {
            resource.recipe = new.clone();
        }
    }
    for route in &mut saved.schedule.layout.routes {
        if route.phase.occurrence.recipe == old {
            route.phase.occurrence.recipe = new.clone();
        }
    }
    for fixed in &mut saved.schedule.layout.fixed_starts {
        if fixed.phase.occurrence.recipe == old {
            fixed.phase.occurrence.recipe = new.clone();
        }
    }
}

pub fn direct_reuse(reheated: bool) -> Fixture {
    let mut saved = seed();
    saved.plan.selections.0.truncate(2);
    saved.plan.batch_links.0.truncate(1);
    saved.context.recipes.0.truncate(1);
    saved.context.stamp.recipe_refs.0.truncate(1);
    saved.evidence.recipes.truncate(1);
    saved.evidence.classes.truncate(1);
    saved.evidence.methods.truncate(1);
    saved.evidence.resources.truncate(1);
    saved.identities.allocations.truncate(2);
    saved.identities.sessions.truncate(2);
    saved.identities.demand_lines.truncate(4);
    saved.schedule.layout.routes.truncate(2);
    saved.schedule.layout.fixed_starts.truncate(2);
    let reference = saved.context.stamp.recipe_refs[0].clone();
    let component = saved.context.recipes[0].components[0].component_id.clone();
    let minutes = if reheated { 5 } else { 1 };
    {
        let recipe = &mut saved.context.recipes[0];
        recipe.title = Title(
            if reheated {
                "Synthetic reheated bean reuse"
            } else {
                "Synthetic cold bean reuse"
            }
            .into(),
        );
        recipe.r#yield.servings = p(2, 1);
        recipe.r#yield.components[0].quantity.amount = p(400, 1);
        for (ingredient, amount) in recipe.ingredients.0.iter_mut().zip([160, 120, 110, 10]) {
            ingredient.quantity.0.as_mut().unwrap().amount = p(amount, 1);
        }
        recipe.steps[0].text = Text(SOURCE_TEXT.into());
        recipe.steps[0].active_minutes = 10;
        recipe.steps[0].elapsed_minutes = 10;
        recipe.steps[0].depends_on.0.clear();
        let mut step = recipe.steps[0].clone();
        step.step_id = 2;
        step.text = Text(if reheated { REHEAT_TEXT } else { COLD_TEXT }.into());
        step.ingredient_ids.0.clear();
        step.depends_on = BoundedVec(vec![1]);
        step.active_minutes = minutes;
        step.elapsed_minutes = minutes;
        recipe.steps.0.push(step);
        recipe.prep[0].earliest_offset_seconds = -600;
        recipe.prep[0].latest_offset_seconds = -600;
        let mut reuse = recipe.prep[0].clone();
        reuse.prep_id = 2;
        reuse.title = Title("Synthetic current reuse".into());
        reuse.step_ids = BoundedVec(vec![2]);
        reuse.depends_on.0.clear(); // exact earlier source edge is added by core
        reuse.ingredient_ids.0.clear();
        reuse.active_minutes = minutes;
        reuse.elapsed_minutes = minutes;
        reuse.earliest_offset_seconds = -(i32::from(minutes) * 60);
        reuse.latest_offset_seconds = -(i32::from(minutes) * 60);
        recipe.prep.0.push(reuse);
        recipe.active_minutes = 10 + minutes;
        recipe.elapsed_minutes = 10 + minutes;
        recipe.storage[0].cooling_instruction = Text(COOLING_TEXT.into());
        recipe.storage[0].reheating_instruction = Text(REHEAT_TEXT.into());
        recipe.portability.cold_suitable = !reheated;
        recipe.portability.reheating_required = reheated;
    }
    let source = &mut saved.plan.selections[0];
    source.servings[0].servings = p(1, 1);
    source.meal_overrides.portions.0.clear();
    source.meal_overrides.eat_by = Nullable(Some(at(5, "18:10:00")));
    if let BatchUse::NewBatch {
        production_servings,
        ..
    } = &mut source.batch_use
    {
        *production_servings = p(2, 1);
    }
    saved.plan.selections[1].recipe = reference.clone();
    saved.plan.selections[1].servings[0].preset_ref.recipe = reference.clone();
    saved.plan.selections[1].meal_overrides.eat_by = Nullable(Some(at(6, "13:00:00")));
    saved.plan.batch_links[0].components[0].quantity.amount = p(200, 1);
    let evidence_ids = saved.evidence.methods[0].evidence_ids.clone();
    saved.evidence.methods.push(MethodPhases {
        portable_cold: None,
        recipe: reference.clone(),
        kind: MethodKind::Reuse,
        prep_ids: vec![2],
        direct_reuse_counts: vec![ComponentReheatCount {
            recipe: reference.clone(),
            component_id: component,
            planned_reheats: u8::from(reheated),
            evidence_ids: evidence_ids.clone(),
        }],
        evidence_ids: evidence_ids.clone(),
    });
    saved.evidence.resources.push(PhaseResources {
        recipe: reference.clone(),
        prep_id: 2,
        equipment: if reheated {
            vec![EquipmentCode::Hob]
        } else {
            vec![]
        },
        evidence_ids,
    });
    saved.schedule.layout.routes[1].phase.occurrence.recipe = reference.clone();
    saved.schedule.layout.routes[1].phase.prep_id = 2;
    saved.schedule.layout.fixed_starts[0].start = at(5, "18:00:00");
    saved.schedule.layout.fixed_starts[1].phase = saved.schedule.layout.routes[1].phase.clone();
    saved.schedule.layout.fixed_starts[1].start =
        at(6, if reheated { "12:55:00" } else { "12:59:00" });
    saved.schedule.occupied.query.as_mut().unwrap().horizon = TimeInterval {
        start: at(5, "18:00:00"),
        end: at(6, "13:00:00"),
    };
    // Retain exactly both selected occurrences in the live availability premise.
    let week = &mut saved.context.weeks[0];
    for cell in &mut week.cells.0 {
        if !saved.plan.selections.iter().any(|s| s.slot == cell.slot) {
            cell.choice = MealLocation::Skip;
            cell.attendee_ids.0.clear();
        }
    }
    saved.context.requirements = BoundedVec(
        availability::meal_requirements(&saved.context.household, week, &[])
            .unwrap()
            .into(),
    );
    refresh_recipe(&mut saved, 0);
    finish(saved)
}

pub fn remix() -> Fixture {
    let mut saved = seed();
    for (recipe, text) in
        saved
            .context
            .recipes
            .0
            .iter_mut()
            .zip([REMIX_SOURCE_TEXT, WRAP_TEXT, POTATO_TEXT])
    {
        recipe.steps[0].text = Text(text.into());
    }
    saved.context.recipes[0].storage[0].cooling_instruction = Text(REMIX_COOLING_TEXT.into());
    saved.context.recipes[0].storage[0].reheating_instruction = Text("Reheat each separate reserved 300 g portion once only in a saucepan, stirring until steaming throughout. Serve immediately and discard any remainder.".into());
    // No-storage remix components have a narrowed immediate-service window.
    for recipe in &mut saved.context.recipes.0 {
        recipe.prep[0].earliest_offset_seconds = -600;
        recipe.prep[0].latest_offset_seconds = -600;
    }
    saved.context.recipes[2].equipment =
        BoundedVec(vec![EquipmentCode::Hob, EquipmentCode::Microwave]);
    saved.evidence.resources[2].equipment = vec![EquipmentCode::Hob, EquipmentCode::Microwave];
    saved.context.household.preferences.equipment =
        BoundedVec(vec![EquipmentCode::Hob, EquipmentCode::Microwave]);
    saved.context.requirements = BoundedVec(
        availability::meal_requirements(&saved.context.household, &saved.context.weeks[0], &[])
            .unwrap()
            .into(),
    );
    for index in 0..3 {
        refresh_recipe(&mut saved, index);
    }
    finish(saved)
}

fn source_only_basis() -> SavedFoodBasis {
    let mut saved = direct_reuse(false).historical.basis;
    saved.plan.selections.0.truncate(1);
    saved.plan.batch_links.0.clear();
    saved.identities.allocations.truncate(1);
    saved.identities.sessions.truncate(1);
    saved.schedule.layout.routes.truncate(1);
    saved.schedule.layout.fixed_starts.truncate(1);
    saved.evidence.methods.truncate(1);
    saved.evidence.resources.truncate(1);
    let week = &mut saved.context.weeks[0];
    for cell in &mut week.cells.0 {
        if cell.slot != saved.plan.selections[0].slot {
            cell.choice = MealLocation::Skip;
            cell.attendee_ids.0.clear();
        }
    }
    saved.context.requirements = BoundedVec(
        availability::meal_requirements(&saved.context.household, week, &[])
            .unwrap()
            .into(),
    );
    saved
}

pub fn standalone() -> Fixture {
    let mut saved = source_only_basis();
    saved.context.recipes[0].steps[0].text = Text(STANDALONE_TEXT.into());
    saved.context.recipes[0].storage[0].cooling_instruction = Text("Cool both 200 g portions promptly in clean shallow covered containers; refrigerate within one hour at 5 C or below. The original clock is Sunday 19:10. Serve the allocated portion cold on Monday at 18:10; discard unallocated food within 48 hours of the original clock.".into());
    saved.context.recipes[0].prep[0].earliest_offset_seconds = -83400;
    saved.context.recipes[0].prep[0].latest_offset_seconds = -83400;
    saved.schedule.layout.routes[0].session.role = SessionRole::AdvancePrep;
    saved.identities.sessions[0].0.role = SessionRole::AdvancePrep;
    saved.schedule.layout.fixed_starts[0].start = at(4, "19:00:00");
    saved.schedule.occupied.query.as_mut().unwrap().horizon = TimeInterval {
        start: at(4, "19:00:00"),
        end: at(4, "19:10:00"),
    };
    refresh_recipe(&mut saved, 0);
    finish(saved)
}

pub fn with_extra() -> Fixture {
    let mut saved = source_only_basis();
    saved.context.recipes[0].storage[0].cooling_instruction = Text("Transfer the unallocated 200 g portion to a clean shallow covered container, cool promptly and refrigerate within one hour at 5 C or below. No later meal is assigned this portion. Discard it within 48 hours of Monday 18:10 production completion.".into());
    let mut extra = saved.context.recipes[0].clone();
    extra.recipe_id = RecipeId(format!("rc_{:032x}", 999));
    extra.title = Title("Synthetic fresh tomato extra".into());
    extra.ingredients.0.truncate(1);
    extra.ingredients[0].ingredient_id = IngredientId(format!("ig_{:032x}", 999));
    extra.ingredients[0].name = NameText("Synthetic fresh tomatoes".into());
    extra.ingredients[0].quantity = Nullable(Some(Quantity {
        amount: p(100, 1),
        unit: Unit::G,
        form: IngredientForm::Raw,
    }));
    let component = ComponentId(format!("co_{:032x}", 999));
    extra.ingredients[0].component_id = component.clone();
    extra.r#yield.servings = p(1, 1);
    extra.r#yield.components[0] = ComponentYield {
        component_id: component.clone(),
        quantity: Quantity {
            amount: p(100, 1),
            unit: Unit::G,
            form: IngredientForm::Raw,
        },
    };
    extra.components[0].component_id = component.clone();
    extra.components[0].ingredient_ids =
        BoundedVec(vec![extra.ingredients[0].ingredient_id.clone()]);
    extra.components[0].storage_rule_ids.0.clear();
    extra.components[0].production = Nullable(Some(ComponentProductionSpec::Unheated {
        first_handling_prep_id: 1,
    }));
    extra.steps.0.truncate(1);
    extra.steps[0].text = Text(EXTRA_TEXT.into());
    extra.steps[0].ingredient_ids = extra.components[0].ingredient_ids.clone();
    extra.steps[0].component_ids = BoundedVec(vec![component.clone()]);
    extra.steps[0].active_minutes = 1;
    extra.steps[0].elapsed_minutes = 1;
    extra.prep.0.truncate(1);
    extra.prep[0].ingredient_ids = extra.components[0].ingredient_ids.clone();
    extra.prep[0].active_minutes = 1;
    extra.prep[0].elapsed_minutes = 1;
    extra.prep[0].earliest_offset_seconds = -660;
    extra.prep[0].latest_offset_seconds = -660;
    extra.active_minutes = 1;
    extra.elapsed_minutes = 1;
    extra.storage.0.clear();
    extra.capabilities.leftovers = false;
    extra.equipment.0.clear();
    let reference = RecipeRef {
        recipe_id: extra.recipe_id.clone(),
        revision: extra.revision,
        digest: recipe_digest(&extra).unwrap(),
    };
    let extra_id = ExtraId(format!("ex_{:032x}", 999));
    saved.context.recipes[0].extras = BoundedVec(vec![ExtraSpec {
        extra_id: extra_id.clone(),
        title: extra.title.clone(),
        recipe: reference.clone(),
        eligible_person_requirements: BoundedVec(vec![]),
        supported_with: BoundedVec(vec![
            saved.context.recipes[0].components[0].component_id.clone(),
        ]),
    }]);
    let mut recipient = saved.plan.selections[0].servings[0].clone();
    recipient.preset_ref.recipe = reference.clone();
    saved.plan.selections[0].extras = BoundedVec(vec![ExtraSelection {
        extra_id: extra_id.clone(),
        recipe: reference.clone(),
        recipients: BoundedVec(vec![recipient]),
    }]);
    let mut basis = saved.evidence.recipes[0].clone();
    basis.recipe = reference.clone();
    let mut classes = saved.evidence.classes[0].clone();
    classes.recipe = reference.clone();
    classes.components[0].recipe = reference.clone();
    classes.components[0].component_id = component;
    saved.evidence.recipes.push(basis);
    saved.evidence.classes.push(classes);
    let mut method = saved.evidence.methods[0].clone();
    method.recipe = reference.clone();
    saved.evidence.methods.push(method);
    let mut resource = saved.evidence.resources[0].clone();
    resource.recipe = reference.clone();
    resource.equipment.clear();
    saved.evidence.resources.push(resource);
    saved.context.recipes.0.push(extra);
    saved.context.stamp.recipe_refs.0.push(reference.clone());
    let phase = PhaseKey {
        occurrence: OccurrenceKey {
            slot: saved.plan.selections[0].slot.clone(),
            extra_id: Some(extra_id),
            recipe: reference,
        },
        prep_id: 1,
    };
    saved.schedule.layout.routes.push(PhaseRoute {
        phase: phase.clone(),
        session: saved.identities.sessions[0].0.clone(),
    });
    saved
        .schedule
        .layout
        .fixed_starts
        .push(PhaseStartConstraint {
            phase,
            start: at(5, "17:59:00"),
        });
    saved.schedule.occupied.query.as_mut().unwrap().horizon = TimeInterval {
        start: at(5, "17:59:00"),
        end: at(5, "18:10:00"),
    };
    refresh_recipe(&mut saved, 0);
    saved.evidence.catalogue_digest = food_evidence_digest(&saved.evidence).unwrap();
    saved.context.weeks[0].plan = saved.plan.clone();
    saved.schedule.occupied.digest = occupancy_snapshot_digest(&saved.schedule.occupied).unwrap();
    let keys = required_identity_keys(
        &saved.plan,
        &saved.context,
        &saved.evidence,
        &saved.schedule,
    )
    .unwrap();
    saved.identities.demand_lines = keys
        .demand_lines
        .into_iter()
        .enumerate()
        .map(|(i, key)| (key, DemandLineId(format!("dl_{:032x}", 401 + i))))
        .collect();
    finish(saved)
}

pub struct LinkedFixture {
    pub subject: Fixture,
    pub prior: Fixture,
}

pub fn linked_prior() -> LinkedFixture {
    let mut saved = source_only_basis();
    let week_id = WeekId(format!("wk_{:032x}", 990));
    let slot = SlotId {
        week_id: week_id.clone(),
        date: LocalDate("2026-10-04".into()),
        meal_type: MealType::Dinner,
    };
    saved.plan.week_id = week_id.clone();
    saved.plan.selections[0].slot = slot.clone();
    saved.plan.selections[0].meal_overrides.eat_by = Nullable(Some(at(4, "19:15:00")));
    let batch_id = BatchId(format!("ba_{:032x}", 990));
    if let BatchUse::NewBatch { batch_id: id, .. } = &mut saved.plan.selections[0].batch_use {
        *id = batch_id.clone();
    }
    saved.identities.allocations[0].0.batch_id = batch_id;
    saved.identities.allocations[0].0.target_slot = slot.clone();
    saved.identities.allocations[0].1 = AllocationId(format!("al_{:032x}", 990));
    saved.identities.sessions[0].0.anchor_slot = slot.clone();
    saved.identities.sessions[0].1 = PrepSessionId(format!("ps_{:032x}", 990));
    for (i, (_, line_id)) in saved.identities.demand_lines.iter_mut().enumerate() {
        *line_id = DemandLineId(format!("dl_{:032x}", 990 + i));
    }
    for pin in &mut saved.context.stamp.read_weeks.0 {
        pin.week_id = week_id.clone();
    }
    for pin in &mut saved.context.stamp.write_weeks.0 {
        pin.week_id = week_id.clone();
    }
    let week = &mut saved.context.weeks[0];
    week.range.week_id = week_id.clone();
    week.range.monday = LocalDate("2026-09-28".into());
    week.plan = saved.plan.clone();
    for (index, cell) in week.cells.0.iter_mut().enumerate() {
        cell.slot.week_id = week_id.clone();
        let day = index / 3;
        cell.slot.date = LocalDate(if day < 3 {
            format!("2026-09-{}", 28 + day)
        } else {
            format!("2026-10-{:02}", day - 2)
        });
        cell.choice = if cell.slot == slot {
            MealLocation::AtHome
        } else {
            MealLocation::Skip
        };
        cell.attendee_ids = BoundedVec(if cell.slot == slot {
            vec![saved.context.household.diners[0].diner_id.clone()]
        } else {
            vec![]
        });
    }
    saved.context.household.preferences.equipment =
        BoundedVec(vec![EquipmentCode::Hob, EquipmentCode::Oven]);
    saved.context.requirements = BoundedVec(
        availability::meal_requirements(&saved.context.household, week, &[])
            .unwrap()
            .into(),
    );
    let recipe = &mut saved.context.recipes[0];
    recipe.recipe_id = RecipeId(format!("rc_{:032x}", 990));
    recipe.title = Title("Synthetic prior oven bean dish".into());
    recipe.equipment = BoundedVec(vec![EquipmentCode::Oven]);
    recipe.steps[0].text = Text(PRIOR_MIX_TEXT.into());
    recipe.steps[0].active_minutes = 5;
    recipe.steps[0].elapsed_minutes = 5;
    recipe.steps[1].text = Text(PRIOR_BAKE_TEXT.into());
    recipe.steps[1].active_minutes = 0;
    recipe.steps[1].elapsed_minutes = 20;
    recipe.prep[0].active_minutes = 5;
    recipe.prep[0].elapsed_minutes = 5;
    recipe.prep[0].earliest_offset_seconds = -1500;
    recipe.prep[0].latest_offset_seconds = -1500;
    recipe.prep[1].title = Title("Synthetic unattended oven phase".into());
    recipe.prep[1].active_minutes = 0;
    recipe.prep[1].elapsed_minutes = 20;
    recipe.prep[1].earliest_offset_seconds = -1200;
    recipe.prep[1].latest_offset_seconds = -1200;
    recipe.prep[1].depends_on = BoundedVec(vec![1]);
    recipe.active_minutes = 5;
    recipe.elapsed_minutes = 25;
    recipe.components[0].production = Nullable(Some(ComponentProductionSpec::Cooked {
        completion_prep_id: 2,
    }));
    recipe.components[0].storage_rule_ids.0.clear();
    recipe.storage.0.clear();
    recipe.capabilities.leftovers = false;
    saved.evidence.methods[0].prep_ids = vec![1, 2];
    saved.evidence.resources[0].equipment.clear();
    let mut resource = saved.evidence.resources[0].clone();
    resource.prep_id = 2;
    resource.equipment = vec![EquipmentCode::Oven];
    saved.evidence.resources.push(resource);
    saved.schedule.layout.routes[0].phase.occurrence.slot = slot.clone();
    saved.schedule.layout.routes[0].session.anchor_slot = slot;
    let mut route = saved.schedule.layout.routes[0].clone();
    route.phase.prep_id = 2;
    saved.schedule.layout.routes.push(route);
    saved.schedule.layout.fixed_starts[0].phase = saved.schedule.layout.routes[0].phase.clone();
    saved.schedule.layout.fixed_starts[0].start = at(4, "18:50:00");
    saved
        .schedule
        .layout
        .fixed_starts
        .push(PhaseStartConstraint {
            phase: saved.schedule.layout.routes[1].phase.clone(),
            start: at(4, "18:55:00"),
        });
    saved.schedule.occupied.query.as_mut().unwrap().horizon = TimeInterval {
        start: at(4, "18:50:00"),
        end: at(4, "19:15:00"),
    };
    refresh_recipe(&mut saved, 0);
    let prior = finish(saved);
    let basis = &prior.historical.basis;
    let pin = PlanOwnerPin {
        week: WeekRevision {
            week_id: basis.plan.week_id.clone(),
            revision: basis.plan.revision,
        },
        derived_digest: prior.historical.derived.digest.clone(),
        basis_digest: saved_basis_digest(basis).unwrap(),
    };
    let bindings = prep::validate_schedule(
        &basis.plan,
        &basis.context,
        &basis.evidence,
        &basis.identities,
        &basis.schedule,
        &prior.historical.derived.prep_sessions,
    )
    .unwrap();
    let mut owner = OccupiedOwner {
        pin: pin.clone(),
        sessions: prior.historical.derived.prep_sessions.0.clone(),
        task_resources: bindings
            .iter()
            .map(|b| {
                let resource = basis
                    .evidence
                    .resources
                    .iter()
                    .find(|r| r.recipe == b.phase.occurrence.recipe && r.prep_id == b.phase.prep_id)
                    .unwrap();
                OccupiedTaskResources {
                    task: b.task.clone(),
                    equipment: resource.equipment.clone(),
                    evidence_ids: resource.evidence_ids.clone(),
                }
            })
            .collect(),
        projection_digest: PayloadDigest("0".repeat(64)),
    };
    owner.projection_digest = occupied_owner_digest(&owner).unwrap();
    let mut child = standalone().historical.basis;
    child.context.recipes[0].prep[0].can_attach_to_prior_session = true;
    refresh_recipe(&mut child, 0);
    child.schedule.layout.attachments = vec![AttachmentChoice {
        session: child.identities.sessions[0].0.clone(),
        target_owner: pin,
        target_session: prior.historical.derived.prep_sessions[0].session_id.clone(),
    }];
    child.schedule.occupied.owners = vec![owner];
    let subject = finish(child);
    LinkedFixture { subject, prior }
}

pub fn finish(mut saved: SavedFoodBasis) -> Fixture {
    saved.context.weeks[0].plan = saved.plan.clone();
    saved.evidence.catalogue_digest = food_evidence_digest(&saved.evidence).unwrap();
    saved.identities.digest = food_identities_digest(&saved.identities).unwrap();
    saved.schedule.occupied.digest = occupancy_snapshot_digest(&saved.schedule.occupied).unwrap();
    let derived = derive_plan(
        &saved.plan,
        &saved.context,
        &FoodInputs {
            evidence: &saved.evidence,
            identities: &saved.identities,
            schedule: &saved.schedule,
            feasibility: &f11::feasibility(),
        },
    )
    .expect("full synthetic A/B/C/D derivation")
    .plan;
    prep::validate_schedule(
        &saved.plan,
        &saved.context,
        &saved.evidence,
        &saved.identities,
        &saved.schedule,
        &derived.prep_sessions,
    )
    .expect("complete phase/task schedule");
    let owner_identity = saved.plan.week_id.0.strip_prefix("wk_").unwrap();
    let approved = ApprovedPlan {
        approval_id: ApprovalId(format!("au_{owner_identity}")),
        week_id: derived.week_id.clone(),
        plan_revision: derived.plan_revision,
        derived_digest: derived.digest.clone(),
        context_stamp: derived.context_stamp.clone(),
        approved_at: at(3, "09:00:00").instant,
    };
    let order = ConfirmedOrder {
        order_id: OrderId(format!("or_{owner_identity}")),
        confirmation: OrderConfirmation {
            week_id: approved.week_id.clone(),
            approval_id: approved.approval_id.clone(),
            delivery_start: at(3, "10:00:00"),
            delivery_end: at(3, "11:00:00"),
            confirmed_at: at(3, "09:30:00").instant,
            source: OrderConfirmationSource::UserConfirmation {},
        },
        approved_plan: approved.clone(),
        ordered_recipe_refs: derived.recipe_refs.clone(),
    };
    let zero = || PayloadDigest("0".repeat(64));
    let mut current = ContinuationInputs {
        revision: Revision(1),
        digest: zero(),
        sources: vec![],
        requirements: LiveRequirements {
            revision: Revision(1),
            digest: zero(),
            meals: saved
                .context
                .requirements
                .iter()
                .filter(|r| saved.plan.selections.iter().any(|s| s.slot == r.slot))
                .cloned()
                .collect(),
            unconfirmed: vec![],
        },
        evidence: RelevantEvidenceSnapshot {
            revision: Revision(1),
            digest: zero(),
            recipes: saved
                .evidence
                .recipes
                .iter()
                .map(|basis| LiveRecipeState::Admitted {
                    basis: basis.clone(),
                    classes: saved
                        .evidence
                        .classes
                        .iter()
                        .find(|c| c.recipe == basis.recipe)
                        .unwrap()
                        .clone(),
                    methods: saved
                        .evidence
                        .methods
                        .iter()
                        .filter(|m| m.recipe == basis.recipe)
                        .cloned()
                        .collect(),
                    resources: saved
                        .evidence
                        .resources
                        .iter()
                        .filter(|r| r.recipe == basis.recipe)
                        .cloned()
                        .collect(),
                })
                .collect(),
        },
        policy: LiveFoodPolicy {
            version: Revision(1),
            digest: zero(),
            general_seconds: 172800,
            rice_seconds: 86400,
            maximum_reheats: 1,
        },
        occupied: saved.schedule.occupied.clone(),
        feasibility: f11::feasibility(),
    };
    current.occupied.query.as_mut().unwrap().excluded_subject = Some(PlanOwnerPin {
        week: WeekRevision {
            week_id: saved.plan.week_id.clone(),
            revision: saved.plan.revision,
        },
        derived_digest: derived.digest.clone(),
        basis_digest: saved_basis_digest(&saved).unwrap(),
    });
    current.feasibility.delivery = DeliveryInput::Confirmed {
        order: order.clone(),
    };
    current.feasibility.evaluated_at = at(3, "12:00:00").instant;
    current.feasibility.digest = digest_feasibility_inputs(&current.feasibility).unwrap();
    current.requirements.digest = live_requirements_digest(&current.requirements).unwrap();
    current.evidence.digest = relevant_evidence_digest(&current.evidence).unwrap();
    current.policy.digest = live_food_policy_digest(&current.policy).unwrap();
    current.occupied.digest = occupancy_snapshot_digest(&current.occupied).unwrap();
    for (source, revision, digest) in [
        (LiveSource::Essentials, Revision(1), zero()),
        (
            LiveSource::FoodPolicy,
            current.policy.version,
            current.policy.digest.clone(),
        ),
        (
            LiveSource::Occupancy,
            current.occupied.occupancy_epoch,
            current.occupied.digest.clone(),
        ),
        (LiveSource::Order, Revision(1), zero()),
        (
            LiveSource::ProductEvidence,
            current.feasibility.products.revision,
            current.feasibility.products.digest.clone(),
        ),
        (
            LiveSource::RecipeEvidence,
            current.evidence.revision,
            current.evidence.digest.clone(),
        ),
        (
            LiveSource::Requirements,
            current.requirements.revision,
            current.requirements.digest.clone(),
        ),
    ] {
        current.sources.push(LiveSourcePin {
            source,
            revision,
            digest,
            valid_until: None,
        });
    }
    current.sources[0].digest =
        essentials_source_digest(&current.sources[0], &current.feasibility.confirmations).unwrap();
    current.sources[3].digest = order_source_digest(&current.sources[3], &order).unwrap();
    current.digest = continuation_inputs_digest(&current).unwrap();
    let historical = HistoricalApproval {
        basis: saved,
        derived,
        approved,
        order,
    };
    let checked = verify_historical(&historical).expect("full historical integrity");
    let report = assess_continuation(&checked, &current).expect("full current E continuation");
    assert!(
        report.blocking.is_empty(),
        "synthetic premise must be nonblocking: {report:?}"
    );
    Fixture {
        historical,
        current,
    }
}

#[test]
fn full_direct_reuse_premises_are_accepted_before_calendar() {
    for reheated in [false, true] {
        let fixture = direct_reuse(reheated);
        let h = &fixture.historical;
        assert_eq!(fixture.current.requirements.meals.len(), 2);
        assert_eq!(fixture.current.sources.len(), 7);
        assert_eq!(h.derived.batches.len(), 1);
        assert_eq!(h.derived.allocations.len(), 2);
        assert_eq!(h.derived.prep_sessions.len(), 2);
        assert_eq!(h.derived.demand.lines.len(), 4);
        assert_eq!(
            h.derived.batches[0].components[0].original_production,
            ProductionMoment::Cooked {
                completed_at: at(5, "18:10:00")
            }
        );
        for allocation in &h.derived.allocations.0 {
            assert_eq!(allocation.quantity.amount, p(200, 1));
        }
        assert_eq!(h.derived.allocations[1].planned_reheats, u8::from(reheated));
        let source = &h.derived.prep_sessions[0].tasks[0];
        let target = &h.derived.prep_sessions[1].tasks[0];
        assert_eq!(source.elapsed.start, at(5, "18:00:00"));
        assert_eq!(source.elapsed.end, at(5, "18:10:00"));
        assert_eq!(
            target.elapsed.start,
            at(6, if reheated { "12:55:00" } else { "12:59:00" })
        );
        assert_eq!(target.elapsed.end, at(6, "13:00:00"));
        assert_eq!(target.depends_on.len(), 1);
        assert!(h.derived.prep_sessions[1].ingredient_ids.is_empty());
        // The use graph intentionally carries original ingredient lineage into
        // reuse for product-date checks. It is not duplicate shopping demand.
        for (line, expected) in h.derived.demand.lines.iter().zip([160, 120, 110, 10]) {
            assert_eq!(line.quantity.amount, p(expected, 1));
        }
    }
}

#[test]
fn full_remix_premise_is_accepted_before_calendar() {
    let fixture = remix();
    let h = &fixture.historical;
    assert_eq!(fixture.current.requirements.meals.len(), 3);
    assert_eq!(h.derived.batches.len(), 1);
    assert_eq!(h.derived.allocations.len(), 3);
    assert_eq!(h.derived.prep_sessions.len(), 3);
    assert_eq!(h.derived.demand.lines.len(), 6);
    assert_eq!(h.derived.allocations[0].quantity.amount, p(600, 1));
    assert_eq!(h.derived.allocations[1].quantity.amount, p(300, 1));
    assert_eq!(h.derived.allocations[2].quantity.amount, p(300, 1));
}

#[test]
fn full_standalone_premise_is_accepted_before_calendar() {
    let fixture = standalone();
    assert_eq!(fixture.historical.derived.prep_sessions.len(), 1);
    assert_eq!(
        fixture.historical.derived.prep_sessions[0].start,
        at(4, "19:00:00")
    );
    assert_eq!(
        fixture.historical.basis.identities.sessions[0].0.role,
        SessionRole::AdvancePrep
    );
}

#[test]
fn full_extra_premise_is_accepted_before_calendar() {
    let fixture = with_extra();
    let h = &fixture.historical;
    assert_eq!(h.derived.prep_sessions.len(), 1);
    assert_eq!(h.derived.prep_sessions[0].tasks.len(), 2);
    assert_eq!(h.derived.demand.lines.len(), 5);
    assert_eq!(h.basis.plan.selections[0].extras[0].recipients.len(), 1);
    assert_eq!(h.derived.demand.lines[4].quantity.amount, p(100, 1));
}

#[test]
fn full_linked_prior_histories_are_accepted_before_calendar() {
    let f = linked_prior();
    assert_ne!(
        f.subject.historical.order.order_id,
        f.prior.historical.order.order_id
    );
    assert_ne!(
        f.subject.historical.approved.approval_id,
        f.prior.historical.approved.approval_id
    );
    assert_eq!(f.subject.historical.derived.prep_sessions.len(), 1);
    assert_eq!(f.prior.historical.derived.prep_sessions.len(), 1);
    assert_eq!(f.prior.historical.derived.prep_sessions[0].tasks.len(), 2);
    assert_eq!(
        f.subject.historical.derived.prep_sessions[0].attached_to.0,
        Some(
            f.prior.historical.derived.prep_sessions[0]
                .session_id
                .clone()
        )
    );
    assert_eq!(
        f.subject.historical.derived.prep_sessions[0].start,
        at(4, "19:00:00")
    );
    assert_eq!(
        f.prior.historical.derived.prep_sessions[0].start,
        at(4, "18:50:00")
    );
    assert_eq!(
        f.prior.historical.derived.prep_sessions[0].end,
        at(4, "19:15:00")
    );
}

#[test]
fn full_normal_c01_history_is_accepted_before_calendar() {
    let f = normal();
    let h = &f.historical;
    assert_eq!(h.order.confirmation.delivery_start, at(12, "16:00:00"));
    assert_eq!(h.order.confirmation.delivery_end, at(12, "17:00:00"));
    assert_eq!(h.basis.context.household.diners.len(), 2);
    assert_eq!(h.derived.recipe_refs.len(), 3);
    assert_eq!(h.derived.prep_sessions.len(), 2);
    assert_eq!(h.derived.prep_sessions[0].tasks.len(), 4);
    assert_eq!(h.derived.prep_sessions[1].tasks.len(), 1);
    assert_eq!(h.derived.prep_sessions[0].start, at(12, "18:00:00"));
    assert_eq!(h.derived.prep_sessions[0].end, at(12, "18:10:00"));
    assert_eq!(h.derived.prep_sessions[1].start, at(13, "07:55:00"));
    assert_eq!(h.derived.prep_sessions[1].end, at(13, "08:00:00"));
    assert_eq!(h.derived.prep_sessions[0].linked_slots.len(), 2);
    assert!(
        h.derived
            .prep_sessions
            .iter()
            .all(|s| s.attached_to.0.is_none())
    );
    assert_eq!(h.derived.demand.lines.len(), 5);
    for (line, expected) in h.derived.demand.lines.iter().zip([2, 60, 100, 40, 100]) {
        assert_eq!(line.quantity.amount, p(expected, 1));
    }
    assert_eq!(h.derived.batches.len(), 2);
    assert_eq!(h.derived.allocations.len(), 3);
    assert!(h.basis.plan.batch_links.is_empty());
    assert_eq!(
        h.derived.batches[1].components[0].original_production,
        ProductionMoment::Unheated {
            preparation_started_at: at(12, "18:05:00")
        }
    );
    assert_eq!(
        h.basis.plan.selections[0].extras[0].recipients[0].person,
        PersonRef::Diner {
            diner_id: h.basis.context.household.diners[1].diner_id.clone()
        }
    );
    assert_eq!(f.current.sources.len(), 7);
    assert_eq!(f.current.requirements.meals.len(), 2);
}

#[test]
fn full_normal_standalone_c03_history_is_accepted_before_calendar() {
    let f = normal_standalone();
    let h = &f.historical;
    assert_eq!(h.order.confirmation.delivery_start, at(10, "16:00:00"));
    assert_eq!(h.order.confirmation.delivery_end, at(10, "17:00:00"));
    assert_eq!(h.derived.prep_sessions.len(), 3);
    assert_eq!(h.derived.demand.lines.len(), 5);
    assert_eq!(h.derived.allocations.len(), 3);
    assert_eq!(h.derived.batches.len(), 2);
    let prep_id = PrepSessionId(format!("ps_{:032x}", 3));
    let breakfast_id = PrepSessionId(format!("ps_{:032x}", 2));
    let dinner_id = PrepSessionId(format!("ps_{:032x}", 1));
    let prep = h
        .derived
        .prep_sessions
        .iter()
        .find(|s| s.session_id == prep_id)
        .unwrap();
    let breakfast = h
        .derived
        .prep_sessions
        .iter()
        .find(|s| s.session_id == breakfast_id)
        .unwrap();
    let dinner = h
        .derived
        .prep_sessions
        .iter()
        .find(|s| s.session_id == dinner_id)
        .unwrap();
    assert_eq!(
        (prep.start.clone(), prep.end.clone()),
        (at(11, "19:00:00"), at(11, "19:03:00"))
    );
    assert_eq!(
        (breakfast.start.clone(), breakfast.end.clone()),
        (at(12, "07:55:00"), at(12, "08:00:00"))
    );
    assert_eq!(
        (dinner.start.clone(), dinner.end.clone()),
        (at(12, "18:00:00"), at(12, "18:10:00"))
    );
    assert_eq!(prep.tasks.len(), 1);
    assert_eq!(breakfast.tasks.len(), 1);
    assert_eq!(dinner.tasks.len(), 3);
    assert!(
        h.derived
            .prep_sessions
            .iter()
            .all(|s| s.attached_to.0.is_none())
    );
    assert_eq!(dinner.linked_slots.len(), 1);
    assert_eq!(
        h.basis
            .identities
            .sessions
            .iter()
            .find(|(_, id)| *id == prep_id)
            .unwrap()
            .0
            .role,
        SessionRole::AdvancePrep
    );
    assert_eq!(breakfast.tasks[0].depends_on[0].session_id, prep_id);
    let oats = h
        .derived
        .batches
        .iter()
        .find(|b| b.recipe.recipe_id == RecipeId(format!("rc_{:032x}", 103)))
        .unwrap();
    assert_eq!(
        oats.components[0].original_production,
        ProductionMoment::Unheated {
            preparation_started_at: at(11, "19:00:00")
        }
    );
    assert_eq!(f.current.sources.len(), 7);
    assert_eq!(f.current.requirements.meals.len(), 2);
}
