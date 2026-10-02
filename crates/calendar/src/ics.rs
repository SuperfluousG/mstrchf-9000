//! Fixed snapshot profile. Returns bytes only; no export, clock or account effect.
use crate::{
    model::*,
    projection::{bound, error, require, shape},
};
use mstrchf_contracts::*;
use mstrchf_core::food::continuation_report_digest;
use std::collections::BTreeSet;

struct Writer {
    bytes: Vec<u8>,
    column: usize,
}
impl Writer {
    fn append(&mut self, bytes: &[u8]) -> Result<(), CalendarFailure> {
        let size = self
            .bytes
            .len()
            .checked_add(bytes.len())
            .ok_or_else(|| error(ErrorCode::ArithmeticOverflow, "/ics/bytes"))?;
        bound(size, MAX_ARTEFACT_BYTES, "/ics/bytes")?;
        self.bytes.extend_from_slice(bytes);
        Ok(())
    }
    fn scalar(&mut self, ch: char) -> Result<(), CalendarFailure> {
        let mut buf = [0; 4];
        let bytes = ch.encode_utf8(&mut buf).as_bytes();
        if self.column + bytes.len() > 75 {
            self.append(b"\r\n ")?;
            self.column = 1;
        }
        self.append(bytes)?;
        self.column += bytes.len();
        Ok(())
    }
    fn line(&mut self, prefix: &str, value: &str, text: bool) -> Result<(), CalendarFailure> {
        self.column = 0;
        for ch in prefix.chars() {
            self.scalar(ch)?;
        }
        for ch in value.chars() {
            if text {
                match ch {
                    '\\' | ',' | ';' => {
                        self.scalar('\\')?;
                        self.scalar(ch)?;
                    }
                    '\n' => {
                        self.scalar('\\')?;
                        self.scalar('n')?;
                    }
                    _ => self.scalar(ch)?,
                }
            } else {
                self.scalar(ch)?;
            }
        }
        self.append(b"\r\n")
    }
}
fn utc(at: &Instant) -> String {
    at.0.chars().filter(|c| !matches!(c, '-' | ':')).collect()
}

/// Encode a validated present snapshot. This cannot establish missing history or source truth.
pub fn render_ics(snapshot: &CalendarSnapshot) -> Result<Vec<u8>, CalendarFailure> {
    bound(snapshot.events.len(), 256, "/events")?;
    bound(snapshot.reports.len(), 65, "/reports")?;
    let mut count = 0;
    for row in &snapshot.events {
        bound(row.contributors.len(), 4352, "/event/contributors")?;
        count += row.contributors.len();
        bound(count, 4352, "/events/total_contributors")?;
    }
    shape(&snapshot.subject_order_id)?;
    shape(&snapshot.evaluated_at)?;
    for report in &snapshot.reports {
        require(
            continuation_report_digest(report)? == report.digest,
            ErrorCode::DigestMismatch,
            "/reports/digest",
        )?;
    }
    if snapshot.reports.iter().any(|r| !r.blocking.is_empty()) {
        return Err(CalendarFailure::Blocked(snapshot.reports.clone()));
    }
    let mut keys = BTreeSet::new();
    let mut ids = BTreeSet::new();
    let mut members = BTreeSet::new();
    let mut sorted = Vec::new();
    // Validate all present records before emitting even the calendar header.
    for row in &snapshot.events {
        let e = &row.event;
        shape(e)?;
        shape(&row.revised_at)?;
        bound(row.sequence as usize, i32::MAX as usize, "/event/sequence")?;
        let key = canonical_bytes(&e.logical_key)?;
        require(
            keys.insert(key.clone()),
            ErrorCode::InvalidValue,
            "/events/duplicate_key",
        )?;
        require(
            ids.insert(e.event_id.0.clone()),
            ErrorCode::InvalidValue,
            "/events/duplicate_id",
        )?;
        if e.logical_key.kind == CalendarEventKind::Delivery {
            require(
                e.start.instant < e.end.instant,
                ErrorCode::InvalidValue,
                "/delivery/end",
            )?;
            require(
                row.contributors.is_empty(),
                ErrorCode::InvalidValue,
                "/contributors/delivery",
            )?;
        } else {
            require(
                row.contributors.iter().any(|c| {
                    c.owner.week.week_id == e.logical_key.week_id
                        && Some(&c.session_id) == e.logical_key.session_id.0.as_ref()
                }),
                ErrorCode::MissingReference,
                "/contributors/root",
            )?;
        }
        for c in &row.contributors {
            shape(&c.owner.week)?;
            shape(&c.owner.derived_digest)?;
            shape(&c.owner.basis_digest)?;
            shape(&c.session_id)?;
            require(
                members.insert((c.owner.week.week_id.0.clone(), c.session_id.0.clone())),
                ErrorCode::InvalidValue,
                "/contributors/duplicate_identity",
            )?;
        }
        sorted.push((key, row));
    }
    sorted.sort_by(|a, b| {
        a.1.event
            .start
            .instant
            .cmp(&b.1.event.start.instant)
            .then(a.0.cmp(&b.0))
    });
    let mut writer = Writer {
        bytes: Vec::new(),
        column: 0,
    };
    writer.line("BEGIN:", "VCALENDAR", false)?;
    writer.line("VERSION:", "2.0", false)?;
    writer.line(
        "PRODID:",
        "-//Bayes Ltd//MSTRCHF 9000 Calendar 1//EN",
        false,
    )?;
    writer.line("CALSCALE:", "GREGORIAN", false)?;
    for (_, row) in sorted {
        let e = &row.event;
        writer.line("BEGIN:", "VEVENT", false)?;
        writer.line("UID:", &format!("mstrchf-9000-{}", e.event_id.0), false)?;
        writer.line("DTSTAMP:", &utc(&row.revised_at), false)?;
        writer.line("SEQUENCE:", &row.sequence.to_string(), false)?;
        writer.line("DTSTART:", &utc(&e.start.instant), false)?;
        if e.start.instant != e.end.instant {
            writer.line("DTEND:", &utc(&e.end.instant), false)?;
        }
        writer.line("SUMMARY:", &e.title.0, true)?;
        writer.line("DESCRIPTION:", &e.body.0, true)?;
        writer.line("END:", "VEVENT", false)?;
    }
    writer.line("END:", "VCALENDAR", false)?;
    Ok(writer.bytes)
}
