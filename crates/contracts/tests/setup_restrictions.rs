use mstrchf_contracts::*;
use serde_json::{Value, json};
fn old_draft() -> Value {
    serde_json::from_slice(include_bytes!(
        "../../../tests/fixtures/contracts/household-draft.json"
    ))
    .unwrap()
}
#[test]
fn issues_are_required_in_the_wire_contract() {
    let error =
        parse_document::<HouseholdDraft>(&serde_json::to_vec(&old_draft()).unwrap()).unwrap_err();
    assert_eq!(error.code, ErrorCode::MissingField);
    assert_eq!(error.issues[0].path, "/unresolved_restrictions");
}
#[test]
fn explicit_empty_issues_are_admitted() {
    let mut draft = old_draft();
    draft["unresolved_restrictions"] = json!([]);
    parse_document::<HouseholdDraft>(&serde_json::to_vec(&draft).unwrap()).unwrap();
}

fn fixture(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/contracts")
            .join(name),
    )
    .unwrap()
}
fn draft() -> HouseholdDraft {
    parse_document(&fixture("ruling005-current--issues-valid.json")).unwrap()
}
fn command() -> AppCommandV1 {
    parse_document(&fixture("ruling005-current--edits-valid.json")).unwrap()
}
fn rejected<T: ValidateShape + serde::de::DeserializeOwned>(v: &T, code: ErrorCode, path: &str) {
    let direct = v.validate_shape().unwrap_err();
    assert!(
        direct.iter().any(|i| i.code == code && i.path == path),
        "{direct:?}"
    );
    let error = parse_document::<T>(&serde_json::to_vec(v).unwrap())
        .err()
        .unwrap();
    assert!(
        error
            .issues
            .iter()
            .any(|i| i.code == code && i.path == path),
        "{error:?}"
    );
}
#[test]
fn native_issue_identity_membership_order_and_bounds_are_checked() {
    let control = draft();
    control.validate_shape().unwrap();
    let mut d = control.clone();
    d.unresolved_restrictions.0[1].diner_id = d.unresolved_restrictions[0].diner_id.clone();
    d.unresolved_restrictions.0[1].supplied_text =
        Text("different text for duplicate owner".into());
    rejected(&d, ErrorCode::InvalidValue, "/unresolved_restrictions");
    let mut d = control.clone();
    d.unresolved_restrictions.0.reverse();
    rejected(&d, ErrorCode::NoncanonicalValue, "/unresolved_restrictions");
    let mut d = control.clone();
    d.unresolved_restrictions.0.truncate(1);
    d.unresolved_restrictions.0[0].diner_id = DinerId(format!("dn_{:032x}", 3));
    rejected(&d, ErrorCode::ForeignReference, "/unresolved_restrictions");
    let mut d = control.clone();
    d.diners.0[0].restrictions = RestrictionDeclaration::ConfirmedNone {
        confirmed_at: Instant("2026-12-21T12:00:00Z".into()),
    };
    rejected(&d, ErrorCode::InvalidValue, "/unresolved_restrictions");
    let mut d = control.clone();
    d.unresolved_restrictions.0[0].diner_id = DinerId("bad".into());
    rejected(
        &d,
        ErrorCode::InvalidValue,
        "/unresolved_restrictions/0/diner_id",
    );
    let mut d = control.clone();
    d.unresolved_restrictions.0 = vec![control.unresolved_restrictions[0].clone(); 65];
    rejected(&d, ErrorCode::LimitExceeded, "/unresolved_restrictions");
    let mut d = control;
    d.diners.0 = (1..=64)
        .map(|i| {
            let mut v = d.diners[0].clone();
            v.diner_id = DinerId(format!("dn_{i:032x}"));
            v
        })
        .collect();
    d.unresolved_restrictions.0 = d
        .diners
        .iter()
        .map(|v| SetupRestrictionIssue {
            diner_id: v.diner_id.clone(),
            supplied_text: Text("unresolved".into()),
        })
        .collect();
    d.validate_shape().unwrap();
    parse_document::<HouseholdDraft>(&canonical_bytes(&d).unwrap()).unwrap();
}
#[test]
fn edit_identity_ignores_tag_and_checks_native_command_and_envelope() {
    let mut c = command();
    let AppCommand::SaveHouseholdDraft {
        restriction_issue_edits,
        ..
    } = &mut c.command
    else {
        unreachable!()
    };
    restriction_issue_edits.0[1] = SetupRestrictionEdit::ReplaceText {
        diner_id: DinerId(format!("dn_{:032x}", 1)),
    };
    rejected(
        &c.command,
        ErrorCode::InvalidValue,
        "/data/restriction_issue_edits",
    );
    rejected(
        &c,
        ErrorCode::InvalidValue,
        "/command/data/restriction_issue_edits",
    );
    let mut c = command();
    let AppCommand::SaveHouseholdDraft {
        restriction_issue_edits,
        ..
    } = &mut c.command
    else {
        unreachable!()
    };
    restriction_issue_edits.0.reverse();
    rejected(
        &c,
        ErrorCode::NoncanonicalValue,
        "/command/data/restriction_issue_edits",
    );
    let mut c = command();
    let AppCommand::SaveHouseholdDraft {
        restriction_issue_edits,
        ..
    } = &mut c.command
    else {
        unreachable!()
    };
    restriction_issue_edits.0 = vec![
        SetupRestrictionEdit::Resolve {
            diner_id: DinerId(format!("dn_{:032x}", 1))
        };
        65
    ];
    rejected(
        &c,
        ErrorCode::LimitExceeded,
        "/command/data/restriction_issue_edits",
    );
    let mut c = command();
    let AppCommand::SaveHouseholdDraft {
        restriction_issue_edits,
        ..
    } = &mut c.command
    else {
        unreachable!()
    };
    restriction_issue_edits.0 = (1..=64)
        .map(|i| match i % 3 {
            0 => SetupRestrictionEdit::ReplaceText {
                diner_id: DinerId(format!("dn_{i:032x}")),
            },
            1 => SetupRestrictionEdit::Resolve {
                diner_id: DinerId(format!("dn_{i:032x}")),
            },
            _ => SetupRestrictionEdit::RemoveDiner {
                diner_id: DinerId(format!("dn_{i:032x}")),
            },
        })
        .collect();
    // Structural admission alone does not claim old/new transition correspondence.
    c.validate_shape().unwrap();
    parse_document::<AppCommandV1>(&canonical_bytes(&c).unwrap()).unwrap();
    let AppCommand::SaveHouseholdDraft {
        restriction_issue_edits,
        ..
    } = &mut c.command
    else {
        unreachable!()
    };
    restriction_issue_edits.0[0] = SetupRestrictionEdit::Resolve {
        diner_id: DinerId("bad".into()),
    };
    rejected(
        &c,
        ErrorCode::InvalidValue,
        "/command/data/restriction_issue_edits/0/data/diner_id",
    );
}
#[test]
fn text_scalar_limits_and_exact_unicode_whitespace_fidelity() {
    for text in [
        "🟣".repeat(4000),
        "e\u{301} 🟣\nTab\tinside".into(),
        "x".into(),
    ] {
        let mut d = draft();
        d.unresolved_restrictions.0[0].supplied_text = Text(text.clone());
        d.validate_shape().unwrap();
        let parsed: HouseholdDraft = parse_document(&canonical_bytes(&d).unwrap()).unwrap();
        assert_eq!(
            parsed.unresolved_restrictions[0].supplied_text.0.as_bytes(),
            text.as_bytes()
        );
    }
    for (text, code) in [
        ("x".repeat(4001), ErrorCode::LimitExceeded),
        ("".into(), ErrorCode::InvalidValue),
        (" x".into(), ErrorCode::InvalidValue),
        ("x\n".into(), ErrorCode::InvalidValue),
        ("x\0y".into(), ErrorCode::InvalidValue),
    ] {
        let mut d = draft();
        d.unresolved_restrictions.0[0].supplied_text = Text(text);
        rejected(&d, code, "/unresolved_restrictions/0/supplied_text");
    }
}
#[test]
fn nested_draft_arrays_are_required_in_snapshot_and_command() {
    for (name, root, path) in [
        ("snapshot-initial-privacy", "snapshot", "/household_draft"),
        ("app-command", "app-command", "/command/data/draft"),
    ] {
        let initial: Value =
            serde_json::from_slice(&fixture(&format!("ruling005-current--{name}.json"))).unwrap();
        for absent in [true, false] {
            let mut v = initial.clone();
            let d = v.pointer_mut(path).unwrap();
            if absent {
                d.as_object_mut().unwrap().remove("unresolved_restrictions");
            } else {
                d["unresolved_restrictions"] = Value::Null;
            }
            let e = schema::parse_root(root, &serde_json::to_vec(&v).unwrap()).unwrap_err();
            assert_eq!(
                e.code,
                if absent {
                    ErrorCode::MissingField
                } else {
                    ErrorCode::InvalidValue
                }
            );
            assert_eq!(e.issues[0].path, format!("{path}/unresolved_restrictions"));
        }
    }
}
#[test]
fn partial_stages_remain_presence_based_and_household_requires_confirmation() {
    let profile: HouseholdProfile =
        parse_document(&fixture("household-profile-confirmed-none.json")).unwrap();
    for mask in 0..8 {
        let mut d = draft();
        d.preferences = Nullable((mask & 1 != 0).then(|| profile.preferences.clone()));
        d.usual_week = Nullable((mask & 2 != 0).then(|| profile.usual_week.clone()));
        d.connections = Nullable((mask & 4 != 0).then(|| profile.connections.clone()));
        d.completed_stages = BoundedVec(vec![]);
        if mask & 4 != 0 {
            d.completed_stages.0.push(HouseholdStage::Connections);
        }
        if mask & 1 != 0 {
            d.completed_stages.0.push(HouseholdStage::FoodPreferences);
        }
        if mask & 2 != 0 {
            d.completed_stages.0.push(HouseholdStage::UsualWeek);
        }
        d.validate_shape().unwrap();
        d.completed_stages.0.insert(
            usize::from(mask & 4 != 0) + usize::from(mask & 1 != 0),
            HouseholdStage::Household,
        );
        rejected(&d, ErrorCode::InvalidValue, "/completed_stages");
    }
    let mut d = draft();
    d.unresolved_restrictions.0.clear();
    d.validate_shape().unwrap();
    d.completed_stages = BoundedVec(vec![HouseholdStage::Household]);
    rejected(&d, ErrorCode::InvalidValue, "/completed_stages");
    for diner in &mut d.diners.0 {
        diner.restrictions = profile.diners[0].restrictions.clone();
    }
    d.validate_shape().unwrap();
}
#[test]
fn synthetic_current_provenance_and_independent_canonical_vectors() {
    use sha2::{Digest, Sha256};
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let p: Value = serde_json::from_slice(&fixture("ruling005-provenance.json")).unwrap();
    assert_eq!(p["adaptations"].as_array().unwrap().len(), 12);
    for a in p["adaptations"].as_array().unwrap() {
        for (path, digest) in [("source", "source_sha256"), ("current", "current_sha256")] {
            let bytes = std::fs::read(root.join(a[path].as_str().unwrap())).unwrap();
            assert_eq!(
                Sha256::digest(bytes)
                    .iter()
                    .map(|b| format!("{b:02x}"))
                    .collect::<String>(),
                a[digest].as_str().unwrap()
            );
        }
    }
    let vectors: Value =
        serde_json::from_slice(&fixture("ruling005-canonical-vectors.json")).unwrap();
    for v in vectors["vectors"].as_array().unwrap() {
        let value: Value = serde_json::from_slice(&fixture(v["file"].as_str().unwrap())).unwrap();
        let bytes = canonical_bytes(&value).unwrap();
        assert_eq!(bytes, v["canonical_utf8"].as_str().unwrap().as_bytes());
        assert_eq!(
            Sha256::digest(bytes)
                .iter()
                .map(|b| format!("{b:02x}"))
                .collect::<String>(),
            v["sha256"].as_str().unwrap()
        );
    }
    assert_eq!(schema::ROOT_NAMES.len(), 19);
}
