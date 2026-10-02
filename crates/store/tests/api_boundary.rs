use serde_json::Value;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::{Command, Output};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT_TEMP: AtomicU64 = AtomicU64::new(0);

const PUBLIC_CONTROL_SOURCE: &str = r#"
fn main() {
    let _: Option<mstrchf_store::Store> = None;
    let _: Option<mstrchf_store::StoreConfig> = None;
}
"#;

const TESTS_MODULE_SOURCE: &str = r#"
fn main() {
    mstrchf_store::tests::fresh_database_has_real_policy_and_no_onboarding();
}
"#;

fn unique_temp_root() -> PathBuf {
    let nonce = NEXT_TEMP.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir().join(format!(
        "mstrchf-s31-api-boundary-{}-{}-{nonce}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ))
}

fn fixture_path(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/api-boundary")
        .join(relative)
}

fn write_downstream_fixture(temp_root: &Path, source_override: Option<&str>) -> PathBuf {
    let source_root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_manifest = fs::read_to_string(fixture_path("Cargo.toml")).unwrap();
    let store_path = source_root.to_string_lossy().replace('\\', "/");
    let fixture_manifest =
        fixture_manifest.replace("path = \"../..\"", &format!("path = \"{store_path}\""));
    assert_ne!(
        fixture_manifest,
        fs::read_to_string(fixture_path("Cargo.toml")).unwrap()
    );

    let source_main = fs::read_to_string(fixture_path("src/main.rs")).unwrap();
    assert!(!source_main.trim().is_empty());
    fs::create_dir_all(temp_root.join("src")).unwrap();
    fs::write(temp_root.join("Cargo.toml"), fixture_manifest).unwrap();
    fs::write(
        temp_root.join("src/main.rs"),
        source_override.unwrap_or(&source_main),
    )
    .unwrap();
    temp_root.join("Cargo.toml")
}

fn cargo_check(manifest: &Path, target_dir: &Path) -> Output {
    Command::new(env!("CARGO"))
        .arg("check")
        .arg("--offline")
        .arg("--manifest-path")
        .arg(manifest)
        .arg("--target-dir")
        .arg(target_dir)
        .arg("--message-format=json")
        .env("CARGO_TERM_COLOR", "never")
        .output()
        .expect("run downstream cargo check")
}

fn diagnostic_errors(output: &Output) -> Vec<(Option<String>, String)> {
    String::from_utf8_lossy(&output.stdout)
        .lines()
        .filter_map(|line| serde_json::from_str::<Value>(line).ok())
        .filter(|message| message["reason"] == "compiler-message")
        .filter_map(|message| {
            let diagnostic = &message["message"];
            (diagnostic["level"] == "error").then(|| {
                (
                    diagnostic["code"]["code"].as_str().map(str::to_owned),
                    diagnostic["rendered"]
                        .as_str()
                        .unwrap_or_default()
                        .to_owned(),
                )
            })
        })
        .collect()
}

fn assert_success(output: &Output) {
    assert!(
        output.status.success(),
        "downstream public API control failed:\n{}\n{}",
        String::from_utf8_lossy(&output.stdout),
        String::from_utf8_lossy(&output.stderr)
    );
}

fn assert_only_error_code(output: &Output, expected: &str, diagnostic_hint: &str) {
    assert!(
        !output.status.success(),
        "negative compile unexpectedly passed"
    );
    let errors = diagnostic_errors(output);
    assert!(
        !errors.is_empty(),
        "no rustc errors found in cargo JSON output"
    );
    for (code, rendered) in &errors {
        assert_eq!(code.as_deref(), Some(expected), "{rendered}");
        assert!(
            rendered
                .to_lowercase()
                .contains(&diagnostic_hint.to_lowercase()),
            "expected diagnostic containing {diagnostic_hint:?}, got {rendered}"
        );
    }
    println!(
        "downstream negative: expected {expected}, observed {} matching compiler error(s)",
        errors.len()
    );
}

#[test]
fn downstream_consumers_cannot_reach_internal_mutation_or_test_modules() {
    let temp_root = unique_temp_root();
    let target_dir = temp_root.join("target");
    let manifest =
        write_downstream_fixture(&temp_root.join("consumer"), Some(PUBLIC_CONTROL_SOURCE));
    assert_success(&cargo_check(&manifest, &target_dir));
    println!("downstream public control compiled with resolved dependencies");

    let manifest = write_downstream_fixture(&temp_root.join("consumer"), None);
    assert_only_error_code(&cargo_check(&manifest, &target_dir), "E0603", "private");

    let manifest = write_downstream_fixture(&temp_root.join("consumer"), Some(TESTS_MODULE_SOURCE));
    assert_only_error_code(&cargo_check(&manifest, &target_dir), "E0433", "tests");

    let manifest = write_downstream_fixture(
        &temp_root.join("consumer"),
        Some(
            "fn main() { let _: Option<mstrchf_store::backup::VerifiedBackupReceipt> = None; let _: Option<mstrchf_store::custody::SealedFile> = None; }",
        ),
    );
    assert_only_error_code(&cargo_check(&manifest, &target_dir), "E0603", "private");
    let manifest = write_downstream_fixture(
        &temp_root.join("consumer"),
        Some("fn main() { mstrchf_store::upgrade_fixture::apply(); }"),
    );
    assert_only_error_code(
        &cargo_check(&manifest, &target_dir),
        "E0433",
        "upgrade_fixture",
    );
}

#[test]
fn normal_consumer_binary_refuses_closed_fixture_v2_without_changing_source() {
    // This integration executable links the ordinary library (cfg(test) is off
    // for that library). It cannot select the private fixture catalogue.
    let root = unique_temp_root();
    drop(
        mstrchf_store::Store::open(mstrchf_store::StoreConfig {
            app_data_root: root.clone(),
        })
        .unwrap(),
    );
    let path = root.join("planner.sqlite");
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute_batch("CREATE TABLE fixture_probe(id INTEGER PRIMARY KEY CHECK(id=1), value INTEGER NOT NULL) STRICT; INSERT INTO fixture_probe VALUES(1,7);").unwrap();
    c.execute(
        "UPDATE schema_migrations SET checksum=?1 WHERE version=1",
        [
            mstrchf_contracts::bytes_digest(include_bytes!(
                "fixtures/backup-upgrade/fixture_v1.sql"
            ))
            .0,
        ],
    )
    .unwrap();
    let sql = include_str!("fixtures/backup-upgrade/fixture_v1_to_v2.sql");
    c.execute_batch(sql).unwrap();
    c.execute(
        "INSERT INTO schema_migrations VALUES(2,?1,'2026-10-02T00:00:00Z')",
        [mstrchf_contracts::bytes_digest(sql.as_bytes()).0],
    )
    .unwrap();
    c.pragma_update(None, "user_version", 2).unwrap();
    c.close().unwrap();
    let before = fs::read(&path).unwrap();
    assert_eq!(
        mstrchf_store::Store::open(mstrchf_store::StoreConfig {
            app_data_root: root.clone()
        })
        .unwrap_err()
        .code,
        mstrchf_contracts::ErrorCode::UnsupportedVersion
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    for suffix in ["-wal", "-shm", "-journal"] {
        assert!(!root.join(format!("planner.sqlite{suffix}")).exists());
    }
    assert!(!root.join("private-backups").exists());
}
