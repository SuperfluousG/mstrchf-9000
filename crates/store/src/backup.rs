//! Private foundation. No production successor is allocated; no public maintenance API.
#![allow(dead_code)]
use crate::{
    audit::AuditedRows, custody::*, database::Database, records::storage, startup::Catalogue,
};
use mstrchf_contracts::{DomainError, bytes_digest, canonical_bytes};
use rusqlite::{
    Connection, OpenFlags,
    backup::{Backup, StepResult},
};
use serde::{Deserialize, Serialize};
use std::{
    marker::PhantomData,
    rc::Rc,
    time::{Duration, Instant},
};

fn limit() -> DomainError {
    DomainError::new(
        mstrchf_contracts::ErrorCode::LimitExceeded,
        "/storage/backup",
    )
}
fn asset_budget(count: usize, total: u64, next: u64) -> Result<u64, DomainError> {
    let sum = total.checked_add(next).ok_or_else(limit)?;
    if count > 10_000 || next > 33_554_432 || sum > 1_073_741_824 {
        Err(limit())
    } else {
        Ok(sum)
    }
}
#[derive(Default)]
struct StepPolicy {
    busy: u8,
}
impl StepPolicy {
    fn observe(&mut self, step: StepResult) -> Result<bool, DomainError> {
        match step {
            StepResult::Done => Ok(true),
            StepResult::More => Ok(false),
            StepResult::Busy if self.busy < 3 => {
                self.busy += 1;
                Ok(false)
            }
            _ => Err(storage("backup step refused")),
        }
    }
}

fn point(_name: &str) -> Result<(), DomainError> {
    #[cfg(test)]
    {
        crate::backup_upgrade_tests::barrier(_name);
        if crate::backup_upgrade_tests::fault(_name) {
            return Err(storage("injected backup failure"));
        }
    }
    Ok(())
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Phase {
    Auditing,
    Copying,
    Verified,
    Upgrading,
    Finished,
    Poisoned,
}
pub(crate) struct Maintenance<'db> {
    pub(super) database: &'db mut Database,
    id: String,
    pub(super) source: AuditedRows,
    data_version: i64,
    pub(super) catalogue: Catalogue,
    intent: Intent,
    phase: Phase,
    _not_send: PhantomData<Rc<()>>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct Intent {
    from: u32,
    target: u32,
    catalogue_sha256: String,
}
impl<'db> Maintenance<'db> {
    pub(crate) fn begin(
        database: &'db mut Database,
        catalogue: Catalogue,
    ) -> Result<Self, DomainError> {
        database.verify_owner()?;
        let source = catalogue.audit(&database.connection)?;
        let version: u32 = database
            .connection
            .pragma_query_value(None, "user_version", |r| r.get(0))
            .map_err(storage)?;
        let target = match catalogue {
            Catalogue::B1 => 1,
            #[cfg(test)]
            Catalogue::Fixture => 2,
            #[cfg(test)]
            Catalogue::ReaderUnavailable => return Err(storage("reader unavailable")),
            #[cfg(test)]
            Catalogue::AssetFixture => 1,
        };
        let catalogue_sha256 = catalogue_digest(catalogue)?;
        let data_version = data_version(&database.connection)?;
        let mut id = None;
        for _ in 0..3 {
            let mut bytes = [0u8; 16];
            getrandom::fill(&mut bytes).map_err(storage)?;
            if bytes.iter().any(|b| *b != 0) {
                id = Some(bytes.iter().map(|b| format!("{b:02x}")).collect());
                break;
            }
        }
        Ok(Self {
            database,
            id: id.ok_or_else(|| storage("maintenance RNG exhausted"))?,
            source,
            data_version,
            catalogue,
            intent: Intent {
                from: version,
                target,
                catalogue_sha256,
            },
            phase: Phase::Auditing,
            _not_send: PhantomData,
        })
    }
    pub(crate) fn backup(&mut self) -> Result<VerifiedBackupReceipt, DomainError> {
        if self.phase != Phase::Auditing {
            return Err(storage("maintenance phase"));
        }
        self.phase = Phase::Copying;
        let result = create(self);
        self.phase = if result.is_ok() {
            Phase::Verified
        } else {
            Phase::Poisoned
        };
        result
    }
    pub(crate) fn consume(
        &mut self,
        mut receipt: VerifiedBackupReceipt,
    ) -> Result<VerifiedBackupReceipt, DomainError> {
        if self.phase != Phase::Verified
            || receipt.maintenance_id != self.id
            || receipt.source != self.source
            || receipt.intent != self.intent
            || receipt.root_identity != self.database.root.identity()
            || receipt.live_identity != self.database.live.identity()
            || receipt.data_version != self.data_version
        {
            return Err(storage("receipt binding"));
        }
        receipt.database.reverify()?;
        receipt.manifest.reverify()?;
        for asset in &mut receipt.assets {
            asset.reverify()?;
        }
        self.check_source()?;
        self.phase = Phase::Upgrading;
        Ok(receipt)
    }
    pub(crate) fn check_source(&self) -> Result<(), DomainError> {
        self.database.live.reverify()?;
        if data_version(&self.database.connection)? != self.data_version
            || self.catalogue.audit(&self.database.connection)? != self.source
        {
            return Err(storage("source changed"));
        }
        Ok(())
    }
    pub(crate) fn source_data_version(&self) -> i64 {
        self.data_version
    }
    pub(crate) fn finished(&mut self) {
        self.phase = Phase::Finished;
    }
    pub(crate) fn poison(&mut self) {
        self.phase = Phase::Poisoned;
        self.database.recovery_only = true;
    }
}

pub(crate) struct VerifiedBackupReceipt {
    maintenance_id: String,
    root_identity: FileIdentity,
    live_identity: FileIdentity,
    source: AuditedRows,
    data_version: i64,
    intent: Intent,
    database: SealedFile,
    manifest: SealedFile,
    assets: Vec<SealedFile>,
    directory: PrivateBackupDir,
    _not_send: PhantomData<Rc<()>>,
}
impl VerifiedBackupReceipt {
    #[cfg(test)]
    pub(crate) fn path(&self) -> &std::path::Path {
        self.database.path()
    }
}
pub(crate) fn data_version(c: &Connection) -> Result<i64, DomainError> {
    c.pragma_query_value(None, "data_version", |r| r.get(0))
        .map_err(storage)
}
fn catalogue_digest(catalogue: Catalogue) -> Result<String, DomainError> {
    #[derive(Serialize)]
    struct Entry {
        version: u32,
        sql_sha256: String,
        reader: &'static str,
        closure: &'static str,
    }
    let entries = match catalogue {
        Catalogue::B1 => vec![Entry {
            version: 1,
            sql_sha256: bytes_digest(crate::migrate::SQL.as_bytes()).0,
            reader: "b1-contracts003",
            closure: "b1-v1-no-assets",
        }],
        #[cfg(test)]
        Catalogue::Fixture => vec![
            Entry {
                version: 1,
                sql_sha256: bytes_digest(crate::audit::fixture_source_sql().as_bytes()).0,
                reader: "b1-contracts003",
                closure: "test-fixture",
            },
            Entry {
                version: 2,
                sql_sha256: bytes_digest(crate::audit::FIXTURE_V1_TO_V2_SQL.as_bytes()).0,
                reader: "b1-contracts003",
                closure: "test-fixture",
            },
        ],
        #[cfg(test)]
        Catalogue::ReaderUnavailable => return Err(storage("reader unavailable")),
        #[cfg(test)]
        Catalogue::AssetFixture => vec![Entry {
            version: 1,
            sql_sha256: bytes_digest(crate::audit::asset_source_sql().as_bytes()).0,
            reader: "b1-contracts003",
            closure: "test-fixture-single-asset",
        }],
    };
    Ok(bytes_digest(&canonical_bytes(&entries)?).0)
}

#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct IdentityRecord {
    volume_serial: String,
    file_id: String,
}
impl From<FileIdentity> for IdentityRecord {
    fn from(id: FileIdentity) -> Self {
        Self {
            volume_serial: id.volume_serial.to_string(),
            file_id: id.file_id.iter().map(|b| format!("{b:02x}")).collect(),
        }
    }
}
#[derive(Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct FileRecord {
    name: String,
    identity: IdentityRecord,
    length: u64,
    sha256: String,
}
#[derive(Serialize)]
struct Manifest<'a> {
    format: &'static str,
    backup_id: String,
    root_identity: IdentityRecord,
    directory_identity: IdentityRecord,
    source: SourceRecord<'a>,
    intent: &'a Intent,
    database: FileRecord,
    assets: Vec<FileRecord>,
}
#[derive(Serialize)]
struct SourceRecord<'a> {
    live_identity: IdentityRecord,
    lineage: &'a str,
    storage_version: u32,
    reader: &'a str,
    closure: &'a str,
    counters: &'a crate::audit::Counters,
    ledger: &'a [crate::audit::LedgerEntry],
    schema_sha256: &'a mstrchf_contracts::Digest,
    contents_sha256: &'a mstrchf_contracts::Digest,
    snapshot_sha256: &'a mstrchf_contracts::Digest,
}

fn create(m: &mut Maintenance<'_>) -> Result<VerifiedBackupReceipt, DomainError> {
    let began = Instant::now();
    m.check_source()?;
    let directory = m.database.root.create_backup_dir()?;
    point("bu_after_directory")?;
    let reserved = directory.reserve_database()?;
    point("bu_after_reserve")?;
    let mut destination = Connection::open_with_flags_and_vfs(
        reserved.sqlite_path(),
        OpenFlags::SQLITE_OPEN_READ_WRITE | OpenFlags::SQLITE_OPEN_PRIVATE_CACHE,
        "win32",
    )
    .map_err(storage)?;
    destination
        .execute_batch(
            "PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA temp_store=MEMORY;",
        )
        .map_err(storage)?;
    {
        let tx = m.database.connection.transaction().map_err(storage)?;
        let captured = m.catalogue.audit(&tx)?;
        if captured != m.source {
            return Err(storage("snapshot source changed"));
        }
        {
            let backup = Backup::new(&tx, &mut destination).map_err(storage)?;
            let mut policy = StepPolicy::default();
            let mut first = true;
            loop {
                if began.elapsed() > Duration::from_secs(120) {
                    return Err(storage("backup deadline"));
                }
                let step = backup.step(128).map_err(storage)?;
                policy.observe(step)?;
                match step {
                    StepResult::Done => {
                        if backup.progress().remaining != 0 {
                            return Err(storage("incomplete backup"));
                        }
                        point("bu_after_done")?;
                        break;
                    }
                    StepResult::More => {
                        if first {
                            point("bu_after_first_step")?;
                            first = false;
                        }
                    }
                    StepResult::Busy => {
                        std::thread::sleep(Duration::from_millis(25));
                    }
                    _ => return Err(storage("backup step refused")),
                }
            }
        }
        tx.commit().map_err(storage)?;
    }
    // Source read transaction is ended before any destination hashing/manifest work.
    if data_version(&m.database.connection)? != m.data_version {
        return Err(storage("source changed during copy"));
    }
    let mode: String = destination
        .pragma_query_value(None, "journal_mode", |r| r.get(0))
        .map_err(storage)?;
    if mode == "wal" {
        let (busy, total, done): (i64, i64, i64) = destination
            .query_row("PRAGMA wal_checkpoint(TRUNCATE)", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?))
            })
            .map_err(storage)?;
        if busy != 0 || (total >= 0 && total != done) {
            return Err(storage("destination checkpoint"));
        }
    }
    let mode: String = destination
        .query_row("PRAGMA journal_mode=DELETE", [], |r| r.get(0))
        .map_err(storage)?;
    if mode != "delete" {
        return Err(storage("destination journal mode"));
    }
    destination.close().map_err(|(_, e)| storage(e))?;
    #[cfg(test)]
    if crate::backup_upgrade_tests::fault("corrupt_destination") {
        use std::io::Write;
        let mut file = std::fs::OpenOptions::new()
            .write(true)
            .open(reserved.sqlite_path())
            .map_err(storage)?;
        file.write_all(b"invalid synthetic copied database")
            .map_err(storage)?;
        file.sync_all().map_err(storage)?;
    }
    let database = reserved.seal_after_close()?;
    let read = Connection::open_with_flags_and_vfs(
        database.path(),
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_PRIVATE_CACHE,
        "win32",
    )
    .map_err(storage)?;
    read.execute_batch("PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF; PRAGMA query_only=ON; PRAGMA temp_store=MEMORY;").map_err(storage)?;
    let audited = m.catalogue.audit(&read);
    read.close().map_err(|(_, e)| storage(e))?;
    if audited? != m.source {
        return Err(storage("backup content mismatch"));
    }
    let mut assets = Vec::new();
    let mut asset_records = Vec::new();
    let mut total = 0u64;
    if m.source.asset_refs.len() > 10_000 {
        return Err(limit());
    }
    for reference in &m.source.asset_refs {
        total = asset_budget(m.source.asset_refs.len(), total, reference.length)?;
        let mut source = m
            .database
            .root
            .seal_asset(&reference.digest, &reference.extension)?;
        if source.length() != reference.length || source.sha256() != reference.digest {
            return Err(storage("asset mismatch"));
        }
        let mut asset =
            source.copy_to(directory.reserve_asset(&reference.digest, &reference.extension)?)?;
        asset.reverify()?;
        source.reverify()?;
        point("bu_after_asset_copy")?;
        asset_records.push(FileRecord {
            name: format!("assets/{}.{}", reference.digest, reference.extension),
            identity: asset.identity().into(),
            length: asset.length(),
            sha256: asset.sha256().into(),
        });
        assets.push(asset);
    }
    point("bu_after_closure")?;
    let manifest_bytes = canonical_bytes(&Manifest {
        format: "mstrchf-private-backup/1",
        backup_id: directory.id().into(),
        root_identity: m.database.root.identity().into(),
        directory_identity: directory.identity().into(),
        source: SourceRecord {
            live_identity: m.database.live.identity().into(),
            lineage: &m.source.lineage,
            storage_version: m.source.version,
            reader: m.source.reader,
            closure: m.source.closure,
            counters: &m.source.counters,
            ledger: &m.source.ledger,
            schema_sha256: &m.source.schema_sha256,
            contents_sha256: &m.source.contents_sha256,
            snapshot_sha256: &m.source.snapshot_sha256,
        },
        intent: &m.intent,
        database: FileRecord {
            name: "database.sqlite".into(),
            identity: database.identity().into(),
            length: database.length(),
            sha256: database.sha256().to_owned(),
        },
        assets: asset_records,
    })?;
    if manifest_bytes.len() > 2_097_152 {
        return Err(limit());
    }
    let manifest = directory.write_manifest(&manifest_bytes)?;
    m.check_source()?;
    if began.elapsed() > Duration::from_secs(120) {
        return Err(storage("backup deadline"));
    }
    let receipt = VerifiedBackupReceipt {
        maintenance_id: m.id.clone(),
        root_identity: m.database.root.identity(),
        live_identity: m.database.live.identity(),
        source: m.source.clone(),
        data_version: m.data_version,
        intent: m.intent.clone(),
        database,
        manifest,
        assets,
        directory,
        _not_send: PhantomData,
    };
    point("bu_after_verified")?;
    Ok(receipt)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn finite_asset_limits_and_step_refusals() {
        assert_eq!(
            asset_budget(10_000, 1_040_187_392, 33_554_432).unwrap(),
            1_073_741_824
        );
        for (count, total, next) in [
            (10_001, 0, 1),
            (1, 0, 33_554_433),
            (1, 1_073_741_824, 1),
            (1, u64::MAX, 1),
        ] {
            assert_eq!(
                asset_budget(count, total, next).unwrap_err().code,
                mstrchf_contracts::ErrorCode::LimitExceeded
            );
        }
        let mut policy = StepPolicy::default();
        assert!(!policy.observe(StepResult::More).unwrap());
        for _ in 0..3 {
            assert!(!policy.observe(StepResult::Busy).unwrap());
        }
        assert!(policy.observe(StepResult::Busy).is_err());
        assert!(policy.observe(StepResult::Locked).is_err());
        assert!(StepPolicy::default().observe(StepResult::Done).unwrap());
    }
}
