//! SYNTHETIC scheduler mechanics only. Quantities, five-minute handling,
//! continuous chilling and cold suitability are test premises, not production
//! food admission, observed temperatures, transport or cooking evidence.
use mstrchf_contracts::*;
use mstrchf_core::{availability, food::*, portable, prep};

#[path = "support/original_component.rs"]
mod original_component;
use original_component::*;

fn assert_path<T: std::fmt::Debug>(result: Result<T, FoodFailure>, path: &str) {
    assert!(
        matches!(&result, Err(FoodFailure::Invalid(error))
            if error.issues.iter().any(|issue| issue.path == path)),
        "Expected {path}, got {result:?}"
    );
}

#[test]
fn zero_offset_portable_declaration_bytes_remain_stable() {
    let saved=fixture();
    assert_eq!(bytes_digest(&canonical_bytes(&saved.plan.selections[1].portable_cold).unwrap()).0,
        "eb47f370ac18d7029eac1049291eb983f2d67146072ef707ba6e73b8a617a749");
}

#[test]
fn portable_v1_refuses_an_earlier_original_endpoint_instead_of_binding_meal_time() {
    let mut saved=fixture();
    let mut source=saved.context.recipes[0].clone();
    for phase in source.prep.iter_mut().filter(|phase|matches!(phase.prep_id,1|3|4)) {
        phase.earliest_offset_seconds-=300;phase.latest_offset_seconds-=300;
    }
    refresh_claims(&mut source);replace_source(&mut saved,source);
    let selected=&saved.plan.selections[1];
    let journey=selected.portable_cold.as_ref().unwrap();
    let result=portable::declare_journey(&saved.context,selected,journey.packing_complete_at.clone(),
        journey.depart_at.clone(),journey.declaration.confirmed_at.clone());
    assert!(matches!(&result,Err(error) if error.issues.iter().any(|issue|issue.path=="/portable_cold/source_completion")),"{result:?}");
    let result=portable::validate_journey(&saved.context,selected);
    assert!(matches!(&result,Err(error) if error.issues.iter().any(|issue|issue.path=="/portable_cold/source_completion")),"{result:?}");
}

fn redeclare(saved: &mut SavedFoodBasis, packed: ZonedMoment, departed: ZonedMoment) {
    // A changed synthetic journey gets a new explicit declaration; do not ask
    // availability to validate a transient new slot carrying the old one.
    saved.plan.selections[1].portable_cold = None;
    saved.context.weeks[0].plan = saved.plan.clone();
    saved.context.requirements = BoundedVec(
        availability::meal_requirements(&saved.context.household, &saved.context.weeks[0], &[])
            .unwrap()
            .into(),
    );
    let journey = portable::declare_journey(
        &saved.context,
        &saved.plan.selections[1],
        packed,
        departed,
        Instant("2026-10-18T12:00:00Z".into()),
    )
    .unwrap();
    saved.plan.selections[1].portable_cold = Some(journey);
    saved.context.weeks[0].plan = saved.plan.clone();
}

fn repin_cold(saved: &mut SavedFoodBasis, mut recipe: RecipeRevision) {
    let old = saved.plan.selections[1].recipe.clone();
    refresh_claims(&mut recipe);
    let reference = reference(&recipe);
    let (basis, classes) = premise_basis(&recipe);
    saved.context.recipes[1] = recipe;
    saved.context.stamp.recipe_refs[1] = reference.clone();
    saved.evidence.recipes[1] = basis;
    saved.evidence.classes[1] = classes;
    saved.plan.selections[1].recipe = reference.clone();
    for serving in saved.plan.selections[1].servings.iter_mut() {
        serving.preset_ref.recipe = reference.clone();
    }
    for method in saved
        .evidence
        .methods
        .iter_mut()
        .filter(|value| value.recipe == old)
    {
        method.recipe = reference.clone();
    }
    for resource in saved
        .evidence
        .resources
        .iter_mut()
        .filter(|value| value.recipe == old)
    {
        resource.recipe = reference.clone();
    }
    for route in saved
        .schedule
        .layout
        .routes
        .iter_mut()
        .filter(|value| value.phase.occurrence.recipe == old)
    {
        route.phase.occurrence.recipe = reference.clone();
    }
    for fixed in saved
        .schedule
        .layout
        .fixed_starts
        .iter_mut()
        .filter(|value| value.phase.occurrence.recipe == old)
    {
        fixed.phase.occurrence.recipe = reference.clone();
    }
}

fn fixture() -> SavedFoodBasis {
    let mut saved = cooling_case();
    saved.plan.selections[1].origin = SelectionOrigin::Fixed;
    let mut cold = saved.context.recipes[1].clone();
    cold.equipment = BoundedVec(vec![EquipmentCode::Fridge]);
    cold.steps[0].text = Text("SYNTHETIC home packing, five active minutes; no observed temperature or handling qualification.".into());
    cold.prep[0].title = Title("SYNTHETIC home packing".into());
    let mut service = cold.steps[0].clone();
    service.step_id = 2;
    service.depends_on = BoundedVec(vec![1]);
    service.text = Text("SYNTHETIC appliance-free cold service, five active minutes; owner-declared chilling is not transport evidence.".into());
    cold.steps.0.push(service);
    let mut phase = cold.prep[0].clone();
    phase.prep_id = 2;
    phase.step_ids = BoundedVec(vec![2]);
    phase.depends_on = BoundedVec(vec![1]);
    phase.title = Title("SYNTHETIC portable cold service".into());
    cold.prep.0.push(phase);
    cold.reused_inputs[0].step_ids = BoundedVec(vec![1, 2]);
    cold.active_minutes = 10;
    cold.elapsed_minutes = 10;
    repin_cold(&mut saved, cold);
    let reference = saved.plan.selections[1].recipe.clone();
    let method = saved
        .evidence
        .methods
        .iter_mut()
        .find(|method| method.recipe == reference)
        .unwrap();
    method.prep_ids = vec![1, 2];
    method.portable_cold = Some(PortableColdMethod {
        packing_prep_id: 1,
        service_prep_id: 2,
    });
    saved
        .evidence
        .resources
        .iter_mut()
        .find(|resource| resource.recipe == reference && resource.prep_id == 1)
        .unwrap()
        .equipment = vec![EquipmentCode::Fridge];
    saved.evidence.resources.push(PhaseResources {
        recipe: reference.clone(),
        prep_id: 2,
        equipment: vec![],
        evidence_ids: vec![eid(3)],
    });
    let packing = saved
        .schedule
        .layout
        .routes
        .iter_mut()
        .find(|route| route.phase.occurrence.recipe == reference)
        .unwrap();
    packing.session.role = SessionRole::AdvancePrep;
    let mut service = packing.clone();
    service.phase.prep_id = 2;
    service.session.role = SessionRole::Cooking;
    saved.schedule.layout.routes.push(service.clone());
    saved
        .schedule
        .layout
        .fixed_starts
        .push(PhaseStartConstraint {
            phase: service.phase,
            start: at(20, "11:55:00"),
        });
    retime(
        &mut saved,
        at(19, "18:00:00"),
        at(20, "12:00:00"),
        at(20, "07:30:00"),
        at(20, "08:00:00"),
    );
    saved
}

fn retime(
    saved: &mut SavedFoodBasis,
    source: ZonedMoment,
    service: ZonedMoment,
    packed: ZonedMoment,
    departed: ZonedMoment,
) {
    let old = saved
        .plan
        .selections
        .iter()
        .map(|selection| selection.slot.clone())
        .collect::<Vec<_>>();
    let moments = [source, service];
    for i in 0..2 {
        saved.plan.selections[i].slot.date = moments[i].local_date.clone();
        saved.plan.selections[i].meal_overrides.eat_by = Nullable(Some(moments[i].clone()));
        let slot = saved.plan.selections[i].slot.clone();
        let cell = saved.context.weeks[0]
            .cells
            .iter_mut()
            .find(|cell| cell.slot == old[i])
            .unwrap();
        cell.choice = MealLocation::Skip;
        cell.attendee_ids = BoundedVec(vec![]);
        let cell = saved.context.weeks[0]
            .cells
            .iter_mut()
            .find(|cell| cell.slot == slot)
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
            .filter(|route| route.phase.occurrence.slot == old[i])
        {
            route.phase.occurrence.slot = slot.clone();
            route.session.anchor_slot = slot.clone();
        }
        for fixed in saved
            .schedule
            .layout
            .fixed_starts
            .iter_mut()
            .filter(|fixed| fixed.phase.occurrence.slot == old[i])
        {
            fixed.phase.occurrence.slot = slot.clone();
            fixed.start = if i == 0 {
                shifted(
                    &moments[i],
                    match fixed.phase.prep_id {
                        1 => -3600,
                        3 => 0,
                        4 => 300,
                        _ => panic!("Unexpected source phase"),
                    },
                )
            } else if fixed.phase.prep_id == 1 {
                shifted(&packed, -300)
            } else {
                shifted(&moments[i], -300)
            };
        }
    }
    saved.plan.batch_links[0].source_slot = saved.plan.selections[0].slot.clone();
    saved.plan.batch_links[0].target_slot = saved.plan.selections[1].slot.clone();
    canonicalize_schedule(saved);
    saved.evidence.catalogue_digest = food_evidence_digest(&saved.evidence).unwrap();
    redeclare(saved, packed, departed);
    exact_fixed_occupancy(saved);
    assign_identities(saved);
}

fn task<'a>(derived: &'a FoodDerivation, reference: &RecipeRef, id: u16) -> &'a ScheduledPrepTask {
    derived
        .plan
        .prep_sessions
        .iter()
        .flat_map(|session| session.tasks.iter())
        .find(|task| task.recipe == *reference && task.prep_id == id)
        .unwrap()
}

#[test]
fn saved_morning_packing_and_lunch_service_preserve_original_clock_and_demands() {
    let saved = fixture();
    let result = derive(&saved).unwrap();
    let cold = &saved.plan.selections[1].recipe;
    let packing = task(&result, cold, 1);
    let service = task(&result, cold, 2);
    assert_eq!(packing.elapsed.start, at(20, "07:25:00"));
    assert_eq!(packing.elapsed.end, at(20, "07:30:00"));
    assert_eq!(service.elapsed.start, at(20, "11:55:00"));
    assert_eq!(service.elapsed.end, at(20, "12:00:00"));
    let portable_sessions = result
        .plan
        .prep_sessions
        .iter()
        .filter(|session| session.tasks.iter().any(|task| task.recipe == *cold))
        .collect::<Vec<_>>();
    assert_eq!(portable_sessions.len(), 2);
    assert!(
        portable_sessions
            .iter()
            .all(|session| session.ingredient_ids.is_empty())
    );
    assert_eq!(
        packing.depends_on.len(),
        3,
        "Cooking and both named cooling phases precede packing"
    );
    assert_eq!(
        result.plan.demand.lines.len(),
        6,
        "Only the six synthetic source inputs are bought"
    );
    assert_eq!(
        result.plan.batches[0].components[0].original_production,
        ProductionMoment::Cooked {
            completed_at: at(19, "18:00:00")
        }
    );
    assert_eq!(result.plan.allocations.len(), 2);
    assert!(
        result
            .plan
            .allocations
            .iter()
            .all(|allocation| allocation.planned_reheats == 0)
    );
    let portable_allocation = result
        .plan
        .allocations
        .iter()
        .find(|allocation| allocation.target_slot == saved.plan.selections[1].slot)
        .unwrap();
    assert_eq!(portable_allocation.use_at, at(20, "12:00:00"));
    prep::validate_schedule(
        &saved.plan,
        &saved.context,
        &saved.evidence,
        &saved.identities,
        &saved.schedule,
        &result.plan.prep_sessions,
    )
    .unwrap();
}

#[test]
fn exact_cooling_packing_departure_service_boundaries_pass_and_one_second_early_or_late_fails() {
    let mut saved = fixture();
    retime(
        &mut saved,
        at(19, "23:00:00"),
        at(20, "01:10:00"),
        at(20, "01:05:00"),
        at(20, "01:05:00"),
    );
    derive(&saved).unwrap();
    let mut early = saved.clone();
    let packed = at(20, "01:04:59");
    early
        .schedule
        .layout
        .fixed_starts
        .iter_mut()
        .find(|fixed| {
            fixed.phase.occurrence.recipe == early.plan.selections[1].recipe
                && fixed.phase.prep_id == 1
        })
        .unwrap()
        .start = shifted(&packed, -300);
    redeclare(&mut early, packed, at(20, "01:05:00"));
    assert_path(derive(&early), "/horizon/dependencies");
    let mut late = saved;
    redeclare(&mut late, at(20, "01:05:00"), at(20, "01:05:01"));
    assert_path(derive(&late), "/prep/window");
}

#[test]
fn portable_roles_cannot_share_a_session_or_attach_to_another_session() {
    let saved = fixture();
    let reference = &saved.plan.selections[1].recipe;
    let packing = saved
        .schedule
        .layout
        .routes
        .iter()
        .find(|route| route.phase.occurrence.recipe == *reference && route.phase.prep_id == 1)
        .unwrap()
        .session
        .clone();
    let mut merged = saved.clone();
    merged
        .schedule
        .layout
        .routes
        .iter_mut()
        .find(|route| route.phase.occurrence.recipe == *reference && route.phase.prep_id == 2)
        .unwrap()
        .session = packing;
    assert_path(
        required_identity_keys(
            &merged.plan,
            &merged.context,
            &merged.evidence,
            &merged.schedule,
        ),
        "/portable_cold/sessions",
    );
    for phase_id in [1, 2] {
        let mut attached = saved.clone();
        let session = attached
            .schedule
            .layout
            .routes
            .iter()
            .find(|route| {
                route.phase.occurrence.recipe == *reference && route.phase.prep_id == phase_id
            })
            .unwrap()
            .session
            .clone();
        attached.schedule.layout.attachments.push(AttachmentChoice {
            session,
            target_owner: PlanOwnerPin {
                week: WeekRevision {
                    week_id: WeekId(format!("wk_{:032x}", 0xf900)),
                    revision: Revision(1),
                },
                derived_digest: PayloadDigest("b".repeat(64)),
                basis_digest: PayloadDigest("c".repeat(64)),
            },
            target_session: PrepSessionId(format!("ps_{:032x}", 0xf900)),
        });
        assert_path(
            required_identity_keys(
                &attached.plan,
                &attached.context,
                &attached.evidence,
                &attached.schedule,
            ),
            "/portable_cold/attachments",
        );
    }
}

#[test]
fn saved_schedule_cannot_move_packing_or_omit_original_cooling_dependencies() {
    let saved = fixture();
    let result = derive(&saved).unwrap();
    let reference = &saved.plan.selections[1].recipe;
    let mut shifted_output = result.plan.prep_sessions.0.clone();
    let session = shifted_output
        .iter_mut()
        .find(|session| {
            session
                .tasks
                .iter()
                .any(|task| task.recipe == *reference && task.prep_id == 1)
        })
        .unwrap();
    session.start = shifted(&session.start, -1);
    session.end = shifted(&session.end, -1);
    session.ingredient_needed_at = session.start.clone();
    session.tasks[0].elapsed.start = session.start.clone();
    session.tasks[0].elapsed.end = session.end.clone();
    session.tasks[0].active_intervals = BoundedVec(vec![session.tasks[0].elapsed.clone()]);
    assert_path(
        prep::validate_schedule(
            &saved.plan,
            &saved.context,
            &saved.evidence,
            &saved.identities,
            &saved.schedule,
            &shifted_output,
        ),
        "/tasks/time",
    );
    let mut missing_edge = result.plan.prep_sessions.0.clone();
    missing_edge
        .iter_mut()
        .flat_map(|session| session.tasks.iter_mut())
        .find(|task| task.recipe == *reference && task.prep_id == 1)
        .unwrap()
        .depends_on
        .0
        .pop();
    assert_path(
        prep::validate_schedule(
            &saved.plan,
            &saved.context,
            &saved.evidence,
            &saved.identities,
            &saved.schedule,
            &missing_edge,
        ),
        "/tasks/depends_on",
    );
}

fn reserve(saved: &mut SavedFoodBasis, phase_id: u16, active: bool, equipment: Vec<EquipmentCode>) {
    let derived = derive(saved).unwrap();
    let mut task = task(&derived, &saved.plan.selections[1].recipe, phase_id).clone();
    task.task_id = 1;
    task.depends_on = BoundedVec(vec![]);
    task.active_intervals = BoundedVec(if active {
        vec![task.elapsed.clone()]
    } else {
        vec![]
    });
    let week_id = WeekId(format!("wk_{:032x}", 0xf900));
    let session_id = PrepSessionId(format!("ps_{:032x}", 0xf900));
    let mut slot = saved.plan.selections[1].slot.clone();
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
            equipment,
            evidence_ids: vec![eid(3)],
        }],
        projection_digest: PayloadDigest("0".repeat(64)),
    };
    owner.projection_digest = occupied_owner_digest(&owner).unwrap();
    saved.schedule.occupied.owners.push(owner);
    saved.schedule.occupied.digest = occupancy_snapshot_digest(&saved.schedule.occupied).unwrap();
}

#[test]
fn packing_keeps_global_equipment_collision_and_equipment_free_service_keeps_attention_collision() {
    let saved = fixture();
    let mut packing = saved.clone();
    reserve(&mut packing, 1, false, vec![EquipmentCode::Fridge]);
    assert_path(derive(&packing), "/placement/no_schedule");
    let mut active_service = saved.clone();
    reserve(&mut active_service, 2, true, vec![]);
    assert_path(derive(&active_service), "/placement/no_schedule");
    let mut passive_fridge_at_service = saved;
    reserve(
        &mut passive_fridge_at_service,
        2,
        false,
        vec![EquipmentCode::Fridge],
    );
    derive(&passive_fridge_at_service).unwrap();
}

#[test]
fn original_expiry_reaches_service_and_dst_fold_compares_real_journey_instants() {
    let mut saved = fixture();
    retime(
        &mut saved,
        at(19, "18:00:00"),
        at(20, "18:00:00"),
        at(20, "07:30:00"),
        at(20, "08:00:00"),
    );
    derive(&saved).unwrap();
    let mut late = saved.clone();
    late.plan.selections[1].meal_overrides.eat_by = Nullable(Some(at(20, "18:00:01")));
    late.schedule
        .layout
        .fixed_starts
        .iter_mut()
        .find(|fixed| {
            fixed.phase.occurrence.recipe == late.plan.selections[1].recipe
                && fixed.phase.prep_id == 2
        })
        .unwrap()
        .start = at(20, "17:55:01");
    redeclare(&mut late, at(20, "07:30:00"), at(20, "08:00:00"));
    exact_fixed_occupancy(&mut late);
    assign_identities(&mut late);
    let expired = derive(&late);
    assert!(
        matches!(&expired, Err(FoodFailure::Blocked { conflicts, .. })
            if conflicts.iter().any(|conflict| conflict.code == ErrorCode::DomainConflict)),
        "{expired:?}"
    );
    let expiry = mstrchf_core::batches::check_reuse(
        &at(19, "18:00:00"),
        &at(20, "18:00:01"),
        &late.context.recipes[0].storage[0],
        172800,
        0,
    )
    .unwrap_err();
    assert!(expiry.issues.iter().any(|issue| issue.path == "/use_at"));
    let zone = ZoneId("Europe/London".into());
    let fold = |time: &str, resolution: Resolution| {
        availability::resolve_meal_time(
            &LocalDate("2026-10-25".into()),
            &LocalTime(time.into()),
            &zone,
            Some(resolution),
        )
        .unwrap()
    };
    let packed = fold("01:35:00", Resolution::Earlier);
    let departed = fold("01:10:00", Resolution::Later);
    let service = fold("01:20:00", Resolution::Later);
    retime(
        &mut saved,
        at(24, "23:30:00"),
        service.clone(),
        packed.clone(),
        departed.clone(),
    );
    let result = derive(&saved).unwrap();
    assert!(packed.local_time > departed.local_time);
    assert!(packed.instant < departed.instant);
    assert_eq!(
        task(&result, &saved.plan.selections[1].recipe, 1)
            .elapsed
            .end,
        packed
    );
    assert_eq!(
        task(&result, &saved.plan.selections[1].recipe, 2)
            .elapsed
            .end,
        service
    );
}

fn domain_path<T: std::fmt::Debug>(
    result: Result<T, DomainError>,
    code: ErrorCode,
    path: &str,
    label: &str,
) {
    assert!(
        matches!(&result, Err(error) if error.code == code
            && error.issues.iter().any(|issue| issue.path == path)),
        "{label}: expected {code:?} at {path}, got {result:?}"
    );
}

#[test]
fn declaration_binds_original_source_quantity_clock_and_full_reference() {
    let saved = fixture();
    portable::validate_journey(&saved.context, &saved.plan.selections[1]).unwrap();
    for edit in ["quantity", "clock", "reference"] {
        let mut changed = saved.clone();
        let source = &mut changed.context.weeks[0].plan.selections[0];
        match edit {
            "quantity" => {
                let BatchUse::NewBatch {
                    production_servings,
                    ..
                } = &mut source.batch_use
                else {
                    panic!("Expected original production");
                };
                *production_servings = p(3);
            }
            "clock" => source.meal_overrides.eat_by = Nullable(Some(at(19, "18:00:01"))),
            "reference" => {
                // Supply a shape-valid newer revision in the context. The cold
                // input still names the original full reference, so refusal
                // must come from custody rather than a missing catalogue pin.
                let mut recipe = changed.context.recipes[0].clone();
                recipe.revision = Revision(2);
                refresh_claims(&mut recipe);
                let reference = reference(&recipe);
                // Both revisions remain present because the retained target
                // recipe's input still requires the original catalogue pin.
                changed.context.recipes.0.insert(1, recipe);
                changed
                    .context
                    .stamp
                    .recipe_refs
                    .0
                    .insert(1, reference.clone());
                source.recipe = reference.clone();
                for serving in source.servings.iter_mut() {
                    serving.preset_ref.recipe = reference.clone();
                }
            }
            _ => unreachable!(),
        }
        let (code, path) = if edit == "reference" {
            changed.context.validate_shape().unwrap();
            (ErrorCode::IdentityMismatch, "/portable_cold/source")
        } else {
            (
                ErrorCode::DigestMismatch,
                "/portable_cold/declaration/journey_digest",
            )
        };
        domain_path(
            portable::validate_journey(&changed.context, &changed.plan.selections[1]),
            code,
            path,
            edit,
        );
    }
}

#[test]
fn declaration_binds_target_portions_service_packing_departure_and_existing_batch_identity() {
    let saved = fixture();
    for edit in ["portions", "service", "packing", "departure", "batch"] {
        let mut changed = saved.clone();
        let target = &mut changed.plan.selections[1];
        match edit {
            "portions" => target.servings[0].servings = p(2),
            "service" => target.meal_overrides.eat_by = Nullable(Some(at(20, "12:00:01"))),
            "packing" => {
                target.portable_cold.as_mut().unwrap().packing_complete_at = at(20, "07:30:01")
            }
            "departure" => target.portable_cold.as_mut().unwrap().depart_at = at(20, "08:00:01"),
            "batch" => {
                let batch = BatchId(format!("ba_{:032x}", 0xfa01));
                let BatchUse::Leftover { batch_id, .. } = &mut target.batch_use else {
                    panic!("Expected original-component reuse");
                };
                *batch_id = batch.clone();
                let BatchUse::NewBatch { batch_id, .. } =
                    &mut changed.context.weeks[0].plan.selections[0].batch_use
                else {
                    panic!("Expected original production");
                };
                // Even a present, otherwise matching source under another ID
                // cannot inherit the declaration for the original batch.
                *batch_id = batch;
            }
            _ => unreachable!(),
        }
        domain_path(
            portable::validate_journey(&changed.context, target),
            ErrorCode::DigestMismatch,
            "/portable_cold/declaration/journey_digest",
            edit,
        );
    }
    let mut orphan = saved.clone();
    let BatchUse::Leftover { batch_id, .. } = &mut orphan.plan.selections[1].batch_use else {
        panic!("Expected original-component reuse");
    };
    *batch_id = BatchId(format!("ba_{:032x}", 0xfa02));
    domain_path(
        portable::validate_journey(&orphan.context, &orphan.plan.selections[1]),
        ErrorCode::MissingReference,
        "/portable_cold/source",
        "missing original batch",
    );
}

fn portable_eligibility(
    saved: &SavedFoodBasis,
) -> Result<mstrchf_core::eligibility::Eligibility, DomainError> {
    let reviewed = mstrchf_core::recipes::assess_revision(
        &saved.context.recipes[1],
        &saved.evidence.recipes[1],
        &saved.evidence.classes[1],
    )
    .unwrap();
    let selection = &saved.plan.selections[1];
    let meal = saved
        .context
        .requirements
        .iter()
        .find(|meal| meal.slot == selection.slot)
        .unwrap();
    mstrchf_core::eligibility::eligible_with_food(
        &reviewed,
        meal,
        selection,
        &saved.context,
        &saved.evidence,
    )
}

#[test]
fn home_fridge_mobile_cold_storage_and_owner_confirmation_are_independent_requirements() {
    use mstrchf_core::eligibility::{Eligibility, eligible};
    let saved = fixture();
    assert_eq!(portable_eligibility(&saved).unwrap(), Eligibility::Eligible);
    let reviewed = mstrchf_core::recipes::assess_revision(
        &saved.context.recipes[1],
        &saved.evidence.recipes[1],
        &saved.evidence.classes[1],
    )
    .unwrap();
    let meal = saved
        .context
        .requirements
        .iter()
        .find(|meal| meal.slot == saved.plan.selections[1].slot)
        .unwrap();
    assert!(
        !matches!(eligible(&reviewed, meal).unwrap(), Eligibility::Eligible),
        "Ordinary equipped OnTheMove eligibility remains conservative"
    );
    for remove in ["home_fridge", "mobile_cold_storage"] {
        let mut changed = saved.clone();
        if remove == "home_fridge" {
            changed
                .context
                .household
                .preferences
                .equipment
                .0
                .retain(|equipment| *equipment != EquipmentCode::Fridge);
            assert!(changed.context.household.preferences.mobility.cold_storage);
        } else {
            changed.context.household.preferences.mobility.cold_storage = false;
            assert!(
                changed
                    .context
                    .household
                    .preferences
                    .equipment
                    .contains(&EquipmentCode::Fridge)
            );
        }
        changed.context.requirements = BoundedVec(
            availability::meal_requirements(
                &changed.context.household,
                &changed.context.weeks[0],
                &[],
            )
            .unwrap()
            .into(),
        );
        domain_path(
            portable_eligibility(&changed),
            ErrorCode::UnsupportedCapability,
            "/portable_cold/scope",
            remove,
        );
    }
    let mut undeclared = saved;
    undeclared.plan.selections[1].portable_cold = None;
    domain_path(
        portable_eligibility(&undeclared),
        ErrorCode::UnsupportedCapability,
        "/portable_cold/method",
        "missing owner confirmation",
    );
}

#[test]
fn forged_service_appliances_method_roles_or_extra_phase_do_not_gain_portable_eligibility() {
    let saved = fixture();
    for edit in [
        "service_equipment",
        "swapped_roles",
        "duplicate_role",
        "extra_method_phase",
    ] {
        let mut changed = saved.clone();
        let reference = changed.plan.selections[1].recipe.clone();
        let method = changed
            .evidence
            .methods
            .iter_mut()
            .find(|method| method.recipe == reference)
            .unwrap();
        let (code, path) = match edit {
            "service_equipment" => {
                changed
                    .evidence
                    .resources
                    .iter_mut()
                    .find(|resource| resource.recipe == reference && resource.prep_id == 2)
                    .unwrap()
                    .equipment = vec![EquipmentCode::Hob];
                (
                    ErrorCode::UnsupportedCapability,
                    "/methods/portable_cold/resources",
                )
            }
            "swapped_roles" => {
                method.portable_cold = Some(PortableColdMethod {
                    packing_prep_id: 2,
                    service_prep_id: 1,
                });
                (
                    ErrorCode::UnsupportedCapability,
                    "/methods/portable_cold/resources",
                )
            }
            "duplicate_role" => {
                method.portable_cold.as_mut().unwrap().service_prep_id = 1;
                (ErrorCode::InvalidValue, "/methods/portable_cold")
            }
            "extra_method_phase" => {
                method.prep_ids.push(3);
                (ErrorCode::InvalidValue, "/methods/portable_cold")
            }
            _ => unreachable!(),
        };
        // Recompute custody so refusal proves semantic closure, not just an
        // obsolete digest on the intentionally altered synthetic evidence.
        match food_evidence_digest(&changed.evidence) {
            Ok(digest) => changed.evidence.catalogue_digest = digest,
            Err(error) => {
                // Invalid role shape is rejected before evidence can receive a
                // digest, and the public eligibility entry must reject it too.
                assert!(matches!(edit, "duplicate_role" | "extra_method_phase"));
                domain_path(Err::<(), _>(error), code, path, edit);
            }
        }
        domain_path(portable_eligibility(&changed), code, path, edit);
    }
    let mut extra = saved;
    let mut recipe = extra.context.recipes[1].clone();
    let mut step = recipe.steps[1].clone();
    step.step_id = 3;
    recipe.steps.0.push(step);
    let mut phase = recipe.prep[1].clone();
    phase.prep_id = 3;
    phase.step_ids = BoundedVec(vec![3]);
    recipe.prep.0.push(phase);
    recipe.reused_inputs[0].step_ids.0.push(3);
    recipe.active_minutes += 5;
    recipe.elapsed_minutes += 5;
    repin_cold(&mut extra, recipe);
    extra.evidence.resources.push(PhaseResources {
        recipe: extra.plan.selections[1].recipe.clone(),
        prep_id: 3,
        equipment: vec![],
        evidence_ids: vec![eid(3)],
    });
    canonicalize_schedule(&mut extra);
    extra.evidence.catalogue_digest = food_evidence_digest(&extra.evidence).unwrap();
    domain_path(
        portable_eligibility(&extra),
        ErrorCode::UnsupportedCapability,
        "/methods/portable_cold/closure",
        "undeclared third recipe phase",
    );
}

fn suggested_patch(
    saved: &SavedFoodBasis,
    slot: SlotId,
    replacements: Vec<ProposedSelection>,
) -> Result<mstrchf_core::selection::SelectionPatch, mstrchf_core::selection::SelectionFailure> {
    let reviewed = saved
        .context
        .recipes
        .iter()
        .enumerate()
        .map(|(i, recipe)| {
            mstrchf_core::recipes::assess_revision(
                recipe,
                &saved.evidence.recipes[i],
                &saved.evidence.classes[i],
            )
            .unwrap()
        })
        .collect::<Vec<_>>();
    let intent = mstrchf_core::selection::SelectionIntent {
        mode: PlanMode::SwapNamed,
        slots: vec![slot],
        origin: SelectionOrigin::Suggested,
        repeat_permissions: vec![],
    };
    let proposal = PlanProposal {
        changes: BoundedVec(vec![WeekChange {
            week_id: saved.plan.week_id.clone(),
            base_revision: saved.context.weeks[0].revision,
            replace_slots: BoundedVec(replacements),
        }]),
    };
    mstrchf_core::selection::prepare_selection_patch_with_food(
        &saved.context,
        &intent,
        &proposal,
        &reviewed,
        &[],
        &saved.evidence,
    )
}

fn authority_path(
    result: Result<
        mstrchf_core::selection::SelectionPatch,
        mstrchf_core::selection::SelectionFailure,
    >,
    path: &str,
    label: &str,
) {
    assert!(
        matches!(&result, Err(mstrchf_core::selection::SelectionFailure::Invalid(error))
        if error.code == ErrorCode::AuthorityDenied && error.issues.iter().any(|issue| issue.path == path)),
        "{label}: expected owner custody refusal at {path}, got {result:?}"
    );
}

#[test]
fn suggested_patches_cannot_delete_reassign_or_change_source_of_active_or_inactive_declared_lunch()
{
    let saved = fixture();
    for inactive in [false, true] {
        let mut context = saved.clone();
        if inactive {
            context.context.weeks[0].day_enabled[1] = false;
            context.context.requirements = BoundedVec(
                availability::meal_requirements(
                    &context.context.household,
                    &context.context.weeks[0],
                    &[],
                )
                .unwrap()
                .into(),
            );
        }
        let source_slot = context.plan.selections[0].slot.clone();
        let target_slot = context.plan.selections[1].slot.clone();
        let unchanged = suggested_patch(&context, source_slot.clone(), vec![]).unwrap();
        assert_eq!(
            unchanged.context, context.context,
            "An empty patch preserves retained custody"
        );
        assert!(unchanged.changed_slots.is_empty());
        let before = context.clone();
        authority_path(
            suggested_patch(
                &context,
                target_slot.clone(),
                vec![ProposedSelection {
                    slot: target_slot.clone(),
                    selection: Nullable(None),
                }],
            ),
            "/proposal/selection/portable_cold",
            "target deletion",
        );
        let mut reassigned = context.plan.selections[1].clone();
        reassigned.portable_cold = None;
        reassigned.origin = SelectionOrigin::Suggested;
        reassigned.repeat_authorised = false;
        reassigned.recipe = context.plan.selections[0].recipe.clone();
        for serving in reassigned.servings.iter_mut() {
            serving.preset_ref.recipe = reassigned.recipe.clone();
        }
        authority_path(
            suggested_patch(
                &context,
                target_slot.clone(),
                vec![ProposedSelection {
                    slot: target_slot,
                    selection: Nullable(Some(reassigned)),
                }],
            ),
            "/proposal/selection/portable_cold",
            "target reassignment",
        );
        for edit in ["source_quantity", "source_clock"] {
            let mut source = context.plan.selections[0].clone();
            source.origin = SelectionOrigin::Suggested;
            if edit == "source_quantity" {
                let BatchUse::NewBatch {
                    production_servings,
                    ..
                } = &mut source.batch_use
                else {
                    panic!("Expected original production");
                };
                *production_servings = p(3);
            } else {
                source.meal_overrides.eat_by = Nullable(Some(at(19, "18:00:01")));
            }
            authority_path(
                suggested_patch(
                    &context,
                    source_slot.clone(),
                    vec![ProposedSelection {
                        slot: source_slot.clone(),
                        selection: Nullable(Some(source)),
                    }],
                ),
                "/proposal/selection/portable_cold/source",
                edit,
            );
        }
        assert_eq!(
            context, before,
            "Rejected AI patches leave the complete input unchanged"
        );
    }
}

#[test]
fn public_food_eligibility_binds_the_reviewed_recipe_to_the_exact_selection_reference() {
    let saved = fixture();
    let reviewed = mstrchf_core::recipes::assess_revision(
        &saved.context.recipes[1],
        &saved.evidence.recipes[1],
        &saved.evidence.classes[1],
    )
    .unwrap();
    let meal = saved
        .context
        .requirements
        .iter()
        .find(|meal| meal.slot == saved.plan.selections[1].slot)
        .unwrap();
    for field in ["id", "revision", "digest"] {
        let mut selection = saved.plan.selections[1].clone();
        match field {
            "id" => selection.recipe.recipe_id = saved.plan.selections[0].recipe.recipe_id.clone(),
            "revision" => selection.recipe.revision = Revision(2),
            "digest" => selection.recipe.digest = RecipeDigest("0".repeat(64)),
            _ => unreachable!(),
        }
        domain_path(
            mstrchf_core::eligibility::eligible_with_food(
                &reviewed,
                meal,
                &selection,
                &saved.context,
                &saved.evidence,
            ),
            ErrorCode::IdentityMismatch,
            "/portable_cold/recipe",
            field,
        );
    }
}

// Local adaptation of food_review_storage's historical/current snapshot
// pattern, retaining both the original producer and the declared cold lunch.
fn portable_continuation_fixture(
    saved: SavedFoodBasis,
) -> (HistoricalApproval, ContinuationInputs) {
    let derived = derive(&saved).unwrap().plan;
    let approved = ApprovedPlan {
        approval_id: ApprovalId(format!("au_{:032x}", 0xfa01)),
        week_id: derived.week_id.clone(),
        plan_revision: derived.plan_revision,
        derived_digest: derived.digest.clone(),
        context_stamp: derived.context_stamp.clone(),
        approved_at: at(18, "09:00:00").instant,
    };
    let order = ConfirmedOrder {
        order_id: OrderId(format!("or_{:032x}", 0xfa01)),
        confirmation: OrderConfirmation {
            week_id: approved.week_id.clone(),
            approval_id: approved.approval_id.clone(),
            delivery_start: at(18, "10:00:00"),
            delivery_end: at(18, "11:00:00"),
            confirmed_at: at(18, "09:30:00").instant,
            source: OrderConfirmationSource::UserConfirmation {},
        },
        approved_plan: approved.clone(),
        ordered_recipe_refs: derived.recipe_refs.clone(),
    };
    let mut products = ProductUseSnapshot {
        revision: Revision(1),
        digest: PayloadDigest("0".repeat(64)),
        valid_until: None,
        evidence: vec![],
        bindings: vec![],
        facts: vec![],
    };
    products.digest = digest_product_uses(&products).unwrap();
    let mut feasibility = FeasibilityInputs {
        revision: Revision(1),
        digest: PayloadDigest("0".repeat(64)),
        delivery: DeliveryInput::Confirmed {
            order: order.clone(),
        },
        products,
        confirmations: vec![],
        evaluated_at: at(18, "12:00:00").instant,
    };
    feasibility.digest = digest_feasibility_inputs(&feasibility).unwrap();
    let mut current = ContinuationInputs {
        revision: Revision(1),
        digest: PayloadDigest("0".repeat(64)),
        sources: vec![],
        requirements: LiveRequirements {
            revision: Revision(1),
            digest: PayloadDigest("0".repeat(64)),
            meals: saved
                .context
                .requirements
                .iter()
                .filter(|meal| {
                    saved
                        .plan
                        .selections
                        .iter()
                        .any(|selection| selection.slot == meal.slot)
                })
                .cloned()
                .collect(),
            unconfirmed: vec![],
        },
        evidence: RelevantEvidenceSnapshot {
            revision: Revision(1),
            digest: PayloadDigest("0".repeat(64)),
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
                        .find(|classes| classes.recipe == basis.recipe)
                        .unwrap()
                        .clone(),
                    methods: saved
                        .evidence
                        .methods
                        .iter()
                        .filter(|method| method.recipe == basis.recipe)
                        .cloned()
                        .collect(),
                    resources: saved
                        .evidence
                        .resources
                        .iter()
                        .filter(|resource| resource.recipe == basis.recipe)
                        .cloned()
                        .collect(),
                })
                .collect(),
        },
        policy: LiveFoodPolicy {
            version: Revision(1),
            digest: PayloadDigest("0".repeat(64)),
            general_seconds: 172800,
            rice_seconds: 86400,
            maximum_reheats: 0,
        },
        occupied: saved.schedule.occupied.clone(),
        feasibility,
    };
    current.occupied.query.as_mut().unwrap().excluded_subject = Some(PlanOwnerPin {
        week: WeekRevision {
            week_id: saved.plan.week_id.clone(),
            revision: saved.plan.revision,
        },
        derived_digest: derived.digest.clone(),
        basis_digest: saved_basis_digest(&saved).unwrap(),
    });
    current.requirements.digest = live_requirements_digest(&current.requirements).unwrap();
    current.evidence.digest = relevant_evidence_digest(&current.evidence).unwrap();
    current.policy.digest = live_food_policy_digest(&current.policy).unwrap();
    current.occupied.digest = occupancy_snapshot_digest(&current.occupied).unwrap();
    for (source, revision, digest) in [
        (
            LiveSource::Essentials,
            Revision(1),
            PayloadDigest("0".repeat(64)),
        ),
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
        (
            LiveSource::Order,
            Revision(1),
            PayloadDigest("0".repeat(64)),
        ),
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
    (
        HistoricalApproval {
            basis: saved,
            derived,
            approved,
            order,
        },
        current,
    )
}

#[test]
fn continuation_preserves_history_and_rechecks_portable_mapping_chilling_and_home_equipment() {
    let (historical, current) = portable_continuation_fixture(fixture());
    let before = historical.clone();
    let checked = verify_historical(&historical).unwrap();
    let report = assess_continuation(&checked, &current).unwrap();
    assert!(
        report.blocking.is_empty(),
        "Unchanged declared route: {report:?}"
    );
    let target = &historical.basis.plan.selections[1];
    for edit in ["method_roles", "mobile_cold_storage", "home_fridge"] {
        let mut changed = current.clone();
        match edit {
            "method_roles" => {
                let LiveRecipeState::Admitted { methods, .. } = changed.evidence.recipes.iter_mut()
                    .find(|recipe| matches!(recipe, LiveRecipeState::Admitted { basis, .. } if basis.recipe == target.recipe)).unwrap()
                else { unreachable!() };
                methods[0].portable_cold = Some(PortableColdMethod {
                    packing_prep_id: 2,
                    service_prep_id: 1,
                });
            }
            "mobile_cold_storage" => {
                changed
                    .requirements
                    .meals
                    .iter_mut()
                    .find(|meal| meal.slot == target.slot)
                    .unwrap()
                    .mobility
                    .cold_storage = false
            }
            "home_fridge" => {
                let source = &historical.basis.plan.selections[0];
                let equipment = &mut changed
                    .requirements
                    .meals
                    .iter_mut()
                    .find(|meal| meal.slot == source.slot)
                    .unwrap()
                    .equipment
                    .0;
                assert!(equipment.contains(&EquipmentCode::Fridge));
                equipment.retain(|value| *value != EquipmentCode::Fridge);
            }
            _ => unreachable!(),
        }
        changed.requirements.digest = live_requirements_digest(&changed.requirements).unwrap();
        changed.sources[6].digest = changed.requirements.digest.clone();
        changed.evidence.digest = relevant_evidence_digest(&changed.evidence).unwrap();
        changed.sources[5].digest = changed.evidence.digest.clone();
        changed.digest = continuation_inputs_digest(&changed).unwrap();
        let report = assess_continuation(&checked, &changed).unwrap();
        assert!(!report.blocking.is_empty(), "{edit}: {report:?}");
        let expected = match edit {
            "method_roles" => ErrorCode::DomainConflict,
            "mobile_cold_storage" => ErrorCode::UnsupportedCapability,
            "home_fridge" => ErrorCode::DigestMismatch,
            _ => unreachable!(),
        };
        assert!(
            report
                .other_checks
                .iter()
                .any(|conflict| conflict.code == expected
                    && conflict.recipe_refs.contains(&target.recipe)
                    && if edit == "method_roles" {
                        conflict.slot_ids.is_empty()
                    } else {
                        conflict.slot_ids.contains(&target.slot)
                    }),
            "{edit}: expected current portable-route refusal {expected:?}, got {report:?}"
        );
        assert_eq!(
            historical, before,
            "Fresh refusal preserves the historical approval"
        );
    }
}
