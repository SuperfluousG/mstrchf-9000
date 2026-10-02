//! Fixed-profile renderer literals; these records do not claim food admission.
use mstrchf_calendar::*;
use mstrchf_contracts::*;
use mstrchf_core::food::PlanOwnerPin;

fn repin(e: &mut CalendarEvent) {
    let mut v = serde_json::to_value(&*e).unwrap();
    v.as_object_mut().unwrap().remove("event_payload_digest");
    v["schema_version"] = 1.into();
    e.event_payload_digest = PayloadDigest(canonical_digest(&v).unwrap().0);
}
fn members(e: &CalendarEvent) -> Vec<SessionContribution> {
    e.logical_key
        .session_id
        .0
        .as_ref()
        .map(|id| {
            vec![SessionContribution {
                owner: PlanOwnerPin {
                    week: WeekRevision {
                        week_id: e.logical_key.week_id.clone(),
                        revision: Revision(1),
                    },
                    derived_digest: PayloadDigest("a".repeat(64)),
                    basis_digest: PayloadDigest("b".repeat(64)),
                },
                session_id: id.clone(),
            }]
        })
        .unwrap_or_default()
}
fn normal() -> CalendarSnapshot {
    let v: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/calendars/ics/normal.expected.events.json"
    ))
    .unwrap();
    let events = v["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|v| {
            let event: CalendarEvent = serde_json::from_value(v["event"].clone()).unwrap();
            SavedCalendarEvent {
                contributors: members(&event),
                event,
                sequence: v["sequence"].as_u64().unwrap() as u32,
                revised_at: serde_json::from_value(v["revised_at"].clone()).unwrap(),
            }
        })
        .collect();
    CalendarSnapshot {
        subject_order_id: OrderId(format!("or_{:032x}", 1)),
        evaluated_at: Instant("2026-10-11T12:05:00Z".into()),
        reports: vec![],
        events,
    }
}
fn single(body: &str) -> CalendarSnapshot {
    let mut s = normal();
    s.events = vec![s.events[1].clone()];
    s.events[0].event.body = LongText(body.into());
    repin(&mut s.events[0].event);
    s
}
fn text(s: &CalendarSnapshot) -> String {
    String::from_utf8(render_ics(s).unwrap()).unwrap()
}
fn invalid(s: &CalendarSnapshot, code: ErrorCode) {
    match render_ics(s) {
        Err(CalendarFailure::Invalid(e)) => assert_eq!(e.code, code),
        other => panic!("expected {code:?}: {other:?}"),
    }
}
fn physical_description(s: &str) -> String {
    let mut lines = s
        .split("\r\n")
        .skip_while(|l| !l.starts_with("DESCRIPTION:"));
    let first = lines.next().unwrap();
    let mut out = format!("{first}\r\n");
    for l in lines {
        if !l.starts_with(' ') {
            break;
        }
        out.push_str(l);
        out.push_str("\r\n");
    }
    out
}
fn moment(date: &str, local: &str, utc: &str, resolution: &str) -> ZonedMoment {
    ZonedMoment {
        local_date: LocalDate(date.into()),
        local_time: LocalTime(local.into()),
        zone: ZoneId("Europe/London".into()),
        resolution: serde_json::from_value(serde_json::json!(resolution)).unwrap(),
        instant: Instant(utc.into()),
    }
}

#[test]
fn c27_complete_normal_bytes_equal_independent_frozen_oracle() {
    let s = normal();
    let before = s.clone();
    let bytes = render_ics(&s).unwrap();
    assert_eq!(
        bytes,
        include_bytes!("../../../tests/fixtures/calendars/ics/normal.expected.ics")
    );
    assert_eq!(s, before);
    println!(
        "ICS_EVIDENCE={}",
        serde_json::json!({"fixture":"literal-normal","events":s.events.iter().map(|r|serde_json::json!({"event":r.event,"sequence":r.sequence,"revised_at":r.revised_at})).collect::<Vec<_>>(),"ics":String::from_utf8(bytes).unwrap()})
    );
    let mut shuffled = s.clone();
    shuffled.events.reverse();
    assert_eq!(render_ics(&s).unwrap(), render_ics(&shuffled).unwrap());
}
#[test]
fn c28_c31_c37_scalar_folding_matches_each_literal_boundary() {
    let controls: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/calendars/ics/controls.json"
    ))
    .unwrap();
    for row in controls["folding"].as_array().unwrap() {
        let body = row["input"]
            .as_str()
            .unwrap()
            .strip_prefix("DESCRIPTION:")
            .unwrap();
        let actual = physical_description(&text(&single(body)));
        assert_eq!(actual, row["expected_physical"].as_str().unwrap());
        let lengths = actual
            .split("\r\n")
            .filter(|l| !l.is_empty())
            .map(str::len)
            .collect::<Vec<_>>();
        assert_eq!(serde_json::json!(lengths), row["expected_lengths"]);
    }
    assert_eq!(
        physical_description(&text(&single("short"))),
        "DESCRIPTION:short\r\n"
    );
}
#[test]
fn c29_text_escaping_is_exact_once_and_preserves_colons_tabs_and_unicode() {
    let controls: serde_json::Value = serde_json::from_str(include_str!(
        "../../../tests/fixtures/calendars/ics/controls.json"
    ))
    .unwrap();
    let row = &controls["escaping"];
    assert_eq!(
        physical_description(&text(&single(row["input"].as_str().unwrap()))),
        format!("DESCRIPTION:{}\r\n", row["expected"].as_str().unwrap())
    );
    assert_eq!(
        physical_description(&text(&single("A\tB: \"café\""))),
        "DESCRIPTION:A\tB: \"café\"\r\n"
    );
    assert_ne!(
        render_ics(&single("café")).unwrap(),
        render_ics(&single("cafe\u{301}")).unwrap()
    );
}
#[test]
fn c30_injection_is_text_and_raw_controls_are_rejected() {
    let body = "First\nEND:VEVENT\nBEGIN:VEVENT\nLast";
    let output = text(&single(body));
    assert_eq!(output.matches("\r\nBEGIN:VEVENT\r\n").count(), 1);
    assert!(output.contains("DESCRIPTION:First\\nEND:VEVENT\\nBEGIN:VEVENT\\nLast\r\n"));
    for bad in ["First\rLast", "First\0Last", "First\u{7f}Last"] {
        assert!(matches!(
            render_ics(&single(bad)),
            Err(CalendarFailure::Invalid(_))
        ));
    }
    let mut s = single("Complete body.");
    s.events[0].event.title = Title("Bad\nTitle".into());
    repin(&mut s.events[0].event);
    assert!(render_ics(&s).is_err());
}
#[test]
fn c31_profile_has_only_fixed_properties_crlf_and_no_bom() {
    let bytes = render_ics(&normal()).unwrap();
    assert!(!bytes.starts_with(&[0xef, 0xbb, 0xbf]));
    assert!(bytes.ends_with(b"END:VCALENDAR\r\n"));
    let content = std::str::from_utf8(&bytes).unwrap();
    for line in content.split("\r\n") {
        assert!(!line.contains(['\r', '\n']));
        assert!(line.len() <= 75);
    }
    for forbidden in [
        "METHOD:",
        "VALARM",
        "ATTENDEE:",
        "ORGANIZER:",
        "STATUS:",
        "TZID=",
        "LOCATION:",
        "RRULE:",
        "X-",
    ] {
        assert!(!content.contains(forbidden));
    }
    assert_eq!(content.matches("\r\nUID:").count(), 3);
    assert_eq!(content.matches("\r\nDTSTAMP:").count(), 3);
}
#[test]
fn c32_folds_preserve_selected_instants_and_ninety_minute_span() {
    for (resolution, utc, expected) in [
        ("earlier", "2026-10-25T00:30:00Z", "20261025T003000Z"),
        ("later", "2026-10-25T01:30:00Z", "20261025T013000Z"),
    ] {
        let mut s = single("Synthetic fold.");
        let e = &mut s.events[0].event;
        e.start = moment("2026-10-25", "01:30:00", utc, resolution);
        e.end = e.start.clone();
        repin(e);
        assert!(text(&s).contains(&format!("DTSTART:{expected}\r\n")));
    }
    let mut s = single("Synthetic span.");
    let e = &mut s.events[0].event;
    e.start = moment("2026-10-25", "01:15:00", "2026-10-25T00:15:00Z", "earlier");
    e.end = moment("2026-10-25", "01:45:00", "2026-10-25T01:45:00Z", "later");
    repin(e);
    let out = text(&s);
    assert!(out.contains("DTSTART:20261025T001500Z\r\nDTEND:20261025T014500Z\r\n"));
}
#[test]
fn c33_c34_gaps_and_inconsistent_native_moments_are_refused() {
    for resolution in ["earlier", "later", "unambiguous"] {
        let mut s = single("Synthetic gap.");
        let e = &mut s.events[0].event;
        e.start = moment("2026-03-29", "01:30:00", "2026-03-29T01:30:00Z", resolution);
        e.end = e.start.clone();
        repin(e);
        assert!(render_ics(&s).is_err());
    }
    for (local, utc, expected) in [
        ("00:30:00", "2026-03-29T00:30:00Z", "20260329T003000Z"),
        ("02:30:00", "2026-03-29T01:30:00Z", "20260329T013000Z"),
    ] {
        let mut s = single("Gap neighbour.");
        let e = &mut s.events[0].event;
        e.start = moment("2026-03-29", local, utc, "unambiguous");
        e.end = e.start.clone();
        repin(e);
        assert!(text(&s).contains(&format!("DTSTART:{expected}\r\n")));
    }
    for (date, local, utc, res) in [
        (
            "2026-10-25",
            "01:30:00",
            "2026-10-25T00:30:00Z",
            "unambiguous",
        ),
        ("2026-10-12", "18:00:00", "2026-10-12T17:00:00Z", "earlier"),
        (
            "2026-10-12",
            "18:00:00",
            "2026-10-12T18:00:00Z",
            "unambiguous",
        ),
    ] {
        let mut s = single("Inconsistent fields.");
        let e = &mut s.events[0].event;
        e.start = moment(date, local, utc, res);
        e.end = e.start.clone();
        repin(e);
        assert!(render_ics(&s).is_err());
    }
}
#[test]
fn c35_c36_zero_duration_omits_end_without_inventing_time() {
    let mut s = single("Complete zero-duration step.");
    s.events[0].event.end = s.events[0].event.start.clone();
    repin(&mut s.events[0].event);
    assert!(!text(&s).contains("DTEND:"));
    s.events[0].event.end.zone = ZoneId("UTC".into());
    s.events[0].event.end.local_time = LocalTime("17:00:00".into());
    repin(&mut s.events[0].event);
    assert!(!text(&s).contains("DTEND:"));
    s.events[0].event.end.local_time = LocalTime("16:59:59".into());
    s.events[0].event.end.instant = Instant("2026-10-12T16:59:59Z".into());
    repin(&mut s.events[0].event);
    assert!(render_ics(&s).is_err());
    let mut delivery = normal();
    delivery.events.truncate(1);
    delivery.events[0].event.end = delivery.events[0].event.start.clone();
    repin(&mut delivery.events[0].event);
    invalid(&delivery, ErrorCode::InvalidValue);
}
#[test]
fn c37_utf8_body_boundary_and_one_more_scalar() {
    let s = single(&"🍲".repeat(16384));
    assert_eq!(s.events[0].event.body.0.len(), 65536);
    assert!(render_ics(&s).is_ok());
    assert!(render_ics(&single(&"🍲".repeat(16385))).is_err());
}
fn many(n: usize, body: &str) -> CalendarSnapshot {
    let mut s = single(body);
    let original = s.events[0].clone();
    s.events = (0..n)
        .map(|i| {
            let mut row = original.clone();
            row.event.event_id = CalendarEventId(format!("ce_{:032x}", i + 1));
            row.event.logical_key.session_id =
                Nullable(Some(PrepSessionId(format!("ps_{:032x}", i + 1))));
            row.contributors = members(&row.event);
            repin(&mut row.event);
            row
        })
        .collect();
    s
}
#[test]
fn c38_event_and_final_byte_caps_fail_without_partial_output() {
    let roots_and_delivery = |n| {
        let mut s = many(n, "Complete bounded body.");
        let mut delivery = normal().events[0].clone();
        delivery.event.event_id = CalendarEventId(format!("ce_{:032x}", n + 1));
        repin(&mut delivery.event);
        s.events.push(delivery);
        s
    };
    assert!(render_ics(&roots_and_delivery(255)).is_ok());
    invalid(&roots_and_delivery(256), ErrorCode::LimitExceeded);
    let s = many(256, &";".repeat(65536));
    invalid(&s, ErrorCode::LimitExceeded);
}
#[test]
fn c39_metadata_is_outside_payload_digest_but_payload_changes_are_not() {
    let mut s = single("Complete body.");
    let digest = s.events[0].event.event_payload_digest.clone();
    s.events[0].sequence = 3;
    s.events[0].revised_at = Instant("2026-10-11T13:00:00Z".into());
    let out = text(&s);
    assert!(out.contains("DTSTAMP:20261011T130000Z\r\nSEQUENCE:3\r\n"));
    assert_eq!(s.events[0].event.event_payload_digest, digest);
    for field in ["revision", "body", "start", "self_hash"] {
        let mut t = s.clone();
        let e = &mut t.events[0].event;
        match field {
            "revision" => e.payload_revision = Revision(2),
            "body" => e.body = LongText("Changed body.".into()),
            "start" => {
                e.start.local_time = LocalTime("18:01:00".into());
                e.start.instant = Instant("2026-10-12T17:01:00Z".into());
            }
            _ => {
                let mut v = serde_json::to_value(&*e).unwrap();
                v["schema_version"] = 1.into();
                e.event_payload_digest = PayloadDigest(canonical_digest(&v).unwrap().0);
            }
        }
        invalid(&t, ErrorCode::DigestMismatch);
    }
}
#[test]
fn c40_duplicate_keys_ids_or_invalid_metadata_are_refused() {
    let mut s = many(2, "Complete body.");
    s.events[1].event.event_id = s.events[0].event.event_id.clone();
    repin(&mut s.events[1].event);
    invalid(&s, ErrorCode::InvalidValue);
    let mut s = many(2, "Complete body.");
    s.events[1].event.logical_key = s.events[0].event.logical_key.clone();
    s.events[1].contributors = members(&s.events[1].event);
    repin(&mut s.events[1].event);
    invalid(&s, ErrorCode::InvalidValue);
    let mut s = single("Complete body.");
    s.events[0].sequence = u32::MAX;
    invalid(&s, ErrorCode::LimitExceeded);
    s.events[0].sequence = 0;
    s.events[0].revised_at = Instant(String::new());
    assert!(render_ics(&s).is_err());
    let mut s = single("Complete body.");
    s.events[0].contributors.clear();
    invalid(&s, ErrorCode::MissingReference);
    let mut s = single("Complete body.");
    s.events[0].contributors[0].owner.basis_digest = PayloadDigest(String::new());
    assert!(render_ics(&s).is_err());
    let mut s = single("Complete body.");
    s.events[0].contributors = vec![s.events[0].contributors[0].clone(); 4353];
    invalid(&s, ErrorCode::LimitExceeded);
    let mut s = many(2, "Complete body.");
    s.events[0].contributors = vec![s.events[0].contributors[0].clone(); 4352];
    invalid(&s, ErrorCode::LimitExceeded);
}
