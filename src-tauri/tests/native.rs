use super::*;
use mstrchf_contracts::*;
use serde_json::{Value, json};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

static NEXT: AtomicU64 = AtomicU64::new(0);

#[test]
fn meal_choice_survives_store_reopen_through_native_commands() {
    let p = root();
    let mut written = None;
    for phase in ["write", "read"] {
        let result = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "development::tests::m1_persistence_child",
                "--nocapture",
            ])
            .env("MSTRCHF_M1_CHILD_ROOT", &p)
            .env("MSTRCHF_M1_CHILD_PHASE", phase)
            .output()
            .unwrap();
        assert!(
            result.status.success(),
            "{phase}: {}\n{}",
            String::from_utf8_lossy(&result.stdout),
            String::from_utf8_lossy(&result.stderr)
        );
        let stdout = String::from_utf8(result.stdout).unwrap();
        let line = stdout
            .lines()
            .find_map(|line| line.strip_prefix("M1_SAVED_IDENTITY="))
            .expect("child must report its actual saved week and selection");
        let actual: Value = serde_json::from_str(line).unwrap();
        if phase == "write" {
            written = Some(actual);
        } else {
            assert_eq!(
                Some(actual),
                written,
                "entire saved slot, recipe digest and servings must survive process exit"
            );
        }
    }
}

#[test]
fn m1_persistence_child() {
    let Some(p) = std::env::var_os("MSTRCHF_M1_CHILD_ROOT") else {
        return;
    };
    let phase = std::env::var("MSTRCHF_M1_CHILD_PHASE").unwrap();
    let app = builder(tauri::test::mock_builder())
        .manage(Arc::new(
            Store::open_development(StoreConfig {
                app_data_root: PathBuf::from(p),
            })
            .unwrap(),
        ))
        .build(tauri::generate_context!("development/tauri.conf.json"))
        .unwrap();
    let w =
        tauri::WebviewWindowBuilder::new(&app, WINDOW, tauri::WebviewUrl::App("index.html".into()))
            .data_directory(root().join("webview"))
            .build()
            .unwrap();
    let read = || -> Snapshot {
        serde_json::from_value(
            invoke(&w, "planner_snapshot", "http://tauri.localhost/", json!({})).unwrap(),
        )
        .unwrap()
    };
    if phase == "write" {
        let s = read();
        let profile = mstrchf_store::development::seed_profile(&s).unwrap();
        let result = invoke(
            &w,
            "planner_dispatch",
            "http://tauri.localhost/",
            json!({"commandJson":command(100,s.snapshot_revision.0,AppCommand::UpdateProfile { profile, expected_profile_revision:Revision(1) })}),
        );
        assert!(
            result.is_ok(),
            "UpdateProfile must be persisted through the registered native handler: {result:?}"
        );
        let s = read();
        let at = mstrchf_store::development::now().unwrap();
        let week = mstrchf_store::development::seed_week(&s, &at).unwrap();
        assert_eq!(
            week.range.monday.0,
            first_test_monday(&at, &week.range.zone)
        );
        let mut edit = json!({"week_id":week.range.week_id,"defaults_snapshot":week.defaults_snapshot,"day_enabled":week.day_enabled,"meal_type_enabled":week.meal_type_enabled,"cells":week.cells,"plan":week.plan});
        let save = |id: u64, s: &Snapshot, edit: Value, rev: u64| {
            json!({"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":s.snapshot_revision.0.to_string(),"command":{"kind":"save_week","data":{"week":edit,"expected_week_revision":rev.to_string()}}}).to_string()
        };
        invoke(
            &w,
            "planner_dispatch",
            "http://tauri.localhost/",
            json!({"commandJson":save(101,&s,edit.clone(),1)}),
        )
        .unwrap();
        let board = invoke(
            &w,
            "planner_development_board",
            "http://tauri.localhost/",
            json!({}),
        )
        .unwrap();
        assert_eq!(board["choices"].as_array().unwrap().len(), 2);
        assert!(
            board["choices"]
                .as_array()
                .unwrap()
                .iter()
                .all(|c| c["title"].as_str().unwrap().contains("vegetarian"))
        );
        edit["plan"]["selections"] = json!([board["choices"][0]["selection"].clone()]);
        let s = read();
        let original = save(102, &s, edit.clone(), 1);
        invoke(
            &w,
            "planner_dispatch",
            "http://tauri.localhost/",
            json!({"commandJson":original.clone()}),
        )
        .unwrap();
        let saved = read();
        assert_eq!(saved.snapshot_revision.0, 4);
        assert_eq!(saved.weeks[0].plan.selections.len(), 1);
        // Both snapshot and week CAS refuse; no previously chosen meal is replaced.
        let stale = invoke(
            &w,
            "planner_dispatch",
            "http://tauri.localhost/",
            json!({"commandJson":save(103,&s,edit.clone(),1)}),
        )
        .unwrap_err();
        assert_eq!(stale["code"], "context_stale");
        let stale_week = invoke(
            &w,
            "planner_dispatch",
            "http://tauri.localhost/",
            json!({"commandJson":save(105,&saved,edit.clone(),1)}),
        )
        .unwrap_err();
        assert_eq!(stale_week["code"], "context_stale");
        let mut incompatible = edit.clone();
        incompatible["plan"]["revision"] = json!("2");
        incompatible["plan"]["selections"][0]["recipe"] = serde_json::to_value(
            mstrchf_store::development::recipe_reference(
                &mstrchf_store::development::catalogue().unwrap()[2],
            )
            .unwrap(),
        )
        .unwrap();
        incompatible["plan"]["selections"][0]["servings"][0]["preset_ref"]["recipe"] =
            incompatible["plan"]["selections"][0]["recipe"].clone();
        let forged = invoke(
            &w,
            "planner_dispatch",
            "http://tauri.localhost/",
            json!({"commandJson":save(104,&saved,incompatible,2)}),
        )
        .unwrap_err();
        assert_eq!(forged["code"], "domain_conflict");
        let mut wrong_portion = edit.clone();
        wrong_portion["plan"]["revision"] = json!("2");
        wrong_portion["plan"]["selections"][0]["servings"][0]["servings"] =
            json!({"numerator":"2","denominator":"1"});
        let forged_portion = invoke(
            &w,
            "planner_dispatch",
            "http://tauri.localhost/",
            json!({"commandJson":save(106,&saved,wrong_portion,2)}),
        )
        .unwrap_err();
        assert_eq!(forged_portion["code"], "identity_mismatch");
        assert_eq!(
            serde_json::to_value(read()).unwrap(),
            serde_json::to_value(&saved).unwrap()
        );
        let replay = invoke(
            &w,
            "planner_dispatch",
            "http://tauri.localhost/",
            json!({"commandJson":original}),
        )
        .unwrap();
        assert_eq!(replay["disposition"], "replayed");
        assert_eq!(read().snapshot_revision.0, 4);
    }
    let saved = read();
    let selection = &saved.weeks[0].plan.selections[0];
    assert_eq!(saved.snapshot_revision.0, 4);
    assert_eq!(saved.weeks[0].revision.0, 2);
    assert_eq!(
        selection.slot.week_id.0,
        "wk_00000000000000000000000000000001"
    );
    assert_eq!(selection.slot.date, saved.weeks[0].range.monday);
    assert_eq!(selection.slot.meal_type, MealType::Dinner);
    assert_eq!(
        selection.recipe.recipe_id.0,
        "rc_00000000000000000000000000000001"
    );
    assert_eq!(selection.recipe.revision.0, 1);
    assert_eq!(
        selection.servings[0].servings,
        PositiveRational::new(1, 1).unwrap()
    );
    println!(
        "M1_SAVED_IDENTITY={}",
        json!({"week":saved.weeks[0].range,"selection":selection})
    );
}
fn first_test_monday(at: &Instant, zone: &ZoneId) -> String {
    use chrono::{Datelike, Days, Weekday};
    let zone: chrono_tz::Tz = zone.0.parse().unwrap();
    let today = chrono::DateTime::parse_from_rfc3339(&at.0)
        .unwrap()
        .with_timezone(&zone)
        .date_naive();
    let mut date = today.checked_add_days(Days::new(7)).unwrap();
    while date.weekday() != Weekday::Mon {
        date = date.checked_add_days(Days::new(1)).unwrap();
    }
    date.to_string()
}
#[test]
fn persistence_horizon_oracle_covers_week_rollover_and_year_boundary() {
    let zone = ZoneId("Europe/London".into());
    for (at, expected) in [
        ("2026-10-12T12:00:00Z", "2026-10-19"),
        ("2026-10-13T12:00:00Z", "2026-10-26"),
        ("2026-12-22T12:00:00Z", "2027-01-04"),
    ] {
        assert_eq!(first_test_monday(&Instant(at.into()), &zone), expected);
    }
}
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
fn actual_app_manifest_acl_restricts_all_five_custom_commands() {
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
        "planner_development_board",
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
        "planner_development_board",
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
