unsafe extern "C" fn reviewer_deny_rollback(_: *mut std::ffi::c_void, action:i32, arg1:*const std::ffi::c_char, _: *const std::ffi::c_char, _: *const std::ffi::c_char, _: *const std::ffi::c_char) -> i32 {
    if action==rusqlite::ffi::SQLITE_TRANSACTION && !arg1.is_null() && unsafe {std::ffi::CStr::from_ptr(arg1)}.to_bytes()==b"ROLLBACK" {rusqlite::ffi::SQLITE_DENY} else {rusqlite::ffi::SQLITE_OK}
}
#[test]
fn reviewer_rollback_error_must_poison_owner() {
    let p=root("reviewer rollback error");
    fixture(&p);
    let mut db=database::Database::open_catalogue(&p,startup::Catalogue::Fixture).unwrap();
    let mut m=backup::Maintenance::begin(&mut db,startup::Catalogue::Fixture).unwrap();
    let receipt=m.backup().unwrap();
    let backup_path=receipt.path().to_owned();
    let rc=unsafe {rusqlite::ffi::sqlite3_set_authorizer(m.database.connection.handle(),Some(reviewer_deny_rollback),std::ptr::null_mut())};
    assert_eq!(rc,rusqlite::ffi::SQLITE_OK);
    FAULT.with(|f| f.set("ug_after_ddl"));
    assert!(upgrade_fixture::apply(&mut m,receipt).is_err());
    let backup_write_open=std::fs::OpenOptions::new().write(true).open(&backup_path).is_ok();
    println!("REVIEWER rollback returned error: autocommit={} recovery_only={} backup_write_open={backup_write_open}",m.database.connection.is_autocommit(),m.database.recovery_only);
    assert!(m.database.recovery_only,"rollback error returned with owner still enabled");
}

static REVIEWER_BEGAN: std::sync::atomic::AtomicBool=std::sync::atomic::AtomicBool::new(false);
unsafe extern "C" fn reviewer_deny_recheck_and_rollback(_: *mut std::ffi::c_void, action:i32, arg1:*const std::ffi::c_char, _: *const std::ffi::c_char, _: *const std::ffi::c_char, _: *const std::ffi::c_char) -> i32 {
    use std::sync::atomic::Ordering;
    let arg=if arg1.is_null(){b"".as_slice()}else{unsafe{std::ffi::CStr::from_ptr(arg1)}.to_bytes()};
    if action==rusqlite::ffi::SQLITE_TRANSACTION && arg==b"BEGIN" {REVIEWER_BEGAN.store(true,Ordering::SeqCst);}
    if (action==rusqlite::ffi::SQLITE_TRANSACTION && arg==b"ROLLBACK") || (action==rusqlite::ffi::SQLITE_PRAGMA && arg==b"data_version" && REVIEWER_BEGAN.load(Ordering::SeqCst)) {rusqlite::ffi::SQLITE_DENY} else {rusqlite::ffi::SQLITE_OK}
}
#[test]
fn reviewer_early_recheck_failure_must_poison_owner() {
    let p=root("reviewer recheck error");
    fixture(&p);
    let mut db=database::Database::open_catalogue(&p,startup::Catalogue::Fixture).unwrap();
    let mut m=backup::Maintenance::begin(&mut db,startup::Catalogue::Fixture).unwrap();
    let receipt=m.backup().unwrap();
    let backup_path=receipt.path().to_owned();
    REVIEWER_BEGAN.store(false,std::sync::atomic::Ordering::SeqCst);
    let rc=unsafe{rusqlite::ffi::sqlite3_set_authorizer(m.database.connection.handle(),Some(reviewer_deny_recheck_and_rollback),std::ptr::null_mut())};
    assert_eq!(rc,rusqlite::ffi::SQLITE_OK);
    assert!(upgrade_fixture::apply(&mut m,receipt).is_err());
    let backup_write_open=std::fs::OpenOptions::new().write(true).open(&backup_path).is_ok();
    println!("REVIEWER early source recheck error: autocommit={} recovery_only={} backup_write_open={backup_write_open}",m.database.connection.is_autocommit(),m.database.recovery_only);
    assert!(m.database.recovery_only,"post-BEGIN recheck error returned with owner still enabled");
}
