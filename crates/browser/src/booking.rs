//! Local order custody. A user confirmation records their external checkout;
//! basket readiness and calendar export never manufacture an order.
use crate::*;
use mstrchf_core::food::{HistoricalApproval, verify_historical};

pub(crate) fn resolve_moment(date: &LocalDate, time: &LocalTime, zone: &ZoneId, resolution: Resolution) -> Result<ZonedMoment, DomainError> {
    use chrono::TimeZone;
    let local = chrono::NaiveDateTime::parse_from_str(&format!("{}T{}", date.0, time.0), "%Y-%m-%dT%H:%M:%S").map_err(records::storage)?;
    let tz: chrono_tz::Tz = zone.0.parse().map_err(records::storage)?;
    let actual = match tz.from_local_datetime(&local) {
        chrono::LocalResult::Single(value) if resolution == Resolution::Unambiguous => value,
        chrono::LocalResult::Ambiguous(first, second) if resolution == Resolution::Earlier => first.min(second),
        chrono::LocalResult::Ambiguous(first, second) if resolution == Resolution::Later => first.max(second),
        _ => return Err(error(ErrorCode::InvalidValue, "/delivery/local_time")),
    };
    ZonedMoment::new(date.clone(), time.clone(), zone.clone(), resolution, Instant(actual.with_timezone(&chrono::Utc).to_rfc3339_opts(chrono::SecondsFormat::Secs, true)))
}

pub(crate) fn approve(
    view: &catalogue::RuntimeCatalogue,
    snapshot: &Snapshot,
    week_id: &WeekId,
    plan_revision: Revision,
    digest: &PayloadDigest,
    at: &Instant,
) -> Result<ApprovedPlan, DomainError> {
    let profile = snapshot.profile.0.as_ref().ok_or_else(|| error(ErrorCode::MissingField, "/profile"))?;
    let week = snapshot.weeks.iter().find(|w| &w.range.week_id == week_id).ok_or_else(|| error(ErrorCode::ForeignReference, "/week_id"))?;
    if week.confirmed_order.0.is_some() {
        return Err(error(ErrorCode::UnsupportedCapability, "/order/amendment"));
    }
    if week.plan.revision != plan_revision {
        return Err(error(ErrorCode::ContextStale, "/plan_revision"));
    }
    let prior_clock = CLOCK.with(|clock| clock.replace(Some(at.clone())));
    let result = planner_view::derive(view, snapshot, profile, week);
    CLOCK.with(|clock| *clock.borrow_mut() = prior_clock);
    let food = result?.ok_or_else(|| error(ErrorCode::MissingField, "/plan/selections"))?;
    if &food.plan.digest != digest {
        return Err(error(ErrorCode::ContextStale, "/derived_digest"));
    }
    // Frozen engineering fixtures cannot become production order content.
    if food.plan.recipe_refs.iter().any(|reference| !view.is_personal_recipe(reference)) {
        return Err(error(ErrorCode::UnsupportedCapability, "/order/engineering_recipe"));
    }
    Ok(ApprovedPlan {
        approval_id: ApprovalId(planner_view::stable_id("au", &json!([snapshot.household_draft.household_id, week_id, plan_revision, digest, snapshot.snapshot_revision, at]))?),
        week_id: week_id.clone(), plan_revision, derived_digest: food.plan.digest,
        context_stamp: food.plan.context_stamp, approved_at: at.clone(),
    })
}

pub(crate) fn confirm(snapshot: &Snapshot, given: &OrderConfirmation, at: &Instant) -> Result<ConfirmedOrder, DomainError> {
    if !matches!(given.source, OrderConfirmationSource::UserConfirmation {}) {
        return Err(error(ErrorCode::UnsupportedCapability, "/order/source"));
    }
    let week = snapshot.weeks.iter().find(|w| w.range.week_id == given.week_id).ok_or_else(|| error(ErrorCode::ForeignReference, "/week_id"))?;
    if week.confirmed_order.0.is_some() {
        return Err(error(ErrorCode::UnsupportedCapability, "/order/amendment"));
    }
    let approved = week.approved_plan.0.as_ref().ok_or_else(|| error(ErrorCode::MissingReference, "/approval_id"))?;
    if approved.approval_id != given.approval_id || approved.plan_revision != week.plan.revision {
        return Err(error(ErrorCode::ContextStale, "/approval_id"));
    }
    if at < &approved.approved_at || given.confirmed_at > *at {
        return Err(error(ErrorCode::InvalidValue, "/confirmed_at"));
    }
    let mut confirmation = given.clone(); confirmation.confirmed_at = at.clone();
    let active = mstrchf_core::availability::eligible_slots(week)?;
    let mut refs: Vec<_> = week.plan.selections.iter().filter(|s| active.contains(&s.slot)).flat_map(|s| std::iter::once(s.recipe.clone()).chain(s.extras.iter().map(|e| e.recipe.clone()))).collect();
    refs.sort_by(mstrchf_core::food::recipe_cmp); refs.dedup();
    let order = ConfirmedOrder {
        order_id: OrderId(planner_view::stable_id("or", &json!([snapshot.household_draft.household_id, confirmation, approved]))?),
        confirmation, approved_plan: approved.clone(), ordered_recipe_refs: BoundedVec(refs),
    };
    parse_document(&canonical_bytes(&order)?)
}

/// A read projection of the append-only command journal. The original order,
/// approval, confirmation clock and ordered food never change.
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(crate) struct EffectiveDelivery {
    pub order_id: OrderId,
    pub delivery_start: ZonedMoment,
    pub delivery_end: ZonedMoment,
    pub correction_count: u32,
    pub last_corrected_at: Option<Instant>,
}
fn validate_delivery(start: &ZonedMoment, end: &ZonedMoment) -> Result<(), DomainError> {
    for moment in [start, end] {
        let resolved = resolve_moment(&moment.local_date, &moment.local_time, &moment.zone, moment.resolution)?;
        if &resolved != moment { return Err(error(ErrorCode::IdentityMismatch, "/order/delivery_moment")); }
    }
    if start.instant >= end.instant { return Err(error(ErrorCode::InvalidValue, "/delivery_end")); }
    Ok(())
}
pub(crate) fn effective_delivery(order: &ConfirmedOrder, receipts: &[ReceiptRecord]) -> Result<EffectiveDelivery, DomainError> {
    let mut result = EffectiveDelivery { order_id: order.order_id.clone(),
        delivery_start: order.confirmation.delivery_start.clone(), delivery_end: order.confirmation.delivery_end.clone(),
        correction_count: 0, last_corrected_at: None };
    for receipt in receipts {
        if let AppCommand::CorrectOrderDelivery { order_id, delivery_start, delivery_end } = &receipt.command.command {
            if order_id != &order.order_id { continue; }
            validate_delivery(delivery_start, delivery_end)?;
            if receipt.committed_at < order.confirmation.confirmed_at
                || result.last_corrected_at.as_ref().is_some_and(|prior| &receipt.committed_at < prior) {
                return Err(error(ErrorCode::InvalidValue, "/order/corrected_at"));
            }
            if result.delivery_start == *delivery_start && result.delivery_end == *delivery_end {
                return Err(error(ErrorCode::DomainConflict, "/order/delivery_unchanged"));
            }
            result.delivery_start = delivery_start.clone(); result.delivery_end = delivery_end.clone();
            result.correction_count = result.correction_count.checked_add(1).ok_or_else(|| error(ErrorCode::LimitExceeded, "/order/delivery_corrections"))?;
            result.last_corrected_at = Some(receipt.committed_at.clone());
        }
    }
    Ok(result)
}
pub(crate) fn correction_week(snapshot: &Snapshot, receipts: &[ReceiptRecord], order_id: &OrderId,
    delivery_start: &ZonedMoment, delivery_end: &ZonedMoment, at: &Instant) -> Result<WeekId, DomainError> {
    let week = snapshot.weeks.iter().find(|week| week.confirmed_order.0.as_ref().is_some_and(|order| &order.order_id == order_id))
        .ok_or_else(|| error(ErrorCode::MissingReference, "/order_id"))?;
    let order = week.confirmed_order.0.as_ref().unwrap();
    if delivery_start.zone != week.range.zone || delivery_end.zone != week.range.zone {
        return Err(error(ErrorCode::IdentityMismatch, "/order/delivery_zone"));
    }
    validate_delivery(delivery_start, delivery_end)?;
    let current = effective_delivery(order, receipts)?;
    if at < &order.confirmation.confirmed_at || current.last_corrected_at.as_ref().is_some_and(|prior| at < prior) {
        return Err(error(ErrorCode::InvalidValue, "/order/corrected_at"));
    }
    if &current.delivery_start == delivery_start && &current.delivery_end == delivery_end {
        return Err(error(ErrorCode::DomainConflict, "/order/delivery_unchanged"));
    }
    Ok(week.range.week_id.clone())
}
pub(crate) fn corrected_publication(publication: PublicationState) -> PublicationState {
    match publication { PublicationState::Prepared | PublicationState::Published => PublicationState::NotRequested, other => other }
}
/// Calendar-only projection after the owner has audited the entire receipt
/// prefix. It is never persisted as a replacement confirmation or order.
pub(crate) fn effective_order(order: &ConfirmedOrder, receipts: &[ReceiptRecord]) -> Result<(ConfirmedOrder, EffectiveDelivery), DomainError> {
    let delivery = effective_delivery(order, receipts)?;
    let mut projected = order.clone(); projected.confirmation.delivery_start = delivery.delivery_start.clone();
    projected.confirmation.delivery_end = delivery.delivery_end.clone();
    Ok((projected, delivery))
}

/// Only command-specific metadata transitions may create approval/order state.
pub(crate) fn check_transition(view: &catalogue::RuntimeCatalogue, before: &Snapshot, recorded: &std::collections::BTreeMap<WeekId, Week>, after: &Snapshot, command: &AppCommand, at: &Instant) -> Result<(), DomainError> {
    for next in after.weeks.iter() {
        let old = before.weeks.iter().find(|w| w.range.week_id == next.range.week_id).or_else(|| recorded.get(&next.range.week_id));
        let mut approval = old.and_then(|w| w.approved_plan.0.clone());
        let mut order = old.and_then(|w| w.confirmed_order.0.clone());
        let mut publication = old.map_or(PublicationState::NotRequested, |w| w.publication);
        match command {
            AppCommand::ApprovePlan { week_id, plan_revision, derived_digest } if week_id == &next.range.week_id => {
                approval = Some(approve(view, before, week_id, *plan_revision, derived_digest, at)?);
            }
            AppCommand::ConfirmOrder { confirmation } if confirmation.week_id == next.range.week_id => {
                order = Some(confirm(before, confirmation, at)?);
            }
            AppCommand::RequestCalendarPublication { order_id, .. } if order.as_ref().is_some_and(|saved| &saved.order_id == order_id) => {
                publication = PublicationState::Prepared;
            }
            AppCommand::ConfirmCalendarPublication { confirmation } if order.as_ref().is_some_and(|saved| saved.order_id == confirmation.order_id) => {
                publication = PublicationState::Published;
            }
            AppCommand::CorrectOrderDelivery { order_id, .. } if order.as_ref().is_some_and(|saved| &saved.order_id == order_id) => {
                publication = corrected_publication(publication);
            }
            AppCommand::UpdateProfile { .. } if publication == PublicationState::Published => {
                publication = PublicationState::NeedsUser;
            }
            _ => {
                if order.is_none() && !matches!(command, AppCommand::ApprovePlan { .. } | AppCommand::ConfirmOrder { .. } | AppCommand::CorrectOrderDelivery { .. } | AppCommand::ConfirmCalendarPublication { .. } | AppCommand::SuggestPlanningHorizon { .. }) { approval = None; }
                if old.is_some_and(|w| w.plan != next.plan) {
                    approval = None;
                    if order.is_some() { publication = PublicationState::NeedsUser; }
                }
            }
        }
        if next.approved_plan.0 != approval || next.confirmed_order.0 != order || next.publication != publication {
            return Err(error(ErrorCode::IdentityMismatch, "/receipts/order_transition"));
        }
    }
    Ok(())
}

pub(crate) fn catalogue_for_pin(pin: &catalogue::CataloguePin) -> Result<catalogue::RuntimeCatalogue, DomainError> {
    let mut records = Vec::new();
    for reference in maker_packets::available_packet_refs()? {
        if pin.entries.iter().any(|entry| entry.recipe == reference.recipe && entry.packet_digest == reference.packet_digest) {
            records.push(maker_packets::known_packet(&reference)?.record().clone());
        }
    }
    let view = catalogue::RuntimeCatalogue::from_records(&records)?; view.verify_pin(pin)?; Ok(view)
}

/// Recover the exact pre-approval food inputs from their original receipt/time.
/// Current household/catalogue changes never rewrite the ordered food basis.
pub(crate) fn historical(state: &BrowserState, order: &ConfirmedOrder) -> Result<(catalogue::RuntimeCatalogue, HistoricalApproval), DomainError> {
    let index = state.receipts.iter().position(|r| matches!(&r.command.command, AppCommand::ApprovePlan { week_id, .. } if week_id == &order.confirmation.week_id) && r.response.weeks.iter().any(|w| w.approved_plan.0.as_ref() == Some(&order.approved_plan)))
        .ok_or_else(|| error(ErrorCode::MissingReference, "/order/approval_receipt"))?;
    let receipt = &state.receipts[index];
    let before = if index == 0 { initial(state.snapshot.household_draft.household_id.clone()) } else { state.receipts[index - 1].response.clone() };
    let view = match &receipt.food_catalogue_pin { Some(pin) => catalogue_for_pin(pin)?, None => catalogue::RuntimeCatalogue::legacy()? };
    let profile = before.profile.0.as_ref().ok_or_else(|| error(ErrorCode::MissingField, "/order/original_profile"))?;
    let week = before.weeks.iter().find(|w| w.range.week_id == order.confirmation.week_id).ok_or_else(|| error(ErrorCode::MissingReference, "/order/original_week"))?;
    let prior_clock = CLOCK.with(|clock| clock.replace(Some(receipt.committed_at.clone())));
    let material = planner_view::derive_material(&view, &before, profile, week);
    CLOCK.with(|clock| *clock.borrow_mut() = prior_clock);
    let (basis, food) = material?.ok_or_else(|| error(ErrorCode::MissingReference, "/order/original_food"))?;
    let historical = HistoricalApproval { basis, derived: food.plan, approved: order.approved_plan.clone(), order: order.clone() };
    verify_historical(&historical)?;
    Ok((view, historical))
}
