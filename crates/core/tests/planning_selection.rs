use mstrchf_contracts::*;
use mstrchf_core::{availability::meal_requirements, recipes::*, selection::*};

fn p(n: u64, d: u64) -> PositiveRational {
    PositiveRational::new(n, d).unwrap()
}
fn reference(r: &RecipeRevision) -> RecipeRef {
    RecipeRef {
        recipe_id: r.recipe_id.clone(),
        revision: r.revision,
        digest: recipe_digest(r).unwrap(),
    }
}
fn recipe() -> RecipeRevision {
    parse_document(include_bytes!(
        "../../../tests/fixtures/food/admission/ruling004-current--complete-synthetic.json"
    ))
    .unwrap()
}
fn inputs(r: &RecipeRevision) -> (EvidenceBasis, ComponentClassView) {
    let rf = reference(r);
    // Explicit synthetic caller admission: each digest binds actual fixture
    // statement bytes. These pins establish no real-world food source truth.
    (EvidenceBasis::new(rf.clone(),(1..=32).map(|n| {let id=EvidenceId(format!("ev_{n:032x}")); EvidencePin { evidence_id:id.clone(),revision:Revision(1),digest:canonical_digest(&serde_json::json!({"fixture":"p03cd-selection-synthetic-admission-v1","recipe":rf,"evidence_id":id,"statement":"Synthetic test owner admits this fixture fact for the exact recipe; not production evidence."})).unwrap() }}).collect(),vec![AssetDigest("a".repeat(64))]).unwrap(),
     ComponentClassView::new(rf.clone(),vec![ComponentClassEvidence { recipe:rf,component_id:r.components[0].component_id.clone(),class:ComponentClass::General,evidence_ids:vec![EvidenceId(format!("ev_{:032x}",8))] }]).unwrap())
}
fn context(recipes: Vec<RecipeRevision>, count: usize) -> PlanningContext {
    let mut h: HouseholdProfile = parse_document(include_bytes!(
        "../../../tests/fixtures/households/p03ab/a1a2-profile.json"
    ))
    .unwrap();
    for d in &mut h.diners.0 {
        d.restrictions = RestrictionDeclaration::ConfirmedNone {
            confirmed_at: Instant("2026-12-01T12:00:00Z".into()),
        };
    }
    h.preferences.mobility.temperature = MobilityTemperature::Hot;
    let weeks: Vec<_> = ["2026-12-28", "2027-01-04", "2027-01-11", "2027-01-18"]
        .iter()
        .take(count)
        .enumerate()
        .map(|(i, date)| {
            mstrchf_core::weeks::new_week(
                &h,
                &h.connections.privacy,
                &Instant("2026-12-21T12:00:00Z".into()),
                &WeekRange {
                    week_id: WeekId(format!("wk_{:032x}", i + 1)),
                    monday: LocalDate((*date).into()),
                    zone: ZoneId("Europe/London".into()),
                },
            )
            .unwrap()
        })
        .collect();
    let refs = recipes.iter().map(reference).collect();
    let revisions: Vec<_> = weeks
        .iter()
        .map(|w| WeekRevision {
            week_id: w.range.week_id.clone(),
            revision: w.revision,
        })
        .collect();
    let requirements = weeks
        .iter()
        .flat_map(|w| meal_requirements(&h, w, &[]).unwrap())
        .collect();
    PlanningContext {
        schema_version: SchemaVersion,
        settings_revision: Revision(1),
        privacy: h.connections.privacy.clone(),
        requirements: BoundedVec(requirements),
        weeks: BoundedVec(weeks),
        recipes: BoundedVec(recipes),
        history: PlanningHistory {
            revision: Revision(1),
            selected: BoundedVec(vec![]),
            ordered: BoundedVec(vec![]),
            feedback: BoundedVec(vec![]),
            hidden_recipe_ids: BoundedVec(vec![]),
        },
        stamp: ContextStamp {
            household_revision: h.revision,
            settings_revision: Revision(1),
            privacy_revision: h.connections.privacy.revision,
            history_revision: Revision(1),
            catalogue_revision: Revision(1),
            ranking_policy_version: Revision(1),
            recipe_refs: BoundedVec(refs),
            read_weeks: BoundedVec(revisions.clone()),
            write_weeks: BoundedVec(revisions),
        },
        household: h,
        ranking_policy: RankingPolicyRef {
            version: Revision(1),
            digest: PayloadDigest(
                "cff488e1f4729bc788b1a13daece0ace72b714768de10b0446d9bd049b214daa".into(),
            ),
        },
    }
}

fn context_cache_assess(c: &PlanningContext) -> Result<ChoiceAssessment,DomainError> {
    let r=recipe();let (basis,classes)=inputs(&r);
    let reviewed=vec![assess_revision(&r,&basis,&classes).unwrap()];
    assess_choice(c,&c.weeks[0].cells[2].slot,&choice(&r),SelectionOrigin::Suggested,false,&reviewed)
}

fn cold_context_cache_assess(c: &PlanningContext) -> Result<ChoiceAssessment,DomainError> {
    let c=c.clone();std::thread::spawn(move||context_cache_assess(&c)).join().unwrap()
}

#[test]
fn repeated_context_admission_preserves_warm_cold_choice_and_revalidates_all_changed_inputs() {
    let c=context(vec![recipe()],2);let expected=cold_context_cache_assess(&c);
    assert!(matches!(expected,Ok(ChoiceAssessment::Suitable(_))));
    assert_eq!(context_cache_assess(&c),expected);
    assert_eq!(context_cache_assess(&c),expected);
    let mutations:Vec<(&str,fn(&mut PlanningContext))>=vec![
        ("requirements",|v|v.requirements[2].attendees[0].portion.adjustment=p(4,1)),
        ("household projection",|v|v.household.diners[0].restrictions=RestrictionDeclaration::Unconfirmed{}),
        ("recipe bytes",|v|v.recipes[0].ingredients[0].name=NameText("Changed recipe bytes".into())),
        ("history",|v|v.history.hidden_recipe_ids.0.push(RecipeId("invalid".into()))),
        ("privacy",|v|v.privacy.revision=Revision(0)),
        ("context stamp",|v|v.stamp.history_revision=Revision(0)),
        ("explicit meal clock",|v|v.requirements[2].eat_by.0.as_mut().unwrap().instant=Instant("2000-01-01T00:00:00Z".into())),
    ];
    for (label,mutate) in mutations {
        assert_eq!(context_cache_assess(&c),expected);
        let mut changed=c.clone();mutate(&mut changed);
        let cold=cold_context_cache_assess(&changed);assert!(cold.is_err(),"{label}");
        assert_eq!(context_cache_assess(&changed),cold,"{label}");
        assert_eq!(context_cache_assess(&changed),cold,"repeat failure {label}");
        assert_eq!(context_cache_assess(&c),expected,"prior proof after invalid {label}");
    }
}
fn choice(r: &RecipeRevision) -> ChoiceSpec {
    ChoiceSpec {
        portable_cold: None,
        recipe: reference(r),
        extras: vec![],
        meal_overrides: MealOverrides {
            portions: BoundedVec(vec![]),
            eat_by: Nullable(None),
        },
        batch_use: BatchUse::NewBatch {
            batch_id: BatchId(format!("ba_{:032x}", 1)),
            production_servings: p(2, 1),
        },
    }
}
fn intent(c: &PlanningContext, mode: PlanMode) -> SelectionIntent {
    SelectionIntent {
        mode,
        slots: c.weeks.iter().map(|w| w.cells[2].slot.clone()).collect(),
        origin: SelectionOrigin::Suggested,
        repeat_permissions: vec![],
    }
}
fn build(
    c: &PlanningContext,
    slot: &SlotId,
    x: &ChoiceSpec,
    reviewed: &[ReviewedRecipe<'_>],
    origin: SelectionOrigin,
) -> MealSelection {
    match assess_choice(c, slot, x, origin, false, reviewed).unwrap() {
        ChoiceAssessment::Suitable(s) => s,
        other => panic!("{other:?}"),
    }
}
fn proposal(c: &PlanningContext, selections: Vec<Option<MealSelection>>) -> PlanProposal {
    PlanProposal {
        changes: BoundedVec(
            c.weeks
                .iter()
                .zip(selections)
                .map(|(w, s)| WeekChange {
                    week_id: w.range.week_id.clone(),
                    base_revision: w.revision,
                    replace_slots: BoundedVec(vec![ProposedSelection {
                        slot: w.cells[2].slot.clone(),
                        selection: Nullable(s),
                    }]),
                })
                .collect(),
        ),
    }
}
fn reproject(c: &mut PlanningContext) {
    c.requirements = BoundedVec(
        c.weeks
            .iter()
            .flat_map(|w| meal_requirements(&c.household, w, &[]).unwrap())
            .collect(),
    );
}
fn saved(s: &MealSelection) -> SelectionHistory {
    SelectionHistory {
        week_id: s.slot.week_id.clone(),
        slot: s.slot.clone(),
        recipe: s.recipe.clone(),
        servings: s.servings.clone(),
        extras: s.extras.clone(),
        repeat_authorised: s.repeat_authorised,
        selected_at: Instant("2026-12-21T12:00:00Z".into()),
    }
}

#[test]
fn sc02_sc03_hard_conflict_and_missing_evidence_remain_distinct_on_both_routes() {
    let r = recipe();
    let (basis, classes) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &basis, &classes).unwrap()];
    for (requirement, known) in [
        (
            HardRequirement::IngredientExclusion {
                ingredient_id: r.ingredients[0].ingredient_id.clone(),
            },
            true,
        ),
        (
            HardRequirement::AllergenExclusion {
                code: AllergenCode::Peanuts,
            },
            false,
        ),
    ] {
        let mut c = context(vec![r.clone()], 1);
        c.household.diners[0].restrictions = RestrictionDeclaration::Confirmed {
            confirmed_at: Instant("2026-12-01T12:00:00Z".into()),
            requirements: BoundedVec(vec![requirement]),
        };
        reproject(&mut c);
        for origin in [SelectionOrigin::Fixed, SelectionOrigin::Suggested] {
            let outcome = assess_choice(
                &c,
                &c.weeks[0].cells[2].slot,
                &choice(&r),
                origin,
                false,
                &reviewed,
            )
            .unwrap();
            assert_eq!(matches!(outcome, ChoiceAssessment::Ineligible(_)), known);
            assert_eq!(matches!(outcome, ChoiceAssessment::Unresolved(_)), !known);
        }
    }
}

#[test]
fn sc05_extra_uses_full_overrides_before_recipient_projection() {
    let mut base = recipe();
    let mut extra = recipe();
    extra.recipe_id = RecipeId(format!("rc_{:032x}", 2));
    base.extras.0.push(ExtraSpec {
        extra_id: ExtraId(format!("ex_{:032x}", 1)),
        title: Title("Synthetic extra".into()),
        recipe: reference(&extra),
        eligible_person_requirements: BoundedVec(vec![]),
        supported_with: BoundedVec(vec![base.components[0].component_id.clone()]),
    });
    let (b, bc) = inputs(&base);
    let (e, ec) = inputs(&extra);
    let reviewed = vec![
        assess_revision(&base, &b, &bc).unwrap(),
        assess_revision(&extra, &e, &ec).unwrap(),
    ];
    let c = context(vec![base.clone(), extra.clone()], 1);
    let slot = &c.weeks[0].cells[2].slot;
    let first = PersonRef::Diner {
        diner_id: c.household.diners[0].diner_id.clone(),
    };
    let second = PersonRef::Diner {
        diner_id: c.household.diners[1].diner_id.clone(),
    };
    let mut x = choice(&base);
    x.extras.push(ExtraChoice {
        extra_id: base.extras[0].extra_id.clone(),
        recipe: reference(&extra),
        recipients: vec![first.clone()],
    });
    x.meal_overrides.portions = BoundedVec(vec![
        PersonPortionOverride {
            person: first,
            preset: PortionPresetName::Regular,
            adjustment: p(3, 2),
        },
        PersonPortionOverride {
            person: second,
            preset: PortionPresetName::Regular,
            adjustment: p(1, 2),
        },
    ]);
    let selection = build(&c, slot, &x, &reviewed, SelectionOrigin::Fixed);
    assert_eq!(selection.servings[0].servings, p(3, 2));
    assert_eq!(selection.servings[1].servings, p(1, 2));
    assert_eq!(selection.extras[0].recipients[0].servings, p(3, 2));
    assert_eq!(selection.meal_overrides, x.meal_overrides);
    x.meal_overrides.portions[1].person = PersonRef::Diner {
        diner_id: DinerId(format!("dn_{:032x}", 99)),
    };
    assert_eq!(
        assess_choice(&c, slot, &x, SelectionOrigin::Fixed, false, &reviewed)
            .unwrap_err()
            .code,
        ErrorCode::ForeignReference
    );
}

#[test]
fn sc06_guest_identity_is_scoped_to_actual_occasion() {
    let r = recipe();
    let (basis, classes) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &basis, &classes).unwrap()];
    let mut c = context(vec![r.clone()], 1);
    let cell = &mut c.weeks[0].cells[2];
    cell.occasion = Nullable(Some(Occasion {
        occasion_id: OccasionId(format!("oc_{:032x}", 1)),
        host_diner_ids: cell.attendee_ids.clone(),
        guests: BoundedVec(vec![Guest {
            guest_id: GuestId(format!("gu_{:032x}", 1)),
            age_band: AgeBand::Adult,
            restrictions: RestrictionDeclaration::ConfirmedNone {
                confirmed_at: Instant("2026-12-01T12:00:00Z".into()),
            },
            portion: c.household.diners[0].portions.clone(),
        }]),
    }));
    reproject(&mut c);
    let mut x = choice(&r);
    let guest = PersonRef::Guest {
        occasion_id: OccasionId(format!("oc_{:032x}", 1)),
        guest_id: GuestId(format!("gu_{:032x}", 1)),
    };
    x.meal_overrides.portions.0.push(PersonPortionOverride {
        person: guest.clone(),
        preset: PortionPresetName::Regular,
        adjustment: p(3, 2),
    });
    let selected = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &x,
        &reviewed,
        SelectionOrigin::Fixed,
    );
    assert_eq!(
        selected
            .servings
            .iter()
            .find(|p| p.person == guest)
            .unwrap()
            .servings,
        p(3, 2)
    );
    assert_eq!(c.household.diners.len(), 2);
    x.meal_overrides.portions[0].person = PersonRef::Guest {
        occasion_id: OccasionId(format!("oc_{:032x}", 2)),
        guest_id: GuestId(format!("gu_{:032x}", 1)),
    };
    assert!(
        assess_choice(
            &c,
            &c.weeks[0].cells[2].slot,
            &x,
            SelectionOrigin::Fixed,
            false,
            &reviewed
        )
        .is_err()
    );
}

#[test]
fn sc08_sc09_swaps_clear_only_named_slots_and_preserve_other_state() {
    let r = recipe();
    let (basis, classes) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &basis, &classes).unwrap()];
    let mut c = context(vec![r.clone()], 1);
    let s = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Fixed,
    );
    c.weeks[0].plan.selections.0.push(s);
    let mut other = build(
        &c,
        &c.weeks[0].cells[5].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Suggested,
    );
    other.repeat_authorised = true;
    c.weeks[0].plan.selections.0.push(other.clone());
    let p = proposal(&c, vec![None]);
    let patched =
        prepare_selection_patch(&c, &intent(&c, PlanMode::SwapNamed), &p, &reviewed, &[]).unwrap();
    assert_eq!(patched.context.weeks[0].plan.selections.0, vec![other]);
    assert_eq!(patched.context.household, c.household);
    assert_eq!(patched.context.history, c.history);
    assert_eq!(patched.context.weeks[0].revision, c.weeks[0].revision);
    assert_eq!(
        patched.context.weeks[0].defaults_snapshot,
        c.weeks[0].defaults_snapshot
    );
    assert!(
        prepare_selection_patch(
            &c,
            &intent(&c, PlanMode::ReplaceSuggestions),
            &p,
            &reviewed,
            &[]
        )
        .is_err()
    );
    c.weeks[0].plan.selections[0].origin = SelectionOrigin::Suggested;
    assert!(
        prepare_selection_patch(
            &c,
            &intent(&c, PlanMode::ReplaceSuggestions),
            &p,
            &reviewed,
            &[]
        )
        .is_ok()
    );
    let mut no_op = p;
    no_op.changes[0].replace_slots.0.clear();
    let patched =
        prepare_selection_patch(&c, &intent(&c, PlanMode::FillOpen), &no_op, &reviewed, &[])
            .unwrap();
    assert_eq!(patched.context, c);
    assert!(patched.changed_slots.is_empty());
}

#[test]
fn sc10_provider_cannot_promote_origin_or_repeat_permission() {
    let r = recipe();
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    let c = context(vec![r.clone()], 1);
    let mut s = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Fixed,
    );
    let i = intent(&c, PlanMode::FillOpen);
    assert!(
        prepare_selection_patch(&c, &i, &proposal(&c, vec![Some(s.clone())]), &reviewed, &[])
            .is_err()
    );
    s.origin = SelectionOrigin::Suggested;
    s.repeat_authorised = true;
    assert!(prepare_selection_patch(&c, &i, &proposal(&c, vec![Some(s)]), &reviewed, &[]).is_err());
}

#[test]
fn sc11_equal_totals_do_not_authorise_wrong_per_person_amounts() {
    let r = recipe();
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    let c = context(vec![r.clone()], 1);
    let mut x = choice(&r);
    x.meal_overrides.portions = BoundedVec(
        c.household
            .diners
            .iter()
            .enumerate()
            .map(|(i, d)| PersonPortionOverride {
                person: PersonRef::Diner {
                    diner_id: d.diner_id.clone(),
                },
                preset: PortionPresetName::Regular,
                adjustment: if i == 0 { p(3, 2) } else { p(1, 2) },
            })
            .collect(),
    );
    let mut s = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &x,
        &reviewed,
        SelectionOrigin::Suggested,
    );
    for serving in &mut s.servings.0 {
        serving.servings = p(1, 1);
    }
    assert!(
        matches!(prepare_selection_patch(&c,&intent(&c,PlanMode::FillOpen),&proposal(&c,vec![Some(s)]),&reviewed,&[]),Err(SelectionFailure::Invalid(e)) if e.code==ErrorCode::IdentityMismatch)
    );
}

#[test]
fn sc12_real_a3_repair_of_stale_attendance_preserves_full_context() {
    let r = recipe();
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    let mut c = context(vec![r.clone()], 1);
    let mut s = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Suggested,
    );
    c.weeks[0].plan.selections.0.push(s.clone());
    c.weeks[0].cells[2].attendee_ids.0.truncate(1);
    reproject(&mut c);
    assert!(c.validate_shape().is_err());
    assert!(assess_choice(&c, &s.slot, &choice(&r), s.origin, false, &reviewed).is_err());
    s.servings.0.truncate(1);
    let patched = prepare_selection_patch(
        &c,
        &intent(&c, PlanMode::SwapNamed),
        &proposal(&c, vec![Some(s)]),
        &reviewed,
        &[],
    )
    .unwrap();
    patched.context.validate_shape().unwrap();
    assert_eq!(patched.context.recipes, c.recipes);
    assert_eq!(patched.context.stamp, c.stamp);
    let mut untargeted = patched.context.weeks[0].plan.selections[0].clone();
    untargeted.slot = c.weeks[0].cells[5].slot.clone();
    c.weeks[0].plan.selections.0.push(untargeted);
    let replacement = patched.context.weeks[0].plan.selections[0].clone();
    assert!(
        prepare_selection_patch(
            &c,
            &intent(&c, PlanMode::SwapNamed),
            &proposal(&c, vec![Some(replacement)]),
            &reviewed,
            &[]
        )
        .is_err()
    );
}

#[test]
fn supplied_requirement_portions_are_not_authoritative() {
    let r = recipe();
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    let mut c = context(vec![r.clone()], 1);
    c.requirements[2].attendees[0].portion.adjustment = p(4, 1);
    c.validate_shape().unwrap();
    assert_eq!(
        assess_choice(
            &c,
            &c.weeks[0].cells[2].slot,
            &choice(&r),
            SelectionOrigin::Fixed,
            false,
            &reviewed
        )
        .unwrap_err()
        .code,
        ErrorCode::DomainConflict
    );
}

#[test]
fn sc13_all_write_weeks_exact_base_and_shape_required() {
    let r = recipe();
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    let c = context(vec![r.clone()], 2);
    let s = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Suggested,
    );
    let i = intent(&c, PlanMode::FillOpen);
    let mut p = proposal(&c, vec![Some(s)]);
    assert!(prepare_selection_patch(&c, &i, &p, &reviewed, &[]).is_err());
    p.changes.0.push(WeekChange {
        week_id: c.weeks[1].range.week_id.clone(),
        base_revision: c.weeks[1].revision,
        replace_slots: BoundedVec(vec![]),
    });
    assert_eq!(
        prepare_selection_patch(&c, &i, &p, &reviewed, &[])
            .unwrap()
            .changed_slots
            .len(),
        1
    );
    p.changes[0].base_revision = Revision(99);
    assert!(prepare_selection_patch(&c, &i, &p, &reviewed, &[]).is_err());
}

#[test]
fn sc14_eat_by_and_resolution_foreign_or_inactive_inputs_refuse() {
    let r = recipe();
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    let c = context(vec![r.clone()], 1);
    let mut x = choice(&r);
    x.meal_overrides.eat_by = c.requirements[5].eat_by.clone();
    assert!(
        assess_choice(
            &c,
            &c.weeks[0].cells[2].slot,
            &x,
            SelectionOrigin::Fixed,
            false,
            &reviewed
        )
        .is_err()
    );
    x.meal_overrides.eat_by = c.requirements[2].eat_by.clone();
    let s = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &x,
        &reviewed,
        SelectionOrigin::Suggested,
    );
    let resolution = mstrchf_core::availability::SlotResolution {
        slot: s.slot.clone(),
        resolution: Resolution::Unambiguous,
    };
    assert!(
        prepare_selection_patch(
            &c,
            &intent(&c, PlanMode::FillOpen),
            &proposal(&c, vec![Some(s)]),
            &reviewed,
            &[resolution]
        )
        .is_err()
    );
}

#[test]
fn sc15_leftover_assessment_is_provisional_and_patch_requires_retained_owner() {
    let r = recipe();
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    let c = context(vec![r.clone()], 1);
    let mut x = choice(&r);
    x.batch_use = BatchUse::Leftover {
        batch_id: BatchId(format!("ba_{:032x}", 99)),
        component_ids: BoundedVec(vec![r.components[0].component_id.clone()]),
    };
    let s = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &x,
        &reviewed,
        SelectionOrigin::Suggested,
    );
    assert!(
        matches!(prepare_selection_patch(&c,&intent(&c,PlanMode::FillOpen),&proposal(&c,vec![Some(s)]),&reviewed,&[]),Err(SelectionFailure::Invalid(e)) if e.code==ErrorCode::MissingReference)
    );
}

#[test]
fn sr06_selected_history_is_superseded_by_cleared_draft_but_orders_survive() {
    let r = recipe();
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    let mut c = context(vec![r.clone()], 2);
    let old = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Fixed,
    );
    c.history.selected.0.push(saved(&old));
    let s = build(
        &c,
        &c.weeks[1].cells[2].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Suggested,
    );
    let mut p = proposal(&c, vec![None, Some(s)]);
    p.changes[0].replace_slots.0.clear();
    assert!(
        prepare_selection_patch(&c, &intent(&c, PlanMode::FillOpen), &p, &reviewed, &[]).is_ok()
    );
    c.history.ordered.0.push(OrderedHistory {
        selection: saved(&old),
        order_id: OrderId(format!("or_{:032x}", 1)),
        ordered_at: Instant("2026-12-21T12:00:00Z".into()),
    });
    assert!(matches!(
        prepare_selection_patch(&c, &intent(&c, PlanMode::FillOpen), &p, &reviewed, &[]),
        Err(SelectionFailure::Blocked(_))
    ));
}

#[test]
fn sh07_distinct_timestamps_do_not_allow_duplicate_current_slot() {
    let r = recipe();
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    let mut c = context(vec![r.clone()], 1);
    let s = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Fixed,
    );
    let a = saved(&s);
    let mut b = a.clone();
    b.selected_at = Instant("2026-12-22T12:00:00Z".into());
    c.history.selected = BoundedVec(vec![a, b]);
    c.validate_shape().unwrap();
    let e = assess_choice(&c, &s.slot, &choice(&r), s.origin, false, &reviewed).unwrap_err();
    assert_eq!(e.code, ErrorCode::InvalidValue);
    assert_eq!(e.issues[0].path, "/history/selected");
    assert!(
        matches!(prepare_selection_patch(&c,&intent(&c,PlanMode::FillOpen),&proposal(&c,vec![Some(s)]),&reviewed,&[]),Err(SelectionFailure::Invalid(e)) if e.issues[0].path=="/history/selected")
    );
}

#[test]
fn public_bounds_and_reviewed_source_mismatch_fail_before_acceptance() {
    let r = recipe();
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    let c = context(vec![r.clone()], 1);
    let mut other = r.clone();
    other.revision = Revision(2);
    let (ob, ok) = inputs(&other);
    let wrong = vec![assess_revision(&other, &ob, &ok).unwrap()];
    assert_eq!(
        assess_choice(
            &c,
            &c.weeks[0].cells[2].slot,
            &choice(&r),
            SelectionOrigin::Fixed,
            false,
            &wrong
        )
        .unwrap_err()
        .code,
        ErrorCode::IdentityMismatch
    );
    let mut oversized = c.clone();
    oversized.recipes[0].ingredients.0 = vec![r.ingredients[0].clone(); 257];
    assert_eq!(
        assess_choice(
            &oversized,
            &c.weeks[0].cells[2].slot,
            &choice(&r),
            SelectionOrigin::Fixed,
            false,
            &reviewed
        )
        .unwrap_err()
        .code,
        ErrorCode::LimitExceeded
    );
    let mut i = intent(&c, PlanMode::FillOpen);
    i.slots = vec![i.slots[0].clone(); 85];
    assert!(
        matches!(prepare_selection_patch(&c,&i,&proposal(&c,vec![None]),&reviewed,&[]),Err(SelectionFailure::Invalid(e)) if e.code==ErrorCode::LimitExceeded)
    );
}

#[test]
fn explicit_null_is_not_an_authority_free_noop() {
    let c = context(vec![], 1);
    let i = intent(&c, PlanMode::FillOpen);
    let p = proposal(&c, vec![None]);
    assert!(prepare_selection_patch(&c, &i, &p, &[], &[]).is_err());
    let mut i = intent(&c, PlanMode::SwapNamed);
    i.slots = vec![c.weeks[0].cells[5].slot.clone()];
    assert!(prepare_selection_patch(&c, &i, &p, &[], &[]).is_err());
    let mut foreign = p;
    foreign.changes[0].replace_slots[0].slot.date = LocalDate("2027-01-11".into());
    assert!(prepare_selection_patch(&c, &i, &foreign, &[], &[]).is_err());
}

#[test]
fn sc04_extra_conflict_is_scoped_to_actual_recipients() {
    let mut base = recipe();
    let mut chicken = recipe();
    chicken.recipe_id = RecipeId(format!("rc_{:032x}", 2));
    chicken.dietary_evidence[0].verdict = DietaryVerdict::Conflicting;
    base.extras.0.push(ExtraSpec {
        extra_id: ExtraId(format!("ex_{:032x}", 1)),
        title: Title("Synthetic chicken".into()),
        recipe: reference(&chicken),
        eligible_person_requirements: BoundedVec(vec![]),
        supported_with: BoundedVec(vec![base.components[0].component_id.clone()]),
    });
    let (b, bc) = inputs(&base);
    let (e, ec) = inputs(&chicken);
    let reviewed = vec![
        assess_revision(&base, &b, &bc).unwrap(),
        assess_revision(&chicken, &e, &ec).unwrap(),
    ];
    let mut c = context(vec![base.clone(), chicken.clone()], 1);
    c.household.diners[0].restrictions = RestrictionDeclaration::Confirmed {
        confirmed_at: Instant("2026-12-01T12:00:00Z".into()),
        requirements: BoundedVec(vec![HardRequirement::Diet {
            code: DietCode::Vegetarian,
        }]),
    };
    reproject(&mut c);
    let people: Vec<_> = c
        .household
        .diners
        .iter()
        .map(|d| PersonRef::Diner {
            diner_id: d.diner_id.clone(),
        })
        .collect();
    let mut x = choice(&base);
    x.extras.push(ExtraChoice {
        extra_id: base.extras[0].extra_id.clone(),
        recipe: reference(&chicken),
        recipients: vec![people[1].clone()],
    });
    assert!(matches!(
        assess_choice(
            &c,
            &c.weeks[0].cells[2].slot,
            &x,
            SelectionOrigin::Fixed,
            false,
            &reviewed
        )
        .unwrap(),
        ChoiceAssessment::Suitable(_)
    ));
    x.extras[0].recipients.push(people[0].clone());
    assert!(matches!(
        assess_choice(
            &c,
            &c.weeks[0].cells[2].slot,
            &x,
            SelectionOrigin::Fixed,
            false,
            &reviewed
        )
        .unwrap(),
        ChoiceAssessment::Ineligible(_)
    ));
}

#[test]
fn sc05_child_preset_and_local_adjustment_are_evidenced_not_imputed() {
    let mut r = recipe();
    let mut child = r.portion_presets[0].clone();
    child.age_band = AgeBand::Child4To6;
    child.servings = p(1, 2);
    r.portion_presets.0.push(child);
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    let mut c = context(vec![r.clone()], 1);
    c.household.diners[1].age_band = AgeBand::Child4To6;
    reproject(&mut c);
    let mut x = choice(&r);
    x.meal_overrides.portions = BoundedVec(
        c.household
            .diners
            .iter()
            .enumerate()
            .map(|(i, d)| PersonPortionOverride {
                person: PersonRef::Diner {
                    diner_id: d.diner_id.clone(),
                },
                preset: PortionPresetName::Regular,
                adjustment: if i == 0 { p(3, 2) } else { p(2, 1) },
            })
            .collect(),
    );
    let s = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &x,
        &reviewed,
        SelectionOrigin::Fixed,
    );
    assert_eq!(s.servings[0].servings, p(3, 2));
    assert_eq!(s.servings[1].servings, p(1, 1));
    assert_eq!(s.servings[1].preset_ref.age_band, AgeBand::Child4To6);
    x.meal_overrides
        .portions
        .0
        .push(x.meal_overrides.portions[0].clone());
    assert!(
        assess_choice(
            &c,
            &c.weeks[0].cells[2].slot,
            &x,
            SelectionOrigin::Fixed,
            false,
            &reviewed
        )
        .is_err()
    );
}

#[test]
fn sr01_sr02_sr03_exact_repeat_permission_and_civil_week_boundaries() {
    let r = recipe();
    let mut newer = r.clone();
    newer.revision = Revision(2);
    let (b, k) = inputs(&r);
    let (nb, nk) = inputs(&newer);
    let reviewed = vec![
        assess_revision(&r, &b, &k).unwrap(),
        assess_revision(&newer, &nb, &nk).unwrap(),
    ];
    let mut c = context(vec![r.clone(), newer.clone()], 3);
    let old = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Fixed,
    );
    c.weeks[0].plan.selections.0.push(old);
    for target in [0, 1, 2] {
        let slot = c.weeks[target].cells[5].slot.clone();
        let s = build(
            &c,
            &slot,
            &choice(&newer),
            &reviewed,
            SelectionOrigin::Suggested,
        );
        let mut p = PlanProposal {
            changes: BoundedVec(
                c.weeks
                    .iter()
                    .map(|w| WeekChange {
                        week_id: w.range.week_id.clone(),
                        base_revision: w.revision,
                        replace_slots: BoundedVec(vec![]),
                    })
                    .collect(),
            ),
        };
        p.changes[target].replace_slots.0.push(ProposedSelection {
            slot: slot.clone(),
            selection: Nullable(Some(s)),
        });
        let mut i = intent(&c, PlanMode::SwapNamed);
        i.slots = vec![slot.clone()];
        let result = prepare_selection_patch(&c, &i, &p, &reviewed, &[]);
        assert_eq!(
            matches!(result, Err(SelectionFailure::Blocked(_))),
            target == 1,
            "target {target}: {result:?}"
        );
        if target == 1 {
            p.changes[target].replace_slots[0]
                .selection
                .0
                .as_mut()
                .unwrap()
                .repeat_authorised = true;
            i.repeat_permissions.push(RepeatPermission {
                slot: slot.clone(),
                recipe: reference(&r),
            });
            assert!(
                matches!(prepare_selection_patch(&c,&i,&p,&reviewed,&[]),Err(SelectionFailure::Invalid(e)) if e.code==ErrorCode::AuthorityDenied)
            );
            i.repeat_permissions[0].recipe = reference(&newer);
            assert!(prepare_selection_patch(&c, &i, &p, &reviewed, &[]).is_ok());
            i.repeat_permissions[0].slot = c.weeks[1].cells[2].slot.clone();
            i.slots.push(i.repeat_permissions[0].slot.clone());
            assert!(prepare_selection_patch(&c, &i, &p, &reviewed, &[]).is_err());
        }
    }
}

#[test]
fn sr04_breakfast_lunch_and_provisional_leftovers_are_repeat_exempt() {
    let mut r = recipe();
    r.meal_types = BoundedVec(vec![MealType::Breakfast, MealType::Dinner, MealType::Lunch]);
    for meal_type in [MealType::Breakfast, MealType::Lunch] {
        let mut preset = r.portion_presets[0].clone();
        preset.meal_type = meal_type;
        r.portion_presets.0.push(preset);
    }
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    for index in [0, 1, 2] {
        let mut c = context(vec![r.clone()], 2);
        let source = build(
            &c,
            &c.weeks[0].cells[index].slot,
            &choice(&r),
            &reviewed,
            SelectionOrigin::Fixed,
        );
        c.weeks[0].plan.selections.0.push(source);
        let mut x = choice(&r);
        if index == 2 {
            x.batch_use = BatchUse::Leftover {
                batch_id: BatchId(format!("ba_{:032x}", 1)),
                component_ids: BoundedVec(vec![r.components[0].component_id.clone()]),
            };
        }
        let slot = c.weeks[1].cells[index].slot.clone();
        let selected = build(&c, &slot, &x, &reviewed, SelectionOrigin::Suggested);
        let mut p = proposal(&c, vec![None, None]);
        for change in &mut p.changes.0 {
            change.replace_slots.0.clear();
        }
        p.changes[1].replace_slots.0.push(ProposedSelection {
            slot: slot.clone(),
            selection: Nullable(Some(selected)),
        });
        let mut i = intent(&c, PlanMode::FillOpen);
        i.slots = vec![slot];
        let result = prepare_selection_patch(&c, &i, &p, &reviewed, &[]).unwrap();
        assert_eq!(result.rederive_weeks.len(), if index == 2 { 2 } else { 1 });
    }
}

#[test]
fn sr07_supplied_boundary_history_catches_previous_civil_week() {
    let r = recipe();
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    let mut c = context(vec![r.clone()], 1);
    let selected = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Suggested,
    );
    let mut past = saved(&selected);
    past.week_id = WeekId(format!("wk_{:032x}", 77));
    past.slot.week_id = past.week_id.clone();
    past.slot.date = LocalDate("2026-12-22".into());
    c.history.selected.0.push(past);
    assert!(matches!(
        prepare_selection_patch(
            &c,
            &intent(&c, PlanMode::FillOpen),
            &proposal(&c, vec![Some(selected)]),
            &reviewed,
            &[]
        ),
        Err(SelectionFailure::Blocked(_))
    ));
}

#[test]
fn sr08_hidden_fixed_and_disabled_retention_is_unchanged() {
    let r = recipe();
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    let mut c = context(vec![r.clone()], 1);
    c.history.hidden_recipe_ids.0.push(r.recipe_id.clone());
    let fixed = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Fixed,
    );
    let mut i = intent(&c, PlanMode::FillOpen);
    i.origin = SelectionOrigin::Fixed;
    let patched = prepare_selection_patch(
        &c,
        &i,
        &proposal(&c, vec![Some(fixed.clone())]),
        &reviewed,
        &[],
    )
    .unwrap();
    assert_eq!(patched.context.weeks[0].plan.selections[0], fixed);
    let mut suggested = fixed.clone();
    suggested.origin = SelectionOrigin::Suggested;
    i.origin = SelectionOrigin::Suggested;
    assert!(
        prepare_selection_patch(&c, &i, &proposal(&c, vec![Some(suggested)]), &reviewed, &[])
            .is_err()
    );
    c.weeks[0].plan.selections.0.push(fixed);
    c.weeks[0].day_enabled[0] = false;
    reproject(&mut c);
    let mut no_op = proposal(&c, vec![None]);
    no_op.changes[0].replace_slots.0.clear();
    let retained = prepare_selection_patch(&c, &i, &no_op, &reviewed, &[]).unwrap();
    assert_eq!(retained.context, c);
    assert!(retained.changed_slots.is_empty());
    assert!(
        prepare_selection_patch(
            &c,
            &intent(&c, PlanMode::SwapNamed),
            &proposal(&c, vec![None]),
            &reviewed,
            &[]
        )
        .is_err()
    );
}

#[test]
fn sc15_source_replacement_preserves_links_and_reports_owners() {
    let r = recipe();
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    let mut c = context(vec![r.clone()], 1);
    let source = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Fixed,
    );
    let mut x = choice(&r);
    x.batch_use = BatchUse::Leftover {
        batch_id: BatchId(format!("ba_{:032x}", 1)),
        component_ids: BoundedVec(vec![r.components[0].component_id.clone()]),
    };
    let target = build(
        &c,
        &c.weeks[0].cells[5].slot,
        &x,
        &reviewed,
        SelectionOrigin::Fixed,
    );
    c.weeks[0].plan.batch_links.0.push(BatchLink {
        batch_id: BatchId(format!("ba_{:032x}", 1)),
        source_slot: source.slot.clone(),
        target_slot: target.slot.clone(),
        components: BoundedVec(vec![ComponentAllocation {
            component_id: r.components[0].component_id.clone(),
            quantity: Quantity {
                amount: p(1, 1),
                unit: Unit::G,
                form: IngredientForm::Raw,
            },
        }]),
    });
    c.weeks[0].plan.selections = BoundedVec(vec![source, target]);
    let patched = prepare_selection_patch(
        &c,
        &intent(&c, PlanMode::SwapNamed),
        &proposal(&c, vec![None]),
        &reviewed,
        &[],
    )
    .unwrap();
    assert_eq!(
        patched.context.weeks[0].plan.batch_links,
        c.weeks[0].plan.batch_links
    );
    assert_eq!(
        patched.rederive_weeks,
        vec![c.weeks[0].range.week_id.clone()]
    );
}

#[test]
fn sc14_retained_fold_override_survives_unrelated_patch() {
    let r = recipe();
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    let mut c = context(vec![r.clone()], 1);
    let range = WeekRange {
        week_id: c.weeks[0].range.week_id.clone(),
        monday: LocalDate("2026-10-19".into()),
        zone: ZoneId("Europe/London".into()),
    };
    c.weeks[0] = mstrchf_core::weeks::new_week(
        &c.household,
        &c.privacy,
        &Instant("2026-12-21T12:00:00Z".into()),
        &range,
    )
    .unwrap();
    reproject(&mut c);
    let mut x = choice(&r);
    let fold_slot = c.weeks[0].cells[20].slot.clone();
    x.meal_overrides.eat_by = Nullable(Some(
        mstrchf_core::availability::resolve_meal_time(
            &fold_slot.date,
            &LocalTime("01:30:00".into()),
            &range.zone,
            Some(Resolution::Later),
        )
        .unwrap(),
    ));
    let fold = build(&c, &fold_slot, &x, &reviewed, SelectionOrigin::Fixed);
    c.weeks[0].plan.selections.0.push(fold.clone());
    reproject(&mut c);
    let s = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Suggested,
    );
    let patched = prepare_selection_patch(
        &c,
        &intent(&c, PlanMode::FillOpen),
        &proposal(&c, vec![Some(s)]),
        &reviewed,
        &[],
    )
    .unwrap();
    assert_eq!(
        patched.context.weeks[0]
            .plan
            .selections
            .iter()
            .find(|s| s.slot == fold_slot),
        Some(&fold)
    );
    assert_eq!(
        patched
            .context
            .requirements
            .iter()
            .find(|r| r.slot == fold_slot)
            .unwrap()
            .eat_by,
        fold.meal_overrides.eat_by
    );
}

#[test]
fn sc13_duplicate_and_foreign_patch_entries_refuse_atomically() {
    let r = recipe();
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    let c = context(vec![r.clone()], 1);
    let s = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Suggested,
    );
    let i = intent(&c, PlanMode::FillOpen);
    let original = proposal(&c, vec![Some(s)]);
    let mut duplicate = original.clone();
    duplicate.changes.0.push(duplicate.changes[0].clone());
    assert!(prepare_selection_patch(&c, &i, &duplicate, &reviewed, &[]).is_err());
    let mut duplicate = original.clone();
    let repeated_slot = duplicate.changes[0].replace_slots[0].clone();
    duplicate.changes[0].replace_slots.0.push(repeated_slot);
    assert!(prepare_selection_patch(&c, &i, &duplicate, &reviewed, &[]).is_err());
    let mut foreign = original.clone();
    foreign.changes[0].replace_slots[0]
        .selection
        .0
        .as_mut()
        .unwrap()
        .recipe
        .recipe_id = RecipeId(format!("rc_{:032x}", 99));
    assert!(prepare_selection_patch(&c, &i, &foreign, &reviewed, &[]).is_err());
    assert!(c.weeks[0].plan.selections.is_empty());
    assert!(prepare_selection_patch(&c, &i, &original, &reviewed, &[]).is_ok());
}

#[test]
fn sc01_manual_and_suggested_have_identical_real_portions() {
    let r = recipe();
    let (basis, classes) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &basis, &classes).unwrap()];
    let c = context(vec![r.clone()], 1);
    c.validate_shape().unwrap();
    let slot = &c.weeks[0].cells[2].slot;
    let a = build(&c, slot, &choice(&r), &reviewed, SelectionOrigin::Fixed);
    let b = build(&c, slot, &choice(&r), &reviewed, SelectionOrigin::Suggested);
    assert_eq!(a.servings, b.servings);
    assert_eq!(a.servings.len(), 2);
    assert_eq!(a.servings[0].servings, p(1, 1));
    assert_eq!(a.servings[1].servings, p(1, 1));
}

#[test]
fn sc07_fill_open_cannot_replace_fixed_or_suggested_or_hidden() {
    let r = recipe();
    let (basis, classes) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &basis, &classes).unwrap()];
    for origin in [SelectionOrigin::Fixed, SelectionOrigin::Suggested] {
        let mut c = context(vec![r.clone()], 1);
        let slot = c.weeks[0].cells[2].slot.clone();
        let s = build(&c, &slot, &choice(&r), &reviewed, origin);
        c.weeks[0].plan.selections.0.push(s.clone());
        c.history.hidden_recipe_ids.0.push(r.recipe_id.clone());
        let mut changed = s;
        changed.servings[0].servings = p(3, 2);
        assert!(
            prepare_selection_patch(
                &c,
                &intent(&c, PlanMode::FillOpen),
                &proposal(&c, vec![Some(changed)]),
                &reviewed,
                &[]
            )
            .is_err()
        );
        assert!(
            prepare_selection_patch(
                &c,
                &intent(&c, PlanMode::FillOpen),
                &proposal(&c, vec![None]),
                &reviewed,
                &[]
            )
            .is_err()
        );
    }
}

#[test]
fn sr05_simultaneous_cross_week_repeat_is_checked_on_complete_candidate() {
    let r = recipe();
    let (basis, classes) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &basis, &classes).unwrap()];
    let c = context(vec![r.clone()], 2);
    let selections: Vec<_> = c
        .weeks
        .iter()
        .map(|w| {
            Some(build(
                &c,
                &w.cells[2].slot,
                &choice(&r),
                &reviewed,
                SelectionOrigin::Suggested,
            ))
        })
        .collect();
    let mut proposal = proposal(&c, selections);
    let mut i = intent(&c, PlanMode::FillOpen);
    assert!(matches!(
        prepare_selection_patch(&c, &i, &proposal, &reviewed, &[]),
        Err(SelectionFailure::Blocked(_))
    ));
    proposal.changes.0.reverse();
    assert!(matches!(
        prepare_selection_patch(&c, &i, &proposal, &reviewed, &[]),
        Err(SelectionFailure::Blocked(_))
    ));
    for ch in &mut proposal.changes.0 {
        let s = ch.replace_slots[0].selection.0.as_mut().unwrap();
        s.repeat_authorised = true;
        i.repeat_permissions.push(RepeatPermission {
            slot: s.slot.clone(),
            recipe: s.recipe.clone(),
        });
    }
    let patched = prepare_selection_patch(&c, &i, &proposal, &reviewed, &[]).unwrap();
    assert_eq!(patched.changed_slots.len(), 2);
    assert_eq!(patched.context.stamp, c.stamp);
}
// Reviewer-owned probes; original implementation and expected fixtures unchanged.

#[test]
fn review_untouched_active_portions_must_match_real_food_b() {
    let r = recipe();
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    let mut c = context(vec![r.clone()], 1);
    let mut old = build(
        &c,
        &c.weeks[0].cells[5].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Fixed,
    );
    old.servings[0].servings = p(3, 2); // no matching override: structurally valid, wrong Food B quantity
    c.weeks[0].plan.selections.0.push(old.clone());
    c.validate_shape().unwrap();
    let replacement = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Suggested,
    );
    let result = prepare_selection_patch(
        &c,
        &intent(&c, PlanMode::FillOpen),
        &proposal(&c, vec![Some(replacement)]),
        &reviewed,
        &[],
    );
    assert!(
        result.is_err(),
        "untouched invalid native serving survived complete candidate checks"
    );
}

#[test]
fn review_untouched_active_hard_conflict_must_block_complete_candidate() {
    let r = recipe();
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    let mut c = context(vec![r.clone()], 1);
    let old = build(
        &c,
        &c.weeks[0].cells[5].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Fixed,
    );
    c.weeks[0].plan.selections.0.push(old);
    c.household.diners[0].restrictions = RestrictionDeclaration::Confirmed {
        confirmed_at: Instant("2026-12-01T12:00:00Z".into()),
        requirements: BoundedVec(vec![HardRequirement::IngredientExclusion {
            ingredient_id: r.ingredients[0].ingredient_id.clone(),
        }]),
    };
    // The target is attended only by the unrestricted diner; the retained other meal remains unsafe.
    c.weeks[0].cells[2].attendee_ids.0.remove(0);
    reproject(&mut c);
    c.validate_shape().unwrap();
    assert!(matches!(
        assess_choice(
            &c,
            &c.weeks[0].cells[5].slot,
            &choice(&r),
            SelectionOrigin::Fixed,
            false,
            &reviewed
        )
        .unwrap(),
        ChoiceAssessment::Ineligible(_)
    ));
    let replacement = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Suggested,
    );
    let result = prepare_selection_patch(
        &c,
        &intent(&c, PlanMode::FillOpen),
        &proposal(&c, vec![Some(replacement)]),
        &reviewed,
        &[],
    );
    assert!(
        result.is_err(),
        "untouched known hard conflict survived complete candidate checks"
    );
}

#[test]
fn review_disabled_retained_bad_portion_does_not_block_unrelated_active_patch() {
    let r = recipe();
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    let mut c = context(vec![r.clone()], 1);
    let mut old = build(
        &c,
        &c.weeks[0].cells[5].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Fixed,
    );
    old.servings[0].servings = p(3, 2);
    c.weeks[0].plan.selections.0.push(old.clone());
    c.weeks[0].day_enabled[1] = false;
    reproject(&mut c);
    let replacement = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Suggested,
    );
    let patch = prepare_selection_patch(
        &c,
        &intent(&c, PlanMode::FillOpen),
        &proposal(&c, vec![Some(replacement)]),
        &reviewed,
        &[],
    )
    .unwrap();
    assert_eq!(patch.context.weeks[0].plan.selections[0], old);
}

#[test]
fn r2_retained_repeats_are_rechecked_for_food_without_new_repeat_permission() {
    let r = recipe();
    let (b, k) = inputs(&r);
    let reviewed = vec![assess_revision(&r, &b, &k).unwrap()];
    let mut c = context(vec![r.clone()], 2);
    for index in 0..2 {
        let retained = build(
            &c,
            &c.weeks[index].cells[5].slot,
            &choice(&r),
            &reviewed,
            SelectionOrigin::Fixed,
        );
        assert!(!retained.repeat_authorised);
        c.weeks[index].plan.selections.0.push(retained);
    }
    let mut replacement = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &choice(&r),
        &reviewed,
        SelectionOrigin::Suggested,
    );
    replacement.repeat_authorised = true;
    let mut action = intent(&c, PlanMode::FillOpen);
    action.repeat_permissions.push(RepeatPermission {
        slot: replacement.slot.clone(),
        recipe: replacement.recipe.clone(),
    });
    let mut patch = proposal(&c, vec![Some(replacement), None]);
    patch.changes[1].replace_slots.0.clear();
    let result = prepare_selection_patch(&c, &action, &patch, &reviewed, &[]).unwrap();
    assert_eq!(result.changed_slots, vec![c.weeks[0].cells[2].slot.clone()]);
    for index in 0..2 {
        assert_eq!(
            result.context.weeks[index].plan.selections[0],
            c.weeks[index].plan.selections[0]
        );
    }
}

#[test]
fn r2_retained_extra_portions_and_source_closure_are_checked() {
    let mut base = recipe();
    let mut extra = recipe();
    extra.recipe_id = RecipeId(format!("rc_{:032x}", 2));
    base.extras.0.push(ExtraSpec {
        extra_id: ExtraId(format!("ex_{:032x}", 1)),
        title: Title("Synthetic extra".into()),
        recipe: reference(&extra),
        eligible_person_requirements: BoundedVec(vec![]),
        supported_with: BoundedVec(vec![base.components[0].component_id.clone()]),
    });
    let (b, bc) = inputs(&base);
    let (e, ec) = inputs(&extra);
    let reviewed = vec![
        assess_revision(&base, &b, &bc).unwrap(),
        assess_revision(&extra, &e, &ec).unwrap(),
    ];
    let mut c = context(vec![base.clone(), extra.clone()], 1);
    let mut configured = choice(&base);
    configured.extras.push(ExtraChoice {
        extra_id: base.extras[0].extra_id.clone(),
        recipe: reference(&extra),
        recipients: vec![PersonRef::Diner {
            diner_id: c.household.diners[0].diner_id.clone(),
        }],
    });
    let retained = build(
        &c,
        &c.weeks[0].cells[5].slot,
        &configured,
        &reviewed,
        SelectionOrigin::Fixed,
    );
    c.weeks[0].plan.selections.0.push(retained);
    let replacement = build(
        &c,
        &c.weeks[0].cells[2].slot,
        &choice(&base),
        &reviewed,
        SelectionOrigin::Suggested,
    );
    let patch = proposal(&c, vec![Some(replacement)]);
    let action = intent(&c, PlanMode::FillOpen);
    assert!(prepare_selection_patch(&c, &action, &patch, &reviewed, &[]).is_ok());
    assert!(
        matches!(prepare_selection_patch(&c, &action, &patch, &reviewed[..1], &[]), Err(SelectionFailure::Invalid(e)) if e.code == ErrorCode::MissingReference)
    );
    c.weeks[0].plan.selections[0].extras[0].recipients[0].servings = p(3, 2);
    c.validate_shape().unwrap();
    assert!(
        matches!(prepare_selection_patch(&c, &action, &patch, &reviewed, &[]), Err(SelectionFailure::Invalid(e)) if e.code == ErrorCode::IdentityMismatch)
    );
}
