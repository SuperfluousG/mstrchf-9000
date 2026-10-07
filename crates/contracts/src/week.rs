//! Week, retained selections and per-meal requirement records.

use crate::*;

string_enum! {
    pub enum MealType {
        Breakfast,
        Lunch,
        Dinner,
    }
}

string_enum! {
    pub enum Weekday {
        Monday,
        Tuesday,
        Wednesday,
        Thursday,
        Friday,
        Saturday,
        Sunday,
    }
}

string_enum! {
    pub enum MealLocation {
        AtHome,
        OnTheMove,
        Skip,
    }
}

string_enum! {
    pub enum SelectionOrigin {
        Fixed,
        Suggested,
    }
}

string_enum! {
    pub enum FeedbackRating {
        Liked,
        Okay,
        NotForMe,
    }
}

string_enum! {
    pub enum FeedbackReason {
        TooSpicy,
        TooMuch,
        TooLittle,
        TooMuchEffort,
    }
}

pub type DayEnabled = BoundedVec<bool, 7, 7>;

record! {
    pub struct MealTypeEnabled {
        pub breakfast: bool,
        pub lunch: bool,
        pub dinner: bool,
    }
}

tagged! {
    pub enum PersonRef {
        Diner {
            diner_id: DinerId,
        },
        Guest {
            occasion_id: OccasionId,
            guest_id: GuestId,
        },
    }
}

tagged! {
    pub enum BatchUse {
        NewBatch {
            batch_id: BatchId,
            production_servings: PositiveRational,
        },
        Leftover {
            batch_id: BatchId,
            component_ids: BoundedVec<ComponentId, 1, 128>,
        },
    }
}

record! {
    pub struct SlotId {
        pub week_id: WeekId,
        pub date: LocalDate,
        pub meal_type: MealType,
    }
}

record! {
    pub struct WeekRange {
        pub week_id: WeekId,
        pub monday: LocalDate,
        pub zone: ZoneId,
    }
}

record! {
    pub struct Occasion {
        pub occasion_id: OccasionId,
        pub guests: BoundedVec<Guest, 0, 64>,
        pub host_diner_ids: BoundedVec<DinerId, 0, 64>,
    }
}

record! {
    pub struct Guest {
        pub guest_id: GuestId,
        pub age_band: AgeBand,
        pub restrictions: RestrictionDeclaration,
        pub portion: PortionPreference,
    }
}

record! {
    pub struct Cell {
        pub slot: SlotId,
        pub choice: MealLocation,
        pub attendee_ids: BoundedVec<DinerId, 0, 64>,
        pub occasion: Nullable<Occasion>,
    }
}

record! {
    pub struct ComponentAllocation {
        pub component_id: ComponentId,
        pub quantity: Quantity,
    }
}

record! {
    pub struct BatchLink {
        pub batch_id: BatchId,
        pub source_slot: SlotId,
        pub target_slot: SlotId,
        pub components: BoundedVec<ComponentAllocation, 1, 128>,
    }
}

record! {
    pub struct PortionPresetRef {
        pub recipe: RecipeRef,
        pub preset: PortionPresetName,
        pub age_band: AgeBand,
        pub meal_type: MealType,
    }
}

record! {
    pub struct DinerServing {
        pub person: PersonRef,
        pub preset_ref: PortionPresetRef,
        pub servings: PositiveRational,
    }
}

record! {
    pub struct ExtraSelection {
        pub extra_id: ExtraId,
        pub recipe: RecipeRef,
        pub recipients: BoundedVec<DinerServing, 1, 64>,
    }
}

record! {
    pub struct PersonPortionOverride {
        pub person: PersonRef,
        pub preset: PortionPresetName,
        pub adjustment: PositiveRational,
    }
}

record! {
    pub struct MealOverrides {
        pub portions: BoundedVec<PersonPortionOverride, 0, 64>,
        pub eat_by: Nullable<ZonedMoment>,
    }
}

record! {
    pub struct MealSelection {
        pub slot: SlotId,
        pub recipe: RecipeRef,
        pub origin: SelectionOrigin,
        pub repeat_authorised: bool,
        pub servings: BoundedVec<DinerServing, 1, 64>,
        pub extras: BoundedVec<ExtraSelection, 0, 64>,
        pub batch_use: BatchUse,
        pub meal_overrides: MealOverrides,
    }
}

record! {
    pub struct WeekPlan {
        pub week_id: WeekId,
        pub revision: Revision,
        pub selections: BoundedVec<MealSelection, 0, 21>,
        pub batch_links: BoundedVec<BatchLink, 0, 256>,
    }
}

record! {
    pub struct WeekPlanDocumentV1 {
        pub schema_version: SchemaVersion,
        pub week_plan: WeekPlan,
    }
}

record! {
    pub struct WeekEdit {
        pub week_id: WeekId,
        pub defaults_snapshot: UsualWeek,
        pub day_enabled: DayEnabled,
        pub meal_type_enabled: MealTypeEnabled,
        pub cells: BoundedVec<Cell, 21, 21>,
        // Null retains the existing plan; a supplied plan is validated by the native writer.
        pub plan: Nullable<WeekPlan>,
    }
}

record! {
    pub struct Week {
        pub schema_version: SchemaVersion,
        pub range: WeekRange,
        pub revision: Revision,
        pub defaults_snapshot: UsualWeek,
        pub day_enabled: DayEnabled,
        pub meal_type_enabled: MealTypeEnabled,
        pub cells: BoundedVec<Cell, 21, 21>,
        pub plan: WeekPlan,
        pub approved_plan: Nullable<ApprovedPlan>,
        pub confirmed_order: Nullable<ConfirmedOrder>,
        pub publication: PublicationState,
    }
}

record! {
    pub struct AttendeeRequirement {
        pub person: PersonRef,
        pub age_band: AgeBand,
        pub hard_requirements: BoundedVec<HardRequirement, 0, 64>,
        pub portion: PortionPreference,
    }
}

record! {
    pub struct MealRequirements {
        pub slot: SlotId,
        pub enabled: bool,
        pub location: MealLocation,
        pub attendees: BoundedVec<AttendeeRequirement, 0, 64>,
        pub shared_hard_requirements: BoundedVec<HardRequirement, 0, 64>,
        pub equipment: BoundedVec<EquipmentCode, 0, 9>,
        pub mobility: Mobility,
        pub eat_by: Nullable<ZonedMoment>,
        pub preferred_hands_on_minutes: u16,
        pub leftovers_allowed: bool,
    }
}

record! {
    pub struct MealRequirementsDocumentV1 {
        pub schema_version: SchemaVersion,
        pub meal_requirements: MealRequirements,
    }
}

record! {
    pub struct ProposedSelection {
        pub slot: SlotId,
        pub selection: Nullable<MealSelection>,
    }
}

record! {
    pub struct WeekChange {
        pub week_id: WeekId,
        pub base_revision: Revision,
        pub replace_slots: BoundedVec<ProposedSelection, 0, 21>,
    }
}

record! {
    pub struct PlanProposal {
        pub changes: BoundedVec<WeekChange, 1, 4>,
    }
}

record! {
    pub struct SelectionHistory {
        pub week_id: WeekId,
        pub slot: SlotId,
        pub recipe: RecipeRef,
        pub servings: BoundedVec<DinerServing, 1, 64>,
        pub extras: BoundedVec<ExtraSelection, 0, 64>,
        pub repeat_authorised: bool,
        pub selected_at: Instant,
    }
}

record! {
    pub struct OrderedHistory {
        pub selection: SelectionHistory,
        pub order_id: OrderId,
        pub ordered_at: Instant,
    }
}

record! {
    pub struct Feedback {
        pub slot: SlotId,
        pub recipe: RecipeRef,
        pub rating: FeedbackRating,
        pub reasons: BoundedVec<FeedbackReason, 0, 4>,
        pub hide: bool,
        pub portion_change: Nullable<PersonPortionOverride>,
        pub portion_change_accepted: bool,
        pub recorded_at: Instant,
    }
}

record! {
    pub struct PlanningHistory {
        pub revision: Revision,
        pub selected: BoundedVec<SelectionHistory, 0, 4096>,
        pub ordered: BoundedVec<OrderedHistory, 0, 4096>,
        pub feedback: BoundedVec<Feedback, 0, 4096>,
        pub hidden_recipe_ids: BoundedVec<RecipeId, 0, 512>,
    }
}

record! {
    pub struct RankingPolicyRef {
        pub version: Revision,
        pub digest: PayloadDigest,
    }
}
