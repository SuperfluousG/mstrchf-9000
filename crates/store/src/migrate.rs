use crate::{records::*, snapshot};
use mstrchf_contracts::*;
use rusqlite::{Connection, params};
pub(crate) const SQL: &str = include_str!("../migrations/001_initial.sql");

pub(crate) fn initialise(c: &mut Connection, lineage: &str) -> Result<(), DomainError> {
    let tx = c.transaction().map_err(storage)?;
    tx.execute_batch(SQL).map_err(storage)?;
    let s = initial(HouseholdId(format!("hh_{}", random_id()?)));
    tx.execute(
        "INSERT INTO schema_migrations VALUES(1,?1,strftime('%Y-%m-%dT%H:%M:%SZ','now'))",
        [bytes_digest(SQL.as_bytes()).0],
    )
    .map_err(storage)?;
    tx.execute(
        "INSERT INTO app_meta VALUES(1,?1,'1','1','1','1')",
        [lineage],
    )
    .map_err(storage)?;
    tx.execute(
        "INSERT INTO household VALUES(1,?1,'1',?2,'1',?3,NULL,NULL)",
        params![
            s.household_draft.household_id.0,
            canonical_bytes(&s.household_draft)?,
            canonical_bytes(&s.privacy)?
        ],
    )
    .map_err(storage)?;
    tx.pragma_update(None, "user_version", 1).map_err(storage)?;
    snapshot::load(&tx)?;
    #[cfg(test)]
    crate::tests::barrier("init_before_commit");
    tx.commit().map_err(storage)
}

fn schema(c: &Connection) -> Result<Vec<(String, String, String)>, DomainError> {
    let mut s=c.prepare("SELECT type,name,sql FROM sqlite_schema WHERE name NOT LIKE 'sqlite_%' ORDER BY type,name").map_err(storage)?;
    s.query_map([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)
}
pub(crate) fn verify(c: &Connection) -> Result<(), DomainError> {
    let version: i64 = c
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(storage)?;
    if version > 1 {
        return Err(DomainError::new(
            ErrorCode::UnsupportedVersion,
            "/storage_version",
        ));
    }
    if version != 1 {
        return Err(storage("backup required or unknown database"));
    }
    let expected = Connection::open_in_memory().map_err(storage)?;
    expected.execute_batch(SQL).map_err(storage)?;
    if schema(c)? != schema(&expected)? {
        return Err(storage("schema mismatch"));
    }
    let mut q = c
        .prepare("SELECT version,checksum FROM schema_migrations ORDER BY version")
        .map_err(storage)?;
    let rows = q
        .query_map([], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, String>(1)?)))
        .map_err(storage)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage)?;
    if rows != vec![(1, bytes_digest(SQL.as_bytes()).0)] {
        return Err(storage("migration ledger"));
    }
    let integrity: String = c
        .query_row("PRAGMA integrity_check", [], |r| r.get(0))
        .map_err(storage)?;
    let fk: bool = c
        .prepare("PRAGMA foreign_key_check")
        .map_err(storage)?
        .exists([])
        .map_err(storage)?;
    if integrity != "ok" || fk {
        return Err(storage("integrity"));
    }
    snapshot::load(c)?;
    Ok(())
}
