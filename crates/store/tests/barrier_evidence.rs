//! Read-only kill-barrier observations. These values confer no backup custody.
//! Capture runs after the child is reaped and before any fresh verifier starts.
#![cfg(test)]

use crate::{audit, custody, startup};
use mstrchf_contracts::{bytes_digest, canonical_bytes};
use rusqlite::{Connection, OpenFlags};
use serde::Serialize;
use serde_json::{Value, json};
use sha2::{Digest as _, Sha256};
use std::os::windows::fs::OpenOptionsExt;
use std::{
    collections::BTreeMap,
    fs,
    io::Read,
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct Identity {
    volume_serial: String,
    file_id: String,
}
fn identity(path: &Path, directory: bool) -> Identity {
    let native = custody::identity_for_test(path, directory);
    Identity {
        volume_serial: native.volume_serial.to_string(),
        file_id: native
            .file_id
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect(),
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
struct Item {
    identity: Identity,
    directory: bool,
    length: Option<u64>,
    sha256: Option<String>,
}
type Inventory = BTreeMap<String, Item>;

fn inventory(attempt: &Path) -> Inventory {
    fn walk(attempt: &Path, path: &Path, found: &mut Inventory, depth: usize) {
        assert!(depth <= 2, "unexpected nested backup directory");
        let metadata = fs::symlink_metadata(path).unwrap();
        assert!(
            !metadata.file_type().is_symlink(),
            "evidence must not follow a link"
        );
        let name = path
            .strip_prefix(attempt)
            .unwrap()
            .to_str()
            .unwrap()
            .replace('\\', "/");
        let directory = metadata.is_dir();
        let before_identity = identity(path, directory);
        let (length, sha256) = if directory {
            (None, None)
        } else {
            assert!(metadata.is_file());
            let mut file = fs::File::open(path).unwrap();
            let mut length = 0u64;
            let mut hash = Sha256::new();
            let mut buffer = [0u8; 65_536];
            loop {
                let count = file.read(&mut buffer).unwrap();
                if count == 0 {
                    break;
                }
                length = length.checked_add(count as u64).unwrap();
                assert!(
                    length <= 1_073_741_824,
                    "unexpected oversized fixture evidence"
                );
                hash.update(&buffer[..count]);
            }
            assert_eq!(length, metadata.len());
            (
                Some(length),
                Some(
                    hash.finalize()
                        .iter()
                        .map(|byte| format!("{byte:02x}"))
                        .collect(),
                ),
            )
        };
        assert_eq!(
            identity(path, directory),
            before_identity,
            "identity changed during capture"
        );
        assert!(
            found
                .insert(
                    name,
                    Item {
                        identity: before_identity,
                        directory,
                        length,
                        sha256
                    }
                )
                .is_none()
        );
        assert!(
            found.len() <= 8,
            "unexpected number of fixture evidence items"
        );
        if directory {
            for entry in fs::read_dir(path).unwrap() {
                walk(attempt, &entry.unwrap().path(), found, depth + 1);
            }
        }
    }
    let mut result = Inventory::new();
    walk(attempt, attempt, &mut result, 0);
    result
}

pub(crate) struct Evidence {
    root: PathBuf,
    attempt: PathBuf,
    root_identity: Identity,
    live_identity: Identity,
    original: Inventory,
    barrier: String,
}

impl Evidence {
    /// New sibling attempts may exist; the original attempt must be untouched.
    pub(crate) fn assert_preserved(&self) {
        assert_eq!(identity(&self.root, true), self.root_identity);
        assert_eq!(
            identity(&self.root.join("planner.sqlite"), false),
            self.live_identity
        );
        assert_eq!(
            inventory(&self.attempt),
            self.original,
            "original backup changed after recovery at {}",
            self.barrier
        );
        println!(
            "BACKUP_EVIDENCE barrier={} {}",
            self.barrier,
            json!({
                "phase":"preserved-after-recovery", "attempt":self.attempt.file_name().unwrap().to_str().unwrap(),
                "directory_identity":self.original[""].identity, "items":self.original.len()
            })
        );
    }
}

fn file_record(name: &str, items: &Inventory) -> Value {
    let item = items
        .get(name)
        .unwrap_or_else(|| panic!("missing evidence file {name}"));
    assert!(!item.directory);
    json!({"name":name,"identity":item.identity,"length":item.length.unwrap(),"sha256":item.sha256.as_ref().unwrap()})
}

fn intent(catalogue: startup::Catalogue, from: u32) -> Value {
    if matches!(catalogue, startup::Catalogue::B1) {
        let entries = json!([{"version":1,"sql_sha256":bytes_digest(crate::migrate::SQL.as_bytes()).0,"reader":"B1RetainedV1","closure":"b1-v1-no-assets"},{"version":2,"sql_sha256":bytes_digest(crate::migrate::SQL_002.as_bytes()).0,"reader":"B1Draft005","closure":"b1-v1-no-assets"}]);
        let transform = Some((
            bytes_digest(include_bytes!("../src/upgrade.rs")),
            bytes_digest(include_bytes!("../src/compat/b1_v1.rs")),
        ));
        return json!({"from":from,"target":2,"catalogue_sha256":bytes_digest(&canonical_bytes(&(entries,transform)).unwrap()).0});
    }
    let entry = |version: u32, sql: &str, closure: &str| {
        json!({
            "version":version,"sql_sha256":bytes_digest(sql.as_bytes()).0,
            "reader":"b1-contracts003","closure":closure
        })
    };
    let (target, entries) = match catalogue {
        startup::Catalogue::B1 => (1, vec![entry(1, crate::migrate::SQL, "b1-v1-no-assets")]),
        startup::Catalogue::Fixture => (
            2,
            vec![
                entry(1, &audit::fixture_source_sql(), "test-fixture"),
                entry(2, audit::FIXTURE_V1_TO_V2_SQL, "test-fixture"),
            ],
        ),
        startup::Catalogue::AssetFixture => (
            1,
            vec![entry(
                1,
                &audit::asset_source_sql(),
                "test-fixture-single-asset",
            )],
        ),
        startup::Catalogue::FkFixture => (
            1,
            vec![entry(
                1,
                &audit::fk_source_sql(),
                "test-fixture-foreign-key",
            )],
        ),
        startup::Catalogue::ReaderUnavailable => {
            panic!("no backup can have unavailable source reader")
        }
    };
    json!({"from":from,"target":target,"catalogue_sha256":bytes_digest(&canonical_bytes(&entries).unwrap()).0})
}

pub(crate) fn capture(
    root: &Path,
    barrier: &str,
    catalogue: startup::Catalogue,
    expected: &audit::AuditedRows,
) -> Evidence {
    let attempts: Vec<_> = fs::read_dir(root.join("private-backups"))
        .unwrap()
        .map(|entry| entry.unwrap().path())
        .collect();
    assert_eq!(
        attempts.len(),
        1,
        "exactly one attempt must exist before fresh recovery"
    );
    let attempt = attempts.into_iter().next().unwrap();
    let id = attempt.file_name().unwrap().to_str().unwrap();
    assert_eq!(id.len(), 32);
    assert!(
        id.bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
    );
    assert!(id.bytes().any(|byte| byte != b'0'));
    let original = inventory(&attempt);
    assert!(original[""].directory);
    let root_identity = identity(root, true);
    let live_identity = identity(&root.join("planner.sqlite"), false);
    let (closed, manifest_required) = match barrier {
        "bu_after_directory" | "bu_after_reserve" | "bu_after_first_step" | "bu_after_done" => {
            (false, false)
        }
        "bu_after_database_flush" | "bu_after_asset_copy" | "bu_after_closure" => (true, false),
        "bu_after_manifest_write" | "bu_after_verified" => (true, true),
        "ug_after_begin" | "ug_after_ddl" | "ug_after_insert" | "ug_after_data"
        | "ug_after_ledger" | "ug_after_version" | "ug_before_commit" | "ug_after_commit"
        | "ug_before_ready" => (true, true),
        _ => panic!("unknown backup barrier {barrier}"),
    };
    assert_eq!(
        original.contains_key("verified.json"),
        manifest_required,
        "manifest phase mismatch at {barrier}"
    );
    assert_eq!(
        original.contains_key("database.sqlite"),
        barrier != "bu_after_directory"
    );
    if barrier == "bu_after_directory" {
        assert_eq!(original.len(), 1);
    }
    if barrier == "bu_after_reserve" {
        assert_eq!(original["database.sqlite"].length, Some(0));
        assert_eq!(original.len(), 2);
    }
    let assets_copied =
        barrier == "bu_after_asset_copy" || barrier == "bu_after_closure" || manifest_required;
    let mut asset_records = Vec::new();
    if assets_copied {
        assert!(
            expected.asset_refs.len() <= 1,
            "closed test catalogue has at most one asset"
        );
        for reference in &expected.asset_refs {
            assert_eq!(reference.extension, "png");
            assert_eq!(reference.length, 21);
            assert_eq!(reference.digest, bytes_digest(audit::ASSET_FIXTURE_BYTES).0);
            let name = format!("assets/{}.png", reference.digest);
            let record = file_record(&name, &original);
            assert_eq!(record["length"], reference.length);
            assert_eq!(record["sha256"], reference.digest);
            asset_records.push(record);
        }
    }
    let expected_asset_count = if assets_copied {
        expected.asset_refs.len()
    } else {
        0
    };
    assert_eq!(
        original
            .keys()
            .filter(|name| name.starts_with("assets/"))
            .count(),
        expected_asset_count
    );
    assert_eq!(original.contains_key("assets"), expected_asset_count != 0);
    if closed {
        for suffix in ["-wal", "-shm", "-journal"] {
            assert!(
                !original.contains_key(&format!("database.sqlite{suffix}")),
                "closed copy has a sidecar"
            );
        }
        let copied = attempt.join("database.sqlite");
        // This is an already-closed Online Backup image, never the live source.
        // Native read-only custody prevents writes/deletion during the audit.
        let mut sealed = fs::OpenOptions::new()
            .read(true)
            .share_mode(1)
            .open(&copied)
            .unwrap();
        let mut header = [0u8; 100];
        sealed.read_exact(&mut header).unwrap();
        assert_eq!(&header[..16], b"SQLite format 3\0");
        assert_eq!(
            &header[18..20],
            &[1, 1],
            "closed backup must use DELETE format"
        );
        let read = Connection::open_with_flags_and_vfs(
            &copied,
            OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_PRIVATE_CACHE,
            "win32",
        )
        .unwrap();
        read.execute_batch("PRAGMA query_only=ON; PRAGMA temp_store=MEMORY; PRAGMA foreign_keys=ON; PRAGMA trusted_schema=OFF;").unwrap();
        // AuditedRows includes all historical command/response BLOB bytes in
        // contents_sha256, plus exact counters, ledger and snapshot digests.
        assert_eq!(
            &catalogue.audit(&read).unwrap(),
            expected,
            "closed backup lost exact old source audit at {barrier}"
        );
        read.close().unwrap();
        assert_eq!(
            inventory(&attempt),
            original,
            "read-only inspection changed backup"
        );
        drop(sealed);
        let expected_items = 2
            + usize::from(manifest_required)
            + expected_asset_count
            + usize::from(expected_asset_count != 0);
        assert_eq!(
            original.len(),
            expected_items,
            "unexpected closed backup item"
        );
    } else {
        // Partial copies are physical evidence only: never ask SQLite to read,
        // repair or classify them, including the DONE-before-close barrier.
        for name in original.keys() {
            assert!(
                [
                    "",
                    "database.sqlite",
                    "database.sqlite-wal",
                    "database.sqlite-shm",
                    "database.sqlite-journal"
                ]
                .contains(&name.as_str()),
                "unexpected partial backup item {name}"
            );
        }
    }
    if manifest_required {
        let manifest = original.get("verified.json").unwrap();
        assert!(manifest.length.unwrap() <= 2_097_152);
        let bytes = fs::read(attempt.join("verified.json")).unwrap();
        let parsed: Value = serde_json::from_slice(&bytes).unwrap();
        assert_eq!(
            canonical_bytes(&parsed).unwrap(),
            bytes,
            "manifest bytes are not canonical JSON"
        );
        let required = json!({
            "format":"mstrchf-private-backup/1", "backup_id":id,
            "root_identity":root_identity, "directory_identity":original[""].identity,
            "source":{
                "live_identity":live_identity, "lineage":expected.lineage,
                "storage_version":expected.version,"reader":expected.reader,"closure":expected.closure,
                "counters":expected.counters,"ledger":expected.ledger,
                "schema_sha256":expected.schema_sha256,"contents_sha256":expected.contents_sha256,
                "snapshot_sha256":expected.snapshot_sha256
            },
            "intent":intent(catalogue, expected.version),
            "database":file_record("database.sqlite", &original), "assets":asset_records
        });
        assert_eq!(
            parsed, required,
            "manifest fields differ from independent barrier evidence"
        );
    }
    let line = json!({
        "phase":if manifest_required {"manifest-written"} else if closed {"closed-copy-audited"} else {"partial-physical-only"},
        "attempt":id,"root_identity":root_identity,"live_identity":live_identity,"inventory":original,
        "old_contents_sha256":if closed {Some(&expected.contents_sha256)} else {None},
        "old_snapshot_sha256":if closed {Some(&expected.snapshot_sha256)} else {None}
    }).to_string();
    assert!(line.len() <= 16_384, "bounded fixture evidence exceeded");
    println!("BACKUP_EVIDENCE barrier={barrier} {line}");
    Evidence {
        root: root.into(),
        attempt,
        root_identity,
        live_identity,
        original,
        barrier: barrier.into(),
    }
}
