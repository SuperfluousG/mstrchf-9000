use crate::{migrate, records::*, windows_fs};
use mstrchf_contracts::*;
use rusqlite::{Connection, OpenFlags};
use std::{
    fs::{File, OpenOptions},
    io::{Read, Write},
    path::Path,
};

#[derive(serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Initialisation {
    lineage: String,
    stage: String,
    identity: String,
}

pub(crate) fn prepare(root: &Path) -> Result<(), DomainError> {
    let live = root.join("planner.sqlite");
    if live.exists() {
        return Ok(());
    }
    let marker = root.join("initialisation.json");
    let record = if marker.exists() {
        let mut bytes = Vec::new();
        File::open(&marker)
            .map_err(storage)?
            .take(4097)
            .read_to_end(&mut bytes)
            .map_err(storage)?;
        if bytes.len() > 4096 {
            return Err(storage("marker bound"));
        }
        let record: Initialisation = serde_json::from_slice(&bytes).map_err(storage)?;
        if record.lineage.len() != 32
            || !record.lineage.bytes().all(|b| b.is_ascii_hexdigit())
            || record.stage != format!("init-{}.sqlite", record.lineage)
        {
            return Err(storage("marker identity"));
        }
        record
    } else {
        let lineage = random_id()?;
        let stage = format!("init-{lineage}.sqlite");
        let file = OpenOptions::new()
            .create_new(true)
            .read(true)
            .write(true)
            .open(root.join(&stage))
            .map_err(storage)?;
        file.sync_all().map_err(storage)?;
        let record = Initialisation {
            lineage,
            stage,
            identity: windows_fs::identity(&file)?,
        };
        drop(file);
        #[cfg(test)]
        crate::tests::barrier("init_before_record");
        let mut output = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&marker)
            .map_err(storage)?;
        output
            .write_all(&serde_json::to_vec(&record).map_err(storage)?)
            .map_err(storage)?;
        output.sync_all().map_err(storage)?;
        record
    };
    #[cfg(test)]
    crate::tests::barrier("init_after_record");
    let stage = root.join(&record.stage);
    let file = File::open(&stage).map_err(storage)?;
    if windows_fs::identity(&file)? != record.identity {
        return Err(storage("stage replaced"));
    }
    let empty = file.metadata().map_err(storage)?.len() == 0;
    drop(file);
    let mut c =
        Connection::open_with_flags(&stage, OpenFlags::SQLITE_OPEN_READ_WRITE).map_err(storage)?;
    c.pragma_update(None, "foreign_keys", true)
        .map_err(storage)?;
    // Only a recorded app-created empty stage may receive schema001.
    if empty {
        migrate::initialise(&mut c, &record.lineage)?;
    } else {
        // A killed initial transaction may leave a valid empty SQLite header.
        let n: i64 = c
            .query_row("SELECT count(*) FROM sqlite_schema", [], |r| r.get(0))
            .map_err(storage)?;
        if n == 0 {
            migrate::initialise(&mut c, &record.lineage)?;
        }
    }
    migrate::verify(&c)?;
    let lineage: String = c
        .query_row("SELECT lineage FROM app_meta", [], |r| r.get(0))
        .map_err(storage)?;
    if lineage != record.lineage {
        return Err(storage("stage lineage"));
    }
    #[cfg(test)]
    crate::tests::barrier("init_after_commit");
    c.close().map_err(|(_, e)| storage(e))?;
    OpenOptions::new()
        .write(true)
        .open(&stage)
        .map_err(storage)?
        .sync_all()
        .map_err(storage)?;
    windows_fs::publish(&stage, &live)?;
    #[cfg(test)]
    crate::tests::barrier("init_after_publish");
    Ok(())
}
