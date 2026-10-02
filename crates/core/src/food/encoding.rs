//! Explicit projections: no native input implements Deserialize or accepts opaque JSON authority.
use super::*;
use serde_json::{Value, json};
use std::cmp::Ordering;
pub(crate) trait NativeValue {
    fn native(&self) -> Value;
}
impl<T: NativeValue> NativeValue for Vec<T> {
    fn native(&self) -> Value {
        Value::Array(self.iter().map(NativeValue::native).collect())
    }
}
impl<T: NativeValue> NativeValue for Option<T> {
    fn native(&self) -> Value {
        self.as_ref()
            .map(NativeValue::native)
            .unwrap_or(Value::Null)
    }
}
impl<A: NativeValue, B: NativeValue> NativeValue for (A, B) {
    fn native(&self) -> Value {
        json!([self.0.native(), self.1.native()])
    }
}
macro_rules! wire { ($($t:ty),*) => { $(impl NativeValue for $t { fn native(&self)->Value {json!(self)} })* }; }
wire!(
    u8,
    u16,
    u32,
    Revision,
    RecipeRef,
    EvidenceId,
    Digest,
    AssetDigest,
    PayloadDigest,
    IngredientId,
    ComponentId,
    SlotId,
    ExtraId,
    BatchId,
    PersonRef,
    AllocationId,
    PrepSessionId,
    DemandLineId,
    EquipmentCode,
    Unit,
    IngredientForm,
    PrepTaskRef,
    WeekRevision,
    TimeInterval,
    PrepSession,
    ZonedMoment,
    LocalDate,
    Instant,
    ProductRef,
    ConfirmedOrder,
    EssentialsConfirmation,
    Conflict,
    WeekPlan,
    PlanningContext,
    DerivedPlan,
    ApprovedPlan,
    MealRequirements,
    OrderId,
    PlanInputDigest
);
macro_rules! projection { ($t:ty {$($f:ident),*}) => {impl NativeValue for $t { fn native(&self)->Value {json!({$(stringify!($f):self.$f.native()),*})} }}; }
macro_rules! units { ($t:ident {$($v:ident:$s:literal),*}) => { impl NativeValue for $t {fn native(&self)->Value {json!({"kind":match self {$(Self::$v=>$s),*}})}} }; }
units!(ComponentClass {
    General: "general",
    Rice: "rice",
    Unknown: "unknown"
});
units!(MethodKind {
    Production: "production",
    Reuse: "reuse"
});
units!(SessionRole {
    Cooking: "cooking",
    AdvancePrep: "advance_prep"
});
units!(ProductDateAssessment {
    WithinKnownLimit: "within_known_limit",
    UnresolvedDateEvidence: "unresolved_date_evidence",
    ConflictingKnownLimit: "conflicting_known_limit"
});
units!(DeadlineBoundary {
    Inclusive: "inclusive",
    Exclusive: "exclusive"
});
units!(UseIssue {
    UnknownProductDate: "unknown_product_date",
    UnknownOpeningAnchor: "unknown_opening_anchor",
    OpeningChronologyConflict: "opening_chronology_conflict",
    UseByConflict: "use_by_conflict",
    ShorterDeadlineConflict: "shorter_deadline_conflict"
});
units!(FoodSummaryKind {
    UnknownUseEvidence: "unknown_use_evidence",
    OtherPendingChecks: "other_pending_checks",
    BlockingChecks: "blocking_checks"
});
units!(LiveSource {
    Requirements: "requirements",
    RecipeEvidence: "recipe_evidence",
    FoodPolicy: "food_policy",
    Occupancy: "occupancy",
    ProductEvidence: "product_evidence",
    Essentials: "essentials",
    Order: "order"
});
projection!(EvidencePin {
    evidence_id,
    revision,
    digest
});
projection!(EvidenceBasis {
    recipe,
    admitted,
    verified_asset_digests
});
projection!(ComponentClassEvidence {
    recipe,
    component_id,
    class,
    evidence_ids
});
projection!(ComponentClassView { recipe, components });
impl NativeValue for UseKey {
    fn native(&self) -> Value {
        match self {
            Self::Preparation { phase } => {
                json!({"kind":"preparation","data":{"phase":phase.native()}})
            }
            Self::Consumption {
                occurrence,
                component_id,
                person,
            } => {
                json!({"kind":"consumption","data":{"occurrence":occurrence.native(),"component_id":component_id,"person":person}})
            }
        }
    }
}
impl NativeValue for OpeningAnchor {
    fn native(&self) -> Value {
        match self {
            Self::Unknown => json!({"kind":"unknown"}),
            Self::Observed { at, evidence_id } => {
                json!({"kind":"observed","data":{"at":at,"evidence_id":evidence_id}})
            }
            Self::PlannedPhaseStart { phase } => {
                json!({"kind":"planned_phase_start","data":{"phase":phase.native()}})
            }
        }
    }
}
impl NativeValue for DeadlineRule {
    fn native(&self) -> Value {
        match self {
            Self::Absolute {
                latest_use,
                boundary,
            } => {
                json!({"kind":"absolute","data":{"latest_use":latest_use,"boundary":boundary.native()}})
            }
            Self::AfterOpening {
                anchor,
                opening_boundary,
                maximum_elapsed_seconds,
                boundary,
            } => {
                json!({"kind":"after_opening","data":{"anchor":anchor.native(),"opening_boundary":opening_boundary.native(),"maximum_elapsed_seconds":maximum_elapsed_seconds,"boundary":boundary.native()}})
            }
        }
    }
}
impl NativeValue for DeliveryInput {
    fn native(&self) -> Value {
        match self {
            Self::Unselected => json!({"kind":"unselected"}),
            Self::Proposed {
                start,
                end,
                revision,
            } => json!({"kind":"proposed","data":{"start":start,"end":end,"revision":revision}}),
            Self::Confirmed { order } => json!({"kind":"confirmed","data":{"order":order}}),
        }
    }
}
impl NativeValue for LiveRecipeState {
    fn native(&self) -> Value {
        match self {
            Self::Admitted {
                basis,
                classes,
                methods,
                resources,
            } => {
                json!({"kind":"admitted","data":{"basis":basis.native(),"classes":classes.native(),"methods":methods.native(),"resources":resources.native()}})
            }
            Self::Withdrawn { recipe, evidence } => {
                json!({"kind":"withdrawn","data":{"recipe":recipe,"evidence":evidence.native()}})
            }
        }
    }
}

pub(crate) fn hash_native(domain: &str, material: Value) -> Result<PayloadDigest, DomainError> {
    Ok(PayloadDigest(
        canonical_digest(&json!({"domain":domain,"material":material}))?.0,
    ))
}
fn omitted<T: NativeValue>(v: &T, field: &str) -> Value {
    let mut material = v.native();
    material
        .as_object_mut()
        .expect("native record")
        .remove(field);
    material
}
pub fn slot_cmp(a: &SlotId, b: &SlotId) -> Ordering {
    fn rank(v: MealType) -> u8 {
        match v {
            MealType::Breakfast => 0,
            MealType::Lunch => 1,
            MealType::Dinner => 2,
        }
    }
    (&a.week_id, &a.date, rank(a.meal_type)).cmp(&(&b.week_id, &b.date, rank(b.meal_type)))
}
pub fn recipe_cmp(a: &RecipeRef, b: &RecipeRef) -> Ordering {
    (&a.recipe_id, a.revision, &a.digest).cmp(&(&b.recipe_id, b.revision, &b.digest))
}
pub fn person_cmp(a: &PersonRef, b: &PersonRef) -> Ordering {
    match (a, b) {
        (PersonRef::Diner { diner_id: a }, PersonRef::Diner { diner_id: b }) => a.cmp(b),
        (
            PersonRef::Guest {
                occasion_id: a,
                guest_id: x,
            },
            PersonRef::Guest {
                occasion_id: b,
                guest_id: y,
            },
        ) => (a, x).cmp(&(b, y)),
        (PersonRef::Diner { .. }, _) => Ordering::Less,
        _ => Ordering::Greater,
    }
}
pub fn occurrence_cmp(a: &OccurrenceKey, b: &OccurrenceKey) -> Ordering {
    slot_cmp(&a.slot, &b.slot)
        .then_with(|| a.extra_id.cmp(&b.extra_id))
        .then_with(|| recipe_cmp(&a.recipe, &b.recipe))
}
pub fn phase_cmp(a: &PhaseKey, b: &PhaseKey) -> Ordering {
    occurrence_cmp(&a.occurrence, &b.occurrence).then(a.prep_id.cmp(&b.prep_id))
}
pub fn session_cmp(a: &SessionKey, b: &SessionKey) -> Ordering {
    slot_cmp(&a.anchor_slot, &b.anchor_slot)
        .then_with(|| {
            a.role.native()["kind"]
                .as_str()
                .cmp(&b.role.native()["kind"].as_str())
        })
        .then(a.ordinal.cmp(&b.ordinal))
}
pub fn allocation_cmp(a: &AllocationKey, b: &AllocationKey) -> Ordering {
    (&a.batch_id, &a.component_id)
        .cmp(&(&b.batch_id, &b.component_id))
        .then_with(|| slot_cmp(&a.target_slot, &b.target_slot))
        .then_with(|| person_cmp(&a.person, &b.person))
}
pub fn demand_cmp(a: &DemandKey, b: &DemandKey) -> Ordering {
    a.ingredient_id
        .cmp(&b.ingredient_id)
        .then_with(|| wire_cmp(&a.unit, &b.unit))
        .then_with(|| wire_cmp(&a.form, &b.form))
}
fn wire_cmp<T: NativeValue>(a: &T, b: &T) -> Ordering {
    a.native().as_str().cmp(&b.native().as_str())
}
pub fn task_cmp(a: &PrepTaskRef, b: &PrepTaskRef) -> Ordering {
    (&a.session_id, a.task_id).cmp(&(&b.session_id, b.task_id))
}
pub fn owner_cmp(a: &PlanOwnerPin, b: &PlanOwnerPin) -> Ordering {
    (
        &a.week.week_id,
        a.week.revision,
        &a.derived_digest,
        &a.basis_digest,
    )
        .cmp(&(
            &b.week.week_id,
            b.week.revision,
            &b.derived_digest,
            &b.basis_digest,
        ))
}
pub fn product_cmp(a: &ProductRef, b: &ProductRef) -> Ordering {
    json!(a.retailer)
        .as_str()
        .cmp(&json!(b.retailer).as_str())
        .then_with(|| a.product_id.cmp(&b.product_id))
        .then_with(|| {
            a.variant_id
                .0
                .as_ref()
                .map(|x| &x.0)
                .cmp(&b.variant_id.0.as_ref().map(|x| &x.0))
        })
}
pub fn use_cmp(a: &UseKey, b: &UseKey) -> Ordering {
    match (a, b) {
        (UseKey::Preparation { phase: a }, UseKey::Preparation { phase: b }) => phase_cmp(a, b),
        (
            UseKey::Consumption {
                occurrence: a,
                component_id: x,
                person: p,
            },
            UseKey::Consumption {
                occurrence: b,
                component_id: y,
                person: q,
            },
        ) => occurrence_cmp(a, b)
            .then(x.cmp(y))
            .then_with(|| person_cmp(p, q)),
        (UseKey::Consumption { .. }, _) => Ordering::Less,
        _ => Ordering::Greater,
    }
}
pub fn scope_cmp(a: &ProductUseScope, b: &ProductUseScope) -> Ordering {
    a.line_id
        .cmp(&b.line_id)
        .then_with(|| product_cmp(&a.product, &b.product))
        .then_with(|| use_cmp(&a.use_key, &b.use_key))
}
pub(crate) fn fact_bytes(v: &DeadlineFact) -> Result<Vec<u8>, DomainError> {
    canonical_bytes(&v.native())
}

pub fn food_evidence_digest(v: &FoodEvidence) -> Result<PayloadDigest, DomainError> {
    validate_evidence_shape(v)?;
    hash_native("mstrchf-r3-food_evidence", omitted(v, "catalogue_digest"))
}
pub fn food_identities_digest(v: &FoodIdentities) -> Result<PayloadDigest, DomainError> {
    validate_identities_shape(v)?;
    hash_native("mstrchf-r3-food_identities", omitted(v, "digest"))
}
pub fn occupied_owner_digest(v: &OccupiedOwner) -> Result<PayloadDigest, DomainError> {
    validate_owner_shape(v)?;
    hash_native("mstrchf-r3-occupied_owner", omitted(v, "projection_digest"))
}
pub fn occupancy_snapshot_digest(v: &OccupancySnapshot) -> Result<PayloadDigest, DomainError> {
    validate_occupancy_shape(v)?;
    hash_native("mstrchf-r3-occupancy_snapshot", omitted(v, "digest"))
}
pub fn digest_product_uses(v: &ProductUseSnapshot) -> Result<PayloadDigest, DomainError> {
    validate_products_shape(v)?;
    hash_native("mstrchf-product-uses-r3", omitted(v, "digest"))
}
pub fn digest_feasibility_inputs(v: &FeasibilityInputs) -> Result<PayloadDigest, DomainError> {
    validate_feasibility_shape(v)?;
    hash_native("mstrchf-feasibility-inputs-r3", omitted(v, "digest"))
}
pub fn digest_feasibility_report(v: &FeasibilityReport) -> Result<PayloadDigest, DomainError> {
    validate_report_shape(v)?;
    hash_native("mstrchf-feasibility-report-r3", omitted(v, "digest"))
}
pub fn saved_basis_digest(v: &SavedFoodBasis) -> Result<PayloadDigest, DomainError> {
    validate_saved_shape(v)?;
    hash_native("mstrchf-saved-food-basis-r3", v.native())
}
pub fn food_inputs_digest(
    input: &PlanInputDigest,
    evidence: &FoodEvidence,
    identities: &FoodIdentities,
    schedule: &ScheduleInputs,
) -> Result<PayloadDigest, DomainError> {
    shape(input)?;
    require(
        food_evidence_digest(evidence)? == evidence.catalogue_digest,
        ErrorCode::DigestMismatch,
        "/evidence/catalogue_digest",
    )?;
    require(
        food_identities_digest(identities)? == identities.digest,
        ErrorCode::DigestMismatch,
        "/identities/digest",
    )?;
    validate_schedule_shape(schedule)?;
    hash_native(
        "mstrchf-food-inputs-r3",
        json!({"input_plan_digest":input,"evidence":evidence.native(),"identities":identities.native(),"schedule":schedule.native()}),
    )
}
pub fn live_requirements_digest(v: &LiveRequirements) -> Result<PayloadDigest, DomainError> {
    validate_live_requirements(v)?;
    hash_native("mstrchf-r3-live_requirements", omitted(v, "digest"))
}
pub fn relevant_evidence_digest(
    v: &RelevantEvidenceSnapshot,
) -> Result<PayloadDigest, DomainError> {
    validate_live_evidence(v)?;
    hash_native(
        "mstrchf-r3-relevant_evidence_snapshot",
        omitted(v, "digest"),
    )
}
pub fn live_food_policy_digest(v: &LiveFoodPolicy) -> Result<PayloadDigest, DomainError> {
    validate_policy(v)?;
    hash_native("mstrchf-r3-live_food_policy", omitted(v, "digest"))
}
pub fn continuation_inputs_digest(v: &ContinuationInputs) -> Result<PayloadDigest, DomainError> {
    validate_continuation_shape(v)?;
    hash_native("mstrchf-current-food-inputs-r3", omitted(v, "digest"))
}
pub fn continuation_report_digest(v: &ContinuationReport) -> Result<PayloadDigest, DomainError> {
    validate_continuation_report(v)?;
    hash_native("mstrchf-food-continuation-r3", omitted(v, "digest"))
}
pub fn essentials_source_digest(
    pin: &LiveSourcePin,
    values: &[EssentialsConfirmation],
) -> Result<PayloadDigest, DomainError> {
    require(
        pin.source == LiveSource::Essentials,
        ErrorCode::InvalidValue,
        "/source",
    )?;
    shape(&pin.revision)?;
    if let Some(t) = &pin.valid_until {
        shape(t)?;
    }
    limit(values.len(), 4096, "/values")?;
    ordered(values, |a, b| a.line_id.cmp(&b.line_id), "/values")?;
    for v in values {
        shape(v)?;
    }
    hash_native(
        "mstrchf-r3-live_source",
        json!({"source":pin.source.native(),"revision":pin.revision,"valid_until":pin.valid_until.native(),"values":values}),
    )
}
pub fn order_source_digest(
    pin: &LiveSourcePin,
    order: &ConfirmedOrder,
) -> Result<PayloadDigest, DomainError> {
    require(
        pin.source == LiveSource::Order,
        ErrorCode::InvalidValue,
        "/source",
    )?;
    shape(&pin.revision)?;
    if let Some(t) = &pin.valid_until {
        shape(t)?;
    }
    shape(order)?;
    hash_native(
        "mstrchf-r3-live_source",
        json!({"source":pin.source.native(),"revision":pin.revision,"valid_until":pin.valid_until.native(),"values":order}),
    )
}
projection!(PhaseResources {
    recipe,
    prep_id,
    equipment,
    evidence_ids
});
projection!(ComponentReheatCount {
    recipe,
    component_id,
    planned_reheats,
    evidence_ids
});
projection!(MethodPhases {
    recipe,
    kind,
    prep_ids,
    direct_reuse_counts,
    evidence_ids
});
projection!(FoodEvidence {
    catalogue_revision,
    catalogue_digest,
    policy_version,
    recipes,
    classes,
    methods,
    resources,
    essential_ingredient_ids
});
projection!(OccurrenceKey {
    slot,
    extra_id,
    recipe
});
projection!(PhaseKey {
    occurrence,
    prep_id
});
projection!(SessionKey {
    anchor_slot,
    role,
    ordinal
});
projection!(PhaseRoute { phase, session });
projection!(PhaseStartConstraint { phase, start });
projection!(AttachmentChoice {
    session,
    target_owner,
    target_session
});
projection!(ScheduleLayout {
    routes,
    fixed_starts,
    attachments
});
projection!(AllocationKey {
    batch_id,
    component_id,
    target_slot,
    person
});
projection!(DemandKey {
    ingredient_id,
    unit,
    form
});
projection!(FoodIdentities {
    revision,
    digest,
    allocations,
    sessions,
    demand_lines
});
projection!(TaskBinding { phase, task });
projection!(ScheduleInputs { layout, occupied });
projection!(PlanOwnerPin {
    week,
    derived_digest,
    basis_digest
});
projection!(OccupancyQuery {
    horizon,
    excluded_subject
});
projection!(OccupiedTaskResources {
    task,
    equipment,
    evidence_ids
});
projection!(OccupiedOwner {
    pin,
    sessions,
    task_resources,
    projection_digest
});
projection!(OccupancySnapshot {
    query,
    occupancy_epoch,
    owners,
    digest
});
projection!(DemandUse {
    line_id,
    use_key,
    at
});
projection!(UseGraph {
    derived_digest,
    uses
});
projection!(ProductUseScope {
    line_id,
    product,
    use_key
});
projection!(DateFact {
    use_by,
    evidence_ids
});
projection!(DeadlineFact { rule, evidence_ids });
projection!(ProductUseBinding {
    scope,
    evidence_ids
});
projection!(ProductUseFacts {
    scope,
    use_by,
    shorter
});
projection!(ProductUseSnapshot {
    revision,
    digest,
    valid_until,
    evidence,
    bindings,
    facts
});
projection!(FeasibilityInputs {
    revision,
    digest,
    delivery,
    products,
    confirmations,
    evaluated_at
});
projection!(UseCheck {
    line_id,
    use_key,
    assessment,
    issues
});
projection!(FoodSummary { kind, conflict });
projection!(FeasibilityReport {
    derived_digest,
    inputs_digest,
    uses,
    other_checks,
    warnings,
    blocking,
    digest
});
projection!(SavedFoodBasis {
    plan,
    context,
    evidence,
    identities,
    schedule
});
projection!(HistoricalApproval {
    basis,
    derived,
    approved,
    order
});
projection!(LiveRequirements {
    revision,
    digest,
    meals,
    unconfirmed
});
projection!(RelevantEvidenceSnapshot {
    revision,
    digest,
    recipes
});
projection!(LiveFoodPolicy {
    version,
    digest,
    general_seconds,
    rice_seconds,
    maximum_reheats
});
projection!(LiveSourcePin {
    source,
    revision,
    digest,
    valid_until
});
projection!(ContinuationInputs {
    revision,
    digest,
    sources,
    requirements,
    evidence,
    policy,
    occupied,
    feasibility
});
projection!(ContinuationReport {
    order_id,
    approved_digest,
    historical_basis_digest,
    current_inputs_digest,
    feasibility,
    other_checks,
    warnings,
    blocking,
    digest
});
