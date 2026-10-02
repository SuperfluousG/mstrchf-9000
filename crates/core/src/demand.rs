//! Complete food demand, purchase projection and explicit product-use assessment.
//! No stock is inferred from past purchases; evidence completeness belongs to its producer.
use crate::eligibility::shape;
use crate::food::*;
use mstrchf_contracts::*;
mod feasibility;
mod uses;
pub use feasibility::assess_feasibility;
pub(crate) use feasibility::{canonicalise_other_checks, summarise};
pub use uses::reconstruct_use_graph;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ShoppingProjection {
    pub required: DemandSet,
    pub covered_line_ids: Vec<DemandLineId>,
    pub shopping_lines: Vec<DemandLine>,
    pub renewal_line_ids: Vec<DemandLineId>,
}

pub fn project_essentials(
    demand: &DemandSet,
    confirmations: &[EssentialsConfirmation],
    evaluated_at: &Instant,
) -> Result<ShoppingProjection, DomainError> {
    limit(demand.lines.len(), 4096, "/demand/lines")?;
    limit(confirmations.len(), 4096, "/confirmations")?;
    shape(demand, "/demand")?;
    shape(evaluated_at, "/evaluated_at")?;
    for (i, c) in confirmations.iter().enumerate() {
        shape(c, "/confirmations")?;
        require(
            c.confirmed_at <= *evaluated_at,
            ErrorCode::InvalidValue,
            "/confirmations/confirmed_at",
        )?;
        if let Some(previous) = i.checked_sub(1).map(|p| &confirmations[p]) {
            require(
                previous.line_id != c.line_id,
                ErrorCode::InvalidValue,
                "/confirmations/line_id",
            )?;
            require(
                previous.line_id < c.line_id,
                ErrorCode::NoncanonicalValue,
                "/confirmations",
            )?;
        }
    }
    let mut result = ShoppingProjection {
        required: demand.clone(),
        covered_line_ids: vec![],
        shopping_lines: vec![],
        renewal_line_ids: vec![],
    };
    for line in demand.lines.iter() {
        let c = confirmations.iter().find(|c| c.line_id == line.line_id);
        let current = c.is_some_and(|c| {
            c.demand_digest == demand.digest && c.displayed_quantity == line.quantity
        });
        if line.essential && current && c.is_some_and(|c| c.choice == EssentialsChoice::HaveEnough)
        {
            result.covered_line_ids.push(line.line_id.clone());
        } else {
            result.shopping_lines.push(line.clone());
            if line.essential && c.is_some() && !current {
                result.renewal_line_ids.push(line.line_id.clone());
            }
        }
    }
    // Confirmations for lines removed by a new demand projection convey no stock.
    result.covered_line_ids.sort();
    result.renewal_line_ids.sort();
    Ok(result)
}

// The adopted native API retains the complete report in FoodFailure by value.
#[allow(clippy::result_large_err)]
pub(crate) fn demand_keys(
    plan: &WeekPlan,
    context: &PlanningContext,
    evidence: &FoodEvidence,
) -> Result<Vec<DemandKey>, FoodFailure> {
    let occurrences = occurrences(plan, context, evidence)?;
    let mut keys = vec![];
    for occurrence in occurrences
        .iter()
        .filter(|o| o.kind == MethodKind::Production)
    {
        for ingredient in &occurrence.scaled.ingredients {
            let key = ingredient_key(ingredient);
            if !keys.contains(&key) {
                keys.push(key);
            }
            limit(keys.len(), 4096, "/demand/lines")?;
        }
    }
    keys.sort_by(demand_cmp);
    Ok(keys)
}

pub(crate) fn ingredient_key(ingredient: &crate::portions::ScaledIngredient) -> DemandKey {
    DemandKey {
        ingredient_id: ingredient.ingredient_id.clone(),
        unit: ingredient.quantity.unit,
        form: ingredient.quantity.form,
    }
}

#[allow(clippy::result_large_err)]
pub(crate) fn derive_demand(
    plan: &WeekPlan,
    context: &PlanningContext,
    evidence: &FoodEvidence,
    identities: &FoodIdentities,
    input_digest: &PlanInputDigest,
) -> Result<DemandSet, FoodFailure> {
    let occurrences = occurrences(plan, context, evidence)?;
    let expected = demand_keys(plan, context, evidence)?;
    require(
        identities.demand_lines.len() == expected.len()
            && identities
                .demand_lines
                .iter()
                .zip(&expected)
                .all(|((k, _), expected)| k == expected),
        ErrorCode::IdentityMismatch,
        "/identities/demand_lines",
    )?;
    let mut lines: Vec<DemandLine> = vec![];
    for occurrence in occurrences
        .iter()
        .filter(|o| o.kind == MethodKind::Production)
    {
        for ingredient in &occurrence.scaled.ingredients {
            let key = ingredient_key(ingredient);
            let id = identities
                .demand_lines
                .iter()
                .find(|(k, _)| k == &key)
                .ok_or_else(|| err(ErrorCode::MissingReference, "/identities/demand_lines"))?
                .1
                .clone();
            if let Some(line) = lines.iter_mut().find(|l| l.line_id == id) {
                require(
                    line.ingredient_id == key.ingredient_id
                        && line.quantity.unit == key.unit
                        && line.quantity.form == key.form,
                    ErrorCode::IdentityMismatch,
                    "/identities/demand_lines",
                )?;
                line.quantity.amount = PositiveRational(
                    line.quantity
                        .amount
                        .0
                        .checked_add(&ingredient.quantity.amount.0)?,
                );
                add_origin(line, occurrence)?;
            } else {
                let mut line = DemandLine {
                    line_id: id,
                    ingredient_id: ingredient.ingredient_id.clone(),
                    quantity: ingredient.quantity.clone(),
                    origin_batches: BoundedVec(vec![]),
                    origin_extras: BoundedVec(vec![]),
                    essential: evidence
                        .essential_ingredient_ids
                        .contains(&ingredient.ingredient_id),
                    conversion_evidence_ids: BoundedVec(vec![]),
                };
                add_origin(&mut line, occurrence)?;
                lines.push(line);
            }
        }
    }
    // This expansion retains the exact admitted ingredient unit/form. No conversion
    // is performed, so no unrelated conversion receipt is claimed as provenance.
    lines.sort_by(|a, b| a.line_id.cmp(&b.line_id));
    let lines = BoundedVec(lines);
    let digest = demand_digest(&DemandMaterial {
        schema_version: SchemaVersion,
        input_plan_digest: input_digest.clone(),
        lines: lines.clone(),
    })?;
    let demand = DemandSet {
        input_plan_digest: input_digest.clone(),
        lines,
        digest,
    };
    shape(&demand, "/demand")?;
    Ok(demand)
}

fn add_origin(line: &mut DemandLine, occurrence: &Occurrence) -> Result<(), DomainError> {
    if let Some(extra) = &occurrence.key.extra_id {
        if !line.origin_extras.contains(extra) {
            line.origin_extras.0.push(extra.clone());
            line.origin_extras.0.sort();
        }
    } else if !line.origin_batches.contains(&occurrence.batch_id) {
        line.origin_batches.0.push(occurrence.batch_id.clone());
        line.origin_batches.0.sort();
    }
    limit(line.origin_batches.len(), 256, "/demand/origin_batches")?;
    limit(line.origin_extras.len(), 64, "/demand/origin_extras")
}
