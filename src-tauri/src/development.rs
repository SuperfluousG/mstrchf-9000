//! Compiled only for the explicitly selected debug synthetic-development entry.
use crate::boundary::{development_root, dispatch_json, replay_json};
use crate::features::ShellFeatures;
use crate::receipt::ShellCommandReceipt;
use mstrchf_contracts::{DomainError, ErrorCode, Snapshot};
use mstrchf_store::{Store, StoreConfig};
use std::sync::Arc;
use tauri::{Manager, Runtime, State, WebviewWindow};

pub(crate) const WINDOW: &str = "development-main";

pub(crate) fn authorise(label: &str, url: &str) -> Result<(), DomainError> {
    let allowed = tauri::Url::parse(url).is_ok_and(|url| {
        url.username().is_empty()
            && url.password().is_none()
            && ((url.scheme() == "http"
                && url.host_str() == Some("127.0.0.1")
                && url.port() == Some(1420))
                || (url.scheme() == "http"
                    && url.host_str() == Some("tauri.localhost")
                    && url.port().is_none())
                || (url.scheme() == "tauri"
                    && url.host_str() == Some("localhost")
                    && url.port().is_none()))
    });
    if label == WINDOW && allowed {
        Ok(())
    } else {
        Err(DomainError::new(
            ErrorCode::AuthorityDenied,
            "/shell/window",
        ))
    }
}

fn check_window<R: Runtime>(window: &WebviewWindow<R>) -> Result<(), DomainError> {
    let url = window
        .url()
        .map_err(|_| DomainError::new(ErrorCode::AuthorityDenied, "/shell/origin"))?;
    authorise(window.label(), url.as_str())
}

#[tauri::command]
async fn planner_snapshot<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<Store>>,
) -> Result<Snapshot, DomainError> {
    check_window(&window)?;
    let store = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || store.snapshot())
        .await
        .map_err(|_| DomainError::new(ErrorCode::StorageFailure, "/shell/task"))?
}

#[tauri::command]
async fn planner_dispatch<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<Store>>,
    command_json: String,
) -> Result<ShellCommandReceipt, DomainError> {
    check_window(&window)?;
    let store = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        #[cfg(all(test, feature = "native-tests"))]
        tests::before_dispatch(&store, &command_json);
        dispatch_json(&store, &command_json)
    })
    .await
    .map_err(|_| DomainError::new(ErrorCode::StorageFailure, "/shell/task"))?
}

#[tauri::command]
async fn planner_replay_receipt<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<Store>>,
    original_command_json: String,
) -> Result<ShellCommandReceipt, DomainError> {
    check_window(&window)?;
    let store = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || replay_json(&store, &original_command_json))
        .await
        .map_err(|_| DomainError::new(ErrorCode::StorageFailure, "/shell/task"))?
}

#[tauri::command]
fn planner_features<R: Runtime>(window: WebviewWindow<R>) -> Result<ShellFeatures, DomainError> {
    check_window(&window)?;
    Ok(ShellFeatures::synthetic())
}

pub(crate) fn builder<R: Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder.invoke_handler(tauri::generate_handler![
        planner_snapshot,
        planner_dispatch,
        planner_features,
        planner_replay_receipt,
        planner_development_board
    ])
}

#[tauri::command]
async fn planner_development_board<R: Runtime>(
    window: WebviewWindow<R>,
    state: State<'_, Arc<Store>>,
    week_id: Option<mstrchf_contracts::WeekId>,
    slot: Option<mstrchf_contracts::SlotId>,
    meal_overrides:Option<mstrchf_contracts::MealOverrides>,
    extra_recipients:Option<Vec<mstrchf_contracts::PersonRef>>,
    make_extra:Option<bool>,
    leftover_source:Option<mstrchf_contracts::SlotId>,
    eat_by_time:Option<mstrchf_contracts::LocalTime>,
    time_resolution:Option<mstrchf_contracts::Resolution>,
) -> Result<mstrchf_store::development::DevelopmentBoard, DomainError> {
    check_window(&window)?;
    let store = Arc::clone(state.inner());
    tauri::async_runtime::spawn_blocking(move || {
        store.planner_board(week_id,slot,mstrchf_store::planner::ChoiceOptions{overrides:meal_overrides.unwrap_or_else(||mstrchf_store::planner::ChoiceOptions::default().overrides),extra_recipients:extra_recipients.unwrap_or_default(),make_extra:make_extra.unwrap_or(false),leftover_source,eat_by_time,time_resolution:time_resolution.unwrap_or(mstrchf_contracts::Resolution::Unambiguous)})
    })
    .await
    .map_err(|_| DomainError::new(ErrorCode::StorageFailure, "/shell/task"))?
}

pub(crate) fn run() {
    builder(tauri::Builder::default())
        .setup(|app| {
            // No env override, renderer path, or normal household root is reachable here.
            let root = development_root(&app.path().local_data_dir()?);
            let store = Store::open_development(StoreConfig {
                app_data_root: root,
            })?;
            mstrchf_store::development::seed_runtime(&store)?;
            app.manage(Arc::new(store));
            Ok(())
        })
        .run(tauri::generate_context!("development/tauri.conf.json"))
        .expect("run synthetic development shell");
}

#[cfg(all(test, feature = "native-tests"))]
#[path = "../tests/native.rs"]
mod tests;
