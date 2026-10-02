//! Exact local projection of the adopted private food occurrence calculation.
//! Historical validation runs first; this module does not admit source meaning.
use crate::model::*;
use mstrchf_contracts::*;
use mstrchf_core::{food::*, portions::*, recipes::*};

pub(crate) fn error(code: ErrorCode, path: &str) -> CalendarFailure {
    DomainError::new(code, path).into()
}
pub(crate) fn require(ok: bool, code: ErrorCode, path: &str) -> Result<(), CalendarFailure> {
    if ok { Ok(()) } else { Err(error(code, path)) }
}
pub(crate) fn shape<T: ValidateShape>(v: &T) -> Result<(), CalendarFailure> {
    v.validate_shape().map_err(|issues| {
        CalendarFailure::Invalid(DomainError {
            code: issues.first().map_or(ErrorCode::InvalidValue, |i| i.code),
            truncated: issues.len() > 100,
            issues: BoundedVec(issues.into_iter().take(100).collect()),
            retry: Retry::Never,
        })
    })
}
pub(crate) fn bound(n: usize, max: usize, path: &str) -> Result<(), CalendarFailure> {
    require(
        n <= max,
        ErrorCode::LimitExceeded,
        &format!("{path}/observed={n}/limit={max}"),
    )
}
pub(crate) fn pins(v: &[EvidencePin], path: &str) -> Result<(), CalendarFailure> {
    bound(v.len(), 256, path)?;
    require(!v.is_empty(), ErrorCode::MissingReference, path)?;
    for p in v {
        shape(&p.evidence_id)?;
        shape(&p.revision)?;
        shape(&p.digest)?;
    }
    require(
        v.windows(2).all(|w| w[0].evidence_id < w[1].evidence_id),
        ErrorCode::NoncanonicalValue,
        path,
    )
}
pub(crate) fn sum(v: &[DinerServing]) -> Result<PositiveRational, CalendarFailure> {
    let mut n = Rational::new(0, 1)?;
    for s in v {
        n = n.checked_add(&s.servings.0)?;
    }
    Ok(PositiveRational::new(n.numerator.0, n.denominator.0)?)
}
pub(crate) fn recipe<'a>(
    h: &'a HistoricalApproval,
    key: &RecipeRef,
) -> Result<&'a RecipeRevision, CalendarFailure> {
    h.basis
        .context
        .recipes
        .iter()
        .find(|r| r.recipe_id == key.recipe_id && r.revision == key.revision)
        .ok_or_else(|| error(ErrorCode::MissingReference, "/recipe"))
}
fn reviewed<'a>(
    h: &'a HistoricalApproval,
    key: &RecipeRef,
) -> Result<ReviewedRecipe<'a>, CalendarFailure> {
    Ok(assess_revision(
        recipe(h, key)?,
        h.basis
            .evidence
            .recipes
            .iter()
            .find(|e| e.recipe == *key)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/recipe/evidence"))?,
        h.basis
            .evidence
            .classes
            .iter()
            .find(|e| e.recipe == *key)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/recipe/classes"))?,
    )?)
}
#[derive(Clone)]
pub(crate) struct Occurrence {
    pub key: OccurrenceKey,
    pub method: MethodKind,
    pub scaled: ScaledRecipe,
    pub servings: Vec<DinerServing>,
    pub eat_by: ZonedMoment,
}
fn direct_scaled(
    r: &ReviewedRecipe<'_>,
    reference: &RecipeRef,
    requested: PositiveRational,
) -> Result<ScaledRecipe, CalendarFailure> {
    let y = r.recipe().r#yield.servings.0;
    let mut n = u128::from(requested.0.numerator.0) * u128::from(y.denominator.0);
    let mut d = u128::from(requested.0.denominator.0) * u128::from(y.numerator.0);
    let (mut a, mut b) = (n, d);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    n /= a;
    d /= a;
    require(
        n <= u128::from(MAX_NUMERATOR) && d <= u128::from(MAX_DENOMINATOR),
        ErrorCode::LimitExceeded,
        "/servings",
    )?;
    let multiplier = PositiveRational::new(n as u64, d as u64)?;
    let mut components = r.recipe().r#yield.components.0.clone();
    for c in &mut components {
        c.quantity.amount = PositiveRational(c.quantity.amount.0.checked_mul(&multiplier.0)?);
    }
    Ok(ScaledRecipe {
        recipe: reference.clone(),
        multiplier,
        requested_servings: requested,
        produced_servings: requested,
        unallocated_servings: Rational::new(0, 1)?,
        discrete_batches: None,
        ingredients: vec![],
        components,
    })
}
pub(crate) fn occurrences(h: &HistoricalApproval) -> Result<Vec<Occurrence>, CalendarFailure> {
    let mut result = Vec::new();
    for s in h.basis.plan.selections.iter() {
        let meal = h
            .basis
            .context
            .requirements
            .iter()
            .find(|m| m.slot == s.slot)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/requirements"))?;
        if !meal.enabled || meal.location == MealLocation::Skip {
            continue;
        }
        let r = reviewed(h, &s.recipe)?;
        // Historical verification already checks all eligibility and resolved servings.
        let servings = resolve_servings(&r, meal, &s.meal_overrides)
            .map_err(|_| error(ErrorCode::IdentityMismatch, "/servings"))?;
        require(
            servings == s.servings.0,
            ErrorCode::IdentityMismatch,
            "/servings",
        )?;
        let eat_by = s
            .meal_overrides
            .eat_by
            .0
            .as_ref()
            .or(meal.eat_by.0.as_ref())
            .ok_or_else(|| error(ErrorCode::UnsupportedCapability, "/eat_by"))?
            .clone();
        let requested = sum(&servings)?;
        let (method, scaled) = match &s.batch_use {
            BatchUse::NewBatch {
                production_servings,
                ..
            } => (
                MethodKind::Production,
                scale_recipe(&r, *production_servings)?,
            ),
            BatchUse::Leftover { .. } if r.recipe().reused_inputs.is_empty() => {
                (MethodKind::Reuse, direct_scaled(&r, &s.recipe, requested)?)
            }
            BatchUse::Leftover { .. } => (MethodKind::Production, scale_recipe(&r, requested)?),
        };
        result.push(Occurrence {
            key: OccurrenceKey {
                slot: s.slot.clone(),
                extra_id: None,
                recipe: s.recipe.clone(),
            },
            method,
            scaled,
            servings,
            eat_by: eat_by.clone(),
        });
        for e in s.extras.iter() {
            let r = reviewed(h, &e.recipe)?;
            let mut meal = meal.clone();
            meal.attendees
                .0
                .retain(|a| e.recipients.iter().any(|p| p.person == a.person));
            let mut overrides = s.meal_overrides.clone();
            overrides
                .portions
                .0
                .retain(|p| e.recipients.iter().any(|a| p.person == a.person));
            let servings = resolve_servings(&r, &meal, &overrides)
                .map_err(|_| error(ErrorCode::IdentityMismatch, "/extra/recipients"))?;
            require(
                servings == e.recipients.0,
                ErrorCode::IdentityMismatch,
                "/extra/recipients",
            )?;
            result.push(Occurrence {
                key: OccurrenceKey {
                    slot: s.slot.clone(),
                    extra_id: Some(e.extra_id.clone()),
                    recipe: e.recipe.clone(),
                },
                method: MethodKind::Production,
                scaled: scale_recipe(&r, sum(&servings)?)?,
                servings,
                eat_by: eat_by.clone(),
            });
        }
    }
    result.sort_by(|a, b| occurrence_cmp(&a.key, &b.key));
    Ok(result)
}

/// Topological instruction order, with source array position as the stable tie breaker.
pub(crate) fn ordered_steps(r: &RecipeRevision, prep_id: u16) -> Result<Vec<u16>, CalendarFailure> {
    let p = r
        .prep
        .iter()
        .find(|p| p.prep_id == prep_id)
        .ok_or_else(|| error(ErrorCode::MissingReference, "/prep"))?;
    let mut out = Vec::new();
    while out.len() < p.step_ids.len() {
        let next = r
            .steps
            .iter()
            .find(|s| {
                p.step_ids.contains(&s.step_id)
                    && !out.contains(&s.step_id)
                    && s.depends_on
                        .iter()
                        .all(|d| !p.step_ids.contains(d) || out.contains(d))
            })
            .ok_or_else(|| error(ErrorCode::DomainConflict, "/steps/dependency_cycle"))?;
        out.push(next.step_id);
    }
    Ok(out)
}
