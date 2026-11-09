//! Separate output-validator controls and full admitted synthetic derivation tests.
// Test helpers preserve the required by-value R3 failure/report signature.
#![allow(clippy::result_large_err)]
use mstrchf_contracts::*;
use mstrchf_core::prep::*;
use mstrchf_core::{
    availability::meal_requirements, food::*, portions::resolve_servings, recipes::*,
};
#[path = "food_prep/review_r2.rs"]
mod review_r2;
fn at(date: &str, time: &str) -> ZonedMoment {
    ZonedMoment::new(
        LocalDate(date.into()),
        LocalTime(time.into()),
        ZoneId("UTC".into()),
        Resolution::Unambiguous,
        Instant(format!("{date}T{time}Z")),
    )
    .unwrap()
}
fn reference() -> RecipeRef {
    RecipeRef {
        recipe_id: RecipeId(format!("rc_{:032x}", 1)),
        revision: Revision(1),
        digest: RecipeDigest("a".repeat(64)),
    }
}
fn slot() -> SlotId {
    SlotId {
        week_id: WeekId(format!("wk_{:032x}", 1)),
        date: LocalDate("2026-10-05".into()),
        meal_type: MealType::Breakfast,
    }
}
fn sid(n: u32) -> PrepSessionId {
    PrepSessionId(format!("ps_{n:032x}"))
}
fn task(
    id: u16,
    start: ZonedMoment,
    end: ZonedMoment,
    active: bool,
    deps: Vec<PrepTaskRef>,
) -> ScheduledPrepTask {
    let elapsed = TimeInterval { start, end };
    ScheduledPrepTask {
        task_id: id,
        recipe: reference(),
        prep_id: id,
        depends_on: BoundedVec(deps),
        active_intervals: BoundedVec(if active {
            vec![elapsed.clone()]
        } else {
            vec![]
        }),
        elapsed,
    }
}
fn session(id: u32, tasks: Vec<ScheduledPrepTask>) -> PrepSession {
    let start = tasks
        .iter()
        .map(|t| t.elapsed.start.clone())
        .min_by(|a, b| a.instant.cmp(&b.instant))
        .unwrap();
    let end = tasks
        .iter()
        .map(|t| t.elapsed.end.clone())
        .max_by(|a, b| a.instant.cmp(&b.instant))
        .unwrap();
    PrepSession {
        session_id: sid(id),
        week_id: slot().week_id,
        linked_slots: BoundedVec(vec![slot()]),
        attached_to: Nullable(None),
        start: start.clone(),
        end,
        ingredient_needed_at: start,
        tasks: BoundedVec(tasks),
        ingredient_ids: BoundedVec(vec![]),
    }
}
#[test]
fn food_f25_sunday_standalone_timeline_validator() {
    let a = task(
        1,
        at("2026-10-04", "20:00:00"),
        at("2026-10-04", "20:10:00"),
        true,
        vec![],
    );
    let b = task(
        2,
        a.elapsed.end.clone(),
        at("2026-10-05", "08:00:00"),
        false,
        vec![PrepTaskRef {
            session_id: sid(1),
            task_id: 1,
        }],
    );
    let s = session(1, vec![a, b]);
    validate_timeline(std::slice::from_ref(&s)).unwrap();
    assert_eq!(s.start, at("2026-10-04", "20:00:00"));
    assert_eq!(s.end, at("2026-10-05", "08:00:00"));
    check_delivery(&at("2026-10-04", "19:00:00"), &s.ingredient_needed_at).unwrap();
}
#[test]
fn food_f26_attached_free_cook_interval_validator() {
    let target = session(
        1,
        vec![task(
            1,
            at("2026-10-04", "19:00:00"),
            at("2026-10-04", "21:00:00"),
            false,
            vec![],
        )],
    );
    let mut attached = session(
        2,
        vec![task(
            1,
            at("2026-10-04", "20:00:00"),
            at("2026-10-04", "20:10:00"),
            true,
            vec![],
        )],
    );
    attached.attached_to = Nullable(Some(sid(1)));
    validate_timeline(&[target.clone(), attached.clone()]).unwrap();
    let occupied = session(
        3,
        vec![task(
            1,
            at("2026-10-04", "20:05:00"),
            at("2026-10-04", "20:15:00"),
            true,
            vec![],
        )],
    );
    assert!(validate_timeline(&[target, attached, occupied]).is_err());
}
#[test]
fn food_f27_late_delivery_predicate_and_exact_equality() {
    let need = at("2026-10-04", "20:00:00");
    assert_eq!(
        check_delivery(&at("2026-10-04", "21:00:00"), &need)
            .unwrap_err()
            .code,
        ErrorCode::DomainConflict
    );
    check_delivery(&need, &need).unwrap();
    assert!(check_delivery(&at("2026-10-04", "20:00:01"), &need).is_err());
}
#[test]
fn food_f28_overlap_output_validator() {
    let make = |id, start, end, active, deps| {
        task(
            id,
            at("2026-10-05", start),
            at("2026-10-05", end),
            active,
            deps,
        )
    };
    let dep = |id| PrepTaskRef {
        session_id: sid(1),
        task_id: id,
    };
    let s = session(
        1,
        vec![
            make(1, "17:00:00", "17:10:00", true, vec![]),
            make(2, "17:10:00", "17:40:00", false, vec![dep(1)]),
            make(3, "17:10:00", "17:25:00", true, vec![]),
            make(4, "17:25:00", "17:30:00", false, vec![dep(3)]),
            make(5, "17:40:00", "17:50:00", true, vec![dep(2), dep(4)]),
        ],
    );
    validate_timeline(std::slice::from_ref(&s)).unwrap();
    assert_eq!(s.start, at("2026-10-05", "17:00:00"));
    assert_eq!(s.end, at("2026-10-05", "17:50:00"));
    let active: i64 = s
        .tasks
        .iter()
        .flat_map(|t| t.active_intervals.iter())
        .map(|i| {
            chrono::DateTime::parse_from_rfc3339(&i.end.instant.0)
                .unwrap()
                .timestamp()
                - chrono::DateTime::parse_from_rfc3339(&i.start.instant.0)
                    .unwrap()
                    .timestamp()
        })
        .sum();
    assert_eq!(active, 35 * 60);
}
#[test]
fn food_f29_unknown_product_date_is_unresolved_predicate() {
    assert_eq!(
        check_product_use(None, &at("2026-10-07", "18:00:00")).unwrap(),
        ProductDateAssessment::UnresolvedDateEvidence
    );
}
#[test]
fn food_f54_known_expired_product_date_conflicts_predicate() {
    assert_eq!(
        check_product_use(
            Some(&LocalDate("2026-10-06".into())),
            &at("2026-10-07", "18:00:00")
        )
        .unwrap(),
        ProductDateAssessment::ConflictingKnownLimit
    );
    assert_eq!(
        check_product_use(
            Some(&LocalDate("2026-10-07".into())),
            &at("2026-10-07", "18:00:00")
        )
        .unwrap(),
        ProductDateAssessment::WithinKnownLimit
    );
}
#[test]
fn food_cross_zone_and_gap_fold_are_explicit() {
    let utc = at("2026-10-25", "01:30:00");
    let london = ZonedMoment::new(
        LocalDate("2026-10-25".into()),
        LocalTime("01:30:00".into()),
        ZoneId("Europe/London".into()),
        Resolution::Later,
        utc.instant.clone(),
    )
    .unwrap();
    check_delivery(&utc, &london).unwrap();
    assert!(
        ZonedMoment::new(
            LocalDate("2026-03-29".into()),
            LocalTime("01:30:00".into()),
            ZoneId("Europe/London".into()),
            Resolution::Unambiguous,
            Instant("2026-03-29T01:30:00Z".into())
        )
        .is_err()
    );
    let earlier = ZonedMoment::new(
        LocalDate("2026-10-25".into()),
        LocalTime("01:30:00".into()),
        ZoneId("Europe/London".into()),
        Resolution::Earlier,
        Instant("2026-10-25T00:30:00Z".into()),
    )
    .unwrap();
    assert!(check_delivery(&london, &earlier).is_err());
    let mut forged = london;
    forged.resolution = Resolution::Unambiguous;
    assert!(check_product_use(None, &forged).is_err());
}
#[test]
fn food_timeline_missing_duplicate_dependency_and_cycle_reject() {
    let a = task(
        1,
        at("2026-10-05", "17:00:00"),
        at("2026-10-05", "17:10:00"),
        true,
        vec![],
    );
    let mut s = session(1, vec![a.clone()]);
    s.tasks[0].depends_on = BoundedVec(vec![PrepTaskRef {
        session_id: sid(9),
        task_id: 1,
    }]);
    assert!(validate_timeline(&[s]).is_err());
    assert!(validate_timeline(&[session(1, vec![a.clone(), a])]).is_err());
    let zero = at("2026-10-05", "17:00:00");
    let x = task(
        1,
        zero.clone(),
        zero.clone(),
        false,
        vec![PrepTaskRef {
            session_id: sid(1),
            task_id: 2,
        }],
    );
    let y = task(
        2,
        zero.clone(),
        zero,
        false,
        vec![PrepTaskRef {
            session_id: sid(1),
            task_id: 1,
        }],
    );
    assert!(validate_timeline(&[session(1, vec![x, y])]).is_err());
}
#[test]
fn food_timeline_rejects_wrong_session_envelope() {
    let mut s = session(
        1,
        vec![task(
            1,
            at("2026-10-05", "17:00:00"),
            at("2026-10-05", "17:10:00"),
            true,
            vec![],
        )],
    );
    s.end = at("2026-10-05", "17:11:00");
    assert!(validate_timeline(&[s]).is_err());
}

struct Fixture {
    plan: WeekPlan,
    context: PlanningContext,
    evidence: FoodEvidence,
    identities: FoodIdentities,
    schedule: ScheduleInputs,
    feasibility: FeasibilityInputs,
}

pub fn run_oracle_case(id: u8) {
    match id {
        25 => {
            food_f25_sunday_standalone_timeline_validator();
            food_f25_full_derivation_keeps_sunday_start();
        }
        26 => {
            food_f26_attached_free_cook_interval_validator();
            food_f26_full_derivation_attaches_only_in_free_cook_time();
        }
        27 => food_f27_late_delivery_predicate_and_exact_equality(),
        28 => {
            food_f28_overlap_output_validator();
            food_f28_full_derivation_has_literal_overlap_and_slack();
        }
        29 => food_f29_unknown_product_date_is_unresolved_predicate(),
        54 => food_f54_known_expired_product_date_conflicts_predicate(),
        _ => panic!("D does not own F{id:02}"),
    }
}
fn pin(n: u32) -> EvidenceId {
    EvidenceId(format!("ev_{n:032x}"))
}
fn blank() -> PayloadDigest {
    PayloadDigest("0".repeat(64))
}
impl Fixture {
    fn derive(&self) -> Result<FoodDerivation, FoodFailure> {
        derive_plan(
            &self.plan,
            &self.context,
            &FoodInputs {
                evidence: &self.evidence,
                identities: &self.identities,
                schedule: &self.schedule,
                feasibility: &self.feasibility,
            },
        )
    }
    fn rehash_occupancy(&mut self) {
        for o in &mut self.schedule.occupied.owners {
            o.projection_digest = occupied_owner_digest(o).unwrap();
        }
        self.schedule.occupied.digest = occupancy_snapshot_digest(&self.schedule.occupied).unwrap();
    }
}
// Full A-admitted recipe, B-resolved servings and A3-generated requirements.
fn derivation_fixture(overnight: bool) -> Fixture {
    let mut recipe: RecipeRevision = parse_document(include_bytes!(
        "../../../tests/fixtures/food/admission/ruling004-current--complete-synthetic.json"
    ))
    .unwrap();
    recipe.scaling = ScalingPolicy::Linear {
        minimum: PositiveRational::new(1, 100).unwrap(),
        maximum: PositiveRational::new(100, 1).unwrap(),
    };
    let meal_type = if overnight {
        MealType::Breakfast
    } else {
        MealType::Dinner
    };
    recipe.meal_types = BoundedVec(vec![meal_type]);
    recipe.portion_presets[0].meal_type = meal_type;
    recipe.equipment = BoundedVec(if overnight {
        vec![EquipmentCode::Fridge]
    } else {
        vec![EquipmentCode::Hob, EquipmentCode::Oven]
    });
    let durations: Vec<(u16, u16, Vec<u16>)> = if overnight {
        vec![(10, 10, vec![]), (0, 710, vec![1])]
    } else {
        vec![
            (10, 10, vec![]),
            (0, 30, vec![1]),
            (15, 15, vec![]),
            (0, 5, vec![3]),
            (10, 10, vec![2, 4]),
        ]
    };
    let ingredient = recipe.ingredients[0].ingredient_id.clone();
    let component = recipe.components[0].component_id.clone();
    recipe.steps = BoundedVec(
        durations
            .iter()
            .enumerate()
            .map(|(i, (a, e, d))| InstructionStep {
                step_id: i as u16 + 1,
                text: Text(format!("SYNTHETIC reviewed atomic phase {}", i + 1)),
                ingredient_ids: BoundedVec(if i == 0 {
                    vec![ingredient.clone()]
                } else {
                    vec![]
                }),
                component_ids: BoundedVec(vec![component.clone()]),
                active_minutes: *a,
                elapsed_minutes: *e,
                depends_on: BoundedVec(d.clone()),
            })
            .collect(),
    );
    recipe.prep = BoundedVec(
        durations
            .iter()
            .enumerate()
            .map(|(i, (a, e, d))| PrepSpec {
                prep_id: i as u16 + 1,
                title: Title(format!("SYNTHETIC phase {}", i + 1)),
                step_ids: BoundedVec(vec![i as u16 + 1]),
                depends_on: BoundedVec(d.clone()),
                active_minutes: *a,
                elapsed_minutes: *e,
                earliest_offset_seconds: if overnight { -43200 } else { -3600 },
                latest_offset_seconds: 0,
                ingredient_ids: BoundedVec(if i == 0 {
                    vec![ingredient.clone()]
                } else {
                    vec![]
                }),
                can_attach_to_prior_session: true,
            })
            .collect(),
    );
    recipe.active_minutes = if overnight { 10 } else { 35 };
    recipe.elapsed_minutes = if overnight { 720 } else { 50 };
    recipe.components[0].production = Nullable(Some(if overnight {
        ComponentProductionSpec::Unheated {
            first_handling_prep_id: 1,
        }
    } else {
        ComponentProductionSpec::Cooked {
            completion_prep_id: 5,
        }
    }));
    let rf = RecipeRef {
        recipe_id: recipe.recipe_id.clone(),
        revision: recipe.revision,
        digest: recipe_digest(&recipe).unwrap(),
    };
    let basis = EvidenceBasis::new(
        rf.clone(),
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
        rf.clone(),
        vec![ComponentClassEvidence {
            recipe: rf.clone(),
            component_id: component,
            class: ComponentClass::General,
            evidence_ids: vec![pin(8)],
        }],
    )
    .unwrap();
    let reviewed = assess_revision(&recipe, &basis, &classes).unwrap();
    let mut context: PlanningContext = parse_document(include_bytes!(
        "../../../tests/fixtures/contracts/ruling004-current--planning-context.json"
    ))
    .unwrap();
    context.household.diners.0.truncate(1);
    context.household.preferences.equipment = recipe.equipment.clone();
    context.weeks.0.truncate(1);
    context.stamp.read_weeks.0.truncate(1);
    context.stamp.write_weeks.0.truncate(1);
    for cell in context.weeks[0].cells.0.iter_mut() {
        cell.attendee_ids = BoundedVec(vec![context.household.diners[0].diner_id.clone()]);
        cell.occasion = Nullable(None);
    }
    context.weeks[0].plan.selections.0.clear();
    context.weeks[0].plan.batch_links.0.clear();
    context.requirements = BoundedVec(
        meal_requirements(&context.household, &context.weeks[0], &[])
            .unwrap()
            .to_vec(),
    );
    let meal = context
        .requirements
        .iter()
        .find(|m| m.slot.meal_type == meal_type && m.slot.date.0 == "2026-12-28")
        .unwrap();
    let selected_slot = meal.slot.clone();
    let overrides = MealOverrides {
        portions: BoundedVec(vec![]),
        eat_by: Nullable(None),
    };
    let servings = resolve_servings(&reviewed, meal, &overrides).unwrap();
    let plan = WeekPlan {
        week_id: selected_slot.week_id.clone(),
        revision: Revision(1),
        selections: BoundedVec(vec![MealSelection {
            portable_cold: None,
            slot: selected_slot.clone(),
            recipe: rf.clone(),
            origin: SelectionOrigin::Fixed,
            repeat_authorised: false,
            servings: BoundedVec(servings),
            extras: BoundedVec(vec![]),
            batch_use: BatchUse::NewBatch {
                batch_id: BatchId(format!("ba_{:032x}", 1)),
                production_servings: PositiveRational::new(1, 1).unwrap(),
            },
            meal_overrides: overrides,
        }]),
        batch_links: BoundedVec(vec![]),
    };
    context.stamp.recipe_refs = BoundedVec(vec![rf.clone()]);
    context.recipes = BoundedVec(vec![recipe.clone()]);
    context.validate_shape().unwrap();
    let mut evidence = FoodEvidence {
        catalogue_revision: context.stamp.catalogue_revision,
        catalogue_digest: blank(),
        policy_version: Revision(1),
        recipes: vec![basis],
        classes: vec![classes],
        methods: vec![MethodPhases {
            portable_cold: None,
            recipe: rf.clone(),
            kind: MethodKind::Production,
            prep_ids: (1..=durations.len() as u16).collect(),
            direct_reuse_counts: vec![],
            evidence_ids: vec![pin(8)],
        }],
        resources: (1..=durations.len() as u16)
            .map(|i| PhaseResources {
                recipe: rf.clone(),
                prep_id: i,
                equipment: if overnight {
                    if i == 2 {
                        vec![EquipmentCode::Fridge]
                    } else {
                        vec![]
                    }
                } else {
                    match i {
                        1 | 2 => vec![EquipmentCode::Oven],
                        3 | 4 => vec![EquipmentCode::Hob],
                        _ => vec![],
                    }
                },
                evidence_ids: vec![pin(8)],
            })
            .collect(),
        essential_ingredient_ids: vec![],
    };
    evidence.catalogue_digest = food_evidence_digest(&evidence).unwrap();
    let key = SessionKey {
        anchor_slot: selected_slot.clone(),
        role: if overnight {
            SessionRole::AdvancePrep
        } else {
            SessionRole::Cooking
        },
        ordinal: 1,
    };
    let phase = |i| PhaseKey {
        occurrence: OccurrenceKey {
            slot: selected_slot.clone(),
            extra_id: None,
            recipe: rf.clone(),
        },
        prep_id: i,
    };
    let start = if overnight {
        at("2026-12-27", "20:00:00")
    } else {
        at("2026-12-28", "17:00:00")
    };
    let end = at(
        "2026-12-28",
        if overnight { "08:00:00" } else { "18:00:00" },
    );
    let mut schedule = ScheduleInputs {
        layout: ScheduleLayout {
            routes: (1..=durations.len() as u16)
                .map(|i| PhaseRoute {
                    phase: phase(i),
                    session: key.clone(),
                })
                .collect(),
            fixed_starts: vec![PhaseStartConstraint {
                phase: phase(1),
                start: start.clone(),
            }],
            attachments: vec![],
        },
        occupied: OccupancySnapshot {
            query: Some(OccupancyQuery {
                horizon: TimeInterval { start, end },
                excluded_subject: None,
            }),
            occupancy_epoch: Revision(1),
            owners: vec![],
            digest: blank(),
        },
    };
    schedule.occupied.digest = occupancy_snapshot_digest(&schedule.occupied).unwrap();
    let keys = required_identity_keys(&plan, &context, &evidence, &schedule).unwrap();
    let mut identities = FoodIdentities {
        revision: Revision(1),
        digest: blank(),
        allocations: keys
            .allocations
            .into_iter()
            .enumerate()
            .map(|(i, k)| (k, AllocationId(format!("al_{:032x}", i + 1))))
            .collect(),
        sessions: keys
            .sessions
            .into_iter()
            .enumerate()
            .map(|(i, k)| (k, sid(i as u32 + 1)))
            .collect(),
        demand_lines: keys
            .demand_lines
            .into_iter()
            .enumerate()
            .map(|(i, k)| (k, DemandLineId(format!("dl_{:032x}", i + 1))))
            .collect(),
    };
    identities.digest = food_identities_digest(&identities).unwrap();
    let mut products = ProductUseSnapshot {
        revision: Revision(1),
        digest: blank(),
        valid_until: None,
        evidence: vec![],
        bindings: vec![],
        facts: vec![],
    };
    products.digest = digest_product_uses(&products).unwrap();
    let mut feasibility = FeasibilityInputs {
        revision: Revision(1),
        digest: blank(),
        delivery: DeliveryInput::Proposed {
            start: at("2026-12-27", "18:00:00"),
            end: at("2026-12-27", "19:00:00"),
            revision: Revision(1),
        },
        products,
        confirmations: vec![],
        evaluated_at: Instant("2026-12-27T12:00:00Z".into()),
    };
    feasibility.digest = digest_feasibility_inputs(&feasibility).unwrap();
    Fixture {
        plan,
        context,
        evidence,
        identities,
        schedule,
        feasibility,
    }
}
#[test]
fn food_f28_full_derivation_has_literal_overlap_and_slack() {
    let f = derivation_fixture(false);
    let d = f.derive().unwrap();
    let s = &d.plan.prep_sessions[0];
    assert_eq!(s.start.instant, Instant("2026-12-28T17:00:00Z".into()));
    assert_eq!(s.end.instant, Instant("2026-12-28T17:50:00Z".into()));
    assert_eq!(s.tasks.len(), 5);
    assert_eq!(d.feasibility.warnings.len(), 1);
    assert!(d.feasibility.blocking.is_empty());
}
#[test]
fn food_f25_full_derivation_keeps_sunday_start() {
    let f = derivation_fixture(true);
    let d = f.derive().unwrap();
    let s = &d.plan.prep_sessions[0];
    assert_eq!(s.start.instant, Instant("2026-12-27T20:00:00Z".into()));
    assert_eq!(s.end.instant, Instant("2026-12-28T08:00:00Z".into()));
    assert_eq!(s.tasks.len(), 2);
}
#[test]
fn food_schedule_reconstructs_closure_and_rejects_output_tampering() {
    let f = derivation_fixture(false);
    let d = f.derive().unwrap();
    let check = |s: &[PrepSession]| {
        validate_schedule(
            &f.plan,
            &f.context,
            &f.evidence,
            &f.identities,
            &f.schedule,
            s,
        )
    };
    assert_eq!(check(&d.plan.prep_sessions).unwrap().len(), 5);
    let mut s = d.plan.prep_sessions.0.clone();
    s[0].tasks[1].depends_on.0.clear();
    assert!(check(&s).is_err());
    let mut s = d.plan.prep_sessions.0.clone();
    s[0].ingredient_ids.0.clear();
    assert!(check(&s).is_err());
    let mut s = d.plan.prep_sessions.0.clone();
    s[0].tasks[0].prep_id = 3;
    assert!(check(&s).is_err());
}
#[test]
fn food_schedule_rejects_removed_anchor_and_short_horizon() {
    let mut f = derivation_fixture(false);
    let d = f.derive().unwrap();
    f.schedule.layout.routes[0].session.anchor_slot.date = LocalDate("2026-12-29".into());
    assert!(
        validate_schedule(
            &f.plan,
            &f.context,
            &f.evidence,
            &f.identities,
            &f.schedule,
            &d.plan.prep_sessions
        )
        .is_err()
    );
    let mut f = derivation_fixture(false);
    f.schedule.occupied.query.as_mut().unwrap().horizon.start = at("2026-12-28", "17:01:00");
    f.rehash_occupancy();
    assert!(f.derive().is_err());
}

// Native retained projection premises are synthetic store inputs. These tests
// prove supplied occupancy closure/collisions, not the producer's query completeness.
fn prior_owner(f: &Fixture, active: bool, equipment: Vec<EquipmentCode>) -> OccupiedOwner {
    let mut s = session(
        10,
        vec![task(
            1,
            at("2026-12-27", "19:00:00"),
            at("2026-12-27", "21:00:00"),
            active,
            vec![],
        )],
    );
    s.week_id = WeekId(format!("wk_{:032x}", 2));
    s.linked_slots[0] = f.plan.selections[0].slot.clone();
    s.linked_slots[0].week_id = s.week_id.clone();
    OccupiedOwner {
        pin: PlanOwnerPin {
            week: WeekRevision {
                week_id: s.week_id.clone(),
                revision: Revision(1),
            },
            derived_digest: PayloadDigest("b".repeat(64)),
            basis_digest: PayloadDigest("c".repeat(64)),
        },
        sessions: vec![s],
        task_resources: vec![OccupiedTaskResources {
            task: PrepTaskRef {
                session_id: sid(10),
                task_id: 1,
            },
            equipment,
            evidence_ids: vec![pin(8)],
        }],
        projection_digest: blank(),
    }
}
#[test]
fn food_f26_full_derivation_attaches_only_in_free_cook_time() {
    let mut f = derivation_fixture(true);
    let owner = prior_owner(&f, false, vec![]);
    f.schedule.layout.attachments.push(AttachmentChoice {
        session: f.identities.sessions[0].0.clone(),
        target_owner: owner.pin.clone(),
        target_session: sid(10),
    });
    f.schedule.occupied.owners.push(owner);
    f.rehash_occupancy();
    let d = f.derive().unwrap();
    assert_eq!(d.plan.prep_sessions[0].attached_to.0, Some(sid(10)));
    assert_eq!(
        d.plan.prep_sessions[0].end.instant,
        Instant("2026-12-28T08:00:00Z".into())
    );
    let mut busy = session(
        11,
        vec![task(
            1,
            at("2026-12-27", "20:05:00"),
            at("2026-12-27", "20:15:00"),
            true,
            vec![],
        )],
    );
    busy.week_id = f.schedule.occupied.owners[0].pin.week.week_id.clone();
    busy.linked_slots[0] = f.schedule.occupied.owners[0].sessions[0].linked_slots[0].clone();
    f.schedule.occupied.owners[0].sessions.push(busy);
    f.schedule.occupied.owners[0]
        .task_resources
        .push(OccupiedTaskResources {
            task: PrepTaskRef {
                session_id: sid(11),
                task_id: 1,
            },
            equipment: vec![],
            evidence_ids: vec![pin(8)],
        });
    f.rehash_occupancy();
    assert!(f.derive().is_err());
}
#[test]
fn food_prior_unattended_equipment_blocks_and_exact_adjacency_passes() {
    let mut f = derivation_fixture(true);
    let owner = prior_owner(&f, false, vec![EquipmentCode::Fridge]);
    f.schedule.occupied.owners.push(owner);
    f.rehash_occupancy();
    assert!(f.derive().is_err());
    let boundary = at("2026-12-27", "20:10:00");
    let s = &mut f.schedule.occupied.owners[0].sessions[0];
    s.end = boundary.clone();
    s.tasks[0].elapsed.end = boundary;
    f.rehash_occupancy();
    assert!(f.derive().is_ok());
}
#[test]
fn food_occupancy_stale_digest_missing_resources_and_wrong_owner_reject() {
    let mut f = derivation_fixture(true);
    let owner = prior_owner(&f, false, vec![]);
    f.schedule.occupied.owners.push(owner);
    f.rehash_occupancy();
    assert!(f.derive().is_ok());
    f.schedule.occupied.occupancy_epoch = Revision(2);
    assert!(f.derive().is_err());
    f.rehash_occupancy();
    f.schedule.occupied.owners[0].task_resources.clear();
    assert!(occupied_owner_digest(&f.schedule.occupied.owners[0]).is_err());
    let mut f = derivation_fixture(true);
    let mut owner = prior_owner(&f, false, vec![]);
    owner.sessions[0].week_id = WeekId(format!("wk_{:032x}", 3));
    assert!(occupied_owner_digest(&owner).is_err());
    f.schedule.occupied.owners.push(owner);
    assert!(f.derive().is_err());
}
#[test]
fn food_full_derivation_preserves_every_task_after_missing_resource_evidence() {
    let mut f = derivation_fixture(false);
    f.evidence.resources.pop();
    f.evidence.catalogue_digest = food_evidence_digest(&f.evidence).unwrap();
    assert!(f.derive().is_err());
}

fn extra_fixture() -> Fixture {
    let mut f = derivation_fixture(false);
    let mut extra = f.context.recipes[0].clone();
    extra.recipe_id = RecipeId(format!("rc_{:032x}", 2));
    extra.steps.0.truncate(1);
    extra.prep.0.truncate(1);
    extra.steps[0].active_minutes = 1;
    extra.steps[0].elapsed_minutes = 1;
    extra.prep[0].active_minutes = 1;
    extra.prep[0].elapsed_minutes = 1;
    extra.active_minutes = 1;
    extra.elapsed_minutes = 1;
    extra.equipment = BoundedVec(vec![]);
    extra.components[0].production = Nullable(Some(ComponentProductionSpec::Cooked {
        completion_prep_id: 1,
    }));
    let er = RecipeRef {
        recipe_id: extra.recipe_id.clone(),
        revision: extra.revision,
        digest: recipe_digest(&extra).unwrap(),
    };
    let extras: Vec<_> = (1..=2)
        .map(|i| ExtraSpec {
            extra_id: ExtraId(format!("ex_{i:032x}")),
            title: Title(format!("Synthetic extra {i}")),
            recipe: er.clone(),
            eligible_person_requirements: BoundedVec(vec![]),
            supported_with: BoundedVec(vec![
                f.context.recipes[0].components[0].component_id.clone(),
            ]),
        })
        .collect();
    f.context.recipes[0].extras = BoundedVec(extras.clone());
    let sr = RecipeRef {
        recipe_id: f.context.recipes[0].recipe_id.clone(),
        revision: f.context.recipes[0].revision,
        digest: recipe_digest(&f.context.recipes[0]).unwrap(),
    };
    f.plan.selections[0].recipe = sr.clone();
    for s in f.plan.selections[0].servings.0.iter_mut() {
        s.preset_ref.recipe = sr.clone();
    }
    f.evidence.recipes[0].recipe = sr.clone();
    f.evidence.classes[0].recipe = sr.clone();
    for c in &mut f.evidence.classes[0].components {
        c.recipe = sr.clone();
    }
    f.evidence.methods[0].recipe = sr.clone();
    for r in &mut f.evidence.resources {
        r.recipe = sr.clone();
    }
    for r in &mut f.schedule.layout.routes {
        r.phase.occurrence.recipe = sr.clone();
    }
    for r in &mut f.schedule.layout.fixed_starts {
        r.phase.occurrence.recipe = sr.clone();
    }
    let mut eb = f.evidence.recipes[0].clone();
    eb.recipe = er.clone();
    let mut ec = f.evidence.classes[0].clone();
    ec.recipe = er.clone();
    for c in &mut ec.components {
        c.recipe = er.clone();
    }
    let admitted = assess_revision(&extra, &eb, &ec).unwrap();
    let meal = f
        .context
        .requirements
        .iter()
        .find(|m| m.slot == f.plan.selections[0].slot)
        .unwrap();
    let recipients =
        resolve_servings(&admitted, meal, &f.plan.selections[0].meal_overrides).unwrap();
    f.plan.selections[0].extras = BoundedVec(
        extras
            .iter()
            .map(|e| ExtraSelection {
                extra_id: e.extra_id.clone(),
                recipe: er.clone(),
                recipients: BoundedVec(recipients.clone()),
            })
            .collect(),
    );
    f.context.recipes.0.push(extra);
    f.context.stamp.recipe_refs = BoundedVec(vec![sr, er.clone()]);
    f.evidence.recipes.push(eb);
    f.evidence.classes.push(ec);
    f.evidence.methods.push(MethodPhases {
        portable_cold: None,
        recipe: er.clone(),
        kind: MethodKind::Production,
        prep_ids: vec![1],
        direct_reuse_counts: vec![],
        evidence_ids: vec![pin(8)],
    });
    f.evidence.resources.push(PhaseResources {
        recipe: er.clone(),
        prep_id: 1,
        equipment: vec![],
        evidence_ids: vec![pin(8)],
    });
    f.evidence.catalogue_digest = food_evidence_digest(&f.evidence).unwrap();
    for (i, e) in extras.iter().enumerate() {
        let phase = PhaseKey {
            occurrence: OccurrenceKey {
                slot: f.plan.selections[0].slot.clone(),
                extra_id: Some(e.extra_id.clone()),
                recipe: er.clone(),
            },
            prep_id: 1,
        };
        f.schedule.layout.routes.push(PhaseRoute {
            phase: phase.clone(),
            session: f.identities.sessions[0].0.clone(),
        });
        f.schedule.layout.fixed_starts.push(PhaseStartConstraint {
            phase,
            start: at("2026-12-28", if i == 0 { "17:50:00" } else { "17:51:00" }),
        });
    }
    let keys = required_identity_keys(&f.plan, &f.context, &f.evidence, &f.schedule).unwrap();
    f.identities.allocations = keys
        .allocations
        .into_iter()
        .enumerate()
        .map(|(i, k)| (k, AllocationId(format!("al_{:032x}", i + 1))))
        .collect();
    f.identities.demand_lines = keys
        .demand_lines
        .into_iter()
        .enumerate()
        .map(|(i, k)| (k, DemandLineId(format!("dl_{:032x}", i + 1))))
        .collect();
    f.identities.digest = food_identities_digest(&f.identities).unwrap();
    f
}
#[test]
fn food_same_recipe_extras_have_distinct_phase_rank_and_fixed_time() {
    let f = extra_fixture();
    let d = f.derive().unwrap();
    let map = validate_schedule(
        &f.plan,
        &f.context,
        &f.evidence,
        &f.identities,
        &f.schedule,
        &d.plan.prep_sessions,
    )
    .unwrap();
    let extras: Vec<_> = map
        .iter()
        .filter(|m| m.phase.occurrence.extra_id.is_some())
        .collect();
    assert_eq!(extras.len(), 2);
    assert_eq!(extras[0].task.task_id, 6);
    assert_eq!(extras[1].task.task_id, 7);
    let mut swapped = d.plan.prep_sessions.0.clone();
    let a = swapped[0].tasks[5].elapsed.clone();
    swapped[0].tasks[5].elapsed = swapped[0].tasks[6].elapsed.clone();
    swapped[0].tasks[5].active_intervals = BoundedVec(vec![swapped[0].tasks[5].elapsed.clone()]);
    swapped[0].tasks[6].elapsed = a.clone();
    swapped[0].tasks[6].active_intervals = BoundedVec(vec![a]);
    assert!(
        validate_schedule(
            &f.plan,
            &f.context,
            &f.evidence,
            &f.identities,
            &f.schedule,
            &swapped
        )
        .is_err()
    );
}
#[test]
fn food_layout_native_permutations_and_duplicate_extra_reject() {
    let mut f = extra_fixture();
    f.schedule.layout.routes.swap(5, 6);
    assert!(f.derive().is_err());
    let mut f = extra_fixture();
    f.plan.selections[0].extras[1].extra_id = f.plan.selections[0].extras[0].extra_id.clone();
    assert!(f.derive().is_err());
}
#[test]
fn food_layout_4097_routes_and_257_identity_sessions_fail_bounds() {
    let mut f = derivation_fixture(false);
    let route = f.schedule.layout.routes[0].clone();
    f.schedule.layout.routes = vec![route; 4097];
    assert!(required_identity_keys(&f.plan, &f.context, &f.evidence, &f.schedule).is_err());
    let mut f = derivation_fixture(false);
    f.identities.sessions = vec![f.identities.sessions[0].clone(); 257];
    assert_eq!(
        food_identities_digest(&f.identities).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
}
