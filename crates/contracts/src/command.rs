use crate::*;
record! {pub struct Conflict {pub code:ErrorCode,pub week_ids:BoundedVec<WeekId,0,4>,pub slot_ids:BoundedVec<SlotId,0,84>,pub recipe_refs:BoundedVec<RecipeRef,0,512>,pub evidence_ids:BoundedVec<EvidenceId,0,256>,pub blocking:bool}}
tagged! {pub enum AppCommand {
 SaveHouseholdDraft{draft:HouseholdDraft},UpdateProfile{profile:HouseholdProfile,expected_profile_revision:Revision},SetPrivacy{policy:PrivacyPolicy,expected_policy_revision:Revision},SaveWeek{week:WeekEdit,expected_week_revision:Revision},ApplyPlanProposal{proposal:PlanProposal,context_stamp:ContextStamp},ConfirmEssentials{confirmations:BoundedVec<EssentialsConfirmation,1,4096>},ApprovePlan{week_id:WeekId,plan_revision:Revision,derived_digest:PayloadDigest},ConfirmOrder{confirmation:OrderConfirmation},RequestJob{action:Action,scope:ActionScope},CancelJob{job_id:JobId},RecordUserStop{worker:WorkerIdentity,qualification_id:EvidenceId,procedure_version:Revision,visible_status:VisibleTerminalStatus},RequestCalendarPublication{order_id:OrderId,destination_id:CalendarDestinationId,event_batch_digest:Digest}
}}
record! {pub struct AppCommandV1 {pub schema_version:SchemaVersion,pub command_id:CommandId,pub expected_snapshot_revision:Revision,pub command:AppCommand}}
record! {pub struct Snapshot {pub schema_version:SchemaVersion,pub snapshot_revision:Revision,pub household_draft:HouseholdDraft,pub profile:Nullable<HouseholdProfile>,pub weeks:BoundedVec<Week,0,4>,pub jobs:BoundedVec<JobSummary,0,1024>,pub conflicts:BoundedVec<Conflict,0,100>}}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallerClass {
    LocalUi,
    ExchangeImport,
    QualifiedAdapter,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandKind {
    SaveHouseholdDraft,
    UpdateProfile,
    SetPrivacy,
    SaveWeek,
    ApplyPlanProposal,
    ConfirmEssentials,
    ApprovePlan,
    ConfirmOrder,
    RequestJob,
    CancelJob,
    RecordUserStop,
    RequestCalendarPublication,
}
impl CommandKind {
    pub const ALL: [Self; 12] = [
        Self::SaveHouseholdDraft,
        Self::UpdateProfile,
        Self::SetPrivacy,
        Self::SaveWeek,
        Self::ApplyPlanProposal,
        Self::ConfirmEssentials,
        Self::ApprovePlan,
        Self::ConfirmOrder,
        Self::RequestJob,
        Self::CancelJob,
        Self::RecordUserStop,
        Self::RequestCalendarPublication,
    ];
}
impl AppCommand {
    pub fn kind(&self) -> CommandKind {
        match self {
            Self::SaveHouseholdDraft { .. } => CommandKind::SaveHouseholdDraft,
            Self::UpdateProfile { .. } => CommandKind::UpdateProfile,
            Self::SetPrivacy { .. } => CommandKind::SetPrivacy,
            Self::SaveWeek { .. } => CommandKind::SaveWeek,
            Self::ApplyPlanProposal { .. } => CommandKind::ApplyPlanProposal,
            Self::ConfirmEssentials { .. } => CommandKind::ConfirmEssentials,
            Self::ApprovePlan { .. } => CommandKind::ApprovePlan,
            Self::ConfirmOrder { .. } => CommandKind::ConfirmOrder,
            Self::RequestJob { .. } => CommandKind::RequestJob,
            Self::CancelJob { .. } => CommandKind::CancelJob,
            Self::RecordUserStop { .. } => CommandKind::RecordUserStop,
            Self::RequestCalendarPublication { .. } => CommandKind::RequestCalendarPublication,
        }
    }
}
pub fn authorize_command(kind: CommandKind, caller: CallerClass) -> Result<(), DomainError> {
    let allowed = match caller {
        CallerClass::ExchangeImport => false,
        CallerClass::QualifiedAdapter => matches!(kind, CommandKind::ConfirmOrder),
        CallerClass::LocalUi => match kind {
            CommandKind::SaveHouseholdDraft
            | CommandKind::UpdateProfile
            | CommandKind::SetPrivacy
            | CommandKind::SaveWeek
            | CommandKind::ApplyPlanProposal
            | CommandKind::ConfirmEssentials
            | CommandKind::ApprovePlan
            | CommandKind::ConfirmOrder
            | CommandKind::RequestJob
            | CommandKind::CancelJob
            | CommandKind::RecordUserStop
            | CommandKind::RequestCalendarPublication => true,
        },
    };
    if allowed {
        Ok(())
    } else {
        Err(DomainError::new(ErrorCode::AuthorityDenied, "/command"))
    }
}
