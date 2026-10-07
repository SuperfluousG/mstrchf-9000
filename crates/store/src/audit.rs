//! Closed, read-only B1 custody audit. Callers retain a stable SQLite snapshot
//! (or native deny-write custody) for the whole call. Nothing here grants custody.
#[cfg(test)]
use crate::snapshot;
use crate::{
    compat::{self, Profile},
    migrate,
    records::storage,
};
use mstrchf_contracts::*;
use rusqlite::{Connection, Row, types::ValueRef};
use serde::Serialize;
use sha2::{Digest as ShaDigest, Sha256};
use std::time::Instant as Clock;

const MAX_DB_BYTES: u64 = 1_073_741_824;
const AUDIT_SECONDS: u64 = 120;
const INTERNAL_SCHEMA_GLOB: &str = "sqlite_*";

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(super) struct Counters {
    pub(super) snapshot: Revision,
    pub(super) settings: Revision,
    pub(super) history: Revision,
    pub(super) catalogue: Revision,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(super) struct LedgerEntry {
    pub(super) version: u32,
    pub(super) sql_sha256: Digest,
    pub(super) applied_utc: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(super) struct AssetRef {
    pub(super) digest: String,
    pub(super) extension: String,
    pub(super) length: u64,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
pub(super) struct AuditedRows {
    pub(super) version: u32,
    pub(super) reader: &'static str,
    pub(super) closure: &'static str,
    pub(super) counters: Counters,
    pub(super) lineage: String,
    pub(super) ledger: Vec<LedgerEntry>,
    pub(super) schema_sha256: Digest,
    pub(super) contents_sha256: Digest,
    pub(super) snapshot_sha256: Digest,
    // Production B1 always supplies an empty vector. Only the closed test
    // catalogue below can produce a reference; no filesystem path is accepted.
    pub(super) asset_refs: Vec<AssetRef>,
    pub(super) row_counts: Vec<(&'static str, u64)>,
}

#[cfg(test)]
pub(super) const FIXTURE_PROBE_SQL: &str = "CREATE TABLE fixture_probe(id INTEGER PRIMARY KEY CHECK(id=1), value INTEGER NOT NULL) STRICT;\n";
#[cfg(test)]
pub(super) const FIXTURE_V1_TO_V2_SQL: &str =
    include_str!("../tests/fixtures/backup-upgrade/fixture_v1_to_v2.sql");
#[cfg(test)]
pub(super) fn fixture_source_sql() -> String {
    include_str!("../tests/fixtures/backup-upgrade/fixture_v1.sql").to_owned()
}
#[cfg(test)]
pub(super) const ASSET_FIXTURE_SQL: &str = "CREATE TABLE fixture_assets(digest TEXT PRIMARY KEY, extension TEXT NOT NULL,length INTEGER NOT NULL) STRICT;\n";
#[cfg(test)]
pub(super) const ASSET_FIXTURE_BYTES: &[u8; 21] = b"backup-asset-fixture\n";
#[cfg(test)]
pub(super) fn asset_source_sql() -> String {
    format!("{}{}", migrate::SQL, ASSET_FIXTURE_SQL)
}
#[cfg(test)]
pub(super) const FK_FIXTURE_SQL: &str = "CREATE TABLE fixture_parent(id INTEGER PRIMARY KEY) STRICT;\nCREATE TABLE fixture_child(id INTEGER PRIMARY KEY,parent_id INTEGER NOT NULL REFERENCES fixture_parent(id)) STRICT;\n";
#[cfg(test)]
pub(super) fn fk_source_sql() -> String {
    format!("{}{}", migrate::SQL, FK_FIXTURE_SQL)
}
#[cfg(test)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(super) enum FixtureVersion {
    V1,
    V2,
}

#[derive(Clone, Copy)]
enum Catalogue {
    B1,
    Draft005,
    #[cfg(test)]
    Fixture(FixtureVersion),
    #[cfg(test)]
    AssetFixture,
    #[cfg(test)]
    FkFixture,
}
impl Catalogue {
    fn version(self) -> u32 {
        match self {
            Self::B1 => 1,
            Self::Draft005 => 2,
            #[cfg(test)]
            Self::Fixture(FixtureVersion::V1) => 1,
            #[cfg(test)]
            Self::Fixture(FixtureVersion::V2) => 2,
            #[cfg(test)]
            Self::AssetFixture => 1,
            #[cfg(test)]
            Self::FkFixture => 1,
        }
    }
    fn expected(self) -> Result<Connection, DomainError> {
        let c = Connection::open_in_memory().map_err(storage)?;
        c.execute_batch(migrate::SQL).map_err(storage)?;
        if let Self::Draft005 = self {
            c.execute_batch(migrate::SQL_002).map_err(storage)?;
        }
        #[cfg(test)]
        if let Self::Fixture(version) = self {
            c.execute_batch(FIXTURE_PROBE_SQL).map_err(storage)?;
            if version == FixtureVersion::V2 {
                c.execute_batch(FIXTURE_V1_TO_V2_SQL).map_err(storage)?;
            }
        }
        #[cfg(test)]
        if let Self::AssetFixture = self {
            c.execute_batch(ASSET_FIXTURE_SQL).map_err(storage)?;
        }
        #[cfg(test)]
        if let Self::FkFixture = self {
            c.execute_batch(FK_FIXTURE_SQL).map_err(storage)?;
        }
        Ok(c)
    }
    fn checksums(self) -> Vec<Digest> {
        match self {
            Self::B1 => vec![bytes_digest(migrate::SQL.as_bytes())],
            Self::Draft005 => vec![
                bytes_digest(migrate::SQL.as_bytes()),
                bytes_digest(migrate::SQL_002.as_bytes()),
            ],
            #[cfg(test)]
            Self::Fixture(version) => {
                let mut out = vec![bytes_digest(fixture_source_sql().as_bytes())];
                if version == FixtureVersion::V2 {
                    out.push(bytes_digest(FIXTURE_V1_TO_V2_SQL.as_bytes()));
                }
                out
            }
            #[cfg(test)]
            Self::AssetFixture => vec![bytes_digest(asset_source_sql().as_bytes())],
            #[cfg(test)]
            Self::FkFixture => vec![bytes_digest(fk_source_sql().as_bytes())],
        }
    }
    fn closure(self) -> &'static str {
        match self {
            #[cfg(test)]
            Self::AssetFixture => "test-fixture-single-asset",
            _ => "b1-v1-no-assets",
        }
    }
}

pub(super) fn audit_b1(c: &Connection) -> Result<AuditedRows, DomainError> {
    let version: i64 = c
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(storage)?;
    match version {
        1 => audit(c, Catalogue::B1),
        2 => audit(c, Catalogue::Draft005),
        _ => Err(DomainError::new(
            ErrorCode::UnsupportedVersion,
            "/storage_version",
        )),
    }
}

#[cfg(test)]
pub(super) fn audit_fixture(
    c: &Connection,
    version: FixtureVersion,
) -> Result<AuditedRows, DomainError> {
    audit(c, Catalogue::Fixture(version))
}

#[cfg(test)]
pub(super) fn audit_asset_fixture(c: &Connection) -> Result<AuditedRows, DomainError> {
    audit(c, Catalogue::AssetFixture)
}

#[cfg(test)]
pub(super) fn audit_fk_fixture(c: &Connection) -> Result<AuditedRows, DomainError> {
    audit(c, Catalogue::FkFixture)
}

fn fail() -> DomainError {
    storage("closed row audit")
}
fn limit() -> DomainError {
    DomainError::new(ErrorCode::LimitExceeded, "/storage/backup")
}
fn closure() -> DomainError {
    DomainError::new(ErrorCode::UnsupportedCapability, "/storage/backup/closure")
}
fn budget(started: Clock) -> Result<(), DomainError> {
    if started.elapsed().as_secs() >= AUDIT_SECONDS {
        Err(storage("audit deadline"))
    } else {
        Ok(())
    }
}

pub(super) fn database_size(pages: i64, page_size: i64) -> Result<u64, DomainError> {
    let pages = u64::try_from(pages).map_err(|_| limit())?;
    let page_size = u64::try_from(page_size).map_err(|_| limit())?;
    let bytes = pages.checked_mul(page_size).ok_or_else(limit)?;
    if pages == 0 || page_size == 0 || bytes > MAX_DB_BYTES {
        return Err(limit());
    }
    Ok(bytes)
}

type SchemaRows = Vec<(String, String, String)>;
pub(super) fn schema(c: &Connection) -> Result<SchemaRows, DomainError> {
    let mut statement = c
        .prepare(
            "SELECT type,name,sql FROM sqlite_schema WHERE name NOT GLOB ?1 ORDER BY type,name",
        )
        .map_err(storage)?;
    let mut rows = statement.query([INTERNAL_SCHEMA_GLOB]).map_err(storage)?;
    let mut result = Vec::new();
    while let Some(row) = rows.next().map_err(storage)? {
        if result.len() >= 32 {
            return Err(closure());
        }
        let kind = text(row, 0)?;
        let name = text(row, 1)?;
        let sql = text(row, 2)?;
        if kind.len() > 16 || name.len() > 128 || sql.len() > 65_536 {
            return Err(closure());
        }
        result.push((kind.into(), name.into(), sql.into()));
    }
    Ok(result)
}

pub(super) fn integrity(c: &Connection) -> Result<(), DomainError> {
    let mut statement = c.prepare("PRAGMA integrity_check").map_err(storage)?;
    let mut rows = statement.query([]).map_err(storage)?;
    let first = rows.next().map_err(storage)?.ok_or_else(fail)?;
    if text(first, 0)? != "ok" || rows.next().map_err(storage)?.is_some() {
        return Err(fail());
    }
    let mut statement = c.prepare("PRAGMA foreign_key_check").map_err(storage)?;
    if statement
        .query([])
        .map_err(storage)?
        .next()
        .map_err(storage)?
        .is_some()
    {
        return Err(fail());
    }
    Ok(())
}

fn audit(c: &Connection, catalogue: Catalogue) -> Result<AuditedRows, DomainError> {
    let started = Clock::now();
    let profile = if matches!(catalogue, Catalogue::B1) {
        Profile::Old
    } else {
        Profile::Current
    };
    let version: i64 = c
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(storage)?;
    if version > i64::from(catalogue.version()) {
        return Err(DomainError::new(
            ErrorCode::UnsupportedVersion,
            "/storage_version",
        ));
    }
    if version != i64::from(catalogue.version()) {
        return Err(fail());
    }
    let page_count = c
        .pragma_query_value(None, "page_count", |r| r.get(0))
        .map_err(storage)?;
    let page_size = c
        .pragma_query_value(None, "page_size", |r| r.get(0))
        .map_err(storage)?;
    database_size(page_count, page_size)?;
    let actual_schema = schema(c)?;
    if actual_schema != schema(&catalogue.expected()?)? {
        return Err(fail());
    }
    integrity(c)?;
    if matches!(catalogue, Catalogue::Draft005) {
        metadata(c)?;
    }
    budget(started)?;

    let (counters, lineage) = meta(c)?;
    let household_id: String = c
        .query_row(
            "SELECT household_id FROM household WHERE singleton=1",
            [],
            |r| r.get(0),
        )
        .map_err(storage)?;
    compat::scalar("HouseholdId", &household_id)?;
    let mut hasher = Sha256::new();
    hasher.update(b"MSTRCHF9000-STORE-CONTENTS-v1");
    let mut ledger = Vec::new();
    let checksums = catalogue.checksums();
    let mut counts = Vec::new();
    #[allow(unused_mut)]
    let mut asset_refs = Vec::new();
    for (table, sql, columns) in table_queries(catalogue) {
        hash_bytes(&mut hasher, table.as_bytes())?;
        let mut statement = c.prepare(sql).map_err(storage)?;
        let mut rows = statement.query([]).map_err(storage)?;
        let mut count = 0u64;
        while let Some(row) = rows.next().map_err(storage)? {
            budget(started)?;
            count = count.checked_add(1).ok_or_else(limit)?;
            hasher.update([0x52]);
            for col in 0..columns {
                hash_value(&mut hasher, row.get_ref(col).map_err(storage)?)?;
            }
            match table {
                "app_meta" => {
                    if count != 1 || integer(row, 0)? != 1 {
                        return Err(fail());
                    }
                }
                "household" => audit_household_frozen(row, &household_id, &counters, profile)?,
                "jobs" => audit_job_frozen(
                    profile,
                    text(row, 0)?,
                    integer(row, 1)?,
                    integer(row, 2)?,
                    blob(row, 3)?,
                )?,
                "command_receipts" => {
                    let bound = if matches!(catalogue, Catalogue::Draft005) {
                        let tag: String = c
                            .query_row(
                                "SELECT profile FROM receipt_formats WHERE command_id=?1",
                                [text(row, 0)?],
                                |r| r.get(0),
                            )
                            .map_err(storage)?;
                        Profile::bound(&tag)?
                    } else {
                        profile
                    };
                    compat::receipt(
                        bound,
                        text(row, 0)?,
                        &household_id,
                        text(row, 5)?,
                        &counters.snapshot.0.to_string(),
                        text(row, 1)?,
                        text(row, 4)?,
                        blob(row, 2)?,
                        blob(row, 3)?,
                    )?;
                }
                "store_format" => {
                    if count != 1 || integer(row, 0)? != 1 || text(row, 1)? != "B1Draft005" {
                        return Err(fail());
                    }
                }
                "receipt_formats" => {
                    compat::scalar("CommandId", text(row, 0)?)?;
                    Profile::bound(text(row, 1)?)?;
                }
                "resource_quarantine" => {
                    // B1 has no resource producer and no filesystem resolver.
                    // The opaque key participates in the exact raw digest only.
                    let _ = opaque_text(row, 0)?;
                    compat::scalar("JobId", text(row, 1)?)?;
                    flag(integer(row, 2)?)?;
                }
                "schema_migrations" => {
                    let index = ledger.len();
                    if index >= checksums.len()
                        || integer(row, 0)? != (index + 1) as i64
                        || text(row, 1)? != checksums[index].0
                    {
                        return Err(fail());
                    }
                    let timestamp = text(row, 2)?;
                    compat::scalar("Instant", timestamp)?;
                    ledger.push(LedgerEntry {
                        version: u32::try_from(index + 1).map_err(|_| limit())?,
                        sql_sha256: checksums[index].clone(),
                        applied_utc: timestamp.into(),
                    });
                }
                #[cfg(test)]
                "fixture_probe" => {
                    let expected = if catalogue.version() == 1 { 7 } else { 11 };
                    if count != 1 || integer(row, 0)? != 1 || integer(row, 1)? != expected {
                        return Err(fail());
                    }
                }
                #[cfg(test)]
                "fixture_added" => {
                    if count != 1 || integer(row, 0)? != 1 || text(row, 1)? != "upgrade-complete" {
                        return Err(fail());
                    }
                }
                #[cfg(test)]
                "fixture_assets" => {
                    if count != 1
                        || text(row, 0)? != bytes_digest(ASSET_FIXTURE_BYTES).0
                        || text(row, 1)? != "png"
                        || integer(row, 2)? != 21
                    {
                        return Err(closure());
                    }
                    asset_refs.push(AssetRef {
                        digest: text(row, 0)?.into(),
                        extension: "png".into(),
                        length: 21,
                    });
                }
                #[cfg(test)]
                "fixture_parent" => {
                    if count != 1 || integer(row, 0)? != 1 {
                        return Err(fail());
                    }
                }
                #[cfg(test)]
                "fixture_child" => {
                    if count != 1 || integer(row, 0)? != 1 || integer(row, 1)? != 1 {
                        return Err(fail());
                    }
                }
                _ => return Err(closure()),
            }
        }
        if matches!(
            table,
            "app_meta"
                | "store_format"
                | "household"
                | "fixture_probe"
                | "fixture_added"
                | "fixture_assets"
                | "fixture_parent"
                | "fixture_child"
        ) && count != 1
        {
            return Err(fail());
        }
        hasher.update([0x45]);
        hasher.update(count.to_be_bytes());
        counts.push((table, count));
    }
    if ledger.len() != checksums.len() {
        return Err(fail());
    }
    // This bounded public projection is an additional check, never the history audit.
    let current = frozen_snapshot(c, profile)?;
    budget(started)?;
    Ok(AuditedRows {
        version: catalogue.version(),
        reader: profile.name(),
        closure: catalogue.closure(),
        counters,
        lineage,
        ledger,
        schema_sha256: canonical_digest(&actual_schema).map_err(|_| fail())?,
        contents_sha256: finish(hasher),
        snapshot_sha256: bytes_digest(&current),
        asset_refs,
        row_counts: counts,
    })
}

type TableQuery = (&'static str, &'static str, usize);
fn table_queries(catalogue: Catalogue) -> Vec<TableQuery> {
    let mut tables = vec![
        (
            "app_meta",
            "SELECT singleton,lineage,snapshot_revision,settings_revision,history_revision,catalogue_revision FROM app_meta ORDER BY singleton",
            6,
        ),
        (
            "command_receipts",
            "SELECT command_id,command_digest,command,response,response_digest,snapshot_revision FROM command_receipts ORDER BY command_id",
            6,
        ),
        (
            "household",
            "SELECT singleton,household_id,draft_revision,draft,privacy_revision,privacy,profile_revision,profile FROM household ORDER BY singleton",
            8,
        ),
        (
            "jobs",
            "SELECT job_id,creation_sequence,terminal,summary FROM jobs ORDER BY job_id",
            4,
        ),
        (
            "resource_quarantine",
            "SELECT resource_key,job_id,quarantined FROM resource_quarantine ORDER BY resource_key",
            3,
        ),
        (
            "schema_migrations",
            "SELECT version,checksum,applied_utc FROM schema_migrations ORDER BY version",
            3,
        ),
    ];
    if matches!(catalogue, Catalogue::Draft005) {
        tables.push((
            "receipt_formats",
            "SELECT command_id,profile FROM receipt_formats ORDER BY command_id COLLATE BINARY",
            2,
        ));
        tables.push((
            "store_format",
            "SELECT singleton,live_profile FROM store_format ORDER BY singleton",
            2,
        ));
    }
    #[cfg(test)]
    if let Catalogue::Fixture(version) = catalogue {
        tables.push((
            "fixture_probe",
            "SELECT id,value FROM fixture_probe ORDER BY id",
            2,
        ));
        if version == FixtureVersion::V2 {
            tables.push((
                "fixture_added",
                "SELECT id,note FROM fixture_added ORDER BY id",
                2,
            ));
        }
    }
    #[cfg(test)]
    if let Catalogue::AssetFixture = catalogue {
        tables.push((
            "fixture_assets",
            "SELECT digest,extension,length FROM fixture_assets ORDER BY digest",
            3,
        ));
    }
    #[cfg(test)]
    if let Catalogue::FkFixture = catalogue {
        tables.push((
            "fixture_parent",
            "SELECT id FROM fixture_parent ORDER BY id",
            1,
        ));
        tables.push((
            "fixture_child",
            "SELECT id,parent_id FROM fixture_child ORDER BY id",
            2,
        ));
    }
    tables.sort_by_key(|t| t.0);
    tables
}

pub(super) fn meta(c: &Connection) -> Result<(Counters, String), DomainError> {
    let mut statement = c.prepare("SELECT singleton,lineage,snapshot_revision,settings_revision,history_revision,catalogue_revision FROM app_meta").map_err(storage)?;
    let mut rows = statement.query([]).map_err(storage)?;
    let row = rows.next().map_err(storage)?.ok_or_else(fail)?;
    let lineage = text(row, 1)?;
    if integer(row, 0)? != 1
        || lineage.len() != 32
        || !lineage
            .bytes()
            .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
        || lineage.bytes().all(|b| b == b'0')
    {
        return Err(fail());
    }
    let counters = Counters {
        snapshot: revision(text(row, 2)?)?,
        settings: revision(text(row, 3)?)?,
        history: revision(text(row, 4)?)?,
        catalogue: revision(text(row, 5)?)?,
    };
    if counters.history != Revision(1) || counters.catalogue != Revision(1) {
        return Err(fail());
    }
    let lineage = lineage.to_owned();
    if rows.next().map_err(storage)?.is_some() {
        return Err(fail());
    }
    Ok((counters, lineage))
}

pub(crate) fn metadata(c: &Connection) -> Result<(), DomainError> {
    let mut q = c
        .prepare("SELECT singleton,live_profile FROM store_format ORDER BY singleton")
        .map_err(storage)?;
    let rows = q
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)?;
    if rows != vec![(1, "B1Draft005".into())] {
        return Err(fail());
    }
    for sql in [
        "SELECT r.command_id FROM command_receipts AS r LEFT JOIN receipt_formats AS f ON f.command_id=r.command_id WHERE f.command_id IS NULL LIMIT 1",
        "SELECT f.command_id FROM receipt_formats AS f LEFT JOIN command_receipts AS r ON r.command_id=f.command_id WHERE r.command_id IS NULL LIMIT 1",
    ] {
        if c.prepare(sql)
            .map_err(storage)?
            .exists([])
            .map_err(storage)?
        {
            return Err(fail());
        }
    }
    Ok(())
}
fn audit_household_frozen(
    row: &Row<'_>,
    id: &str,
    counters: &Counters,
    p: Profile,
) -> Result<(), DomainError> {
    let raw = blob(row, 3)?;
    // A field which the exact old grammar cannot represent denotes unavailable
    // interpretation, never a route to the current decoder.
    if p == Profile::Old
        && serde_json::from_slice::<serde_json::Value>(raw)
            .ok()
            .is_some_and(|v| v.get("unresolved_restrictions").is_some())
    {
        return Err(DomainError::new(
            ErrorCode::UnsupportedVersion,
            "/storage_reader",
        ));
    }
    let d = compat::value(p, "draft", raw)?;
    let policy = compat::value(p, "privacy", blob(row, 5)?)?;
    if integer(row, 0)? != 1
        || text(row, 1)? != id
        || d["household_id"] != id
        || d["revision"] != text(row, 2)?
        || policy["revision"] != text(row, 4)?
        || revision(text(row, 4)?)? != counters.settings
    {
        return Err(fail());
    }
    if !d["connections"].is_null() && d["connections"]["privacy"] != policy {
        return Err(fail());
    }
    match (
        row.get_ref(6).map_err(storage)?,
        row.get_ref(7).map_err(storage)?,
    ) {
        (ValueRef::Null, ValueRef::Null) => (),
        (ValueRef::Text(rev), ValueRef::Blob(b)) => {
            let v = compat::value(p, "profile", b)?;
            if v["revision"] != std::str::from_utf8(rev).map_err(storage)?
                || v["household_id"] != id
                || v["connections"]["privacy"] != policy
            {
                return Err(fail());
            }
        }
        _ => return Err(fail()),
    }
    Ok(())
}
fn audit_job_frozen(
    p: Profile,
    id: &str,
    sequence: i64,
    is_terminal: i64,
    bytes: &[u8],
) -> Result<(), DomainError> {
    let v = compat::value(p, "job", bytes)?;
    let terminal = matches!(
        v["state"].as_str(),
        Some("needs_user" | "succeeded" | "failed" | "cancelled")
    );
    if sequence <= 0 || v["job_id"] != id || terminal != flag(is_terminal)? {
        return Err(fail());
    }
    Ok(())
}
pub(crate) fn frozen_snapshot(c: &Connection, p: Profile) -> Result<Vec<u8>, DomainError> {
    let (rev,d,policy,profile):(String,Vec<u8>,Vec<u8>,Option<Vec<u8>>)=c.query_row("SELECT snapshot_revision,draft,privacy,profile FROM app_meta JOIN household USING(singleton)",[],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?))).map_err(storage)?;
    let live: i64 = c
        .query_row("SELECT count(*) FROM jobs WHERE terminal=0", [], |r| {
            r.get(0)
        })
        .map_err(storage)?;
    if live > 1024 {
        return Err(DomainError::new(ErrorCode::LimitExceeded, "/jobs"));
    }
    let mut q=c.prepare("SELECT summary FROM jobs ORDER BY terminal ASC,creation_sequence DESC,job_id ASC LIMIT 1024").map_err(storage)?;
    let mut rows = q.query([]).map_err(storage)?;
    let mut jobs = Vec::new();
    while let Some(row) = rows.next().map_err(storage)? {
        jobs.push(compat::value(p, "job", blob(row, 0)?)?);
    }
    let v = serde_json::json!({"schema_version":1,"snapshot_revision":rev,"household_draft":compat::value(p,"draft",&d)?,"privacy":compat::value(p,"privacy",&policy)?,"profile":profile.map(|b|compat::value(p,"profile",&b)).transpose()?,"jobs":jobs,"weeks":[],"conflicts":[]});
    let bytes = canonical_bytes(&v)?;
    compat::value(p, "snapshot", &bytes)?;
    Ok(bytes)
}
#[cfg(test)]
fn audit_job_row(id: &str, sequence: i64, t: i64, b: &[u8]) -> Result<(), DomainError> {
    audit_job_frozen(Profile::Current, id, sequence, t, b)
}
#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn audit_receipt(
    id: &str,
    cd: &str,
    a: &[u8],
    b: &[u8],
    rd: &str,
    rev: &str,
    hh: &str,
    max: Revision,
) -> Result<(), DomainError> {
    compat::receipt(
        Profile::Current,
        id,
        hh,
        rev,
        &max.0.to_string(),
        cd,
        rd,
        a,
        b,
    )
    .map(|_| ())
}
fn revision(value: &str) -> Result<Revision, DomainError> {
    let parsed = value.parse::<u64>().map_err(|_| fail())?;
    if parsed == 0 || parsed.to_string() != value {
        return Err(fail());
    }
    Ok(Revision(parsed))
}
fn flag(value: i64) -> Result<bool, DomainError> {
    match value {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(fail()),
    }
}
fn text<'r>(row: &'r Row<'_>, index: usize) -> Result<&'r str, DomainError> {
    match row.get_ref(index).map_err(storage)? {
        ValueRef::Text(bytes) if bytes.len() <= MAX_DOCUMENT_BYTES => {
            std::str::from_utf8(bytes).map_err(|_| fail())
        }
        ValueRef::Text(_) => Err(limit()),
        _ => Err(fail()),
    }
}
fn opaque_text<'r>(row: &'r Row<'_>, index: usize) -> Result<&'r str, DomainError> {
    match row.get_ref(index).map_err(storage)? {
        ValueRef::Text(bytes) if bytes.len() as u64 <= MAX_DB_BYTES => {
            std::str::from_utf8(bytes).map_err(|_| fail())
        }
        ValueRef::Text(_) => Err(limit()),
        _ => Err(fail()),
    }
}
fn blob<'r>(row: &'r Row<'_>, index: usize) -> Result<&'r [u8], DomainError> {
    match row.get_ref(index).map_err(storage)? {
        ValueRef::Blob(bytes) if bytes.len() <= MAX_DOCUMENT_BYTES => Ok(bytes),
        ValueRef::Blob(_) => Err(limit()),
        _ => Err(fail()),
    }
}
fn integer(row: &Row<'_>, index: usize) -> Result<i64, DomainError> {
    match row.get_ref(index).map_err(storage)? {
        ValueRef::Integer(n) => Ok(n),
        _ => Err(fail()),
    }
}
fn hash_bytes(hasher: &mut Sha256, bytes: &[u8]) -> Result<(), DomainError> {
    hasher.update(
        u64::try_from(bytes.len())
            .map_err(|_| limit())?
            .to_be_bytes(),
    );
    hasher.update(bytes);
    Ok(())
}
fn hash_value(hasher: &mut Sha256, value: ValueRef<'_>) -> Result<(), DomainError> {
    match value {
        ValueRef::Null => hasher.update([0x00]),
        ValueRef::Integer(n) => {
            hasher.update([0x01]);
            hasher.update(n.to_be_bytes());
        }
        ValueRef::Text(bytes) => {
            if bytes.len() as u64 > MAX_DB_BYTES {
                return Err(limit());
            }
            std::str::from_utf8(bytes).map_err(|_| fail())?;
            hasher.update([0x03]);
            hash_bytes(hasher, bytes)?;
        }
        ValueRef::Blob(bytes) => {
            if bytes.len() > MAX_DOCUMENT_BYTES {
                return Err(limit());
            }
            hasher.update([0x04]);
            hash_bytes(hasher, bytes)?;
        }
        ValueRef::Real(_) => return Err(fail()),
    }
    Ok(())
}
fn finish(hasher: Sha256) -> Digest {
    Digest(
        hasher
            .finalize()
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    )
}

// Canonical raw SQLite cell encoding, including every original receipt column.
pub(crate) fn receipt_contents(c: &Connection) -> Result<Digest, DomainError> {
    let mut h = Sha256::new();
    h.update(b"MSTRCHF9000-RECEIPT-CUSTODY-v1");
    let mut q=c.prepare("SELECT command_id,command_digest,command,response,response_digest,snapshot_revision FROM command_receipts ORDER BY command_id COLLATE BINARY").map_err(storage)?;
    let mut rows = q.query([]).map_err(storage)?;
    while let Some(row) = rows.next().map_err(storage)? {
        h.update([0x52]);
        for i in 0..6 {
            hash_value(&mut h, row.get_ref(i).map_err(storage)?)?;
        }
    }
    Ok(finish(h))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{commands, records};

    fn source() -> Connection {
        let mut c = Connection::open_in_memory().unwrap();
        c.pragma_update(None, "foreign_keys", true).unwrap();
        migrate::initialise(&mut c, "00000000000000000000000000000001").unwrap();
        c
    }
    fn asset_source(rows: &[(&str, &str, i64)]) -> Connection {
        let c = Connection::open_in_memory().unwrap();
        c.pragma_update(None, "foreign_keys", true).unwrap();
        c.execute_batch(&asset_source_sql()).unwrap();
        c.execute(
            "INSERT INTO schema_migrations VALUES(1,?1,'2026-10-02T00:00:00Z')",
            [bytes_digest(asset_source_sql().as_bytes()).0],
        )
        .unwrap();
        c.execute(
            "INSERT INTO app_meta VALUES(1,'00000000000000000000000000000001','1','1','1','1')",
            [],
        )
        .unwrap();
        let initial = records::initial(HouseholdId("hh_00000000000000000000000000000001".into()));
        c.execute(
            "INSERT INTO household VALUES(1,?1,'1',?2,'1',?3,NULL,NULL)",
            rusqlite::params![
                initial.household_draft.household_id.0,
                canonical_bytes(&initial.household_draft).unwrap(),
                canonical_bytes(&initial.privacy).unwrap(),
            ],
        )
        .unwrap();
        for (digest, extension, length) in rows {
            c.execute(
                "INSERT INTO fixture_assets VALUES(?1,?2,?3)",
                rusqlite::params![digest, extension, length],
            )
            .unwrap();
        }
        c.pragma_update(None, "user_version", 1).unwrap();
        c
    }

    fn fk_source(include_parent: bool) -> Connection {
        let c = Connection::open_in_memory().unwrap();
        // The deliberately incomplete case is a synthetic fixture constructed
        // from literals, never an edit to household or receipt state.
        c.pragma_update(None, "foreign_keys", false).unwrap();
        c.execute_batch(&fk_source_sql()).unwrap();
        c.execute(
            "INSERT INTO schema_migrations VALUES(1,?1,'2026-10-02T00:00:00Z')",
            [bytes_digest(fk_source_sql().as_bytes()).0],
        )
        .unwrap();
        c.execute(
            "INSERT INTO app_meta VALUES(1,'00000000000000000000000000000001','1','1','1','1')",
            [],
        )
        .unwrap();
        let initial = records::initial(HouseholdId("hh_00000000000000000000000000000001".into()));
        c.execute(
            "INSERT INTO household VALUES(1,?1,'1',?2,'1',?3,NULL,NULL)",
            rusqlite::params![
                initial.household_draft.household_id.0,
                canonical_bytes(&initial.household_draft).unwrap(),
                canonical_bytes(&initial.privacy).unwrap(),
            ],
        )
        .unwrap();
        if include_parent {
            c.execute("INSERT INTO fixture_parent VALUES(1)", [])
                .unwrap();
        }
        c.execute("INSERT INTO fixture_child VALUES(1,1)", [])
            .unwrap();
        c.pragma_update(None, "user_version", 1).unwrap();
        c.pragma_update(None, "foreign_keys", true).unwrap();
        c
    }

    #[test]
    fn closed_fk_fixture_audits_parent_and_child_and_refuses_missing_parent() {
        let valid = fk_source(true);
        let audited = audit_fk_fixture(&valid).unwrap();
        assert!(audited.asset_refs.is_empty());
        assert_eq!(
            audited.ledger[0].sql_sha256,
            bytes_digest(fk_source_sql().as_bytes())
        );
        assert_eq!(
            audited.row_counts.iter().find(|r| r.0 == "fixture_parent"),
            Some(&("fixture_parent", 1))
        );
        assert_eq!(
            audited.row_counts.iter().find(|r| r.0 == "fixture_child"),
            Some(&("fixture_child", 1))
        );
        assert_eq!(
            audit_b1(&valid).unwrap_err().code,
            ErrorCode::StorageFailure
        );
        let missing = fk_source(false);
        let integrity_result: String = missing
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .unwrap();
        assert_eq!(integrity_result, "ok");
        let broken_reference: (String, i64, String, i64) = missing
            .query_row("PRAGMA foreign_key_check", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })
            .unwrap();
        assert_eq!(
            broken_reference,
            ("fixture_child".into(), 1, "fixture_parent".into(), 0)
        );
        assert_eq!(
            audit_fk_fixture(&missing).unwrap_err().code,
            ErrorCode::StorageFailure
        );
    }

    #[test]
    fn closed_asset_fixture_has_one_exact_reference_and_separate_catalogue() {
        const HASH: &str = "7d51503664b77f2147ce760635926a8a4ec49a926b6115a3fed5b4feed3002f4";
        assert_eq!(ASSET_FIXTURE_BYTES.len(), 21);
        assert_eq!(bytes_digest(ASSET_FIXTURE_BYTES).0, HASH);
        let c = asset_source(&[(HASH, "png", 21)]);
        let audited = audit_asset_fixture(&c).unwrap();
        assert_eq!(audited.closure, "test-fixture-single-asset");
        assert_eq!(
            audited.asset_refs,
            vec![AssetRef {
                digest: HASH.into(),
                extension: "png".into(),
                length: 21
            }]
        );
        assert_eq!(
            audited.row_counts.iter().find(|r| r.0 == "fixture_assets"),
            Some(&("fixture_assets", 1))
        );
        assert_eq!(
            audited.ledger[0].sql_sha256,
            bytes_digest(asset_source_sql().as_bytes())
        );
        assert_eq!(audited, audit_asset_fixture(&c).unwrap());
        assert_eq!(audit_b1(&c).unwrap_err().code, ErrorCode::StorageFailure);
        assert!(audit_fixture(&c, FixtureVersion::V1).is_err());
        assert!(audit_asset_fixture(&source()).is_err());
    }

    #[test]
    fn closed_asset_fixture_refuses_missing_altered_or_extra_rows() {
        const HASH: &str = "7d51503664b77f2147ce760635926a8a4ec49a926b6115a3fed5b4feed3002f4";
        const OTHER: &str = "0000000000000000000000000000000000000000000000000000000000000001";
        let cases = [
            vec![],
            vec![(OTHER, "png", 21)],
            vec![(HASH, "jpg", 21)],
            vec![(HASH, "png", 20)],
            vec![(HASH, "png", 22)],
            vec![(HASH, "png", -1)],
            vec![(HASH, "png", 21), (OTHER, "png", 21)],
        ];
        for rows in cases {
            assert!(audit_asset_fixture(&asset_source(&rows)).is_err());
        }
    }
    fn privacy_command(s: &Snapshot, id: u64) -> AppCommandV1 {
        let mut policy = s.privacy.clone();
        policy.use_ai_features = !policy.use_ai_features;
        AppCommandV1 {
            schema_version: SchemaVersion,
            command_id: CommandId(format!("cm_{id:032x}")),
            expected_snapshot_revision: s.snapshot_revision,
            command: AppCommand::SetPrivacy {
                policy,
                expected_policy_revision: s.privacy.revision,
            },
        }
    }
    fn literal_receipt(id: u64) -> (AppCommandV1, Snapshot) {
        let mut before =
            records::initial(HouseholdId("hh_00000000000000000000000000000001".into()));
        before.snapshot_revision = Revision(id);
        before.privacy.revision = Revision(id);
        let command = privacy_command(&before, id);
        let mut after = before;
        after.snapshot_revision = Revision(id + 1);
        if let AppCommand::SetPrivacy { policy, .. } = &command.command {
            after.privacy = policy.clone();
            after.privacy.revision = Revision(id + 1);
        }
        (command, after)
    }
    fn check_literal(command: &AppCommandV1, response: &Snapshot) -> Result<(), DomainError> {
        let a = canonical_bytes(command).unwrap();
        let b = canonical_bytes(response).unwrap();
        audit_receipt(
            &command.command_id.0,
            &bytes_digest(&a).0,
            &a,
            &b,
            &bytes_digest(&b).0,
            &response.snapshot_revision.0.to_string(),
            &response.household_draft.household_id.0,
            Revision(u64::MAX),
        )
    }

    #[test]
    fn empty_and_two_command_b1_audit_preserves_original_bytes() {
        let mut c = source();
        let empty = audit_b1(&c).unwrap();
        assert_eq!(empty.asset_refs, []);
        assert_eq!(
            empty.row_counts,
            vec![
                ("app_meta", 1),
                ("command_receipts", 0),
                ("household", 1),
                ("jobs", 0),
                ("receipt_formats", 0),
                ("resource_quarantine", 0),
                ("schema_migrations", 2),
                ("store_format", 1)
            ]
        );
        let initial = snapshot::load(&c).unwrap();
        let first = privacy_command(&initial, 0x11);
        let original_command = canonical_bytes(&first).unwrap();
        let original_response = commands::execute(&mut c, &first, &original_command).unwrap();
        let second = privacy_command(&snapshot::load(&c).unwrap(), 0x12);
        commands::execute(&mut c, &second, &canonical_bytes(&second).unwrap()).unwrap();
        let audited = audit_b1(&c).unwrap();
        assert_eq!(
            audited.counters,
            Counters {
                snapshot: Revision(3),
                settings: Revision(3),
                history: Revision(1),
                catalogue: Revision(1)
            }
        );
        assert_eq!(audited.asset_refs, []);
        assert_ne!(audited.contents_sha256, empty.contents_sha256);
        let saved: (Vec<u8>, Vec<u8>) = c
            .query_row(
                "SELECT command,response FROM command_receipts WHERE command_id=?1",
                [&first.command_id.0],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )
            .unwrap();
        assert_eq!(saved, (original_command, original_response));
        assert_eq!(audited, audit_b1(&c).unwrap());
    }

    #[test]
    fn every_retained_receipt_is_audited_past_1024() {
        let mut c = source();
        let mut s = snapshot::load(&c).unwrap();
        for id in 1..=1025 {
            let command = privacy_command(&s, id);
            let result =
                commands::execute(&mut c, &command, &canonical_bytes(&command).unwrap()).unwrap();
            s = parse_document(&result).unwrap();
        }
        let audited = audit_b1(&c).unwrap();
        assert_eq!(audited.counters.snapshot, Revision(1026));
        assert_eq!(
            audited
                .row_counts
                .iter()
                .find(|v| v.0 == "command_receipts")
                .unwrap()
                .1,
            1025
        );
    }

    #[test]
    fn pure_retained_job_vectors_include_old_terminals_and_reject_last_invalid_row() {
        let rows: Vec<_> = (1..=1025)
            .map(|id| {
                let job = JobSummary {
                    job_id: JobId(format!("jb_{id:032x}")),
                    action: Action::Setup,
                    state: JobState::Succeeded,
                    reason: Nullable(None),
                    next_action: Nullable(None),
                };
                (
                    job.job_id.0.clone(),
                    i64::from(id),
                    canonical_bytes(&job).unwrap(),
                )
            })
            .collect();
        let mut count = 0;
        for (id, sequence, bytes) in &rows {
            audit_job_row(id, *sequence, 1, bytes).unwrap();
            count += 1;
        }
        assert_eq!(count, 1025);
        let (id, sequence, bytes) = rows.last().unwrap();
        assert!(audit_job_row(id, *sequence, 0, bytes).is_err());
        assert!(audit_job_row("jb_00000000000000000000000000000001", *sequence, 1, bytes).is_err());
        assert!(audit_job_row(id, 0, 1, bytes).is_err());
        assert!(audit_job_row(id, *sequence, 2, bytes).is_err());
    }

    #[test]
    fn pure_receipts_reject_noncanonical_originals_digests_and_revision_projections() {
        let (command, response) = literal_receipt(1);
        check_literal(&command, &response).unwrap();
        let a = canonical_bytes(&command).unwrap();
        let b = canonical_bytes(&response).unwrap();
        let id = &response.household_draft.household_id.0;
        let mut spaced = a.clone();
        spaced.push(b' ');
        assert!(
            audit_receipt(
                &command.command_id.0,
                &bytes_digest(&spaced).0,
                &spaced,
                &b,
                &bytes_digest(&b).0,
                "2",
                id,
                Revision(2)
            )
            .is_err()
        );
        assert!(
            audit_receipt(
                &command.command_id.0,
                &"0".repeat(64),
                &a,
                &b,
                &bytes_digest(&b).0,
                "2",
                id,
                Revision(2)
            )
            .is_err()
        );
        assert!(
            audit_receipt(
                &command.command_id.0,
                &bytes_digest(&a).0,
                &a,
                &b,
                &bytes_digest(&b).0,
                "3",
                id,
                Revision(3)
            )
            .is_err()
        );
        assert!(
            audit_receipt(
                "cm_00000000000000000000000000000002",
                &bytes_digest(&a).0,
                &a,
                &b,
                &bytes_digest(&b).0,
                "2",
                id,
                Revision(2)
            )
            .is_err()
        );
        assert!(
            audit_receipt(
                &command.command_id.0,
                &bytes_digest(&a).0,
                &a,
                &b,
                &bytes_digest(&b).0,
                "2",
                id,
                Revision(1)
            )
            .is_err()
        );
        let mut wrong_expected = command.clone();
        wrong_expected.expected_snapshot_revision = Revision(2);
        assert!(check_literal(&wrong_expected, &response).is_err());
    }

    #[test]
    fn pure_unsupported_command_and_historical_closure_refuse() {
        let (mut command, mut response) = literal_receipt(1);
        command.command = AppCommand::RequestJob {
            action: Action::Plan,
            scope: ActionScope::Plan {
                mode: PlanMode::FillOpen,
                slots: BoundedVec(vec![]),
            },
        };
        assert_eq!(
            check_literal(&command, &response).unwrap_err().code,
            ErrorCode::UnsupportedCapability
        );
        let (command, _) = literal_receipt(1);
        response.conflicts.0.push(Conflict {
            code: ErrorCode::ContextStale,
            week_ids: BoundedVec(vec![]),
            slot_ids: BoundedVec(vec![]),
            recipe_refs: BoundedVec(vec![]),
            evidence_ids: BoundedVec(vec![]),
            blocking: true,
        });
        assert_eq!(
            check_literal(&command, &response).unwrap_err().code,
            ErrorCode::UnsupportedCapability
        );
    }

    #[test]
    fn numeric_and_blob_size_limits_are_checked_without_large_database_allocation() {
        assert_eq!(
            revision("18446744073709551615").unwrap(),
            Revision(u64::MAX)
        );
        for bad in ["0", "01", "-1", "18446744073709551616", "1x", ""] {
            assert!(revision(bad).is_err(), "{bad}");
        }
        assert_eq!(database_size(262144, 4096).unwrap(), MAX_DB_BYTES);
        assert!(database_size(262145, 4096).is_err());
        assert!(database_size(i64::MAX, i64::MAX).is_err());
        assert!(database_size(-1, 4096).is_err());
        let at_limit = vec![0u8; MAX_DOCUMENT_BYTES];
        hash_value(&mut Sha256::new(), ValueRef::Blob(&at_limit)).unwrap();
        let over = vec![0u8; MAX_DOCUMENT_BYTES + 1];
        assert_eq!(
            hash_value(&mut Sha256::new(), ValueRef::Blob(&over))
                .unwrap_err()
                .code,
            ErrorCode::LimitExceeded
        );
    }

    #[test]
    fn raw_hash_encoding_distinguishes_types_and_rejects_real_or_invalid_utf8() {
        let mut h = Sha256::new();
        hash_value(&mut h, ValueRef::Null).unwrap();
        hash_value(&mut h, ValueRef::Integer(-2)).unwrap();
        hash_value(&mut h, ValueRef::Text(b"A")).unwrap();
        hash_value(&mut h, ValueRef::Blob(b"B")).unwrap();
        let expected = [
            0, 1, 255, 255, 255, 255, 255, 255, 255, 254, 3, 0, 0, 0, 0, 0, 0, 0, 1, 65, 4, 0, 0,
            0, 0, 0, 0, 0, 1, 66,
        ];
        assert_eq!(finish(h), bytes_digest(&expected));
        assert!(hash_value(&mut Sha256::new(), ValueRef::Real(1.0)).is_err());
        assert!(hash_value(&mut Sha256::new(), ValueRef::Text(&[255])).is_err());
    }

    #[test]
    fn ordinary_extra_schema_refuses_and_generated_autoindexes_are_excluded() {
        let c = source();
        assert!(audit_b1(&c).is_ok());
        c.execute_batch("CREATE TABLE fixture_unrecognised(value TEXT UNIQUE) STRICT;")
            .unwrap();
        assert_eq!(audit_b1(&c).unwrap_err().code, ErrorCode::StorageFailure);
        assert!(
            !schema(&c)
                .unwrap()
                .iter()
                .any(|v| v.1.starts_with("sqlite_"))
        );
        assert!(
            schema(&c)
                .unwrap()
                .iter()
                .any(|v| v.1 == "fixture_unrecognised")
        );
    }
}
