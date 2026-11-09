//! Pure fixed-layout food scheduling. Native evidence is checked for closure;
//! its source authority and reservation-query completeness remain store-owned.
// R3 deliberately retains the complete by-value feasibility failure report.
#![allow(clippy::result_large_err)]
use crate::food::*;
use chrono::{DateTime, LocalResult, TimeZone, Utc};
use chrono_tz::Tz;
use mstrchf_contracts::*;
use std::collections::{BTreeMap, BTreeSet};

pub use crate::food::ProductDateAssessment;
const PLACEMENT_LIMIT: usize = 100_000;

fn error(code: ErrorCode, path: &str) -> DomainError {
    DomainError::new(code, path)
}
fn ensure(ok: bool, code: ErrorCode, path: &str) -> Result<(), DomainError> {
    if ok { Ok(()) } else { Err(error(code, path)) }
}
fn shape(v: &impl ValidateShape) -> Result<(), DomainError> {
    v.validate_shape().map_err(|issues| {
        error(
            issues.first().map_or(ErrorCode::InvalidValue, |v| v.code),
            issues.first().map_or("/prep", |v| v.path.as_str()),
        )
    })
}
pub(crate) fn seconds(v: &ZonedMoment) -> Result<i64, DomainError> {
    shape(v)?;
    DateTime::parse_from_rfc3339(&v.instant.0)
        .map(|v| v.timestamp())
        .map_err(|_| error(ErrorCode::InvalidValue, "/instant"))
}
fn moment(value: i64, zone: &ZoneId) -> Result<ZonedMoment, DomainError> {
    shape(zone)?;
    let utc = DateTime::<Utc>::from_timestamp(value, 0)
        .ok_or_else(|| error(ErrorCode::ArithmeticOverflow, "/instant"))?;
    let tz: Tz = zone
        .0
        .parse()
        .map_err(|_| error(ErrorCode::InvalidValue, "/zone"))?;
    let local = utc.with_timezone(&tz);
    let resolution = match tz.from_local_datetime(&local.naive_local()) {
        LocalResult::Single(_) => Resolution::Unambiguous,
        LocalResult::Ambiguous(a, b) => {
            if local == a.min(b) {
                Resolution::Earlier
            } else {
                Resolution::Later
            }
        }
        LocalResult::None => return Err(error(ErrorCode::InvalidValue, "/resolution")),
    };
    ZonedMoment::new(
        LocalDate(local.format("%Y-%m-%d").to_string()),
        LocalTime(local.format("%H:%M:%S").to_string()),
        zone.clone(),
        resolution,
        Instant(utc.format("%Y-%m-%dT%H:%M:%SZ").to_string()),
    )
}
fn add(a: i64, b: i64) -> Result<i64, DomainError> {
    a.checked_add(b)
        .ok_or_else(|| error(ErrorCode::ArithmeticOverflow, "/time"))
}
fn overlaps(a: (i64, i64), b: (i64, i64)) -> bool {
    a.0 < a.1 && b.0 < b.1 && a.0 < b.1 && b.0 < a.1
}

/// Date-only assessment; unknown dates remain visible and nonblocking.
pub fn check_product_use(
    use_by: Option<&LocalDate>,
    use_at: &ZonedMoment,
) -> Result<ProductDateAssessment, DomainError> {
    shape(use_at)?;
    match use_by {
        None => Ok(ProductDateAssessment::UnresolvedDateEvidence),
        Some(date) => {
            shape(date)?;
            Ok(if use_at.local_date > *date {
                ProductDateAssessment::ConflictingKnownLimit
            } else {
                ProductDateAssessment::WithinKnownLimit
            })
        }
    }
}
/// Delivery end equality is allowed; compare real instants, not local clocks.
pub fn check_delivery(
    delivery_end: &ZonedMoment,
    needed_at: &ZonedMoment,
) -> Result<(), DomainError> {
    ensure(
        seconds(delivery_end)? <= seconds(needed_at)?,
        ErrorCode::DomainConflict,
        "/delivery_end",
    )
}

fn timeline(sessions: &[PrepSession], allow_external: bool) -> Result<(), DomainError> {
    ensure(sessions.len() <= 256, ErrorCode::LimitExceeded, "/sessions")?;
    ensure(
        sessions.iter().map(|s| s.tasks.len()).sum::<usize>() <= 4096,
        ErrorCode::LimitExceeded,
        "/tasks",
    )?;
    let mut refs = Vec::new();
    let mut active = Vec::new();
    for (si, s) in sessions.iter().enumerate() {
        shape(s)?;
        ensure(
            !sessions[..si].iter().any(|p| p.session_id == s.session_id),
            ErrorCode::InvalidValue,
            "/sessions/session_id",
        )?;
        let lo = seconds(&s.start)?;
        let hi = seconds(&s.end)?;
        ensure(
            lo <= hi
                && seconds(&s.ingredient_needed_at)? >= lo
                && seconds(&s.ingredient_needed_at)? <= hi,
            ErrorCode::InvalidValue,
            "/sessions/envelope",
        )?;
        let mut earliest = i64::MAX;
        let mut latest = i64::MIN;
        for (ti, t) in s.tasks.iter().enumerate() {
            ensure(
                !s.tasks[..ti].iter().any(|p| p.task_id == t.task_id),
                ErrorCode::InvalidValue,
                "/tasks/task_id",
            )?;
            let a = seconds(&t.elapsed.start)?;
            let b = seconds(&t.elapsed.end)?;
            ensure(
                lo <= a && a <= b && b <= hi,
                ErrorCode::InvalidValue,
                "/tasks/elapsed",
            )?;
            earliest = earliest.min(a);
            latest = latest.max(b);
            refs.push((
                PrepTaskRef {
                    session_id: s.session_id.clone(),
                    task_id: t.task_id,
                },
                t,
            ));
            for interval in t.active_intervals.iter() {
                active.push((seconds(&interval.start)?, seconds(&interval.end)?));
            }
        }
        ensure(
            earliest == lo && latest == hi,
            ErrorCode::InvalidValue,
            "/sessions/envelope",
        )?;
        if let Some(target) = &s.attached_to.0 {
            ensure(
                target != &s.session_id,
                ErrorCode::InvalidValue,
                "/attached_to",
            )?;
            if !allow_external {
                ensure(
                    sessions.iter().any(|p| &p.session_id == target),
                    ErrorCode::ForeignReference,
                    "/attached_to",
                )?;
            }
        }
    }
    active.sort_unstable();
    let mut active_end = i64::MIN;
    for &(a, b) in &active {
        if a < b {
            ensure(
                a >= active_end,
                ErrorCode::DomainConflict,
                "/active_intervals",
            )?;
            active_end = active_end.max(b);
        }
    }
    let mut edges = vec![Vec::new(); refs.len()];
    for (i, (_, task)) in refs.iter().enumerate() {
        for (di, dependency) in task.depends_on.iter().enumerate() {
            ensure(
                !task.depends_on[..di].contains(dependency),
                ErrorCode::InvalidValue,
                "/depends_on",
            )?;
            if let Some(j) = refs.iter().position(|(r, _)| r == dependency) {
                ensure(i != j, ErrorCode::InvalidValue, "/depends_on")?;
                ensure(
                    seconds(&refs[j].1.elapsed.end)? <= seconds(&task.elapsed.start)?,
                    ErrorCode::DomainConflict,
                    "/depends_on",
                )?;
                edges[i].push(j);
            } else {
                ensure(allow_external, ErrorCode::ForeignReference, "/depends_on")?;
            }
        }
    }
    topological(&edges)?;
    for s in sessions {
        let mut chain = BTreeSet::new();
        let mut next = Some(s);
        while let Some(node) = next {
            ensure(
                chain.insert(node.session_id.clone()),
                ErrorCode::InvalidValue,
                "/attached_to",
            )?;
            next = node
                .attached_to
                .0
                .as_ref()
                .and_then(|id| sessions.iter().find(|v| &v.session_id == id));
        }
    }
    Ok(())
}
/// Output-only graph/active-time check. Use validate_schedule for recipe closure.
pub fn validate_timeline(sessions: &[PrepSession]) -> Result<(), Vec<Conflict>> {
    timeline(sessions, false).map_err(|e| {
        vec![Conflict {
            code: e.code,
            week_ids: BoundedVec(vec![]),
            slot_ids: BoundedVec(vec![]),
            recipe_refs: BoundedVec(vec![]),
            evidence_ids: BoundedVec(vec![]),
            blocking: true,
        }]
    })
}
fn topological(edges: &[Vec<usize>]) -> Result<Vec<usize>, DomainError> {
    let mut done = vec![false; edges.len()];
    let mut out = Vec::with_capacity(edges.len());
    while out.len() < edges.len() {
        let next = (0..edges.len()).find(|&i| !done[i] && edges[i].iter().all(|&j| done[j]));
        let i = next.ok_or_else(|| error(ErrorCode::DomainConflict, "/dependencies/cycle"))?;
        done[i] = true;
        out.push(i);
    }
    Ok(out)
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum PortableRole {
    Packing,
    Service,
}

#[derive(Clone)]
struct Phase {
    key: PhaseKey,
    spec: PrepSpec,
    ingredient_ids: Vec<IngredientId>,
    equipment: Vec<EquipmentCode>,
    zone: ZoneId,
    earliest: i64,
    latest: i64,
    duration: i64,
    deadline: i64,
    dependencies: Vec<PhaseKey>,
    portable_role: Option<PortableRole>,
}
fn phase_index(phases: &[Phase], key: &PhaseKey) -> Result<usize, DomainError> {
    phases
        .iter()
        .position(|p| &p.key == key)
        .ok_or_else(|| error(ErrorCode::ForeignReference, "/phase"))
}

// Production quantities are independent of the diners served at this occasion.
// Cooling handles the original remainder, including unallocated portions.
fn retains_portions(occurrence: &Occurrence) -> Result<bool, DomainError> {
    let mut consumed = Rational::new(0, 1)?;
    for serving in &occurrence.servings {
        consumed = consumed.checked_add(&serving.servings.0)?;
    }
    let produced = occurrence.scaled.produced_servings.0;
    let left = u128::from(consumed.numerator.0) * u128::from(produced.denominator.0);
    let right = u128::from(produced.numerator.0) * u128::from(consumed.denominator.0);
    ensure(
        left <= right,
        ErrorCode::DomainConflict,
        "/cooling/production_servings",
    )?;
    Ok(left < right)
}

fn model(
    plan: &WeekPlan,
    context: &PlanningContext,
    evidence: &FoodEvidence,
) -> Result<Vec<Phase>, FoodFailure> {
    let occurrences = crate::food::occurrences(plan, context, evidence)?;
    let mut phases = Vec::new();
    for occurrence in &occurrences {
        let reviewed = crate::food::reviewed(context, evidence, &occurrence.key.recipe)?;
        let recipe = reviewed.recipe();
        let method = evidence
            .methods
            .iter()
            .find(|m| m.recipe == occurrence.key.recipe && m.kind == occurrence.kind)
            .ok_or_else(|| error(ErrorCode::UnsupportedCapability, "/methods"))?;
        ensure(
            evidence
                .methods
                .iter()
                .filter(|m| m.recipe == occurrence.key.recipe && m.kind == occurrence.kind)
                .count()
                == 1,
            ErrorCode::InvalidValue,
            "/methods",
        )?;
        ensure(
            method.prep_ids.len() <= 128,
            ErrorCode::LimitExceeded,
            "/methods/prep_ids",
        )?;
        ensure(
            method.prep_ids.windows(2).all(|v| v[0] < v[1]),
            ErrorCode::NoncanonicalValue,
            "/methods/prep_ids",
        )?;
        // The admitted method names the only two portable roles. The owner
        // declaration is validated by occurrences; this model enforces the
        // actual task clocks, including saved-output revalidation.
        let portable = method
            .portable_cold
            .as_ref()
            .map(|method| {
                let journey = plan
                    .selections
                    .iter()
                    .find(|selection| {
                        selection.slot == occurrence.key.slot
                            && selection.recipe == occurrence.key.recipe
                            && occurrence.key.extra_id.is_none()
                    })
                    .and_then(|selection| selection.portable_cold.as_ref())
                    .ok_or_else(|| error(ErrorCode::MissingReference, "/portable_cold/journey"))?;
                let packing_complete = seconds(&journey.packing_complete_at)?;
                let departure = seconds(&journey.depart_at)?;
                ensure(
                    packing_complete <= departure && departure <= seconds(&occurrence.use_at)?,
                    ErrorCode::DomainConflict,
                    "/portable_cold/chronology",
                )?;
                Ok::<_, DomainError>((method, journey, packing_complete, departure))
            })
            .transpose()?;
        let mut cooling_limits = BTreeMap::new();
        for component in recipe.components.iter() {
            if let Some(ComponentProductionSpec::CookedWithCooling {
                completion_prep_id,
                cooling_prep_ids,
                max_cooling_seconds,
            }) = &component.production.0
            {
                let completion = recipe.prep.iter()
                    .find(|phase| phase.prep_id == *completion_prep_id)
                    .ok_or_else(|| error(ErrorCode::MissingReference, "/cooling/completion"))?;
                let endpoint = i64::from(completion.earliest_offset_seconds)
                    + i64::from(completion.elapsed_minutes) * 60;
                for &id in cooling_prep_ids.iter() {
                    ensure(
                        cooling_limits.insert(id, (endpoint, *max_cooling_seconds)).is_none(),
                        ErrorCode::DomainConflict,
                        "/cooling/roles",
                    )?;
                }
                if method.kind == MethodKind::Production {
                    ensure(
                        occurrence.is_source && occurrence.key.extra_id.is_none(),
                        ErrorCode::UnsupportedCapability,
                        "/cooling/source",
                    )?;
                    ensure(
                        method.prep_ids.contains(completion_prep_id)
                            && cooling_prep_ids
                                .iter()
                                .all(|id| method.prep_ids.contains(id)),
                        ErrorCode::MissingReference,
                        "/cooling/method",
                    )?;
                } else {
                    ensure(
                        cooling_prep_ids
                            .iter()
                            .all(|id| !method.prep_ids.contains(id)),
                        ErrorCode::DomainConflict,
                        "/cooling/reuse_method",
                    )?;
                }
            }
        }
        let needs_cooling =
            !cooling_limits.is_empty() && occurrence.is_source && retains_portions(occurrence)?;
        let mut steps = BTreeSet::new();
        for &id in &method.prep_ids {
            let cooling_limit = cooling_limits.get(&id).copied();
            if cooling_limit.is_some() && !needs_cooling {
                continue;
            }
            let spec = recipe
                .prep
                .iter()
                .find(|p| p.prep_id == id)
                .ok_or_else(|| error(ErrorCode::ForeignReference, "/prep_id"))?
                .clone();
            ensure(
                spec.active_minutes == 0 || spec.active_minutes == spec.elapsed_minutes,
                ErrorCode::UnsupportedCapability,
                "/prep/atomic_phase",
            )?;
            for step in spec.step_ids.iter() {
                ensure(
                    steps.insert(*step),
                    ErrorCode::InvalidValue,
                    "/prep/step_ids",
                )?;
            }
            let mut active = 0u32;
            let mut elapsed = 0u32;
            for step_id in spec.step_ids.iter() {
                let step = recipe
                    .steps
                    .iter()
                    .find(|s| s.step_id == *step_id)
                    .ok_or_else(|| error(ErrorCode::MissingReference, "/prep/step_ids"))?;
                ensure(
                    step.active_minutes == 0 || step.active_minutes == step.elapsed_minutes,
                    ErrorCode::UnsupportedCapability,
                    "/steps/atomic_phase",
                )?;
                active += u32::from(step.active_minutes);
                elapsed += u32::from(step.elapsed_minutes);
                for dep in step
                    .depends_on
                    .iter()
                    .filter(|d| !spec.step_ids.contains(d))
                {
                    if let Some(other) = recipe
                        .prep
                        .iter()
                        .find(|p| method.prep_ids.contains(&p.prep_id) && p.step_ids.contains(dep))
                    {
                        ensure(
                            spec.depends_on.contains(&other.prep_id),
                            ErrorCode::MissingReference,
                            "/prep/instruction_dependency",
                        )?;
                    } else {
                        ensure(
                            occurrence.kind == MethodKind::Reuse,
                            ErrorCode::MissingReference,
                            "/prep/instruction_dependency",
                        )?;
                    }
                }
            }
            ensure(
                active == u32::from(spec.active_minutes)
                    && elapsed == u32::from(spec.elapsed_minutes),
                ErrorCode::UnsupportedCapability,
                "/prep/instruction_duration",
            )?;
            let resources = evidence
                .resources
                .iter()
                .find(|r| r.recipe == occurrence.key.recipe && r.prep_id == id)
                .ok_or_else(|| error(ErrorCode::UnsupportedCapability, "/resources"))?;
            ensure(
                evidence
                    .resources
                    .iter()
                    .filter(|r| r.recipe == occurrence.key.recipe && r.prep_id == id)
                    .count()
                    == 1,
                ErrorCode::InvalidValue,
                "/resources",
            )?;
            let requirement = context
                .requirements
                .iter()
                .find(|r| r.slot == occurrence.key.slot)
                .ok_or_else(|| error(ErrorCode::ForeignReference, "/requirements"))?;
            ensure(
                resources
                    .equipment
                    .iter()
                    .all(|e| requirement.equipment.contains(e)),
                ErrorCode::DomainConflict,
                "/equipment",
            )?;
            let meal_time = seconds(&occurrence.use_at)?;
            let portable_role = match portable {
                Some((method, ..)) if id == method.packing_prep_id => Some(PortableRole::Packing),
                Some((method, ..)) if id == method.service_prep_id => Some(PortableRole::Service),
                Some(_) => {
                    return Err(error(ErrorCode::ForeignReference, "/portable_cold/phase").into());
                }
                None => None,
            };
            let anchor = match (portable_role, portable) {
                (Some(PortableRole::Packing), Some((_, _, packing_complete, _))) => {
                    packing_complete
                }
                _ => meal_time,
            };
            let deadline = match cooling_limit {
                Some((endpoint, limit)) => add(add(meal_time, endpoint)?, i64::from(limit))?,
                None => anchor,
            };
            let duration = i64::from(spec.elapsed_minutes) * 60;
            let mut earliest = add(anchor, i64::from(spec.earliest_offset_seconds))?;
            let mut latest =
                add(anchor, i64::from(spec.latest_offset_seconds))?.min(add(deadline, -duration)?);
            if let Some((_, _, packing_complete, departure)) = portable {
                match portable_role {
                    Some(PortableRole::Packing) => {
                        // A saved completion time names the actual endpoint,
                        // rather than silently allowing earlier packing.
                        let start = add(packing_complete, -duration)?;
                        earliest = earliest.max(start);
                        latest = latest.min(start);
                    }
                    Some(PortableRole::Service) => earliest = earliest.max(departure),
                    None => {}
                }
            }
            ensure(
                earliest <= latest,
                ErrorCode::DomainConflict,
                "/prep/window",
            )?;
            let mut dependencies: Vec<_> = spec
                .depends_on
                .iter()
                .map(|id| PhaseKey {
                    occurrence: occurrence.key.clone(),
                    prep_id: *id,
                })
                .collect();
            if let (Some(PortableRole::Service), Some((method, ..))) = (portable_role, portable) {
                dependencies.push(PhaseKey {
                    occurrence: occurrence.key.clone(),
                    prep_id: method.packing_prep_id,
                });
            }
            // Only ingredients produced locally require new raw ingredients here.
            // Reused source ingredients retain their original procurement/use graph.
            let ingredient_ids = occurrence
                .scaled
                .ingredients
                .iter()
                .filter(|ingredient| {
                    occurrence.kind == MethodKind::Production
                        && cooling_limit.is_none()
                        && (spec.ingredient_ids.contains(&ingredient.ingredient_id)
                            || recipe.steps.iter().any(|step| {
                                spec.step_ids.contains(&step.step_id)
                                    && (step.ingredient_ids.contains(&ingredient.ingredient_id)
                                        || step.component_ids.contains(&ingredient.component_id))
                            }))
                })
                .map(|ingredient| ingredient.ingredient_id.clone())
                .collect();
            phases.push(Phase {
                key: PhaseKey {
                    occurrence: occurrence.key.clone(),
                    prep_id: id,
                },
                spec,
                ingredient_ids,
                equipment: resources.equipment.clone(),
                zone: match (portable_role, portable) {
                    (Some(PortableRole::Packing), Some((_, journey, ..))) => {
                        journey.packing_complete_at.zone.clone()
                    }
                    _ => occurrence.use_at.zone.clone(),
                },
                earliest,
                latest,
                duration,
                deadline,
                dependencies,
                portable_role,
            });
            ensure(phases.len() <= 4096, ErrorCode::LimitExceeded, "/phases")?;
        }
    }
    phases.sort_by(|a, b| phase_cmp(&a.key, &b.key));
    ensure(
        phases
            .windows(2)
            .all(|v| phase_cmp(&v[0].key, &v[1].key).is_lt()),
        ErrorCode::InvalidValue,
        "/phases",
    )?;
    // Every consuming phase has explicit edges to its exact original components.
    for occurrence in occurrences
        .iter()
        .filter(|o| !o.is_source && o.key.extra_id.is_none())
    {
        let source = occurrences
            .iter()
            .find(|o| o.is_source && o.batch_id == occurrence.batch_id)
            .ok_or_else(|| error(ErrorCode::ForeignReference, "/source"))?;
        let selected = plan
            .selections
            .iter()
            .find(|s| s.slot == occurrence.key.slot)
            .ok_or_else(|| error(ErrorCode::ForeignReference, "/selection"))?;
        let components = match &selected.batch_use {
            BatchUse::Leftover { component_ids, .. } => component_ids,
            _ => return Err(error(ErrorCode::InvalidValue, "/source").into()),
        };
        let reviewed = crate::food::reviewed(context, evidence, &source.key.recipe)?;
        let mut deps = Vec::new();
        for component in components.iter() {
            let spec = reviewed
                .recipe()
                .components
                .iter()
                .find(|c| &c.component_id == component)
                .ok_or_else(|| error(ErrorCode::ForeignReference, "/component"))?;
            if matches!(
                spec.production.0,
                Some(ComponentProductionSpec::CookedWithCooling { .. })
            ) {
                let consuming = crate::food::reviewed(context, evidence, &occurrence.key.recipe)?;
                ensure(
                    phases
                        .iter()
                        .filter(|p| p.key.occurrence == occurrence.key)
                        .any(|phase| {
                            consuming.recipe().steps.iter().any(|step| {
                                phase.spec.step_ids.contains(&step.step_id)
                                    && step.component_ids.contains(component)
                            })
                        }),
                    ErrorCode::MissingReference,
                    "/cooling/consuming_phase",
                )?;
            }
            let prep_ids = match &spec.production.0 {
                Some(ComponentProductionSpec::Cooked { completion_prep_id }) => {
                    vec![*completion_prep_id]
                }
                Some(ComponentProductionSpec::CookedWithCooling {
                    completion_prep_id,
                    cooling_prep_ids,
                    ..
                }) => {
                    let mut ids = vec![*completion_prep_id];
                    ids.extend(cooling_prep_ids.iter().copied());
                    ids
                }
                Some(ComponentProductionSpec::Unheated {
                    first_handling_prep_id,
                }) => vec![*first_handling_prep_id],
                None => return Err(error(ErrorCode::UnsupportedCapability, "/production").into()),
            };
            for prep_id in prep_ids {
                let key = PhaseKey {
                    occurrence: source.key.clone(),
                    prep_id,
                };
                phase_index(&phases, &key)?;
                deps.push((component.clone(), key));
            }
        }
        let consuming = crate::food::reviewed(context, evidence, &occurrence.key.recipe)?;
        for phase in phases
            .iter_mut()
            .filter(|p| p.key.occurrence == occurrence.key)
        {
            phase.dependencies.extend(
                deps.iter()
                    .filter(|(component, _)| {
                        phase.portable_role == Some(PortableRole::Packing)
                            || consuming.recipe().steps.iter().any(|step| {
                                phase.spec.step_ids.contains(&step.step_id)
                                    && step.component_ids.contains(component)
                            })
                    })
                    .map(|(_, key)| key.clone()),
            );
        }
    }
    for p in &mut phases {
        p.dependencies.sort_by(phase_cmp);
        p.dependencies.dedup();
    }
    let edges = phases
        .iter()
        .map(|p| {
            p.dependencies
                .iter()
                .map(|d| phase_index(&phases, d))
                .collect()
        })
        .collect::<Result<Vec<Vec<_>>, _>>()?;
    topological(&edges)?;
    Ok(phases)
}
pub fn required_phase_keys(
    plan: &WeekPlan,
    context: &PlanningContext,
    evidence: &FoodEvidence,
) -> Result<Vec<PhaseKey>, FoodFailure> {
    Ok(model(plan, context, evidence)?
        .into_iter()
        .map(|p| p.key)
        .collect())
}

fn layout_keys(
    plan: &WeekPlan,
    phases: &[Phase],
    layout: &ScheduleLayout,
) -> Result<Vec<SessionKey>, DomainError> {
    ensure(
        layout.routes.len() <= 4096
            && layout.fixed_starts.len() <= 4096
            && layout.attachments.len() <= 256,
        ErrorCode::LimitExceeded,
        "/layout",
    )?;
    ensure(
        layout.routes.len() == phases.len(),
        ErrorCode::InvalidValue,
        "/layout/routes",
    )?;
    ensure(
        layout
            .routes
            .windows(2)
            .all(|v| phase_cmp(&v[0].phase, &v[1].phase).is_lt()),
        ErrorCode::NoncanonicalValue,
        "/layout/routes",
    )?;
    ensure(
        layout
            .fixed_starts
            .windows(2)
            .all(|v| phase_cmp(&v[0].phase, &v[1].phase).is_lt()),
        ErrorCode::NoncanonicalValue,
        "/layout/fixed_starts",
    )?;
    ensure(
        layout
            .attachments
            .windows(2)
            .all(|v| session_cmp(&v[0].session, &v[1].session).is_lt()),
        ErrorCode::NoncanonicalValue,
        "/layout/attachments",
    )?;
    let mut sessions = Vec::new();
    for (route, phase) in layout.routes.iter().zip(phases) {
        ensure(
            route.phase == phase.key,
            ErrorCode::ForeignReference,
            "/layout/routes",
        )?;
        shape(&route.session.anchor_slot)?;
        ensure(
            route.session.ordinal > 0 && route.session.ordinal <= 256,
            ErrorCode::InvalidValue,
            "/session/ordinal",
        )?;
        ensure(
            route.phase.occurrence.slot.week_id == plan.week_id
                && route.session.anchor_slot.week_id == plan.week_id,
            ErrorCode::ForeignReference,
            "/session/week",
        )?;
        if !sessions.contains(&route.session) {
            sessions.push(route.session.clone());
        }
    }
    ensure(
        sessions.len() <= 256,
        ErrorCode::LimitExceeded,
        "/layout/sessions",
    )?;
    sessions.sort_by(session_cmp);
    for key in &sessions {
        let routes: Vec<_> = layout.routes.iter().filter(|r| &r.session == key).collect();
        let mut packing = false;
        let mut service = false;
        for route in &routes {
            match phases[phase_index(phases, &route.phase)?].portable_role {
                Some(PortableRole::Packing) => packing = true,
                Some(PortableRole::Service) => service = true,
                None => {}
            }
        }
        ensure(
            !(packing && service),
            ErrorCode::DomainConflict,
            "/portable_cold/sessions",
        )?;
        ensure(
            routes.len() <= 256,
            ErrorCode::LimitExceeded,
            "/session/tasks",
        )?;
        ensure(
            routes
                .iter()
                .any(|r| r.phase.occurrence.slot == key.anchor_slot),
            ErrorCode::ForeignReference,
            "/session/anchor_slot",
        )?;
        let mut ingredients = BTreeSet::new();
        let mut slots = Vec::new();
        for route in routes {
            ingredients.extend(
                phases[phase_index(phases, &route.phase)?]
                    .ingredient_ids
                    .iter()
                    .cloned(),
            );
            if !slots.contains(&route.phase.occurrence.slot) {
                slots.push(route.phase.occurrence.slot.clone());
            }
        }
        ensure(
            ingredients.len() <= 256 && slots.len() <= 21,
            ErrorCode::LimitExceeded,
            "/session/union",
        )?;
    }
    for fixed in &layout.fixed_starts {
        let p = &phases[phase_index(phases, &fixed.phase)?];
        let at = seconds(&fixed.start)?;
        ensure(
            p.earliest <= at && at <= p.latest,
            ErrorCode::DomainConflict,
            "/fixed_starts/window",
        )?;
    }
    for attachment in &layout.attachments {
        ensure(
            sessions.contains(&attachment.session),
            ErrorCode::ForeignReference,
            "/attachments/session",
        )?;
        shape(&attachment.target_session)?;
        for r in layout
            .routes
            .iter()
            .filter(|r| r.session == attachment.session)
        {
            ensure(
                phases[phase_index(phases, &r.phase)?]
                    .portable_role
                    .is_none(),
                ErrorCode::UnsupportedCapability,
                "/portable_cold/attachments",
            )?;
            ensure(
                phases[phase_index(phases, &r.phase)?]
                    .spec
                    .can_attach_to_prior_session,
                ErrorCode::UnsupportedCapability,
                "/attachments/support",
            )?;
        }
    }
    Ok(sessions)
}
pub(crate) fn validate_layout(
    plan: &WeekPlan,
    context: &PlanningContext,
    evidence: &FoodEvidence,
    layout: &ScheduleLayout,
) -> Result<Vec<SessionKey>, FoodFailure> {
    layout_keys(plan, &model(plan, context, evidence)?, layout).map_err(Into::into)
}
fn bindings(
    phases: &[Phase],
    layout: &ScheduleLayout,
    identities: &FoodIdentities,
    keys: &[SessionKey],
) -> Result<Vec<TaskBinding>, DomainError> {
    ensure(
        food_identities_digest(identities)? == identities.digest,
        ErrorCode::DigestMismatch,
        "/identities/digest",
    )?;
    ensure(
        identities.sessions.len() == keys.len(),
        ErrorCode::InvalidValue,
        "/identities/sessions",
    )?;
    let mut ids = BTreeSet::new();
    for ((key, id), expected) in identities.sessions.iter().zip(keys) {
        ensure(
            key == expected,
            ErrorCode::NoncanonicalValue,
            "/identities/sessions",
        )?;
        shape(id)?;
        ensure(
            ids.insert(id.clone()),
            ErrorCode::InvalidValue,
            "/identities/sessions/id",
        )?;
    }
    let mut out = Vec::with_capacity(phases.len());
    for p in phases {
        let route = layout
            .routes
            .iter()
            .find(|r| r.phase == p.key)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/route"))?;
        let rank = layout
            .routes
            .iter()
            .filter(|r| r.session == route.session)
            .position(|r| r.phase == p.key)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/route"))?
            + 1;
        let id = &identities
            .sessions
            .iter()
            .find(|(k, _)| k == &route.session)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/session_identity"))?
            .1;
        out.push(TaskBinding {
            phase: p.key.clone(),
            task: PrepTaskRef {
                session_id: id.clone(),
                task_id: rank as u16,
            },
        });
    }
    Ok(out)
}
fn horizon(phases: &[Phase], layout: &ScheduleLayout) -> Result<Option<(i64, i64)>, DomainError> {
    if phases.is_empty() {
        return Ok(None);
    }
    let mut lower = vec![0; phases.len()];
    let mut upper = vec![0; phases.len()];
    let edges = phases
        .iter()
        .map(|p| {
            p.dependencies
                .iter()
                .map(|d| phase_index(phases, d))
                .collect()
        })
        .collect::<Result<Vec<Vec<_>>, _>>()?;
    let order = topological(&edges)?;
    for &i in &order {
        let p = &phases[i];
        let fixed = layout
            .fixed_starts
            .iter()
            .find(|f| f.phase == p.key)
            .map(|f| seconds(&f.start))
            .transpose()?;
        lower[i] = fixed.unwrap_or(p.earliest);
        upper[i] = fixed.unwrap_or(p.latest);
        for &d in &edges[i] {
            lower[i] = lower[i].max(add(lower[d], phases[d].duration)?);
        }
        ensure(
            lower[i] <= upper[i],
            ErrorCode::DomainConflict,
            "/horizon/dependencies",
        )?;
    }
    for &i in order.iter().rev() {
        for &d in &edges[i] {
            upper[d] = upper[d].min(add(upper[i], -phases[d].duration)?);
            ensure(
                lower[d] <= upper[d],
                ErrorCode::DomainConflict,
                "/horizon/dependencies",
            )?;
        }
    }
    let lo = *lower
        .iter()
        .min()
        .ok_or_else(|| error(ErrorCode::InvalidValue, "/horizon"))?;
    let hi = upper
        .iter()
        .zip(phases)
        .map(|(u, p)| add(*u, p.duration))
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .max()
        .ok_or_else(|| error(ErrorCode::InvalidValue, "/horizon"))?;
    ensure(
        lo < hi,
        ErrorCode::UnsupportedCapability,
        "/horizon/zero_only",
    )?;
    Ok(Some((lo, hi)))
}
#[derive(Clone)]
struct Reservation {
    start: i64,
    end: i64,
    active: Vec<(i64, i64)>,
    equipment: Vec<EquipmentCode>,
}
fn occupied(
    plan: &WeekPlan,
    schedule: &ScheduleInputs,
    expected: Option<(i64, i64)>,
    new_ids: &[PrepSessionId],
) -> Result<Vec<Reservation>, DomainError> {
    let snapshot = &schedule.occupied;
    ensure(
        occupancy_snapshot_digest(snapshot)? == snapshot.digest,
        ErrorCode::DigestMismatch,
        "/occupied/digest",
    )?;
    ensure(
        snapshot.owners.len() <= 64,
        ErrorCode::LimitExceeded,
        "/occupied/owners",
    )?;
    let query = match (&snapshot.query, expected) {
        (None, None) => {
            ensure(
                snapshot.owners.is_empty() && schedule.layout.attachments.is_empty(),
                ErrorCode::InvalidValue,
                "/occupied/empty",
            )?;
            return Ok(vec![]);
        }
        (Some(q), Some((lo, hi))) => {
            shape(&q.horizon)?;
            ensure(
                seconds(&q.horizon.start)? == lo && seconds(&q.horizon.end)? == hi,
                ErrorCode::InvalidValue,
                "/occupied/horizon",
            )?;
            q
        }
        _ => return Err(error(ErrorCode::InvalidValue, "/occupied/query")),
    };
    if let Some(excluded) = &query.excluded_subject {
        ensure(
            excluded.week.week_id == plan.week_id,
            ErrorCode::ForeignReference,
            "/occupied/excluded_subject",
        )?;
    }
    ensure(
        snapshot
            .owners
            .windows(2)
            .all(|v| owner_cmp(&v[0].pin, &v[1].pin).is_lt()),
        ErrorCode::NoncanonicalValue,
        "/occupied/owners",
    )?;
    let mut all_sessions = BTreeSet::new();
    let mut total_tasks = 0usize;
    let mut total_sessions = 0usize;
    let mut out = Vec::new();
    let (lo, hi) = expected.ok_or_else(|| error(ErrorCode::InvalidValue, "/occupied/horizon"))?;
    for (oi, owner) in snapshot.owners.iter().enumerate() {
        ensure(
            !snapshot.owners[..oi]
                .iter()
                .any(|o| o.pin.week == owner.pin.week),
            ErrorCode::IdentityMismatch,
            "/occupied/owner",
        )?;
        ensure(
            query.excluded_subject.as_ref() != Some(&owner.pin),
            ErrorCode::InvalidValue,
            "/occupied/exclusion",
        )?;
        ensure(
            occupied_owner_digest(owner)? == owner.projection_digest,
            ErrorCode::DigestMismatch,
            "/occupied/owner/digest",
        )?;
        ensure(
            owner.sessions.len() <= 256 && owner.task_resources.len() <= 4096,
            ErrorCode::LimitExceeded,
            "/occupied/owner",
        )?;
        ensure(
            owner
                .sessions
                .windows(2)
                .all(|v| v[0].session_id < v[1].session_id),
            ErrorCode::NoncanonicalValue,
            "/occupied/sessions",
        )?;
        ensure(
            owner
                .task_resources
                .windows(2)
                .all(|v| task_cmp(&v[0].task, &v[1].task).is_lt()),
            ErrorCode::NoncanonicalValue,
            "/occupied/resources",
        )?;
        total_sessions += owner.sessions.len();
        total_tasks += owner.sessions.iter().map(|s| s.tasks.len()).sum::<usize>();
        ensure(
            total_sessions <= 4096 && total_tasks <= 16384,
            ErrorCode::LimitExceeded,
            "/occupied/total",
        )?;
        let mut row_count = 0;
        for session in &owner.sessions {
            shape(session)?;
            ensure(
                !session
                    .attached_to
                    .0
                    .as_ref()
                    .is_some_and(|id| new_ids.contains(id)),
                ErrorCode::DomainConflict,
                "/occupied/attachment_cycle",
            )?;
            ensure(
                session.week_id == owner.pin.week.week_id,
                ErrorCode::ForeignReference,
                "/occupied/week",
            )?;
            ensure(
                all_sessions.insert(session.session_id.clone())
                    && !new_ids.contains(&session.session_id),
                ErrorCode::IdentityMismatch,
                "/occupied/session_id",
            )?;
            let start = seconds(&session.start)?;
            let end = seconds(&session.end)?;
            ensure(
                start <= seconds(&session.ingredient_needed_at)?
                    && seconds(&session.ingredient_needed_at)? <= end,
                ErrorCode::InvalidValue,
                "/occupied/ingredient_needed_at",
            )?;
            ensure(
                overlaps((start, end), (lo, hi)),
                ErrorCode::InvalidValue,
                "/occupied/horizon_membership",
            )?;
            let mut task_ids = BTreeSet::new();
            let mut min = i64::MAX;
            let mut max = i64::MIN;
            for task in session.tasks.iter() {
                ensure(
                    !task
                        .depends_on
                        .iter()
                        .any(|r| new_ids.contains(&r.session_id)),
                    ErrorCode::DomainConflict,
                    "/occupied/dependency_cycle",
                )?;
                ensure(
                    task_ids.insert(task.task_id),
                    ErrorCode::InvalidValue,
                    "/occupied/task_id",
                )?;
                let a = seconds(&task.elapsed.start)?;
                let b = seconds(&task.elapsed.end)?;
                min = min.min(a);
                max = max.max(b);
                ensure(
                    start <= a && a <= b && b <= end,
                    ErrorCode::InvalidValue,
                    "/occupied/envelope",
                )?;
                let r = PrepTaskRef {
                    session_id: session.session_id.clone(),
                    task_id: task.task_id,
                };
                let resources = owner
                    .task_resources
                    .iter()
                    .find(|v| v.task == r)
                    .ok_or_else(|| error(ErrorCode::MissingReference, "/occupied/resources"))?;
                ensure(
                    resources.evidence_ids.len() <= 256
                        && !resources.evidence_ids.is_empty()
                        && resources.equipment.len() <= 9,
                    ErrorCode::LimitExceeded,
                    "/occupied/resources",
                )?;
                row_count += 1;
                out.push(Reservation {
                    start: a,
                    end: b,
                    active: task
                        .active_intervals
                        .iter()
                        .map(|v| Ok((seconds(&v.start)?, seconds(&v.end)?)))
                        .collect::<Result<Vec<_>, DomainError>>()?,
                    equipment: resources.equipment.clone(),
                });
            }
            ensure(
                min == start && max == end,
                ErrorCode::InvalidValue,
                "/occupied/envelope",
            )?;
        }
        ensure(
            row_count == owner.task_resources.len(),
            ErrorCode::ForeignReference,
            "/occupied/resources",
        )?;
    }
    validate_retained_graphs(&snapshot.owners)?;
    for attachment in &schedule.layout.attachments {
        let owner = snapshot
            .owners
            .iter()
            .find(|o| o.pin == attachment.target_owner)
            .ok_or_else(|| error(ErrorCode::ForeignReference, "/attachment/owner"))?;
        ensure(
            owner
                .sessions
                .iter()
                .any(|s| s.session_id == attachment.target_session),
            ErrorCode::ForeignReference,
            "/attachment/session",
        )?;
    }
    Ok(out)
}
fn validate_retained_graphs(owners: &[OccupiedOwner]) -> Result<(), DomainError> {
    let sessions: Vec<_> = owners.iter().flat_map(|owner| &owner.sessions).collect();
    let session_indices: BTreeMap<_, _> = sessions
        .iter()
        .enumerate()
        .map(|(i, session)| (&session.session_id, i))
        .collect();
    let tasks: Vec<_> = sessions
        .iter()
        .flat_map(|session| session.tasks.iter().map(|task| (&session.session_id, task)))
        .collect();
    let task_indices: BTreeMap<_, _> = tasks
        .iter()
        .enumerate()
        .map(|(i, (session, task))| ((*session, task.task_id), i))
        .collect();
    let mut dependencies = vec![vec![]; tasks.len()];
    for (i, (_, task)) in tasks.iter().enumerate() {
        for reference in task.depends_on.iter() {
            // A supplied session is complete. Only an absent session can be an
            // out-of-horizon edge whose validity remains the producer's duty.
            if session_indices.contains_key(&reference.session_id) {
                let &j = task_indices
                    .get(&(&reference.session_id, reference.task_id))
                    .ok_or_else(|| error(ErrorCode::MissingReference, "/occupied/dependencies"))?;
                ensure(
                    i != j,
                    ErrorCode::DomainConflict,
                    "/occupied/dependency_cycle",
                )?;
                ensure(
                    seconds(&tasks[j].1.elapsed.end)? <= seconds(&task.elapsed.start)?,
                    ErrorCode::DomainConflict,
                    "/occupied/dependencies/time",
                )?;
                dependencies[i].push(j);
            }
        }
    }
    retained_acyclic(&dependencies, "/occupied/dependency_cycle")?;
    let mut attachments = vec![vec![]; sessions.len()];
    for (i, session) in sessions.iter().enumerate() {
        if let Some(target) = session.attached_to.0.as_ref()
            && let Some(&j) = session_indices.get(target)
        {
            attachments[i].push(j);
        }
    }
    retained_acyclic(&attachments, "/occupied/attachment_cycle")
}
fn retained_acyclic(edges: &[Vec<usize>], path: &str) -> Result<(), DomainError> {
    let mut remaining: Vec<_> = edges.iter().map(Vec::len).collect();
    let mut dependents = vec![vec![]; edges.len()];
    for (i, dependencies) in edges.iter().enumerate() {
        for &j in dependencies {
            dependents[j].push(i);
        }
    }
    let mut ready: Vec<_> = (0..edges.len()).filter(|&i| remaining[i] == 0).collect();
    let mut count = 0;
    while let Some(i) = ready.pop() {
        count += 1;
        for &j in &dependents[i] {
            remaining[j] -= 1;
            if remaining[j] == 0 {
                ready.push(j);
            }
        }
    }
    ensure(count == edges.len(), ErrorCode::DomainConflict, path)
}
fn task_dependencies(
    phase: &Phase,
    mapping: &[TaskBinding],
) -> Result<Vec<PrepTaskRef>, DomainError> {
    let mut refs = phase
        .dependencies
        .iter()
        .map(|key| {
            mapping
                .iter()
                .find(|m| &m.phase == key)
                .map(|m| m.task.clone())
                .ok_or_else(|| error(ErrorCode::MissingReference, "/dependencies"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    refs.sort_by(task_cmp);
    refs.dedup();
    ensure(refs.len() <= 256, ErrorCode::LimitExceeded, "/dependencies")?;
    Ok(refs)
}
fn attachment_envelope(
    key: &SessionKey,
    schedule: &ScheduleInputs,
) -> Result<Option<(i64, i64)>, DomainError> {
    if let Some(a) = schedule
        .layout
        .attachments
        .iter()
        .find(|a| &a.session == key)
    {
        let target = schedule
            .occupied
            .owners
            .iter()
            .find(|o| o.pin == a.target_owner)
            .and_then(|o| o.sessions.iter().find(|s| s.session_id == a.target_session))
            .ok_or_else(|| error(ErrorCode::ForeignReference, "/attachment"))?;
        Ok(Some((seconds(&target.start)?, seconds(&target.end)?)))
    } else {
        Ok(None)
    }
}
fn collision(phase: &Phase, start: i64, prior: &[Reservation]) -> Result<Option<i64>, DomainError> {
    let end = add(start, phase.duration)?;
    let mut skip = None;
    for other in prior {
        if phase.equipment.iter().any(|e| other.equipment.contains(e))
            && overlaps((start, end), (other.start, other.end))
        {
            skip = Some(skip.map_or(other.end, |v: i64| v.max(other.end)));
        }
        if phase.spec.active_minutes > 0 {
            for &(a, b) in &other.active {
                if overlaps((start, end), (a, b)) {
                    skip = Some(skip.map_or(b, |v: i64| v.max(b)));
                }
            }
        }
    }
    Ok(skip)
}
fn reservation(p: &Phase, start: i64) -> Result<Reservation, DomainError> {
    let end = add(start, p.duration)?;
    Ok(Reservation {
        start,
        end,
        active: if p.spec.active_minutes > 0 {
            vec![(start, end)]
        } else {
            vec![]
        },
        equipment: p.equipment.clone(),
    })
}
fn session_members(key: &SessionKey, phases: &[Phase], layout: &ScheduleLayout) -> Vec<usize> {
    phases
        .iter()
        .enumerate()
        .filter(|(_, p)| {
            layout
                .routes
                .iter()
                .any(|r| r.phase == p.key && &r.session == key)
        })
        .map(|(i, _)| i)
        .collect()
}

/// Reconstruct the complete phase/task bijection and check all saved output.
pub fn validate_schedule(
    plan: &WeekPlan,
    context: &PlanningContext,
    evidence: &FoodEvidence,
    identities: &FoodIdentities,
    schedule: &ScheduleInputs,
    sessions: &[PrepSession],
) -> Result<Vec<TaskBinding>, FoodFailure> {
    validate_schedule_shape(schedule)?;
    let phases = model(plan, context, evidence)?;
    let keys = layout_keys(plan, &phases, &schedule.layout)?;
    let mapping = bindings(&phases, &schedule.layout, identities, &keys)?;
    ensure(
        sessions.len() == keys.len(),
        ErrorCode::InvalidValue,
        "/sessions",
    )?;
    let mut reservations = occupied(
        plan,
        schedule,
        horizon(&phases, &schedule.layout)?,
        &identities
            .sessions
            .iter()
            .map(|(_, id)| id.clone())
            .collect::<Vec<_>>(),
    )?;
    timeline(sessions, true)?;
    let mut starts = vec![0; phases.len()];
    for key in &keys {
        let id = &identities
            .sessions
            .iter()
            .find(|(k, _)| k == key)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/session"))?
            .1;
        let session = sessions
            .iter()
            .find(|s| &s.session_id == id)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/sessions"))?;
        ensure(
            session.week_id == plan.week_id,
            ErrorCode::ForeignReference,
            "/sessions/week_id",
        )?;
        let attachment = schedule
            .layout
            .attachments
            .iter()
            .find(|a| &a.session == key);
        ensure(
            session.attached_to.0 == attachment.map(|a| a.target_session.clone()),
            ErrorCode::ForeignReference,
            "/sessions/attached_to",
        )?;
        let envelope = attachment_envelope(key, schedule)?;
        let members = session_members(key, &phases, &schedule.layout);
        ensure(
            members.len() == session.tasks.len(),
            ErrorCode::InvalidValue,
            "/tasks",
        )?;
        let mut ingredients = BTreeSet::new();
        let mut slots = Vec::new();
        let mut needed = i64::MAX;
        for (rank, &index) in members.iter().enumerate() {
            let p = &phases[index];
            let task = session
                .tasks
                .iter()
                .find(|t| t.task_id == rank as u16 + 1)
                .ok_or_else(|| error(ErrorCode::MissingReference, "/tasks/task_id"))?;
            ensure(
                task.recipe == p.key.occurrence.recipe && task.prep_id == p.key.prep_id,
                ErrorCode::ForeignReference,
                "/tasks/phase",
            )?;
            ensure(
                task.depends_on.0 == task_dependencies(p, &mapping)?,
                ErrorCode::ForeignReference,
                "/tasks/depends_on",
            )?;
            let start = seconds(&task.elapsed.start)?;
            let end = seconds(&task.elapsed.end)?;
            starts[index] = start;
            ensure(
                end == add(start, p.duration)?
                    && start >= p.earliest
                    && start <= p.latest
                    && end <= p.deadline,
                ErrorCode::DomainConflict,
                "/tasks/time",
            )?;
            if let Some(fixed) = schedule
                .layout
                .fixed_starts
                .iter()
                .find(|f| f.phase == p.key)
            {
                ensure(
                    start == seconds(&fixed.start)?,
                    ErrorCode::DomainConflict,
                    "/tasks/fixed_start",
                )?;
            }
            if p.spec.active_minutes > 0 {
                ensure(
                    task.active_intervals.len() == 1,
                    ErrorCode::InvalidValue,
                    "/tasks/active_intervals",
                )?;
                let interval = &task.active_intervals[0];
                ensure(
                    seconds(&interval.start)? == start && seconds(&interval.end)? == end,
                    ErrorCode::InvalidValue,
                    "/tasks/active_intervals",
                )?;
                if let Some((a, b)) = envelope {
                    ensure(
                        a <= start && end <= b,
                        ErrorCode::DomainConflict,
                        "/attachment/active_interval",
                    )?;
                }
            } else {
                ensure(
                    task.active_intervals.is_empty(),
                    ErrorCode::InvalidValue,
                    "/tasks/active_intervals",
                )?;
            }
            ensure(
                collision(p, start, &reservations)?.is_none(),
                ErrorCode::DomainConflict,
                "/tasks/resource_collision",
            )?;
            reservations.push(reservation(p, start)?);
            ingredients.extend(p.ingredient_ids.iter().cloned());
            if !p.ingredient_ids.is_empty() {
                needed = needed.min(start);
            }
            if !slots.contains(&p.key.occurrence.slot) {
                slots.push(p.key.occurrence.slot.clone());
            }
        }
        slots.sort_by(slot_cmp);
        let mut actual_slots = session.linked_slots.0.clone();
        actual_slots.sort_by(slot_cmp);
        ensure(
            actual_slots == slots
                && session.ingredient_ids.0 == ingredients.into_iter().collect::<Vec<_>>(),
            ErrorCode::ForeignReference,
            "/sessions/closure",
        )?;
        if needed == i64::MAX {
            needed = seconds(&session.start)?;
        }
        ensure(
            seconds(&session.ingredient_needed_at)? == needed,
            ErrorCode::InvalidValue,
            "/sessions/ingredient_needed_at",
        )?;
    }
    for (i, p) in phases.iter().enumerate() {
        for dependency in &p.dependencies {
            let d = phase_index(&phases, dependency)?;
            ensure(
                add(starts[d], phases[d].duration)? <= starts[i],
                ErrorCode::DomainConflict,
                "/tasks/dependencies",
            )?;
        }
    }
    Ok(mapping)
}

fn place(
    phases: &[Phase],
    layout: &ScheduleLayout,
    schedule: &ScheduleInputs,
    prior: &[Reservation],
) -> Result<Vec<i64>, DomainError> {
    let edges = phases
        .iter()
        .map(|p| {
            p.dependencies
                .iter()
                .map(|d| phase_index(phases, d))
                .collect()
        })
        .collect::<Result<Vec<Vec<_>>, _>>()?;
    let order = topological(&edges)?;
    let mut starts = vec![None; phases.len()];
    let mut next = vec![None; phases.len()];
    let mut depth = 0;
    let mut attempts = 0;
    while depth < order.len() {
        let i = order[depth];
        let p = &phases[i];
        let fixed = layout
            .fixed_starts
            .iter()
            .find(|f| f.phase == p.key)
            .map(|f| seconds(&f.start))
            .transpose()?;
        let mut lo = fixed.unwrap_or(p.earliest);
        let mut hi = fixed.unwrap_or(p.latest);
        for &d in &edges[i] {
            let start = starts[d]
                .ok_or_else(|| error(ErrorCode::MissingReference, "/placement/dependency"))?;
            lo = lo.max(add(start, phases[d].duration)?);
        }
        let route = layout
            .routes
            .iter()
            .find(|r| r.phase == p.key)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/route"))?;
        if p.spec.active_minutes > 0
            && let Some((a, b)) = attachment_envelope(&route.session, schedule)?
        {
            lo = lo.max(a);
            hi = hi.min(add(b, -p.duration)?);
        }
        let candidate = next[depth].unwrap_or(lo).max(lo);
        if candidate > hi {
            next[depth] = None;
            starts[i] = None;
            if depth == 0 {
                return Err(error(ErrorCode::DomainConflict, "/placement/no_schedule"));
            }
            depth -= 1;
            let previous = order[depth];
            let old = starts[previous]
                .take()
                .ok_or_else(|| error(ErrorCode::InvalidValue, "/placement"))?;
            next[depth] = Some(add(old, 1)?);
            continue;
        }
        if attempts == PLACEMENT_LIMIT {
            return Err(error(
                ErrorCode::UnsupportedCapability,
                "/placement/100000_attempts",
            ));
        }
        attempts += 1;
        let mut reservations = prior.to_vec();
        for (j, start) in starts.iter().enumerate() {
            if let Some(at) = start {
                reservations.push(reservation(&phases[j], *at)?);
            }
        }
        if let Some(skip) = collision(p, candidate, &reservations)? {
            next[depth] = Some(skip.max(add(candidate, 1)?));
            continue;
        }
        starts[i] = Some(candidate);
        depth += 1;
        if depth < order.len() {
            next[depth] = None;
        }
    }
    starts
        .into_iter()
        .map(|s| s.ok_or_else(|| error(ErrorCode::InvalidValue, "/placement")))
        .collect()
}

pub(crate) fn derive_schedule(
    plan: &WeekPlan,
    context: &PlanningContext,
    evidence: &FoodEvidence,
    identities: &FoodIdentities,
    schedule: &ScheduleInputs,
) -> Result<(Vec<PrepSession>, Vec<TaskBinding>), FoodFailure> {
    validate_schedule_shape(schedule)?;
    let phases = model(plan, context, evidence)?;
    let keys = layout_keys(plan, &phases, &schedule.layout)?;
    let mapping = bindings(&phases, &schedule.layout, identities, &keys)?;
    let prior = occupied(
        plan,
        schedule,
        horizon(&phases, &schedule.layout)?,
        &identities
            .sessions
            .iter()
            .map(|(_, id)| id.clone())
            .collect::<Vec<_>>(),
    )?;
    let starts = place(&phases, &schedule.layout, schedule, &prior)?;
    let mut sessions = Vec::new();
    for key in keys {
        let id = identities
            .sessions
            .iter()
            .find(|(k, _)| k == &key)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/identities"))?
            .1
            .clone();
        let members = session_members(&key, &phases, &schedule.layout);
        let zone = &phases[members[0]].zone;
        let mut tasks = Vec::new();
        let mut slots = Vec::new();
        let mut ingredients = BTreeSet::new();
        let mut needed = i64::MAX;
        let mut lo = i64::MAX;
        let mut hi = i64::MIN;
        for (rank, i) in members.into_iter().enumerate() {
            let p = &phases[i];
            let start = starts[i];
            let end = add(start, p.duration)?;
            lo = lo.min(start);
            hi = hi.max(end);
            let elapsed = TimeInterval {
                start: moment(start, &p.zone)?,
                end: moment(end, &p.zone)?,
            };
            let active_intervals = if p.spec.active_minutes > 0 {
                vec![elapsed.clone()]
            } else {
                vec![]
            };
            tasks.push(ScheduledPrepTask {
                task_id: rank as u16 + 1,
                recipe: p.key.occurrence.recipe.clone(),
                prep_id: p.key.prep_id,
                depends_on: BoundedVec(task_dependencies(p, &mapping)?),
                active_intervals: BoundedVec(active_intervals),
                elapsed,
            });
            if !slots.contains(&p.key.occurrence.slot) {
                slots.push(p.key.occurrence.slot.clone());
            }
            ingredients.extend(p.ingredient_ids.iter().cloned());
            if !p.ingredient_ids.is_empty() {
                needed = needed.min(start);
            }
        }
        slots.sort_by(slot_cmp);
        if needed == i64::MAX {
            needed = lo;
        }
        sessions.push(PrepSession {
            session_id: id,
            week_id: plan.week_id.clone(),
            linked_slots: BoundedVec(slots),
            attached_to: Nullable(
                schedule
                    .layout
                    .attachments
                    .iter()
                    .find(|a| a.session == key)
                    .map(|a| a.target_session.clone()),
            ),
            start: moment(lo, zone)?,
            end: moment(hi, zone)?,
            ingredient_needed_at: moment(needed, zone)?,
            tasks: BoundedVec(tasks),
            ingredient_ids: BoundedVec(ingredients.into_iter().collect()),
        });
    }
    sessions.sort_by(|a, b| a.session_id.cmp(&b.session_id));
    let checked = validate_schedule(plan, context, evidence, identities, schedule, &sessions)?;
    ensure(
        checked == mapping,
        ErrorCode::IdentityMismatch,
        "/task_bindings",
    )?;
    Ok((sessions, mapping))
}

/// Current continuation checks exact saved task times against current occupancy;
/// it never reopens the historical placement windows or changes layout.
pub(crate) fn validate_saved_occupancy(
    basis: &SavedFoodBasis,
    sessions: &[PrepSession],
    snapshot: &OccupancySnapshot,
) -> Result<(), FoodFailure> {
    let phases = model(&basis.plan, &basis.context, &basis.evidence)?;
    let keys = layout_keys(&basis.plan, &phases, &basis.schedule.layout)?;
    let mapping = bindings(&phases, &basis.schedule.layout, &basis.identities, &keys)?;
    let all: Vec<_> = sessions.iter().flat_map(|s| s.tasks.iter()).collect();
    let expected = if all.is_empty() {
        None
    } else {
        Some((
            all.iter()
                .map(|t| seconds(&t.elapsed.start))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .min()
                .ok_or_else(|| error(ErrorCode::InvalidValue, "/horizon"))?,
            all.iter()
                .map(|t| seconds(&t.elapsed.end))
                .collect::<Result<Vec<_>, _>>()?
                .into_iter()
                .max()
                .ok_or_else(|| error(ErrorCode::InvalidValue, "/horizon"))?,
        ))
    };
    if let Some((lo, hi)) = expected {
        ensure(
            lo < hi,
            ErrorCode::UnsupportedCapability,
            "/horizon/zero_only",
        )?;
    }
    let mut layout = basis.schedule.layout.clone();
    for attachment in &mut layout.attachments {
        let owners: Vec<_> = snapshot
            .owners
            .iter()
            .filter(|o| {
                o.pin.week.week_id == attachment.target_owner.week.week_id
                    && o.sessions
                        .iter()
                        .any(|s| s.session_id == attachment.target_session)
            })
            .collect();
        ensure(
            owners.len() == 1,
            ErrorCode::ForeignReference,
            "/saved/attachment_identity",
        )?;
        attachment.target_owner = owners[0].pin.clone();
    }
    let schedule = ScheduleInputs {
        layout,
        occupied: snapshot.clone(),
    };
    let mut reservations = occupied(
        &basis.plan,
        &schedule,
        expected,
        &basis
            .identities
            .sessions
            .iter()
            .map(|(_, id)| id.clone())
            .collect::<Vec<_>>(),
    )?;
    for binding in &mapping {
        let phase = &phases[phase_index(&phases, &binding.phase)?];
        let task = sessions
            .iter()
            .find(|s| s.session_id == binding.task.session_id)
            .and_then(|s| s.tasks.iter().find(|t| t.task_id == binding.task.task_id))
            .ok_or_else(|| error(ErrorCode::ForeignReference, "/saved/task"))?;
        let start = seconds(&task.elapsed.start)?;
        ensure(
            collision(phase, start, &reservations)?.is_none(),
            ErrorCode::DomainConflict,
            "/saved/occupancy",
        )?;
        let route = schedule
            .layout
            .routes
            .iter()
            .find(|r| r.phase == binding.phase)
            .ok_or_else(|| error(ErrorCode::ForeignReference, "/route"))?;
        if phase.spec.active_minutes > 0
            && let Some((lo, hi)) = attachment_envelope(&route.session, &schedule)?
        {
            ensure(
                start >= lo && add(start, phase.duration)? <= hi,
                ErrorCode::DomainConflict,
                "/saved/attachment",
            )?;
        }
        reservations.push(reservation(phase, start)?);
    }
    Ok(())
}
pub(crate) fn validate_continuation_occupancy(
    saved: &SavedFoodBasis,
    derived: &DerivedPlan,
    current: &OccupancySnapshot,
) -> Result<(), FoodFailure> {
    validate_saved_occupancy(saved, &derived.prep_sessions, current)
}

#[cfg(test)]
mod search_tests {
    use super::*;
    // Private search controls, deliberately separate from admitted food tests.
    fn phase(id: u16, duration: i64, latest: i64) -> Phase {
        let slot = SlotId {
            week_id: WeekId(format!("wk_{:032x}", 1)),
            date: LocalDate("2026-10-05".into()),
            meal_type: MealType::Dinner,
        };
        let key = PhaseKey {
            occurrence: OccurrenceKey {
                slot,
                extra_id: None,
                recipe: RecipeRef {
                    recipe_id: RecipeId(format!("rc_{:032x}", 1)),
                    revision: Revision(1),
                    digest: RecipeDigest("a".repeat(64)),
                },
            },
            prep_id: id,
        };
        Phase {
            key,
            spec: PrepSpec {
                prep_id: id,
                title: Title("Synthetic search interval".into()),
                step_ids: BoundedVec(vec![id]),
                depends_on: BoundedVec(vec![]),
                active_minutes: 1,
                elapsed_minutes: 1,
                earliest_offset_seconds: 0,
                latest_offset_seconds: 0,
                ingredient_ids: BoundedVec(vec![]),
                can_attach_to_prior_session: false,
            },
            ingredient_ids: vec![],
            equipment: vec![],
            zone: ZoneId("UTC".into()),
            earliest: 0,
            latest,
            duration,
            deadline: 300_000,
            dependencies: vec![],
        }
    }
    fn schedule(phases: &[Phase]) -> ScheduleInputs {
        let key = SessionKey {
            anchor_slot: phases[0].key.occurrence.slot.clone(),
            role: SessionRole::Cooking,
            ordinal: 1,
        };
        ScheduleInputs {
            layout: ScheduleLayout {
                routes: phases
                    .iter()
                    .map(|p| PhaseRoute {
                        phase: p.key.clone(),
                        session: key.clone(),
                    })
                    .collect(),
                fixed_starts: vec![],
                attachments: vec![],
            },
            occupied: OccupancySnapshot {
                query: None,
                occupancy_epoch: Revision(1),
                owners: vec![],
                digest: PayloadDigest("0".repeat(64)),
            },
        }
    }
    #[test]
    fn food_search_exact_100000_attempts_returns_unsupported() {
        let phases = vec![phase(1, 60, 200_000), phase(2, 200_060, 0)];
        let schedule = schedule(&phases);
        let error = place(&phases, &schedule.layout, &schedule, &[]).unwrap_err();
        assert_eq!(error.code, ErrorCode::UnsupportedCapability);
        assert_eq!(error.issues[0].path, "/placement/100000_attempts");
    }
    #[test]
    fn food_search_allows_adjacency_and_zero_work() {
        let phases = vec![phase(1, 60, 0), phase(2, 60, 60)];
        let schedule = schedule(&phases);
        assert_eq!(
            place(&phases, &schedule.layout, &schedule, &[]).unwrap(),
            vec![0, 60]
        );
        let empty = ScheduleInputs {
            layout: ScheduleLayout {
                routes: vec![],
                fixed_starts: vec![],
                attachments: vec![],
            },
            occupied: schedule.occupied,
        };
        assert!(place(&[], &empty.layout, &empty, &[]).unwrap().is_empty());
    }
}
