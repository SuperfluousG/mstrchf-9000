use super::*;
use std::cmp::Ordering;

pub(super) fn pin(p: &EvidencePin) -> Result<(), DomainError> {
    shape(&p.evidence_id)?;
    shape(&p.revision)?;
    shape(&p.digest)
}
pub(super) fn pins(p: &[EvidencePin]) -> Result<(), DomainError> {
    limit(p.len(), 4096, "/products/evidence")?;
    for p in p {
        pin(p)?;
    }
    ordered(
        p,
        |a, b| a.evidence_id.cmp(&b.evidence_id),
        "/products/evidence",
    )
}
pub(super) fn fields(f: &[SourceField]) -> Result<(), DomainError> {
    limit(f.len(), 3, "/products/source_fields")?;
    require(
        !f.is_empty(),
        ErrorCode::MissingReference,
        "/products/source_fields",
    )?;
    ordered(f, Ord::cmp, "/products/source_fields")
}
pub(super) fn claim(ids: &[EvidenceId], f: &[SourceField]) -> Result<(), DomainError> {
    limit(ids.len(), 256, "/products/claim/evidence_ids")?;
    require(
        !ids.is_empty(),
        ErrorCode::MissingReference,
        "/products/claim/evidence_ids",
    )?;
    for id in ids {
        shape(id)?;
    }
    ordered(ids, Ord::cmp, "/products/claim/evidence_ids")?;
    fields(f)
}
pub(super) fn key(k: &DemandKey) -> Result<(), DomainError> {
    shape(&k.ingredient_id)?;
    shape(&k.unit)?;
    shape(&k.form)
}
pub(super) fn occurrence(k: &OccurrenceKey) -> Result<(), DomainError> {
    shape(&k.slot)?;
    shape(&k.recipe)?;
    if let Some(id) = &k.extra_id {
        shape(id)?;
    }
    Ok(())
}
pub(super) fn requirements(v: &[HardRequirement]) -> Result<(), DomainError> {
    limit(v.len(), 64, "/products/requirements")?;
    let mut keys = Vec::with_capacity(v.len());
    for r in v {
        shape(r)?;
        keys.push(canonical_bytes(r)?);
    }
    ordered(&keys, Ord::cmp, "/products/requirements")
}
pub(super) fn requirement_cmp(a: &HardRequirement, b: &HardRequirement) -> Ordering {
    // Both are shape-checked before this comparator is used.
    canonical_bytes(a)
        .expect("checked requirement")
        .cmp(&canonical_bytes(b).expect("checked requirement"))
}
pub(super) fn cooking(c: &CookingClaim) -> Result<(), DomainError> {
    shape(&c.recipe)?;
    occurrence(&c.occurrence)?;
    require(
        c.recipe == c.occurrence.recipe,
        ErrorCode::IdentityMismatch,
        "/products/cooking/recipe",
    )?;
    claim(&c.evidence_ids, &c.source_fields)
}
pub(super) fn policy(p: &MatchPolicy) -> Result<(), DomainError> {
    limit(p.rules.len(), 256, "/products/policy/rules")?;
    shape(&p.version)?;
    shape(&p.digest)?;
    for r in &p.rules {
        shape(&r.requirement)?;
        fields(&r.required_fields)?;
        pin(&r.review)?;
    }
    ordered(
        &p.rules,
        |a, b| requirement_cmp(&a.requirement, &b.requirement),
        "/products/policy/rules",
    )
}
pub(super) fn meaning(m: &ProductMeaning) -> Result<(), DomainError> {
    limit(
        m.requirement_claims.len(),
        64,
        "/products/meaning/requirements",
    )?;
    limit(m.cooking_claims.len(), 512, "/products/meaning/cooking")?;
    shape(&m.product)?;
    shape(&m.observation_digest)?;
    key(&m.key_claim.supply_key)?;
    claim(&m.key_claim.evidence_ids, &m.key_claim.source_fields)?;
    for c in &m.requirement_claims {
        shape(&c.requirement)?;
        claim(&c.evidence_ids, &c.source_fields)?;
    }
    for c in &m.cooking_claims {
        cooking(c)?;
    }
    ordered(
        &m.requirement_claims,
        |a, b| requirement_cmp(&a.requirement, &b.requirement),
        "/products/meaning/requirements",
    )?;
    ordered(
        &m.cooking_claims,
        |a, b| occurrence_cmp(&a.occurrence, &b.occurrence),
        "/products/meaning/cooking",
    )
}
pub(super) fn admission(a: &ProductMeaningAdmission) -> Result<(), DomainError> {
    pins(&a.evidence)?;
    pin(&a.review)?;
    pin(&a.freshness_rule)?;
    shape(&a.meaning_digest)?;
    shape(&a.policy_digest)?;
    shape(&a.meaning_admitted_at)?;
    shape(&a.valid_until)?;
    require(
        a.evidence.contains(&a.review) && a.evidence.contains(&a.freshness_rule),
        ErrorCode::MissingReference,
        "/products/admission/roles",
    )?;
    require(
        a.meaning_admitted_at < a.valid_until,
        ErrorCode::InvalidValue,
        "/products/admission/expiry",
    )
}
pub(super) fn quantity(q: &Quantity) -> Result<(), DomainError> {
    PositiveRational::new(q.amount.0.numerator.0, q.amount.0.denominator.0)?;
    shape(q)
}
pub(super) fn origin_cmp(a: &SupplyOrigin, b: &SupplyOrigin) -> Ordering {
    occurrence_cmp(&a.occurrence, &b.occurrence)
        .then(a.batch_id.cmp(&b.batch_id))
        .then(a.component_id.cmp(&b.component_id))
        .then(a.ingredient_id.cmp(&b.ingredient_id))
}
pub(super) fn origin(o: &SupplyOrigin, k: &DemandKey) -> Result<(), DomainError> {
    occurrence(&o.occurrence)?;
    shape(&o.batch_id)?;
    shape(&o.component_id)?;
    shape(&o.ingredient_id)?;
    quantity(&o.original_quantity)?;
    require(
        o.ingredient_id == k.ingredient_id,
        ErrorCode::IdentityMismatch,
        "/products/origin/ingredient",
    )?;
    match &o.relation {
        SupplyRelation::Exact => require(
            o.original_quantity.unit == k.unit && o.original_quantity.form == k.form,
            ErrorCode::IdentityMismatch,
            "/products/origin/exact",
        ),
        SupplyRelation::ForwardConversion { evidence } => {
            limit(evidence.len(), 64, "/products/origin/conversions")?;
            require(
                !evidence.is_empty(),
                ErrorCode::MissingReference,
                "/products/origin/conversions",
            )?;
            for (e, p) in evidence {
                shape(e)?;
                PositiveRational::new(e.factor.0.numerator.0, e.factor.0.denominator.0)?;
                pin(p)?;
                require(
                    e.evidence_id == p.evidence_id,
                    ErrorCode::IdentityMismatch,
                    "/products/origin/pin",
                )?;
                validate_conversion(
                    e,
                    &o.ingredient_id,
                    k.unit,
                    k.form,
                    o.original_quantity.unit,
                    o.original_quantity.form,
                )?;
                require(
                    e.factor == evidence[0].0.factor,
                    ErrorCode::DomainConflict,
                    "/products/origin/factor",
                )?;
            }
            ordered(
                evidence,
                |a, b| a.0.evidence_id.cmp(&b.0.evidence_id),
                "/products/origin/conversions",
            )
        }
    }
}
pub(super) fn basis(b: &MatchBasis) -> Result<(), DomainError> {
    limit(b.uses.len(), 16384, "/products/uses")?;
    limit(b.origins.len(), 16384, "/products/origins")?;
    pins(&b.evidence)?;
    pin(&b.review)?;
    pin(&b.freshness_rule)?;
    shape(&b.saved_basis_digest)?;
    shape(&b.derived_digest)?;
    shape(&b.demand_digest)?;
    shape(&b.context)?;
    shape(&b.policy_digest)?;
    shape(&b.line_id)?;
    shape(&b.line)?;
    quantity(&b.line.quantity)?;
    shape(&b.product)?;
    shape(&b.observation_digest)?;
    shape(&b.meaning_digest)?;
    key(&b.supply_key)?;
    require(
        b.line_id == b.line.line_id && b.supply_key.ingredient_id == b.line.ingredient_id,
        ErrorCode::IdentityMismatch,
        "/products/basis/line",
    )?;
    require(
        b.evidence.contains(&b.review) && b.evidence.contains(&b.freshness_rule),
        ErrorCode::MissingReference,
        "/products/basis/roles",
    )?;
    for t in [
        &b.item_observed_at,
        &b.read_observed_at,
        &b.read_admitted_at,
        &b.meaning_admitted_at,
        &b.evaluated_at,
        &b.valid_until,
    ] {
        shape(t)?;
    }
    require(
        b.item_observed_at <= b.read_observed_at
            && b.read_observed_at <= b.read_admitted_at
            && b.read_admitted_at <= b.meaning_admitted_at
            && b.meaning_admitted_at <= b.evaluated_at
            && b.evaluated_at < b.valid_until,
        ErrorCode::InvalidValue,
        "/products/basis/chronology",
    )?;
    require(
        !b.origins.is_empty()
            && b.uses
                .iter()
                .any(|u| matches!(u.use_key, UseKey::Consumption { .. })),
        ErrorCode::MissingReference,
        "/products/basis/closure",
    )?;
    let mut all_requirements = vec![];
    let mut all_occurrences = vec![];
    for u in &b.uses {
        occurrence(use_occurrence(&u.use_key))?;
        shape(&u.at)?;
        requirements(&u.requirements)?;
        for r in &u.requirements {
            if !all_requirements.contains(r) {
                all_requirements.push(r.clone());
                limit(all_requirements.len(), 64, "/products/line/requirements")?;
            }
        }
        if !all_occurrences.contains(&u.cooking.occurrence) {
            all_occurrences.push(u.cooking.occurrence.clone());
            limit(all_occurrences.len(), 512, "/products/cooking_occurrences")?;
        }
        cooking(&u.cooking)?;
        require(
            u.cooking.occurrence == *use_occurrence(&u.use_key)
                && u.cooking.state == ClaimState::Supported,
            ErrorCode::IdentityMismatch,
            "/products/basis/cooking",
        )?;
        match &u.use_key {
            UseKey::Consumption {
                component_id,
                person,
                ..
            } => {
                shape(component_id)?;
                shape(person)?;
            }
            UseKey::Preparation { phase } => {
                require(
                    phase.prep_id > 0,
                    ErrorCode::InvalidValue,
                    "/products/basis/prep_id",
                )?;
            }
        }
        for id in &u.cooking.evidence_ids {
            require(
                b.evidence.iter().any(|p| p.evidence_id == *id),
                ErrorCode::MissingReference,
                "/products/basis/cooking/evidence",
            )?;
        }
    }
    ordered(
        &b.uses,
        |a, b| use_cmp(&a.use_key, &b.use_key),
        "/products/uses",
    )?;
    ordered(&b.origins, origin_cmp, "/products/origins")?;
    for o in &b.origins {
        origin(o, &b.supply_key)?;
        if !all_occurrences.contains(&o.occurrence) {
            all_occurrences.push(o.occurrence.clone());
            limit(all_occurrences.len(), 512, "/products/cooking_occurrences")?;
        }
    }
    conserve(&b.line, &b.origins)
}

// No temporal or semantic validators here: native public wrappers may contain
// malformed text. Bound lists first, then shape-check, then use time parsers.
pub(super) fn read_bounds(s: &ProductReadBasis<'_>) -> Result<(), DomainError> {
    let ReadBody::Products { items, .. } = &s.observation.body else {
        return Err(err(ErrorCode::ScopeViolation, "/products/read/body"));
    };
    limit(items.len(), 256, "/products/read/items")?;
    fn fact<T>(f: &Fact<T>) -> Result<(), DomainError> {
        if let Fact::Conflicting { values } = f {
            limit(values.len(), 8, "/products/read/fact")?;
        }
        Ok(())
    }
    for i in &items.0 {
        limit(i.offers.len(), 64, "/products/read/offers")?;
        fact(&i.ingredients)?;
        fact(&i.allergens)?;
        fact(&i.storage)?;
        fact(&i.pack.net)?;
        fact(&i.pack.drained)?;
        fact(&i.pack.count)?;
    }
    fn scope(s: &ReadScope) -> Result<(), DomainError> {
        limit(s.products.len(), 256, "/products/read/scope/products")?;
        limit(s.event_ids.len(), 256, "/products/read/scope/events")?;
        limit(s.allowed_kinds.len(), 4, "/products/read/scope/kinds")
    }
    let JobPayload::ProductObservation {
        read_scope,
        products,
        ..
    } = &s.request.payload
    else {
        return Err(err(ErrorCode::ScopeViolation, "/products/read/payload"));
    };
    scope(read_scope)?;
    limit(products.len(), 256, "/products/read/products")?;
    let ActionScope::Shop { read_scope, .. } = &s.request.scope else {
        return Err(err(ErrorCode::ScopeViolation, "/products/read/scope"));
    };
    scope(read_scope)?;
    limit(
        s.request.required_capabilities.len(),
        8,
        "/products/read/capabilities",
    )?;
    limit(
        s.request.allowed_artefacts.len(),
        64,
        "/products/read/artefacts",
    )?;
    limit(
        s.request.context_stamp.recipe_refs.len(),
        512,
        "/products/read/recipes",
    )?;
    limit(
        s.request.context_stamp.read_weeks.len(),
        4,
        "/products/read/weeks",
    )?;
    limit(
        s.request.context_stamp.write_weeks.len(),
        4,
        "/products/read/weeks",
    )
}
pub(super) fn conserve(line: &DemandLine, origins: &[SupplyOrigin]) -> Result<(), DomainError> {
    require(
        !origins.is_empty(),
        ErrorCode::MissingReference,
        "/products/origins",
    )?;
    let mut total = Rational::new(0, 1)?;
    let mut batches = vec![];
    let mut extras = vec![];
    for o in origins {
        require(
            o.ingredient_id == line.ingredient_id
                && o.original_quantity.unit == line.quantity.unit
                && o.original_quantity.form == line.quantity.form,
            ErrorCode::IdentityMismatch,
            "/products/origins/measure",
        )?;
        total = total.checked_add(&o.original_quantity.amount.0)?;
        if let Some(e) = &o.occurrence.extra_id {
            extras.push(e.clone());
        } else {
            batches.push(o.batch_id.clone());
        }
    }
    batches.sort();
    batches.dedup();
    extras.sort();
    extras.dedup();
    require(
        total == line.quantity.amount.0
            && batches == line.origin_batches.0
            && extras == line.origin_extras.0,
        ErrorCode::IdentityMismatch,
        "/products/origins/conservation",
    )
}
