//! Isolated typed feasibility probe, NOT food admission or a reviewed recipe.
//! Published six quantified inputs/servings are read from the retained intake.
//! Unspecified stock, water, seasoning and greasing are deliberately omitted
//! from this SYNTHETIC mechanical model. Timings, component units, class,
//! safe-cooling/chilling and cold suitability below are explicit TEST PREMISES,
//! never human cooking/calibration, actual transport or product evidence.
use mstrchf_contracts::*;
use mstrchf_core::{availability, food::*, recipes::*};
use serde_json::{Value, json};

#[path = "../../../../tests/fixtures/food/batches/f11_fixture.rs"]
mod f11;
pub(super) const INTAKE: &[u8] = include_bytes!(
    "../../../../sites/mstrchf-9000/content/intake/2026-10-12-starter-source-candidates.json"
);
pub(super) const INTAKE_HASH: &str =
    "65176d9d94ae13ac202ca59f36b948fd98480dff273c8fe1d1d8aa08286295f4";
pub(super) const SOURCE_URL: &str =
    "https://en.wikibooks.org/w/index.php?title=Cookbook:Zucchini_Pasta_Bake&oldid=4525784";
pub(super) fn p(n: u64) -> PositiveRational {
    PositiveRational::new(n, 1).unwrap()
}
pub(super) fn units(n: u64) -> Quantity {
    Quantity {
        amount: p(n),
        unit: Unit::Each,
        form: IngredientForm::Cooked,
    }
}
pub(super) fn eid(n: u32) -> EvidenceId {
    EvidenceId(format!("ev_{:032x}", 0xf000 + n))
}
pub(super) fn reference(recipe: &RecipeRevision) -> RecipeRef {
    RecipeRef {
        recipe_id: recipe.recipe_id.clone(),
        revision: recipe.revision,
        digest: recipe_digest(recipe).unwrap(),
    }
}
pub(super) fn at(day: u8, time: &str) -> ZonedMoment {
    f11::at(day, time)
}
pub(super) fn shifted(moment: &ZonedMoment, seconds: i64) -> ZonedMoment {
    let utc = chrono::DateTime::parse_from_rfc3339(&moment.instant.0).unwrap()
        + chrono::Duration::seconds(seconds);
    let tz: chrono_tz::Tz = moment.zone.0.parse().unwrap();
    let local = utc.with_timezone(&tz);
    use chrono::TimeZone;
    let resolution = match tz.from_local_datetime(&local.naive_local()) {
        chrono::LocalResult::Single(_) => Resolution::Unambiguous,
        chrono::LocalResult::Ambiguous(a, b) => {
            if local == a.min(b) {
                Resolution::Earlier
            } else {
                Resolution::Later
            }
        }
        chrono::LocalResult::None => panic!("An instant never maps to a missing local time"),
    };
    ZonedMoment::new(
        LocalDate(local.format("%Y-%m-%d").to_string()),
        LocalTime(local.format("%H:%M:%S").to_string()),
        moment.zone.clone(),
        resolution,
        Instant(utc.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
    )
    .unwrap()
}
pub(super) fn premise_basis(recipe: &RecipeRevision) -> (EvidenceBasis, ComponentClassView) {
    let rf = reference(recipe);
    let pins=(1..=5).map(|n|EvidencePin{evidence_id:eid(n),revision:Revision(1),digest:canonical_digest(&json!({
        "purpose":"SYNTHETIC native feasibility probe only","source_intake":INTAKE_HASH,"recipe":rf,"premise":n,
        "human_or_physical_observation":false,"runtime_admission":false
    })).unwrap()}).collect();
    let basis = EvidenceBasis::new(rf.clone(), pins, vec![]).unwrap();
    let classes = ComponentClassView::new(
        rf.clone(),
        vec![ComponentClassEvidence {
            recipe: rf,
            component_id: recipe.components[0].component_id.clone(),
            class: ComponentClass::General,
            evidence_ids: vec![eid(3)],
        }],
    )
    .unwrap();
    (basis, classes)
}
pub(super) fn refresh_claims(recipe: &mut RecipeRevision) {
    // These fixture claims are themselves checked by the real assessment.
    // Neither test recipe has distribution/image authority; only the original
    // source has the explicitly synthetic storage/class premise for reuse.
    recipe.capabilities = RecipeCapabilities {
        local_display: true,
        shopping: true,
        leftovers: !recipe.storage.is_empty(),
        starter_distribution: false,
        blocking_issues: BoundedVec(vec![]),
    };
    let (basis, classes) = premise_basis(recipe);
    recipe.capabilities = assess_revision(recipe, &basis, &classes)
        .unwrap()
        .assessment()
        .capabilities
        .clone();
}
pub(super) fn recipes() -> (RecipeRevision, RecipeRevision) {
    assert_eq!(bytes_digest(INTAKE).0, INTAKE_HASH);
    let intake: Value = serde_json::from_slice(INTAKE).unwrap();
    let source = intake["candidates"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["intake_id"] == "wikibooks-zucchini-pasta-bake")
        .unwrap();
    assert_eq!(source["source"]["permanent_url"], SOURCE_URL);
    assert_eq!(source["yield_as_published"]["servings"], 4);
    let mut recipe = f11::fixture().context.recipes[0].clone();
    recipe.recipe_id = RecipeId(format!("rc_{:032x}", 0xf001));
    recipe.revision = Revision(1);
    recipe.title = Title("SYNTHETIC courgette bake full-batch probe, not admitted food".into());
    recipe.source=SourceRecord{url:Nullable(Some(HttpUrl(SOURCE_URL.into()))),source_revision:Nullable(Some(Text("oldid=4525784".into()))),retrieved_at:Instant("2026-10-12T14:02:49Z".into()),authors:BoundedVec(vec![Text("Wikibooks contributors; retained intake attribution".into())]),adaptation_notice:Nullable(Some(Text("SYNTHETIC test reification of six quantified source inputs only. No new source retrieval; unspecified inputs omitted. Storage, units and timings are test premises, not source facts or food authority.".into()))),origin:RecipeSourceOrigin::Adapted};
    recipe.text_rights=RightsRecord{licence_id:Text("CC-BY-SA-4.0 source; SYNTHETIC personal test".into()),licence_url:Nullable(Some(HttpUrl("https://creativecommons.org/licenses/by-sa/4.0/".into()))),attribution:Text(format!("Wikibooks contributors; {SOURCE_URL}; synthetic mechanical adaptation")),adaptations:BoundedVec(vec![Text("Explicit incomplete-source mechanical probe; not usable recipe or distribution qualification".into())]),distribution:RightsDistribution::PersonalOnly,evidence_ids:BoundedVec(vec![eid(1)])};
    let component = ComponentId(format!("co_{:032x}", 0xf001));
    let literal = [
        ("small dry pasta", 400, Unit::G, IngredientForm::Dry),
        ("grated cheese", 250, Unit::G, IngredientForm::Prepared),
        ("eggs", 2, Unit::Each, IngredientForm::Whole),
        ("milk", 200, Unit::Ml, IngredientForm::Raw),
        ("butter", 50, Unit::G, IngredientForm::Prepared),
        ("medium courgettes", 3, Unit::Each, IngredientForm::Whole),
    ];
    let prototype = recipe.ingredients[0].clone();
    recipe.ingredients = BoundedVec(
        literal
            .iter()
            .enumerate()
            .map(|(i, (name, n, unit, form))| {
                assert_eq!(source["ingredients"][i]["amount"], n.to_string());
                let mut ingredient = prototype.clone();
                ingredient.ingredient_id = IngredientId(format!("ig_{:032x}", 0xf001 + i));
                ingredient.name = NameText((*name).into());
                ingredient.quantity = Nullable(Some(Quantity {
                    amount: p(*n),
                    unit: *unit,
                    form: *form,
                }));
                ingredient.discrete = *unit == Unit::Each;
                ingredient.conversion_evidence = BoundedVec(vec![]);
                ingredient.component_id = component.clone();
                ingredient.step_ids = BoundedVec(vec![1]);
                ingredient
            })
            .collect(),
    );
    let ids = recipe
        .ingredients
        .iter()
        .map(|i| i.ingredient_id.clone())
        .collect::<Vec<_>>();
    recipe.r#yield=YieldSpec{servings:p(4),basis:Text("SYNTHETIC 4 original source-serving units; no output mass, nutrition or measured portion yield".into()),components:BoundedVec(vec![ComponentYield{component_id:component.clone(),quantity:units(4)}])};
    recipe.scaling = ScalingPolicy::DiscreteBatches {
        batch_yield: p(4),
        maximum_batches: 1,
    };
    recipe.components = BoundedVec(vec![ComponentSpec {
        component_id: component.clone(),
        title: Title("SYNTHETIC original cooked bake".into()),
        ingredient_ids: BoundedVec(ids.clone()),
        storage_rule_ids: BoundedVec(vec![eid(3)]),
        production: Nullable(Some(ComponentProductionSpec::Cooked {
            completion_prep_id: 1,
        })),
    }]);
    recipe.steps=BoundedVec(vec![
        InstructionStep{step_id:1,text:Text("SYNTHETIC production premise: reserve 60 minutes of full attention for this full 4-unit bake using home appliances. No real cooking method, doneness or timing qualification.".into()),ingredient_ids:BoundedVec(ids.clone()),component_ids:BoundedVec(vec![component.clone()]),active_minutes:60,elapsed_minutes:60,depends_on:BoundedVec(vec![])},
        InstructionStep{step_id:2,text:Text("SYNTHETIC cold-service premise: use one safely cooled original unit without reheating or resetting its source clock. Actual cooling and transport are not observed.".into()),ingredient_ids:BoundedVec(vec![]),component_ids:BoundedVec(vec![component.clone()]),active_minutes:5,elapsed_minutes:5,depends_on:BoundedVec(vec![])}]);
    recipe.prep = BoundedVec(vec![
        PrepSpec {
            prep_id: 1,
            title: Title("SYNTHETIC full-batch full-attention reservation".into()),
            step_ids: BoundedVec(vec![1]),
            depends_on: BoundedVec(vec![]),
            active_minutes: 60,
            elapsed_minutes: 60,
            earliest_offset_seconds: -3600,
            latest_offset_seconds: -3600,
            ingredient_ids: BoundedVec(ids),
            can_attach_to_prior_session: false,
        },
        PrepSpec {
            prep_id: 2,
            title: Title("SYNTHETIC cold portion handling".into()),
            step_ids: BoundedVec(vec![2]),
            depends_on: BoundedVec(vec![]),
            active_minutes: 5,
            elapsed_minutes: 5,
            earliest_offset_seconds: -300,
            latest_offset_seconds: -300,
            ingredient_ids: BoundedVec(vec![]),
            can_attach_to_prior_session: false,
        },
    ]);
    recipe.equipment = BoundedVec(vec![
        EquipmentCode::Fridge,
        EquipmentCode::Hob,
        EquipmentCode::Oven,
    ]);
    recipe.active_minutes = 65;
    recipe.elapsed_minutes = 65;
    recipe.meal_types = BoundedVec(vec![MealType::Lunch, MealType::Dinner]);
    recipe.portability = PortabilityEvidence {
        cold_suitable: true,
        reheating_required: false,
        cold_storage_required: true,
        evidence_ids: BoundedVec(vec![eid(5)]),
    };
    recipe.storage=BoundedVec(vec![StorageRule{evidence_id:eid(3),component_id:component.clone(),max_chilled_seconds:86400,cooling_instruction:Text("SYNTHETIC assumed safe cooling and continuous chilling; no physical evidence and not yet scheduled".into()),reheating_instruction:Text("SYNTHETIC cold service only; zero reheats".into()),max_reheats:0,source:recipe.source.clone()}]);
    recipe.portion_presets = BoundedVec(
        [MealType::Lunch, MealType::Dinner]
            .map(|meal_type| PortionPreset {
                preset: PortionPresetName::Regular,
                age_band: AgeBand::Adult,
                meal_type,
                servings: p(1),
                evidence_ids: BoundedVec(vec![eid(4)]),
            })
            .to_vec(),
    );
    recipe.extras = BoundedVec(vec![]);
    recipe.reused_inputs = BoundedVec(vec![]);
    recipe.dietary_evidence = BoundedVec(vec![]);
    recipe.nutrition = Nullable(None);
    recipe.image = Nullable(None);
    recipe.reviews=BoundedVec([ReviewKind::SourceReview,ReviewKind::QuantityReview].into_iter().enumerate().map(|(i,kind)|ReviewRecord{evidence_id:eid(i as u32+1),kind,actor:ReviewActor::Automated,at:Instant("2026-10-13T12:00:00Z".into()),note:Text("SYNTHETIC automated fixture premise; not source clearance, food admission, human calibration or cooking observation".into())}).collect());
    refresh_claims(&mut recipe);
    let mut cold = recipe.clone();
    cold.recipe_id = RecipeId(format!("rc_{:032x}", 0xf002));
    cold.title = Title("SYNTHETIC zero-addition cold original-unit service".into());
    let child_component = ComponentId(format!("co_{:032x}", 0xf002));
    cold.ingredients = BoundedVec(vec![]);
    cold.equipment = BoundedVec(vec![]);
    cold.storage = BoundedVec(vec![]);
    cold.r#yield = YieldSpec {
        servings: p(1),
        basis: Text(
            "SYNTHETIC one original source-serving unit, no new ingredient or measured output"
                .into(),
        ),
        components: BoundedVec(vec![ComponentYield {
            component_id: child_component.clone(),
            quantity: units(1),
        }]),
    };
    cold.scaling = ScalingPolicy::Linear {
        minimum: p(1),
        maximum: p(1),
    };
    cold.components = BoundedVec(vec![ComponentSpec {
        component_id: child_component.clone(),
        title: Title("SYNTHETIC served original portion".into()),
        ingredient_ids: BoundedVec(vec![]),
        storage_rule_ids: BoundedVec(vec![]),
        production: Nullable(Some(ComponentProductionSpec::Unheated {
            first_handling_prep_id: 1,
        })),
    }]);
    let mut step = recipe.steps[1].clone();
    step.step_id = 1;
    step.component_ids = BoundedVec(vec![component.clone(), child_component]);
    cold.steps = BoundedVec(vec![step]);
    let mut prep = recipe.prep[1].clone();
    prep.prep_id = 1;
    prep.step_ids = BoundedVec(vec![1]);
    cold.prep = BoundedVec(vec![prep]);
    cold.active_minutes = 5;
    cold.elapsed_minutes = 5;
    cold.meal_types = BoundedVec(vec![MealType::Lunch]);
    cold.portion_presets
        .0
        .retain(|preset| preset.meal_type == MealType::Lunch);
    cold.reused_inputs = BoundedVec(vec![ReusedComponentInput {
        source_recipe: reference(&recipe),
        source_component_id: component,
        quantity_per_recipe_yield: units(1),
        step_ids: BoundedVec(vec![1]),
        planned_reheats: 0,
        evidence_ids: BoundedVec(vec![eid(2)]),
    }]);
    refresh_claims(&mut cold);
    (recipe, cold)
}

pub(super) fn assemble(source: RecipeRevision, cold: Option<RecipeRevision>) -> SavedFoodBasis {
    let mut saved = f11::fixture();
    let portable = cold.is_some();
    let source_ref = reference(&source);
    let component = source.components[0].component_id.clone();
    let mut recipes = vec![source];
    if let Some(cold) = cold {
        recipes.push(cold);
    }
    let refs = recipes.iter().map(reference).collect::<Vec<_>>();
    saved.context.recipes = BoundedVec(recipes);
    saved.context.stamp.recipe_refs = BoundedVec(refs.clone());
    saved.context.household.diners[0].age_band = AgeBand::Adult;
    saved.context.household.diners[0].restrictions = RestrictionDeclaration::ConfirmedNone {
        confirmed_at: Instant("2026-10-01T00:00:00Z".into()),
    };
    saved.context.household.diners[0].portions = PortionPreference {
        preset: PortionPresetName::Regular,
        adjustment: p(1),
        meal_overrides: BoundedVec(vec![]),
    };
    saved.context.household.preferences.equipment = BoundedVec(vec![
        EquipmentCode::Fridge,
        EquipmentCode::Hob,
        EquipmentCode::Oven,
    ]);
    saved.context.household.preferences.mobility = Mobility {
        reheating: false,
        cold_storage: true,
        temperature: MobilityTemperature::Cold,
    };
    saved.plan.selections.0.truncate(2);
    saved.plan.batch_links.0.truncate(1);
    let week_id = saved.plan.week_id.clone();
    let slots = [(19, MealType::Dinner), (20, MealType::Lunch)].map(|(day, meal_type)| SlotId {
        week_id: week_id.clone(),
        date: LocalDate(format!("2026-10-{day:02}")),
        meal_type,
    });
    for (i, selection) in saved.plan.selections.iter_mut().enumerate() {
        selection.slot = slots[i].clone();
        selection.recipe = if i == 0 || !portable {
            source_ref.clone()
        } else {
            refs[1].clone()
        };
        selection.servings[0].servings = p(1);
        selection.servings[0].preset_ref.recipe = selection.recipe.clone();
        selection.servings[0].preset_ref.meal_type = slots[i].meal_type;
        selection.meal_overrides.portions = BoundedVec(vec![]);
        selection.meal_overrides.eat_by = Nullable(Some(at(
            19 + i as u8,
            if i == 0 { "18:00:00" } else { "12:00:00" },
        )));
        if i == 0 {
            if let BatchUse::NewBatch {
                production_servings,
                ..
            } = &mut selection.batch_use
            {
                *production_servings = p(4);
            }
        } else {
            if let BatchUse::Leftover { component_ids, .. } = &mut selection.batch_use {
                *component_ids = BoundedVec(vec![component.clone()]);
            }
        }
    }
    let link = &mut saved.plan.batch_links[0];
    link.source_slot = slots[0].clone();
    link.target_slot = slots[1].clone();
    link.components = BoundedVec(vec![ComponentAllocation {
        component_id: component,
        quantity: units(1),
    }]);
    let week = &mut saved.context.weeks[0];
    week.range.monday = LocalDate("2026-10-19".into());
    for (i, cell) in week.cells.iter_mut().enumerate() {
        cell.slot.date = LocalDate(format!("2026-10-{:02}", 19 + i / 3));
        cell.choice = if cell.slot == slots[0] {
            MealLocation::AtHome
        } else if cell.slot == slots[1] {
            if portable {
                MealLocation::OnTheMove
            } else {
                MealLocation::AtHome
            }
        } else {
            MealLocation::Skip
        };
        cell.attendee_ids = BoundedVec(if cell.choice != MealLocation::Skip {
            vec![saved.context.household.diners[0].diner_id.clone()]
        } else {
            vec![]
        });
    }
    week.plan = saved.plan.clone();
    saved.context.requirements = BoundedVec(
        availability::meal_requirements(&saved.context.household, week, &[])
            .unwrap()
            .into(),
    );
    // Native MealRequirements already permits distinct location equipment. This
    // is an explicit typed input, not an appliance erased from the source recipe.
    // Browser availability currently copies household equipment to every cell.
    if portable {
        saved
            .context
            .requirements
            .iter_mut()
            .find(|m| m.slot == slots[1])
            .unwrap()
            .equipment = BoundedVec(vec![]);
    }
    let mut evidence = FoodEvidence {
        catalogue_revision: saved.context.stamp.catalogue_revision,
        catalogue_digest: PayloadDigest("0".repeat(64)),
        policy_version: Revision(1),
        recipes: vec![],
        classes: vec![],
        methods: vec![],
        resources: vec![],
        essential_ingredient_ids: vec![],
    };
    for (i, recipe) in saved.context.recipes.iter().enumerate() {
        let rf = reference(recipe);
        let (basis, classes) = premise_basis(recipe);
        evidence.recipes.push(basis);
        evidence.classes.push(classes);
        evidence.methods.push(MethodPhases {
            portable_cold: None,
            recipe: rf.clone(),
            kind: MethodKind::Production,
            prep_ids: vec![1],
            direct_reuse_counts: vec![],
            evidence_ids: vec![eid(1), eid(2)],
        });
        evidence.resources.push(PhaseResources {
            recipe: rf.clone(),
            prep_id: 1,
            equipment: recipe.equipment.0.clone(),
            evidence_ids: vec![eid(3)],
        });
        if i == 0 {
            evidence.methods.push(MethodPhases {
                portable_cold: None,
                recipe: rf.clone(),
                kind: MethodKind::Reuse,
                prep_ids: vec![2],
                direct_reuse_counts: vec![ComponentReheatCount {
                    recipe: rf.clone(),
                    component_id: recipe.components[0].component_id.clone(),
                    planned_reheats: 0,
                    evidence_ids: vec![eid(3)],
                }],
                evidence_ids: vec![eid(3)],
            });
            evidence.resources.push(PhaseResources {
                recipe: rf,
                prep_id: 2,
                equipment: vec![],
                evidence_ids: vec![eid(3)],
            });
        }
    }
    evidence.catalogue_digest = food_evidence_digest(&evidence).unwrap();
    saved.evidence = evidence;
    saved.schedule.layout = ScheduleLayout {
        routes: vec![],
        fixed_starts: vec![],
        attachments: vec![],
    };
    for (i, selection) in saved.plan.selections.iter().enumerate() {
        let phase = PhaseKey {
            occurrence: OccurrenceKey {
                slot: selection.slot.clone(),
                extra_id: None,
                recipe: selection.recipe.clone(),
            },
            prep_id: if i == 0 || portable { 1 } else { 2 },
        };
        let session = SessionKey {
            anchor_slot: selection.slot.clone(),
            role: SessionRole::Cooking,
            ordinal: 1,
        };
        saved.schedule.layout.routes.push(PhaseRoute {
            phase: phase.clone(),
            session,
        });
        saved
            .schedule
            .layout
            .fixed_starts
            .push(PhaseStartConstraint {
                phase,
                start: shifted(
                    selection.meal_overrides.eat_by.0.as_ref().unwrap(),
                    if i == 0 { -3600 } else { -300 },
                ),
            });
    }
    exact_fixed_occupancy(&mut saved);
    assign_identities(&mut saved);
    saved
}
pub(super) fn exact_fixed_occupancy(saved: &mut SavedFoodBasis) {
    // Ask the public native model which phases actually execute. This fixture
    // fixes every start; its occupancy envelope is therefore the exact minimum
    // fixed start and maximum phase end, never a speculative broader query.
    let keys =
        mstrchf_core::prep::required_phase_keys(&saved.plan, &saved.context, &saved.evidence)
            .unwrap();
    let mut bounds = Vec::new();
    for key in keys {
        let start = &saved
            .schedule
            .layout
            .fixed_starts
            .iter()
            .find(|fixed| fixed.phase == key)
            .unwrap()
            .start;
        let recipe = saved
            .context
            .recipes
            .iter()
            .find(|recipe| reference(recipe) == key.occurrence.recipe)
            .unwrap();
        let spec = recipe
            .prep
            .iter()
            .find(|spec| spec.prep_id == key.prep_id)
            .unwrap();
        bounds.push(start.clone());
        bounds.push(shifted(start, i64::from(spec.elapsed_minutes) * 60));
    }
    bounds.sort_by(|a, b| a.instant.cmp(&b.instant));
    saved.schedule.occupied.query = Some(OccupancyQuery {
        horizon: TimeInterval {
            start: bounds.first().unwrap().clone(),
            end: bounds.last().unwrap().clone(),
        },
        excluded_subject: None,
    });
    saved.schedule.occupied.digest = occupancy_snapshot_digest(&saved.schedule.occupied).unwrap();
}
pub(super) fn assign_identities(saved: &mut SavedFoodBasis) {
    let keys = required_identity_keys(
        &saved.plan,
        &saved.context,
        &saved.evidence,
        &saved.schedule,
    )
    .unwrap();
    saved.identities = FoodIdentities {
        revision: Revision(1),
        digest: PayloadDigest("0".repeat(64)),
        allocations: keys
            .allocations
            .into_iter()
            .enumerate()
            .map(|(i, key)| (key, AllocationId(format!("al_{:032x}", 0xf001 + i))))
            .collect(),
        sessions: keys
            .sessions
            .into_iter()
            .enumerate()
            .map(|(i, key)| (key, PrepSessionId(format!("ps_{:032x}", 0xf001 + i))))
            .collect(),
        demand_lines: keys
            .demand_lines
            .into_iter()
            .enumerate()
            .map(|(i, key)| (key, DemandLineId(format!("dl_{:032x}", 0xf001 + i))))
            .collect(),
    };
    saved.identities.digest = food_identities_digest(&saved.identities).unwrap();
}
pub(super) fn derive(saved: &SavedFoodBasis) -> Result<FoodDerivation, FoodFailure> {
    let mut feasibility = f11::feasibility();
    feasibility.evaluated_at = Instant("2026-10-18T12:00:00Z".into());
    feasibility.digest = digest_feasibility_inputs(&feasibility).unwrap();
    derive_plan(
        &saved.plan,
        &saved.context,
        &FoodInputs {
            evidence: &saved.evidence,
            identities: &saved.identities,
            schedule: &saved.schedule,
            feasibility: &feasibility,
        },
    )
}
pub(super) fn check_output(saved: &SavedFoodBasis, derived: &DerivedPlan) {
    assert_eq!(derived.batches.len(), 1);
    let component = &derived.batches[0].components[0];
    assert_eq!(component.produced, units(4));
    assert_eq!(component.allocated, units(2));
    assert_eq!(component.unallocated.amount, Rational::new(2, 1).unwrap());
    assert_eq!(
        component.original_production,
        ProductionMoment::Cooked {
            completed_at: at(19, "18:00:00")
        }
    );
    assert_eq!(derived.allocations.len(), 2);
    assert!(
        derived
            .allocations
            .iter()
            .all(|a| a.quantity == units(1) && a.planned_reheats == 0)
    );
    assert_eq!(derived.demand.lines.len(), 6);
    for ingredient in saved.context.recipes[0].ingredients.iter() {
        let line = derived
            .demand
            .lines
            .iter()
            .find(|line| line.ingredient_id == ingredient.ingredient_id)
            .unwrap();
        assert_eq!(
            line.quantity,
            ingredient.quantity.0.as_ref().unwrap().clone()
        );
        assert_eq!(line.origin_batches.len(), 1);
        assert!(line.origin_extras.is_empty());
    }
    assert_eq!(derived.prep_sessions.len(), 2);
    assert_eq!(derived.prep_sessions[0].tasks.len(), 1);
    assert_eq!(derived.prep_sessions[1].tasks.len(), 1);
    assert_eq!(
        derived.prep_sessions[0].ingredient_needed_at,
        at(19, "17:00:00")
    );
    assert!(derived.prep_sessions[1].ingredient_ids.is_empty());
}

// The deliberately mismatched-source case needs all ordinary typed context
// material, but identity discovery must be allowed to report its exact failure.
pub(super) fn assemble_without_keys(
    source: RecipeRevision,
    cold: RecipeRevision,
) -> SavedFoodBasis {
    let (valid_source, valid_cold) = recipes();
    let mut saved = assemble(valid_source, Some(valid_cold));
    let rf = reference(&source);
    saved.context.recipes[0] = source;
    saved.context.recipes[1] = cold;
    let refs = saved
        .context
        .recipes
        .iter()
        .map(reference)
        .collect::<Vec<_>>();
    saved.context.stamp.recipe_refs = BoundedVec(refs.clone());
    saved.plan.selections[0].recipe = rf.clone();
    saved.plan.selections[0].servings[0].preset_ref.recipe = rf.clone();
    saved.plan.selections[1].recipe = refs[1].clone();
    saved.plan.selections[1].servings[0].preset_ref.recipe = refs[1].clone();
    saved.context.weeks[0].plan = saved.plan.clone();
    for i in 0..2 {
        let (basis, classes) = premise_basis(&saved.context.recipes[i]);
        saved.evidence.recipes[i] = basis;
        saved.evidence.classes[i] = classes;
    }
    for method in &mut saved.evidence.methods {
        if method.recipe.recipe_id == rf.recipe_id {
            method.recipe = rf.clone();
            for count in &mut method.direct_reuse_counts {
                count.recipe = rf.clone();
            }
        }
    }
    for resource in &mut saved.evidence.resources {
        if resource.recipe.recipe_id == rf.recipe_id {
            resource.recipe = rf.clone();
        }
    }
    for route in &mut saved.schedule.layout.routes {
        if route.phase.occurrence.recipe.recipe_id == rf.recipe_id {
            route.phase.occurrence.recipe = rf.clone();
        }
    }
    for fixed in &mut saved.schedule.layout.fixed_starts {
        if fixed.phase.occurrence.recipe.recipe_id == rf.recipe_id {
            fixed.phase.occurrence.recipe = rf.clone();
        }
    }
    saved.evidence.catalogue_digest = food_evidence_digest(&saved.evidence).unwrap();
    saved
}

/// Explicit synthetic schedule, not physical cooling or food admission:
/// cook ends at T; five full-attention minutes handle the remainder, then
/// 115 passive minutes reserve the fridge. Original clock remains T.
pub(super) fn cooling_recipe() -> RecipeRevision {
    let (mut source, _) = recipes();
    source.components[0].production = Nullable(Some(ComponentProductionSpec::CookedWithCooling {
        completion_prep_id: 1,
        cooling_prep_ids: BoundedVec(vec![3, 4]),
        max_cooling_seconds: 7200,
    }));
    for (id, active, elapsed, offset, dep, title) in [
        (3, 5, 5, 0, 1, "SYNTHETIC post-service container handling"),
        (4, 0, 115, 300, 3, "SYNTHETIC passive chilling reservation"),
    ] {
        source.steps.0.push(InstructionStep {
            step_id: id,
            text: Text(format!("{title}. Test premise only; no measured temperature, safety or readiness observation.")),
            ingredient_ids: BoundedVec(vec![]),
            component_ids: BoundedVec(vec![source.components[0].component_id.clone()]),
            active_minutes: active,
            elapsed_minutes: elapsed,
            depends_on: BoundedVec(vec![dep]),
        });
        source.prep.0.push(PrepSpec {
            prep_id: id,
            title: Title(title.into()),
            step_ids: BoundedVec(vec![id]),
            depends_on: BoundedVec(vec![dep]),
            active_minutes: active,
            elapsed_minutes: elapsed,
            earliest_offset_seconds: offset,
            latest_offset_seconds: offset,
            ingredient_ids: BoundedVec(vec![]),
            can_attach_to_prior_session: false,
        });
    }
    source.active_minutes += 5;
    source.elapsed_minutes += 120;
    refresh_claims(&mut source);
    source
}

pub(super) fn cooling_case() -> SavedFoodBasis {
    let (source, cold) = recipes();
    let mut saved = assemble(source, Some(cold));
    replace_source(&mut saved, cooling_recipe());
    let rf = reference(&saved.context.recipes[0]);
    saved
        .evidence
        .methods
        .iter_mut()
        .find(|method| method.recipe == rf && method.kind == MethodKind::Production)
        .unwrap()
        .prep_ids = vec![1, 3, 4];
    for (id, offset) in [(3, 0), (4, 300)] {
        saved.evidence.resources.push(PhaseResources {
            recipe: rf.clone(),
            prep_id: id,
            equipment: vec![EquipmentCode::Fridge],
            evidence_ids: vec![eid(3)],
        });
        let phase = PhaseKey {
            occurrence: OccurrenceKey {
                slot: saved.plan.selections[0].slot.clone(),
                extra_id: None,
                recipe: rf.clone(),
            },
            prep_id: id,
        };
        saved.schedule.layout.routes.push(PhaseRoute {
            phase: phase.clone(),
            session: saved.schedule.layout.routes[0].session.clone(),
        });
        saved
            .schedule
            .layout
            .fixed_starts
            .push(PhaseStartConstraint {
                phase,
                start: shifted(&at(19, "18:00:00"), offset),
            });
    }
    canonicalize_schedule(&mut saved);
    saved.evidence.catalogue_digest = food_evidence_digest(&saved.evidence).unwrap();
    exact_fixed_occupancy(&mut saved);
    assign_identities(&mut saved);
    saved
}

/// Re-pins exact fixture bodies, evidence and schedule references after an
/// intentional synthetic source change; no authority is borrowed from old refs.
pub(super) fn replace_source(saved: &mut SavedFoodBasis, source: RecipeRevision) {
    let old_refs = saved
        .context
        .recipes
        .iter()
        .map(reference)
        .collect::<Vec<_>>();
    let rf = reference(&source);
    saved.context.recipes[0] = source;
    if saved.context.recipes.len() > 1 {
        saved.context.recipes[1].reused_inputs[0].source_recipe = rf;
        let mut cold = saved.context.recipes[1].clone();
        refresh_claims(&mut cold);
        saved.context.recipes[1] = cold;
    }
    let refs = saved
        .context
        .recipes
        .iter()
        .map(reference)
        .collect::<Vec<_>>();
    saved.context.stamp.recipe_refs = BoundedVec(refs.clone());
    for (i, recipe) in saved.context.recipes.iter().enumerate() {
        let (basis, classes) = premise_basis(recipe);
        saved.evidence.recipes[i] = basis;
        saved.evidence.classes[i] = classes;
        for selection in saved
            .plan
            .selections
            .iter_mut()
            .filter(|selection| selection.recipe == old_refs[i])
        {
            selection.recipe = refs[i].clone();
            for serving in selection.servings.iter_mut() {
                serving.preset_ref.recipe = refs[i].clone();
            }
        }
        for method in saved
            .evidence
            .methods
            .iter_mut()
            .filter(|method| method.recipe == old_refs[i])
        {
            method.recipe = refs[i].clone();
            for count in &mut method.direct_reuse_counts {
                count.recipe = refs[i].clone();
            }
        }
        for resource in saved
            .evidence
            .resources
            .iter_mut()
            .filter(|resource| resource.recipe == old_refs[i])
        {
            resource.recipe = refs[i].clone();
        }
        for route in saved
            .schedule
            .layout
            .routes
            .iter_mut()
            .filter(|route| route.phase.occurrence.recipe == old_refs[i])
        {
            route.phase.occurrence.recipe = refs[i].clone();
        }
        for fixed in saved
            .schedule
            .layout
            .fixed_starts
            .iter_mut()
            .filter(|fixed| fixed.phase.occurrence.recipe == old_refs[i])
        {
            fixed.phase.occurrence.recipe = refs[i].clone();
        }
    }
    saved.context.weeks[0].plan = saved.plan.clone();
    saved.evidence.catalogue_digest = food_evidence_digest(&saved.evidence).unwrap();
}

pub(super) fn canonicalize_schedule(saved: &mut SavedFoodBasis) {
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
    saved
        .evidence
        .resources
        .sort_by(|a, b| recipe_cmp(&a.recipe, &b.recipe).then(a.prep_id.cmp(&b.prep_id)));
}
