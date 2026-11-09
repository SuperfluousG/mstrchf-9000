// TEST ONLY: explicit construction from independent synthetic literals; never application admission.
#![allow(dead_code)]
use mstrchf_contracts::*;
use mstrchf_core::{food::*, recipes::*};
use serde_json::Value;
pub trait Read {
    fn read(v: &Value) -> Self;
}
pub fn read<T: Read>(v: &Value) -> T {
    T::read(v)
}
impl<T: Read> Read for Vec<T> {
    fn read(v: &Value) -> Self {
        v.as_array().unwrap().iter().map(T::read).collect()
    }
}
impl<T: Read> Read for Option<T> {
    fn read(v: &Value) -> Self {
        if v.is_null() { None } else { Some(T::read(v)) }
    }
}
impl<A: Read, B: Read> Read for (A, B) {
    fn read(v: &Value) -> Self {
        (A::read(&v[0]), B::read(&v[1]))
    }
}
macro_rules! wire {($($t:ty),*)=>{$(impl Read for $t{fn read(v:&Value)->Self{serde_json::from_value(v.clone()).unwrap()}})*};}
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
macro_rules! record {($t:ident{$($f:ident),*})=>{impl Read for $t{fn read(v:&Value)->Self{Self{$($f:read(&v[stringify!($f)])),*}}}};}
macro_rules! units {($t:ident{$($v:ident:$s:literal),*})=>{impl Read for $t{fn read(v:&Value)->Self{match v["kind"].as_str().unwrap(){$($s=>Self::$v),*,v=>panic!("unknown test kind {v}")}}}};}
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
record!(EvidencePin {
    evidence_id,
    revision,
    digest
});
record!(EvidenceBasis {
    recipe,
    admitted,
    verified_asset_digests
});
record!(ComponentClassEvidence {
    recipe,
    component_id,
    class,
    evidence_ids
});
record!(ComponentClassView { recipe, components });
impl Read for UseKey {
    fn read(v: &Value) -> Self {
        let d = &v["data"];
        match v["kind"].as_str().unwrap() {
            "preparation" => Self::Preparation {
                phase: read(&d["phase"]),
            },
            "consumption" => Self::Consumption {
                occurrence: read(&d["occurrence"]),
                component_id: read(&d["component_id"]),
                person: read(&d["person"]),
            },
            x => panic!("{x}"),
        }
    }
}
impl Read for OpeningAnchor {
    fn read(v: &Value) -> Self {
        let d = &v["data"];
        match v["kind"].as_str().unwrap() {
            "unknown" => Self::Unknown,
            "observed" => Self::Observed {
                at: read(&d["at"]),
                evidence_id: read(&d["evidence_id"]),
            },
            "planned_phase_start" => Self::PlannedPhaseStart {
                phase: read(&d["phase"]),
            },
            x => panic!("{x}"),
        }
    }
}
impl Read for DeadlineRule {
    fn read(v: &Value) -> Self {
        let d = &v["data"];
        match v["kind"].as_str().unwrap() {
            "absolute" => Self::Absolute {
                latest_use: read(&d["latest_use"]),
                boundary: read(&d["boundary"]),
            },
            "after_opening" => Self::AfterOpening {
                anchor: read(&d["anchor"]),
                opening_boundary: read(&d["opening_boundary"]),
                maximum_elapsed_seconds: read(&d["maximum_elapsed_seconds"]),
                boundary: read(&d["boundary"]),
            },
            x => panic!("{x}"),
        }
    }
}
impl Read for DeliveryInput {
    fn read(v: &Value) -> Self {
        let d = &v["data"];
        match v["kind"].as_str().unwrap() {
            "unselected" => Self::Unselected,
            "proposed" => Self::Proposed {
                start: read(&d["start"]),
                end: read(&d["end"]),
                revision: read(&d["revision"]),
            },
            "confirmed" => Self::Confirmed {
                order: read(&d["order"]),
            },
            x => panic!("{x}"),
        }
    }
}
impl Read for LiveRecipeState {
    fn read(v: &Value) -> Self {
        let d = &v["data"];
        match v["kind"].as_str().unwrap() {
            "admitted" => Self::Admitted {
                basis: read(&d["basis"]),
                classes: read(&d["classes"]),
                methods: read(&d["methods"]),
                resources: read(&d["resources"]),
            },
            "withdrawn" => Self::Withdrawn {
                recipe: read(&d["recipe"]),
                evidence: read(&d["evidence"]),
            },
            x => panic!("{x}"),
        }
    }
}
pub fn value(id: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/food/native/v4")
        .join(format!("{id}.input.json"));
    serde_json::from_slice::<Value>(&std::fs::read(path).unwrap()).unwrap()["value"].clone()
}
pub fn basis(id: &str) -> SavedFoodBasis {
    read(&value(id))
}
pub fn derived(prefix: &str) -> DerivedPlan {
    let mut v = value(&format!("{prefix}-derived"));
    let manifest: Value = serde_json::from_slice(include_bytes!("v4/manifest-v4.json")).unwrap();
    v["digest"] = manifest["vectors"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["id"] == format!("{prefix}-derived"))
        .unwrap()["sha256"]
        .clone();
    read(&v)
}
pub fn historical() -> HistoricalApproval {
    HistoricalApproval {
        basis: basis("subject-saved-basis"),
        derived: derived("subject"),
        approved: read(&value("subject-approved")),
        order: read(&value("subject-order")),
    }
}
record!(PhaseResources {
    recipe,
    prep_id,
    equipment,
    evidence_ids
});
record!(ComponentReheatCount {
    recipe,
    component_id,
    planned_reheats,
    evidence_ids
});
record!(PortableColdMethod {
    packing_prep_id,
    service_prep_id
});
record!(MethodPhases {
    recipe,
    kind,
    prep_ids,
    direct_reuse_counts,
    evidence_ids,
    portable_cold
});
record!(FoodEvidence {
    catalogue_revision,
    catalogue_digest,
    policy_version,
    recipes,
    classes,
    methods,
    resources,
    essential_ingredient_ids
});
record!(OccurrenceKey {
    slot,
    extra_id,
    recipe
});
record!(PhaseKey {
    occurrence,
    prep_id
});
record!(SessionKey {
    anchor_slot,
    role,
    ordinal
});
record!(PhaseRoute { phase, session });
record!(PhaseStartConstraint { phase, start });
record!(AttachmentChoice {
    session,
    target_owner,
    target_session
});
record!(ScheduleLayout {
    routes,
    fixed_starts,
    attachments
});
record!(AllocationKey {
    batch_id,
    component_id,
    target_slot,
    person
});
record!(DemandKey {
    ingredient_id,
    unit,
    form
});
record!(FoodIdentities {
    revision,
    digest,
    allocations,
    sessions,
    demand_lines
});
record!(TaskBinding { phase, task });
record!(ScheduleInputs { layout, occupied });
record!(PlanOwnerPin {
    week,
    derived_digest,
    basis_digest
});
record!(OccupancyQuery {
    horizon,
    excluded_subject
});
record!(OccupiedTaskResources {
    task,
    equipment,
    evidence_ids
});
record!(OccupiedOwner {
    pin,
    sessions,
    task_resources,
    projection_digest
});
record!(OccupancySnapshot {
    query,
    occupancy_epoch,
    owners,
    digest
});
record!(DemandUse {
    line_id,
    use_key,
    at
});
record!(UseGraph {
    derived_digest,
    uses
});
record!(ProductUseScope {
    line_id,
    product,
    use_key
});
record!(DateFact {
    use_by,
    evidence_ids
});
record!(DeadlineFact { rule, evidence_ids });
record!(ProductUseBinding {
    scope,
    evidence_ids
});
record!(ProductUseFacts {
    scope,
    use_by,
    shorter
});
record!(ProductUseSnapshot {
    revision,
    digest,
    valid_until,
    evidence,
    bindings,
    facts
});
record!(FeasibilityInputs {
    revision,
    digest,
    delivery,
    products,
    confirmations,
    evaluated_at
});
record!(UseCheck {
    line_id,
    use_key,
    assessment,
    issues
});
record!(FoodSummary { kind, conflict });
record!(FeasibilityReport {
    derived_digest,
    inputs_digest,
    uses,
    other_checks,
    warnings,
    blocking,
    digest
});
record!(SavedFoodBasis {
    plan,
    context,
    evidence,
    identities,
    schedule
});
record!(HistoricalApproval {
    basis,
    derived,
    approved,
    order
});
record!(LiveRequirements {
    revision,
    digest,
    meals,
    unconfirmed
});
record!(RelevantEvidenceSnapshot {
    revision,
    digest,
    recipes
});
record!(LiveFoodPolicy {
    version,
    digest,
    general_seconds,
    rice_seconds,
    maximum_reheats
});
record!(LiveSourcePin {
    source,
    revision,
    digest,
    valid_until
});
record!(ContinuationInputs {
    revision,
    digest,
    sources,
    requirements,
    evidence,
    policy,
    occupied,
    feasibility
});
record!(ContinuationReport {
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
