//! Exact reviewed serving and quantity calculations; no inferred food conversions.
//! Exact reviewed portion, production and conversion arithmetic.
use crate::recipes::ReviewedRecipe;
use mstrchf_contracts::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScaledIngredient {
    pub ingredient_id: IngredientId,
    pub component_id: ComponentId,
    pub quantity: Quantity,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScaledRecipe {
    pub recipe: RecipeRef,
    pub multiplier: PositiveRational,
    pub requested_servings: PositiveRational,
    pub produced_servings: PositiveRational,
    pub unallocated_servings: Rational,
    pub discrete_batches: Option<u16>,
    pub ingredients: Vec<ScaledIngredient>,
    pub components: Vec<ComponentYield>,
}
pub fn resolve_servings(
    recipe: &ReviewedRecipe<'_>,
    meal: &MealRequirements,
    overrides: &MealOverrides,
) -> Result<Vec<DinerServing>, Vec<Conflict>> {
    resolve_inner(recipe, meal, overrides).map_err(|e| {
        // Malformed input references must not be copied into a public conflict.
        let slot_valid = meal.slot.validate_shape().is_ok();
        vec![Conflict {
            code: e.code,
            week_ids: BoundedVec(if slot_valid {
                vec![meal.slot.week_id.clone()]
            } else {
                vec![]
            }),
            slot_ids: BoundedVec(if slot_valid {
                vec![meal.slot.clone()]
            } else {
                vec![]
            }),
            recipe_refs: BoundedVec(crate::eligibility::recipe_ref(recipe).into_iter().collect()),
            evidence_ids: BoundedVec(vec![]),
            blocking: true,
        }]
    })
}
fn resolve_inner(
    recipe: &ReviewedRecipe<'_>,
    meal: &MealRequirements,
    overrides: &MealOverrides,
) -> Result<Vec<DinerServing>, DomainError> {
    use crate::eligibility::{bound, check_meal, recipe_ref, shape};
    check_meal(meal)?;
    bound(overrides.portions.len(), 0, 64, "/overrides/portions")?;
    shape(overrides, "/overrides")?;
    ready(recipe)?;
    if !recipe.recipe().meal_types.contains(&meal.slot.meal_type) {
        return Err(DomainError::new(
            ErrorCode::UnsupportedCapability,
            "/meal/slot/meal_type",
        ));
    }
    for (i, o) in overrides.portions.iter().enumerate() {
        if overrides.portions[..i].iter().any(|p| p.person == o.person) {
            return Err(DomainError::new(
                ErrorCode::DuplicateKey,
                "/overrides/portions",
            ));
        }
        if !meal.attendees.iter().any(|a| a.person == o.person) {
            return Err(DomainError::new(
                ErrorCode::ForeignReference,
                "/overrides/portions",
            ));
        }
    }
    let rf = recipe_ref(recipe)?;
    let mut servings = Vec::new();
    for a in meal.attendees.iter() {
        let (mut name, mut adjustment) = (a.portion.preset, a.portion.adjustment);
        if let Some(o) = a
            .portion
            .meal_overrides
            .iter()
            .find(|o| o.meal_type == meal.slot.meal_type)
        {
            name = o.preset;
            adjustment = o.adjustment;
        }
        if let Some(o) = overrides.portions.iter().find(|o| o.person == a.person) {
            name = o.preset;
            adjustment = o.adjustment;
        }
        let presets: Vec<_> = recipe
            .recipe()
            .portion_presets
            .iter()
            .filter(|p| {
                p.preset == name && p.age_band == a.age_band && p.meal_type == meal.slot.meal_type
            })
            .collect();
        if presets.len() != 1 || presets[0].evidence_ids.is_empty() {
            return Err(DomainError::new(
                ErrorCode::UnsupportedCapability,
                "/portion_presets",
            ));
        }
        let value = PositiveRational(presets[0].servings.0.checked_mul(&adjustment.0)?);
        servings.push(DinerServing {
            person: a.person.clone(),
            preset_ref: PortionPresetRef {
                recipe: rf.clone(),
                preset: name,
                age_band: a.age_band,
                meal_type: meal.slot.meal_type,
            },
            servings: value,
        });
    }
    Ok(servings)
}
pub fn scale_recipe(
    recipe: &ReviewedRecipe<'_>,
    requested: PositiveRational,
) -> Result<ScaledRecipe, DomainError> {
    crate::eligibility::shape(&requested, "/requested")?;
    ready(recipe)?;
    let r = recipe.recipe();
    let (produced, discrete_batches) = match r.scaling {
        ScalingPolicy::Linear { .. } => (requested, None),
        ScalingPolicy::DiscreteBatches {
            batch_yield,
            maximum_batches,
        } => {
            // No separate attestation in this API establishes a differing
            // nominal recipe/batch basis. Do not infer that equivalence.
            if batch_yield != r.r#yield.servings {
                return Err(DomainError::new(
                    ErrorCode::UnsupportedCapability,
                    "/scaling/batch_yield",
                ));
            }
            let n = ceil_ratio(requested.0, batch_yield.0)?;
            if n == 0 || n > u128::from(maximum_batches) {
                return Err(DomainError::new(
                    ErrorCode::UnsupportedCapability,
                    "/scaling/maximum_batches",
                ));
            }
            (
                PositiveRational(batch_yield.0.checked_mul(&Rational::new(n as u64, 1)?)?),
                Some(n as u16),
            )
        }
    };
    let multiplier = PositiveRational(divide(produced.0, r.r#yield.servings.0)?);
    if let ScalingPolicy::Linear { minimum, maximum } = r.scaling
        && (compare(multiplier.0, minimum.0)?.is_lt() || compare(multiplier.0, maximum.0)?.is_gt())
    {
        return Err(DomainError::new(
            ErrorCode::UnsupportedCapability,
            "/scaling",
        ));
    }
    let mut ingredients = Vec::new();
    for (index, i) in r.ingredients.iter().enumerate() {
        let mut q = i.quantity.0.clone().ok_or_else(|| {
            DomainError::new(
                ErrorCode::UnsupportedCapability,
                &format!("/ingredients/{index}/quantity"),
            )
        })?;
        q.amount = PositiveRational(q.amount.0.checked_mul(&multiplier.0)?);
        if i.discrete && q.amount.0.denominator.0 != 1 {
            return Err(DomainError::new(
                ErrorCode::UnsupportedCapability,
                &format!("/ingredients/{index}/quantity"),
            ));
        }
        ingredients.push(ScaledIngredient {
            ingredient_id: i.ingredient_id.clone(),
            component_id: i.component_id.clone(),
            quantity: q,
        });
    }
    let mut components = r.r#yield.components.0.clone();
    for c in &mut components {
        c.quantity.amount = PositiveRational(c.quantity.amount.0.checked_mul(&multiplier.0)?);
    }
    Ok(ScaledRecipe {
        recipe: crate::eligibility::recipe_ref(recipe)?,
        multiplier,
        requested_servings: requested,
        produced_servings: produced,
        unallocated_servings: subtract(produced.0, requested.0)?,
        discrete_batches,
        ingredients,
        components,
    })
}
pub fn convert_quantity(
    recipe: &ReviewedRecipe<'_>,
    ingredient: &IngredientId,
    quantity: &Quantity,
    to_unit: Unit,
    to_form: IngredientForm,
    evidence_id: &EvidenceId,
) -> Result<Quantity, DomainError> {
    use crate::eligibility::shape;
    shape(ingredient, "/ingredient")?;
    shape(quantity, "/quantity")?;
    shape(evidence_id, "/evidence_id")?;
    ready(recipe)?;
    let ingredients: Vec<_> = recipe
        .recipe()
        .ingredients
        .iter()
        .filter(|i| &i.ingredient_id == ingredient)
        .collect();
    if ingredients.is_empty() {
        return Err(DomainError::new(ErrorCode::ForeignReference, "/ingredient"));
    }
    // Ingredient identity can recur in different components. Every applicable
    // rule for that identity participates; the first component is not authority.
    let rules: Vec<_> = ingredients
        .iter()
        .flat_map(|i| i.conversion_evidence.iter())
        .collect();
    let selected: Vec<_> = rules
        .iter()
        .filter(|e| &e.evidence_id == evidence_id)
        .collect();
    if selected.is_empty() {
        return Err(DomainError::new(
            ErrorCode::UnsupportedCapability,
            "/conversion_evidence",
        ));
    }
    // Repeated component occurrences may carry the same admitted fact. Only
    // complete record equality permits coalescing; identity alone is not enough.
    let e = *selected[0];
    if selected.iter().any(|other| **other != e) {
        return Err(DomainError::new(
            ErrorCode::DomainConflict,
            "/conversion_evidence/evidence_id",
        ));
    }
    let direct = |e: &ConversionEvidence| {
        e.from_unit == quantity.unit
            && e.from_form == quantity.form
            && e.to_unit == to_unit
            && e.to_form == to_form
    };
    let inverse = |e: &ConversionEvidence| {
        e.to_unit == quantity.unit
            && e.to_form == quantity.form
            && e.from_unit == to_unit
            && e.from_form == to_form
    };
    if !direct(e) && !inverse(e) {
        return Err(DomainError::new(
            ErrorCode::UnsupportedCapability,
            "/conversion_evidence/endpoints",
        ));
    }
    let amount = if direct(e) {
        quantity.amount.0.checked_mul(&e.factor.0)?
    } else {
        divide(quantity.amount.0, e.factor.0)?
    };
    let ratio = |rule: &ConversionEvidence| {
        if direct(rule) {
            (rule.factor.0.numerator.0, rule.factor.0.denominator.0)
        } else {
            (rule.factor.0.denominator.0, rule.factor.0.numerator.0)
        }
    };
    let (n, d) = ratio(e);
    for other in rules.iter().filter(|e| direct(e) || inverse(e)) {
        let (on, od) = ratio(other);
        let consistent = product(n, od)? == product(on, d)?;
        if !consistent {
            return Err(DomainError::new(
                ErrorCode::DomainConflict,
                "/conversion_evidence",
            ));
        }
    }
    Ok(Quantity {
        amount: PositiveRational(amount),
        unit: to_unit,
        form: to_form,
    })
}
fn ready(recipe: &ReviewedRecipe<'_>) -> Result<(), DomainError> {
    if recipe.assessment().capabilities.shopping {
        Ok(())
    } else {
        Err(DomainError::new(
            ErrorCode::UnsupportedCapability,
            "/capabilities/shopping",
        ))
    }
}
fn reduced(mut n: u128, mut d: u128) -> Result<Rational, DomainError> {
    if d == 0 {
        return Err(DomainError::new(ErrorCode::InvalidValue, "/quantity"));
    }
    let (mut a, mut b) = (n, d);
    while b != 0 {
        (a, b) = (b, a % b);
    }
    n /= a;
    d /= a;
    if n > u128::from(MAX_NUMERATOR) || d > u128::from(MAX_DENOMINATOR) {
        return Err(DomainError::new(ErrorCode::LimitExceeded, "/quantity"));
    }
    Rational::new(n as u64, d as u64)
}
fn product(a: u64, b: u64) -> Result<u128, DomainError> {
    u128::from(a)
        .checked_mul(u128::from(b))
        .ok_or_else(|| DomainError::new(ErrorCode::ArithmeticOverflow, "/quantity"))
}
fn parts(a: Rational, b: Rational) -> Result<(u128, u128), DomainError> {
    Rational::new(a.numerator.0, a.denominator.0)?;
    Rational::new(b.numerator.0, b.denominator.0)?;
    Ok((
        product(a.numerator.0, b.denominator.0)?,
        product(b.numerator.0, a.denominator.0)?,
    ))
}
fn divide(a: Rational, b: Rational) -> Result<Rational, DomainError> {
    let (n, d) = parts(a, b)?;
    reduced(n, d)
}
fn compare(a: Rational, b: Rational) -> Result<std::cmp::Ordering, DomainError> {
    let (n, d) = parts(a, b)?;
    Ok(n.cmp(&d))
}
fn subtract(a: Rational, b: Rational) -> Result<Rational, DomainError> {
    let (n, m) = parts(a, b)?;
    let d = product(a.denominator.0, b.denominator.0)?;
    reduced(
        n.checked_sub(m)
            .ok_or_else(|| DomainError::new(ErrorCode::DomainConflict, "/quantity"))?,
        d,
    )
}
fn ceil_ratio(a: Rational, b: Rational) -> Result<u128, DomainError> {
    let (n, d) = parts(a, b)?;
    if d == 0 {
        return Err(DomainError::new(ErrorCode::InvalidValue, "/quantity"));
    }
    Ok(n / d + u128::from(n % d != 0))
}
