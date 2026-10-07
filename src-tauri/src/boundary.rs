//! Native store admission shared by the development wrapper and real-store tests.
use crate::receipt::{ShellCommandReceipt, ShellRequestEncoding, project};
use mstrchf_contracts::{AppCommandV1, DomainError, canonical_bytes, parse_document};
use mstrchf_store::Store;
use std::path::{Path, PathBuf};

pub(crate) fn development_root(local_data: &Path) -> PathBuf {
    local_data
        .join("MSTRCHF9000-Development")
        .join("loop001-m1")
}

pub(crate) fn dispatch_json(
    store: &Store,
    command_json: &str,
) -> Result<ShellCommandReceipt, DomainError> {
    dispatch_with(command_json, |command| store.dispatch_ui(command))
}

fn dispatch_with(
    command_json: &str,
    execute: impl FnOnce(AppCommandV1) -> Result<mstrchf_store::UiCommandOutcome, DomainError>,
) -> Result<ShellCommandReceipt, DomainError> {
    // The shared decoder performs byte, depth, duplicate-key, collection and shape admission.
    let command = parse_document::<AppCommandV1>(command_json.as_bytes())?;
    if !matches!(
        &command.command,
        mstrchf_contracts::AppCommand::SaveHouseholdDraft { .. }
            | mstrchf_contracts::AppCommand::SetPrivacy { .. }
            | mstrchf_contracts::AppCommand::CancelJob { .. }
            | mstrchf_contracts::AppCommand::UpdateProfile { .. }
            | mstrchf_contracts::AppCommand::SaveWeek { .. }
    ) {
        return Err(DomainError::new(
            mstrchf_contracts::ErrorCode::UnsupportedCapability,
            "/command",
        ));
    }
    let canonical = canonical_bytes(&command)?;
    let outcome = execute(command.clone())?;
    project(
        &outcome,
        command_json,
        ShellRequestEncoding::CurrentJson,
        &canonical,
        Some(&command),
    )
}

pub(crate) fn replay_json(
    store: &Store,
    original_command_json: &str,
) -> Result<ShellCommandReceipt, DomainError> {
    let outcome = store.replay_ui(original_command_json.as_bytes())?;
    project(
        &outcome,
        original_command_json,
        ShellRequestEncoding::CanonicalReceiptJson,
        original_command_json.as_bytes(),
        None,
    )
}

#[cfg(test)]
#[path = "../tests/boundary.rs"]
mod tests;
