use mstrchf_contracts::*;
use mstrchf_core::setup::*;

const U: &str = "Avoid every purple ingredient; this rule is not in the app vocabulary.";
const U2: &str = "Avoid every silver ingredient; this remains unresolved.";
const EDITS: &str = "/command/data/restriction_issue_edits";

fn profile() -> HouseholdProfile {
    parse_document(include_bytes!(
        "../../../tests/fixtures/households/p03ab/a1a2-profile.json"
    ))
    .unwrap()
}
fn draft() -> HouseholdDraft {
    parse_document(include_bytes!(
        "../../../tests/fixtures/households/p03ab/ruling005-current--a1a2-draft.json"
    ))
    .unwrap()
}
fn empty() -> HouseholdDraft {
    parse_document(include_bytes!(
        "../../../tests/fixtures/households/p03ab/ruling005-current--a1a2-empty-draft.json"
    ))
    .unwrap()
}
fn time() -> Instant {
    Instant("2026-12-21T12:00:00Z".into())
}
fn policy() -> PrivacyPolicy {
    profile().connections.privacy
}
fn id(n: usize) -> DinerId {
    DinerId(format!("dn_{n:032x}"))
}
fn confirmed() -> RestrictionDeclaration {
    RestrictionDeclaration::ConfirmedNone {
        confirmed_at: time(),
    }
}
fn issue_draft() -> HouseholdDraft {
    let mut d = draft();
    d.diners[0].restrictions = RestrictionDeclaration::Unconfirmed {};
    d.unresolved_restrictions.0.push(SetupRestrictionIssue {
        diner_id: id(1),
        supplied_text: Text(U.into()),
    });
    d.completed_stages
        .0
        .retain(|s| *s != HouseholdStage::Household);
    d
}
fn transition(
    a: &HouseholdDraft,
    b: &HouseholdDraft,
    edits: &[SetupRestrictionEdit],
) -> Result<(), DomainError> {
    let before = (a.clone(), b.clone(), edits.to_vec());
    let result = validate_setup_draft_transition(a, b, edits, &policy(), &time());
    assert_eq!((&before.0, &before.1, before.2.as_slice()), (a, b, edits));
    result
}
fn binding(
    current: Option<&HouseholdProfile>,
    saved: &HouseholdDraft,
    proposed: &HouseholdProfile,
) -> Result<(), DomainError> {
    let before = (current.cloned(), saved.clone(), proposed.clone());
    let result = validate_profile_draft_binding(current, saved, proposed, &policy(), &time());
    assert_eq!(
        (before.0.as_ref(), &before.1, &before.2),
        (current, saved, proposed)
    );
    result
}
fn error<T: std::fmt::Debug>(result: Result<T, DomainError>, code: ErrorCode, path: Option<&str>) {
    let e = result.unwrap_err();
    assert_eq!(e.code, code, "{e:?}");
    assert!(!e.issues.is_empty() && e.issues.len() <= 100);
    if let Some(path) = path {
        assert!(e.issues.iter().any(|i| i.path == path), "{e:?}");
    }
    let rendered = format!("{e:?} {e} {}", serde_json::to_string(&e).unwrap());
    assert!(!rendered.contains(U) && !rendered.contains(U2));
}
fn resolved(d: &HouseholdDraft, declaration: RestrictionDeclaration) -> HouseholdDraft {
    let mut p = d.clone();
    p.unresolved_restrictions.0.clear();
    p.diners[0].restrictions = declaration;
    p.completed_stages = BoundedVec(vec![
        HouseholdStage::Connections,
        HouseholdStage::FoodPreferences,
        HouseholdStage::Household,
        HouseholdStage::UsualWeek,
    ]);
    p
}

#[test]
fn sp06_unchanged_text_survives_unrelated_edits_without_acknowledgement() {
    let a = issue_draft();
    let mut b = a.clone();
    b.diners[0].display_name = Nullable(Some(NameText("Renamed".into())));
    b.diners[0].portions.preset = PortionPresetName::Smaller;
    b.preferences.0.as_mut().unwrap().quick_meals = !b.preferences.0.as_ref().unwrap().quick_meals;
    transition(&a, &b, &[]).unwrap();
    assert_eq!(b.unresolved_restrictions, a.unresolved_restrictions);
    assert_eq!(b.revision, Revision(7));
    error(
        transition(
            &a,
            &b,
            &[SetupRestrictionEdit::ReplaceText { diner_id: id(1) }],
        ),
        ErrorCode::InvalidValue,
        Some(EDITS),
    );
}

#[test]
fn sp06_replacement_requires_exact_edit_and_stays_unconfirmed() {
    let a = issue_draft();
    let mut b = a.clone();
    b.unresolved_restrictions[0].supplied_text = Text(U2.into());
    let replace = SetupRestrictionEdit::ReplaceText { diner_id: id(1) };
    transition(&a, &b, std::slice::from_ref(&replace)).unwrap();
    for edits in [
        vec![],
        vec![SetupRestrictionEdit::Resolve { diner_id: id(1) }],
        vec![SetupRestrictionEdit::RemoveDiner { diner_id: id(1) }],
        vec![replace.clone(), replace],
    ] {
        error(
            transition(&a, &b, &edits),
            ErrorCode::InvalidValue,
            Some(EDITS),
        );
    }
    assert_eq!(
        b.diners[0].restrictions,
        RestrictionDeclaration::Unconfirmed {}
    );
}

#[test]
fn sp07_erasure_cannot_reset_owner_to_unanswered() {
    let a = issue_draft();
    let mut b = a.clone();
    b.unresolved_restrictions.0.clear();
    for edits in [
        vec![],
        vec![SetupRestrictionEdit::Resolve { diner_id: id(1) }],
        vec![SetupRestrictionEdit::RemoveDiner { diner_id: id(1) }],
    ] {
        error(
            transition(&a, &b, &edits),
            ErrorCode::InvalidValue,
            Some(EDITS),
        );
    }
}

#[test]
fn sp07_explicit_resolution_requires_real_declaration_and_admission_time() {
    let a = issue_draft();
    let edit = [SetupRestrictionEdit::Resolve { diner_id: id(1) }];
    let declarations = [
        confirmed(),
        RestrictionDeclaration::Confirmed {
            confirmed_at: time(),
            requirements: BoundedVec(vec![HardRequirement::Diet {
                code: DietCode::Vegetarian,
            }]),
        },
    ];
    for declaration in declarations {
        let b = resolved(&a, declaration);
        transition(&a, &b, &edit).unwrap();
        error(
            transition(&a, &b, &[]),
            ErrorCode::InvalidValue,
            Some(EDITS),
        );
        error(
            transition(
                &a,
                &b,
                &[SetupRestrictionEdit::ReplaceText { diner_id: id(1) }],
            ),
            ErrorCode::InvalidValue,
            Some(EDITS),
        );
    }
    for at in ["2026-12-21T12:00:01Z", "invalid-time"] {
        let b = resolved(
            &a,
            RestrictionDeclaration::ConfirmedNone {
                confirmed_at: Instant(at.into()),
            },
        );
        error(transition(&a, &b, &edit), ErrorCode::InvalidValue, None);
    }
    let b = resolved(
        &a,
        RestrictionDeclaration::Confirmed {
            confirmed_at: time(),
            requirements: BoundedVec(vec![]),
        },
    );
    error(transition(&a, &b, &edit), ErrorCode::InvalidValue, None);
}

#[test]
fn sp08_remove_owner_requires_edit_and_closed_default_references() {
    let a = issue_draft();
    let mut b = a.clone();
    b.unresolved_restrictions.0.clear();
    b.diners.0.remove(0);
    b.completed_stages.0.insert(2, HouseholdStage::Household);
    let edits = [SetupRestrictionEdit::RemoveDiner { diner_id: id(1) }];
    error(
        transition(&a, &b, &edits),
        ErrorCode::ForeignReference,
        None,
    );
    for cell in &mut b.usual_week.0.as_mut().unwrap().cells.0 {
        cell.attendee_ids.0.retain(|i| *i != id(1));
    }
    transition(&a, &b, &edits).unwrap();
    error(
        transition(&a, &b, &[]),
        ErrorCode::InvalidValue,
        Some(EDITS),
    );
    error(
        transition(&a, &b, &[SetupRestrictionEdit::Resolve { diner_id: id(1) }]),
        ErrorCode::InvalidValue,
        Some(EDITS),
    );
}

#[test]
fn sp08_new_owner_does_not_discharge_retained_old_owner() {
    let a = issue_draft();
    let mut b = a.clone();
    b.diners[1].restrictions = RestrictionDeclaration::Unconfirmed {};
    b.unresolved_restrictions[0].diner_id = id(2);
    for edits in [
        vec![],
        vec![SetupRestrictionEdit::Resolve { diner_id: id(1) }],
        vec![SetupRestrictionEdit::ReplaceText { diner_id: id(1) }],
    ] {
        error(
            transition(&a, &b, &edits),
            ErrorCode::InvalidValue,
            Some(EDITS),
        );
    }
    b.diners[0].restrictions = confirmed();
    transition(&a, &b, &[SetupRestrictionEdit::Resolve { diner_id: id(1) }]).unwrap();
    assert_eq!(b.unresolved_restrictions[0].supplied_text, Text(U.into()));
}

#[test]
fn sp09_new_issue_has_no_old_issue_edit() {
    let a = draft();
    let b = issue_draft();
    transition(&a, &b, &[]).unwrap();
    for edit in [
        SetupRestrictionEdit::Resolve { diner_id: id(1) },
        SetupRestrictionEdit::ReplaceText { diner_id: id(1) },
        SetupRestrictionEdit::RemoveDiner { diner_id: id(2) },
    ] {
        error(
            transition(&a, &b, &[edit]),
            ErrorCode::InvalidValue,
            Some(EDITS),
        );
    }
}

#[test]
fn sp09_edits_order_by_owner_not_variant_and_duplicates_refuse() {
    let mut a = issue_draft();
    a.diners[1].restrictions = RestrictionDeclaration::Unconfirmed {};
    a.unresolved_restrictions.0.push(SetupRestrictionIssue {
        diner_id: id(2),
        supplied_text: Text(U2.into()),
    });
    let mut b = a.clone();
    b.diners[0].restrictions = confirmed();
    b.unresolved_restrictions.0.remove(0);
    b.unresolved_restrictions[0].supplied_text = Text(U.into());
    let edits = [
        SetupRestrictionEdit::Resolve { diner_id: id(1) },
        SetupRestrictionEdit::ReplaceText { diner_id: id(2) },
    ];
    transition(&a, &b, &edits).unwrap();
    error(
        transition(&a, &b, &[edits[1].clone(), edits[0].clone()]),
        ErrorCode::NoncanonicalValue,
        Some(EDITS),
    );
    error(
        transition(
            &a,
            &b,
            &[
                edits[0].clone(),
                SetupRestrictionEdit::ReplaceText { diner_id: id(1) },
            ],
        ),
        ErrorCode::InvalidValue,
        Some(EDITS),
    );
    error(
        transition(
            &a,
            &b,
            &[SetupRestrictionEdit::Resolve {
                diner_id: DinerId("invalid".into()),
            }],
        ),
        ErrorCode::InvalidValue,
        None,
    );
}

#[test]
fn sp09_sixty_four_edits_allowed_sixty_five_refuse_before_expansion() {
    let mut a = empty();
    for n in 1..=64 {
        let mut d = profile().diners[0].clone();
        d.diner_id = id(n);
        d.restrictions = RestrictionDeclaration::Unconfirmed {};
        a.diners.0.push(d);
        a.unresolved_restrictions.0.push(SetupRestrictionIssue {
            diner_id: id(n),
            supplied_text: Text(U.into()),
        });
    }
    let mut b = a.clone();
    for issue in &mut b.unresolved_restrictions.0 {
        issue.supplied_text = Text(U2.into());
    }
    let mut edits: Vec<_> = (1..=64)
        .map(|n| SetupRestrictionEdit::ReplaceText { diner_id: id(n) })
        .collect();
    transition(&a, &b, &edits).unwrap();
    edits.push(SetupRestrictionEdit::Resolve {
        diner_id: DinerId("invalid".into()),
    });
    error(
        transition(&a, &b, &edits),
        ErrorCode::LimitExceeded,
        Some(EDITS),
    );
}

#[test]
fn text_is_preserved_as_scalars_and_never_normalised() {
    let mut a = issue_draft();
    for text in ["e\u{301}\n\t🦊".to_owned(), "🦊".repeat(4000)] {
        a.unresolved_restrictions[0].supplied_text = Text(text.clone());
        transition(&a, &a, &[]).unwrap();
        let reparsed: HouseholdDraft = parse_document(&canonical_bytes(&a).unwrap()).unwrap();
        assert_eq!(
            reparsed.unresolved_restrictions[0]
                .supplied_text
                .0
                .as_bytes(),
            text.as_bytes()
        );
    }
    for text in [
        "".to_owned(),
        " leading".into(),
        "trailing ".into(),
        "bad\u{1}control".into(),
        "🦊".repeat(4001),
    ] {
        let mut b = issue_draft();
        b.unresolved_restrictions[0].supplied_text = Text(text);
        assert!(
            transition(
                &issue_draft(),
                &b,
                &[SetupRestrictionEdit::ReplaceText { diner_id: id(1) }]
            )
            .is_err()
        );
    }
}

#[test]
fn projection_preserves_all_fields_and_independent_revision() {
    let p = profile();
    let before = p.clone();
    for r in [Revision(1), Revision(7), Revision(u64::MAX)] {
        let mut expected = draft();
        expected.revision = r;
        assert_eq!(
            draft_from_profile(&p, r, &policy(), &time()).unwrap(),
            expected
        );
    }
    assert_eq!(p, before);
    assert!(draft_from_profile(&p, Revision(0), &policy(), &time()).is_err());
}

#[test]
fn sp10_initial_binding_is_exact_real_promotion_at_revision_one() {
    let d = draft();
    let mut p = profile();
    p.revision = Revision(1);
    binding(None, &d, &p).unwrap();
    let mut alternatives = Vec::new();
    let mut c = p.clone();
    c.revision = Revision(3);
    alternatives.push(c);
    let mut c = p.clone();
    c.preferences.quick_meals = !c.preferences.quick_meals;
    alternatives.push(c);
    let mut c = p.clone();
    c.diners[0].display_name = Nullable(Some(NameText("Different".into())));
    alternatives.push(c);
    let mut c = p.clone();
    c.diners[0].restrictions = confirmed();
    alternatives.push(c);
    let mut c = p.clone();
    c.diners.0.reverse();
    alternatives.push(c);
    for c in alternatives {
        assert!(binding(None, &d, &c).is_err());
    }
    assert!(binding(None, &empty(), &p).is_err());
    assert!(binding(None, &issue_draft(), &p).is_err());
}

#[test]
fn sp11_clean_draft_accepts_valid_direct_edit_at_current_profile_revision() {
    let current = profile();
    let mut proposed = current.clone();
    proposed.preferences.quick_meals = !proposed.preferences.quick_meals;
    binding(Some(&current), &draft(), &proposed).unwrap();
    binding(Some(&current), &draft(), &current).unwrap();
    for r in [Revision(1), Revision(4), Revision(7)] {
        proposed.revision = r;
        error(
            binding(Some(&current), &draft(), &proposed),
            ErrorCode::ContextStale,
            Some("/revision"),
        );
    }
}

#[test]
fn sp12_complete_dirty_draft_allows_only_exact_saved_content() {
    let current = profile();
    let mut saved = draft();
    saved.preferences.0.as_mut().unwrap().quick_meals = !current.preferences.quick_meals;
    let mut proposed = current.clone();
    proposed.preferences = saved.preferences.0.clone().unwrap();
    binding(Some(&current), &saved, &proposed).unwrap();
    error(
        binding(Some(&current), &saved, &current),
        ErrorCode::DomainConflict,
        Some("/household_draft"),
    );
    proposed.diners[0].restrictions = confirmed();
    error(
        binding(Some(&current), &saved, &proposed),
        ErrorCode::DomainConflict,
        Some("/household_draft"),
    );
}

#[test]
fn sp13_incomplete_or_unresolved_dirty_draft_cannot_be_discarded() {
    let current = profile();
    for saved in [empty(), issue_draft()] {
        error(
            binding(Some(&current), &saved, &current),
            ErrorCode::DomainConflict,
            Some("/household_draft"),
        );
    }
    let mut saved = draft();
    saved.preferences = Nullable(None);
    saved
        .completed_stages
        .0
        .retain(|s| *s != HouseholdStage::FoodPreferences);
    error(
        binding(Some(&current), &saved, &current),
        ErrorCode::DomainConflict,
        Some("/household_draft"),
    );
}

#[test]
fn timestamp_and_diner_order_differences_are_pending_content() {
    let current = profile();
    for reorder in [false, true] {
        let mut saved = draft();
        let mut proposed = current.clone();
        if reorder {
            saved.diners.0.reverse();
            proposed.diners.0.reverse();
        } else {
            saved.diners[0].restrictions = confirmed();
            proposed.diners[0].restrictions = confirmed();
        }
        error(
            binding(Some(&current), &saved, &current),
            ErrorCode::DomainConflict,
            Some("/household_draft"),
        );
        binding(Some(&current), &saved, &proposed).unwrap();
    }
}

#[test]
fn foreign_households_and_draft_counter_changes_refuse() {
    let a = issue_draft();
    let mut b = a.clone();
    b.household_id = HouseholdId(format!("hh_{:032x}", 2));
    error(
        transition(&a, &b, &[]),
        ErrorCode::IdentityMismatch,
        Some("/household_id"),
    );
    b = a.clone();
    b.revision = Revision(8);
    error(
        transition(&a, &b, &[]),
        ErrorCode::ContextStale,
        Some("/revision"),
    );
    let current = profile();
    let mut saved = draft();
    saved.household_id = HouseholdId(format!("hh_{:032x}", 2));
    error(
        binding(Some(&current), &saved, &current),
        ErrorCode::IdentityMismatch,
        Some("/household_id"),
    );
    let mut proposed = current.clone();
    proposed.household_id = saved.household_id.clone();
    error(
        binding(Some(&current), &draft(), &proposed),
        ErrorCode::IdentityMismatch,
        Some("/household_id"),
    );
    error(
        binding(None, &draft(), &proposed),
        ErrorCode::IdentityMismatch,
        Some("/household_id"),
    );
}

#[test]
fn all_helpers_reject_invalid_native_policy_and_time() {
    let p = profile();
    let d = draft();
    let mut q = policy();
    q.revision = Revision(0);
    for (q, at) in [(q, time()), (policy(), Instant("bad".into()))] {
        assert!(validate_setup_draft_transition(&d, &d, &[], &q, &at).is_err());
        assert!(draft_from_profile(&p, Revision(7), &q, &at).is_err());
        assert!(validate_profile_draft_binding(Some(&p), &d, &p, &q, &at).is_err());
    }
}

#[test]
fn stale_privacy_copies_and_future_confirmation_refuse() {
    let p = profile();
    let d = draft();
    let mut q = policy();
    q.revision = Revision(2);
    error(
        validate_setup_draft_transition(&d, &d, &[], &q, &time()),
        ErrorCode::ContextStale,
        None,
    );
    error(
        draft_from_profile(&p, Revision(7), &q, &time()),
        ErrorCode::ContextStale,
        None,
    );
    error(
        validate_profile_draft_binding(Some(&p), &d, &p, &q, &time()),
        ErrorCode::ContextStale,
        None,
    );
    let mut future = p.clone();
    future.diners[0].restrictions = RestrictionDeclaration::ConfirmedNone {
        confirmed_at: Instant("2026-12-21T12:00:01Z".into()),
    };
    error(
        draft_from_profile(&future, Revision(7), &policy(), &time()),
        ErrorCode::InvalidValue,
        None,
    );
    error(
        binding(Some(&p), &d, &future),
        ErrorCode::InvalidValue,
        None,
    );
    error(
        binding(Some(&future), &d, &p),
        ErrorCode::InvalidValue,
        None,
    );
}

#[test]
fn current_and_proposed_native_shape_and_stage_claims_are_checked() {
    let d = draft();
    let mut invalid = Vec::new();
    let mut b = d.clone();
    b.revision = Revision(0);
    invalid.push(b);
    let mut b = d.clone();
    b.diners[0].portions.adjustment.0.denominator = DecimalU64(0);
    invalid.push(b);
    let mut b = d.clone();
    b.completed_stages.0.reverse();
    invalid.push(b);
    let mut b = d.clone();
    b.completed_stages.0.pop();
    invalid.push(b);
    let mut b = d.clone();
    b.diners = BoundedVec(vec![b.diners[0].clone(); 65]);
    invalid.push(b);
    let mut b = issue_draft();
    b.unresolved_restrictions = BoundedVec(vec![b.unresolved_restrictions[0].clone(); 65]);
    invalid.push(b);
    for bad in invalid {
        assert!(transition(&bad, &d, &[]).is_err());
        assert!(transition(&d, &bad, &[]).is_err());
        assert!(binding(Some(&profile()), &bad, &profile()).is_err());
    }
}

#[test]
fn absent_stages_stay_absent_and_maximum_counters_are_not_incremented() {
    let mut a = empty();
    a.revision = Revision(u64::MAX);
    transition(&a, &a, &[]).unwrap();
    assert_eq!(a, {
        let mut d = empty();
        d.revision = Revision(u64::MAX);
        d
    });
    let mut p = profile();
    p.revision = Revision(u64::MAX);
    let mut saved = draft();
    saved.revision = Revision(u64::MAX);
    binding(Some(&p), &saved, &p).unwrap();
}

#[test]
fn sp08_removal_preserves_other_owner_and_allows_separate_new_owner() {
    let mut a = issue_draft();
    a.diners[1].restrictions = RestrictionDeclaration::Unconfirmed {};
    a.unresolved_restrictions.0.push(SetupRestrictionIssue {
        diner_id: id(2),
        supplied_text: Text(U2.into()),
    });
    let mut b = a.clone();
    b.diners.0.remove(0);
    b.unresolved_restrictions.0.remove(0);
    for cell in &mut b.usual_week.0.as_mut().unwrap().cells.0 {
        cell.attendee_ids.0.retain(|i| *i != id(1));
    }
    let edit = [SetupRestrictionEdit::RemoveDiner { diner_id: id(1) }];
    transition(&a, &b, &edit).unwrap();
    assert_eq!(b.unresolved_restrictions[0], a.unresolved_restrictions[1]);
    let mut new_diner = b.diners[0].clone();
    new_diner.diner_id = id(3);
    b.diners.0.push(new_diner);
    b.unresolved_restrictions.0.push(SetupRestrictionIssue {
        diner_id: id(3),
        supplied_text: Text(U.into()),
    });
    transition(&a, &b, &edit).unwrap();
    error(
        transition(&a, &b, &[]),
        ErrorCode::InvalidValue,
        Some(EDITS),
    );
}

#[test]
fn issues_require_unique_canonical_existing_unconfirmed_owners() {
    let a = issue_draft();
    let mut invalid = Vec::new();
    let mut b = a.clone();
    b.unresolved_restrictions[0].diner_id = id(3);
    invalid.push((b, ErrorCode::ForeignReference));
    let mut b = a.clone();
    b.unresolved_restrictions[0].diner_id = id(2);
    invalid.push((b, ErrorCode::InvalidValue));
    let mut b = a.clone();
    b.unresolved_restrictions.0.push(SetupRestrictionIssue {
        diner_id: id(1),
        supplied_text: Text(U2.into()),
    });
    invalid.push((b, ErrorCode::InvalidValue));
    let mut b = a.clone();
    b.diners[1].restrictions = RestrictionDeclaration::Unconfirmed {};
    b.unresolved_restrictions.0.insert(
        0,
        SetupRestrictionIssue {
            diner_id: id(2),
            supplied_text: Text(U2.into()),
        },
    );
    invalid.push((b, ErrorCode::NoncanonicalValue));
    for (bad, code) in invalid {
        error(transition(&a, &bad, &[]), code, None);
    }
}

#[test]
fn pending_issue_allows_exact_eight_later_stage_presence_combinations() {
    let a = issue_draft();
    for bits in 0..8 {
        let mut b = a.clone();
        if bits & 1 == 0 {
            b.connections = Nullable(None);
            b.completed_stages
                .0
                .retain(|s| *s != HouseholdStage::Connections);
        }
        if bits & 2 == 0 {
            b.preferences = Nullable(None);
            b.completed_stages
                .0
                .retain(|s| *s != HouseholdStage::FoodPreferences);
        }
        if bits & 4 == 0 {
            b.usual_week = Nullable(None);
            b.completed_stages
                .0
                .retain(|s| *s != HouseholdStage::UsualWeek);
        }
        transition(&a, &b, &[]).unwrap();
        assert_eq!(b.unresolved_restrictions, a.unresolved_restrictions);
        assert!(!b.completed_stages.contains(&HouseholdStage::Household));
    }
}

#[test]
fn every_native_profile_entrypoint_checks_children_and_references() {
    let p = profile();
    let mut invalid = Vec::new();
    let mut c = p.clone();
    c.revision = Revision(0);
    invalid.push(c);
    let mut c = p.clone();
    c.diners[0].display_name = Nullable(Some(NameText(" invalid ".into())));
    invalid.push(c);
    let mut c = p.clone();
    c.diners[0].portions.adjustment.0.denominator = DecimalU64(0);
    invalid.push(c);
    let mut c = p.clone();
    c.diners[0].restrictions = RestrictionDeclaration::Unconfirmed {};
    invalid.push(c);
    let mut c = p.clone();
    c.usual_week.cells[0].attendee_ids = BoundedVec(vec![id(3)]);
    invalid.push(c);
    let mut c = p.clone();
    c.preferences.equipment = BoundedVec(vec![EquipmentCode::Hob; 10]);
    invalid.push(c);
    let mut c = p.clone();
    c.diners = BoundedVec(vec![c.diners[0].clone(); 65]);
    invalid.push(c);
    for bad in invalid {
        assert!(draft_from_profile(&bad, Revision(7), &policy(), &time()).is_err());
        assert!(binding(Some(&bad), &draft(), &p).is_err());
        assert!(binding(Some(&p), &draft(), &bad).is_err());
        assert!(binding(None, &draft(), &bad).is_err());
    }
}

#[test]
fn authoritative_privacy_compares_full_content_even_with_same_revision() {
    let p = profile();
    let d = draft();
    let mut q = policy();
    q.use_ai_features = true;
    q.confirmed_at = Nullable(Some(time()));
    error(
        draft_from_profile(&p, Revision(7), &q, &time()),
        ErrorCode::ContextStale,
        None,
    );
    error(
        validate_setup_draft_transition(&d, &d, &[], &q, &time()),
        ErrorCode::ContextStale,
        None,
    );
    error(
        validate_profile_draft_binding(Some(&p), &d, &p, &q, &time()),
        ErrorCode::ContextStale,
        None,
    );
    let mut partial = empty();
    partial.diners = BoundedVec(p.diners.0.clone());
    partial.completed_stages.0.push(HouseholdStage::Household);
    validate_setup_draft_transition(&partial, &partial, &[], &q, &time()).unwrap();
    assert!(partial.connections.0.is_none());
}

#[test]
fn projection_retains_explicit_ai_consent_and_confirmation_fields() {
    let mut p = profile();
    p.connections.privacy.use_ai_features = true;
    p.connections.privacy.include_household_requirements = true;
    p.connections.privacy.confirmed_at = Nullable(Some(time()));
    p.connections.privacy.revision = Revision(9);
    let mut expected = draft();
    expected.connections = Nullable(Some(p.connections.clone()));
    let q = p.connections.privacy.clone();
    assert_eq!(
        draft_from_profile(&p, Revision(7), &q, &time()).unwrap(),
        expected
    );
    let mut initial = p.clone();
    initial.revision = Revision(1);
    validate_profile_draft_binding(None, &expected, &initial, &q, &time()).unwrap();
    initial.connections.privacy.confirmed_at =
        Nullable(Some(Instant("2026-12-20T12:00:00Z".into())));
    error(
        validate_profile_draft_binding(None, &expected, &initial, &q, &time()),
        ErrorCode::ContextStale,
        None,
    );
}

#[test]
fn stale_current_draft_privacy_cannot_be_used_to_clear_pending_text() {
    let mut a = issue_draft();
    a.connections.0.as_mut().unwrap().privacy.revision = Revision(2);
    error(
        transition(&a, &issue_draft(), &[]),
        ErrorCode::ContextStale,
        None,
    );
}

#[test]
fn nested_collection_limits_precede_invalid_children() {
    let mut p = profile();
    p.preferences.dislikes = BoundedVec(vec![IngredientId("bad".into()); 257]);
    error(
        draft_from_profile(&p, Revision(7), &policy(), &time()),
        ErrorCode::LimitExceeded,
        Some("/preferences/dislikes"),
    );
    let mut d = draft();
    d.usual_week.0.as_mut().unwrap().cells[0].attendee_ids =
        BoundedVec(vec![DinerId("bad".into()); 65]);
    error(
        transition(&draft(), &d, &[]),
        ErrorCode::LimitExceeded,
        Some("/usual_week/cells/attendee_ids"),
    );
}

#[test]
fn binding_validates_native_candidate_before_comparing_identity_and_revision() {
    let p = profile();
    let mut bad = p.clone();
    bad.revision = Revision(0);
    error(
        binding(Some(&p), &draft(), &bad),
        ErrorCode::NoncanonicalValue,
        Some("/revision"),
    );
    bad = p.clone();
    bad.household_id = HouseholdId("malformed".into());
    error(
        binding(Some(&p), &draft(), &bad),
        ErrorCode::InvalidValue,
        Some("/household_id"),
    );
}

#[test]
fn sn_r1_malformed_edit_precedes_stale_revision() {
    let current = issue_draft();
    let mut proposed = current.clone();
    proposed.revision = Revision(8);
    let edits = [SetupRestrictionEdit::Resolve {
        diner_id: DinerId("malformed".into()),
    }];
    error(
        transition(&current, &proposed, &edits),
        ErrorCode::InvalidValue,
        Some("/data/diner_id"),
    );
}

#[test]
fn sn_r1_malformed_edit_precedes_foreign_household() {
    let current = issue_draft();
    let mut proposed = current.clone();
    proposed.household_id = HouseholdId("hh_ffffffffffffffffffffffffffffffff".into());
    let edits = [SetupRestrictionEdit::Resolve {
        diner_id: DinerId("malformed".into()),
    }];
    error(
        transition(&current, &proposed, &edits),
        ErrorCode::InvalidValue,
        Some("/data/diner_id"),
    );
}

#[test]
fn sn_r1_all_native_edits_precede_semantic_admission_and_owner_comparisons() {
    let current = issue_draft();
    let malformed = SetupRestrictionEdit::Resolve {
        diner_id: DinerId("malformed".into()),
    };
    let mut proposed = current.clone();
    proposed.connections.0.as_mut().unwrap().privacy.revision = Revision(2);
    error(
        transition(&current, &proposed, std::slice::from_ref(&malformed)),
        ErrorCode::InvalidValue,
        Some("/data/diner_id"),
    );
    let duplicate = SetupRestrictionEdit::Resolve { diner_id: id(1) };
    error(
        transition(
            &current,
            &current,
            &[duplicate.clone(), duplicate, malformed],
        ),
        ErrorCode::InvalidValue,
        Some("/data/diner_id"),
    );
}

#[test]
fn sn_r1_collection_bounds_still_precede_native_edit_validation() {
    let current = issue_draft();
    let malformed = SetupRestrictionEdit::Resolve {
        diner_id: DinerId("malformed".into()),
    };
    error(
        transition(&current, &current, &vec![malformed.clone(); 65]),
        ErrorCode::LimitExceeded,
        Some(EDITS),
    );
    let mut proposed = current.clone();
    proposed.unresolved_restrictions =
        BoundedVec(vec![proposed.unresolved_restrictions[0].clone(); 65]);
    error(
        transition(&current, &proposed, &[malformed]),
        ErrorCode::LimitExceeded,
        Some("/unresolved_restrictions"),
    );
}
