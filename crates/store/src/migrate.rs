use crate::records::*;
use mstrchf_contracts::*;
use rusqlite::{Connection, params};
pub(crate) const SQL: &str = include_str!("../migrations/001_initial.sql");
pub(crate) const SQL_002: &str = include_str!("../migrations/002_draft005_compatibility.sql");
// GLOB treats '_' literally and is case-sensitive. SQLite-generated internal
// objects use the reserved lowercase sqlite_ prefix.
#[cfg(test)]
const INTERNAL_SCHEMA_GLOB: &str = "sqlite_*";

pub(crate) fn initialise(c: &mut Connection, lineage: &str) -> Result<(), DomainError> {
    let tx = c.transaction().map_err(storage)?;
    tx.execute_batch(SQL).map_err(storage)?;
    tx.execute_batch(SQL_002).map_err(storage)?;
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
    tx.execute(
        "INSERT INTO schema_migrations VALUES(2,?1,strftime('%Y-%m-%dT%H:%M:%SZ','now'))",
        [bytes_digest(SQL_002.as_bytes()).0],
    )
    .map_err(storage)?;
    tx.execute("INSERT INTO store_format VALUES(1,'B1Draft005')", [])
        .map_err(storage)?;
    tx.pragma_update(None, "user_version", 2).map_err(storage)?;
    crate::audit::audit_b1(&tx)?;
    #[cfg(test)]
    crate::tests::barrier("init_before_commit");
    tx.commit().map_err(storage)
}

#[cfg(test)]
fn schema(c: &Connection) -> Result<Vec<(String, String, String)>, DomainError> {
    let mut s = c
        .prepare(
            "SELECT type,name,sql FROM sqlite_schema WHERE name NOT GLOB ?1 ORDER BY type,name",
        )
        .map_err(storage)?;
    s.query_map([INTERNAL_SCHEMA_GLOB], |r| {
        Ok((r.get(0)?, r.get(1)?, r.get(2)?))
    })
    .map_err(storage)?
    .collect::<Result<Vec<_>, _>>()
    .map_err(storage)
}
pub(crate) fn verify(c: &Connection) -> Result<(), DomainError> {
    crate::audit::audit_b1(c).map(|_| ())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn internal_schema_pattern_requires_literal_lowercase_prefix() {
        let c = Connection::open_in_memory().unwrap();
        for (name, expected) in [
            ("sqlite_autoindex_household_1", true),
            ("sqlite_stat1", true),
            ("sqlite_sequence", true),
            ("sqliteX_control", false),
            ("SQLITEX_control", false),
            ("SQLITE_autoindex_household_1", false),
            ("sqlite", false),
            ("xsqlite_autoindex_household_1", false),
        ] {
            let actual: bool = c
                .query_row("SELECT ?1 GLOB ?2", [name, INTERNAL_SCHEMA_GLOB], |r| {
                    r.get(0)
                })
                .unwrap();
            assert_eq!(actual, expected, "{name}");
        }
    }

    #[test]
    fn actual_generated_autoindex_is_excluded_but_its_table_is_retained() {
        let c = Connection::open_in_memory().unwrap();
        c.execute_batch("CREATE TABLE ordinary_control(value TEXT UNIQUE) STRICT;")
            .unwrap();
        let internal: String = c
            .query_row(
                "SELECT name FROM sqlite_schema WHERE type='index'",
                [],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(internal, "sqlite_autoindex_ordinary_control_1");
        let objects = schema(&c).unwrap();
        assert_eq!(objects.len(), 1);
        assert_eq!(objects[0].0, "table");
        assert_eq!(objects[0].1, "ordinary_control");
    }
}
