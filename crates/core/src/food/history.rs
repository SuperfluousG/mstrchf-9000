//! Historical byte integrity and fresh applicability are separate boundaries.
use super::*;

#[derive(Debug)]
pub struct CheckedHistorical<'a> {
    saved: &'a HistoricalApproval,
}
impl<'a> CheckedHistorical<'a> {
    pub fn historical(&self) -> &'a HistoricalApproval {
        self.saved
    }
}
pub fn verify_historical(saved: &HistoricalApproval) -> Result<CheckedHistorical<'_>, DomainError> {
    verify_core(&saved.derived, &saved.basis)?;
    shape(&saved.approved)?;
    shape(&saved.order)?;
    let d = &saved.derived;
    let a = &saved.approved;
    let o = &saved.order;
    require(
        a.week_id == d.week_id
            && a.plan_revision == d.plan_revision
            && a.derived_digest == d.digest
            && a.context_stamp == d.context_stamp,
        ErrorCode::IdentityMismatch,
        "/approved",
    )?;
    require(
        o.approved_plan == *a
            && o.confirmation.week_id == a.week_id
            && o.confirmation.approval_id == a.approval_id
            && o.ordered_recipe_refs == d.recipe_refs,
        ErrorCode::IdentityMismatch,
        "/order",
    )?;
    require(
        o.confirmation.confirmed_at >= a.approved_at,
        ErrorCode::InvalidValue,
        "/order/confirmed_at",
    )?;
    Ok(CheckedHistorical { saved })
}
fn conflict(
    week: &WeekId,
    slot: Option<&SlotId>,
    recipe: Option<&RecipeRef>,
    code: ErrorCode,
) -> Conflict {
    Conflict {
        code,
        week_ids: BoundedVec(vec![week.clone()]),
        slot_ids: BoundedVec(slot.into_iter().cloned().collect()),
        recipe_refs: BoundedVec(recipe.into_iter().cloned().collect()),
        evidence_ids: BoundedVec(vec![]),
        blocking: true,
    }
}
fn method_semantics(a: &MethodPhases, b: &MethodPhases) -> bool {
    a.recipe == b.recipe
        && a.kind == b.kind
        && a.prep_ids == b.prep_ids
        && a.direct_reuse_counts
            .iter()
            .map(|r| (&r.component_id, r.planned_reheats))
            .eq(b
                .direct_reuse_counts
                .iter()
                .map(|r| (&r.component_id, r.planned_reheats)))
}
fn record_eligibility(
    checks: &mut Vec<Conflict>,
    e: Result<crate::eligibility::Eligibility, DomainError>,
    week: &WeekId,
    slot: &SlotId,
    recipe: &RecipeRef,
) {
    match e {
        Ok(crate::eligibility::Eligibility::Eligible) => (),
        Ok(crate::eligibility::Eligibility::Ineligible(c))
        | Ok(crate::eligibility::Eligibility::Unresolved(c)) => checks.extend(c),
        Err(e) => checks.push(conflict(week, Some(slot), Some(recipe), e.code)),
    }
}

pub fn assess_continuation(
    history: &CheckedHistorical<'_>,
    current: &ContinuationInputs,
) -> Result<ContinuationReport, DomainError> {
    validate_continuation_shape(current)?;
    require(
        continuation_inputs_digest(current)? == current.digest,
        ErrorCode::DigestMismatch,
        "/current/digest",
    )?;
    let h = history.saved;
    let saved = &h.basis;
    let d = &h.derived;
    let week = &saved.plan.week_id;
    require(
        order_source_digest(&current.sources[3], &h.order)? == current.sources[3].digest,
        ErrorCode::DigestMismatch,
        "/sources/order",
    )?;
    require(
        matches!(&current.feasibility.delivery,DeliveryInput::Confirmed{order} if order==&h.order),
        ErrorCode::IdentityMismatch,
        "/delivery/order",
    )?;
    require(
        current.feasibility.evaluated_at >= h.order.confirmation.confirmed_at,
        ErrorCode::InvalidValue,
        "/evaluated_at",
    )?;
    let original = occurrences(&saved.plan, &saved.context, &saved.evidence)
        .map_err(FoodFailure::into_domain)?;
    let mut slots = original
        .iter()
        .map(|o| o.key.slot.clone())
        .collect::<Vec<_>>();
    slots.sort_by(slot_cmp);
    slots.dedup();
    require(
        current
            .requirements
            .meals
            .iter()
            .map(|m| &m.slot)
            .eq(slots.iter()),
        ErrorCode::IdentityMismatch,
        "/requirements/closure",
    )?;
    require(
        current
            .evidence
            .recipes
            .iter()
            .map(live_recipe_ref)
            .eq(saved.evidence.recipes.iter().map(|e| &e.recipe)),
        ErrorCode::IdentityMismatch,
        "/evidence/closure",
    )?;
    let mut checks = Vec::new();
    for (slot, _) in &current.requirements.unconfirmed {
        checks.push(conflict(
            week,
            Some(slot),
            None,
            ErrorCode::UnsupportedCapability,
        ));
    }
    let mut fresh = saved.evidence.clone();
    fresh.recipes.clear();
    fresh.classes.clear();
    fresh.methods.clear();
    fresh.resources.clear();
    for r in &current.evidence.recipes {
        match r {
            LiveRecipeState::Withdrawn { recipe, .. } => checks.push(conflict(
                week,
                None,
                Some(recipe),
                ErrorCode::UnsupportedCapability,
            )),
            LiveRecipeState::Admitted {
                basis,
                classes,
                methods,
                resources,
            } => {
                fresh.recipes.push(basis.clone());
                fresh.classes.push(classes.clone());
                fresh.methods.extend(methods.clone());
                fresh.resources.extend(resources.clone());
                for old in saved.evidence.methods.iter().filter(|m| {
                    m.recipe == basis.recipe
                        && original
                            .iter()
                            .any(|o| o.key.recipe == m.recipe && o.kind == m.kind)
                }) {
                    if !methods.iter().any(|m| method_semantics(old, m)) {
                        checks.push(conflict(
                            week,
                            None,
                            Some(&basis.recipe),
                            ErrorCode::DomainConflict,
                        ));
                    }
                }
                for old in saved.evidence.resources.iter().filter(|r| {
                    r.recipe == basis.recipe
                        && saved.schedule.layout.routes.iter().any(|p| {
                            p.phase.occurrence.recipe == r.recipe && p.phase.prep_id == r.prep_id
                        })
                }) {
                    if !resources.iter().any(|r| {
                        r.recipe == old.recipe
                            && r.prep_id == old.prep_id
                            && r.equipment == old.equipment
                    }) {
                        checks.push(conflict(
                            week,
                            None,
                            Some(&basis.recipe),
                            ErrorCode::DomainConflict,
                        ));
                    }
                }
            }
        }
    }
    fresh.catalogue_digest = food_evidence_digest(&fresh)?;
    for m in &current.requirements.meals {
        let old = saved
            .context
            .requirements
            .iter()
            .find(|a| a.slot == m.slot)
            .ok_or_else(|| err(ErrorCode::MissingReference, "/historical/requirements"))?;
        let selected = saved
            .plan
            .selections
            .iter()
            .find(|s| s.slot == m.slot)
            .ok_or_else(|| err(ErrorCode::MissingReference, "/historical/selection"))?;
        let old_people: Vec<_> = old
            .attendees
            .iter()
            .map(|a| (&a.person, a.age_band))
            .collect();
        let now_people: Vec<_> = m
            .attendees
            .iter()
            .map(|a| (&a.person, a.age_band))
            .collect();
        if !m.enabled
            || old_people != now_people
            || m.eat_by != old.eat_by
            || m.location != old.location
            || m.mobility != old.mobility
            || (!m.leftovers_allowed && matches!(selected.batch_use, BatchUse::Leftover { .. }))
        {
            checks.push(conflict(
                week,
                Some(&m.slot),
                None,
                ErrorCode::DomainConflict,
            ));
        }
        let mut meal = m.clone();
        meal.preferred_hands_on_minutes = old.preferred_hands_on_minutes;
        for a in meal.attendees.0.iter_mut() {
            if let Some(o) = old.attendees.iter().find(|o| o.person == a.person) {
                a.portion = o.portion.clone();
            }
        }
        match reviewed(&saved.context, &fresh, &selected.recipe) {
            Ok(base) => {
                record_eligibility(
                    &mut checks,
                    crate::eligibility::eligible(&base, &meal),
                    week,
                    &m.slot,
                    &selected.recipe,
                );
                for extra in selected.extras.iter() {
                    match reviewed(&saved.context, &fresh, &extra.recipe) {
                        Ok(e) => record_eligibility(
                            &mut checks,
                            crate::eligibility::validate_extra(
                                &base,
                                &e,
                                extra,
                                &meal,
                                &selected.meal_overrides,
                            ),
                            week,
                            &m.slot,
                            &extra.recipe,
                        ),
                        Err(e) => {
                            checks.push(conflict(week, Some(&m.slot), Some(&extra.recipe), e.code))
                        }
                    }
                }
            }
            Err(e) => checks.push(conflict(
                week,
                Some(&m.slot),
                Some(&selected.recipe),
                e.code,
            )),
        }
    }
    for allocation in d.allocations.iter() {
        let batch = d
            .batches
            .iter()
            .find(|b| b.batch_id == allocation.batch_id)
            .ok_or_else(|| err(ErrorCode::MissingReference, "/batches"))?;
        let c = batch
            .components
            .iter()
            .find(|c| c.component_id == allocation.component_id)
            .ok_or_else(|| err(ErrorCode::MissingReference, "/components"))?;
        let class = fresh
            .classes
            .iter()
            .find(|v| v.recipe == batch.recipe)
            .and_then(|v| {
                v.components
                    .iter()
                    .find(|v| v.component_id == c.component_id)
            });
        let max = match class.map(|c| c.class) {
            Some(ComponentClass::General) => current.policy.general_seconds,
            Some(ComponentClass::Rice) => current.policy.rice_seconds,
            // Frozen A/B preserves unclassified fresh sources. Apply today's
            // general maximum without granting class support for later reuse.
            _ if allocation.target_slot == batch.source_slot => current.policy.general_seconds,
            _ => 0,
        };
        let original = match &c.original_production {
            ProductionMoment::Cooked { completed_at } => completed_at,
            ProductionMoment::Unheated {
                preparation_started_at,
            } => preparation_started_at,
        };
        let source_without_storage =
            allocation.target_slot == batch.source_slot && c.storage_rule.0.is_none();
        // Current known class ceilings also bind first service, independently
        // of nullable source storage and the unchanged reviewed reheat count.
        let age_valid = max > 0
            && crate::batches::check_original_age(original, &allocation.use_at, max).is_ok();
        let storage_valid = match &c.storage_rule.0 {
            Some(rule) => {
                max > 0
                    && crate::batches::check_reuse(
                        original,
                        &allocation.use_at,
                        rule,
                        max,
                        allocation.planned_reheats,
                    )
                    .is_ok()
            }
            None => source_without_storage,
        };
        let valid = allocation.planned_reheats <= current.policy.maximum_reheats
            && age_valid
            && storage_valid;
        if !valid {
            checks.push(conflict(
                week,
                Some(&allocation.target_slot),
                Some(&batch.recipe),
                ErrorCode::DomainConflict,
            ));
        }
    }
    let expected_pin = PlanOwnerPin {
        week: WeekRevision {
            week_id: week.clone(),
            revision: saved.plan.revision,
        },
        derived_digest: d.digest.clone(),
        basis_digest: saved_basis_digest(saved)?,
    };
    if let Some(query) = &current.occupied.query {
        require(
            query.excluded_subject.as_ref() == Some(&expected_pin),
            ErrorCode::IdentityMismatch,
            "/occupied/excluded_subject",
        )?;
    }
    if let Err(e) = crate::prep::validate_continuation_occupancy(saved, d, &current.occupied) {
        checks.push(conflict(week, None, None, e.into_domain().code));
    }
    let feasibility = crate::demand::assess_feasibility(d, saved, &current.feasibility)?;
    let checks = crate::demand::canonicalise_other_checks(&checks)?;
    let mut combined = feasibility.other_checks.clone();
    combined.extend(checks.clone());
    let combined = crate::demand::canonicalise_other_checks(&combined)?;
    let (warnings, blocking) = crate::demand::summarise(week, &feasibility.uses, &combined)?;
    let mut report = ContinuationReport {
        order_id: h.order.order_id.clone(),
        approved_digest: h.approved.derived_digest.clone(),
        historical_basis_digest: saved_basis_digest(saved)?,
        current_inputs_digest: current.digest.clone(),
        feasibility,
        other_checks: checks,
        warnings,
        blocking,
        digest: PayloadDigest("0".repeat(64)),
    };
    report.digest = continuation_report_digest(&report)?;
    Ok(report)
}
