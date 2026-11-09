use crate::{model::*, projection::*};
use mstrchf_contracts::*;
use mstrchf_core::{food::*, prep::validate_schedule};

struct Owner<'a> {
    input: OwnerInput<'a>,
    pin: PlanOwnerPin,
    bindings: Vec<TaskBinding>,
    occurrences: Vec<Occurrence>,
    report: ContinuationReport,
}
struct Session {
    owner: usize,
    index: usize,
    root: usize,
}
fn reconciliation(path: &str) -> CalendarFailure {
    CalendarFailure::ReconciliationRequired {
        reason: DomainError::new(ErrorCode::ReconciliationRequired, path),
        keys: vec![],
    }
}
fn owner_pin(h: &HistoricalApproval) -> Result<PlanOwnerPin, CalendarFailure> {
    Ok(PlanOwnerPin {
        week: WeekRevision {
            week_id: h.derived.week_id.clone(),
            revision: h.derived.plan_revision,
        },
        derived_digest: h.derived.digest.clone(),
        basis_digest: saved_basis_digest(&h.basis)?,
    })
}
fn validate_pin(p: &PlanOwnerPin) -> Result<(), CalendarFailure> {
    shape(&p.week)?;
    shape(&p.derived_digest)?;
    shape(&p.basis_digest)
}
fn validate_member(m: &SessionContribution) -> Result<(), CalendarFailure> {
    validate_pin(&m.owner)?;
    shape(&m.session_id)
}
fn contribution(s: &Session, owners: &[Owner<'_>]) -> SessionContribution {
    SessionContribution {
        owner: owners[s.owner].pin.clone(),
        session_id: owners[s.owner].input.historical.derived.prep_sessions[s.index]
            .session_id
            .clone(),
    }
}
fn member_cmp(a: &SessionContribution, b: &SessionContribution) -> std::cmp::Ordering {
    owner_cmp(&a.owner, &b.owner).then(a.session_id.cmp(&b.session_id))
}
fn key(s: &Session, owners: &[Owner<'_>]) -> Result<CalendarEventLogicalKey, CalendarFailure> {
    let o = &owners[s.owner];
    let h = o.input.historical;
    let session = &h.derived.prep_sessions[s.index];
    let role = h
        .basis
        .identities
        .sessions
        .iter()
        .find(|(_, id)| *id == session.session_id)
        .ok_or_else(|| error(ErrorCode::MissingReference, "/session/identity"))?
        .0
        .role;
    Ok(CalendarEventLogicalKey {
        week_id: h.derived.week_id.clone(),
        kind: match role {
            SessionRole::Cooking => CalendarEventKind::Cooking,
            SessionRole::AdvancePrep => CalendarEventKind::Prep,
        },
        session_id: Nullable(Some(session.session_id.clone())),
        order_id: Nullable(None),
    })
}
fn food_error(e: FoodFailure) -> CalendarFailure {
    match e {
        FoodFailure::Invalid(e) => e.into(),
        FoodFailure::Blocked { .. } => error(ErrorCode::DomainConflict, "/historical/schedule"),
    }
}

/// Source-complete planned drafts. This checks consistency, never source truth or cooking completion.
pub fn prepare_events(input: &CalendarInputs<'_>) -> Result<PreparedCalendar, CalendarFailure> {
    // Counts precede history reconstruction and body expansion, including direct Rust wrappers.
    bound(input.linked_owners.len(), 64, "/linked_owners")?;
    bound(input.assemblies.len(), 256, "/assemblies")?;
    bound(input.instruction_text.len(), 20480, "/instruction_text")?;
    let inputs = std::iter::once(input.subject)
        .chain(input.linked_owners.iter().copied())
        .collect::<Vec<_>>();
    let mut ns = 0usize;
    let mut nt = 0usize;
    let mut nc = 0usize;
    for o in &inputs {
        ns = ns
            .checked_add(o.historical.derived.prep_sessions.len())
            .ok_or_else(|| error(ErrorCode::ArithmeticOverflow, "/sessions"))?;
        bound(ns, 4352, "/sessions")?;
        for s in o.historical.derived.prep_sessions.iter() {
            nt = nt
                .checked_add(s.tasks.len())
                .ok_or_else(|| error(ErrorCode::ArithmeticOverflow, "/tasks"))?;
            bound(nt, 20480, "/tasks")?;
        }
    }
    for a in input.assemblies {
        bound(a.contributors.len(), 4352, "/assembly/contributors")?;
        nc += a.contributors.len();
        bound(nc, 4352, "/assembly/total_contributors")?;
        bound(a.evidence.len(), 256, "/assembly/evidence")?;
    }
    for t in input.instruction_text {
        bound(t.steps.len(), 256, "/instruction/steps")?;
        bound(t.evidence.len(), 256, "/instruction/evidence")?;
    }
    shape(input.evaluated_at)?;
    let mut owners = Vec::new();
    for i in inputs {
        require(
            i.current.feasibility.evaluated_at == *input.evaluated_at,
            ErrorCode::IdentityMismatch,
            "/evaluated_at",
        )?;
        let checked = verify_historical(i.historical)?;
        let report = assess_continuation(&checked, i.current)?;
        let pin = owner_pin(i.historical)?;
        require(
            !owners
                .iter()
                .any(|o: &Owner<'_>| o.pin.week.week_id == pin.week.week_id),
            ErrorCode::IdentityMismatch,
            "/owners/duplicate_week",
        )?;
        owners.push(Owner {
            input: i,
            pin,
            bindings: vec![],
            occurrences: vec![],
            report,
        });
    }
    owners.sort_by(|a, b| owner_cmp(&a.pin, &b.pin));
    let reports = owners.iter().map(|o| o.report.clone()).collect::<Vec<_>>();
    if reports.iter().any(|r| !r.blocking.is_empty()) {
        return Err(CalendarFailure::Blocked(reports));
    }
    for o in &mut owners {
        let h = o.input.historical;
        let b = &h.basis;
        o.bindings = validate_schedule(
            &b.plan,
            &b.context,
            &b.evidence,
            &b.identities,
            &b.schedule,
            &h.derived.prep_sessions,
        )
        .map_err(food_error)?;
        o.occurrences = occurrences(h)?;
        for occurrence in &o.occurrences {
            require(
                o.bindings
                    .iter()
                    .any(|b| b.phase.occurrence == occurrence.key),
                ErrorCode::UnsupportedCapability,
                "/occurrence/no_executable_phase",
            )?;
        }
    }
    let mut sessions = Vec::new();
    for (owner, o) in owners.iter().enumerate() {
        for (index, _) in o.input.historical.derived.prep_sessions.iter().enumerate() {
            let root = sessions.len();
            sessions.push(Session { owner, index, root });
        }
    }
    // IDs are app-owned and cannot ambiguously identify sessions across owners.
    for (i, s) in sessions.iter().enumerate() {
        let id = &owners[s.owner].input.historical.derived.prep_sessions[s.index].session_id;
        require(
            !sessions[..i].iter().any(|x| {
                owners[x.owner].input.historical.derived.prep_sessions[x.index].session_id == *id
            }),
            ErrorCode::IdentityMismatch,
            "/sessions/duplicate_id",
        )?;
    }
    let mut parents = vec![None; sessions.len()];
    for (i, s) in sessions.iter().enumerate() {
        let h = owners[s.owner].input.historical;
        let session = &h.derived.prep_sessions[s.index];
        if let Some(target_id) = &session.attached_to.0 {
            let sk = &h
                .basis
                .identities
                .sessions
                .iter()
                .find(|(_, id)| *id == session.session_id)
                .ok_or_else(|| error(ErrorCode::MissingReference, "/attachment/session"))?
                .0;
            let a = h
                .basis
                .schedule
                .layout
                .attachments
                .iter()
                .find(|a| a.session == *sk)
                .ok_or_else(|| error(ErrorCode::MissingReference, "/attachment"))?;
            let target = owners
                .iter()
                .position(|o| o.pin == a.target_owner)
                .ok_or_else(|| {
                    reconciliation(&format!(
                        "/attachment/owner_custody/{}/{}",
                        a.target_owner.week.week_id.0, target_id.0
                    ))
                })?;
            let parent = sessions
                .iter()
                .position(|x| {
                    x.owner == target
                        && owners[target].input.historical.derived.prep_sessions[x.index].session_id
                            == *target_id
                })
                .ok_or_else(|| {
                    reconciliation(&format!(
                        "/attachment/session_custody/{}/{}",
                        a.target_owner.week.week_id.0, target_id.0
                    ))
                })?;
            require(parent != i, ErrorCode::InvalidValue, "/attachment/self")?;
            check_projection(&owners[target], &h.basis.schedule.occupied)?;
            check_projection(&owners[target], &owners[s.owner].input.current.occupied)?;
            parents[i] = Some(parent);
        }
    }
    for (i, s) in sessions.iter_mut().enumerate() {
        let mut root = i;
        let mut seen = Vec::new();
        while let Some(p) = parents[root] {
            require(
                !seen.contains(&p),
                ErrorCode::InvalidValue,
                "/attachment/cycle",
            )?;
            seen.push(p);
            root = p;
        }
        s.root = root;
    }
    let subject_week = &input.subject.historical.derived.week_id;
    let mut roots = sessions
        .iter()
        .filter(|s| owners[s.owner].pin.week.week_id == *subject_week)
        .map(|s| s.root)
        .collect::<Vec<_>>();
    roots.sort_unstable();
    roots.dedup();
    bound(roots.len() + 1, 256, "/drafts")?;
    require(
        input.assemblies.len() == roots.len(),
        ErrorCode::MissingReference,
        "/assemblies/closure",
    )?;
    let mut assembly_members = Vec::new();
    for &root in &roots {
        let root_member = contribution(&sessions[root], &owners);
        let rows = input
            .assemblies
            .iter()
            .filter(|a| a.root == root_member)
            .collect::<Vec<_>>();
        require(
            rows.len() == 1,
            if rows.is_empty() {
                ErrorCode::MissingReference
            } else {
                ErrorCode::InvalidValue
            },
            "/assemblies/root",
        )?;
        let a = rows[0];
        validate_member(&a.root)?;
        pins(&a.evidence, "/assembly/evidence")?;
        let mut expected = sessions
            .iter()
            .filter(|s| s.root == root)
            .map(|s| contribution(s, &owners))
            .collect::<Vec<_>>();
        expected.sort_by(member_cmp);
        let mut actual = a.contributors.clone();
        for m in &actual {
            validate_member(m)?;
        }
        actual.sort_by(member_cmp);
        require(
            actual.windows(2).all(|w| w[0] != w[1]),
            ErrorCode::InvalidValue,
            "/assembly/duplicate_member",
        )?;
        if actual != expected {
            return Err(CalendarFailure::ReconciliationRequired {
                reason: DomainError::new(
                    ErrorCode::ReconciliationRequired,
                    "/assembly/contributor_closure",
                ),
                keys: vec![key(&sessions[root], &owners)?],
            });
        }
        assembly_members.push(expected);
    }
    // Validate complete presentation rows only for occurrences contributing to these roots.
    let mut needed = Vec::new();
    for s in sessions.iter().filter(|s| roots.contains(&s.root)) {
        let o = &owners[s.owner];
        let id = &o.input.historical.derived.prep_sessions[s.index].session_id;
        for b in o.bindings.iter().filter(|b| b.task.session_id == *id) {
            if !needed.iter().any(|(owner, occ): &(usize, OccurrenceKey)| {
                *owner == s.owner && *occ == b.phase.occurrence
            }) {
                needed.push((s.owner, b.phase.occurrence.clone()));
            }
        }
    }
    require(
        input.instruction_text.len() == needed.len(),
        ErrorCode::MissingReference,
        "/instruction_text/closure",
    )?;
    for (owner, occ) in &needed {
        let o = &owners[*owner];
        let rows = input
            .instruction_text
            .iter()
            .filter(|t| t.owner == o.pin && t.occurrence == *occ)
            .collect::<Vec<_>>();
        require(
            rows.len() == 1,
            if rows.is_empty() {
                ErrorCode::MissingReference
            } else {
                ErrorCode::InvalidValue
            },
            "/instruction_text/occurrence",
        )?;
        let t = rows[0];
        validate_pin(&t.owner)?;
        shape(&t.occurrence.slot)?;
        shape(&t.occurrence.recipe)?;
        if let Some(id) = &t.occurrence.extra_id {
            shape(id)?;
        }
        pins(&t.evidence, "/instruction_text/evidence")?;
        let expected = o
            .occurrences
            .iter()
            .find(|a| a.key == *occ)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/occurrence"))?;
        require(
            t.method == expected.method && t.scaled == expected.scaled,
            ErrorCode::IdentityMismatch,
            "/instruction_text/scaled",
        )?;
        let r = recipe(o.input.historical, &occ.recipe)?;
        let mut phases = o
            .bindings
            .iter()
            .filter(|b| b.phase.occurrence == *occ)
            .map(|b| &b.phase)
            .collect::<Vec<_>>();
        phases.sort_by(|a, b| phase_cmp(a, b));
        let mut steps = Vec::new();
        for p in phases {
            for step in ordered_steps(r, p.prep_id)? {
                steps.push((p.prep_id, step));
            }
        }
        require(
            t.steps.iter().map(|s| (s.prep_id, s.step_id)).eq(steps),
            ErrorCode::MissingReference,
            "/instruction_text/selected_steps",
        )?;
        for step in &t.steps {
            shape(&step.text)?;
        }
        check_external_dependencies(o, expected)?;
    }
    let mut drafts = Vec::new();
    for (n, &root) in roots.iter().enumerate() {
        drafts.push(session_draft(
            root,
            &sessions,
            &owners,
            &assembly_members[n],
            input.instruction_text,
        )?);
    }
    let h = input.subject.historical;
    let c = &h.order.confirmation;
    let mut delivery = Body::new("delivery".into());
    delivery.line(format!(
        "Grocery delivery\n{} to {}",
        moment(&c.delivery_start),
        moment(&c.delivery_end)
    ))?;
    delivery.line("Confirmed delivery window. Check delivered packs against the planned ingredients and unresolved date evidence.".into())?;
    // All owner reports remain typed; the delivery body retains all subject warnings.
    let subject = owners
        .iter()
        .find(|o| o.pin.week.week_id == *subject_week)
        .ok_or_else(|| error(ErrorCode::MissingReference, "/subject"))?;
    warnings(&mut delivery, subject, None)?;
    drafts.push(EventDraft {
        logical_key: CalendarEventLogicalKey {
            week_id: h.derived.week_id.clone(),
            kind: CalendarEventKind::Delivery,
            session_id: Nullable(None),
            order_id: Nullable(Some(h.order.order_id.clone())),
        },
        contributors: vec![],
        start: c.delivery_start.clone(),
        end: c.delivery_end.clone(),
        title: Title("Grocery delivery".into()),
        body: delivery.finish()?,
    });
    let mut keyed = drafts
        .into_iter()
        .map(|d| {
            Ok((
                canonical_bytes(
                    &serde_json::to_value(&d.logical_key)
                        .map_err(|_| error(ErrorCode::InvalidValue, "/logical_key"))?,
                )?,
                d,
            ))
        })
        .collect::<Result<Vec<_>, CalendarFailure>>()?;
    keyed.sort_by(|a, b| {
        a.1.start
            .instant
            .cmp(&b.1.start.instant)
            .then(a.0.cmp(&b.0))
    });
    Ok(PreparedCalendar {
        subject_order_id: h.order.order_id.clone(),
        evaluated_at: input.evaluated_at.clone(),
        reports,
        drafts: keyed.into_iter().map(|(_, d)| d).collect(),
    })
}

fn check_projection(
    owner: &Owner<'_>,
    occupied: &OccupancySnapshot,
) -> Result<(), CalendarFailure> {
    let p = occupied
        .owners
        .iter()
        .find(|p| p.pin == owner.pin)
        .ok_or_else(|| {
            reconciliation(&format!(
                "/attachment/occupied_owner/{}",
                owner.pin.week.week_id.0
            ))
        })?;
    let mut resources = Vec::new();
    let h = owner.input.historical;
    for b in &owner.bindings {
        let r = h
            .basis
            .evidence
            .resources
            .iter()
            .find(|r| r.recipe == b.phase.occurrence.recipe && r.prep_id == b.phase.prep_id)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/projection/resources"))?;
        resources.push(OccupiedTaskResources {
            task: b.task.clone(),
            equipment: r.equipment.clone(),
            evidence_ids: r.evidence_ids.clone(),
        });
    }
    resources.sort_by(|a, b| task_cmp(&a.task, &b.task));
    require(
        p.sessions == h.derived.prep_sessions.0 && p.task_resources == resources,
        ErrorCode::IdentityMismatch,
        "/attachment/owner_projection",
    )
}
fn binding_task<'a>(
    o: &'a Owner<'_>,
    phase: &PhaseKey,
) -> Result<(&'a TaskBinding, &'a ScheduledPrepTask), CalendarFailure> {
    let b = o
        .bindings
        .iter()
        .find(|b| b.phase == *phase)
        .ok_or_else(|| error(ErrorCode::MissingReference, "/phase/binding"))?;
    let s = o
        .input
        .historical
        .derived
        .prep_sessions
        .iter()
        .find(|s| s.session_id == b.task.session_id)
        .ok_or_else(|| error(ErrorCode::MissingReference, "/phase/session"))?;
    Ok((
        b,
        s.tasks
            .iter()
            .find(|t| t.task_id == b.task.task_id)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/phase/task"))?,
    ))
}
/// Core validates source edges; additionally bind every out-of-method instruction dependency
/// to the exact selected component production/cooling phases, not a coincident step in another recipe.
fn check_external_dependencies(o: &Owner<'_>, occ: &Occurrence) -> Result<(), CalendarFailure> {
    let r = recipe(o.input.historical, &occ.key.recipe)?;
    let method = o
        .input
        .historical
        .basis
        .evidence
        .methods
        .iter()
        .find(|m| m.recipe == occ.key.recipe && m.kind == occ.method)
        .ok_or_else(|| error(ErrorCode::MissingReference, "/method"))?;
    let selected = r
        .prep
        .iter()
        .filter(|p| method.prep_ids.contains(&p.prep_id))
        .flat_map(|p| p.step_ids.iter().copied())
        .collect::<Vec<_>>();
    for step in r.steps.iter().filter(|s| selected.contains(&s.step_id)) {
        for dep in step.depends_on.iter().filter(|d| !selected.contains(d)) {
            require(
                occ.method == MethodKind::Reuse,
                ErrorCode::MissingReference,
                "/steps/dependency",
            )?;
            let sources = source_components(o, occ)?;
            let current_prep = r
                .prep
                .iter()
                .find(|p| {
                    method.prep_ids.contains(&p.prep_id) && p.step_ids.contains(&step.step_id)
                })
                .ok_or_else(|| error(ErrorCode::MissingReference, "/steps/phase"))?;
            let (_, current_task) = binding_task(
                o,
                &PhaseKey {
                    occurrence: occ.key.clone(),
                    prep_id: current_prep.prep_id,
                },
            )?;
            let mut matched = false;
            for (_, source, phase) in &sources {
                let sr = recipe(o.input.historical, &source.recipe)?;
                let p = sr
                    .prep
                    .iter()
                    .find(|p| p.prep_id == phase.prep_id)
                    .ok_or_else(|| error(ErrorCode::MissingReference, "/source/phase"))?;
                let (bound, _) = binding_task(o, phase)?;
                if source.recipe == occ.key.recipe
                    && p.step_ids.contains(dep)
                    && current_task.depends_on.contains(&bound.task)
                {
                    matched = true;
                }
            }
            require(
                matched,
                ErrorCode::MissingReference,
                "/steps/exact_source_prerequisite",
            )?;
        }
    }
    Ok(())
}
fn source_components<'a>(
    o: &'a Owner<'_>,
    occ: &Occurrence,
) -> Result<Vec<(&'a DerivedComponent, OccurrenceKey, PhaseKey)>, CalendarFailure> {
    if occ.key.extra_id.is_some() {
        return Ok(vec![]);
    }
    let h = o.input.historical;
    let selection = h
        .basis
        .plan
        .selections
        .iter()
        .find(|s| s.slot == occ.key.slot)
        .ok_or_else(|| error(ErrorCode::MissingReference, "/selection"))?;
    let BatchUse::Leftover {
        batch_id,
        component_ids,
        ..
    } = &selection.batch_use
    else {
        return Ok(vec![]);
    };
    let batch = h
        .derived
        .batches
        .iter()
        .find(|b| b.batch_id == *batch_id)
        .ok_or_else(|| error(ErrorCode::MissingReference, "/source/batch"))?;
    let source = OccurrenceKey {
        slot: batch.source_slot.clone(),
        extra_id: None,
        recipe: batch.recipe.clone(),
    };
    let r = recipe(h, &source.recipe)?;
    let mut out = Vec::new();
    for id in component_ids.iter() {
        let c = batch
            .components
            .iter()
            .find(|c| c.component_id == *id)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/source/component"))?;
        let spec = r
            .components
            .iter()
            .find(|c| c.component_id == *id)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/source/component_spec"))?;
        let prep_ids = match &spec.production.0 {
            Some(ComponentProductionSpec::Cooked { completion_prep_id }) => {
                vec![*completion_prep_id]
            }
            Some(ComponentProductionSpec::CookedWithCooling {
                completion_prep_id,
                cooling_prep_ids,
                ..
            }) => std::iter::once(*completion_prep_id)
                .chain(cooling_prep_ids.iter().copied())
                .collect(),
            Some(ComponentProductionSpec::Unheated {
                first_handling_prep_id,
            }) => vec![*first_handling_prep_id],
            None => {
                return Err(error(
                    ErrorCode::UnsupportedCapability,
                    "/source/production",
                ));
            }
        };
        for prep_id in prep_ids {
            let phase = PhaseKey {
                occurrence: source.clone(),
                prep_id,
            };
            binding_task(o, &phase)?;
            out.push((c, source.clone(), phase));
        }
    }
    Ok(out)
}

fn cooling_phase_limit(recipe: &RecipeRevision, prep_id: u16) -> Option<u32> {
    recipe
        .components
        .iter()
        .filter_map(|component| match &component.production.0 {
            Some(ComponentProductionSpec::CookedWithCooling {
                cooling_prep_ids,
                max_cooling_seconds,
                ..
            }) if cooling_prep_ids.contains(&prep_id) => Some(*max_cooling_seconds),
            _ => None,
        })
        .min()
}

fn cooling_duration(seconds: u32) -> String {
    if seconds % 60 == 0 {
        format!("{} minutes", seconds / 60)
    } else {
        format!("{seconds} seconds")
    }
}

// Only the newly permitted earlier endpoint changes prose. A zero-offset
// original retains its exact historical calendar wording and receipts.
fn earlier_cooking_endpoint(recipe: &RecipeRevision, cooling_id: Option<u16>) -> Option<u16> {
    recipe.components.iter().find_map(|component| {
        let Some(ComponentProductionSpec::CookedWithCooling { completion_prep_id, cooling_prep_ids, .. }) = &component.production.0 else {return None;};
        if cooling_id.is_some_and(|id| !cooling_prep_ids.contains(&id)) {return None;}
        let completion = recipe.prep.iter().find(|phase| phase.prep_id == *completion_prep_id)?;
        (i64::from(completion.earliest_offset_seconds) + i64::from(completion.elapsed_minutes)*60 < 0)
            .then_some(*completion_prep_id)
    })
}

struct Body {
    value: String,
    path: String,
}
impl Body {
    fn new(path: String) -> Self {
        Self {
            value: String::new(),
            path,
        }
    }
    fn line(&mut self, text: String) -> Result<(), CalendarFailure> {
        let bytes = self
            .value
            .len()
            .checked_add(text.len() + if self.value.is_empty() { 0 } else { 1 })
            .ok_or_else(|| error(ErrorCode::ArithmeticOverflow, "/body"))?;
        bound(bytes, 65536, &format!("/events/{}/body/bytes", self.path))?;
        if !self.value.is_empty() {
            self.value.push('\n');
        }
        self.value.push_str(&text);
        Ok(())
    }
    fn finish(self) -> Result<LongText, CalendarFailure> {
        let body = LongText(self.value);
        shape(&body)?;
        Ok(body)
    }
}
fn moment(m: &ZonedMoment) -> String {
    let fold = match m.resolution {
        Resolution::Unambiguous => "",
        Resolution::Earlier => " (earlier occurrence)",
        Resolution::Later => " (later occurrence)",
    };
    // Every moment here came through verify_historical/assess_continuation.
    // Derive the displayed offset from that validated local/UTC pair; no new
    // timezone resolution or environment timezone is involved.
    let local = chrono::NaiveDateTime::parse_from_str(
        &format!("{}T{}", m.local_date.0, m.local_time.0),
        "%Y-%m-%dT%H:%M:%S",
    )
    .expect("validated ZonedMoment local time");
    let offset = local.and_utc().timestamp() - m.instant.timestamp_seconds();
    let sign = if offset < 0 { '-' } else { '+' };
    let offset = offset.unsigned_abs();
    format!(
        "{} {} {}{} [UTC{}{:02}:{:02}; {}]",
        m.local_date.0,
        m.local_time.0,
        m.zone.0,
        fold,
        sign,
        offset / 3600,
        (offset % 3600) / 60,
        m.instant.0
    )
}
fn rational(r: Rational) -> String {
    if r.denominator.0 == 1 {
        r.numerator.0.to_string()
    } else {
        format!("{}/{}", r.numerator.0, r.denominator.0)
    }
}
fn quantity(q: &Quantity) -> String {
    let unit = match q.unit {
        Unit::G => "g",
        Unit::Ml => "ml",
        Unit::Each => "each",
    };
    format!("{} {} ({:?})", rational(q.amount.0), unit, q.form)
}
fn warnings(
    body: &mut Body,
    o: &Owner<'_>,
    occurrences: Option<&[OccurrenceKey]>,
) -> Result<(), CalendarFailure> {
    let mut any = false;
    let mut shown = Vec::new();
    for u in &o.report.feasibility.uses {
        let occurrence = match &u.use_key {
            UseKey::Preparation { phase } => &phase.occurrence,
            UseKey::Consumption { occurrence, .. } => occurrence,
        };
        if occurrences.is_some_and(|v| !v.contains(occurrence)) || u.issues.is_empty() {
            continue;
        }
        let group = (
            u.line_id.clone(),
            occurrence.clone(),
            u.assessment,
            u.issues.clone(),
        );
        if shown.contains(&group) {
            continue;
        }
        shown.push(group);
        let mut affected = Vec::new();
        for other in &o.report.feasibility.uses {
            let (other_occurrence, label) = match &other.use_key {
                UseKey::Preparation { phase } => (
                    &phase.occurrence,
                    format!("preparation phase {}", phase.prep_id),
                ),
                UseKey::Consumption { occurrence, .. } => (occurrence, "consumption".into()),
            };
            if other.line_id == u.line_id
                && other_occurrence == occurrence
                && other.assessment == u.assessment
                && other.issues == u.issues
                && !affected.contains(&label)
            {
                affected.push(label);
            }
        }
        if !any {
            body.line("Warnings — unresolved food evidence".into())?;
            any = true;
        }
        let line = o
            .input
            .historical
            .derived
            .demand
            .lines
            .iter()
            .find(|l| l.line_id == u.line_id)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/warning/line"))?;
        let name = o
            .input
            .historical
            .basis
            .context
            .recipes
            .iter()
            .flat_map(|r| r.ingredients.iter())
            .find(|i| i.ingredient_id == line.ingredient_id)
            .map(|i| i.name.0.as_str())
            .unwrap_or("ingredient");
        body.line(format!("{} {:?}: {} — {:?}; {:?} ({}). Check delivered pack and opening evidence; freshness through this use has not been established.",occurrence.slot.date.0,occurrence.slot.meal_type,name,u.assessment,u.issues,affected.join("; ")))?;
    }
    for w in &o.report.warnings {
        body.line(format!("Warning: {:?}; {:?}", w.kind, w.conflict.code))?;
    }
    Ok(())
}
fn session_draft(
    root: usize,
    sessions: &[Session],
    owners: &[Owner<'_>],
    members: &[SessionContribution],
    texts: &[OccurrenceText],
) -> Result<EventDraft, CalendarFailure> {
    let root_session = &sessions[root];
    let logical_key = key(root_session, owners)?;
    let contributors = sessions
        .iter()
        .filter(|s| s.root == root)
        .collect::<Vec<_>>();
    let first = &owners[root_session.owner]
        .input
        .historical
        .derived
        .prep_sessions[root_session.index];
    let mut start = first.start.clone();
    let mut end = first.end.clone();
    let mut phases = Vec::new();
    let mut selected = Vec::new();
    for s in &contributors {
        let o = &owners[s.owner];
        let session = &o.input.historical.derived.prep_sessions[s.index];
        if session.start.instant < start.instant {
            start = session.start.clone();
        }
        if session.end.instant > end.instant {
            end = session.end.clone();
        }
        for b in o
            .bindings
            .iter()
            .filter(|b| b.task.session_id == session.session_id)
        {
            let (_, t) = binding_task(o, &b.phase)?;
            phases.push((s.owner, b, t));
            if !selected.contains(&(s.owner, b.phase.occurrence.clone())) {
                selected.push((s.owner, b.phase.occurrence.clone()));
            }
        }
    }
    selected.sort_by(|a, b| {
        owner_cmp(&owners[a.0].pin, &owners[b.0].pin).then_with(|| occurrence_cmp(&a.1, &b.1))
    });
    // Task dependencies can break equal-start ties (including zero-duration phases).
    let mut ordered = Vec::new();
    while ordered.len() < phases.len() {
        let mut ready = phases
            .iter()
            .enumerate()
            .filter(|(idx, (_, _, t))| {
                !ordered.contains(idx)
                    && t.depends_on.iter().all(|d| {
                        phases
                            .iter()
                            .position(|(_, b, _)| b.task == *d)
                            .is_none_or(|i| ordered.contains(&i))
                    })
            })
            .collect::<Vec<_>>();
        ready.sort_by(|a, b| {
            a.1.2
                .elapsed
                .start
                .instant
                .cmp(&b.1.2.elapsed.start.instant)
                .then_with(|| owner_cmp(&owners[a.1.0].pin, &owners[b.1.0].pin))
                .then_with(|| phase_cmp(&a.1.1.phase, &b.1.1.phase))
        });
        ordered.push(
            ready
                .first()
                .ok_or_else(|| error(ErrorCode::DomainConflict, "/tasks/dependency_cycle"))?
                .0,
        );
    }
    let mut body = Body::new(first.session_id.0.clone());
    body.line(format!(
        "Planned session: {} to {}",
        moment(&start),
        moment(&end)
    ))?;
    body.line("Elapsed envelope includes active and unattended phases; it is not continuous hands-on work.".into())?;
    for (i, o) in owners.iter().enumerate() {
        let keys = selected
            .iter()
            .filter(|(j, _)| i == *j)
            .map(|(_, k)| k.clone())
            .collect::<Vec<_>>();
        if !keys.is_empty() {
            warnings(&mut body, o, Some(&keys))?;
        }
    }
    let mut people = owners
        .iter()
        .flat_map(|o| {
            o.occurrences
                .iter()
                .flat_map(|p| p.servings.iter().map(|s| s.person.clone()))
        })
        .collect::<Vec<_>>();
    people.sort_by(person_cmp);
    people.dedup();
    let mut titles = Vec::new();
    for (owner, key) in &selected {
        let o = &owners[*owner];
        let occ = o
            .occurrences
            .iter()
            .find(|o| o.key == *key)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/body/occurrence"))?;
        let r = recipe(o.input.historical, &key.recipe)?;
        titles.push(r.title.0.clone());
        let service_label = if occ.method == MethodKind::Production
            && r.components.iter().any(|component| {
                matches!(
                    &component.production.0,
                    Some(ComponentProductionSpec::CookedWithCooling { .. })
                )
            }) {
            if earlier_cooking_endpoint(r, None).is_some() {
                "Hot meal service (original cooking ends earlier; portioning/cooling is scheduled separately)"
            } else {
                "Hot meal service (cooling of remaining shares follows when scheduled)"
            }
        } else {
            "Eat by"
        };
        body.line(format!(
            "\n{} — {} {:?}{}\n{}: {}",
            r.title.0,
            key.slot.date.0,
            key.slot.meal_type,
            if key.extra_id.is_some() {
                " (extra)"
            } else {
                ""
            },
            service_label,
            moment(&occ.eat_by)
        ))?;
        body.line(format!(
            "{}: {}; allocated for this meal: {}",
            if occ.method == MethodKind::Reuse {
                "Portions reused"
            } else {
                "Produced portions"
            },
            rational(occ.scaled.produced_servings.0),
            rational(sum(&occ.servings)?.0)
        ))?;
        for s in &occ.servings {
            let label = people
                .iter()
                .position(|p| p == &s.person)
                .ok_or_else(|| error(ErrorCode::MissingReference, "/person"))?
                + 1;
            body.line(format!(
                "Diner {label}: {} servings",
                rational(s.servings.0)
            ))?;
        }
        if !occ.scaled.ingredients.is_empty() {
            body.line("Ingredients — total for this occurrence; used across steps".into())?;
            for i in &occ.scaled.ingredients {
                let name = r
                    .ingredients
                    .iter()
                    .find(|v| v.ingredient_id == i.ingredient_id)
                    .ok_or_else(|| error(ErrorCode::MissingReference, "/ingredient"))?;
                body.line(format!("{} {}", quantity(&i.quantity), name.name.0))?;
            }
        }
        allocations(&mut body, o, occ, &people)?;
    }
    body.line("\nInstructions — execute each listed phase once".into())?;
    for index in ordered {
        let (owner, b, t) = phases[index];
        let o = &owners[owner];
        let r = recipe(o.input.historical, &b.phase.occurrence.recipe)?;
        let title = r
            .prep
            .iter()
            .find(|p| p.prep_id == b.phase.prep_id)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/phase/title"))?;
        body.line(format!(
            "{} — {} {:?}, {}: {} to {} ({})",
            r.title.0,
            b.phase.occurrence.slot.date.0,
            b.phase.occurrence.slot.meal_type,
            title.title.0,
            moment(&t.elapsed.start),
            moment(&t.elapsed.end),
            if t.active_intervals.is_empty() {
                "unattended"
            } else {
                "active"
            }
        ))?;
        if let Some(limit) = cooling_phase_limit(r, b.phase.prep_id) {
            let occurrence = o
                .occurrences
                .iter()
                .find(|occurrence| occurrence.key == b.phase.occurrence)
                .ok_or_else(|| error(ErrorCode::MissingReference, "/cooling/occurrence"))?;
            if let Some(prep_id) = earlier_cooking_endpoint(r, Some(b.phase.prep_id)) {
                let phase = PhaseKey {occurrence:b.phase.occurrence.clone(),prep_id};
                let (_, completion) = binding_task(o, &phase)?;
                body.line(format!(
                    "Portioning/cooling/put-away after original cooking at {}. Hot service: {}. Finish cooling and refrigeration within {} of that cooking endpoint. These times are planned; follow the recipe handling instructions.",
                    moment(&completion.elapsed.end), moment(&occurrence.eat_by), cooling_duration(limit)
                ))?;
            } else { body.line(format!(
                "After-service cooling/put-away of remaining portions. Hot service: {}. Cool and refrigerate the remaining portions within {} of cooking. These times are planned; follow the recipe handling instructions.",
                moment(&occurrence.eat_by),
                cooling_duration(limit)
            ))?; }
        }
        for active in t.active_intervals.iter() {
            body.line(format!(
                "Active: {} to {}",
                moment(&active.start),
                moment(&active.end)
            ))?;
        }
        for dep in t.depends_on.iter() {
            if let Some(local) = o.bindings.iter().find(|v| v.task == *dep)
                && local.phase.occurrence == b.phase.occurrence
            {
                body.line(format!(
                    "Requires phase {} for this meal",
                    local.phase.prep_id
                ))?;
            }
        }
        let text = texts
            .iter()
            .find(|p| p.owner == o.pin && p.occurrence == b.phase.occurrence)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/text"))?;
        for step in ordered_steps(r, b.phase.prep_id)? {
            let text = text
                .steps
                .iter()
                .find(|p| p.prep_id == b.phase.prep_id && p.step_id == step)
                .ok_or_else(|| error(ErrorCode::MissingReference, "/text/step"))?;
            body.line(format!("{step}. {}", text.text.0))?;
        }
    }
    let mut credited = Vec::new();
    for (owner, key) in &selected {
        let o = &owners[*owner];
        let occ = o
            .occurrences
            .iter()
            .find(|o| o.key == *key)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/occurrence"))?;
        let r = recipe(o.input.historical, &key.recipe)?;
        // Explicitly name phases for this occurrence executed in other sessions.
        for b in o.bindings.iter().filter(|b| {
            b.phase.occurrence == *key
                && !phases
                    .iter()
                    .any(|(n, v, _)| *n == *owner && v.phase == b.phase)
        }) {
            let (_, t) = binding_task(o, &b.phase)?;
            if let Some(limit) = cooling_phase_limit(r, b.phase.prep_id) {
                if let Some(prep_id) = earlier_cooking_endpoint(r, Some(b.phase.prep_id)) {
                    let phase=PhaseKey{occurrence:b.phase.occurrence.clone(),prep_id};
                    let (_, completion)=binding_task(o,&phase)?;
                    body.line(format!(
                        "Remaining-portion handling/cooling scheduled from original cooking at {}: phase {}, {} to {}; hot service at {}. Finish cooling and refrigeration within {} of that cooking endpoint. These times are planned; follow the recipe handling instructions.",
                        moment(&completion.elapsed.end),b.phase.prep_id,moment(&t.elapsed.start),moment(&t.elapsed.end),moment(&occ.eat_by),cooling_duration(limit)
                    ))?;
                } else { body.line(format!(
                    "Remaining-portion cooling scheduled after hot service at {}: phase {}, {} to {}; cool and refrigerate within {} of cooking. These times are planned; follow the recipe handling instructions.",
                    moment(&occ.eat_by),
                    b.phase.prep_id,
                    moment(&t.elapsed.start),
                    moment(&t.elapsed.end),
                    cooling_duration(limit)
                ))?; }
            } else {
                body.line(format!(
                    "Preparation scheduled elsewhere for {} {:?}: phase {}, {} to {}; required by {}",
                    key.slot.date.0,
                    key.slot.meal_type,
                    b.phase.prep_id,
                    moment(&t.elapsed.start),
                    moment(&t.elapsed.end),
                    moment(&occ.eat_by)
                ))?;
            }
        }
        let mut stored_components = Vec::new();
        for (component, source, phase) in source_components(o, occ)? {
            let (_, t) = binding_task(o, &phase)?;
            body.line(format!("Preparation scheduled earlier for this reuse: {} {:?}, component {}, phase {}, {} to {}",source.slot.date.0,source.slot.meal_type,component.component_id.0,phase.prep_id,moment(&t.elapsed.start),moment(&t.elapsed.end)))?;
            if !stored_components.contains(&component.component_id) {
                storage(&mut body, component)?;
                stored_components.push(component.component_id.clone());
            }
            if !credited.contains(&(*owner, source.recipe.clone())) {
                credits(&mut body, recipe(o.input.historical, &source.recipe)?)?;
                credited.push((*owner, source.recipe.clone()));
            }
        }
        // New production retains the same source clocks and complete storage instructions.
        if key.extra_id.is_none() {
            for batch in o
                .input
                .historical
                .derived
                .batches
                .iter()
                .filter(|b| b.source_slot == key.slot && b.recipe == key.recipe)
            {
                for c in batch.components.iter() {
                    storage(&mut body, c)?;
                }
            }
        }
        if !credited.contains(&(*owner, key.recipe.clone())) {
            credits(&mut body, r)?;
            credited.push((*owner, key.recipe.clone()));
        }
    }
    let title = titles.join(" + ");
    let title = Title(if title.chars().count() > 200 {
        "Cooking session".into()
    } else {
        title
    });
    shape(&title)?;
    Ok(EventDraft {
        logical_key,
        contributors: members.to_vec(),
        start,
        end,
        title,
        body: body.finish()?,
    })
}
fn allocations(
    body: &mut Body,
    o: &Owner<'_>,
    occ: &Occurrence,
    people: &[PersonRef],
) -> Result<(), CalendarFailure> {
    if occ.key.extra_id.is_some() {
        return Ok(());
    }
    let h = o.input.historical;
    let selection = h
        .basis
        .plan
        .selections
        .iter()
        .find(|s| s.slot == occ.key.slot)
        .ok_or_else(|| error(ErrorCode::MissingReference, "/selection"))?;
    let (batch_id, source) = match &selection.batch_use {
        BatchUse::NewBatch { batch_id, .. } => (batch_id, true),
        BatchUse::Leftover { batch_id, .. } => (batch_id, false),
    };
    for a in h
        .derived
        .allocations
        .iter()
        .filter(|a| a.batch_id == *batch_id && (source || a.target_slot == occ.key.slot))
    {
        let diner = people
            .iter()
            .position(|p| *p == a.person)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/allocation/person"))?
            + 1;
        body.line(format!(
            "Allocated component {}: {} to Diner {}, {} {:?}; use at {}. Planned reheats: {}",
            a.component_id.0,
            quantity(&a.quantity),
            diner,
            a.target_slot.date.0,
            a.target_slot.meal_type,
            moment(&a.use_at),
            a.planned_reheats
        ))?;
    }
    if source {
        let batch = h
            .derived
            .batches
            .iter()
            .find(|b| b.batch_id == *batch_id)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/batch"))?;
        for c in batch.components.iter() {
            body.line(format!(
                "Unallocated component {}: {} {:?} ({:?})",
                c.component_id.0,
                rational(c.unallocated.amount),
                c.unallocated.unit,
                c.unallocated.form
            ))?;
        }
    }
    Ok(())
}
fn storage(body: &mut Body, c: &DerivedComponent) -> Result<(), CalendarFailure> {
    let (label, at) = match &c.original_production {
        ProductionMoment::Cooked { completed_at } => ("planned cooking completion", completed_at),
        ProductionMoment::Unheated {
            preparation_started_at,
        } => ("planned first handling", preparation_started_at),
    };
    body.line(format!("Original storage clock — {label}: {}", moment(at)))?;
    if let Some(r) = &c.storage_rule.0 {
        body.line(format!("Cooling/storage: {}\nMaximum chilled elapsed: {} seconds; maximum reheats: {}\nAdmitted reheating method (only when planned): {}",r.cooling_instruction.0,r.max_chilled_seconds,r.max_reheats,r.reheating_instruction.0))?;
        source_credit(body, &r.source)?;
    }
    Ok(())
}
fn source_credit(body: &mut Body, s: &SourceRecord) -> Result<(), CalendarFailure> {
    for a in s.authors.iter() {
        body.line(format!("Source author: {}", a.0))?;
    }
    if let Some(url) = &s.url.0 {
        body.line(format!("Source: {}", url.0))?;
    }
    if let Some(revision) = &s.source_revision.0 {
        body.line(format!("Source revision: {}", revision.0))?;
    }
    if let Some(notice) = &s.adaptation_notice.0 {
        body.line(format!("Adaptation: {}", notice.0))?;
    }
    Ok(())
}
fn credits(body: &mut Body, r: &RecipeRevision) -> Result<(), CalendarFailure> {
    body.line(format!(
        "\nCredits — {}\n{}\nLicence: {} ({:?})",
        r.title.0,
        r.text_rights.attribution.0,
        r.text_rights.licence_id.0,
        r.text_rights.distribution
    ))?;
    if let Some(url) = &r.text_rights.licence_url.0 {
        body.line(format!("Licence URL: {}", url.0))?;
    }
    for adaptation in r.text_rights.adaptations.iter() {
        body.line(format!("Adaptation: {}", adaptation.0))?;
    }
    source_credit(body, &r.source)
}
