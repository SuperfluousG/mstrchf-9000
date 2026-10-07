//! Host-owned synthetic development input. Never production recipe admission.
use mstrchf_contracts::*;

pub fn now() -> Result<Instant, DomainError> {
    let seconds = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH)
        .map_err(crate::records::storage)?.as_secs();
    let date = chrono::DateTime::from_timestamp(i64::try_from(seconds).map_err(crate::records::storage)?, 0)
        .ok_or_else(|| DomainError::new(ErrorCode::InvalidValue, "/development/time"))?;
    Instant::try_from(date.to_rfc3339_opts(chrono::SecondsFormat::Secs, true).as_str())
}

pub fn seed_profile(snapshot: &Snapshot) -> Result<HouseholdProfile, DomainError> {
    let mut p: HouseholdProfile = parse_document(include_bytes!("../../../tests/fixtures/households/p03ab/a1a2-profile.json"))?;
    p.household_id = snapshot.household_draft.household_id.clone();
    p.revision = Revision(1);
    p.diners.0.truncate(1);
    p.diners[0].display_name = Nullable(Some(NameText("Development diner".into())));
    p.diners[0].restrictions = RestrictionDeclaration::Confirmed {
        confirmed_at: Instant("2026-01-01T12:00:00Z".into()),
        requirements: BoundedVec(vec![HardRequirement::Diet { code: DietCode::Vegetarian }]),
    };
    p.usual_week.suggest_leftovers = false;
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
    if let Some(week) = snapshot.weeks.first() { return Ok(week.clone()); }
    let profile = snapshot.profile.0.clone().map_or_else(|| seed_profile(snapshot), Ok)?;
    let today = mstrchf_core::weeks::local_planning_date(evaluated_at, &profile.usual_week.zone)?;
    let ids = [1, 2, 3, 4].map(|n| WeekId(format!("wk_{n:032x}")));
    let ranges = mstrchf_core::weeks::planning_horizon(&today, &profile.usual_week.zone, &ids)?;
    mstrchf_core::weeks::new_week(&profile, &snapshot.privacy, evaluated_at, &ranges[0])
}

#[cfg(test)]
mod tests {
    use super::*;
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
