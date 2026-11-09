//! Read-only manual list projection. Food quantities and pantry deductions stay
//! with the admitted core; this presentation seam performs no food arithmetic.
use crate::{BrowserState, catalogue::{CataloguePin, RuntimeCatalogue}};
use mstrchf_contracts::*;
use serde::Serialize;

#[derive(Serialize)]
pub struct ShoppingListProjection {
    pub format: &'static str,
    pub version: u8,
    pub snapshot_revision: Revision,
    pub week_id: WeekId,
    pub week_range: WeekRange,
    pub week_revision: Revision,
    pub catalogue_pin: CataloguePin,
    pub derived_digest: PayloadDigest,
    pub demand_digest: DemandDigest,
    pub evaluated_at: Instant,
    pub open_slots: Vec<SlotId>,
    pub contains_engineering: bool,
    pub retailer_status: &'static str,
    pub price_status: &'static str,
    pub lines: Vec<ShoppingListLine>,
}

#[derive(Serialize)]
pub struct ShoppingListLine {
    pub demand: DemandLine,
    pub name: NameText,
    pub action: &'static str,
    pub confirmation_status: &'static str,
    pub confirmation: Option<EssentialsConfirmation>,
    pub origins: Vec<ShoppingListOrigin>,
}

#[derive(Serialize)]
pub struct ShoppingListOrigin {
    pub recipe: RecipeRef,
    pub title: Title,
    pub admission: &'static str,
    pub batch_id: Option<BatchId>,
    pub extra_id: Option<ExtraId>,
    pub source_slot: SlotId,
    pub target_slots: Vec<SlotId>,
}

fn admission(view: &RuntimeCatalogue, recipe: &RecipeRef) -> &'static str {
    if view.is_public_recipe(recipe) {"public_reviewed"} else if view.is_personal_recipe(recipe) { "personal_reviewed" } else { "engineering" }
}

pub(crate) fn project(
    state: &BrowserState,
    view: &RuntimeCatalogue,
    week: &Week,
    derived: &DerivedPlan,
    now: &Instant,
) -> Result<ShoppingListProjection, DomainError> {
    let pantry = mstrchf_core::demand::project_essentials(&derived.demand, &state.essentials.0, now)?;
    let active = mstrchf_core::availability::eligible_slots(week)?;
    let open_slots = active.iter().filter(|slot| !week.plan.selections.iter().any(|meal| &meal.slot == *slot)).cloned().collect();
    let mut lines = Vec::new();
    let mut contains_engineering = false;
    for line in &derived.demand.lines.0 {
        let mut origins = Vec::new();
        for id in &line.origin_batches.0 {
            let batch = derived.batches.iter().find(|batch| &batch.batch_id == id)
                .ok_or_else(|| DomainError::new(ErrorCode::MissingReference, "/shopping_list/origin_batches"))?;
            let recipe = view.resolve_recipe(&batch.recipe)?;
            let mut target_slots: Vec<SlotId> = derived.allocations.iter().filter(|allocation| &allocation.batch_id == id)
                .map(|allocation| allocation.target_slot.clone()).collect();
            target_slots.sort_by(|a,b| (&a.week_id.0,&a.date.0,a.meal_type).cmp(&(&b.week_id.0,&b.date.0,b.meal_type))); target_slots.dedup();
            let supplies_input = |recipe: &RecipeRevision| recipe.ingredients.iter().any(|ingredient|
                ingredient.ingredient_id == line.ingredient_id && ingredient.quantity.0.as_ref()
                    .is_some_and(|quantity| quantity.unit == line.quantity.unit && quantity.form == line.quantity.form));
            if supplies_input(recipe) {
                origins.push(ShoppingListOrigin { recipe: batch.recipe.clone(), title: recipe.title.clone(),
                    admission: admission(view, &batch.recipe), batch_id: Some(id.clone()), extra_id: None,
                    source_slot: batch.source_slot.clone(), target_slots });
            }
            // A distinct production remix retains the original batch identity,
            // including for its local fresh inputs. Attribute those inputs to
            // the exact selected service, without repeating the original food.
            for selected in week.plan.selections.iter().filter(|selected|
                active.contains(&selected.slot) && selected.recipe != batch.recipe
                    && matches!(&selected.batch_use, BatchUse::Leftover { batch_id, .. } if batch_id == id)) {
                let recipe = view.resolve_recipe(&selected.recipe)?;
                if recipe.reused_inputs.is_empty() || !supplies_input(recipe) { continue; }
                if !derived.allocations.iter().any(|allocation| &allocation.batch_id == id
                    && allocation.target_slot == selected.slot
                    && allocation.remix_recipe.0.as_ref() == Some(&selected.recipe)) {
                    return Err(DomainError::new(ErrorCode::MissingReference, "/shopping_list/remix_origin"));
                }
                origins.push(ShoppingListOrigin {recipe:selected.recipe.clone(), title:recipe.title.clone(),
                    admission:admission(view,&selected.recipe), batch_id:Some(id.clone()), extra_id:None,
                    source_slot:selected.slot.clone(), target_slots:vec![selected.slot.clone()]});
            }
        }
        // ExtraId identifies a declared extra, not a unique meal occurrence.
        // Retain each matching active occurrence; never split its ingredient
        // quantity or attribute the whole aggregate to an arbitrary first meal.
        for selected in week.plan.selections.iter().filter(|selected| active.contains(&selected.slot)) {
            for extra in selected.extras.iter().filter(|extra| line.origin_extras.contains(&extra.extra_id)) {
                let recipe = view.resolve_recipe(&extra.recipe)?;
                if !recipe.ingredients.iter().any(|ingredient| ingredient.ingredient_id == line.ingredient_id
                    && ingredient.quantity.0.as_ref().is_some_and(|quantity| quantity.unit == line.quantity.unit && quantity.form == line.quantity.form)) { continue; }
                origins.push(ShoppingListOrigin {recipe:extra.recipe.clone(), title:recipe.title.clone(),
                    admission:admission(view,&extra.recipe), batch_id:None, extra_id:Some(extra.extra_id.clone()),
                    source_slot:selected.slot.clone(), target_slots:vec![selected.slot.clone()]});
            }
        }
        if origins.is_empty() { return Err(DomainError::new(ErrorCode::MissingReference, "/shopping_list/origins")); }
        contains_engineering |= origins.iter().any(|origin| origin.admission == "engineering");
        let name = origins.iter().find_map(|origin| view.resolve_recipe(&origin.recipe).ok()
            .and_then(|recipe| recipe.ingredients.iter().find(|ingredient| ingredient.ingredient_id == line.ingredient_id))
            .map(|ingredient| ingredient.name.clone()))
            .ok_or_else(|| DomainError::new(ErrorCode::MissingReference, "/shopping_list/ingredient"))?;
        let confirmation = state.essentials.iter().find(|confirmation| confirmation.line_id == line.line_id).cloned();
        let status = if !line.essential { "not_offered" }
            else if pantry.renewal_line_ids.contains(&line.line_id) { "stale" }
            else if confirmation.is_some() { "current" } else { "none" };
        lines.push(ShoppingListLine { demand:line.clone(), name,
            action:if pantry.covered_line_ids.contains(&line.line_id) { "have_enough" } else { "add_to_shop" },
            confirmation_status:status, confirmation, origins });
    }
    Ok(ShoppingListProjection {format:"mstrchf-manual-shopping-list", version:1,
        snapshot_revision:state.snapshot.snapshot_revision, week_id:week.range.week_id.clone(), week_range:week.range.clone(), week_revision:week.revision,
        catalogue_pin:view.pin().clone(), derived_digest:derived.digest.clone(), demand_digest:derived.demand.digest.clone(),
        evaluated_at:now.clone(), open_slots, contains_engineering, retailer_status:"unqualified", price_status:"unavailable", lines})
}
