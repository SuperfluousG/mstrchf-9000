//! Conditional native product matching. Trusted producers attest source meaning,
//! policy and currentness; this module verifies scope and custody, not source truth.
//! No retailer access, title interpretation, pack optimisation or checkout authority.
use crate::food::*;
use crate::recipes::EvidencePin;
use mstrchf_contracts::*;
mod encoding;
mod validation;
pub use encoding::{match_basis_digest, match_basis_material, meaning_digest, policy_digest};

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum SourceField {
    Ingredients,
    Allergens,
    Storage,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ClaimState {
    Supported,
    Refuted,
    Unknown,
    Contradictory,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RuleBinding {
    pub requirement: HardRequirement,
    pub required_fields: Vec<SourceField>,
    pub review: EvidencePin,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchPolicy {
    pub version: Revision,
    pub digest: PayloadDigest,
    pub rules: Vec<RuleBinding>,
}
#[derive(Clone, Debug)]
pub struct ProductReadBasis<'a> {
    pub request: &'a JobRequestV1,
    pub expected_reader: &'a ReadActor,
    pub observation: &'a ReadObservationV1,
    pub accepted: &'a AcceptedReadRef,
    pub product: ProductRef,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct IdentityClaim {
    pub supply_key: DemandKey,
    pub state: ClaimState,
    pub evidence_ids: Vec<EvidenceId>,
    pub source_fields: Vec<SourceField>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RequirementClaim {
    pub requirement: HardRequirement,
    pub state: ClaimState,
    pub evidence_ids: Vec<EvidenceId>,
    pub source_fields: Vec<SourceField>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CookingClaim {
    pub recipe: RecipeRef,
    pub occurrence: OccurrenceKey,
    pub state: ClaimState,
    pub evidence_ids: Vec<EvidenceId>,
    pub source_fields: Vec<SourceField>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductMeaning {
    pub product: ProductRef,
    pub observation_digest: Digest,
    pub key_claim: IdentityClaim,
    pub requirement_claims: Vec<RequirementClaim>,
    pub cooking_claims: Vec<CookingClaim>,
}
/// The caller attests retained, current, nonwithdrawn evidence and source-specific
/// expiry. Hashes cannot establish those facts. Untrusted JSON cannot self-admit.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProductMeaningAdmission {
    pub review: EvidencePin,
    pub evidence: Vec<EvidencePin>,
    pub meaning_digest: PayloadDigest,
    pub policy_digest: PayloadDigest,
    pub freshness_rule: EvidencePin,
    pub meaning_admitted_at: Instant,
    pub valid_until: Instant,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SupplyRelation {
    Exact,
    ForwardConversion {
        evidence: Vec<(ConversionEvidence, EvidencePin)>,
    },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SupplyOrigin {
    pub occurrence: OccurrenceKey,
    pub batch_id: BatchId,
    pub component_id: ComponentId,
    pub ingredient_id: IngredientId,
    pub original_quantity: Quantity,
    pub relation: SupplyRelation,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchUse {
    pub use_key: UseKey,
    pub at: ZonedMoment,
    pub requirements: Vec<HardRequirement>,
    pub cooking: CookingClaim,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchBasis {
    pub saved_basis_digest: PayloadDigest,
    pub derived_digest: PayloadDigest,
    pub demand_digest: DemandDigest,
    pub context: ContextStamp,
    pub policy_digest: PayloadDigest,
    pub line_id: DemandLineId,
    pub line: DemandLine,
    pub product: ProductRef,
    pub observation_digest: Digest,
    pub meaning_digest: PayloadDigest,
    pub review: EvidencePin,
    pub evidence: Vec<EvidencePin>,
    pub freshness_rule: EvidencePin,
    pub item_observed_at: Instant,
    pub read_observed_at: Instant,
    pub read_admitted_at: Instant,
    pub meaning_admitted_at: Instant,
    pub valid_until: Instant,
    pub evaluated_at: Instant,
    pub uses: Vec<MatchUse>,
    pub supply_key: DemandKey,
    pub origins: Vec<SupplyOrigin>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MatchReason {
    MissingMeaning,
    MissingAdmission,
    StaleMeaning,
    UnknownIdentity,
    ContradictoryIdentity,
    RefutedIdentity,
    UnknownRequirement,
    ContradictoryRequirement,
    RefutedRequirement,
    UnsupportedPolicy,
    UnknownCooking,
    ContradictoryCooking,
    ChangedCooking,
    UnsupportedSupplyRelation,
    ConflictingSupplyRelation,
    PackMeasureMismatch,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchIssue {
    pub line_id: DemandLineId,
    pub product: ProductRef,
    pub requirement: Option<HardRequirement>,
    pub uses: Vec<UseKey>,
    pub reason: MatchReason,
    pub evidence_ids: Vec<EvidenceId>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)] // Native complete candidate, not a transport record.
pub enum MatchAssessment {
    Suitable(MatchedCandidate),
    Unsuitable(Vec<MatchIssue>),
    Unresolved(Vec<MatchIssue>),
}
/// Only the assessment route can construct a candidate. Reassess against current
/// native state before consuming a retained result; it is not an authority token.
/// Every field is supplied so this fails for privacy, not missing fields.
/// ```compile_fail,E0451
/// use mstrchf_contracts::{PayloadDigest, ProductObservation};
/// use mstrchf_core::products::{MatchBasis, MatchedCandidate};
/// fn forge(basis: MatchBasis, observation: ProductObservation, digest: PayloadDigest)
///     -> MatchedCandidate
/// {
///     MatchedCandidate { basis, observation, digest }
/// }
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MatchedCandidate {
    basis: MatchBasis,
    observation: ProductObservation,
    digest: PayloadDigest,
}
impl MatchedCandidate {
    pub fn basis(&self) -> &MatchBasis {
        &self.basis
    }
    pub fn observation(&self) -> &ProductObservation {
        &self.observation
    }
    pub fn digest(&self) -> &PayloadDigest {
        &self.digest
    }
}
/// Only validated current native state can create a matching context.
/// ```compile_fail,E0451
/// use mstrchf_contracts::{Instant, PayloadDigest, PlanningContext, DerivedPlan};
/// use mstrchf_core::food::{SavedFoodBasis, UseGraph};
/// use mstrchf_core::products::{MatchContext, MatchPolicy, SupplyOrigin};
/// fn forge<'a>(
///     current: &'a PlanningContext, saved: &'a SavedFoodBasis,
///     derived: &'a DerivedPlan, policy: &'a MatchPolicy, evaluated_at: Instant,
///     saved_digest: PayloadDigest, graph: UseGraph, origins: Vec<SupplyOrigin>,
/// ) -> MatchContext<'a> {
///     MatchContext { current, saved, derived, policy, evaluated_at,
///         saved_digest, graph, origins }
/// }
/// ```
#[derive(Debug)]
pub struct MatchContext<'a> {
    current: &'a PlanningContext,
    saved: &'a SavedFoodBasis,
    derived: &'a DerivedPlan,
    policy: &'a MatchPolicy,
    evaluated_at: Instant,
    saved_digest: PayloadDigest,
    graph: UseGraph,
    origins: Vec<SupplyOrigin>,
}
/// Only source and meaning admission can create an assessed product.
/// ```compile_fail,E0451
/// use mstrchf_contracts::{Instant, PayloadDigest, ProductObservation};
/// use mstrchf_core::products::{
///     ProductReadBasis, ProductMeaning, ProductMeaningAdmission, ReviewedProduct,
/// };
/// fn forge<'a>(
///     source: &'a ProductReadBasis<'a>, item: &'a ProductObservation,
///     meaning: Option<&'a ProductMeaning>, admission: Option<&'a ProductMeaningAdmission>,
///     context_digest: PayloadDigest, evaluated_at: Instant, policy_digest: PayloadDigest,
/// ) -> ReviewedProduct<'a> {
///     ReviewedProduct { source, item, meaning, admission,
///         context_digest, evaluated_at, policy_digest }
/// }
/// ```
#[derive(Debug)]
pub struct ReviewedProduct<'a> {
    source: &'a ProductReadBasis<'a>,
    item: &'a ProductObservation,
    meaning: Option<&'a ProductMeaning>,
    admission: Option<&'a ProductMeaningAdmission>,
    context_digest: PayloadDigest,
    evaluated_at: Instant,
    policy_digest: PayloadDigest,
}
pub fn prepare_match_context<'a>(
    current: &'a PlanningContext,
    saved: &'a SavedFoodBasis,
    derived: &'a DerivedPlan,
    policy: &'a MatchPolicy,
    evaluated_at: Instant,
) -> Result<MatchContext<'a>, DomainError> {
    // Validate the full current producer projection before any consumer filtering.
    crate::selection::validate_selection_context(current)?;
    require(
        current == &saved.context,
        ErrorCode::ContextStale,
        "/products/current",
    )?;
    require(
        current
            .weeks
            .iter()
            .any(|w| w.range.week_id == saved.plan.week_id && w.plan == saved.plan),
        ErrorCode::ContextStale,
        "/products/selected_plan",
    )?;
    limit(derived.demand.lines.len(), 4096, "/products/demand")?;
    validate_evidence_shape(&saved.evidence)?;
    validate_identities_shape(&saved.identities)?;
    validate_schedule_shape(&saved.schedule)?;
    shape(&evaluated_at)?;
    require(
        policy_digest(policy)? == policy.digest,
        ErrorCode::DigestMismatch,
        "/products/policy/digest",
    )?;
    // Count unscaled ingredient rows before the existing derivation expands
    // recipes, servings and uses. Direct leftovers do not purchase new rows.
    let mut origin_count = 0usize;
    for selection in &saved.plan.selections.0 {
        let refs = (!matches!(selection.batch_use, BatchUse::Leftover { .. }))
            .then_some(&selection.recipe)
            .into_iter()
            .chain(selection.extras.iter().map(|e| &e.recipe));
        for reference in refs {
            let recipe = current
                .recipes
                .iter()
                .find(|r| r.recipe_id == reference.recipe_id && r.revision == reference.revision)
                .ok_or_else(|| err(ErrorCode::MissingReference, "/products/origins/recipe"))?;
            origin_count = origin_count
                .checked_add(recipe.ingredients.len())
                .ok_or_else(|| err(ErrorCode::ArithmeticOverflow, "/products/origins"))?;
            limit(origin_count, 16384, "/products/origins")?;
        }
    }
    // This strict reconstruction propagates missing mandatory recipe conversion
    // pins. It must not be caught and downgraded to an unsupported relation.
    let graph = crate::demand::reconstruct_use_graph(derived, saved)?;
    limit(graph.uses.len(), 16384, "/products/uses")?;
    let occurrences =
        occurrences(&saved.plan, current, &saved.evidence).map_err(FoodFailure::into_domain)?;
    let mut origins = vec![];
    for o in occurrences
        .iter()
        .filter(|o| o.kind == MethodKind::Production)
    {
        limit(
            origins
                .len()
                .checked_add(o.scaled.ingredients.len())
                .ok_or_else(|| err(ErrorCode::ArithmeticOverflow, "/products/origins"))?,
            16384,
            "/products/origins",
        )?;
        for i in &o.scaled.ingredients {
            origins.push(SupplyOrigin {
                occurrence: o.key.clone(),
                batch_id: o.batch_id.clone(),
                component_id: i.component_id.clone(),
                ingredient_id: i.ingredient_id.clone(),
                original_quantity: i.quantity.clone(),
                relation: SupplyRelation::Exact,
            });
        }
    }
    origins.sort_by(validation::origin_cmp);
    ordered(&origins, validation::origin_cmp, "/products/origins")?;
    let c = MatchContext {
        current,
        saved,
        derived,
        policy,
        evaluated_at,
        saved_digest: saved_basis_digest(saved)?,
        graph,
        origins,
    };
    // Require complete nonempty consumers and bounded requirement unions for
    // every demand line, even if a caller subsequently asks about only one.
    for line in &derived.demand.lines.0 {
        let rows = c.line_origins(line);
        validation::conserve(line, &rows)?;
        let uses = c.line_uses(line)?;
        require(
            uses.iter()
                .any(|(u, _)| matches!(u.use_key, UseKey::Consumption { .. })),
            ErrorCode::MissingReference,
            "/products/consumers",
        )?;
    }
    Ok(c)
}

pub fn assess_product_meaning<'a>(
    source: &'a ProductReadBasis<'a>,
    meaning: Option<&'a ProductMeaning>,
    admission: Option<&'a ProductMeaningAdmission>,
    context: &MatchContext<'_>,
) -> Result<ReviewedProduct<'a>, DomainError> {
    // Bound returned products and their nested lists before shape serialisation.
    validation::read_bounds(source)?;
    shape(source.observation)?;
    shape(source.accepted)?;
    shape(&source.product)?;
    shape(source.expected_reader)?;
    shape(source.request)?;
    validate_read_shape(source.observation)?;
    validate_request_shape(source.request)?;
    validate_read_binding(
        source.observation,
        source.request,
        source.expected_reader,
        source.accepted.admitted_at.clone(),
    )?;
    let o = source.observation;
    let a = source.accepted;
    require(
        a.job_id == o.job_id
            && a.read_id == o.read_id
            && a.observation_id == o.observation_id
            && a.resource == o.resource
            && a.observed_at == o.observed_at,
        ErrorCode::IdentityMismatch,
        "/products/accepted",
    )?;
    require(
        a.digest == canonical_digest(o)?,
        ErrorCode::DigestMismatch,
        "/products/accepted/digest",
    )?;
    let ReadBody::Products {
        delivery_context,
        items,
    } = &o.body
    else {
        return Err(err(ErrorCode::ScopeViolation, "/products/read/body"));
    };
    let ExternalResource::Basket { key } = &o.resource else {
        return Err(err(ErrorCode::ScopeViolation, "/products/read/resource"));
    };
    for item in &items.0 {
        require(
            item.product.retailer == key.retailer
                && item.product.retailer == delivery_context.retailer
                && item.delivery_context == *delivery_context,
            ErrorCode::IdentityMismatch,
            "/products/retailer",
        )?;
        check_pack_quantities(item)?;
    }
    let item = items
        .iter()
        .find(|i| i.product == source.product)
        .ok_or_else(|| err(ErrorCode::MissingReference, "/products/selected"))?;
    require(
        item.observed_at <= o.observed_at
            && o.observed_at <= a.admitted_at
            && a.admitted_at <= context.evaluated_at,
        ErrorCode::InvalidValue,
        "/products/read/chronology",
    )?;
    if let Some(m) = meaning {
        validation::meaning(m)?;
        require(
            m.product == source.product,
            ErrorCode::IdentityMismatch,
            "/products/meaning/product",
        )?;
        require(
            m.observation_digest == a.digest,
            ErrorCode::DigestMismatch,
            "/products/meaning/observation",
        )?;
        for c in &m.cooking_claims {
            require(
                context
                    .graph
                    .uses
                    .iter()
                    .any(|u| use_occurrence(&u.use_key) == &c.occurrence)
                    || context.origins.iter().any(|o| o.occurrence == c.occurrence),
                ErrorCode::ForeignReference,
                "/products/meaning/cooking/occurrence",
            )?;
        }
    }
    if let Some(ad) = admission {
        validation::admission(ad)?;
        require(
            ad.policy_digest == context.policy.digest,
            ErrorCode::DigestMismatch,
            "/products/admission/policy",
        )?;
        require(
            a.admitted_at <= ad.meaning_admitted_at
                && ad.meaning_admitted_at <= context.evaluated_at,
            ErrorCode::InvalidValue,
            "/products/admission/chronology",
        )?;
        let m = meaning
            .ok_or_else(|| err(ErrorCode::MissingReference, "/products/admission/meaning"))?;
        require(
            ad.meaning_digest == meaning_digest(m)?,
            ErrorCode::DigestMismatch,
            "/products/admission/meaning_digest",
        )?;
        for id in m
            .key_claim
            .evidence_ids
            .iter()
            .chain(m.requirement_claims.iter().flat_map(|c| &c.evidence_ids))
            .chain(m.cooking_claims.iter().flat_map(|c| &c.evidence_ids))
        {
            require(
                ad.evidence.iter().any(|p| p.evidence_id == *id),
                ErrorCode::MissingReference,
                "/products/admission/claim_evidence",
            )?;
        }
        // Same evidence ID with different current rule bytes is a forged binding,
        // not simply an absent applicable rule.
        for r in &context.policy.rules {
            if let Some(p) = ad
                .evidence
                .iter()
                .find(|p| p.evidence_id == r.review.evidence_id)
            {
                require(
                    p == &r.review,
                    ErrorCode::DigestMismatch,
                    "/products/admission/rule_pin",
                )?;
            }
        }
    }
    Ok(ReviewedProduct {
        source,
        item,
        meaning,
        admission,
        context_digest: context.saved_digest.clone(),
        evaluated_at: context.evaluated_at.clone(),
        policy_digest: context.policy.digest.clone(),
    })
}
pub fn match_product(
    context: &MatchContext<'_>,
    line_id: &DemandLineId,
    product: &ReviewedProduct<'_>,
) -> Result<MatchAssessment, DomainError> {
    shape(line_id)?;
    require(
        product.context_digest == context.saved_digest
            && product.evaluated_at == context.evaluated_at
            && product.policy_digest == context.policy.digest,
        ErrorCode::ContextStale,
        "/products/reviewed_context",
    )?;
    let line = context
        .derived
        .demand
        .lines
        .iter()
        .find(|l| &l.line_id == line_id)
        .ok_or_else(|| err(ErrorCode::MissingReference, "/products/line"))?;
    let scoped = context.line_uses(line)?;
    let keys: Vec<_> = scoped.iter().map(|(u, _)| u.use_key.clone()).collect();
    let mut issues = Issues {
        line_id: line_id.clone(),
        product: product.item.product.clone(),
        all_uses: keys,
        values: vec![],
        refuted: false,
    };
    let Some(m) = product.meaning else {
        issues.add(MatchReason::MissingMeaning, None, None, &[], false);
        return Ok(issues.finish());
    };
    let Some(a) = product.admission else {
        issues.add(MatchReason::MissingAdmission, None, None, &[], false);
        return Ok(issues.finish());
    };
    if context.evaluated_at >= a.valid_until {
        issues.add(
            MatchReason::StaleMeaning,
            None,
            None,
            std::slice::from_ref(&a.freshness_rule.evidence_id),
            false,
        );
        return Ok(issues.finish());
    }
    if context.policy.version != Revision(1) {
        issues.add(MatchReason::UnsupportedPolicy, None, None, &[], false);
        return Ok(issues.finish());
    }
    let identity = raw_state(
        product.item,
        &m.key_claim.source_fields,
        if m.key_claim
            .source_fields
            .contains(&SourceField::Ingredients)
        {
            m.key_claim.state
        } else {
            ClaimState::Unknown
        },
    );
    match identity {
        ClaimState::Supported if m.key_claim.supply_key.ingredient_id == line.ingredient_id => (),
        ClaimState::Supported | ClaimState::Refuted => issues.add(
            MatchReason::RefutedIdentity,
            None,
            None,
            &m.key_claim.evidence_ids,
            true,
        ),
        ClaimState::Unknown => issues.add(
            MatchReason::UnknownIdentity,
            None,
            None,
            &m.key_claim.evidence_ids,
            false,
        ),
        ClaimState::Contradictory => issues.add(
            MatchReason::ContradictoryIdentity,
            None,
            None,
            &m.key_claim.evidence_ids,
            false,
        ),
    }
    // One aggregated line is governed by the union of its actual consumers.
    let mut requirements = vec![];
    for (_, rs) in &scoped {
        for r in rs {
            if !requirements.contains(r) {
                requirements.push(r.clone());
            }
        }
    }
    requirements.sort_by(validation::requirement_cmp);
    for r in &requirements {
        let uses: Vec<_> = scoped
            .iter()
            .filter(|(_, rs)| rs.contains(r))
            .map(|(u, _)| u.use_key.clone())
            .collect();
        let claim = m.requirement_claims.iter().find(|c| c.requirement == *r);
        let rule = context.policy.rules.iter().find(|b| b.requirement == *r);
        let ids = claim.map(|c| c.evidence_ids.as_slice()).unwrap_or(&[]);
        let Some(rule) = rule.filter(|b| a.evidence.contains(&b.review)) else {
            issues.add(
                MatchReason::UnsupportedPolicy,
                Some(r),
                Some(&uses),
                ids,
                false,
            );
            continue;
        };
        let state = claim
            .map(|c| {
                if rule
                    .required_fields
                    .iter()
                    .all(|f| c.source_fields.contains(f))
                {
                    c.state
                } else {
                    ClaimState::Unknown
                }
            })
            .unwrap_or(ClaimState::Unknown);
        // An admitted claim may cite more sources than the policy minimum.
        // Every cited dependency remains relevant to its interpretation.
        let mut fields = rule.required_fields.clone();
        if let Some(claim) = claim {
            fields.extend_from_slice(&claim.source_fields);
        }
        fields.sort();
        fields.dedup();
        match raw_state(product.item, &fields, state) {
            ClaimState::Supported => (),
            ClaimState::Refuted => issues.add(
                MatchReason::RefutedRequirement,
                Some(r),
                Some(&uses),
                ids,
                true,
            ),
            ClaimState::Unknown => issues.add(
                MatchReason::UnknownRequirement,
                Some(r),
                Some(&uses),
                ids,
                false,
            ),
            ClaimState::Contradictory => issues.add(
                MatchReason::ContradictoryRequirement,
                Some(r),
                Some(&uses),
                ids,
                false,
            ),
        }
    }
    let mut origins = context.line_origins(line);
    if identity == ClaimState::Supported
        && m.key_claim.supply_key.ingredient_id == line.ingredient_id
    {
        check_supply(context, &m.key_claim.supply_key, &mut origins, &mut issues)?;
        let pack = if m.key_claim.supply_key.form == IngredientForm::Drained {
            &product.item.pack.drained
        } else {
            &product.item.pack.net
        };
        if let Fact::Known { value } = pack
            && (value.unit != m.key_claim.supply_key.unit
                || value.form != m.key_claim.supply_key.form)
        {
            issues.add(
                MatchReason::PackMeasureMismatch,
                None,
                None,
                &m.key_claim.evidence_ids,
                false,
            );
        }
    }
    let mut occurrence_keys: Vec<_> = scoped
        .iter()
        .map(|(u, _)| use_occurrence(&u.use_key).clone())
        .chain(origins.iter().map(|o| o.occurrence.clone()))
        .collect();
    occurrence_keys.sort_by(occurrence_cmp);
    occurrence_keys.dedup();
    limit(occurrence_keys.len(), 512, "/products/cooking_occurrences")?;
    for occurrence in &occurrence_keys {
        let affected: Vec<_> = scoped
            .iter()
            .filter(|(u, _)| use_occurrence(&u.use_key) == occurrence)
            .map(|(u, _)| u.use_key.clone())
            .collect();
        let claim = m
            .cooking_claims
            .iter()
            .find(|c| c.occurrence == *occurrence);
        let state = claim
            .map(|c| raw_state(product.item, &c.source_fields, c.state))
            .unwrap_or(ClaimState::Unknown);
        let ids = claim.map(|c| c.evidence_ids.as_slice()).unwrap_or(&[]);
        match state {
            ClaimState::Supported => (),
            ClaimState::Refuted => issues.add(
                MatchReason::ChangedCooking,
                None,
                Some(&affected),
                ids,
                false,
            ),
            ClaimState::Unknown => issues.add(
                MatchReason::UnknownCooking,
                None,
                Some(&affected),
                ids,
                false,
            ),
            ClaimState::Contradictory => issues.add(
                MatchReason::ContradictoryCooking,
                None,
                Some(&affected),
                ids,
                false,
            ),
        }
    }
    if !issues.values.is_empty() {
        return Ok(issues.finish());
    }
    let uses = scoped
        .into_iter()
        .map(|(u, requirements)| MatchUse {
            use_key: u.use_key.clone(),
            at: u.at.clone(),
            requirements,
            cooking: m
                .cooking_claims
                .iter()
                .find(|c| c.occurrence == *use_occurrence(&u.use_key))
                .expect("complete supported cooking closure")
                .clone(),
        })
        .collect();
    let basis = MatchBasis {
        saved_basis_digest: context.saved_digest.clone(),
        derived_digest: context.derived.digest.clone(),
        demand_digest: context.derived.demand.digest.clone(),
        context: context.current.stamp.clone(),
        policy_digest: context.policy.digest.clone(),
        line_id: line_id.clone(),
        line: line.clone(),
        product: product.item.product.clone(),
        observation_digest: product.source.accepted.digest.clone(),
        meaning_digest: a.meaning_digest.clone(),
        review: a.review.clone(),
        evidence: a.evidence.clone(),
        freshness_rule: a.freshness_rule.clone(),
        item_observed_at: product.item.observed_at.clone(),
        read_observed_at: product.source.observation.observed_at.clone(),
        read_admitted_at: product.source.accepted.admitted_at.clone(),
        meaning_admitted_at: a.meaning_admitted_at.clone(),
        valid_until: a.valid_until.clone(),
        evaluated_at: context.evaluated_at.clone(),
        uses,
        supply_key: m.key_claim.supply_key.clone(),
        origins,
    };
    let digest = match_basis_digest(&basis)?;
    Ok(MatchAssessment::Suitable(MatchedCandidate {
        basis,
        observation: product.item.clone(),
        digest,
    }))
}
fn use_occurrence(k: &UseKey) -> &OccurrenceKey {
    match k {
        UseKey::Preparation { phase } => &phase.occurrence,
        UseKey::Consumption { occurrence, .. } => occurrence,
    }
}

fn check_pack_quantities(item: &ProductObservation) -> Result<(), DomainError> {
    for fact in [&item.pack.net, &item.pack.drained] {
        match fact {
            Fact::Known { value } => validation::quantity(value)?,
            Fact::Conflicting { values } => {
                for value in &values.0 {
                    validation::quantity(value)?;
                }
            }
            Fact::Unknown { .. } => (),
        }
    }
    Ok(())
}
type ScopedRequirements<'a> = Vec<(&'a DemandUse, Vec<HardRequirement>)>;
impl MatchContext<'_> {
    /// Read-only inputs for one actual derived line. The caller supplies only
    /// its identity; transported quantities, requirements or origins cannot
    /// replace this prepared context. These clones confer no product authority.
    pub fn line_context(
        &self,
        line_id: &DemandLineId,
    ) -> Result<(DemandLine, Vec<(DemandUse, Vec<HardRequirement>)>, Vec<SupplyOrigin>), DomainError> {
        shape(line_id)?;
        let line = self.derived.demand.lines.iter().find(|line| &line.line_id == line_id)
            .ok_or_else(|| err(ErrorCode::MissingReference, "/products/line"))?;
        let uses = self.line_uses(line)?.into_iter()
            .map(|(demand_use, requirements)| (demand_use.clone(), requirements)).collect();
        Ok((line.clone(), uses, self.line_origins(line)))
    }

    fn line_origins(&self, line: &DemandLine) -> Vec<SupplyOrigin> {
        self.origins
            .iter()
            .filter(|o| {
                o.ingredient_id == line.ingredient_id
                    && o.original_quantity.unit == line.quantity.unit
                    && o.original_quantity.form == line.quantity.form
            })
            .cloned()
            .collect()
    }
    fn line_uses(&self, line: &DemandLine) -> Result<ScopedRequirements<'_>, DomainError> {
        let mut out = vec![];
        let mut all = vec![];
        for u in self.graph.uses.iter().filter(|u| u.line_id == line.line_id) {
            let mut req = vec![];
            if let UseKey::Consumption {
                occurrence, person, ..
            } = &u.use_key
            {
                let meal = self
                    .current
                    .requirements
                    .iter()
                    .find(|m| m.slot == occurrence.slot)
                    .ok_or_else(|| err(ErrorCode::MissingReference, "/products/meal"))?;
                let attendee = meal
                    .attendees
                    .iter()
                    .find(|a| a.person == *person)
                    .ok_or_else(|| err(ErrorCode::MissingReference, "/products/person"))?;
                req.extend(attendee.hard_requirements.iter().cloned());
                if let Some(extra_id) = &occurrence.extra_id {
                    let selection = self
                        .saved
                        .plan
                        .selections
                        .iter()
                        .find(|s| s.slot == occurrence.slot)
                        .ok_or_else(|| {
                            err(ErrorCode::MissingReference, "/products/extra/selection")
                        })?;
                    let base = reviewed(self.current, &self.saved.evidence, &selection.recipe)?;
                    let spec = base
                        .recipe()
                        .extras
                        .iter()
                        .find(|e| e.extra_id == *extra_id && e.recipe == occurrence.recipe)
                        .ok_or_else(|| err(ErrorCode::MissingReference, "/products/extra/spec"))?;
                    req.extend(spec.eligible_person_requirements.iter().cloned());
                }
                req.sort_by(validation::requirement_cmp);
                req.dedup();
                validation::requirements(&req)?;
                for r in &req {
                    if !all.contains(r) {
                        all.push(r.clone());
                        limit(all.len(), 64, "/products/line/requirements")?;
                    }
                }
            }
            out.push((u, req));
        }
        out.sort_by(|a, b| use_cmp(&a.0.use_key, &b.0.use_key));
        Ok(out)
    }
}

fn raw_state(item: &ProductObservation, fields: &[SourceField], claim: ClaimState) -> ClaimState {
    let mut unknown = false;
    for f in fields {
        let fact = match f {
            SourceField::Ingredients => &item.ingredients,
            SourceField::Allergens => &item.allergens,
            SourceField::Storage => &item.storage,
        };
        match fact {
            Fact::Conflicting { .. } => return ClaimState::Contradictory,
            Fact::Unknown { .. } => unknown = true,
            Fact::Known { .. } => (),
        }
    }
    if unknown { ClaimState::Unknown } else { claim }
}
struct Issues {
    line_id: DemandLineId,
    product: ProductRef,
    all_uses: Vec<UseKey>,
    values: Vec<MatchIssue>,
    refuted: bool,
}
impl Issues {
    fn add(
        &mut self,
        reason: MatchReason,
        requirement: Option<&HardRequirement>,
        uses: Option<&[UseKey]>,
        evidence_ids: &[EvidenceId],
        refuted: bool,
    ) {
        self.refuted |= refuted;
        self.values.push(MatchIssue {
            line_id: self.line_id.clone(),
            product: self.product.clone(),
            requirement: requirement.cloned(),
            uses: uses.unwrap_or(&self.all_uses).to_vec(),
            reason,
            evidence_ids: evidence_ids.to_vec(),
        });
    }
    fn finish(self) -> MatchAssessment {
        if self.refuted {
            MatchAssessment::Unsuitable(self.values)
        } else {
            MatchAssessment::Unresolved(self.values)
        }
    }
}
fn check_supply(
    context: &MatchContext<'_>,
    supply: &DemandKey,
    origins: &mut [SupplyOrigin],
    issues: &mut Issues,
) -> Result<(), DomainError> {
    for o in origins {
        if supply.unit == o.original_quantity.unit && supply.form == o.original_quantity.form {
            continue;
        }
        let rr = reviewed(
            context.current,
            &context.saved.evidence,
            &o.occurrence.recipe,
        )?;
        let ingredient = rr
            .recipe()
            .ingredients
            .iter()
            .find(|i| i.ingredient_id == o.ingredient_id && i.component_id == o.component_id)
            .ok_or_else(|| err(ErrorCode::MissingReference, "/products/origin/ingredient"))?;
        let admitted = context
            .saved
            .evidence
            .recipes
            .iter()
            .find(|r| r.recipe == o.occurrence.recipe)
            .ok_or_else(|| err(ErrorCode::MissingReference, "/products/origin/recipe"))?;
        let mut evidence = vec![];
        for e in ingredient.conversion_evidence.iter().filter(|e| {
            e.ingredient_id == o.ingredient_id
                && e.from_unit == supply.unit
                && e.from_form == supply.form
                && e.to_unit == o.original_quantity.unit
                && e.to_form == o.original_quantity.form
        }) {
            shape(e)?;
            PositiveRational::new(e.factor.0.numerator.0, e.factor.0.denominator.0)?;
            validate_conversion(
                e,
                &o.ingredient_id,
                supply.unit,
                supply.form,
                o.original_quantity.unit,
                o.original_quantity.form,
            )?;
            let pin = admitted
                .admitted
                .iter()
                .find(|p| p.evidence_id == e.evidence_id)
                .ok_or_else(|| {
                    err(
                        ErrorCode::MissingReference,
                        "/products/origin/conversion_pin",
                    )
                })?;
            evidence.push((e.clone(), pin.clone()));
        }
        evidence.sort_by(|a, b| a.0.evidence_id.cmp(&b.0.evidence_id));
        let uses: Vec<_> = issues
            .all_uses
            .iter()
            .filter(|u| use_occurrence(u) == &o.occurrence)
            .cloned()
            .collect();
        if evidence.is_empty() {
            issues.add(
                MatchReason::UnsupportedSupplyRelation,
                None,
                Some(&uses),
                &[],
                false,
            );
            continue;
        }
        if evidence
            .iter()
            .any(|(e, _)| e.factor != evidence[0].0.factor)
        {
            issues.add(
                MatchReason::ConflictingSupplyRelation,
                None,
                Some(&uses),
                &evidence
                    .iter()
                    .map(|(e, _)| e.evidence_id.clone())
                    .collect::<Vec<_>>(),
                false,
            );
            continue;
        }
        o.relation = SupplyRelation::ForwardConversion { evidence };
    }
    Ok(())
}
