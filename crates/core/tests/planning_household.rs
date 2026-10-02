use mstrchf_contracts::*;
use mstrchf_core::household::*;

fn profile() -> HouseholdProfile {
    parse_document(include_bytes!(
        "../../../tests/fixtures/households/p03ab/a1a2-profile.json"
    ))
    .unwrap()
}
fn draft() -> HouseholdDraft {
    parse_document(include_bytes!(
        "../../../tests/fixtures/households/p03ab/a1a2-draft.json"
    ))
    .unwrap()
}
fn empty() -> HouseholdDraft {
    parse_document(include_bytes!(
        "../../../tests/fixtures/households/p03ab/a1a2-empty-draft.json"
    ))
    .unwrap()
}
fn week() -> Week {
    parse_document(include_bytes!(
        "../../../tests/fixtures/households/p03ab/a1a2-week.json"
    ))
    .unwrap()
}
fn time() -> Instant {
    Instant("2026-12-21T12:00:00Z".into())
}
fn policy() -> PrivacyPolicy {
    profile().connections.privacy
}
fn id(n: u8) -> DinerId {
    DinerId(format!("dn_{n:032x}"))
}
fn none() -> RestrictionDeclaration {
    RestrictionDeclaration::ConfirmedNone {
        confirmed_at: Instant("2026-12-01T12:00:00Z".into()),
    }
}
fn gluten_free() -> RestrictionDeclaration {
    RestrictionDeclaration::Confirmed {
        confirmed_at: time(),
        requirements: BoundedVec(vec![HardRequirement::Diet {
            code: DietCode::GlutenFree,
        }]),
    }
}
fn error<T: std::fmt::Debug>(r: Result<T, DomainError>, code: ErrorCode) -> DomainError {
    let e = r.unwrap_err();
    assert_eq!(e.code, code, "{e:?}");
    assert!(!e.issues.is_empty());
    assert!(e.issues.len() <= 100);
    e
}
fn unchanged_reject(p: HouseholdProfile, code: ErrorCode) {
    let before = p.clone();
    error(validate_profile(&p, &policy(), &time()), code);
    assert_eq!(p, before);
}
fn update(
    current: &HouseholdProfile,
    proposed: &HouseholdProfile,
    weeks: &[Week],
) -> Result<ProfileChange, DomainError> {
    prepare_profile_update(current, proposed, Revision(3), &policy(), &time(), weeks)
}
fn selection() -> MealSelection {
    let w: Week = parse_document(include_bytes!(
        "../../../tests/fixtures/contracts/ruling004-current--week-disabled-retained.json"
    ))
    .unwrap();
    let mut s = w.plan.selections[0].clone();
    s.slot.date = LocalDate("2026-12-29".into());
    s.slot.meal_type = MealType::Lunch;
    s.servings.0[0].preset_ref.meal_type = MealType::Lunch;
    s
}

#[test]
fn h01_empty_progress_does_not_complete() {
    let e = empty();
    let before = e.clone();
    assert!(validate_draft_progress(&e, &policy(), &time()).is_ok());
    error(
        profile_from_draft(&e, &policy(), &time()),
        ErrorCode::MissingField,
    );
    assert_eq!(e, before);
    let required: Vec<String> = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/households/p03ab/a1a2-required-case-ids.json"
    ))
    .unwrap();
    let manifest: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/households/p03ab/a1a2-manifest.json"
    ))
    .unwrap();
    let expected = (1..=14)
        .map(|i| format!("H{i:02}"))
        .chain(["R05".into()])
        .chain((1..=12).map(|i| format!("W{i:02}")))
        .collect::<Vec<_>>();
    assert_eq!(required, expected);
    assert_eq!(
        manifest["cases"]
            .as_array()
            .unwrap()
            .iter()
            .map(|c| c["case_id"].as_str().unwrap())
            .collect::<Vec<_>>(),
        expected.iter().map(String::as_str).collect::<Vec<_>>()
    );
}
#[test]
fn h02_unconfirmed_household_stays_incomplete() {
    let mut d = empty();
    let mut diner = profile().diners[0].clone();
    diner.restrictions = RestrictionDeclaration::Unconfirmed {};
    d.diners.0.push(diner);
    validate_draft_progress(&d, &policy(), &time()).unwrap();
    error(
        profile_from_draft(&d, &policy(), &time()),
        ErrorCode::DomainConflict,
    );
    d.completed_stages.0.push(HouseholdStage::Household);
    let before = d.clone();
    error(
        validate_draft_progress(&d, &policy(), &time()),
        ErrorCode::InvalidValue,
    );
    assert_eq!(d, before);
}
#[test]
fn h03_confirmed_none_optional_names_and_separate_revision() {
    let mut d = draft();
    for diner in &mut d.diners.0 {
        diner.restrictions = none();
    }
    let before = d.clone();
    let p = profile_from_draft(&d, &policy(), &time()).unwrap();
    assert_eq!(p.revision, Revision(1));
    assert_eq!(d.revision, Revision(7));
    assert_eq!(p.household_id, d.household_id);
    assert_eq!(p.diners.0, d.diners.0);
    assert_eq!(p.diners.len(), 2);
    assert!(
        p.diners
            .iter()
            .all(|d| d.display_name.0.is_none() && d.restrictions == none())
    );
    assert_eq!(d, before);
}
#[test]
fn h04_promotion_preserves_every_supplied_field() {
    let d = draft();
    let before = d.clone();
    let mut expected = profile();
    expected.revision = Revision(1);
    assert_eq!(
        profile_from_draft(&d, &policy(), &time()).unwrap(),
        expected
    );
    assert_eq!(d, before);
    assert!(!expected.connections.privacy.use_ai_features);
    assert!(!expected.connections.privacy.include_household_requirements);
    assert_eq!(expected.connections.privacy.confirmed_at, Nullable(None));
}
#[test]
fn h05_each_absent_stage_is_valid_progress_and_missing_completion() {
    for stage in [
        HouseholdStage::FoodPreferences,
        HouseholdStage::UsualWeek,
        HouseholdStage::Connections,
    ] {
        let mut d = draft();
        match stage {
            HouseholdStage::FoodPreferences => d.preferences = Nullable(None),
            HouseholdStage::UsualWeek => d.usual_week = Nullable(None),
            _ => d.connections = Nullable(None),
        }
        d.completed_stages.0.retain(|s| *s != stage);
        let before = d.clone();
        validate_draft_progress(&d, &policy(), &time()).unwrap();
        error(
            profile_from_draft(&d, &policy(), &time()),
            ErrorCode::MissingField,
        );
        assert_eq!(d, before);
        d.completed_stages.0.push(stage);
        assert!(validate_draft_progress(&d, &policy(), &time()).is_err());
    }
}
#[test]
fn h06_connections_can_be_supplied_out_of_sequence() {
    let mut d = empty();
    d.connections = draft().connections;
    d.completed_stages = BoundedVec(vec![HouseholdStage::Connections]);
    let before = d.clone();
    validate_draft_progress(&d, &policy(), &time()).unwrap();
    error(
        profile_from_draft(&d, &policy(), &time()),
        ErrorCode::MissingField,
    );
    assert_eq!(d, before);
}
#[test]
fn h07_direct_invalid_values_are_rejected_without_mutation_or_panic() {
    let mut p = profile();
    p.revision = Revision(0);
    unchanged_reject(p, ErrorCode::NoncanonicalValue);
    let mut p = profile();
    p.household_id = HouseholdId("bad".into());
    unchanged_reject(p, ErrorCode::InvalidValue);
    let mut p = profile();
    p.diners.0[0].portions.adjustment = PositiveRational(Rational {
        numerator: DecimalU64(0),
        denominator: DecimalU64(1),
    });
    unchanged_reject(p, ErrorCode::InvalidValue);
    let mut p = profile();
    p.diners.0[0].portions.adjustment = PositiveRational(Rational {
        numerator: DecimalU64(2),
        denominator: DecimalU64(2),
    });
    unchanged_reject(p, ErrorCode::NoncanonicalValue);
    let mut p = profile();
    p.usual_week.day_enabled.0.pop();
    unchanged_reject(p, ErrorCode::InvalidValue);
    let mut p = profile();
    p.diners.0.push(p.diners[0].clone());
    unchanged_reject(p, ErrorCode::InvalidValue);
    let mut p = profile();
    let o = MealPortionOverride {
        meal_type: MealType::Lunch,
        preset: PortionPresetName::Regular,
        adjustment: PositiveRational::new(1, 1).unwrap(),
    };
    p.diners.0[0].portions.meal_overrides = BoundedVec(vec![o.clone(), o]);
    unchanged_reject(p, ErrorCode::InvalidValue);
    let mut p = profile();
    p.preferences.meal_times.0[1] = p.preferences.meal_times[0].clone();
    unchanged_reject(p, ErrorCode::InvalidValue);
    let mut p = profile();
    p.usual_week.cells.0[1] = p.usual_week.cells[0].clone();
    unchanged_reject(p, ErrorCode::InvalidValue);
    let mut d = draft();
    d.revision = Revision(0);
    let before = d.clone();
    error(
        validate_draft_progress(&d, &policy(), &time()),
        ErrorCode::NoncanonicalValue,
    );
    assert_eq!(d, before);
    let mut p = profile();
    for diner in &mut p.diners.0 {
        diner.diner_id = DinerId("bad".into());
    }
    for c in &mut p.usual_week.cells.0 {
        c.attendee_ids = BoundedVec((0..64).map(|_| DinerId("bad".into())).collect());
    }
    let e = validate_profile(&p, &policy(), &time()).unwrap_err();
    assert_eq!(e.issues.len(), 100);
    assert!(e.truncated);
}
#[test]
fn h08_restrictions_and_evaluation_timestamps_are_admitted() {
    let mut p = profile();
    p.diners.0[0].restrictions = RestrictionDeclaration::Confirmed {
        confirmed_at: time(),
        requirements: BoundedVec(vec![]),
    };
    unchanged_reject(p, ErrorCode::InvalidValue);
    let mut p = profile();
    p.diners.0[0].restrictions = RestrictionDeclaration::Unconfirmed {};
    unchanged_reject(p, ErrorCode::InvalidValue);
    for at in ["2026-12-21T12:00:01Z", "bad"] {
        let mut p = profile();
        p.diners.0[0].restrictions = RestrictionDeclaration::ConfirmedNone {
            confirmed_at: Instant(at.into()),
        };
        unchanged_reject(p, ErrorCode::InvalidValue);
        let mut d = draft();
        d.diners.0[0].restrictions = RestrictionDeclaration::ConfirmedNone {
            confirmed_at: Instant(at.into()),
        };
        let before = d.clone();
        error(
            validate_draft_progress(&d, &policy(), &time()),
            ErrorCode::InvalidValue,
        );
        assert_eq!(d, before);
    }
    let p = profile();
    error(
        validate_profile(&p, &policy(), &Instant("bad".into())),
        ErrorCode::InvalidValue,
    );
    let mut p = profile();
    p.diners.0[0].restrictions = RestrictionDeclaration::ConfirmedNone {
        confirmed_at: time(),
    };
    validate_profile(&p, &policy(), &time()).unwrap();
}
#[test]
fn h09_usual_attendees_close_against_present_household() {
    let mut p = profile();
    p.usual_week.cells.0[4].attendee_ids.0.push(id(3));
    unchanged_reject(p, ErrorCode::ForeignReference);
    let mut p = profile();
    p.usual_week.cells.0[4].attendee_ids.0.clear();
    unchanged_reject(p.clone(), ErrorCode::InvalidValue);
    p.usual_week.day_enabled.0[1] = false;
    validate_profile(&p, &policy(), &time()).unwrap();
    p.usual_week.day_enabled.0[1] = true;
    p.usual_week.cells.0[4].choice = MealLocation::Skip;
    validate_profile(&p, &policy(), &time()).unwrap();
    let mut d = draft();
    d.usual_week.0.as_mut().unwrap().cells.0[4]
        .attendee_ids
        .0
        .push(id(3));
    error(
        validate_draft_progress(&d, &policy(), &time()),
        ErrorCode::ForeignReference,
    );
}
#[test]
fn h10_authoritative_policy_must_match_in_every_field() {
    for i in 0..4 {
        let mut d = draft();
        let privacy = &mut d.connections.0.as_mut().unwrap().privacy;
        match i {
            0 => privacy.revision = Revision(2),
            1 => privacy.use_ai_features = true,
            2 => privacy.include_household_requirements = true,
            _ => privacy.confirmed_at = Nullable(Some(time())),
        }
        let before = d.clone();
        error(
            profile_from_draft(&d, &policy(), &time()),
            ErrorCode::ContextStale,
        );
        assert_eq!(d, before);
        let mut p = profile();
        p.connections = d.connections.0.unwrap();
        unchanged_reject(p, ErrorCode::ContextStale);
    }
    let mut bad = policy();
    bad.revision = Revision(0);
    error(
        validate_profile(&profile(), &bad, &time()),
        ErrorCode::NoncanonicalValue,
    );
}
#[test]
fn h11_update_preserves_identity_and_current_revision() {
    let p = profile();
    let before = p.clone();
    error(
        prepare_profile_update(&p, &p, Revision(2), &policy(), &time(), &[]),
        ErrorCode::ContextStale,
    );
    error(
        prepare_profile_update(&p, &p, Revision(0), &policy(), &time(), &[]),
        ErrorCode::NoncanonicalValue,
    );
    let mut q = p.clone();
    q.revision = Revision(4);
    error(update(&p, &q, &[]), ErrorCode::ContextStale);
    q = p.clone();
    q.household_id = HouseholdId(format!("hh_{:032x}", 2));
    error(update(&p, &q, &[]), ErrorCode::IdentityMismatch);
    let change = update(&p, &p, &[]).unwrap();
    assert_eq!(change.candidate, p);
    assert_eq!(change.candidate.revision, Revision(3));
    assert!(change.changed_hard_diner_ids.is_empty());
    assert_eq!(p, before);
    let w = week();
    error(
        update(&p, &p, &[w.clone(), w.clone()]),
        ErrorCode::InvalidValue,
    );
    error(
        update(&p, &p, &vec![w.clone(); 5]),
        ErrorCode::LimitExceeded,
    );
    let mut malformed = w.clone();
    malformed.day_enabled.0.clear();
    error(update(&p, &p, &[malformed]), ErrorCode::InvalidValue);
    let weeks = [w];
    let saved = weeks.clone();
    update(&p, &p, &weeks).unwrap();
    assert_eq!(weeks, saved);
}
#[test]
fn h12_only_semantic_hard_changes_are_reported_sorted() {
    let p = profile();
    let mut q = p.clone();
    q.diners.0[0].restrictions = gluten_free();
    let before = q.clone();
    let change = update(&p, &q, &[]).unwrap();
    assert_eq!(change.changed_hard_diner_ids, vec![id(1)]);
    assert_eq!(change.candidate, q);
    assert_eq!(q, before);
    q = p.clone();
    q.diners.0[0].restrictions = RestrictionDeclaration::ConfirmedNone {
        confirmed_at: time(),
    };
    assert!(
        update(&p, &q, &[])
            .unwrap()
            .changed_hard_diner_ids
            .is_empty()
    );
    q = p.clone();
    q.diners.0[0].display_name = Nullable(Some(NameText("Changed".into())));
    q.diners.0[0]
        .dislikes
        .0
        .push(IngredientId(format!("ig_{:032x}", 1)));
    assert!(
        update(&p, &q, &[])
            .unwrap()
            .changed_hard_diner_ids
            .is_empty()
    );
    q = p.clone();
    q.diners.0[0].restrictions = gluten_free();
    q.diners.0[1].restrictions = none();
    q.diners.0.reverse();
    assert_eq!(
        update(&p, &q, &[]).unwrap().changed_hard_diner_ids,
        vec![id(1), id(2)]
    );
    q = p.clone();
    let mut added = q.diners[0].clone();
    added.diner_id = id(3);
    q.diners.0.push(added);
    let weeks = [week()];
    let saved = weeks.clone();
    let change = update(&p, &q, &weeks).unwrap();
    assert_eq!(change.candidate.usual_week, p.usual_week);
    assert_eq!(weeks, saved);
    let mut before = p.clone();
    before.diners.0[0].restrictions = RestrictionDeclaration::Confirmed {
        confirmed_at: time(),
        requirements: BoundedVec(vec![
            HardRequirement::Diet {
                code: DietCode::GlutenFree,
            },
            HardRequirement::Diet {
                code: DietCode::Vegetarian,
            },
        ]),
    };
    let mut after = before.clone();
    if let RestrictionDeclaration::Confirmed { requirements, .. } =
        &mut after.diners.0[0].restrictions
    {
        requirements.0.reverse();
    }
    assert!(
        update(&before, &after, &[])
            .unwrap()
            .changed_hard_diner_ids
            .is_empty()
    );
    // The delta itself is bounded even when two individually valid profiles differ entirely.
    let mut before = p.clone();
    before.diners.0 = (1..=64)
        .map(|n| {
            let mut d = p.diners[0].clone();
            d.diner_id = id(n);
            d.restrictions = gluten_free();
            d
        })
        .collect();
    let mut after = before.clone();
    for d in &mut after.diners.0 {
        d.restrictions = none();
    }
    assert_eq!(
        update(&before, &after, &[])
            .unwrap()
            .changed_hard_diner_ids
            .len(),
        64
    );
    for (n, d) in after.diners.0.iter_mut().enumerate() {
        d.diner_id = id(n as u8 + 65);
        d.restrictions = gluten_free();
    }
    for c in &mut after.usual_week.cells.0 {
        c.attendee_ids = BoundedVec(vec![id(65)]);
    }
    error(update(&before, &after, &[]), ErrorCode::LimitExceeded);
}
#[test]
fn h13_independent_priorities_and_harness_do_not_enable_ai() {
    for flags in 0..8 {
        let mut d = draft();
        let prefs = d.preferences.0.as_mut().unwrap();
        prefs.spending_style = SpendingStyle::Value;
        prefs.higher_protein = flags & 1 != 0;
        prefs.more_variety = flags & 2 != 0;
        prefs.quick_meals = flags & 4 != 0;
        d.connections.0.as_mut().unwrap().harness = Nullable(Some(Provider::CodexLocal));
        let p = profile_from_draft(&d, &policy(), &time()).unwrap();
        assert_eq!(Some(p.preferences), d.preferences.0);
        assert_eq!(p.connections.privacy, policy());
        assert_eq!(p.connections.harness, Nullable(Some(Provider::CodexLocal)));
    }
}
#[test]
fn h14_reminders_require_explicit_channels_and_prompts() {
    let mut d = draft();
    let r = &mut d.connections.0.as_mut().unwrap().reminders;
    r.enabled = true;
    r.prompts = BoundedVec(vec![ReminderPrompt::UnfinishedPlan]);
    error(
        profile_from_draft(&d, &policy(), &time()),
        ErrorCode::InvalidValue,
    );
    let r = &mut d.connections.0.as_mut().unwrap().reminders;
    r.channels = BoundedVec(vec![ReminderChannel::Local]);
    r.prompts.0.clear();
    error(
        profile_from_draft(&d, &policy(), &time()),
        ErrorCode::InvalidValue,
    );
    d.connections.0.as_mut().unwrap().reminders.prompts =
        BoundedVec(vec![ReminderPrompt::UnfinishedPlan]);
    let expected = d.connections.0.as_ref().unwrap().reminders.clone();
    let p = profile_from_draft(&d, &policy(), &time()).unwrap();
    assert_eq!(p.connections.reminders, expected);
}
#[test]
fn r05_referenced_removal_is_refused_and_historical_defaults_retained() {
    let p = profile();
    let mut q = p.clone();
    q.diners.0.remove(0);
    error(update(&p, &q, &[]), ErrorCode::ForeignReference);
    for c in &mut q.usual_week.cells.0 {
        c.attendee_ids.0.retain(|d| *d != id(1));
    }
    let w = week();
    let before = w.clone();
    error(
        update(&p, &q, std::slice::from_ref(&w)),
        ErrorCode::ForeignReference,
    );
    assert_eq!(w, before);
    let mut cleared = w.clone();
    for c in &mut cleared.cells.0 {
        c.attendee_ids.0.retain(|d| *d != id(1));
    }
    // Old immutable defaults still name dn1; only current references block removal.
    let change = update(&p, &q, std::slice::from_ref(&cleared)).unwrap();
    assert_eq!(change.candidate, q);
    assert_eq!(cleared.defaults_snapshot, w.defaults_snapshot);
    for kind in 0..3 {
        let mut referenced = cleared.clone();
        referenced.day_enabled.0[1] = false;
        let mut s = selection();
        if kind > 0 {
            s.servings.0[0].person = PersonRef::Diner { diner_id: id(2) };
        }
        if kind == 1 {
            let mut recipient = s.servings[0].clone();
            recipient.person = PersonRef::Diner { diner_id: id(1) };
            s.extras.0.push(ExtraSelection {
                extra_id: ExtraId(format!("ex_{:032x}", 1)),
                recipe: s.recipe.clone(),
                recipients: BoundedVec(vec![recipient]),
            });
        }
        if kind == 2 {
            s.meal_overrides.portions.0.push(PersonPortionOverride {
                person: PersonRef::Diner { diner_id: id(1) },
                preset: PortionPresetName::Regular,
                adjustment: PositiveRational::new(1, 1).unwrap(),
            });
        }
        referenced.plan.selections.0.push(s);
        let saved = referenced.clone();
        error(
            update(&p, &q, std::slice::from_ref(&referenced)),
            ErrorCode::ForeignReference,
        );
        assert_eq!(referenced, saved);
    }
    let mut unknown = cleared.clone();
    unknown.cells.0[0].attendee_ids.0.push(id(3));
    error(update(&p, &p, &[unknown]), ErrorCode::ForeignReference);
    let mut foreign_guest = cleared.clone();
    let mut s = selection();
    s.servings.0[0].person = PersonRef::Guest {
        occasion_id: OccasionId(format!("oc_{:032x}", 1)),
        guest_id: GuestId(format!("gu_{:032x}", 1)),
    };
    foreign_guest.plan.selections.0.push(s);
    error(
        update(&p, &p, &[foreign_guest]),
        ErrorCode::ForeignReference,
    );
    let mut scoped = cleared.clone();
    let mut s = selection();
    let occasion_id = OccasionId(format!("oc_{:032x}", 1));
    let guest_id = GuestId(format!("gu_{:032x}", 1));
    let occasion = Occasion {
        occasion_id: occasion_id.clone(),
        host_diner_ids: BoundedVec(vec![id(2)]),
        guests: BoundedVec(vec![Guest {
            guest_id: guest_id.clone(),
            age_band: AgeBand::Adult,
            restrictions: none(),
            portion: p.diners[0].portions.clone(),
        }]),
    };
    scoped.cells.0[4].occasion = Nullable(Some(occasion.clone()));
    s.servings.0[0].person = PersonRef::Guest {
        occasion_id,
        guest_id,
    };
    scoped.plan.selections.0.push(s);
    scoped.day_enabled.0[1] = false;
    update(&p, &q, std::slice::from_ref(&scoped)).unwrap();
    scoped.cells.0[4].occasion = Nullable(None);
    scoped.cells.0[14].occasion = Nullable(Some(occasion.clone()));
    error(
        update(&p, &q, std::slice::from_ref(&scoped)),
        ErrorCode::ForeignReference,
    );
    scoped.cells.0[4].occasion = Nullable(Some(occasion));
    error(update(&p, &q, &[scoped]), ErrorCode::IdentityMismatch);
    assert_eq!(p, profile());
}
