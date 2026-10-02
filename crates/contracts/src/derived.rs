//! Derived plan and demand wire records. These shapes carry no planning algorithm.

use crate::*;

pub fn validate_essentials_binding(
    confirmation: &EssentialsConfirmation,
    demand: &DemandSet,
) -> Result<(), DomainError> {
    let current = demand_digest(&DemandMaterial {
        schema_version: SchemaVersion,
        input_plan_digest: demand.input_plan_digest.clone(),
        lines: demand.lines.clone(),
    })?;
    if current != demand.digest || confirmation.demand_digest != current {
        return Err(DomainError::new(
            ErrorCode::DigestMismatch,
            "/demand_digest",
        ));
    }
    let line = demand
        .lines
        .iter()
        .find(|line| line.line_id == confirmation.line_id)
        .ok_or_else(|| DomainError::new(ErrorCode::MissingReference, "/line_id"))?;
    if line.quantity != confirmation.displayed_quantity {
        return Err(DomainError::new(
            ErrorCode::DigestMismatch,
            "/displayed_quantity",
        ));
    }
    Ok(())
}

record! {
    pub struct DerivedPlan {
        pub schema_version: SchemaVersion,
        pub week_id: WeekId,
        pub plan_revision: Revision,
        pub input_plan_digest: PlanInputDigest,
        pub food_inputs_digest: PayloadDigest,
        pub context_stamp: ContextStamp,
        pub recipe_refs: BoundedVec<RecipeRef, 0, 512>,
        pub batches: BoundedVec<DerivedBatch, 0, 256>,
        pub allocations: BoundedVec<Allocation, 0, 4096>,
        pub prep_sessions: BoundedVec<PrepSession, 0, 256>,
        pub demand: DemandSet,
        pub warnings: BoundedVec<Conflict, 0, 100>,
        pub derivation_policy_version: Revision,
        pub digest: PayloadDigest,
    }
}

record! {
    pub struct DerivedBatch {
        pub batch_id: BatchId,
        pub recipe: RecipeRef,
        pub source_slot: SlotId,
        pub components: BoundedVec<DerivedComponent, 1, 128>,
    }
}

record! {
    pub struct DerivedComponent {
        pub component_id: ComponentId,
        pub produced: Quantity,
        pub allocated: Quantity,
        pub unallocated: NonnegativeQuantity,
        pub original_production: ProductionMoment,
        pub storage_rule: Nullable<StorageRule>,
    }
}

record! {
    pub struct Allocation {
        pub allocation_id: AllocationId,
        pub batch_id: BatchId,
        pub component_id: ComponentId,
        pub target_slot: SlotId,
        pub person: PersonRef,
        pub servings: PositiveRational,
        pub quantity: Quantity,
        pub use_at: ZonedMoment,
        pub planned_reheats: u8,
        pub remix_recipe: Nullable<RecipeRef>,
    }
}

record! {
    pub struct PrepSession {
        pub session_id: PrepSessionId,
        pub week_id: WeekId,
        pub linked_slots: BoundedVec<SlotId, 1, 21>,
        pub attached_to: Nullable<PrepSessionId>,
        pub start: ZonedMoment,
        pub end: ZonedMoment,
        pub ingredient_needed_at: ZonedMoment,
        pub tasks: BoundedVec<ScheduledPrepTask, 1, 256>,
        pub ingredient_ids: BoundedVec<IngredientId, 0, 256>,
    }
}

record! {
    pub struct PrepTaskRef {
        pub session_id: PrepSessionId,
        pub task_id: u16,
    }
}

record! {
    pub struct TimeInterval {
        pub start: ZonedMoment,
        pub end: ZonedMoment,
    }
}

record! {
    pub struct ScheduledPrepTask {
        pub task_id: u16,
        pub recipe: RecipeRef,
        pub prep_id: u16,
        pub depends_on: BoundedVec<PrepTaskRef, 0, 256>,
        pub active_intervals: BoundedVec<TimeInterval, 0, 256>,
        pub elapsed: TimeInterval,
    }
}

record! {
    pub struct DemandSet {
        pub input_plan_digest: PlanInputDigest,
        pub lines: BoundedVec<DemandLine, 0, 4096>,
        pub digest: DemandDigest,
    }
}

record! {
    pub struct DemandLine {
        pub line_id: DemandLineId,
        pub ingredient_id: IngredientId,
        pub quantity: Quantity,
        pub origin_batches: BoundedVec<BatchId, 0, 256>,
        pub origin_extras: BoundedVec<ExtraId, 0, 64>,
        pub essential: bool,
        pub conversion_evidence_ids: BoundedVec<EvidenceId, 0, 256>,
    }
}

record! {
    pub struct EssentialsConfirmation {
        pub line_id: DemandLineId,
        pub choice: EssentialsChoice,
        pub displayed_quantity: Quantity,
        pub demand_digest: DemandDigest,
        pub confirmed_at: Instant,
    }
}

string_enum! {
    pub enum EssentialsChoice {
        HaveEnough,
        AddToShop,
    }
}

record! {
    pub struct ApprovedPlan {
        pub approval_id: ApprovalId,
        pub week_id: WeekId,
        pub plan_revision: Revision,
        pub derived_digest: PayloadDigest,
        pub context_stamp: ContextStamp,
        pub approved_at: Instant,
    }
}

tagged! {
    pub enum ProductionMoment {
        Cooked { completed_at: ZonedMoment },
        Unheated { preparation_started_at: ZonedMoment },
    }
}
