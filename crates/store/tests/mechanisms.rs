#![cfg(test)]
// Included as a unit module: private mechanisms never enter downstream builds.
use super::*;
use rusqlite::{Connection, params};
use std::{
    io::{BufRead, Read, Write},
    path::Path,
    process::{Command, Stdio},
    time::Duration,
};
thread_local! {static FAULT:std::cell::Cell<&'static str>=const{std::cell::Cell::new("")};}
thread_local! {static RECOVERY_UNAVAILABLE:std::cell::Cell<bool>=const{std::cell::Cell::new(false)};}
pub(crate) fn recovery_unavailable() -> bool {
    RECOVERY_UNAVAILABLE.with(|f| f.replace(false))
}
pub(crate) fn fault(name: &str) -> bool {
    FAULT.with(|f| {
        if f.get() == name {
            f.set("");
            true
        } else {
            false
        }
    })
}
fn inject(name: &'static str) {
    FAULT.with(|f| f.set(name));
}

fn root(label: &str) -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "MSTRCHF B1 ü {label} {} {}",
        std::process::id(),
        records::random_id().unwrap()
    ));
    std::fs::create_dir_all(&p).unwrap();
    p
}
fn open(p: &Path) -> Store {
    Store::open(StoreConfig {
        app_data_root: p.into(),
    })
    .unwrap()
}
fn policy_command(id: u64, rev: u64, policy_rev: u64) -> AppCommandV1 {
    AppCommandV1 {
        schema_version: SchemaVersion,
        command_id: CommandId(format!("cm_{id:032x}")),
        expected_snapshot_revision: Revision(rev),
        command: AppCommand::SetPrivacy {
            policy: PrivacyPolicy {
                revision: Revision(policy_rev),
                use_ai_features: true,
                include_household_requirements: false,
                confirmed_at: Nullable(None),
            },
            expected_policy_revision: Revision(policy_rev),
        },
    }
}
pub(crate) fn barrier(name: &str) {
    if std::env::var("MSTRCHF_B1_BARRIER").ok().as_deref() == Some(name) {
        println!("\nBARRIER:{name}");
        std::io::stdout().flush().unwrap();
        let mut b = [0];
        std::io::stdin().read_exact(&mut b).unwrap();
    }
}
fn spawn(p: &Path, mode: &str, at: &str) -> std::process::Child {
    Command::new(std::env::current_exe().unwrap())
        .args(["--exact", "tests::child", "--nocapture", "--test-threads=1"])
        .env("MSTRCHF_B1_CHILD", mode)
        .env("MSTRCHF_B1_ROOT", p)
        .env("MSTRCHF_B1_BARRIER", at)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap()
}
fn wait_barrier(child: &mut std::process::Child, at: &str) -> std::thread::JoinHandle<()> {
    let output = child.stdout.take().unwrap();
    let (send, receive) = mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in std::io::BufReader::new(output).lines() {
            let line = line.unwrap();
            let _ = send.send(line);
        }
    });
    loop {
        match receive.recv_timeout(Duration::from_secs(30)) {
            Ok(line) if line.ends_with(&format!("BARRIER:{at}")) => break,
            Ok(_) => {}
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("barrier {at} missing: {e}");
            }
        }
    }
    reader
}
fn kill_at(p: &Path, mode: &str, at: &str) {
    let mut child = spawn(p, mode, at);
    let reader = wait_barrier(&mut child, at);
    child.kill().unwrap();
    let status = child.wait().unwrap();
    assert!(!status.success());
    reader.join().unwrap();
    println!("terminated barrier={at} pid={} exit={status}", child.id());
}
fn fresh_verify(p: &Path, rev: u64) {
    let child = spawn(p, &format!("verify-{rev}"), "");
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "fresh reopen: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
    print!("{}", String::from_utf8_lossy(&output.stdout));
}
#[test]
fn child() {
    let Ok(mode) = std::env::var("MSTRCHF_B1_CHILD") else {
        return;
    };
    let p = PathBuf::from(std::env::var_os("MSTRCHF_B1_ROOT").unwrap());
    if mode == "lock-refused" {
        assert_eq!(
            Store::open(StoreConfig { app_data_root: p })
                .unwrap_err()
                .code,
            ErrorCode::StorageFailure
        );
        return;
    }
    let store = open(&p);
    if mode == "lock" {
        barrier("lock_owned");
        return;
    }
    if mode == "command" {
        let s = store.dispatch_ui(policy_command(1, 1, 1)).unwrap();
        assert_eq!(s.snapshot_revision, Revision(2));
    }
    if mode == "verify-seeded" {
        let expected: Snapshot =
            parse_document(&std::fs::read(p.join("expected.json")).unwrap()).unwrap();
        let actual = store.snapshot().unwrap();
        assert_eq!(actual, expected);
        if actual.snapshot_revision == Revision(2) {
            assert_eq!(
                store.dispatch_ui(policy_command(1, 1, 1)).unwrap(),
                expected
            );
        }
        drop(store);
        let c = Connection::open(p.join("planner.sqlite")).unwrap();
        let changed = expected.snapshot_revision == Revision(2);
        assert_eq!(
            c.query_row("SELECT settings_revision FROM app_meta", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            if changed { "2" } else { "1" }
        );
        assert_eq!(
            c.query_row("SELECT quarantined FROM resource_quarantine", [], |r| r
                .get::<_, bool>(0))
                .unwrap(),
            changed
        );
        assert_eq!(
            c.query_row("SELECT count(*) FROM command_receipts", [], |r| r
                .get::<_, i64>(0))
                .unwrap(),
            i64::from(changed)
        );
        assert_eq!(
            c.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        assert!(
            !c.prepare("PRAGMA foreign_key_check")
                .unwrap()
                .exists([])
                .unwrap()
        );
        println!(
            "seeded fresh pid={} revision={} draft={} profile={} policy={} quarantine={changed} integrity=ok fk=0",
            std::process::id(),
            actual.snapshot_revision.0,
            actual.household_draft.revision.0,
            actual.profile.0.as_ref().unwrap().revision.0,
            actual.privacy.revision.0
        );
        return;
    }
    if let Some(rev) = mode.strip_prefix("verify-") {
        let rev: u64 = rev.parse().unwrap();
        let s = store.snapshot().unwrap();
        assert_eq!(s.snapshot_revision, Revision(rev));
        assert_eq!(s.privacy.revision, Revision(rev));
        assert_eq!(s.privacy.use_ai_features, rev == 2);
        assert_eq!(s.household_draft.revision, Revision(1));
        assert!(s.household_draft.connections.0.is_none());
        assert!(s.profile.0.is_none());
        if rev == 2 {
            let replay = store.dispatch_ui(policy_command(1, 1, 1)).unwrap();
            assert_eq!(replay, s);
        }
        drop(store);
        let c = Connection::open(p.join("planner.sqlite")).unwrap();
        let n: i64 = c
            .query_row("SELECT count(*) FROM command_receipts", [], |r| r.get(0))
            .unwrap();
        assert_eq!(n, if rev == 2 { 1 } else { 0 });
        assert_eq!(
            c.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
                .unwrap(),
            "ok"
        );
        assert!(
            !c.prepare("PRAGMA foreign_key_check")
                .unwrap()
                .exists([])
                .unwrap()
        );
        if rev == 2 {
            let bytes: Vec<u8> = c
                .query_row("SELECT response FROM command_receipts", [], |r| r.get(0))
                .unwrap();
            assert_eq!(bytes, canonical_bytes(&s).unwrap());
        }
        println!(
            "fresh pid={} revision={rev} policy={rev} draft=1 receipts={n} integrity=ok fk=0 digest={}",
            std::process::id(),
            canonical_digest(&s).unwrap().0
        );
    }
}
#[test]
fn fresh_database_has_real_policy_and_no_onboarding() {
    let p = root("fresh");
    let store = open(&p);
    let s = store.snapshot().unwrap();
    assert_eq!(s.snapshot_revision, Revision(1));
    assert_eq!(s.privacy.revision, Revision(1));
    assert!(!s.privacy.use_ai_features);
    assert!(s.household_draft.connections.0.is_none());
    assert!(s.profile.0.is_none());
    assert!(s.weeks.is_empty());
    assert!(
        Store::open(StoreConfig {
            app_data_root: p.clone()
        })
        .is_err()
    );
    drop(store);
    assert_eq!(open(&p).snapshot().unwrap(), s);
}
#[test]
fn existing_empty_foreign_corrupt_databases_are_unchanged() {
    for bytes in [vec![], b"unrelated bytes".to_vec()] {
        let p = root("foreign");
        std::fs::write(p.join("planner.sqlite"), &bytes).unwrap();
        assert!(
            Store::open(StoreConfig {
                app_data_root: p.clone()
            })
            .is_err()
        );
        assert_eq!(std::fs::read(p.join("planner.sqlite")).unwrap(), bytes);
    }
    let p = root("sqlite foreign");
    let c = Connection::open(p.join("planner.sqlite")).unwrap();
    c.execute_batch("CREATE TABLE unrelated(value TEXT);")
        .unwrap();
    drop(c);
    let before = std::fs::read(p.join("planner.sqlite")).unwrap();
    assert!(
        Store::open(StoreConfig {
            app_data_root: p.clone()
        })
        .is_err()
    );
    assert_eq!(std::fs::read(p.join("planner.sqlite")).unwrap(), before);
}
#[test]
fn sole_owner_process_lock_survives_filename() {
    let p = root("lock");
    let mut owner = spawn(&p, "lock", "lock_owned");
    let reader = wait_barrier(&mut owner, "lock_owned");
    let out = spawn(&p, "lock-refused", "").wait_with_output().unwrap();
    assert!(out.status.success());
    owner.kill().unwrap();
    assert!(!owner.wait().unwrap().success());
    reader.join().unwrap();
    assert!(p.join("store.lock").exists());
    fresh_verify(&p, 1);
}
#[test]
fn initialisation_crash_boundaries() {
    for at in [
        "init_before_record",
        "init_after_record",
        "init_before_commit",
        "init_after_commit",
        "init_after_publish",
    ] {
        let p = root(at);
        kill_at(&p, "init", at);
        fresh_verify(&p, 1);
    }
    fresh_verify(&root("init positive control"), 1);
}
#[test]
fn command_crash_boundaries_and_exact_fresh_replay() {
    for (at, rev) in [
        ("command_before_write", 1),
        ("command_after_write", 1),
        ("command_before_commit", 1),
        ("command_after_commit", 2),
    ] {
        let p = root(at);
        drop(open(&p));
        kill_at(&p, "command", at);
        fresh_verify(&p, rev);
    }
    let p = root("command positive control");
    drop(open(&p));
    let out = spawn(&p, "command", "").wait_with_output().unwrap();
    assert!(out.status.success());
    fresh_verify(&p, 2);
}
#[test]
fn migration_ledger_and_future_version_refuse_unchanged() {
    for sql in [
        "UPDATE schema_migrations SET checksum='altered'",
        "DELETE FROM schema_migrations",
        "PRAGMA user_version=2",
        "PRAGMA user_version=0",
        "CREATE TABLE unexpected(x TEXT)",
        "UPDATE schema_migrations SET version=2",
    ] {
        let p = root("ledger");
        drop(open(&p));
        let c = Connection::open(p.join("planner.sqlite")).unwrap();
        c.execute_batch(sql).unwrap();
        drop(c);
        let before = std::fs::read(p.join("planner.sqlite")).unwrap();
        assert!(
            Store::open(StoreConfig {
                app_data_root: p.clone()
            })
            .is_err()
        );
        assert_eq!(std::fs::read(p.join("planner.sqlite")).unwrap(), before);
    }
    let p = root("migration once");
    drop(open(&p));
    drop(open(&p));
    let c = Connection::open(p.join("planner.sqlite")).unwrap();
    let rows: i64 = c
        .query_row(
            "SELECT count(*) FROM schema_migrations WHERE checksum=?1",
            [bytes_digest(migrate::SQL.as_bytes()).0],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(rows, 1);
}

#[test]
fn schema_validation_rejects_inert_sqlite_like_ordinary_tables() {
    for name in [
        "sqliteX_control",
        "SQLITEX_control",
        "sqlite",
        "ordinary_control",
    ] {
        let p = root("schema name control");
        drop(open(&p));
        let c = Connection::open(p.join("planner.sqlite")).unwrap();
        // Fixed harmless names: an empty table only, with no triggers or data changes.
        c.execute_batch(&format!("CREATE TABLE {name}(value TEXT) STRICT;"))
            .unwrap();
        drop(c);
        let before = std::fs::read(p.join("planner.sqlite")).unwrap();
        let result = Store::open(StoreConfig {
            app_data_root: p.clone(),
        });
        assert!(
            matches!(result, Err(ref e) if e.code == ErrorCode::StorageFailure),
            "unexpected schema accepted: {name}"
        );
        assert_eq!(std::fs::read(p.join("planner.sqlite")).unwrap(), before);
    }
}
#[test]
fn full_u64_counters_overflow_and_sql_grammar() {
    for value in [1u64 << 63, u64::MAX] {
        let p = root("counter");
        drop(open(&p));
        let c = Connection::open(p.join("planner.sqlite")).unwrap();
        c.execute(
            "UPDATE app_meta SET snapshot_revision=?1",
            [value.to_string()],
        )
        .unwrap();
        drop(c);
        let store = open(&p);
        let before = store.snapshot().unwrap();
        assert_eq!(before.snapshot_revision, Revision(value));
        let outcome = store.dispatch_ui(policy_command(1, value, 1));
        if value == u64::MAX {
            assert_eq!(outcome.unwrap_err().code, ErrorCode::ArithmeticOverflow);
            assert_eq!(store.snapshot().unwrap(), before);
        } else {
            assert_eq!(outcome.unwrap().snapshot_revision, Revision(value + 1));
        }
    }
    let p = root("counter grammar");
    drop(open(&p));
    let c = Connection::open(p.join("planner.sqlite")).unwrap();
    for v in ["0", "01", "-1", "18446744073709551616", "1x", ""] {
        assert!(
            c.execute("UPDATE app_meta SET snapshot_revision=?1", [v])
                .is_err()
        );
    }
}
fn seed_job(c: &Connection, id: u64, state: JobState) {
    let job = JobSummary {
        job_id: JobId(format!("jb_{id:032x}")),
        action: Action::Setup,
        state,
        reason: Nullable(None),
        next_action: Nullable(None),
    };
    c.execute(
        "INSERT INTO jobs VALUES(?1,?2,?3,?4)",
        params![
            job.job_id.0,
            id as i64,
            records::terminal(state),
            canonical_bytes(&job).unwrap()
        ],
    )
    .unwrap();
}
#[test]
fn local_cancellation_preserves_terminal_and_quarantines_live() {
    for state in [
        JobState::Succeeded,
        JobState::Failed,
        JobState::NeedsUser,
        JobState::Cancelled,
        JobState::Running,
    ] {
        let p = root("cancel");
        drop(open(&p));
        let c = Connection::open(p.join("planner.sqlite")).unwrap();
        seed_job(&c, 1, state);
        c.execute(
            "INSERT INTO resource_quarantine VALUES('synthetic',?1,0)",
            [format!("jb_{:032x}", 1)],
        )
        .unwrap();
        drop(c);
        let store = open(&p);
        let before = store.snapshot().unwrap();
        let command = AppCommandV1 {
            schema_version: SchemaVersion,
            command_id: CommandId(format!("cm_{:032x}", 1)),
            expected_snapshot_revision: Revision(1),
            command: AppCommand::CancelJob {
                job_id: JobId(format!("jb_{:032x}", 1)),
            },
        };
        let after = store.dispatch_ui(command).unwrap();
        assert_eq!(after.snapshot_revision, Revision(2));
        if state == JobState::Running {
            assert_eq!(after.jobs[0].state, JobState::Cancelled);
            assert_eq!(
                after.jobs[0].next_action.0,
                Some(RequiredAction::StopWorker)
            );
        } else {
            assert_eq!(after.jobs, before.jobs);
        }
        drop(store);
        let c = Connection::open(p.join("planner.sqlite")).unwrap();
        let q: i64 = c
            .query_row("SELECT quarantined FROM resource_quarantine", [], |r| {
                r.get(0)
            })
            .unwrap();
        assert_eq!(q, if state == JobState::Running { 1 } else { 0 });
    }
}
#[test]
fn privacy_quarantines_live_without_releasing_resource() {
    let p = root("privacy live");
    drop(open(&p));
    let c = Connection::open(p.join("planner.sqlite")).unwrap();
    seed_job(&c, 1, JobState::Running);
    c.execute(
        "INSERT INTO resource_quarantine VALUES('synthetic',?1,0)",
        [format!("jb_{:032x}", 1)],
    )
    .unwrap();
    drop(c);
    let store = open(&p);
    let s = store.dispatch_ui(policy_command(1, 1, 1)).unwrap();
    assert_eq!(s.jobs[0].reason.0, Some(ErrorCode::PrivacyDenied));
    drop(store);
    let c = Connection::open(p.join("planner.sqlite")).unwrap();
    assert_eq!(
        c.query_row("SELECT quarantined FROM resource_quarantine", [], |r| r
            .get::<_, i64>(0))
            .unwrap(),
        1
    );
}
#[test]
fn identity_projection_and_foreign_key_fail_closed() {
    let p = root("projection");
    let mut db = database::Database::open(&p).unwrap();
    assert!(
        db.connection
            .execute(
                "INSERT INTO resource_quarantine VALUES('synthetic','missing',1)",
                []
            )
            .is_err()
    );
    db.connection
        .execute("UPDATE household SET draft_revision='2'", [])
        .unwrap();
    assert_eq!(db.snapshot().unwrap_err().code, ErrorCode::StorageFailure);
}
#[test]
fn sqlite_runtime_evidence() {
    let p = root("sqlite evidence");
    let db = database::Database::open(&p).unwrap();
    let c = &db.connection;
    assert!(rusqlite::version_number() >= 3_051_003);
    println!(
        "sqlite={} source_id={}",
        rusqlite::version(),
        c.query_row("SELECT sqlite_source_id()", [], |r| r.get::<_, String>(0))
            .unwrap()
    );
    let mut q = c.prepare("PRAGMA compile_options").unwrap();
    let options = q
        .query_map([], |r| r.get::<_, String>(0))
        .unwrap()
        .collect::<Result<Vec<_>, _>>()
        .unwrap();
    println!("options={options:?}");
    for (p, want) in [
        ("synchronous", 2),
        ("foreign_keys", 1),
        ("trusted_schema", 0),
        ("busy_timeout", 2000),
    ] {
        assert_eq!(
            c.pragma_query_value(None, p, |r| r.get::<_, i64>(0))
                .unwrap(),
            want
        );
        println!("{p}={want}");
    }
}

#[test]
fn commit_return_error_resolves_receipt_without_reapplying() {
    let p = root("uncertain");
    let mut db = database::Database::open(&p).unwrap();
    let command = policy_command(1, 1, 1);
    let bytes = canonical_bytes(&command).unwrap();
    inject("after_commit_error");
    let response = db.dispatch(&command, &bytes).unwrap();
    assert_eq!(db.snapshot().unwrap().snapshot_revision, Revision(2));
    assert_eq!(db.dispatch(&command, &bytes).unwrap(), response);
    assert!(
        !fault("after_commit_error"),
        "required failure was not reached"
    );
}
#[test]
fn rollback_rejections_preserve_all_records_and_receipts() {
    for failure in [
        "before_commit_error",
        "oversize_response",
        "receipt_conflict",
    ] {
        let p = root(failure);
        let mut db = database::Database::open(&p).unwrap();
        let before = db.snapshot().unwrap();
        let command = policy_command(1, 1, 1);
        inject(failure);
        let result = db.dispatch(&command, &canonical_bytes(&command).unwrap());
        assert!(result.is_err(), "{failure}");
        assert!(!fault(failure), "required failure was not reached");
        assert_eq!(db.snapshot().unwrap(), before);
        assert_eq!(
            db.connection
                .query_row("SELECT count(*) FROM command_receipts", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        assert_eq!(
            db.connection
                .query_row("SELECT settings_revision FROM app_meta", [], |r| r
                    .get::<_, String>(0))
                .unwrap(),
            "1"
        );
    }
}
#[test]
fn job_bounds_and_deterministic_terminal_projection() {
    let p = root("jobs bounds");
    let mut db = database::Database::open(&p).unwrap();
    let tx = db.connection.transaction().unwrap();
    for n in 1..=1025 {
        seed_job(&tx, n, JobState::Succeeded);
    }
    tx.commit().unwrap();
    let s = db.snapshot().unwrap();
    assert_eq!(s.jobs.len(), 1024);
    assert_eq!(s.jobs[0].job_id.0, format!("jb_{:032x}", 1025));
    assert_eq!(s.jobs[1023].job_id.0, format!("jb_{:032x}", 2));
    let tx = db.connection.transaction().unwrap();
    for n in 1026..=2050 {
        seed_job(&tx, n, JobState::Running);
    }
    tx.commit().unwrap();
    assert_eq!(db.snapshot().unwrap_err().code, ErrorCode::LimitExceeded);
}
#[test]
fn detached_snapshot_and_competing_cas_are_coherent() {
    let p = root("cas");
    let store = std::sync::Arc::new(open(&p));
    let old = store.snapshot().unwrap();
    let threads: Vec<_> = (1..=2)
        .map(|id| {
            let s = store.clone();
            std::thread::spawn(move || s.dispatch_ui(policy_command(id, 1, 1)))
        })
        .collect();
    let results: Vec<_> = threads.into_iter().map(|t| t.join().unwrap()).collect();
    assert_eq!(results.iter().filter(|r| r.is_ok()).count(), 1);
    assert_eq!(
        results
            .iter()
            .filter(|r| r.as_ref().is_err_and(|e| e.code == ErrorCode::ContextStale))
            .count(),
        1
    );
    assert_eq!(old.snapshot_revision, Revision(1));
    assert!(!old.privacy.use_ai_features);
    let new = store.snapshot().unwrap();
    assert_eq!(new.snapshot_revision, Revision(2));
    assert_eq!(new.privacy.revision, Revision(2));
}

#[test]
fn present_profile_privacy_is_synchronised_without_a_profile_producer() {
    let p = root("private profile");
    let mut db = database::Database::open(&p).unwrap();
    let initial = db.snapshot().unwrap();
    let mut profile: HouseholdProfile = parse_document(include_bytes!(
        "../../../tests/fixtures/contracts/household-profile.json"
    ))
    .unwrap();
    profile.household_id = initial.household_draft.household_id;
    profile.connections.privacy = initial.privacy;
    db.connection
        .execute(
            "UPDATE household SET profile=?1,profile_revision='1'",
            [canonical_bytes(&profile).unwrap()],
        )
        .unwrap();
    let command = policy_command(1, 1, 1);
    db.dispatch(&command, &canonical_bytes(&command).unwrap())
        .unwrap();
    let changed = db.snapshot().unwrap();
    let profile = changed.profile.0.unwrap();
    assert_eq!(profile.revision, Revision(2));
    assert_eq!(profile.connections.privacy, changed.privacy);
    assert!(changed.household_draft.connections.0.is_none());
}

#[test]
fn unknown_cancellation_and_foreign_draft_references_leave_no_receipt() {
    let p = root("rejected admission");
    let mut db = database::Database::open(&p).unwrap();
    let before = db.snapshot().unwrap();
    let mut command = policy_command(1, 1, 1);
    command.command = AppCommand::CancelJob {
        job_id: JobId(format!("jb_{:032x}", 1)),
    };
    assert_eq!(
        db.dispatch(&command, &canonical_bytes(&command).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    );
    let profile: HouseholdProfile = parse_document(include_bytes!(
        "../../../tests/fixtures/contracts/household-profile.json"
    ))
    .unwrap();
    let mut draft = before.household_draft.clone();
    draft.usual_week = Nullable(Some(profile.usual_week));
    draft.completed_stages = BoundedVec(vec![HouseholdStage::UsualWeek]);
    command.command = AppCommand::SaveHouseholdDraft {
        restriction_issue_edits: BoundedVec(vec![]),
        draft,
    };
    assert_eq!(
        db.dispatch(&command, &canonical_bytes(&command).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::ForeignReference
    );
    assert_eq!(db.snapshot().unwrap(), before);
    assert_eq!(
        db.connection
            .query_row("SELECT count(*) FROM command_receipts", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        0
    );
}
#[test]
fn sqlite_read_transaction_holds_coherent_old_state() {
    let p = root("read snapshot");
    let store = open(&p);
    let mut reader = Connection::open_with_flags(
        p.join("planner.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let tx = reader.transaction().unwrap();
    let old = snapshot::load(&tx).unwrap();
    let updated = store.dispatch_ui(policy_command(1, 1, 1)).unwrap();
    assert_eq!(snapshot::load(&tx).unwrap(), old);
    tx.commit().unwrap();
    assert_eq!(snapshot::read(&mut reader).unwrap(), updated);
}
#[test]
fn interrupted_initialisation_cannot_adopt_replaced_stage_or_overwrite_live() {
    let p = root("replaced stage");
    kill_at(&p, "init", "init_after_record");
    let marker: serde_json::Value =
        serde_json::from_slice(&std::fs::read(p.join("initialisation.json")).unwrap()).unwrap();
    let stage = p.join(marker["stage"].as_str().unwrap());
    std::fs::rename(&stage, p.join("preserved-original-stage")).unwrap();
    std::fs::write(&stage, []).unwrap();
    assert_eq!(
        Store::open(StoreConfig {
            app_data_root: p.clone()
        })
        .unwrap_err()
        .code,
        ErrorCode::StorageFailure
    );
    assert!(!p.join("planner.sqlite").exists());
    let from = p.join("source");
    let to = p.join("existing");
    std::fs::write(&from, b"new").unwrap();
    std::fs::write(&to, b"old").unwrap();
    assert!(windows_fs::publish(&from, &to).is_err());
    assert_eq!(std::fs::read(&to).unwrap(), b"old");
    assert_eq!(std::fs::read(&from).unwrap(), b"new");
}

#[test]
fn unreadable_uncertain_commit_blocks_mutation_until_reopen() {
    let p = root("uncertain blocked");
    let mut db = database::Database::open(&p).unwrap();
    let command = policy_command(1, 1, 1);
    let bytes = canonical_bytes(&command).unwrap();
    inject("after_commit_error");
    RECOVERY_UNAVAILABLE.with(|f| f.set(true));
    assert_eq!(
        db.dispatch(&command, &bytes).unwrap_err().code,
        ErrorCode::StorageFailure
    );
    assert!(
        !recovery_unavailable(),
        "recovery failure point not reached"
    );
    let next = policy_command(2, 2, 2);
    assert_eq!(
        db.dispatch(&next, &canonical_bytes(&next).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::StorageFailure
    );
    drop(db);
    fresh_verify(&p, 2);
}

#[test]
fn bounded_queue_refuses_overload_and_recovers_without_mutation() {
    let p = root("queue");
    let store = open(&p);
    let (ready, waiting) = mpsc::sync_channel(1);
    let (resume, blocked) = mpsc::channel();
    store.sender.send(Request::Pause(ready, blocked)).unwrap();
    waiting.recv_timeout(Duration::from_secs(5)).unwrap();
    let mut replies = Vec::new();
    for _ in 0..64 {
        let (send, recv) = mpsc::sync_channel(1);
        store.sender.try_send(Request::Snapshot(send)).unwrap();
        replies.push(recv);
    }
    let outcome = store.dispatch_ui(policy_command(1, 1, 1));
    resume.send(()).unwrap();
    assert_eq!(outcome.unwrap_err().code, ErrorCode::StorageFailure);
    for reply in replies {
        assert_eq!(
            reply
                .recv_timeout(Duration::from_secs(5))
                .unwrap()
                .unwrap()
                .snapshot_revision,
            Revision(1)
        );
    }
    assert_eq!(store.snapshot().unwrap().snapshot_revision, Revision(1));
    assert_eq!(
        store
            .dispatch_ui(policy_command(1, 1, 1))
            .unwrap()
            .snapshot_revision,
        Revision(2)
    );
}
#[test]
fn exact_response_bytes_survive_later_commands_and_reopen() {
    let p = root("receipt bytes");
    let mut db = database::Database::open(&p).unwrap();
    let original = policy_command(1, 1, 1);
    let bytes = canonical_bytes(&original).unwrap();
    let response = db.dispatch(&original, &bytes).unwrap();
    let next = policy_command(2, 2, 2);
    db.dispatch(&next, &canonical_bytes(&next).unwrap())
        .unwrap();
    assert_eq!(db.dispatch(&original, &bytes).unwrap(), response);
    assert_eq!(db.snapshot().unwrap().snapshot_revision, Revision(3));
    drop(db);
    let mut db = database::Database::open(&p).unwrap();
    assert_eq!(db.dispatch(&original, &bytes).unwrap(), response);
    assert_eq!(db.snapshot().unwrap().snapshot_revision, Revision(3));
    assert_eq!(
        db.connection
            .query_row("SELECT count(*) FROM command_receipts", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        2
    );
}

#[test]
fn privacy_copy_and_resource_crashes_are_wholly_old_or_new() {
    for (at, changed) in [
        ("command_before_write", false),
        ("command_after_write", false),
        ("command_before_commit", false),
        ("command_after_commit", true),
        ("", true),
    ] {
        let p = root("privacy copies crash");
        let mut db = database::Database::open(&p).unwrap();
        let initial = db.snapshot().unwrap();
        let mut profile: HouseholdProfile = parse_document(include_bytes!(
            "../../../tests/fixtures/contracts/household-profile.json"
        ))
        .unwrap();
        profile.household_id = initial.household_draft.household_id.clone();
        profile.connections.privacy = initial.privacy.clone();
        let mut draft = initial.household_draft;
        draft.connections = Nullable(Some(profile.connections.clone()));
        draft.completed_stages = BoundedVec(vec![HouseholdStage::Connections]);
        db.connection
            .execute(
                "UPDATE household SET draft=?1,profile=?2,profile_revision='1'",
                params![
                    canonical_bytes(&draft).unwrap(),
                    canonical_bytes(&profile).unwrap()
                ],
            )
            .unwrap();
        seed_job(&db.connection, 1, JobState::Running);
        db.connection
            .execute(
                "INSERT INTO resource_quarantine VALUES('synthetic',?1,0)",
                [format!("jb_{:032x}", 1)],
            )
            .unwrap();
        let mut expected = db.snapshot().unwrap();
        drop(db);
        if changed {
            expected.snapshot_revision = Revision(2);
            expected.privacy.revision = Revision(2);
            expected.privacy.use_ai_features = true;
            expected.household_draft.revision = Revision(2);
            expected
                .household_draft
                .connections
                .0
                .as_mut()
                .unwrap()
                .privacy = expected.privacy.clone();
            let profile = expected.profile.0.as_mut().unwrap();
            profile.revision = Revision(2);
            profile.connections.privacy = expected.privacy.clone();
            expected.jobs.0[0].state = JobState::NeedsUser;
            expected.jobs.0[0].reason = Nullable(Some(ErrorCode::PrivacyDenied));
            expected.jobs.0[0].next_action = Nullable(Some(RequiredAction::StopWorker));
        }
        std::fs::write(p.join("expected.json"), canonical_bytes(&expected).unwrap()).unwrap();
        if at.is_empty() {
            assert!(
                spawn(&p, "command", "")
                    .wait_with_output()
                    .unwrap()
                    .status
                    .success()
            );
        } else {
            kill_at(&p, "command", at);
        }
        let output = spawn(&p, "verify-seeded", "").wait_with_output().unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(String::from_utf8_lossy(&output.stdout).contains("1 passed"));
        print!("{}", String::from_utf8_lossy(&output.stdout));
    }
}

#[test]
fn cancellation_validates_job_projection_outside_visible_history() {
    let p = root("hidden projection");
    let mut db = database::Database::open(&p).unwrap();
    let tx = db.connection.transaction().unwrap();
    seed_job(&tx, 1, JobState::Running);
    tx.execute("UPDATE jobs SET terminal=1 WHERE creation_sequence=1", [])
        .unwrap();
    for n in 2..=1025 {
        seed_job(&tx, n, JobState::Succeeded);
    }
    tx.commit().unwrap();
    let before = db.snapshot().unwrap();
    assert!(
        !before
            .jobs
            .iter()
            .any(|j| j.job_id.0 == format!("jb_{:032x}", 1))
    );
    let mut command = policy_command(1, 1, 1);
    command.command = AppCommand::CancelJob {
        job_id: JobId(format!("jb_{:032x}", 1)),
    };
    assert_eq!(
        db.dispatch(&command, &canonical_bytes(&command).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::StorageFailure
    );
    assert_eq!(db.snapshot().unwrap(), before);
}

// Ruling005 wire-only boundary. This proves refusal, not issue transition support.
#[test]
fn ruling005_issue_inputs_refuse_without_mutation_or_receipt() {
    for (old_issue, new_issue, submitted_edit) in [
        (false, true, false),
        (false, false, true),
        (true, false, false),
        (true, true, false),
        (true, false, true),
    ] {
        let p = root("ruling005 unsupported setup");
        let mut db = database::Database::open(&p).unwrap();
        let mut seeded = db.snapshot().unwrap().household_draft;
        let profile: HouseholdProfile = parse_document(include_bytes!(
            "../../../tests/fixtures/contracts/household-profile-confirmed-none.json"
        ))
        .unwrap();
        let mut diner = profile.diners[0].clone();
        diner.restrictions = RestrictionDeclaration::Unconfirmed {};
        let issue = SetupRestrictionIssue {
            diner_id: diner.diner_id.clone(),
            supplied_text: Text("synthetic unresolved marker".into()),
        };
        seeded.diners = BoundedVec(vec![diner]);
        if old_issue {
            seeded.unresolved_restrictions = BoundedVec(vec![issue.clone()]);
        }
        // Private synthetic seed into a fresh test database, never production admission.
        db.connection
            .execute(
                "UPDATE household SET draft=?1 WHERE singleton=1",
                params![canonical_bytes(&seeded).unwrap()],
            )
            .unwrap();
        let before = db.snapshot().unwrap();
        let before_body: Vec<u8> = db
            .connection
            .query_row("SELECT draft FROM household", [], |r| r.get(0))
            .unwrap();
        let before_meta: (String,String,String) = db.connection.query_row(
            "SELECT snapshot_revision,settings_revision,privacy_revision FROM app_meta CROSS JOIN household", [],
            |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
        let mut proposed = seeded;
        proposed.unresolved_restrictions = BoundedVec(if new_issue {
            vec![issue.clone()]
        } else {
            vec![]
        });
        let mut command = policy_command(1, 1, 1);
        command.command = AppCommand::SaveHouseholdDraft {
            draft: proposed,
            restriction_issue_edits: BoundedVec(if submitted_edit {
                vec![SetupRestrictionEdit::Resolve {
                    diner_id: issue.diner_id,
                }]
            } else {
                vec![]
            }),
        };
        command.validate_shape().unwrap();
        let error = db
            .dispatch(&command, &canonical_bytes(&command).unwrap())
            .unwrap_err();
        assert_eq!(error.code, ErrorCode::UnsupportedCapability);
        assert!(
            !String::from_utf8(canonical_bytes(&error).unwrap())
                .unwrap()
                .contains("synthetic unresolved marker")
        );
        assert_eq!(db.snapshot().unwrap(), before);
        let body: Vec<u8> = db
            .connection
            .query_row("SELECT draft FROM household", [], |r| r.get(0))
            .unwrap();
        assert_eq!(body, before_body);
        let meta: (String,String,String) = db.connection.query_row(
            "SELECT snapshot_revision,settings_revision,privacy_revision FROM app_meta CROSS JOIN household", [],
            |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).unwrap();
        assert_eq!(meta, before_meta);
        assert_eq!(
            db.connection
                .query_row("SELECT count(*) FROM command_receipts", [], |r| r
                    .get::<_, i64>(0))
                .unwrap(),
            0
        );
        drop(db);
        let mut reopened = database::Database::open(&p).unwrap();
        assert_eq!(reopened.snapshot().unwrap(), before);
    }
}
