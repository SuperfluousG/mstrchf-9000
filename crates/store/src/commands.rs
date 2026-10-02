use crate::{admission, receipts, records::storage, snapshot};
use mstrchf_contracts::*;
use rusqlite::{Connection, TransactionBehavior};

pub(crate) fn execute_outcome(
    c: &mut Connection,
    command: &AppCommandV1,
    bytes: &[u8],
) -> Result<crate::UiCommandOutcome, DomainError> {
    let tx = c
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(storage)?;
    if let Some(response) = receipts::lookup(&tx, command, bytes)? {
        #[cfg(test)]
        if crate::tests::fault("replay_return_error") {
            return Err(storage("injected replay return error"));
        }
        return Ok(response);
    }
    let state = snapshot::load(&tx)?;
    if command.expected_snapshot_revision != state.snapshot_revision {
        return Err(DomainError::new(
            ErrorCode::ContextStale,
            "/expected_snapshot_revision",
        ));
    }
    let mutation = admission::prepare(&tx, &state, &command.command)?;
    let revision = state.snapshot_revision.checked_increment()?;
    #[cfg(test)]
    crate::tests::barrier("command_before_write");
    admission::commit_mutation(&tx, &state, mutation)?;
    tx.execute(
        "UPDATE app_meta SET snapshot_revision=?1 WHERE singleton=1",
        [revision.0.to_string()],
    )
    .map_err(storage)?;
    #[cfg(test)]
    crate::tests::barrier("command_after_write");
    let result = snapshot::load(&tx)?;
    let response = canonical_bytes(&result)?;
    #[cfg(test)]
    let response = if crate::tests::fault("oversize_response") {
        vec![b' '; MAX_DOCUMENT_BYTES + 1]
    } else {
        response
    };
    let _: Snapshot = parse_document(&response)?;
    receipts::insert(&tx, command, bytes, &result, &response)?;
    #[cfg(test)]
    if crate::tests::fault("receipt_conflict") {
        receipts::insert(&tx, command, bytes, &result, &response)?;
    }
    #[cfg(test)]
    crate::tests::barrier("command_before_commit");
    #[cfg(test)]
    if crate::tests::fault("before_commit_error") {
        return Err(storage("injected precommit error"));
    }
    let outcome = receipts::lookup(&tx, command, bytes)?
        .ok_or_else(|| storage("new receipt unavailable"))?
        .applied();
    tx.commit().map_err(storage)?;
    #[cfg(test)]
    if crate::tests::fault("after_commit_error") {
        return Err(storage("injected commit return error"));
    }
    #[cfg(test)]
    crate::tests::barrier("command_after_commit");
    Ok(outcome)
}

#[cfg(test)]
pub(crate) fn execute(
    c: &mut Connection,
    command: &AppCommandV1,
    bytes: &[u8],
) -> Result<Vec<u8>, DomainError> {
    execute_outcome(c, command, bytes).map(|o| o.original_response_bytes().to_vec())
}
