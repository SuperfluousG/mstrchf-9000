//! Pure native recipe admission. Evidence authority is supplied by the caller.
use mstrchf_contracts::*;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeValidation {
    pub capabilities: RecipeCapabilities,
    pub issues: Vec<ValidationIssue>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidencePin {
    pub evidence_id: EvidenceId,
    pub revision: Revision,
    pub digest: Digest,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EvidenceBasis {
    pub recipe: RecipeRef,
    pub admitted: Vec<EvidencePin>,
    pub verified_asset_digests: Vec<AssetDigest>,
}
impl EvidenceBasis {
    /// The caller attests semantic provenance and currentness for these exact
    /// recipe bytes and evidence revisions/digests. This checks shape, not truth.
    pub fn new(
        recipe: RecipeRef,
        admitted: Vec<EvidencePin>,
        verified_asset_digests: Vec<AssetDigest>,
    ) -> Result<Self, DomainError> {
        let value = Self {
            recipe,
            admitted,
            verified_asset_digests,
        };
        value.check()?;
        Ok(value)
    }
    fn check(&self) -> Result<(), DomainError> {
        limit(self.admitted.len(), 256, "/basis/admitted")?;
        limit(
            self.verified_asset_digests.len(),
            512,
            "/basis/verified_asset_digests",
        )?;
        shape(&self.recipe, "/basis/recipe")?;
        for (i, pin) in self.admitted.iter().enumerate() {
            shape(
                &pin.evidence_id,
                &format!("/basis/admitted/{i}/evidence_id"),
            )?;
            shape(&pin.revision, &format!("/basis/admitted/{i}/revision"))?;
            shape(&pin.digest, &format!("/basis/admitted/{i}/digest"))?;
        }
        ordered(
            self.admitted.iter().map(|p| &p.evidence_id),
            "/basis/admitted",
        )?;
        for (i, digest) in self.verified_asset_digests.iter().enumerate() {
            shape(digest, &format!("/basis/verified_asset_digests/{i}"))?;
        }
        ordered(
            self.verified_asset_digests.iter(),
            "/basis/verified_asset_digests",
        )
    }
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ComponentClass {
    General,
    Rice,
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentClassEvidence {
    pub recipe: RecipeRef,
    pub component_id: ComponentId,
    pub class: ComponentClass,
    pub evidence_ids: Vec<EvidenceId>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ComponentClassView {
    pub recipe: RecipeRef,
    pub components: Vec<ComponentClassEvidence>,
}
impl ComponentClassView {
    pub fn new(
        recipe: RecipeRef,
        components: Vec<ComponentClassEvidence>,
    ) -> Result<Self, DomainError> {
        let value = Self { recipe, components };
        value.check()?;
        Ok(value)
    }
    fn check(&self) -> Result<(), DomainError> {
        limit(self.components.len(), 128, "/classes/components")?;
        shape(&self.recipe, "/classes/recipe")?;
        for (i, fact) in self.components.iter().enumerate() {
            let path = format!("/classes/components/{i}");
            limit(
                fact.evidence_ids.len(),
                256,
                &format!("{path}/evidence_ids"),
            )?;
            shape(&fact.recipe, &format!("{path}/recipe"))?;
            shape(&fact.component_id, &format!("{path}/component_id"))?;
            same_ref(&fact.recipe, &self.recipe, &format!("{path}/recipe"))?;
            if fact.class != ComponentClass::Unknown && fact.evidence_ids.is_empty() {
                return Err(DomainError::new(
                    ErrorCode::MissingReference,
                    &format!("{path}/evidence_ids"),
                ));
            }
            for (j, id) in fact.evidence_ids.iter().enumerate() {
                shape(id, &format!("{path}/evidence_ids/{j}"))?;
            }
            ordered(fact.evidence_ids.iter(), &format!("{path}/evidence_ids"))?;
        }
        ordered(
            self.components.iter().map(|c| &c.component_id),
            "/classes/components",
        )
    }
}
#[derive(Debug)]
pub struct ReviewedRecipe<'a> {
    recipe: &'a RecipeRevision,
    basis: &'a EvidenceBasis,
    classes: &'a ComponentClassView,
    assessment: RecipeValidation,
}
impl ReviewedRecipe<'_> {
    pub fn recipe(&self) -> &RecipeRevision {
        self.recipe
    }
    pub fn assessment(&self) -> &RecipeValidation {
        &self.assessment
    }
    pub fn component_classes(&self) -> &ComponentClassView {
        self.classes
    }
}
pub fn validate_recipe(candidate: &RecipeCandidate) -> Result<RecipeValidation, DomainError> {
    bounded(&candidate.ingredients, "/ingredients")?;
    bounded(&candidate.steps, "/steps")?;
    bounded(&candidate.unresolved, "/unresolved")?;
    shape(candidate, "")?;
    let mut missing = candidate.unresolved.0.clone();
    let mut add = |code, path: &str| {
        if !missing.iter().any(|i| i.code == code && i.path.0 == path) {
            missing.push(CandidateIssue {
                code,
                path: Text(path.into()),
                evidence_ids: BoundedVec(vec![]),
            });
        }
    };
    for (i, ingredient) in candidate.ingredients.iter().enumerate() {
        if ingredient.quantity.0.is_none() {
            add(
                CandidateIssueCode::MissingQuantity,
                &format!("/ingredients/{i}/quantity"),
            );
        }
        if ingredient.ingredient_id.0.is_none() || ingredient.component_id.0.is_none() {
            add(
                CandidateIssueCode::UnresolvedIngredient,
                &format!("/ingredients/{i}"),
            );
        }
    }
    if candidate.ingredients.is_empty() {
        add(CandidateIssueCode::UnresolvedIngredient, "/ingredients");
    }
    if candidate.r#yield.0.is_none() {
        add(CandidateIssueCode::MissingYield, "/yield");
    }
    if candidate.steps.is_empty() {
        add(CandidateIssueCode::InconsistentInstructions, "/steps");
    }
    add(CandidateIssueCode::MissingRights, "/text_rights");
    add(CandidateIssueCode::MissingStorage, "/storage");
    add(
        CandidateIssueCode::MissingPortionEvidence,
        "/portion_presets",
    );
    if candidate.image.0.is_none() {
        add(CandidateIssueCode::MissingImage, "/image");
    }
    // The wire has no truncation marker for capability issues. Never silently
    // discard missing facts to fit its bound.
    assessment(false, false, false, missing)
}
pub fn assess_revision<'a>(
    recipe: &'a RecipeRevision,
    basis: &'a EvidenceBasis,
    classes: &'a ComponentClassView,
) -> Result<ReviewedRecipe<'a>, DomainError> {
    recipe_bounds(recipe)?;
    shape(recipe, "").map_err(|e| {
        if e.issues
            .iter()
            .any(|i| i.code == ErrorCode::DomainConflict && i.path.starts_with("/capabilities/"))
        {
            DomainError::new(ErrorCode::DomainConflict, "/capabilities")
        } else {
            e
        }
    })?;
    basis.check()?;
    classes.check()?;
    let actual = RecipeRef {
        recipe_id: recipe.recipe_id.clone(),
        revision: recipe.revision,
        digest: recipe_digest(recipe)?,
    };
    same_ref(&basis.recipe, &actual, "/basis/recipe")?;
    same_ref(&classes.recipe, &actual, "/classes/recipe")?;
    limit(
        classes.components.len(),
        recipe.components.len(),
        "/classes/components",
    )?;
    for (i, fact) in classes.components.iter().enumerate() {
        if !recipe
            .components
            .iter()
            .any(|c| c.component_id == fact.component_id)
        {
            return Err(DomainError::new(
                ErrorCode::ForeignReference,
                &format!("/classes/components/{i}/component_id"),
            ));
        }
        pins(
            basis,
            &fact.evidence_ids,
            &format!("/classes/components/{i}/evidence_ids"),
        )?;
    }
    let mut reviewed = ReviewedRecipe {
        recipe,
        basis,
        classes,
        assessment: RecipeValidation {
            capabilities: RecipeCapabilities {
                local_display: true,
                shopping: false,
                leftovers: false,
                starter_distribution: false,
                blocking_issues: BoundedVec(vec![]),
            },
            issues: vec![],
        },
    };
    evidence_closure(reviewed.recipe, reviewed.basis)?;
    for (i, component) in recipe.components.iter().enumerate() {
        let applicable: BTreeSet<_> = recipe
            .storage
            .iter()
            .filter(|rule| rule.component_id == component.component_id)
            .map(|rule| &rule.evidence_id)
            .collect();
        let linked: BTreeSet<_> = component.storage_rule_ids.iter().collect();
        // A rule's component binding establishes applicability. The recipe's
        // link list cannot select away any of those admitted restrictions.
        if linked != applicable {
            return Err(DomainError::new(
                ErrorCode::DomainConflict,
                &format!("/components/{i}/storage_rule_ids"),
            ));
        }
    }
    let mut missing = Vec::new();
    let quantity_review = recipe
        .reviews
        .iter()
        .any(|r| r.kind == ReviewKind::QuantityReview);
    if !quantity_review {
        missing.push(missing_fact(
            CandidateIssueCode::MissingQuantity,
            "/reviews",
        ));
    }
    let mut amounts = true;
    for (i, ingredient) in recipe.ingredients.iter().enumerate() {
        if ingredient.quantity.0.is_none() {
            missing.push(missing_fact(
                CandidateIssueCode::MissingQuantity,
                &format!("/ingredients/{i}/quantity"),
            ));
            amounts = false;
        }
    }
    let shopping = quantity_review && amounts;
    let mut reuse = true;
    for (i, component) in recipe.components.iter().enumerate() {
        let ceiling = classes
            .components
            .iter()
            .find(|c| c.component_id == component.component_id)
            .and_then(|c| match c.class {
                ComponentClass::General => Some(172800),
                ComponentClass::Rice => Some(86400),
                ComponentClass::Unknown => None,
            });
        let rules: Vec<_> = recipe
            .storage
            .iter()
            .filter(|s| s.component_id == component.component_id)
            .collect();
        // Every applicable admitted rule may shorten the ceiling. A's boolean
        // means supported reuse exists; C must check the actual elapsed use.
        let effective = ceiling.filter(|_| !rules.is_empty()).map(|c| {
            rules
                .iter()
                .fold(c, |limit, rule| limit.min(rule.max_chilled_seconds))
        });
        let permits_required_reheat = !recipe.portability.reheating_required
            || rules.iter().all(|rule| rule.max_reheats >= 1);
        if effective.is_none_or(|seconds| seconds == 0) || !permits_required_reheat {
            reuse = false;
            missing.push(missing_fact(
                CandidateIssueCode::MissingStorage,
                &format!("/components/{i}/storage_rule_ids"),
            ));
        }
    }
    let source_review = recipe
        .reviews
        .iter()
        .any(|r| r.kind == ReviewKind::SourceReview);
    let text_rights = recipe.text_rights.distribution == RightsDistribution::Permitted
        && !recipe.text_rights.evidence_ids.is_empty();
    let image_rights = recipe.image.0.as_ref().is_some_and(|i| {
        i.rights.distribution == RightsDistribution::Permitted
            && !i.rights.evidence_ids.is_empty()
            && basis.verified_asset_digests.contains(&i.digest)
    });
    if !source_review || !text_rights {
        missing.push(missing_fact(
            CandidateIssueCode::MissingRights,
            "/text_rights",
        ));
    }
    if !image_rights {
        missing.push(missing_fact(CandidateIssueCode::MissingImage, "/image"));
    }
    let computed = assessment(
        shopping,
        shopping && reuse,
        source_review && text_rights && image_rights,
        missing,
    )?;
    // Imported diagnostics are structurally valid claims, never assessment
    // inputs. Root's diagnostics ruling compares only the four capability bits.
    let actual = &computed.capabilities;
    let claimed = &recipe.capabilities;
    if actual.local_display != claimed.local_display
        || actual.shopping != claimed.shopping
        || actual.leftovers != claimed.leftovers
        || actual.starter_distribution != claimed.starter_distribution
    {
        return Err(DomainError::new(ErrorCode::DomainConflict, "/capabilities"));
    }
    reviewed.assessment = computed;
    Ok(reviewed)
}

fn unsupported(path: &str) -> ValidationIssue {
    ValidationIssue {
        path: path.into(),
        code: ErrorCode::UnsupportedCapability,
        entity_id: Nullable(None),
    }
}
fn missing_fact(code: CandidateIssueCode, path: &str) -> CandidateIssue {
    CandidateIssue {
        code,
        path: Text(path.into()),
        evidence_ids: BoundedVec(vec![]),
    }
}
fn assessment(
    shopping: bool,
    leftovers: bool,
    starter_distribution: bool,
    mut missing: Vec<CandidateIssue>,
) -> Result<RecipeValidation, DomainError> {
    missing.sort_by(|a, b| (&a.path, a.code).cmp(&(&b.path, b.code)));
    missing.dedup_by(|a, b| a.path == b.path && a.code == b.code);
    limit(missing.len(), 256, "/capabilities/blocking_issues")?;
    let mut issues: Vec<_> = missing.iter().map(|i| unsupported(&i.path.0)).collect();
    issues.dedup_by(|a, b| a.path == b.path);
    limit(issues.len(), 100, "/issues")?;
    Ok(RecipeValidation {
        capabilities: RecipeCapabilities {
            local_display: true,
            shopping,
            leftovers,
            starter_distribution,
            blocking_issues: BoundedVec(missing),
        },
        issues,
    })
}
fn limit(len: usize, max: usize, path: &str) -> Result<(), DomainError> {
    if len > max {
        Err(DomainError::new(ErrorCode::LimitExceeded, path))
    } else {
        Ok(())
    }
}
fn bounded<T, const A: usize, const B: usize>(
    v: &BoundedVec<T, A, B>,
    path: &str,
) -> Result<(), DomainError> {
    limit(v.len(), B, path)?;
    if v.len() < A {
        Err(DomainError::new(ErrorCode::InvalidValue, path))
    } else {
        Ok(())
    }
}
fn ordered<'a, T: Ord + 'a>(
    items: impl Iterator<Item = &'a T>,
    path: &str,
) -> Result<(), DomainError> {
    let items: Vec<_> = items.collect();
    let set: BTreeSet<_> = items.iter().collect();
    if set.len() != items.len() {
        return Err(DomainError::new(ErrorCode::InvalidValue, path));
    }
    if items.windows(2).any(|p| p[0] > p[1]) {
        return Err(DomainError::new(ErrorCode::NoncanonicalValue, path));
    }
    Ok(())
}
fn shape<T: ValidateShape>(value: &T, prefix: &str) -> Result<(), DomainError> {
    value.validate_shape().map_err(|issues| DomainError {
        code: issues.first().map_or(ErrorCode::InvalidValue, |i| i.code),
        truncated: issues.len() > 100,
        issues: BoundedVec(
            issues
                .into_iter()
                .take(100)
                .map(|mut i| {
                    i.path = format!("{prefix}{}", i.path);
                    i
                })
                .collect(),
        ),
        retry: Retry::Never,
    })
}
fn same_ref(actual: &RecipeRef, expected: &RecipeRef, path: &str) -> Result<(), DomainError> {
    if actual.recipe_id != expected.recipe_id || actual.revision != expected.revision {
        return Err(DomainError::new(ErrorCode::IdentityMismatch, path));
    }
    if actual.digest != expected.digest {
        return Err(DomainError::new(
            ErrorCode::DigestMismatch,
            &format!("{path}/digest"),
        ));
    }
    Ok(())
}
fn pin(basis: &EvidenceBasis, id: &EvidenceId, path: &str) -> Result<(), DomainError> {
    if basis
        .admitted
        .binary_search_by(|p| p.evidence_id.cmp(id))
        .is_err()
    {
        Err(DomainError::new(ErrorCode::MissingReference, path))
    } else {
        Ok(())
    }
}
fn pins(basis: &EvidenceBasis, ids: &[EvidenceId], path: &str) -> Result<(), DomainError> {
    for (i, id) in ids.iter().enumerate() {
        pin(basis, id, &format!("{path}/{i}"))?;
    }
    Ok(())
}
fn evidence_closure(r: &RecipeRevision, b: &EvidenceBasis) -> Result<(), DomainError> {
    for (i, v) in r.reviews.iter().enumerate() {
        pin(b, &v.evidence_id, &format!("/reviews/{i}/evidence_id"))?;
    }
    pins(b, &r.text_rights.evidence_ids, "/text_rights/evidence_ids")?;
    if let Some(image) = &r.image.0 {
        pins(b, &image.rights.evidence_ids, "/image/rights/evidence_ids")?;
    }
    pins(b, &r.portability.evidence_ids, "/portability/evidence_ids")?;
    for (i, v) in r.portion_presets.iter().enumerate() {
        pins(
            b,
            &v.evidence_ids,
            &format!("/portion_presets/{i}/evidence_ids"),
        )?;
    }
    for (i, v) in r.storage.iter().enumerate() {
        pin(b, &v.evidence_id, &format!("/storage/{i}/evidence_id"))?;
    }
    for (i, v) in r.dietary_evidence.iter().enumerate() {
        pin(
            b,
            &v.evidence_id,
            &format!("/dietary_evidence/{i}/evidence_id"),
        )?;
        pin(b, &v.review_id, &format!("/dietary_evidence/{i}/review_id"))?;
        if !r.reviews.iter().any(|review| {
            review.evidence_id == v.review_id && review.kind == ReviewKind::DietaryReview
        }) {
            return Err(DomainError::new(
                ErrorCode::ForeignReference,
                &format!("/dietary_evidence/{i}/review_id"),
            ));
        }
    }
    for (i, v) in r.ingredients.iter().enumerate() {
        for (j, c) in v.conversion_evidence.iter().enumerate() {
            pin(
                b,
                &c.evidence_id,
                &format!("/ingredients/{i}/conversion_evidence/{j}/evidence_id"),
            )?;
        }
    }
    Ok(())
}
fn recipe_bounds(r: &RecipeRevision) -> Result<(), DomainError> {
    bounded(&r.ingredients, "/ingredients")?;
    bounded(&r.components, "/components")?;
    bounded(&r.steps, "/steps")?;
    bounded(&r.reviews, "/reviews")?;
    bounded(&r.storage, "/storage")?;
    bounded(&r.portion_presets, "/portion_presets")?;
    bounded(&r.dietary_evidence, "/dietary_evidence")?;
    bounded(&r.prep, "/prep")?;
    bounded(&r.extras, "/extras")?;
    bounded(&r.equipment, "/equipment")?;
    bounded(&r.meal_types, "/meal_types")?;
    bounded(&r.r#yield.components, "/yield/components")?;
    bounded(
        &r.capabilities.blocking_issues,
        "/capabilities/blocking_issues",
    )?;
    for (i, v) in r.ingredients.iter().enumerate() {
        bounded(
            &v.conversion_evidence,
            &format!("/ingredients/{i}/conversion_evidence"),
        )?;
        bounded(&v.step_ids, &format!("/ingredients/{i}/step_ids"))?;
    }
    for (i, v) in r.components.iter().enumerate() {
        bounded(
            &v.ingredient_ids,
            &format!("/components/{i}/ingredient_ids"),
        )?;
        bounded(
            &v.storage_rule_ids,
            &format!("/components/{i}/storage_rule_ids"),
        )?;
    }
    for (i, v) in r.steps.iter().enumerate() {
        bounded(&v.ingredient_ids, &format!("/steps/{i}/ingredient_ids"))?;
        bounded(&v.component_ids, &format!("/steps/{i}/component_ids"))?;
        bounded(&v.depends_on, &format!("/steps/{i}/depends_on"))?;
    }
    Ok(())
}
