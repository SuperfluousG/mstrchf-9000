//! Synthetic version-stage records exercise custody consistency, not food admission.
use mstrchf_calendar::*;
use mstrchf_contracts::*;
use mstrchf_core::food::PlanOwnerPin;

fn at(hour: &str) -> Instant {
    Instant(format!("2026-10-12T{hour}:00Z"))
}
fn key(n: u64, kind: CalendarEventKind) -> CalendarEventLogicalKey {
    CalendarEventLogicalKey {
        week_id: WeekId(format!("wk_{n:032x}")),
        kind,
        session_id: Nullable(
            (kind != CalendarEventKind::Delivery).then(|| PrepSessionId(format!("ps_{n:032x}"))),
        ),
        order_id: Nullable(
            (kind == CalendarEventKind::Delivery).then(|| OrderId(format!("or_{n:032x}"))),
        ),
    }
}
fn member(n: u64) -> SessionContribution {
    SessionContribution {
        owner: PlanOwnerPin {
            week: WeekRevision {
                week_id: WeekId(format!("wk_{n:032x}")),
                revision: Revision(1),
            },
            derived_digest: PayloadDigest("a".repeat(64)),
            basis_digest: PayloadDigest("b".repeat(64)),
        },
        session_id: PrepSessionId(format!("ps_{n:032x}")),
    }
}
fn identity(m: &SessionContribution) -> SessionIdentity {
    SessionIdentity {
        week_id: m.owner.week.week_id.clone(),
        session_id: m.session_id.clone(),
    }
}
fn draft(n: u64, kind: CalendarEventKind) -> EventDraft {
    let moment = |local: &str, utc: &str| {
        serde_json::from_value(serde_json::json!({"local_date":"2026-10-12","local_time":local,"zone":"Europe/London","resolution":"unambiguous","instant":format!("2026-10-12T{utc}Z")})).unwrap()
    };
    EventDraft {
        logical_key: key(n, kind),
        contributors: if kind == CalendarEventKind::Delivery {
            vec![]
        } else {
            vec![member(n)]
        },
        start: moment("18:00:00", "17:00:00"),
        end: moment("18:10:00", "17:10:00"),
        title: Title("Synthetic cooking".into()),
        body: LongText("Cook the exact admitted portion.\nServe immediately.".into()),
    }
}
fn prepared(drafts: Vec<EventDraft>) -> PreparedCalendar {
    PreparedCalendar {
        subject_order_id: OrderId(format!("or_{:032x}", 1)),
        evaluated_at: at("12:00"),
        reports: vec![],
        drafts,
    }
}
fn event(d: &EventDraft, n: u64, revision: u64) -> CalendarEvent {
    let mut e = CalendarEvent {
        event_id: CalendarEventId(format!("ce_{n:032x}")),
        logical_key: d.logical_key.clone(),
        payload_revision: Revision(revision),
        start: d.start.clone(),
        end: d.end.clone(),
        title: d.title.clone(),
        body: d.body.clone(),
        event_payload_digest: PayloadDigest("0".repeat(64)),
    };
    digest(&mut e);
    e
}
fn digest(e: &mut CalendarEvent) {
    let mut value = serde_json::to_value(&*e).unwrap();
    value
        .as_object_mut()
        .unwrap()
        .remove("event_payload_digest");
    value["schema_version"] = 1.into();
    e.event_payload_digest = PayloadDigest(canonical_digest(&value).unwrap().0);
}
fn saved(d: &EventDraft, n: u64) -> SavedCalendarEvent {
    SavedCalendarEvent {
        event: event(d, n, 1),
        contributors: d.contributors.clone(),
        sequence: 0,
        revised_at: at("11:00"),
    }
}
fn projection(p: &PreparedCalendar, old: &[SavedCalendarEvent]) -> CalendarUpdateProjection {
    let scope = p
        .drafts
        .iter()
        .map(|d| ScopedEvent {
            key: d.logical_key.clone(),
            disposition: UpdateDisposition::Upsert,
            prior: old
                .iter()
                .find(|o| o.event.logical_key == d.logical_key)
                .cloned()
                .map(PriorEventState::Retained)
                .unwrap_or(PriorEventState::NeverVersioned),
        })
        .collect();
    let mut members = old
        .iter()
        .flat_map(|o| o.contributors.iter())
        .chain(p.drafts.iter().flat_map(|d| d.contributors.iter()))
        .map(identity)
        .collect::<Vec<_>>();
    members.sort_by(|a, b| (&a.week_id.0, &a.session_id.0).cmp(&(&b.week_id.0, &b.session_id.0)));
    members.dedup();
    let membership = members
        .into_iter()
        .map(|m| ContributorTransition {
            before: old
                .iter()
                .find(|o| o.contributors.iter().any(|c| identity(c) == m))
                .map(|o| o.event.logical_key.clone()),
            after: p
                .drafts
                .iter()
                .find(|d| d.contributors.iter().any(|c| identity(c) == m))
                .map(|d| d.logical_key.clone()),
            member: m,
        })
        .collect();
    CalendarUpdateProjection { scope, membership }
}
fn fresh(p: &PreparedCalendar) -> Vec<CalendarIdentity> {
    p.drafts
        .iter()
        .enumerate()
        .map(|(n, d)| CalendarIdentity {
            key: d.logical_key.clone(),
            event_id: CalendarEventId(format!("ce_{:032x}", n + 1)),
        })
        .collect()
}
fn invalid(r: Result<CalendarSnapshot, CalendarFailure>, code: ErrorCode) {
    match r {
        Err(CalendarFailure::Invalid(e)) => assert_eq!(e.code, code),
        other => panic!("expected {code:?}: {other:?}"),
    }
}
fn reconcile(r: Result<CalendarSnapshot, CalendarFailure>) {
    match r {
        Err(CalendarFailure::ReconciliationRequired { keys, .. }) => assert!(!keys.is_empty()),
        other => panic!("expected named reconciliation: {other:?}"),
    }
}

#[test]
fn c19_first_version_and_repeat_preserve_exact_identity_and_timestamp() {
    let p = prepared(vec![draft(1, CalendarEventKind::Cooking)]);
    let u = projection(&p, &[]);
    let first = version_events(&p, &u, &fresh(&p), &at("12:00")).unwrap();
    assert_eq!(first.events[0].event, event(&p.drafts[0], 1, 1));
    assert_eq!(first.events[0].sequence, 0);
    assert_eq!(first.events[0].revised_at, at("12:00"));
    let mut later = p.clone();
    later.evaluated_at = at("13:00");
    let again = version_events(
        &later,
        &projection(&later, &first.events),
        &[],
        &at("14:00"),
    )
    .unwrap();
    assert_eq!(again.events, first.events);
    assert_eq!(again.evaluated_at, at("13:00"));
}
#[test]
fn c20_changed_content_keeps_id_and_increments_both_versions() {
    for field in ["title", "body", "start", "end"] {
        let d = draft(1, CalendarEventKind::Cooking);
        let old = saved(&d, 77);
        let mut p = prepared(vec![d]);
        match field {
            "title" => p.drafts[0].title = Title("Changed recipe title".into()),
            "body" => p.drafts[0].body = LongText("Use 90 g of admitted cheese.".into()),
            "start" => {
                p.drafts[0].start.local_time.0 = "18:01:00".into();
                p.drafts[0].start.instant.0 = "2026-10-12T17:01:00Z".into();
            }
            _ => {
                p.drafts[0].end.local_time.0 = "18:11:00".into();
                p.drafts[0].end.instant.0 = "2026-10-12T17:11:00Z".into();
            }
        }
        let out = version_events(
            &p,
            &projection(&p, std::slice::from_ref(&old)),
            &[],
            &at("12:00"),
        )
        .unwrap();
        assert_eq!(out.events[0].event, event(&p.drafts[0], 77, 2));
        assert_eq!(out.events[0].sequence, 1);
        assert_eq!(out.events[0].revised_at, at("12:00"));
        assert_eq!(old.event.payload_revision, Revision(1));
    }
}
#[test]
fn c21_provenance_renewal_updates_evidence_without_payload_churn() {
    let d = draft(1, CalendarEventKind::Cooking);
    let old = saved(&d, 77);
    let mut p = prepared(vec![d]);
    p.drafts[0].contributors[0].owner.week.revision = Revision(2);
    p.drafts[0].contributors[0].owner.derived_digest = PayloadDigest("c".repeat(64));
    let out = version_events(
        &p,
        &projection(&p, std::slice::from_ref(&old)),
        &[],
        &at("12:00"),
    )
    .unwrap();
    assert_eq!(out.events[0].event, old.event);
    assert_eq!(out.events[0].sequence, 0);
    assert_eq!(out.events[0].revised_at, old.revised_at);
    assert_eq!(out.events[0].contributors, p.drafts[0].contributors);
}
#[test]
fn c22_fresh_ids_are_an_exact_bijection_without_replacements() {
    let p = prepared(vec![
        draft(1, CalendarEventKind::Cooking),
        draft(2, CalendarEventKind::Delivery),
    ]);
    let u = projection(&p, &[]);
    let mut f = fresh(&p);
    invalid(
        version_events(&p, &u, &f[..1], &at("12:00")),
        ErrorCode::MissingReference,
    );
    f[1].event_id = f[0].event_id.clone();
    invalid(
        version_events(&p, &u, &f, &at("12:00")),
        ErrorCode::InvalidValue,
    );
    let mut f = fresh(&p);
    f.push(CalendarIdentity {
        key: key(3, CalendarEventKind::Cooking),
        event_id: CalendarEventId(format!("ce_{:032x}", 3)),
    });
    invalid(
        version_events(&p, &u, &f, &at("12:00")),
        ErrorCode::InvalidValue,
    );
    let old = saved(&p.drafts[0], 77);
    invalid(
        version_events(&p, &projection(&p, &[old]), &fresh(&p), &at("12:00")),
        ErrorCode::IdentityMismatch,
    );
}
#[test]
fn c25_maxima_are_preserved_unchanged_and_never_wrap_when_changed() {
    for (revision, sequence) in [(u64::MAX, 0), (1, i32::MAX as u32)] {
        let d = draft(1, CalendarEventKind::Cooking);
        let mut old = saved(&d, 1);
        old.event.payload_revision = Revision(revision);
        digest(&mut old.event);
        old.sequence = sequence;
        let mut p = prepared(vec![d]);
        assert_eq!(
            version_events(
                &p,
                &projection(&p, std::slice::from_ref(&old)),
                &[],
                &at("12:00")
            )
            .unwrap()
            .events[0],
            old
        );
        p.drafts[0].body = LongText("Changed complete instruction.".into());
        let r = version_events(&p, &projection(&p, &[old]), &[], &at("12:00"));
        assert!(
            matches!(r,Err(CalendarFailure::Invalid(e)) if matches!(e.code,ErrorCode::ArithmeticOverflow|ErrorCode::LimitExceeded))
        );
    }
}
#[test]
fn c26_revision_time_and_retained_payload_are_checked() {
    let d = draft(1, CalendarEventKind::Cooking);
    let old = saved(&d, 1);
    let mut p = prepared(vec![d]);
    p.drafts[0].body = LongText("Changed complete instruction.".into());
    invalid(
        version_events(
            &p,
            &projection(&p, std::slice::from_ref(&old)),
            &[],
            &at("11:59"),
        ),
        ErrorCode::InvalidValue,
    );
    let mut future = old.clone();
    future.revised_at = at("13:00");
    invalid(
        version_events(&p, &projection(&p, &[future]), &[], &at("12:00")),
        ErrorCode::InvalidValue,
    );
    let mut same_second = old.clone();
    same_second.revised_at = at("12:00");
    let same = version_events(&p, &projection(&p, &[same_second]), &[], &at("12:00")).unwrap();
    assert_eq!(same.events[0].sequence, 1);
    assert_eq!(same.events[0].revised_at, at("12:00"));
    let mut broken = old.clone();
    broken.event.body = LongText("Tampered retained body.".into());
    invalid(
        version_events(&p, &projection(&p, &[broken]), &[], &at("12:00")),
        ErrorCode::DigestMismatch,
    );
    let mut broken = old;
    broken.event.payload_revision = Revision(0);
    digest(&mut broken.event);
    assert!(matches!(
        version_events(&p, &projection(&p, &[broken]), &[], &at("12:00")),
        Err(CalendarFailure::Invalid(_))
    ));
}

#[test]
fn r2_s01_scoped_shared_update_retains_siblings_and_leaves_unrelated_ledger_rows() {
    let mut root = draft(1, CalendarEventKind::Cooking);
    root.contributors.push(member(2));
    let old = saved(&root, 10);
    let outside = vec![
        saved(&draft(1, CalendarEventKind::Delivery), 11),
        saved(&draft(4, CalendarEventKind::Cooking), 12),
    ];
    let before_outside = outside.clone();
    root.contributors.push(member(3));
    let p = prepared(vec![root, draft(3, CalendarEventKind::Delivery)]);
    let u = projection(&p, std::slice::from_ref(&old));
    let f = vec![CalendarIdentity {
        key: p.drafts[1].logical_key.clone(),
        event_id: CalendarEventId(format!("ce_{:032x}", 13)),
    }];
    let out = version_events(&p, &u, &f, &at("12:00")).unwrap();
    assert_eq!(out.events.len(), 2);
    let shared = out
        .events
        .iter()
        .find(|e| e.event.logical_key == old.event.logical_key)
        .unwrap();
    assert_eq!(shared.event.event_id, old.event.event_id);
    assert_eq!(shared.event.payload_revision, Revision(2));
    assert_eq!(shared.sequence, 1);
    assert_eq!(shared.contributors.len(), 3);
    assert_eq!(outside, before_outside);
    let evidence_event = |saved: &SavedCalendarEvent| {
        serde_json::json!({
            "event":saved.event,"sequence":saved.sequence,"revised_at":saved.revised_at,
            "contributors":saved.contributors.iter().map(|c|serde_json::json!({"week_id":c.owner.week.week_id,"session_id":c.session_id,"owner_revision":c.owner.week.revision,"derived_digest":c.owner.derived_digest,"basis_digest":c.owner.basis_digest})).collect::<Vec<_>>()
        })
    };
    println!(
        "VERSION_EVIDENCE={}",
        serde_json::json!({"case":"R2-S01 synthetic version-stage scope","retained_root":evidence_event(&old),"out_of_scope_retained":outside.iter().map(evidence_event).collect::<Vec<_>>(),"snapshot_upserts":out.events.iter().map(evidence_event).collect::<Vec<_>>()})
    );
    for untouched in outside {
        assert!(
            !out.events
                .iter()
                .any(|e| e.event.event_id == untouched.event.event_id)
        );
    }
}
#[test]
fn r2_s02_s03_unavailable_or_absent_scope_never_initialises_an_existing_key() {
    let p = prepared(vec![draft(1, CalendarEventKind::Cooking)]);
    let mut u = projection(&p, &[]);
    u.scope[0].prior = PriorEventState::Unavailable;
    reconcile(version_events(&p, &u, &fresh(&p), &at("12:00")));
    u.scope.clear();
    reconcile(version_events(&p, &u, &fresh(&p), &at("12:00")));
}
#[test]
fn r2_s04_s11_explicit_session_delivery_and_whole_owner_removals_return_no_snapshot() {
    for kinds in [
        vec![CalendarEventKind::Cooking],
        vec![CalendarEventKind::Delivery],
        vec![CalendarEventKind::Cooking, CalendarEventKind::Delivery],
    ] {
        let old = kinds
            .into_iter()
            .map(|k| {
                saved(
                    &draft(1, k),
                    if k == CalendarEventKind::Delivery {
                        2
                    } else {
                        1
                    },
                )
            })
            .collect::<Vec<_>>();
        let p = prepared(vec![]);
        let mut u = projection(&p, &old);
        u.scope = old
            .iter()
            .map(|o| ScopedEvent {
                key: o.event.logical_key.clone(),
                prior: PriorEventState::Retained(o.clone()),
                disposition: UpdateDisposition::Remove,
            })
            .collect();
        let before = u.clone();
        reconcile(version_events(&p, &u, &[], &at("12:00")));
        assert_eq!(u, before);
    }
}
#[test]
fn r2_s05_s06_s07_missing_sibling_is_contradiction_or_explicit_reconciliation() {
    let mut d = draft(1, CalendarEventKind::Cooking);
    d.contributors.push(member(2));
    let old = saved(&d, 1);
    d.contributors.pop();
    let p = prepared(vec![d]);
    let mut u = projection(&p, &[old]);
    reconcile(version_events(&p, &u, &[], &at("12:00")));
    u.membership
        .iter_mut()
        .find(|t| t.member == identity(&member(2)))
        .unwrap()
        .after = Some(p.drafts[0].logical_key.clone());
    invalid(
        version_events(&p, &u, &[], &at("12:00")),
        ErrorCode::IdentityMismatch,
    );
}
#[test]
fn r2_s08_role_change_cannot_generate_a_replacement_beside_retained_event() {
    let old = saved(&draft(1, CalendarEventKind::Cooking), 1);
    let p = prepared(vec![draft(1, CalendarEventKind::Prep)]);
    let mut u = projection(&p, std::slice::from_ref(&old));
    u.scope.push(ScopedEvent {
        key: old.event.logical_key.clone(),
        prior: PriorEventState::Retained(old),
        disposition: UpdateDisposition::Remove,
    });
    reconcile(version_events(&p, &u, &fresh(&p), &at("12:00")));
}
#[test]
fn r2_s09_reparent_and_missing_old_root_require_reconciliation() {
    let mut a = draft(1, CalendarEventKind::Cooking);
    a.contributors.push(member(3));
    let mut b = draft(2, CalendarEventKind::Cooking);
    let old = vec![saved(&a, 1), saved(&b, 2)];
    a.contributors.pop();
    b.contributors.push(member(3));
    let p = prepared(vec![a, b]);
    let mut u = projection(&p, &old);
    reconcile(version_events(&p, &u, &[], &at("12:00")));
    u.scope.remove(0);
    reconcile(version_events(&p, &u, &[], &at("12:00")));
}
#[test]
fn r2_s10_split_and_merge_preserve_old_identity_for_reconciliation() {
    let mut original = draft(1, CalendarEventKind::Cooking);
    original.contributors.push(member(2));
    let old = saved(&original, 1);
    original.contributors.pop();
    let p = prepared(vec![original, draft(2, CalendarEventKind::Cooking)]);
    let u = projection(&p, &[old]);
    let f = vec![CalendarIdentity {
        key: p.drafts[1].logical_key.clone(),
        event_id: CalendarEventId(format!("ce_{:032x}", 2)),
    }];
    reconcile(version_events(&p, &u, &f, &at("12:00")));
    let a = draft(1, CalendarEventKind::Cooking);
    let b = draft(2, CalendarEventKind::Cooking);
    let old = vec![saved(&a, 1), saved(&b, 2)];
    let mut merged = a;
    merged.contributors.push(member(2));
    let p = prepared(vec![merged]);
    let mut u = projection(&p, &old);
    u.scope.push(ScopedEvent {
        key: b.logical_key.clone(),
        prior: PriorEventState::Retained(old[1].clone()),
        disposition: UpdateDisposition::Remove,
    });
    reconcile(version_events(&p, &u, &[], &at("12:00")));
}
#[test]
fn malformed_scope_membership_and_contributors_refuse_atomically() {
    let p = prepared(vec![draft(1, CalendarEventKind::Cooking)]);
    let f = fresh(&p);
    let base = projection(&p, &[]);
    let mut u = base.clone();
    u.scope.push(u.scope[0].clone());
    invalid(
        version_events(&p, &u, &f, &at("12:00")),
        ErrorCode::InvalidValue,
    );
    let mut u = base.clone();
    u.membership.push(u.membership[0].clone());
    invalid(
        version_events(&p, &u, &f, &at("12:00")),
        ErrorCode::InvalidValue,
    );
    let mut u = base.clone();
    u.membership.clear();
    invalid(
        version_events(&p, &u, &f, &at("12:00")),
        ErrorCode::MissingReference,
    );
    let mut p = p;
    p.drafts[0].contributors.clear();
    invalid(
        version_events(&p, &base, &f, &at("12:00")),
        ErrorCode::MissingReference,
    );
}
#[test]
fn count_bounds_precede_nested_validation() {
    let mut p = prepared(vec![draft(1, CalendarEventKind::Cooking)]);
    let u = projection(&p, &[]);
    let f = fresh(&p);
    p.drafts = vec![p.drafts[0].clone(); 257];
    invalid(
        version_events(&p, &u, &f, &at("12:00")),
        ErrorCode::LimitExceeded,
    );
}

#[test]
fn independent_literal_preimages_match_new_and_changed_event_payloads() {
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/calendars/version-vectors.json"
    ))
    .unwrap();
    let expected = vectors
        .as_array()
        .unwrap()
        .iter()
        .map(|v| serde_json::from_value::<CalendarEvent>(v["event"].clone()).unwrap())
        .collect::<Vec<_>>();
    let to_draft = |e: &CalendarEvent| {
        let mut m = member(1);
        m.owner.week.week_id = e.logical_key.week_id.clone();
        m.session_id = e.logical_key.session_id.0.clone().unwrap();
        EventDraft {
            logical_key: e.logical_key.clone(),
            contributors: vec![m],
            start: e.start.clone(),
            end: e.end.clone(),
            title: e.title.clone(),
            body: e.body.clone(),
        }
    };
    let p = prepared(vec![to_draft(&expected[0])]);
    let f = vec![CalendarIdentity {
        key: expected[0].logical_key.clone(),
        event_id: expected[0].event_id.clone(),
    }];
    let first = version_events(&p, &projection(&p, &[]), &f, &at("12:00")).unwrap();
    assert_eq!(first.events[0].event, expected[0]);
    let p = prepared(vec![to_draft(&expected[1])]);
    let second = version_events(&p, &projection(&p, &first.events), &[], &at("13:00")).unwrap();
    assert_eq!(second.events[0].event, expected[1]);
    for (i, event) in [
        first.events[0].event.clone(),
        second.events[0].event.clone(),
    ]
    .into_iter()
    .enumerate()
    {
        let mut material = serde_json::to_value(&event).unwrap();
        material
            .as_object_mut()
            .unwrap()
            .remove("event_payload_digest");
        material["schema_version"] = 1.into();
        assert_eq!(
            canonical_bytes(&material).unwrap(),
            vectors[i]["canonical_preimage"]
                .as_str()
                .unwrap()
                .as_bytes()
        );
        assert_eq!(
            event.event_payload_digest.0,
            vectors[i]["sha256"].as_str().unwrap()
        );
        event.validate_shape().unwrap();
    }
}

#[test]
fn remaining_projection_bounds_precede_malformed_nested_rows() {
    let p = prepared(vec![draft(1, CalendarEventKind::Cooking)]);
    let base = projection(&p, &[]);
    let f = fresh(&p);
    let mut u = base.clone();
    u.scope = vec![u.scope[0].clone(); 257];
    invalid(
        version_events(&p, &u, &f, &at("12:00")),
        ErrorCode::LimitExceeded,
    );
    let mut u = base.clone();
    u.membership = vec![u.membership[0].clone(); 8705];
    invalid(
        version_events(&p, &u, &f, &at("12:00")),
        ErrorCode::LimitExceeded,
    );
    let ids = vec![f[0].clone(); 257];
    invalid(
        version_events(&p, &base, &ids, &at("12:00")),
        ErrorCode::LimitExceeded,
    );
    let mut large = p.clone();
    large.drafts[0].contributors = vec![member(1); 4353];
    invalid(
        version_events(&large, &base, &f, &at("12:00")),
        ErrorCode::LimitExceeded,
    );
    let mut old = saved(&p.drafts[0], 1);
    old.contributors = vec![member(1); 4353];
    let mut u = base.clone();
    u.scope[0].prior = PriorEventState::Retained(old);
    invalid(
        version_events(&p, &u, &[], &at("12:00")),
        ErrorCode::LimitExceeded,
    );
}

#[test]
fn malformed_native_values_and_retained_id_collisions_refuse() {
    let p = prepared(vec![draft(1, CalendarEventKind::Cooking)]);
    let u = projection(&p, &[]);
    let f = fresh(&p);
    invalid(
        version_events(&p, &u, &f, &Instant("bad clock".into())),
        ErrorCode::InvalidValue,
    );
    let mut old = saved(&p.drafts[0], 1);
    old.sequence = u32::MAX;
    invalid(
        version_events(&p, &projection(&p, &[old]), &[], &at("12:00")),
        ErrorCode::LimitExceeded,
    );
    let mut broken = p.clone();
    broken.drafts[0].body = LongText("raw\rcontrol".into());
    assert!(matches!(
        version_events(&broken, &u, &f, &at("12:00")),
        Err(CalendarFailure::Invalid(_))
    ));
    let p = prepared(vec![
        draft(1, CalendarEventKind::Cooking),
        draft(2, CalendarEventKind::Cooking),
    ]);
    let old = vec![saved(&p.drafts[0], 77), saved(&p.drafts[1], 77)];
    invalid(
        version_events(&p, &projection(&p, &old), &[], &at("12:00")),
        ErrorCode::InvalidValue,
    );
    let old = vec![saved(&p.drafts[0], 77)];
    let f = vec![CalendarIdentity {
        key: p.drafts[1].logical_key.clone(),
        event_id: old[0].event.event_id.clone(),
    }];
    invalid(
        version_events(&p, &projection(&p, &old), &f, &at("12:00")),
        ErrorCode::InvalidValue,
    );
}

#[test]
fn argument_order_does_not_change_semantic_event_order_or_versions() {
    let p = prepared(vec![
        draft(2, CalendarEventKind::Cooking),
        draft(1, CalendarEventKind::Cooking),
    ]);
    let u = projection(&p, &[]);
    let f = fresh(&p);
    let expected = version_events(&p, &u, &f, &at("12:00")).unwrap();
    let mut p = p;
    let mut u = u;
    let mut f = f;
    p.drafts.reverse();
    u.scope.reverse();
    u.membership.reverse();
    f.reverse();
    assert_eq!(version_events(&p, &u, &f, &at("12:00")).unwrap(), expected);
    assert_eq!(
        expected.events[0].event.logical_key,
        key(1, CalendarEventKind::Cooking)
    );
}

#[path = "../../../tests/fixtures/calendars/native.rs"]
mod native;
mod support;
#[test]
fn admitted_native_preparation_versions_without_losing_reports_or_bodies() {
    let f = native::normal();
    let h = &f.historical;
    let p = support::prepare(
        h,
        &f.current,
        &support::assemblies(h),
        &support::production_text(h),
    )
    .unwrap();
    let before = p.clone();
    let snapshot = version_events(&p, &projection(&p, &[]), &fresh(&p), &p.evaluated_at).unwrap();
    assert_eq!(snapshot.events.len(), 3);
    assert_eq!(snapshot.reports, p.reports);
    assert_eq!(p, before);
    for event in &snapshot.events {
        let draft = p
            .drafts
            .iter()
            .find(|d| d.logical_key == event.event.logical_key)
            .unwrap();
        assert_eq!(event.event.body, draft.body);
        assert_eq!(event.contributors, draft.contributors);
        event.event.validate_shape().unwrap();
    }
    let mut renewed_current = f.current.clone();
    renewed_current.revision = Revision(2);
    support::repin_current(&mut renewed_current, h);
    let renewed = support::prepare(
        h,
        &renewed_current,
        &support::assemblies(h),
        &support::production_text(h),
    )
    .unwrap();
    assert_eq!(renewed.drafts, p.drafts);
    assert_ne!(renewed.reports, p.reports);
    let again = version_events(
        &renewed,
        &projection(&renewed, &snapshot.events),
        &[],
        &renewed.evaluated_at,
    )
    .unwrap();
    assert_eq!(again.events, snapshot.events);
    assert_eq!(again.reports, renewed.reports);
    let mut blocked = p.clone();
    blocked.reports[0].digest = PayloadDigest("0".repeat(64));
    invalid(
        version_events(
            &blocked,
            &projection(&blocked, &[]),
            &fresh(&blocked),
            &blocked.evaluated_at,
        ),
        ErrorCode::DigestMismatch,
    );
}

#[test]
fn blocked_native_reports_are_retained_completely_without_a_snapshot() {
    use mstrchf_core::food::{assess_continuation, verify_historical};
    let mut f = native::direct_reuse(true);
    f.current.policy.maximum_reheats = 0;
    support::repin_current(&mut f.current, &f.historical);
    let report =
        assess_continuation(&verify_historical(&f.historical).unwrap(), &f.current).unwrap();
    assert!(!report.blocking.is_empty());
    let mut p = prepared(vec![]);
    p.reports = vec![report.clone()];
    assert_eq!(
        version_events(&p, &projection(&p, &[]), &[], &at("12:00")),
        Err(CalendarFailure::Blocked(vec![report]))
    );
    p.reports = vec![p.reports[0].clone(); 66];
    invalid(
        version_events(&p, &projection(&p, &[]), &[], &at("12:00")),
        ErrorCode::LimitExceeded,
    );
}
