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

    let contracts_path = source_root
        .join("../contracts")
        .to_string_lossy()
        .replace('\\', "/");
    let source_main = fs::read_to_string(fixture_path("src/main.rs")).unwrap();
    assert!(!source_main.trim().is_empty());
    fs::create_dir_all(temp_root.join("src")).unwrap();
    fs::write(
        temp_root.join("Cargo.toml"),
        format!("{fixture_manifest}\nserde = \"=1.0.229\"\nmstrchf-contracts = {{ path = \"{contracts_path}\" }}\n"),
    )
    .unwrap();
    fs::write(
        temp_root.join("src/main.rs"),
        source_override.unwrap_or(&source_main),
    )
    .unwrap();
    temp_root.join("Cargo.toml")
}

// Resolve the exact toolchain that built this test before running any executable.
// In particular, never invoke a Rustup proxy or a floating stable/nightly channel.
#[derive(Debug)]
struct ExactToolchain {
    cargo: PathBuf,
    rustc: PathBuf,
    rustdoc: PathBuf,
    name: String,
    release: String,
    host: String,
}
impl ExactToolchain {
    fn layout(cargo: &Path) -> Result<Self, String> {
        let bin = cargo.parent().ok_or("Cargo has no parent")?;
        let root = bin.parent().ok_or("Cargo has no toolchain root")?;
        let name = root
            .file_name()
            .and_then(|v| v.to_str())
            .ok_or("toolchain name")?;
        let (release, host) = name.split_once('-').ok_or("expected numbered toolchain")?;
        let parts: Vec<_> = release.split('.').collect();
        if !cargo.is_absolute()
            || cargo.file_name() != Some(std::ffi::OsStr::new("cargo.exe"))
            || bin.file_name() != Some(std::ffi::OsStr::new("bin"))
            || root.parent().and_then(Path::file_name) != Some(std::ffi::OsStr::new("toolchains"))
            || parts.len() != 3
            || parts
                .iter()
                .any(|p| p.is_empty() || !p.bytes().all(|c| c.is_ascii_digit()))
            || host.is_empty()
        {
            return Err("Cargo must be inside an exact numbered installed toolchain; no proxy/channel fallback".into());
        }
        Ok(Self {
            cargo: cargo.into(),
            rustc: bin.join("rustc.exe"),
            rustdoc: bin.join("rustdoc.exe"),
            name: name.into(),
            release: release.into(),
            host: host.into(),
        })
    }

    fn command(&self, exe: &Path) -> Command {
        let mut command = Command::new(exe);
        let bin = self.cargo.parent().unwrap();
        let paths = std::iter::once(bin.to_path_buf())
            .chain(std::env::split_paths(
                &std::env::var_os("PATH").unwrap_or_default(),
            ))
            .collect::<Vec<_>>();
        command
            .env(
                "PATH",
                std::env::join_paths(paths).expect("exact toolchain PATH"),
            )
            .env("CARGO", &self.cargo)
            .env("RUSTC", &self.rustc)
            .env("RUSTDOC", &self.rustdoc)
            .env("RUSTUP_TOOLCHAIN", &self.name)
            .env("RUSTUP_AUTO_INSTALL", "0")
            // Empty primary settings override alternate environment and Cargo config.
            .env("RUSTC_WRAPPER", "")
            .env("RUSTC_WORKSPACE_WRAPPER", "");
        command
    }

    fn verify_versions(&self, cargo: &str, rustc: &str, rustdoc: &str) -> Result<(), String> {
        let field = |text: &str, name: &str| {
            text.lines()
                .find_map(|l| l.strip_prefix(name))
                .map(str::trim)
                .map(str::to_owned)
        };
        for (kind, text) in [("cargo", cargo), ("rustc", rustc), ("rustdoc", rustdoc)] {
            if !text.starts_with(&format!("{kind} "))
                || field(text, "release:").as_deref() != Some(self.release.as_str())
                || field(text, "host:").as_deref() != Some(self.host.as_str())
            {
                return Err(format!(
                    "{kind} does not match exact Cargo toolchain {}",
                    self.name
                ));
            }
        }
        if field(rustc, "commit-hash:").is_none()
            || field(rustc, "commit-hash:") != field(rustdoc, "commit-hash:")
        {
            return Err("rustdoc and rustc compiler identities differ".into());
        }
        Ok(())
    }

    fn from_cargo(build_cargo: &Path) -> Result<Self, String> {
        // Validate the unexecuted path first, then its actual filesystem identity.
        Self::layout(build_cargo)?;
        let cargo = fs::canonicalize(build_cargo).map_err(|e| e.to_string())?;
        let mut toolchain = Self::layout(&cargo)?;
        for executable in [&mut toolchain.rustc, &mut toolchain.rustdoc] {
            *executable = fs::canonicalize(&*executable).map_err(|e| e.to_string())?;
            if executable.parent() != toolchain.cargo.parent() {
                return Err("compiler escaped the exact Cargo toolchain directory".into());
            }
        }
        let version = |exe: &Path| -> Result<String, String> {
            let output = toolchain
                .command(exe)
                .arg("-Vv")
                .output()
                .map_err(|e| e.to_string())?;
            if !output.status.success() {
                return Err(format!("cannot establish tool identity: {}", exe.display()));
            }
            String::from_utf8(output.stdout).map_err(|e| e.to_string())
        };
        let cargo = version(&toolchain.cargo)?;
        let rustc = version(&toolchain.rustc)?;
        let rustdoc = version(&toolchain.rustdoc)?;
        toolchain.verify_versions(&cargo, &rustc, &rustdoc)?;
        println!(
            "Exact downstream tools: {} | {} | {}; release={} host={} auto_install=0",
            toolchain.cargo.display(),
            toolchain.rustc.display(),
            toolchain.rustdoc.display(),
            toolchain.release,
            toolchain.host
        );
        Ok(toolchain)
    }
}

fn cargo_check(manifest: &Path, target_dir: &Path) -> Output {
    static TOOLCHAIN: std::sync::OnceLock<ExactToolchain> = std::sync::OnceLock::new();
    let toolchain = TOOLCHAIN.get_or_init(|| {
        ExactToolchain::from_cargo(Path::new(env!("CARGO"))).unwrap_or_else(|e| {
            panic!(
                "refusing downstream compiler launch: {e}; build Cargo={}",
                env!("CARGO")
            )
        })
    });
    toolchain
        .command(&toolchain.cargo)
        .arg("check")
        .arg("--offline")
        .arg("--manifest-path")
        .arg(manifest)
        .arg("--target-dir")
        .arg(target_dir)
        .arg("--message-format=json")
        .env("CARGO_TERM_COLOR", "never")
        .output()
        .expect("run exact downstream cargo check")
}

#[test]
fn compiler_launcher_rejects_proxy_channel_missing_and_mismatched_identity() {
    let good = ExactToolchain::layout(Path::new(env!("CARGO"))).unwrap();
    for path in [
        PathBuf::from("cargo.exe"),
        good.cargo
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .parent()
            .unwrap()
            .join("stable-x86_64-pc-windows-msvc/bin/cargo.exe"),
        std::env::temp_dir().join(".cargo/bin/cargo.exe"),
    ] {
        assert!(ExactToolchain::layout(&path).is_err(), "{}", path.display());
    }
    let absent = unique_temp_root()
        .join("toolchains")
        .join(&good.name)
        .join("bin/cargo.exe");
    assert!(ExactToolchain::from_cargo(&absent).is_err());
    let command = good.command(&good.cargo);
    let environment: std::collections::BTreeMap<_, _> = command.get_envs().collect();
    for (key, value) in [
        ("CARGO", good.cargo.as_os_str()),
        ("RUSTC", good.rustc.as_os_str()),
        ("RUSTDOC", good.rustdoc.as_os_str()),
        ("RUSTUP_TOOLCHAIN", std::ffi::OsStr::new(&good.name)),
        ("RUSTUP_AUTO_INSTALL", std::ffi::OsStr::new("0")),
    ] {
        assert_eq!(
            environment.get(std::ffi::OsStr::new(key)),
            Some(&Some(value))
        );
    }
    for key in ["RUSTC_WRAPPER", "RUSTC_WORKSPACE_WRAPPER"] {
        assert_eq!(
            environment.get(std::ffi::OsStr::new(key)),
            Some(&Some(std::ffi::OsStr::new("")))
        );
    }
    let version = |kind: &str| {
        format!(
            "{kind} {}\nrelease: {}\nhost: {}\ncommit-hash: exact\n",
            good.release, good.release, good.host
        )
    };
    let cargo = version("cargo");
    let rustc = version("rustc");
    let rustdoc = version("rustdoc");
    assert!(good.verify_versions(&cargo, &rustc, &rustdoc).is_ok());
    for (c, r, d) in [
        ("".to_owned(), rustc.clone(), rustdoc.clone()),
        (
            cargo.clone(),
            rustc.replace(&good.release, "99.0.0"),
            rustdoc.clone(),
        ),
        (
            cargo.clone(),
            rustc.clone(),
            rustdoc.replace(&good.host, "foreign-host"),
        ),
        (
            cargo.clone(),
            rustc.clone(),
            rustdoc.replace("commit-hash: exact", "commit-hash: other"),
        ),
    ] {
        assert!(good.verify_versions(&c, &r, &d).is_err());
    }
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
    for (source, code, hint) in [
        (
            "fn f(o:&mstrchf_store::UiCommandOutcome){o.original_response_bytes()[0]=0;} fn main(){}",
            "E0594",
            "cannot assign",
        ),
        (
            "fn f(s:&mstrchf_store::HistoricalB1Snapshot){s.household_draft().diners().clear();} fn main(){}",
            "E0596",
            "cannot borrow",
        ),
        (
            "fn f(s:&mstrchf_store::Draft005ReceiptSnapshot){s.household_draft().diners().clear();} fn main(){}",
            "E0596",
            "cannot borrow",
        ),
        (
            "fn f(s:&mstrchf_store::HistoricalB1Snapshot){*s.household_draft().diners()[0].restrictions()=mstrchf_store::historical_b1::RestrictionDeclaration::Unconfirmed{};} fn main(){}",
            "E0594",
            "cannot assign",
        ),
        (
            "fn f(s:&mstrchf_store::Draft005ReceiptSnapshot){*s.household_draft().diners()[0].restrictions()=mstrchf_store::draft005_receipt::RestrictionDeclaration::Unconfirmed{};} fn main(){}",
            "E0594",
            "cannot assign",
        ),
        (
            "fn main(){let _=mstrchf_store::HistoricalB1Snapshot(Default::default());}",
            "E0423",
            "private",
        ),
        (
            "fn main(){let _=mstrchf_store::Draft005ReceiptSnapshot(Default::default());}",
            "E0423",
            "private",
        ),
        (
            r#"fn f(response:mstrchf_store::ReceiptSnapshot)->mstrchf_store::UiCommandOutcome{
            mstrchf_store::UiCommandOutcome {
                disposition:mstrchf_store::CommandDisposition::Replayed, response,
                command:vec![], bytes:vec![],
                command_digest:mstrchf_contracts::Digest("".into()),
                response_digest:mstrchf_contracts::Digest("".into())
            }
        } fn main(){}"#,
            "E0451",
            "private",
        ),
        (
            "fn main(){mstrchf_store::historical_b1::snapshot(&[]);mstrchf_store::draft005_receipt::snapshot(&[]);}",
            "E0425",
            "snapshot",
        ),
        (
            "fn main(){mstrchf_store::historical_b1::transition_draft(&[]);}",
            "E0425",
            "transition_draft",
        ),
    ] {
        let manifest = write_downstream_fixture(&temp_root.join("consumer"), Some(source));
        assert_only_error_code(&cargo_check(&manifest, &target_dir), code, hint);
    }

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
    let manifest = write_downstream_fixture(
        &temp_root.join("consumer"),
        Some(
            r#"
fn decode<T:serde::de::DeserializeOwned>(){}
fn main(){ decode::<mstrchf_store::UiCommandOutcome>(); decode::<mstrchf_store::HistoricalB1Snapshot>(); decode::<mstrchf_store::Draft005ReceiptSnapshot>(); }
"#,
        ),
    );
    assert_only_error_code(&cargo_check(&manifest, &target_dir), "E0277", "Deserialize");
    let manifest = write_downstream_fixture(
        &temp_root.join("consumer"),
        Some(
            r#"
fn current<T: Into<mstrchf_contracts::Snapshot>>() {}
fn main() { current::<mstrchf_store::HistoricalB1Snapshot>(); current::<mstrchf_store::Draft005ReceiptSnapshot>(); }
"#,
        ),
    );
    assert_only_error_code(&cargo_check(&manifest, &target_dir), "E0277", "From");

    let manifest = write_downstream_fixture(
        &temp_root.join("consumer"),
        Some("fn main(){ let _: Option<mstrchf_store::upgrade::UpgradeOutcome> = None; }"),
    );
    assert_only_error_code(&cargo_check(&manifest, &target_dir), "E0603", "private");
}

#[test]
fn normal_consumer_binary_refuses_closed_fixture_v2_without_changing_source() {
    // This integration executable links the ordinary library (cfg(test) is off
    // for that library). It cannot select the private fixture catalogue.
    let root = unique_temp_root();
    fs::create_dir_all(&root).unwrap();
    let path = root.join("planner.sqlite");
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute_batch(&include_str!("fixtures/backup-upgrade/fixture_v1.sql").replace("\r\n", "\n"))
        .unwrap();
    c.execute(
        "INSERT INTO schema_migrations VALUES(1,?1,'2026-10-02T00:00:00Z')",
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
        mstrchf_contracts::ErrorCode::StorageFailure
    );
    assert_eq!(fs::read(&path).unwrap(), before);
    for suffix in ["-wal", "-shm", "-journal"] {
        assert!(!root.join(format!("planner.sqlite{suffix}")).exists());
    }
    assert!(!root.join("private-backups").exists());
}

#[test]
fn downstream_traverses_every_frozen_response_enum_in_both_profiles() {
    let temp_root = unique_temp_root();
    for (profile, snapshot) in [
        ("historical_b1", "HistoricalB1Snapshot"),
        ("draft005_receipt", "Draft005ReceiptSnapshot"),
    ] {
        let source = include_str!("frozen_navigation.rs.txt")
            .replace("__PROFILE__", profile)
            .replace("__SNAPSHOT__", snapshot)
            + "\nfn main() {}\n";
        let manifest = write_downstream_fixture(&temp_root.join(profile), Some(&source));
        assert_success(&cargo_check(&manifest, &temp_root.join("target")));
        println!("Traversed all23 reachable enums: {profile}");
    }
}

// Each probe gets only a child environment and a uniquely owned temporary cwd.
// No process-wide environment or user/global Cargo configuration is changed.
#[test]
fn compiler_wrapper_configurations_cannot_interpose() {
    let root = unique_temp_root();
    let mut failures = Vec::new();
    for (label, environment, config_key) in [
        ("alternate-wrapper", Some("CARGO_BUILD_RUSTC_WRAPPER"), None),
        (
            "alternate-workspace-wrapper",
            Some("CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER"),
            None,
        ),
        ("config-wrapper", None, Some("rustc-wrapper")),
        (
            "config-workspace-wrapper",
            None,
            Some("rustc-workspace-wrapper"),
        ),
    ] {
        let scope = root.join(label);
        fs::create_dir_all(&scope).unwrap();
        let missing = scope.join("must-not-run-wrapper.exe");
        assert!(!missing.exists());
        if let Some(key) = config_key {
            fs::create_dir_all(scope.join(".cargo")).unwrap();
            let path = missing.to_string_lossy().replace('\\', "/");
            fs::write(
                scope.join(".cargo/config.toml"),
                format!("[build]\n{key} = \"{path}\"\n"),
            )
            .unwrap();
        }
        let mut child = Command::new(std::env::current_exe().unwrap());
        child
            .arg("--exact")
            .arg("compiler_wrapper_probe_child")
            .arg("--nocapture")
            .current_dir(&scope)
            .env("MSTRCHF_WRAPPER_PROBE_ROOT", &scope)
            .env_remove("RUSTC_WRAPPER")
            .env_remove("RUSTC_WORKSPACE_WRAPPER")
            .env_remove("CARGO_BUILD_RUSTC_WRAPPER")
            .env_remove("CARGO_BUILD_RUSTC_WORKSPACE_WRAPPER");
        if let Some(key) = environment {
            child.env(key, &missing);
        }
        let output = child.output().expect("run isolated actual-launcher probe");
        fs::write(scope.join("stdout.log"), &output.stdout).unwrap();
        fs::write(scope.join("stderr.log"), &output.stderr).unwrap();
        println!(
            "wrapper probe {label}: status={} evidence={}",
            output.status,
            scope.display()
        );
        if !output.status.success() {
            failures.push(format!(
                "{label}:\n{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            ));
        }
        assert!(!missing.exists());
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}

#[test]
fn compiler_wrapper_probe_child() {
    let Some(root) = std::env::var_os("MSTRCHF_WRAPPER_PROBE_ROOT") else {
        return;
    };
    let root = PathBuf::from(root);
    let consumer = root.join("consumer");
    fs::create_dir_all(consumer.join("src")).unwrap();
    fs::write(consumer.join("Cargo.toml"),
        "[package]\nname = \"wrapper-probe\"\nversion = \"0.0.0\"\nedition = \"2024\"\n[workspace]\n").unwrap();
    let source = consumer.join("src/main.rs");
    fs::write(&source, "fn main() {}\n").unwrap();
    let manifest = consumer.join("Cargo.toml");
    let target = root.join("target");
    let positive = cargo_check(&manifest, &target);
    fs::write(root.join("positive.stdout.log"), &positive.stdout).unwrap();
    fs::write(root.join("positive.stderr.log"), &positive.stderr).unwrap();
    assert_success(&positive);
    fs::write(&source, "fn main() { let _: u32 = true; }\n").unwrap();
    let negative = cargo_check(&manifest, &target);
    fs::write(root.join("negative.stdout.log"), &negative.stdout).unwrap();
    fs::write(root.join("negative.stderr.log"), &negative.stderr).unwrap();
    assert_only_error_code(&negative, "E0308", "mismatched types");
}
