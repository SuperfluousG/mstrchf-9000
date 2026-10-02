use mstrchf_calendar::*;
use mstrchf_contracts::*;
#[path = "../../../tests/fixtures/calendars/native.rs"]
mod native;
use native as fixtures;
#[path = "../../../tests/fixtures/calendars/aggregate.rs"]
mod aggregate;
mod support;

#[test]
fn aggregate_session_limit_refuses_individually_valid_owners_before_expansion() {
    let corpus = aggregate::session_limit_owners();
    assert_eq!(corpus.len(), 18);
    assert_eq!(
        corpus
            .iter()
            .map(|f| f.historical.derived.prep_sessions.len())
            .sum::<usize>(),
        4608
    );
    assert_aggregate_refusal(corpus, "/sessions/observed=4608/limit=4352");
}
#[test]
fn aggregate_task_limit_refuses_individually_valid_owners_before_expansion() {
    let corpus = aggregate::task_limit_owners();
    assert_eq!(corpus.len(), 61);
    assert_eq!(
        corpus
            .iter()
            .flat_map(|f| f.historical.derived.prep_sessions.iter())
            .map(|s| s.tasks.len())
            .sum::<usize>(),
        20496
    );
    assert_aggregate_refusal(corpus, "/tasks/observed=20496/limit=20480");
}
fn assert_aggregate_refusal(corpus: &[fixtures::Fixture], path: &str) {
    let linked = corpus[1..]
        .iter()
        .map(|f| OwnerInput {
            historical: &f.historical,
            current: &f.current,
        })
        .collect::<Vec<_>>();
    let result = prepare_events(&CalendarInputs {
        subject: OwnerInput {
            historical: &corpus[0].historical,
            current: &corpus[0].current,
        },
        linked_owners: &linked,
        assemblies: &[],
        instruction_text: &[],
        evaluated_at: &corpus[0].current.feasibility.evaluated_at,
    });
    match result {
        Err(CalendarFailure::Invalid(e)) => {
            assert_eq!(e.code, ErrorCode::LimitExceeded);
            assert_eq!(e.issues[0].path, path);
        }
        other => panic!("expected atomic aggregate refusal: {other:?}"),
    }
}

#[test]
fn c01_original_normal_matrix_has_three_complete_events_without_duplicate_prep() {
    let f = fixtures::normal();
    let h = &f.historical;
    let p = support::prepare(
        h,
        &f.current,
        &support::assemblies(h),
        &support::production_text(h),
    )
    .unwrap();
    assert_eq!(p.drafts.len(), 3);
    assert!(p.reports[0].warnings.is_empty());
    let dinner = p
        .drafts
        .iter()
        .find(|d| {
            d.logical_key.kind == CalendarEventKind::Cooking && d.start.local_date.0 == "2026-10-12"
        })
        .unwrap();
    let breakfast = p
        .drafts
        .iter()
        .find(|d| d.start.local_date.0 == "2026-10-13")
        .unwrap();
    assert_eq!(dinner.start.local_time.0, "18:00:00");
    assert_eq!(dinner.end.local_time.0, "18:10:00");
    assert_eq!(breakfast.start.local_time.0, "07:55:00");
    assert_eq!(breakfast.end.local_time.0, "08:00:00");
    for text in [
        fixtures::normal_case::BASE_PREP,
        fixtures::normal_case::BASE_GRILL,
        fixtures::normal_case::TOMATO_EXTRA,
        fixtures::normal_case::OATS_PREP,
    ] {
        assert_eq!(dinner.body.0.matches(text).count(), 1);
    }
    for amount in ["2 each (Whole)", "60 g", "100 g", "40 g", "100 ml"] {
        assert!(dinner.body.0.contains(amount), "missing {amount}");
    }
    assert!(dinner.body.0.contains("Diner 2: 1 servings"));
    assert_eq!(
        breakfast
            .body
            .0
            .matches(fixtures::normal_case::OATS_FINISH)
            .count(),
        1
    );
    assert!(!breakfast.body.0.contains(fixtures::normal_case::OATS_PREP));
    assert!(breakfast.body.0.contains("Preparation scheduled elsewhere"));
    assert!(
        breakfast
            .body
            .0
            .contains(fixtures::normal_case::OATS_STORAGE)
    );
    assert_eq!(
        p.drafts
            .iter()
            .filter(|d| d.logical_key.kind == CalendarEventKind::Prep)
            .count(),
        0
    );
    let drafts=p.drafts.iter().map(|d|serde_json::json!({"logical_key":d.logical_key,"start":d.start,"end":d.end,"title":d.title,"body":d.body})).collect::<Vec<_>>();
    println!(
        "CALENDAR_EVIDENCE={}",
        serde_json::json!({"fixture":"C01_normal_synthetic","saved_basis_digest":mstrchf_core::food::saved_basis_digest(&h.basis).unwrap(),"derived_digest":h.derived.digest,"current_inputs_digest":f.current.digest,"report_digest":p.reports[0].digest,"drafts":drafts})
    );
}

#[test]
fn c03_original_standalone_matrix_retains_dinner_and_four_events() {
    let f = fixtures::normal_standalone();
    let h = &f.historical;
    let p = support::prepare(
        h,
        &f.current,
        &support::assemblies(h),
        &support::production_text(h),
    )
    .unwrap();
    assert_eq!(p.drafts.len(), 4);
    assert!(p.reports[0].warnings.is_empty());
    let delivery = &p.drafts[0];
    let prep = &p.drafts[1];
    let breakfast = &p.drafts[2];
    let dinner = &p.drafts[3];
    assert_eq!(delivery.logical_key.kind, CalendarEventKind::Delivery);
    assert_eq!(delivery.start.local_date.0, "2026-10-10");
    assert_eq!(prep.logical_key.kind, CalendarEventKind::Prep);
    assert_eq!(prep.start.local_date.0, "2026-10-11");
    assert_eq!(prep.start.local_time.0, "19:00:00");
    assert_eq!(prep.end.local_time.0, "19:03:00");
    assert_eq!(breakfast.start.local_date.0, "2026-10-12");
    assert_eq!(breakfast.start.local_time.0, "07:55:00");
    assert_eq!(breakfast.end.local_time.0, "08:00:00");
    assert_eq!(dinner.start.local_date.0, "2026-10-12");
    assert_eq!(dinner.start.local_time.0, "18:00:00");
    assert_eq!(dinner.end.local_time.0, "18:10:00");
    assert_eq!(
        prep.body
            .0
            .matches(fixtures::normal_case::C03_OATS_PREP)
            .count(),
        1
    );
    assert_eq!(
        breakfast
            .body
            .0
            .matches(fixtures::normal_case::C03_OATS_FINISH)
            .count(),
        1
    );
    assert!(
        breakfast
            .body
            .0
            .contains(fixtures::normal_case::C03_OATS_STORAGE)
    );
    assert!(breakfast.body.0.contains("2026-10-11 19:00:00"));
    assert!(
        !breakfast
            .body
            .0
            .contains(fixtures::normal_case::C03_OATS_PREP)
    );
    assert!(!dinner.body.0.contains("oats"));
    for text in [
        fixtures::normal_case::BASE_PREP,
        fixtures::normal_case::BASE_GRILL,
        fixtures::normal_case::TOMATO_EXTRA,
    ] {
        assert_eq!(dinner.body.0.matches(text).count(), 1);
    }
    let drafts=p.drafts.iter().map(|d|serde_json::json!({"logical_key":d.logical_key,"start":d.start,"end":d.end,"title":d.title,"body":d.body})).collect::<Vec<_>>();
    println!(
        "CALENDAR_EVIDENCE={}",
        serde_json::json!({"fixture":"C03_standalone_synthetic","saved_basis_digest":mstrchf_core::food::saved_basis_digest(&h.basis).unwrap(),"derived_digest":h.derived.digest,"current_inputs_digest":f.current.digest,"report_digest":p.reports[0].digest,"drafts":drafts})
    );
}

#[test]
fn c01_c02_extra_is_separate_quantity_but_not_an_extra_event() {
    let f = fixtures::with_extra();
    let h = &f.historical;
    let p = support::prepare(
        h,
        &f.current,
        &support::assemblies(h),
        &support::production_text(h),
    )
    .unwrap();
    assert_eq!(p.drafts.len(), 2);
    let d = p
        .drafts
        .iter()
        .find(|d| d.logical_key.kind == CalendarEventKind::Cooking)
        .unwrap();
    assert_eq!(d.start.local_time.0, "17:59:00");
    assert_eq!(d.end.local_time.0, "18:10:00");
    assert_eq!(d.body.0.matches(fixtures::EXTRA_TEXT).count(), 1);
    assert_eq!(d.body.0.matches(fixtures::SOURCE_TEXT).count(), 1);
    assert!(d.body.0.contains("100 g (Raw) Synthetic fresh tomatoes"));
    assert_eq!(d.contributors.len(), 1);
}
#[test]
fn c03_standalone_prep_preserves_role_and_original_clock() {
    let f = fixtures::standalone();
    let h = &f.historical;
    let p = support::prepare(
        h,
        &f.current,
        &support::assemblies(h),
        &support::production_text(h),
    )
    .unwrap();
    assert_eq!(p.drafts.len(), 2);
    let d = p
        .drafts
        .iter()
        .find(|d| d.logical_key.kind == CalendarEventKind::Prep)
        .unwrap();
    assert_eq!(d.start.local_date.0, "2026-10-04");
    assert_eq!(d.start.local_time.0, "19:00:00");
    assert!(d.body.0.contains(fixtures::STANDALONE_TEXT));
    assert!(d.body.0.contains("2026-10-05 18:10:00 Europe/London"));
    assert!(d.body.0.contains("2026-10-04 19:10:00 Europe/London"));
}
#[test]
fn c14_remix_retains_source_and_only_adds_new_ingredients() {
    let f = fixtures::remix();
    let h = &f.historical;
    let p = support::prepare(
        h,
        &f.current,
        &support::assemblies(h),
        &support::production_text(h),
    )
    .unwrap();
    assert_eq!(p.drafts.len(), 4);
    let wrap = p
        .drafts
        .iter()
        .find(|d| d.body.0.contains(fixtures::WRAP_TEXT))
        .unwrap();
    assert!(wrap.body.0.contains("1 each (Prepared) Synthetic wrap"));
    assert!(wrap.body.0.contains("300 g (Cooked)"));
    assert!(!wrap.body.0.contains("480 g"));
    assert!(!wrap.body.0.contains(fixtures::REMIX_SOURCE_TEXT));
    assert!(wrap.body.0.contains(fixtures::REMIX_COOLING_TEXT));
}
#[test]
fn c16_c17_missing_or_duplicate_contributor_refuses() {
    let f = fixtures::direct_reuse(false);
    let h = &f.historical;
    let t = support::direct_text(h);
    let mut a = support::assemblies(h);
    a[0].contributors.clear();
    assert!(matches!(
        support::prepare(h, &f.current, &a, &t),
        Err(CalendarFailure::ReconciliationRequired { .. })
    ));
    let mut a = support::assemblies(h);
    let root = a[0].root.clone();
    a[0].contributors.push(root);
    assert!(matches!(
        support::prepare(h, &f.current, &a, &t),
        Err(CalendarFailure::Invalid(_))
    ));
}

#[test]
fn c15_shared_prior_event_retains_both_owners_and_full_instructions() {
    let f = fixtures::linked_prior();
    let h = &f.subject.historical;
    let prior = &f.prior.historical;
    let root = support::assemblies(prior)[0].root.clone();
    let child = support::assemblies(h)[0].root.clone();
    let a = vec![AssemblyEvidence {
        root: root.clone(),
        contributors: vec![root, child],
        evidence: h.basis.evidence.recipes[0].admitted.clone(),
    }];
    let mut t = support::production_text(h);
    t.extend(support::production_text(prior));
    let linked = [OwnerInput {
        historical: prior,
        current: &f.prior.current,
    }];
    let make = |a: &[AssemblyEvidence], t: &[OccurrenceText], linked: &[OwnerInput<'_>]| {
        prepare_events(&CalendarInputs {
            subject: OwnerInput {
                historical: h,
                current: &f.subject.current,
            },
            linked_owners: linked,
            assemblies: a,
            instruction_text: t,
            evaluated_at: &f.subject.current.feasibility.evaluated_at,
        })
    };
    let p = make(&a, &t, &linked).unwrap();
    assert_eq!(p.drafts.len(), 2);
    assert_eq!(p.reports.len(), 2);
    let d = p
        .drafts
        .iter()
        .find(|d| d.logical_key.kind == CalendarEventKind::Cooking)
        .unwrap();
    assert_eq!(d.logical_key.week_id, prior.derived.week_id);
    assert_eq!(d.contributors.len(), 2);
    assert_eq!(d.start.local_time.0, "18:50:00");
    assert_eq!(d.end.local_time.0, "19:15:00");
    for literal in [
        fixtures::PRIOR_MIX_TEXT,
        fixtures::PRIOR_BAKE_TEXT,
        fixtures::STANDALONE_TEXT,
    ] {
        assert_eq!(d.body.0.matches(literal).count(), 1);
    }
    assert!(d.body.0.contains("unattended"));
    assert_eq!(
        p.drafts
            .iter()
            .filter(|d| d.logical_key.kind == CalendarEventKind::Delivery)
            .count(),
        1
    );
    match make(&a, &t, &[]) {
        Err(CalendarFailure::ReconciliationRequired { reason, .. }) => {
            assert!(reason.issues[0].path.contains(&prior.derived.week_id.0));
            assert!(
                reason.issues[0]
                    .path
                    .contains(&prior.derived.prep_sessions[0].session_id.0)
            );
        }
        other => panic!("expected owner custody refusal: {other:?}"),
    }
    let mut missing = a.clone();
    missing[0].contributors.pop();
    match make(&missing, &t, &linked) {
        Err(CalendarFailure::ReconciliationRequired { keys, .. }) => {
            assert_eq!(keys, vec![d.logical_key.clone()])
        }
        other => panic!("expected contributor custody refusal: {other:?}"),
    }
}

#[test]
fn all_contributing_blocking_reports_are_retained_before_output() {
    use mstrchf_core::food::*;
    let mut f = fixtures::linked_prior();
    for fixture in [&mut f.subject, &mut f.prior] {
        let h = &fixture.historical;
        fixture.current.evidence.recipes[0] = LiveRecipeState::Withdrawn {
            recipe: h.basis.evidence.recipes[0].recipe.clone(),
            evidence: h.basis.evidence.recipes[0].admitted.clone(),
        };
        support::repin_current(&mut fixture.current, h);
    }
    let linked = [OwnerInput {
        historical: &f.prior.historical,
        current: &f.prior.current,
    }];
    let result = prepare_events(&CalendarInputs {
        subject: OwnerInput {
            historical: &f.subject.historical,
            current: &f.subject.current,
        },
        linked_owners: &linked,
        assemblies: &[],
        instruction_text: &[],
        evaluated_at: &f.subject.current.feasibility.evaluated_at,
    });
    let mut expected = vec![
        (
            support::pin(&f.subject.historical),
            assess_continuation(
                &verify_historical(&f.subject.historical).unwrap(),
                &f.subject.current,
            )
            .unwrap(),
        ),
        (
            support::pin(&f.prior.historical),
            assess_continuation(
                &verify_historical(&f.prior.historical).unwrap(),
                &f.prior.current,
            )
            .unwrap(),
        ),
    ];
    expected.sort_by(|a, b| owner_cmp(&a.0, &b.0));
    assert!(expected.iter().all(|(_, r)| !r.blocking.is_empty()));
    assert_eq!(
        result,
        Err(CalendarFailure::Blocked(
            expected.into_iter().map(|(_, r)| r).collect()
        ))
    );
}
