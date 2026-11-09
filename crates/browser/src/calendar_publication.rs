//! Local acknowledgement of a complete user-reviewed Google read. Neither the
//! source label nor its digest independently authenticates external calendar IO.
use crate::{BrowserState, calendar_owner, error};
use mstrchf_contracts::*;
use std::collections::BTreeSet;

fn require(ok: bool, code: ErrorCode, path: &str) -> Result<(), DomainError> {
    if ok { Ok(()) } else { Err(error(code, path)) }
}

/// The caller audits native state first. Historical callers reconstruct the
/// exact pre-command prefix; live dispatch additionally checks today's renderer.
/// This validator never changes the order, history or retained preparation.
pub(crate) fn confirm(
    state: &BrowserState,
    confirmation: &CalendarPublicationConfirmation,
    at: &Instant,
) -> Result<WeekId, DomainError> {
    let _: CalendarPublicationConfirmation = parse_document(&canonical_bytes(confirmation)?)?;
    let _: Instant = parse_document(&canonical_bytes(at)?)?;
    let week = state.snapshot.weeks.iter()
        .find(|week| week.confirmed_order.0.as_ref().is_some_and(|order| order.order_id == confirmation.order_id))
        .ok_or_else(|| error(ErrorCode::MissingReference, "/calendar/publication/order"))?;
    require(matches!(week.publication, PublicationState::Prepared | PublicationState::Published),
        ErrorCode::ContextStale, "/calendar/publication/not_prepared")?;
    let (receipt, saved) = state.receipts.iter().rev()
        .filter_map(|receipt| receipt.prepared_calendar.as_ref().map(|journal| (receipt, journal)))
        .find(|(_, journal)| journal.batch.order_id == confirmation.order_id
            && journal.batch.destination_id == confirmation.destination_id)
        .ok_or_else(|| error(ErrorCode::MissingReference, "/calendar/publication/preparation"))?;
    require(receipt.command.command_id == confirmation.preparation_command_id,
        ErrorCode::ContextStale, "/calendar/publication/preparation_command_id")?;
    let AppCommand::RequestCalendarPublication { order_id, destination_id, event_batch_digest } = &receipt.command.command else {
        return Err(error(ErrorCode::IdentityMismatch, "/calendar/publication/preparation_command"));
    };
    require(order_id == &confirmation.order_id && destination_id == &confirmation.destination_id
        && event_batch_digest == &confirmation.event_batch_digest
        && canonical_digest(&saved.batch)? == confirmation.event_batch_digest,
        ErrorCode::ContextStale, "/calendar/publication/event_batch_digest")?;
    require(saved.batch.week_id == week.range.week_id,
        ErrorCode::IdentityMismatch, "/calendar/publication/week_id")?;
    let (current, _) = calendar_owner::prepare_for_renderer(state, &confirmation.order_id,
        &confirmation.destination_id, at, saved.renderer_version)?;
    require(current.batch == saved.batch, ErrorCode::ContextStale, "/calendar/publication/current_batch")?;

    let clock = chrono::DateTime::parse_from_rfc3339(&at.0)
        .map_err(|_| error(ErrorCode::InvalidValue, "/calendar/publication/clock"))?;
    let fresh = |observed: &Instant| -> Result<(), DomainError> {
        require(observed >= &receipt.committed_at && observed >= &saved.prepared_at
            && observed <= at, ErrorCode::ContextStale, "/calendar/publication/observed_at")?;
        let observed_clock = chrono::DateTime::parse_from_rfc3339(&observed.0)
            .map_err(|_| error(ErrorCode::InvalidValue, "/calendar/publication/observed_at"))?;
        require(clock.signed_duration_since(observed_clock).num_seconds() <= 300,
            ErrorCode::ContextStale, "/calendar/publication/observed_at")
    };
    fresh(&confirmation.observed_at)?;
    require(confirmation.events.len() == current.batch.events.len(),
        ErrorCode::ScopeViolation, "/calendar/publication/events")?;
    let mut native_ids = BTreeSet::new();
    let mut provider_ids = BTreeSet::new();
    for observed in confirmation.events.iter() {
        require(native_ids.insert(observed.event_id.clone())
            && provider_ids.insert(observed.provider_event_id.clone()),
            ErrorCode::DuplicateKey, "/calendar/publication/events")?;
        fresh(&observed.observed_at)?;
        require(observed.observed_at <= confirmation.observed_at,
            ErrorCode::InvalidValue, "/calendar/publication/events/observed_at")?;
        let expected = current.batch.events.iter().find(|event| event.event_id == observed.event_id)
            .ok_or_else(|| error(ErrorCode::ForeignReference, "/calendar/publication/events/event_id"))?;
        require(observed.title == expected.title && observed.body == expected.body
            && observed.start == expected.start.instant && observed.end == expected.end.instant
            && observed.zone == expected.start.zone && observed.zone == expected.end.zone
            && observed.start < observed.end,
            ErrorCode::IdentityMismatch, "/calendar/publication/events/content")?;
    }
    Ok(week.range.week_id.clone())
}

#[cfg(test)]
mod tests {
    use super::*;
    const NOW: &str = "2026-10-12T21:30:00Z";

    fn fixture() -> (BrowserState, CalendarPublicationConfirmation) {
        let value: serde_json::Value = serde_json::from_slice(include_bytes!("../tests/fixtures/calendar-v15-renderer2-prepared.json")).unwrap();
        let state: BrowserState = serde_json::from_value(value["state"].clone()).unwrap();
        let receipt = state.receipts.last().unwrap();
        let journal = receipt.prepared_calendar.as_ref().unwrap();
        let confirmation = CalendarPublicationConfirmation {
            preparation_command_id: receipt.command.command_id.clone(),
            order_id: journal.batch.order_id.clone(), destination_id: journal.batch.destination_id.clone(),
            event_batch_digest: canonical_digest(&journal.batch).unwrap(),
            calendar_id: Text("synthetic-calendar@example.invalid".into()),
            source: CalendarPublicationConfirmationSource::UserReviewedGoogleRead {
                read_id: Text("rd_10000000000000000000000000000001".into()), read_digest: Digest("a".repeat(64)),
            },
            observed_at: Instant(NOW.into()),
            events: BoundedVec(journal.batch.events.iter().map(|event| PublishedCalendarEvent {
                event_id: event.event_id.clone(), provider_event_id: Text(format!("synthetic-{}", event.event_id.0)),
                title: event.title.clone(), body: event.body.clone(), start: event.start.instant.clone(),
                end: event.end.instant.clone(), zone: event.start.zone.clone(), observed_at: Instant(NOW.into()),
            }).collect()),
        };
        (state, confirmation)
    }
    fn check(state: &BrowserState, confirmation: &CalendarPublicationConfirmation) -> Result<WeekId, DomainError> {
        confirm(state, confirmation, &Instant(NOW.into()))
    }

    #[test]
    fn exact_complete_observation_preserves_every_input_and_accepts_reordered_membership() {
        let (state, mut confirmation) = fixture(); let original = state.clone();
        assert_eq!(check(&state, &confirmation).unwrap(), state.snapshot.weeks[0].range.week_id);
        confirmation.events.0.reverse(); assert!(check(&state, &confirmation).is_ok());
        assert_eq!(canonical_bytes(&state).unwrap(), canonical_bytes(&original).unwrap());
        let mut published = state.clone();
        let week_id = published.snapshot.weeks[0].range.week_id.clone();
        for week in published.snapshot.weeks.iter_mut().chain(published.retained_weeks.iter_mut()).filter(|week|week.range.week_id==week_id) {
            week.publication = PublicationState::Published;
        }
        assert!(check(&published, &confirmation).is_ok());
    }

    #[test]
    fn full_unique_mapping_and_exact_content_are_required() {
        let (state, confirmation) = fixture();
        for case in 0..10 {
            let mut changed = confirmation.clone();
            match case {
                0 => { changed.events.0.pop(); },
                1 => changed.events.0[1].event_id = changed.events[0].event_id.clone(),
                2 => changed.events.0[1].provider_event_id = changed.events[0].provider_event_id.clone(),
                3 => changed.events.0[0].event_id = CalendarEventId("ce_ffffffffffffffffffffffffffffffff".into()),
                4 => changed.events.0[0].title.0.push_str(" changed"),
                5 => changed.events.0[0].body.0.push_str("\nChanged method."),
                6 => changed.events.0[0].body.0 = changed.events[0].body.0.replace('\n', "\r\n"),
                7 => changed.events.0[0].start = changed.events[0].end.clone(),
                8 => changed.events.0[0].zone = ZoneId("UTC".into()),
                _ => changed.events.0[0].end = changed.events[0].start.clone(),
            }
            assert!(check(&state, &changed).is_err(), "case {case}");
        }
    }

    #[test]
    fn every_observation_is_recent_after_preparation_and_no_later_than_parent_or_clock() {
        let (state, confirmation) = fixture();
        assert!(confirm(&state, &confirmation, &Instant("2026-10-12T21:35:00Z".into())).is_ok());
        assert!(confirm(&state, &confirmation, &Instant("2026-10-12T21:35:01Z".into())).is_err());
        for case in 0..4 {
            let mut bad = confirmation.clone();
            match case {
                0 => bad.observed_at = Instant("2026-10-12T21:30:01Z".into()),
                1 => bad.events.0[0].observed_at = Instant("2026-10-12T21:24:59Z".into()),
                2 => bad.events.0[0].observed_at = Instant("2026-10-12T20:00:00Z".into()),
                _ => bad.observed_at = Instant("2026-10-12T21:29:59Z".into()),
            }
            assert!(check(&state, &bad).is_err(), "case {case}");
        }
    }

    #[test]
    fn stale_preparation_batch_order_destination_and_nonprepared_state_refuse() {
        let (state, confirmation) = fixture();
        for case in 0..4 {
            let mut bad = confirmation.clone();
            match case {
                0 => bad.preparation_command_id = CommandId("cm_ffffffffffffffffffffffffffffffff".into()),
                1 => bad.event_batch_digest = Digest("f".repeat(64)),
                2 => bad.order_id = OrderId("or_ffffffffffffffffffffffffffffffff".into()),
                _ => bad.destination_id = CalendarDestinationId("cd_ffffffffffffffffffffffffffffffff".into()),
            }
            assert!(check(&state, &bad).is_err(), "case {case}");
        }
        for publication in [PublicationState::NotRequested, PublicationState::NeedsUser, PublicationState::Failed] {
            let mut bad = state.clone(); bad.snapshot.weeks[0].publication = publication;
            assert!(check(&bad, &confirmation).is_err());
        }
        let mut newer = state.clone(); let mut receipt = newer.receipts.last().unwrap().clone();
        receipt.command.command_id = CommandId("cm_ffffffffffffffffffffffffffffffff".into());
        newer.receipts.0.push(receipt); assert!(check(&newer, &confirmation).is_err());
    }

    #[test]
    fn delivery_invalidation_preserves_needs_user_and_other_existing_states() {
        for before in [PublicationState::Prepared, PublicationState::Published] {
            assert_eq!(crate::booking::corrected_publication(before), PublicationState::NotRequested);
        }
        for before in [PublicationState::NotRequested, PublicationState::NeedsUser, PublicationState::Failed, PublicationState::ExportedManualImportRequired, PublicationState::Publishing] {
            assert_eq!(crate::booking::corrected_publication(before), before);
        }
    }
}
