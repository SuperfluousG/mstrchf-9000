use crate::boundary::{development_root, dispatch_json, replay_json};
use crate::receipt_fixture::*;
use mstrchf_contracts::{ErrorCode, bytes_digest};
use serde_json::{Value, json};
fn snapshot(s: &mstrchf_store::Store) -> Value {
    serde_json::to_value(s.snapshot().unwrap()).unwrap()
}
fn receipt(s: &mstrchf_store::Store, g: &str, input: &str, expected: &str, lookup: bool) {
    let a = literal(g, input);
    let got = if lookup {
        replay_json(s, &a)
    } else {
        dispatch_json(s, &a)
    }
    .unwrap();
    assert_eq!(serde_json::to_value(got).unwrap(), value(g, expected));
}
#[test]
fn current_applied_replayed_cas_and_reopen_are_bound_to_frozen_literals() {
    let p = seed("r1", "current-initial-snapshot.json", None);
    let s = open(&p);
    receipt(
        &s,
        "r1",
        "transport-current-save.txt",
        "current-applied-receipt.json",
        false,
    );
    assert_eq!(snapshot(&s), value("r1", "current-save-response.json"));
    dispatch_json(&s, &literal("r1", "current-privacy-command.json")).unwrap();
    receipt(
        &s,
        "r1",
        "transport-current-save.txt",
        "current-replayed-receipt.json",
        false,
    );
    let current = value("r1", "current-after-privacy-snapshot.json");
    assert_eq!(snapshot(&s), current);
    for (file, code) in [
        ("current-stale-command.json", ErrorCode::ContextStale),
        (
            "current-same-id-wrong-bytes.json",
            ErrorCode::ReplayConflict,
        ),
    ] {
        assert_eq!(
            dispatch_json(&s, &literal("r1", file)).unwrap_err().code,
            code
        );
        assert_eq!(snapshot(&s), current);
    }
    drop(s);
    assert_eq!(count(&p), 2);
    stored(
        &p,
        "r1",
        "current-save-command.json",
        "current-save-response.json",
        "B1Draft005",
    );
    let s = open(&p);
    receipt(
        &s,
        "r1",
        "transport-current-save.txt",
        "current-replayed-receipt.json",
        false,
    );
    assert_eq!(snapshot(&s), current);
}
#[test]
fn historical_save_uses_exact_lookup_only_and_never_becomes_current_state() {
    let p = seed(
        "r1",
        "old-save-response.json",
        Some(("old-save-command.json", "old-save-response.json")),
    );
    let s = open(&p);
    assert!(dispatch_json(&s, &literal("r1", "old-save-command.json")).is_err());
    receipt(
        &s,
        "r1",
        "old-save-command.json",
        "old-replayed-receipt.json",
        true,
    );
    let current = value("r1", "old-save-current-after-upgrade.json");
    assert_eq!(snapshot(&s), current);
    assert_eq!(
        replay_json(&s, &literal("r1", "old-save-spaced.txt"))
            .unwrap_err()
            .code,
        ErrorCode::ReplayConflict
    );
    let missing = replay_json(&s, &literal("r1", "old-missing-command.json")).unwrap_err();
    assert_eq!(missing.code, ErrorCode::MissingReference);
    assert_eq!(snapshot(&s), current);
    drop(s);
    assert_eq!(count(&p), 1);
    stored(
        &p,
        "r1",
        "old-save-command.json",
        "old-save-response.json",
        "B1RetainedV1",
    );
    let s = open(&p);
    receipt(
        &s,
        "r1",
        "old-save-command.json",
        "old-replayed-receipt.json",
        true,
    );
}
#[test]
fn current_dispatch_can_replay_old_privacy() {
    let p = seed(
        "r1",
        "old-privacy-response.json",
        Some(("old-privacy-command.json", "old-privacy-response.json")),
    );
    let s = open(&p);
    receipt(
        &s,
        "r1",
        "old-privacy-command.json",
        "old-privacy-via-current-receipt.json",
        false,
    );
    assert_eq!(snapshot(&s)["snapshot_revision"], "3");
}
#[test]
fn terminal_cancel_applies_once_replays_after_privacy_and_reopen() {
    let p = seed("r2", "cancel-current-initial-snapshot.json", None);
    let s = open(&p);
    receipt(
        &s,
        "r2",
        "cancel-current-transport.txt",
        "cancel-current-applied-receipt.json",
        false,
    );
    assert_eq!(snapshot(&s), value("r2", "cancel-current-response.json"));
    dispatch_json(
        &s,
        &literal("r2", "cancel-current-advance-privacy-command.json"),
    )
    .unwrap();
    receipt(
        &s,
        "r2",
        "cancel-current-transport.txt",
        "cancel-current-replayed-receipt.json",
        false,
    );
    assert_eq!(
        snapshot(&s),
        value("r2", "cancel-current-after-privacy-snapshot.json")
    );
    drop(s);
    assert_eq!(count(&p), 2);
    stored(
        &p,
        "r2",
        "cancel-current-command.json",
        "cancel-current-response.json",
        "B1Draft005",
    );
    let s = open(&p);
    receipt(
        &s,
        "r2",
        "cancel-current-transport.txt",
        "cancel-current-replayed-receipt.json",
        false,
    );
}
#[test]
fn historical_cancel_dispatch_and_lookup_retain_old_profile() {
    let p = seed(
        "r2",
        "cancel-old-live-before-upgrade.json",
        Some(("cancel-old-command.json", "cancel-old-response.json")),
    );
    for _ in 0..2 {
        let s = open(&p);
        receipt(
            &s,
            "r2",
            "cancel-old-current-transport.txt",
            "cancel-old-dispatch-receipt.json",
            false,
        );
        receipt(
            &s,
            "r2",
            "cancel-old-command.json",
            "cancel-old-lookup-receipt.json",
            true,
        );
        assert_eq!(
            snapshot(&s),
            value("r2", "cancel-old-current-after-upgrade.json")
        );
        drop(s);
        assert_eq!(count(&p), 1);
        stored(
            &p,
            "r2",
            "cancel-old-command.json",
            "cancel-old-response.json",
            "B1RetainedV1",
        );
    }
}
#[test]
fn rich_empty_and_quick_preferences_preserve_every_frozen_household_field() {
    for mode in ["empty", "quick"] {
        let p = seed("r2", "rich-initial-snapshot.json", None);
        let s = open(&p);
        receipt(
            &s,
            "r2",
            &format!("rich-{mode}-transport.txt"),
            &format!("rich-{mode}-applied-receipt.json"),
            false,
        );
        let want = value("r2", &format!("rich-{mode}-response.json"));
        assert_eq!(snapshot(&s), want);
        drop(s);
        assert_eq!(count(&p), 1);
        stored(
            &p,
            "r2",
            &format!("rich-{mode}-command.json"),
            &format!("rich-{mode}-response.json"),
            "B1Draft005",
        );
        let s = open(&p);
        assert_eq!(snapshot(&s), want);
        receipt(
            &s,
            "r2",
            &format!("rich-{mode}-transport.txt"),
            &format!("rich-{mode}-replayed-receipt.json"),
            false,
        );
    }
}
#[test]
fn malformed_routes_cannot_mutate_or_create_receipts() {
    let p = seed("r1", "current-initial-snapshot.json", None);
    let s = open(&p);
    let before = snapshot(&s);
    let mut extra = value("r1", "current-save-command.json");
    extra["profile"] = json!("B1RetainedV1");
    let deep = format!("{}0{}", "[".repeat(33), "]".repeat(33));
    for a in [
        literal("r1", "duplicate-command-id.txt"),
        extra.to_string(),
        deep,
        " ".repeat(8_388_609),
        "not json".into(),
    ] {
        assert!(dispatch_json(&s, &a).is_err());
        assert!(replay_json(&s, &a).is_err());
        assert_eq!(snapshot(&s), before);
    }
    drop(s);
    assert_eq!(count(&p), 0);
}
#[test]
fn sparse_issue_noop_and_missing_fields_preserve_ownership() {
    let p = seed("r1", "new-issue-response.json", None);
    let s = open(&p);
    for n in [
        "missing-restriction_issue_edits-command.json",
        "missing-unresolved_restrictions-command.json",
        "issue-unacknowledged-replace-command.json",
    ] {
        assert!(dispatch_json(&s, &literal("r1", n)).is_err());
    }
    dispatch_json(&s, &literal("r1", "issue-preserving-save-command.json")).unwrap();
    assert_eq!(
        snapshot(&s),
        value("r1", "issue-preserving-save-response.json")
    );
}
#[test]
fn fresh_005_root_preserves_sibling_sentinels_and_exact_metadata() {
    let base = root();
    let old = base.join("MSTRCHF9000-Development/loop001-u1-contracts003");
    let normal = base.join("MSTRCHF9000");
    for p in [&old, &normal] {
        std::fs::create_dir_all(p).unwrap();
        std::fs::write(p.join("sentinel"), b"unchanged").unwrap();
    }
    let p = development_root(&base);
    assert_eq!(
        p,
        base.join("MSTRCHF9000-Development/loop001-u1-contracts005")
    );
    let s = open(&p);
    let first = snapshot(&s);
    assert_eq!(first["snapshot_revision"], "1");
    assert_eq!(
        first["household_draft"]["unresolved_restrictions"],
        json!([])
    );
    drop(s);
    let c = rusqlite::Connection::open(p.join("planner.sqlite")).unwrap();
    assert_eq!(
        c.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))
            .unwrap(),
        2
    );
    assert_eq!(
        c.query_row(
            "SELECT count(*) FROM sqlite_schema WHERE type='table' AND name NOT LIKE 'sqlite_%'",
            [],
            |r| r.get::<_, i64>(0)
        )
        .unwrap(),
        8
    );
    for (v, sql) in [(1, SQL1), (2, SQL2)] {
        assert_eq!(
            c.query_row(
                "SELECT checksum FROM schema_migrations WHERE version=?1",
                [v],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
            bytes_digest(sql.as_bytes()).0
        );
    }
    assert_eq!(
        c.query_row("SELECT live_profile FROM store_format", [], |r| r
            .get::<_, String>(0))
            .unwrap(),
        "B1Draft005"
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM receipt_formats", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        0
    );
    drop(c);
    assert_eq!(snapshot(&open(&p)), first);
    for p in [&old, &normal] {
        assert_eq!(std::fs::read(p.join("sentinel")).unwrap(), b"unchanged");
        assert_eq!(std::fs::read_dir(p).unwrap().count(), 1);
    }
}
#[test]
fn sealed_corrupt_receipts_refuse_startup_without_repair() {
    for sql in [
        "UPDATE command_receipts SET response=x'7b7d'",
        "UPDATE command_receipts SET command_digest='bad'",
        "UPDATE command_receipts SET command_id='cm_00000000000000000000000000000009'",
        "UPDATE command_receipts SET snapshot_revision='1'",
        "UPDATE household SET household_id='hh_00000000000000000000000000000009'",
    ] {
        let p = seed(
            "r1",
            "old-save-response.json",
            Some(("old-save-command.json", "old-save-response.json")),
        );
        let path = p.join("planner.sqlite");
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute(sql, []).unwrap();
        drop(c);
        let before = std::fs::read(&path).unwrap();
        assert!(
            mstrchf_store::Store::open(mstrchf_store::StoreConfig { app_data_root: p }).is_err()
        );
        assert_eq!(std::fs::read(path).unwrap(), before);
    }
}
#[test]
fn projection_refuses_wrong_endpoint_bytes_and_current_identity() {
    use crate::receipt::{ShellRequestEncoding, project};
    let p = seed("r1", "current-initial-snapshot.json", None);
    let s = open(&p);
    let input = literal("r1", "transport-current-save.txt");
    let command: mstrchf_contracts::AppCommandV1 =
        mstrchf_contracts::parse_document(input.as_bytes()).unwrap();
    let canonical = mstrchf_contracts::canonical_bytes(&command).unwrap();
    let outcome = s.dispatch_ui(command.clone()).unwrap();
    for result in [
        project(
            &outcome,
            &input,
            ShellRequestEncoding::CurrentJson,
            b"wrong",
            Some(&command),
        ),
        project(
            &outcome,
            &input,
            ShellRequestEncoding::CanonicalReceiptJson,
            &canonical,
            None,
        ),
    ] {
        let error = result.unwrap_err();
        assert_eq!(error.code, ErrorCode::StorageFailure);
        assert_eq!(error.issues.0[0].path, "/shell/receipt");
    }
    let mut wrong = command;
    wrong.command_id.0 = "cm_00000000000000000000000000000099".into();
    assert!(
        project(
            &outcome,
            &input,
            ShellRequestEncoding::CurrentJson,
            &canonical,
            Some(&wrong)
        )
        .is_err()
    );
}
#[test]
fn invalid_existing_005_root_is_not_repaired_or_redirected() {
    let p = development_root(&root());
    std::fs::create_dir_all(&p).unwrap();
    let file = p.join("planner.sqlite");
    std::fs::write(&file, b"not a database").unwrap();
    assert!(mstrchf_store::Store::open(mstrchf_store::StoreConfig { app_data_root: p }).is_err());
    assert_eq!(std::fs::read(file).unwrap(), b"not a database");
}
#[test]
fn sealed_current_wrong_profile_and_response_digest_refuse_without_repair() {
    for sql in [
        "UPDATE receipt_formats SET profile='B1RetainedV1'",
        "UPDATE command_receipts SET response_digest='bad'",
    ] {
        let p = seed("r1", "current-initial-snapshot.json", None);
        let s = open(&p);
        dispatch_json(&s, &literal("r1", "transport-current-save.txt")).unwrap();
        drop(s);
        let path = p.join("planner.sqlite");
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute(sql, []).unwrap();
        drop(c);
        let before = std::fs::read(&path).unwrap();
        assert!(
            mstrchf_store::Store::open(mstrchf_store::StoreConfig { app_data_root: p }).is_err()
        );
        assert_eq!(std::fs::read(path).unwrap(), before);
    }
}
#[test]
fn issue_null_missing_and_over_limit_edits_cannot_reach_mutation() {
    let p = seed("r1", "new-issue-response.json", None);
    let s = open(&p);
    let before = snapshot(&s);
    let base = value("r1", "issue-preserving-save-command.json");
    let edit = json!({"kind":"resolve","data":{"diner_id":"dn_00000000000000000000000000000001"}});
    let mut null_issues = base.clone();
    null_issues["command"]["data"]["draft"]["unresolved_restrictions"] = Value::Null;
    let mut null_edits = base.clone();
    null_edits["command"]["data"]["restriction_issue_edits"] = Value::Null;
    let mut overflow = base.clone();
    overflow["command"]["data"]["restriction_issue_edits"] = json!(vec![edit.clone(); 65]);
    let mut duplicates = base.clone();
    duplicates["command"]["data"]["restriction_issue_edits"] = json!([edit.clone(), edit]);
    let mut wrong_kind = base.clone();
    wrong_kind["command"]["data"]["restriction_issue_edits"] =
        json!([{"kind":"invented","data":{}}]);
    for v in [null_issues, null_edits, overflow, duplicates, wrong_kind] {
        assert!(dispatch_json(&s, &v.to_string()).is_err());
        assert_eq!(snapshot(&s), before);
    }
    let mut removed = base;
    removed["command"]["data"]["draft"]["unresolved_restrictions"] = json!([]);
    let error = dispatch_json(&s, &removed.to_string()).unwrap_err();
    assert_eq!(error.code, ErrorCode::InvalidValue);
    assert_eq!(
        error.issues.0[0].path,
        "/command/data/restriction_issue_edits"
    );
    assert_eq!(snapshot(&s), before);
    drop(s);
    assert_eq!(count(&p), 0);
}
