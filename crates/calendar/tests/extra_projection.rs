//! Genuine admitted-premise consumer regression; no new dietary policy.
use mstrchf_calendar::*;
use mstrchf_contracts::*;
use mstrchf_core::food::*;
#[path = "../../../tests/fixtures/calendars/extra_projection.rs"]
mod extra_projection;
#[path = "../../../tests/fixtures/calendars/native.rs"]
mod native;
mod support;

#[test]
fn asymmetric_extra_passes_native_history_and_calendar_with_unchanged_base() {
    let f = extra_projection::asymmetric_extra();
    let h = &f.historical;
    let checked = verify_historical(h).unwrap();
    let report = assess_continuation(&checked, &f.current).unwrap();
    assert!(report.blocking.is_empty());
    let meal = h
        .basis
        .context
        .requirements
        .iter()
        .find(|m| m.slot.meal_type == MealType::Dinner)
        .unwrap();
    assert_eq!(meal.shared_hard_requirements.len(), 1);
    assert!(meal.attendees[1].hard_requirements.is_empty());
    let order = h.order.clone();
    let p = support::prepare(
        h,
        &f.current,
        &support::assemblies(h),
        &support::production_text(h),
    )
    .unwrap();
    assert_eq!(p.drafts.len(), 3);
    assert_eq!(p.reports, vec![report]);
    let dinner = p
        .drafts
        .iter()
        .find(|d| d.start.local_time.0 == "18:00:00")
        .unwrap();
    for text in [
        native::normal_case::BASE_PREP,
        native::normal_case::BASE_GRILL,
        native::normal_case::TOMATO_EXTRA,
    ] {
        assert!(dinner.body.0.contains(text));
    }
    for amount in [
        "2 each (Whole)",
        "60 g (Prepared)",
        "100 g (Raw)",
        "Diner 2: 1 servings",
    ] {
        assert!(dinner.body.0.contains(amount));
    }
    assert_eq!(h.order, order);
}

#[test]
fn actual_extra_recipient_and_base_restrictions_still_block_complete_output() {
    for f in [
        extra_projection::actual_recipient_restricted(),
        extra_projection::base_requirement_unsupported(),
    ] {
        let h = &f.historical;
        let report = assess_continuation(&verify_historical(h).unwrap(), &f.current).unwrap();
        assert!(!report.blocking.is_empty());
        assert_eq!(
            support::prepare(
                h,
                &f.current,
                &support::assemblies(h),
                &support::production_text(h)
            ),
            Err(CalendarFailure::Blocked(vec![report]))
        );
    }
}

#[test]
fn extra_spec_restriction_cannot_be_forged_into_an_approved_history() {
    let s = extra_projection::extra_spec_unsupported();
    assert!(matches!(
        required_identity_keys(&s.plan, &s.context, &s.evidence, &s.schedule),
        Err(FoodFailure::Blocked { .. })
    ));
}

#[test]
fn complete_native_extra_pipeline_preserves_content_and_retry_bytes() {
    let f = extra_projection::asymmetric_extra();
    let h = &f.historical;
    let p = support::prepare(
        h,
        &f.current,
        &support::assemblies(h),
        &support::production_text(h),
    )
    .unwrap();
    let mut update = CalendarUpdateProjection {
        scope: p
            .drafts
            .iter()
            .map(|d| ScopedEvent {
                key: d.logical_key.clone(),
                prior: PriorEventState::NeverVersioned,
                disposition: UpdateDisposition::Upsert,
            })
            .collect(),
        membership: p
            .drafts
            .iter()
            .flat_map(|d| {
                d.contributors.iter().map(|c| ContributorTransition {
                    member: SessionIdentity {
                        week_id: c.owner.week.week_id.clone(),
                        session_id: c.session_id.clone(),
                    },
                    before: None,
                    after: Some(d.logical_key.clone()),
                })
            })
            .collect(),
    };
    let fresh = p
        .drafts
        .iter()
        .enumerate()
        .map(|(i, d)| CalendarIdentity {
            key: d.logical_key.clone(),
            event_id: CalendarEventId(format!("ce_{:032x}", i + 1)),
        })
        .collect::<Vec<_>>();
    let s = version_events(&p, &update, &fresh, &p.evaluated_at).unwrap();
    assert_eq!(s.events.len(), 3);
    for d in &p.drafts {
        let e = &s
            .events
            .iter()
            .find(|r| r.event.logical_key == d.logical_key)
            .unwrap()
            .event;
        assert_eq!(e.body, d.body);
        assert_eq!(e.title, d.title);
        assert_eq!(e.start, d.start);
        assert_eq!(e.end, d.end);
    }
    let bytes = render_ics(&s).unwrap();
    for scope in &mut update.scope {
        scope.prior = PriorEventState::Retained(
            s.events
                .iter()
                .find(|r| r.event.logical_key == scope.key)
                .unwrap()
                .clone(),
        );
    }
    for transition in &mut update.membership {
        transition.before = transition.after.clone();
    }
    let retry = version_events(&p, &update, &[], &Instant("2026-10-11T13:00:00Z".into())).unwrap();
    assert_eq!(s, retry);
    assert_eq!(bytes, render_ics(&retry).unwrap());
    println!(
        "ICS_EVIDENCE={}",
        serde_json::json!({"fixture":"native-asymmetric-extra","events":s.events.iter().map(|r|serde_json::json!({"event":r.event,"sequence":r.sequence,"revised_at":r.revised_at})).collect::<Vec<_>>(),"ics":String::from_utf8(bytes).unwrap()})
    );
    let blocked = extra_projection::actual_recipient_restricted();
    let report = assess_continuation(
        &verify_historical(&blocked.historical).unwrap(),
        &blocked.current,
    )
    .unwrap();
    assert!(!report.blocking.is_empty());
    let mut stopped = s.clone();
    stopped.reports = vec![report.clone()];
    assert_eq!(
        render_ics(&stopped),
        Err(CalendarFailure::Blocked(vec![report]))
    );
    stopped.reports[0].digest = PayloadDigest("0".repeat(64));
    assert!(matches!(
        render_ics(&stopped),
        Err(CalendarFailure::Invalid(DomainError {
            code: ErrorCode::DigestMismatch,
            ..
        }))
    ));
    stopped.reports = vec![s.reports[0].clone(); 66];
    assert!(matches!(
        render_ics(&stopped),
        Err(CalendarFailure::Invalid(DomainError {
            code: ErrorCode::LimitExceeded,
            ..
        }))
    ));
}
