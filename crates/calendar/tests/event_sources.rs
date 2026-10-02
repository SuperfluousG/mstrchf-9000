use mstrchf_calendar::*;
use mstrchf_contracts::*;
use mstrchf_core::food::*;
#[path = "../../../tests/fixtures/calendars/native.rs"]
mod fixtures;
#[path = "../../../tests/fixtures/food/native/support.rs"]
mod native;
mod support;

#[test]
fn c14_direct_cold_and_reheated_keep_exact_prior_source_without_rebuying() {
    for reheat in [false, true] {
        let f = fixtures::direct_reuse(reheat);
        let h = &f.historical;
        let output = support::prepare(
            h,
            &f.current,
            &support::assemblies(h),
            &support::direct_text(h),
        )
        .unwrap();
        assert_eq!(output.drafts.len(), 3);
        assert_eq!(
            output.reports,
            vec![assess_continuation(&verify_historical(h).unwrap(), &f.current).unwrap()]
        );
        let target = output
            .drafts
            .iter()
            .find(|d| d.start.local_date.0 == "2026-10-06")
            .unwrap();
        assert_eq!(
            target.start.local_time.0,
            if reheat { "12:55:00" } else { "12:59:00" }
        );
        assert_eq!(target.end.local_time.0, "13:00:00");
        assert!(
            target
                .body
                .0
                .contains("Preparation scheduled earlier for this reuse")
        );
        assert!(target.body.0.contains("2026-10-05 18:10:00 Europe/London"));
        assert!(target.body.0.contains("200 g"));
        assert!(target.body.0.contains(if reheat {
            fixtures::REHEAT_TEXT
        } else {
            fixtures::COLD_TEXT
        }));
        assert!(target.body.0.contains(fixtures::COOLING_TEXT));
        assert!(!target.body.0.contains(fixtures::SOURCE_TEXT));
        assert!(!target.body.0.contains("480 g"));
        assert!(target.body.0.contains(if reheat {
            "Planned reheats: 1"
        } else {
            "Planned reheats: 0"
        }));
    }
}

#[test]
fn r2_r06_current_reuse_cannot_repeat_source_production_text() {
    let f = fixtures::direct_reuse(false);
    let h = &f.historical;
    let mut text = support::direct_text(h);
    let source_step = text[0].steps[0].clone();
    text[1].steps.insert(0, source_step);
    assert!(matches!(
        support::prepare(h, &f.current, &support::assemblies(h), &text),
        Err(CalendarFailure::Invalid(_))
    ));
}

#[test]
fn c12_exact_scaled_projection_and_presentation_evidence_are_required() {
    let f = fixtures::direct_reuse(false);
    let h = &f.historical;
    for mode in 0..3 {
        let mut text = support::direct_text(h);
        match mode {
            0 => text[0].scaled.multiplier = PositiveRational::new(3, 2).unwrap(),
            1 => text[0].evidence.clear(),
            _ => text[1].steps.clear(),
        }
        assert!(matches!(
            support::prepare(h, &f.current, &support::assemblies(h), &text),
            Err(CalendarFailure::Invalid(_))
        ));
    }
}

// These failures precede content construction. Full native premises are checked
// explicitly so a broken fixture cannot masquerade as a calendar refusal.
#[test]
fn c09_blocking_current_preserves_complete_report_and_order() {
    let h = native::historical();
    let c: ContinuationInputs = native::read(&native::value("upper-exclusive-continuation-inputs"));
    let expected = assess_continuation(&verify_historical(&h).unwrap(), &c).unwrap();
    assert!(!expected.blocking.is_empty());
    let original = h.clone();
    let result = prepare_events(&CalendarInputs {
        subject: OwnerInput {
            historical: &h,
            current: &c,
        },
        linked_owners: &[],
        assemblies: &[],
        instruction_text: &[],
        evaluated_at: &c.feasibility.evaluated_at,
    });
    assert_eq!(result, Err(CalendarFailure::Blocked(vec![expected])));
    assert_eq!(h, original);
}

#[test]
fn c08_missing_current_source_fails_before_body() {
    let h = native::historical();
    let mut c: ContinuationInputs = native::read(&native::value("known-continuation-inputs"));
    assert!(
        assess_continuation(&verify_historical(&h).unwrap(), &c)
            .unwrap()
            .blocking
            .is_empty()
    );
    c.sources.remove(0);
    let result = prepare_events(&CalendarInputs {
        subject: OwnerInput {
            historical: &h,
            current: &c,
        },
        linked_owners: &[],
        assemblies: &[],
        instruction_text: &[],
        evaluated_at: &c.feasibility.evaluated_at,
    });
    assert!(matches!(result, Err(CalendarFailure::Invalid(_))));
}

#[test]
fn malformed_native_clock_does_not_panic_or_guess() {
    let h = native::historical();
    let c: ContinuationInputs = native::read(&native::value("known-continuation-inputs"));
    assert!(
        assess_continuation(&verify_historical(&h).unwrap(), &c)
            .unwrap()
            .blocking
            .is_empty()
    );
    let result = prepare_events(&CalendarInputs {
        subject: OwnerInput {
            historical: &h,
            current: &c,
        },
        linked_owners: &[],
        assemblies: &[],
        instruction_text: &[],
        evaluated_at: &Instant("bad clock".into()),
    });
    assert!(matches!(result, Err(CalendarFailure::Invalid(_))));
}

#[test]
fn c04_delivery_end_equality_is_allowed_one_second_late_is_atomic() {
    for late in [false, true] {
        let mut f = fixtures::direct_reuse(false);
        f.historical.order.confirmation.delivery_end =
            fixtures::at(5, if late { "18:00:01" } else { "18:00:00" });
        f.current.feasibility.delivery = DeliveryInput::Confirmed {
            order: f.historical.order.clone(),
        };
        support::repin_current(&mut f.current, &f.historical);
        let h = &f.historical;
        let expected = assess_continuation(&verify_historical(h).unwrap(), &f.current).unwrap();
        assert_eq!(!expected.blocking.is_empty(), late);
        let result = support::prepare(
            h,
            &f.current,
            &support::assemblies(h),
            &support::direct_text(h),
        );
        if late {
            assert_eq!(result, Err(CalendarFailure::Blocked(vec![expected])));
        } else {
            assert_eq!(result.unwrap().drafts.len(), 3);
        }
    }
}
#[test]
fn c05_unknown_dates_remain_visible_for_source_and_reuse() {
    let f = fixtures::direct_reuse(false);
    let h = &f.historical;
    let p = support::prepare(
        h,
        &f.current,
        &support::assemblies(h),
        &support::direct_text(h),
    )
    .unwrap();
    assert!(
        p.reports[0]
            .feasibility
            .uses
            .iter()
            .any(|u| u.issues.contains(&UseIssue::UnknownProductDate))
    );
    for d in &p.drafts {
        assert!(d.body.0.contains("UnknownProductDate"));
        assert!(
            d.body
                .0
                .contains("freshness through this use has not been established")
        );
    }
}
#[test]
fn c06_c07_history_tamper_cannot_be_hidden_by_outer_hash() {
    let f = fixtures::direct_reuse(false);
    for variant in 0..5 {
        let mut h = f.historical.clone();
        match variant {
            0 => h.basis.context.recipes[0].title = Title("Different bytes".into()),
            1 => h.order.ordered_recipe_refs.0.clear(),
            2 => h.derived.food_inputs_digest = PayloadDigest("a".repeat(64)),
            3 => h.approved.plan_revision = Revision(2),
            _ => h.derived.allocations[0].quantity.amount = PositiveRational::new(9, 1).unwrap(),
        }
        assert!(matches!(
            support::prepare(&h, &f.current, &[], &[]),
            Err(CalendarFailure::Invalid(_))
        ));
    }
}
#[test]
fn c10_current_restriction_preserves_all_reports() {
    let mut f = fixtures::direct_reuse(true);
    f.current.policy.maximum_reheats = 0;
    support::repin_current(&mut f.current, &f.historical);
    let expected =
        assess_continuation(&verify_historical(&f.historical).unwrap(), &f.current).unwrap();
    assert!(!expected.blocking.is_empty());
    assert_eq!(
        support::prepare(&f.historical, &f.current, &[], &[]),
        Err(CalendarFailure::Blocked(vec![expected]))
    );
}

#[test]
fn r2_r03_r05_wrong_source_batch_or_component_are_refused() {
    let f = fixtures::direct_reuse(false);
    for variant in 0..3 {
        let mut h = f.historical.clone();
        if let BatchUse::Leftover {
            batch_id,
            component_ids,
            ..
        } = &mut h.basis.plan.selections[1].batch_use
        {
            if variant == 0 {
                *batch_id = BatchId(format!("ba_{:032x}", 999));
            } else {
                component_ids[0] = ComponentId(format!("co_{:032x}", 999));
            }
        }
        if variant == 2 {
            h.basis.schedule.layout.routes.remove(0);
        }
        assert!(matches!(
            support::prepare(&h, &f.current, &[], &[]),
            Err(CalendarFailure::Invalid(_))
        ));
    }
}

#[test]
fn c13_fractional_quantities_are_not_rounded() {
    let mut saved = fixtures::standalone().historical.basis;
    saved.context.recipes[0].ingredients[0]
        .quantity
        .0
        .as_mut()
        .unwrap()
        .amount = PositiveRational::new(1, 3).unwrap();
    saved.context.recipes[0].steps[0].text=Text("Use exactly 1/3 g of the synthetic first ingredient and the other declared amounts; follow the admitted ten-minute preparation and cooling method.".into());
    fixtures::refresh_recipe(&mut saved, 0);
    let f = fixtures::finish(saved);
    let h = &f.historical;
    let p = support::prepare(
        h,
        &f.current,
        &support::assemblies(h),
        &support::production_text(h),
    )
    .unwrap();
    let d = p
        .drafts
        .iter()
        .find(|d| d.logical_key.kind == CalendarEventKind::Prep)
        .unwrap();
    assert!(d.body.0.contains("1/3 g"));
    assert!(!d.body.0.contains("0.33"));
}

#[test]
fn c18_complete_body_overflow_refuses_without_truncation() {
    let mut saved = fixtures::standalone().historical.basis;
    let r = &mut saved.context.recipes[0];
    let mut step = r.steps[0].clone();
    step.active_minutes = 0;
    step.elapsed_minutes = 0;
    step.ingredient_ids.0.clear();
    step.component_ids.0.clear();
    for id in 3..=22 {
        step.step_id = id;
        step.depends_on = BoundedVec(vec![if id == 3 { 1 } else { id - 1 }]);
        step.text = Text(format!(
            "Synthetic admitted instruction {id}: {}",
            "x".repeat(3900)
        ));
        r.steps.0.push(step.clone());
        r.prep[0].step_ids.0.push(id);
    }
    fixtures::refresh_recipe(&mut saved, 0);
    let f = fixtures::finish(saved);
    let h = &f.historical;
    let result = support::prepare(
        h,
        &f.current,
        &support::assemblies(h),
        &support::production_text(h),
    );
    match result {
        Err(CalendarFailure::Invalid(e)) => {
            assert_eq!(e.code, ErrorCode::LimitExceeded);
            assert!(e.issues[0].path.contains("/body/bytes/observed="));
            assert!(e.issues[0].path.contains("limit=65536"));
        }
        other => panic!("expected atomic body refusal: {other:?}"),
    }
}

#[test]
fn c11_empty_selected_method_is_refused_without_inventing_a_session() {
    let mut h = fixtures::direct_reuse(false).historical;
    h.basis.evidence.methods[1].prep_ids.clear();
    // Source validity is intentionally broken here. Calendar must preserve the
    // earlier native refusal rather than fabricate a replacement phase.
    let c = fixtures::direct_reuse(false).current;
    assert!(matches!(
        support::prepare(&h, &c, &[], &[]),
        Err(CalendarFailure::Invalid(_))
    ));
}

#[test]
fn instruction_array_is_topologically_ordered_with_source_order_ties() {
    let mut saved = fixtures::standalone().historical.basis;
    let r = &mut saved.context.recipes[0];
    let mut last = r.steps[0].clone();
    last.step_id = 4;
    last.active_minutes = 0;
    last.elapsed_minutes = 0;
    last.ingredient_ids.0.clear();
    last.component_ids.0.clear();
    last.depends_on = BoundedVec(vec![3]);
    last.text = Text("Third instruction follows second.".into());
    let mut second = last.clone();
    second.step_id = 3;
    second.depends_on = BoundedVec(vec![1]);
    second.text = Text("Second instruction follows first.".into());
    r.steps.0.insert(0, last);
    r.steps.0.push(second);
    r.prep[0].step_ids = BoundedVec(vec![1, 3, 4]);
    fixtures::refresh_recipe(&mut saved, 0);
    let f = fixtures::finish(saved);
    let h = &f.historical;
    let mut t = support::production_text(h);
    t[0].steps.sort_by_key(|s| s.step_id);
    let p = support::prepare(h, &f.current, &support::assemblies(h), &t).unwrap();
    let body = &p
        .drafts
        .iter()
        .find(|d| d.logical_key.kind == CalendarEventKind::Prep)
        .unwrap()
        .body
        .0;
    assert!(body.find("Second instruction").unwrap() < body.find("Third instruction").unwrap());
    t[0].steps.swap(0, 1);
    assert!(matches!(
        support::prepare(h, &f.current, &support::assemblies(h), &t),
        Err(CalendarFailure::Invalid(_))
    ));
}

#[test]
fn native_text_and_pin_wrappers_are_revalidated() {
    let f = fixtures::direct_reuse(false);
    let h = &f.historical;
    for variant in 0..5 {
        let mut t = support::direct_text(h);
        match variant {
            0 => t[0].steps[0].text = Text("invalid\rtext".into()),
            1 => t[0].evidence[0].revision = Revision(0),
            2 => t[0].owner.basis_digest = PayloadDigest("bad".into()),
            3 => t[0].steps[0].text = Text("".into()),
            _ => {
                t[0].scaled.components[0].quantity.amount = PositiveRational(Rational {
                    numerator: DecimalU64(0),
                    denominator: DecimalU64(0),
                })
            }
        }
        assert!(matches!(
            support::prepare(h, &f.current, &support::assemblies(h), &t),
            Err(CalendarFailure::Invalid(_))
        ));
    }
}

#[test]
fn local_clock_labels_include_verified_offset_and_original_instant() {
    let f = fixtures::direct_reuse(false);
    let h = &f.historical;
    let p = support::prepare(
        h,
        &f.current,
        &support::assemblies(h),
        &support::direct_text(h),
    )
    .unwrap();
    let source = p
        .drafts
        .iter()
        .find(|d| {
            d.logical_key.kind == CalendarEventKind::Cooking && d.start.local_date.0 == "2026-10-05"
        })
        .unwrap();
    assert!(source.body.0.contains("UTC+01:00"));
    assert!(source.body.0.contains("2026-10-05T17:00:00Z"));
}

#[test]
fn evidence_snapshot_for_review_retains_complete_bodies() {
    let f = fixtures::direct_reuse(true);
    let h = &f.historical;
    let p = support::prepare(
        h,
        &f.current,
        &support::assemblies(h),
        &support::direct_text(h),
    )
    .unwrap();
    assert_eq!(p.drafts.len(), 3);
    let drafts=p.drafts.iter().map(|d|serde_json::json!({"logical_key":d.logical_key,"start":d.start,"end":d.end,"title":d.title,"body":d.body})).collect::<Vec<_>>();
    println!(
        "CALENDAR_EVIDENCE={}",
        serde_json::json!({"fixture":"reheated_reuse_synthetic","saved_basis_digest":saved_basis_digest(&h.basis).unwrap(),"derived_digest":h.derived.digest,"current_inputs_digest":f.current.digest,"report_digest":p.reports[0].digest,"drafts":drafts})
    );
}

#[test]
fn reuse_body_distinguishes_portions_from_production_and_consolidates_warnings() {
    let f = fixtures::direct_reuse(false);
    let h = &f.historical;
    let p = support::prepare(
        h,
        &f.current,
        &support::assemblies(h),
        &support::direct_text(h),
    )
    .unwrap();
    let body = &p
        .drafts
        .iter()
        .find(|d| d.start.local_date.0 == "2026-10-06")
        .unwrap()
        .body
        .0;
    assert!(body.contains("Portions reused: 1"));
    assert!(!body.contains("Produced portions:"));
    assert_eq!(body.matches("Synthetic beans —").count(), 1);
    assert!(body.contains("preparation phase 2"));
    assert!(body.contains("consumption"));
    assert_eq!(body.matches("Credits —").count(), 1);
}
