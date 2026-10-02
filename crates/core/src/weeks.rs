//! Explicit local dates and immutable new-week construction, with no clock or IO.
use crate::household::validate_profile;
use chrono::{Datelike, Days, NaiveDate};
use chrono_tz::Tz;
use mstrchf_contracts::*;
use std::collections::BTreeSet;

/// Resolve the supplied instant in the supplied zone, then admit its civil date.
pub fn local_planning_date(
    evaluated_at: &Instant,
    zone: &ZoneId,
) -> Result<LocalDate, DomainError> {
    shape(evaluated_at)?;
    shape(zone)?;
    let instant = chrono::DateTime::parse_from_rfc3339(&evaluated_at.0)
        .map_err(|_| DomainError::new(ErrorCode::InvalidValue, "/evaluated_at"))?;
    let zone: Tz = zone
        .0
        .parse()
        .map_err(|_| DomainError::new(ErrorCode::InvalidValue, "/zone"))?;
    let local_date = instant.with_timezone(&zone).date_naive().to_string();
    LocalDate::try_from(local_date.as_str())
        .map_err(|_| DomainError::new(ErrorCode::InvalidValue, "/local_date"))
}

/// Four whole Monday-Sunday weeks, starting at least seven calendar days ahead.
/// The caller supplies all identities; no horizon is returned on endpoint overflow.
pub fn planning_horizon(
    today: &LocalDate,
    zone: &ZoneId,
    week_ids: &[WeekId; 4],
) -> Result<[WeekRange; 4], DomainError> {
    shape(today)?;
    shape(zone)?;
    let mut seen = BTreeSet::new();
    for id in week_ids {
        shape(id)?;
        if !seen.insert(id) {
            return Err(DomainError::new(ErrorCode::InvalidValue, "/week_ids"));
        }
    }
    let date = checked_date(today)?;
    let earliest = add_days(date, 7)?;
    let offset = (7 - earliest.weekday().num_days_from_monday()) % 7;
    let first = add_days(earliest, u64::from(offset))?;
    let mut ranges = Vec::with_capacity(4);
    for (index, id) in week_ids.iter().enumerate() {
        let monday = add_days(first, index as u64 * 7)?;
        // WeekRange carries only Monday. Explicitly check the final Sunday too.
        bounded_date(add_days(monday, 6)?)?;
        ranges.push(WeekRange {
            week_id: id.clone(),
            monday: bounded_date(monday)?,
            zone: zone.clone(),
        });
    }
    ranges
        .try_into()
        .map_err(|_| DomainError::new(ErrorCode::InvalidValue, "/week_ids"))
}

/// Copy the validated usual week into 21 dated cells and an empty revision-one plan.
pub fn new_week(
    profile: &HouseholdProfile,
    policy: &PrivacyPolicy,
    evaluated_at: &Instant,
    range: &WeekRange,
) -> Result<Week, DomainError> {
    validate_profile(profile, policy, evaluated_at)?;
    shape(range)?;
    if range.zone != profile.usual_week.zone {
        return Err(DomainError::new(ErrorCode::IdentityMismatch, "/range/zone"));
    }
    let monday = checked_date(&range.monday)?;
    let usual = &profile.usual_week;
    let weekdays = [
        Weekday::Monday,
        Weekday::Tuesday,
        Weekday::Wednesday,
        Weekday::Thursday,
        Weekday::Friday,
        Weekday::Saturday,
        Weekday::Sunday,
    ];
    let mut cells = Vec::with_capacity(21);
    for (offset, weekday) in weekdays.into_iter().enumerate() {
        let date = bounded_date(add_days(monday, offset as u64)?)?;
        for meal_type in [MealType::Breakfast, MealType::Lunch, MealType::Dinner] {
            let default = usual
                .cells
                .iter()
                .find(|c| c.weekday == weekday && c.meal_type == meal_type)
                .ok_or_else(|| DomainError::new(ErrorCode::MissingField, "/usual_week/cells"))?;
            cells.push(Cell {
                slot: SlotId {
                    week_id: range.week_id.clone(),
                    date: date.clone(),
                    meal_type,
                },
                choice: default.choice,
                attendee_ids: default.attendee_ids.clone(),
                occasion: Nullable(None),
            });
        }
    }
    let week = Week {
        schema_version: profile.schema_version,
        range: range.clone(),
        revision: Revision(1),
        defaults_snapshot: usual.clone(),
        day_enabled: usual.day_enabled.clone(),
        meal_type_enabled: MealTypeEnabled {
            breakfast: usual.enabled_meal_types.contains(&MealType::Breakfast),
            lunch: usual.enabled_meal_types.contains(&MealType::Lunch),
            dinner: usual.enabled_meal_types.contains(&MealType::Dinner),
        },
        cells: BoundedVec(cells),
        plan: WeekPlan {
            week_id: range.week_id.clone(),
            revision: Revision(1),
            selections: BoundedVec(vec![]),
            batch_links: BoundedVec(vec![]),
        },
        approved_plan: Nullable(None),
        confirmed_order: Nullable(None),
        publication: PublicationState::NotRequested,
    };
    shape(&week)?;
    Ok(week)
}

fn shape(value: &impl ValidateShape) -> Result<(), DomainError> {
    value.validate_shape().map_err(|issues| DomainError {
        code: issues.first().map_or(ErrorCode::InvalidValue, |i| i.code),
        truncated: issues.len() > 100,
        issues: BoundedVec(issues.into_iter().take(100).collect()),
        retry: Retry::Never,
    })
}

fn checked_date(date: &LocalDate) -> Result<NaiveDate, DomainError> {
    LocalDate::try_from(date.0.as_str())?;
    NaiveDate::parse_from_str(&date.0, "%Y-%m-%d")
        .map_err(|_| DomainError::new(ErrorCode::InvalidValue, "/date"))
}

fn add_days(date: NaiveDate, days: u64) -> Result<NaiveDate, DomainError> {
    date.checked_add_days(Days::new(days))
        .ok_or_else(|| DomainError::new(ErrorCode::ArithmeticOverflow, "/range"))
}

fn bounded_date(date: NaiveDate) -> Result<LocalDate, DomainError> {
    LocalDate::try_from(date.to_string().as_str())
        .map_err(|_| DomainError::new(ErrorCode::ArithmeticOverflow, "/range"))
}
