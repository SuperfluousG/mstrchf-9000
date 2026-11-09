//! One debug-only current schema. Historical readers and imports retain their closed audit.
use crate::{UiCommandOutcome, audit, records::storage, snapshot};
use mstrchf_contracts::*;
use rusqlite::{Connection, OptionalExtension, params, types::ValueRef};

const SQL: &str = "CREATE TABLE m1_development(singleton INTEGER PRIMARY KEY CHECK(singleton=1), format INTEGER NOT NULL CHECK(format=1)) STRICT;
CREATE TABLE weeks(week_id TEXT PRIMARY KEY NOT NULL COLLATE BINARY, revision TEXT NOT NULL, body BLOB NOT NULL) STRICT;
CREATE TABLE local_command_receipts(command_id TEXT PRIMARY KEY NOT NULL COLLATE BINARY, command_digest TEXT NOT NULL, command BLOB NOT NULL, response BLOB NOT NULL, response_digest TEXT NOT NULL, snapshot_revision TEXT NOT NULL) STRICT;";

const SQL_M2: &str = "CREATE TABLE active_weeks(position INTEGER PRIMARY KEY CHECK(position>=0 AND position<4), week_id TEXT NOT NULL UNIQUE REFERENCES weeks(week_id)) STRICT;";

pub(crate) fn m2(c: &Connection) -> Result<bool, DomainError> {
    Ok(c.pragma_query_value::<i64,_>(None,"user_version",|r|r.get(0)).map_err(storage)? >= 1002)
}

pub(crate) fn active(c: &Connection) -> Result<bool, DomainError> {
    c.query_row(
        "SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE type='table' AND name='m1_development')",
        [],
        |r| r.get(0),
    )
    .map_err(storage)
}
fn empty_source(c: &Connection) -> Result<audit::AuditedRows, DomainError> {
    let rows = audit::audit_b1(c)?;
    let s = snapshot::load(c)?;
    let receipts: i64 = c
        .query_row("SELECT count(*) FROM command_receipts", [], |r| r.get(0))
        .map_err(storage)?;
    // The only admitted predecessor is the empty, host-owned checkpoint-1 root.
    if s != crate::records::initial(s.household_draft.household_id.clone()) || receipts != 0 {
        return Err(DomainError::new(
            ErrorCode::AuthorityDenied,
            "/development/nonempty_source",
        ));
    }
    Ok(rows)
}
pub(crate) fn initialise(c: &mut Connection) -> Result<(), DomainError> {
    if active(c)? {
        audit(c)?;
        if !m2(c)? {
            let tx=c.transaction().map_err(storage)?;
            let retained=all_weeks(&tx)?;
            tx.execute_batch(SQL_M2).map_err(storage)?;
            for (index,w) in retained.iter().enumerate() {
                tx.execute("INSERT INTO active_weeks VALUES(?1,?2)",params![index as i64,w.range.week_id.0]).map_err(storage)?;
            }
            tx.pragma_update(None,"user_version",1002).map_err(storage)?;
            audit(&tx)?;
            tx.commit().map_err(storage)?;
        }
        if c.pragma_query_value::<i64,_>(None,"user_version",|r|r.get(0)).map_err(storage)?==1002 {
            let tx=c.transaction().map_err(storage)?;
            crate::planner_state::initialise(&tx)?;
            tx.pragma_update(None,"user_version",1003).map_err(storage)?;
            audit(&tx)?;tx.commit().map_err(storage)?;
        }
        return Ok(());
    }
    let tx = c.transaction().map_err(storage)?;
    empty_source(&tx)?;
    tx.execute_batch(SQL).map_err(storage)?;
    tx.execute("INSERT INTO m1_development VALUES(1,1)", [])
        .map_err(storage)?;
    tx.pragma_update(None, "user_version", 1001)
        .map_err(storage)?;
    audit(&tx)?;
    tx.commit().map_err(storage)?;
    initialise(c)
}
pub(crate) fn weeks(c: &Connection) -> Result<Vec<Week>, DomainError> {
    let all=all_weeks(c)?;
    if !m2(c)? {return Ok(all);}
    let mut q=c.prepare("SELECT week_id FROM active_weeks ORDER BY position").map_err(storage)?;
    let ids=q.query_map([],|r|r.get::<_,String>(0)).map_err(storage)?.collect::<Result<Vec<_>,_>>().map_err(storage)?;
    ids.into_iter().map(|id|all.iter().find(|w|w.range.week_id.0==id).cloned().ok_or_else(||storage("missing active week"))).collect()
}
pub(crate) fn all_weeks(c: &Connection) -> Result<Vec<Week>, DomainError> {
    if !active(c)? {
        return Ok(vec![]);
    }
    let count: i64 = c
        .query_row("SELECT count(*) FROM weeks", [], |r| r.get(0))
        .map_err(storage)?;
    if count > if m2(c)? {4096} else {1} {
        return Err(DomainError::new(
            ErrorCode::LimitExceeded,
            "/development/weeks",
        ));
    }
    let mut q = c
        .prepare("SELECT week_id,revision,body FROM weeks ORDER BY json_extract(CAST(body AS TEXT),'$.range.monday'),week_id")
        .map_err(storage)?;
    let rows = q
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, Vec<u8>>(2)?,
            ))
        })
        .map_err(storage)?;
    rows.map(|row| {
        let (id, revision, body) = row.map_err(storage)?;
        let w: Week = parse_document(&body)?;
        if id != w.range.week_id.0 || revision != w.revision.0.to_string() {
            return Err(storage("week projection"));
        }
        Ok(w)
    })
    .collect()
}

pub(crate) fn conflicts(c:&Connection,s:&Snapshot)->Result<Vec<Conflict>,DomainError>{
    if !m2(c)? {return Ok(vec![]);}
    let Some(p)=&s.profile.0 else{return Ok(vec![]);};
    let mut result=Vec::new();
    for mut w in all_weeks(c)? {
        for conflict in crate::planner::retained_conflicts(p,&w)? {if result.len()<100&&!result.contains(&conflict){result.push(conflict);}}
        let active=mstrchf_core::availability::eligible_slots(&w)?;
        w.plan.selections.0.retain(|selection|active.contains(&selection.slot));
        if w.plan.selections.is_empty() && w.plan.batch_links.is_empty(){continue;}
        if let Err(error)=crate::development::validate_saved_plan(s,p,&w) {
            if result.len()==100 {break;}
            result.push(Conflict{code:error.code,week_ids:BoundedVec(vec![w.range.week_id.clone()]),slot_ids:BoundedVec(w.plan.selections.iter().map(|selection|selection.slot.clone()).collect()),recipe_refs:BoundedVec(w.plan.selections.iter().map(|selection|selection.recipe.clone()).collect()),evidence_ids:BoundedVec(vec![]),blocking:true});
        }
    }
    Ok(result)
}
pub(crate) fn audit(c: &Connection) -> Result<audit::AuditedRows, DomainError> {
    if !active(c)? {
        return empty_source(c);
    }
    let version: i64 = c
        .pragma_query_value(None, "user_version", |r| r.get(0))
        .map_err(storage)?;
    if version != 1001 && version != 1002 && version != 1003 {
        return Err(DomainError::new(
            ErrorCode::UnsupportedVersion,
            "/development/schema",
        ));
    }
    audit::database_size(
        c.pragma_query_value(None, "page_count", |r| r.get(0))
            .map_err(storage)?,
        c.pragma_query_value(None, "page_size", |r| r.get(0))
            .map_err(storage)?,
    )?;
    let expected = Connection::open_in_memory().map_err(storage)?;
    expected
        .execute_batch(crate::migrate::SQL)
        .map_err(storage)?;
    expected
        .execute_batch(crate::migrate::SQL_002)
        .map_err(storage)?;
    expected.execute_batch(SQL).map_err(storage)?;
    if version>=1002 {expected.execute_batch(SQL_M2).map_err(storage)?;}
    if version>=1003 {expected.execute_batch(crate::planner_state::SQL).map_err(storage)?;}
    let schema = audit::schema(c)?;
    if schema != audit::schema(&expected)? {
        return Err(storage("current schema differs"));
    }
    audit::integrity(c)?;
    audit::metadata(c)?;
    let marker: (i64, i64) = c
        .query_row("SELECT singleton,format FROM m1_development", [], |r| {
            Ok((r.get(0)?, r.get(1)?))
        })
        .map_err(storage)?;
    if marker != (1, 1) {
        return Err(storage("current format"));
    }
    let (counters, lineage) = audit::meta(c)?;
    let state = snapshot::load(c)?;
    if counters.settings != state.privacy.revision {
        return Err(storage("current settings projection"));
    }
    let mut statement = c
        .prepare("SELECT version,checksum,applied_utc FROM schema_migrations ORDER BY version")
        .map_err(storage)?;
    let rows = statement
        .query_map([], |r| {
            Ok((
                r.get::<_, u32>(0)?,
                r.get::<_, String>(1)?,
                r.get::<_, String>(2)?,
            ))
        })
        .map_err(storage)?;
    let mut ledger = Vec::new();
    let checksums = [
        bytes_digest(crate::migrate::SQL.as_bytes()),
        bytes_digest(crate::migrate::SQL_002.as_bytes()),
    ];
    for row in rows {
        let (version, checksum, applied_utc) = row.map_err(storage)?;
        let index = ledger.len();
        if index >= checksums.len() || version != index as u32 + 1 || checksum != checksums[index].0
        {
            return Err(storage("current base ledger"));
        }
        let _: Instant = Instant::try_from(applied_utc.as_str())?;
        ledger.push(audit::LedgerEntry {
            version,
            sql_sha256: checksums[index].clone(),
            applied_utc,
        });
    }
    if ledger.len() != checksums.len() {
        return Err(storage("current base ledger length"));
    }
    let mut contents = Vec::new();
    let mut counts = Vec::new();
    let mut tables=vec![
        "app_meta",
        "household",
        "schema_migrations",
        "store_format",
        "receipt_formats",
        "command_receipts",
        "jobs",
        "resource_quarantine",
        "m1_development",
        "weeks",
        "local_command_receipts",
    ];
    if version>=1002 {tables.push("active_weeks");}
    if version>=1003 {tables.extend(["recipe_library","planner_history","essentials","local_assets"]);}
    for table in tables {
        let sql = format!("SELECT * FROM {table} ORDER BY 1"); // Closed identifiers above.
        let mut q = c.prepare(&sql).map_err(storage)?;
        let columns = q.column_count();
        let mut rows = q.query([]).map_err(storage)?;
        let mut count = 0;
        while let Some(row) = rows.next().map_err(storage)? {
            count += 1;
            if count > 4096 {
                return Err(DomainError::new(
                    ErrorCode::LimitExceeded,
                    "/development/rows",
                ));
            }
            let mut values = Vec::new();
            for col in 0..columns {
                values.push(match row.get_ref(col).map_err(storage)? {
                    ValueRef::Null => serde_json::json!(["null"]),
                    ValueRef::Integer(n) => serde_json::json!(["integer", n.to_string()]),
                    ValueRef::Text(b) => {
                        if b.len() > MAX_DOCUMENT_BYTES {
                            return Err(storage("text bound"));
                        }
                        serde_json::json!(["text", std::str::from_utf8(b).map_err(storage)?])
                    }
                    ValueRef::Blob(b) => {
                        if b.len() > MAX_DOCUMENT_BYTES {
                            return Err(storage("blob bound"));
                        }
                        serde_json::json!(["blob", b.len(), bytes_digest(b)])
                    }
                    ValueRef::Real(_) => return Err(storage("unexpected float")),
                });
            }
            contents.push(serde_json::json!([table, values]));
            if table == "local_command_receipts" {
                let id: String = row.get(0).map_err(storage)?;
                let command: Vec<u8> = row.get(2).map_err(storage)?;
                lookup(c, &id, &command)?.ok_or_else(|| storage("receipt absent"))?;
            }
        }
        if matches!(
            table,
            "app_meta" | "household" | "store_format" | "m1_development"
        ) && count != 1
        {
            return Err(storage("singleton count"));
        }
        if matches!(
            table,
            "receipt_formats" | "command_receipts" | "jobs" | "resource_quarantine"
        ) && count != 0
        {
            return Err(storage("unsupported development rows"));
        }
        counts.push((table, count));
    }
    if let Some(profile) = &state.profile.0 {
        let mut expected = crate::development::seed_profile(&state)?;
        expected.revision = profile.revision;
        if version==1001 && *profile != expected {
            return Err(storage("fixed synthetic profile"));
        }
        let evaluated_at = crate::development::now()?;
        mstrchf_core::household::validate_profile(profile, &state.privacy, &evaluated_at)?;
        for w in &all_weeks(c)? {
            let fixed =
                mstrchf_core::weeks::new_week(profile, &state.privacy, &evaluated_at, &w.range)?;
            if version==1001 && (w.defaults_snapshot != fixed.defaults_snapshot
                || w.cells != fixed.cells
                || w.day_enabled != fixed.day_enabled
                || w.meal_type_enabled != fixed.meal_type_enabled)
            {
                return Err(storage("fixed synthetic week"));
            }
            if version==1001 {crate::development::validate_legacy_plan(&state, profile, w)?;}
            else {
                let edit=WeekEdit{week_id:w.range.week_id.clone(),defaults_snapshot:w.defaults_snapshot.clone(),day_enabled:w.day_enabled.clone(),meal_type_enabled:w.meal_type_enabled.clone(),cells:w.cells.clone(),plan:Nullable(None)};
                mstrchf_core::availability::prepare_week_edit(w,&edit,w.revision,profile,&evaluated_at)?;
                for selection in &w.plan.selections.0 {
                    if !crate::planner::catalogue()?.iter().any(|r|crate::development::recipe_reference(r).as_ref()==Ok(&selection.recipe)) {
                        return Err(storage("unavailable saved recipe revision"));
                    }
                }
            }
        }
    } else if !state.weeks.is_empty() {
        return Err(storage("week without profile"));
    }
    if version>=1003 {crate::planner_state::audit(c)?;}
    Ok(audit::AuditedRows {
        version: version as u32,
        reader: "DevelopmentCurrent",
        closure: if version>=1003 {"m2-engineering-library-and-personal-candidates"} else {"synthetic-m1-no-assets"},
        counters,
        lineage,
        ledger,
        schema_sha256: canonical_digest(&schema)?,
        contents_sha256: canonical_digest(&contents)?,
        snapshot_sha256: bytes_digest(&canonical_bytes(&state)?),
        asset_refs: vec![],
        row_counts: counts,
    })
}

pub(crate) fn lookup(
    c: &Connection,
    id: &str,
    request: &[u8],
) -> Result<Option<UiCommandOutcome>, DomainError> {
    let row:Option<(String,Vec<u8>,Vec<u8>,String,String)>=c.query_row("SELECT command_digest,command,response,response_digest,snapshot_revision FROM local_command_receipts WHERE command_id=?1",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?))).optional().map_err(storage)?;
    let Some((cd, a, b, rd, rev)) = row else {
        return Ok(None);
    };
    let command: AppCommandV1 = parse_document(&a)?;
    let state: Snapshot = parse_document(&b)?;
    let (hh, max): (String, String) = c
        .query_row(
            "SELECT household_id,snapshot_revision FROM household JOIN app_meta USING(singleton)",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(storage)?;
    let max: Revision = parse_document(&canonical_bytes(&max)?)?;
    if command.command_id.0 != id
        || canonical_bytes(&command)? != a
        || canonical_bytes(&state)? != b
        || bytes_digest(&a).0 != cd
        || bytes_digest(&b).0 != rd
        || state.snapshot_revision.0.to_string() != rev
        || state.household_draft.household_id.0 != hh
        || state.snapshot_revision > max
        || command.expected_snapshot_revision.checked_increment()? != state.snapshot_revision
    {
        return Err(storage("current receipt binding"));
    }
    authorize_command(command.command.kind(), CallerClass::LocalUi)?;
    if !matches!(
        command.command.kind(),
        CommandKind::UpdateProfile
            | CommandKind::SuggestMeals
            | CommandKind::SavePersonalRecipe
            | CommandKind::RecordFeedback
            | CommandKind::RestoreHiddenRecipe
            | CommandKind::ConfirmEssentials
            | CommandKind::OpenPlanningHorizon
            | CommandKind::SaveWeek
            | CommandKind::SetPrivacy
            | CommandKind::SaveHouseholdDraft
            | CommandKind::CancelJob
    ) {
        return Err(storage("current receipt kind"));
    }
    if a != request {
        return Err(DomainError::new(ErrorCode::ReplayConflict, "/command_id"));
    }
    Ok(Some(UiCommandOutcome::development(a, b, state)))
}
pub(crate) fn insert(
    c: &Connection,
    command: &AppCommandV1,
    input: &[u8],
    s: &Snapshot,
    response: &[u8],
) -> Result<(), DomainError> {
    c.execute(
        "INSERT INTO local_command_receipts VALUES(?1,?2,?3,?4,?5,?6)",
        params![
            command.command_id.0,
            bytes_digest(input).0,
            input,
            response,
            bytes_digest(response).0,
            s.snapshot_revision.0.to_string()
        ],
    )
    .map_err(storage)?;
    lookup(c, &command.command_id.0, input)?.ok_or_else(|| storage("current receipt missing"))?;
    Ok(())
}
