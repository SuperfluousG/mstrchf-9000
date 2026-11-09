//! Current native matching inputs only. This projection creates no source,
//! meaning, suitability, basket approval or external-effect authority.
use crate::{BrowserState, catalogue::CataloguePin, planner_view, shopping_list};
use mstrchf_contracts::*;
use mstrchf_core::{food::{OccurrenceKey, UseKey, occurrence_cmp, saved_basis_digest},
    products::{MatchPolicy, policy_digest, prepare_match_context}};
use serde::Serialize;

// Serialize-only presentation types. Future matching must reconstruct its
// native inputs from the owner, never deserialize these projected facts.
#[derive(Serialize)]
pub(crate) struct ProductMatchContext {
    format: &'static str,
    version: u8,
    snapshot_revision: Revision,
    week_id: WeekId,
    week_revision: Revision,
    plan_revision: Revision,
    context: ContextStamp,
    catalogue_pin: CataloguePin,
    saved_basis_digest: PayloadDigest,
    derived_digest: PayloadDigest,
    demand_digest: DemandDigest,
    evaluated_at: Instant,
    open_slots: Vec<SlotId>,
    contains_engineering: bool,
    match_status: &'static str,
    retailer_status: &'static str,
    effect_enabled: bool,
    lines: Vec<LineContext>,
}

#[derive(Serialize)]
struct LineContext {
    demand: DemandLine,
    shopping_action: &'static str,
    uses: Vec<UseContext>,
    cooking_occurrences: Vec<Occurrence>,
    origins: Vec<Origin>,
}
#[derive(Serialize)]
struct Occurrence {
    slot: SlotId,
    extra_id: Option<ExtraId>,
    recipe: RecipeRef,
}
impl From<&OccurrenceKey> for Occurrence {
    fn from(value: &OccurrenceKey) -> Self {
        Self {slot: value.slot.clone(), extra_id: value.extra_id.clone(), recipe: value.recipe.clone()}
    }
}
#[derive(Serialize)]
struct Phase {
    occurrence: Occurrence,
    prep_id: u16,
}
#[derive(Serialize)]
#[serde(tag = "kind", content = "data", rename_all = "snake_case")]
enum UseIdentity {
    Preparation {phase: Phase},
    Consumption {occurrence: Occurrence, component_id: ComponentId, person: PersonRef},
}
#[derive(Serialize)]
struct UseContext {
    use_key: UseIdentity,
    at: ZonedMoment,
    requirements: Vec<HardRequirement>,
}
#[derive(Serialize)]
struct Origin {
    occurrence: Occurrence,
    batch_id: BatchId,
    component_id: ComponentId,
    ingredient_id: IngredientId,
    original_quantity: Quantity,
}
fn error(code: ErrorCode, path: &str) -> DomainError { DomainError::new(code, path) }

pub(crate) fn project(state: &BrowserState, week_id: &WeekId, at: &Instant) -> Result<ProductMatchContext, DomainError> {
    let view = crate::runtime_catalogue(state)?;
    let week = state.snapshot.weeks.iter().find(|week| &week.range.week_id == week_id)
        .ok_or_else(|| error(ErrorCode::MissingReference, "/products/preflight/week"))?;
    let profile = state.snapshot.profile.0.as_ref()
        .ok_or_else(|| error(ErrorCode::MissingField, "/products/preflight/profile"))?;
    let (basis, food) = planner_view::derive_material(&view, &state.snapshot, profile, week)?
        .ok_or_else(|| error(ErrorCode::MissingReference, "/products/preflight/food"))?;
    // An empty policy calls the existing strict context reconstruction without
    // inventing dietary rule reviews. It admits no meaning and is not exported
    // as a policy for any later product assessment.
    let mut validation_policy = MatchPolicy {version: Revision(1), digest: PayloadDigest("0".repeat(64)), rules: vec![]};
    validation_policy.digest = policy_digest(&validation_policy)?;
    let prepared = prepare_match_context(&basis.context, &basis, &food.plan, &validation_policy, at.clone())?;
    let list = shopping_list::project(state, &view, week, &food.plan, at)?;
    let mut lines = Vec::new();
    for actual in food.plan.demand.lines.iter() {
        let (demand, scoped, supplied) = prepared.line_context(&actual.line_id)?;
        let action = list.lines.iter().find(|line| line.demand.line_id == demand.line_id)
            .ok_or_else(|| error(ErrorCode::MissingReference, "/products/preflight/shopping_line"))?.action;
        let mut cooking: Vec<OccurrenceKey> = supplied.iter().map(|origin| origin.occurrence.clone()).collect();
        let mut uses = Vec::new();
        for (native, requirements) in scoped {
            let (occurrence, identity) = match &native.use_key {
                UseKey::Preparation {phase} => (&phase.occurrence, UseIdentity::Preparation {phase: Phase {
                    occurrence: (&phase.occurrence).into(), prep_id: phase.prep_id}}),
                UseKey::Consumption {occurrence, component_id, person} => (occurrence, UseIdentity::Consumption {
                    occurrence: occurrence.into(), component_id: component_id.clone(), person: person.clone()}),
            };
            cooking.push(occurrence.clone());
            uses.push(UseContext {use_key: identity, at: native.at, requirements});
        }
        cooking.sort_by(occurrence_cmp); cooking.dedup();
        lines.push(LineContext {demand, shopping_action: action, uses,
            cooking_occurrences: cooking.iter().map(Occurrence::from).collect(),
            origins: supplied.into_iter().map(|origin| Origin {occurrence: (&origin.occurrence).into(),
                batch_id: origin.batch_id, component_id: origin.component_id, ingredient_id: origin.ingredient_id,
                original_quantity: origin.original_quantity}).collect()});
    }
    Ok(ProductMatchContext {format: "mstrchf-product-match-context", version: 1,
        snapshot_revision: state.snapshot.snapshot_revision, week_id: week_id.clone(), week_revision: week.revision,
        plan_revision: week.plan.revision, context: basis.context.stamp.clone(), catalogue_pin: view.pin().clone(),
        saved_basis_digest: saved_basis_digest(&basis)?, derived_digest: food.plan.digest.clone(),
        demand_digest: food.plan.demand.digest.clone(), evaluated_at: at.clone(), open_slots: list.open_slots,
        contains_engineering: list.contains_engineering, match_status: "not_assessed", retailer_status: "unqualified",
        effect_enabled: false, lines})
}
