//! R2 controls for FN-R1-02 and FN-R1-03; original reviewer evidence is immutable.
use super::*;

#[path = "../../../../tests/fixtures/food/native/support.rs"]
mod native;

fn reference_to(session: u32, task_id: u16) -> PrepTaskRef {
    PrepTaskRef {
        session_id: sid(session),
        task_id,
    }
}
fn retained_task(id: u16, start: &str, end: &str, deps: Vec<PrepTaskRef>) -> ScheduledPrepTask {
    task(
        id,
        at("2026-12-27", start),
        at("2026-12-27", end),
        false,
        deps,
    )
}
fn owner_with(f: &Fixture, week: u32, sessions: Vec<PrepSession>) -> OccupiedOwner {
    let mut owner = prior_owner(f, false, vec![]);
    owner.pin.week.week_id = WeekId(format!("wk_{week:032x}"));
    owner.sessions = sessions;
    owner.task_resources.clear();
    for s in &mut owner.sessions {
        s.week_id = owner.pin.week.week_id.clone();
        s.linked_slots[0] = f.plan.selections[0].slot.clone();
        s.linked_slots[0].week_id = s.week_id.clone();
        for t in s.tasks.iter() {
            owner.task_resources.push(OccupiedTaskResources {
                task: PrepTaskRef {
                    session_id: s.session_id.clone(),
                    task_id: t.task_id,
                },
                equipment: vec![],
                evidence_ids: vec![pin(8)],
            });
        }
    }
    owner
}
fn invalid_path(f: &Fixture) -> String {
    match f.derive().unwrap_err() {
        FoodFailure::Invalid(error) => error.issues[0].path.clone(),
        other => panic!("expected local occupancy error, got {other:?}"),
    }
}

#[test]
fn retained_self_dependency_and_missing_task_in_complete_session_reject() {
    for (task_id, expected) in [
        (1, "/occupied/dependency_cycle"),
        (2, "/occupied/dependencies"),
    ] {
        let mut f = derivation_fixture(true);
        let mut owner = prior_owner(&f, false, vec![]);
        owner.sessions[0].tasks[0].depends_on = BoundedVec(vec![reference_to(10, task_id)]);
        f.schedule.occupied.owners.push(owner);
        f.rehash_occupancy();
        assert_eq!(invalid_path(&f), expected);
    }
}
#[test]
fn retained_included_dependency_must_finish_by_consumer_start() {
    let mut f = derivation_fixture(true);
    let owner = owner_with(
        &f,
        2,
        vec![session(
            10,
            vec![
                retained_task(1, "19:00:00", "19:30:00", vec![reference_to(10, 2)]),
                retained_task(2, "19:30:00", "21:00:00", vec![]),
            ],
        )],
    );
    f.schedule.occupied.owners.push(owner);
    f.rehash_occupancy();
    assert_eq!(invalid_path(&f), "/occupied/dependencies/time");
}
#[test]
fn retained_zero_duration_cycle_rejects_despite_equal_dependency_times() {
    let mut f = derivation_fixture(true);
    let owner = owner_with(
        &f,
        2,
        vec![session(
            10,
            vec![
                retained_task(1, "20:30:00", "20:30:00", vec![reference_to(10, 2)]),
                retained_task(2, "20:30:00", "20:30:00", vec![reference_to(10, 1)]),
                retained_task(3, "19:00:00", "21:00:00", vec![]),
            ],
        )],
    );
    f.schedule.occupied.owners.push(owner);
    f.rehash_occupancy();
    assert_eq!(invalid_path(&f), "/occupied/dependency_cycle");
}
#[test]
fn retained_cross_owner_dependency_resolves_complete_target() {
    let mut f = derivation_fixture(true);
    let predecessor = owner_with(
        &f,
        2,
        vec![session(
            10,
            vec![retained_task(1, "19:00:00", "20:30:00", vec![])],
        )],
    );
    let consumer = owner_with(
        &f,
        3,
        vec![session(
            11,
            vec![retained_task(
                1,
                "20:30:00",
                "21:00:00",
                vec![reference_to(10, 1)],
            )],
        )],
    );
    f.schedule.occupied.owners = vec![predecessor, consumer];
    f.rehash_occupancy();
    f.derive().unwrap();
    f.schedule.occupied.owners[1].sessions[0].tasks[0].depends_on[0].task_id = 2;
    f.rehash_occupancy();
    assert_eq!(invalid_path(&f), "/occupied/dependencies");
}
#[test]
fn retained_same_session_dependency_adjacency_passes() {
    let mut f = derivation_fixture(true);
    let owner = owner_with(
        &f,
        2,
        vec![session(
            10,
            vec![
                retained_task(1, "19:00:00", "20:30:00", vec![]),
                retained_task(2, "20:30:00", "21:00:00", vec![reference_to(10, 1)]),
            ],
        )],
    );
    f.schedule.occupied.owners.push(owner);
    f.rehash_occupancy();
    f.derive().unwrap();
}
#[test]
fn retained_self_attachment_and_cross_owner_attachment_cycle_reject() {
    let mut f = derivation_fixture(true);
    let mut owner = prior_owner(&f, false, vec![]);
    owner.sessions[0].attached_to = Nullable(Some(sid(10)));
    f.schedule.layout.attachments.push(AttachmentChoice {
        session: f.identities.sessions[0].0.clone(),
        target_owner: owner.pin.clone(),
        target_session: sid(10),
    });
    f.schedule.occupied.owners.push(owner);
    f.rehash_occupancy();
    assert_eq!(invalid_path(&f), "/occupied/attachment_cycle");
    let second = owner_with(
        &f,
        3,
        vec![session(
            11,
            vec![retained_task(1, "19:00:00", "21:00:00", vec![])],
        )],
    );
    f.schedule.occupied.owners.push(second);
    f.schedule.occupied.owners[0].sessions[0].attached_to = Nullable(Some(sid(11)));
    f.rehash_occupancy();
    f.derive().unwrap();
    f.schedule.occupied.owners[1].sessions[0].attached_to = Nullable(Some(sid(10)));
    f.rehash_occupancy();
    assert_eq!(invalid_path(&f), "/occupied/attachment_cycle");
}
#[test]
fn retained_out_of_horizon_references_remain_producer_owned() {
    let mut f = derivation_fixture(true);
    let mut owner = prior_owner(&f, false, vec![]);
    owner.sessions[0].attached_to = Nullable(Some(sid(99)));
    owner.sessions[0].tasks[0].depends_on = BoundedVec(vec![reference_to(98, 3)]);
    f.schedule.occupied.owners.push(owner);
    f.rehash_occupancy();
    f.derive().unwrap();
}
#[test]
fn valid_prior_reservations_may_conflict_away_from_subject() {
    let mut f = derivation_fixture(true);
    let mut owner = owner_with(
        &f,
        2,
        vec![
            session(10, vec![retained_task(1, "20:30:00", "21:00:00", vec![])]),
            session(11, vec![retained_task(1, "20:30:00", "21:00:00", vec![])]),
        ],
    );
    for s in &mut owner.sessions {
        s.tasks[0].active_intervals = BoundedVec(vec![s.tasks[0].elapsed.clone()]);
    }
    f.schedule.occupied.owners.push(owner);
    f.rehash_occupancy();
    f.derive().unwrap();
}

fn rebind(f: &mut Fixture) {
    let old = f.plan.selections[0].recipe.clone();
    let r = RecipeRef {
        recipe_id: f.context.recipes[0].recipe_id.clone(),
        revision: f.context.recipes[0].revision,
        digest: recipe_digest(&f.context.recipes[0]).unwrap(),
    };
    f.plan.selections[0].recipe = r.clone();
    for s in &mut f.plan.selections[0].servings.0 {
        s.preset_ref.recipe = r.clone();
    }
    for s in &mut f.context.stamp.recipe_refs.0 {
        if *s == old {
            *s = r.clone();
        }
    }
    for b in &mut f.evidence.recipes {
        b.recipe = r.clone();
    }
    for b in &mut f.evidence.classes {
        b.recipe = r.clone();
        for c in &mut b.components {
            c.recipe = r.clone();
        }
    }
    for m in &mut f.evidence.methods {
        m.recipe = r.clone();
    }
    for p in &mut f.evidence.resources {
        p.recipe = r.clone();
    }
    for p in &mut f.schedule.layout.routes {
        p.phase.occurrence.recipe = r.clone();
    }
    for p in &mut f.schedule.layout.fixed_starts {
        p.phase.occurrence.recipe = r.clone();
    }
    f.evidence.catalogue_digest = food_evidence_digest(&f.evidence).unwrap();
}
fn assert_session_ingredients(f: &Fixture) -> FoodDerivation {
    let d = f.derive().unwrap();
    assert_eq!(
        d.plan.prep_sessions[0].ingredient_ids.0,
        vec![f.context.recipes[0].ingredients[0].ingredient_id.clone()]
    );
    assert_eq!(
        d.plan.prep_sessions[0].ingredient_needed_at.instant.0,
        "2026-12-28T17:00:00Z"
    );
    validate_schedule(
        &f.plan,
        &f.context,
        &f.evidence,
        &f.identities,
        &f.schedule,
        &d.plan.prep_sessions,
    )
    .unwrap();
    d
}
#[test]
fn executed_ingredient_need_precedes_late_phase_metadata_and_forged_output_rejects() {
    let mut f = derivation_fixture(false);
    let ingredient = f.context.recipes[0].prep[0].ingredient_ids[0].clone();
    f.context.recipes[0].prep[0].ingredient_ids.0.clear();
    f.context.recipes[0].prep[4].ingredient_ids.0 = vec![ingredient];
    rebind(&mut f);
    let d = assert_session_ingredients(&f);
    let mut forged = d.plan.prep_sessions.0.clone();
    forged[0].ingredient_needed_at = at("2026-12-28", "17:40:00");
    assert!(
        validate_schedule(
            &f.plan,
            &f.context,
            &f.evidence,
            &f.identities,
            &f.schedule,
            &forged
        )
        .is_err()
    );
    f.feasibility.delivery = DeliveryInput::Proposed {
        start: at("2026-12-28", "17:20:00"),
        end: at("2026-12-28", "17:30:00"),
        revision: Revision(1),
    };
    f.feasibility.digest = digest_feasibility_inputs(&f.feasibility).unwrap();
    match f.derive().unwrap_err() {
        FoodFailure::Blocked {
            feasibility: Some(report),
            ..
        } => assert!(!report.blocking.is_empty()),
        other => panic!("late delivery must retain complete blocking report: {other:?}"),
    }
}
#[test]
fn explicit_instruction_ingredients_supply_session_union_without_phase_metadata() {
    let mut f = derivation_fixture(false);
    for p in &mut f.context.recipes[0].prep.0 {
        p.ingredient_ids.0.clear();
    }
    for step in f.context.recipes[0].steps.0.iter_mut().take(4) {
        step.component_ids.0.clear();
    }
    rebind(&mut f);
    let d = assert_session_ingredients(&f);
    let mut forged = d.plan.prep_sessions.0.clone();
    forged[0].ingredient_ids.0.clear();
    assert!(
        validate_schedule(
            &f.plan,
            &f.context,
            &f.evidence,
            &f.identities,
            &f.schedule,
            &forged
        )
        .is_err()
    );
}
#[test]
fn instruction_component_handling_supplies_local_ingredient_union() {
    let mut f = derivation_fixture(false);
    for p in &mut f.context.recipes[0].prep.0 {
        p.ingredient_ids.0.clear();
    }
    for step in &mut f.context.recipes[0].steps.0 {
        step.ingredient_ids.0.clear();
    }
    rebind(&mut f);
    assert_session_ingredients(&f);
}
#[test]
fn phase_metadata_remains_sufficient_for_local_ingredient_need() {
    let mut f = derivation_fixture(false);
    for (i, step) in f.context.recipes[0].steps.0.iter_mut().enumerate() {
        step.ingredient_ids.0.clear();
        if i < 4 {
            step.component_ids.0.clear();
        }
    }
    rebind(&mut f);
    assert_session_ingredients(&f);
}
#[test]
fn direct_reuse_instructions_do_not_add_new_raw_ingredients() {
    let mut saved = native::basis("subject-saved-basis");
    let mut sessions: Vec<PrepSession> =
        serde_json::from_value(native::value("subject-derived")["prep_sessions"].clone()).unwrap();
    // Split the literal's shared session into two local test sessions so the
    // direct-reuse phase's raw-ingredient exclusion is observable alone.
    let dinner = saved.plan.selections[1].slot.clone();
    let key = SessionKey {
        anchor_slot: dinner.clone(),
        role: SessionRole::Cooking,
        ordinal: 1,
    };
    saved.schedule.layout.routes[1].session = key.clone();
    saved.identities.sessions.push((key, sid(2)));
    saved.identities.digest = food_identities_digest(&saved.identities).unwrap();
    let mut reuse_task = sessions[0].tasks.0.pop().unwrap();
    reuse_task.task_id = 1;
    let mut reuse = session(2, vec![reuse_task]);
    reuse.linked_slots = BoundedVec(vec![dinner]);
    sessions[0].linked_slots.0.truncate(1);
    sessions[0].end = sessions[0].tasks[0].elapsed.end.clone();
    sessions.push(reuse);
    validate_schedule(
        &saved.plan,
        &saved.context,
        &saved.evidence,
        &saved.identities,
        &saved.schedule,
        &sessions,
    )
    .unwrap();
    assert!(sessions[1].ingredient_ids.is_empty());
    sessions[1].ingredient_ids = sessions[0].ingredient_ids.clone();
    assert!(
        validate_schedule(
            &saved.plan,
            &saved.context,
            &saved.evidence,
            &saved.identities,
            &saved.schedule,
            &sessions
        )
        .is_err()
    );
}
