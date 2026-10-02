//! Genuine individually admitted owner histories for calendar aggregate limits.
//! Literal independent counts were frozen in aggregate-expectations.md first.
#![allow(dead_code)]
use super::native::{self, Fixture};
use mstrchf_contracts::*;
use mstrchf_core::{availability, food::*};
use std::sync::OnceLock;

fn p(n: u64) -> PositiveRational {
    PositiveRational::new(n, 1).unwrap()
}

/// A complete 16-stage artificial recipe. Every stage has actual source text,
/// dependencies, timing, evidence and resources; no fabricated derived tasks.
fn owner(number: usize, split_sessions: bool) -> Fixture {
    let mut saved = native::direct_reuse(false).historical.basis;
    saved.plan.selections.0.truncate(1);
    saved.plan.batch_links.0.clear();
    saved.evidence.methods.truncate(1);
    saved.evidence.resources.truncate(1);
    let ingredient_ids = saved.context.recipes[0]
        .ingredients
        .iter()
        .map(|i| i.ingredient_id.clone())
        .collect::<Vec<_>>();
    {
        let recipe = &mut saved.context.recipes[0];
        recipe.title = Title("Synthetic 16-stage bean simmer".into());
        recipe.steps.0.truncate(1);
        recipe.prep.0.truncate(1);
        let base_step = recipe.steps[0].clone();
        let base_prep = recipe.prep[0].clone();
        recipe.steps.0.clear();
        recipe.prep.0.clear();
        for id in 1..=16 {
            let mut step = base_step.clone();
            step.step_id = id;
            step.text = Text(match id {
                1 => "Combine 160 g drained beans, 120 g chopped tomatoes, 110 g chopped onion and 10 ml oil in a covered pan. Begin heating over low heat and stir continuously for this first minute.".into(),
                16 => "Continue stirring over low heat for the final minute until steaming throughout. This artificial source declares 400 g cooked yield after 16 minutes. Divide into two 200 g portions, serve one immediately and discard the other. No leftover is retained.".into(),
                _ => format!("During minute {id} of 16, keep the covered bean mixture over low heat and stir continuously for one minute. Retain all of this same mixture for the next phase; add no further ingredients."),
            });
            step.active_minutes = 1;
            step.elapsed_minutes = 1;
            step.depends_on = BoundedVec(if id == 1 { vec![] } else { vec![id - 1] });
            step.ingredient_ids = BoundedVec(if id == 1 {
                ingredient_ids.clone()
            } else {
                vec![]
            });
            // Exact phase dependencies retain the in-progress mixture. Declare
            // its production in the final step, not a new component use at
            // every minute; the ingredients are introduced explicitly at step 1.
            if id != 16 {
                step.component_ids.0.clear();
            }
            let mut phase = base_prep.clone();
            phase.prep_id = id;
            phase.title = Title(format!("Synthetic simmer minute {id}"));
            phase.step_ids = BoundedVec(vec![id]);
            phase.depends_on = BoundedVec(if id == 1 { vec![] } else { vec![id - 1] });
            phase.active_minutes = 1;
            phase.elapsed_minutes = 1;
            phase.earliest_offset_seconds = -(17 - i32::from(id)) * 60;
            phase.latest_offset_seconds = phase.earliest_offset_seconds;
            phase.ingredient_ids = step.ingredient_ids.clone();
            recipe.steps.0.push(step);
            recipe.prep.0.push(phase);
        }
        recipe.components[0].production = Nullable(Some(ComponentProductionSpec::Cooked {
            completion_prep_id: 16,
        }));
        recipe.active_minutes = 16;
        recipe.elapsed_minutes = 16;
        recipe.meal_types =
            BoundedVec(vec![MealType::Breakfast, MealType::Lunch, MealType::Dinner]);
        let mut breakfast = recipe.portion_presets[0].clone();
        breakfast.meal_type = MealType::Breakfast;
        recipe.portion_presets.0.push(breakfast);
        recipe.portability.cold_suitable = false;
        recipe.portability.cold_storage_required = false;
        recipe.components[0].storage_rule_ids.0.clear();
        recipe.storage.0.clear();
        recipe.capabilities.leftovers = false;
    }
    let reference = RecipeRef {
        recipe_id: saved.context.recipes[0].recipe_id.clone(),
        revision: saved.context.recipes[0].revision,
        digest: recipe_digest(&saved.context.recipes[0]).unwrap(),
    };
    saved.context.stamp.recipe_refs = BoundedVec(vec![reference.clone()]);
    saved.evidence.recipes[0].recipe = reference.clone();
    saved.evidence.classes[0].recipe = reference.clone();
    saved.evidence.classes[0].components[0].recipe = reference.clone();
    saved.evidence.methods[0].recipe = reference.clone();
    saved.evidence.methods[0].prep_ids = (1..=16).collect();
    let mut resource = saved.evidence.resources[0].clone();
    resource.recipe = reference.clone();
    saved.evidence.resources = (1..=16)
        .map(|prep_id| PhaseResources {
            prep_id,
            ..resource.clone()
        })
        .collect();
    saved.evidence.catalogue_digest = food_evidence_digest(&saved.evidence).unwrap();

    let week_id = WeekId(format!("wk_{:032x}", 10000 + number));
    saved.plan.week_id = week_id.clone();
    for stamp in saved
        .context
        .stamp
        .read_weeks
        .0
        .iter_mut()
        .chain(saved.context.stamp.write_weeks.0.iter_mut())
    {
        stamp.week_id = week_id.clone();
    }
    let prototype = saved.plan.selections[0].clone();
    saved.plan.selections.0.clear();
    saved.schedule.layout.routes.clear();
    saved.schedule.layout.fixed_starts.clear();
    let diner = saved.context.household.diners[0].diner_id.clone();
    let week = &mut saved.context.weeks[0];
    week.range.week_id = week_id.clone();
    week.meal_type_enabled = MealTypeEnabled {
        breakfast: true,
        lunch: true,
        dinner: true,
    };
    let mut first_start = None;
    let mut last_end = None;
    for cell in &mut week.cells.0 {
        cell.slot.week_id = week_id.clone();
        let selected = !split_sessions || saved.plan.selections.len() < 16;
        cell.choice = if selected {
            MealLocation::AtHome
        } else {
            MealLocation::Skip
        };
        cell.attendee_ids = BoundedVec(if selected {
            vec![diner.clone()]
        } else {
            vec![]
        });
        if !selected {
            continue;
        }
        let index = saved.plan.selections.len();
        let mut selection = prototype.clone();
        selection.slot = cell.slot.clone();
        selection.recipe = reference.clone();
        selection.servings[0].preset_ref.recipe = reference.clone();
        selection.servings[0].preset_ref.meal_type = cell.slot.meal_type;
        selection.batch_use = BatchUse::NewBatch {
            batch_id: BatchId(format!("ba_{:032x}", (10000 + number) * 100 + index)),
            production_servings: p(2),
        };
        let service_hour = match cell.slot.meal_type {
            MealType::Breakfast => 8,
            MealType::Lunch => 13,
            MealType::Dinner => 19,
        };
        let day: u8 = cell.slot.date.0[8..10].parse().unwrap();
        let end = native::at(day, &format!("{service_hour:02}:00:00"));
        selection.meal_overrides.eat_by = Nullable(Some(end.clone()));
        for prep_id in 1..=16 {
            let phase = PhaseKey {
                occurrence: OccurrenceKey {
                    slot: cell.slot.clone(),
                    extra_id: None,
                    recipe: reference.clone(),
                },
                prep_id,
            };
            let session = SessionKey {
                anchor_slot: cell.slot.clone(),
                role: SessionRole::Cooking,
                ordinal: if split_sessions { prep_id } else { 1 },
            };
            let minute = service_hour * 60 - 17 + i32::from(prep_id);
            let start = native::at(day, &format!("{:02}:{:02}:00", minute / 60, minute % 60));
            first_start.get_or_insert_with(|| start.clone());
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
        last_end = Some(end);
        saved.plan.selections.0.push(selection);
    }
    week.plan = saved.plan.clone();
    saved.context.requirements = BoundedVec(
        availability::meal_requirements(&saved.context.household, week, &[])
            .unwrap()
            .into(),
    );
    saved.schedule.occupied.query = Some(OccupancyQuery {
        horizon: TimeInterval {
            start: first_start.unwrap(),
            end: last_end.unwrap(),
        },
        excluded_subject: None,
    });
    saved.schedule.occupied.digest = occupancy_snapshot_digest(&saved.schedule.occupied).unwrap();
    let keys = required_identity_keys(
        &saved.plan,
        &saved.context,
        &saved.evidence,
        &saved.schedule,
    )
    .expect("complete aggregate identity premise");
    let base = (10000 + number) * 10000;
    saved.identities.sessions = keys
        .sessions
        .into_iter()
        .enumerate()
        .map(|(i, k)| (k, PrepSessionId(format!("ps_{:032x}", base + i))))
        .collect();
    saved.identities.allocations = keys
        .allocations
        .into_iter()
        .enumerate()
        .map(|(i, k)| (k, AllocationId(format!("al_{:032x}", base + i))))
        .collect();
    saved.identities.demand_lines = keys
        .demand_lines
        .into_iter()
        .enumerate()
        .map(|(i, k)| (k, DemandLineId(format!("dl_{:032x}", base + i))))
        .collect();
    // finish derives approval/order identity from this owner's unique week ID
    // before performing all actual food-domain checks; no second mutation or
    // duplicate continuation run is necessary.
    let fixture = native::finish(saved);
    #[cfg(test)]
    eprintln!(
        "Aggregate owner {number}: {} complete sessions, {} tasks; full food checks accepted",
        fixture.historical.derived.prep_sessions.len(),
        fixture
            .historical
            .derived
            .prep_sessions
            .iter()
            .map(|s| s.tasks.len())
            .sum::<usize>()
    );
    fixture
}

pub fn session_limit_owners() -> &'static [Fixture] {
    static OWNERS: OnceLock<Vec<Fixture>> = OnceLock::new();
    OWNERS.get_or_init(|| (0..18).map(|n| owner(n, true)).collect())
}
pub fn task_limit_owners() -> &'static [Fixture] {
    static OWNERS: OnceLock<Vec<Fixture>> = OnceLock::new();
    OWNERS.get_or_init(|| (100..161).map(|n| owner(n, false)).collect())
}

#[test]
fn aggregate_session_owners_are_individually_admitted() {
    let owners = session_limit_owners();
    assert_eq!(owners.len(), 18);
    for owner in owners {
        assert_eq!(owner.historical.basis.plan.selections.len(), 16);
        assert_eq!(owner.historical.derived.prep_sessions.len(), 256);
        assert!(
            owner
                .historical
                .derived
                .prep_sessions
                .iter()
                .all(|s| s.tasks.len() == 1)
        );
    }
    assert_eq!(
        owners
            .iter()
            .map(|o| o.historical.derived.prep_sessions.len())
            .sum::<usize>(),
        4608
    );
}

#[test]
fn aggregate_task_owners_are_individually_admitted() {
    let owners = task_limit_owners();
    assert_eq!(owners.len(), 61);
    for owner in owners {
        assert_eq!(owner.historical.basis.plan.selections.len(), 21);
        assert_eq!(owner.historical.derived.prep_sessions.len(), 21);
        assert!(
            owner
                .historical
                .derived
                .prep_sessions
                .iter()
                .all(|s| s.tasks.len() == 16)
        );
    }
    assert_eq!(
        owners
            .iter()
            .flat_map(|o| o.historical.derived.prep_sessions.iter())
            .map(|s| s.tasks.len())
            .sum::<usize>(),
        20496
    );
}
