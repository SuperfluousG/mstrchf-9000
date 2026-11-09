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
    eligible_internal(recipe, meal, false)
}

/// Only the exact admitted packing/service closure can qualify home equipment
/// for an on-the-move meal. The ordinary two-argument API remains conservative.
pub fn eligible_with_food(
    recipe: &ReviewedRecipe<'_>,
    meal: &MealRequirements,
    selection: &MealSelection,
    context: &PlanningContext,
    evidence: &crate::food::FoodEvidence,
) -> Result<Eligibility, DomainError> {
    crate::food::validate_evidence_shape(evidence)?;
    let assessed = recipe.recipe();
    if selection.recipe.recipe_id != assessed.recipe_id
        || selection.recipe.revision != assessed.revision
        || selection.recipe.digest != recipe_digest(assessed)?
    {
        return Err(DomainError::new(ErrorCode::IdentityMismatch, "/portable_cold/recipe"));
    }
    if evidence.catalogue_revision != context.stamp.catalogue_revision
        || crate::food::food_evidence_digest(evidence)? != evidence.catalogue_digest
        || context.requirements.iter().find(|m| m.slot == selection.slot) != Some(meal)
    {
        return Err(DomainError::new(ErrorCode::ContextStale, "/portable_cold/food_evidence"));
    }
    let method = crate::portable::method(recipe.recipe(), evidence)?;
    if method.is_some() != selection.portable_cold.is_some() {
        return Err(DomainError::new(ErrorCode::UnsupportedCapability, "/portable_cold/method"));
    }
    crate::portable::validate_journey(context, selection)?;
    eligible_internal(recipe, meal, method.is_some())
}

fn eligible_internal(
    recipe: &ReviewedRecipe<'_>,
    meal: &MealRequirements,
    portable_home_packing: bool,
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
    let (mut known,mut missing)=hard_conflicts(recipe,&meal.slot,&meal.shared_hard_requirements.0)?;
    if !r.meal_types.contains(&meal.slot.meal_type)
        || (!portable_home_packing && r.equipment.iter().any(|e| !meal.equipment.contains(e)))
    {
        known.push(conflict(recipe, meal, ErrorCode::DomainConflict, vec![])?);
    }
    // Equipment has no phase/location dimension in this contract. Its presence
    // cannot prove that an earlier preparation appliance is available on the move.
    if meal.location == MealLocation::OnTheMove && !r.equipment.is_empty() && !portable_home_packing {
        missing.push(conflict(recipe,meal,ErrorCode::UnsupportedCapability,vec![])?);
    }
    let portability = &r.portability;
    // An explicit recipe Fridge requirement is home equipment, rather than
    // evidence of an on-the-move cold route. Keep no-Fridge recipes on their
    // original mobility predicates so historical catalogue proofs do not drift.
    let home_fridge = meal.location == MealLocation::AtHome
        && r.equipment.contains(&EquipmentCode::Fridge);
    // Mobility describes access while travelling. An explicitly admitted home
    // Fridge/Hob method can reheat on the actual home Hob without that claim.
    // Keep no-Fridge recipes on their frozen mobility predicates.
    let home_reheating = home_fridge
        && r.equipment.contains(&EquipmentCode::Hob)
        && meal.equipment.contains(&EquipmentCode::Hob);
    let needs_portability = meal.location == MealLocation::OnTheMove
        || (!home_fridge && meal.mobility.temperature == MobilityTemperature::Cold)
        || portability.reheating_required || portability.cold_storage_required;
    if needs_portability {
        if portability.evidence_ids.is_empty() {missing.push(conflict(recipe,meal,ErrorCode::UnsupportedCapability,vec![])?);} else {
            if (portability.reheating_required && !home_reheating && !meal.mobility.reheating)
                || (!home_fridge && meal.mobility.temperature == MobilityTemperature::Cold && !portability.cold_suitable) {
                known.push(conflict(recipe,meal,ErrorCode::DomainConflict,portability.evidence_ids.0.clone())?);
            }
            let cold_storage = if home_fridge {
                meal.equipment.contains(&EquipmentCode::Fridge)
            } else {
                meal.mobility.cold_storage
            };
            if portability.cold_storage_required && !cold_storage {
                missing.push(conflict(recipe,meal,ErrorCode::UnsupportedCapability,portability.evidence_ids.0.clone())?);
            }
        }
    }
    coalesce(known,missing)
}

/// Checks hard requirements of a retained choice, without activating its cell,
/// resolving a meal time, calculating a portion or producing ingredient demand.
pub fn eligible_retained(recipe:&ReviewedRecipe<'_>,slot:&SlotId,requirements:&[HardRequirement])->Result<Eligibility,DomainError>{
    shape(slot,"/slot")?;bound(requirements.len(),0,64,"/requirements")?;
    for r in requirements {shape(r,"/requirements")?;}
    let (known,missing)=hard_conflicts(recipe,slot,requirements)?;coalesce(known,missing)
}
fn hard_conflicts(recipe:&ReviewedRecipe<'_>,slot:&SlotId,requirements:&[HardRequirement])->Result<(Vec<Conflict>,Vec<Conflict>),DomainError>{
    let r=recipe.recipe();let mut known=Vec::new();let mut missing=Vec::new();
    for requirement in requirements {
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
            known.push(conflict_at(recipe, slot, ErrorCode::DomainConflict, ids)?);
        } else if facts.is_empty() || facts.iter().any(|e| e.verdict != DietaryVerdict::Supported) {
            missing.push(conflict_at(
                recipe,
                slot,
                ErrorCode::UnsupportedCapability,
                ids,
            )?);
        }
    }
    Ok((known,missing))
}
fn coalesce(known:Vec<Conflict>,missing:Vec<Conflict>)->Result<Eligibility,DomainError>{
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

/// `overrides` must be the authoritative overrides of the same current meal
/// selection. This pure API checks their values; the caller establishes binding.
pub fn validate_extra(
    base: &ReviewedRecipe<'_>,
    extra: &ReviewedRecipe<'_>,
    selected: &ExtraSelection,
    meal: &MealRequirements,
    overrides: &MealOverrides,
) -> Result<Eligibility, DomainError> {
    check_meal(meal)?;
    // Check the entire supplied context before projecting recipients. Otherwise
    // foreign, duplicate or malformed nonrecipient overrides could disappear.
    bound(overrides.portions.len(), 0, 64, "/overrides/portions")?;
    shape(overrides, "/overrides")?;
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
    let mut recipient_overrides = overrides.clone();
    recipient_overrides
        .portions
        .0
        .retain(|o| recipients.attendees.iter().any(|a| a.person == o.person));
    // Only portions affect this calculation. eat_by was shape-checked above;
    // scheduling and binding to the current selection remain caller obligations.
    let resolved = match crate::portions::resolve_servings(extra, &recipients, &recipient_overrides)
    {
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
    Ok(recipe.reference().clone())
}
pub(crate) fn conflict(
    recipe: &ReviewedRecipe<'_>,
    meal: &MealRequirements,
    code: ErrorCode,
    mut ids: Vec<EvidenceId>,
) -> Result<Conflict, DomainError> {
    conflict_at(recipe,&meal.slot,code,ids)
}
fn conflict_at(recipe:&ReviewedRecipe<'_>,slot:&SlotId,code:ErrorCode,mut ids:Vec<EvidenceId>)->Result<Conflict,DomainError>{
    ids.sort();
    ids.dedup();
    bound(ids.len(), 0, 256, "/conflict/evidence_ids")?;
    Ok(Conflict {
        code,
        week_ids: BoundedVec(vec![slot.week_id.clone()]),
        slot_ids: BoundedVec(vec![slot.clone()]),
        recipe_refs: BoundedVec(vec![recipe_ref(recipe)?]),
        evidence_ids: BoundedVec(ids),
        blocking: true,
    })
}
