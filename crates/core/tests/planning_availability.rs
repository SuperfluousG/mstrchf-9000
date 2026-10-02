use mstrchf_contracts::*;
use mstrchf_core::availability::*;

fn profile() -> HouseholdProfile {
    parse_document(include_bytes!(
        "../../../tests/fixtures/households/p03ab/a1a2-profile.json"
    ))
    .unwrap()
}
fn week() -> Week {
    parse_document(include_bytes!(
        "../../../tests/fixtures/households/p03ab/a1a2-week.json"
    ))
    .unwrap()
}
fn at() -> Instant {
    Instant("2026-12-21T12:00:00Z".into())
}
fn edit(w: &Week) -> WeekEdit {
    WeekEdit {
        week_id: w.range.week_id.clone(),
        defaults_snapshot: w.defaults_snapshot.clone(),
        day_enabled: w.day_enabled.clone(),
        meal_type_enabled: w.meal_type_enabled.clone(),
        cells: w.cells.clone(),
    }
}
fn apply(w: &Week, e: &WeekEdit, p: &HouseholdProfile) -> Result<WeekEditEffect, DomainError> {
    let original = (w.clone(), e.clone(), p.clone());
    let result = prepare_week_edit(w, e, w.revision, p, &at());
    assert_eq!((w, e, p), (&original.0, &original.1, &original.2));
    result
}
fn project(
    p: &HouseholdProfile,
    w: &Week,
    r: &[SlotResolution],
) -> Result<[MealRequirements; 21], DomainError> {
    let original = (p.clone(), w.clone());
    let result = meal_requirements(p, w, r);
    assert_eq!((p, w), (&original.0, &original.1));
    if let Ok(reqs) = &result {
        assert_eq!(reqs.len(), 21);
        for (i, req) in reqs.iter().enumerate() {
            assert_eq!(
                req.slot.meal_type,
                [MealType::Breakfast, MealType::Lunch, MealType::Dinner][i % 3]
            );
            req.validate_shape().unwrap();
        }
        assert!(reqs.windows(2).all(|r| r[0].slot != r[1].slot));
    }
    result
}
fn error<T: std::fmt::Debug>(r: Result<T, DomainError>, code: ErrorCode) -> DomainError {
    let e = r.unwrap_err();
    assert_eq!(e.code, code, "{e:?}");
    assert!(!e.issues.is_empty());
    assert!(e.issues.len() <= 100);
    e
}
fn selection(w: &Week, index: usize) -> MealSelection {
    let c: Week = parse_document(include_bytes!(
        "../../../tests/fixtures/contracts/ruling004-current--week-disabled-retained.json"
    ))
    .unwrap();
    let mut s = c.plan.selections[0].clone();
    s.slot = w.cells[index].slot.clone();
    s.origin = SelectionOrigin::Fixed;
    s.servings.0[0].person = PersonRef::Diner {
        diner_id: profile().diners[0].diner_id.clone(),
    };
    s.servings.0[0].preset_ref.meal_type = s.slot.meal_type;
    let mut second = s.servings[0].clone();
    second.person = PersonRef::Diner {
        diner_id: profile().diners[1].diner_id.clone(),
    };
    s.servings = BoundedVec(vec![s.servings[0].clone(), second]);
    if let BatchUse::NewBatch {
        production_servings,
        ..
    } = &mut s.batch_use
    {
        *production_servings = PositiveRational::new(2, 1).unwrap();
    } else {
        panic!("the selected synthetic fixture must commit a new batch");
    }
    s.extras = BoundedVec(vec![]);
    s.meal_overrides = MealOverrides {
        portions: BoundedVec(vec![]),
        eat_by: Nullable(None),
    };
    s
}
fn selected() -> Week {
    parse_document(include_bytes!(
        "../../../tests/fixtures/households/p03a3/ruling004-current--selected-week.json"
    ))
    .unwrap()
}
fn guest(w: &mut Week, index: usize, declaration: RestrictionDeclaration) {
    w.cells.0[index].occasion = Nullable(Some(Occasion {
        occasion_id: OccasionId(format!("oc_{:032x}", index + 1)),
        host_diner_ids: w.cells[index].attendee_ids.clone(),
        guests: BoundedVec(vec![Guest {
            guest_id: GuestId(format!("gu_{:032x}", 1)),
            age_band: AgeBand::Adult,
            restrictions: declaration,
            portion: profile().diners[0].portions.clone(),
        }]),
    }));
}
fn declaration(confirmed: bool, instant: &str) -> RestrictionDeclaration {
    if confirmed {
        RestrictionDeclaration::Confirmed {
            confirmed_at: Instant(instant.into()),
            requirements: BoundedVec(vec![HardRequirement::AllergenExclusion {
                code: AllergenCode::Sesame,
            }]),
        }
    } else {
        RestrictionDeclaration::ConfirmedNone {
            confirmed_at: Instant(instant.into()),
        }
    }
}
fn exclude(w: &mut Week, index: usize, mode: usize) {
    match mode {
        0 => w.day_enabled.0[index / 3] = false,
        1 => match w.cells[index].slot.meal_type {
            MealType::Breakfast => w.meal_type_enabled.breakfast = false,
            MealType::Lunch => w.meal_type_enabled.lunch = false,
            MealType::Dinner => w.meal_type_enabled.dinner = false,
        },
        _ => w.cells.0[index].choice = MealLocation::Skip,
    }
}
fn trace(day_first: bool, skip: bool) {
    let p = profile();
    let mut w = selected();
    if skip {
        w.cells.0[4].choice = MealLocation::Skip;
    }
    let original = w.clone();
    for step in 0..4 {
        let mut e = edit(&w);
        let day = (step % 2 == 0) == day_first;
        if day {
            e.day_enabled.0[1] = step >= 2;
        } else {
            e.meal_type_enabled.lunch = step >= 2;
        }
        let effect = apply(&w, &e, &p).unwrap();
        w = effect.candidate;
        assert_eq!(
            eligible_slots(&w).unwrap().contains(&w.cells[4].slot),
            step == 3 && !skip
        );
        assert_eq!(w.plan, original.plan);
        assert_eq!(w.cells, original.cells);
        assert_eq!(
            (
                &w.approved_plan,
                &w.confirmed_order,
                &w.publication,
                w.revision
            ),
            (
                &original.approved_plan,
                &original.confirmed_order,
                &original.publication,
                original.revision
            )
        );
        if step == 2 {
            assert_eq!(
                eligible_slots(&w).unwrap().contains(&w.cells[3].slot),
                day_first
            );
            assert_eq!(
                eligible_slots(&w).unwrap().contains(&w.cells[1].slot),
                !day_first
            );
        }
    }
    assert_eq!(w, original);
}
#[test]
fn a01_row_then_day_retains_selection() {
    trace(false, false);
}
#[test]
fn a02_day_then_row_retains_selection() {
    trace(true, false);
}
#[test]
fn a03_skip_survives_both_overlay_orders() {
    trace(false, true);
    trace(true, true);
}
#[test]
fn a04_hidden_location_edit_and_truth_table() {
    for choice in [
        MealLocation::AtHome,
        MealLocation::OnTheMove,
        MealLocation::Skip,
    ] {
        for day in [false, true] {
            for row in [false, true] {
                let w = selected();
                let mut e = edit(&w);
                e.day_enabled.0[1] = day;
                e.meal_type_enabled.lunch = row;
                e.cells.0[4].choice = choice;
                let effect = apply(&w, &e, &profile()).unwrap();
                assert_eq!(
                    eligible_slots(&effect.candidate)
                        .unwrap()
                        .contains(&w.cells[4].slot),
                    day && row && choice != MealLocation::Skip
                );
                assert_eq!(effect.candidate.plan, w.plan);
                assert_eq!(effect.candidate.cells[3], w.cells[3]);
            }
        }
    }
}
#[test]
fn a05_empty_slots_count() {
    let w = week();
    assert_eq!(eligible_slots(&w).unwrap().len(), 21);
    let mut e = edit(&w);
    e.meal_type_enabled = MealTypeEnabled {
        breakfast: false,
        lunch: false,
        dinner: false,
    };
    let c = apply(&w, &e, &profile()).unwrap().candidate;
    assert!(eligible_slots(&c).unwrap().is_empty());
    assert!(active_selections(&c).is_empty());
    assert_eq!(c.cells, w.cells);
}
#[test]
fn a06_snapshot_is_immutable() {
    let w = week();
    let mut p = profile();
    p.usual_week
        .enabled_meal_types
        .0
        .retain(|t| *t != MealType::Lunch);
    let r = WeekRange {
        week_id: WeekId(format!("wk_{:032x}", 2)),
        ..w.range.clone()
    };
    let new = mstrchf_core::weeks::new_week(&p, &p.connections.privacy, &at(), &r).unwrap();
    assert!(!new.meal_type_enabled.lunch);
    assert!(w.meal_type_enabled.lunch);
    let mut e = edit(&w);
    e.defaults_snapshot = p.usual_week.clone();
    error(apply(&w, &e, &p), ErrorCode::IdentityMismatch);
}
#[test]
fn a07_invalid_edit_and_stale_revision() {
    let w = week();
    let p = profile();
    for branch in 0..5 {
        let mut e = edit(&w);
        match branch {
            0 => {
                e.week_id = WeekId(format!("wk_{:032x}", 2));
                for c in &mut e.cells.0 {
                    c.slot.week_id = e.week_id.clone();
                }
            }
            1 => e.cells.0[0].slot.week_id = WeekId(format!("wk_{:032x}", 2)),
            2 => e.cells.0[0].slot.date = LocalDate("2027-01-04".into()),
            3 => e.cells.0[0] = e.cells[1].clone(),
            _ => {
                e.cells.0.pop();
            }
        }
        assert!(apply(&w, &e, &p).is_err(), "branch {branch}");
    }
    error(
        prepare_week_edit(&w, &edit(&w), Revision(4), &p, &at()),
        ErrorCode::ContextStale,
    );
}
#[test]
fn a08_scoped_guests_and_canonical_union() {
    let p = profile();
    let mut w = week();
    guest(&mut w, 14, declaration(true, &at().0));
    let r = project(&p, &w, &[]).unwrap();
    assert_eq!(r[14].attendees.len(), 3);
    assert_eq!(r[14].shared_hard_requirements.len(), 2);
    assert_eq!(r[11].attendees.len(), 2);
    assert_eq!(r[11].shared_hard_requirements.len(), 1);
    assert_eq!(p.diners.len(), 2);
    assert_eq!(r[14].equipment, p.preferences.equipment);
    assert_eq!(r[14].mobility, p.preferences.mobility);
}
#[test]
fn a09_bad_occasion_scope_and_bounds() {
    for branch in 0..4 {
        let w = week();
        let mut e = edit(&w);
        let mut x = w.clone();
        guest(&mut x, 14, declaration(false, &at().0));
        match branch {
            0 => x.cells.0[14]
                .occasion
                .0
                .as_mut()
                .unwrap()
                .host_diner_ids
                .0
                .clear(),
            1 => {
                let g = x.cells[14].occasion.0.as_ref().unwrap().guests[0].clone();
                x.cells.0[14].occasion.0.as_mut().unwrap().guests.0 = (1..=63)
                    .map(|i| Guest {
                        guest_id: GuestId(format!("gu_{i:032x}")),
                        ..g.clone()
                    })
                    .collect();
            }
            2 => {
                let g = x.cells[14].occasion.0.as_ref().unwrap().guests[0].clone();
                x.cells.0[14].occasion.0.as_mut().unwrap().guests.0.push(g);
            }
            _ => x.cells.0[11].occasion = x.cells[14].occasion.clone(),
        };
        e.cells = x.cells;
        assert!(apply(&w, &e, &profile()).is_err(), "branch {branch}");
    }
}
#[test]
fn a10_unconfirmed_drafts_refuse_projection() {
    unconfirmed_modes(&[0, 1, 2, 3]);
}
fn unconfirmed_modes(modes: &[usize]) {
    for mode in modes {
        let w = week();
        let mut x = w.clone();
        guest(&mut x, 14, RestrictionDeclaration::Unconfirmed {});
        if *mode < 3 {
            exclude(&mut x, 14, *mode);
        }
        let c = apply(&w, &edit(&x), &profile()).unwrap().candidate;
        assert_eq!(c.cells, x.cells);
        error(project(&profile(), &c, &[]), ErrorCode::DomainConflict);
    }
}
#[test]
fn a11_foreign_guest_in_each_retained_reference_and_removal() {
    for branch in 0..3 {
        let mut w = selected();
        guest(&mut w, 14, declaration(false, &at().0));
        w.day_enabled.0[1] = false;
        let o = w.cells[14].occasion.0.as_ref().unwrap();
        let person = PersonRef::Guest {
            occasion_id: o.occasion_id.clone(),
            guest_id: o.guests[0].guest_id.clone(),
        };
        let s = &mut w.plan.selections.0[0];
        match branch {
            0 => s.servings.0[0].person = person,
            1 => {
                let mut recipient = s.servings[0].clone();
                recipient.person = person;
                s.extras.0.push(ExtraSelection {
                    extra_id: ExtraId(format!("ex_{:032x}", 1)),
                    recipe: s.recipe.clone(),
                    recipients: BoundedVec(vec![recipient]),
                });
            }
            _ => s.meal_overrides.portions.0.push(PersonPortionOverride {
                person,
                preset: PortionPresetName::Regular,
                adjustment: profile().diners[0].portions.adjustment,
            }),
        };
        error(
            apply(&w, &edit(&w), &profile()),
            ErrorCode::ForeignReference,
        );
        error(project(&profile(), &w, &[]), ErrorCode::ForeignReference);
    }
    let mut w = selected();
    guest(&mut w, 4, declaration(false, &at().0));
    let o = w.cells[4].occasion.0.as_ref().unwrap();
    w.plan.selections.0[0].servings.0[0].person = PersonRef::Guest {
        occasion_id: o.occasion_id.clone(),
        guest_id: o.guests[0].guest_id.clone(),
    };
    let mut e = edit(&w);
    e.cells.0[4].occasion = Nullable(None);
    error(apply(&w, &e, &profile()), ErrorCode::ForeignReference);
}
fn changed_profile() -> HouseholdProfile {
    let mut p = profile();
    p.diners.0[0].restrictions = RestrictionDeclaration::Confirmed {
        confirmed_at: at(),
        requirements: BoundedVec(vec![HardRequirement::Diet {
            code: DietCode::GlutenFree,
        }]),
    };
    p
}
#[test]
fn a12_full_union_excludes_dislikes() {
    let mut p = changed_profile();
    p.diners.0[0]
        .dislikes
        .0
        .push(IngredientId(format!("ig_{:032x}", 42)));
    let r = project(&p, &week(), &[]).unwrap();
    for m in r {
        assert_eq!(m.attendees.len(), 2);
        assert_eq!(m.shared_hard_requirements.len(), 2);
        assert!(
            m.shared_hard_requirements
                .iter()
                .all(|x| matches!(x, HardRequirement::Diet { .. }))
        );
    }
}
#[test]
fn a13_inactive_null_retains_location() {
    for mode in 0..3 {
        let mut w = selected();
        exclude(&mut w, 4, mode);
        let r = project(&profile(), &w, &[]).unwrap();
        assert!(!r[4].enabled);
        assert!(r[4].eat_by.0.is_none());
        assert!(r[4].attendees.is_empty());
        assert!(r[4].shared_hard_requirements.is_empty());
        assert_eq!(r[4].location, w.cells[4].choice);
        assert!(active_selections(&w).is_empty());
    }
}
#[test]
fn a14_portion_preferences_and_explicit_override_retained() {
    let mut p = profile();
    p.diners.0[0].portions.preset = PortionPresetName::Larger;
    p.diners.0[0]
        .portions
        .meal_overrides
        .0
        .push(MealPortionOverride {
            meal_type: MealType::Lunch,
            preset: PortionPresetName::Smaller,
            adjustment: PositiveRational::new(1, 2).unwrap(),
        });
    let mut w = selected();
    w.plan.selections.0[0]
        .meal_overrides
        .portions
        .0
        .push(PersonPortionOverride {
            person: PersonRef::Diner {
                diner_id: p.diners[0].diner_id.clone(),
            },
            preset: PortionPresetName::Regular,
            adjustment: PositiveRational::new(3, 4).unwrap(),
        });
    let r = project(&p, &w, &[]).unwrap();
    assert_eq!(r[4].attendees[0].portion, p.diners[0].portions);
    assert_eq!(apply(&w, &edit(&w), &p).unwrap().candidate.plan, w.plan);
}
fn moment(
    date: &str,
    time: &str,
    zone: &str,
    resolution: Resolution,
    instant: &str,
) -> ZonedMoment {
    ZonedMoment::new(
        LocalDate(date.into()),
        LocalTime(time.into()),
        ZoneId(zone.into()),
        resolution,
        Instant(instant.into()),
    )
    .unwrap()
}
#[test]
fn a15_explicit_time_binding() {
    let mut w = week();
    w.plan.selections.0.push(selection(&w, 14));
    let m = moment(
        "2027-01-01",
        "19:15:00",
        "Europe/London",
        Resolution::Unambiguous,
        "2027-01-01T19:15:00Z",
    );
    w.plan.selections.0[0].meal_overrides.eat_by = Nullable(Some(m.clone()));
    let r = project(&profile(), &w, &[]).unwrap();
    assert_eq!(r[14].eat_by.0, Some(m));
    assert_eq!(
        r[11].eat_by.0.as_ref().unwrap().instant.0,
        "2026-12-31T18:30:00Z"
    );
    for m in [
        moment(
            "2027-01-02",
            "19:15:00",
            "Europe/London",
            Resolution::Unambiguous,
            "2027-01-02T19:15:00Z",
        ),
        moment(
            "2027-01-01",
            "19:15:00",
            "Asia/Tokyo",
            Resolution::Unambiguous,
            "2027-01-01T10:15:00Z",
        ),
    ] {
        w.plan.selections.0[0].meal_overrides.eat_by = Nullable(Some(m));
        error(project(&profile(), &w, &[]), ErrorCode::IdentityMismatch);
    }
}
#[test]
fn a16_explicit_fold_and_gap() {
    for r in [
        None,
        Some(Resolution::Unambiguous),
        Some(Resolution::Earlier),
        Some(Resolution::Later),
    ] {
        error(
            resolve_meal_time(
                &LocalDate("2026-03-29".into()),
                &LocalTime("01:30:00".into()),
                &ZoneId("Europe/London".into()),
                r,
            ),
            ErrorCode::DomainConflict,
        );
    }
    let d = LocalDate("2026-10-25".into());
    let t = LocalTime("01:30:00".into());
    let z = ZoneId("Europe/London".into());
    error(
        resolve_meal_time(&d, &t, &z, None),
        ErrorCode::DomainConflict,
    );
    for (r, utc) in [
        (Resolution::Earlier, "2026-10-25T00:30:00Z"),
        (Resolution::Later, "2026-10-25T01:30:00Z"),
    ] {
        assert_eq!(
            resolve_meal_time(&d, &t, &z, Some(r)).unwrap().instant.0,
            utc
        );
    }
}
#[test]
fn a17_unique_and_malformed_direct_values() {
    let d = LocalDate("2026-03-29".into());
    let t = LocalTime("02:30:00".into());
    let z = ZoneId("Europe/London".into());
    assert_eq!(
        resolve_meal_time(&d, &t, &z, None).unwrap().instant.0,
        "2026-03-29T01:30:00Z"
    );
    error(
        resolve_meal_time(&d, &t, &z, Some(Resolution::Earlier)),
        ErrorCode::InvalidValue,
    );
    for (d, t, z) in [
        (LocalDate("2026-02-30".into()), t.clone(), z.clone()),
        (d.clone(), LocalTime("25:00:00".into()), z.clone()),
        (d, t, ZoneId("Bad/Zone".into())),
    ] {
        error(resolve_meal_time(&d, &t, &z, None), ErrorCode::InvalidValue);
    }
}
#[test]
fn a18_union_limit_and_hidden_attendance_mismatch() {
    let mut p = profile();
    for (i, d) in p.diners.0.iter_mut().enumerate() {
        d.restrictions = RestrictionDeclaration::Confirmed {
            confirmed_at: at(),
            requirements: BoundedVec(
                (1..=33)
                    .map(|n| HardRequirement::IngredientExclusion {
                        ingredient_id: IngredientId(format!("ig_{:032x}", n + i * 33)),
                    })
                    .collect(),
            ),
        };
    }
    error(project(&p, &week(), &[]), ErrorCode::LimitExceeded);
    let mut w = selected();
    w.day_enabled.0[1] = false;
    w.cells.0[4].attendee_ids.0.remove(0);
    let mut e = edit(&w);
    e.day_enabled.0[1] = true;
    let effect = apply(&w, &e, &profile()).unwrap();
    assert!(effect.recheck_slots.contains(&w.cells[4].slot));
    assert_eq!(effect.candidate.plan, w.plan);
}

fn admission_matrix(offset: &str, modes: &[usize], want: Option<ErrorCode>) {
    for confirmed in [false, true] {
        for mode in modes {
            let w = week();
            let mut x = w.clone();
            guest(&mut x, 14, declaration(confirmed, offset));
            if *mode < 3 {
                exclude(&mut x, 14, *mode);
            }
            let e = edit(&x);
            let r = apply(&w, &e, &profile());
            if let Some(code) = want {
                let err = error(r, code);
                assert!(
                    err.issues
                        .iter()
                        .any(|i| i.path == "/cells/14/occasion/guests/0/restrictions/confirmed_at")
                );
            } else {
                assert_eq!(r.unwrap().candidate.cells, e.cells);
            }
        }
    }
}
#[test]
fn ag01_equality_both_declarations() {
    admission_matrix(&at().0, &[3], None);
}
#[test]
fn ag02_future_both_declarations() {
    admission_matrix("2026-12-21T12:00:01Z", &[3], Some(ErrorCode::InvalidValue));
}
#[test]
fn ag03_inactive_equality_and_future_each_mode() {
    admission_matrix(&at().0, &[0, 1, 2], None);
    admission_matrix(
        "2026-12-21T12:00:01Z",
        &[0, 1, 2],
        Some(ErrorCode::InvalidValue),
    );
}
#[test]
fn ag04_malformed_evaluation_and_guest() {
    for v in ["not-an-instant", "2026-02-30T12:00:00Z"] {
        let w = week();
        error(
            prepare_week_edit(&w, &edit(&w), w.revision, &profile(), &Instant(v.into())),
            ErrorCode::InvalidValue,
        );
        for confirmed in [false, true] {
            let mut e = w.clone();
            guest(&mut e, 14, declaration(confirmed, v));
            error(apply(&w, &edit(&e), &profile()), ErrorCode::InvalidValue);
        }
    }
}
#[test]
fn ag05_previous_second_both_variants() {
    admission_matrix("2026-12-21T11:59:59Z", &[3], None);
}
#[test]
fn ag06_unconfirmed_active() {
    unconfirmed_modes(&[3]);
}
#[test]
fn ag07_unconfirmed_all_exclusions() {
    unconfirmed_modes(&[0, 1, 2]);
}
#[test]
fn ag08_later_guest_never_short_circuits() {
    for other_cell in [false, true] {
        let w = week();
        let mut e = w.clone();
        guest(&mut e, 14, declaration(false, &at().0));
        if other_cell {
            guest(&mut e, 17, declaration(false, "2026-12-21T12:00:01Z"));
            exclude(&mut e, 17, 0);
        } else {
            let mut g = e.cells[14].occasion.0.as_ref().unwrap().guests[0].clone();
            g.guest_id = GuestId(format!("gu_{:032x}", 2));
            g.restrictions = declaration(false, "2026-12-21T12:00:01Z");
            e.cells.0[14].occasion.0.as_mut().unwrap().guests.0.push(g);
        }
        error(apply(&w, &edit(&e), &profile()), ErrorCode::InvalidValue);
    }
}
#[test]
fn ag09_no_guests_invalid_clock() {
    let w = week();
    let e = error(
        prepare_week_edit(
            &w,
            &edit(&w),
            w.revision,
            &profile(),
            &Instant("bad".into()),
        ),
        ErrorCode::InvalidValue,
    );
    assert_eq!(e.issues[0].path, "/evaluated_at");
}

fn time_week(monday: &str, zone: &str, time: &str) -> (HouseholdProfile, Week) {
    let mut p = profile();
    let mut w = week();
    w.range.monday = LocalDate(monday.into());
    w.range.zone = ZoneId(zone.into());
    p.usual_week.zone = w.range.zone.clone();
    w.defaults_snapshot.zone = w.range.zone.clone();
    let start = chrono::NaiveDate::parse_from_str(monday, "%Y-%m-%d").unwrap();
    for (i, c) in w.cells.0.iter_mut().enumerate() {
        c.slot.date = LocalDate((start + chrono::Days::new((i / 3) as u64)).to_string());
    }
    p.preferences
        .meal_times
        .0
        .iter_mut()
        .find(|m| m.meal_type == MealType::Breakfast)
        .unwrap()
        .time = LocalTime(time.into());
    (p, w)
}
fn resolution(w: &Week, i: usize, r: Resolution) -> SlotResolution {
    SlotResolution {
        slot: w.cells[i].slot.clone(),
        resolution: r,
    }
}
#[test]
fn at01_ordinary_inactive_all_modes() {
    a13_inactive_null_retains_location();
}
#[test]
fn at02_inactive_london_gap_each_exclusion() {
    for mode in 0..3 {
        let (p, mut w) = time_week("2026-03-23", "Europe/London", "01:30:00");
        exclude(&mut w, 18, mode);
        assert!(project(&p, &w, &[]).unwrap()[18].eat_by.0.is_none());
    }
}
#[test]
fn at03_active_gap_every_resolution() {
    let (p, w) = time_week("2026-03-23", "Europe/London", "01:30:00");
    error(project(&p, &w, &[]), ErrorCode::DomainConflict);
    for r in [
        Resolution::Unambiguous,
        Resolution::Earlier,
        Resolution::Later,
    ] {
        error(
            project(&p, &w, &[resolution(&w, 18, r)]),
            ErrorCode::DomainConflict,
        );
    }
}
#[test]
fn at04_inactive_fold_rejects_every_unused_resolution() {
    let (p, mut w) = time_week("2026-10-19", "Europe/London", "01:30:00");
    exclude(&mut w, 18, 0);
    assert!(project(&p, &w, &[]).unwrap()[18].eat_by.0.is_none());
    for r in [
        Resolution::Earlier,
        Resolution::Later,
        Resolution::Unambiguous,
    ] {
        error(
            project(&p, &w, &[resolution(&w, 18, r)]),
            ErrorCode::InvalidValue,
        );
    }
}
#[test]
fn at05_active_fold_requires_exact_choice() {
    let (p, w) = time_week("2026-10-19", "Europe/London", "01:30:00");
    error(project(&p, &w, &[]), ErrorCode::DomainConflict);
    error(
        project(&p, &w, &[resolution(&w, 18, Resolution::Unambiguous)]),
        ErrorCode::InvalidValue,
    );
    for (r, s) in [
        (Resolution::Earlier, "2026-10-25T00:30:00Z"),
        (Resolution::Later, "2026-10-25T01:30:00Z"),
    ] {
        assert_eq!(
            project(&p, &w, &[resolution(&w, 18, r)]).unwrap()[18]
                .eat_by
                .0
                .as_ref()
                .unwrap()
                .instant
                .0,
            s
        );
    }
}
#[test]
fn at06_retained_later_override_survives_reactivation() {
    let (p, mut w) = time_week("2026-10-19", "Europe/London", "01:30:00");
    let mut s = selection(&w, 18);
    let m = moment(
        "2026-10-25",
        "01:30:00",
        "Europe/London",
        Resolution::Later,
        "2026-10-25T01:30:00Z",
    );
    s.meal_overrides.eat_by = Nullable(Some(m.clone()));
    w.plan.selections.0.push(s);
    let original = w.clone();
    let mut e = edit(&w);
    e.day_enabled.0[6] = false;
    let c = apply(&w, &e, &p).unwrap().candidate;
    assert!(project(&p, &c, &[]).unwrap()[18].eat_by.0.is_none());
    assert_eq!(c.plan, w.plan);
    w = apply(&c, &edit(&original), &p).unwrap().candidate;
    assert_eq!(project(&p, &w, &[]).unwrap()[18].eat_by.0, Some(m));
    error(
        project(&p, &w, &[resolution(&w, 18, Resolution::Earlier)]),
        ErrorCode::InvalidValue,
    );
}
fn apia() -> (HouseholdProfile, Week) {
    let (mut p, mut w) = time_week("2011-12-26", "Pacific/Apia", "00:00:00");
    for m in &mut p.preferences.meal_times.0 {
        m.time = LocalTime(
            match m.meal_type {
                MealType::Breakfast => "00:00:00",
                MealType::Lunch => "12:00:00",
                MealType::Dinner => "23:59:00",
            }
            .into(),
        );
    }
    w.day_enabled.0[4] = false;
    (p, w)
}
#[test]
fn at07_apia_retains_missing_civil_day() {
    let (p, w) = apia();
    let r = project(&p, &w, &[]).unwrap();
    for req in &r[12..15] {
        assert_eq!(req.slot.date.0, "2011-12-30");
        assert!(!req.enabled);
        assert!(req.eat_by.0.is_none());
    }
    assert_eq!(r[20].slot.date.0, "2012-01-01");
}
#[test]
fn at08_apia_each_meal_every_resolution_refuses() {
    for i in 12..15 {
        let (p, mut w) = apia();
        w.day_enabled.0[4] = true;
        for j in 12..15 {
            if j != i {
                w.cells.0[j].choice = MealLocation::Skip;
            }
        }
        error(project(&p, &w, &[]), ErrorCode::DomainConflict);
        for r in [
            Resolution::Unambiguous,
            Resolution::Earlier,
            Resolution::Later,
        ] {
            error(
                project(&p, &w, &[resolution(&w, i, r)]),
                ErrorCode::DomainConflict,
            );
        }
    }
}
#[test]
fn at09_unused_duplicate_foreign_and_overlong_resolutions() {
    let (p, w) = apia();
    error(
        project(&p, &w, &[resolution(&w, 12, Resolution::Unambiguous)]),
        ErrorCode::InvalidValue,
    );
    let r = resolution(&w, 0, Resolution::Unambiguous);
    error(
        project(&p, &w, &[r.clone(), r.clone()]),
        ErrorCode::InvalidValue,
    );
    let mut foreign = r.clone();
    foreign.slot.week_id = WeekId(format!("wk_{:032x}", 2));
    error(project(&p, &w, &[foreign]), ErrorCode::InvalidValue);
    error(project(&p, &w, &vec![r; 22]), ErrorCode::LimitExceeded);
}
#[test]
fn at10_inactive_overrides_never_launder_invalid_state() {
    for branch in 0..4 {
        let mut w = selected();
        exclude(&mut w, 4, 0);
        let mut m = moment(
            "2026-12-29",
            "18:00:00",
            "Europe/London",
            Resolution::Unambiguous,
            "2026-12-29T18:00:00Z",
        );
        match branch {
            0 => m.instant = Instant("bad".into()),
            1 => {
                m = moment(
                    "2026-12-30",
                    "18:00:00",
                    "Europe/London",
                    Resolution::Unambiguous,
                    "2026-12-30T18:00:00Z",
                )
            }
            2 => {
                m = moment(
                    "2026-12-29",
                    "18:00:00",
                    "Asia/Tokyo",
                    Resolution::Unambiguous,
                    "2026-12-29T09:00:00Z",
                )
            }
            _ => m.instant = Instant("2026-12-29T19:00:00Z".into()),
        };
        w.plan.selections.0[0].meal_overrides.eat_by = Nullable(Some(m));
        error(
            project(&profile(), &w, &[]),
            if branch == 1 || branch == 2 {
                ErrorCode::IdentityMismatch
            } else {
                ErrorCode::InvalidValue
            },
        );
        assert!(apply(&w, &edit(&w), &profile()).is_err());
    }
    let (p, mut w) = apia();
    let mut s = selection(&w, 12);
    s.meal_overrides.eat_by = Nullable(Some(ZonedMoment {
        local_date: LocalDate("2011-12-30".into()),
        local_time: LocalTime("12:00:00".into()),
        zone: ZoneId("Pacific/Apia".into()),
        resolution: Resolution::Unambiguous,
        instant: Instant("2011-12-30T22:00:00Z".into()),
    }));
    w.plan.selections.0.push(s);
    error(project(&p, &w, &[]), ErrorCode::InvalidValue);
}
#[test]
fn at11_unique_implicit_explicit_and_invalid_fold_sides() {
    let d = LocalDate("2026-12-29".into());
    let t = LocalTime("18:00:00".into());
    let z = ZoneId("Europe/London".into());
    for r in [None, Some(Resolution::Unambiguous)] {
        assert_eq!(
            resolve_meal_time(&d, &t, &z, r).unwrap().instant.0,
            "2026-12-29T18:00:00Z"
        );
    }
    for r in [Resolution::Earlier, Resolution::Later] {
        error(
            resolve_meal_time(&d, &t, &z, Some(r)),
            ErrorCode::InvalidValue,
        );
    }
}
#[test]
fn at15_malformed_public_override_refuses_projection() {
    at10_inactive_overrides_never_launder_invalid_state();
}
#[test]
fn at16_all_inactive_twenty_one_nulls_and_guest_control() {
    let mut w = selected();
    w.day_enabled = BoundedVec(vec![false; 7]);
    let r = project(&profile(), &w, &[]).unwrap();
    assert!(r.iter().all(|r| !r.enabled
        && r.eat_by.0.is_none()
        && r.attendees.is_empty()
        && r.shared_hard_requirements.is_empty()));
    assert!(eligible_slots(&w).unwrap().is_empty());
    guest(&mut w, 14, RestrictionDeclaration::Unconfirmed {});
    error(project(&profile(), &w, &[]), ErrorCode::DomainConflict);
}
#[test]
fn at18_mixed_missing_duplicate_foreign_references() {
    for branch in 0..3 {
        let mut w = week();
        exclude(&mut w, 4, 0);
        match branch {
            0 => {
                w.cells.0.pop();
            }
            1 => w.cells.0[0] = w.cells[1].clone(),
            _ => w.cells.0[4].attendee_ids.0[0] = DinerId(format!("dn_{:032x}", 99)),
        };
        assert!(project(&profile(), &w, &[]).is_err());
    }
}
fn targets(
    before: &HouseholdProfile,
    after: &HouseholdProfile,
    w: &Week,
    date: &str,
) -> Vec<SlotId> {
    let original = (before.clone(), after.clone(), w.clone());
    let r = restriction_recheck_slots(before, after, w, &LocalDate(date.into())).unwrap();
    assert_eq!((before, after, w), (&original.0, &original.1, &original.2));
    r
}
#[test]
fn r01_restriction_change_targets_hidden_and_skip() {
    for mode in 0..4 {
        let mut w = selected();
        if mode < 3 {
            exclude(&mut w, 4, mode);
        }
        assert_eq!(
            targets(&profile(), &changed_profile(), &w, "2026-12-28"),
            vec![w.cells[4].slot.clone()]
        );
    }
}
fn friday() -> Week {
    let mut w = week();
    w.plan.selections.0.push(selection(&w, 14));
    w.cells.0[14].attendee_ids.0.remove(0);
    w.plan.selections.0[0].servings.0.remove(0);
    w
}
#[test]
fn r02_precise_negative_premise() {
    rr01_no_changed_diner_participation();
}
#[test]
fn r03_date_boundary() {
    let w = selected();
    assert!(targets(&profile(), &changed_profile(), &w, "2026-12-30").is_empty());
    assert_eq!(
        targets(&profile(), &changed_profile(), &w, "2026-12-29"),
        vec![w.cells[4].slot.clone()]
    );
}
#[test]
fn r04_semantic_changes_timestamp_noop_and_empty() {
    let w = selected();
    let mut p = profile();
    p.diners.0[0].restrictions = declaration(false, "2026-12-21T11:00:00Z");
    assert!(targets(&profile(), &p, &w, "2026-12-28").is_empty());
    assert_eq!(
        targets(&profile(), &changed_profile(), &w, "2026-12-28").len(),
        1
    );
    assert_eq!(
        targets(&changed_profile(), &profile(), &w, "2026-12-28").len(),
        1
    );
    assert!(targets(&profile(), &changed_profile(), &week(), "2026-12-28").is_empty());
}
#[test]
fn rr01_no_changed_diner_participation() {
    for inactive in [false, true] {
        for with_guest in [false, true] {
            let mut w = friday();
            if inactive {
                exclude(&mut w, 14, 0);
            }
            if with_guest {
                guest(&mut w, 14, declaration(false, &at().0));
                let o = w.cells[14].occasion.0.as_ref().unwrap();
                w.plan.selections.0[0].servings.0[0].person = PersonRef::Guest {
                    occasion_id: o.occasion_id.clone(),
                    guest_id: o.guests[0].guest_id.clone(),
                };
            }
            assert!(targets(&profile(), &changed_profile(), &w, "2026-12-28").is_empty());
        }
    }
}
#[test]
fn rr02_cell_only_target() {
    let mut w = friday();
    w.cells.0[14].attendee_ids = week().cells[14].attendee_ids.clone();
    assert_eq!(
        targets(&profile(), &changed_profile(), &w, "2026-12-28"),
        vec![w.cells[14].slot.clone()]
    );
}
#[test]
fn rr03_hidden_serving_only_target() {
    let mut w = friday();
    exclude(&mut w, 14, 0);
    w.plan.selections.0[0].servings.0[0].person = PersonRef::Diner {
        diner_id: profile().diners[0].diner_id.clone(),
    };
    assert_eq!(
        targets(&profile(), &changed_profile(), &w, "2026-12-28"),
        vec![w.cells[14].slot.clone()]
    );
}
#[test]
fn rr04_hidden_extra_only_target() {
    let mut w = friday();
    exclude(&mut w, 14, 0);
    let mut recipient = w.plan.selections[0].servings[0].clone();
    recipient.person = PersonRef::Diner {
        diner_id: profile().diners[0].diner_id.clone(),
    };
    let recipe = w.plan.selections[0].recipe.clone();
    w.plan.selections.0[0].extras.0.push(ExtraSelection {
        extra_id: ExtraId(format!("ex_{:032x}", 1)),
        recipe,
        recipients: BoundedVec(vec![recipient]),
    });
    assert_eq!(
        targets(&profile(), &changed_profile(), &w, "2026-12-28"),
        vec![w.cells[14].slot.clone()]
    );
}
#[test]
fn rr05_both_once_empty_plan_none() {
    let w = selected();
    assert_eq!(
        targets(&profile(), &changed_profile(), &w, "2026-12-28"),
        vec![w.cells[4].slot.clone()]
    );
    assert!(targets(&profile(), &changed_profile(), &week(), "2026-12-28").is_empty());
}
#[test]
fn malformed_all_public_entrypoints_and_recheck_identity() {
    let mut w = week();
    w.day_enabled.0.clear();
    assert!(eligible_slots(&w).is_err());
    assert!(apply(&w, &edit(&w), &profile()).is_err());
    assert!(project(&profile(), &w, &[]).is_err());
    assert!(
        restriction_recheck_slots(
            &profile(),
            &changed_profile(),
            &w,
            &LocalDate("2026-12-28".into())
        )
        .is_err()
    );
    let mut after = profile();
    after.household_id = HouseholdId(format!("hh_{:032x}", 2));
    error(
        restriction_recheck_slots(&profile(), &after, &week(), &LocalDate("2026-12-28".into())),
        ErrorCode::IdentityMismatch,
    );
    error(
        restriction_recheck_slots(
            &profile(),
            &profile(),
            &week(),
            &LocalDate("2026-02-30".into()),
        ),
        ErrorCode::InvalidValue,
    );
}

#[test]
fn literal_time_vectors() {
    let rows: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/households/p03a3/time-vectors.json"
    ))
    .unwrap();
    for row in rows.as_array().unwrap() {
        let r: Resolution = serde_json::from_value(row["resolution"].clone()).unwrap();
        let got = resolve_meal_time(
            &LocalDate(row["date"].as_str().unwrap().into()),
            &LocalTime(row["time"].as_str().unwrap().into()),
            &ZoneId(row["zone"].as_str().unwrap().into()),
            Some(r),
        )
        .unwrap();
        assert_eq!(got.instant.0, row["instant"].as_str().unwrap());
    }
}
#[test]
fn complete_context_refuses_unconfirmed_guests_all_modes() {
    let mut context: PlanningContext = parse_document(include_bytes!(
        "../../../tests/fixtures/contracts/ruling004-current--planning-context.json"
    ))
    .unwrap();
    context.household = profile();
    context.privacy = context.household.connections.privacy.clone();
    context.stamp.household_revision = context.household.revision;
    context.stamp.privacy_revision = context.privacy.revision;
    context.stamp.read_weeks = BoundedVec(vec![WeekRevision {
        week_id: week().range.week_id,
        revision: Revision(5),
    }]);
    context.stamp.write_weeks = context.stamp.read_weeks.clone();
    for mode in 0..4 {
        let mut w = week();
        if mode < 3 {
            exclude(&mut w, 14, mode);
        }
        context.requirements = BoundedVec(project(&profile(), &w, &[]).unwrap().to_vec());
        context.weeks = BoundedVec(vec![w.clone()]);
        validate_context(&context).unwrap();
        guest(&mut w, 14, RestrictionDeclaration::Unconfirmed {});
        context.weeks = BoundedVec(vec![w]);
        let e = validate_context(&context).unwrap_err();
        assert_eq!(e[0].code, ErrorCode::DomainConflict);
        assert!(e[0].path.contains("restrictions"));
    }
}
#[test]
fn retained_approval_order_links_and_overrides_are_byte_equal() {
    let mut w = selected();
    let c: PlanningContext = parse_document(include_bytes!(
        "../../../tests/fixtures/contracts/ruling004-current--planning-context.json"
    ))
    .unwrap();
    let a = ApprovedPlan {
        approval_id: ApprovalId(format!("au_{:032x}", 1)),
        week_id: w.range.week_id.clone(),
        plan_revision: w.revision,
        derived_digest: PayloadDigest("1".repeat(64)),
        context_stamp: c.stamp,
        approved_at: at(),
    };
    w.approved_plan = Nullable(Some(a.clone()));
    w.confirmed_order = Nullable(Some(ConfirmedOrder {
        order_id: OrderId(format!("or_{:032x}", 1)),
        confirmation: OrderConfirmation {
            week_id: w.range.week_id.clone(),
            approval_id: a.approval_id.clone(),
            delivery_start: moment(
                "2026-12-28",
                "10:00:00",
                "Europe/London",
                Resolution::Unambiguous,
                "2026-12-28T10:00:00Z",
            ),
            delivery_end: moment(
                "2026-12-28",
                "11:00:00",
                "Europe/London",
                Resolution::Unambiguous,
                "2026-12-28T11:00:00Z",
            ),
            confirmed_at: at(),
            source: OrderConfirmationSource::UserConfirmation {},
        },
        approved_plan: a,
        ordered_recipe_refs: BoundedVec(vec![w.plan.selections[0].recipe.clone()]),
    }));
    w.publication = PublicationState::Failed;
    // Synthetic shape-valid record: preservation only, never proof of approval admission.
    w.plan.selections.0[0].repeat_authorised = false;
    w.plan.selections.0[0].meal_overrides.eat_by = Nullable(Some(moment(
        "2026-12-29",
        "12:30:00",
        "Europe/London",
        Resolution::Unambiguous,
        "2026-12-29T12:30:00Z",
    )));
    let original = canonical_bytes(&w).unwrap();
    let mut e = edit(&w);
    e.day_enabled.0[1] = false;
    let mut candidate = apply(&w, &e, &profile()).unwrap().candidate;
    candidate.day_enabled = w.day_enabled.clone();
    assert_eq!(canonical_bytes(&candidate).unwrap(), original);
}
#[test]
fn edit_recheck_exact_sorted_targets_and_noop() {
    let w = week();
    assert!(
        apply(&w, &edit(&w), &profile())
            .unwrap()
            .recheck_slots
            .is_empty()
    );
    let mut e = edit(&w);
    e.day_enabled.0[1] = false;
    let r = apply(&w, &e, &profile()).unwrap();
    assert_eq!(
        r.recheck_slots,
        w.cells[3..6]
            .iter()
            .map(|c| c.slot.clone())
            .collect::<Vec<_>>()
    );
    let mut e = edit(&w);
    e.cells.0.reverse();
    assert!(apply(&w, &e, &profile()).unwrap().recheck_slots.is_empty());
    let r = project(
        &profile(),
        &apply(&w, &e, &profile()).unwrap().candidate,
        &[],
    )
    .unwrap();
    assert_eq!(r[0].slot.date.0, "2026-12-28");
    assert_eq!(r[20].slot.date.0, "2027-01-03");
}
#[test]
fn selected_unconfirmed_guest_refuses_even_inactive() {
    for mode in 0..4 {
        let w = selected();
        let mut e = w.clone();
        guest(&mut e, 4, RestrictionDeclaration::Unconfirmed {});
        if mode < 3 {
            exclude(&mut e, 4, mode);
        }
        error(apply(&w, &edit(&e), &profile()), ErrorCode::DomainConflict);
    }
}
#[test]
fn future_profile_confirmation_not_admitted() {
    for confirmed in [false, true] {
        let mut p = profile();
        p.diners.0[0].restrictions = declaration(confirmed, "2026-12-21T12:00:01Z");
        let w = week();
        error(apply(&w, &edit(&w), &p), ErrorCode::InvalidValue);
    }
}
#[test]
fn recheck_diner_removal_uses_before_and_changes_sorted() {
    let before = changed_profile();
    let mut after = profile();
    after.diners.0.remove(0);
    for c in &mut after.usual_week.cells.0 {
        c.attendee_ids.0.remove(0);
    }
    let w = selected();
    assert_eq!(
        targets(&before, &after, &w, "2026-12-28"),
        vec![w.cells[4].slot.clone()]
    );
}

#[test]
fn nonempty_extras_portions_and_links_survive_all_overlay_steps() {
    let p = profile();
    let mut w = selected();
    let s = &mut w.plan.selections.0[0];
    s.extras.0.push(ExtraSelection {
        extra_id: ExtraId(format!("ex_{:032x}", 1)),
        recipe: s.recipe.clone(),
        recipients: BoundedVec(vec![s.servings[0].clone()]),
    });
    s.meal_overrides.portions.0.push(PersonPortionOverride {
        person: s.servings[0].person.clone(),
        preset: PortionPresetName::Smaller,
        adjustment: PositiveRational::new(3, 4).unwrap(),
    });
    w.plan.batch_links.0.push(BatchLink {
        batch_id: BatchId(format!("ba_{:032x}", 1)),
        source_slot: w.cells[4].slot.clone(),
        target_slot: w.cells[7].slot.clone(),
        components: BoundedVec(vec![ComponentAllocation {
            component_id: ComponentId(format!("co_{:032x}", 1)),
            quantity: Quantity {
                amount: PositiveRational::new(1, 1).unwrap(),
                unit: Unit::Each,
                form: IngredientForm::Prepared,
            },
        }]),
    });
    let original = canonical_bytes(&w.plan).unwrap();
    for step in 0..4 {
        let mut e = edit(&w);
        if step % 2 == 0 {
            e.meal_type_enabled.lunch = step >= 2;
        } else {
            e.day_enabled.0[1] = step >= 2;
        }
        w = apply(&w, &e, &p).unwrap().candidate;
        assert_eq!(canonical_bytes(&w.plan).unwrap(), original);
    }
}

#[test]
fn at15_active_malformed_instant_and_mismatch() {
    for instant in ["not-an-instant", "2026-12-29T19:00:00Z"] {
        let mut w = selected();
        let mut m = moment(
            "2026-12-29",
            "18:00:00",
            "Europe/London",
            Resolution::Unambiguous,
            "2026-12-29T18:00:00Z",
        );
        m.instant = Instant(instant.into());
        w.plan.selections.0[0].meal_overrides.eat_by = Nullable(Some(m));
        error(project(&profile(), &w, &[]), ErrorCode::InvalidValue);
    }
}
#[test]
fn projection_is_clock_free_union_deduplicates_and_preserves_false() {
    let mut p = profile();
    p.diners.0[0].restrictions = p.diners[1].restrictions.clone();
    p.diners.0[1].restrictions = RestrictionDeclaration::Confirmed {
        confirmed_at: Instant("2199-01-01T00:00:00Z".into()),
        requirements: BoundedVec(vec![HardRequirement::Diet {
            code: DietCode::Vegetarian,
        }]),
    };
    let mut w = week();
    w.defaults_snapshot.suggest_leftovers = false;
    let r = project(&p, &w, &[]).unwrap();
    assert!(r.iter().all(|r| r.shared_hard_requirements.len() == 1
        && !r.leftovers_allowed
        && !r.mobility.reheating));
    let mut after = p.clone();
    after.diners.0[0].restrictions = after.diners[1].restrictions.clone();
    after.diners.0[0].display_name = Nullable(Some(NameText("Different".into())));
    assert!(targets(&p, &after, &selected(), "2026-12-28").is_empty());
}
#[test]
fn resolution_native_invalid_slot_and_retained_override_all_choices() {
    let p = profile();
    let mut w = selected();
    let mut r = resolution(&w, 4, Resolution::Unambiguous);
    r.slot.date = LocalDate("2026-02-30".into());
    error(project(&p, &w, &[r]), ErrorCode::InvalidValue);
    w.plan.selections.0[0].meal_overrides.eat_by = Nullable(Some(moment(
        "2026-12-29",
        "12:30:00",
        "Europe/London",
        Resolution::Unambiguous,
        "2026-12-29T12:30:00Z",
    )));
    for value in [
        Resolution::Unambiguous,
        Resolution::Earlier,
        Resolution::Later,
    ] {
        error(
            project(&p, &w, &[resolution(&w, 4, value)]),
            ErrorCode::InvalidValue,
        );
    }
}
