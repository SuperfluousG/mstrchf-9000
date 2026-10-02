//! Pure availability candidates and projections. No persistence or food verdicts.
use chrono::{LocalResult, NaiveDate, NaiveTime, TimeZone, Utc};
use chrono_tz::Tz;
use mstrchf_contracts::*;
use std::collections::{BTreeMap, BTreeSet};

/// Retains input counters. Store owns CAS, increments and downstream reassessment.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WeekEditEffect {
    pub candidate: Week,
    pub recheck_slots: Vec<SlotId>,
}

/// An explicit choice for one active default wall time, never an override.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SlotResolution {
    pub slot: SlotId,
    pub resolution: Resolution,
}

/// Includes unoccupied active cells; this is not recipe eligibility.
pub fn eligible_slots(week: &Week) -> Result<Vec<SlotId>, DomainError> {
    active_slots(week)
}

pub fn prepare_week_edit(
    current: &Week,
    edit: &WeekEdit,
    expected_revision: Revision,
    profile: &HouseholdProfile,
    evaluated_at: &Instant,
) -> Result<WeekEditEffect, DomainError> {
    shape_at(evaluated_at, "/evaluated_at")?;
    shape(&expected_revision)?;
    shape(current)?;
    shape(edit)?;
    crate::household::validate_profile(profile, &profile.connections.privacy, evaluated_at)?;
    if edit.week_id != current.range.week_id || edit.defaults_snapshot != current.defaults_snapshot
    {
        return Err(err(
            ErrorCode::IdentityMismatch,
            "/week_id/defaults_snapshot",
        ));
    }
    if expected_revision != current.revision {
        return Err(err(ErrorCode::ContextStale, "/revision"));
    }
    references(profile, current, false)?;
    let mut candidate = current.clone();
    candidate.day_enabled = edit.day_enabled.clone();
    candidate.meal_type_enabled = edit.meal_type_enabled.clone();
    candidate.cells = edit.cells.clone();
    shape(&candidate)?;
    references(profile, &candidate, false)?;
    for (i, cell) in candidate.cells.iter().enumerate() {
        if let Some(occasion) = &cell.occasion.0 {
            for (j, guest) in occasion.guests.iter().enumerate() {
                let timestamp = match &guest.restrictions {
                    RestrictionDeclaration::Unconfirmed {} => continue,
                    RestrictionDeclaration::ConfirmedNone { confirmed_at }
                    | RestrictionDeclaration::Confirmed { confirmed_at, .. } => confirmed_at,
                };
                // Both operands have passed complete shape admission.
                if timestamp > evaluated_at {
                    return Err(err(
                        ErrorCode::InvalidValue,
                        &format!("/cells/{i}/occasion/guests/{j}/restrictions/confirmed_at"),
                    ));
                }
            }
        }
    }
    let before = active_slots(current)?;
    let after = active_slots(&candidate)?;
    let mut recheck_slots = Vec::new();
    for cell in &candidate.cells.0 {
        let original = current
            .cells
            .iter()
            .find(|c| c.slot == cell.slot)
            .ok_or_else(|| err(ErrorCode::ForeignReference, "/cells/slot"))?;
        if original != cell || before.contains(&cell.slot) != after.contains(&cell.slot) {
            recheck_slots.push(cell.slot.clone());
        }
    }
    sort_slots(&mut recheck_slots);
    Ok(WeekEditEffect {
        candidate,
        recheck_slots,
    })
}

pub fn resolve_meal_time(
    date: &LocalDate,
    time: &LocalTime,
    zone: &ZoneId,
    resolution: Option<Resolution>,
) -> Result<ZonedMoment, DomainError> {
    shape(date)?;
    shape(time)?;
    shape(zone)?;
    // Checked parsing also protects direct public tuple values before arithmetic.
    let d = NaiveDate::parse_from_str(&date.0, "%Y-%m-%d")
        .map_err(|_| err(ErrorCode::InvalidValue, "/date"))?;
    let t = NaiveTime::parse_from_str(&time.0, "%H:%M:%S")
        .map_err(|_| err(ErrorCode::InvalidValue, "/time"))?;
    let tz: Tz = zone
        .0
        .parse()
        .map_err(|_| err(ErrorCode::InvalidValue, "/zone"))?;
    let (value, selected) = match tz.from_local_datetime(&d.and_time(t)) {
        LocalResult::None => return Err(err(ErrorCode::DomainConflict, "/time")),
        LocalResult::Single(value) => match resolution {
            None | Some(Resolution::Unambiguous) => (value, Resolution::Unambiguous),
            _ => return Err(err(ErrorCode::InvalidValue, "/resolution")),
        },
        LocalResult::Ambiguous(a, b) => match resolution {
            Some(Resolution::Earlier) => (a.min(b), Resolution::Earlier),
            Some(Resolution::Later) => (a.max(b), Resolution::Later),
            None => return Err(err(ErrorCode::DomainConflict, "/resolution")),
            Some(Resolution::Unambiguous) => {
                return Err(err(ErrorCode::InvalidValue, "/resolution"));
            }
        },
    };
    ZonedMoment::new(
        date.clone(),
        time.clone(),
        zone.clone(),
        selected,
        Instant::try_from(
            value
                .with_timezone(&Utc)
                .format("%Y-%m-%dT%H:%M:%SZ")
                .to_string()
                .as_str(),
        )?,
    )
}

/// Derive all 21 civil slots atomically, validating retained state before null projection.
pub fn meal_requirements(
    profile: &HouseholdProfile,
    week: &Week,
    resolutions: &[SlotResolution],
) -> Result<[MealRequirements; 21], DomainError> {
    shape(profile)?;
    shape(week)?;
    references(profile, week, true)?;
    let active = active_slots(week)?;
    if resolutions.len() > 21 {
        return Err(err(ErrorCode::LimitExceeded, "/resolutions"));
    }
    let mut seen = Vec::new();
    for (i, entry) in resolutions.iter().enumerate() {
        let path = format!("/resolutions/{i}");
        shape_at(&entry.slot, &format!("{path}/slot"))?;
        if seen.contains(&entry.slot)
            || !active.contains(&entry.slot)
            || week
                .plan
                .selections
                .iter()
                .any(|s| s.slot == entry.slot && s.meal_overrides.eat_by.0.is_some())
        {
            return Err(err(ErrorCode::InvalidValue, &path));
        }
        seen.push(entry.slot.clone());
    }
    let mut cells: Vec<_> = week.cells.iter().collect();
    cells.sort_by_key(|c| slot_key(&c.slot));
    let mut requirements = Vec::with_capacity(21);
    for cell in cells {
        let enabled = active.contains(&cell.slot);
        let mut attendees = Vec::new();
        if enabled {
            for id in &cell.attendee_ids.0 {
                let diner = profile
                    .diners
                    .iter()
                    .find(|d| &d.diner_id == id)
                    .ok_or_else(|| err(ErrorCode::ForeignReference, "/cells/attendee_ids"))?;
                attendees.push(AttendeeRequirement {
                    person: PersonRef::Diner {
                        diner_id: id.clone(),
                    },
                    age_band: diner.age_band,
                    hard_requirements: BoundedVec(hard(
                        &diner.restrictions,
                        "/diners/restrictions",
                    )?),
                    portion: diner.portions.clone(),
                });
            }
            if let Some(occasion) = &cell.occasion.0 {
                for guest in &occasion.guests.0 {
                    attendees.push(AttendeeRequirement {
                        person: PersonRef::Guest {
                            occasion_id: occasion.occasion_id.clone(),
                            guest_id: guest.guest_id.clone(),
                        },
                        age_band: guest.age_band,
                        hard_requirements: BoundedVec(hard(
                            &guest.restrictions,
                            "/occasion/guests/restrictions",
                        )?),
                        portion: guest.portion.clone(),
                    });
                }
            }
            // Sort by canonical PersonRef bytes, not enum declaration order.
            let mut keyed: Vec<_> = attendees
                .into_iter()
                .map(|a| Ok((canonical_bytes(&a.person)?, a)))
                .collect::<Result<_, DomainError>>()?;
            keyed.sort_by(|a, b| a.0.cmp(&b.0));
            attendees = keyed.into_iter().map(|(_, a)| a).collect();
        }
        let mut union = BTreeMap::new();
        for requirement in attendees.iter().flat_map(|a| a.hard_requirements.iter()) {
            union.insert(canonical_bytes(requirement)?, requirement.clone());
        }
        if union.len() > 64 {
            return Err(err(ErrorCode::LimitExceeded, "/shared_hard_requirements"));
        }
        let eat_by = if !enabled {
            None
        } else if let Some(moment) = week
            .plan
            .selections
            .iter()
            .find(|s| s.slot == cell.slot)
            .and_then(|s| s.meal_overrides.eat_by.0.as_ref())
        {
            Some(moment.clone())
        } else {
            let time = profile
                .preferences
                .meal_times
                .iter()
                .find(|m| m.meal_type == cell.slot.meal_type)
                .ok_or_else(|| err(ErrorCode::MissingField, "/preferences/meal_times"))?;
            let resolution = resolutions
                .iter()
                .find(|r| r.slot == cell.slot)
                .map(|r| r.resolution);
            Some(resolve_meal_time(
                &cell.slot.date,
                &time.time,
                &week.range.zone,
                resolution,
            )?)
        };
        let requirement = MealRequirements {
            slot: cell.slot.clone(),
            enabled,
            location: cell.choice,
            attendees: BoundedVec(attendees),
            shared_hard_requirements: BoundedVec(union.into_values().collect()),
            equipment: profile.preferences.equipment.clone(),
            mobility: profile.preferences.mobility.clone(),
            eat_by: Nullable(eat_by),
            preferred_hands_on_minutes: profile.preferences.preferred_hands_on_minutes,
            leftovers_allowed: week.defaults_snapshot.suggest_leftovers,
        };
        shape(&requirement)?;
        requirements.push(requirement);
    }
    requirements
        .try_into()
        .map_err(|_| err(ErrorCode::InvalidValue, "/cells"))
}

/// Conservative selected-slot targets only. The store must supply complete retained
/// current and original inputs; this function cannot prove archive completeness.
pub fn restriction_recheck_slots(
    before: &HouseholdProfile,
    after: &HouseholdProfile,
    week: &Week,
    as_of: &LocalDate,
) -> Result<Vec<SlotId>, DomainError> {
    shape(before)?;
    shape(after)?;
    shape(week)?;
    shape(as_of)?;
    if before.household_id != after.household_id {
        return Err(err(ErrorCode::IdentityMismatch, "/household_id"));
    }
    references(before, week, false)?;
    let ids: BTreeSet<_> = before
        .diners
        .iter()
        .chain(after.diners.iter())
        .map(|d| &d.diner_id)
        .collect();
    let mut changed = BTreeSet::new();
    for id in ids {
        if hard_set(before.diners.iter().find(|d| &d.diner_id == id))?
            != hard_set(after.diners.iter().find(|d| &d.diner_id == id))?
        {
            changed.insert(id);
        }
    }
    let changed_person =
        |p: &PersonRef| matches!(p, PersonRef::Diner { diner_id } if changed.contains(diner_id));
    let mut slots = Vec::new();
    for selection in &week.plan.selections.0 {
        if &selection.slot.date < as_of {
            continue;
        }
        let cell = week
            .cells
            .iter()
            .find(|c| c.slot == selection.slot)
            .ok_or_else(|| err(ErrorCode::ForeignReference, "/plan/selections/slot"))?;
        if cell.attendee_ids.iter().any(|id| changed.contains(id))
            || selection.servings.iter().any(|s| changed_person(&s.person))
            || selection
                .extras
                .iter()
                .flat_map(|e| e.recipients.iter())
                .any(|s| changed_person(&s.person))
        {
            slots.push(selection.slot.clone());
        }
    }
    sort_slots(&mut slots);
    slots.dedup();
    Ok(slots)
}

fn references(
    profile: &HouseholdProfile,
    week: &Week,
    require_confirmed: bool,
) -> Result<(), DomainError> {
    let mut occasions = BTreeSet::new();
    for (i, cell) in week.cells.iter().enumerate() {
        for id in &cell.attendee_ids.0 {
            if !profile.diners.iter().any(|d| &d.diner_id == id) {
                return Err(err(
                    ErrorCode::ForeignReference,
                    &format!("/cells/{i}/attendee_ids"),
                ));
            }
        }
        if let Some(occasion) = &cell.occasion.0 {
            if !occasions.insert(&occasion.occasion_id) {
                return Err(err(
                    ErrorCode::IdentityMismatch,
                    &format!("/cells/{i}/occasion/occasion_id"),
                ));
            }
            if require_confirmed || week.plan.selections.iter().any(|s| s.slot == cell.slot) {
                for guest in &occasion.guests.0 {
                    hard(
                        &guest.restrictions,
                        &format!("/cells/{i}/occasion/guests/restrictions"),
                    )?;
                }
            }
        }
    }
    for (i, selection) in week.plan.selections.iter().enumerate() {
        let path = format!("/plan/selections/{i}");
        let cell = week
            .cells
            .iter()
            .find(|c| c.slot == selection.slot)
            .ok_or_else(|| err(ErrorCode::ForeignReference, &format!("{path}/slot")))?;
        for person in selection
            .servings
            .iter()
            .map(|s| &s.person)
            .chain(
                selection
                    .extras
                    .iter()
                    .flat_map(|e| e.recipients.iter().map(|s| &s.person)),
            )
            .chain(selection.meal_overrides.portions.iter().map(|p| &p.person))
        {
            let known = match person {
                PersonRef::Diner { diner_id } => {
                    profile.diners.iter().any(|d| &d.diner_id == diner_id)
                }
                PersonRef::Guest {
                    occasion_id,
                    guest_id,
                } => cell.occasion.0.as_ref().is_some_and(|o| {
                    &o.occasion_id == occasion_id
                        && o.guests.iter().any(|g| &g.guest_id == guest_id)
                }),
            };
            if !known {
                return Err(err(ErrorCode::ForeignReference, &format!("{path}/person")));
            }
        }
        if let Some(moment) = &selection.meal_overrides.eat_by.0
            && (moment.local_date != cell.slot.date || moment.zone != week.range.zone)
        {
            return Err(err(
                ErrorCode::IdentityMismatch,
                &format!("{path}/meal_overrides/eat_by"),
            ));
        }
    }
    Ok(())
}
fn hard(
    declaration: &RestrictionDeclaration,
    path: &str,
) -> Result<Vec<HardRequirement>, DomainError> {
    match declaration {
        RestrictionDeclaration::Unconfirmed {} => Err(err(ErrorCode::DomainConflict, path)),
        RestrictionDeclaration::ConfirmedNone { .. } => Ok(vec![]),
        RestrictionDeclaration::Confirmed { requirements, .. } => {
            let mut keyed = BTreeMap::new();
            for r in &requirements.0 {
                keyed.insert(canonical_bytes(r)?, r.clone());
            }
            Ok(keyed.into_values().collect())
        }
    }
}
fn hard_set(diner: Option<&Diner>) -> Result<BTreeSet<Vec<u8>>, DomainError> {
    match diner {
        Some(d) => hard(&d.restrictions, "/diners/restrictions")?
            .iter()
            .map(canonical_bytes)
            .collect(),
        None => Ok(BTreeSet::new()),
    }
}
fn slot_key(slot: &SlotId) -> (&LocalDate, u8) {
    (
        &slot.date,
        match slot.meal_type {
            MealType::Breakfast => 0,
            MealType::Lunch => 1,
            MealType::Dinner => 2,
        },
    )
}
fn sort_slots(slots: &mut [SlotId]) {
    slots.sort_by(|a, b| slot_key(a).cmp(&slot_key(b)));
}
fn err(code: ErrorCode, path: &str) -> DomainError {
    DomainError::new(code, path)
}
fn shape(value: &impl ValidateShape) -> Result<(), DomainError> {
    shape_at(value, "")
}
fn shape_at(value: &impl ValidateShape, prefix: &str) -> Result<(), DomainError> {
    value.validate_shape().map_err(|mut issues| {
        for issue in &mut issues {
            issue.path = format!("{prefix}{}", issue.path);
        }
        DomainError {
            code: issues.first().map_or(ErrorCode::InvalidValue, |i| i.code),
            truncated: issues.len() > 100,
            issues: BoundedVec(issues.into_iter().take(100).collect()),
            retry: Retry::Never,
        }
    })
}
