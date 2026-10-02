use crate::records::{storage, terminal};
use mstrchf_contracts::*;
use rusqlite::Connection;

pub(crate) fn load(c: &Connection) -> Result<Snapshot, DomainError> {
    let (rev,id,dr,draft,pr,privacy): (String,String,String,Vec<u8>,String,Vec<u8>) = c.query_row(
        "SELECT snapshot_revision,household_id,draft_revision,draft,privacy_revision,privacy FROM app_meta JOIN household USING(singleton)", [],
        |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?,r.get(5)?))).map_err(storage)?;
    let draft: HouseholdDraft = parse_document(&draft)?;
    let privacy: PrivacyPolicy = parse_document(&privacy)?;
    if draft.household_id.0 != id
        || draft.revision.0.to_string() != dr
        || privacy.revision.0.to_string() != pr
    {
        return Err(storage("projection"));
    }
    let (profile_revision, profile): (Option<String>, Option<Vec<u8>>) = c
        .query_row(
            "SELECT profile_revision,profile FROM household WHERE singleton=1",
            [],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .map_err(storage)?;
    let profile = match (profile_revision, profile) {
        (None, None) => None,
        (Some(rev), Some(bytes)) => {
            let p: HouseholdProfile = parse_document(&bytes)?;
            if p.household_id.0 != id || p.revision.0.to_string() != rev {
                return Err(storage("profile projection"));
            }
            Some(p)
        }
        _ => return Err(storage("profile projection")),
    };
    let live: i64 = c
        .query_row("SELECT count(*) FROM jobs WHERE terminal=0", [], |r| {
            r.get(0)
        })
        .map_err(storage)?;
    if live > 1024 {
        return Err(DomainError::new(ErrorCode::LimitExceeded, "/jobs"));
    }
    let mut stmt=c.prepare("SELECT job_id,terminal,summary FROM jobs ORDER BY terminal ASC,creation_sequence DESC,job_id ASC LIMIT 1024").map_err(storage)?;
    let rows = stmt
        .query_map([], |r| {
            Ok((
                r.get::<_, String>(0)?,
                r.get::<_, bool>(1)?,
                r.get::<_, Vec<u8>>(2)?,
            ))
        })
        .map_err(storage)?;
    let mut jobs = Vec::new();
    for row in rows {
        let (id, t, body) = row.map_err(storage)?;
        let job: JobSummary = parse_document(&body)?;
        if job.job_id.0 != id || terminal(job.state) != t {
            return Err(storage("job projection"));
        }
        jobs.push(job);
    }
    let snapshot = Snapshot {
        schema_version: SchemaVersion,
        snapshot_revision: parse_document(&canonical_bytes(&rev)?)?,
        privacy,
        household_draft: draft,
        profile: Nullable(profile),
        weeks: BoundedVec(vec![]),
        jobs: BoundedVec(jobs),
        conflicts: BoundedVec(vec![]),
    };
    let bytes = canonical_bytes(&snapshot)?;
    parse_document(&bytes)
}

pub(crate) fn read(c: &mut Connection) -> Result<Snapshot, DomainError> {
    let tx = c.transaction().map_err(storage)?;
    let s = load(&tx)?;
    tx.commit().map_err(storage)?;
    Ok(s)
}
