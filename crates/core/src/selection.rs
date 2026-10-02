//! Bounded pure selection checks. Native intent is data, never write authority.
//! Suitable proves Food A/B fit only; full food derivation and store CAS remain required.
use crate::{
    availability::{SlotResolution, meal_requirements},
    eligibility::{Eligibility, bound, eligible, shape, validate_extra},
    portions::resolve_servings,
    recipes::ReviewedRecipe,
};
use chrono::{Datelike, NaiveDate};
use mstrchf_contracts::*;
use std::collections::{BTreeMap, BTreeSet};
mod bounds;

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RepeatPermission {
    pub slot: SlotId,
    pub recipe: RecipeRef,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionIntent {
    pub mode: PlanMode,
    pub slots: Vec<SlotId>,
    pub origin: SelectionOrigin,
    pub repeat_permissions: Vec<RepeatPermission>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ExtraChoice {
    pub extra_id: ExtraId,
    pub recipe: RecipeRef,
    pub recipients: Vec<PersonRef>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChoiceSpec {
    pub recipe: RecipeRef,
    pub extras: Vec<ExtraChoice>,
    pub meal_overrides: MealOverrides,
    pub batch_use: BatchUse,
}
#[derive(Clone, Debug, PartialEq, Eq)]
// The adopted native API carries MealSelection directly; boxing changes that signature.
#[allow(clippy::large_enum_variant)]
pub enum ChoiceAssessment {
    Suitable(MealSelection),
    Ineligible(Vec<Conflict>),
    Unresolved(Vec<Conflict>),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SelectionPatch {
    pub context: PlanningContext,
    pub changed_slots: Vec<SlotId>,
    pub rederive_weeks: Vec<WeekId>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SelectionFailure {
    Invalid(DomainError),
    Blocked(Vec<Conflict>),
}
impl From<DomainError> for SelectionFailure {
    fn from(e: DomainError) -> Self {
        Self::Invalid(e)
    }
}

fn error(code: ErrorCode, path: &str) -> DomainError {
    DomainError::new(code, path)
}

/// Strict admission of the current selected projection, not historical receipts.
pub(crate) fn validate_current_selected(history: &PlanningHistory) -> Result<(), DomainError> {
    crate::history::validate_selected_projection(history)
}

/// Checks native bounds and exact target membership; authentication is caller-owned.
pub(crate) fn validate_intent(
    context: &PlanningContext,
    intent: &SelectionIntent,
) -> Result<(), DomainError> {
    bound(intent.slots.len(), 1, 84, "/intent/slots")?;
    bound(
        intent.repeat_permissions.len(),
        0,
        84,
        "/intent/repeat_permissions",
    )?;
    let mut slots = BTreeSet::new();
    for slot in &intent.slots {
        shape(slot, "/intent/slots")?;
        if !slots.insert(canonical_bytes(slot)?) {
            return Err(error(ErrorCode::InvalidValue, "/intent/slots"));
        }
        if !context
            .weeks
            .iter()
            .any(|w| w.cells.iter().any(|c| &c.slot == slot))
        {
            return Err(error(ErrorCode::ForeignReference, "/intent/slots"));
        }
        if !context
            .stamp
            .write_weeks
            .iter()
            .any(|w| w.week_id == slot.week_id)
        {
            return Err(error(ErrorCode::AuthorityDenied, "/intent/slots"));
        }
    }
    let mut permissions = BTreeSet::new();
    for p in &intent.repeat_permissions {
        shape(&p.slot, "/intent/repeat_permissions/slot")?;
        shape(&p.recipe, "/intent/repeat_permissions/recipe")?;
        if !permissions.insert(canonical_bytes(&p.slot)?) {
            return Err(error(ErrorCode::InvalidValue, "/intent/repeat_permissions"));
        }
        if !intent.slots.contains(&p.slot) || !context.stamp.recipe_refs.contains(&p.recipe) {
            return Err(error(
                ErrorCode::ForeignReference,
                "/intent/repeat_permissions",
            ));
        }
    }
    Ok(())
}

fn constituents(c: &PlanningContext) -> Result<(), DomainError> {
    bounds::context(c)?;
    // Bound public vectors before validators, serialisation, copies or expansion.
    bound(c.weeks.len(), 0, 4, "/weeks")?;
    bound(c.requirements.len(), 0, 84, "/requirements")?;
    bound(c.recipes.len(), 0, 512, "/recipes")?;
    bound(c.history.selected.len(), 0, 4096, "/history/selected")?;
    bound(c.history.ordered.len(), 0, 4096, "/history/ordered")?;
    bound(c.history.feedback.len(), 0, 4096, "/history/feedback")?;
    bound(
        c.history.hidden_recipe_ids.len(),
        0,
        512,
        "/history/hidden_recipe_ids",
    )?;
    shape(&c.schema_version, "")?;
    shape(&c.household, "/household")?;
    shape(&c.settings_revision, "/settings_revision")?;
    shape(&c.privacy, "/privacy")?;
    shape(&c.weeks, "/weeks")?;
    shape(&c.requirements, "/requirements")?;
    shape(&c.recipes, "/recipes")?;
    shape(&c.stamp, "/stamp")?;
    shape(&c.history, "/history")?;
    shape(&c.ranking_policy, "/ranking_policy")?;
    validate_current_selected(&c.history)
}

fn retained_resolutions(c: &PlanningContext, w: &Week) -> Vec<SlotResolution> {
    c.requirements
        .iter()
        .filter(|r| {
            r.enabled
                && r.slot.week_id == w.range.week_id
                && !w
                    .plan
                    .selections
                    .iter()
                    .any(|s| s.slot == r.slot && s.meal_overrides.eat_by.0.is_some())
        })
        .filter_map(|r| {
            r.eat_by
                .0
                .as_ref()
                .filter(|t| t.resolution != Resolution::Unambiguous)
                .map(|t| SlotResolution {
                    slot: r.slot.clone(),
                    resolution: t.resolution,
                })
        })
        .collect()
}

/// Full ordinary-context admission plus genuine A3 projection binding.
pub(crate) fn validate_selection_context(c: &PlanningContext) -> Result<(), DomainError> {
    constituents(c)?;
    shape(c, "")?;
    for w in &c.weeks.0 {
        for actual in meal_requirements(&c.household, w, &retained_resolutions(c, w))? {
            if c.requirements.iter().find(|r| r.slot == actual.slot) != Some(&actual) {
                return Err(error(ErrorCode::DomainConflict, "/requirements"));
            }
        }
    }
    Ok(())
}

fn choice_shape(choice: &ChoiceSpec) -> Result<(), DomainError> {
    bound(choice.extras.len(), 0, 64, "/choice/extras")?;
    bound(
        choice.meal_overrides.portions.len(),
        0,
        64,
        "/choice/meal_overrides/portions",
    )?;
    if let BatchUse::Leftover { component_ids, .. } = &choice.batch_use {
        bounds::vec(component_ids)?;
    }
    shape(&choice.recipe, "/choice/recipe")?;
    shape(&choice.meal_overrides, "/choice/meal_overrides")?;
    shape(&choice.batch_use, "/choice/batch_use")?;
    let mut extras = BTreeSet::new();
    for e in &choice.extras {
        bound(e.recipients.len(), 1, 64, "/choice/extras/recipients")?;
        shape(&e.extra_id, "/choice/extras/extra_id")?;
        shape(&e.recipe, "/choice/extras/recipe")?;
        if !extras.insert(e.extra_id.clone()) {
            return Err(error(ErrorCode::InvalidValue, "/choice/extras"));
        }
        let mut people = BTreeSet::new();
        for person in &e.recipients {
            shape(person, "/choice/extras/recipients")?;
            if !people.insert(canonical_bytes(person)?) {
                return Err(error(ErrorCode::InvalidValue, "/choice/extras/recipients"));
            }
        }
    }
    Ok(())
}

fn reviewed_inputs(
    c: &PlanningContext,
    reviewed: &[ReviewedRecipe<'_>],
) -> Result<(), DomainError> {
    bound(reviewed.len(), 0, 512, "/reviewed")?;
    let mut seen = BTreeSet::new();
    for rr in reviewed {
        let rf = crate::eligibility::recipe_ref(rr)?;
        if !seen.insert(canonical_bytes(&rf)?) {
            return Err(error(ErrorCode::InvalidValue, "/reviewed"));
        }
        if !c.stamp.recipe_refs.contains(&rf) || !c.recipes.iter().any(|r| r == rr.recipe()) {
            return Err(error(ErrorCode::IdentityMismatch, "/reviewed"));
        }
    }
    Ok(())
}

fn resolve<'a, 'r>(
    c: &PlanningContext,
    rf: &RecipeRef,
    reviewed: &'a [ReviewedRecipe<'r>],
) -> Result<&'a ReviewedRecipe<'r>, DomainError> {
    if !c.stamp.recipe_refs.contains(rf) {
        return Err(error(ErrorCode::MissingReference, "/choice/recipe"));
    }
    reviewed
        .iter()
        .find(|r| r.recipe().recipe_id == rf.recipe_id && r.recipe().revision == rf.revision)
        .ok_or_else(|| error(ErrorCode::MissingReference, "/reviewed"))
}

fn outcome(e: Eligibility) -> Option<ChoiceAssessment> {
    match e {
        Eligibility::Eligible => None,
        Eligibility::Ineligible(c) => Some(ChoiceAssessment::Ineligible(c)),
        Eligibility::Unresolved(c) => Some(ChoiceAssessment::Unresolved(c)),
    }
}

pub fn assess_choice(
    c: &PlanningContext,
    slot: &SlotId,
    choice: &ChoiceSpec,
    origin: SelectionOrigin,
    repeat_authorised: bool,
    reviewed: &[ReviewedRecipe<'_>],
) -> Result<ChoiceAssessment, DomainError> {
    validate_selection_context(c)?;
    assess_choice_in_validated_context(c, slot, choice, origin, repeat_authorised, reviewed)
}

/// Crate-private optimisation for a bounded pool already admitted against its
/// genuine full context. No fabricated context or public acceptance token.
pub(crate) fn assess_choice_in_validated_context(
    c: &PlanningContext,
    slot: &SlotId,
    choice: &ChoiceSpec,
    origin: SelectionOrigin,
    repeat_authorised: bool,
    reviewed: &[ReviewedRecipe<'_>],
) -> Result<ChoiceAssessment, DomainError> {
    choice_shape(choice)?;
    reviewed_inputs(c, reviewed)?;
    food_choice(c, slot, choice, origin, repeat_authorised, reviewed)
}

// Only callers that have admitted the genuine complete candidate can reach here.
fn food_choice(
    c: &PlanningContext,
    slot: &SlotId,
    choice: &ChoiceSpec,
    origin: SelectionOrigin,
    repeat_authorised: bool,
    reviewed: &[ReviewedRecipe<'_>],
) -> Result<ChoiceAssessment, DomainError> {
    shape(slot, "/slot")?;
    choice_shape(choice)?;
    let w = c
        .weeks
        .iter()
        .find(|w| w.range.week_id == slot.week_id)
        .ok_or_else(|| error(ErrorCode::ForeignReference, "/slot"))?;
    let meal = c
        .requirements
        .iter()
        .find(|m| &m.slot == slot)
        .ok_or_else(|| error(ErrorCode::MissingReference, "/requirements"))?;
    if !meal.enabled || !active_slots(w)?.contains(slot) {
        return Err(error(ErrorCode::DomainConflict, "/slot"));
    }
    let base = resolve(c, &choice.recipe, reviewed)?;
    if matches!(choice.batch_use, BatchUse::NewBatch { .. })
        && !base.recipe().reused_inputs.is_empty()
    {
        return Err(error(ErrorCode::DomainConflict, "/choice/batch_use"));
    }
    for e in &choice.extras {
        resolve(c, &e.recipe, reviewed)?;
        if !base
            .recipe()
            .extras
            .iter()
            .any(|spec| spec.extra_id == e.extra_id && spec.recipe == e.recipe)
        {
            return Err(error(ErrorCode::ForeignReference, "/choice/extras"));
        }
        if e.recipients
            .iter()
            .any(|p| !meal.attendees.iter().any(|a| &a.person == p))
        {
            return Err(error(
                ErrorCode::ForeignReference,
                "/choice/extras/recipients",
            ));
        }
    }
    for o in &choice.meal_overrides.portions.0 {
        if !meal.attendees.iter().any(|a| a.person == o.person) {
            return Err(error(
                ErrorCode::ForeignReference,
                "/choice/meal_overrides/portions",
            ));
        }
    }
    if let Some(t) = &choice.meal_overrides.eat_by.0
        && (t.local_date != slot.date || t.zone != w.range.zone)
    {
        return Err(error(
            ErrorCode::IdentityMismatch,
            "/choice/meal_overrides/eat_by",
        ));
    }
    if let Some(result) = outcome(eligible(base, meal)?) {
        return Ok(result);
    }
    let servings = match resolve_servings(base, meal, &choice.meal_overrides) {
        Ok(s) => s,
        Err(c) => return Ok(ChoiceAssessment::Unresolved(c)),
    };
    let mut extras = Vec::new();
    for e in &choice.extras {
        let rr = resolve(c, &e.recipe, reviewed)?;
        let mut recipients = meal.clone();
        recipients
            .attendees
            .0
            .retain(|a| e.recipients.contains(&a.person));
        let mut union = BTreeMap::new();
        for a in &recipients.attendees.0 {
            for h in &a.hard_requirements.0 {
                union.insert(canonical_bytes(h)?, h.clone());
            }
        }
        bound(union.len(), 0, 64, "/choice/extras/requirements")?;
        recipients.shared_hard_requirements = BoundedVec(union.into_values().collect());
        let mut overrides = choice.meal_overrides.clone();
        overrides
            .portions
            .0
            .retain(|o| e.recipients.contains(&o.person));
        let amounts = match resolve_servings(rr, &recipients, &overrides) {
            Ok(s) => s,
            Err(c) => return Ok(ChoiceAssessment::Unresolved(c)),
        };
        let selected = ExtraSelection {
            extra_id: e.extra_id.clone(),
            recipe: e.recipe.clone(),
            recipients: BoundedVec(amounts),
        };
        if let Some(result) = outcome(validate_extra(
            base,
            rr,
            &selected,
            meal,
            &choice.meal_overrides,
        )?) {
            return Ok(result);
        }
        extras.push(selected);
    }
    let selection = MealSelection {
        slot: slot.clone(),
        recipe: choice.recipe.clone(),
        origin,
        repeat_authorised,
        servings: BoundedVec(servings),
        extras: BoundedVec(extras),
        batch_use: choice.batch_use.clone(),
        meal_overrides: choice.meal_overrides.clone(),
    };
    shape(&selection, "/selection")?;
    Ok(ChoiceAssessment::Suitable(selection))
}

fn spec(s: &MealSelection) -> ChoiceSpec {
    ChoiceSpec {
        recipe: s.recipe.clone(),
        extras: s
            .extras
            .iter()
            .map(|e| ExtraChoice {
                extra_id: e.extra_id.clone(),
                recipe: e.recipe.clone(),
                recipients: e.recipients.iter().map(|p| p.person.clone()).collect(),
            })
            .collect(),
        meal_overrides: s.meal_overrides.clone(),
        batch_use: s.batch_use.clone(),
    }
}

pub fn prepare_selection_patch(
    c: &PlanningContext,
    intent: &SelectionIntent,
    proposal: &PlanProposal,
    reviewed: &[ReviewedRecipe<'_>],
    resolutions: &[SlotResolution],
) -> Result<SelectionPatch, SelectionFailure> {
    constituents(c)?;
    validate_intent(c, intent)?;
    reviewed_inputs(c, reviewed)?;
    bound(proposal.changes.len(), 1, 4, "/proposal/changes")?;
    bound(resolutions.len(), 0, 84, "/resolutions")?;
    for change in &proposal.changes.0 {
        bounds::vec(&change.replace_slots)?;
        for p in &change.replace_slots.0 {
            if let Some(s) = &p.selection.0 {
                crate::history::selection_bounds(s)?;
            }
        }
    }
    shape(proposal, "/proposal")?;
    if proposal.changes.len() != c.stamp.write_weeks.len()
        || c.stamp.write_weeks.iter().any(|w| {
            !proposal
                .changes
                .iter()
                .any(|p| p.week_id == w.week_id && p.base_revision == w.revision)
        })
    {
        return Err(error(ErrorCode::ContextStale, "/proposal/changes").into());
    }
    let mut seen = BTreeSet::new();
    for r in resolutions {
        shape(&r.slot, "/resolutions/slot")?;
        if !seen.insert(canonical_bytes(&r.slot)?) {
            return Err(error(ErrorCode::InvalidValue, "/resolutions").into());
        }
        if !c
            .weeks
            .iter()
            .any(|w| w.cells.iter().any(|cell| cell.slot == r.slot))
        {
            return Err(error(ErrorCode::ForeignReference, "/resolutions").into());
        }
    }
    let mut candidate = c.clone();
    let mut changed = Vec::new();
    for change in &proposal.changes.0 {
        let original = c
            .weeks
            .iter()
            .find(|w| w.range.week_id == change.week_id)
            .ok_or_else(|| error(ErrorCode::ForeignReference, "/proposal/changes/week_id"))?;
        if change.base_revision != original.revision {
            return Err(error(ErrorCode::ContextStale, "/proposal/changes/base_revision").into());
        }
        let target = candidate
            .weeks
            .0
            .iter_mut()
            .find(|w| w.range.week_id == change.week_id)
            .ok_or_else(|| error(ErrorCode::ForeignReference, "/proposal/changes/week_id"))?;
        for replacement in &change.replace_slots.0 {
            let old = original
                .plan
                .selections
                .iter()
                .find(|s| s.slot == replacement.slot);
            let new = replacement.selection.0.as_ref();
            if !intent.slots.contains(&replacement.slot)
                || !active_slots(original)?.contains(&replacement.slot)
            {
                return Err(error(ErrorCode::AuthorityDenied, "/proposal/replace_slots").into());
            }
            let permitted = match intent.mode {
                PlanMode::FillOpen => old.is_none() && new.is_some(),
                PlanMode::SwapNamed => true,
                PlanMode::ReplaceSuggestions => {
                    old.is_some_and(|s| s.origin == SelectionOrigin::Suggested)
                }
            };
            if !permitted {
                return Err(error(ErrorCode::AuthorityDenied, "/proposal/replace_slots").into());
            }
            if old == new {
                continue;
            }
            if let Some(s) = new {
                let permission = intent
                    .repeat_permissions
                    .iter()
                    .any(|p| p.slot == s.slot && p.recipe == s.recipe)
                    || old.is_some_and(|o| o.recipe == s.recipe && o.repeat_authorised);
                if s.origin != intent.origin || s.repeat_authorised != permission {
                    return Err(error(
                        ErrorCode::AuthorityDenied,
                        "/proposal/selection/origin_repeat",
                    )
                    .into());
                }
                if s.origin == SelectionOrigin::Suggested
                    && c.history.hidden_recipe_ids.contains(&s.recipe.recipe_id)
                {
                    return Err(
                        error(ErrorCode::AuthorityDenied, "/proposal/selection/hidden").into(),
                    );
                }
            }
            if let Some(index) = target
                .plan
                .selections
                .iter()
                .position(|s| s.slot == replacement.slot)
            {
                if let Some(s) = new {
                    target.plan.selections.0[index] = s.clone();
                } else {
                    target.plan.selections.0.remove(index);
                }
            } else if let Some(s) = new {
                target.plan.selections.0.push(s.clone());
            }
            changed.push(replacement.slot.clone());
        }
    }
    // Real A3 sees the complete tentative week, including submitted overrides.
    // Rebuild affected weeks; unaffected stale requirements remain errors.
    for w in &candidate.weeks.0 {
        let mut local: Vec<_> = resolutions
            .iter()
            .filter(|r| r.slot.week_id == w.range.week_id)
            .cloned()
            .collect();
        for r in retained_resolutions(c, w) {
            if !local.iter().any(|given| given.slot == r.slot) {
                local.push(r);
            }
        }
        let projected = meal_requirements(&candidate.household, w, &local)?;
        if changed.iter().any(|s| s.week_id == w.range.week_id) {
            for r in projected {
                let old = candidate
                    .requirements
                    .0
                    .iter_mut()
                    .find(|x| x.slot == r.slot)
                    .ok_or_else(|| error(ErrorCode::MissingReference, "/requirements"))?;
                *old = r;
            }
        } else {
            for r in projected {
                if candidate.requirements.iter().find(|x| x.slot == r.slot) != Some(&r) {
                    return Err(error(ErrorCode::DomainConflict, "/requirements").into());
                }
            }
        }
    }
    shape(&candidate, "")?;
    let mut conflicts = Vec::new();
    // Structural shape and A3 attendance are necessary but do not establish food
    // eligibility or exact portions. Rebuild every active choice, including retained
    // ones; inactive choices remain untouched and outside this food assessment.
    for week in &candidate.weeks.0 {
        for s in active_selections(week) {
            match food_choice(
                &candidate,
                &s.slot,
                &spec(s),
                s.origin,
                s.repeat_authorised,
                reviewed,
            )? {
                ChoiceAssessment::Suitable(actual) => {
                    if &actual != s {
                        return Err(error(
                            ErrorCode::IdentityMismatch,
                            "/proposal/selection/servings_extras",
                        )
                        .into());
                    }
                }
                ChoiceAssessment::Ineligible(mut e) | ChoiceAssessment::Unresolved(mut e) => {
                    if conflicts.len() + e.len() > 100 {
                        return Err(error(ErrorCode::LimitExceeded, "/conflicts").into());
                    }
                    conflicts.append(&mut e);
                }
            }
            // Retained repeats are not new actions. Food rechecking must not widen
            // the repeat-permission policy beyond changed/new non-null targets.
            if changed.contains(&s.slot) && repeat_needs_permission(&candidate, s)? {
                conflicts.push(repeat_conflict(s));
                bound(conflicts.len(), 0, 100, "/conflicts")?;
            }
        }
    }
    if !conflicts.is_empty() {
        return Err(SelectionFailure::Blocked(conflicts));
    }
    changed.sort_by_key(|s| canonical_bytes(s).unwrap_or_default());
    let rederive_weeks = owners(c, &candidate, &changed)?;
    Ok(SelectionPatch {
        context: candidate,
        changed_slots: changed,
        rederive_weeks,
    })
}

fn monday(date: &LocalDate) -> Result<NaiveDate, DomainError> {
    shape(date, "/date")?;
    let date = NaiveDate::parse_from_str(&date.0, "%Y-%m-%d")
        .map_err(|_| error(ErrorCode::InvalidValue, "/date"))?;
    date.checked_sub_signed(chrono::Duration::days(i64::from(
        date.weekday().num_days_from_monday(),
    )))
    .ok_or_else(|| error(ErrorCode::ArithmeticOverflow, "/date"))
}

/// Checks the one-target counterfactual against every other actual retained slot.
/// Consulted draft cells supersede selected history; ordered rows never disappear.
pub(crate) fn repeat_needs_permission(
    c: &PlanningContext,
    s: &MealSelection,
) -> Result<bool, DomainError> {
    if s.repeat_authorised
        || s.slot.meal_type != MealType::Dinner
        || matches!(s.batch_use, BatchUse::Leftover { .. })
    {
        return Ok(false);
    }
    let target = monday(&s.slot.date)?;
    let relevant = |slot: &SlotId, recipe: &RecipeRef| -> Result<bool, DomainError> {
        Ok(slot != &s.slot
            && slot.meal_type == MealType::Dinner
            && recipe.recipe_id == s.recipe.recipe_id
            && (monday(&slot.date)? - target).num_days().abs() == 7)
    };
    for w in &c.weeks.0 {
        for other in active_selections(w) {
            if relevant(&other.slot, &other.recipe)? {
                return Ok(true);
            }
        }
    }
    for row in &c.history.selected.0 {
        if c.weeks
            .iter()
            .any(|w| w.cells.iter().any(|cell| cell.slot == row.slot))
        {
            continue;
        }
        if relevant(&row.slot, &row.recipe)? {
            return Ok(true);
        }
    }
    for row in &c.history.ordered.0 {
        if relevant(&row.selection.slot, &row.selection.recipe)? {
            return Ok(true);
        }
    }
    Ok(false)
}

fn repeat_conflict(s: &MealSelection) -> Conflict {
    Conflict {
        code: ErrorCode::AuthorityDenied,
        week_ids: BoundedVec(vec![s.slot.week_id.clone()]),
        slot_ids: BoundedVec(vec![s.slot.clone()]),
        recipe_refs: BoundedVec(vec![s.recipe.clone()]),
        evidence_ids: BoundedVec(vec![]),
        blocking: true,
    }
}

fn owners(
    before: &PlanningContext,
    after: &PlanningContext,
    changed: &[SlotId],
) -> Result<Vec<WeekId>, DomainError> {
    let mut slots = changed.to_vec();
    // Follow retained graph edges from both old and new states. Identities and links
    // remain unchanged here; C/D/E must rederive their actual feasibility later.
    loop {
        let previous = slots.len();
        for c in [before, after] {
            for link in c.weeks.iter().flat_map(|w| w.plan.batch_links.iter()) {
                if slots.contains(&link.source_slot) || slots.contains(&link.target_slot) {
                    for slot in [&link.source_slot, &link.target_slot] {
                        if !c
                            .weeks
                            .iter()
                            .any(|w| w.cells.iter().any(|cell| &cell.slot == slot))
                        {
                            return Err(error(ErrorCode::MissingReference, "/batch_links/owner"));
                        }
                        if !slots.contains(slot) {
                            slots.push(slot.clone());
                        }
                    }
                }
            }
            for s in c.weeks.iter().flat_map(|w| w.plan.selections.iter()) {
                if let BatchUse::Leftover { batch_id, .. } = &s.batch_use {
                    let source=c.weeks.iter().flat_map(|w|w.plan.selections.iter()).find(|p|matches!(&p.batch_use,BatchUse::NewBatch{batch_id:id,..} if id==batch_id));
                    if slots.contains(&s.slot) || source.is_some_and(|p| slots.contains(&p.slot)) {
                        // Search the other genuine state too when a source was just cleared.
                        let source=source.or_else(||[before,after].into_iter().flat_map(|c|c.weeks.iter().flat_map(|w|w.plan.selections.iter())).find(|p|matches!(&p.batch_use,BatchUse::NewBatch{batch_id:id,..} if id==batch_id))).ok_or_else(||error(ErrorCode::MissingReference,"/batch_use/source_owner"))?;
                        for slot in [&s.slot, &source.slot] {
                            if !slots.contains(slot) {
                                slots.push(slot.clone());
                            }
                        }
                    }
                }
            }
        }
        bound(slots.len(), 0, 84, "/rederive_slots")?;
        if slots.len() == previous {
            break;
        }
    }
    Ok(slots
        .into_iter()
        .map(|s| s.week_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect())
}
