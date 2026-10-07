//! Closed synthetic SQLite seeds. No production fixture authority.
use mstrchf_contracts::{bytes_digest, canonical_bytes};
use mstrchf_store::{Store, StoreConfig};
use rusqlite::{Connection, params};
use serde_json::Value;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
static NEXT: AtomicU64 = AtomicU64::new(0);
pub(crate) fn root() -> PathBuf {
    let p = std::env::temp_dir().join(format!(
        "mstrchf-adapter-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ));
    eprintln!("synthetic fixture: {}", p.display());
    p
}
pub(crate) fn literal(group: &str, name: &str) -> String {
    std::fs::read_to_string(
        Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("tests/fixtures/receipt")
            .join(group)
            .join(name),
    )
    .unwrap()
}
pub(crate) fn value(group: &str, name: &str) -> Value {
    serde_json::from_str(&literal(group, name)).unwrap()
}
pub(crate) fn open(p: &Path) -> Store {
    Store::open(StoreConfig {
        app_data_root: p.into(),
    })
    .unwrap()
}
pub(crate) const SQL1: &str = include_str!("../../crates/store/migrations/001_initial.sql");
pub(crate) const SQL2: &str =
    include_str!("../../crates/store/migrations/002_draft005_compatibility.sql");
pub(crate) fn seed(group: &str, snapshot: &str, old_receipt: Option<(&str, &str)>) -> PathBuf {
    let p = root();
    std::fs::create_dir_all(&p).unwrap();
    let c = Connection::open(p.join("planner.sqlite")).unwrap();
    c.execute_batch(SQL1).unwrap();
    c.execute(
        "INSERT INTO schema_migrations VALUES(1,?1,'2026-10-02T00:00:00Z')",
        [bytes_digest(SQL1.as_bytes()).0],
    )
    .unwrap();
    let s = value(group, snapshot);
    let old = old_receipt.is_some();
    if !old {
        c.execute_batch(SQL2).unwrap();
        c.execute(
            "INSERT INTO schema_migrations VALUES(2,?1,'2026-10-02T00:00:00Z')",
            [bytes_digest(SQL2.as_bytes()).0],
        )
        .unwrap();
    }
    c.execute(
        "INSERT INTO app_meta VALUES(1,'00000000000000000000000000000001',?1,?2,'1','1')",
        params![
            s["snapshot_revision"].as_str().unwrap(),
            s["privacy"]["revision"].as_str().unwrap()
        ],
    )
    .unwrap();
    c.execute(
        "INSERT INTO household VALUES(1,?1,?2,?3,?4,?5,NULL,NULL)",
        params![
            s["household_draft"]["household_id"].as_str().unwrap(),
            s["household_draft"]["revision"].as_str().unwrap(),
            canonical_bytes(&s["household_draft"]).unwrap(),
            s["privacy"]["revision"].as_str().unwrap(),
            canonical_bytes(&s["privacy"]).unwrap()
        ],
    )
    .unwrap();
    for (i, j) in s["jobs"].as_array().unwrap().iter().enumerate() {
        c.execute(
            "INSERT INTO jobs VALUES(?1,?2,1,?3)",
            params![
                j["job_id"].as_str().unwrap(),
                i as i64 + 1,
                canonical_bytes(j).unwrap()
            ],
        )
        .unwrap();
    }
    if let Some((command, response)) = old_receipt {
        let a = literal(group, command);
        let b = literal(group, response);
        let h: Value = serde_json::from_str(&a).unwrap();
        c.execute(
            "INSERT INTO command_receipts VALUES(?1,?2,?3,?4,?5,?6)",
            params![
                h["command_id"].as_str().unwrap(),
                bytes_digest(a.as_bytes()).0,
                a.as_bytes(),
                b.as_bytes(),
                bytes_digest(b.as_bytes()).0,
                s["snapshot_revision"].as_str().unwrap()
            ],
        )
        .unwrap();
    } else {
        c.execute("INSERT INTO store_format VALUES(1,'B1Draft005')", [])
            .unwrap();
    }
    c.pragma_update(None, "user_version", if old { 1 } else { 2 })
        .unwrap();
    drop(c);
    p
}
pub(crate) fn count(p: &Path) -> i64 {
    let c = Connection::open(p.join("planner.sqlite")).unwrap();
    c.query_row("SELECT count(*) FROM command_receipts", [], |r| r.get(0))
        .unwrap()
}
pub(crate) fn stored(p: &Path, group: &str, command: &str, response: &str, profile: &str) {
    let a = literal(group, command);
    let b = literal(group, response);
    let v: Value = serde_json::from_str(&a).unwrap();
    let c = Connection::open(p.join("planner.sqlite")).unwrap();
    let (ac,bc,ad,bd,pr):(Vec<u8>,Vec<u8>,String,String,String)=c.query_row("SELECT command,response,command_digest,response_digest,profile FROM command_receipts JOIN receipt_formats USING(command_id) WHERE command_id=?1",[v["command_id"].as_str().unwrap()],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).unwrap();
    assert_eq!(ac, a.as_bytes());
    assert_eq!(bc, b.as_bytes());
    assert_eq!(ad, bytes_digest(a.as_bytes()).0);
    assert_eq!(bd, bytes_digest(b.as_bytes()).0);
    assert_eq!(pr, profile);
}
