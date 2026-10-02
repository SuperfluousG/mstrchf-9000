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
