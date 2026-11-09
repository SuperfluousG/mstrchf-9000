use super::*;

pub fn assess_feasibility(
    derived: &DerivedPlan,
    saved: &SavedFoodBasis,
    inputs: &FeasibilityInputs,
) -> Result<FeasibilityReport, DomainError> {
    let checked = verify_core(derived, saved)?;
    assess_verified_feasibility(&checked, inputs)
}

pub(crate) fn assess_verified_feasibility(
    checked: &VerifiedCore<'_>,
    inputs: &FeasibilityInputs,
) -> Result<FeasibilityReport, DomainError> {
    let derived = checked.derived();
    let graph = super::uses::reconstruct_verified(checked)?;
    let bindings = checked.bindings();
    validate_inputs(inputs)?;
    require(
        digest_product_uses(&inputs.products)? == inputs.products.digest,
        ErrorCode::DigestMismatch,
        "/products/digest",
    )?;
    require(
        digest_feasibility_inputs(inputs)? == inputs.digest,
        ErrorCode::DigestMismatch,
        "/feasibility/digest",
    )?;
    if let Some(expiry) = &inputs.products.valid_until {
        require(
            inputs.evaluated_at < *expiry,
            ErrorCode::ContextStale,
            "/products/valid_until",
        )?;
    }
    for binding in &inputs.products.bindings {
        require(
            graph
                .uses
                .iter()
                .any(|u| u.line_id == binding.scope.line_id && u.use_key == binding.scope.use_key),
            ErrorCode::ForeignReference,
            "/products/scope",
        )?;
    }
    let mut checks = Vec::with_capacity(graph.uses.len());
    for usage in &graph.uses {
        let mut flags = [false; 5];
        let mut applicable = false;
        for facts in inputs
            .products
            .facts
            .iter()
            .filter(|f| f.scope.line_id == usage.line_id && f.scope.use_key == usage.use_key)
        {
            applicable = true;
            if let Some(date) = &facts.use_by {
                if usage.at.local_date > date.use_by {
                    flags[3] = true;
                }
            } else {
                flags[0] = true;
            }
            for fact in &facts.shorter {
                match &fact.rule {
                    DeadlineRule::Absolute {
                        latest_use,
                        boundary,
                    } => {
                        if exceeds(
                            usage.at.instant.timestamp_seconds(),
                            latest_use.instant.timestamp_seconds(),
                            *boundary,
                        ) {
                            flags[4] = true;
                        }
                    }
                    DeadlineRule::AfterOpening {
                        anchor,
                        opening_boundary,
                        maximum_elapsed_seconds,
                        boundary,
                    } => {
                        let at = match anchor {
                            OpeningAnchor::Unknown => {
                                flags[1] = true;
                                None
                            }
                            OpeningAnchor::Observed { at, .. } => {
                                require(
                                    at.instant <= inputs.evaluated_at,
                                    ErrorCode::InvalidValue,
                                    "/products/anchor/observed_at",
                                )?;
                                Some(at)
                            }
                            OpeningAnchor::PlannedPhaseStart { phase } => {
                                Some(super::uses::task_at(derived, bindings, phase)?)
                            }
                        };
                        if let Some(at) = at {
                            let opening = at.instant.timestamp_seconds();
                            let use_at = usage.at.instant.timestamp_seconds();
                            if use_at < opening
                                || (use_at == opening
                                    && *opening_boundary == DeadlineBoundary::Exclusive)
                            {
                                flags[2] = true;
                            }
                            let latest = opening
                                .checked_add(i64::from(*maximum_elapsed_seconds))
                                .ok_or_else(|| {
                                err(ErrorCode::ArithmeticOverflow, "/products/anchor/deadline")
                            })?;
                            if exceeds(use_at, latest, *boundary) {
                                flags[4] = true;
                            }
                        }
                    }
                }
            }
        }
        if !applicable {
            flags[0] = true;
        }
        checks.push(UseCheck {
            line_id: usage.line_id.clone(),
            use_key: usage.use_key.clone(),
            assessment: if flags[2..].iter().any(|f| *f) {
                ProductDateAssessment::ConflictingKnownLimit
            } else if flags[..2].iter().any(|f| *f) {
                ProductDateAssessment::UnresolvedDateEvidence
            } else {
                ProductDateAssessment::WithinKnownLimit
            },
            issues: [
                UseIssue::UnknownProductDate,
                UseIssue::UnknownOpeningAnchor,
                UseIssue::OpeningChronologyConflict,
                UseIssue::UseByConflict,
                UseIssue::ShorterDeadlineConflict,
            ]
            .into_iter()
            .zip(flags)
            .filter_map(|(issue, set)| set.then_some(issue))
            .collect(),
        });
    }
    let shopping =
        project_essentials(&derived.demand, &inputs.confirmations, &inputs.evaluated_at)?;
    let mut other = derived.warnings.0.clone();
    let delivery_end = match &inputs.delivery {
        DeliveryInput::Unselected => {
            if !shopping.shopping_lines.is_empty() {
                other.push(conflict(
                    &derived.week_id,
                    ErrorCode::UnsupportedCapability,
                    false,
                ));
            }
            None
        }
        DeliveryInput::Proposed { end, .. } => Some(end),
        DeliveryInput::Confirmed { order } => {
            require(
                order.approved_plan.derived_digest == derived.digest
                    && order.approved_plan.week_id == derived.week_id
                    && order.approved_plan.plan_revision == derived.plan_revision
                    && order.approved_plan.context_stamp == derived.context_stamp,
                ErrorCode::IdentityMismatch,
                "/delivery/order/approved_plan",
            )?;
            require(
                order.ordered_recipe_refs == derived.recipe_refs,
                ErrorCode::IdentityMismatch,
                "/delivery/order/ordered_recipe_refs",
            )?;
            Some(&order.confirmation.delivery_end)
        }
    };
    if let Some(end) = delivery_end {
        for usage in &graph.uses {
            if let UseKey::Preparation { phase } = &usage.use_key
                && shopping
                    .shopping_lines
                    .iter()
                    .any(|l| l.line_id == usage.line_id)
                && end.instant > usage.at.instant
            {
                let mut c = conflict(&derived.week_id, ErrorCode::DomainConflict, true);
                c.slot_ids.0.push(phase.occurrence.slot.clone());
                c.recipe_refs.0.push(phase.occurrence.recipe.clone());
                // Multiple ingredients at one phase remain one real timing detail.
                if !other.contains(&c) {
                    other.push(c);
                }
                limit(other.len(), 100, "/other_checks")?;
            }
        }
    }
    if !shopping.renewal_line_ids.is_empty() {
        other.push(conflict(
            &derived.week_id,
            ErrorCode::UnsupportedCapability,
            false,
        ));
    }
    let other_checks = canonicalise_other_checks(&other)?;
    let (warnings, blocking) = summarise(&derived.week_id, &checks, &other_checks)?;
    let mut report = FeasibilityReport {
        derived_digest: derived.digest.clone(),
        inputs_digest: inputs.digest.clone(),
        uses: checks,
        other_checks,
        warnings,
        blocking,
        digest: PayloadDigest("0".repeat(64)),
    };
    report.digest = digest_feasibility_report(&report)?;
    Ok(report)
}

fn exceeds(actual: i64, latest: i64, boundary: DeadlineBoundary) -> bool {
    actual > latest || (actual == latest && boundary == DeadlineBoundary::Exclusive)
}

pub(crate) fn canonicalise_other_checks(checks: &[Conflict]) -> Result<Vec<Conflict>, DomainError> {
    // Two already bounded reports may contribute overlapping detail sets.
    limit(checks.len(), 200, "/other_checks")?;
    let mut keyed = vec![];
    for check in checks {
        shape(check, "/other_checks")?;
        keyed.push((canonical_bytes(check)?, check.clone()));
    }
    keyed.sort_by(|a, b| a.0.cmp(&b.0));
    keyed.dedup_by(|a, b| a.0 == b.0);
    limit(keyed.len(), 100, "/other_checks")?;
    Ok(keyed.into_iter().map(|(_, c)| c).collect())
}

pub(crate) fn summarise(
    week: &WeekId,
    uses: &[UseCheck],
    other: &[Conflict],
) -> Result<(Vec<FoodSummary>, Vec<FoodSummary>), DomainError> {
    shape(week, "/week")?;
    limit(uses.len(), 16384, "/uses")?;
    let other = canonicalise_other_checks(other)?;
    let mut warnings = vec![];
    let mut blocking = vec![];
    if uses.iter().any(|u| {
        u.issues.iter().any(|i| {
            matches!(
                i,
                UseIssue::UnknownProductDate | UseIssue::UnknownOpeningAnchor
            )
        })
    }) {
        warnings.push(FoodSummary {
            kind: FoodSummaryKind::UnknownUseEvidence,
            conflict: conflict(week, ErrorCode::UnsupportedCapability, false),
        });
    }
    if other.iter().any(|c| !c.blocking) {
        warnings.push(FoodSummary {
            kind: FoodSummaryKind::OtherPendingChecks,
            conflict: conflict(week, ErrorCode::UnsupportedCapability, false),
        });
    }
    if uses.iter().any(|u| {
        u.issues.iter().any(|i| {
            matches!(
                i,
                UseIssue::OpeningChronologyConflict
                    | UseIssue::UseByConflict
                    | UseIssue::ShorterDeadlineConflict
            )
        })
    }) || other.iter().any(|c| c.blocking)
    {
        blocking.push(FoodSummary {
            kind: FoodSummaryKind::BlockingChecks,
            conflict: conflict(week, ErrorCode::DomainConflict, true),
        });
    }
    Ok((warnings, blocking))
}

fn conflict(week: &WeekId, code: ErrorCode, blocking: bool) -> Conflict {
    Conflict {
        code,
        week_ids: BoundedVec(vec![week.clone()]),
        slot_ids: BoundedVec(vec![]),
        recipe_refs: BoundedVec(vec![]),
        evidence_ids: BoundedVec(vec![]),
        blocking,
    }
}

fn validate_inputs(inputs: &FeasibilityInputs) -> Result<(), DomainError> {
    shape(&inputs.revision, "/revision")?;
    shape(&inputs.digest, "/digest")?;
    shape(&inputs.evaluated_at, "/evaluated_at")?;
    limit(inputs.confirmations.len(), 4096, "/confirmations")?;
    for c in &inputs.confirmations {
        shape(c, "/confirmations")?;
        require(
            c.confirmed_at <= inputs.evaluated_at,
            ErrorCode::InvalidValue,
            "/confirmations/confirmed_at",
        )?;
    }
    ordered(
        &inputs.confirmations,
        |a, b| a.line_id.cmp(&b.line_id),
        "/confirmations",
    )?;
    match &inputs.delivery {
        DeliveryInput::Unselected => (),
        DeliveryInput::Proposed {
            start,
            end,
            revision,
        } => {
            shape(start, "/delivery/start")?;
            shape(end, "/delivery/end")?;
            shape(revision, "/delivery/revision")?;
            require(
                start.instant <= end.instant,
                ErrorCode::InvalidValue,
                "/delivery/end",
            )?;
        }
        DeliveryInput::Confirmed { order } => {
            shape(order, "/delivery/order")?;
            require(
                order.confirmation.confirmed_at <= inputs.evaluated_at
                    && order.approved_plan.approved_at <= order.confirmation.confirmed_at,
                ErrorCode::InvalidValue,
                "/delivery/order/confirmed_at",
            )?;
        }
    }
    validate_products_shape(&inputs.products)
}
