//! Order and calendar wire records. Provider integration remains outside this crate.

use crate::*;

string_enum! {
    pub enum CalendarEventKind {
        Cooking,
        Prep,
        Delivery,
    }
}

string_enum! {
    pub enum CalendarObservationState {
        Present,
        Absent,
        Unknown,
    }
}

string_enum! {
    pub enum PublicationState {
        NotRequested,
        Prepared,
        ExportedManualImportRequired,
        Publishing,
        Published,
        NeedsUser,
        Failed,
    }
}

tagged! {
    pub enum OrderConfirmationSource {
        UserConfirmation {},
        AuthorisedRead {
            evidence_id: EvidenceId,
        },
    }
}

record! {
    pub struct CalendarSettings {
        pub destination_id: Nullable<CalendarDestinationId>,
        pub zone: ZoneId,
        pub meal_times: BoundedVec<MealTime, 3, 3>,
        pub reminders: ReminderSettings,
    }
}

record! {
    pub struct OrderConfirmation {
        pub week_id: WeekId,
        pub approval_id: ApprovalId,
        pub delivery_start: ZonedMoment,
        pub delivery_end: ZonedMoment,
        pub confirmed_at: Instant,
        pub source: OrderConfirmationSource,
    }
}

record! {
    pub struct ConfirmedOrder {
        pub order_id: OrderId,
        pub confirmation: OrderConfirmation,
        pub approved_plan: ApprovedPlan,
        pub ordered_recipe_refs: BoundedVec<RecipeRef, 0, 512>,
    }
}

record! {
    pub struct CalendarEventLogicalKey {
        pub week_id: WeekId,
        pub kind: CalendarEventKind,
        pub session_id: Nullable<PrepSessionId>,
        pub order_id: Nullable<OrderId>,
    }
}

record! {
    pub struct CalendarEvent {
        pub event_id: CalendarEventId,
        pub logical_key: CalendarEventLogicalKey,
        pub payload_revision: Revision,
        pub start: ZonedMoment,
        pub end: ZonedMoment,
        pub title: Title,
        pub body: LongText,
        pub event_payload_digest: PayloadDigest,
    }
}

record! {
    pub struct EventBatch {
        pub week_id: WeekId,
        pub order_id: OrderId,
        pub approved_plan_digest: PayloadDigest,
        pub destination_id: CalendarDestinationId,
        pub events: BoundedVec<CalendarEvent, 1, 256>,
    }
}

record! {
    pub struct EventBatchDocumentV1 {
        pub schema_version: SchemaVersion,
        pub event_batch: EventBatch,
    }
}

record! {
    pub struct CalendarObservation {
        pub event_id: CalendarEventId,
        pub provider_event_ref: Nullable<Text>,
        pub observed_payload_digest: Nullable<Digest>,
        pub state: CalendarObservationState,
        pub observed_at: Instant,
        pub user_changed: bool,
    }
}

tagged! {
    pub enum CalendarPublicationConfirmationSource {
        UserReviewedGoogleRead {
            read_id: Text,
            read_digest: Digest,
        },
    }
}

record! {
    /// Exact content observed for one explicitly mapped Google event. This is
    /// retained user-reviewed evidence, not independently authenticated IO.
    pub struct PublishedCalendarEvent {
        pub event_id: CalendarEventId,
        pub provider_event_id: Text,
        pub title: Title,
        pub body: LongText,
        pub start: Instant,
        pub end: Instant,
        pub zone: ZoneId,
        pub observed_at: Instant,
    }
}

record! {
    pub struct CalendarPublicationConfirmation {
        pub preparation_command_id: CommandId,
        pub order_id: OrderId,
        pub destination_id: CalendarDestinationId,
        pub event_batch_digest: Digest,
        pub calendar_id: Text,
        pub source: CalendarPublicationConfirmationSource,
        pub observed_at: Instant,
        pub events: BoundedVec<PublishedCalendarEvent, 1, 256>,
    }
}
