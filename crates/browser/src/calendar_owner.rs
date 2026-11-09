//! Owner-built calendar custody. Wire journals are copies, not native authority.
//! The calling domain boundary must audit BrowserState before invoking prepare.
use crate::{booking, calendar_view::*, catalogue::RuntimeCatalogue, BrowserState};
use chrono::{LocalResult, TimeZone};
use mstrchf_calendar::*;
use mstrchf_contracts::*;
use mstrchf_core::food::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CalendarJournalOwnerPin {
    pub week: WeekRevision,
    pub derived_digest: PayloadDigest,
    pub basis_digest: PayloadDigest,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CalendarJournalContributor {
    pub owner: CalendarJournalOwnerPin,
    pub session_id: PrepSessionId,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CalendarJournalEvent {
    pub event: CalendarEvent,
    pub contributors: BoundedVec<CalendarJournalContributor, 0, 4352>,
    pub sequence: u32,
    pub revised_at: Instant,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CalendarJournal {
    #[serde(default = "legacy_renderer", skip_serializing_if = "is_legacy_renderer")]
    pub renderer_version: u8,
    pub batch: EventBatch,
    pub prepared_at: Instant,
    pub saved_events: BoundedVec<CalendarJournalEvent, 1, 256>,
}
fn legacy_renderer() -> u8 { 1 }
fn is_legacy_renderer(version: &u8) -> bool { *version == 1 }
#[cfg(test)]
#[path = "../tests/support/portable_calendar_occupancy.rs"]
mod portable_occupancy_tests;
pub const CURRENT_RENDERER: u8 = 3;
pub const CORRECTED_DELIVERY_RENDERER: u8 = 4;
pub const ORIGINAL_COMPONENT_RENDERER: u8 = 5;
pub const CORRECTED_ORIGINAL_COMPONENT_RENDERER: u8 = 6;
pub const PORTABLE_COLD_RENDERER: u8 = 7;
pub const CORRECTED_PORTABLE_COLD_RENDERER: u8 = 8;
pub const GF_CHICKEN_RENDERER: u8 = 9;
pub const CORRECTED_GF_CHICKEN_RENDERER: u8 = 10;
pub const CHILLI_REMIX_RENDERER: u8 = 11;
pub const CORRECTED_CHILLI_REMIX_RENDERER: u8 = 12;
pub const OVERNIGHT_OATS_RENDERER: u8 = 13;
pub const CORRECTED_OVERNIGHT_OATS_RENDERER: u8 = 14;
fn renderer_for_plan(plan: &WeekPlan, corrected: bool) -> u8 {
    if contains_overnight_plan(plan) {
        return if corrected { CORRECTED_OVERNIGHT_OATS_RENDERER } else { OVERNIGHT_OATS_RENDERER };
    }
    if contains_chilli_plan(plan) {
        return if corrected { CORRECTED_CHILLI_REMIX_RENDERER } else { CHILLI_REMIX_RENDERER };
    }
    if contains_gf_chicken_plan(plan) {
        return if corrected { CORRECTED_GF_CHICKEN_RENDERER } else { GF_CHICKEN_RENDERER };
    }
    if contains_portable_plan(plan) {
        return if corrected { CORRECTED_PORTABLE_COLD_RENDERER } else { PORTABLE_COLD_RENDERER };
    }
    match (contains_courgette_plan(plan),corrected) {
        (false,false)=>CURRENT_RENDERER, (false,true)=>CORRECTED_DELIVERY_RENDERER,
        (true,false)=>ORIGINAL_COMPONENT_RENDERER, (true,true)=>CORRECTED_ORIGINAL_COMPONENT_RENDERER,
    }
}
fn current_presentation(draft: &mut EventDraft, test_kitchen: bool) -> Result<(), DomainError> {
    if draft.logical_key.kind != CalendarEventKind::Delivery {
        const WARNINGS: &str = "Warnings — unresolved food evidence";
        if let Some(start) = draft.body.0.find(WARNINGS) {
            let tail = &draft.body.0[start..];
            let end = tail.find("\n\n").ok_or_else(|| error(ErrorCode::IdentityMismatch, "/calendar/warnings_layout"))? + start;
            let content = &draft.body.0[end + 2..];
            let credits = content.find("\n\nCredits —").ok_or_else(|| error(ErrorCode::IdentityMismatch, "/calendar/credits_layout"))?;
            draft.body.0 = format!("{}Check the ingredient-label and date warnings below before starting. Freshness has not been established.\n\n{}\n\n{}{}", &draft.body.0[..start], &content[..credits], &draft.body.0[start..end], &content[credits..]);
        }
        // This packet's pinned one-step text already numbers its four actions.
        // Remove only the renderer's redundant outer number and add line breaks.
        if let (Some(start), Some(end)) = (draft.body.0.find("Instructions — execute each listed phase once"), draft.body.0.find("\nOriginal storage clock")) {
            let method = &draft.body.0[start..end];
            if method.contains("1. 1. Measure the pasta") {
                let readable = method.replacen("1. 1. Measure the pasta", "1. Measure the pasta", 1).replace(" 2. ", "\n2. ").replace(" 3. ", "\n3. ").replace(" 4. ", "\n4. ");
                draft.body.0 = format!("{}{}{}", &draft.body.0[..start], readable, &draft.body.0[end..]);
            }
        }
    }
    if test_kitchen {
        draft.title = Title(format!("MasterChef TEST · {}", draft.title.0));
        draft.body = LongText(format!("FICTIONAL TEST KITCHEN. No real purchase, delivery or cooking is confirmed by this entry.\n\n{}", draft.body.0));
    }
    draft.title.validate_shape().map_err(|_| error(ErrorCode::LimitExceeded, "/calendar/current_title"))?;
    draft.body.validate_shape().map_err(|_| error(ErrorCode::LimitExceeded, "/calendar/current_body"))?;
    Ok(())
}
fn error(code: ErrorCode, path: &str) -> DomainError {
    DomainError::new(code, path)
}
fn require(ok: bool, code: ErrorCode, path: &str) -> Result<(), DomainError> {
    if ok {
        Ok(())
    } else {
        Err(error(code, path))
    }
}
fn calendar_error(failure: CalendarFailure) -> DomainError {
    match failure {
        CalendarFailure::Invalid(error) => error,
        CalendarFailure::ReconciliationRequired { reason, .. } => reason,
        CalendarFailure::Blocked(_) => {
            error(ErrorCode::DomainConflict, "/calendar/current_continuation")
        }
    }
}
impl CalendarJournalEvent {
    fn native(&self) -> SavedCalendarEvent {
        SavedCalendarEvent {
            event: self.event.clone(),
            contributors: self
                .contributors
                .iter()
                .map(|member| SessionContribution {
                    owner: PlanOwnerPin {
                        week: member.owner.week.clone(),
                        derived_digest: member.owner.derived_digest.clone(),
                        basis_digest: member.owner.basis_digest.clone(),
                    },
                    session_id: member.session_id.clone(),
                })
                .collect(),
            sequence: self.sequence,
            revised_at: self.revised_at.clone(),
        }
    }
    fn from_native(row: &SavedCalendarEvent) -> Self {
        Self {
            event: row.event.clone(),
            contributors: BoundedVec(
                row.contributors
                    .iter()
                    .map(|member| CalendarJournalContributor {
                        owner: CalendarJournalOwnerPin {
                            week: member.owner.week.clone(),
                            derived_digest: member.owner.derived_digest.clone(),
                            basis_digest: member.owner.basis_digest.clone(),
                        },
                        session_id: member.session_id.clone(),
                    })
                    .collect(),
            ),
            sequence: row.sequence,
            revised_at: row.revised_at.clone(),
        }
    }
}

fn local_candidates(
    date: &LocalDate,
    time: &LocalTime,
    zone: &ZoneId,
) -> Result<Vec<i64>, DomainError> {
    let local = chrono::NaiveDateTime::parse_from_str(
        &format!("{}T{}", date.0, time.0),
        "%Y-%m-%dT%H:%M:%S",
    )
    .map_err(|_| error(ErrorCode::InvalidValue, "/calendar/occupancy/time"))?;
    let tz: chrono_tz::Tz = zone
        .0
        .parse()
        .map_err(|_| error(ErrorCode::InvalidValue, "/calendar/occupancy/zone"))?;
    match tz.from_local_datetime(&local) {
        LocalResult::Single(at) => Ok(vec![at.timestamp()]),
        LocalResult::Ambiguous(a, b) => Ok(vec![a.timestamp(), b.timestamp()]),
        LocalResult::None => Err(error(
            ErrorCode::ReconciliationRequired,
            "/calendar/occupancy/unresolved_clock",
        )),
    }
}
/// Only a conservative search bound, never an inferred event or recipe time.
/// The contract permits phase offsets +/-7d and elapsed envelopes up to7d.
fn coarse_span(week: &Week) -> Result<(i64, i64), DomainError> {
    let sunday = chrono::NaiveDate::parse_from_str(&week.range.monday.0, "%Y-%m-%d")
        .ok()
        .and_then(|date| date.checked_add_days(chrono::Days::new(6)))
        .ok_or_else(|| error(ErrorCode::InvalidValue, "/calendar/occupancy/week"))?;
    let lo = local_candidates(
        &week.range.monday,
        &LocalTime("00:00:00".into()),
        &week.range.zone,
    )?
    .into_iter()
    .min()
    .unwrap()
        - 604800;
    let hi = local_candidates(
        &LocalDate(sunday.format("%Y-%m-%d").to_string()),
        &LocalTime("23:59:59".into()),
        &week.range.zone,
    )?
    .into_iter()
    .max()
    .unwrap()
        + 1209600;
    Ok((lo, hi))
}
fn intersects(a: (i64, i64), b: (i64, i64)) -> bool {
    a.0 <= b.1 && b.0 <= a.1
}
/// Tighter search bound using only exact declared phase windows. Both fold
/// candidates remain possible until the native derivation verifies resolution.
fn declared_span(
    view: &RuntimeCatalogue,
    profile: &HouseholdProfile,
    week: &Week,
) -> Result<Option<(i64, i64)>, DomainError> {
    let active = mstrchf_core::availability::eligible_slots(week)?;
    let mut lo = i64::MAX;
    let mut hi = i64::MIN;
    for selection in week
        .plan
        .selections
        .iter()
        .filter(|selection| active.contains(&selection.slot))
    {
        let clocks = if let Some(at) = &selection.meal_overrides.eat_by.0 {
            vec![chrono::DateTime::parse_from_rfc3339(&at.instant.0)
                .map_err(|_| error(ErrorCode::InvalidValue, "/calendar/occupancy/clock"))?
                .timestamp()]
        } else {
            let time = profile
                .preferences
                .meal_times
                .iter()
                .find(|time| time.meal_type == selection.slot.meal_type)
                .ok_or_else(|| {
                    error(ErrorCode::MissingReference, "/calendar/occupancy/meal_time")
                })?;
            local_candidates(&selection.slot.date, &time.time, &week.range.zone)?
        };
        let packing_clock = if let Some(journey) = &selection.portable_cold {
            let recipe = view.resolve_recipe(&selection.recipe)?;
            let evidence = view.food_evidence(std::slice::from_ref(&selection.recipe))?;
            let method = mstrchf_core::portable::method(recipe, &evidence)?
                .ok_or_else(|| error(ErrorCode::MissingReference, "/portable_cold/method"))?;
            let at = chrono::DateTime::parse_from_rfc3339(&journey.packing_complete_at.instant.0)
                .map_err(|_| error(ErrorCode::InvalidValue, "/calendar/occupancy/clock"))?
                .timestamp();
            Some((method.packing_prep_id, at))
        } else {
            None
        };
        for (reference, is_base) in std::iter::once((&selection.recipe, true))
            .chain(selection.extras.iter().map(|extra| (&extra.recipe, false)))
        {
            let recipe = view.resolve_recipe(reference)?;
            for prep in recipe.prep.iter() {
                // This is only the occupancy search bound. Native derivation
                // still proves the saved declaration, cooling and journey.
                let phase_clocks = match packing_clock.as_ref() {
                    Some((packing_id, at))
                        if is_base && prep.prep_id == *packing_id =>
                    {
                        std::slice::from_ref(at)
                    }
                    _ => clocks.as_slice(),
                };
                for at in phase_clocks {
                    lo = lo.min(at + i64::from(prep.earliest_offset_seconds));
                    hi = hi.max(
                        at + i64::from(prep.latest_offset_seconds)
                            + i64::from(prep.elapsed_minutes) * 60,
                    );
                }
            }
        }
    }
    Ok((lo != i64::MAX).then_some((lo, hi)))
}
fn seconds(at: &Instant) -> Result<i64, DomainError> {
    chrono::DateTime::parse_from_rfc3339(&at.0)
        .map(|at| at.timestamp())
        .map_err(|_| error(ErrorCode::InvalidValue, "/calendar/occupancy/instant"))
}
fn occupied_owner(
    basis: &SavedFoodBasis,
    derived: &DerivedPlan,
    horizon: (i64, i64),
) -> Result<Option<OccupiedOwner>, DomainError> {
    let mut sessions = derived
        .prep_sessions
        .iter()
        .filter_map(|session| {
            match (
                seconds(&session.start.instant),
                seconds(&session.end.instant),
            ) {
                (Ok(lo), Ok(hi)) if intersects((lo, hi), horizon) => Some(Ok(session.clone())),
                (Ok(_), Ok(_)) => None,
                (Err(error), _) | (_, Err(error)) => Some(Err(error)),
            }
        })
        .collect::<Result<Vec<_>, _>>()?;
    if sessions.is_empty() {
        return Ok(None);
    }
    sessions.sort_by(|a, b| a.session_id.cmp(&b.session_id));
    let mut resources = vec![];
    for session in &sessions {
        for task in session.tasks.iter() {
            let phase = basis
                .evidence
                .resources
                .iter()
                .find(|resource| resource.recipe == task.recipe && resource.prep_id == task.prep_id)
                .ok_or_else(|| {
                    error(ErrorCode::MissingReference, "/calendar/occupancy/resources")
                })?;
            resources.push(OccupiedTaskResources {
                task: PrepTaskRef {
                    session_id: session.session_id.clone(),
                    task_id: task.task_id,
                },
                equipment: phase.equipment.clone(),
                evidence_ids: phase.evidence_ids.clone(),
            });
        }
    }
    resources.sort_by(|a, b| task_cmp(&a.task, &b.task));
    let mut owner = OccupiedOwner {
        pin: owner_pin(basis, derived)?,
        sessions,
        task_resources: resources,
        projection_digest: PayloadDigest("0".repeat(64)),
    };
    owner.projection_digest = occupied_owner_digest(&owner)?;
    Ok(Some(owner))
}
fn full_occupancy(
    state: &BrowserState,
    view: &RuntimeCatalogue,
    historical: &HistoricalApproval,
    at: &Instant,
) -> Result<OccupancySnapshot, DomainError> {
    let first = historical
        .derived
        .prep_sessions
        .iter()
        .map(|session| &session.start)
        .min_by_key(|at| &at.instant)
        .ok_or_else(|| error(ErrorCode::MissingReference, "/calendar/preparation"))?;
    let last = historical
        .derived
        .prep_sessions
        .iter()
        .map(|session| &session.end)
        .max_by_key(|at| &at.instant)
        .unwrap();
    let horizon = (seconds(&first.instant)?, seconds(&last.instant)?);
    let profile = state
        .snapshot
        .profile
        .0
        .as_ref()
        .ok_or_else(|| error(ErrorCode::MissingField, "/profile"))?;
    let mut weeks = BTreeMap::new();
    for week in state
        .retained_weeks
        .iter()
        .chain(state.snapshot.weeks.iter())
    {
        if let Some(previous) = weeks.insert(week.range.week_id.clone(), week) {
            require(
                previous == week,
                ErrorCode::IdentityMismatch,
                "/calendar/occupancy/contradictory_week",
            )?;
        }
    }
    let mut owners = vec![];
    for week in weeks
        .values()
        .copied()
        .filter(|week| week.range.week_id != historical.derived.week_id)
    {
        if !intersects(coarse_span(week)?, horizon) {
            continue;
        }
        let material = if let Some(order) = &week.confirmed_order.0 {
            let (_, original) = booking::historical(state, order).map_err(|_| {
                error(
                    ErrorCode::ReconciliationRequired,
                    "/calendar/occupancy/booked_owner",
                )
            })?;
            let old = &original.basis.context.weeks[0];
            if week.plan != original.basis.plan
                || week.day_enabled != old.day_enabled
                || week.meal_type_enabled != old.meal_type_enabled
                || week.cells != old.cells
                || week.defaults_snapshot != old.defaults_snapshot
            {
                let old_spans = original
                    .derived
                    .prep_sessions
                    .iter()
                    .map(|session| {
                        Ok((
                            seconds(&session.start.instant)?,
                            seconds(&session.end.instant)?,
                        ))
                    })
                    .collect::<Result<Vec<_>, DomainError>>()?;
                let old_overlap = old_spans.into_iter().any(|span| intersects(span, horizon));
                // An unresolved changed owner inside the conservative query
                // horizon is unavailable custody, never evidence of no work.
                let new_overlap = declared_span(view, profile, week)
                    .map_err(|_| {
                        error(
                            ErrorCode::ReconciliationRequired,
                            "/calendar/occupancy/unknown_owner",
                        )
                    })?
                    .is_some_and(|span| intersects(span, horizon));
                require(
                    !old_overlap && !new_overlap,
                    ErrorCode::ReconciliationRequired,
                    "/calendar/occupancy/edited_booked_owner",
                )?;
                continue;
            }
            Some((original.basis, original.derived))
        } else {
            match declared_span(view, profile, week) {
                Ok(None) => continue,
                Ok(Some(span)) if !intersects(span, horizon) => continue,
                Err(_) => {
                    return Err(error(
                        ErrorCode::ReconciliationRequired,
                        "/calendar/occupancy/unknown_owner",
                    ))
                }
                _ => {}
            }
            let (basis, food) = material_at(view, &state.snapshot, profile, week, at)
                .map_err(|_| {
                    error(
                        ErrorCode::ReconciliationRequired,
                        "/calendar/occupancy/unknown_owner",
                    )
                })?
                .ok_or_else(|| {
                    error(
                        ErrorCode::ReconciliationRequired,
                        "/calendar/occupancy/unknown_owner",
                    )
                })?;
            Some((basis, food.plan))
        };
        if let Some((basis, derived)) = material {
            if let Some(owner) = occupied_owner(&basis, &derived, horizon)? {
                owners.push(owner);
            }
        }
        require(
            owners.len() <= 64,
            ErrorCode::LimitExceeded,
            "/calendar/occupancy/owners",
        )?;
    }
    owners.sort_by(|a, b| owner_cmp(&a.pin, &b.pin));
    let mut occupied = OccupancySnapshot {
        query: Some(OccupancyQuery {
            horizon: TimeInterval {
                start: first.clone(),
                end: last.clone(),
            },
            excluded_subject: Some(owner_pin(&historical.basis, &historical.derived)?),
        }),
        occupancy_epoch: state.snapshot.snapshot_revision,
        owners,
        digest: PayloadDigest("0".repeat(64)),
    };
    occupied.digest = occupancy_snapshot_digest(&occupied)?;
    Ok(occupied)
}

fn prior_update(
    state: &BrowserState,
    prepared: &PreparedCalendar,
    order: &ConfirmedOrder,
    destination: &CalendarDestinationId,
) -> Result<CalendarUpdateProjection, DomainError> {
    // Receipt audit establishes these copies; this function never recursively
    // calls audit/run_json and cannot admit a journal supplied by a model.
    let previous = state
        .receipts
        .iter()
        .rev()
        .filter_map(|receipt| receipt.prepared_calendar.as_ref())
        .find(|journal| {
            journal.batch.order_id == order.order_id && journal.batch.destination_id == *destination
        });
    let mut scope = vec![];
    let mut membership = vec![];
    let mut used = BTreeSet::new();
    if let Some(journal) = previous {
        require(
            journal.batch.week_id == order.confirmation.week_id
                && journal.batch.approved_plan_digest == order.approved_plan.derived_digest
                && journal.batch.events.0
                    == journal
                        .saved_events
                        .iter()
                        .map(|row| row.event.clone())
                        .collect::<Vec<_>>(),
            ErrorCode::IdentityMismatch,
            "/calendar/retained_journal",
        )?;
    }
    for draft in &prepared.drafts {
        let old = previous.and_then(|journal| {
            journal
                .saved_events
                .iter()
                .find(|row| row.event.logical_key == draft.logical_key)
        });
        if previous.is_some() && old.is_none() {
            return Err(error(
                ErrorCode::ReconciliationRequired,
                "/calendar/retained_key",
            ));
        }
        if let Some(old) = old {
            used.insert(old.event.event_id.clone());
            for member in old.contributors.iter() {
                membership.push(ContributorTransition {
                    member: SessionIdentity {
                        week_id: member.owner.week.week_id.clone(),
                        session_id: member.session_id.clone(),
                    },
                    before: Some(old.event.logical_key.clone()),
                    after: Some(draft.logical_key.clone()),
                });
            }
        } else {
            for member in &draft.contributors {
                membership.push(ContributorTransition {
                    member: SessionIdentity {
                        week_id: member.owner.week.week_id.clone(),
                        session_id: member.session_id.clone(),
                    },
                    before: None,
                    after: Some(draft.logical_key.clone()),
                });
            }
        }
        scope.push(ScopedEvent {
            key: draft.logical_key.clone(),
            prior: old
                .map(|row| PriorEventState::Retained(row.native()))
                .unwrap_or(PriorEventState::NeverVersioned),
            disposition: UpdateDisposition::Upsert,
        });
    }
    if let Some(journal) = previous {
        require(
            journal
                .saved_events
                .iter()
                .all(|row| used.contains(&row.event.event_id)),
            ErrorCode::ReconciliationRequired,
            "/calendar/retained_membership",
        )?;
    }
    Ok(CalendarUpdateProjection { scope, membership })
}

/// Reconstruct all native authority from owner receipt/state custody. The
/// returned journal and ICS string are review/download data, never publication.
pub fn prepare(
    state: &BrowserState,
    order_id: &OrderId,
    destination_id: &CalendarDestinationId,
    at: &Instant,
) -> Result<(CalendarJournal, String), DomainError> {
    let week = state.snapshot.weeks.iter().find(|week|week.confirmed_order.0.as_ref().is_some_and(|order| &order.order_id==order_id))
        .ok_or_else(||error(ErrorCode::UnsupportedCapability,"/calendar/active_order"))?;
    let order=week.confirmed_order.0.as_ref().unwrap();
    let delivery = booking::effective_delivery(order, &state.receipts.0)?;
    let renderer = renderer_for_plan(&week.plan,delivery.correction_count>0);
    prepare_for_renderer(state, order_id, destination_id, at, renderer)
}
// Historical receipt replay uses its retained renderer. New preparation can
// revise presentation without rewriting previously saved event identities.
pub fn prepare_for_renderer(
    state: &BrowserState,
    order_id: &OrderId,
    destination_id: &CalendarDestinationId,
    at: &Instant,
    renderer_version: u8,
) -> Result<(CalendarJournal, String), DomainError> {
    require(matches!(renderer_version, 1..=14), ErrorCode::UnsupportedVersion, "/calendar/renderer_version")?;
    order_id
        .validate_shape()
        .map_err(|_| error(ErrorCode::InvalidValue, "/order_id"))?;
    destination_id
        .validate_shape()
        .map_err(|_| error(ErrorCode::InvalidValue, "/destination_id"))?;
    at.validate_shape()
        .map_err(|_| error(ErrorCode::InvalidValue, "/calendar/clock"))?;
    let week = state
        .snapshot
        .weeks
        .iter()
        .find(|week| {
            week.confirmed_order
                .0
                .as_ref()
                .is_some_and(|order| order.order_id == *order_id)
        })
        .ok_or_else(|| error(ErrorCode::UnsupportedCapability, "/calendar/active_order"))?;
    let order = week.confirmed_order.0.as_ref().unwrap();
    let (effective_order, delivery) = booking::effective_order(order, &state.receipts.0)?;
    let corrected_renderer=matches!(renderer_version,CORRECTED_DELIVERY_RENDERER|CORRECTED_ORIGINAL_COMPONENT_RENDERER|CORRECTED_PORTABLE_COLD_RENDERER|CORRECTED_GF_CHICKEN_RENDERER|CORRECTED_CHILLI_REMIX_RENDERER|CORRECTED_OVERNIGHT_OATS_RENDERER);
    require((delivery.correction_count>0)==corrected_renderer,
        ErrorCode::ContextStale,"/calendar/delivery_correction_renderer")?;
    let overnight_renderer=matches!(renderer_version,OVERNIGHT_OATS_RENDERER|CORRECTED_OVERNIGHT_OATS_RENDERER);
    require(overnight_renderer==contains_overnight_plan(&week.plan),ErrorCode::UnsupportedCapability,"/calendar/overnight_renderer")?;
    let chilli_renderer=matches!(renderer_version,CHILLI_REMIX_RENDERER|CORRECTED_CHILLI_REMIX_RENDERER)
        || overnight_renderer && contains_chilli_plan(&week.plan);
    require(chilli_renderer==contains_chilli_plan(&week.plan),ErrorCode::UnsupportedCapability,"/calendar/chilli_renderer")?;
    let gf_renderer=matches!(renderer_version,GF_CHICKEN_RENDERER|CORRECTED_GF_CHICKEN_RENDERER)
        || (chilli_renderer || overnight_renderer) && contains_gf_chicken_plan(&week.plan);
    require(gf_renderer==contains_gf_chicken_plan(&week.plan),ErrorCode::UnsupportedCapability,"/calendar/gf_renderer")?;
    let portable_renderer=matches!(renderer_version,PORTABLE_COLD_RENDERER|CORRECTED_PORTABLE_COLD_RENDERER)
        || (gf_renderer || chilli_renderer || overnight_renderer) && contains_portable_plan(&week.plan);
    require(portable_renderer==contains_portable_plan(&week.plan),
        ErrorCode::UnsupportedCapability,"/calendar/portable_renderer")?;
    require(overnight_renderer || chilli_renderer || gf_renderer || portable_renderer || (renderer_version>=ORIGINAL_COMPONENT_RENDERER)==contains_courgette_plan(&week.plan),
        ErrorCode::UnsupportedCapability,"/calendar/original_component_renderer")?;
    require(delivery.last_corrected_at.as_ref().is_none_or(|corrected|corrected<=at),ErrorCode::InvalidValue,"/calendar/correction_clock")?;
    let (original_catalogue, historical) = booking::historical(state, order)?;
    let current_catalogue = RuntimeCatalogue::from_records(&state.reviewed_packets.0)?;
    let profile = state
        .snapshot
        .profile
        .0
        .as_ref()
        .ok_or_else(|| error(ErrorCode::MissingField, "/profile"))?;
    let occupied = full_occupancy(state, &current_catalogue, &historical, at)?;
    let mut policy = LiveFoodPolicy {
        version: Revision(1),
        digest: PayloadDigest("0".repeat(64)),
        general_seconds: 172800,
        rice_seconds: 86400,
        maximum_reheats: 1,
    };
    policy.digest = live_food_policy_digest(&policy)?;
    let mut prepared = prepare_personal_calendar_with_effective_delivery(&PersonalCalendarInputs {
        original_catalogue: &original_catalogue,
        current_catalogue: &current_catalogue,
        basis: &historical.basis,
        derived: &historical.derived,
        approved: &historical.approved,
        order: &historical.order,
        snapshot: &state.snapshot,
        profile,
        week,
        occupied: &occupied,
        essentials: &state.essentials.0,
        policy: &policy,
        evaluated_at: at,
    }, renderer_version, corrected_renderer.then_some(&effective_order))
    .map_err(calendar_error)?;
    if renderer_version >= 2 {
        for draft in &mut prepared.drafts {
            current_presentation(draft, state.engineering_demo)?;
        }
    }
    let update = prior_update(state, &prepared, order, destination_id)?;
    let export = export_personal_calendar(&prepared, &update, &profile.household_id, at)
        .map_err(calendar_error)?;
    let batch = EventBatch {
        week_id: order.confirmation.week_id.clone(),
        order_id: order.order_id.clone(),
        approved_plan_digest: order.approved_plan.derived_digest.clone(),
        destination_id: destination_id.clone(),
        events: BoundedVec(
            export
                .snapshot
                .events
                .iter()
                .map(|row| row.event.clone())
                .collect(),
        ),
    };
    let journal = CalendarJournal {
        renderer_version,
        batch,
        prepared_at: at.clone(),
        saved_events: BoundedVec(
            export
                .snapshot
                .events
                .iter()
                .map(CalendarJournalEvent::from_native)
                .collect(),
        ),
    };
    parse_document::<EventBatch>(&canonical_bytes(&journal.batch)?)?;
    parse_document::<CalendarJournal>(&canonical_bytes(&journal)?)?;
    let ics = String::from_utf8(export.ics)
        .map_err(|_| error(ErrorCode::InvalidValue, "/calendar/ics"))?;
    Ok((journal, ics))
}

#[cfg(test)]
mod tests {
    use super::*;
    // This frozen engineering fixture tests owner projection mechanics only;
    // it cannot qualify engineering food for a personal calendar or an order.
    fn material() -> (BrowserState, RuntimeCatalogue, SavedFoodBasis, DerivedPlan) {
        let state: BrowserState = serde_json::from_value(
            serde_json::from_slice::<serde_json::Value>(include_bytes!(
                "../tests/fixtures/legacy-engineering-receipts.json"
            ))
            .unwrap()["state"]
                .clone(),
        )
        .unwrap();
        let view = RuntimeCatalogue::legacy().unwrap();
        let (basis, food) = material_at(
            &view,
            &state.snapshot,
            state.snapshot.profile.0.as_ref().unwrap(),
            &state.snapshot.weeks[0],
            &Instant("2026-10-12T20:00:00Z".into()),
        )
        .unwrap()
        .unwrap();
        (state, view, basis, food.plan)
    }
    #[test]
    fn occupied_projection_preserves_exact_owner_sessions_and_task_resource_evidence() {
        let (_, _, basis, derived) = material();
        let horizon = (i64::MIN, i64::MAX);
        let projected = occupied_owner(&basis, &derived, horizon).unwrap().unwrap();
        assert_eq!(projected.pin, owner_pin(&basis, &derived).unwrap());
        assert_eq!(
            projected.projection_digest,
            occupied_owner_digest(&projected).unwrap()
        );
        assert_eq!(projected.sessions.len(), derived.prep_sessions.len());
        for session in &projected.sessions {
            for task in session.tasks.iter() {
                let source = basis
                    .evidence
                    .resources
                    .iter()
                    .find(|source| source.recipe == task.recipe && source.prep_id == task.prep_id)
                    .unwrap();
                let resources = projected
                    .task_resources
                    .iter()
                    .find(|resources| {
                        resources.task
                            == PrepTaskRef {
                                session_id: session.session_id.clone(),
                                task_id: task.task_id,
                            }
                    })
                    .unwrap();
                assert_eq!(resources.equipment, source.equipment);
                assert_eq!(resources.evidence_ids, source.evidence_ids);
            }
        }
        assert!(occupied_owner(&basis, &derived, (0, 0)).unwrap().is_none());
        let mut missing = basis.clone();
        missing.evidence.resources.clear();
        assert_eq!(
            occupied_owner(&missing, &derived, horizon)
                .unwrap_err()
                .code,
            ErrorCode::MissingReference
        );
    }
    #[test]
    fn declared_query_bound_refuses_unknown_active_recipe_and_does_not_invent_gap_resolution() {
        let (state, view, _, _) = material();
        let profile = state.snapshot.profile.0.as_ref().unwrap();
        let week = &state.snapshot.weeks[0];
        assert!(declared_span(&view, profile, week).unwrap().is_some());
        let mut unknown = week.clone();
        unknown.plan.selections[0].recipe.recipe_id =
            RecipeId("rc_ffffffffffffffffffffffffffffffff".into());
        assert!(declared_span(&view, profile, &unknown).is_err());
        assert_eq!(
            local_candidates(
                &LocalDate("2026-03-29".into()),
                &LocalTime("01:30:00".into()),
                &ZoneId("Europe/London".into())
            )
            .unwrap_err()
            .code,
            ErrorCode::ReconciliationRequired
        );
        let fold = local_candidates(
            &LocalDate("2026-10-25".into()),
            &LocalTime("01:30:00".into()),
            &ZoneId("Europe/London".into()),
        )
        .unwrap();
        assert_eq!(fold.len(), 2);
        assert_eq!((fold[1] - fold[0]).abs(), 3600);
    }
}
