use super::*;
use mstrchf_contracts::*;
use mstrchf_store::{Store, StoreConfig};
use serde_json::{Value, json};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
fn root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "mstrchf-u1-native-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}
fn open(root: &Path) -> Store {
    Store::open(StoreConfig {
        app_data_root: root.to_owned(),
    })
    .unwrap()
}
fn command(id: u64, revision: u64, command: AppCommand) -> String {
    serde_json::to_string(&AppCommandV1 {
        schema_version: SchemaVersion,
        command_id: CommandId(format!("cm_{id:032x}")),
        expected_snapshot_revision: Revision(revision),
        command,
    })
    .unwrap()
}
fn privacy(base: &Snapshot, id: u64) -> String {
    command(
        id,
        base.snapshot_revision.0,
        AppCommand::SetPrivacy {
            policy: base.privacy.clone(),
            expected_policy_revision: base.privacy.revision,
        },
    )
}

#[test]
fn host_path_is_fixed_and_disjoint_from_normal_household_root() {
    let local = root();
    assert_eq!(
        development_root(&local),
        local.join("MSTRCHF9000-Development/loop001-m1")
    );
    assert!(!development_root(&local).starts_with(local.join("MSTRCHF9000")));
    assert!(
        !local.exists(),
        "path resolution must never create or open a Store"
    );
}

#[test]
fn shared_parser_refuses_oversize_duplicate_unknown_and_path_payloads_without_mutation() {
    let store = open(&root());
    let base = store.snapshot().unwrap();
    let valid = privacy(&base, 1);
    let mut unknown: Value = serde_json::from_str(&valid).unwrap();
    unknown["path"] = json!("C:/Users/real-household");
    let mut unknown_kind: Value = serde_json::from_str(&valid).unwrap();
    unknown_kind["command"]["kind"] = json!("open_arbitrary_path");
    for (body, code) in [
        (" ".repeat(MAX_DOCUMENT_BYTES + 1), ErrorCode::LimitExceeded),
        (
            format!("{{\"schema_version\":1,{}", &valid[1..]),
            ErrorCode::DuplicateKey,
        ),
        (
            serde_json::to_string(&unknown).unwrap(),
            ErrorCode::UnknownField,
        ),
        ("C:/arbitrary/file".into(), ErrorCode::MalformedJson),
        (unknown_kind.to_string(), ErrorCode::InvalidValue),
    ] {
        assert_eq!(dispatch_json(&store, &body).unwrap_err().code, code);
        assert_eq!(store.snapshot().unwrap(), base);
    }
}

#[test]
fn draft_privacy_and_exact_historical_receipt_survive_restart() {
    let root = root();
    let store = open(&root);
    let initial = store.snapshot().unwrap();
    let mut draft = initial.household_draft.clone();
    draft.household_id = initial.household_draft.household_id.clone();
    draft.revision = initial.household_draft.revision;
    let save = command(
        1,
        1,
        AppCommand::SaveHouseholdDraft {
            draft: draft.clone(),
            restriction_issue_edits: BoundedVec(vec![]),
        },
    );
    let receipt = dispatch_json(&store, &save).unwrap();
    let saved = store.snapshot().unwrap();
    assert_eq!(saved.snapshot_revision, Revision(2));
    assert_eq!(saved.household_draft.preferences, draft.preferences);
    let mut policy = saved.privacy.clone();
    policy.use_ai_features = true;
    policy.include_household_requirements = true;
    // Match the actual workbench: preserve the host snapshot's confirmation value.
    assert_eq!(policy.confirmed_at, Nullable(None));
    let update = command(
        2,
        2,
        AppCommand::SetPrivacy {
            policy: policy.clone(),
            expected_policy_revision: saved.privacy.revision,
        },
    );
    dispatch_json(&store, &update).unwrap();
    let current = store.snapshot().unwrap();
    assert_eq!(current.snapshot_revision, Revision(3));
    assert!(current.privacy.use_ai_features);
    assert!(current.privacy.include_household_requirements);
    assert_eq!(current.privacy.confirmed_at, policy.confirmed_at);
    assert_eq!(current.household_draft.preferences, draft.preferences);
    assert_eq!(
        dispatch_json(&store, &save).unwrap().response_digest,
        receipt.response_digest
    );
    assert_eq!(store.snapshot().unwrap(), current);
    drop(store);
    let reopened = open(&root);
    assert_eq!(reopened.snapshot().unwrap(), current);
    assert_eq!(
        dispatch_json(&reopened, &save).unwrap().response_digest,
        receipt.response_digest
    );
    assert_eq!(reopened.snapshot().unwrap(), current);
}

#[test]
fn lost_response_recovers_through_same_envelope_without_second_mutation() {
    let store = open(&root());
    let initial = store.snapshot().unwrap();
    let envelope = privacy(&initial, 1);
    let lost_response = dispatch_json(&store, &envelope).unwrap();
    let recovered = dispatch_json(&store, &envelope).unwrap();
    assert_eq!(recovered.response_digest, lost_response.response_digest);
    assert_eq!(
        recovered.disposition,
        crate::receipt::ShellReceiptDisposition::Replayed
    );
    assert_eq!(store.snapshot().unwrap().snapshot_revision, Revision(2));
    let mut changed: Value = serde_json::from_str(&envelope).unwrap();
    changed["expected_snapshot_revision"] = json!("2");
    assert_eq!(
        dispatch_json(&store, &changed.to_string())
            .unwrap_err()
            .code,
        ErrorCode::ReplayConflict
    );
}

#[test]
fn advertised_support_does_not_bypass_store_admission() {
    let store = open(&root());
    let initial = store.snapshot().unwrap();
    let unsupported = command(
        1,
        1,
        AppCommand::UpdateProfile {
            profile: parse_document(include_bytes!(
                "../../tests/fixtures/contracts/household-profile.json"
            ))
            .unwrap(),
            expected_profile_revision: Revision(1),
        },
    );
    assert_eq!(
        dispatch_json(&store, &unsupported).unwrap_err().code,
        ErrorCode::UnsupportedCapability
    );
    let cancel = command(
        2,
        1,
        AppCommand::CancelJob {
            job_id: JobId("jb_00000000000000000000000000000001".into()),
        },
    );
    assert_eq!(
        dispatch_json(&store, &cancel).unwrap_err().code,
        ErrorCode::MissingReference,
        "support does not invent a job"
    );
    assert_eq!(store.snapshot().unwrap(), initial);
}

#[test]
fn unsupported_current_command_is_refused_before_store_entry() {
    let request: JobRequestV1 = parse_document(include_bytes!(
        "../../tests/fixtures/contracts/ruling004-current--job-request-product-observation.json"
    ))
    .unwrap();
    let request_job = command(
        2,
        1,
        AppCommand::RequestJob {
            action: request.action,
            scope: request.scope,
        },
    );
    let publish_calendar = command(
        3,
        1,
        AppCommand::RequestCalendarPublication {
            order_id: OrderId("or_00000000000000000000000000000001".into()),
            destination_id: CalendarDestinationId("cd_00000000000000000000000000000001".into()),
            event_batch_digest: Digest("a".repeat(64)),
        },
    );
    for valid in [request_job, publish_calendar] {
        let mut entered = false;
        let error = dispatch_with(&valid, |_| {
            entered = true;
            Err(DomainError::new(
                ErrorCode::StorageFailure,
                "/entry-barrier",
            ))
        })
        .unwrap_err();
        assert!(
            !entered,
            "unsupported producer reached Store execution entry"
        );
        assert_eq!(
            error,
            DomainError::new(ErrorCode::UnsupportedCapability, "/command")
        );
    }
}
