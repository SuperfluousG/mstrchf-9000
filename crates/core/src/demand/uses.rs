use super::*;

pub fn reconstruct_use_graph(
    derived: &DerivedPlan,
    saved: &SavedFoodBasis,
) -> Result<UseGraph, DomainError> {
    verify_core(derived, saved)?;
    reconstruct_verified(derived, saved)
}

pub(super) fn bindings(
    derived: &DerivedPlan,
    saved: &SavedFoodBasis,
) -> Result<Vec<TaskBinding>, DomainError> {
    crate::prep::validate_schedule(
        &saved.plan,
        &saved.context,
        &saved.evidence,
        &saved.identities,
        &saved.schedule,
        &derived.prep_sessions,
    )
    .map_err(FoodFailure::into_domain)
}

pub(super) fn task_at<'a>(
    derived: &'a DerivedPlan,
    bindings: &[TaskBinding],
    phase: &PhaseKey,
) -> Result<&'a ZonedMoment, DomainError> {
    let binding = bindings
        .iter()
        .find(|b| &b.phase == phase)
        .ok_or_else(|| err(ErrorCode::MissingReference, "/use/phase"))?;
    let session = derived
        .prep_sessions
        .iter()
        .find(|s| s.session_id == binding.task.session_id)
        .ok_or_else(|| err(ErrorCode::MissingReference, "/use/session"))?;
    Ok(&session
        .tasks
        .iter()
        .find(|t| t.task_id == binding.task.task_id)
        .ok_or_else(|| err(ErrorCode::MissingReference, "/use/task"))?
        .elapsed
        .start)
}

pub(super) fn reconstruct_verified(
    derived: &DerivedPlan,
    saved: &SavedFoodBasis,
) -> Result<UseGraph, DomainError> {
    let occurrences = occurrences(&saved.plan, &saved.context, &saved.evidence)
        .map_err(FoodFailure::into_domain)?;
    let bindings = bindings(derived, saved)?;
    let mut uses = vec![];
    for occurrence in &occurrences {
        let admitted = reviewed(&saved.context, &saved.evidence, &occurrence.key.recipe)?;
        let recipe = admitted.recipe();
        let source = occurrences
            .iter()
            .find(|o| o.is_source && o.batch_id == occurrence.batch_id);
        for binding in bindings
            .iter()
            .filter(|b| b.phase.occurrence == occurrence.key)
        {
            let prep = recipe
                .prep
                .iter()
                .find(|p| p.prep_id == binding.phase.prep_id)
                .ok_or_else(|| err(ErrorCode::MissingReference, "/use/prep"))?;
            let at = task_at(derived, &bindings, &binding.phase)?;
            let key = UseKey::Preparation {
                phase: binding.phase.clone(),
            };
            // Explicit ingredient references and component handling both carry
            // lineage. External component references resolve only to this batch.
            let steps: Vec<_> = recipe
                .steps
                .iter()
                .filter(|s| prep.step_ids.contains(&s.step_id))
                .collect();
            let local = occurrence.kind == MethodKind::Production;
            for ingredient in &occurrence.scaled.ingredients {
                if local
                    && (prep.ingredient_ids.contains(&ingredient.ingredient_id)
                        || steps.iter().any(|s| {
                            s.ingredient_ids.contains(&ingredient.ingredient_id)
                                || s.component_ids.contains(&ingredient.component_id)
                        }))
                {
                    add(&mut uses, line(derived, ingredient)?, &key, at)?;
                }
            }
            if !occurrence.is_source && occurrence.key.extra_id.is_none() {
                let source =
                    source.ok_or_else(|| err(ErrorCode::MissingReference, "/use/source"))?;
                for ingredient in &source.scaled.ingredients {
                    let component_handled = steps
                        .iter()
                        .any(|s| s.component_ids.contains(&ingredient.component_id));
                    let direct_ingredient = occurrence.kind == MethodKind::Reuse
                        && (prep.ingredient_ids.contains(&ingredient.ingredient_id)
                            || steps
                                .iter()
                                .any(|s| s.ingredient_ids.contains(&ingredient.ingredient_id)));
                    if component_handled || direct_ingredient {
                        require(
                            derived.allocations.iter().any(|a| {
                                a.batch_id == source.batch_id
                                    && a.component_id == ingredient.component_id
                                    && a.target_slot == occurrence.key.slot
                            }),
                            ErrorCode::MissingReference,
                            "/use/allocation",
                        )?;
                        add(&mut uses, line(derived, ingredient)?, &key, at)?;
                    }
                }
            }
        }
        if occurrence.kind == MethodKind::Production && !occurrence.is_source {
            for ingredient in &occurrence.scaled.ingredients {
                for serving in &occurrence.servings {
                    let key = UseKey::Consumption {
                        occurrence: occurrence.key.clone(),
                        component_id: ingredient.component_id.clone(),
                        person: serving.person.clone(),
                    };
                    add(
                        &mut uses,
                        line(derived, ingredient)?,
                        &key,
                        &occurrence.use_at,
                    )?;
                }
            }
        }
        if occurrence.key.extra_id.is_none() {
            let source = source.ok_or_else(|| err(ErrorCode::MissingReference, "/use/source"))?;
            for allocation in derived.allocations.iter().filter(|a| {
                a.batch_id == occurrence.batch_id && a.target_slot == occurrence.key.slot
            }) {
                require(
                    occurrence
                        .servings
                        .iter()
                        .any(|s| s.person == allocation.person),
                    ErrorCode::ForeignReference,
                    "/use/person",
                )?;
                let key = UseKey::Consumption {
                    occurrence: occurrence.key.clone(),
                    component_id: allocation.component_id.clone(),
                    person: allocation.person.clone(),
                };
                let mut any = false;
                for ingredient in source
                    .scaled
                    .ingredients
                    .iter()
                    .filter(|i| i.component_id == allocation.component_id)
                {
                    add(
                        &mut uses,
                        line(derived, ingredient)?,
                        &key,
                        &allocation.use_at,
                    )?;
                    any = true;
                }
                require(
                    any,
                    ErrorCode::UnsupportedCapability,
                    "/use/component/lineage",
                )?;
            }
        }
    }
    for line in derived.demand.lines.iter() {
        require(
            uses.iter().any(|u| u.line_id == line.line_id),
            ErrorCode::MissingReference,
            "/use/complete_demand",
        )?;
    }
    uses.sort_by(|a, b| {
        a.line_id
            .cmp(&b.line_id)
            .then_with(|| use_cmp(&a.use_key, &b.use_key))
    });
    Ok(UseGraph {
        derived_digest: derived.digest.clone(),
        uses,
    })
}

fn line<'a>(
    derived: &'a DerivedPlan,
    ingredient: &crate::portions::ScaledIngredient,
) -> Result<&'a DemandLineId, DomainError> {
    let matches: Vec<_> = derived
        .demand
        .lines
        .iter()
        .filter(|l| {
            l.ingredient_id == ingredient.ingredient_id
                && l.quantity.unit == ingredient.quantity.unit
                && l.quantity.form == ingredient.quantity.form
        })
        .collect();
    require(
        matches.len() == 1,
        ErrorCode::MissingReference,
        "/use/demand_line",
    )?;
    Ok(&matches[0].line_id)
}

fn add(
    uses: &mut Vec<DemandUse>,
    line_id: &DemandLineId,
    use_key: &UseKey,
    at: &ZonedMoment,
) -> Result<(), DomainError> {
    if let Some(existing) = uses
        .iter()
        .find(|u| &u.line_id == line_id && &u.use_key == use_key)
    {
        require(existing.at == *at, ErrorCode::IdentityMismatch, "/use/at")?;
    } else {
        limit(uses.len() + 1, 16384, "/uses")?;
        uses.push(DemandUse {
            line_id: line_id.clone(),
            use_key: use_key.clone(),
            at: at.clone(),
        });
    }
    Ok(())
}
