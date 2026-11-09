//! Exact authored cold-home packet and location-sensitive core controls.
//! Test EvidenceBasis construction attests this fixture only; it is not an
//! inbound runtime admission seam, product/temperature observation or cook test.
#![allow(clippy::result_large_err)]
use mstrchf_contracts::*;
use mstrchf_core::{availability, eligibility::*, food::*, portions, recipes::*};
use serde_json::Value;

const PACKET: &[u8] = include_bytes!(
    "../../../sites/mstrchf-9000/content/intake/three-bean-personal-reviewed-v1/packet.json"
);
const DOCUMENTS: &[u8] = include_bytes!(
    "../../../sites/mstrchf-9000/content/intake/three-bean-personal-reviewed-v1/evidence-documents.json"
);
const REGISTRY: &[u8] = include_bytes!(
    "../../../sites/mstrchf-9000/content/intake/three-bean-personal-reviewed-v1/ingredient-registry.json"
);
const CAPTURES: &[u8] = include_bytes!(
    "../../../sites/mstrchf-9000/content/intake/three-bean-personal-reviewed-v1/source-captures.json"
);
fn packet() -> Value {
    serde_json::from_slice(PACKET).unwrap()
}
fn salad() -> RecipeRevision {
    parse_document(&serde_json::to_vec(&packet()["maker_recipe_revision"]).unwrap()).unwrap()
}
fn reference(r: &RecipeRevision) -> RecipeRef {
    RecipeRef {
        recipe_id: r.recipe_id.clone(),
        revision: r.revision,
        digest: recipe_digest(r).unwrap(),
    }
}
fn inputs(r: &RecipeRevision) -> (EvidenceBasis, ComponentClassView) {
    let documents: Vec<Value> = serde_json::from_slice(DOCUMENTS).unwrap();
    let pins = documents
        .iter()
        .map(|d| EvidencePin {
            evidence_id: serde_json::from_value(d["evidence_id"].clone()).unwrap(),
            revision: serde_json::from_value(d["revision"].clone()).unwrap(),
            digest: canonical_digest(d).unwrap(),
        })
        .collect();
    let rf = reference(r);
    (
        EvidenceBasis::new(rf.clone(), pins, vec![]).unwrap(),
        ComponentClassView::new(rf, vec![]).unwrap(),
    )
}
fn meal() -> MealRequirements {
    let mut m = parse_document::<MealRequirementsDocumentV1>(include_bytes!(
        "../../../tests/fixtures/contracts/meal-requirements.json"
    ))
    .unwrap()
    .meal_requirements;
    m.slot.meal_type = MealType::Lunch;
    m.attendees.0.truncate(1);
    m.attendees[0].hard_requirements.0.clear();
    m.shared_hard_requirements.0.clear();
    m.equipment = BoundedVec(vec![EquipmentCode::Fridge]);
    m.mobility = Mobility {
        reheating: false,
        cold_storage: false,
        temperature: MobilityTemperature::Either,
    };
    m
}
fn fit(r: &RecipeRevision, m: &MealRequirements) -> Eligibility {
    let (basis, classes) = inputs(r);
    eligible(&assess_revision(r, &basis, &classes).unwrap(), m).unwrap()
}

#[test]
fn exact_packet_contracts_and_all_canonical_pins_are_bound() {
    let p = packet();
    let r = salad();
    let rf = reference(&r);
    assert_eq!(
        canonical_digest(&p).unwrap().0,
        "ec3894c5071806462224a3ba282467b159b8a84ad05df8ae12995489784802ac"
    );
    assert_eq!(serde_json::to_value(&rf).unwrap(), p["recipe_reference"]);
    assert_eq!(
        rf.digest.0,
        "247de874dc0348b4afdc06fe6b5aab31cdebf2869ace5098393d61dae9a80377"
    );
    for (field, bytes) in [
        ("ingredient_registry_digest", REGISTRY),
        ("source_captures_digest", CAPTURES),
        ("evidence_documents_digest", DOCUMENTS),
    ] {
        let v: Value = serde_json::from_slice(bytes).unwrap();
        assert_eq!(canonical_digest(&v).unwrap().0, p[field].as_str().unwrap());
    }
    let docs: Vec<Value> = serde_json::from_slice(DOCUMENTS).unwrap();
    assert_eq!(docs.len(), 6);
    for (doc, pin) in docs
        .iter()
        .zip(p["maker_evidence_pins"].as_array().unwrap())
    {
        assert_eq!(doc["evidence_id"], pin["evidence_id"]);
        assert_eq!(
            canonical_digest(doc).unwrap().0,
            pin["digest"].as_str().unwrap()
        );
        let at: Instant = serde_json::from_value(doc["at"].clone()).unwrap();
        at.validate_shape().unwrap();
        assert_eq!(at, Instant("2026-10-12T21:02:09Z".into()));
    }
    assert_eq!(docs[5]["recipe_reference"], p["recipe_reference"]);
    assert_eq!(
        docs[5]["accepted_ingredient_registry_digest"],
        p["ingredient_registry_digest"]
    );
    assert_eq!(
        docs[5]["accepts"]["operational_scope"],
        p["operational_scope"]
    );
    assert!(
        r.reviews
            .iter()
            .all(|v| v.actor == ReviewActor::Automated && v.kind != ReviewKind::CookTest)
    );
    let (basis, classes) = inputs(&r);
    let rr = assess_revision(&r, &basis, &classes).unwrap();
    assert!(rr.assessment().capabilities.local_display && rr.assessment().capabilities.shopping);
    assert!(
        !rr.assessment().capabilities.leftovers
            && !rr.assessment().capabilities.starter_distribution
    );
    assert!(
        r.image.0.is_none()
            && r.nutrition.0.is_none()
            && r.storage.is_empty()
            && r.reused_inputs.is_empty()
            && r.dietary_evidence.is_empty()
    );
    assert!(classes.components.is_empty());
}

#[test]
fn actual_explicit_home_fridge_resolves_cold_storage_without_travel_flags() {
    let r = salad();
    let mut m = meal();
    for temperature in [
        MobilityTemperature::Either,
        MobilityTemperature::Hot,
        MobilityTemperature::Cold,
    ] {
        m.mobility.temperature = temperature;
        assert_eq!(fit(&r, &m), Eligibility::Eligible);
    }
    m.equipment.0.clear();
    assert!(matches!(fit(&r, &m), Eligibility::Ineligible(_)));
    // An on-the-move setting cannot substitute for the actual home appliance.
    m.mobility.cold_storage = true;
    assert!(matches!(fit(&r, &m), Eligibility::Ineligible(_)));
    m.equipment = BoundedVec(vec![EquipmentCode::Fridge]);
    m.location = MealLocation::OnTheMove;
    assert!(matches!(fit(&r, &m), Eligibility::Unresolved(_)));
}

#[test]
fn home_fridge_does_not_manufacture_handling_or_dietary_evidence() {
    let mut r = salad();
    let mut m = meal();
    r.portability.evidence_ids.0.clear();
    assert!(matches!(fit(&r, &m), Eligibility::Unresolved(_)));
    r = salad();
    let vegan = HardRequirement::Diet {
        code: DietCode::Vegan,
    };
    m.attendees[0].hard_requirements = BoundedVec(vec![vegan.clone()]);
    m.shared_hard_requirements = BoundedVec(vec![vegan]);
    assert!(matches!(fit(&r, &m), Eligibility::Unresolved(_)));
    m.shared_hard_requirements = BoundedVec(vec![HardRequirement::IngredientExclusion {
        ingredient_id: r.ingredients[0].ingredient_id.clone(),
    }]);
    m.attendees[0].hard_requirements = m.shared_hard_requirements.clone();
    assert!(matches!(fit(&r, &m), Eligibility::Ineligible(_)));
    m = meal();
    r.portability.reheating_required = true;
    assert!(
        matches!(fit(&r, &m), Eligibility::Ineligible(_)),
        "Fridge cannot prove reheating"
    );
}

#[test]
fn explicit_home_fridge_and_hob_reheat_without_travel_access_or_temperature_claims() {
    // Synthetic variation for the equipment predicate only, not an admitted
    // replacement for the exact cold salad or evidence of physical reheating.
    let mut r = salad();
    r.equipment = BoundedVec(vec![EquipmentCode::Fridge, EquipmentCode::Hob]);
    r.portability.reheating_required = true;
    r.portability.cold_suitable = false;
    let mut m = meal();
    m.equipment = r.equipment.clone();
    m.mobility.temperature = MobilityTemperature::Cold;
    assert!(!m.mobility.reheating && !m.mobility.cold_storage);
    assert_eq!(fit(&r, &m), Eligibility::Eligible);

    for missing in [EquipmentCode::Fridge, EquipmentCode::Hob] {
        let mut absent = m.clone();
        absent.equipment.0.retain(|equipment| *equipment != missing);
        absent.mobility.reheating = true;
        absent.mobility.cold_storage = true;
        assert!(matches!(fit(&r, &absent), Eligibility::Ineligible(_)),
            "Travel access cannot replace missing declared home equipment");
    }
    let mut unproved = r.clone();
    unproved.portability.evidence_ids.0.clear();
    assert!(matches!(fit(&unproved, &m), Eligibility::Unresolved(_)));

    let mut undeclared = r.clone();
    undeclared.equipment = BoundedVec(vec![EquipmentCode::Fridge]);
    assert!(matches!(fit(&undeclared, &m), Eligibility::Ineligible(_)),
        "A household Hob alone does not declare a recipe reheating method");
    undeclared.equipment = BoundedVec(vec![EquipmentCode::Hob]);
    assert!(matches!(fit(&undeclared, &m), Eligibility::Ineligible(_)),
        "No-Fridge recipes retain their existing travel predicates");

    m.location = MealLocation::OnTheMove;
    assert!(matches!(fit(&r, &m), Eligibility::Ineligible(_)));
    m.mobility = Mobility { reheating:true, cold_storage:true, temperature:MobilityTemperature::Hot };
    assert!(matches!(fit(&r, &m), Eligibility::Unresolved(_)),
        "The home appliances do not qualify an equipped on-the-move route");
}

fn legacy_inputs(r: &RecipeRevision) -> (EvidenceBasis, ComponentClassView) {
    let rf = reference(r);
    let basis = EvidenceBasis::new(
        rf.clone(),
        (1..=32)
            .map(|n| EvidencePin {
                evidence_id: EvidenceId(format!("ev_{n:032x}")),
                revision: Revision(1),
                digest: Digest(format!("{n:064x}")),
            })
            .collect(),
        r.image.0.iter().map(|image| image.digest.clone()).collect(),
    )
    .unwrap();
    let classes = ComponentClassView::new(
        rf.clone(),
        r.components
            .iter()
            .map(|c| ComponentClassEvidence {
                recipe: rf.clone(),
                component_id: c.component_id.clone(),
                class: ComponentClass::General,
                evidence_ids: vec![EvidenceId(format!("ev_{:032x}", 8))],
            })
            .collect(),
    )
    .unwrap();
    (basis, classes)
}
fn legacy_fit(r: &RecipeRevision, m: &MealRequirements) -> Eligibility {
    let (b, c) = legacy_inputs(r);
    eligible(&assess_revision(r, &b, &c).unwrap(), m).unwrap()
}
#[test]
fn frozen_no_fridge_recipes_keep_their_original_travel_predicates() {
    for bytes in [include_bytes!("../../browser/assets/catalogue/engineering-v1/rc_00000000000000000000000000000004-r1.canonical.json").as_slice(),
                  include_bytes!("../../browser/assets/catalogue/engineering-v1/rc_00000000000000000000000000000005-r1.canonical.json").as_slice()] {
        let r:RecipeRevision=parse_document(bytes).unwrap();assert!(!r.equipment.contains(&EquipmentCode::Fridge));
        let mut m=meal();m.slot.meal_type=r.meal_types[0];m.equipment=r.equipment.clone();m.equipment.0.push(EquipmentCode::Fridge);
        m.equipment.0.sort_by_key(|equipment|serde_json::to_string(equipment).unwrap());
        assert!(matches!(legacy_fit(&r,&m),Eligibility::Unresolved(_)),"Adding household Fridge cannot alter old recipe proof");
        m.mobility.cold_storage=true;assert_eq!(legacy_fit(&r,&m),Eligibility::Eligible);
        m.location=MealLocation::OnTheMove;assert!(matches!(legacy_fit(&r,&m),Eligibility::Unresolved(_)));
    }
    let mut r: RecipeRevision = parse_document(include_bytes!(
        "../../../tests/fixtures/food/admission/ruling004-current--complete-synthetic.json"
    ))
    .unwrap();
    let mut m = meal();
    m.slot.meal_type = r.meal_types[0];
    m.equipment = r.equipment.clone();
    m.mobility.temperature = MobilityTemperature::Cold;
    m.mobility.cold_storage = true;
    r.portability.cold_suitable = false;
    r.portability.evidence_ids = BoundedVec(vec![EvidenceId(format!("ev_{:032x}", 8))]);
    assert!(matches!(legacy_fit(&r, &m), Eligibility::Ineligible(_)));
}

#[test]
fn exact_authored_quantities_and_only_one_adult_regular_unit() {
    let r = salad();
    let (b, c) = inputs(&r);
    let rr = assess_revision(&r, &b, &c).unwrap();
    let mut m = meal();
    let empty = MealOverrides {
        portions: BoundedVec(vec![]),
        eat_by: Nullable(None),
    };
    let resolved = portions::resolve_servings(&rr, &m, &empty).unwrap();
    assert_eq!(resolved[0].servings, PositiveRational::new(1, 1).unwrap());
    let scaled = portions::scale_recipe(&rr, resolved[0].servings).unwrap();
    let quantities: Vec<_> = scaled
        .ingredients
        .iter()
        .map(|v| {
            (
                v.quantity.amount.0.numerator.0,
                v.quantity.unit,
                v.quantity.form,
            )
        })
        .collect();
    assert_eq!(
        quantities,
        vec![
            (80, Unit::G, IngredientForm::Drained),
            (80, Unit::G, IngredientForm::Drained),
            (80, Unit::G, IngredientForm::Drained),
            (40, Unit::G, IngredientForm::Raw),
            (20, Unit::G, IngredientForm::Raw),
            (60, Unit::G, IngredientForm::Raw),
            (10, Unit::Ml, IngredientForm::Prepared),
            (15, Unit::Ml, IngredientForm::Prepared),
            (5, Unit::G, IngredientForm::Prepared)
        ]
    );
    assert_eq!(scaled.components[0].quantity.unit, Unit::Each);
    assert_eq!(scaled.components[0].quantity.form, IngredientForm::Prepared);
    assert!(
        r.ingredients
            .iter()
            .all(|v| v.conversion_evidence.is_empty())
    );
    assert!(portions::scale_recipe(&rr, PositiveRational::new(2, 1).unwrap()).is_err());
    m.attendees[0].age_band = AgeBand::Child4To6;
    assert!(portions::resolve_servings(&rr, &m, &empty).is_err());
    m.attendees[0].age_band = AgeBand::Adult;
    m.attendees[0].portion.preset = PortionPresetName::Larger;
    assert!(portions::resolve_servings(&rr, &m, &empty).is_err());
}

fn at(time: &str) -> ZonedMoment {
    availability::resolve_meal_time(
        &LocalDate("2026-12-28".into()),
        &LocalTime(time.into()),
        &ZoneId("Europe/London".into()),
        None,
    )
    .unwrap()
}
fn blank() -> PayloadDigest {
    PayloadDigest("0".repeat(64))
}
fn fixture() -> (SavedFoodBasis, FeasibilityInputs) {
    let r = salad();
    let rf = reference(&r);
    let (basis, classes) = inputs(&r);
    let mut context: PlanningContext = parse_document(include_bytes!(
        "../../../tests/fixtures/contracts/ruling004-current--planning-context.json"
    ))
    .unwrap();
    context.household.diners.0.truncate(1);
    context.household.diners[0].restrictions = RestrictionDeclaration::ConfirmedNone {
        confirmed_at: Instant("2026-12-01T12:00:00Z".into()),
    };
    context.household.preferences.equipment = r.equipment.clone();
    context.household.preferences.mobility.cold_storage = false;
    context.weeks.0.truncate(1);
    context.stamp.read_weeks.0.truncate(1);
    context.stamp.write_weeks.0.truncate(1);
    for cell in &mut context.weeks[0].cells.0 {
        cell.attendee_ids = BoundedVec(vec![context.household.diners[0].diner_id.clone()]);
        cell.occasion = Nullable(None);
    }
    context.weeks[0].plan.selections.0.clear();
    context.weeks[0].plan.batch_links.0.clear();
    context.requirements = BoundedVec(
        availability::meal_requirements(&context.household, &context.weeks[0], &[])
            .unwrap()
            .into(),
    );
    let m = context
        .requirements
        .iter()
        .find(|m| m.slot.date.0 == "2026-12-28" && m.slot.meal_type == MealType::Lunch)
        .unwrap();
    let slot = m.slot.clone();
    let overrides = MealOverrides {
        portions: BoundedVec(vec![]),
        eat_by: Nullable(Some(at("12:00:00"))),
    };
    let rr = assess_revision(&r, &basis, &classes).unwrap();
    let servings = portions::resolve_servings(&rr, m, &overrides).unwrap();
    let plan = WeekPlan {
        week_id: slot.week_id.clone(),
        revision: Revision(1),
        selections: BoundedVec(vec![MealSelection {
            portable_cold: None,
            slot: slot.clone(),
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
    context.recipes = BoundedVec(vec![r.clone()]);
    context.stamp.recipe_refs = BoundedVec(vec![rf.clone()]);
    context.weeks[0].plan = plan.clone();
    context.requirements = BoundedVec(
        availability::meal_requirements(&context.household, &context.weeks[0], &[])
            .unwrap()
            .into(),
    );
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
            prep_ids: vec![1, 2],
            direct_reuse_counts: vec![],
            evidence_ids: vec![EvidenceId("ev_80000000000000000000000000000024".into())],
        }],
        resources: (1..=2)
            .map(|prep_id| PhaseResources {
                recipe: rf.clone(),
                prep_id,
                equipment: vec![EquipmentCode::Fridge],
                evidence_ids: vec![EvidenceId("ev_80000000000000000000000000000023".into())],
            })
            .collect(),
        essential_ingredient_ids: vec![],
    };
    evidence.catalogue_digest = food_evidence_digest(&evidence).unwrap();
    let session = SessionKey {
        anchor_slot: slot.clone(),
        role: SessionRole::AdvancePrep,
        ordinal: 1,
    };
    let phase = |prep_id| PhaseKey {
        occurrence: OccurrenceKey {
            slot: slot.clone(),
            extra_id: None,
            recipe: rf.clone(),
        },
        prep_id,
    };
    let mut schedule = ScheduleInputs {
        layout: ScheduleLayout {
            routes: (1..=2)
                .map(|id| PhaseRoute {
                    phase: phase(id),
                    session: session.clone(),
                })
                .collect(),
            fixed_starts: vec![
                PhaseStartConstraint {
                    phase: phase(1),
                    start: at("10:45:00"),
                },
                PhaseStartConstraint {
                    phase: phase(2),
                    start: at("11:00:00"),
                },
            ],
            attachments: vec![],
        },
        occupied: OccupancySnapshot {
            query: Some(OccupancyQuery {
                horizon: TimeInterval {
                    start: at("10:45:00"),
                    end: at("12:00:00"),
                },
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
            .map(|(i, k)| (k, PrepSessionId(format!("ps_{:032x}", i + 1))))
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
        delivery: DeliveryInput::Unselected,
        products,
        confirmations: vec![],
        evaluated_at: Instant("2026-12-27T12:00:00Z".into()),
    };
    feasibility.digest = digest_feasibility_inputs(&feasibility).unwrap();
    (
        SavedFoodBasis {
            plan,
            context,
            evidence,
            identities,
            schedule,
        },
        feasibility,
    )
}
fn derive(s: &SavedFoodBasis, f: &FeasibilityInputs) -> Result<FoodDerivation, FoodFailure> {
    derive_plan(
        &s.plan,
        &s.context,
        &FoodInputs {
            evidence: &s.evidence,
            identities: &s.identities,
            schedule: &s.schedule,
            feasibility: f,
        },
    )
}
#[test]
fn two_phase_unheated_derivation_keeps_first_handling_demand_and_minimum_chill() {
    let (s, f) = fixture();
    let d = derive(&s, &f).unwrap();
    let session = &d.plan.prep_sessions[0];
    assert_eq!(session.start, at("10:45:00"));
    assert_eq!(session.end, at("12:00:00"));
    assert_eq!(session.ingredient_needed_at, at("10:45:00"));
    assert_eq!(session.tasks.len(), 2);
    assert_eq!(session.tasks[0].active_intervals.len(), 1);
    assert!(session.tasks[1].active_intervals.is_empty());
    assert_eq!(session.tasks[0].elapsed.end, at("11:00:00"));
    assert_eq!(session.tasks[1].elapsed.start, at("11:00:00"));
    assert_eq!(session.tasks[1].elapsed.end, at("12:00:00"));
    assert_eq!(
        d.plan.batches[0].components[0].original_production,
        ProductionMoment::Unheated {
            preparation_started_at: at("10:45:00")
        }
    );
    assert_eq!(d.plan.allocations[0].use_at, at("12:00:00"));
    assert_eq!(d.plan.allocations[0].planned_reheats, 0);
    assert!(d.plan.batches[0].components[0].storage_rule.0.is_none());
    assert_eq!(
        d.plan.demand.lines.len(),
        9,
        "Second chill phase cannot create a second purchase of ingredients"
    );
    for i in &s.context.recipes[0].ingredients.0 {
        let line = d
            .plan
            .demand
            .lines
            .iter()
            .find(|line| line.ingredient_id == i.ingredient_id)
            .unwrap();
        assert_eq!(Some(&line.quantity), i.quantity.0.as_ref());
    }
    assert_eq!(derive(&s, &f).unwrap(), d);
}

#[test]
fn cold_home_derivation_refuses_late_delivery_missing_fridge_and_shortened_chill() {
    let (s, mut f) = fixture();
    f.delivery = DeliveryInput::Proposed {
        start: at("10:30:00"),
        end: at("10:45:00"),
        revision: Revision(1),
    };
    f.digest = digest_feasibility_inputs(&f).unwrap();
    assert!(derive(&s, &f).is_ok());
    f.delivery = DeliveryInput::Proposed {
        start: at("10:30:00"),
        end: at("10:45:01"),
        revision: Revision(1),
    };
    f.digest = digest_feasibility_inputs(&f).unwrap();
    assert!(
        derive(&s, &f).is_err(),
        "Delivery after actual first handling cannot be accepted at chill end"
    );
    let (mut s, f) = fixture();
    s.context.household.preferences.equipment.0.clear();
    s.context.requirements = BoundedVec(
        availability::meal_requirements(&s.context.household, &s.context.weeks[0], &[])
            .unwrap()
            .into(),
    );
    assert!(derive(&s, &f).is_err());
    let (mut s, f) = fixture();
    s.schedule.layout.fixed_starts[1].start = at("11:00:01");
    assert!(
        derive(&s, &f).is_err(),
        "Pinned minimum chill cannot silently shrink to meet deadline"
    );
}

#[test]
fn source_bytes_and_both_existing_packets_are_unchanged() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../sites/mstrchf-9000/content/intake");
    let folder = root.join("three-bean-personal-reviewed-v1");
    let p: Value = serde_json::from_slice(
        &std::fs::read(folder.join("previous-packet-preservation.json")).unwrap(),
    )
    .unwrap();
    for (path, digest) in p["files"].as_object().unwrap() {
        assert_eq!(
            bytes_digest(&std::fs::read(root.join(path)).unwrap()).0,
            digest.as_str().unwrap()
        );
    }
    let captures: Value = serde_json::from_slice(CAPTURES).unwrap();
    for c in captures["captures"].as_array().unwrap() {
        let name = c["retained_file"].as_str().unwrap();
        let bytes = std::fs::read(folder.join(name)).unwrap();
        assert_eq!(bytes_digest(&bytes).0, c["body_sha256"].as_str().unwrap());
        assert_eq!(
            bytes,
            std::fs::read(root.join("three-bean-lunch-draft-v1").join(name)).unwrap()
        );
    }
}
