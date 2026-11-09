use crate::*;
record! {pub struct Conflict {pub code:ErrorCode,pub week_ids:BoundedVec<WeekId,0,4>,pub slot_ids:BoundedVec<SlotId,0,84>,pub recipe_refs:BoundedVec<RecipeRef,0,512>,pub evidence_ids:BoundedVec<EvidenceId,0,256>,pub blocking:bool}}
tagged! {pub enum SetupRestrictionEdit {
 ReplaceText{diner_id:DinerId},Resolve{diner_id:DinerId},RemoveDiner{diner_id:DinerId}
}}

record! {
 /// Exact reference to a maker-reviewed immutable packet. Identity is not authority.
 pub struct ReviewedRecipePacketRef {pub recipe:RecipeRef,pub packet_digest:Digest}
}

tagged! {pub enum AppCommand {
 OpenPlanningHorizon{},
 SuggestMeals{week_id:WeekId,mode:PlanMode,slots:BoundedVec<SlotId,0,21>,allow_repeats:bool},
 SuggestPlanningHorizon{ordered_week_ids:BoundedVec<WeekId,4,4>,mode:PlanMode,allow_repeats:bool},
 SavePersonalRecipe{candidate:RecipeCandidate,previous:Nullable<RecipeRef>},
 AdmitReviewedRecipe{packet:ReviewedRecipePacketRef,expected_catalogue_revision:Revision},
 RecordFeedback{feedback:Feedback},
 RestoreHiddenRecipe{recipe_id:RecipeId},
 CorrectOrderDelivery{order_id:OrderId,delivery_start:ZonedMoment,delivery_end:ZonedMoment},
 ConfirmCalendarPublication{confirmation:CalendarPublicationConfirmation},
 SetPrivacyPreservingEssentials{policy:PrivacyPolicy,expected_policy_revision:Revision},
 SaveHouseholdDraft{draft:HouseholdDraft,restriction_issue_edits:BoundedVec<SetupRestrictionEdit,0,64>},UpdateProfile{profile:HouseholdProfile,expected_profile_revision:Revision},SetPrivacy{policy:PrivacyPolicy,expected_policy_revision:Revision},SaveWeek{week:WeekEdit,expected_week_revision:Revision},ApplyPlanProposal{proposal:PlanProposal,context_stamp:ContextStamp},ConfirmEssentials{confirmations:BoundedVec<EssentialsConfirmation,1,4096>},ApprovePlan{week_id:WeekId,plan_revision:Revision,derived_digest:PayloadDigest},ConfirmOrder{confirmation:OrderConfirmation},RequestJob{action:Action,scope:ActionScope},CancelJob{job_id:JobId},RecordUserStop{worker:WorkerIdentity,qualification_id:EvidenceId,procedure_version:Revision,visible_status:VisibleTerminalStatus},RequestCalendarPublication{order_id:OrderId,destination_id:CalendarDestinationId,event_batch_digest:Digest}
}}
record! {pub struct AppCommandV1 {pub schema_version:SchemaVersion,pub command_id:CommandId,pub expected_snapshot_revision:Revision,pub command:AppCommand}}
record! {pub struct Snapshot {pub schema_version:SchemaVersion,pub snapshot_revision:Revision,pub privacy:PrivacyPolicy,pub household_draft:HouseholdDraft,pub profile:Nullable<HouseholdProfile>,pub weeks:BoundedVec<Week,0,4>,pub jobs:BoundedVec<JobSummary,0,1024>,pub conflicts:BoundedVec<Conflict,0,100>}}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CallerClass {
    LocalUi,
    ExchangeImport,
    QualifiedAdapter,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommandKind {
    OpenPlanningHorizon,
    SuggestMeals,
    SuggestPlanningHorizon,
    SavePersonalRecipe,
    AdmitReviewedRecipe,
    RecordFeedback,
    RestoreHiddenRecipe,
    SaveHouseholdDraft,
    UpdateProfile,
    SetPrivacy,
    SetPrivacyPreservingEssentials,
    SaveWeek,
    ApplyPlanProposal,
    ConfirmEssentials,
    ApprovePlan,
    ConfirmOrder,
    CorrectOrderDelivery,
    RequestJob,
    CancelJob,
    RecordUserStop,
    RequestCalendarPublication,
    ConfirmCalendarPublication,
}
impl CommandKind {
    pub const ALL: [Self; 22] = [
        Self::OpenPlanningHorizon,
        Self::SuggestMeals,
        Self::SuggestPlanningHorizon,
        Self::SavePersonalRecipe,
        Self::AdmitReviewedRecipe,
        Self::RecordFeedback,
        Self::RestoreHiddenRecipe,
        Self::SaveHouseholdDraft,
        Self::UpdateProfile,
        Self::SetPrivacy,
        Self::SetPrivacyPreservingEssentials,
        Self::SaveWeek,
        Self::ApplyPlanProposal,
        Self::ConfirmEssentials,
        Self::ApprovePlan,
        Self::ConfirmOrder,
        Self::CorrectOrderDelivery,
        Self::RequestJob,
        Self::CancelJob,
        Self::RecordUserStop,
        Self::RequestCalendarPublication,
        Self::ConfirmCalendarPublication,
    ];
}
impl AppCommand {
    pub fn kind(&self) -> CommandKind {
        match self {
            Self::OpenPlanningHorizon { .. } => CommandKind::OpenPlanningHorizon,
            Self::SuggestMeals { .. } => CommandKind::SuggestMeals,
            Self::SuggestPlanningHorizon { .. } => CommandKind::SuggestPlanningHorizon,
            Self::SavePersonalRecipe { .. } => CommandKind::SavePersonalRecipe,
            Self::AdmitReviewedRecipe { .. } => CommandKind::AdmitReviewedRecipe,
            Self::RecordFeedback { .. } => CommandKind::RecordFeedback,
            Self::RestoreHiddenRecipe { .. } => CommandKind::RestoreHiddenRecipe,
            Self::SaveHouseholdDraft { .. } => CommandKind::SaveHouseholdDraft,
            Self::UpdateProfile { .. } => CommandKind::UpdateProfile,
            Self::SetPrivacy { .. } => CommandKind::SetPrivacy,
            Self::SetPrivacyPreservingEssentials { .. } => CommandKind::SetPrivacyPreservingEssentials,
            Self::SaveWeek { .. } => CommandKind::SaveWeek,
            Self::ApplyPlanProposal { .. } => CommandKind::ApplyPlanProposal,
            Self::ConfirmEssentials { .. } => CommandKind::ConfirmEssentials,
            Self::ApprovePlan { .. } => CommandKind::ApprovePlan,
            Self::ConfirmOrder { .. } => CommandKind::ConfirmOrder,
            Self::CorrectOrderDelivery { .. } => CommandKind::CorrectOrderDelivery,
            Self::RequestJob { .. } => CommandKind::RequestJob,
            Self::CancelJob { .. } => CommandKind::CancelJob,
            Self::RecordUserStop { .. } => CommandKind::RecordUserStop,
            Self::RequestCalendarPublication { .. } => CommandKind::RequestCalendarPublication,
            Self::ConfirmCalendarPublication { .. } => CommandKind::ConfirmCalendarPublication,
        }
    }
}

/// Rejects RequestJob commands whose typed action and scope are incompatible.
/// Current-state and authority checks remain the command handler's responsibility.
pub fn validate_request_job_command(command: &AppCommandV1) -> Result<(), DomainError> {
    let AppCommand::RequestJob { action, scope } = &command.command else {
        return Ok(());
    };
    let compatible = matches!(
        (action, scope),
        (Action::Setup, ActionScope::Setup { .. })
            | (Action::Plan, ActionScope::Plan { .. })
            | (Action::AddAdaptRecipe, ActionScope::Recipe { .. })
            | (Action::PrepareShop, ActionScope::Shop { .. })
            | (Action::FinishWeek, ActionScope::Calendar { .. })
            | (Action::CheckBooking, ActionScope::Booking { .. })
    );
    if compatible {
        Ok(())
    } else {
        Err(DomainError::new(
            ErrorCode::ScopeViolation,
            "/command/data/scope",
        ))
    }
}

pub fn authorize_command(kind: CommandKind, caller: CallerClass) -> Result<(), DomainError> {
    let allowed = match caller {
        CallerClass::ExchangeImport => false,
        CallerClass::QualifiedAdapter => matches!(kind, CommandKind::ConfirmOrder),
        CallerClass::LocalUi => match kind {
            CommandKind::OpenPlanningHorizon | CommandKind::SuggestMeals | CommandKind::SuggestPlanningHorizon | CommandKind::SavePersonalRecipe | CommandKind::AdmitReviewedRecipe | CommandKind::RecordFeedback | CommandKind::RestoreHiddenRecipe => true,
            CommandKind::SaveHouseholdDraft
            | CommandKind::UpdateProfile
            | CommandKind::SetPrivacy
            | CommandKind::SetPrivacyPreservingEssentials
            | CommandKind::SaveWeek
            | CommandKind::ApplyPlanProposal
            | CommandKind::ConfirmEssentials
            | CommandKind::ApprovePlan
            | CommandKind::ConfirmOrder
            | CommandKind::CorrectOrderDelivery
            | CommandKind::RequestJob
            | CommandKind::CancelJob
            | CommandKind::RecordUserStop
            | CommandKind::RequestCalendarPublication
            | CommandKind::ConfirmCalendarPublication => true,
        },
    };
    if allowed {
        Ok(())
    } else {
        Err(DomainError::new(ErrorCode::AuthorityDenied, "/command"))
    }
}
