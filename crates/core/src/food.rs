//! Native food derivation. Inputs are explicit admitted snapshots, never transport authority.
// R3 fixes the by-value failure envelope so callers retain the complete report.
#![allow(clippy::result_large_err)]
use crate::portions::ScaledRecipe;
use crate::recipes::*;
use mstrchf_contracts::*;
mod encoding;
mod history;
mod types;
mod validation;
pub use encoding::*;
pub use history::*;
pub use types::*;
pub(crate) use validation::*;

#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)] // Required native Blocked report, not a truncated diagnostic.
pub enum FoodFailure {
    Invalid(DomainError),
    Blocked {
        conflicts: Vec<Conflict>,
        feasibility: Option<FeasibilityReport>,
    },
}
impl From<DomainError> for FoodFailure {
    fn from(e: DomainError) -> Self {
        Self::Invalid(e)
    }
}
impl FoodFailure {
    pub(crate) fn into_domain(self) -> DomainError {
        match self {
            Self::Invalid(e) => e,
            Self::Blocked { conflicts, .. } => err(
                conflicts
                    .first()
                    .map(|c| c.code)
                    .unwrap_or(ErrorCode::DomainConflict),
                "/food",
            ),
        }
    }
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoodIdentityKeys {
    pub allocations: Vec<AllocationKey>,
    pub sessions: Vec<SessionKey>,
    pub demand_lines: Vec<DemandKey>,
}
pub struct FoodInputs<'a> {
    pub evidence: &'a FoodEvidence,
    pub identities: &'a FoodIdentities,
    pub schedule: &'a ScheduleInputs,
    pub feasibility: &'a FeasibilityInputs,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FoodDerivation {
    pub plan: DerivedPlan,
    pub shopping: crate::demand::ShoppingProjection,
    pub feasibility: FeasibilityReport,
}

#[derive(Clone, Debug)]
pub(crate) struct Occurrence {
    pub key: OccurrenceKey,
    pub kind: MethodKind,
    pub servings: Vec<DinerServing>,
    pub scaled: ScaledRecipe,
    pub batch_id: BatchId,
    pub use_at: ZonedMoment,
    pub is_source: bool,
}
pub(crate) fn err(code: ErrorCode, path: &str) -> DomainError {
    DomainError::new(code, path)
}
pub(crate) fn require(ok: bool, code: ErrorCode, path: &str) -> Result<(), DomainError> {
    if ok { Ok(()) } else { Err(err(code, path)) }
}
pub(crate) fn limit(len: usize, max: usize, path: &str) -> Result<(), DomainError> {
    require(len <= max, ErrorCode::LimitExceeded, path)
}
pub(crate) fn blocked(plan: &WeekPlan, code: ErrorCode) -> FoodFailure {
    FoodFailure::Blocked {
        conflicts: vec![Conflict {
            code,
            week_ids: BoundedVec(vec![plan.week_id.clone()]),
            slot_ids: BoundedVec(vec![]),
            recipe_refs: BoundedVec(vec![]),
            evidence_ids: BoundedVec(vec![]),
            blocking: true,
        }],
        feasibility: None,
    }
}

pub(crate) fn reviewed<'a>(
    context: &'a PlanningContext,
    evidence: &'a FoodEvidence,
    reference: &RecipeRef,
) -> Result<ReviewedRecipe<'a>, DomainError> {
    let recipe = context
        .recipes
        .iter()
        .find(|r| r.recipe_id == reference.recipe_id && r.revision == reference.revision)
        .ok_or_else(|| err(ErrorCode::MissingReference, "/recipe"))?;
    require(
        recipe_digest(recipe)? == reference.digest,
        ErrorCode::DigestMismatch,
        "/recipe/digest",
    )?;
    let basis = evidence
        .recipes
        .iter()
        .find(|r| r.recipe == *reference)
        .ok_or_else(|| err(ErrorCode::MissingReference, "/evidence/recipes"))?;
    let classes = evidence
        .classes
        .iter()
        .find(|r| r.recipe == *reference)
        .ok_or_else(|| err(ErrorCode::MissingReference, "/evidence/classes"))?;
    assess_revision(recipe, basis, classes)
}
fn eligibility_result(result: crate::eligibility::Eligibility) -> Result<(), FoodFailure> {
    match result {
        crate::eligibility::Eligibility::Eligible => Ok(()),
        crate::eligibility::Eligibility::Ineligible(conflicts)
        | crate::eligibility::Eligibility::Unresolved(conflicts) => Err(FoodFailure::Blocked {
            conflicts,
            feasibility: None,
        }),
    }
}
fn sum_servings(s: &[DinerServing]) -> Result<PositiveRational, DomainError> {
    let mut total = Rational::new(0, 1)?;
    for v in s {
        total = total.checked_add(&v.servings.0)?;
    }
    PositiveRational::new(total.numerator.0, total.denominator.0)
}

/// Reconstruct occurrence identity and servings from admitted A/B inputs. No fixture IDs.
pub(crate) fn occurrences(
    plan: &WeekPlan,
    context: &PlanningContext,
    evidence: &FoodEvidence,
) -> Result<Vec<Occurrence>, FoodFailure> {
    shape(plan)?;
    shape(context)?;
    validate_evidence_shape(evidence)?;
    require(
        evidence.catalogue_revision == context.stamp.catalogue_revision,
        ErrorCode::ContextStale,
        "/catalogue_revision",
    )?;
    require(
        food_evidence_digest(evidence)? == evidence.catalogue_digest,
        ErrorCode::DigestMismatch,
        "/catalogue_digest",
    )?;
    // Every supplied record must resolve, including the chosen counterpart
    // method retained for a recipe even when this plan does not execute it.
    for basis in &evidence.recipes {
        let r = reviewed(context, evidence, &basis.recipe)?;
        for method in evidence.methods.iter().filter(|m| m.recipe == basis.recipe) {
            require(
                method
                    .prep_ids
                    .iter()
                    .all(|id| r.recipe().prep.iter().any(|p| p.prep_id == *id)),
                ErrorCode::MissingReference,
                "/methods/prep_ids",
            )?;
            if method.kind == MethodKind::Reuse {
                require(
                    method.direct_reuse_counts.len() == r.recipe().components.len()
                        && method.direct_reuse_counts.iter().all(|c| {
                            r.recipe()
                                .components
                                .iter()
                                .any(|s| s.component_id == c.component_id)
                        }),
                    ErrorCode::MissingReference,
                    "/direct_reuse_counts",
                )?;
            }
        }
        for resource in evidence
            .resources
            .iter()
            .filter(|m| m.recipe == basis.recipe)
        {
            require(
                r.recipe()
                    .prep
                    .iter()
                    .any(|p| p.prep_id == resource.prep_id),
                ErrorCode::MissingReference,
                "/resources/prep_id",
            )?;
        }
    }
    let mut out = Vec::new();
    let mut sources = std::collections::BTreeSet::new();
    for s in plan.selections.iter() {
        require(
            s.slot.week_id == plan.week_id,
            ErrorCode::ForeignReference,
            "/slot",
        )?;
        let meal = context
            .requirements
            .iter()
            .find(|m| m.slot == s.slot)
            .ok_or_else(|| err(ErrorCode::MissingReference, "/requirements"))?;
        if !meal.enabled || meal.location == MealLocation::Skip {
            continue;
        }
        let r = reviewed(context, evidence, &s.recipe)?;
        eligibility_result(crate::eligibility::eligible(&r, meal)?)?;
        let servings = crate::portions::resolve_servings(&r, meal, &s.meal_overrides).map_err(
            |conflicts| FoodFailure::Blocked {
                conflicts,
                feasibility: None,
            },
        )?;
        require(
            servings == s.servings.0,
            ErrorCode::IdentityMismatch,
            "/servings",
        )?;
        let use_at = s
            .meal_overrides
            .eat_by
            .0
            .as_ref()
            .or(meal.eat_by.0.as_ref())
            .ok_or_else(|| err(ErrorCode::UnsupportedCapability, "/eat_by"))?
            .clone();
        shape(&use_at)?;
        let requested = sum_servings(&servings)?;
        let (batch_id, is_source, kind, scaled) = match &s.batch_use {
            BatchUse::NewBatch {
                batch_id,
                production_servings,
            } => {
                require(
                    sources.insert(batch_id),
                    ErrorCode::InvalidValue,
                    "/batch_id",
                )?;
                require(
                    r.recipe().reused_inputs.is_empty(),
                    ErrorCode::UnsupportedCapability,
                    "/reused_inputs",
                )?;
                let scaled = crate::portions::scale_recipe(&r, *production_servings)?;
                require(
                    scaled.produced_servings == *production_servings,
                    ErrorCode::DomainConflict,
                    "/production_servings",
                )?;
                (batch_id.clone(), true, MethodKind::Production, scaled)
            }
            BatchUse::Leftover { batch_id, .. } => {
                require(
                    meal.leftovers_allowed,
                    ErrorCode::DomainConflict,
                    "/leftovers_allowed",
                )?;
                let kind = if r.recipe().reused_inputs.is_empty() {
                    MethodKind::Reuse
                } else {
                    MethodKind::Production
                };
                let scaled = if kind == MethodKind::Reuse {
                    // Direct portions do not repeat discrete ingredient production.
                    let y = r.recipe().r#yield.servings.0;
                    let mut n = u128::from(requested.0.numerator.0) * u128::from(y.denominator.0);
                    let mut den = u128::from(requested.0.denominator.0) * u128::from(y.numerator.0);
                    let (mut a, mut b) = (n, den);
                    while b != 0 {
                        (a, b) = (b, a % b);
                    }
                    n /= a;
                    den /= a;
                    require(
                        n <= u128::from(MAX_NUMERATOR) && den <= u128::from(MAX_DENOMINATOR),
                        ErrorCode::LimitExceeded,
                        "/servings",
                    )?;
                    let multiplier = PositiveRational::new(n as u64, den as u64)?;
                    let mut components = r.recipe().r#yield.components.0.clone();
                    for c in &mut components {
                        c.quantity.amount =
                            PositiveRational(c.quantity.amount.0.checked_mul(&multiplier.0)?);
                    }
                    ScaledRecipe {
                        recipe: s.recipe.clone(),
                        multiplier,
                        requested_servings: requested,
                        produced_servings: requested,
                        unallocated_servings: Rational::new(0, 1)?,
                        discrete_batches: None,
                        ingredients: vec![],
                        components,
                    }
                } else {
                    let scaled = crate::portions::scale_recipe(&r, requested)?;
                    require(
                        scaled.produced_servings == requested,
                        ErrorCode::UnsupportedCapability,
                        "/remix/production_servings",
                    )?;
                    scaled
                };
                (batch_id.clone(), false, kind, scaled)
            }
        };
        out.push(Occurrence {
            key: OccurrenceKey {
                slot: s.slot.clone(),
                extra_id: None,
                recipe: s.recipe.clone(),
            },
            kind,
            servings,
            scaled,
            batch_id: batch_id.clone(),
            use_at: use_at.clone(),
            is_source,
        });
        for extra in s.extras.iter() {
            let e = reviewed(context, evidence, &extra.recipe)?;
            eligibility_result(crate::eligibility::validate_extra(
                &r,
                &e,
                extra,
                meal,
                &s.meal_overrides,
            )?)?;
            require(
                e.recipe().reused_inputs.is_empty(),
                ErrorCode::UnsupportedCapability,
                "/extra/reused_inputs",
            )?;
            let mut extra_meal = meal.clone();
            extra_meal
                .attendees
                .0
                .retain(|a| extra.recipients.iter().any(|v| v.person == a.person));
            let mut overrides = s.meal_overrides.clone();
            overrides
                .portions
                .0
                .retain(|p| extra.recipients.iter().any(|v| v.person == p.person));
            let resolved = crate::portions::resolve_servings(&e, &extra_meal, &overrides).map_err(
                |conflicts| FoodFailure::Blocked {
                    conflicts,
                    feasibility: None,
                },
            )?;
            require(
                resolved == extra.recipients.0,
                ErrorCode::IdentityMismatch,
                "/extra/recipients",
            )?;
            let scaled = crate::portions::scale_recipe(&e, sum_servings(&resolved)?)?;
            out.push(Occurrence {
                key: OccurrenceKey {
                    slot: s.slot.clone(),
                    extra_id: Some(extra.extra_id.clone()),
                    recipe: extra.recipe.clone(),
                },
                kind: MethodKind::Production,
                servings: resolved,
                scaled,
                batch_id: batch_id.clone(),
                use_at: use_at.clone(),
                is_source: false,
            });
        }
    }
    out.sort_by(|a, b| occurrence_cmp(&a.key, &b.key));
    let mut refs = out.iter().map(|o| o.key.recipe.clone()).collect::<Vec<_>>();
    refs.sort_by(recipe_cmp);
    refs.dedup();
    require(
        evidence.recipes.iter().map(|b| &b.recipe).eq(refs.iter()),
        ErrorCode::IdentityMismatch,
        "/evidence/recipe_closure",
    )?;
    for o in &out {
        let r = reviewed(context, evidence, &o.key.recipe)?;
        let method = evidence
            .methods
            .iter()
            .find(|m| m.recipe == o.key.recipe && m.kind == o.kind)
            .ok_or_else(|| err(ErrorCode::UnsupportedCapability, "/methods"))?;
        for id in &method.prep_ids {
            require(
                r.recipe().prep.iter().any(|p| p.prep_id == *id),
                ErrorCode::MissingReference,
                "/methods/prep_ids",
            )?;
            require(
                evidence
                    .resources
                    .iter()
                    .any(|p| p.recipe == o.key.recipe && p.prep_id == *id),
                ErrorCode::MissingReference,
                "/resources",
            )?;
        }
        if method.kind == MethodKind::Reuse {
            require(
                method.direct_reuse_counts.len() == r.recipe().components.len()
                    && method.direct_reuse_counts.iter().all(|c| {
                        r.recipe()
                            .components
                            .iter()
                            .any(|s| s.component_id == c.component_id)
                    }),
                ErrorCode::MissingReference,
                "/direct_reuse_counts",
            )?;
        }
    }
    Ok(out)
}

pub fn required_identity_keys(
    plan: &WeekPlan,
    context: &PlanningContext,
    evidence: &FoodEvidence,
    schedule: &ScheduleInputs,
) -> Result<FoodIdentityKeys, FoodFailure> {
    occurrences(plan, context, evidence)?;
    validate_schedule_shape(schedule)?;
    Ok(FoodIdentityKeys {
        allocations: crate::batches::allocation_keys(plan, context, evidence)?,
        sessions: crate::prep::validate_layout(plan, context, evidence, &schedule.layout)?,
        demand_lines: crate::demand::demand_keys(plan, context, evidence)?,
    })
}
fn check_identity_keys(
    plan: &WeekPlan,
    context: &PlanningContext,
    evidence: &FoodEvidence,
    identities: &FoodIdentities,
    schedule: &ScheduleInputs,
) -> Result<(), FoodFailure> {
    require(
        food_identities_digest(identities)? == identities.digest,
        ErrorCode::DigestMismatch,
        "/identities/digest",
    )?;
    let keys = required_identity_keys(plan, context, evidence, schedule)?;
    require(
        identities
            .allocations
            .iter()
            .map(|x| &x.0)
            .eq(keys.allocations.iter())
            && identities
                .sessions
                .iter()
                .map(|x| &x.0)
                .eq(keys.sessions.iter())
            && identities
                .demand_lines
                .iter()
                .map(|x| &x.0)
                .eq(keys.demand_lines.iter()),
        ErrorCode::IdentityMismatch,
        "/identities/keys",
    )?;
    Ok(())
}
fn core_plan(
    plan: &WeekPlan,
    context: &PlanningContext,
    evidence: &FoodEvidence,
    identities: &FoodIdentities,
    schedule: &ScheduleInputs,
) -> Result<DerivedPlan, FoodFailure> {
    check_identity_keys(plan, context, evidence, identities, schedule)?;
    let input_plan_digest = plan_input_digest(&PlanInputMaterial {
        schema_version: SchemaVersion,
        week_plan: plan.clone(),
        context_stamp: context.stamp.clone(),
    })?;
    let food_inputs_digest =
        food_inputs_digest(&input_plan_digest, evidence, identities, schedule)?;
    let (sessions, bindings) =
        crate::prep::derive_schedule(plan, context, evidence, identities, schedule)?;
    let (batches, allocations) =
        crate::batches::derive_batches(plan, context, evidence, identities, &sessions, &bindings)?;
    let demand =
        crate::demand::derive_demand(plan, context, evidence, identities, &input_plan_digest)?;
    let mut derived = DerivedPlan {
        schema_version: SchemaVersion,
        week_id: plan.week_id.clone(),
        plan_revision: plan.revision,
        input_plan_digest,
        food_inputs_digest,
        context_stamp: context.stamp.clone(),
        recipe_refs: BoundedVec(evidence.recipes.iter().map(|r| r.recipe.clone()).collect()),
        batches: BoundedVec(batches),
        allocations: BoundedVec(allocations),
        prep_sessions: BoundedVec(sessions),
        demand,
        warnings: BoundedVec(vec![]),
        derivation_policy_version: evidence.policy_version,
        digest: PayloadDigest("0".repeat(64)),
    };
    derived.digest = derived_digest(&DerivedMaterial::from(&derived))?;
    shape(&derived)?;
    Ok(derived)
}
pub fn derive_plan(
    plan: &WeekPlan,
    context: &PlanningContext,
    inputs: &FoodInputs<'_>,
) -> Result<FoodDerivation, FoodFailure> {
    let derived = core_plan(
        plan,
        context,
        inputs.evidence,
        inputs.identities,
        inputs.schedule,
    )?;
    let saved = SavedFoodBasis {
        plan: plan.clone(),
        context: context.clone(),
        evidence: inputs.evidence.clone(),
        identities: inputs.identities.clone(),
        schedule: inputs.schedule.clone(),
    };
    let feasibility = crate::demand::assess_feasibility(&derived, &saved, inputs.feasibility)?;
    if !feasibility.blocking.is_empty() {
        return Err(FoodFailure::Blocked {
            conflicts: feasibility
                .blocking
                .iter()
                .map(|s| s.conflict.clone())
                .collect(),
            feasibility: Some(feasibility),
        });
    }
    let shopping = crate::demand::project_essentials(
        &derived.demand,
        &inputs.feasibility.confirmations,
        &inputs.feasibility.evaluated_at,
    )?;
    Ok(FoodDerivation {
        plan: derived,
        shopping,
        feasibility,
    })
}
pub(crate) fn verify_core(
    derived: &DerivedPlan,
    saved: &SavedFoodBasis,
) -> Result<(), DomainError> {
    validate_saved_shape(saved)?;
    let expected = core_plan(
        &saved.plan,
        &saved.context,
        &saved.evidence,
        &saved.identities,
        &saved.schedule,
    )
    .map_err(FoodFailure::into_domain)?;
    require(&expected == derived, ErrorCode::DigestMismatch, "/derived")
}
pub use crate::demand::{assess_feasibility, project_essentials};
