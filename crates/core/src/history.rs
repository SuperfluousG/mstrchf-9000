//! Pure selected-index deltas. Admission and persistence belong to the store.
use mstrchf_contracts::*;
use std::collections::BTreeSet;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectedIndexDelta {
    pub remove_slots: Vec<SlotId>,
    pub upsert: Vec<SelectionHistory>,
}

/// Compare one bounded batch of genuine retained weeks. This cannot establish
/// completeness of the caller's current selected index or allocate revisions.
pub fn selected_index_delta(
    before: &[Week],
    after: &[Week],
    committed_at: &Instant,
) -> Result<SelectedIndexDelta, DomainError> {
    if !(1..=4).contains(&before.len()) || !(1..=4).contains(&after.len()) {
        return Err(DomainError::new(ErrorCode::LimitExceeded, "/weeks"));
    }
    if before.len() != after.len() {
        return Err(DomainError::new(ErrorCode::IdentityMismatch, "/weeks"));
    }
    shape(committed_at)?;
    for weeks in [before, after] {
        let mut ids = BTreeSet::new();
        for week in weeks {
            week_bounds(week)?;
            shape(week)?;
            if !ids.insert(&week.range.week_id) {
                return Err(DomainError::new(ErrorCode::InvalidValue, "/weeks"));
            }
        }
    }
    let mut delta = SelectedIndexDelta {
        remove_slots: vec![],
        upsert: vec![],
    };
    for old in before {
        let new = after
            .iter()
            .find(|w| w.range.week_id == old.range.week_id)
            .ok_or_else(|| DomainError::new(ErrorCode::IdentityMismatch, "/weeks"))?;
        if old.range != new.range {
            return Err(DomainError::new(
                ErrorCode::IdentityMismatch,
                "/weeks/range",
            ));
        }
        let old_active = active_slots(old)?;
        let new_active = active_slots(new)?;
        for selection in &old.plan.selections.0 {
            if old_active.contains(&selection.slot)
                && (!new_active.contains(&selection.slot)
                    || !new.plan.selections.iter().any(|s| s.slot == selection.slot))
            {
                delta.remove_slots.push(selection.slot.clone());
            }
        }
        for selection in &new.plan.selections.0 {
            if !new_active.contains(&selection.slot) {
                continue;
            }
            let previous = old
                .plan
                .selections
                .iter()
                .find(|s| s.slot == selection.slot);
            if old_active.contains(&selection.slot) && previous == Some(selection) {
                continue;
            }
            delta.upsert.push(SelectionHistory {
                week_id: selection.slot.week_id.clone(),
                slot: selection.slot.clone(),
                recipe: selection.recipe.clone(),
                servings: selection.servings.clone(),
                extras: selection.extras.clone(),
                repeat_authorised: selection.repeat_authorised,
                selected_at: committed_at.clone(),
            });
        }
    }
    delta.remove_slots.sort_by_key(slot_key);
    delta.upsert.sort_by_key(|s| slot_key(&s.slot));
    validate_delta(&delta)?;
    Ok(delta)
}

/// Validate a native delta before using it; this is not transaction authority.
pub fn validate_delta(delta: &SelectedIndexDelta) -> Result<(), DomainError> {
    if delta.remove_slots.len() > 84 || delta.upsert.len() > 84 {
        return Err(DomainError::new(ErrorCode::LimitExceeded, "/delta"));
    }
    let mut slots = BTreeSet::new();
    for slot in &delta.remove_slots {
        shape(slot)?;
        if !slots.insert(slot_key(slot)) {
            return Err(DomainError::new(
                ErrorCode::InvalidValue,
                "/delta/remove_slots",
            ));
        }
    }
    for record in &delta.upsert {
        selected_bounds(record)?;
        shape(record)?;
        if !slots.insert(slot_key(&record.slot)) {
            return Err(DomainError::new(ErrorCode::InvalidValue, "/delta/upsert"));
        }
    }
    Ok(())
}

/// Native current projection is stricter than historical wire identity.
pub(crate) fn validate_selected_projection(history: &PlanningHistory) -> Result<(), DomainError> {
    if history.selected.len() > 4096 {
        return Err(DomainError::new(
            ErrorCode::LimitExceeded,
            "/history/selected",
        ));
    }
    let mut slots = BTreeSet::new();
    for record in &history.selected.0 {
        if !slots.insert(slot_key(&record.slot)) {
            return Err(DomainError::new(
                ErrorCode::InvalidValue,
                "/history/selected",
            ));
        }
    }
    Ok(())
}

pub(crate) fn week_bounds(week: &Week) -> Result<(), DomainError> {
    bounded(&week.cells, "/weeks/cells")?;
    bounded(&week.day_enabled, "/weeks/day_enabled")?;
    usual_bounds(&week.defaults_snapshot)?;
    bounded(&week.plan.selections, "/weeks/plan/selections")?;
    bounded(&week.plan.batch_links, "/weeks/plan/batch_links")?;
    for cell in &week.cells.0 {
        bounded(&cell.attendee_ids, "/weeks/cells/attendee_ids")?;
        if let Some(occasion) = &cell.occasion.0 {
            bounded(&occasion.guests, "/weeks/cells/occasion/guests")?;
            bounded(
                &occasion.host_diner_ids,
                "/weeks/cells/occasion/host_diner_ids",
            )?;
            for guest in &occasion.guests.0 {
                bounded(
                    &guest.portion.meal_overrides,
                    "/weeks/guests/portion/meal_overrides",
                )?;
                restriction_bounds(&guest.restrictions)?;
            }
        }
    }
    for selection in &week.plan.selections.0 {
        selection_bounds(selection)?;
    }
    for link in &week.plan.batch_links.0 {
        bounded(&link.components, "/weeks/plan/batch_links/components")?;
    }
    if let Some(approval) = &week.approved_plan.0 {
        stamp_bounds(&approval.context_stamp)?;
    }
    if let Some(order) = &week.confirmed_order.0 {
        bounded(
            &order.ordered_recipe_refs,
            "/weeks/confirmed_order/ordered_recipe_refs",
        )?;
        stamp_bounds(&order.approved_plan.context_stamp)?;
    }
    Ok(())
}
pub(crate) fn selection_bounds(selection: &MealSelection) -> Result<(), DomainError> {
    bounded(&selection.servings, "/selection/servings")?;
    extras_bounds(&selection.extras)?;
    bounded(
        &selection.meal_overrides.portions,
        "/selection/meal_overrides/portions",
    )?;
    if let BatchUse::Leftover { component_ids, .. } = &selection.batch_use {
        bounded(component_ids, "/selection/batch_use/component_ids")?;
    }
    Ok(())
}
pub(crate) fn selected_bounds(record: &SelectionHistory) -> Result<(), DomainError> {
    bounded(&record.servings, "/history/selected/servings")?;
    extras_bounds(&record.extras)
}
fn extras_bounds(extras: &BoundedVec<ExtraSelection, 0, 64>) -> Result<(), DomainError> {
    bounded(extras, "/selection/extras")?;
    for extra in &extras.0 {
        bounded(&extra.recipients, "/selection/extras/recipients")?;
    }
    Ok(())
}
pub(crate) fn usual_bounds(usual: &UsualWeek) -> Result<(), DomainError> {
    bounded(&usual.enabled_meal_types, "/usual_week/enabled_meal_types")?;
    bounded(&usual.day_enabled, "/usual_week/day_enabled")?;
    bounded(&usual.cells, "/usual_week/cells")?;
    for cell in &usual.cells.0 {
        bounded(&cell.attendee_ids, "/usual_week/cells/attendee_ids")?;
    }
    Ok(())
}
pub(crate) fn restriction_bounds(restrictions: &RestrictionDeclaration) -> Result<(), DomainError> {
    if let RestrictionDeclaration::Confirmed { requirements, .. } = restrictions {
        bounded(requirements, "/restrictions/requirements")?;
    }
    Ok(())
}
pub(crate) fn stamp_bounds(stamp: &ContextStamp) -> Result<(), DomainError> {
    bounded(&stamp.recipe_refs, "/stamp/recipe_refs")?;
    bounded(&stamp.read_weeks, "/stamp/read_weeks")?;
    bounded(&stamp.write_weeks, "/stamp/write_weeks")
}
pub(crate) fn bounded<T, const MIN: usize, const MAX: usize>(
    value: &BoundedVec<T, MIN, MAX>,
    path: &str,
) -> Result<(), DomainError> {
    if value.len() < MIN || value.len() > MAX {
        Err(DomainError::new(ErrorCode::LimitExceeded, path))
    } else {
        Ok(())
    }
}

fn slot_key(slot: &SlotId) -> (WeekId, LocalDate, MealType) {
    (slot.week_id.clone(), slot.date.clone(), slot.meal_type)
}
fn shape(value: &impl ValidateShape) -> Result<(), DomainError> {
    value.validate_shape().map_err(|issues| DomainError {
        code: issues.first().map_or(ErrorCode::InvalidValue, |i| i.code),
        truncated: issues.len() > 100,
        issues: BoundedVec(issues.into_iter().take(100).collect()),
        retry: Retry::Never,
    })
}
