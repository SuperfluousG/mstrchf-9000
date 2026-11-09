//! Host-owned synthetic development input. Never production recipe admission.
use mstrchf_contracts::*;
use mstrchf_core::{food::*, recipes::*, selection::*};

#[derive(serde::Serialize)]
pub struct DevelopmentChoice {
    pub title: String,
    pub selection: MealSelection,
    pub plan: Option<WeekPlan>,
}
#[derive(serde::Serialize)]
pub struct DevelopmentBoard {
    pub week: Week,
    pub choices: Vec<DevelopmentChoice>,
    pub snapshot_revision:Revision,
    pub details:Option<crate::planner_state::PlannerDetails>,
}

pub fn recipe_reference(r: &RecipeRevision) -> Result<RecipeRef, DomainError> {
    Ok(RecipeRef {
        recipe_id: r.recipe_id.clone(),
        revision: r.revision,
        digest: recipe_digest(r)?,
    })
}
pub fn catalogue() -> Result<Vec<RecipeRevision>, DomainError> {
    static CATALOGUE:std::sync::OnceLock<Result<Vec<RecipeRevision>,DomainError>>=std::sync::OnceLock::new();
    CATALOGUE.get_or_init(build_catalogue).clone()
}
fn build_catalogue() -> Result<Vec<RecipeRevision>, DomainError> {
    let original: RecipeRevision = parse_document(include_bytes!(
        "../../../tests/fixtures/food/admission/ruling004-current--complete-synthetic.json"
    ))?;
    (1..=3)
        .map(|id| {
            let mut r = original.clone();
            r.recipe_id = RecipeId(format!("rc_{id:032x}"));
            r.title = Title(if id == 3 {
                "Synthetic meat control".into()
            } else {
                format!(
                    "Synthetic vegetarian bowl {}",
                    if id == 1 { "A" } else { "B" }
                )
            });
            r.source.retrieved_at = Instant("2026-01-01T12:00:00Z".into());
            r.scaling = ScalingPolicy::Linear {
                minimum: PositiveRational::new(1, 2)?,
                maximum: PositiveRational::new(2, 1)?,
            };
            r.components[0].production = Nullable(Some(ComponentProductionSpec::Cooked {
                completion_prep_id: 1,
            }));
            r.active_minutes = 10;
            r.steps[0].active_minutes = 10;
            r.r#yield.components[0].quantity.form = IngredientForm::Cooked;
            r.prep = BoundedVec(vec![PrepSpec {
                prep_id: 1,
                title: Title("Synthetic cooking phase".into()),
                step_ids: BoundedVec(vec![1]),
                depends_on: BoundedVec(vec![]),
                active_minutes: 10,
                elapsed_minutes: 10,
                earliest_offset_seconds: -600,
                latest_offset_seconds: -600,
                ingredient_ids: BoundedVec(vec![r.ingredients[0].ingredient_id.clone()]),
                can_attach_to_prior_session: false,
            }]);
            if id == 3 {
                r.dietary_evidence[0].verdict = DietaryVerdict::Conflicting;
            }
            parse_document(&canonical_bytes(&r)?)
        })
        .collect()
}
pub(crate) fn evidence_inputs(r: &RecipeRevision) -> Result<(EvidenceBasis, ComponentClassView), DomainError> {
    let rf = recipe_reference(r)?;
    let pins=(1..=8).map(|n| {let id=EvidenceId(format!("ev_{n:032x}"));Ok(EvidencePin{evidence_id:id.clone(),revision:Revision(1),digest:canonical_digest(&serde_json::json!({"fixture":"m1-host-owned-synthetic-v1","recipe":rf,"evidence_id":id,"statement":"Synthetic engineering premise only; no production food or rights admission."}))?})}).collect::<Result<Vec<_>,DomainError>>()?;
    Ok((
        EvidenceBasis::new(rf.clone(), pins, r.image.0.iter().map(|v|v.digest.clone()).collect())?,
        ComponentClassView::new(
            rf.clone(),
            vec![ComponentClassEvidence {
                recipe: rf,
                component_id: r.components[0].component_id.clone(),
                class: ComponentClass::General,
                evidence_ids: vec![EvidenceId(format!("ev_{:032x}", 8))],
            }],
        )?,
    ))
}
pub(crate) fn context(
    s: &Snapshot,
    p: &HouseholdProfile,
    w: &Week,
    recipes: Vec<RecipeRevision>,
) -> Result<PlanningContext, DomainError> {
    let references = recipes
        .iter()
        .map(recipe_reference)
        .collect::<Result<Vec<_>, _>>()?;
    let revisions = vec![WeekRevision {
        week_id: w.range.week_id.clone(),
        revision: w.revision,
    }];
    Ok(PlanningContext {
        schema_version: SchemaVersion,
        household: p.clone(),
        settings_revision: s.privacy.revision,
        privacy: s.privacy.clone(),
        requirements: BoundedVec(mstrchf_core::availability::meal_requirements(p, w, &[])?.into()),
        weeks: BoundedVec(vec![w.clone()]),
        recipes: BoundedVec(recipes),
        history: PlanningHistory {
            revision: Revision(1),
            selected: BoundedVec(vec![]),
            ordered: BoundedVec(vec![]),
            feedback: BoundedVec(vec![]),
            hidden_recipe_ids: BoundedVec(vec![]),
        },
        stamp: ContextStamp {
            household_revision: p.revision,
            settings_revision: s.privacy.revision,
            privacy_revision: s.privacy.revision,
            history_revision: Revision(1),
            catalogue_revision: Revision(1),
            ranking_policy_version: Revision(1),
            recipe_refs: BoundedVec(references),
            read_weeks: BoundedVec(revisions.clone()),
            write_weeks: BoundedVec(revisions),
        },
        ranking_policy: RankingPolicyRef {
            version: Revision(1),
            digest: PayloadDigest(
                "cff488e1f4729bc788b1a13daece0ace72b714768de10b0446d9bd049b214daa".into(),
            ),
        },
    })
}
fn food_error(e: FoodFailure) -> DomainError {
    match e {
        FoodFailure::Invalid(e) => e,
        FoodFailure::Blocked { conflicts, .. } => DomainError::new(
            conflicts
                .first()
                .map_or(ErrorCode::DomainConflict, |c| c.code),
            "/development/food",
        ),
    }
}

fn validate_m1_saved_plan(
    s: &Snapshot,
    p: &HouseholdProfile,
    w: &Week,
) -> Result<Option<FoodDerivation>, DomainError> {
    if !w.plan.batch_links.is_empty() {
        return Err(crate::records::unsupported());
    }
    if w.plan.selections.is_empty() {
        return Ok(None);
    }
    // M1 has one fresh Monday dinner. Later extras, batches and other slots remain M2.
    if w.plan.selections.len() != 1 {
        return Err(crate::records::unsupported());
    }
    let selected = &w.plan.selections[0];
    if selected.slot != w.cells[2].slot
        || !selected.extras.is_empty()
        || !selected.meal_overrides.portions.is_empty()
        || selected.meal_overrides.eat_by.0.is_some()
        || !matches!(selected.batch_use, BatchUse::NewBatch { .. })
    {
        return Err(crate::records::unsupported());
    }
    let c = context(s, p, w, catalogue()?)?;
    let inputs = c
        .recipes
        .iter()
        .map(evidence_inputs)
        .collect::<Result<Vec<_>, _>>()?;
    let reviewed = c
        .recipes
        .iter()
        .zip(&inputs)
        .map(|(r, (b, k))| assess_revision(r, b, k))
        .collect::<Result<Vec<_>, _>>()?;
    let choice = ChoiceSpec {
        portable_cold: None,
        recipe: selected.recipe.clone(),
        extras: vec![],
        meal_overrides: selected.meal_overrides.clone(),
        batch_use: selected.batch_use.clone(),
    };
    match assess_choice(
        &c,
        &selected.slot,
        &choice,
        selected.origin,
        selected.repeat_authorised,
        &reviewed,
    )? {
        ChoiceAssessment::Suitable(actual) if actual == *selected => {}
        ChoiceAssessment::Suitable(_) => {
            return Err(DomainError::new(
                ErrorCode::IdentityMismatch,
                "/command/data/week/plan/servings",
            ));
        }
        ChoiceAssessment::Ineligible(conflicts) | ChoiceAssessment::Unresolved(conflicts) => {
            return Err(DomainError::new(
                conflicts
                    .first()
                    .map_or(ErrorCode::DomainConflict, |c| c.code),
                "/command/data/week/plan/restrictions",
            ));
        }
    }
    let relevant = inputs
        .iter()
        .filter(|x| x.0.recipe == selected.recipe)
        .collect::<Vec<_>>();
    let mut evidence = FoodEvidence {
        catalogue_revision: Revision(1),
        catalogue_digest: PayloadDigest("0".repeat(64)),
        policy_version: Revision(1),
        recipes: relevant.iter().map(|x| x.0.clone()).collect(),
        classes: relevant.iter().map(|x| x.1.clone()).collect(),
        methods: vec![],
        resources: vec![],
        essential_ingredient_ids: vec![],
    };
    let ev = vec![EvidenceId(format!("ev_{:032x}", 8))];
    evidence.methods.push(MethodPhases {
        portable_cold: None,
        recipe: selected.recipe.clone(),
        kind: MethodKind::Production,
        prep_ids: vec![1],
        direct_reuse_counts: vec![],
        evidence_ids: ev.clone(),
    });
    evidence.resources.push(PhaseResources {
        recipe: selected.recipe.clone(),
        prep_id: 1,
        equipment: vec![EquipmentCode::Hob],
        evidence_ids: ev,
    });
    evidence.catalogue_digest = food_evidence_digest(&evidence)?;
    let phase = PhaseKey {
        occurrence: OccurrenceKey {
            slot: selected.slot.clone(),
            extra_id: None,
            recipe: selected.recipe.clone(),
        },
        prep_id: 1,
    };
    let session = SessionKey {
        anchor_slot: selected.slot.clone(),
        role: SessionRole::Cooking,
        ordinal: 1,
    };
    let meal = c
        .requirements
        .iter()
        .find(|m| m.slot == selected.slot)
        .ok_or_else(|| DomainError::new(ErrorCode::MissingReference, "/requirements"))?;
    let end = meal
        .eat_by
        .0
        .clone()
        .ok_or_else(|| DomainError::new(ErrorCode::MissingReference, "/eat_by"))?;
    let local = chrono::NaiveDateTime::parse_from_str(
        &format!("{}T{}", end.local_date.0, end.local_time.0),
        "%Y-%m-%dT%H:%M:%S",
    )
    .map_err(crate::records::storage)?
        - chrono::Duration::seconds(600);
    let start = mstrchf_core::availability::resolve_meal_time(
        &LocalDate(local.format("%Y-%m-%d").to_string()),
        &LocalTime(local.format("%H:%M:%S").to_string()),
        &end.zone,
        None,
    )?;
    let mut occupied = OccupancySnapshot {
        query: Some(OccupancyQuery {
            horizon: TimeInterval {
                start: start.clone(),
                end: end.clone(),
            },
            excluded_subject: None,
        }),
        occupancy_epoch: Revision(1),
        owners: vec![],
        digest: PayloadDigest("0".repeat(64)),
    };
    occupied.digest = occupancy_snapshot_digest(&occupied)?;
    let schedule = ScheduleInputs {
        layout: ScheduleLayout {
            routes: vec![PhaseRoute {
                phase: phase.clone(),
                session,
            }],
            fixed_starts: vec![PhaseStartConstraint { phase, start }],
            attachments: vec![],
        },
        occupied,
    };
    let keys = required_identity_keys(&w.plan, &c, &evidence, &schedule).map_err(food_error)?;
    let mut identities = FoodIdentities {
        revision: Revision(1),
        digest: PayloadDigest("0".repeat(64)),
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
    identities.digest = food_identities_digest(&identities)?;
    let mut products = ProductUseSnapshot {
        revision: Revision(1),
        digest: PayloadDigest("0".repeat(64)),
        valid_until: None,
        evidence: vec![],
        bindings: vec![],
        facts: vec![],
    };
    products.digest = digest_product_uses(&products)?;
    let mut feasibility = FeasibilityInputs {
        revision: Revision(1),
        digest: PayloadDigest("0".repeat(64)),
        delivery: DeliveryInput::Unselected,
        products,
        confirmations: vec![],
        evaluated_at: now()?,
    };
    feasibility.digest = digest_feasibility_inputs(&feasibility)?;
    Ok(Some(
        derive_plan(
            &w.plan,
            &c,
            &FoodInputs {
                evidence: &evidence,
                identities: &identities,
                schedule: &schedule,
                feasibility: &feasibility,
            },
        )
        .map_err(food_error)?,
    ))
}
pub fn validate_saved_plan(s:&Snapshot,p:&HouseholdProfile,w:&Week)->Result<Option<FoodDerivation>,DomainError>{
    crate::planner::derive(s,p,w)
}
pub(crate) fn validate_legacy_plan(s:&Snapshot,p:&HouseholdProfile,w:&Week)->Result<Option<FoodDerivation>,DomainError>{validate_m1_saved_plan(s,p,w)}
pub fn board(s: &Snapshot) -> Result<DevelopmentBoard, DomainError> {
    board_for(s,None,None)
}
pub fn board_for(s:&Snapshot,week_id:Option<&WeekId>,slot:Option<&SlotId>)->Result<DevelopmentBoard,DomainError>{
    let week = match week_id {
        Some(id)=>s.weeks.iter().find(|w|&w.range.week_id==id).cloned().ok_or_else(||DomainError::new(ErrorCode::ForeignReference,"/week_id"))?,
        None=>seed_week(s,&now()?)?,
    };
    let slot=slot.cloned().unwrap_or_else(||week.cells[2].slot.clone());
    let profile = s.profile.0.clone().map_or_else(|| seed_profile(s), Ok)?;
    let choices=crate::planner::choices(s,&profile,&week,&slot,MealOverrides{portions:BoundedVec(vec![]),eat_by:Nullable(None)})?;
    Ok(DevelopmentBoard { week, choices,snapshot_revision:s.snapshot_revision,details:None })
}

pub fn now() -> Result<Instant, DomainError> {
    let seconds = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map_err(crate::records::storage)?
        .as_secs();
    let date = chrono::DateTime::from_timestamp(
        i64::try_from(seconds).map_err(crate::records::storage)?,
        0,
    )
    .ok_or_else(|| DomainError::new(ErrorCode::InvalidValue, "/development/time"))?;
    Instant::try_from(
        date.to_rfc3339_opts(chrono::SecondsFormat::Secs, true)
            .as_str(),
    )
}

pub fn seed_profile(snapshot: &Snapshot) -> Result<HouseholdProfile, DomainError> {
    let mut p: HouseholdProfile = parse_document(include_bytes!(
        "../../../tests/fixtures/households/p03ab/a1a2-profile.json"
    ))?;
    p.household_id = snapshot.household_draft.household_id.clone();
    p.revision = Revision(1);
    p.diners.0.truncate(1);
    p.diners[0].display_name = Nullable(Some(NameText("Development diner".into())));
    p.diners[0].restrictions = RestrictionDeclaration::Confirmed {
        confirmed_at: Instant("2026-01-01T12:00:00Z".into()),
        requirements: BoundedVec(vec![HardRequirement::Diet {
            code: DietCode::Vegetarian,
        }]),
    };
    p.usual_week.suggest_leftovers = false;
    p.preferences.mobility.temperature = MobilityTemperature::Hot;
    for cell in &mut p.usual_week.cells.0 {
        cell.attendee_ids = BoundedVec(vec![p.diners[0].diner_id.clone()]);
    }
    p.connections.privacy = snapshot.privacy.clone();
    p.connections.harness = Nullable(None);
    p.connections.retailer = Nullable(None);
    p.connections.calendar_destination = Nullable(None);
    Ok(p)
}

pub fn seed_week(snapshot: &Snapshot, evaluated_at: &Instant) -> Result<Week, DomainError> {
    if let Some(week) = snapshot.weeks.first() {
        return Ok(week.clone());
    }
    let profile = snapshot
        .profile
        .0
        .clone()
        .map_or_else(|| seed_profile(snapshot), Ok)?;
    let today = mstrchf_core::weeks::local_planning_date(evaluated_at, &profile.usual_week.zone)?;
    let ids = [1, 2, 3, 4].map(|n| WeekId(format!("wk_{n:032x}")));
    let ranges = mstrchf_core::weeks::planning_horizon(&today, &profile.usual_week.zone, &ids)?;
    mstrchf_core::weeks::new_week(&profile, &snapshot.privacy, evaluated_at, &ranges[0])
}

pub fn seed_runtime(store: &crate::Store) -> Result<(), DomainError> {
    let mut s = store.snapshot()?;
    if s.profile.0.is_none() {
        store.dispatch_ui(AppCommandV1 {
            schema_version: SchemaVersion,
            command_id: CommandId(format!("cm_{}", crate::records::random_id()?)),
            expected_snapshot_revision: s.snapshot_revision,
            command: AppCommand::UpdateProfile {
                profile: seed_profile(&s)?,
                expected_profile_revision: Revision(1),
            },
        })?;
        s = store.snapshot()?;
    }
    if s.weeks.is_empty() {
        let w = seed_week(&s, &now()?)?;
        store.dispatch_ui(AppCommandV1 {
            schema_version: SchemaVersion,
            command_id: CommandId(format!("cm_{}", crate::records::random_id()?)),
            expected_snapshot_revision: s.snapshot_revision,
            command: AppCommand::SaveWeek {
                expected_week_revision: w.revision,
                week: WeekEdit {
                    week_id: w.range.week_id,
                    defaults_snapshot: w.defaults_snapshot,
                    day_enabled: w.day_enabled,
                    meal_type_enabled: w.meal_type_enabled,
                    cells: w.cells,
                    plan: Nullable(Some(w.plan)),
                },
            },
        })?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn development_choices_enforce_diet_and_derive_exact_portion_and_prep() {
        let mut s = crate::records::initial(HouseholdId(format!("hh_{:032x}", 1)));
        s.profile = Nullable(Some(seed_profile(&s).unwrap()));
        let w = seed_week(&s, &Instant("2026-10-07T12:00:00Z".into())).unwrap();
        s.weeks = BoundedVec(vec![w]);
        let choices = board(&s).unwrap();
        assert_eq!(choices.choices.len(), 4);
        assert_eq!(
            choices.choices[0].selection.recipe.recipe_id.0,
            "rc_00000000000000000000000000000001"
        );
        assert_eq!(
            choices.choices[1].selection.recipe.recipe_id.0,
            "rc_00000000000000000000000000000002"
        );
        let mut w = s.weeks[0].clone();
        w.plan.selections = BoundedVec(vec![choices.choices[0].selection.clone()]);
        let food = validate_saved_plan(&s, s.profile.0.as_ref().unwrap(), &w)
            .unwrap()
            .unwrap();
        assert_eq!(food.plan.demand.lines.len(), 1);
        assert_eq!(
            food.plan.demand.lines[0].quantity.amount,
            PositiveRational::new(200, 1).unwrap()
        );
        assert_eq!(food.plan.demand.lines[0].quantity.unit, Unit::G);
        assert_eq!(
            food.plan.batches[0].components[0].produced.amount,
            PositiveRational::new(200, 1).unwrap()
        );
        assert_eq!(food.plan.prep_sessions.len(), 1);
        assert_eq!(food.plan.prep_sessions[0].start.local_time.0, "18:20:00");
        assert_eq!(food.plan.prep_sessions[0].end.local_time.0, "18:30:00");
    }
    #[test]
    fn development_week_uses_the_native_planning_horizon_across_year_end() {
        let snapshot = crate::records::initial(HouseholdId(format!("hh_{:032x}", 1)));
        let week = seed_week(&snapshot, &Instant("2026-12-22T12:00:00Z".into())).unwrap();
        assert_eq!(week.range.monday.0, "2027-01-04");
        assert_eq!(week.cells.last().unwrap().slot.date.0, "2027-01-10");
        assert_eq!(week.cells.len(), 21);
        assert!(week.plan.selections.is_empty());
    }
}
