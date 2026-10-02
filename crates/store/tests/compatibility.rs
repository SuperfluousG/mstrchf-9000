//! Integrated compatibility controls; literals are accepted immutable reader fixtures.
use super::*;
use rusqlite::{Connection, params};
pub(crate) fn literal(name: &str) -> Vec<u8> {
    std::fs::read(
        std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("src/compat/fixtures")
            .join(name),
    )
    .unwrap()
}
pub(crate) fn old() -> Connection {
    let c = Connection::open_in_memory().unwrap();
    c.execute_batch(migrate::SQL).unwrap();
    c.execute(
        "INSERT INTO schema_migrations VALUES(1,?1,'2026-10-02T00:00:00Z')",
        [bytes_digest(migrate::SQL.as_bytes()).0],
    )
    .unwrap();
    c.execute(
        "INSERT INTO app_meta VALUES(1,'00000000000000000000000000000001','2','2','1','1')",
        [],
    )
    .unwrap();
    let s: serde_json::Value =
        serde_json::from_slice(&literal("old-privacy-response.json")).unwrap();
    c.execute(
        "INSERT INTO household VALUES(1,?1,'1',?2,'2',?3,NULL,NULL)",
        params![
            s["household_draft"]["household_id"].as_str().unwrap(),
            canonical_bytes(&s["household_draft"]).unwrap(),
            canonical_bytes(&s["privacy"]).unwrap()
        ],
    )
    .unwrap();
    let a = literal("old-privacy-command.json");
    let b = literal("old-privacy-response.json");
    c.execute(
        "INSERT INTO command_receipts VALUES(?1,?2,?3,?4,?5,'2')",
        params![
            "cm_00000000000000000000000000000002",
            bytes_digest(&a).0,
            a,
            b,
            bytes_digest(&b).0
        ],
    )
    .unwrap();
    c.pragma_update(None, "user_version", 1).unwrap();
    c
}
#[test]
fn retained_literal_is_audited_by_old_graph() {
    let c = old();
    assert_eq!(audit::audit_b1(&c).unwrap().reader, "B1RetainedV1");
}
#[test]
fn fresh_creation_has_complete_target_identity() {
    let mut c = Connection::open_in_memory().unwrap();
    migrate::initialise(&mut c, "00000000000000000000000000000001").unwrap();
    let a = audit::audit_b1(&c).unwrap();
    assert_eq!(a.version, 2);
    assert_eq!(a.reader, "B1Draft005");
    assert_eq!(a.ledger.len(), 2);
    assert_eq!(a.row_counts.len(), 8);
    assert_eq!(a.counters.snapshot, Revision(1));
}
#[test]
fn bare001_current_body_refuses() {
    let c = old();
    c.execute(
        "UPDATE household SET draft=?1,draft_revision='2'",
        [literal("new-transition-draft.json")],
    )
    .unwrap();
    assert_eq!(
        audit::audit_b1(&c).unwrap_err().code,
        ErrorCode::UnsupportedVersion
    );
}
#[test]
fn every_hidden_terminal_is_checked() {
    let c = old();
    for n in 1..=1025 {
        let b = format!(
            r#"{{"action":"setup","job_id":"jb_{n:032x}","next_action":null,"reason":null,"state":"succeeded"}}"#
        );
        c.execute(
            "INSERT INTO jobs VALUES(?1,?2,1,?3)",
            params![format!("jb_{n:032x}"), n, b.as_bytes()],
        )
        .unwrap();
    }
    assert!(audit::audit_b1(&c).is_ok());
    c.execute(
        "UPDATE jobs SET summary=x'7b7d' WHERE creation_sequence=1",
        [],
    )
    .unwrap();
    assert!(audit::audit_b1(&c).is_err());
}

fn native_old(label: &str) -> std::path::PathBuf {
    let p = std::env::temp_dir().join(format!(
        "MSTRCHF compat {label} {}",
        records::random_id().unwrap()
    ));
    std::fs::create_dir_all(&p).unwrap();
    let c = old();
    let mut dst = Connection::open(p.join("planner.sqlite")).unwrap();
    {
        let b = rusqlite::backup::Backup::new(&c, &mut dst).unwrap();
        b.run_to_completion(100, std::time::Duration::from_millis(1), None)
            .unwrap();
    }
    dst.close().unwrap();
    p
}
#[test]
fn retained_native_upgrade_has_exact_live_and_historical_bytes() {
    let p = native_old("retained transition");
    let store = Store::open(StoreConfig {
        app_data_root: p.clone(),
    })
    .unwrap();
    assert_eq!(
        canonical_bytes(&store.snapshot().unwrap()).unwrap(),
        literal("new-transition-snapshot.json")
    );
    drop(store);
    let attempt = std::fs::read_dir(p.join("private-backups"))
        .unwrap()
        .next()
        .unwrap()
        .unwrap()
        .path();
    let backup = Connection::open_with_flags(
        attempt.join("database.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        audit::frozen_snapshot(&backup, compat::Profile::Old).unwrap(),
        literal("old-privacy-response.json")
    );
    assert_eq!(audit::audit_b1(&backup).unwrap().version, 1);
    backup.close().unwrap();
    let c = Connection::open(p.join("planner.sqlite")).unwrap();
    assert_eq!(audit::audit_b1(&c).unwrap().version, 2);
    let r:(Vec<u8>,Vec<u8>,String)=c.query_row("SELECT command,response,profile FROM command_receipts JOIN receipt_formats USING(command_id)",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
    assert_eq!(
        r,
        (
            literal("old-privacy-command.json"),
            literal("old-privacy-response.json"),
            "B1RetainedV1".into()
        )
    );
}
#[test]
fn metadata_missing_and_misbinding_refuse() {
    let mut c = Connection::open_in_memory().unwrap();
    migrate::initialise(&mut c, "00000000000000000000000000000001").unwrap();
    c.execute("DELETE FROM store_format", []).unwrap();
    assert!(audit::audit_b1(&c).is_err());
}

#[test]
fn old_replay_is_lookup_only_and_does_not_rewind() {
    let p = native_old("lookup only");
    let store = Store::open(StoreConfig { app_data_root: p }).unwrap();
    let before = store.snapshot().unwrap();
    let a = literal("old-privacy-command.json");
    let r = store.replay_ui(&a).unwrap();
    assert_eq!(r.disposition(), CommandDisposition::Replayed);
    assert!(matches!(r.response(), ReceiptSnapshot::B1RetainedV1(_)));
    assert_eq!(
        r.original_response_bytes(),
        literal("old-privacy-response.json")
    );
    let current: AppCommandV1 = parse_document(&a).unwrap();
    assert_eq!(
        store
            .dispatch_ui(current)
            .unwrap()
            .original_response_bytes(),
        r.original_response_bytes()
    );
    assert_eq!(store.snapshot().unwrap(), before);
    let mut v: serde_json::Value = serde_json::from_slice(&a).unwrap();
    v["command_id"] = serde_json::json!("cm_00000000000000000000000000000009");
    assert_eq!(
        store
            .replay_ui(&canonical_bytes(&v).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    );
    v["command_id"] = serde_json::json!("cm_00000000000000000000000000000002");
    v["command"]["data"] = serde_json::json!({"current_shape":true});
    assert_eq!(
        store
            .replay_ui(&canonical_bytes(&v).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::ReplayConflict
    );
    assert_eq!(store.snapshot().unwrap(), before);
}

thread_local! {static CLOCK:std::cell::Cell<Option<&'static str>>=const{std::cell::Cell::new(None)};}
pub(crate) fn clock() -> Option<&'static str> {
    CLOCK.with(|c| c.get())
}
#[test]
fn issue_resolve_exact_literals_and_noop_counters() {
    CLOCK.with(|c| c.set(Some("2026-12-21T12:00:00Z")));
    let p = native_old("setup producer");
    let mut db = database::Database::open(&p).unwrap();
    for (a, b) in [
        ("new-issue-command.json", "new-issue-response.json"),
        ("new-resolve-command.json", "new-resolve-response.json"),
    ] {
        let bytes = literal(a);
        let cmd: AppCommandV1 = parse_document(&bytes).unwrap();
        let o = db.dispatch_outcome(&cmd, &bytes).unwrap();
        assert_eq!(o.disposition(), CommandDisposition::Applied);
        assert_eq!(o.original_response_bytes(), literal(b));
    }
    let state = db.snapshot().unwrap();
    let bytes = literal("new-issue-command.json");
    let o = db
        .replay("cm_00000000000000000000000000000003", &bytes)
        .unwrap();
    assert_eq!(
        o.original_response_bytes(),
        literal("new-issue-response.json")
    );
    assert_eq!(db.snapshot().unwrap(), state);
    let c = AppCommandV1 {
        schema_version: SchemaVersion,
        command_id: CommandId("cm_00000000000000000000000000000005".into()),
        expected_snapshot_revision: state.snapshot_revision,
        command: AppCommand::SaveHouseholdDraft {
            draft: state.household_draft.clone(),
            restriction_issue_edits: BoundedVec(vec![]),
        },
    };
    db.dispatch_outcome(&c, &canonical_bytes(&c).unwrap())
        .unwrap();
    let after = db.snapshot().unwrap();
    assert_eq!(
        after.household_draft.revision,
        state.household_draft.revision
    );
    assert_eq!(after.snapshot_revision, Revision(6));
    CLOCK.with(|c| c.set(None));
}

fn current() -> Connection {
    let mut c = Connection::open_in_memory().unwrap();
    migrate::initialise(&mut c, "00000000000000000000000000000001").unwrap();
    c.execute("UPDATE household SET household_id='hh_00000000000000000000000000000001',draft=?1,draft_revision='2',privacy=?2,privacy_revision='2'",params![literal("new-transition-draft.json"),canonical_bytes(&serde_json::json!({"revision":"2","confirmed_at":null,"include_household_requirements":false,"use_ai_features":false})).unwrap()]).unwrap();
    c.execute(
        "UPDATE app_meta SET snapshot_revision='3',settings_revision='2'",
        [],
    )
    .unwrap();
    c
}
fn bind(c: &Connection, a: &[u8], b: &[u8], tag: Option<&str>) {
    let cmd: serde_json::Value = serde_json::from_slice(a).unwrap();
    let response: serde_json::Value = serde_json::from_slice(b).unwrap();
    let id = cmd["command_id"].as_str().unwrap();
    c.execute(
        "INSERT INTO command_receipts VALUES(?1,?2,?3,?4,?5,?6)",
        params![
            id,
            bytes_digest(a).0,
            a,
            b,
            bytes_digest(b).0,
            response["snapshot_revision"].as_str().unwrap()
        ],
    )
    .unwrap();
    if let Some(tag) = tag {
        c.execute(
            "INSERT INTO receipt_formats VALUES(?1,?2)",
            params![id, tag],
        )
        .unwrap();
    }
}
#[test]
fn all_r2_receipt_controls_run_through_whole_database_audit() {
    let manifest: serde_json::Value =
        serde_json::from_slice(&literal("correction-known-answers-r2.json")).unwrap();
    for case in manifest["cases"].as_array().unwrap() {
        let is_old = case["profile"] == "B1RetainedV1";
        let c = if is_old {
            let c = old();
            c.execute("DELETE FROM command_receipts", []).unwrap();
            c
        } else {
            current()
        };
        let a = literal(
            case["command_file"]
                .as_str()
                .unwrap()
                .strip_prefix("fixtures/")
                .unwrap(),
        );
        let b = literal(
            case["response_file"]
                .as_str()
                .unwrap()
                .strip_prefix("fixtures/")
                .unwrap(),
        );
        assert_eq!(bytes_digest(&a).0, case["command_sha256"]);
        assert_eq!(bytes_digest(&b).0, case["response_sha256"]);
        bind(&c, &a, &b, if is_old { None } else { Some("B1Draft005") });
        assert_eq!(
            audit::audit_b1(&c).is_ok(),
            case["expected_pair_audit"] == "supported",
            "{} {}",
            case["profile"],
            case["case"]
        );
    }
}
#[test]
fn target_binding_totality_misbinding_and_receipt_integrity() {
    for fault in [
        "missing",
        "wrong",
        "orphan",
        "unknown",
        "digest",
        "bytes",
        "projection",
        "command",
        "singleton",
    ] {
        let c = current();
        bind(
            &c,
            &literal("old-privacy-command.json"),
            &literal("old-privacy-response.json"),
            Some("B1RetainedV1"),
        );
        assert!(audit::audit_b1(&c).is_ok());
        c.pragma_update(None, "foreign_keys", false).unwrap();
        c.pragma_update(None, "ignore_check_constraints", true)
            .unwrap();
        let sql = match fault {
            "missing" => "DELETE FROM receipt_formats",
            "wrong" => "UPDATE receipt_formats SET profile='B1Draft005'",
            "orphan" => {
                "INSERT INTO receipt_formats VALUES('cm_00000000000000000000000000000009','B1Draft005')"
            }
            "unknown" => "UPDATE receipt_formats SET profile='unknown'",
            "digest" => {
                "UPDATE command_receipts SET response_digest='0000000000000000000000000000000000000000000000000000000000000000'"
            }
            "bytes" => "UPDATE command_receipts SET response=x'7b7d'",
            "projection" => "UPDATE command_receipts SET snapshot_revision='1'",
            "command" => "UPDATE command_receipts SET command=x'7b7d'",
            _ => "DELETE FROM store_format",
        };
        c.execute(sql, []).unwrap();
        assert!(audit::audit_b1(&c).is_err(), "{fault}");
    }
}
#[test]
fn old_mixed_and_unsupported_receipts_refuse() {
    for (a, b) in [
        ("new-issue-command.json", "new-issue-response.json"),
        (
            "unsupported-update-profile-command.json",
            "old-privacy-response.json",
        ),
        (
            "old-privacy-command.json",
            "unsupported-conflict-snapshot.json",
        ),
    ] {
        let c = old();
        c.execute("DELETE FROM command_receipts", []).unwrap();
        c.execute("UPDATE app_meta SET snapshot_revision='9'", [])
            .unwrap();
        bind(&c, &literal(a), &literal(b), None);
        assert!(audit::audit_b1(&c).is_err());
    }
}
#[test]
fn envelope_rejects_duplicates_limits_and_never_executes() {
    let c = current();
    let mut dbbytes = literal("old-privacy-command.json");
    dbbytes.push(b' ');
    assert!(compat::envelope(&dbbytes).is_ok());
    let duplicate = String::from_utf8(literal("old-privacy-command.json"))
        .unwrap()
        .replacen(
            "{",
            "{\"command_id\":\"cm_00000000000000000000000000000002\",",
            1,
        );
    assert_eq!(
        compat::envelope(duplicate.as_bytes()).unwrap_err().code,
        ErrorCode::DuplicateKey
    );
    assert_eq!(
        compat::envelope(&vec![b' '; MAX_DOCUMENT_BYTES + 1])
            .unwrap_err()
            .code,
        ErrorCode::LimitExceeded
    );
    let a = literal("old-privacy-command.json");
    let b = literal("old-privacy-response.json");
    bind(&c, &a, &b, Some("B1RetainedV1"));
    assert_eq!(
        receipts::lookup_id(&c, "cm_00000000000000000000000000000002", &dbbytes)
            .unwrap_err()
            .code,
        ErrorCode::ReplayConflict
    );
    for bytes in [b"{}".as_slice(),br#"{"schema_version":1,"command_id":"bad","expected_snapshot_revision":"1","command":{"kind":"set_privacy","data":{}}}"#]{assert!(compat::envelope(bytes).is_err());}
}
#[test]
fn migration_checked_streams_only_and_exact_rollback() {
    for field in ["snapshot", "draft", "unchanged"] {
        let p = native_old(field);
        let c = Connection::open(p.join("planner.sqlite")).unwrap();
        if field == "snapshot" {
            c.execute(
                "UPDATE app_meta SET snapshot_revision='18446744073709551615'",
                [],
            )
            .unwrap();
        } else if field == "draft" {
            let mut d: serde_json::Value =
                serde_json::from_slice(&literal("old-draft.json")).unwrap();
            d["revision"] = serde_json::json!("18446744073709551615");
            c.execute(
                "UPDATE household SET draft=?1,draft_revision='18446744073709551615'",
                [canonical_bytes(&d).unwrap()],
            )
            .unwrap();
        } else {
            let policy = serde_json::json!({"revision":"18446744073709551615","use_ai_features":false,"include_household_requirements":false,"confirmed_at":null});
            c.execute(
                "UPDATE household SET privacy=?1,privacy_revision='18446744073709551615'",
                [canonical_bytes(&policy).unwrap()],
            )
            .unwrap();
            c.execute(
                "UPDATE app_meta SET settings_revision='18446744073709551615'",
                [],
            )
            .unwrap();
        }
        let before = audit::audit_b1(&c).unwrap();
        c.close().unwrap();
        let opened = database::Database::open(&p);
        if field == "unchanged" {
            let db = opened.unwrap();
            let a = audit::audit_b1(&db.connection).unwrap();
            assert_eq!(a.counters.settings, Revision(u64::MAX));
            assert_eq!(a.counters.snapshot, Revision(3));
        } else {
            assert_eq!(opened.err().unwrap().code, ErrorCode::ArithmeticOverflow);
            let c = Connection::open(p.join("planner.sqlite")).unwrap();
            assert_eq!(audit::audit_b1(&c).unwrap(), before);
        }
    }
}
#[test]
fn recorded_complete_old_stage_publishes_then_backs_up_and_upgrades() {
    let p = native_old("old init stage");
    let stage = "init-00000000000000000000000000000001.sqlite";
    std::fs::rename(p.join("planner.sqlite"), p.join(stage)).unwrap();
    let f = std::fs::File::open(p.join(stage)).unwrap();
    let identity = windows_fs::identity(&f).unwrap();
    drop(f);
    std::fs::write(p.join("initialisation.json"),serde_json::to_vec(&serde_json::json!({"lineage":"00000000000000000000000000000001","stage":stage,"identity":identity})).unwrap()).unwrap();
    let db = database::Database::open(&p).unwrap();
    assert_eq!(audit::audit_b1(&db.connection).unwrap().version, 2);
    assert!(p.join("private-backups").is_dir());
    assert_eq!(
        audit::frozen_snapshot(&db.connection, compat::Profile::Current).unwrap(),
        literal("new-transition-snapshot.json")
    );
}
#[test]
fn native_writer_time_refuses_future_confirmation_and_keeps_issue() {
    let p = native_old("native trusted clock");
    let mut db = database::Database::open(&p).unwrap();
    let bytes = literal("new-issue-command.json");
    let cmd: AppCommandV1 = parse_document(&bytes).unwrap();
    db.dispatch_outcome(&cmd, &bytes).unwrap();
    let before = db.snapshot().unwrap();
    let mut cmd: AppCommandV1 = parse_document(&literal("new-resolve-command.json")).unwrap();
    if let AppCommand::SaveHouseholdDraft { draft, .. } = &mut cmd.command {
        draft.diners[0].restrictions = RestrictionDeclaration::ConfirmedNone {
            confirmed_at: Instant::try_from("9999-12-31T23:59:59Z").unwrap(),
        };
    }
    assert!(
        db.dispatch_outcome(&cmd, &canonical_bytes(&cmd).unwrap())
            .is_err()
    );
    assert_eq!(db.snapshot().unwrap(), before);
}

#[test]
fn opaque_resource_keys_use_database_budget_not_document_grammar() {
    let c = old();
    c.execute("INSERT INTO jobs VALUES('jb_00000000000000000000000000000001',1,1,?1)",[br#"{"action":"setup","job_id":"jb_00000000000000000000000000000001","next_action":null,"reason":null,"state":"succeeded"}"#.as_slice()]).unwrap();
    let key = "x".repeat(MAX_DOCUMENT_BYTES + 1);
    c.execute(
        "INSERT INTO resource_quarantine VALUES(?1,'jb_00000000000000000000000000000001',1)",
        [&key],
    )
    .unwrap();
    assert!(audit::audit_b1(&c).unwrap().asset_refs.is_empty());
}
#[test]
fn old_save_replays_exactly_and_current_shape_conflicts_first() {
    let p = native_old("old save");
    let c = Connection::open(p.join("planner.sqlite")).unwrap();
    c.execute("DELETE FROM command_receipts", []).unwrap();
    bind(
        &c,
        &literal("old-save-command.json"),
        &literal("old-save-response.json"),
        None,
    );
    c.close().unwrap();
    let store = Store::open(StoreConfig { app_data_root: p }).unwrap();
    let before = store.snapshot().unwrap();
    assert!(parse_document::<AppCommandV1>(&literal("old-save-command.json")).is_err());
    let o = store.replay_ui(&literal("old-save-command.json")).unwrap();
    assert_eq!(
        o.original_response_bytes(),
        literal("old-save-response.json")
    );
    assert!(matches!(o.response(), ReceiptSnapshot::B1RetainedV1(_)));
    assert_eq!(
        store
            .replay_ui(&literal("new-save-same-id-command.json"))
            .unwrap_err()
            .code,
        ErrorCode::ReplayConflict
    );
    assert_eq!(store.snapshot().unwrap(), before);
}
#[test]
fn recovered_old_and_current_outcomes_remain_typed() {
    let p = native_old("typed command recovery");
    let mut db = database::Database::open(&p).unwrap();
    let a = literal("old-privacy-command.json");
    let cmd: AppCommandV1 = parse_document(&a).unwrap();
    crate::tests::inject("replay_return_error");
    let o = db.dispatch_outcome(&cmd, &a).unwrap();
    assert!(matches!(o.response(), ReceiptSnapshot::B1RetainedV1(_)));
    assert_eq!(o.disposition(), CommandDisposition::Replayed);
    assert_eq!(
        o.original_response_bytes(),
        literal("old-privacy-response.json")
    );
    let a = literal("new-issue-command.json");
    let cmd: AppCommandV1 = parse_document(&a).unwrap();
    crate::tests::inject("after_commit_error");
    let o = db.dispatch_outcome(&cmd, &a).unwrap();
    assert!(matches!(o.response(), ReceiptSnapshot::B1Draft005(_)));
    assert_eq!(
        o.original_response_bytes(),
        literal("new-issue-response.json")
    );
    assert_eq!(db.snapshot().unwrap().snapshot_revision, Revision(4));
}
#[test]
fn confirmed_values_and_pending_draft_profile_divergence_survive_migration() {
    for draft_name in ["old-draft.json", "old-unconfirmed-draft.json"] {
        let p = native_old("profile divergence");
        let c = Connection::open(p.join("planner.sqlite")).unwrap();
        let v: serde_json::Value =
            serde_json::from_slice(&literal("old-profile-readable-snapshot.json")).unwrap();
        let profile = canonical_bytes(&v["profile"]).unwrap();
        let d = literal(draft_name);
        c.execute("DELETE FROM command_receipts", []).unwrap();
        c.execute("UPDATE household SET profile=?1,profile_revision='1',privacy=?2,privacy_revision='1',draft=?3",params![profile,canonical_bytes(&v["privacy"]).unwrap(),d]).unwrap();
        c.execute("UPDATE app_meta SET settings_revision='1'", [])
            .unwrap();
        assert!(audit::audit_b1(&c).is_ok());
        c.close().unwrap();
        let mut db = database::Database::open(&p).unwrap();
        let after = db.snapshot().unwrap();
        assert_eq!(
            canonical_bytes(after.profile.0.as_ref().unwrap()).unwrap(),
            profile
        );
        let mut expected: serde_json::Value = serde_json::from_slice(&d).unwrap();
        expected["revision"] = serde_json::json!("2");
        expected["unresolved_restrictions"] = serde_json::json!([]);
        assert_eq!(
            canonical_bytes(&after.household_draft).unwrap(),
            canonical_bytes(&expected).unwrap()
        );
        if draft_name == "old-unconfirmed-draft.json" {
            assert_eq!(
                canonical_bytes(&after.household_draft).unwrap(),
                literal("new-unconfirmed-transition-draft.json")
            );
        }
        assert_eq!(after.snapshot_revision, Revision(3));
    }
}
unsafe extern "C" fn deny_binding(
    _: *mut std::ffi::c_void,
    action: i32,
    name: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
) -> i32 {
    if action == rusqlite::ffi::SQLITE_INSERT
        && !name.is_null()
        && unsafe { std::ffi::CStr::from_ptr(name) }.to_bytes() == b"receipt_formats"
    {
        rusqlite::ffi::SQLITE_DENY
    } else {
        rusqlite::ffi::SQLITE_OK
    }
}
#[test]
fn failed_binding_insert_rolls_back_body_counters_and_receipt() {
    let mut c = current();
    let before = audit::audit_b1(&c).unwrap();
    unsafe {
        rusqlite::ffi::sqlite3_set_authorizer(c.handle(), Some(deny_binding), std::ptr::null_mut())
    };
    let a = literal("new-issue-command.json");
    let cmd: AppCommandV1 = parse_document(&a).unwrap();
    assert!(commands::execute_outcome(&mut c, &cmd, &a).is_err());
    assert!(c.is_autocommit());
    unsafe { rusqlite::ffi::sqlite3_set_authorizer(c.handle(), None, std::ptr::null_mut()) };
    assert_eq!(audit::audit_b1(&c).unwrap(), before);
}
#[test]
fn noop_preserves_max_draft_and_changed_draft_refuses_overflow() {
    let mut c = current();
    let mut d: HouseholdDraft = parse_document(&literal("new-transition-draft.json")).unwrap();
    d.revision = Revision(u64::MAX);
    c.execute(
        "UPDATE household SET draft=?1,draft_revision='18446744073709551615'",
        [canonical_bytes(&d).unwrap()],
    )
    .unwrap();
    let mut cmd = AppCommandV1 {
        schema_version: SchemaVersion,
        command_id: CommandId("cm_00000000000000000000000000000003".into()),
        expected_snapshot_revision: Revision(3),
        command: AppCommand::SaveHouseholdDraft {
            draft: d,
            restriction_issue_edits: BoundedVec(vec![]),
        },
    };
    commands::execute_outcome(&mut c, &cmd, &canonical_bytes(&cmd).unwrap()).unwrap();
    let state = snapshot::load(&c).unwrap();
    assert_eq!(state.household_draft.revision, Revision(u64::MAX));
    assert_eq!(state.snapshot_revision, Revision(4));
    let before = audit::audit_b1(&c).unwrap();
    cmd.command_id = CommandId("cm_00000000000000000000000000000004".into());
    cmd.expected_snapshot_revision = Revision(4);
    if let AppCommand::SaveHouseholdDraft { draft, .. } = &mut cmd.command {
        let i: AppCommandV1 = parse_document(&literal("new-issue-command.json")).unwrap();
        if let AppCommand::SaveHouseholdDraft { draft: issue, .. } = i.command {
            draft.diners = issue.diners;
            draft.unresolved_restrictions = issue.unresolved_restrictions;
        }
    }
    assert_eq!(
        commands::execute_outcome(&mut c, &cmd, &canonical_bytes(&cmd).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::ArithmeticOverflow
    );
    assert_eq!(audit::audit_b1(&c).unwrap(), before);
}

#[test]
fn old_cancel_overlap_replays_original_without_touching_live_job_or_resource() {
    let p = native_old("old cancellation replay");
    let c = Connection::open(p.join("planner.sqlite")).unwrap();
    let a = literal("old-cancel-command.json");
    let b = literal("old-cancel-response.json");
    let response: serde_json::Value = serde_json::from_slice(&b).unwrap();
    c.execute("DELETE FROM command_receipts", []).unwrap();
    c.execute(
        "INSERT INTO jobs VALUES(?1,1,1,?2)",
        params![
            response["jobs"][0]["job_id"].as_str().unwrap(),
            canonical_bytes(&response["jobs"][0]).unwrap()
        ],
    )
    .unwrap();
    bind(&c, &a, &b, None);
    assert!(audit::audit_b1(&c).is_ok());
    c.close().unwrap();
    let mut db = database::Database::open(&p).unwrap();
    // A separate live synthetic job can differ from its historical receipt.
    db.connection.execute("INSERT INTO jobs VALUES('jb_00000000000000000000000000000009',2,0,?1)",
        [br#"{"action":"setup","job_id":"jb_00000000000000000000000000000009","next_action":null,"reason":null,"state":"running"}"#.as_slice()]).unwrap();
    db.connection.execute("INSERT INTO resource_quarantine VALUES('opaque-live-resource','jb_00000000000000000000000000000009',0)", []).unwrap();
    let before = audit::audit_b1(&db.connection).unwrap();
    let typed: AppCommandV1 = parse_document(&a).unwrap();
    let result = db.dispatch_outcome(&typed, &a).unwrap();
    assert_eq!(result.disposition(), CommandDisposition::Replayed);
    assert!(matches!(
        result.response(),
        ReceiptSnapshot::B1RetainedV1(_)
    ));
    assert_eq!(result.original_response_bytes(), b);
    assert_eq!(audit::audit_b1(&db.connection).unwrap(), before);
    assert_eq!(
        db.replay(&typed.command_id.0, &a)
            .unwrap()
            .original_response_bytes(),
        b
    );
    assert_eq!(audit::audit_b1(&db.connection).unwrap(), before);
}
