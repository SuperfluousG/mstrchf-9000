//! Reuse of the accepted cleanup controls against the production catalogue and transform.
//! Original fixture controls remain separately compiled and unchanged in their own files.
use super::*;
use crate::backup_upgrade_tests::FAULT;
use std::path::{Path, PathBuf};
fn root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "MSTRCHF compat cleanup {label} {}",
        records::random_id().unwrap()
    ))
}
fn fixture(p: &Path) {
    std::fs::create_dir_all(p).unwrap();
    let c = compatibility_tests::old();
    let mut dst = rusqlite::Connection::open(p.join("planner.sqlite")).unwrap();
    {
        let b = rusqlite::backup::Backup::new(&c, &mut dst).unwrap();
        b.run_to_completion(100, std::time::Duration::from_millis(1), None)
            .unwrap();
    }
    dst.close().unwrap();
}
fn policy(id: u64, rev: u64, on: bool) -> AppCommandV1 {
    AppCommandV1 {
        schema_version: SchemaVersion,
        command_id: CommandId(format!("cm_{id:032x}")),
        expected_snapshot_revision: Revision(rev),
        command: AppCommand::SetPrivacy {
            policy: PrivacyPolicy {
                revision: Revision(2),
                use_ai_features: on,
                include_household_requirements: false,
                confirmed_at: Nullable(None),
            },
            expected_policy_revision: Revision(2),
        },
    }
}
fn oracle(c: &rusqlite::Connection, want: u32) {
    let a = audit::audit_b1(c).unwrap();
    assert_eq!(a.version, want);
    assert_eq!(a.counters.snapshot, Revision(if want == 1 { 2 } else { 3 }));
    let b:Vec<u8>=c.query_row("SELECT response FROM command_receipts WHERE command_id='cm_00000000000000000000000000000002'",[],|r|r.get(0)).unwrap();
    assert_eq!(b, compatibility_tests::literal("old-privacy-response.json"));
    assert_eq!(
        audit::frozen_snapshot(
            c,
            if want == 1 {
                compat::Profile::Old
            } else {
                compat::Profile::Current
            }
        )
        .unwrap(),
        compatibility_tests::literal(if want == 1 {
            "old-privacy-response.json"
        } else {
            "new-transition-snapshot.json"
        })
    );
}
unsafe extern "C" fn reviewer_deny_rollback(
    _: *mut std::ffi::c_void,
    action: i32,
    arg1: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
) -> i32 {
    if action == rusqlite::ffi::SQLITE_TRANSACTION
        && !arg1.is_null()
        && unsafe { std::ffi::CStr::from_ptr(arg1) }.to_bytes() == b"ROLLBACK"
    {
        rusqlite::ffi::SQLITE_DENY
    } else {
        rusqlite::ffi::SQLITE_OK
    }
}
#[test]
fn reviewer_rollback_error_must_poison_owner() {
    let p = root("reviewer rollback error");
    fixture(&p);
    let mut db = database::Database::open_catalogue(&p, startup::Catalogue::B1).unwrap();
    let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::B1).unwrap();
    let receipt = m.backup().unwrap();
    let backup_path = receipt.path().to_owned();
    let rc = unsafe {
        rusqlite::ffi::sqlite3_set_authorizer(
            m.database.connection.handle(),
            Some(reviewer_deny_rollback),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(rc, rusqlite::ffi::SQLITE_OK);
    FAULT.with(|f| f.set("ug_after_ddl"));
    assert!(upgrade::apply(&mut m, receipt).is_err());
    let backup_write_open = std::fs::OpenOptions::new()
        .write(true)
        .open(&backup_path)
        .is_ok();
    println!(
        "REVIEWER rollback returned error: autocommit={} recovery_only={} backup_write_open={backup_write_open}",
        m.database.connection.is_autocommit(),
        m.database.recovery_only
    );
    assert!(
        m.database.recovery_only,
        "rollback error returned with owner still enabled"
    );
}

static REVIEWER_BEGAN: std::sync::atomic::AtomicBool = std::sync::atomic::AtomicBool::new(false);
unsafe extern "C" fn reviewer_deny_recheck_and_rollback(
    _: *mut std::ffi::c_void,
    action: i32,
    arg1: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
) -> i32 {
    use std::sync::atomic::Ordering;
    let arg = if arg1.is_null() {
        b"".as_slice()
    } else {
        unsafe { std::ffi::CStr::from_ptr(arg1) }.to_bytes()
    };
    if action == rusqlite::ffi::SQLITE_TRANSACTION && arg == b"BEGIN" {
        REVIEWER_BEGAN.store(true, Ordering::SeqCst);
    }
    if (action == rusqlite::ffi::SQLITE_TRANSACTION && arg == b"ROLLBACK")
        || (action == rusqlite::ffi::SQLITE_PRAGMA
            && arg == b"data_version"
            && REVIEWER_BEGAN.load(Ordering::SeqCst))
    {
        rusqlite::ffi::SQLITE_DENY
    } else {
        rusqlite::ffi::SQLITE_OK
    }
}
#[test]
fn reviewer_early_recheck_failure_must_poison_owner() {
    let p = root("reviewer recheck error");
    fixture(&p);
    let mut db = database::Database::open_catalogue(&p, startup::Catalogue::B1).unwrap();
    let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::B1).unwrap();
    let receipt = m.backup().unwrap();
    let backup_path = receipt.path().to_owned();
    REVIEWER_BEGAN.store(false, std::sync::atomic::Ordering::SeqCst);
    let rc = unsafe {
        rusqlite::ffi::sqlite3_set_authorizer(
            m.database.connection.handle(),
            Some(reviewer_deny_recheck_and_rollback),
            std::ptr::null_mut(),
        )
    };
    assert_eq!(rc, rusqlite::ffi::SQLITE_OK);
    assert!(upgrade::apply(&mut m, receipt).is_err());
    let backup_write_open = std::fs::OpenOptions::new()
        .write(true)
        .open(&backup_path)
        .is_ok();
    println!(
        "REVIEWER early source recheck error: autocommit={} recovery_only={} backup_write_open={backup_write_open}",
        m.database.connection.is_autocommit(),
        m.database.recovery_only
    );
    assert!(
        m.database.recovery_only,
        "post-BEGIN recheck error returned with owner still enabled"
    );
}
// Correction controls extend, without changing, the independent reviewer probes.
unsafe extern "C" fn correction_deny_commit(
    context: *mut std::ffi::c_void,
    action: i32,
    arg1: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
) -> i32 {
    let arg = if arg1.is_null() {
        b"".as_slice()
    } else {
        unsafe { std::ffi::CStr::from_ptr(arg1) }.to_bytes()
    };
    if action == rusqlite::ffi::SQLITE_TRANSACTION
        && (arg == b"COMMIT" || (!context.is_null() && arg == b"ROLLBACK"))
    {
        rusqlite::ffi::SQLITE_DENY
    } else {
        rusqlite::ffi::SQLITE_OK
    }
}

#[test]
fn correction_actual_commit_error_checks_active_transaction_cleanup() {
    for deny_rollback in [false, true] {
        let p = root("actual commit authorization error");
        fixture(&p);
        let mut db = database::Database::open_catalogue(&p, startup::Catalogue::B1).unwrap();
        let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::B1).unwrap();
        let receipt = m.backup().unwrap();
        let saved = receipt.path().to_owned();
        let bytes = std::fs::read(&saved).unwrap();
        // Opaque context points at a live local flag but is never dereferenced.
        let context = if deny_rollback {
            std::ptr::from_ref(&deny_rollback).cast_mut().cast()
        } else {
            std::ptr::null_mut()
        };
        assert_eq!(
            unsafe {
                rusqlite::ffi::sqlite3_set_authorizer(
                    m.database.connection.handle(),
                    Some(correction_deny_commit),
                    context,
                )
            },
            rusqlite::ffi::SQLITE_OK
        );
        assert!(upgrade::apply(&mut m, receipt).is_err());
        assert_eq!(m.database.connection.is_autocommit(), !deny_rollback);
        drop(m);
        assert_disabled_owner(&mut db);
        assert_owner_drop_releases(db, &saved, &bytes);
        let recovered = database::Database::open_catalogue(&p, startup::Catalogue::B1).unwrap();
        oracle(&recovered.connection, 1);
    }
}

fn assert_held_backup(path: &Path) {
    use std::os::windows::fs::OpenOptionsExt;
    use windows_sys::Win32::Storage::FileSystem::DELETE;
    for result in [
        std::fs::OpenOptions::new().write(true).open(path),
        std::fs::OpenOptions::new().access_mode(DELETE).open(path),
    ] {
        assert_eq!(result.unwrap_err().raw_os_error(), Some(32));
    }
    assert!(std::fs::File::open(path).is_ok());
    let manifest = path.parent().unwrap().join("verified.json");
    assert_eq!(
        std::fs::OpenOptions::new()
            .write(true)
            .open(manifest)
            .unwrap_err()
            .raw_os_error(),
        Some(32)
    );
}
fn assert_disabled_owner(db: &mut database::Database) {
    assert!(db.recovery_only);
    assert!(db.snapshot().is_err());
    let command = policy(0x13, 3, true);
    assert!(
        db.dispatch(&command, &canonical_bytes(&command).unwrap())
            .is_err()
    );
    assert!(backup::Maintenance::begin(db, startup::Catalogue::B1).is_err());
}
fn assert_owner_drop_releases(db: database::Database, path: &Path, bytes: &[u8]) {
    assert_held_backup(path);
    drop(db);
    assert!(std::fs::OpenOptions::new().write(true).open(path).is_ok());
    assert!(
        std::fs::OpenOptions::new()
            .write(true)
            .open(path.parent().unwrap().join("verified.json"))
            .is_ok()
    );
    assert_eq!(std::fs::read(path).unwrap(), bytes);
}

#[test]
fn correction_failed_rollback_retains_backup_beyond_maintenance_lifetime() {
    for early in [false, true] {
        let p = root("unresolved rollback lifetime");
        fixture(&p);
        let mut db = database::Database::open_catalogue(&p, startup::Catalogue::B1).unwrap();
        let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::B1).unwrap();
        let receipt = m.backup().unwrap();
        let saved = receipt.path().to_owned();
        let bytes = std::fs::read(&saved).unwrap();
        REVIEWER_BEGAN.store(false, std::sync::atomic::Ordering::SeqCst);
        let authorizer = if early {
            reviewer_deny_recheck_and_rollback
        } else {
            reviewer_deny_rollback
        };
        // Callback is static and dereferences no context; SQLite owns neither it nor data.
        assert_eq!(
            unsafe {
                rusqlite::ffi::sqlite3_set_authorizer(
                    m.database.connection.handle(),
                    Some(authorizer),
                    std::ptr::null_mut(),
                )
            },
            rusqlite::ffi::SQLITE_OK
        );
        if !early {
            FAULT.with(|f| f.set("ug_after_ddl"));
        }
        assert!(upgrade::apply(&mut m, receipt).is_err());
        assert!(!m.database.connection.is_autocommit());
        assert_held_backup(&saved);
        drop(m);
        assert_disabled_owner(&mut db);
        assert_owner_drop_releases(db, &saved, &bytes);
        let recovered = database::Database::open_catalogue(&p, startup::Catalogue::B1).unwrap();
        oracle(&recovered.connection, 1);
        println!(
            "CORRECTION unresolved rollback early={early}: owner disabled; held through Maintenance drop; owner close recovers exact old"
        );
    }
}

unsafe extern "C" fn correction_deny_begin(
    _: *mut std::ffi::c_void,
    action: i32,
    arg1: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
    _: *const std::ffi::c_char,
) -> i32 {
    if action == rusqlite::ffi::SQLITE_TRANSACTION
        && !arg1.is_null()
        && unsafe { std::ffi::CStr::from_ptr(arg1) }.to_bytes() == b"BEGIN"
    {
        rusqlite::ffi::SQLITE_DENY
    } else {
        rusqlite::ffi::SQLITE_OK
    }
}
#[test]
fn correction_begin_denial_and_checked_rollback_leave_disabled_owner() {
    for begin_denied in [true, false] {
        let p = root("begin or rollback resolved");
        fixture(&p);
        let mut db = database::Database::open_catalogue(&p, startup::Catalogue::B1).unwrap();
        let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::B1).unwrap();
        let receipt = m.backup().unwrap();
        let saved = receipt.path().to_owned();
        let bytes = std::fs::read(&saved).unwrap();
        if begin_denied {
            assert_eq!(
                unsafe {
                    rusqlite::ffi::sqlite3_set_authorizer(
                        m.database.connection.handle(),
                        Some(correction_deny_begin),
                        std::ptr::null_mut(),
                    )
                },
                rusqlite::ffi::SQLITE_OK
            );
        } else {
            FAULT.with(|f| f.set("ug_after_ddl"));
        }
        assert!(upgrade::apply(&mut m, receipt).is_err());
        assert!(m.database.connection.is_autocommit());
        assert_eq!(m.catalogue.audit(&m.database.connection).unwrap(), m.source);
        drop(m);
        assert_disabled_owner(&mut db);
        assert_owner_drop_releases(db, &saved, &bytes);
    }
}

#[test]
fn correction_recovery_exits_keep_owner_disabled_and_backup_held() {
    for at in [
        "commit_before_error",
        "ug_after_commit",
        "recovery_before_placeholder",
        "reopen_unavailable",
        "recovery_unreadable",
        "recovery_before_audit",
        "recovery_deny_audit",
        "recovery_mismatch",
        "recovery_lost_identity",
        "recovery_lost_lock",
        "recovery_before_configure",
        "recovery_deny_configure",
        "ug_before_ready",
    ] {
        let p = root(at);
        fixture(&p);
        let mut db = database::Database::open_catalogue(&p, startup::Catalogue::B1).unwrap();
        let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::B1).unwrap();
        let receipt = m.backup().unwrap();
        let saved = receipt.path().to_owned();
        let bytes = std::fs::read(&saved).unwrap();
        FAULT.with(|f| f.set(at));
        assert!(upgrade::apply(&mut m, receipt).is_err(), "{at}");
        assert!(FAULT.with(|f| f.get().is_empty()), "unreached fault {at}");
        drop(m);
        assert_disabled_owner(&mut db);
        assert_owner_drop_releases(db, &saved, &bytes);
        println!(
            "CORRECTION exit={at}: snapshot/command/new-maintenance refused, receipt retained until owner drop"
        );
    }
}

struct CorrectionStatement(*mut rusqlite::ffi::sqlite3_stmt);
impl Drop for CorrectionStatement {
    fn drop(&mut self) {
        // This test finalises before dropping the restored owner connection.
        assert_eq!(
            unsafe { rusqlite::ffi::sqlite3_finalize(self.0) },
            rusqlite::ffi::SQLITE_OK
        );
    }
}
#[test]
fn correction_real_sqlite_busy_close_restores_connection_and_retains_backup() {
    let p = root("actual SQLITE_BUSY close");
    fixture(&p);
    let mut db = database::Database::open_catalogue(&p, startup::Catalogue::B1).unwrap();
    let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::B1).unwrap();
    let receipt = m.backup().unwrap();
    let saved = receipt.path().to_owned();
    let bytes = std::fs::read(&saved).unwrap();
    let mut raw = std::ptr::null_mut();
    let rc = unsafe {
        rusqlite::ffi::sqlite3_prepare_v2(
            m.database.connection.handle(),
            c"SELECT lineage FROM app_meta".as_ptr(),
            -1,
            &mut raw,
            std::ptr::null_mut(),
        )
    };
    assert_eq!(rc, rusqlite::ffi::SQLITE_OK);
    let statement = CorrectionStatement(raw);
    assert!(upgrade::apply(&mut m, receipt).is_err());
    assert!(m.database.connection.is_autocommit());
    drop(statement);
    // A placeholder connection would have no fixture table: this is the restored
    // connection returned by the actual failed sqlite3_close call.
    oracle(&m.database.connection, 2);
    drop(m);
    assert_disabled_owner(&mut db);
    assert_owner_drop_releases(db, &saved, &bytes);
    println!("CORRECTION actual SQLITE_BUSY close: original connection restored; seals retained");
}

#[test]
fn correction_exact_success_releases_custody_and_enables_owner() {
    for at in ["", "commit_after_error"] {
        let p = root("successful resolution");
        fixture(&p);
        let mut db = database::Database::open_catalogue(&p, startup::Catalogue::B1).unwrap();
        let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::B1).unwrap();
        let receipt = m.backup().unwrap();
        let saved = receipt.path().to_owned();
        FAULT.with(|f| f.set(at));
        upgrade::apply(&mut m, receipt).unwrap();
        oracle(&m.database.connection, 2);
        assert!(!m.database.recovery_only);
        assert!(!m.database.has_upgrade_backup());
        drop(m);
        assert!(std::fs::OpenOptions::new().write(true).open(saved).is_ok());
        assert_eq!(db.snapshot().unwrap().snapshot_revision, Revision(3));
        let command = policy(0x13, 3, true);
        let response = db
            .dispatch(&command, &canonical_bytes(&command).unwrap())
            .unwrap();
        assert_eq!(
            parse_document::<Snapshot>(&response)
                .unwrap()
                .snapshot_revision,
            Revision(4)
        );
    }
}
fn spawn(p: &Path, mode: &str, at: &str) -> std::process::Child {
    use std::process::{Command, Stdio};
    Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "compatibility_cleanup_tests::child",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("MSTRCHF_COMPAT_ROOT", p)
        .env("MSTRCHF_COMPAT_MODE", mode)
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
    let Ok(mode) = std::env::var("MSTRCHF_COMPAT_MODE") else {
        return;
    };
    let p = PathBuf::from(std::env::var_os("MSTRCHF_COMPAT_ROOT").unwrap());
    let mut db = database::Database::open_catalogue(&p, startup::Catalogue::B1).unwrap();
    if mode == "verify-old" {
        oracle(&db.connection, 1);
        return;
    }
    if mode == "verify-new" {
        oracle(&db.connection, 2);
        return;
    }
    let mut m = backup::Maintenance::begin(&mut db, startup::Catalogue::B1).unwrap();
    let receipt = m.backup().unwrap();
    upgrade::apply(&mut m, receipt).unwrap();
    oracle(&m.database.connection, 2);
}
#[test]
fn production_transition_kill_matrix() {
    for (at, want) in [
        ("ug_after_begin", 1),
        ("ug_after_ddl", 1),
        ("ug_after_insert", 1),
        ("ug_after_data", 1),
        ("ug_after_ledger", 1),
        ("ug_after_version", 1),
        ("ug_before_commit", 1),
        ("ug_after_commit", 2),
        ("ug_before_ready", 2),
    ] {
        let p = root(at);
        fixture(&p);
        let c = rusqlite::Connection::open(p.join("planner.sqlite")).unwrap();
        let expected = audit::audit_b1(&c).unwrap();
        c.close().unwrap();
        kill_at(&p, "upgrade", at);
        let evidence = barrier_evidence::capture(&p, at, startup::Catalogue::B1, &expected);
        verify_child(
            &p,
            if want == 1 {
                "verify-old"
            } else {
                "verify-new"
            },
        );
        evidence.assert_preserved();
    }
}
