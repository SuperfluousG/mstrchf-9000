use crate::{
    custody::{GuardedLive, StoreLock, TrustedStoreRoot},
    records::*,
    recovery, snapshot,
    startup::{self, Catalogue},
};
use mstrchf_contracts::*;
use rusqlite::Connection;
use std::path::{Path, PathBuf};

pub(crate) struct Database {
    pub(crate) connection: Connection,
    pub(crate) path: PathBuf,
    pub(crate) live: GuardedLive,
    _lock: StoreLock,
    pub(crate) root: TrustedStoreRoot,
    pub(crate) recovery_only: bool,
}
impl Database {
    #[cfg(test)]
    pub(crate) fn invalidate_lock_for_test(&mut self) {
        self._lock.invalidate_identity_for_test();
    }
    pub(crate) fn verify_owner(&self) -> Result<(), DomainError> {
        self._lock.reverify()?;
        self.live.reverify()
    }
    pub(crate) fn open(path: &Path) -> Result<Self, DomainError> {
        Self::open_catalogue(path, Catalogue::B1)
    }
    pub(crate) fn open_catalogue(path: &Path, catalogue: Catalogue) -> Result<Self, DomainError> {
        let root = TrustedStoreRoot::open_or_create(path)?;
        let lock = root.acquire_store_lock()?;
        // Only actual native absence permits the separate initialisation protocol.
        if root.seal_leaf("planner.sqlite")?.is_none() {
            recovery::prepare(root.path())?;
        }
        let (connection, live) = startup::open(&root, &lock, catalogue)?;
        let path = live.path().to_owned();
        Ok(Self {
            connection,
            path,
            live,
            _lock: lock,
            root,
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
            self.verify_owner()?;
            self.connection = startup::writable_query_only(&self.path)?;
            // Preserve B1 command recovery: this retained owner is not fresh
            // startup recognition or a verified-backup maintenance session.
            crate::migrate::verify(&self.connection)?;
            startup::configure(&self.connection)?;
            let receipt = crate::receipts::lookup(&self.connection, command, bytes)?;
            self.recovery_only = false;
            if let Some(response) = receipt {
                return Ok(response);
            }
        }
        result
    }
}
