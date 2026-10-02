//! Recognition never obtains writable SQLite access on an unrecognised source.
use crate::{
    audit::{self, AuditedRows},
    custody::*,
    records::storage,
};
use mstrchf_contracts::{DomainError, ErrorCode};
use rusqlite::{Connection, OpenFlags};
use std::{
    path::Path,
    time::{Duration, Instant},
};

pub(crate) const SOURCE_ID: &str =
    "2026-06-03 19:12:13 d6e03d8c777cfa2d35e3b60d8ec3e0187f3e9f99d8e2ee9cac695fd6fcdf1a24";

#[derive(Clone, Copy)]
pub(crate) enum Catalogue {
    B1,
    #[cfg(test)]
    Fixture,
    #[cfg(test)]
    ReaderUnavailable,
    #[cfg(test)]
    AssetFixture,
    #[cfg(test)]
    FkFixture,
}
impl Catalogue {
    pub(crate) fn audit(self, c: &Connection) -> Result<AuditedRows, DomainError> {
        match self {
            Self::B1 => audit::audit_b1(c),
            #[cfg(test)]
            Self::Fixture => {
                let v: i64 = c
                    .pragma_query_value(None, "user_version", |r| r.get(0))
                    .map_err(storage)?;
                let v = match v {
                    1 => audit::FixtureVersion::V1,
                    2 => audit::FixtureVersion::V2,
                    _ => {
                        return Err(DomainError::new(
                            ErrorCode::UnsupportedVersion,
                            "/storage_version",
                        ));
                    }
                };
                audit::audit_fixture(c, v)
            }
            #[cfg(test)]
            Self::ReaderUnavailable => Err(DomainError::new(
                ErrorCode::UnsupportedVersion,
                "/storage_reader",
            )),
            #[cfg(test)]
            Self::AssetFixture => audit::audit_asset_fixture(c),
            #[cfg(test)]
            Self::FkFixture => audit::audit_fk_fixture(c),
        }
    }
}
fn recovery_required() -> DomainError {
    DomainError::new(ErrorCode::StorageFailure, "/storage/startup_recovery")
}

/// Caller cannot append URI parameters: every path byte outside the safe set is encoded.
fn uri(path: &Path, immutable: bool) -> Result<String, DomainError> {
    let text = path.to_str().ok_or_else(|| storage("path encoding"))?;
    let text = text
        .strip_prefix("\\\\?\\")
        .unwrap_or(text)
        .replace('\\', "/");
    let mut result = String::from("file:///");
    for b in text.bytes() {
        if b.is_ascii_alphanumeric() || matches!(b, b'/' | b':' | b'-' | b'_' | b'.') {
            result.push(char::from(b));
        } else {
            result.push_str(&format!("%{b:02X}"));
        }
    }
    result.push_str(if immutable {
        "?mode=ro&cache=private&immutable=1"
    } else {
        "?mode=ro&cache=private&readonly_shm=1"
    });
    Ok(result)
}

fn reader(path: &Path, immutable: bool) -> Result<Connection, DomainError> {
    if rusqlite::version_number() != 3_053_002 {
        return Err(storage("unpinned SQLite"));
    }
    let c = Connection::open_with_flags_and_vfs(
        uri(path, immutable)?,
        OpenFlags::SQLITE_OPEN_READ_ONLY
            | OpenFlags::SQLITE_OPEN_URI
            | OpenFlags::SQLITE_OPEN_PRIVATE_CACHE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        "win32",
    )
    .map_err(storage)?;
    c.busy_timeout(Duration::from_millis(2000))
        .map_err(storage)?;
    c.execute_batch("PRAGMA query_only=ON; PRAGMA temp_store=MEMORY; PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF;").map_err(storage)?;
    let id: String = c
        .query_row("SELECT sqlite_source_id()", [], |r| r.get(0))
        .map_err(storage)?;
    if id != SOURCE_ID {
        return Err(storage("unpinned SQLite source"));
    }
    for (p, value) in [
        ("query_only", 1),
        ("temp_store", 2),
        ("foreign_keys", 1),
        ("trusted_schema", 0),
        ("busy_timeout", 2000),
    ] {
        let actual: i64 = c
            .pragma_query_value(None, p, |r| r.get(0))
            .map_err(storage)?;
        if actual != value {
            return Err(storage("reader settings"));
        }
    }
    Ok(c)
}

struct PhysicalSet {
    main: SealedFile,
    wal: Option<SealedFile>,
    shm: Option<SealedFile>,
}
impl PhysicalSet {
    fn reverify(&mut self, root: &TrustedStoreRoot) -> Result<(), DomainError> {
        self.main.reverify()?;
        for (name, leaf) in [
            ("planner.sqlite-wal", &mut self.wal),
            ("planner.sqlite-shm", &mut self.shm),
        ] {
            match leaf {
                Some(leaf) => leaf.reverify()?,
                None => {
                    if root.seal_leaf(name)?.is_some() {
                        return Err(storage("source sidecar appeared"));
                    }
                }
            }
        }
        if root.seal_leaf("planner.sqlite-journal")?.is_some() {
            return Err(storage("source journal appeared"));
        }
        Ok(())
    }
}

/// Fields/constructor private; token cannot be restored from persisted evidence.
struct RecognisedStartup {
    state: AuditedRows,
    physical: PhysicalSet,
    _inspection: Option<(PrivateBackupDir, SealedFile)>,
}

fn recognise(
    root: &TrustedStoreRoot,
    lock: &StoreLock,
    catalogue: Catalogue,
) -> Result<RecognisedStartup, DomainError> {
    lock.verify_for(root)?;
    let began = Instant::now();
    let main = root.seal_live()?;
    let mut physical = PhysicalSet {
        main,
        wal: root.seal_leaf("planner.sqlite-wal")?,
        shm: root.seal_leaf("planner.sqlite-shm")?,
    };
    let journal = root.seal_leaf("planner.sqlite-journal")?;
    if journal.is_some() {
        return Err(recovery_required());
    }
    if physical
        .wal
        .as_ref()
        .is_some_and(|f| f.length() > 1_073_741_824)
        || physical
            .shm
            .as_ref()
            .is_some_and(|f| f.length() > 33_554_432)
    {
        return Err(storage("sidecar bound"));
    }
    let header = physical.main.header()?;
    if &header[..16] != b"SQLite format 3\0" || !matches!((header[18], header[19]), (1, 1) | (2, 2))
    {
        return Err(storage("invalid main header"));
    }
    let (mut inspection, opened) = match (&physical.wal, &physical.shm) {
        (None, None) => {
            let dir = root.create_preflight_dir()?;
            let file = physical.main.copy_to(dir.reserve_inspection()?)?;
            physical.reverify(root)?;
            let c = reader(file.path(), true);
            (Some((dir, file)), c)
        }
        (Some(_), Some(_)) if header[18] == 2 => (
            None,
            reader(physical.main.path(), false).map_err(|_| recovery_required()),
        ),
        _ => return Err(recovery_required()),
    };
    let c = match opened {
        Ok(c) => c,
        Err(e) => {
            physical.reverify(root)?;
            if let Some((_, file)) = &mut inspection {
                file.reverify()?;
            }
            return Err(e);
        }
    };
    let audited = catalogue.audit(&c);
    let closed = c.close().map_err(|(_, e)| storage(e));
    // Teardown and custody comparison are required on both success and logical refusal.
    physical.reverify(root)?;
    if let Some((_, file)) = &mut inspection {
        file.reverify()?;
    }
    closed?;
    if began.elapsed() > Duration::from_secs(120) {
        return Err(storage("preflight deadline"));
    }
    Ok(RecognisedStartup {
        state: audited?,
        physical,
        _inspection: inspection,
    })
}

pub(crate) fn open(
    root: &TrustedStoreRoot,
    lock: &StoreLock,
    catalogue: Catalogue,
) -> Result<(Connection, GuardedLive), DomainError> {
    let mut token = recognise(root, lock, catalogue)?;
    token.physical.reverify(root)?;
    let live = root.live()?;
    lock.verify_for(root)?;
    if live.identity() != token.physical.main.identity() {
        return Err(storage("recognised live identity changed"));
    }
    let wal = token
        .physical
        .wal
        .as_ref()
        .map(SealedFile::permissive_guard)
        .transpose()?;
    let shm = token
        .physical
        .shm
        .as_ref()
        .map(SealedFile::permissive_guard)
        .transpose()?;
    let expected = token.state;
    drop(token.physical);
    let c = writable_query_only(live.path())?;
    if catalogue.audit(&c)? != expected {
        return Err(storage("recognised source changed"));
    }
    drop((wal, shm));
    configure(&c)?;
    Ok((c, live))
}

#[cfg(test)]
pub(crate) fn inspect_for_test(
    root: &TrustedStoreRoot,
    lock: &StoreLock,
    catalogue: Catalogue,
) -> Result<(AuditedRows, Option<std::path::PathBuf>), DomainError> {
    let token = recognise(root, lock, catalogue)?;
    Ok((
        token.state,
        token._inspection.as_ref().map(|(_, f)| f.path().to_owned()),
    ))
}

pub(crate) fn writable_query_only(path: &Path) -> Result<Connection, DomainError> {
    let c = Connection::open_with_flags_and_vfs(
        path,
        OpenFlags::SQLITE_OPEN_READ_WRITE
            | OpenFlags::SQLITE_OPEN_PRIVATE_CACHE
            | OpenFlags::SQLITE_OPEN_NO_MUTEX,
        "win32",
    )
    .map_err(storage)?;
    c.busy_timeout(Duration::from_millis(2000))
        .map_err(storage)?;
    c.execute_batch("PRAGMA query_only=ON; PRAGMA temp_store=MEMORY; PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF;").map_err(storage)?;
    Ok(c)
}
pub(crate) fn configure(c: &Connection) -> Result<(), DomainError> {
    c.execute_batch("PRAGMA query_only=OFF; PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF;").map_err(storage)?;
    let mode: String = c
        .pragma_query_value(None, "journal_mode", |r| r.get(0))
        .map_err(storage)?;
    if mode != "wal" {
        return Err(storage("WAL unavailable"));
    }
    for (p, want) in [
        ("synchronous", 2),
        ("foreign_keys", 1),
        ("trusted_schema", 0),
        ("busy_timeout", 2000),
        ("query_only", 0),
    ] {
        let actual: i64 = c
            .pragma_query_value(None, p, |r| r.get(0))
            .map_err(storage)?;
        if actual != want {
            return Err(storage("writer settings"));
        }
    }
    Ok(())
}
