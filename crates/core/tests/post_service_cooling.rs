//! Mechanical tests only. The source quantities, unit yield, cold suitability,
//! storage and cooling schedule are SYNTHETIC premises, not recipe admission,
//! physical temperature, doneness, transport or human calibration evidence.
use mstrchf_contracts::*;
use mstrchf_core::{availability, batches, food::*, prep};

#[path = "support/original_component.rs"]
mod original_component;
use original_component::*;

fn failure_path(result: &Result<FoodDerivation, FoodFailure>, path: &str) {
    assert!(
        matches!(result, Err(FoodFailure::Invalid(error))
        if error.issues.iter().any(|issue| issue.path == path)),
        "Expected {path}, got {result:?}"
    );
}
fn keys_path(saved: &SavedFoodBasis, path: &str) {
    let result = required_identity_keys(
        &saved.plan,
        &saved.context,
        &saved.evidence,
        &saved.schedule,
    );
    assert!(
        matches!(&result, Err(FoodFailure::Invalid(error))
        if error.issues.iter().any(|issue| issue.path == path)),
        "Expected {path}, got {result:?}"
    );
}
fn recipe_path(recipe: &RecipeRevision, path: &str) {
    let result = parse_document::<RecipeRevision>(&canonical_bytes(recipe).unwrap());
    assert!(
        matches!(&result, Err(error)
        if error.issues.iter().any(|issue| issue.path == path)),
        "Expected {path}, got {result:?}"
    );
}
fn retime(saved: &mut SavedFoodBasis, source: ZonedMoment, service: ZonedMoment) {
    let old_slots = saved
        .plan
        .selections
        .iter()
        .map(|selection| selection.slot.clone())
        .collect::<Vec<_>>();
    let moments = [source, service];
    for i in 0..2 {
        saved.plan.selections[i].slot.date = moments[i].local_date.clone();
        saved.plan.selections[i].meal_overrides.eat_by = Nullable(Some(moments[i].clone()));
        let new_slot = saved.plan.selections[i].slot.clone();
        let cell = saved.context.weeks[0]
            .cells
            .iter_mut()
            .find(|cell| cell.slot == old_slots[i])
            .unwrap();
        cell.choice = MealLocation::Skip;
        cell.attendee_ids = BoundedVec(vec![]);
        let cell = saved.context.weeks[0]
            .cells
            .iter_mut()
            .find(|cell| cell.slot == new_slot)
            .unwrap();
        cell.choice = if i == 0 {
            MealLocation::AtHome
        } else {
            MealLocation::OnTheMove
        };
        cell.attendee_ids = BoundedVec(vec![saved.context.household.diners[0].diner_id.clone()]);
        for route in saved
            .schedule
            .layout
            .routes
            .iter_mut()
            .filter(|route| route.phase.occurrence.slot == old_slots[i])
        {
            route.phase.occurrence.slot = new_slot.clone();
            route.session.anchor_slot = new_slot.clone();
        }
        for fixed in saved
            .schedule
            .layout
            .fixed_starts
            .iter_mut()
            .filter(|fixed| fixed.phase.occurrence.slot == old_slots[i])
        {
            fixed.phase.occurrence.slot = new_slot.clone();
            let offset = if i == 0 {
                match fixed.phase.prep_id {
                    1 => -3600,
                    3 => 0,
                    4 => 300,
                    _ => panic!("Unexpected source phase"),
                }
            } else {
                -300
            };
            fixed.start = shifted(&moments[i], offset);
        }
    }
    saved.plan.batch_links[0].source_slot = saved.plan.selections[0].slot.clone();
    saved.plan.batch_links[0].target_slot = saved.plan.selections[1].slot.clone();
    saved.context.weeks[0].plan = saved.plan.clone();
    saved.context.requirements = BoundedVec(
        availability::meal_requirements(&saved.context.household, &saved.context.weeks[0], &[])
            .unwrap()
            .into(),
    );
    saved
        .context
        .requirements
        .iter_mut()
        .find(|requirement| requirement.slot == saved.plan.selections[1].slot)
        .unwrap()
        .equipment = BoundedVec(vec![]);
    canonicalize_schedule(saved);
    exact_fixed_occupancy(saved);
    assign_identities(saved);
}
fn tasks(result: &FoodDerivation) -> Vec<&ScheduledPrepTask> {
    result
        .plan
        .prep_sessions
        .iter()
        .flat_map(|session| session.tasks.iter())
        .collect()
}

#[test]
fn zero_offset_derivation_bytes_remain_stable() {
    let derived = derive(&cooling_case()).unwrap();
    assert_eq!(bytes_digest(&canonical_bytes(&derived.plan).unwrap()).0,
        "85c6255c610ba9be4279ee0b39301e84d58c7861417aa5c43dd9f8b41b2849ed");
}

fn earlier_case(service: ZonedMoment) -> SavedFoodBasis {
    let mut saved=cooling_case();
    retime(&mut saved,at(19,"18:00:00"),service);
    let mut source=saved.context.recipes[0].clone();
    for phase in source.prep.iter_mut().filter(|phase|matches!(phase.prep_id,1|3|4)) {
        phase.earliest_offset_seconds-=300;phase.latest_offset_seconds-=300;
    }
    source.storage[0].max_chilled_seconds=172800;
    refresh_claims(&mut source);replace_source(&mut saved,source);
    for fixed in saved.schedule.layout.fixed_starts.iter_mut()
        .filter(|fixed|fixed.phase.occurrence.slot==saved.plan.selections[0].slot) {
        fixed.start=shifted(&fixed.start,-300);
    }
    canonicalize_schedule(&mut saved);exact_fixed_occupancy(&mut saved);assign_identities(&mut saved);
    saved
}

#[test]
fn earlier_endpoint_preserves_original_clock_and_reduces_the_meal_relative_cooling_budget() {
    let legacy=derive(&cooling_case()).unwrap();
    assert!(tasks(&legacy).iter().any(|task|task.prep_id==4&&task.elapsed.end==at(19,"20:00:00")));
    let saved=earlier_case(at(20,"12:00:00"));let derived=derive(&saved).unwrap();
    assert_eq!(derived.plan.batches[0].components[0].original_production,
        ProductionMoment::Cooked{completed_at:at(19,"17:55:00")});
    assert!(tasks(&derived).iter().any(|task|task.prep_id==3
        &&task.elapsed.start==at(19,"17:55:00")&&task.elapsed.end==at(19,"18:00:00")));
    assert!(tasks(&derived).iter().any(|task|task.prep_id==4&&task.elapsed.end==at(19,"19:55:00")));
    prep::validate_schedule(&saved.plan,&saved.context,&saved.evidence,
        &saved.identities,&saved.schedule,&derived.plan.prep_sessions.0).unwrap();
    let mut forged=derived.plan.prep_sessions.0.clone();
    let session=forged.iter_mut().find(|session|session.tasks.iter().any(|task|task.prep_id==4)).unwrap();
    let task=session.tasks.iter_mut().find(|task|task.prep_id==4).unwrap();
    task.elapsed.end=at(19,"20:00:00");session.end=at(19,"20:00:00");
    assert!(prep::validate_schedule(&saved.plan,&saved.context,&saved.evidence,
        &saved.identities,&saved.schedule,&forged).is_err());
}

#[test]
fn earlier_original_expiry_is_exact_and_consumer_handling_still_waits_for_cooling() {
    assert!(derive(&earlier_case(at(21,"17:55:00"))).is_ok());
    let late=derive(&earlier_case(at(21,"17:55:01")));
    assert!(matches!(late,Err(FoodFailure::Blocked{..})),"{late:?}");
    // Five-minute consumer handling must start after cooling ends at19:55.
    assert!(derive(&earlier_case(at(19,"20:00:00"))).is_ok());
    assert!(derive(&earlier_case(at(19,"19:59:59"))).is_err());
}

#[test]
fn hot_source_surplus_cools_after_service_before_zero_fresh_portable_use() {
    let saved = cooling_case();
    let derived = derive(&saved).unwrap();
    let component = &derived.plan.batches[0].components[0];
    assert_eq!(component.produced, units(4));
    assert_eq!(component.allocated, units(2));
    assert_eq!(component.unallocated.amount, Rational::new(2, 1).unwrap());
    assert_eq!(
        component.original_production,
        ProductionMoment::Cooked {
            completed_at: at(19, "18:00:00")
        }
    );
    assert_eq!(derived.plan.allocations.len(), 2);
    assert!(
        derived
            .plan
            .allocations
            .iter()
            .all(|allocation| allocation.quantity == units(1) && allocation.planned_reheats == 0)
    );
    let source = derived
        .plan
        .prep_sessions
        .iter()
        .find(|session| {
            session
                .linked_slots
                .contains(&saved.plan.selections[0].slot)
        })
        .unwrap();
    assert_eq!(
        source
            .tasks
            .iter()
            .map(|task| task.prep_id)
            .collect::<Vec<_>>(),
        vec![1, 3, 4]
    );
    assert_eq!(source.tasks[0].elapsed.end, at(19, "18:00:00"));
    assert_eq!(source.tasks[1].elapsed.start, at(19, "18:00:00"));
    assert_eq!(source.tasks[1].elapsed.end, at(19, "18:05:00"));
    assert_eq!(source.tasks[2].elapsed.start, at(19, "18:05:00"));
    assert_eq!(source.tasks[2].elapsed.end, at(19, "20:00:00"));
    assert!(source.tasks[2].active_intervals.is_empty());
    assert_eq!(source.ingredient_needed_at, at(19, "17:00:00"));
    assert_eq!(derived.plan.demand.lines.len(), 6);
    for ingredient in saved.context.recipes[0].ingredients.iter() {
        let line = derived
            .plan
            .demand
            .lines
            .iter()
            .find(|line| line.ingredient_id == ingredient.ingredient_id)
            .unwrap();
        assert_eq!(&line.quantity, ingredient.quantity.0.as_ref().unwrap());
        assert_eq!(line.origin_batches.len(), 1);
        assert!(line.origin_extras.is_empty());
    }
    let cold = derived
        .plan
        .prep_sessions
        .iter()
        .find(|session| {
            session
                .linked_slots
                .contains(&saved.plan.selections[1].slot)
        })
        .unwrap();
    assert!(cold.ingredient_ids.is_empty());
    assert_eq!(cold.tasks.len(), 1);
    assert_eq!(
        cold.tasks[0].depends_on.len(),
        3,
        "Consumer requires cook AND every named cooling task"
    );
    assert!(
        source
            .tasks
            .iter()
            .all(|task| cold.tasks[0].depends_on.contains(&PrepTaskRef {
                session_id: source.session_id.clone(),
                task_id: task.task_id
            }))
    );
    assert!(saved.context.recipes[1].ingredients.is_empty());
    assert_eq!(
        saved.context.recipes[0].equipment.0,
        vec![
            EquipmentCode::Fridge,
            EquipmentCode::Hob,
            EquipmentCode::Oven
        ]
    );
    assert!(
        saved
            .context
            .requirements
            .iter()
            .find(|requirement| requirement.slot == saved.plan.selections[1].slot)
            .unwrap()
            .equipment
            .is_empty()
    );
}

#[test]
fn all_four_units_eaten_hot_execute_no_cooling_even_when_method_declares_it() {
    let mut saved = cooling_case();
    saved.plan.selections.0.truncate(1);
    saved.plan.batch_links.0.clear();
    let source_ref = saved.plan.selections[0].recipe.clone();
    saved.context.recipes.0.truncate(1);
    saved.context.stamp.recipe_refs = BoundedVec(vec![source_ref.clone()]);
    saved
        .evidence
        .recipes
        .retain(|basis| basis.recipe == source_ref);
    saved
        .evidence
        .classes
        .retain(|classes| classes.recipe == source_ref);
    saved
        .evidence
        .methods
        .retain(|method| method.recipe == source_ref);
    saved
        .evidence
        .resources
        .retain(|resource| resource.recipe == source_ref);
    saved.evidence.catalogue_digest = food_evidence_digest(&saved.evidence).unwrap();
    let resident = saved.context.household.diners[0].clone();
    let serving = saved.plan.selections[0].servings[0].clone();
    let mut residents = vec![];
    let mut servings = vec![];
    for i in 0..4 {
        let mut diner = resident.clone();
        diner.diner_id = DinerId(format!("dn_{:032x}", 0xf100 + i));
        let mut allocation = serving.clone();
        allocation.person = PersonRef::Diner {
            diner_id: diner.diner_id.clone(),
        };
        residents.push(diner);
        servings.push(allocation);
    }
    saved.context.household.diners = BoundedVec(residents);
    saved.plan.selections[0].servings = BoundedVec(servings);
    let source_slot = saved.plan.selections[0].slot.clone();
    for cell in saved.context.weeks[0].cells.iter_mut() {
        cell.choice = if cell.slot == source_slot {
            MealLocation::AtHome
        } else {
            MealLocation::Skip
        };
        cell.attendee_ids = BoundedVec(if cell.choice == MealLocation::AtHome {
            saved
                .context
                .household
                .diners
                .iter()
                .map(|diner| diner.diner_id.clone())
                .collect()
        } else {
            vec![]
        });
    }
    saved.context.weeks[0].plan = saved.plan.clone();
    saved.context.requirements = BoundedVec(
        availability::meal_requirements(&saved.context.household, &saved.context.weeks[0], &[])
            .unwrap()
            .into(),
    );
    saved
        .schedule
        .layout
        .routes
        .retain(|route| route.phase.occurrence.slot == source_slot && route.phase.prep_id == 1);
    saved
        .schedule
        .layout
        .fixed_starts
        .retain(|fixed| fixed.phase.occurrence.slot == source_slot && fixed.phase.prep_id == 1);
    exact_fixed_occupancy(&mut saved);
    assign_identities(&mut saved);
    let derived = derive(&saved).unwrap();
    assert_eq!(
        tasks(&derived)
            .iter()
            .map(|task| task.prep_id)
            .collect::<Vec<_>>(),
        vec![1]
    );
    let component = &derived.plan.batches[0].components[0];
    assert_eq!(component.produced, units(4));
    assert_eq!(component.allocated, units(4));
    assert_eq!(component.unallocated.amount, Rational::new(0, 1).unwrap());
    assert_eq!(
        component.original_production,
        ProductionMoment::Cooked {
            completed_at: at(19, "18:00:00")
        }
    );
    assert_eq!(derived.plan.allocations.len(), 4);
    assert_eq!(derived.plan.demand.lines.len(), 6);
}

#[test]
fn cooling_cap_and_last_phase_finish_allow_7200_but_refuse_one_second_over() {
    let source = cooling_recipe();
    parse_document::<RecipeRevision>(&canonical_bytes(&source).unwrap()).unwrap();
    let mut invalid = source.clone();
    if let Some(ComponentProductionSpec::CookedWithCooling {
        max_cooling_seconds,
        ..
    }) = &mut invalid.components[0].production.0
    {
        *max_cooling_seconds = 7201;
    }
    let result = parse_document::<RecipeRevision>(&canonical_bytes(&invalid).unwrap());
    assert!(
        matches!(&result, Err(error) if error.issues.iter().any(|issue| issue.path.ends_with("/data/cooling"))),
        "{result:?}"
    );
    let mut invalid = source;
    invalid.prep[3].latest_offset_seconds += 1;
    recipe_path(&invalid, "/components/cooling/phase");
    let mut saved = cooling_case();
    retime(&mut saved, at(19, "23:00:00"), at(20, "01:05:00"));
    let derived = derive(&saved).unwrap();
    let cold = tasks(&derived)
        .into_iter()
        .find(|task| task.recipe == saved.plan.selections[1].recipe)
        .unwrap();
    assert_eq!(cold.elapsed.start, at(20, "01:00:00"));
    assert!(
        tasks(&derived)
            .iter()
            .any(|task| task.prep_id == 4 && task.elapsed.end == cold.elapsed.start)
    );
}

#[test]
fn first_reuse_handling_before_cooling_end_refuses_even_if_consumption_is_later() {
    let mut saved = cooling_case();
    retime(&mut saved, at(19, "23:00:00"), at(20, "01:00:00"));
    // Service equals cooling end, but five-minute handling starts before it.
    // Dependency-aware horizon construction refuses this impossible consumer
    // window before placement: first handling must follow ALL cooling ends.
    failure_path(&derive(&saved), "/horizon/dependencies");
}

#[test]
fn omitted_named_cooling_or_consuming_phases_cannot_evade_dependencies() {
    let mut missing = cooling_case();
    missing
        .evidence
        .methods
        .iter_mut()
        .find(|method| {
            method.recipe == missing.plan.selections[0].recipe
                && method.kind == MethodKind::Production
        })
        .unwrap()
        .prep_ids = vec![1, 3];
    missing.evidence.catalogue_digest = food_evidence_digest(&missing.evidence).unwrap();
    keys_path(&missing, "/cooling/method");
    let mut no_handling = cooling_case();
    no_handling
        .evidence
        .methods
        .iter_mut()
        .find(|method| {
            method.recipe == no_handling.plan.selections[1].recipe
                && method.kind == MethodKind::Production
        })
        .unwrap()
        .prep_ids
        .clear();
    no_handling.evidence.catalogue_digest = food_evidence_digest(&no_handling.evidence).unwrap();
    keys_path(&no_handling, "/cooling/consuming_phase");
}

#[test]
fn forged_schedule_cooling_end_or_consumer_edges_are_rejected() {
    let saved = cooling_case();
    let derived = derive(&saved).unwrap();
    let mut altered = derived.plan.prep_sessions.0.clone();
    let task = altered
        .iter_mut()
        .flat_map(|session| session.tasks.iter_mut())
        .find(|task| task.prep_id == 4)
        .unwrap();
    task.elapsed.end = shifted(&task.elapsed.end, 1);
    let source = altered
        .iter_mut()
        .find(|session| session.tasks.iter().any(|task| task.prep_id == 4))
        .unwrap();
    source.end = shifted(&source.end, 1);
    let result = prep::validate_schedule(
        &saved.plan,
        &saved.context,
        &saved.evidence,
        &saved.identities,
        &saved.schedule,
        &altered,
    );
    assert!(
        matches!(&result, Err(FoodFailure::Invalid(error)) if error.issues.iter().any(|issue| issue.path == "/tasks/time")),
        "{result:?}"
    );
    let mut altered = derived.plan.prep_sessions.0.clone();
    let consumer = altered
        .iter_mut()
        .find(|session| {
            session
                .linked_slots
                .contains(&saved.plan.selections[1].slot)
        })
        .unwrap();
    consumer.tasks[0].depends_on.0.pop();
    let result = prep::validate_schedule(
        &saved.plan,
        &saved.context,
        &saved.evidence,
        &saved.identities,
        &saved.schedule,
        &altered,
    );
    assert!(
        matches!(&result, Err(FoodFailure::Invalid(error)) if error.issues.iter().any(|issue| issue.path == "/tasks/depends_on")),
        "{result:?}"
    );
}

#[test]
fn cooling_cannot_borrow_raw_ingredients_or_an_ordinary_positive_offset_phase() {
    let mut source = cooling_recipe();
    source.prep[2].ingredient_ids = BoundedVec(vec![source.ingredients[0].ingredient_id.clone()]);
    recipe_path(&source, "/components/cooling/phase");
    let mut source = cooling_recipe();
    source.steps[2].ingredient_ids = BoundedVec(vec![source.ingredients[0].ingredient_id.clone()]);
    recipe_path(&source, "/components/cooling/phase");
    let mut ordinary = cooling_case();
    let mut source = ordinary.context.recipes[0].clone();
    source.components[0].production = Nullable(Some(ComponentProductionSpec::Cooked {
        completion_prep_id: 1,
    }));
    refresh_claims(&mut source);
    replace_source(&mut ordinary, source);
    keys_path(&ordinary, "/prep/window");
}

#[test]
fn missing_phase_identity_wrong_completion_and_unconnected_cooling_refuse() {
    let mut source = cooling_recipe();
    if let Some(ComponentProductionSpec::CookedWithCooling {
        cooling_prep_ids, ..
    }) = &mut source.components[0].production.0
    {
        *cooling_prep_ids = BoundedVec(vec![3, 5]);
    }
    recipe_path(&source, "/components/cooling/prep_ids");
    let mut source = cooling_recipe();
    source.prep[0].latest_offset_seconds = -3599;
    recipe_path(&source, "/components/cooling/completion");
    let mut source = cooling_recipe();
    source.prep[3].depends_on = BoundedVec(vec![]);
    recipe_path(&source, "/components/cooling/phase");
}

#[test]
fn source_clock_and_expiry_are_not_renewed_by_two_hour_chilling_or_reuse() {
    let mut saved = cooling_case();
    retime(&mut saved, at(19, "18:00:00"), at(20, "18:00:00"));
    let derived = derive(&saved).unwrap();
    assert_eq!(
        derived.plan.batches[0].components[0].original_production,
        ProductionMoment::Cooked {
            completed_at: at(19, "18:00:00")
        }
    );
    retime(&mut saved, at(19, "18:00:00"), at(20, "18:00:01"));
    let result = derive(&saved);
    assert!(
        matches!(&result, Err(FoodFailure::Blocked { conflicts, .. }) if conflicts.iter().any(|conflict| conflict.code == ErrorCode::DomainConflict)),
        "{result:?}"
    );
    let error = batches::check_reuse(
        &at(19, "18:00:00"),
        &at(20, "18:00:01"),
        &saved.context.recipes[0].storage[0],
        172800,
        0,
    )
    .unwrap_err();
    assert_eq!(error.code, ErrorCode::DomainConflict);
    assert!(error.issues.iter().any(|issue| issue.path == "/use_at"));
}

#[test]
fn passive_chilling_still_reserves_fridge_against_other_retained_owners() {
    let mut saved = cooling_case();
    let derived = derive(&saved).unwrap();
    let mut task = tasks(&derived)
        .into_iter()
        .find(|task| task.prep_id == 4)
        .unwrap()
        .clone();
    task.task_id = 1;
    task.depends_on = BoundedVec(vec![]);
    let week_id = WeekId(format!("wk_{:032x}", 0xf900));
    let session_id = PrepSessionId(format!("ps_{:032x}", 0xf900));
    let mut slot = saved.plan.selections[0].slot.clone();
    slot.week_id = week_id.clone();
    let mut owner = OccupiedOwner {
        pin: PlanOwnerPin {
            week: WeekRevision {
                week_id: week_id.clone(),
                revision: Revision(1),
            },
            derived_digest: PayloadDigest("b".repeat(64)),
            basis_digest: PayloadDigest("c".repeat(64)),
        },
        sessions: vec![PrepSession {
            session_id: session_id.clone(),
            week_id,
            linked_slots: BoundedVec(vec![slot]),
            attached_to: Nullable(None),
            start: task.elapsed.start.clone(),
            end: task.elapsed.end.clone(),
            ingredient_needed_at: task.elapsed.start.clone(),
            tasks: BoundedVec(vec![task]),
            ingredient_ids: BoundedVec(vec![]),
        }],
        task_resources: vec![OccupiedTaskResources {
            task: PrepTaskRef {
                session_id,
                task_id: 1,
            },
            equipment: vec![EquipmentCode::Fridge],
            evidence_ids: vec![eid(3)],
        }],
        projection_digest: PayloadDigest("0".repeat(64)),
    };
    owner.projection_digest = occupied_owner_digest(&owner).unwrap();
    saved.schedule.occupied.owners.push(owner);
    saved.schedule.occupied.digest = occupancy_snapshot_digest(&saved.schedule.occupied).unwrap();
    failure_path(&derive(&saved), "/placement/no_schedule");
}

#[test]
fn cooling_crosses_midnight_and_dst_fold_using_real_instants_not_clock_subtraction() {
    let mut saved = cooling_case();
    let source = at(25, "00:30:00");
    let service = availability::resolve_meal_time(
        &LocalDate("2026-10-25".into()),
        &LocalTime("02:35:00".into()),
        &source.zone,
        None,
    )
    .unwrap();
    retime(&mut saved, source.clone(), service);
    let derived = derive(&saved).unwrap();
    let finish = tasks(&derived)
        .into_iter()
        .find(|task| task.prep_id == 4)
        .unwrap()
        .elapsed
        .end
        .clone();
    assert_eq!(finish.local_date, LocalDate("2026-10-25".into()));
    assert_eq!(finish.local_time, LocalTime("01:30:00".into()));
    assert_eq!(finish.resolution, Resolution::Later);
    assert_eq!(finish.instant, Instant("2026-10-25T01:30:00Z".into()));
    assert_eq!(
        derived.plan.batches[0].components[0].original_production,
        ProductionMoment::Cooked {
            completed_at: source
        }
    );
    let mut midnight = cooling_case();
    retime(&mut midnight, at(19, "23:00:00"), at(20, "01:05:00"));
    let result = derive(&midnight).unwrap();
    assert!(
        tasks(&result)
            .iter()
            .any(|task| task.prep_id == 4 && task.elapsed.end == at(20, "01:00:00"))
    );
}
