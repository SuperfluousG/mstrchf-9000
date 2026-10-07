use crate::{
    UiCommandOutcome,
    compat::{self, Profile},
    records::storage,
};
use mstrchf_contracts::*;
use rusqlite::{Connection, OptionalExtension, params};
type ReceiptRow = (String, Vec<u8>, Vec<u8>, String, String);
pub(crate) fn lookup(
    c: &Connection,
    command: &AppCommandV1,
    bytes: &[u8],
) -> Result<Option<UiCommandOutcome>, DomainError> {
    lookup_id(c, &command.command_id.0, bytes)
}
pub(crate) fn lookup_id(
    c: &Connection,
    id: &str,
    bytes: &[u8],
) -> Result<Option<UiCommandOutcome>, DomainError> {
    #[cfg(all(debug_assertions, feature = "synthetic-development"))]
    if crate::current::active(c)? {
        return crate::current::lookup(c, id, bytes);
    }
    #[cfg(test)]
    let fixture = fixture_catalogue(c)?;
    #[cfg(not(test))]
    let fixture = false;
    if !fixture {
        crate::audit::metadata(c)?;
    }
    let row:Option<ReceiptRow>=c.query_row("SELECT command_digest,command,response,response_digest,snapshot_revision FROM command_receipts WHERE command_id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional().map_err(storage)?;
    let Some((cd, a, b, rd, revision)) = row else {
        return Ok(None);
    };
    let tag: String = if fixture {
        "B1Draft005".into()
    } else {
        c.query_row(
            "SELECT profile FROM receipt_formats WHERE command_id=?1",
            [id],
            |r| r.get(0),
        )
        .map_err(storage)?
    };
    let (hh, max): (String, String) = c
        .query_row(
            "SELECT household_id,snapshot_revision FROM household JOIN app_meta USING(singleton)",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(storage)?;
    let outcome = compat::receipt(
        Profile::bound(&tag)?,
        id,
        &hh,
        &revision,
        &max,
        &cd,
        &rd,
        &a,
        &b,
    )?;
    if a != bytes {
        return Err(DomainError::new(ErrorCode::ReplayConflict, "/command_id"));
    }
    let original: serde_json::Value =
        serde_json::from_slice(outcome.original_command_bytes()).map_err(storage)?;
    let kind = match original["command"]["kind"].as_str() {
        Some("save_household_draft") => CommandKind::SaveHouseholdDraft,
        Some("set_privacy") => CommandKind::SetPrivacy,
        Some("cancel_job") => CommandKind::CancelJob,
        _ => return Err(storage("closed receipt kind")),
    };
    authorize_command(kind, CallerClass::LocalUi)?;
    Ok(Some(outcome))
}
pub(crate) fn insert(
    c: &Connection,
    command: &AppCommandV1,
    input: &[u8],
    snapshot: &Snapshot,
    response: &[u8],
) -> Result<(), DomainError> {
    #[cfg(all(debug_assertions, feature = "synthetic-development"))]
    if crate::current::active(c)? {
        return crate::current::insert(c, command, input, snapshot, response);
    }
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
    #[cfg(test)]
    let fixture = fixture_catalogue(c)?;
    #[cfg(not(test))]
    let fixture = false;
    if !fixture {
        c.execute(
            "INSERT INTO receipt_formats(command_id,profile) VALUES(?1,'B1Draft005')",
            [&command.command_id.0],
        )
        .map_err(storage)?;
    }
    lookup(c, command, input)?.ok_or_else(|| storage("missing new receipt"))?;
    Ok(())
}

/// Explicit private fixture catalogues cannot be selected in a production build.
#[cfg(test)]
fn fixture_catalogue(c: &Connection) -> Result<bool, DomainError> {
    let has = |name: &str| {
        c.query_row(
            "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE name=?1)",
            [name],
            |r| r.get::<_, bool>(0),
        )
        .map_err(storage)
    };
    if has("store_format")? {
        return Ok(false);
    }
    let catalogue = if has("fixture_probe")? {
        crate::startup::Catalogue::Fixture
    } else if has("fixture_assets")? {
        crate::startup::Catalogue::AssetFixture
    } else if has("fixture_parent")? {
        crate::startup::Catalogue::FkFixture
    } else {
        return Ok(false);
    };
    catalogue.audit(c)?;
    Ok(true)
}
