use crate::records::storage;
use mstrchf_contracts::*;
use rusqlite::{Connection, OptionalExtension, params};
type ReceiptRow = (String, Vec<u8>, Vec<u8>, String, String);

pub(crate) fn lookup(
    c: &Connection,
    command: &AppCommandV1,
    bytes: &[u8],
) -> Result<Option<Vec<u8>>, DomainError> {
    let row: Option<ReceiptRow> = c.query_row(
        "SELECT command_digest,command,response,response_digest,snapshot_revision FROM command_receipts WHERE command_id=?1",
        [&command.command_id.0],
        |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)),
    ).optional().map_err(storage)?;
    let Some((digest, stored, response, response_digest, revision)) = row else {
        return Ok(None);
    };
    if bytes_digest(&stored).0 != digest {
        return Err(storage("command receipt corrupt"));
    }
    let original: AppCommandV1 = parse_document(&stored)?;
    if original.command_id != command.command_id {
        return Err(storage("receipt identity"));
    }
    if stored != bytes {
        return Err(DomainError::new(ErrorCode::ReplayConflict, "/command_id"));
    }
    let snapshot: Snapshot = parse_document(&response)?;
    if bytes_digest(&response).0 != response_digest
        || snapshot.snapshot_revision.0.to_string() != revision
        || canonical_bytes(&snapshot)? != response
    {
        return Err(storage("response receipt corrupt"));
    }
    Ok(Some(response))
}
pub(crate) fn insert(
    c: &Connection,
    command: &AppCommandV1,
    input: &[u8],
    snapshot: &Snapshot,
    response: &[u8],
) -> Result<(), DomainError> {
    c.execute(
        "INSERT INTO command_receipts VALUES(?1,?2,?3,?4,?5,?6)",
        params![
            command.command_id.0,
            bytes_digest(input).0,
            input,
            response,
            bytes_digest(response).0,
            snapshot.snapshot_revision.0.to_string()
        ],
    )
    .map_err(storage)?;
    Ok(())
}
