use mstrchf_contracts::*;
use mstrchf_core::{food::*, portions::ScaledRecipe, recipes::EvidencePin};

#[derive(Clone, Copy, Debug)]
pub struct OwnerInput<'a> {
    pub historical: &'a HistoricalApproval,
    pub current: &'a ContinuationInputs,
}
#[derive(Debug)]
pub struct CalendarInputs<'a> {
    pub subject: OwnerInput<'a>,
    pub linked_owners: &'a [OwnerInput<'a>],
    pub assemblies: &'a [AssemblyEvidence],
    pub instruction_text: &'a [OccurrenceText],
    pub evaluated_at: &'a Instant,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionContribution {
    pub owner: PlanOwnerPin,
    pub session_id: PrepSessionId,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct AssemblyEvidence {
    pub root: SessionContribution,
    pub contributors: Vec<SessionContribution>,
    pub evidence: Vec<EvidencePin>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OccurrenceText {
    pub owner: PlanOwnerPin,
    pub occurrence: OccurrenceKey,
    pub method: MethodKind,
    pub scaled: ScaledRecipe,
    pub steps: Vec<PresentedStep>,
    pub evidence: Vec<EvidencePin>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PresentedStep {
    pub prep_id: u16,
    pub step_id: u16,
    pub text: Text,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct EventDraft {
    pub logical_key: CalendarEventLogicalKey,
    pub contributors: Vec<SessionContribution>,
    pub start: ZonedMoment,
    pub end: ZonedMoment,
    pub title: Title,
    pub body: LongText,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PreparedCalendar {
    pub subject_order_id: OrderId,
    pub evaluated_at: Instant,
    pub reports: Vec<ContinuationReport>,
    pub drafts: Vec<EventDraft>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SavedCalendarEvent {
    pub event: CalendarEvent,
    pub contributors: Vec<SessionContribution>,
    pub sequence: u32,
    pub revised_at: Instant,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarIdentity {
    pub key: CalendarEventLogicalKey,
    pub event_id: CalendarEventId,
}
#[derive(Clone, Debug, PartialEq, Eq)]
#[allow(clippy::large_enum_variant)] // Keep the adopted native custody API; scope is capped at 256 rows.
pub enum PriorEventState {
    NeverVersioned,
    Retained(SavedCalendarEvent),
    Unavailable,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum UpdateDisposition {
    Upsert,
    Remove,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ScopedEvent {
    pub key: CalendarEventLogicalKey,
    pub prior: PriorEventState,
    pub disposition: UpdateDisposition,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SessionIdentity {
    pub week_id: WeekId,
    pub session_id: PrepSessionId,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ContributorTransition {
    pub member: SessionIdentity,
    pub before: Option<CalendarEventLogicalKey>,
    pub after: Option<CalendarEventLogicalKey>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarUpdateProjection {
    pub scope: Vec<ScopedEvent>,
    pub membership: Vec<ContributorTransition>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CalendarSnapshot {
    pub subject_order_id: OrderId,
    pub evaluated_at: Instant,
    pub reports: Vec<ContinuationReport>,
    pub events: Vec<SavedCalendarEvent>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CalendarFailure {
    Invalid(DomainError),
    Blocked(Vec<ContinuationReport>),
    ReconciliationRequired {
        reason: DomainError,
        keys: Vec<CalendarEventLogicalKey>,
    },
}
impl From<DomainError> for CalendarFailure {
    fn from(value: DomainError) -> Self {
        Self::Invalid(value)
    }
}
