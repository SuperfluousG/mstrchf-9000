//! Native collection preflight before serde-backed shape validation.
use mstrchf_contracts::*;
pub(super) fn vec<T, const MIN: usize, const MAX: usize>(
    v: &BoundedVec<T, MIN, MAX>,
) -> Result<(), DomainError> {
    super::bound(v.len(), MIN, MAX, "/bounds")
}
fn source(s: &SourceRecord) -> Result<(), DomainError> {
    vec(&s.authors)
}
fn rights(r: &RightsRecord) -> Result<(), DomainError> {
    vec(&r.adaptations)?;
    vec(&r.evidence_ids)
}
fn restriction(r: &RestrictionDeclaration) -> Result<(), DomainError> {
    if let RestrictionDeclaration::Confirmed { requirements, .. } = r {
        vec(requirements)?;
    }
    Ok(())
}
fn usual(w: &UsualWeek) -> Result<(), DomainError> {
    vec(&w.enabled_meal_types)?;
    vec(&w.day_enabled)?;
    vec(&w.cells)?;
    for c in &w.cells.0 {
        vec(&c.attendee_ids)?;
    }
    Ok(())
}
fn recipe(r: &RecipeRevision) -> Result<(), DomainError> {
    source(&r.source)?;
    rights(&r.text_rights)?;
    vec(&r.ingredients)?;
    vec(&r.r#yield.components)?;
    vec(&r.steps)?;
    vec(&r.equipment)?;
    vec(&r.meal_types)?;
    vec(&r.portability.evidence_ids)?;
    vec(&r.components)?;
    vec(&r.extras)?;
    vec(&r.reused_inputs)?;
    vec(&r.dietary_evidence)?;
    vec(&r.storage)?;
    vec(&r.prep)?;
    vec(&r.portion_presets)?;
    vec(&r.reviews)?;
    vec(&r.capabilities.blocking_issues)?;
    for i in &r.ingredients.0 {
        vec(&i.step_ids)?;
        vec(&i.conversion_evidence)?;
        for e in &i.conversion_evidence.0 {
            source(&e.source)?;
        }
    }
    for s in &r.steps.0 {
        vec(&s.ingredient_ids)?;
        vec(&s.component_ids)?;
        vec(&s.depends_on)?;
    }
    for c in &r.components.0 {
        vec(&c.ingredient_ids)?;
        vec(&c.storage_rule_ids)?;
    }
    for e in &r.extras.0 {
        vec(&e.eligible_person_requirements)?;
        vec(&e.supported_with)?;
    }
    for i in &r.reused_inputs.0 {
        vec(&i.step_ids)?;
        vec(&i.evidence_ids)?;
    }
    for e in &r.dietary_evidence.0 {
        source(&e.source)?;
    }
    for s in &r.storage.0 {
        source(&s.source)?;
    }
    for p in &r.prep.0 {
        vec(&p.step_ids)?;
        vec(&p.depends_on)?;
        vec(&p.ingredient_ids)?;
    }
    for p in &r.portion_presets.0 {
        vec(&p.evidence_ids)?;
    }
    for i in &r.capabilities.blocking_issues.0 {
        vec(&i.evidence_ids)?;
    }
    if let Some(n) = &r.nutrition.0 {
        source(&n.source)?;
    }
    if let Some(i) = &r.image.0 {
        source(&i.source)?;
        rights(&i.rights)?;
    }
    Ok(())
}
pub(super) fn context(c: &PlanningContext) -> Result<(), DomainError> {
    vec(&c.weeks)?;
    vec(&c.requirements)?;
    vec(&c.recipes)?;
    vec(&c.stamp.recipe_refs)?;
    vec(&c.stamp.read_weeks)?;
    vec(&c.stamp.write_weeks)?;
    vec(&c.history.selected)?;
    vec(&c.history.ordered)?;
    vec(&c.history.feedback)?;
    vec(&c.history.hidden_recipe_ids)?;
    for h in c
        .history
        .selected
        .iter()
        .chain(c.history.ordered.iter().map(|o| &o.selection))
    {
        crate::history::selected_bounds(h)?;
    }
    for f in &c.history.feedback.0 {
        vec(&f.reasons)?;
    }
    let h = &c.household;
    vec(&h.diners)?;
    vec(&h.preferences.dislikes)?;
    vec(&h.preferences.equipment)?;
    vec(&h.preferences.meal_times)?;
    usual(&h.usual_week)?;
    vec(&h.connections.delivery_preferences)?;
    vec(&h.connections.reminders.channels)?;
    vec(&h.connections.reminders.prompts)?;
    for d in &h.diners.0 {
        restriction(&d.restrictions)?;
        vec(&d.dislikes)?;
        vec(&d.portions.meal_overrides)?;
    }
    for w in &c.weeks.0 {
        crate::history::week_bounds(w)?;
    }
    for r in &c.requirements.0 {
        vec(&r.attendees)?;
        vec(&r.shared_hard_requirements)?;
        vec(&r.equipment)?;
        for a in &r.attendees.0 {
            vec(&a.hard_requirements)?;
            vec(&a.portion.meal_overrides)?;
        }
    }
    for r in &c.recipes.0 {
        recipe(r)?;
    }
    Ok(())
}
