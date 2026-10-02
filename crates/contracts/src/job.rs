use crate::*;
string_enum! {pub enum Action {Setup,Plan,AddAdaptRecipe,PrepareShop,FinishWeek,CheckBooking}}
string_enum! {pub enum Capability {ScopedFileRead,AtomicResultWrite,NativeLaunch,BrowserRead,BrowserMutate,CalendarRead,CalendarWrite,WorkerStop}}
string_enum! {pub enum PlanMode {FillOpen,SwapNamed,ReplaceSuggestions}}
string_enum! {pub enum ShopPhase {Observe,Reconcile,Mutate}}
string_enum! {pub enum JobState {Prepared,AwaitingUser,Running,NeedsUser,Succeeded,Failed,Cancelled}}
string_enum! {pub enum ResultStatus {Complete,NeedsUser,Failed,Cancelled}}
string_enum! {pub enum RequiredAction {ChooseAlternative,InspectProduct,SignIn,StopWorker,Reconcile,RetryRead,ManualExport,ReviewCandidate}}
string_enum! {pub enum CapabilityDetail {Available,Unavailable,PermissionNeeded,Unknown}}
tagged! {pub enum ActionScope {
 Plan{mode:PlanMode,slots:BoundedVec<SlotId,0,84>},
 Recipe{source_ids:BoundedVec<EvidenceId,1,64>},
 Shop{key:BasketKey,phase:ShopPhase,approval_id:Nullable<ApprovalId>,read_scope:ReadScope},
 Calendar{destination_id:CalendarDestinationId,event_ids:BoundedVec<CalendarEventId,1,256>,approval_id:ApprovalId,read_scope:ReadScope},
 Booking{week_ids:BoundedVec<WeekId,1,4>,read_scope:ReadScope},
 Setup{provider:Provider}
}}
record! {pub struct AiPreferences {pub spending_style:SpendingStyle,pub higher_protein:bool,pub more_variety:bool,pub quick_meals:bool,pub whole_food_preference:bool,pub preferred_hands_on_minutes:u16}}
record! {pub struct AiHistory {pub selected_recipe_refs:BoundedVec<RecipeRef,0,512>,pub ordered_recipe_refs:BoundedVec<RecipeRef,0,512>,pub hidden_recipe_ids:BoundedVec<RecipeId,0,512>,pub feedback:BoundedVec<AiFeedback,0,512>}}
record! {pub struct AiFeedback {pub recipe:RecipeRef,pub rating:FeedbackRating,pub reasons:BoundedVec<FeedbackReason,0,4>}}
record! {pub struct OtherWeekSummary {pub week_id:WeekId,pub revision:Revision,pub selections:BoundedVec<OtherSelection,0,21>}}
record! {pub struct OtherSelection {pub slot:SlotId,pub recipe:RecipeRef,pub origin:SelectionOrigin,pub repeat_authorised:bool,pub leftover:bool}}
record! {pub struct RecipeAdaptation {pub requirements:BoundedVec<HardRequirement,0,64>,pub equipment:BoundedVec<EquipmentCode,0,9>,pub portion_requests:BoundedVec<PortionPresetRef,0,192>}}
record! {pub struct ApprovedContributionRef {pub approval_id:ApprovalId,pub derived_digest:PayloadDigest,pub product_evidence_digest:Digest,pub price_approval_digest:Digest,pub lines:BoundedVec<ApprovedProductLine,1,256>}}
record! {pub struct ApprovedProductLine {pub product:ProductRef,pub expected_quantity:u32,pub target_quantity:u32,pub removal_approved:bool,pub planner_attribution_digest:Nullable<Digest>}}
tagged! {pub enum JobPayload {
 PersonalisedPlan{requirements:BoundedVec<MealRequirements,1,84>,recipe_refs:BoundedVec<RecipeRef,0,512>,preferences:AiPreferences,history:AiHistory,other_weeks:BoundedVec<OtherWeekSummary,0,3>},
 GenericCatalogue{public_recipes:BoundedVec<RecipeRevision,1,512>,meal_types:BoundedVec<MealType,0,3>},
 RecipeCandidate{source_id:EvidenceId,source:SourceRecord,source_text:LongText,adaptation:Nullable<RecipeAdaptation>},
 ProductObservation{read_scope:ReadScope,products:BoundedVec<ProductRef,0,256>,approved_contribution:Nullable<ApprovedContributionRef>},
 CalendarPublication{batch:EventBatch,read_scope:ReadScope},
 BookingCheck{week_ids:BoundedVec<WeekId,1,4>,read_scope:ReadScope},
 SetupCheck{provider:Provider,requested_capabilities:BoundedVec<Capability,1,8>}
}}
record! {pub struct ArtefactRef {pub artefact_id:ArtefactId,pub relative_name:Text,pub digest:Digest,pub byte_length:u32,pub media_type:ArtefactMediaType}}
#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
pub enum ArtefactMediaType {
    #[serde(rename = "application/json")]
    Json,
}
record! {pub struct JobRequestV1 {pub schema_version:SchemaVersion,pub job_id:JobId,pub action:Action,pub week_id:Nullable<WeekId>,pub base_plan_revision:Nullable<Revision>,pub context_stamp:ContextStamp,pub input_digest:InputDigest,pub privacy_policy_revision:Revision,pub required_capabilities:BoundedVec<Capability,1,8>,pub allowed_artefacts:BoundedVec<ArtefactRef,0,64>,pub payload:JobPayload,pub scope:ActionScope,pub created_at:Instant,pub expires_at:Instant}}
record! {pub struct RequestMaterial {pub schema_version:SchemaVersion,pub job_id:JobId,pub action:Action,pub week_id:Nullable<WeekId>,pub base_plan_revision:Nullable<Revision>,pub context_stamp:ContextStamp,pub privacy_policy_revision:Revision,pub required_capabilities:BoundedVec<Capability,1,8>,pub allowed_artefacts:BoundedVec<ArtefactRef,0,64>,pub payload:JobPayload,pub scope:ActionScope,pub created_at:Instant,pub expires_at:Instant}}
impl From<&JobRequestV1> for RequestMaterial {
    fn from(r: &JobRequestV1) -> Self {
        Self {
            schema_version: r.schema_version,
            job_id: r.job_id.clone(),
            action: r.action,
            week_id: r.week_id.clone(),
            base_plan_revision: r.base_plan_revision.clone(),
            context_stamp: r.context_stamp.clone(),
            privacy_policy_revision: r.privacy_policy_revision,
            required_capabilities: r.required_capabilities.clone(),
            allowed_artefacts: r.allowed_artefacts.clone(),
            payload: r.payload.clone(),
            scope: r.scope.clone(),
            created_at: r.created_at.clone(),
            expires_at: r.expires_at.clone(),
        }
    }
}
record! {pub struct CapabilityClaim {pub capability:Capability,pub claimed_available:bool,pub detail_code:CapabilityDetail}}
tagged! {pub enum ResultProposal {None{},Plan{proposal:PlanProposal},Recipe{candidate:Box<RecipeCandidate>},Catalogue{recipe_refs:BoundedVec<RecipeRef,1,512>},Setup{capabilities:BoundedVec<CapabilityClaim,1,8>}}}
record! {pub struct ObservationRef {pub operation_id:OperationId,pub digest:Digest}}
tagged! {pub enum ResultObservation {Read{observation:Box<ReadObservationV1>},Operation{reference:ObservationRef}}}
record! {pub struct UnresolvedLine {pub line_id:Nullable<DemandLineId>,pub code:ErrorCode,pub required_action:RequiredAction}}
record! {pub struct JobResultV1 {pub schema_version:SchemaVersion,pub result_id:ResultId,pub job_id:JobId,pub action:Action,pub week_id:Nullable<WeekId>,pub base_plan_revision:Nullable<Revision>,pub input_digest:InputDigest,pub context_stamp:ContextStamp,pub privacy_policy_revision:Revision,pub status:ResultStatus,pub proposal:ResultProposal,pub observations:BoundedVec<ResultObservation,0,256>,pub unresolved:BoundedVec<UnresolvedLine,0,256>,pub error:Nullable<DomainError>,pub completed_at:Instant}}
record! {pub struct JobSummary {pub job_id:JobId,pub action:Action,pub state:JobState,pub reason:Nullable<ErrorCode>,pub next_action:Nullable<RequiredAction>}}
tagged! {pub enum ImportOutcome {Applied{snapshot_revision:Revision},Duplicate{original_snapshot_revision:Revision},Rejected{error:DomainError}}}

fn refuse(code: ErrorCode, path: &str) -> DomainError {
    DomainError::new(code, path)
}
pub fn validate_request_shape(r: &JobRequestV1) -> Result<(), DomainError> {
    if let JobPayload::PersonalisedPlan { requirements, .. } = &r.payload {
        requirements
            .validate_shape()
            .map_err(crate::parse::shape_error)?;
    }
    if let JobPayload::GenericCatalogue { public_recipes, .. } = &r.payload {
        // Validate the embedded children, never recursively validate this request.
        public_recipes
            .validate_shape()
            .map_err(crate::parse::shape_error)?;
    }
    validate_stamp(&r.context_stamp)?;
    if r.created_at.0 >= r.expires_at.0 {
        return Err(refuse(ErrorCode::InvalidValue, "/expires_at"));
    }
    if r.privacy_policy_revision != r.context_stamp.privacy_revision {
        return Err(refuse(ErrorCode::ContextStale, "/privacy_policy_revision"));
    }
    if r.week_id.0.is_some() != r.base_plan_revision.0.is_some() {
        return Err(refuse(ErrorCode::InvalidValue, "/week_id"));
    }
    if let (Some(w), Some(rev)) = (&r.week_id.0, &r.base_plan_revision.0)
        && !r
            .context_stamp
            .read_weeks
            .iter()
            .any(|x| &x.week_id == w && &x.revision == rev)
    {
        return Err(refuse(
            ErrorCode::MissingReference,
            "/context_stamp/read_weeks",
        ));
    }
    let writes = &r.context_stamp.write_weeks;
    let ok = match (&r.action, &r.payload, &r.scope) {
        (
            Action::Setup,
            JobPayload::SetupCheck {
                provider,
                requested_capabilities,
            },
            ActionScope::Setup { provider: p },
        ) => {
            provider == p
                && r.week_id.0.is_none()
                && r.context_stamp.read_weeks.is_empty()
                && writes.is_empty()
                && requested_capabilities
                    .iter()
                    .all(|x| r.required_capabilities.contains(x))
        }
        (
            Action::Plan,
            JobPayload::PersonalisedPlan {
                requirements,
                recipe_refs,
                other_weeks,
                ..
            },
            ActionScope::Plan { slots, .. },
        ) => {
            r.week_id.0.is_some()
                && !writes.is_empty()
                && !slots.is_empty()
                && requirements.len() == slots.len()
                && requirements.iter().all(|q| slots.contains(&q.slot))
                && slots
                    .iter()
                    .all(|s| writes.iter().any(|w| w.week_id == s.week_id))
                && recipe_refs
                    .iter()
                    .all(|x| r.context_stamp.recipe_refs.contains(x))
                && other_weeks.iter().all(|x| {
                    Some(&x.week_id) != r.week_id.0.as_ref()
                        && r.context_stamp
                            .read_weeks
                            .iter()
                            .any(|w| w.week_id == x.week_id && w.revision == x.revision)
                })
        }
        (
            Action::Plan,
            JobPayload::GenericCatalogue { public_recipes, .. },
            ActionScope::Plan { slots, .. },
        ) => {
            r.week_id.0.is_some()
                && writes.is_empty()
                && slots.is_empty()
                && !public_recipes.is_empty()
                && public_recipes.iter().all(|p| {
                    recipe_digest(p).ok().is_some_and(|d| {
                        r.context_stamp.recipe_refs.iter().any(|rf| {
                            rf.recipe_id == p.recipe_id
                                && rf.revision == p.revision
                                && rf.digest == d
                        })
                    })
                })
        }
        (
            Action::AddAdaptRecipe,
            JobPayload::RecipeCandidate { source_id, .. },
            ActionScope::Recipe { source_ids },
        ) => writes.is_empty() && source_ids.contains(source_id),
        (
            Action::PrepareShop,
            JobPayload::ProductObservation {
                read_scope: a,
                approved_contribution,
                products,
            },
            ActionScope::Shop {
                key,
                phase,
                approval_id,
                read_scope: b,
            },
        ) => {
            r.week_id.0.is_some()
                && writes.is_empty()
                && a == b
                && a.resource == ExternalResource::Basket { key: key.clone() }
                && products
                    .iter()
                    .all(|p| a.products.is_empty() || a.products.contains(p))
                && match phase {
                    ShopPhase::Observe | ShopPhase::Reconcile => {
                        approved_contribution.0.is_none() && approval_id.0.is_none()
                    }
                    ShopPhase::Mutate => approval_id
                        .0
                        .as_ref()
                        .zip(approved_contribution.0.as_ref())
                        .is_some_and(|(id, p)| id == &p.approval_id),
                }
        }
        (
            Action::FinishWeek,
            JobPayload::CalendarPublication {
                batch,
                read_scope: a,
            },
            ActionScope::Calendar {
                destination_id,
                event_ids,
                read_scope: b,
                ..
            },
        ) => {
            r.week_id.0.as_ref() == Some(&batch.week_id)
                && writes.is_empty()
                && a == b
                && a.resource
                    == ExternalResource::Calendar {
                        destination_id: destination_id.clone(),
                    }
                && &batch.destination_id == destination_id
                && event_ids.len() == batch.events.len()
                && batch.events.iter().all(|e| event_ids.contains(&e.event_id))
        }
        (
            Action::CheckBooking,
            JobPayload::BookingCheck {
                week_ids: a,
                read_scope: x,
            },
            ActionScope::Booking {
                week_ids: b,
                read_scope: y,
            },
        ) => {
            r.week_id.0.is_some()
                && writes.is_empty()
                && a == b
                && x == y
                && a.iter()
                    .all(|id| r.context_stamp.read_weeks.iter().any(|w| &w.week_id == id))
        }
        _ => false,
    };
    if !ok {
        return Err(refuse(
            if matches!(
                (&r.payload, &r.scope),
                (
                    JobPayload::PersonalisedPlan { .. },
                    ActionScope::Plan { .. }
                )
            ) && writes.is_empty()
            {
                ErrorCode::InvalidValue
            } else {
                ErrorCode::ScopeViolation
            },
            "/payload",
        ));
    }
    if r.allowed_artefacts
        .iter()
        .map(|a| u64::from(a.byte_length))
        .sum::<u64>()
        > MAX_ARTEFACT_BYTES as u64
    {
        return Err(refuse(ErrorCode::LimitExceeded, "/allowed_artefacts"));
    }
    validate_payload_references(&r.payload, &r.context_stamp)?;
    if input_digest(&RequestMaterial::from(r))? != r.input_digest {
        return Err(refuse(ErrorCode::DigestMismatch, "/input_digest"));
    }
    Ok(())
}

fn validate_payload_references(
    payload: &JobPayload,
    stamp: &ContextStamp,
) -> Result<(), DomainError> {
    let known = |reference: &RecipeRef| stamp.recipe_refs.contains(reference);
    let closed = match payload {
        JobPayload::PersonalisedPlan {
            recipe_refs,
            history,
            other_weeks,
            ..
        } => {
            recipe_refs.iter().all(known)
                && history.selected_recipe_refs.iter().all(known)
                && history.ordered_recipe_refs.iter().all(known)
                && history.feedback.iter().all(|f| known(&f.recipe))
                && other_weeks.iter().all(|w| {
                    w.selections
                        .iter()
                        .all(|s| s.slot.week_id == w.week_id && known(&s.recipe))
                })
        }
        JobPayload::GenericCatalogue { public_recipes, .. } => public_recipes.iter().all(|r| {
            r.extras.iter().all(|e| known(&e.recipe))
                && r.reused_inputs
                    .iter()
                    .all(|input| known(&input.source_recipe))
        }),
        JobPayload::RecipeCandidate { adaptation, .. } => adaptation
            .0
            .as_ref()
            .is_none_or(|a| a.portion_requests.iter().all(|p| known(&p.recipe))),
        JobPayload::ProductObservation { .. }
        | JobPayload::CalendarPublication { .. }
        | JobPayload::BookingCheck { .. }
        | JobPayload::SetupCheck { .. } => true,
    };
    if closed {
        Ok(())
    } else {
        Err(refuse(ErrorCode::MissingReference, "/payload"))
    }
}
pub fn validate_result_shape(r: &JobResultV1) -> Result<(), DomainError> {
    validate_stamp(&r.context_stamp)?;
    if r.privacy_policy_revision != r.context_stamp.privacy_revision {
        return Err(refuse(ErrorCode::ContextStale, "/privacy_policy_revision"));
    }
    let none = matches!(r.proposal, ResultProposal::None {});
    let valid = match r.status {
        ResultStatus::NeedsUser => none && !r.unresolved.is_empty() && r.error.0.is_none(),
        ResultStatus::Failed => none && r.error.0.is_some(),
        ResultStatus::Cancelled => none && r.error.0.is_none(),
        ResultStatus::Complete => {
            r.unresolved.is_empty()
                && r.error.0.is_none()
                && match (&r.action, &r.proposal) {
                    (Action::Setup, ResultProposal::Setup { .. })
                    | (Action::Plan, ResultProposal::Plan { .. })
                    | (Action::Plan, ResultProposal::Catalogue { .. })
                    | (Action::AddAdaptRecipe, ResultProposal::Recipe { .. }) => {
                        r.observations.is_empty()
                    }
                    (
                        Action::PrepareShop | Action::FinishWeek | Action::CheckBooking,
                        ResultProposal::None {},
                    ) => !r.observations.is_empty(),
                    _ => false,
                }
        }
    };
    if valid {
        Ok(())
    } else {
        Err(refuse(ErrorCode::InvalidValue, "/status"))
    }
}
pub fn validate_result_binding(
    result: &JobResultV1,
    request: &JobRequestV1,
    current: &ContextStamp,
) -> Result<(), DomainError> {
    request
        .validate_shape()
        .map_err(crate::parse::shape_error)?;
    result.validate_shape().map_err(crate::parse::shape_error)?;
    if result.job_id != request.job_id
        || result.action != request.action
        || result.week_id != request.week_id
        || result.base_plan_revision != request.base_plan_revision
    {
        return Err(refuse(ErrorCode::IdentityMismatch, "/job_id"));
    }
    if result.input_digest != request.input_digest {
        return Err(refuse(ErrorCode::DigestMismatch, "/input_digest"));
    }
    compare_context(&request.context_stamp, &result.context_stamp)?;
    compare_context(&request.context_stamp, current)?;
    if result.privacy_policy_revision != request.privacy_policy_revision {
        return Err(refuse(ErrorCode::ContextStale, "/privacy_policy_revision"));
    }
    if result.completed_at.0 < request.created_at.0 || result.completed_at.0 > request.expires_at.0
    {
        return Err(refuse(ErrorCode::InvalidValue, "/completed_at"));
    }
    for observation in result.observations.iter() {
        match observation {
            ResultObservation::Read { observation: o } => {
                validate_read_binding(o, request, &o.reader, result.completed_at.clone())?;
            }
            ResultObservation::Operation { .. } => {
                if !matches!(request.action, Action::PrepareShop | Action::FinishWeek) {
                    return Err(refuse(ErrorCode::ScopeViolation, "/observations"));
                }
            }
        }
    }
    if result.status != ResultStatus::Complete {
        return Ok(());
    }
    match (&request.payload,&request.scope,&result.proposal){
        (JobPayload::PersonalisedPlan{requirements,recipe_refs,..},ActionScope::Plan{slots,..},ResultProposal::Plan{proposal})=>{
            if proposal.changes.len()!=request.context_stamp.write_weeks.len(){return Err(refuse(ErrorCode::ScopeViolation,"/proposal/data/proposal/changes"));}
            for c in proposal.changes.iter(){if !request.context_stamp.write_weeks.iter().any(|w|w.week_id==c.week_id&&w.revision==c.base_revision)||!c.replace_slots.iter().all(|s|slots.contains(&s.slot)){return Err(refuse(ErrorCode::ScopeViolation,"/proposal/data/proposal/changes"));}}
            for (change_index, change) in proposal.changes.iter().enumerate() {
                for (slot_index, proposed) in change.replace_slots.iter().enumerate() {
                    if let Some(selection) = &proposed.selection.0 {
                        let required = requirements.iter().find(|r|r.slot == proposed.slot).ok_or_else(||refuse(ErrorCode::MissingReference,"/proposal/data/proposal/changes"))?;
                        validate_selection_membership(selection,required,recipe_refs,&request.context_stamp,&format!("/proposal/data/proposal/changes/{change_index}/replace_slots/{slot_index}/selection"))?;
                    }
                }
            }
        },
        (JobPayload::GenericCatalogue{public_recipes,..},_,ResultProposal::Catalogue{recipe_refs})=>{for rf in recipe_refs.iter(){if !public_recipes.iter().any(|r|r.recipe_id==rf.recipe_id&&r.revision==rf.revision&&recipe_digest(r).ok().as_ref()==Some(&rf.digest)){return Err(refuse(ErrorCode::ScopeViolation,"/proposal/data/recipe_refs"));}}},
        (JobPayload::RecipeCandidate{..},_,ResultProposal::Recipe{..})|(JobPayload::SetupCheck{..},_,ResultProposal::Setup{..})=>{},
        (JobPayload::ProductObservation{..},ActionScope::Shop{phase,..},ResultProposal::None{})=>{if *phase!=ShopPhase::Mutate&&!result.observations.iter().any(|o|matches!(o,ResultObservation::Read{..})){return Err(refuse(ErrorCode::ScopeViolation,"/observations"));}},
        (JobPayload::CalendarPublication{..},_,ResultProposal::None{})=>{},
        (JobPayload::BookingCheck{..},_,ResultProposal::None{})=>{if !result.observations.iter().any(|o|matches!(o,ResultObservation::Read{observation} if matches!(observation.body,ReadBody::Slots{..}))){return Err(refuse(ErrorCode::ScopeViolation,"/observations"));}},
        _=>return Err(refuse(ErrorCode::ScopeViolation,"/proposal"))
    }
    Ok(())
}

fn validate_selection_membership(
    selection: &MealSelection,
    required: &MealRequirements,
    allowed: &[RecipeRef],
    stamp: &ContextStamp,
    path: &str,
) -> Result<(), DomainError> {
    let recipe_allowed =
        |recipe: &RecipeRef| allowed.contains(recipe) && stamp.recipe_refs.contains(recipe);
    if !recipe_allowed(&selection.recipe) {
        return Err(refuse(
            ErrorCode::MissingReference,
            &format!("{path}/recipe"),
        ));
    }
    let serving_allowed = |serving: &DinerServing, recipe: &RecipeRef| {
        required
            .attendees
            .iter()
            .any(|a| a.person == serving.person && a.age_band == serving.preset_ref.age_band)
            && serving.preset_ref.recipe == *recipe
            && recipe_allowed(&serving.preset_ref.recipe)
            && serving.preset_ref.meal_type == selection.slot.meal_type
    };
    if !selection
        .servings
        .iter()
        .all(|s| serving_allowed(s, &selection.recipe))
        || (required.enabled
            && required.location != MealLocation::Skip
            && selection.servings.len() != required.attendees.len())
    {
        return Err(refuse(
            ErrorCode::ForeignReference,
            &format!("{path}/servings"),
        ));
    }
    for (index, extra) in selection.extras.iter().enumerate() {
        if !recipe_allowed(&extra.recipe) {
            return Err(refuse(
                ErrorCode::MissingReference,
                &format!("{path}/extras/{index}/recipe"),
            ));
        }
        if !extra.recipients.iter().all(|p| {
            serving_allowed(p, &extra.recipe)
                && selection.servings.iter().any(|s| s.person == p.person)
        }) {
            return Err(refuse(
                ErrorCode::ForeignReference,
                &format!("{path}/extras/{index}/recipients"),
            ));
        }
    }
    if !selection.meal_overrides.portions.iter().all(|p| {
        required.attendees.iter().any(|a| a.person == p.person)
            && selection.servings.iter().any(|s| s.person == p.person)
    }) {
        return Err(refuse(
            ErrorCode::ForeignReference,
            &format!("{path}/meal_overrides/portions"),
        ));
    }
    Ok(())
}
