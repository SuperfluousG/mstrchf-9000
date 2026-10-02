use super::*;
macro_rules! native_record { ($name:ident { $($field:ident : $ty:ty),* $(,)? }) => {
    #[derive(Clone, Debug, PartialEq, Eq)] pub struct $name { $(pub $field: $ty),* }
}; }
macro_rules! native_units { ($name:ident { $($variant:ident),* }) => { #[derive(Clone,Copy,Debug,PartialEq,Eq)] pub enum $name { $($variant),* } }; }
native_units!(MethodKind { Production, Reuse });
native_units!(SessionRole {
    Cooking,
    AdvancePrep
});
native_units!(ProductDateAssessment {
    WithinKnownLimit,
    UnresolvedDateEvidence,
    ConflictingKnownLimit
});
native_units!(DeadlineBoundary {
    Inclusive,
    Exclusive
});
native_units!(UseIssue {
    UnknownProductDate,
    UnknownOpeningAnchor,
    OpeningChronologyConflict,
    UseByConflict,
    ShorterDeadlineConflict
});
native_units!(FoodSummaryKind {
    UnknownUseEvidence,
    OtherPendingChecks,
    BlockingChecks
});
native_units!(LiveSource {
    Requirements,
    RecipeEvidence,
    FoodPolicy,
    Occupancy,
    ProductEvidence,
    Essentials,
    Order
});
native_record!(PhaseResources {recipe:RecipeRef,prep_id:u16,equipment:Vec<EquipmentCode>,evidence_ids:Vec<EvidenceId>});
native_record!(ComponentReheatCount {recipe:RecipeRef,component_id:ComponentId,planned_reheats:u8,evidence_ids:Vec<EvidenceId>});
native_record!(MethodPhases {recipe:RecipeRef,kind:MethodKind,prep_ids:Vec<u16>,direct_reuse_counts:Vec<ComponentReheatCount>,evidence_ids:Vec<EvidenceId>});
native_record!(FoodEvidence {catalogue_revision:Revision,catalogue_digest:PayloadDigest,policy_version:Revision,recipes:Vec<EvidenceBasis>,classes:Vec<ComponentClassView>,methods:Vec<MethodPhases>,resources:Vec<PhaseResources>,essential_ingredient_ids:Vec<IngredientId>});
native_record!(OccurrenceKey {slot:SlotId,extra_id:Option<ExtraId>,recipe:RecipeRef});
native_record!(PhaseKey {
    occurrence: OccurrenceKey,
    prep_id: u16
});
native_record!(SessionKey {
    anchor_slot: SlotId,
    role: SessionRole,
    ordinal: u16
});
native_record!(PhaseRoute {
    phase: PhaseKey,
    session: SessionKey
});
native_record!(PhaseStartConstraint {
    phase: PhaseKey,
    start: ZonedMoment
});
native_record!(AttachmentChoice {
    session: SessionKey,
    target_owner: PlanOwnerPin,
    target_session: PrepSessionId
});
native_record!(ScheduleLayout {routes:Vec<PhaseRoute>,fixed_starts:Vec<PhaseStartConstraint>,attachments:Vec<AttachmentChoice>});
native_record!(AllocationKey {
    batch_id: BatchId,
    component_id: ComponentId,
    target_slot: SlotId,
    person: PersonRef
});
native_record!(DemandKey {
    ingredient_id: IngredientId,
    unit: Unit,
    form: IngredientForm
});
native_record!(FoodIdentities {revision:Revision,digest:PayloadDigest,allocations:Vec<(AllocationKey,AllocationId)>,sessions:Vec<(SessionKey,PrepSessionId)>,demand_lines:Vec<(DemandKey,DemandLineId)>});
native_record!(TaskBinding {
    phase: PhaseKey,
    task: PrepTaskRef
});
native_record!(ScheduleInputs {
    layout: ScheduleLayout,
    occupied: OccupancySnapshot
});
native_record!(PlanOwnerPin {
    week: WeekRevision,
    derived_digest: PayloadDigest,
    basis_digest: PayloadDigest
});
native_record!(OccupancyQuery {horizon:TimeInterval,excluded_subject:Option<PlanOwnerPin>});
native_record!(OccupiedTaskResources {task:PrepTaskRef,equipment:Vec<EquipmentCode>,evidence_ids:Vec<EvidenceId>});
native_record!(OccupiedOwner {pin:PlanOwnerPin,sessions:Vec<PrepSession>,task_resources:Vec<OccupiedTaskResources>,projection_digest:PayloadDigest});
native_record!(OccupancySnapshot {query:Option<OccupancyQuery>,occupancy_epoch:Revision,owners:Vec<OccupiedOwner>,digest:PayloadDigest});
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum UseKey {
    Preparation {
        phase: PhaseKey,
    },
    Consumption {
        occurrence: OccurrenceKey,
        component_id: ComponentId,
        person: PersonRef,
    },
}
native_record!(DemandUse {
    line_id: DemandLineId,
    use_key: UseKey,
    at: ZonedMoment
});
native_record!(UseGraph {derived_digest:PayloadDigest,uses:Vec<DemandUse>});
native_record!(ProductUseScope {
    line_id: DemandLineId,
    product: ProductRef,
    use_key: UseKey
});
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OpeningAnchor {
    Observed {
        at: ZonedMoment,
        evidence_id: EvidenceId,
    },
    PlannedPhaseStart {
        phase: PhaseKey,
    },
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DeadlineRule {
    Absolute {
        latest_use: ZonedMoment,
        boundary: DeadlineBoundary,
    },
    AfterOpening {
        anchor: OpeningAnchor,
        opening_boundary: DeadlineBoundary,
        maximum_elapsed_seconds: u32,
        boundary: DeadlineBoundary,
    },
}
native_record!(DateFact {use_by:LocalDate,evidence_ids:Vec<EvidenceId>});
native_record!(DeadlineFact {rule:DeadlineRule,evidence_ids:Vec<EvidenceId>});
native_record!(ProductUseBinding {scope:ProductUseScope,evidence_ids:Vec<EvidenceId>});
native_record!(ProductUseFacts {scope:ProductUseScope,use_by:Option<DateFact>,shorter:Vec<DeadlineFact>});
native_record!(ProductUseSnapshot {revision:Revision,digest:PayloadDigest,valid_until:Option<Instant>,evidence:Vec<EvidencePin>,bindings:Vec<ProductUseBinding>,facts:Vec<ProductUseFacts>});
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)] // Exact adopted native value contains the confirmed order.
pub enum DeliveryInput {
    Unselected,
    Proposed {
        start: ZonedMoment,
        end: ZonedMoment,
        revision: Revision,
    },
    Confirmed {
        order: ConfirmedOrder,
    },
}
native_record!(FeasibilityInputs {revision:Revision,digest:PayloadDigest,delivery:DeliveryInput,products:ProductUseSnapshot,confirmations:Vec<EssentialsConfirmation>,evaluated_at:Instant});
native_record!(UseCheck {line_id:DemandLineId,use_key:UseKey,assessment:ProductDateAssessment,issues:Vec<UseIssue>});
native_record!(FoodSummary {
    kind: FoodSummaryKind,
    conflict: Conflict
});
native_record!(FeasibilityReport {derived_digest:PayloadDigest,inputs_digest:PayloadDigest,uses:Vec<UseCheck>,other_checks:Vec<Conflict>,warnings:Vec<FoodSummary>,blocking:Vec<FoodSummary>,digest:PayloadDigest});
native_record!(SavedFoodBasis {
    plan: WeekPlan,
    context: PlanningContext,
    evidence: FoodEvidence,
    identities: FoodIdentities,
    schedule: ScheduleInputs
});
native_record!(HistoricalApproval {
    basis: SavedFoodBasis,
    derived: DerivedPlan,
    approved: ApprovedPlan,
    order: ConfirmedOrder
});
native_record!(LiveRequirements {revision:Revision,digest:PayloadDigest,meals:Vec<MealRequirements>,unconfirmed:Vec<(SlotId,PersonRef)>});
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)] // Exact retained admitted and withdrawn native records.
pub enum LiveRecipeState {
    Admitted {
        basis: EvidenceBasis,
        classes: ComponentClassView,
        methods: Vec<MethodPhases>,
        resources: Vec<PhaseResources>,
    },
    Withdrawn {
        recipe: RecipeRef,
        evidence: Vec<EvidencePin>,
    },
}
native_record!(RelevantEvidenceSnapshot {revision:Revision,digest:PayloadDigest,recipes:Vec<LiveRecipeState>});
native_record!(LiveFoodPolicy {
    version: Revision,
    digest: PayloadDigest,
    general_seconds: u32,
    rice_seconds: u32,
    maximum_reheats: u8
});
native_record!(LiveSourcePin {source:LiveSource,revision:Revision,digest:PayloadDigest,valid_until:Option<Instant>});
native_record!(ContinuationInputs {revision:Revision,digest:PayloadDigest,sources:Vec<LiveSourcePin>,requirements:LiveRequirements,evidence:RelevantEvidenceSnapshot,policy:LiveFoodPolicy,occupied:OccupancySnapshot,feasibility:FeasibilityInputs});
native_record!(ContinuationReport {order_id:OrderId,approved_digest:PayloadDigest,historical_basis_digest:PayloadDigest,current_inputs_digest:PayloadDigest,feasibility:FeasibilityReport,other_checks:Vec<Conflict>,warnings:Vec<FoodSummary>,blocking:Vec<FoodSummary>,digest:PayloadDigest});
