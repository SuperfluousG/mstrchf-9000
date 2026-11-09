use super::*;
use chrono::Datelike;

/// Exact root-adopted definition. Scope predicates confer no library admission
/// or evidence that actual products and handling meet the stated conditions.
pub const OVERNIGHT_OATS_SCOPE: PersonalUseScope = PersonalUseScope {
    scope_id: "peach-overnight-oats-sunday-prep-monday-home-breakfast-v1",
    recipe_id: "rc_80000000000000000000000000000014",
    recipe_revision: 1,
    recipe_digest: "6d5a8300e35d2b8586a17f3a07ccf35871ba3a0559b1ca4b3d18f60c91c7f1bf",
    meal_type: MealType::Breakfast,
    source_url: "https://www.nutrition.va.gov/NUTRITION/docs/Recipes/Breakfast/BerryVanillaOvernightOatsOCT2022.pdf",
    packet_canonical_digest: "26bac55278a115905261b2385e46fe27480c4cda41213e6e740490464286482d",
    attention_reservation_minutes: 10,
    timing_is_estimate: true,
    portion_convention: "One complete fixed peach–vanilla oats unit for one unrestricted resident Adult Regular diner; no measured yield, nutrition or portion adequacy claim.",
};

pub fn is_overnight_oats(reference: &RecipeRef) -> bool {
    !OVERNIGHT_OATS_SCOPE.recipe_digest.is_empty() && pinned(reference, &OVERNIGHT_OATS_SCOPE)
}

pub(super) fn notices() -> Vec<String> {
    vec![OVERNIGHT_OATS_SCOPE.portion_convention.into(),
        "Cold at-home Monday breakfast between 06:00 and 10:00 for one confirmed unrestricted resident Adult Regular ×1 diner, with a working fridge. No extras, travel, portion adjustment, linked batch or further carry-over.".into(),
        "Reserve five active minutes on the previous Sunday evening to mix, cover and refrigerate, then five active minutes on Monday to check, retrieve and serve cold. These are separate visible sessions. Ingredients must arrive by the start of Sunday mixing; the overnight interval is not a continuous work or equipment reservation.".into(),
        "Keep covered at a checked 0–5°C continuously for at least five hours before retrieval. Serve only the next local morning and within 24 hours of the start of original preparation, or sooner if actual ingredient labels require it. Unknown or failed chilling, dates or handling mean do not use this breakfast.".into(),
        "The timetable records planned work and conditions, not observed refrigeration or readiness. Actual product suitability, temperature, completion, nutrition and human portion calibration remain unverified.".into()]
}

pub(super) fn check_timing(
    meal: &MealRequirements,
    selection: &MealSelection,
    options: &BoardOptions,
) -> Result<(), DomainError> {
    if meal.location != MealLocation::AtHome || !meal.equipment.contains(&EquipmentCode::Fridge) {
        return Err(unsupported("overnight_home_fridge"));
    }
    if selection.portable_cold.is_some() || options.portable_cold.is_some() {
        return Err(unsupported("overnight_no_travel"));
    }
    let deadline = selection
        .meal_overrides
        .eat_by
        .0
        .as_ref()
        .or(meal.eat_by.0.as_ref())
        .ok_or_else(|| unsupported("overnight_meal_time"))?;
    let validated = ZonedMoment::new(
        deadline.local_date.clone(),
        deadline.local_time.clone(),
        deadline.zone.clone(),
        deadline.resolution,
        deadline.instant.clone(),
    )
    .map_err(|_| refusal(ErrorCode::InvalidValue, "overnight_meal_time"))?;
    if validated.local_date != meal.slot.date {
        return Err(refusal(ErrorCode::IdentityMismatch, "overnight_meal_time"));
    }
    if options
        .overrides
        .as_ref()
        .and_then(|value| value.eat_by.0.as_ref())
        .is_some_and(|value| value != deadline)
        || options
            .eat_by_time
            .as_ref()
            .is_some_and(|value| value != &deadline.local_time)
        || options
            .time_resolution
            .is_some_and(|value| value != deadline.resolution)
    {
        return Err(refusal(ErrorCode::IdentityMismatch, "request_meal_time"));
    }
    let zone: chrono_tz::Tz = validated
        .zone
        .0
        .parse()
        .map_err(|_| refusal(ErrorCode::InvalidValue, "overnight_meal_time"))?;
    let use_at = chrono::DateTime::parse_from_rfc3339(&validated.instant.0)
        .map_err(|_| refusal(ErrorCode::InvalidValue, "overnight_meal_time"))?;
    let local_meal = use_at.with_timezone(&zone);
    let start = use_at
        .checked_sub_signed(chrono::Duration::seconds(43_500))
        .ok_or_else(|| refusal(ErrorCode::InvalidValue, "overnight_first_handling"))?;
    let chilled_from = start + chrono::Duration::minutes(5);
    let retrieval = use_at - chrono::Duration::minutes(5);
    let local_start = start.with_timezone(&zone);
    let time = |hour, minute| chrono::NaiveTime::from_hms_opt(hour, minute, 0).expect("fixed time");
    if local_meal.weekday() != chrono::Weekday::Mon
        || !(time(6, 0)..=time(10, 0)).contains(&local_meal.time())
        || local_start.weekday() != chrono::Weekday::Sun
        || local_start.date_naive().succ_opt() != Some(local_meal.date_naive())
        || !(time(17, 55)..=time(21, 55)).contains(&local_start.time())
        || (retrieval - chilled_from).num_seconds() < 18_000
        || (use_at - start).num_seconds() > 86_400
    {
        return Err(unsupported("overnight_sunday_to_monday"));
    }
    Ok(())
}
