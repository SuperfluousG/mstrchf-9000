//! Pure scoped upserts. Retained custody and truthful scope selection are store duties.
use crate::{
    model::*,
    projection::{bound, error, require, shape},
};
use mstrchf_contracts::*;
use mstrchf_core::food::{continuation_report_digest, owner_cmp};
use std::collections::{BTreeMap, BTreeSet};

type StableMember = (String, String);
type Membership = BTreeMap<StableMember, CalendarEventLogicalKey>;
fn stable(c: &SessionContribution) -> StableMember {
    (c.owner.week.week_id.0.clone(), c.session_id.0.clone())
}
fn encoded(key: &CalendarEventLogicalKey) -> Result<Vec<u8>, CalendarFailure> {
    Ok(canonical_bytes(key)?)
}
fn reconcile(path: &str, mut keys: Vec<CalendarEventLogicalKey>) -> CalendarFailure {
    keys.sort_by_key(|k| {
        (
            k.week_id.0.clone(),
            format!("{:?}", k.kind),
            k.session_id.0.clone(),
            k.order_id.0.clone(),
        )
    });
    keys.dedup();
    CalendarFailure::ReconciliationRequired {
        reason: DomainError::new(ErrorCode::ReconciliationRequired, path),
        keys,
    }
}
fn contributors(
    key: &CalendarEventLogicalKey,
    members: &[SessionContribution],
    map: &mut Membership,
) -> Result<(), CalendarFailure> {
    if key.kind == CalendarEventKind::Delivery {
        return require(
            members.is_empty(),
            ErrorCode::InvalidValue,
            "/contributors/delivery",
        );
    }
    require(
        members.iter().any(|c| {
            c.owner.week.week_id == key.week_id && Some(&c.session_id) == key.session_id.0.as_ref()
        }),
        ErrorCode::MissingReference,
        "/contributors/root",
    )?;
    for c in members {
        shape(&c.owner.week)?;
        shape(&c.owner.derived_digest)?;
        shape(&c.owner.basis_digest)?;
        shape(&c.session_id)?;
        require(
            map.insert(stable(c), key.clone()).is_none(),
            ErrorCode::InvalidValue,
            "/contributors/duplicate_identity",
        )?;
    }
    Ok(())
}
fn event_digest(e: &CalendarEvent) -> Result<PayloadDigest, CalendarFailure> {
    let mut value =
        serde_json::to_value(e).map_err(|_| error(ErrorCode::InvalidValue, "/event"))?;
    value
        .as_object_mut()
        .ok_or_else(|| error(ErrorCode::InvalidValue, "/event"))?
        .remove("event_payload_digest");
    value["schema_version"] = 1.into();
    Ok(PayloadDigest(canonical_digest(&value)?.0))
}
fn semantic_members(members: &[SessionContribution]) -> BTreeSet<StableMember> {
    members.iter().map(stable).collect()
}

/// Return only explicitly scoped upserts; never replace an owner's whole ledger.
pub fn version_events(
    prepared: &PreparedCalendar,
    update: &CalendarUpdateProjection,
    fresh_ids: &[CalendarIdentity],
    revised_at: &Instant,
) -> Result<CalendarSnapshot, CalendarFailure> {
    bound(prepared.drafts.len(), 256, "/drafts")?;
    bound(prepared.reports.len(), 65, "/reports")?;
    bound(update.scope.len(), 256, "/scope")?;
    bound(update.membership.len(), 8704, "/membership")?;
    bound(fresh_ids.len(), 256, "/fresh_ids")?;
    let mut count = 0;
    for d in &prepared.drafts {
        bound(d.contributors.len(), 4352, "/draft/contributors")?;
        count += d.contributors.len();
        bound(count, 4352, "/drafts/total_contributors")?;
    }
    count = 0;
    for row in &update.scope {
        if let PriorEventState::Retained(old) = &row.prior {
            bound(old.contributors.len(), 4352, "/retained/contributors")?;
            count += old.contributors.len();
            bound(count, 4352, "/retained/total_contributors")?;
        }
    }
    shape(&prepared.subject_order_id)?;
    shape(&prepared.evaluated_at)?;
    shape(revised_at)?;
    for report in &prepared.reports {
        require(
            continuation_report_digest(report)? == report.digest,
            ErrorCode::DigestMismatch,
            "/reports/digest",
        )?;
    }
    if prepared.reports.iter().any(|r| !r.blocking.is_empty()) {
        return Err(CalendarFailure::Blocked(prepared.reports.clone()));
    }

    let mut drafts = BTreeMap::new();
    let mut after = Membership::new();
    for d in &prepared.drafts {
        shape(&d.logical_key)?;
        shape(&d.start)?;
        shape(&d.end)?;
        shape(&d.title)?;
        shape(&d.body)?;
        require(
            d.start.instant <= d.end.instant,
            ErrorCode::InvalidValue,
            "/draft/end",
        )?;
        require(
            drafts.insert(encoded(&d.logical_key)?, d).is_none(),
            ErrorCode::InvalidValue,
            "/drafts/duplicate_key",
        )?;
        contributors(&d.logical_key, &d.contributors, &mut after)?;
    }
    let mut scope = BTreeMap::new();
    let mut before = Membership::new();
    let mut retained_ids = BTreeSet::new();
    for row in &update.scope {
        shape(&row.key)?;
        require(
            scope.insert(encoded(&row.key)?, row).is_none(),
            ErrorCode::InvalidValue,
            "/scope/duplicate_key",
        )?;
        match &row.prior {
            PriorEventState::Unavailable => {
                return Err(reconcile("/scope/unavailable", vec![row.key.clone()]));
            }
            PriorEventState::NeverVersioned => require(
                row.disposition == UpdateDisposition::Upsert,
                ErrorCode::InvalidValue,
                "/scope/never_versioned_remove",
            )?,
            PriorEventState::Retained(old) => {
                shape(&old.event)?;
                shape(&old.revised_at)?;
                require(
                    old.sequence <= i32::MAX as u32,
                    ErrorCode::LimitExceeded,
                    "/retained/sequence",
                )?;
                require(
                    old.event.logical_key == row.key,
                    ErrorCode::IdentityMismatch,
                    "/scope/retained_key",
                )?;
                require(
                    retained_ids.insert(old.event.event_id.0.clone()),
                    ErrorCode::InvalidValue,
                    "/retained/duplicate_id",
                )?;
                contributors(&row.key, &old.contributors, &mut before)?;
            }
        }
    }
    for (key, d) in &drafts {
        if !scope.contains_key(key) {
            return Err(reconcile(
                "/scope/missing_draft",
                vec![d.logical_key.clone()],
            ));
        }
    }
    for (key, row) in &scope {
        require(
            match row.disposition {
                UpdateDisposition::Upsert => drafts.contains_key(key),
                UpdateDisposition::Remove => !drafts.contains_key(key),
            },
            ErrorCode::InvalidValue,
            "/scope/draft_disposition",
        )?;
    }
    let mut transitions = BTreeMap::new();
    for t in &update.membership {
        shape(&t.member.week_id)?;
        shape(&t.member.session_id)?;
        require(
            t.before.is_some() || t.after.is_some(),
            ErrorCode::InvalidValue,
            "/membership/empty_transition",
        )?;
        for root in [&t.before, &t.after].into_iter().flatten() {
            shape(root)?;
            require(
                root.kind != CalendarEventKind::Delivery,
                ErrorCode::InvalidValue,
                "/membership/delivery_root",
            )?;
            if !scope.contains_key(&encoded(root)?) {
                return Err(reconcile("/membership/missing_root", vec![root.clone()]));
            }
        }
        let member = (t.member.week_id.0.clone(), t.member.session_id.0.clone());
        require(
            transitions.insert(member.clone(), t).is_none(),
            ErrorCode::InvalidValue,
            "/membership/duplicate_member",
        )?;
        require(
            t.before.as_ref() == before.get(&member) && t.after.as_ref() == after.get(&member),
            ErrorCode::IdentityMismatch,
            "/membership/roots",
        )?;
    }
    let union = before.keys().chain(after.keys()).collect::<BTreeSet<_>>();
    require(
        union.len() == transitions.len() && union.iter().all(|m| transitions.contains_key(*m)),
        ErrorCode::MissingReference,
        "/membership/closure",
    )?;
    let mut affected = Vec::new();
    for t in transitions.values() {
        if t.before.is_some() && t.before != t.after {
            affected.extend(t.before.iter().chain(t.after.iter()).cloned());
        }
    }
    affected.extend(
        update
            .scope
            .iter()
            .filter(|r| r.disposition == UpdateDisposition::Remove)
            .map(|r| r.key.clone()),
    );
    if !affected.is_empty() {
        return Err(reconcile("/scope/removal_or_reparenting", affected));
    }

    let mut fresh = BTreeMap::new();
    let mut ids = retained_ids;
    for f in fresh_ids {
        shape(&f.key)?;
        shape(&f.event_id)?;
        let key = encoded(&f.key)?;
        let row = scope
            .get(&key)
            .ok_or_else(|| error(ErrorCode::InvalidValue, "/fresh_ids/extra"))?;
        require(
            matches!(row.prior, PriorEventState::NeverVersioned),
            ErrorCode::IdentityMismatch,
            "/fresh_ids/replacement",
        )?;
        require(
            fresh.insert(key, f).is_none(),
            ErrorCode::InvalidValue,
            "/fresh_ids/duplicate_key",
        )?;
        require(
            ids.insert(f.event_id.0.clone()),
            ErrorCode::InvalidValue,
            "/fresh_ids/duplicate_id",
        )?;
    }
    for (key, row) in &scope {
        if matches!(row.prior, PriorEventState::NeverVersioned) {
            require(
                fresh.contains_key(key),
                ErrorCode::MissingReference,
                "/fresh_ids/missing",
            )?;
        }
    }

    let mut events = Vec::new();
    for (key, d) in drafts {
        let row = scope[&key];
        let old = if let PriorEventState::Retained(old) = &row.prior {
            Some(old)
        } else {
            None
        };
        let unchanged = old.is_some_and(|o| {
            o.event.start == d.start
                && o.event.end == d.end
                && o.event.title == d.title
                && o.event.body == d.body
                && semantic_members(&o.contributors) == semantic_members(&d.contributors)
        });
        let mut members = d.contributors.clone();
        members.sort_by(|a, b| owner_cmp(&a.owner, &b.owner).then(a.session_id.cmp(&b.session_id)));
        let saved = if let Some(o) = old.filter(|_| unchanged) {
            SavedCalendarEvent {
                event: o.event.clone(),
                contributors: members,
                sequence: o.sequence,
                revised_at: o.revised_at.clone(),
            }
        } else {
            require(
                revised_at >= &prepared.evaluated_at
                    && old.is_none_or(|o| revised_at >= &o.revised_at),
                ErrorCode::InvalidValue,
                "/revised_at",
            )?;
            let (id, revision, sequence) =
                if let Some(o) = old {
                    let revision =
                        o.event.payload_revision.0.checked_add(1).ok_or_else(|| {
                            error(ErrorCode::ArithmeticOverflow, "/payload_revision")
                        })?;
                    let sequence = o
                        .sequence
                        .checked_add(1)
                        .filter(|n| *n <= i32::MAX as u32)
                        .ok_or_else(|| error(ErrorCode::LimitExceeded, "/sequence"))?;
                    (o.event.event_id.clone(), Revision(revision), sequence)
                } else {
                    (fresh[&key].event_id.clone(), Revision(1), 0)
                };
            let mut event = CalendarEvent {
                event_id: id,
                logical_key: d.logical_key.clone(),
                payload_revision: revision,
                start: d.start.clone(),
                end: d.end.clone(),
                title: d.title.clone(),
                body: d.body.clone(),
                event_payload_digest: PayloadDigest("0".repeat(64)),
            };
            event.event_payload_digest = event_digest(&event)?;
            shape(&event)?;
            SavedCalendarEvent {
                event,
                contributors: members,
                sequence,
                revised_at: revised_at.clone(),
            }
        };
        events.push((key, saved));
    }
    events.sort_by(|a, b| {
        a.1.event
            .start
            .instant
            .cmp(&b.1.event.start.instant)
            .then(a.0.cmp(&b.0))
    });
    Ok(CalendarSnapshot {
        subject_order_id: prepared.subject_order_id.clone(),
        evaluated_at: prepared.evaluated_at.clone(),
        reports: prepared.reports.clone(),
        events: events.into_iter().map(|(_, e)| e).collect(),
    })
}
