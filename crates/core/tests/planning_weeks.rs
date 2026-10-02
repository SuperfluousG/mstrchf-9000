use mstrchf_contracts::*;
use mstrchf_core::weeks::*;
use std::collections::BTreeSet;

fn profile() -> HouseholdProfile {
    parse_document(include_bytes!(
        "../../../tests/fixtures/households/p03ab/a1a2-profile.json"
    ))
    .unwrap()
}
fn instant() -> Instant {
    Instant("2026-12-21T12:00:00Z".into())
}
fn zone() -> ZoneId {
    ZoneId("Europe/London".into())
}
fn ids() -> [WeekId; 4] {
    [1, 2, 3, 4].map(|n| WeekId(format!("wk_{n:032x}")))
}
fn error<T: std::fmt::Debug>(r: Result<T, DomainError>, code: ErrorCode) {
    let e = r.unwrap_err();
    assert_eq!(e.code, code, "{e:?}");
    assert!(!e.issues.is_empty());
    assert!(e.issues.len() <= 100);
}
fn vector(case_id: &str) -> [Week; 4] {
    let vectors: serde_json::Value = serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/households/p03ab/a1a2-horizons.json"
    ))
    .unwrap();
    let v = vectors
        .as_array()
        .unwrap()
        .iter()
        .find(|v| v["case_id"] == case_id)
        .unwrap();
    let today = LocalDate(v["today"].as_str().unwrap().into());
    let zone = zone();
    let ids = ids();
    let before = (today.clone(), zone.clone(), ids.clone());
    let ranges = planning_horizon(&today, &zone, &ids).unwrap();
    assert_eq!((today, zone, ids.clone()), before);
    let p = profile();
    let weeks = ranges
        .each_ref()
        .map(|r| new_week(&p, &p.connections.privacy, &instant(), r).unwrap());
    let mut dates = BTreeSet::new();
    let mut slots = BTreeSet::new();
    for (i, (r, w)) in ranges.iter().zip(&weeks).enumerate() {
        assert_eq!(r.week_id, ids[i]);
        assert_eq!(r.zone, self::zone());
        assert_eq!(r.monday.0, v["mondays"][i].as_str().unwrap());
        assert_eq!(w.cells.len(), 21);
        let ordered = w
            .cells
            .iter()
            .map(|c| c.slot.date.0.as_str())
            .collect::<BTreeSet<_>>();
        assert_eq!(ordered.len(), 7);
        assert_eq!(*ordered.first().unwrap(), v["mondays"][i].as_str().unwrap());
        assert_eq!(*ordered.last().unwrap(), v["sundays"][i].as_str().unwrap());
        for c in &w.cells.0 {
            dates.insert(c.slot.date.0.clone());
            assert!(slots.insert((
                c.slot.week_id.0.clone(),
                c.slot.date.0.clone(),
                c.slot.meal_type
            )));
        }
        w.validate_shape().unwrap();
    }
    assert_eq!(dates.len(), 28);
    assert_eq!(slots.len(), 84);
    assert_eq!(p, profile());
    weeks
}
#[test]
fn w01_sunday_cross_year_horizon() {
    vector("W01");
}
#[test]
fn w02_monday_exact_seven_day_lead() {
    vector("W02");
}
#[test]
fn w03_tuesday_skips_too_close_monday() {
    vector("W03");
}
#[test]
fn w04_leap_day_occurs_on_one_date() {
    let weeks = vector("W04");
    assert_eq!(
        weeks
            .iter()
            .flat_map(|w| w.cells.iter())
            .filter(|c| c.slot.date.0 == "2028-02-29")
            .count(),
        3
    );
}
#[test]
fn w05_spring_dst_keeps_seven_civil_dates() {
    vector("W05");
}
#[test]
fn w06_autumn_dst_keeps_seven_civil_dates() {
    vector("W06");
}
#[test]
fn w07_local_midnight_conversion_changes_horizon() {
    let at = Instant("2026-12-21T23:30:00Z".into());
    let tokyo = ZoneId("Asia/Tokyo".into());
    let before = (at.clone(), tokyo.clone());
    let london_date = local_planning_date(&at, &zone()).unwrap();
    let tokyo_date = local_planning_date(&at, &tokyo).unwrap();
    assert_eq!(london_date.0, "2026-12-21");
    assert_eq!(tokyo_date.0, "2026-12-22");
    assert_eq!(
        planning_horizon(&london_date, &zone(), &ids()).unwrap()[0]
            .monday
            .0,
        "2026-12-28"
    );
    assert_eq!(
        planning_horizon(&tokyo_date, &tokyo, &ids()).unwrap()[0]
            .monday
            .0,
        "2027-01-04"
    );
    assert_eq!((at, tokyo), before);
    // Conversion must check the local civil date, including the full supported boundary.
    error(
        local_planning_date(&Instant("1899-12-31T00:00:00Z".into()), &zone()),
        ErrorCode::InvalidValue,
    );
    error(
        local_planning_date(
            &Instant("2199-12-31T23:30:00Z".into()),
            &ZoneId("Asia/Tokyo".into()),
        ),
        ErrorCode::InvalidValue,
    );
    assert_eq!(
        local_planning_date(
            &Instant("1899-12-31T23:30:00Z".into()),
            &ZoneId("Asia/Tokyo".into())
        )
        .unwrap()
        .0,
        "1900-01-01"
    );
}
#[test]
fn w08_lower_date_boundary() {
    vector("W08");
}
#[test]
fn w09_last_sunday_and_lead_overflow_refuse_entire_horizon() {
    for day in ["2199-11-27", "2199-12-31"] {
        let date = LocalDate(day.into());
        let before = date.clone();
        error(
            planning_horizon(&date, &zone(), &ids()),
            ErrorCode::ArithmeticOverflow,
        );
        assert_eq!(date, before);
    }
    let p = profile();
    let range = WeekRange {
        week_id: ids()[0].clone(),
        monday: LocalDate("2199-12-30".into()),
        zone: zone(),
    };
    error(
        new_week(&p, &p.connections.privacy, &instant(), &range),
        ErrorCode::ArithmeticOverflow,
    );
}
#[test]
fn w10_direct_invalid_inputs_refuse_without_panic_or_mutation() {
    let today = LocalDate("2026-12-21".into());
    for day in ["2026-02-29", "2200-01-01", "bad"] {
        error(
            planning_horizon(&LocalDate(day.into()), &zone(), &ids()),
            ErrorCode::InvalidValue,
        );
    }
    error(
        planning_horizon(&today, &ZoneId("Europe/NotAZone".into()), &ids()),
        ErrorCode::InvalidValue,
    );
    let mut bad_ids = ids();
    bad_ids[2] = bad_ids[1].clone();
    let saved = bad_ids.clone();
    error(
        planning_horizon(&today, &zone(), &bad_ids),
        ErrorCode::InvalidValue,
    );
    assert_eq!(bad_ids, saved);
    bad_ids = ids();
    bad_ids[0] = WeekId(format!("wk_{:032x}", 0));
    error(
        planning_horizon(&today, &zone(), &bad_ids),
        ErrorCode::InvalidValue,
    );
    error(
        local_planning_date(&Instant("bad".into()), &zone()),
        ErrorCode::InvalidValue,
    );
    error(
        local_planning_date(&instant(), &ZoneId("Europe/NotAZone".into())),
        ErrorCode::InvalidValue,
    );
    let p = profile();
    let range = WeekRange {
        week_id: ids()[0].clone(),
        monday: LocalDate("2026-12-28".into()),
        zone: zone(),
    };
    let mut malformed = p.clone();
    malformed.usual_week.day_enabled.0.clear();
    error(
        new_week(&malformed, &p.connections.privacy, &instant(), &range),
        ErrorCode::InvalidValue,
    );
    let mut stale = p.connections.privacy.clone();
    stale.revision = Revision(2);
    error(
        new_week(&p, &stale, &instant(), &range),
        ErrorCode::ContextStale,
    );
    let mut bad_range = range.clone();
    bad_range.week_id = WeekId("bad".into());
    error(
        new_week(&p, &p.connections.privacy, &instant(), &bad_range),
        ErrorCode::InvalidValue,
    );
    assert_eq!(p, profile());
}
#[test]
fn w11_new_week_snapshots_defaults_and_exact_empty_state() {
    let p = profile();
    let before = p.clone();
    let range = WeekRange {
        week_id: ids()[0].clone(),
        monday: LocalDate("2026-12-28".into()),
        zone: zone(),
    };
    let range_before = range.clone();
    let w = new_week(&p, &p.connections.privacy, &instant(), &range).unwrap();
    let mut expected: Week = parse_document(include_bytes!(
        "../../../tests/fixtures/households/p03ab/a1a2-week.json"
    ))
    .unwrap();
    expected.revision = Revision(1);
    expected.plan.revision = Revision(1);
    assert_eq!(w, expected);
    assert_eq!(p, before);
    assert_eq!(range, range_before);
    assert_eq!(w.defaults_snapshot, p.usual_week);
    assert!(w.plan.selections.is_empty());
    assert!(w.plan.batch_links.is_empty());
    assert_eq!(w.approved_plan, Nullable(None));
    assert_eq!(w.confirmed_order, Nullable(None));
    assert_eq!(w.publication, PublicationState::NotRequested);
    let mut altered = p.clone();
    altered
        .usual_week
        .enabled_meal_types
        .0
        .retain(|m| *m != MealType::Lunch);
    altered.usual_week.day_enabled.0[1] = false;
    altered.usual_week.cells.0[4].choice = MealLocation::OnTheMove;
    altered.usual_week.cells.0[4].attendee_ids =
        BoundedVec(vec![DinerId(format!("dn_{:032x}", 2))]);
    // Reverse the sequence: construction must match weekday/type, not input index.
    altered.usual_week.cells.0.reverse();
    let other = new_week(&altered, &altered.connections.privacy, &instant(), &range).unwrap();
    assert!(!other.meal_type_enabled.lunch);
    assert!(!other.day_enabled[1]);
    let lunch = other
        .cells
        .iter()
        .find(|c| c.slot.date.0 == "2026-12-29" && c.slot.meal_type == MealType::Lunch)
        .unwrap();
    assert_eq!(lunch.choice, MealLocation::OnTheMove);
    assert_eq!(
        lunch.attendee_ids.0,
        vec![DinerId(format!("dn_{:032x}", 2))]
    );
    assert_eq!(w, expected);
    assert_eq!(other.defaults_snapshot, altered.usual_week);
    altered.usual_week.cells.0[0].choice = MealLocation::Skip;
    assert_ne!(other.defaults_snapshot, altered.usual_week);
}
#[test]
fn w12_zone_mismatch_and_non_monday_refuse_without_retiming() {
    let p = profile();
    let mut range = WeekRange {
        week_id: ids()[0].clone(),
        monday: LocalDate("2026-12-28".into()),
        zone: ZoneId("Asia/Tokyo".into()),
    };
    let saved = range.clone();
    error(
        new_week(&p, &p.connections.privacy, &instant(), &range),
        ErrorCode::IdentityMismatch,
    );
    assert_eq!(range, saved);
    range.zone = zone();
    range.monday = LocalDate("2026-12-29".into());
    error(
        new_week(&p, &p.connections.privacy, &instant(), &range),
        ErrorCode::InvalidValue,
    );
    assert_eq!(p, profile());
}
