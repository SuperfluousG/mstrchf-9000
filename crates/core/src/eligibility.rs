//! Shared meal suitability predicates; implementation follows the reviewed food brief.
//! Pure hard suitability, shared bases and recipient-scoped extras.
use crate::recipes::ReviewedRecipe;
use mstrchf_contracts::*;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Eligibility {
    Eligible,
    Ineligible(Vec<Conflict>),
    Unresolved(Vec<Conflict>),
}
pub fn eligible(
    recipe: &ReviewedRecipe<'_>,
    meal: &MealRequirements,
) -> Result<Eligibility, DomainError> {
    check_meal(meal)?;
    if !recipe.assessment().capabilities.shopping {
        return Ok(Eligibility::Unresolved(vec![conflict(
            recipe,
            meal,
            ErrorCode::UnsupportedCapability,
            vec![],
        )?]));
    }
    let r = recipe.recipe();
    let mut known = Vec::new();
    let mut missing = Vec::new();
    for requirement in meal.shared_hard_requirements.iter() {
        let facts: Vec<_> = r
            .dietary_evidence
            .iter()
            .filter(|e| &e.requirement == requirement)
            .collect();
        let ids = facts.iter().map(|e| e.evidence_id.clone()).collect();
        let excluded = matches!(requirement,HardRequirement::IngredientExclusion{ingredient_id} if r.ingredients.iter().any(|i|&i.ingredient_id==ingredient_id));
        if excluded
            || facts
                .iter()
                .any(|e| e.verdict == DietaryVerdict::Conflicting)
        {
            known.push(conflict(recipe, meal, ErrorCode::DomainConflict, ids)?);
        } else if facts.is_empty() || facts.iter().any(|e| e.verdict != DietaryVerdict::Supported) {
            missing.push(conflict(
                recipe,
                meal,
                ErrorCode::UnsupportedCapability,
                ids,
            )?);
        }
    }
    if !r.meal_types.contains(&meal.slot.meal_type)
        || r.equipment.iter().any(|e| !meal.equipment.contains(e))
    {
        known.push(conflict(recipe, meal, ErrorCode::DomainConflict, vec![])?);
    }
    // Equipment has no phase/location dimension in this contract. Its presence
    // cannot prove that an earlier preparation appliance is available on the move.
    if meal.location == MealLocation::OnTheMove && !r.equipment.is_empty() {
        missing.push(conflict(
            recipe,
            meal,
            ErrorCode::UnsupportedCapability,
            vec![],
        )?);
    }
    let portability = &r.portability;
    let needs_portability = meal.location == MealLocation::OnTheMove
        || meal.mobility.temperature == MobilityTemperature::Cold
        || portability.reheating_required
        || portability.cold_storage_required;
    if needs_portability {
        if portability.evidence_ids.is_empty() {
            missing.push(conflict(
                recipe,
                meal,
                ErrorCode::UnsupportedCapability,
                vec![],
            )?);
        } else {
            if (portability.reheating_required && !meal.mobility.reheating)
                || (meal.mobility.temperature == MobilityTemperature::Cold
                    && !portability.cold_suitable)
            {
                known.push(conflict(
                    recipe,
                    meal,
                    ErrorCode::DomainConflict,
                    portability.evidence_ids.0.clone(),
                )?);
            }
            // F08: absent required cold storage leaves this candidate unresolved.
            if portability.cold_storage_required && !meal.mobility.cold_storage {
                missing.push(conflict(
                    recipe,
                    meal,
                    ErrorCode::UnsupportedCapability,
                    portability.evidence_ids.0.clone(),
                )?);
            }
        }
    }
    // Coalesce identical scoped diagnostics; bounded inputs yield at most 68.
    let mut all = known.clone();
    all.extend(missing);
    all.sort_by(|a, b| (&a.code, &a.evidence_ids.0).cmp(&(&b.code, &b.evidence_ids.0)));
    let mut distinct = Vec::new();
    for c in all {
        if !distinct.contains(&c) {
            distinct.push(c);
        }
    }
    if !known.is_empty() {
        Ok(Eligibility::Ineligible(distinct))
    } else if !distinct.is_empty() {
        Ok(Eligibility::Unresolved(distinct))
    } else {
        Ok(Eligibility::Eligible)
    }
}

pub fn validate_extra(
    base: &ReviewedRecipe<'_>,
    extra: &ReviewedRecipe<'_>,
    selected: &ExtraSelection,
    meal: &MealRequirements,
) -> Result<Eligibility, DomainError> {
    check_meal(meal)?;
    bound(selected.recipients.len(), 1, 64, "/extra/recipients")?;
    shape(selected, "/extra")?;
    let extra_ref = recipe_ref(extra)?;
    if selected.recipe != extra_ref {
        return Err(DomainError::new(
            ErrorCode::IdentityMismatch,
            "/extra/recipe",
        ));
    }
    let spec = base
        .recipe()
        .extras
        .iter()
        .find(|s| s.extra_id == selected.extra_id)
        .ok_or_else(|| DomainError::new(ErrorCode::MissingReference, "/extra/extra_id"))?;
    if spec.recipe != extra_ref {
        return Err(DomainError::new(
            ErrorCode::IdentityMismatch,
            "/extra/recipe",
        ));
    }
    if spec.supported_with.iter().any(|id| {
        !base
            .recipe()
            .components
            .iter()
            .any(|c| &c.component_id == id)
    }) {
        return Err(DomainError::new(
            ErrorCode::ForeignReference,
            "/extra/supported_with",
        ));
    }
    for (i, recipient) in selected.recipients.iter().enumerate() {
        if selected.recipients[..i]
            .iter()
            .any(|r| r.person == recipient.person)
        {
            return Err(DomainError::new(
                ErrorCode::DuplicateKey,
                "/extra/recipients",
            ));
        }
        if !meal.attendees.iter().any(|a| a.person == recipient.person) {
            return Err(DomainError::new(
                ErrorCode::ForeignReference,
                "/extra/recipients",
            ));
        }
    }
    let base_fit = eligible(base, meal)?;
    if base_fit != Eligibility::Eligible {
        return Ok(base_fit);
    }
    let mut recipients = meal.clone();
    recipients
        .attendees
        .0
        .retain(|a| selected.recipients.iter().any(|s| s.person == a.person));
    let mut union = Vec::new();
    for attendee in &mut recipients.attendees.0 {
        for requirement in spec.eligible_person_requirements.iter() {
            if !attendee.hard_requirements.contains(requirement) {
                attendee.hard_requirements.0.push(requirement.clone());
            }
        }
        bound(
            attendee.hard_requirements.len(),
            0,
            64,
            "/extra/requirements",
        )?;
        for requirement in attendee.hard_requirements.iter() {
            if !union.contains(requirement) {
                union.push(requirement.clone());
            }
        }
    }
    bound(union.len(), 0, 64, "/extra/requirements")?;
    recipients.shared_hard_requirements = BoundedVec(union);
    let fit = eligible(extra, &recipients)?;
    if fit != Eligibility::Eligible {
        return Ok(fit);
    }
    // This API carries no per-meal override. Only a serving resolvable from its
    // supplied attendee preferences can be authorised here.
    let resolved = match crate::portions::resolve_servings(
        extra,
        &recipients,
        &MealOverrides {
            portions: BoundedVec(vec![]),
            eat_by: Nullable(None),
        },
    ) {
        Ok(v) => v,
        Err(e) => return Ok(Eligibility::Unresolved(e)),
    };
    for recipient in selected.recipients.iter() {
        if !resolved.contains(recipient) {
            return Ok(Eligibility::Ineligible(vec![conflict(
                extra,
                meal,
                ErrorCode::DomainConflict,
                vec![],
            )?]));
        }
    }
    Ok(Eligibility::Eligible)
}

pub(crate) fn bound(n: usize, min: usize, max: usize, path: &str) -> Result<(), DomainError> {
    if n < min {
        Err(DomainError::new(ErrorCode::InvalidValue, path))
    } else if n > max {
        Err(DomainError::new(ErrorCode::LimitExceeded, path))
    } else {
        Ok(())
    }
}
pub(crate) fn shape<T: ValidateShape>(value: &T, prefix: &str) -> Result<(), DomainError> {
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
pub(crate) fn check_meal(meal: &MealRequirements) -> Result<(), DomainError> {
    bound(meal.attendees.len(), 0, 64, "/meal/attendees")?;
    bound(
        meal.shared_hard_requirements.len(),
        0,
        64,
        "/meal/shared_hard_requirements",
    )?;
    bound(meal.equipment.len(), 0, 9, "/meal/equipment")?;
    for (i, a) in meal.attendees.iter().enumerate() {
        bound(
            a.hard_requirements.len(),
            0,
            64,
            "/meal/attendees/hard_requirements",
        )?;
        bound(
            a.portion.meal_overrides.len(),
            0,
            3,
            "/meal/attendees/portion/meal_overrides",
        )?;
        if meal.attendees[..i].iter().any(|p| p.person == a.person) {
            return Err(DomainError::new(ErrorCode::DuplicateKey, "/meal/attendees"));
        }
        for (j, o) in a.portion.meal_overrides.iter().enumerate() {
            if a.portion.meal_overrides[..j]
                .iter()
                .any(|p| p.meal_type == o.meal_type)
            {
                return Err(DomainError::new(
                    ErrorCode::DuplicateKey,
                    "/meal/attendees/portion/meal_overrides",
                ));
            }
        }
    }
    shape(meal, "/meal")?;
    if !meal.enabled || meal.location == MealLocation::Skip {
        return Err(DomainError::new(ErrorCode::DomainConflict, "/meal/enabled"));
    }
    Ok(())
}
pub(crate) fn recipe_ref(recipe: &ReviewedRecipe<'_>) -> Result<RecipeRef, DomainError> {
    let r = recipe.recipe();
    Ok(RecipeRef {
        recipe_id: r.recipe_id.clone(),
        revision: r.revision,
        digest: recipe_digest(r)?,
    })
}
pub(crate) fn conflict(
    recipe: &ReviewedRecipe<'_>,
    meal: &MealRequirements,
    code: ErrorCode,
    mut ids: Vec<EvidenceId>,
) -> Result<Conflict, DomainError> {
    ids.sort();
    ids.dedup();
    bound(ids.len(), 0, 256, "/conflict/evidence_ids")?;
    Ok(Conflict {
        code,
        week_ids: BoundedVec(vec![meal.slot.week_id.clone()]),
        slot_ids: BoundedVec(vec![meal.slot.clone()]),
        recipe_refs: BoundedVec(vec![recipe_ref(recipe)?]),
        evidence_ids: BoundedVec(ids),
        blocking: true,
    })
}
