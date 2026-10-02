//! Original-batch conservation and direct-original reuse.
use crate::food::{
    self, AllocationKey, FoodEvidence, FoodFailure, FoodIdentities, MethodKind, Occurrence,
    PhaseKey, TaskBinding,
};
use crate::recipes::ComponentClass;
use mstrchf_contracts::*;
use std::cmp::Ordering;
use std::collections::BTreeSet;

#[cfg(test)]
mod tests;

fn err(code: ErrorCode, path: &str) -> DomainError {
    DomainError::new(code, path)
}
fn ensure(value: bool, code: ErrorCode, path: &str) -> Result<(), DomainError> {
    if value { Ok(()) } else { Err(err(code, path)) }
}
fn shape<T: ValidateShape>(value: &T, path: &str) -> Result<(), DomainError> {
    value.validate_shape().map_err(|issues| {
        err(
            issues
                .first()
                .map(|v| v.code)
                .unwrap_or(ErrorCode::InvalidValue),
            path,
        )
    })
}
fn time(value: &ZonedMoment) -> Result<i64, DomainError> {
    let checked = ZonedMoment::new(
        value.local_date.clone(),
        value.local_time.clone(),
        value.zone.clone(),
        value.resolution,
        value.instant.clone(),
    )?;
    Ok(checked.instant.timestamp_seconds())
}
fn original(value: &ProductionMoment) -> &ZonedMoment {
    match value {
        ProductionMoment::Cooked { completed_at } => completed_at,
        ProductionMoment::Unheated {
            preparation_started_at,
        } => preparation_started_at,
    }
}
fn compare(a: Rational, b: Rational) -> Result<Ordering, DomainError> {
    Rational::new(a.numerator.0, a.denominator.0)?;
    Rational::new(b.numerator.0, b.denominator.0)?;
    Ok((u128::from(a.numerator.0) * u128::from(b.denominator.0))
        .cmp(&(u128::from(b.numerator.0) * u128::from(a.denominator.0))))
}
fn reduced(n: u128, d: u128) -> Result<Rational, DomainError> {
    ensure(d != 0, ErrorCode::InvalidValue, "/quantity")?;
    let (mut a, mut b) = (n, d);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    let (n, d) = (n / a, d / a);
    ensure(
        n <= u128::from(MAX_NUMERATOR) && d <= u128::from(MAX_DENOMINATOR),
        ErrorCode::LimitExceeded,
        "/quantity",
    )?;
    Rational::new(n as u64, d as u64)
}
fn divide(a: Rational, b: Rational) -> Result<Rational, DomainError> {
    Rational::new(a.numerator.0, a.denominator.0)?;
    Rational::new(b.numerator.0, b.denominator.0)?;
    reduced(
        u128::from(a.numerator.0) * u128::from(b.denominator.0),
        u128::from(a.denominator.0) * u128::from(b.numerator.0),
    )
}
fn subtract(a: Rational, b: Rational) -> Result<Rational, DomainError> {
    ensure(
        !compare(a, b)?.is_lt(),
        ErrorCode::DomainConflict,
        "/allocated",
    )?;
    reduced(
        u128::from(a.numerator.0) * u128::from(b.denominator.0)
            - u128::from(b.numerator.0) * u128::from(a.denominator.0),
        u128::from(a.denominator.0) * u128::from(b.denominator.0),
    )
}
fn units(a: &Quantity, b: &Quantity) -> bool {
    a.unit == b.unit && a.form == b.form
}

/// Predicate only: the caller supplies admitted classification and source-rule
/// applicability. This checks elapsed instants and caps, never evidence truth.
pub fn check_reuse(
    original: &ZonedMoment,
    use_at: &ZonedMoment,
    source_rule: &StorageRule,
    classified_limit_seconds: u32,
    planned_reheats: u8,
) -> Result<(), DomainError> {
    shape(source_rule, "/source_rule")?;
    ensure(
        (1..=172800).contains(&classified_limit_seconds),
        ErrorCode::InvalidValue,
        "/classified_limit_seconds",
    )?;
    ensure(
        planned_reheats <= 1 && planned_reheats <= source_rule.max_reheats,
        ErrorCode::DomainConflict,
        "/planned_reheats",
    )?;
    let elapsed = time(use_at)?
        .checked_sub(time(original)?)
        .ok_or_else(|| err(ErrorCode::ArithmeticOverflow, "/use_at"))?;
    ensure(
        elapsed >= 0
            && elapsed <= i64::from(classified_limit_seconds.min(source_rule.max_chilled_seconds)),
        ErrorCode::DomainConflict,
        "/use_at",
    )
}

/// Revalidates supplied output closure, exact conservation and the represented
/// original clocks. It does not authenticate derivation, source classification,
/// omitted storage rules, method counts or lineage; `derive_batches` does that
/// against the complete admitted native basis.
pub fn validate_allocations(
    batches: &[DerivedBatch],
    allocations: &[Allocation],
) -> Result<(), Vec<Conflict>> {
    validate_output(batches, allocations).map_err(|e| {
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
fn validate_output(
    batches: &[DerivedBatch],
    allocations: &[Allocation],
) -> Result<(), DomainError> {
    ensure(
        batches.len() <= 256 && allocations.len() <= 4096,
        ErrorCode::LimitExceeded,
        "/batches",
    )?;
    let mut batch_ids = BTreeSet::new();
    for batch in batches {
        ensure(
            (1..=128).contains(&batch.components.len()),
            ErrorCode::LimitExceeded,
            "/components",
        )?;
        shape(batch, "/batches")?;
        ensure(
            batch_ids.insert(&batch.batch_id),
            ErrorCode::InvalidValue,
            "/batch_id",
        )?;
        let mut component_ids = BTreeSet::new();
        for component in &batch.components.0 {
            ensure(
                component_ids.insert(&component.component_id),
                ErrorCode::InvalidValue,
                "/component_id",
            )?;
            time(original(&component.original_production))?;
            ensure(
                units(&component.produced, &component.allocated)
                    && component.produced.unit == component.unallocated.unit
                    && component.produced.form == component.unallocated.form,
                ErrorCode::DomainConflict,
                "/components/quantity",
            )?;
            ensure(
                component
                    .allocated
                    .amount
                    .0
                    .checked_add(&component.unallocated.amount)?
                    == component.produced.amount.0,
                ErrorCode::DomainConflict,
                "/components/conservation",
            )?;
        }
    }
    let mut ids = BTreeSet::new();
    let mut keys = Vec::new();
    for allocation in allocations {
        shape(allocation, "/allocations")?;
        ensure(
            ids.insert(&allocation.allocation_id),
            ErrorCode::InvalidValue,
            "/allocation_id",
        )?;
        let key = allocation_key(allocation);
        ensure(
            !keys.contains(&key),
            ErrorCode::InvalidValue,
            "/allocation_key",
        )?;
        keys.push(key);
        let batch = batches
            .iter()
            .find(|b| b.batch_id == allocation.batch_id)
            .ok_or_else(|| err(ErrorCode::MissingReference, "/batch_id"))?;
        let component = batch
            .components
            .iter()
            .find(|c| c.component_id == allocation.component_id)
            .ok_or_else(|| err(ErrorCode::ForeignReference, "/component_id"))?;
        ensure(
            allocation.target_slot.week_id == batch.source_slot.week_id,
            ErrorCode::ForeignReference,
            "/target_slot",
        )?;
        ensure(
            units(&component.produced, &allocation.quantity),
            ErrorCode::DomainConflict,
            "/quantity",
        )?;
        ensure(
            time(&allocation.use_at)? >= time(original(&component.original_production))?,
            ErrorCode::DomainConflict,
            "/use_at",
        )?;
        if allocation.target_slot == batch.source_slot {
            ensure(
                allocation.planned_reheats == 0 && allocation.remix_recipe.0.is_none(),
                ErrorCode::DomainConflict,
                "/original_service",
            )?;
        } else {
            let rule = component
                .storage_rule
                .0
                .as_ref()
                .ok_or_else(|| err(ErrorCode::UnsupportedCapability, "/storage_rule"))?;
            check_reuse(
                original(&component.original_production),
                &allocation.use_at,
                rule,
                172800,
                allocation.planned_reheats,
            )?;
        }
    }
    for batch in batches {
        let mut equivalent_basis = None;
        for component in &batch.components.0 {
            let mut total = Rational::new(0, 1)?;
            for allocation in allocations.iter().filter(|a| {
                a.batch_id == batch.batch_id && a.component_id == component.component_id
            }) {
                total = total.checked_add(&allocation.quantity.amount.0)?;
                // All components express servings on the same original basis.
                let basis = divide(
                    allocation.servings.0,
                    divide(allocation.quantity.amount.0, component.produced.amount.0)?,
                )?;
                if let Some(expected) = equivalent_basis {
                    ensure(expected == basis, ErrorCode::DomainConflict, "/servings")?;
                } else {
                    equivalent_basis = Some(basis);
                }
            }
            ensure(
                total == component.allocated.amount.0,
                ErrorCode::DomainConflict,
                "/components/allocated",
            )?;
        }
    }
    Ok(())
}
fn allocation_key(value: &Allocation) -> AllocationKey {
    AllocationKey {
        batch_id: value.batch_id.clone(),
        component_id: value.component_id.clone(),
        target_slot: value.target_slot.clone(),
        person: value.person.clone(),
    }
}

struct Portion {
    key: AllocationKey,
    servings: PositiveRational,
    quantity: Quantity,
    use_at: ZonedMoment,
    planned_reheats: u8,
    remix_recipe: Option<RecipeRef>,
}
struct Graph {
    sources: Vec<Occurrence>,
    portions: Vec<Portion>,
}

// R3 deliberately retains the complete feasibility report by value in failures.
#[allow(clippy::result_large_err)]
pub(crate) fn allocation_keys(
    plan: &WeekPlan,
    context: &PlanningContext,
    evidence: &FoodEvidence,
) -> Result<Vec<AllocationKey>, FoodFailure> {
    Ok(expand(plan, context, evidence)?
        .portions
        .into_iter()
        .map(|p| p.key)
        .collect())
}

#[allow(clippy::result_large_err)]
fn expand(
    plan: &WeekPlan,
    context: &PlanningContext,
    evidence: &FoodEvidence,
) -> Result<Graph, FoodFailure> {
    ensure(
        plan.selections.len() <= 21 && plan.batch_links.len() <= 256,
        ErrorCode::LimitExceeded,
        "/plan",
    )?;
    shape(plan, "/plan")?;
    let occurrences = food::occurrences(plan, context, evidence)?;
    let sources: Vec<_> = occurrences
        .iter()
        .filter(|o| o.is_source && o.key.extra_id.is_none())
        .cloned()
        .collect();
    let mut source_ids = BTreeSet::new();
    for source in &sources {
        ensure(
            source_ids.insert(&source.batch_id),
            ErrorCode::InvalidValue,
            "/batch_id",
        )?;
        let reviewed = food::reviewed(context, evidence, &source.key.recipe)?;
        ensure(
            reviewed.recipe().reused_inputs.is_empty(),
            ErrorCode::UnsupportedCapability,
            "/source/reused_inputs",
        )?;
        let selected = plan
            .selections
            .iter()
            .find(|s| s.slot == source.key.slot)
            .ok_or_else(|| err(ErrorCode::MissingReference, "/source/slot"))?;
        if let BatchUse::NewBatch {
            production_servings,
            batch_id,
        } = &selected.batch_use
        {
            ensure(
                batch_id == &source.batch_id
                    && source.scaled.produced_servings == *production_servings,
                ErrorCode::DomainConflict,
                "/production_servings",
            )?;
        } else {
            return Err(err(ErrorCode::ForeignReference, "/source/batch_use").into());
        }
    }
    let mut used_links = BTreeSet::new();
    let mut portions = Vec::new();
    for occurrence in occurrences.iter().filter(|o| o.key.extra_id.is_none()) {
        let source = sources
            .iter()
            .find(|s| s.batch_id == occurrence.batch_id)
            .ok_or_else(|| err(ErrorCode::MissingReference, "/source_batch"))?;
        let recipe = food::reviewed(context, evidence, &occurrence.key.recipe)?;
        let source_recipe = food::reviewed(context, evidence, &source.key.recipe)?;
        let selected = plan
            .selections
            .iter()
            .find(|s| s.slot == occurrence.key.slot)
            .ok_or_else(|| err(ErrorCode::MissingReference, "/selection"))?;
        let mut target_servings = Rational::new(0, 1)?;
        for serving in &occurrence.servings {
            target_servings = target_servings.checked_add(&serving.servings.0)?;
        }
        ensure(
            target_servings.numerator.0 > 0,
            ErrorCode::InvalidValue,
            "/servings",
        )?;
        let mut requirements: Vec<(ComponentId, Quantity, u8)> = Vec::new();
        if occurrence.is_source {
            ensure(
                occurrence.key == source.key && occurrence.kind == MethodKind::Production,
                ErrorCode::IdentityMismatch,
                "/source",
            )?;
            for component in &source.scaled.components {
                let amount = component
                    .quantity
                    .amount
                    .0
                    .checked_mul(&divide(target_servings, source.scaled.produced_servings.0)?)?;
                requirements.push((
                    component.component_id.clone(),
                    Quantity {
                        amount: PositiveRational(amount),
                        ..component.quantity.clone()
                    },
                    0,
                ));
            }
        } else {
            // A supported target recipe cannot erase a conflicting source
            // ingredient. Use conservative whole-source dietary facts only;
            // the source's original meal type and equipment do not apply here.
            let meal = context
                .requirements
                .iter()
                .find(|m| m.slot == occurrence.key.slot)
                .ok_or_else(|| err(ErrorCode::MissingReference, "/source/requirements"))?;
            for serving in &occurrence.servings {
                let diner = meal
                    .attendees
                    .iter()
                    .find(|d| d.person == serving.person)
                    .ok_or_else(|| err(ErrorCode::ForeignReference, "/source/recipient"))?;
                for requirement in &diner.hard_requirements.0 {
                    let facts: Vec<_> = source_recipe
                        .recipe()
                        .dietary_evidence
                        .iter()
                        .filter(|e| &e.requirement == requirement)
                        .collect();
                    let excluded = matches!(requirement, HardRequirement::IngredientExclusion { ingredient_id }
                        if source_recipe.recipe().ingredients.iter().any(|i| &i.ingredient_id == ingredient_id));
                    if excluded
                        || facts
                            .iter()
                            .any(|f| f.verdict == DietaryVerdict::Conflicting)
                    {
                        return Err(food::blocked(plan, ErrorCode::DomainConflict));
                    }
                    if facts.is_empty()
                        || facts.iter().any(|f| f.verdict != DietaryVerdict::Supported)
                    {
                        return Err(food::blocked(plan, ErrorCode::UnsupportedCapability));
                    }
                }
            }
            let component_ids = match &selected.batch_use {
                BatchUse::Leftover {
                    batch_id,
                    component_ids,
                } if batch_id == &source.batch_id => &component_ids.0,
                _ => return Err(err(ErrorCode::ForeignReference, "/batch_use").into()),
            };
            ensure(
                source.key.slot != occurrence.key.slot,
                ErrorCode::DomainConflict,
                "/batch_link/cycle",
            )?;
            if recipe.recipe().reused_inputs.is_empty() {
                ensure(
                    occurrence.key.recipe == source.key.recipe
                        && occurrence.kind == MethodKind::Reuse,
                    ErrorCode::UnsupportedCapability,
                    "/reuse/recipe",
                )?;
                let methods: Vec<_> = evidence
                    .methods
                    .iter()
                    .filter(|m| m.recipe == occurrence.key.recipe && m.kind == MethodKind::Reuse)
                    .collect();
                ensure(
                    methods.len() == 1,
                    ErrorCode::UnsupportedCapability,
                    "/reuse/method",
                )?;
                let method = methods[0];
                ensure(
                    method.direct_reuse_counts.len() == source_recipe.recipe().components.len(),
                    ErrorCode::UnsupportedCapability,
                    "/reuse/counts",
                )?;
                for phase_id in &method.prep_ids {
                    let prep = recipe
                        .recipe()
                        .prep
                        .iter()
                        .find(|p| &p.prep_id == phase_id)
                        .ok_or_else(|| err(ErrorCode::MissingReference, "/reuse/prep"))?;
                    for step_id in &prep.step_ids.0 {
                        let step = recipe
                            .recipe()
                            .steps
                            .iter()
                            .find(|s| &s.step_id == step_id)
                            .ok_or_else(|| err(ErrorCode::MissingReference, "/reuse/step"))?;
                        ensure(
                            step.component_ids
                                .iter()
                                .all(|id| component_ids.contains(id)),
                            ErrorCode::UnsupportedCapability,
                            "/reuse/component_subset",
                        )?;
                    }
                }
                for component_id in component_ids {
                    let component = source
                        .scaled
                        .components
                        .iter()
                        .find(|c| &c.component_id == component_id)
                        .ok_or_else(|| err(ErrorCode::ForeignReference, "/component_ids"))?;
                    let counts: Vec<_> = method
                        .direct_reuse_counts
                        .iter()
                        .filter(|c| &c.component_id == component_id)
                        .collect();
                    ensure(
                        counts.len() == 1,
                        ErrorCode::UnsupportedCapability,
                        "/reuse/counts",
                    )?;
                    let count = counts[0];
                    let basis = evidence
                        .recipes
                        .iter()
                        .find(|b| b.recipe == source.key.recipe)
                        .ok_or_else(|| err(ErrorCode::MissingReference, "/reuse/evidence"))?;
                    ensure(
                        count.recipe == method.recipe
                            && count.planned_reheats <= 1
                            && !count.evidence_ids.is_empty()
                            && count.evidence_ids.iter().all(|id| {
                                method.evidence_ids.contains(id)
                                    && basis.admitted.iter().any(|pin| &pin.evidence_id == id)
                            }),
                        ErrorCode::UnsupportedCapability,
                        "/reuse/counts/evidence",
                    )?;
                    let amount = component.quantity.amount.0.checked_mul(&divide(
                        target_servings,
                        source.scaled.produced_servings.0,
                    )?)?;
                    requirements.push((
                        component_id.clone(),
                        Quantity {
                            amount: PositiveRational(amount),
                            ..component.quantity.clone()
                        },
                        count.planned_reheats,
                    ));
                }
            } else {
                ensure(
                    occurrence.kind == MethodKind::Production
                        && recipe.recipe().reused_inputs.len() == component_ids.len(),
                    ErrorCode::DomainConflict,
                    "/remix/components",
                )?;
                for input in &recipe.recipe().reused_inputs.0 {
                    ensure(
                        input.source_recipe == source.key.recipe
                            && component_ids.contains(&input.source_component_id),
                        ErrorCode::ForeignReference,
                        "/remix/source",
                    )?;
                    let component = source
                        .scaled
                        .components
                        .iter()
                        .find(|c| c.component_id == input.source_component_id)
                        .ok_or_else(|| err(ErrorCode::ForeignReference, "/remix/component"))?;
                    ensure(
                        units(&component.quantity, &input.quantity_per_recipe_yield),
                        ErrorCode::DomainConflict,
                        "/remix/quantity",
                    )?;
                    ensure(
                        input.planned_reheats <= 1,
                        ErrorCode::UnsupportedCapability,
                        "/remix/planned_reheats",
                    )?;
                    let amount = input
                        .quantity_per_recipe_yield
                        .amount
                        .0
                        .checked_mul(&divide(
                            target_servings,
                            recipe.recipe().r#yield.servings.0,
                        )?)?;
                    requirements.push((
                        input.source_component_id.clone(),
                        Quantity {
                            amount: PositiveRational(amount),
                            ..input.quantity_per_recipe_yield.clone()
                        },
                        input.planned_reheats,
                    ));
                }
            }
            requirements.sort_by(|a, b| a.0.cmp(&b.0));
            ensure(
                requirements.iter().map(|r| &r.0).eq(component_ids.iter()),
                ErrorCode::DomainConflict,
                "/component_ids",
            )?;
            let links: Vec<_> = plan
                .batch_links
                .iter()
                .enumerate()
                .filter(|(_, link)| {
                    link.batch_id == source.batch_id && link.target_slot == occurrence.key.slot
                })
                .collect();
            ensure(
                links.len() == 1,
                ErrorCode::MissingReference,
                "/batch_links",
            )?;
            let (index, link) = links[0];
            ensure(
                used_links.insert(index)
                    && link.source_slot == source.key.slot
                    && link.components.len() == requirements.len(),
                ErrorCode::ForeignReference,
                "/batch_links/source",
            )?;
            for (row, expected) in link.components.iter().zip(&requirements) {
                ensure(
                    row.component_id == expected.0 && row.quantity == expected.1,
                    ErrorCode::DomainConflict,
                    "/batch_links/components",
                )?;
            }
        }
        for (component_id, total, reheats) in requirements {
            let component = source
                .scaled
                .components
                .iter()
                .find(|c| c.component_id == component_id)
                .ok_or_else(|| err(ErrorCode::ForeignReference, "/component_id"))?;
            for serving in &occurrence.servings {
                let amount = total
                    .amount
                    .0
                    .checked_mul(&divide(serving.servings.0, target_servings)?)?;
                let source_servings = divide(amount, component.quantity.amount.0)?
                    .checked_mul(&source.scaled.produced_servings.0)?;
                portions.push(Portion {
                    key: AllocationKey {
                        batch_id: source.batch_id.clone(),
                        component_id: component_id.clone(),
                        target_slot: occurrence.key.slot.clone(),
                        person: serving.person.clone(),
                    },
                    servings: PositiveRational(source_servings),
                    quantity: Quantity {
                        amount: PositiveRational(amount),
                        ..total.clone()
                    },
                    use_at: occurrence.use_at.clone(),
                    planned_reheats: reheats,
                    remix_recipe: (!recipe.recipe().reused_inputs.is_empty())
                        .then(|| occurrence.key.recipe.clone()),
                });
                ensure(
                    portions.len() <= 4096,
                    ErrorCode::LimitExceeded,
                    "/allocations",
                )?;
            }
        }
    }
    ensure(
        used_links.len() == plan.batch_links.len(),
        ErrorCode::ForeignReference,
        "/batch_links",
    )?;
    portions.sort_by(|a, b| food::allocation_cmp(&a.key, &b.key));
    ensure(
        portions.windows(2).all(|v| v[0].key != v[1].key),
        ErrorCode::InvalidValue,
        "/allocation_key",
    )?;
    for source in &sources {
        for component in &source.scaled.components {
            let total = portions
                .iter()
                .filter(|a| {
                    a.key.batch_id == source.batch_id
                        && a.key.component_id == component.component_id
                })
                .try_fold(Rational::new(0, 1)?, |sum, p| {
                    sum.checked_add(&p.quantity.amount.0)
                })?;
            if compare(total, component.quantity.amount.0)?.is_gt() {
                return Err(food::blocked(plan, ErrorCode::DomainConflict));
            }
        }
    }
    Ok(Graph { sources, portions })
}

#[allow(clippy::result_large_err)]
pub(crate) fn derive_batches(
    plan: &WeekPlan,
    context: &PlanningContext,
    evidence: &FoodEvidence,
    identities: &FoodIdentities,
    sessions: &[PrepSession],
    bindings: &[TaskBinding],
) -> Result<(Vec<DerivedBatch>, Vec<Allocation>), FoodFailure> {
    ensure(
        sessions.len() <= 256 && bindings.len() <= 4096 && identities.allocations.len() <= 4096,
        ErrorCode::LimitExceeded,
        "/batch_inputs",
    )?;
    let graph = expand(plan, context, evidence)?;
    ensure(
        graph.portions.len() == identities.allocations.len(),
        ErrorCode::IdentityMismatch,
        "/identities/allocations",
    )?;
    let mut allocation_ids = BTreeSet::new();
    let mut allocations = Vec::new();
    for (portion, (key, id)) in graph.portions.iter().zip(&identities.allocations) {
        shape(id, "/allocation_id")?;
        ensure(
            key == &portion.key && allocation_ids.insert(id),
            ErrorCode::IdentityMismatch,
            "/identities/allocations",
        )?;
        allocations.push(Allocation {
            allocation_id: id.clone(),
            batch_id: key.batch_id.clone(),
            component_id: key.component_id.clone(),
            target_slot: key.target_slot.clone(),
            person: key.person.clone(),
            servings: portion.servings,
            quantity: portion.quantity.clone(),
            use_at: portion.use_at.clone(),
            planned_reheats: portion.planned_reheats,
            remix_recipe: Nullable(portion.remix_recipe.clone()),
        });
    }
    let mut batches = Vec::new();
    for source in &graph.sources {
        let reviewed = food::reviewed(context, evidence, &source.key.recipe)?;
        let mut components = Vec::new();
        for produced in &source.scaled.components {
            let spec = reviewed
                .recipe()
                .components
                .iter()
                .find(|c| c.component_id == produced.component_id)
                .ok_or_else(|| err(ErrorCode::MissingReference, "/component"))?;
            let (prep_id, cooked) = match &spec.production.0 {
                Some(ComponentProductionSpec::Cooked { completion_prep_id }) => {
                    (*completion_prep_id, true)
                }
                Some(ComponentProductionSpec::Unheated {
                    first_handling_prep_id,
                }) => (*first_handling_prep_id, false),
                None => {
                    return Err(
                        err(ErrorCode::UnsupportedCapability, "/component/production").into(),
                    );
                }
            };
            let phase = PhaseKey {
                occurrence: source.key.clone(),
                prep_id,
            };
            let matched: Vec<_> = bindings.iter().filter(|b| b.phase == phase).collect();
            ensure(
                matched.len() == 1,
                ErrorCode::MissingReference,
                "/production/binding",
            )?;
            let binding = matched[0];
            let tasks: Vec<_> = sessions
                .iter()
                .filter(|s| s.session_id == binding.task.session_id)
                .flat_map(|s| s.tasks.iter())
                .filter(|t| t.task_id == binding.task.task_id)
                .collect();
            ensure(
                tasks.len() == 1,
                ErrorCode::MissingReference,
                "/production/task",
            )?;
            let task = tasks[0];
            ensure(
                task.recipe == source.key.recipe && task.prep_id == prep_id,
                ErrorCode::ForeignReference,
                "/production/task",
            )?;
            let original_production = if cooked {
                ProductionMoment::Cooked {
                    completed_at: task.elapsed.end.clone(),
                }
            } else {
                ProductionMoment::Unheated {
                    preparation_started_at: task.elapsed.start.clone(),
                }
            };
            time(original(&original_production))?;
            let mut rules: Vec<_> = reviewed
                .recipe()
                .storage
                .iter()
                .filter(|r| r.component_id == produced.component_id)
                .collect();
            rules.sort_by(|a, b| {
                (a.max_chilled_seconds, a.max_reheats, &a.evidence_id).cmp(&(
                    b.max_chilled_seconds,
                    b.max_reheats,
                    &b.evidence_id,
                ))
            });
            let class = reviewed
                .component_classes()
                .components
                .iter()
                .find(|c| c.component_id == produced.component_id)
                .map(|c| c.class);
            let ceiling = match class {
                Some(ComponentClass::General) => Some(172800),
                Some(ComponentClass::Rice) => Some(86400),
                _ => None,
            };
            let mut total = Rational::new(0, 1)?;
            for allocation in allocations.iter().filter(|a| {
                a.batch_id == source.batch_id && a.component_id == produced.component_id
            }) {
                total = total.checked_add(&allocation.quantity.amount.0)?;
                ensure(
                    time(&allocation.use_at)? >= time(original(&original_production))?,
                    ErrorCode::DomainConflict,
                    "/production/use_at",
                )?;
                if allocation.target_slot != source.key.slot {
                    let limit = ceiling
                        .ok_or_else(|| err(ErrorCode::UnsupportedCapability, "/component/class"))?;
                    ensure(
                        !rules.is_empty(),
                        ErrorCode::UnsupportedCapability,
                        "/storage_rule",
                    )?;
                    for rule in &rules {
                        check_reuse(
                            original(&original_production),
                            &allocation.use_at,
                            rule,
                            limit,
                            allocation.planned_reheats,
                        )
                        .map_err(|failure| {
                            if failure.code == ErrorCode::DomainConflict {
                                food::blocked(plan, failure.code)
                            } else {
                                FoodFailure::Invalid(failure)
                            }
                        })?;
                    }
                }
            }
            ensure(
                total.numerator.0 > 0,
                ErrorCode::UnsupportedCapability,
                "/components/allocated",
            )?;
            components.push(DerivedComponent {
                component_id: produced.component_id.clone(),
                produced: produced.quantity.clone(),
                allocated: Quantity {
                    amount: PositiveRational(total),
                    ..produced.quantity.clone()
                },
                unallocated: NonnegativeQuantity {
                    amount: subtract(produced.quantity.amount.0, total)?,
                    unit: produced.quantity.unit,
                    form: produced.quantity.form,
                },
                original_production,
                storage_rule: Nullable(rules.first().map(|r| (*r).clone())),
            });
        }
        components.sort_by(|a, b| a.component_id.cmp(&b.component_id));
        batches.push(DerivedBatch {
            batch_id: source.batch_id.clone(),
            recipe: source.key.recipe.clone(),
            source_slot: source.key.slot.clone(),
            components: BoundedVec(components),
        });
    }
    batches.sort_by(|a, b| a.batch_id.cmp(&b.batch_id));
    allocations.sort_by(|a, b| a.allocation_id.cmp(&b.allocation_id));
    validate_allocations(&batches, &allocations).map_err(|conflicts| FoodFailure::Blocked {
        conflicts,
        feasibility: None,
    })?;
    Ok((batches, allocations))
}
