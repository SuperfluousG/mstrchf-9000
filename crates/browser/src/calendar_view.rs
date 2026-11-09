//! Reuse the native calendar engine for exact personal food sources.
//! All inputs are owner-built native values. This is not an import/admission,
//! confirmation, storage, download, publication or calendar-account boundary.
use crate::{catalogue::RuntimeCatalogue, maker_packets::TrustedPacket, planner_view};
use mstrchf_calendar::*;
use mstrchf_contracts::*;
use mstrchf_core::{food::*, portions::scale_recipe, recipes::*};
mod chilli;
mod overnight;

pub const MANUAL_IMPORT_NOTICE: &str = "Download only. Import this file yourself into the calendar you choose. It does not synchronise changes or confirm calendar publication. Repeated import may create duplicates or leave old entries; update/re-import behaviour must be checked for that calendar. No reminders are added by this export. Your confirmed order remains separate.";

pub struct PersonalCalendarInputs<'a> {
    pub original_catalogue: &'a RuntimeCatalogue,
    pub current_catalogue: &'a RuntimeCatalogue,
    pub basis: &'a SavedFoodBasis,
    pub derived: &'a DerivedPlan,
    pub approved: &'a ApprovedPlan,
    pub order: &'a ConfirmedOrder,
    pub snapshot: &'a Snapshot,
    pub profile: &'a HouseholdProfile,
    pub week: &'a Week,
    /// Complete owner-read occupancy, including the exact excluded subject pin.
    /// Missing/unavailable owner custody must not be replaced with an empty view.
    pub occupied: &'a OccupancySnapshot,
    pub essentials: &'a [EssentialsConfirmation],
    pub policy: &'a LiveFoodPolicy,
    pub evaluated_at: &'a Instant,
}

pub struct ManualCalendarExport {
    pub snapshot: CalendarSnapshot,
    pub ics: Vec<u8>,
    pub notice: &'static str,
}
fn error(code: ErrorCode, path: &str) -> CalendarFailure { DomainError::new(code,path).into() }
fn require(ok: bool, code: ErrorCode, path: &str) -> Result<(),CalendarFailure> {
    if ok { Ok(()) } else { Err(error(code,path)) }
}
fn shape(value: &impl ValidateShape) -> Result<(),CalendarFailure> {
    value.validate_shape().map_err(|issues| error(issues.first().map_or(ErrorCode::InvalidValue,|i|i.code),issues.first().map_or("/calendar",|i|i.path.as_str())))
}
fn packet<'a>(view: &'a RuntimeCatalogue, reference: &RecipeRef) -> Result<&'a TrustedPacket,CalendarFailure> {
    view.resolve_recipe(reference)?;
    view.admitted_packets().iter().find(|packet|packet.reference().recipe==*reference).copied()
        .ok_or_else(||error(ErrorCode::UnsupportedCapability,"/calendar/personal_recipe"))
}
pub(crate) fn contains_courgette_plan(plan: &WeekPlan) -> bool {
    plan.selections.iter().any(|selection| [crate::personal_scope::COURGETTE_HOT_SCOPE.recipe_id,
        crate::personal_scope::COURGETTE_COLD_SCOPE.recipe_id].contains(&selection.recipe.recipe_id.0.as_str()))
}
pub(crate) fn contains_portable_plan(plan: &WeekPlan) -> bool {
    plan.selections.iter().any(|selection| [crate::personal_scope::PORTABLE_HOT_SCOPE.recipe_id,
        crate::personal_scope::PORTABLE_COLD_SCOPE.recipe_id].contains(&selection.recipe.recipe_id.0.as_str()))
}
pub(crate) fn contains_gf_chicken_plan(plan: &WeekPlan) -> bool {
    plan.selections.iter().any(|selection|
        selection.recipe.recipe_id.0==crate::personal_scope::THREE_BEAN_GF_CHICKEN_SCOPE.recipe_id)
}
pub(crate) fn contains_chilli_plan(plan: &WeekPlan) -> bool {
    plan.selections.iter().any(|selection| chilli::family_id(&selection.recipe))
}
pub(crate) fn contains_overnight_plan(plan: &WeekPlan) -> bool {
    plan.selections.iter().any(|selection| selection.recipe.recipe_id.0 == crate::personal_scope::OVERNIGHT_OATS_SCOPE.recipe_id)
}
fn exact_portable(reference: &RecipeRef, hot: bool) -> bool {
    let scope=if hot {&crate::personal_scope::PORTABLE_HOT_SCOPE}else{&crate::personal_scope::PORTABLE_COLD_SCOPE};
    reference.recipe_id.0==scope.recipe_id && reference.revision.0==scope.recipe_revision
        && reference.digest.0==scope.recipe_digest
}
fn exact_original(reference: &RecipeRef, hot: bool, allow_portable: bool) -> bool {
    exact_courgette(reference,hot) || allow_portable && exact_portable(reference,hot)
}
fn exact_courgette(reference: &RecipeRef, hot: bool) -> bool {
    let scope=if hot {&crate::personal_scope::COURGETTE_HOT_SCOPE}else{&crate::personal_scope::COURGETTE_COLD_SCOPE};
    reference.recipe_id.0==scope.recipe_id && reference.revision.0==scope.recipe_revision
        && reference.digest.0==scope.recipe_digest
}
fn original_link(plan: &WeekPlan, link: &BatchLink, allow_portable: bool) -> bool {
    let source=plan.selections.iter().find(|selected|selected.slot==link.source_slot);
    let target=plan.selections.iter().find(|selected|selected.slot==link.target_slot);
    source.zip(target).is_some_and(|(source,target)|
        exact_courgette(&source.recipe,true) && exact_courgette(&target.recipe,false)
        || allow_portable && exact_portable(&source.recipe,true) && exact_portable(&target.recipe,false))
        && source.is_some_and(|source|source.extras.is_empty()
        && matches!(&source.batch_use,BatchUse::NewBatch{batch_id,..} if *batch_id==link.batch_id))
        && target.is_some_and(|target|target.extras.is_empty()
            && matches!(&target.batch_use,BatchUse::Leftover{batch_id,component_ids}
                if *batch_id==link.batch_id && component_ids.len()==1 && link.components.len()==1
                    && component_ids[0]==link.components[0].component_id))
        && link.source_slot!=link.target_slot
}
fn personal_scope(view: &RuntimeCatalogue, context: &PlanningContext, plan: &WeekPlan, allow_named_extra: bool, allow_original: bool, allow_portable: bool, allow_gf: bool, allow_chilli: bool) -> Result<(),CalendarFailure> {
    require(plan.batch_links.is_empty() || allow_original && plan.batch_links.iter().all(|link|original_link(plan,link,allow_portable) || allow_chilli && chilli::link(plan,link)),ErrorCode::UnsupportedCapability,"/calendar/reuse")?;
    for selection in plan.selections.iter() {
        require(selection.extras.is_empty() || allow_named_extra
            && (selection.recipe.recipe_id.0==crate::personal_scope::THREE_BEAN_TWO_UNIT_CHICKEN_EXTRA_SCOPE.recipe_id
                || allow_gf && selection.recipe.recipe_id.0==crate::personal_scope::THREE_BEAN_GF_CHICKEN_SCOPE.recipe_id),
            ErrorCode::UnsupportedCapability,"/calendar/extras")?;
        let base_packet=packet(view,&selection.recipe)?;
        let meal=context.requirements.iter().find(|meal|meal.slot==selection.slot)
            .ok_or_else(||error(ErrorCode::MissingReference,"/calendar/meal"))?;
        crate::personal_scope::check_personal_use_scope(base_packet.recipe(),&selection.recipe,meal,selection,plan,&crate::BoardOptions::default())?;
        for extra in selection.extras.iter() {
            let child=packet(view,&extra.recipe)?;
            if allow_gf && selection.recipe.recipe_id.0==crate::personal_scope::THREE_BEAN_GF_CHICKEN_SCOPE.recipe_id {
                crate::personal_scope::check_gf_chicken_extra_scope(child.recipe(),meal,selection,plan,&crate::BoardOptions::default())?;
            } else {
                crate::personal_scope::check_chicken_extra_scope(child.recipe(),meal,selection,plan,&crate::BoardOptions::default())?;
            }
        }
    }
    Ok(())
}
pub fn owner_pin(basis: &SavedFoodBasis, derived: &DerivedPlan) -> Result<PlanOwnerPin,DomainError> {
    Ok(PlanOwnerPin{week:WeekRevision{week_id:derived.week_id.clone(),revision:derived.plan_revision},derived_digest:derived.digest.clone(),basis_digest:saved_basis_digest(basis)?})
}
/// Explicit-clock access to the exact maintained native derivation material.
/// The trusted caller still owns snapshot validation and catalogue custody.
pub fn material_at(view: &RuntimeCatalogue, snapshot: &Snapshot, profile: &HouseholdProfile,
    week: &Week, at: &Instant) -> Result<Option<(SavedFoodBasis,mstrchf_core::food::FoodDerivation)>,DomainError> {
    at.validate_shape().map_err(|_|DomainError::new(ErrorCode::InvalidValue,"/calendar/clock"))?;
    let previous=crate::CLOCK.with(|clock|clock.replace(Some(at.clone())));
    let result=planner_view::derive_material(view,snapshot,profile,week);
    crate::CLOCK.with(|clock|*clock.borrow_mut()=previous);
    result
}

/// Strict arrival check applies even to ingredients confirmed HaveEnough.
/// Equality is accepted by the existing native UTC-instant comparator.
pub fn check_all_prep_arrival(order: &ConfirmedOrder, derived: &DerivedPlan) -> Result<(),CalendarFailure> {
    shape(order)?;shape(derived)?;
    require(order.approved_plan.derived_digest==derived.digest && order.confirmation.week_id==derived.week_id,ErrorCode::IdentityMismatch,"/calendar/delivery_owner")?;
    for session in derived.prep_sessions.iter() {
        mstrchf_core::prep::check_delivery(&order.confirmation.delivery_end,&session.ingredient_needed_at)?;
    }
    Ok(())
}

/// Immutable historical proof and current evidence are deliberately separate.
/// A later catalogue, profile or pantry confirmation never rewrites the basis.
pub fn prepare_personal_calendar(input: &PersonalCalendarInputs<'_>) -> Result<PreparedCalendar,CalendarFailure> {
    prepare_personal_calendar_for_renderer(input,2)
}

/// Renderer1/2 preserve the exact earlier browser projection. Renderer3 adds
/// only the independently admitted08/09 named-extra occurrence; this is never
/// incoming recipe, product, storage or calendar-account authority.
pub fn prepare_personal_calendar_for_renderer(input: &PersonalCalendarInputs<'_>, renderer_version: u8) -> Result<PreparedCalendar,CalendarFailure> {
    prepare_personal_calendar_with_effective_delivery(input, renderer_version, None)
}

/// Renderer4 takes a receipt-derived window separately from the immutable
/// confirmed order. The owner still establishes correction journal custody;
/// this adapter checks that no other confirmation or food field was replaced.
/// Renderers5/6 add only the exact courgette original-batch home service pair,
/// with6 retaining the same separate effective-delivery custody.
pub(crate) fn prepare_personal_calendar_with_effective_delivery(input: &PersonalCalendarInputs<'_>, renderer_version: u8,
    effective_order: Option<&ConfirmedOrder>) -> Result<PreparedCalendar,CalendarFailure> {
    require(matches!(renderer_version,1..=14),ErrorCode::UnsupportedVersion,"/calendar/renderer_version")?;
    require(matches!(renderer_version,4|6|8|10|12|14)==effective_order.is_some(),ErrorCode::IdentityMismatch,"/calendar/delivery_correction_renderer")?;
    // New renderers are selected only for plans containing the separately
    // reviewed GF pair. Historical renderers keep their exact prior scope.
    let allow_overnight=matches!(renderer_version,13|14);
    require(allow_overnight==contains_overnight_plan(&input.basis.plan),ErrorCode::UnsupportedCapability,"/calendar/overnight_renderer")?;
    let allow_chilli=matches!(renderer_version,11|12) || allow_overnight && contains_chilli_plan(&input.basis.plan);
    require(allow_chilli==contains_chilli_plan(&input.basis.plan),ErrorCode::UnsupportedCapability,"/calendar/chilli_renderer")?;
    let allow_gf=matches!(renderer_version,9|10) || (allow_chilli || allow_overnight) && contains_gf_chicken_plan(&input.basis.plan);
    require(allow_gf==contains_gf_chicken_plan(&input.basis.plan),ErrorCode::UnsupportedCapability,"/calendar/gf_renderer")?;
    let allow_portable=matches!(renderer_version,7|8) || (allow_gf || allow_chilli || allow_overnight) && contains_portable_plan(&input.basis.plan);
    require(allow_portable==contains_portable_plan(&input.basis.plan),ErrorCode::UnsupportedCapability,"/calendar/portable_renderer")?;
    let allow_original=renderer_version>=5;
    require(allow_overnight || allow_chilli || allow_gf || allow_portable || allow_original==contains_courgette_plan(&input.basis.plan),ErrorCode::UnsupportedCapability,"/calendar/original_component_renderer")?;
    let allow_named_extra=renderer_version>=3;
    shape(input.snapshot)?;shape(input.profile)?;shape(input.week)?;shape(input.evaluated_at)?;
    require(input.snapshot.profile.0.as_ref()==Some(input.profile)
        && input.snapshot.weeks.iter().any(|week|week==input.week),ErrorCode::IdentityMismatch,"/calendar/current_owner")?;
    require(input.week.plan==input.basis.plan && input.week.approved_plan.0.as_ref()==Some(input.approved)
        && input.week.confirmed_order.0.as_ref()==Some(input.order),ErrorCode::ContextStale,"/calendar/fixed_order")?;
    require(input.basis.context.stamp.catalogue_revision==input.original_catalogue.pin().revision
        && input.basis.context.recipes.0==input.original_catalogue.recipes(),ErrorCode::IdentityMismatch,"/calendar/original_catalogue")?;
    let original_evidence=input.original_catalogue.food_evidence(&input.derived.recipe_refs.0)?;
    require(original_evidence==input.basis.evidence,ErrorCode::DigestMismatch,"/calendar/original_food")?;
    personal_scope(input.original_catalogue,&input.basis.context,&input.basis.plan,allow_named_extra,allow_original,allow_portable,allow_gf,allow_chilli)?;
    require(!input.derived.prep_sessions.is_empty(),ErrorCode::MissingReference,"/calendar/preparation")?;
    require(input.basis.schedule.layout.attachments.is_empty()
        && input.derived.prep_sessions.iter().all(|session|session.attached_to.0.is_none()),ErrorCode::UnsupportedCapability,"/calendar/attachments")?;
    let mut historical=HistoricalApproval{basis:input.basis.clone(),derived:input.derived.clone(),approved:input.approved.clone(),order:input.order.clone()};
    verify_historical(&historical)?;
    let order=effective_order.unwrap_or(input.order);
    if let Some(effective)=effective_order {
        shape(effective)?;
        let mut expected=input.order.clone();
        expected.confirmation.delivery_start=effective.confirmation.delivery_start.clone();
        expected.confirmation.delivery_end=effective.confirmation.delivery_end.clone();
        require(expected==*effective,ErrorCode::IdentityMismatch,"/calendar/effective_order")?;
        historical.order=effective.clone();
        verify_historical(&historical)?;
    }
    for reference in input.derived.recipe_refs.iter() {
        require(packet(input.original_catalogue,reference)?.not_before()<=&input.approved.approved_at,ErrorCode::AuthorityDenied,"/calendar/approval_clock")?;
    }
    // Exact home scope supplies no leftovers or reheating qualification. The
    // cold packet retains first handling as its original production clock.
    for allocation in input.derived.allocations.iter() {
        let batch=input.derived.batches.iter().find(|batch|batch.batch_id==allocation.batch_id)
            .ok_or_else(||error(ErrorCode::MissingReference,"/calendar/batch"))?;
        let component=batch.components.iter().find(|component|component.component_id==allocation.component_id)
            .ok_or_else(||error(ErrorCode::MissingReference,"/calendar/component"))?;
        if allow_overnight && overnight::exact(&batch.recipe) {
            overnight::allocation(input.original_catalogue,batch,component,allocation)?;
            continue;
        }
        if allow_chilli && chilli::exact(&batch.recipe,true) {
            chilli::allocation(input.original_catalogue,&input.basis.plan,batch,component,allocation)?;
            continue;
        }
        if allow_original && exact_original(&batch.recipe,true,allow_portable) {
            original_allocation(input.original_catalogue,&input.basis.plan,batch,component,allocation,allow_portable)?;
            continue;
        }
        require(allocation.target_slot==batch.source_slot && allocation.planned_reheats==0
            && allocation.remix_recipe.0.is_none() && component.storage_rule.0.is_none(),ErrorCode::UnsupportedCapability,"/calendar/immediate_service")?;
        match &component.original_production {
            ProductionMoment::Cooked{completed_at}=>require(allocation.use_at.instant==completed_at.instant,ErrorCode::UnsupportedCapability,"/calendar/immediate_service")?,
            ProductionMoment::Unheated{preparation_started_at:first_handled_at}=>{
                let timing=crate::personal_scope::cold_home_timing(&batch.recipe)?.ok_or_else(||error(ErrorCode::UnsupportedCapability,"/calendar/production"))?;
                let start=chrono::DateTime::parse_from_rfc3339(&first_handled_at.instant.0).map_err(|_|error(ErrorCode::InvalidValue,"/calendar/production"))?;
                let service=chrono::DateTime::parse_from_rfc3339(&allocation.use_at.instant.0).map_err(|_|error(ErrorCode::InvalidValue,"/calendar/service"))?;
                require(service.signed_duration_since(start).num_seconds()==timing.first_handling_seconds && first_handled_at.local_date==allocation.use_at.local_date,ErrorCode::UnsupportedCapability,"/calendar/same_day_cold_service")?;
            },
        }
    }
    check_all_prep_arrival(order,input.derived)?;
    require(input.policy.version==Revision(1) && input.policy.general_seconds==172800
        && input.policy.rice_seconds==86400 && input.policy.maximum_reheats==1
        && live_food_policy_digest(input.policy)?==input.policy.digest,ErrorCode::IdentityMismatch,"/calendar/current_policy")?;
    require(input.essentials.len()<=4096,ErrorCode::LimitExceeded,"/calendar/essentials")?;
    // The maintained planner reads the browser's explicit request clock. Keep
    // that same clock for direct native callers and restore it before return.
    let current_result=material_at(input.current_catalogue,input.snapshot,input.profile,input.week,input.evaluated_at);
    let (current_basis,_)=current_result?
        .ok_or_else(||error(ErrorCode::MissingReference,"/calendar/current_food"))?;
    personal_scope(input.current_catalogue,&current_basis.context,&input.basis.plan,allow_named_extra,allow_original,allow_portable,allow_gf,allow_chilli)?;
    let mut slots=input.basis.plan.selections.iter().map(|selection|selection.slot.clone()).collect::<Vec<_>>();slots.sort_by(slot_cmp);slots.dedup();
    let mut meals=current_basis.context.requirements.iter().filter(|meal|slots.contains(&meal.slot)).cloned().collect::<Vec<_>>();meals.sort_by(|a,b|slot_cmp(&a.slot,&b.slot));
    let mut requirements=LiveRequirements{revision:input.profile.revision,digest:PayloadDigest("0".repeat(64)),meals,unconfirmed:vec![]};requirements.digest=live_requirements_digest(&requirements)?;
    // derive_material has already refused unconfirmed/ineligible actual diners.
    let current_evidence=input.current_catalogue.food_evidence(&input.derived.recipe_refs.0)?;
    let recipes=current_evidence.recipes.iter().map(|basis|{
        let classes=current_evidence.classes.iter().find(|classes|classes.recipe==basis.recipe).cloned().ok_or_else(||error(ErrorCode::MissingReference,"/calendar/current_classes"))?;
        Ok(LiveRecipeState::Admitted{basis:basis.clone(),classes,methods:current_evidence.methods.iter().filter(|method|method.recipe==basis.recipe).cloned().collect(),resources:current_evidence.resources.iter().filter(|resources|resources.recipe==basis.recipe).cloned().collect()})
    }).collect::<Result<Vec<_>,CalendarFailure>>()?;
    let mut evidence=RelevantEvidenceSnapshot{revision:input.current_catalogue.pin().revision,digest:PayloadDigest("0".repeat(64)),recipes};evidence.digest=relevant_evidence_digest(&evidence)?;
    let mut products=ProductUseSnapshot{revision:Revision(1),digest:PayloadDigest("0".repeat(64)),valid_until:None,evidence:vec![],bindings:vec![],facts:vec![]};products.digest=digest_product_uses(&products)?;
    let mut feasibility=FeasibilityInputs{revision:input.snapshot.snapshot_revision,digest:PayloadDigest("0".repeat(64)),delivery:DeliveryInput::Confirmed{order:order.clone()},products,confirmations:input.essentials.to_vec(),evaluated_at:input.evaluated_at.clone()};feasibility.digest=digest_feasibility_inputs(&feasibility)?;
    let mut sources=vec![];
    for (source,revision,digest) in [
        (LiveSource::Essentials,input.snapshot.snapshot_revision,PayloadDigest("0".repeat(64))),
        (LiveSource::FoodPolicy,input.policy.version,input.policy.digest.clone()),
        (LiveSource::Occupancy,input.occupied.occupancy_epoch,input.occupied.digest.clone()),
        (LiveSource::Order,input.approved.plan_revision,PayloadDigest("0".repeat(64))),
        (LiveSource::ProductEvidence,feasibility.products.revision,feasibility.products.digest.clone()),
        (LiveSource::RecipeEvidence,evidence.revision,evidence.digest.clone()),
        (LiveSource::Requirements,requirements.revision,requirements.digest.clone()),
    ] {sources.push(LiveSourcePin{source,revision,digest,valid_until:None});}
    sources[0].digest=essentials_source_digest(&sources[0],input.essentials)?;sources[3].digest=order_source_digest(&sources[3],order)?;
    let mut current=ContinuationInputs{revision:input.snapshot.snapshot_revision,digest:PayloadDigest("0".repeat(64)),sources,requirements,evidence,policy:input.policy.clone(),occupied:input.occupied.clone(),feasibility};current.digest=continuation_inputs_digest(&current)?;
    let owner=owner_pin(input.basis,input.derived)?;
    let original_phases=if allow_original {mstrchf_core::prep::required_phase_keys(&input.basis.plan,&input.basis.context,&input.basis.evidence).map_err(planner_view::food_error)?}else{vec![]};
    let mut text=vec![];
    for selection in input.basis.plan.selections.iter() {
        let base_packet=packet(input.original_catalogue,&selection.recipe)?;let recipe=base_packet.recipe();
        let production=presentation_servings(selection,allow_original,allow_portable,allow_chilli)?;
        let reviewed=assess_revision(recipe,base_packet.basis(),base_packet.classes())?;
        let method=input.basis.evidence.methods.iter().find(|method|method.recipe==selection.recipe&&method.kind==MethodKind::Production)
            .ok_or_else(||error(ErrorCode::MissingReference,"/calendar/method"))?;
        // Only the exact compiled unit recipes can supply unchanged source text.
        let cold=crate::personal_scope::cold_home_timing(&selection.recipe)?.is_some();
        let original_hot=allow_original && exact_original(&selection.recipe,true,allow_portable);
        let original_cold=allow_original && exact_original(&selection.recipe,false,allow_portable);
        let portable_cold=allow_portable && exact_portable(&selection.recipe,false);
        let chilli_occurrence=allow_chilli && (chilli::exact(&selection.recipe,true) || chilli::exact(&selection.recipe,false));
        let overnight_occurrence=allow_overnight && overnight::exact(&selection.recipe);
        require(if chilli_occurrence {method.prep_ids==recipe.prep.iter().map(|prep|prep.prep_id).collect::<Vec<_>>()}
            else if overnight_occurrence {method.prep_ids==vec![1,2] && recipe.prep.len()==2}
            else if original_hot {method.prep_ids==vec![1,2,3,4,5,6] && recipe.prep.len()==6}
            else if portable_cold {mstrchf_core::portable::method(recipe,&input.basis.evidence)?.is_some()}
            else if original_cold {method.prep_ids==vec![1] && recipe.prep.len()==1 && recipe.ingredients.is_empty() && recipe.reused_inputs.len()==1}
            else if cold {method.prep_ids==vec![1,2] && recipe.prep.len()==2} else {method.prep_ids.len()==1&&recipe.prep.len()==1},ErrorCode::UnsupportedCapability,"/calendar/presentation_method")?;
        // Cooling is absent when all four shares are eaten hot. The native
        // phase closure determines which source instructions actually execute.
        let steps=recipe.prep.iter().filter(|prep|!original_hot && !original_cold && !chilli_occurrence || original_phases.iter().any(|phase|
            phase.occurrence.slot==selection.slot && phase.occurrence.recipe==selection.recipe && phase.prep_id==prep.prep_id))
            .flat_map(|prep|recipe.steps.iter().filter(|step|prep.step_ids.contains(&step.step_id)).map(|step|PresentedStep{prep_id:prep.prep_id,step_id:step.step_id,text:step.text.clone()})).collect();
        text.push(OccurrenceText{owner:owner.clone(),occurrence:OccurrenceKey{slot:selection.slot.clone(),extra_id:None,recipe:selection.recipe.clone()},method:MethodKind::Production,scaled:scale_recipe(&reviewed,production)?,steps,evidence:base_packet.basis().admitted.clone()});
        for extra in selection.extras.iter() {
            // The paired scope has established exactly one unit recipient. The
            // native calendar independently resolves that person and checks the
            // complete occurrence/scaling/step closure before rendering.
            let child=packet(input.original_catalogue,&extra.recipe)?;let recipe=child.recipe();
            let method=input.basis.evidence.methods.iter().find(|method|method.recipe==extra.recipe&&method.kind==MethodKind::Production)
                .ok_or_else(||error(ErrorCode::MissingReference,"/calendar/extra_method"))?;
            require(allow_named_extra && method.prep_ids==vec![1] && recipe.prep.len()==1,
                ErrorCode::UnsupportedCapability,"/calendar/extra_presentation_method")?;
            let reviewed=assess_revision(recipe,child.basis(),child.classes())?;
            let steps=recipe.steps.iter().filter(|step|recipe.prep[0].step_ids.contains(&step.step_id))
                .map(|step|PresentedStep{prep_id:1,step_id:step.step_id,text:step.text.clone()}).collect();
            text.push(OccurrenceText{owner:owner.clone(),occurrence:OccurrenceKey{slot:selection.slot.clone(),extra_id:Some(extra.extra_id.clone()),recipe:extra.recipe.clone()},method:MethodKind::Production,scaled:scale_recipe(&reviewed,extra.recipients[0].servings)?,steps,evidence:child.basis().admitted.clone()});
        }
    }
    let assemblies=input.derived.prep_sessions.iter().map(|session|{
        let member=SessionContribution{owner:owner.clone(),session_id:session.session_id.clone()};
        let first=session.tasks.first().ok_or_else(||error(ErrorCode::MissingReference,"/calendar/task"))?;
        let evidence=packet(input.original_catalogue,&first.recipe)?.basis().admitted.clone();
        Ok(AssemblyEvidence{root:member.clone(),contributors:vec![member],evidence})
    }).collect::<Result<Vec<_>,CalendarFailure>>()?;
    let mut prepared=prepare_events(&CalendarInputs{subject:OwnerInput{historical:&historical,current:&current},linked_owners:&[],assemblies:&assemblies,instruction_text:&text,evaluated_at:input.evaluated_at})?;
    // The mature native profile also renders recipes with unattended phases.
    // Hot personal packets adopt a full-attention estimate; the exact cold
    // packet retains the native active/passive distinction and conditional
    // readiness. Pinned method text and timing values remain unchanged.
    const ORIGINAL:&str="Elapsed envelope includes active and unattended phases; it is not continuous hands-on work.";
    const PERSONAL:&str="This session reserves full attention as an unmeasured estimate. Actual completion is not guaranteed.";
    for draft in prepared.drafts.iter_mut().filter(|draft|draft.logical_key.kind!=CalendarEventKind::Delivery) {
        require(draft.body.0.matches(ORIGINAL).count()==1,ErrorCode::IdentityMismatch,"/calendar/presentation_profile")?;
        if allow_overnight && overnight::presentation(input,draft,ORIGINAL)? {
            shape(&draft.title)?;shape(&draft.body)?;
            continue;
        }
        if allow_chilli && chilli::presentation(input,draft,ORIGINAL)? {
            shape(&draft.title)?;shape(&draft.body)?;
            continue;
        }
        if allow_gf && draft.contributors.iter().any(|member|
            input.derived.prep_sessions.iter().any(|session|session.session_id==member.session_id
                && session.tasks.iter().any(|task|[
                    crate::personal_scope::THREE_BEAN_GF_CHICKEN_SCOPE.recipe_id,
                    crate::personal_scope::CHILLED_GF_CHICKEN_SCOPE.recipe_id,
                ].contains(&task.recipe.recipe_id.0.as_str())))) {
            draft.body.0=format!("Gluten-free recipe conditions: use the exact compatible inputs and prevent gluten cross-contact throughout preparation, storage and service. Actual purchased products and kitchen handling have not been verified. If suitability or separation is unresolved, do not serve this as gluten-free. This is not a wheat-allergy qualification.\n\n{}",draft.body.0);
        }
        if allow_portable && portable_presentation(input,draft,ORIGINAL)? {
            shape(&draft.title)?;shape(&draft.body)?;
            continue;
        }
        if allow_original && original_presentation(input,draft,ORIGINAL) {
            shape(&draft.body)?;
            continue;
        }
        if allow_named_extra && named_extra_presentation(input,draft,ORIGINAL)? {
            shape(&draft.body)?;
            continue;
        }
        let mut cold=None;
        for member in &draft.contributors {for session in input.derived.prep_sessions.iter().filter(|session|session.session_id==member.session_id) {
            for task in &session.tasks.0 {if let Some(timing)=crate::personal_scope::cold_home_timing(&task.recipe)? {cold=Some(timing);}}
        }}
        if let Some(timing)=cold {
            draft.body.0=draft.body.0.replacen(ORIGINAL,&format!("Reserve{}minutes of full attention as an unmeasured estimate, followed by at least{}minutes passive refrigeration. The schedule does not establish temperature or readiness. Keep covered at0–5°C and continue chilling if needed; delay or refuse service until properly chilled. Labels and actual handling still govern.",timing.assembly_minutes,timing.chilling_minutes),1);
        } else {draft.body.0=draft.body.0.replacen(ORIGINAL,PERSONAL,1);}
        shape(&draft.body)?;
    }
    Ok(prepared)
}

fn presentation_servings(selection: &MealSelection, allow_original: bool, allow_portable: bool, allow_chilli: bool) -> Result<PositiveRational,CalendarFailure> {
    match selection.batch_use {
        BatchUse::NewBatch{production_servings,..}=>Ok(production_servings),
        BatchUse::Leftover{..} if allow_original && (exact_original(&selection.recipe,false,allow_portable) || allow_chilli && chilli::exact(&selection.recipe,false))=>{
            let total=selection.servings.iter().try_fold(Rational::new(0,1)?,|total,serving|total.checked_add(&serving.servings.0))?;
            Ok(PositiveRational::new(total.numerator.0,total.denominator.0)?)
        },
        _=>Err(error(ErrorCode::UnsupportedCapability,"/calendar/reuse")),
    }
}

fn original_allocation(view: &RuntimeCatalogue, plan: &WeekPlan, batch: &DerivedBatch,
    component: &DerivedComponent, allocation: &Allocation, allow_portable: bool) -> Result<(),CalendarFailure> {
    require(exact_original(&batch.recipe,true,allow_portable) && allocation.planned_reheats==0
        && component.storage_rule.0.is_some(),ErrorCode::UnsupportedCapability,"/calendar/original_component")?;
    let ProductionMoment::Cooked{completed_at}=&component.original_production else {
        return Err(error(ErrorCode::UnsupportedCapability,"/calendar/original_cooking_clock"));
    };
    if allocation.target_slot==batch.source_slot {
        require(allocation.remix_recipe.0.is_none() && allocation.use_at.instant==completed_at.instant,
            ErrorCode::UnsupportedCapability,"/calendar/original_hot_service")?;
    } else {
        let selected=plan.selections.iter().find(|selected|selected.slot==allocation.target_slot)
            .ok_or_else(||error(ErrorCode::MissingReference,"/calendar/original_cold_service"))?;
        require(exact_original(&selected.recipe,false,allow_portable) && allocation.remix_recipe.0.as_ref()==Some(&selected.recipe),
            ErrorCode::UnsupportedCapability,"/calendar/original_cold_service")?;
        let cold=packet(view,&selected.recipe)?.recipe();
        require(cold.ingredients.is_empty() && cold.reused_inputs.len()==1
            && cold.reused_inputs[0].source_recipe==batch.recipe
            && cold.reused_inputs[0].source_component_id==component.component_id,
            ErrorCode::IdentityMismatch,"/calendar/original_input")?;
        require(plan.batch_links.iter().any(|link|link.batch_id==batch.batch_id
            && link.source_slot==batch.source_slot && link.target_slot==allocation.target_slot && original_link(plan,link,allow_portable)),
            ErrorCode::MissingReference,"/calendar/original_link")?;
    }
    // Native historical verification and continuation still prove exact share
    // conservation, cooling completion, resources and age from this clock.
    Ok(())
}

fn original_presentation(input:&PersonalCalendarInputs<'_>,draft:&mut EventDraft,original:&str)->bool {
    let references=draft.contributors.iter().flat_map(|member|input.derived.prep_sessions.iter()
        .filter(move |session|session.session_id==member.session_id)).flat_map(|session|session.tasks.iter().map(|task|&task.recipe)).collect::<Vec<_>>();
    let hot=references.iter().any(|reference|exact_courgette(reference,true));
    let cold=references.iter().any(|reference|exact_courgette(reference,false));
    if !hot && !cold {return false;}
    let note=if cold {
        "This cold service uses original cooked shares only, with no new ingredient demands. The original cooking-completion clock and expiry remain in force; cold service never restarts them. Timings are unmeasured planning reservations and do not establish temperature or readiness."
    } else {
        "Timings are unmeasured planning reservations. Cooling phases appear only while original shares remain after hot service. The schedule does not establish temperature or readiness; follow the retained cooling and refrigeration instructions."
    };
    draft.body.0=draft.body.0.replacen(original,&format!("{original} {note}"),1);
    true
}

/// Renderer7/8 retain the native per-phase instructions and add only the saved
/// journey's relation. There is no inferred arrival event or temperature proof.
fn portable_presentation(input:&PersonalCalendarInputs<'_>,draft:&mut EventDraft,original:&str)->Result<bool,CalendarFailure> {
    let sessions=draft.contributors.iter().map(|member|input.derived.prep_sessions.iter()
        .find(|session|session.session_id==member.session_id)
        .ok_or_else(||error(ErrorCode::MissingReference,"/calendar/portable_session")))
        .collect::<Result<Vec<_>,_>>()?;
    let hot=sessions.iter().any(|session|session.tasks.iter().any(|task|exact_portable(&task.recipe,true)));
    let cold=sessions.iter().any(|session|session.tasks.iter().any(|task|exact_portable(&task.recipe,false)));
    if !hot && !cold {return Ok(false);}
    require(!(hot&&cold),ErrorCode::UnsupportedCapability,"/calendar/portable_assembly")?;
    if hot {
        let note="Timings are unmeasured planning reservations. Cooling phases appear only while original shares remain after hot service. The schedule does not establish temperature or readiness; follow the retained cooling and refrigeration instructions. A later portable lunch requires its separate saved packing and continuous-chilling declaration.";
        draft.body.0=draft.body.0.replacen(original,&format!("{original} {note}"),1);
        return Ok(true);
    }
    let selected=input.basis.plan.selections.iter().filter(|selection|exact_portable(&selection.recipe,false)
        && sessions.iter().any(|session|session.linked_slots.contains(&selection.slot)
            && session.tasks.iter().any(|task|task.recipe==selection.recipe))).collect::<Vec<_>>();
    require(selected.len()==1 && sessions.len()==1,ErrorCode::UnsupportedCapability,"/calendar/portable_assembly")?;
    let selection=selected[0];
    let journey=selection.portable_cold.as_ref().ok_or_else(||error(ErrorCode::MissingReference,"/calendar/portable_journey"))?;
    let recipe=input.original_catalogue.resolve_recipe(&selection.recipe)?;
    let method=mstrchf_core::portable::method(recipe,&input.basis.evidence)?
        .ok_or_else(||error(ErrorCode::MissingReference,"/calendar/portable_method"))?;
    let tasks=input.derived.prep_sessions.iter().filter(|session|session.linked_slots.contains(&selection.slot))
        .flat_map(|session|session.tasks.iter().filter(|task|task.recipe==selection.recipe).map(move |task|(session,task)))
        .collect::<Vec<_>>();
    require(tasks.len()==2,ErrorCode::IdentityMismatch,"/calendar/portable_tasks")?;
    let packing=tasks.iter().find(|(_,task)|task.prep_id==method.packing_prep_id)
        .ok_or_else(||error(ErrorCode::MissingReference,"/calendar/portable_packing"))?;
    let service=tasks.iter().find(|(_,task)|task.prep_id==method.service_prep_id)
        .ok_or_else(||error(ErrorCode::MissingReference,"/calendar/portable_service"))?;
    require(packing.0.session_id!=service.0.session_id && packing.1.elapsed.end==journey.packing_complete_at
        && journey.depart_at.instant<=service.1.elapsed.start.instant
        && sessions[0].tasks.len()==1,ErrorCode::IdentityMismatch,"/calendar/portable_sessions")?;
    let is_packing=sessions[0].session_id==packing.0.session_id;
    require(is_packing || sessions[0].session_id==service.0.session_id,ErrorCode::IdentityMismatch,"/calendar/portable_session")?;
    let allocations=input.derived.allocations.iter().filter(|allocation|allocation.target_slot==selection.slot
        && allocation.remix_recipe.0.as_ref()==Some(&selection.recipe)).collect::<Vec<_>>();
    require(!allocations.is_empty(),ErrorCode::MissingReference,"/calendar/portable_allocation")?;
    let allocation=allocations[0];
    // Native allocations are per person. All declared shares must retain the
    // same original batch/component, while native scaling proves their total.
    require(allocations.iter().all(|row|row.batch_id==allocation.batch_id
        && row.component_id==allocation.component_id && row.planned_reheats==0),
        ErrorCode::IdentityMismatch,"/calendar/portable_allocation")?;
    let batch=input.derived.batches.iter().find(|batch|batch.batch_id==allocation.batch_id)
        .ok_or_else(||error(ErrorCode::MissingReference,"/calendar/portable_batch"))?;
    let component=batch.components.iter().find(|component|component.component_id==allocation.component_id)
        .ok_or_else(||error(ErrorCode::MissingReference,"/calendar/portable_component"))?;
    let ProductionMoment::Cooked{completed_at}=&component.original_production else {
        return Err(error(ErrorCode::UnsupportedCapability,"/calendar/portable_clock"));
    };
    let rule=component.storage_rule.0.as_ref().ok_or_else(||error(ErrorCode::MissingReference,"/calendar/portable_storage"))?;
    let class=input.basis.evidence.classes.iter().find(|classes|classes.recipe==batch.recipe)
        .and_then(|classes|classes.components.iter().find(|class|class.component_id==component.component_id))
        .ok_or_else(||error(ErrorCode::MissingReference,"/calendar/portable_class"))?;
    let limit=rule.max_chilled_seconds.min(match class.class {
        ComponentClass::General=>input.policy.general_seconds,
        ComponentClass::Rice=>input.policy.rice_seconds,
        _=>return Err(error(ErrorCode::UnsupportedCapability,"/calendar/portable_class")),
    });
    let deadline=chrono::DateTime::parse_from_rfc3339(&completed_at.instant.0)
        .ok().and_then(|at|at.checked_add_signed(chrono::Duration::seconds(i64::from(limit))))
        .ok_or_else(||error(ErrorCode::InvalidValue,"/calendar/portable_deadline"))?;
    let zone=completed_at.zone.0.parse::<chrono_tz::Tz>()
        .map_err(|_|error(ErrorCode::InvalidValue,"/calendar/portable_deadline"))?;
    let deadline=format!("{} ({}, {})",deadline.with_timezone(&zone).format("%Y-%m-%d %H:%M:%S"),
        completed_at.zone.0,deadline.with_timezone(&chrono::Utc).format("%Y-%m-%dT%H:%M:%SZ"));
    let role=if is_packing {"Home packing"}else{"Portable cold service"};
    draft.title=Title(format!("MasterChef · {role}"));
    let note=format!("{role}. This lunch uses original cooked shares only, with no new ingredient demands. Original cooking completion: {}. Original use-by deadline: {deadline}; packing and cold service never restart this clock.\n\nPlanned home packing: {} to {}. Planned departure: {}. Planned cold service: {} to {}.\n\nThe owner declared continuous chilling for this exact saved journey at {}. Use an insulated bag with frozen packs and refrigerate promptly at the destination; keep at 0–5°C until service. No arrival time, bag-only working-day allowance or observed temperature is established. These are conditional plans and unmeasured handling reservations, not confirmation of cooking, cooling, packing or transport. If the required chilling or handling conditions cannot be met or checked, do not use this lunch. Read the separate packing and service events; execute each phase once.",
        readable_moment(completed_at),readable_moment(&packing.1.elapsed.start),readable_moment(&packing.1.elapsed.end),
        readable_moment(&journey.depart_at),readable_moment(&service.1.elapsed.start),readable_moment(&service.1.elapsed.end),
        journey.declaration.confirmed_at.0);
    draft.body.0=draft.body.0.replacen(original,&format!("{original} {note}"),1);
    require(draft.body.0.matches("Produced portions:").count()==1,ErrorCode::IdentityMismatch,"/calendar/portable_portions")?;
    draft.body.0=draft.body.0.replacen("Produced portions:","Original cooked shares:",1);
    Ok(true)
}

fn readable_moment(at:&ZonedMoment)->String {
    format!("{} {} ({}, {})",at.local_date.0,at.local_time.0,at.zone.0,at.instant.0)
}
fn extra_sessions<'a>(input:&'a PersonalCalendarInputs<'_>,selection:&MealSelection)
    ->Result<(&'a PrepSession,&'a PrepSession),CalendarFailure>{
    let child=&selection.extras[0].recipe;
    let bases=input.derived.prep_sessions.iter().filter(|session|session.linked_slots.contains(&selection.slot)
        && session.tasks.iter().any(|task|task.recipe==selection.recipe)).collect::<Vec<_>>();
    let extras=input.derived.prep_sessions.iter().filter(|session|session.linked_slots.contains(&selection.slot)
        && session.tasks.iter().any(|task|&task.recipe==child)).collect::<Vec<_>>();
    require(bases.len()==1 && extras.len()==1 && bases[0].session_id!=extras[0].session_id,
        ErrorCode::IdentityMismatch,"/calendar/extra_sessions")?;
    let (base,extra)=(bases[0],extras[0]);
    require(base.tasks.len()==2 && base.tasks.iter().all(|task|task.recipe==selection.recipe)
        && extra.tasks.len()==1 && extra.tasks[0].recipe==*child && extra.tasks[0].prep_id==1
        && extra.tasks[0].active_intervals.0==vec![extra.tasks[0].elapsed.clone()]
        && extra.start==extra.tasks[0].elapsed.start && extra.end==extra.tasks[0].elapsed.end
        && extra.ingredient_needed_at==extra.start && extra.start.local_date==extra.end.local_date,
        ErrorCode::IdentityMismatch,"/calendar/extra_handling")?;
    require(extra.end.instant.timestamp_seconds()-extra.start.instant.timestamp_seconds()==300
        && base.end==extra.end,ErrorCode::IdentityMismatch,"/calendar/extra_handling")?;
    Ok((base,extra))
}
fn recipient_label(input:&PersonalCalendarInputs<'_>,selection:&MealSelection)->Result<String,CalendarFailure>{
    let person=&selection.extras[0].recipients[0].person;
    // Use precisely the native renderer's complete person ordering. The legend
    // links its DinerN serving label to the actual current saved identity; a
    // temporary guest never inherits a resident label or a different occasion.
    let mut people=input.basis.plan.selections.iter().flat_map(|selected|selected.servings.iter()
        .chain(selected.extras.iter().flat_map(|extra|extra.recipients.iter())).map(|serving|serving.person.clone())).collect::<Vec<_>>();
    people.sort_by(person_cmp);people.dedup();
    let index=people.iter().position(|candidate|candidate==person)
        .ok_or_else(||error(ErrorCode::ForeignReference,"/calendar/extra_recipient"))?+1;
    match person {
        PersonRef::Diner{diner_id}=>{
            let diner=input.profile.diners.iter().find(|diner|&diner.diner_id==diner_id)
                .ok_or_else(||error(ErrorCode::ForeignReference,"/calendar/extra_recipient"))?;
            Ok(match &diner.display_name.0 {Some(name)=>format!("Diner {index} — {} (resident)",name.0),None=>format!("Diner {index} (resident)")})
        },
        PersonRef::Guest{occasion_id,guest_id}=>{
            let occasion=input.week.cells.iter().find(|cell|cell.slot==selection.slot)
                .and_then(|cell|cell.occasion.0.as_ref()).filter(|occasion|&occasion.occasion_id==occasion_id)
                .ok_or_else(||error(ErrorCode::ForeignReference,"/calendar/extra_recipient"))?;
            let guest=occasion.guests.iter().position(|guest|&guest.guest_id==guest_id)
                .ok_or_else(||error(ErrorCode::ForeignReference,"/calendar/extra_recipient"))?+1;
            Ok(format!("Diner {index} — Guest {guest}, {} Lunch (this occasion only)",selection.slot.date.0))
        },
    }
}
/// Both sessions use native identities/instants, and both methods remain in
/// their own native events. No chicken cooking/production/storage clock, batch
/// allocation, safe exposure limit or physical resource observation is invented.
fn named_extra_presentation(input:&PersonalCalendarInputs<'_>,draft:&mut EventDraft,original:&str)->Result<bool,CalendarFailure>{
    let selected=input.basis.plan.selections.iter().filter(|selection|!selection.extras.is_empty())
        .filter(|selection|draft.contributors.iter().any(|member|input.derived.prep_sessions.iter().any(|session|
            session.session_id==member.session_id && session.linked_slots.contains(&selection.slot)
            && session.tasks.iter().any(|task|task.recipe==selection.recipe || task.recipe==selection.extras[0].recipe))))
        .collect::<Vec<_>>();
    if selected.is_empty(){return Ok(false);}
    require(selected.len()==1 && draft.contributors.len()==1,ErrorCode::UnsupportedCapability,"/calendar/extra_assembly")?;
    let selection=selected[0];let (base,extra)=extra_sessions(input,selection)?;let recipient=recipient_label(input,selection)?;
    let is_extra=draft.contributors[0].session_id==extra.session_id;
    require(is_extra || draft.contributors[0].session_id==base.session_id,ErrorCode::IdentityMismatch,"/calendar/extra_assembly")?;
    if is_extra {
        let profile=format!("Named extra recipient: {recipient}; one whole Adult Regular ×1 unit, exactly 50 g edible cooked input. Serve only to this person.\n\nReserve 5 minutes of full attention as an unmeasured counter-handling estimate. Planned retrieval/handling: {} to {}. This is not a cooking or original-product storage clock and never resets product dates, opened-storage or cold-chain requirements. Five minutes is not a validated safe-exposure interval or guaranteed completion. Keep the chicken chilled until retrieval. Fridge access, food scales, dedicated utensils and a separate dish are required. Actual temperature, available space and separation have not been checked. Establish actual ready-to-eat/cold-use/ingredient/date/storage conditions before starting; if unresolved, delay/refuse this extra. Protect both salad units.\n\nLinked base preparation begins {}; read its separate two-phase event. This chicken is excluded from the conditional Vegetarian base; the recipient's combined meal is not Vegetarian.",readable_moment(&extra.start),readable_moment(&extra.end),readable_moment(&base.start));
        draft.body.0=draft.body.0.replacen(original,&profile,1);
    } else {
        // Preserve the existing base active/passive paragraph byte-for-byte;
        // only the new renderer appends its named, separately executed relation.
        let relation=format!("Linked separate extra: {recipient}; exactly 50 g edible cooked chicken, planned handling {} to {}. Read the separate extra event; execute each method once. Chicken never enters either conditional Vegetarian base unit or shares its utensils; the extra recipient's combined meal is not Vegetarian.\n\n",readable_moment(&extra.start),readable_moment(&extra.end));
        draft.body.0=format!("{relation}{}",draft.body.0);
        return Ok(false);
    }
    Ok(true)
}

/// Only explicit native custody states may produce new/retained event versions.
/// Returned bytes do not mark any order, publication or external account state.
pub fn export_personal_calendar(prepared: &PreparedCalendar, update: &CalendarUpdateProjection,
    household: &HouseholdId, revised_at: &Instant) -> Result<ManualCalendarExport,CalendarFailure> {
    shape(household)?;
    let mut fresh=vec![];
    for row in &update.scope {
        if matches!(row.prior,PriorEventState::NeverVersioned) && row.disposition==UpdateDisposition::Upsert {
            fresh.push(CalendarIdentity{key:row.key.clone(),event_id:CalendarEventId(planner_view::stable_id("ce",&serde_json::json!([household,row.key]))?)});
        }
    }
    let snapshot=version_events(prepared,update,&fresh,revised_at)?;
    let ics=render_ics(&snapshot)?;
    Ok(ManualCalendarExport{snapshot,ics,notice:MANUAL_IMPORT_NOTICE})
}
