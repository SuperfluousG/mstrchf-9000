use crate::{migrate, records::*, recovery, snapshot};
use mstrchf_contracts::*;
use rusqlite::{Connection, OpenFlags};
use std::{
    fs::{File, OpenOptions},
    path::{Path, PathBuf},
    time::Duration,
};

pub(crate) struct Database {
    pub(crate) connection: Connection,
    pub(crate) path: PathBuf,
    _lock: File,
    recovery_only: bool,
}
pub(crate) fn connect(path: &Path) -> Result<Connection, DomainError> {
    let ro =
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_ONLY).map_err(storage)?;
    ro.execute_batch("PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF;")
        .map_err(storage)?;
    migrate::verify(&ro)?;
    ro.close().map_err(|(_, e)| storage(e))?;
    let c =
        Connection::open_with_flags(path, OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(storage)?;
    if rusqlite::version_number() < 3_051_003 {
        return Err(storage("unpatched SQLite"));
    }
    c.busy_timeout(Duration::from_millis(2000))
        .map_err(storage)?;
    c.execute_batch("PRAGMA journal_mode=WAL; PRAGMA synchronous=FULL; PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF;").map_err(storage)?;
    let mode: String = c
        .pragma_query_value(None, "journal_mode", |r| r.get(0))
        .map_err(storage)?;
    for (pragma, want) in [
        ("synchronous", 2),
        ("foreign_keys", 1),
        ("trusted_schema", 0),
        ("busy_timeout", 2000),
    ] {
        let got: i64 = c
            .pragma_query_value(None, pragma, |r| r.get(0))
            .map_err(storage)?;
        if got != want {
            return Err(storage("SQLite settings"));
        }
    }
    if mode != "wal" {
        return Err(storage("WAL unavailable"));
    }
    Ok(c)
}
impl Database {
    pub(crate) fn open(root: &Path) -> Result<Self, DomainError> {
        std::fs::create_dir_all(root).map_err(storage)?;
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(root.join("store.lock"))
            .map_err(storage)?;
        lock.try_lock().map_err(storage)?;
        recovery::prepare(root)?;
        let path = root.join("planner.sqlite");
        let connection = connect(&path)?;
        Ok(Self {
            connection,
            path,
            _lock: lock,
            recovery_only: false,
        })
    }
    pub(crate) fn snapshot(&mut self) -> Result<Snapshot, DomainError> {
        if self.recovery_only {
            return Err(storage("recovery required"));
        }
        snapshot::read(&mut self.connection)
    }
    pub(crate) fn dispatch(
        &mut self,
        command: &AppCommandV1,
        bytes: &[u8],
    ) -> Result<Vec<u8>, DomainError> {
        if self.recovery_only {
            return Err(storage("recovery required"));
        }
        let result = crate::commands::execute(&mut self.connection, command, bytes);
        if result
            .as_ref()
            .is_err_and(|e| e.code == ErrorCode::StorageFailure)
        {
            // Never blindly reapply an uncertain command. Close and recover SQLite,
            // retaining the app lock, then look up the exact durable binding.
            self.recovery_only = true;
            let placeholder = Connection::open_in_memory().map_err(storage)?;
            let old = std::mem::replace(&mut self.connection, placeholder);
            old.close().map_err(|(_, e)| storage(e))?;
            #[cfg(test)]
            if crate::tests::recovery_unavailable() {
                return Err(storage("injected reopen failure"));
            }
            self.connection = connect(&self.path)?;
            let receipt = crate::receipts::lookup(&self.connection, command, bytes)?;
            self.recovery_only = false;
            if let Some(response) = receipt {
                return Ok(response);
            }
        }
        result
    }
}
