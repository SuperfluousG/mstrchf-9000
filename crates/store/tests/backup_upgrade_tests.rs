//! Synthetic, private backup/startup fixtures. Never compiled into a consumer build.
use super::*;
use std::path::{Path, PathBuf};

thread_local! { static FAULT: std::cell::Cell<&'static str> = const { std::cell::Cell::new("") }; }
thread_local! { static EXECUTIONS: std::cell::Cell<u32> = const { std::cell::Cell::new(0) }; }
pub(crate) fn count_migration() {
    EXECUTIONS.with(|n| n.set(n.get() + 1));
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
pub(crate) fn barrier(name: &str) {
    use std::io::{Read, Write};
    if std::env::var("MSTRCHF_BU_BARRIER").ok().as_deref() == Some(name) {
        println!("BARRIER:{name}");
        std::io::stdout().flush().unwrap();
        std::io::stdin().read_exact(&mut [0]).unwrap();
    }
}

fn root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "MSTRCHF BU ü {label} {}",
        records::random_id().unwrap()
    ))
}
fn open(path: &Path) -> Result<Store, DomainError> {
    Store::open(StoreConfig {
        app_data_root: path.into(),
    })
}
#[test]
fn startup_existing_journal_refuses_before_sqlite() {
    let p = root("journal refusal");
    drop(open(&p).unwrap());
    let before = std::fs::read(p.join("planner.sqlite")).unwrap();
    std::fs::write(p.join("planner.sqlite-journal"), []).unwrap();
    assert!(
        open(&p).is_err(),
        "any existing journal must refuse before SQLite"
    );
    assert_eq!(std::fs::read(p.join("planner.sqlite")).unwrap(), before);
    assert!(p.join("planner.sqlite-journal").exists());
}

fn policy(id: u64, rev: u64, enabled: bool) -> AppCommandV1 {
    AppCommandV1 {
        schema_version: SchemaVersion,
        command_id: CommandId(format!("cm_{id:032x}")),
        expected_snapshot_revision: Revision(rev),
        command: AppCommand::SetPrivacy {
            policy: PrivacyPolicy {
                revision: Revision(rev),
                use_ai_features: enabled,
                include_household_requirements: false,
                confirmed_at: Nullable(None),
            },
            expected_policy_revision: Revision(rev),
        },
    }
}
fn two_commands(db: &mut database::Database) -> Vec<Vec<u8>> {
    [(0x11, 1, true), (0x12, 2, false)]
        .into_iter()
        .map(|(id, rev, on)| {
            let cmd = policy(id, rev, on);
            db.dispatch(&cmd, &canonical_bytes(&cmd).unwrap()).unwrap()
        })
        .collect()
}
#[test]
fn backup_live_wal_is_complete() {
    let p = root("online live WAL");
    let mut db = database::Database::open(&p).unwrap();
    db.connection
        .pragma_update(None, "wal_autocheckpoint", 0)
        .unwrap();
    let responses = two_commands(&mut db);
    assert!(
        std::fs::metadata(p.join("planner.sqlite-wal"))
            .unwrap()
            .len()
            > 32
    );
    let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::B1).unwrap();
    let receipt = m.backup().unwrap();
    let c = rusqlite::Connection::open_with_flags(
        receipt.path(),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(snapshot::load(&c).unwrap().snapshot_revision, Revision(3));
    for (index, id) in [0x11, 0x12].into_iter().enumerate() {
        let bytes: Vec<u8> = c
            .query_row(
                "SELECT response FROM command_receipts WHERE command_id=?1",
                [format!("cm_{id:032x}")],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(bytes, responses[index]);
    }
    c.close().unwrap();
    assert_eq!(
        m.database
            .connection
            .pragma_query_value(None, "journal_mode", |r| r.get::<_, String>(0))
            .unwrap(),
        "wal"
    );
}

fn fixture(p: &Path) {
    let mut db = database::Database::open(p).unwrap();
    two_commands(&mut db);
    db.connection
        .execute_batch(audit::FIXTURE_PROBE_SQL)
        .unwrap();
    db.connection
        .execute("INSERT INTO fixture_probe VALUES(1,7)", [])
        .unwrap();
    db.connection
        .execute(
            "UPDATE schema_migrations SET checksum=?1 WHERE version=1",
            [bytes_digest(audit::fixture_source_sql().as_bytes()).0],
        )
        .unwrap();
    audit::audit_fixture(&db.connection, audit::FixtureVersion::V1).unwrap();
}
fn oracle(c: &rusqlite::Connection, want: u32) {
    assert_eq!(
        c.pragma_query_value(None, "user_version", |r| r.get::<_, u32>(0))
            .unwrap(),
        want
    );
    assert_eq!(
        c.query_row("SELECT value FROM fixture_probe", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        if want == 1 { 7 } else { 11 }
    );
    assert_eq!(
        c.query_row("SELECT count(*) FROM schema_migrations", [], |r| r
            .get::<_, u32>(0))
            .unwrap(),
        want
    );
    let added: u32 = c
        .query_row(
            "SELECT count(*) FROM sqlite_schema WHERE name='fixture_added'",
            [],
            |r| r.get(0),
        )
        .unwrap();
    assert_eq!(added, if want == 1 { 0 } else { 1 });
    if want == 2 {
        assert_eq!(
            c.query_row("SELECT note FROM fixture_added WHERE id=1", [], |r| r
                .get::<_, String>(0))
                .unwrap(),
            "upgrade-complete"
        );
    }
    assert_eq!(snapshot::load(c).unwrap().snapshot_revision, Revision(3));
    let old:Vec<u8> = c.query_row("SELECT response FROM command_receipts WHERE command_id='cm_00000000000000000000000000000011'",[],|r|r.get(0)).unwrap();
    assert_eq!(
        parse_document::<Snapshot>(&old).unwrap().snapshot_revision,
        Revision(2)
    );
}
#[test]
fn upgrade_fixture_commits_once() {
    let p = root("fixture commit");
    fixture(&p);
    let mut db = database::Database::open_catalogue(&p, startup::Catalogue::Fixture).unwrap();
    let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::Fixture).unwrap();
    let receipt = m.backup().unwrap();
    let backup_path = receipt.path().to_owned();
    assert_eq!(
        upgrade_fixture::apply(&mut m, receipt).unwrap(),
        upgrade_fixture::UpgradeOutcome::Committed
    );
    oracle(&m.database.connection, 2);
    let saved = rusqlite::Connection::open_with_flags(
        backup_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    oracle(&saved, 1);
    saved.close().unwrap();
    drop(m);
    drop(db);
    let db = database::Database::open_catalogue(&p, startup::Catalogue::Fixture).unwrap();
    oracle(&db.connection, 2);
}
#[test]
fn upgrade_fixture_rolls_back_all_effects() {
    for at in [
        "ug_after_ddl",
        "ug_after_insert",
        "ug_after_data",
        "ug_after_ledger",
        "ug_after_version",
        "ug_before_commit",
    ] {
        let p = root(at);
        fixture(&p);
        let mut db = database::Database::open_catalogue(&p, startup::Catalogue::Fixture).unwrap();
        let before = audit::audit_fixture(&db.connection, audit::FixtureVersion::V1).unwrap();
        let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::Fixture).unwrap();
        let receipt = m.backup().unwrap();
        FAULT.with(|f| f.set(at));
        assert!(upgrade_fixture::apply(&mut m, receipt).is_err());
        assert_eq!(
            audit::audit_fixture(&m.database.connection, audit::FixtureVersion::V1).unwrap(),
            before
        );
        oracle(&m.database.connection, 1);
    }
}
#[test]
fn upgrade_uncertain_commit_reopens_exact_state() {
    for (at, want) in [("commit_before_error", 1), ("commit_after_error", 2)] {
        let p = root(at);
        fixture(&p);
        let mut db = database::Database::open_catalogue(&p, startup::Catalogue::Fixture).unwrap();
        let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::Fixture).unwrap();
        let receipt = m.backup().unwrap();
        FAULT.with(|f| f.set(at));
        let result = upgrade_fixture::apply(&mut m, receipt);
        if want == 1 {
            assert!(result.is_err());
        } else {
            assert_eq!(
                result.unwrap(),
                upgrade_fixture::UpgradeOutcome::CommittedAfterRecovery
            );
        }
        oracle(&m.database.connection, want);
    }
}

fn spawn(p: &Path, mode: &str, at: &str) -> std::process::Child {
    use std::process::{Command, Stdio};
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "backup_upgrade_tests::child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("MSTRCHF_BU_ROOT", p)
        .env("MSTRCHF_BU_MODE", mode)
        .env("MSTRCHF_BU_BARRIER", at)
        .env("MSTRCHF_VFS_TRACE_CHILD", "1")
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::inherit())
        .spawn()
        .unwrap()
}
fn kill_at(p: &Path, mode: &str, at: &str) {
    use std::io::BufRead;
    let mut child = spawn(p, mode, at);
    let pid = child.id();
    let stdout = child.stdout.take().unwrap();
    let (send, recv) = std::sync::mpsc::channel();
    let reader = std::thread::spawn(move || {
        for line in std::io::BufReader::new(stdout).lines() {
            let _ = send.send(line.unwrap());
        }
    });
    loop {
        match recv.recv_timeout(std::time::Duration::from_secs(30)) {
            Ok(line) if line.ends_with(&format!("BARRIER:{at}")) => break,
            Ok(_) => {}
            Err(e) => {
                let _ = child.kill();
                let _ = child.wait();
                panic!("barrier {at} missing: {e}");
            }
        }
    }
    child.kill().unwrap();
    let status = child.wait().unwrap();
    assert!(!status.success());
    reader.join().unwrap();
    println!("KILL barrier={at} child={pid} status={status}");
}
fn verify_child(p: &Path, mode: &str) {
    let child = spawn(p, mode, "");
    let pid = child.id();
    let output = child.wait_with_output().unwrap();
    assert!(
        output.status.success(),
        "child {mode}: {}",
        String::from_utf8_lossy(&output.stdout)
    );
    let text = String::from_utf8_lossy(&output.stdout);
    assert!(text.contains("1 passed"));
    println!("VERIFY child={pid} mode={mode}\n{text}");
}
#[test]
fn child() {
    let Ok(mode) = std::env::var("MSTRCHF_BU_MODE") else {
        return;
    };
    let p = PathBuf::from(std::env::var_os("MSTRCHF_BU_ROOT").unwrap());
    if mode == "wal-source" || mode == "wal-future" {
        let mut db = database::Database::open(&p).unwrap();
        db.connection
            .pragma_update(None, "wal_autocheckpoint", 0)
            .unwrap();
        two_commands(&mut db);
        if mode == "wal-future" {
            db.connection
                .pragma_update(None, "user_version", 999)
                .unwrap();
        }
        barrier("source_ready");
        return;
    }
    if mode == "refuse-without-sqlite" || mode == "refuse-future-wal" {
        let root = custody::TrustedStoreRoot::open_or_create(&p).unwrap();
        let _lock = root.acquire_store_lock().unwrap();
        let trace = vfs_trace::TraceGuard::install_for_current_child().unwrap();
        let result = startup::inspect_for_test(&root, &_lock, startup::Catalogue::B1);
        if mode == "refuse-without-sqlite" {
            assert!(result.is_err());
            assert!(trace.snapshot().is_empty());
        } else {
            assert!(matches!(result,Err(e) if e.code==ErrorCode::UnsupportedVersion));
            trace
                .assert_held_wal_readonly(&p.join("planner.sqlite"))
                .unwrap();
        }
        return;
    }
    if mode == "recognise" {
        let root = custody::TrustedStoreRoot::open_or_create(&p).unwrap();
        let _lock = root.acquire_store_lock().unwrap();
        let trace = vfs_trace::TraceGuard::install_for_current_child().unwrap();
        let (state, copy) =
            startup::inspect_for_test(&root, &_lock, startup::Catalogue::B1).unwrap();
        assert_eq!(state.counters.snapshot, Revision(3));
        if let Some(copy) = copy {
            trace.assert_no_opens(&p.join("planner.sqlite")).unwrap();
            trace.assert_no_sidecar_opens(&copy).unwrap();
        } else {
            trace
                .assert_held_wal_readonly(&p.join("planner.sqlite"))
                .unwrap();
            println!(
                "read-only mapping observation: {:?}",
                trace.assert_readonly_shm_mapping(&p.join("planner.sqlite"))
            );
        }
        println!("VFS TRACE {:?}", trace.snapshot());
        drop(trace);
        drop(_lock);
        drop(root);
        let mut db = database::Database::open(&p).unwrap();
        assert_eq!(db.snapshot().unwrap().snapshot_revision, Revision(3));
        let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::B1).unwrap();
        let _receipt = m.backup().unwrap();
        return;
    }
    if mode == "asset-backup" {
        let mut db =
            database::Database::open_catalogue(&p, startup::Catalogue::AssetFixture).unwrap();
        assert_eq!(db.snapshot().unwrap().snapshot_revision, Revision(3));
        let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::AssetFixture).unwrap();
        let receipt = m.backup().unwrap();
        let name = "assets/7d51503664b77f2147ce760635926a8a4ec49a926b6115a3fed5b4feed3002f4.png";
        assert_eq!(
            std::fs::read(receipt.path().parent().unwrap().join(name)).unwrap(),
            b"backup-asset-fixture\n"
        );
        return;
    }
    let mut db = database::Database::open_catalogue(&p, startup::Catalogue::Fixture).unwrap();
    if mode == "verify-old" {
        oracle(&db.connection, 1);
        return;
    }
    if mode == "verify-new" {
        oracle(&db.connection, 2);
        return;
    }
    let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::Fixture).unwrap();
    let receipt = m.backup().unwrap();
    if mode == "backup" {
        return;
    }
    upgrade_fixture::apply(&mut m, receipt).unwrap();
    oracle(&m.database.connection, 2);
}
#[test]
fn startup_closed_and_recognised_crash_positives() {
    let p = root("closed positive");
    let mut db = database::Database::open(&p).unwrap();
    two_commands(&mut db);
    drop(db);
    for sidecar in [
        "planner.sqlite-wal",
        "planner.sqlite-shm",
        "planner.sqlite-journal",
    ] {
        assert!(!p.join(sidecar).exists());
    }
    verify_child(&p, "recognise");
    let p = root("crash positive");
    kill_at(&p, "wal-source", "source_ready");
    assert!(p.join("planner.sqlite-wal").exists());
    assert!(p.join("planner.sqlite-shm").exists());
    verify_child(&p, "recognise");
}

#[test]
fn backup_and_upgrade_kill_matrix() {
    // Asset barrier belongs to its separate closed asset fixture (not a fake B1 hook).
    const CASES: &[(&str, &str, u32)] = &[
        ("bu_after_directory", "backup", 1),
        ("bu_after_reserve", "backup", 1),
        ("bu_after_first_step", "backup", 1),
        ("bu_after_done", "backup", 1),
        ("bu_after_database_flush", "backup", 1),
        ("bu_after_manifest_write", "backup", 1),
        ("bu_after_verified", "backup", 1),
        ("ug_after_begin", "upgrade", 1),
        ("ug_after_ddl", "upgrade", 1),
        ("ug_after_data", "upgrade", 1),
        ("ug_after_ledger", "upgrade", 1),
        ("ug_before_commit", "upgrade", 1),
        ("ug_after_commit", "upgrade", 2),
        ("ug_before_ready", "upgrade", 2),
    ];
    for &(at, mode, want) in CASES {
        let p = root(at);
        fixture(&p);
        if at == "bu_after_first_step" {
            let mut c = rusqlite::Connection::open(p.join("planner.sqlite")).unwrap();
            let tx = c.transaction().unwrap();
            for id in 1..=4000 {
                let job = JobSummary {
                    job_id: JobId(format!("jb_{id:032x}")),
                    action: Action::Setup,
                    state: JobState::Succeeded,
                    reason: Nullable(None),
                    next_action: Nullable(None),
                };
                tx.execute(
                    "INSERT INTO jobs VALUES(?1,?2,1,?3)",
                    rusqlite::params![job.job_id.0, id, canonical_bytes(&job).unwrap()],
                )
                .unwrap();
            }
            tx.commit().unwrap();
            c.close().unwrap();
        }
        let source = rusqlite::Connection::open_with_flags(
            p.join("planner.sqlite"),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        let expected = audit::audit_fixture(&source, audit::FixtureVersion::V1).unwrap();
        source.close().unwrap();
        kill_at(&p, mode, at);
        let evidence = barrier_evidence::capture(&p, at, startup::Catalogue::Fixture, &expected);
        let physical = [
            "planner.sqlite-wal",
            "planner.sqlite-shm",
            "planner.sqlite-journal",
        ]
        .map(|n| p.join(n).exists());
        println!("CLASS barrier={at} WAL/SHM/journal={physical:?} expected={want}");
        verify_child(
            &p,
            if want == 1 {
                "verify-old"
            } else {
                "verify-new"
            },
        );
        if want == 1 {
            verify_child(&p, "upgrade");
        } else {
            verify_child(&p, "verify-new");
        }
        evidence.assert_preserved();
    }
    // Positive no-kill child paths, distinct from the killed and verifier processes.
    let p = root("no kill backup");
    fixture(&p);
    verify_child(&p, "backup");
    verify_child(&p, "verify-old");
    verify_child(&p, "upgrade");
    verify_child(&p, "verify-new");
    let p = root("asset kill");
    asset_fixture(&p);
    let source = rusqlite::Connection::open_with_flags(
        p.join("planner.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    let expected = audit::audit_asset_fixture(&source).unwrap();
    source.close().unwrap();
    kill_at(&p, "asset-backup", "bu_after_asset_copy");
    let evidence = barrier_evidence::capture(
        &p,
        "bu_after_asset_copy",
        startup::Catalogue::AssetFixture,
        &expected,
    );
    println!(
        "CLASS barrier=bu_after_asset_copy WAL/SHM/journal={:?} expected=asset-source-old",
        [
            "planner.sqlite-wal",
            "planner.sqlite-shm",
            "planner.sqlite-journal"
        ]
        .map(|n| p.join(n).exists())
    );
    verify_child(&p, "asset-backup");
    evidence.assert_preserved();
}

fn asset_fixture(p: &Path) {
    let mut db = database::Database::open(p).unwrap();
    two_commands(&mut db);
    db.connection
        .execute_batch(audit::ASSET_FIXTURE_SQL)
        .unwrap();
    db.connection
        .execute(
            "INSERT INTO fixture_assets VALUES(?1,'png',21)",
            ["7d51503664b77f2147ce760635926a8a4ec49a926b6115a3fed5b4feed3002f4"],
        )
        .unwrap();
    db.connection
        .execute(
            "UPDATE schema_migrations SET checksum=?1 WHERE version=1",
            [bytes_digest(audit::asset_source_sql().as_bytes()).0],
        )
        .unwrap();
    std::fs::create_dir(p.join("assets")).unwrap();
    std::fs::write(
        p.join("assets/7d51503664b77f2147ce760635926a8a4ec49a926b6115a3fed5b4feed3002f4.png"),
        b"backup-asset-fixture\n",
    )
    .unwrap();
}
#[test]
fn backup_known_fixture_asset_closure_and_mismatch_refusal() {
    let p = root("asset positive");
    asset_fixture(&p);
    verify_child(&p, "asset-backup");
    let p = root("asset altered");
    asset_fixture(&p);
    std::fs::write(
        p.join("assets/7d51503664b77f2147ce760635926a8a4ec49a926b6115a3fed5b4feed3002f4.png"),
        b"different bytes\n",
    )
    .unwrap();
    let mut db = database::Database::open_catalogue(&p, startup::Catalogue::AssetFixture).unwrap();
    let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::AssetFixture).unwrap();
    assert!(m.backup().is_err());
}

#[test]
fn upgrade_requires_successful_backup() {
    for at in [
        "bu_after_directory",
        "bu_after_reserve",
        "bu_after_done",
        "bu_after_database_flush",
        "bu_after_closure",
        "bu_after_manifest_write",
        "bu_after_verified",
        "backup_deadline",
        "destination_close_error",
        "custody_rng_error",
        "custody_manifest_short_write",
        "custody_manifest_enospc",
        "custody_manifest_flush",
        "custody_reserved_flush",
    ] {
        let p = root(at);
        fixture(&p);
        let mut db = database::Database::open_catalogue(&p, startup::Catalogue::Fixture).unwrap();
        let old = audit::audit_fixture(&db.connection, audit::FixtureVersion::V1).unwrap();
        let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::Fixture).unwrap();
        EXECUTIONS.with(|n| n.set(0));
        FAULT.with(|f| f.set(at));
        assert!(m.backup().is_err());
        assert_eq!(EXECUTIONS.with(|n| n.get()), 0);
        assert_eq!(
            audit::audit_fixture(&m.database.connection, audit::FixtureVersion::V1).unwrap(),
            old
        );
    }
}
#[test]
fn backup_source_commit_invalidates_custody() {
    let p = root("source changed");
    fixture(&p);
    let mut db = database::Database::open_catalogue(&p, startup::Catalogue::Fixture).unwrap();
    let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::Fixture).unwrap();
    let receipt = m.backup().unwrap();
    let c = rusqlite::Connection::open(p.join("planner.sqlite")).unwrap();
    c.execute("UPDATE fixture_probe SET value=8 WHERE id=1", [])
        .unwrap();
    c.close().unwrap();
    EXECUTIONS.with(|n| n.set(0));
    assert!(upgrade_fixture::apply(&mut m, receipt).is_err());
    assert_eq!(EXECUTIONS.with(|n| n.get()), 0);
    assert_eq!(
        m.database
            .connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
}

#[test]
fn backup_source_commit_after_snapshot_refuses_receipt() {
    let p = root("commit after snapshot");
    fixture(&p);
    let mut db = database::Database::open_catalogue(&p, startup::Catalogue::Fixture).unwrap();
    let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::Fixture).unwrap();
    FAULT.with(|f| f.set("source_commit_after_snapshot"));
    EXECUTIONS.with(|n| n.set(0));
    assert!(m.backup().is_err());
    assert_eq!(EXECUTIONS.with(|n| n.get()), 0);
    assert_eq!(
        m.database
            .connection
            .query_row("SELECT value FROM fixture_probe", [], |r| r
                .get::<_, i64>(0))
            .unwrap(),
        8
    );
    assert_eq!(
        m.database
            .connection
            .pragma_query_value(None, "user_version", |r| r.get::<_, i64>(0))
            .unwrap(),
        1
    );
    assert!(
        !std::fs::read_dir(p.join("private-backups"))
            .unwrap()
            .next()
            .unwrap()
            .unwrap()
            .path()
            .join("verified.json")
            .exists()
    );
}

#[test]
fn backup_copied_missing_fk_parent_refuses_receipt() {
    let p = root("copied missing FK parent");
    let mut db = database::Database::open(&p).unwrap();
    two_commands(&mut db);
    db.connection.execute_batch(audit::FK_FIXTURE_SQL).unwrap();
    db.connection
        .execute_batch(
            "INSERT INTO fixture_parent VALUES(1); INSERT INTO fixture_child VALUES(1,1);",
        )
        .unwrap();
    db.connection
        .execute(
            "UPDATE schema_migrations SET checksum=?1 WHERE version=1",
            [bytes_digest(audit::fk_source_sql().as_bytes()).0],
        )
        .unwrap();
    let before = audit::audit_fk_fixture(&db.connection).unwrap();
    {
        let mut healthy =
            backup::Maintenance::begin(&mut db, startup::Catalogue::FkFixture).unwrap();
        let receipt = healthy.backup().unwrap();
        let copy = rusqlite::Connection::open_with_flags(
            receipt.path(),
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        assert_eq!(audit::audit_fk_fixture(&copy).unwrap(), before);
        copy.close().unwrap();
    }
    let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::FkFixture).unwrap();
    FAULT.with(|f| f.set("copied_missing_fk_parent"));
    EXECUTIONS.with(|n| n.set(0));
    assert!(m.backup().is_err());
    assert_eq!(EXECUTIONS.with(|n| n.get()), 0);
    assert_eq!(
        audit::audit_fk_fixture(&m.database.connection).unwrap(),
        before
    );
    let attempt = std::fs::read_dir(p.join("private-backups"))
        .unwrap()
        .map(|e| e.unwrap().path())
        .find(|p| !p.join("verified.json").exists())
        .unwrap();
    assert!(!attempt.join("verified.json").exists());
    let copy = rusqlite::Connection::open_with_flags(
        attempt.join("database.sqlite"),
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(
        copy.query_row("PRAGMA integrity_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "ok"
    );
    assert_eq!(
        copy.query_row("PRAGMA foreign_key_check", [], |r| r.get::<_, String>(0))
            .unwrap(),
        "fixture_child"
    );
    copy.close().unwrap();
}

#[test]
fn upgrade_uncertain_commit_unavailable_mismatch_and_lost_custody_stay_recovery_only() {
    for at in [
        "recovery_unreadable",
        "recovery_mismatch",
        "recovery_lost_identity",
        "recovery_lost_lock",
    ] {
        let p = root(at);
        fixture(&p);
        let mut db = database::Database::open_catalogue(&p, startup::Catalogue::Fixture).unwrap();
        let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::Fixture).unwrap();
        let receipt = m.backup().unwrap();
        let saved = receipt.path().to_owned();
        FAULT.with(|f| f.set(at));
        assert!(upgrade_fixture::apply(&mut m, receipt).is_err(), "{at}");
        assert!(m.database.recovery_only);
        let cmd = policy(0x13, 3, true);
        assert!(
            m.database
                .dispatch(&cmd, &canonical_bytes(&cmd).unwrap())
                .is_err()
        );
        let c = rusqlite::Connection::open_with_flags(
            saved,
            rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
        )
        .unwrap();
        oracle(&c, 1);
        c.close().unwrap();
        if at == "recovery_mismatch" {
            assert_eq!(
                m.catalogue
                    .audit(&m.database.connection)
                    .unwrap()
                    .counters
                    .snapshot,
                Revision(4)
            );
        }
    }
}

#[test]
fn backup_asset_missing_reparse_and_unreferenced_preservation() {
    const NAME: &str = "7d51503664b77f2147ce760635926a8a4ec49a926b6115a3fed5b4feed3002f4.png";
    for mode in ["unreferenced", "missing", "reparse"] {
        let p = root(mode);
        asset_fixture(&p);
        let sentinel = p.join("assets/unreferenced.bin");
        std::fs::write(&sentinel, b"unreferenced preserve exact").unwrap();
        let preserved = p.join("preserved-assets");
        if mode == "missing" {
            std::fs::rename(p.join("assets").join(NAME), p.join("preserved-source.png")).unwrap();
        } else if mode == "reparse" {
            std::fs::rename(p.join("assets"), &preserved).unwrap();
            let out = std::process::Command::new("cmd.exe")
                .args(["/d", "/c", "mklink", "/J"])
                .arg(p.join("assets"))
                .arg(&preserved)
                .output()
                .unwrap();
            assert!(
                out.status.success(),
                "{}",
                String::from_utf8_lossy(&out.stderr)
            );
        }
        let mut db =
            database::Database::open_catalogue(&p, startup::Catalogue::AssetFixture).unwrap();
        let before = audit::audit_asset_fixture(&db.connection).unwrap();
        let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::AssetFixture).unwrap();
        EXECUTIONS.with(|n| n.set(0));
        let result = m.backup();
        if mode == "unreferenced" {
            let receipt = result.unwrap();
            let files = std::fs::read_dir(receipt.path().parent().unwrap().join("assets"))
                .unwrap()
                .map(|e| e.unwrap().file_name())
                .collect::<Vec<_>>();
            assert_eq!(files, vec![std::ffi::OsString::from(NAME)]);
        } else {
            assert!(result.is_err(), "{mode}");
        }
        assert_eq!(EXECUTIONS.with(|n| n.get()), 0);
        assert_eq!(
            audit::audit_asset_fixture(&m.database.connection).unwrap(),
            before
        );
        assert_eq!(
            std::fs::read(if mode == "reparse" {
                preserved.join("unreferenced.bin")
            } else {
                sentinel
            })
            .unwrap(),
            b"unreferenced preserve exact"
        );
    }
}
#[test]
fn startup_partial_sidecars_and_future_closed_refuse_unchanged() {
    for suffix in ["-wal", "-shm"] {
        let p = root("partial");
        drop(open(&p).unwrap());
        let live = p.join("planner.sqlite");
        let before = std::fs::read(&live).unwrap();
        let side = p.join(format!("planner.sqlite{suffix}"));
        std::fs::write(&side, []).unwrap();
        assert!(open(&p).is_err());
        assert_eq!(std::fs::read(live).unwrap(), before);
        assert_eq!(std::fs::metadata(side).unwrap().len(), 0);
        assert!(
            !p.join(if suffix == "-wal" {
                "planner.sqlite-shm"
            } else {
                "planner.sqlite-wal"
            })
            .exists()
        );
        verify_child(&p, "refuse-without-sqlite");
    }
    let p = root("future closed");
    fixture(&p);
    let c = rusqlite::Connection::open(p.join("planner.sqlite")).unwrap();
    c.pragma_update(None, "user_version", 999).unwrap();
    c.close().unwrap();
    let before = std::fs::read(p.join("planner.sqlite")).unwrap();
    assert_eq!(open(&p).unwrap_err().code, ErrorCode::UnsupportedVersion);
    assert_eq!(std::fs::read(p.join("planner.sqlite")).unwrap(), before);
}

fn physical(p: &Path) -> Vec<(String, Option<Digest>)> {
    [
        "planner.sqlite",
        "planner.sqlite-wal",
        "planner.sqlite-shm",
        "planner.sqlite-journal",
    ]
    .into_iter()
    .map(|name| {
        (
            name.into(),
            std::fs::read(p.join(name)).ok().map(|b| bytes_digest(&b)),
        )
    })
    .collect()
}
#[test]
fn startup_wal_future_and_zero_shm_preserve_source_through_close() {
    for zero_shm in [false, true] {
        let p = root("future WAL");
        kill_at(&p, "wal-future", "source_ready");
        if zero_shm {
            std::fs::write(p.join("planner.sqlite-shm"), []).unwrap();
        }
        let before = physical(&p);
        verify_child(&p, "refuse-future-wal");
        assert_eq!(physical(&p), before);
    }
    let p = root("known zero SHM");
    kill_at(&p, "wal-source", "source_ready");
    std::fs::write(p.join("planner.sqlite-shm"), []).unwrap();
    verify_child(&p, "recognise");
}

#[test]
fn backup_receipt_rejects_another_maintenance_and_source() {
    let p = root("receipt source1");
    let q = root("receipt source2");
    fixture(&p);
    fixture(&q);
    let mut db1 = database::Database::open_catalogue(&p, startup::Catalogue::Fixture).unwrap();
    let mut db2 = database::Database::open_catalogue(&q, startup::Catalogue::Fixture).unwrap();
    let mut m1 = backup::Maintenance::begin(&mut db1, startup::Catalogue::Fixture).unwrap();
    let receipt1 = m1.backup().unwrap();
    let mut m2 = backup::Maintenance::begin(&mut db2, startup::Catalogue::Fixture).unwrap();
    let _receipt2 = m2.backup().unwrap();
    assert!(upgrade_fixture::apply(&mut m2, receipt1).is_err());
    oracle(&m2.database.connection, 1);
}

#[test]
fn backup_receipt_saved_bindings_and_seals_refuse_before_migration() {
    for field in [
        "root",
        "live",
        "maintenance",
        "data_version",
        "intent",
        "source",
        "identity",
        "length",
        "hash",
        "manifest_hash",
    ] {
        let p = root(field);
        fixture(&p);
        let mut db = database::Database::open_catalogue(&p, startup::Catalogue::Fixture).unwrap();
        let before = audit::audit_fixture(&db.connection, audit::FixtureVersion::V1).unwrap();
        let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::Fixture).unwrap();
        let mut receipt = m.backup().unwrap();
        let path = receipt.path().to_owned();
        let bytes = std::fs::read(&path).unwrap();
        receipt.invalidate_saved_for_test(field);
        EXECUTIONS.with(|n| n.set(0));
        assert!(upgrade_fixture::apply(&mut m, receipt).is_err(), "{field}");
        assert_eq!(EXECUTIONS.with(|n| n.get()), 0);
        assert_eq!(m.catalogue.audit(&m.database.connection).unwrap(), before);
        assert_eq!(std::fs::read(path).unwrap(), bytes);
    }
}
#[test]
fn backup_copied_corruption_prevents_receipt_and_upgrade() {
    let p = root("corrupt copied destination");
    fixture(&p);
    let mut db = database::Database::open_catalogue(&p, startup::Catalogue::Fixture).unwrap();
    let before = audit::audit_fixture(&db.connection, audit::FixtureVersion::V1).unwrap();
    let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::Fixture).unwrap();
    FAULT.with(|f| f.set("corrupt_destination"));
    assert!(m.backup().is_err());
    assert_eq!(m.catalogue.audit(&m.database.connection).unwrap(), before);
}
#[test]
fn upgrade_preserves_maximum_counter_and_original_receipt_bytes() {
    let p = root("max revision");
    fixture(&p);
    let c = rusqlite::Connection::open(p.join("planner.sqlite")).unwrap();
    c.execute(
        "UPDATE app_meta SET snapshot_revision='18446744073709551615' WHERE singleton=1",
        [],
    )
    .unwrap();
    c.close().unwrap();
    let mut db = database::Database::open_catalogue(&p, startup::Catalogue::Fixture).unwrap();
    let before:Vec<u8>=db.connection.query_row("SELECT response FROM command_receipts WHERE command_id='cm_00000000000000000000000000000011'",[],|r|r.get(0)).unwrap();
    let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::Fixture).unwrap();
    let receipt = m.backup().unwrap();
    upgrade_fixture::apply(&mut m, receipt).unwrap();
    assert_eq!(
        snapshot::load(&m.database.connection)
            .unwrap()
            .snapshot_revision,
        Revision(u64::MAX)
    );
    let cmd = policy(0x11, 1, true);
    assert_eq!(
        m.database
            .dispatch(&cmd, &canonical_bytes(&cmd).unwrap())
            .unwrap(),
        before
    );
}
#[test]
fn upgrade_reader_dependency_refuses_before_backup() {
    let p = root("unavailable reader");
    fixture(&p);
    let mut db = database::Database::open_catalogue(&p, startup::Catalogue::Fixture).unwrap();
    let result = backup::Maintenance::begin(&mut db, startup::Catalogue::ReaderUnavailable);
    assert!(matches!(result,Err(e) if e.code==ErrorCode::UnsupportedVersion));
    assert!(!p.join("private-backups").exists());
}

#[test]
fn backup_serialises_owner_commands_in_private_driver() {
    use std::sync::mpsc;
    let p = root("owner serialisation");
    let owned = p.clone();
    let (command_send, command_receive) = mpsc::channel();
    let (response_send, response_receive) = mpsc::channel();
    let (paused_send, paused_receive) = mpsc::channel();
    let (resume_send, resume_receive) = mpsc::channel();
    let worker = std::thread::spawn(move || {
        let mut db = database::Database::open(&owned).unwrap();
        let first = policy(0x11, 1, true);
        db.dispatch(&first, &canonical_bytes(&first).unwrap())
            .unwrap();
        let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::B1).unwrap();
        let receipt = m.backup().unwrap();
        paused_send.send(receipt.path().to_owned()).unwrap();
        resume_receive.recv().unwrap();
        drop(receipt);
        drop(m);
        let command: AppCommandV1 = command_receive.recv().unwrap();
        let result = db
            .dispatch(&command, &canonical_bytes(&command).unwrap())
            .unwrap();
        response_send.send(result).unwrap();
    });
    let backup_path = paused_receive
        .recv_timeout(std::time::Duration::from_secs(30))
        .unwrap();
    command_send.send(policy(0x12, 2, false)).unwrap();
    assert!(
        response_receive
            .recv_timeout(std::time::Duration::from_millis(50))
            .is_err()
    );
    let c = rusqlite::Connection::open_with_flags(
        backup_path,
        rusqlite::OpenFlags::SQLITE_OPEN_READ_ONLY,
    )
    .unwrap();
    assert_eq!(snapshot::load(&c).unwrap().snapshot_revision, Revision(2));
    c.close().unwrap();
    resume_send.send(()).unwrap();
    let bytes = response_receive
        .recv_timeout(std::time::Duration::from_secs(30))
        .unwrap();
    assert_eq!(
        parse_document::<Snapshot>(&bytes)
            .unwrap()
            .snapshot_revision,
        Revision(3)
    );
    worker.join().unwrap();
}
