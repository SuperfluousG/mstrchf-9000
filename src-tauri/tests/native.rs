use super::*;
use mstrchf_contracts::*;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);
fn root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "mstrchf-u1-native-{}-{}-{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos(),
        NEXT.fetch_add(1, Ordering::Relaxed)
    ))
}
fn open(root: &Path) -> Store {
    Store::open(StoreConfig {
        app_data_root: root.to_owned(),
    })
    .unwrap()
}
fn command(id: u64, revision: u64, command: AppCommand) -> String {
    serde_json::to_string(&AppCommandV1 {
        schema_version: SchemaVersion,
        command_id: CommandId(format!("cm_{id:032x}")),
        expected_snapshot_revision: Revision(revision),
        command,
    })
    .unwrap()
}
fn privacy(base: &Snapshot, id: u64) -> String {
    command(
        id,
        base.snapshot_revision.0,
        AppCommand::SetPrivacy {
            policy: base.privacy.clone(),
            expected_policy_revision: base.privacy.revision,
        },
    )
}

fn invoke(
    window: &WebviewWindow<tauri::test::MockRuntime>,
    cmd: &str,
    origin: &str,
    body: Value,
) -> Result<Value, Value> {
    tauri::test::get_ipc_response(
        window,
        tauri::webview::InvokeRequest {
            cmd: cmd.into(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: origin.parse().unwrap(),
            body: tauri::ipc::InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.into(),
        },
    )
    .map(|body| body.deserialize::<Value>().unwrap())
}

#[test]
fn actual_app_manifest_acl_restricts_all_four_custom_commands() {
    // Real generated AppManifest + real capability resolution + real command handlers;
    // only the window/runtime is Tauri's mock. This does not prove WebView integration.
    let app = builder(tauri::test::mock_builder())
        .manage(Arc::new(open(&root())))
        .build(tauri::generate_context!("development/tauri.conf.json"))
        .unwrap();
    let allowed =
        tauri::WebviewWindowBuilder::new(&app, WINDOW, tauri::WebviewUrl::App("index.html".into()))
            .data_directory(root().join("webview"))
            .build()
            .unwrap();
    let denied = tauri::WebviewWindowBuilder::new(
        &app,
        "untrusted",
        tauri::WebviewUrl::App("index.html".into()),
    )
    .data_directory(root().join("webview"))
    .build()
    .unwrap();
    for name in [
        "planner_snapshot",
        "planner_dispatch",
        "planner_features",
        "planner_replay_receipt",
    ] {
        for (window, origin) in [
            (&denied, "http://tauri.localhost/"),
            (&allowed, "https://example.com/"),
        ] {
            let failure = invoke(window, name, origin, json!({"commandJson":"{}"})).unwrap_err();
            assert!(
                failure.as_str().is_some_and(|s| s.contains("not allowed")),
                "must refuse in Tauri ACL before Rust wrapper: {failure}"
            );
        }
    }
    assert_eq!(
        invoke(
            &allowed,
            "planner_features",
            "http://tauri.localhost/",
            json!({})
        )
        .unwrap(),
        serde_json::to_value(ShellFeatures::synthetic()).unwrap()
    );
    let first: Snapshot = serde_json::from_value(
        invoke(
            &allowed,
            "planner_snapshot",
            "http://tauri.localhost/",
            json!({}),
        )
        .unwrap(),
    )
    .unwrap();
    let saved = invoke(
        &allowed,
        "planner_dispatch",
        "http://tauri.localhost/",
        json!({"commandJson": privacy(&first, 1)}),
    )
    .unwrap();
    assert_eq!(saved["receipt_snapshot_revision"], "2");
    assert!(
        invoke(
            &allowed,
            "unregistered_command",
            "http://tauri.localhost/",
            json!({})
        )
        .is_err()
    );
}

#[test]
fn disabled_configuration_refuses_planner_commands_even_if_handlers_are_present() {
    let app = builder(tauri::test::mock_builder())
        .build(tauri::generate_context!("tauri.conf.json"))
        .unwrap();
    let window =
        tauri::WebviewWindowBuilder::new(&app, WINDOW, tauri::WebviewUrl::App("index.html".into()))
            .data_directory(root().join("webview"))
            .build()
            .unwrap();
    for name in [
        "planner_snapshot",
        "planner_dispatch",
        "planner_features",
        "planner_replay_receipt",
    ] {
        assert!(
            invoke(&window, name, "http://tauri.localhost/", json!({}))
                .unwrap_err()
                .as_str()
                .is_some_and(|s| s.contains("not allowed"))
        );
    }
}
fn fixture_app(
    p: &Path,
) -> (
    tauri::App<tauri::test::MockRuntime>,
    WebviewWindow<tauri::test::MockRuntime>,
) {
    let app = builder(tauri::test::mock_builder())
        .manage(Arc::new(open(p)))
        .build(tauri::generate_context!("development/tauri.conf.json"))
        .unwrap();
    let window =
        tauri::WebviewWindowBuilder::new(&app, WINDOW, tauri::WebviewUrl::App("index.html".into()))
            .data_directory(root().join("webview"))
            .build()
            .unwrap();
    (app, window)
}
#[test]
fn actual_wrappers_preserve_current_cancel_rich_and_historical_receipt_literals() {
    use crate::receipt_fixture::{literal, seed, value};
    let p = seed("r2", "cancel-current-initial-snapshot.json", None);
    let (_app, w) = fixture_app(&p);
    let call = |name: &str, body: Value| invoke(&w, name, "http://tauri.localhost/", body).unwrap();
    assert_eq!(
        call(
            "planner_dispatch",
            json!({"commandJson":literal("r2","cancel-current-transport.txt")})
        ),
        value("r2", "cancel-current-applied-receipt.json")
    );
    assert_eq!(
        call("planner_snapshot", json!({})),
        value("r2", "cancel-current-response.json")
    );
    call(
        "planner_dispatch",
        json!({"commandJson":literal("r2","cancel-current-advance-privacy-command.json")}),
    );
    assert_eq!(
        call(
            "planner_dispatch",
            json!({"commandJson":literal("r2","cancel-current-transport.txt")})
        ),
        value("r2", "cancel-current-replayed-receipt.json")
    );
    assert_eq!(
        call("planner_snapshot", json!({})),
        value("r2", "cancel-current-after-privacy-snapshot.json")
    );
    for mode in ["empty", "quick"] {
        let p = seed("r2", "rich-initial-snapshot.json", None);
        let (_app, w) = fixture_app(&p);
        assert_eq!(
            invoke(
                &w,
                "planner_dispatch",
                "http://tauri.localhost/",
                json!({"commandJson":literal("r2",&format!("rich-{mode}-transport.txt"))})
            )
            .unwrap(),
            value("r2", &format!("rich-{mode}-applied-receipt.json"))
        );
        assert_eq!(
            invoke(&w, "planner_snapshot", "http://tauri.localhost/", json!({})).unwrap(),
            value("r2", &format!("rich-{mode}-response.json"))
        );
    }
    for g in ["save", "cancel"] {
        let (group, live, command, response, _expected) = if g == "save" {
            (
                "r1",
                "old-save-response.json",
                "old-save-command.json",
                "old-save-response.json",
                "old-replayed-receipt.json",
            )
        } else {
            (
                "r2",
                "cancel-old-live-before-upgrade.json",
                "cancel-old-command.json",
                "cancel-old-response.json",
                "cancel-old-lookup-receipt.json",
            )
        };
        let p = seed(group, live, Some((command, response)));
        for _ in 0..2 {
            let output = std::process::Command::new(std::env::current_exe().unwrap())
                .args([
                    "--exact",
                    "development::tests::historical_wrapper_child",
                    "--nocapture",
                ])
                .env("MSTRCHF_RECEIPT_CHILD_ROOT", &p)
                .env("MSTRCHF_RECEIPT_CHILD_KIND", g)
                .output()
                .unwrap();
            eprintln!(
                "historical wrapper child: {}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert!(
                output.status.success(),
                "{}\n{}",
                String::from_utf8_lossy(&output.stdout),
                String::from_utf8_lossy(&output.stderr)
            );
            assert_eq!(crate::receipt_fixture::count(&p), 1);
        }
    }
}
#[test]
fn malformed_ipc_argument_shapes_and_injected_strings_never_mutate() {
    let p = crate::receipt_fixture::seed("r1", "current-initial-snapshot.json", None);
    let (_app, w) = fixture_app(&p);
    let before = invoke(&w, "planner_snapshot", "http://tauri.localhost/", json!({})).unwrap();
    for (command, key) in [
        ("planner_dispatch", "commandJson"),
        ("planner_replay_receipt", "originalCommandJson"),
    ] {
        for val in [Value::Null, json!(42), json!({}), json!([])] {
            let mut body = json!({});
            body[key] = val;
            assert!(invoke(&w, command, "http://tauri.localhost/", body).is_err());
        }
        assert!(invoke(&w, command, "http://tauri.localhost/", json!({})).is_err());
        let mut input = crate::receipt_fixture::value("r1", "current-save-command.json");
        input["root"] = json!("caller-root");
        input["profile"] = json!("B1RetainedV1");
        let mut body = json!({});
        body[key] = json!(input.to_string());
        assert!(invoke(&w, command, "http://tauri.localhost/", body).is_err());
    }
    assert_eq!(
        invoke(&w, "planner_snapshot", "http://tauri.localhost/", json!({})).unwrap(),
        before
    );
}
#[test]
fn held_native_dispatch_can_apply_after_lookup_absence_or_replay_after_retry() {
    use crate::receipt_fixture::{literal, seed, value};
    for retry_first in [false, true] {
        let p = seed("r1", "current-initial-snapshot.json", None);
        let (_app, w) = fixture_app(&p);
        let (ready_tx, ready_rx) = std::sync::mpsc::channel();
        let (release_tx, release_rx) = std::sync::mpsc::channel();
        let held_window = w.clone();
        let transport = literal("r1", "transport-current-save.txt");
        let state = _app.state::<Arc<Store>>();
        DISPATCH_BARRIERS
            .get_or_init(Default::default)
            .lock()
            .unwrap()
            .insert(
                Arc::as_ptr(state.inner()) as usize,
                (transport.clone(), ready_tx, release_rx),
            );
        drop(state);
        // Barrier runs inside the actual wrapper, before this Store receives the first call.
        let held = std::thread::spawn(move || {
            invoke(
                &held_window,
                "planner_dispatch",
                "http://tauri.localhost/",
                json!({"commandJson":transport}),
            )
            .unwrap()
        });
        ready_rx.recv().unwrap();
        let absent = invoke(
            &w,
            "planner_replay_receipt",
            "http://tauri.localhost/",
            json!({"originalCommandJson":literal("r1","current-save-command.json")}),
        )
        .unwrap_err();
        assert_eq!(absent["code"], "missing_reference");
        assert_eq!(
            invoke(&w, "planner_snapshot", "http://tauri.localhost/", json!({})).unwrap(),
            value("r1", "current-initial-snapshot.json")
        );
        if retry_first {
            assert_eq!(
                invoke(
                    &w,
                    "planner_dispatch",
                    "http://tauri.localhost/",
                    json!({"commandJson":literal("r1","transport-current-save.txt")})
                )
                .unwrap(),
                value("r1", "current-applied-receipt.json")
            );
        }
        release_tx.send(()).unwrap();
        assert_eq!(
            held.join().unwrap(),
            value(
                "r1",
                if retry_first {
                    "current-replayed-receipt.json"
                } else {
                    "current-applied-receipt.json"
                }
            )
        );
        assert_eq!(
            invoke(&w, "planner_snapshot", "http://tauri.localhost/", json!({})).unwrap(),
            value("r1", "current-save-response.json")
        );
    }
}

// This entire module is cfg(test, native-tests). Keyed to one synthetic Store and
// exact command bytes, consumed once; retry invocations therefore reach Store.
type DispatchBarrier = (
    String,
    std::sync::mpsc::Sender<()>,
    std::sync::mpsc::Receiver<()>,
);
static DISPATCH_BARRIERS: std::sync::OnceLock<
    std::sync::Mutex<std::collections::HashMap<usize, DispatchBarrier>>,
> = std::sync::OnceLock::new();
pub(super) fn before_dispatch(store: &Arc<Store>, text: &str) {
    let key = Arc::as_ptr(store) as usize;
    let entry = {
        let mut barriers = DISPATCH_BARRIERS
            .get_or_init(Default::default)
            .lock()
            .unwrap();
        if barriers.get(&key).is_some_and(|e| e.0 == text) {
            barriers.remove(&key)
        } else {
            None
        }
    };
    if let Some((_, ready, release)) = entry {
        ready.send(()).unwrap();
        release.recv().unwrap();
    }
}

#[test]
fn historical_wrapper_child() {
    // Process lifetime closes MockRuntime's retained manager references before parent reopens.
    // No production state-management API is weakened for test cleanup.
    let Some(p) = std::env::var_os("MSTRCHF_RECEIPT_CHILD_ROOT") else {
        return;
    };
    let kind = std::env::var("MSTRCHF_RECEIPT_CHILD_KIND").unwrap();
    let (_app, w) = fixture_app(Path::new(&p));
    use crate::receipt_fixture::{literal, value};
    let (group, command, expected, current) = match kind.as_str() {
        "save" => (
            "r1",
            "old-save-command.json",
            "old-replayed-receipt.json",
            "old-save-current-after-upgrade.json",
        ),
        "cancel" => (
            "r2",
            "cancel-old-command.json",
            "cancel-old-lookup-receipt.json",
            "cancel-old-current-after-upgrade.json",
        ),
        _ => panic!("unknown child fixture"),
    };
    assert_eq!(
        invoke(
            &w,
            "planner_replay_receipt",
            "http://tauri.localhost/",
            json!({"originalCommandJson":literal(group,command)})
        )
        .unwrap(),
        value(group, expected)
    );
    if kind == "cancel" {
        assert_eq!(
            invoke(
                &w,
                "planner_dispatch",
                "http://tauri.localhost/",
                json!({"commandJson":literal("r2","cancel-old-current-transport.txt")})
            )
            .unwrap(),
            value("r2", "cancel-old-dispatch-receipt.json")
        );
    }
    assert_eq!(
        invoke(&w, "planner_snapshot", "http://tauri.localhost/", json!({})).unwrap(),
        value(group, current)
    );
}
