use super::*;

use std::cmp::Ordering;
use std::collections::BTreeSet;

pub(crate) fn shape<T: ValidateShape>(v: &T) -> Result<(), DomainError> {
    v.validate_shape().map_err(|e| {
        err(
            e.first().map(|e| e.code).unwrap_or(ErrorCode::InvalidValue),
            e.first().map(|e| e.path.as_str()).unwrap_or("/native"),
        )
    })
}
pub(crate) fn ordered<T>(
    v: &[T],
    cmp: impl Fn(&T, &T) -> Ordering,
    path: &str,
) -> Result<(), DomainError> {
    // Identity rejection takes precedence over ordering, even when a duplicate
    // is separated by another key in a noncanonical input.
    let mut keys: Vec<_> = v.iter().collect();
    keys.sort_by(|a, b| cmp(a, b));
    require(
        !keys.windows(2).any(|p| cmp(p[0], p[1]) == Ordering::Equal),
        ErrorCode::InvalidValue,
        path,
    )?;
    for pair in v.windows(2) {
        match cmp(&pair[0], &pair[1]) {
            Ordering::Less => (),
            Ordering::Equal => return Err(err(ErrorCode::InvalidValue, path)),
            Ordering::Greater => return Err(err(ErrorCode::NoncanonicalValue, path)),
        }
    }
    Ok(())
}
fn ids(v: &[EvidenceId]) -> Result<(), DomainError> {
    require(!v.is_empty(), ErrorCode::MissingReference, "/evidence_ids")?;
    limit(v.len(), 256, "/evidence_ids")?;
    for i in v {
        shape(i)?;
    }
    ordered(v, Ord::cmp, "/evidence_ids")
}
fn pins(v: &[EvidencePin], max: usize) -> Result<(), DomainError> {
    limit(v.len(), max, "/evidence")?;
    ordered(v, |a, b| a.evidence_id.cmp(&b.evidence_id), "/evidence")?;
    for p in v {
        shape(&p.evidence_id)?;
        shape(&p.revision)?;
        shape(&p.digest)?;
    }
    Ok(())
}
fn occurrence_shape(v: &OccurrenceKey) -> Result<(), DomainError> {
    shape(&v.slot)?;
    shape(&v.recipe)?;
    if let Some(e) = &v.extra_id {
        shape(e)?;
    }
    Ok(())
}
fn phase_shape(v: &PhaseKey) -> Result<(), DomainError> {
    occurrence_shape(&v.occurrence)?;
    require(
        (1..=128).contains(&v.prep_id),
        ErrorCode::InvalidValue,
        "/prep_id",
    )
}
fn session_shape(v: &SessionKey) -> Result<(), DomainError> {
    shape(&v.anchor_slot)?;
    require(
        (1..=256).contains(&v.ordinal),
        ErrorCode::InvalidValue,
        "/ordinal",
    )
}
fn owner_pin(v: &PlanOwnerPin) -> Result<(), DomainError> {
    shape(&v.week)?;
    shape(&v.derived_digest)?;
    shape(&v.basis_digest)
}
pub(crate) fn validate_method(v: &MethodPhases) -> Result<(), DomainError> {
    shape(&v.recipe)?;
    ids(&v.evidence_ids)?;
    limit(v.prep_ids.len(), 128, "/prep_ids")?;
    ordered(&v.prep_ids, Ord::cmp, "/prep_ids")?;
    if let Some(portable) = &v.portable_cold {
        require(v.prep_ids.len() == 2 && portable.packing_prep_id != portable.service_prep_id
            && v.prep_ids.contains(&portable.packing_prep_id)
            && v.prep_ids.contains(&portable.service_prep_id),
            ErrorCode::InvalidValue, "/methods/portable_cold")?;
    }
    for id in &v.prep_ids {
        require((1..=128).contains(id), ErrorCode::InvalidValue, "/prep_id")?;
    }
    limit(v.direct_reuse_counts.len(), 128, "/direct_reuse_counts")?;
    ordered(
        &v.direct_reuse_counts,
        |a, b| a.component_id.cmp(&b.component_id),
        "/direct_reuse_counts",
    )?;
    require(
        v.kind != MethodKind::Production || v.direct_reuse_counts.is_empty(),
        ErrorCode::InvalidValue,
        "/direct_reuse_counts",
    )?;
    for c in &v.direct_reuse_counts {
        shape(&c.recipe)?;
        shape(&c.component_id)?;
        ids(&c.evidence_ids)?;
        require(
            c.recipe == v.recipe && c.planned_reheats <= 1,
            ErrorCode::InvalidValue,
            "/direct_reuse_counts",
        )?;
        require(
            c.evidence_ids.iter().all(|i| v.evidence_ids.contains(i)),
            ErrorCode::MissingReference,
            "/direct_reuse_counts/evidence_ids",
        )?;
    }
    Ok(())
}
fn resources(v: &PhaseResources) -> Result<(), DomainError> {
    shape(&v.recipe)?;
    require(
        (1..=128).contains(&v.prep_id),
        ErrorCode::InvalidValue,
        "/prep_id",
    )?;
    ids(&v.evidence_ids)?;
    equipment(&v.equipment)
}
fn equipment(v: &[EquipmentCode]) -> Result<(), DomainError> {
    limit(v.len(), 9, "/equipment")?;
    ordered(
        v,
        |a, b| {
            serde_json::json!(a)
                .as_str()
                .cmp(&serde_json::json!(b).as_str())
        },
        "/equipment",
    )
}
pub(crate) fn validate_evidence_shape(v: &FoodEvidence) -> Result<(), DomainError> {
    shape(&v.catalogue_revision)?;
    shape(&v.policy_version)?;
    require(
        v.policy_version == Revision(1),
        ErrorCode::UnsupportedVersion,
        "/policy_version",
    )?;
    limit(v.recipes.len(), 512, "/recipes")?;
    limit(v.classes.len(), 512, "/classes")?;
    limit(v.methods.len(), 1024, "/methods")?;
    limit(v.resources.len(), 4096, "/resources")?;
    limit(
        v.essential_ingredient_ids.len(),
        4096,
        "/essential_ingredient_ids",
    )?;
    ordered(
        &v.recipes,
        |a, b| recipe_cmp(&a.recipe, &b.recipe),
        "/recipes",
    )?;
    ordered(
        &v.classes,
        |a, b| recipe_cmp(&a.recipe, &b.recipe),
        "/classes",
    )?;
    require(
        v.recipes.len() == v.classes.len(),
        ErrorCode::MissingReference,
        "/classes",
    )?;
    for (b, c) in v.recipes.iter().zip(&v.classes) {
        pins(&b.admitted, 256)?;
        ordered(
            &c.components,
            |a, b| a.component_id.cmp(&b.component_id),
            "/classes/components",
        )?;
        EvidenceBasis::new(
            b.recipe.clone(),
            b.admitted.clone(),
            b.verified_asset_digests.clone(),
        )?;
        ComponentClassView::new(c.recipe.clone(), c.components.clone())?;
        require(
            b.recipe == c.recipe,
            ErrorCode::IdentityMismatch,
            "/classes/recipe",
        )?;
    }
    require(
        v.recipes.windows(2).all(|p| {
            p[0].recipe.recipe_id != p[1].recipe.recipe_id
                || p[0].recipe.revision != p[1].recipe.revision
        }),
        ErrorCode::IdentityMismatch,
        "/recipes",
    )?;
    ordered(
        &v.methods,
        |a, b| {
            recipe_cmp(&a.recipe, &b.recipe)
                .then((a.kind == MethodKind::Reuse).cmp(&(b.kind == MethodKind::Reuse)))
        },
        "/methods",
    )?;
    ordered(
        &v.resources,
        |a, b| recipe_cmp(&a.recipe, &b.recipe).then(a.prep_id.cmp(&b.prep_id)),
        "/resources",
    )?;
    for m in &v.methods {
        validate_method(m)?;
        let b = v
            .recipes
            .iter()
            .find(|b| b.recipe == m.recipe)
            .ok_or_else(|| err(ErrorCode::MissingReference, "/methods/recipe"))?;
        require(
            m.evidence_ids
                .iter()
                .all(|id| b.admitted.iter().any(|p| &p.evidence_id == id)),
            ErrorCode::MissingReference,
            "/methods/evidence_ids",
        )?;
    }
    for r in &v.resources {
        resources(r)?;
        let b = v
            .recipes
            .iter()
            .find(|b| b.recipe == r.recipe)
            .ok_or_else(|| err(ErrorCode::MissingReference, "/resources/recipe"))?;
        require(
            r.evidence_ids
                .iter()
                .all(|id| b.admitted.iter().any(|p| &p.evidence_id == id)),
            ErrorCode::MissingReference,
            "/resources/evidence_ids",
        )?;
    }
    ordered(
        &v.essential_ingredient_ids,
        Ord::cmp,
        "/essential_ingredient_ids",
    )?;
    for i in &v.essential_ingredient_ids {
        shape(i)?;
    }
    Ok(())
}
pub(crate) fn validate_identities_shape(v: &FoodIdentities) -> Result<(), DomainError> {
    shape(&v.revision)?;
    limit(v.allocations.len(), 4096, "/allocations")?;
    limit(v.sessions.len(), 256, "/sessions")?;
    limit(v.demand_lines.len(), 4096, "/demand_lines")?;
    ordered(
        &v.allocations,
        |a, b| allocation_cmp(&a.0, &b.0),
        "/allocations",
    )?;
    ordered(&v.sessions, |a, b| session_cmp(&a.0, &b.0), "/sessions")?;
    ordered(
        &v.demand_lines,
        |a, b| demand_cmp(&a.0, &b.0),
        "/demand_lines",
    )?;
    let mut seen = BTreeSet::new();
    for (k, id) in &v.allocations {
        shape(&k.batch_id)?;
        shape(&k.component_id)?;
        shape(&k.target_slot)?;
        shape(&k.person)?;
        shape(id)?;
        require(seen.insert(id), ErrorCode::InvalidValue, "/allocations/id")?;
    }
    let mut seen = BTreeSet::new();
    for (k, id) in &v.sessions {
        session_shape(k)?;
        shape(id)?;
        require(seen.insert(id), ErrorCode::InvalidValue, "/sessions/id")?;
    }
    let mut seen = BTreeSet::new();
    for (k, id) in &v.demand_lines {
        shape(&k.ingredient_id)?;
        shape(&k.unit)?;
        shape(&k.form)?;
        shape(id)?;
        require(seen.insert(id), ErrorCode::InvalidValue, "/demand_lines/id")?;
    }
    Ok(())
}
pub(crate) fn validate_owner_shape(v: &OccupiedOwner) -> Result<(), DomainError> {
    owner_pin(&v.pin)?;
    limit(v.sessions.len(), 256, "/sessions")?;
    limit(v.task_resources.len(), 4096, "/task_resources")?;
    ordered(
        &v.sessions,
        |a, b| a.session_id.cmp(&b.session_id),
        "/sessions",
    )?;
    ordered(
        &v.task_resources,
        |a, b| task_cmp(&a.task, &b.task),
        "/task_resources",
    )?;
    let mut tasks = Vec::new();
    for s in &v.sessions {
        shape(s)?;
        require(
            s.week_id == v.pin.week.week_id,
            ErrorCode::IdentityMismatch,
            "/sessions/week_id",
        )?;
        for t in s.tasks.iter() {
            tasks.push(PrepTaskRef {
                session_id: s.session_id.clone(),
                task_id: t.task_id,
            });
        }
    }
    tasks.sort_by(task_cmp);
    require(
        tasks.len() == v.task_resources.len(),
        ErrorCode::MissingReference,
        "/task_resources",
    )?;
    for (t, r) in tasks.iter().zip(&v.task_resources) {
        require(
            t == &r.task,
            ErrorCode::MissingReference,
            "/task_resources/task",
        )?;
        equipment(&r.equipment)?;
        ids(&r.evidence_ids)?;
    }
    Ok(())
}
pub(crate) fn validate_occupancy_shape(v: &OccupancySnapshot) -> Result<(), DomainError> {
    shape(&v.occupancy_epoch)?;
    limit(v.owners.len(), 64, "/owners")?;
    ordered(&v.owners, |a, b| owner_cmp(&a.pin, &b.pin), "/owners")?;
    if let Some(q) = &v.query {
        shape(&q.horizon)?;
        if let Some(pin) = &q.excluded_subject {
            owner_pin(pin)?;
            require(
                v.owners.iter().all(|o| &o.pin != pin),
                ErrorCode::IdentityMismatch,
                "/excluded_subject",
            )?;
        }
    } else {
        require(v.owners.is_empty(), ErrorCode::InvalidValue, "/query")?;
    }
    let mut count = 0;
    let mut tasks = 0;
    let mut sessions = BTreeSet::new();
    let mut owners = BTreeSet::new();
    for o in &v.owners {
        validate_owner_shape(o)?;
        require(
            owners.insert((&o.pin.week.week_id, o.pin.week.revision)),
            ErrorCode::InvalidValue,
            "/owners",
        )?;
        require(
            occupied_owner_digest(o)? == o.projection_digest,
            ErrorCode::DigestMismatch,
            "/projection_digest",
        )?;
        count += o.sessions.len();
        tasks += o.task_resources.len();
        for s in &o.sessions {
            require(
                sessions.insert(&s.session_id),
                ErrorCode::InvalidValue,
                "/sessions",
            )?;
        }
    }
    limit(count, 4096, "/sessions")?;
    limit(tasks, 16384, "/tasks")
}
pub(crate) fn validate_schedule_shape(v: &ScheduleInputs) -> Result<(), DomainError> {
    let l = &v.layout;
    limit(l.routes.len(), 4096, "/routes")?;
    limit(l.fixed_starts.len(), 4096, "/fixed_starts")?;
    limit(l.attachments.len(), 256, "/attachments")?;
    ordered(&l.routes, |a, b| phase_cmp(&a.phase, &b.phase), "/routes")?;
    ordered(
        &l.fixed_starts,
        |a, b| phase_cmp(&a.phase, &b.phase),
        "/fixed_starts",
    )?;
    ordered(
        &l.attachments,
        |a, b| session_cmp(&a.session, &b.session),
        "/attachments",
    )?;
    for r in &l.routes {
        phase_shape(&r.phase)?;
        session_shape(&r.session)?;
    }
    for f in &l.fixed_starts {
        phase_shape(&f.phase)?;
        shape(&f.start)?;
    }
    for a in &l.attachments {
        session_shape(&a.session)?;
        owner_pin(&a.target_owner)?;
        shape(&a.target_session)?;
    }
    require(
        occupancy_snapshot_digest(&v.occupied)? == v.occupied.digest,
        ErrorCode::DigestMismatch,
        "/occupied/digest",
    )
}
fn use_shape(v: &UseKey) -> Result<(), DomainError> {
    match v {
        UseKey::Preparation { phase } => phase_shape(phase),
        UseKey::Consumption {
            occurrence,
            component_id,
            person,
        } => {
            occurrence_shape(occurrence)?;
            shape(component_id)?;
            shape(person)
        }
    }
}
fn scope_shape(v: &ProductUseScope) -> Result<(), DomainError> {
    shape(&v.line_id)?;
    shape(&v.product)?;
    use_shape(&v.use_key)
}
pub(crate) fn validate_products_shape(v: &ProductUseSnapshot) -> Result<(), DomainError> {
    shape(&v.revision)?;
    if let Some(t) = &v.valid_until {
        shape(t)?;
    }
    pins(&v.evidence, 4096)?;
    limit(v.bindings.len(), 16384, "/bindings")?;
    limit(v.facts.len(), 16384, "/facts")?;
    ordered(
        &v.bindings,
        |a, b| scope_cmp(&a.scope, &b.scope),
        "/bindings",
    )?;
    ordered(&v.facts, |a, b| scope_cmp(&a.scope, &b.scope), "/facts")?;
    require(
        v.bindings.len() == v.facts.len(),
        ErrorCode::MissingReference,
        "/facts",
    )?;
    let check_ids = |list: &[EvidenceId]| -> Result<(), DomainError> {
        ids(list)?;
        require(
            list.iter()
                .all(|id| v.evidence.iter().any(|p| &p.evidence_id == id)),
            ErrorCode::MissingReference,
            "/evidence_ids",
        )
    };
    let mut count = 0;
    for (b, f) in v.bindings.iter().zip(&v.facts) {
        scope_shape(&b.scope)?;
        require(
            b.scope == f.scope,
            ErrorCode::IdentityMismatch,
            "/facts/scope",
        )?;
        check_ids(&b.evidence_ids)?;
        if let Some(d) = &f.use_by {
            shape(&d.use_by)?;
            check_ids(&d.evidence_ids)?;
        }
        limit(f.shorter.len(), 8, "/shorter")?;
        count += f.shorter.len();
        let bytes = f
            .shorter
            .iter()
            .map(fact_bytes)
            .collect::<Result<Vec<_>, _>>()?;
        ordered(&bytes, Ord::cmp, "/shorter")?;
        for d in &f.shorter {
            check_ids(&d.evidence_ids)?;
            match &d.rule {
                DeadlineRule::Absolute { latest_use, .. } => shape(latest_use)?,
                DeadlineRule::AfterOpening {
                    anchor,
                    maximum_elapsed_seconds,
                    ..
                } => {
                    require(
                        *maximum_elapsed_seconds > 0,
                        ErrorCode::InvalidValue,
                        "/maximum_elapsed_seconds",
                    )?;
                    match anchor {
                        OpeningAnchor::Observed { at, evidence_id } => {
                            shape(at)?;
                            check_ids(std::slice::from_ref(evidence_id))?;
                        }
                        OpeningAnchor::PlannedPhaseStart { phase } => phase_shape(phase)?,
                        OpeningAnchor::Unknown => (),
                    }
                }
            }
        }
    }
    limit(count, 65536, "/shorter")
}
pub(crate) fn validate_feasibility_shape(v: &FeasibilityInputs) -> Result<(), DomainError> {
    shape(&v.revision)?;
    shape(&v.evaluated_at)?;
    require(
        digest_product_uses(&v.products)? == v.products.digest,
        ErrorCode::DigestMismatch,
        "/products/digest",
    )?;
    if let Some(t) = &v.products.valid_until {
        require(
            v.evaluated_at < t.clone(),
            ErrorCode::ContextStale,
            "/products/valid_until",
        )?;
    }
    limit(v.confirmations.len(), 4096, "/confirmations")?;
    ordered(
        &v.confirmations,
        |a, b| a.line_id.cmp(&b.line_id),
        "/confirmations",
    )?;
    for c in &v.confirmations {
        shape(c)?;
        require(
            c.confirmed_at <= v.evaluated_at,
            ErrorCode::InvalidValue,
            "/confirmed_at",
        )?;
    }
    match &v.delivery {
        DeliveryInput::Unselected => (),
        DeliveryInput::Proposed {
            start,
            end,
            revision,
        } => {
            shape(start)?;
            shape(end)?;
            shape(revision)?;
            require(
                start.instant <= end.instant,
                ErrorCode::InvalidValue,
                "/delivery",
            )?;
        }
        DeliveryInput::Confirmed { order } => shape(order)?,
    }
    Ok(())
}
pub(crate) fn validate_report_shape(v: &FeasibilityReport) -> Result<(), DomainError> {
    shape(&v.derived_digest)?;
    shape(&v.inputs_digest)?;
    limit(v.uses.len(), 16384, "/uses")?;
    ordered(
        &v.uses,
        |a, b| {
            a.line_id
                .cmp(&b.line_id)
                .then_with(|| use_cmp(&a.use_key, &b.use_key))
        },
        "/uses",
    )?;
    for u in &v.uses {
        shape(&u.line_id)?;
        use_shape(&u.use_key)?;
        limit(u.issues.len(), 5, "/issues")?;
        ordered(&u.issues, |a, b| (*a as u8).cmp(&(*b as u8)), "/issues")?;
        let a = if u.issues.iter().any(|i| {
            matches!(
                i,
                UseIssue::OpeningChronologyConflict
                    | UseIssue::UseByConflict
                    | UseIssue::ShorterDeadlineConflict
            )
        }) {
            ProductDateAssessment::ConflictingKnownLimit
        } else if !u.issues.is_empty() {
            ProductDateAssessment::UnresolvedDateEvidence
        } else {
            ProductDateAssessment::WithinKnownLimit
        };
        require(a == u.assessment, ErrorCode::InvalidValue, "/assessment")?;
    }
    details(&v.other_checks)?;
    summary_shape(&v.warnings, &v.blocking, &v.uses, &v.other_checks)
}
fn details(v: &[Conflict]) -> Result<(), DomainError> {
    limit(v.len(), 100, "/other_checks")?;
    for c in v {
        shape(c)?;
    }
    let bytes = v
        .iter()
        .map(canonical_bytes)
        .collect::<Result<Vec<_>, _>>()?;
    ordered(&bytes, Ord::cmp, "/other_checks")
}
fn summary_shape(
    warnings: &[FoodSummary],
    blocking: &[FoodSummary],
    uses: &[UseCheck],
    checks: &[Conflict],
) -> Result<(), DomainError> {
    limit(warnings.len() + blocking.len(), 3, "/summaries")?;
    let unknown = uses.iter().any(|u| {
        u.issues.iter().any(|i| {
            matches!(
                i,
                UseIssue::UnknownProductDate | UseIssue::UnknownOpeningAnchor
            )
        })
    });
    let pending = checks.iter().any(|c| !c.blocking);
    let blocked = checks.iter().any(|c| c.blocking)
        || uses
            .iter()
            .any(|u| u.assessment == ProductDateAssessment::ConflictingKnownLimit);
    let expected: Vec<_> = [
        (unknown, FoodSummaryKind::UnknownUseEvidence),
        (pending, FoodSummaryKind::OtherPendingChecks),
    ]
    .into_iter()
    .filter_map(|(x, k)| x.then_some(k))
    .collect();
    require(
        warnings.iter().map(|s| s.kind).collect::<Vec<_>>() == expected,
        ErrorCode::InvalidValue,
        "/warnings",
    )?;
    require(
        blocking.len() == usize::from(blocked),
        ErrorCode::InvalidValue,
        "/blocking",
    )?;
    let mut week = None;
    for s in warnings.iter().chain(blocking) {
        shape(&s.conflict)?;
        let c = &s.conflict;
        let b = s.kind == FoodSummaryKind::BlockingChecks;
        require(
            c.week_ids.len() == 1
                && c.slot_ids.is_empty()
                && c.recipe_refs.is_empty()
                && c.evidence_ids.is_empty()
                && c.blocking == b
                && c.code
                    == if b {
                        ErrorCode::DomainConflict
                    } else {
                        ErrorCode::UnsupportedCapability
                    },
            ErrorCode::InvalidValue,
            "/summary",
        )?;
        if let Some(w) = week {
            require(
                w == &c.week_ids[0],
                ErrorCode::IdentityMismatch,
                "/summary/week_id",
            )?;
        }
        week = Some(&c.week_ids[0]);
    }
    if blocked {
        require(
            blocking[0].kind == FoodSummaryKind::BlockingChecks,
            ErrorCode::InvalidValue,
            "/blocking/kind",
        )?;
    }
    Ok(())
}
pub(crate) fn validate_saved_shape(v: &SavedFoodBasis) -> Result<(), DomainError> {
    shape(&v.plan)?;
    shape(&v.context)?;
    require(
        food_evidence_digest(&v.evidence)? == v.evidence.catalogue_digest,
        ErrorCode::DigestMismatch,
        "/evidence",
    )?;
    require(
        food_identities_digest(&v.identities)? == v.identities.digest,
        ErrorCode::DigestMismatch,
        "/identities",
    )?;
    validate_schedule_shape(&v.schedule)
}
pub(crate) fn validate_live_requirements(v: &LiveRequirements) -> Result<(), DomainError> {
    shape(&v.revision)?;
    limit(v.meals.len(), 21, "/meals")?;
    limit(v.unconfirmed.len(), 1344, "/unconfirmed")?;
    ordered(&v.meals, |a, b| slot_cmp(&a.slot, &b.slot), "/meals")?;
    ordered(
        &v.unconfirmed,
        |a, b| slot_cmp(&a.0, &b.0).then_with(|| person_cmp(&a.1, &b.1)),
        "/unconfirmed",
    )?;
    for m in &v.meals {
        shape(m)?;
    }
    for (s, p) in &v.unconfirmed {
        shape(s)?;
        shape(p)?;
        require(
            v.meals
                .iter()
                .any(|m| &m.slot == s && m.attendees.iter().any(|a| &a.person == p)),
            ErrorCode::ForeignReference,
            "/unconfirmed",
        )?;
    }
    Ok(())
}
pub(crate) fn live_recipe_ref(v: &LiveRecipeState) -> &RecipeRef {
    match v {
        LiveRecipeState::Admitted { basis, .. } => &basis.recipe,
        LiveRecipeState::Withdrawn { recipe, .. } => recipe,
    }
}
pub(crate) fn validate_live_evidence(v: &RelevantEvidenceSnapshot) -> Result<(), DomainError> {
    shape(&v.revision)?;
    limit(v.recipes.len(), 512, "/recipes")?;
    ordered(
        &v.recipes,
        |a, b| recipe_cmp(live_recipe_ref(a), live_recipe_ref(b)),
        "/recipes",
    )?;
    // The full canonical key includes the digest, but one recipe revision
    // cannot acquire two contradictory digests by changing its live state.
    require(
        v.recipes.windows(2).all(|pair| {
            let a = live_recipe_ref(&pair[0]);
            let b = live_recipe_ref(&pair[1]);
            a.recipe_id != b.recipe_id || a.revision != b.revision
        }),
        ErrorCode::IdentityMismatch,
        "/recipes",
    )?;
    for r in &v.recipes {
        match r {
            LiveRecipeState::Withdrawn { recipe, evidence } => {
                shape(recipe)?;
                require(
                    !evidence.is_empty(),
                    ErrorCode::MissingReference,
                    "/evidence",
                )?;
                pins(evidence, 256)?;
            }
            LiveRecipeState::Admitted {
                basis,
                classes,
                methods,
                resources,
            } => {
                let v = FoodEvidence {
                    catalogue_revision: v.revision,
                    catalogue_digest: PayloadDigest("0".repeat(64)),
                    policy_version: Revision(1),
                    recipes: vec![basis.clone()],
                    classes: vec![classes.clone()],
                    methods: methods.clone(),
                    resources: resources.clone(),
                    essential_ingredient_ids: vec![],
                };
                limit(methods.len(), 2, "/methods")?;
                limit(resources.len(), 128, "/resources")?;
                validate_evidence_shape(&v)?;
            }
        }
    }
    Ok(())
}
pub(crate) fn validate_policy(v: &LiveFoodPolicy) -> Result<(), DomainError> {
    shape(&v.version)?;
    require(
        v.version == Revision(1),
        ErrorCode::UnsupportedVersion,
        "/policy/version",
    )?;
    require(
        v.general_seconds > 0
            && v.general_seconds <= 172800
            && v.rice_seconds > 0
            && v.rice_seconds <= 86400.min(v.general_seconds)
            && v.maximum_reheats <= 1,
        ErrorCode::InvalidValue,
        "/policy",
    )
}
pub(crate) fn validate_continuation_shape(v: &ContinuationInputs) -> Result<(), DomainError> {
    shape(&v.revision)?;
    require(v.sources.len() == 7, ErrorCode::InvalidValue, "/sources")?;
    require(
        v.sources
            .iter()
            .enumerate()
            .all(|(i, p)| !v.sources[..i].iter().any(|q| q.source == p.source)),
        ErrorCode::InvalidValue,
        "/sources",
    )?;
    let expected = [
        LiveSource::Essentials,
        LiveSource::FoodPolicy,
        LiveSource::Occupancy,
        LiveSource::Order,
        LiveSource::ProductEvidence,
        LiveSource::RecipeEvidence,
        LiveSource::Requirements,
    ];
    for (p, e) in v.sources.iter().zip(expected) {
        require(p.source == e, ErrorCode::NoncanonicalValue, "/sources")?;
        shape(&p.revision)?;
        shape(&p.digest)?;
        if let Some(t) = &p.valid_until {
            shape(t)?;
            require(
                v.feasibility.evaluated_at < t.clone(),
                ErrorCode::ContextStale,
                "/sources/valid_until",
            )?;
        }
    }
    require(
        live_requirements_digest(&v.requirements)? == v.requirements.digest,
        ErrorCode::DigestMismatch,
        "/requirements",
    )?;
    require(
        relevant_evidence_digest(&v.evidence)? == v.evidence.digest,
        ErrorCode::DigestMismatch,
        "/evidence",
    )?;
    require(
        live_food_policy_digest(&v.policy)? == v.policy.digest,
        ErrorCode::DigestMismatch,
        "/policy",
    )?;
    require(
        occupancy_snapshot_digest(&v.occupied)? == v.occupied.digest,
        ErrorCode::DigestMismatch,
        "/occupied",
    )?;
    require(
        digest_feasibility_inputs(&v.feasibility)? == v.feasibility.digest,
        ErrorCode::DigestMismatch,
        "/feasibility",
    )?;
    let checks = [
        (1, v.policy.version, &v.policy.digest),
        (2, v.occupied.occupancy_epoch, &v.occupied.digest),
        (
            4,
            v.feasibility.products.revision,
            &v.feasibility.products.digest,
        ),
        (5, v.evidence.revision, &v.evidence.digest),
        (6, v.requirements.revision, &v.requirements.digest),
    ];
    for (i, rev, d) in checks {
        require(
            v.sources[i].revision == rev && &v.sources[i].digest == d,
            ErrorCode::DigestMismatch,
            "/sources",
        )?;
    }
    require(
        v.sources[4].valid_until == v.feasibility.products.valid_until,
        ErrorCode::IdentityMismatch,
        "/products/valid_until",
    )?;
    require(
        essentials_source_digest(&v.sources[0], &v.feasibility.confirmations)?
            == v.sources[0].digest,
        ErrorCode::DigestMismatch,
        "/essentials",
    )
}
pub(crate) fn validate_continuation_report(v: &ContinuationReport) -> Result<(), DomainError> {
    shape(&v.order_id)?;
    shape(&v.approved_digest)?;
    shape(&v.historical_basis_digest)?;
    shape(&v.current_inputs_digest)?;
    require(
        digest_feasibility_report(&v.feasibility)? == v.feasibility.digest,
        ErrorCode::DigestMismatch,
        "/feasibility",
    )?;
    details(&v.other_checks)?;
    let mut combined = v.feasibility.other_checks.clone();
    combined.extend(v.other_checks.clone());
    combined.sort_by_key(|x| canonical_bytes(x).unwrap_or_default());
    combined.dedup();
    details(&combined)?;
    summary_shape(&v.warnings, &v.blocking, &v.feasibility.uses, &combined)
}
