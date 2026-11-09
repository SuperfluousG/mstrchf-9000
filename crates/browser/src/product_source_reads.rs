//! Pure source-read correlation over the audited saved food owner.
//! Prepared requests must come from the host's independently retained private
//! journal. Hashes bind their bytes, not source truth, a native task, retailer
//! account, current price, human witness, product meaning or permission to act.
//! The host alone supplies the explicit local user witness or operator report and persists acceptance;
//! these functions never mutate state, jobs, approvals, meals or external goods.
use crate::{BrowserState,catalogue::{CataloguePin,RuntimeCatalogue},calendar_view::material_at};
use mstrchf_contracts::*;
use mstrchf_core::{food::{SavedFoodBasis,saved_basis_digest},products::{MatchPolicy,policy_digest,
    prepare_match_context,ProductReadBasis,assess_product_meaning,match_product,MatchAssessment,MatchReason}};
use schemars::JsonSchema;
use serde::{Deserialize,Serialize};
use serde_json::{Value,json};

#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProductSourceTarget {pub product:ProductRef,pub url:HttpUrl}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProductSourceFood {
    pub snapshot_revision:Revision,pub week_revision:Revision,
    pub saved_basis_digest:PayloadDigest,pub derived_digest:PayloadDigest,pub demand_digest:DemandDigest,
    pub catalogue_pin:CataloguePin,pub contains_engineering:bool,
}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreparedProductSourceRead {
    pub format:String,pub version:u8,pub request:JobRequestV1,pub expected_reader:ReadActor,
    pub delivery:RetailerDeliveryContext,pub targets:BoundedVec<ProductSourceTarget,1,8>,
    pub food:ProductSourceFood,pub prepared_digest:Digest,
}
fn error(code:ErrorCode,path:&str)->DomainError {DomainError::new(code,path)}
fn require(ok:bool,code:ErrorCode,path:&str)->Result<(),DomainError>{if ok{Ok(())}else{Err(error(code,path))}}
fn shape(value:&impl ValidateShape)->Result<(),DomainError>{value.validate_shape().map_err(|issues|DomainError{
    code:issues.first().map_or(ErrorCode::InvalidValue,|issue|issue.code),truncated:issues.len()>100,
    issues:BoundedVec(issues.into_iter().take(100).collect()),retry:Retry::Never})}
pub fn prepared_digest(prepared:&PreparedProductSourceRead)->Result<Digest,DomainError>{
    let mut value=serde_json::to_value(prepared).map_err(|_|error(ErrorCode::InvalidValue,"/products/source/prepared"))?;
    value.as_object_mut().ok_or_else(||error(ErrorCode::InvalidValue,"/products/source/prepared"))?.remove("prepared_digest");
    canonical_digest(&value)
}
struct Material {view:RuntimeCatalogue,basis:SavedFoodBasis,derived:DerivedPlan,week_revision:Revision}
fn material(state:&BrowserState,week_id:&WeekId,at:&Instant)->Result<Material,DomainError>{
    let view=crate::runtime_catalogue(state)?;
    let profile=state.snapshot.profile.0.as_ref().ok_or_else(||error(ErrorCode::MissingField,"/products/source/profile"))?;
    let week=state.snapshot.weeks.iter().find(|week|&week.range.week_id==week_id)
        .ok_or_else(||error(ErrorCode::MissingReference,"/products/source/week"))?;
    let (basis,food)=material_at(&view,&state.snapshot,profile,week,at)?
        .ok_or_else(||error(ErrorCode::MissingReference,"/products/source/food"))?;
    require(!food.plan.demand.lines.is_empty(),ErrorCode::MissingReference,"/products/source/food")?;
    Ok(Material{view,basis,derived:food.plan,week_revision:week.revision})
}
fn food(state:&BrowserState,material:&Material)->Result<ProductSourceFood,DomainError>{
    Ok(ProductSourceFood{snapshot_revision:state.snapshot.snapshot_revision,week_revision:material.week_revision,
        saved_basis_digest:saved_basis_digest(&material.basis)?,derived_digest:material.derived.digest.clone(),
        demand_digest:material.derived.demand.digest.clone(),catalogue_pin:material.view.pin().clone(),
        contains_engineering:material.derived.recipe_refs.iter().any(|reference|!material.view.is_personal_recipe(reference))})
}
fn inputs(reader:&ReadActor,key:&BasketKey,delivery:&RetailerDeliveryContext,
    targets:&BoundedVec<ProductSourceTarget,1,8>,now:&Instant,deadline:&Instant)->Result<(),DomainError>{
    shape(reader)?;shape(key)?;shape(delivery)?;shape(targets)?;shape(now)?;shape(deadline)?;
    require(reader.provider==Provider::CodexLocal,ErrorCode::UnsupportedCapability,"/products/source/reader")?;
    require(key.retailer==Retailer::Tesco && delivery.retailer==key.retailer && delivery.account_ref==key.account_ref,
        ErrorCode::IdentityMismatch,"/products/source/delivery")?;
    let seconds=deadline.timestamp_seconds()-now.timestamp_seconds();
    require((1..=3600).contains(&seconds),ErrorCode::InvalidValue,"/products/source/deadline")?;
    for (index,target) in targets.iter().enumerate(){
        require(target.product.retailer==Retailer::Tesco && target.product.variant_id.0.is_none()
            && !target.product.product_id.0.is_empty() && target.product.product_id.0.len()<=32
            && target.product.product_id.0.bytes().all(|byte|byte.is_ascii_digit())
            && target.url.0==format!("https://www.tesco.com/shop/en-GB/products/{}",target.product.product_id.0),
            ErrorCode::ScopeViolation,"/products/source/target")?;
        require(!targets[..index].iter().any(|prior|prior.product==target.product||prior.url==target.url),
            ErrorCode::DuplicateKey,"/products/source/targets")?;
    }
    Ok(())
}
pub fn prepare(state:&BrowserState,week_id:&WeekId,reader:ReadActor,key:BasketKey,delivery:RetailerDeliveryContext,
    targets:BoundedVec<ProductSourceTarget,1,8>,job_id:JobId,read_id:ReadId,now:&Instant,deadline:Instant)
    ->Result<PreparedProductSourceRead,DomainError>{
    inputs(&reader,&key,&delivery,&targets,now,&deadline)?;shape(&job_id)?;shape(&read_id)?;
    let actual=material(state,week_id,now)?;let mut stamp=actual.basis.context.stamp.clone();stamp.write_weeks.0.clear();
    let request=request_for(week_id,actual.basis.plan.revision,stamp,state.snapshot.privacy.revision,key,
        &targets,job_id,read_id,now,deadline)?;
    let mut prepared=PreparedProductSourceRead{format:"mstrchf-product-source-read".into(),version:1,request,
        expected_reader:reader,delivery,targets,food:food(state,&actual)?,prepared_digest:Digest("0".repeat(64))};
    prepared.prepared_digest=prepared_digest(&prepared)?;shape(&prepared)?;Ok(prepared)
}
fn request_for(week_id:&WeekId,plan_revision:Revision,stamp:ContextStamp,privacy_revision:Revision,key:BasketKey,
    targets:&BoundedVec<ProductSourceTarget,1,8>,job_id:JobId,read_id:ReadId,now:&Instant,deadline:Instant)
    ->Result<JobRequestV1,DomainError>{
    let products=BoundedVec(targets.iter().map(|target|target.product.clone()).collect());
    let read_scope=ReadScope{read_id,resource:ExternalResource::Basket{key:key.clone()},purpose:ReadPurpose::Initial,
        allowed_kinds:BoundedVec(vec![ReadKind::Products]),products:products.clone(),event_ids:BoundedVec(vec![]),
        not_before:now.clone(),expires_at:deadline.clone()};
    let mut request=JobRequestV1{schema_version:SchemaVersion,job_id,action:Action::PrepareShop,
        week_id:Nullable(Some(week_id.clone())),base_plan_revision:Nullable(Some(plan_revision)),
        context_stamp:stamp,privacy_policy_revision:privacy_revision,
        input_digest:InputDigest("0".repeat(64)),required_capabilities:BoundedVec(vec![Capability::BrowserRead]),
        allowed_artefacts:BoundedVec(vec![]),payload:JobPayload::ProductObservation{read_scope:read_scope.clone(),products,approved_contribution:Nullable(None)},
        scope:ActionScope::Shop{key,phase:ShopPhase::Observe,approval_id:Nullable(None),read_scope},created_at:now.clone(),expires_at:deadline};
    request.input_digest=input_digest(&RequestMaterial::from(&request))?;shape(&request)?;validate_request_shape(&request)?;
    Ok(request)
}
fn internal_prepared(prepared:&PreparedProductSourceRead)->Result<(),DomainError>{
    shape(prepared)?;
    require(prepared.format=="mstrchf-product-source-read" && prepared.version==1,ErrorCode::UnsupportedVersion,"/products/source/prepared")?;
    require(prepared_digest(prepared)?==prepared.prepared_digest,ErrorCode::DigestMismatch,"/products/source/prepared_digest")?;
    let request=&prepared.request;validate_request_shape(request)?;
    let ActionScope::Shop{key,read_scope,..}=&request.scope else{return Err(error(ErrorCode::ScopeViolation,"/products/source/request"));};
    inputs(&prepared.expected_reader,key,&prepared.delivery,&prepared.targets,&request.created_at,&request.expires_at)?;
    let week_id=request.week_id.0.as_ref().ok_or_else(||error(ErrorCode::MissingReference,"/products/source/week"))?;
    let plan_revision=request.base_plan_revision.0.ok_or_else(||error(ErrorCode::MissingReference,"/products/source/plan"))?;
    require(request.context_stamp.read_weeks.iter().any(|week|&week.week_id==week_id&&week.revision==prepared.food.week_revision)
        && request.context_stamp.privacy_revision==request.privacy_policy_revision
        && request.context_stamp.catalogue_revision==prepared.food.catalogue_pin.revision,
        ErrorCode::IdentityMismatch,"/products/source/food")?;
    let mut stamp=request.context_stamp.clone();stamp.write_weeks.0.clear();
    let expected=request_for(week_id,plan_revision,stamp,request.privacy_policy_revision,key.clone(),&prepared.targets,
        request.job_id.clone(),read_scope.read_id.clone(),&request.created_at,request.expires_at.clone())?;
    require(expected==*request,ErrorCode::ScopeViolation,"/products/source/request")
}
fn current(state:&BrowserState,prepared:&PreparedProductSourceRead,now:&Instant)->Result<Material,DomainError>{
    current_scope(state,prepared,now,true)
}
fn current_scope(state:&BrowserState,prepared:&PreparedProductSourceRead,now:&Instant,require_ai:bool)->Result<Material,DomainError>{
    internal_prepared(prepared)?;shape(now)?;
    require(prepared.request.privacy_policy_revision==state.snapshot.privacy.revision,
        ErrorCode::ContextStale,"/products/source/privacy")?;
    if require_ai {require(state.snapshot.privacy.use_ai_features,ErrorCode::AuthorityDenied,"/products/source/ai_features")?;}
    require(now>=&prepared.request.created_at && now<&prepared.request.expires_at,ErrorCode::ContextStale,"/products/source/deadline")?;
    let ActionScope::Shop{key,..}=&prepared.request.scope else{return Err(error(ErrorCode::ScopeViolation,"/products/source/request"));};
    let week_id=prepared.request.week_id.0.as_ref().ok_or_else(||error(ErrorCode::MissingReference,"/products/source/week"))?;
    let read_scope=match &prepared.request.scope{ActionScope::Shop{read_scope,..}=>read_scope,_=>unreachable!()};
    // Reconstruct the complete constrained request. A re-signed transport cannot
    // add Basket reads, approvals, contributions, capabilities or write weeks.
    let rebuilt=prepare(state,week_id,prepared.expected_reader.clone(),key.clone(),prepared.delivery.clone(),
        prepared.targets.clone(),prepared.request.job_id.clone(),read_scope.read_id.clone(),
        &prepared.request.created_at,prepared.request.expires_at.clone())?;
    require(&rebuilt==prepared,ErrorCode::ContextStale,"/products/source/current")?;
    // Also reconstruct at the actual check/witness clock; the original saved
    // material never receives authority from a projected or imported DTO.
    let actual=material(state,week_id,now)?;
    require(food(state,&actual)?==prepared.food,ErrorCode::ContextStale,"/products/source/current")?;
    Ok(actual)
}
/// Correlation proof for an owner-private hypothetical draft only. This creates
/// no user-witnessed assessment or persisted acceptance. The owner must retain
/// the independently selected request/reader and never treat this as truth.
pub(crate) fn unwitnessed_correlation(state:&BrowserState,prepared:&PreparedProductSourceRead,observation:&ReadObservationV1,now:&Instant)
    ->Result<AcceptedReadRef,DomainError>{
    current_scope(state,prepared,now,false)?;
    let digest=checked_observation(prepared,observation,now)?;
    let identity=canonical_digest(&json!({"format":"mstrchf-unwitnessed-source-correlation","observation_digest":digest}))?;
    let accepted=AcceptedReadRef{evidence_id:EvidenceId(format!("ev_{}",&identity.0[..32])),job_id:observation.job_id.clone(),
        read_id:observation.read_id.clone(),observation_id:observation.observation_id.clone(),digest,
        resource:observation.resource.clone(),observed_at:observation.observed_at.clone(),admitted_at:now.clone()};
    shape(&accepted)?;Ok(accepted)
}
pub fn inspect(state:&BrowserState,prepared:&PreparedProductSourceRead,now:&Instant)->Result<Value,DomainError>{
    current(state,prepared,now)?;Ok(json!({"current":true}))
}
fn checked(state:&BrowserState,prepared:&PreparedProductSourceRead,observation:&ReadObservationV1,now:&Instant)
    ->Result<(Material,Digest),DomainError>{
    let actual=current(state,prepared,now)?;let digest=checked_observation(prepared,observation,now)?;Ok((actual,digest))
}
fn checked_observation(prepared:&PreparedProductSourceRead,observation:&ReadObservationV1,at:&Instant)->Result<Digest,DomainError>{
    shape(observation)?;shape(at)?;validate_read_binding(observation,&prepared.request,&prepared.expected_reader,at.clone())?;
    let ReadBody::Products{delivery_context,items}=&observation.body else{return Err(error(ErrorCode::ScopeViolation,"/products/source/body"));};
    require(delivery_context==&prepared.delivery,ErrorCode::IdentityMismatch,"/products/source/delivery")?;
    require(items.len()==prepared.targets.len(),ErrorCode::MissingReference,"/products/source/items")?;
    require(observation.observed_at>prepared.request.created_at && observation.observed_at<=*at
        && at<&prepared.request.expires_at,
        ErrorCode::InvalidValue,"/products/source/observed_at")?;
    for item in items.iter(){
        require(prepared.targets.iter().any(|target|target.product==item.product && target.url==item.url),
            ErrorCode::ScopeViolation,"/products/source/target")?;
        require(item.observed_at>prepared.request.created_at && item.observed_at<=observation.observed_at
            && item.observed_at<prepared.request.expires_at,ErrorCode::InvalidValue,"/products/source/item_observed_at")?;
    }
    canonical_digest(observation)
}
/// Inactive-history custody only. This deliberately has no current state/time,
/// privacy authorisation or food/source-truth claim and cannot release a job.
pub fn audit(prepared:&PreparedProductSourceRead,observation:Option<&ReadObservationV1>,accepted:Option<&AcceptedReadRef>)
    ->Result<Value,DomainError>{
    internal_prepared(prepared)?;
    match (observation,accepted){
        (None,None)=>{},
        (None,Some(_))=>return Err(error(ErrorCode::MissingReference,"/products/source/observation")),
        (Some(observation),accepted)=>{
            let at=accepted.map_or(&observation.observed_at,|receipt|&receipt.admitted_at);
            let digest=checked_observation(prepared,observation,at)?;
            if let Some(receipt)=accepted{
                shape(receipt)?;
                let expected=AcceptedReadRef{evidence_id:receipt.evidence_id.clone(),job_id:observation.job_id.clone(),
                    read_id:observation.read_id.clone(),observation_id:observation.observation_id.clone(),digest,
                    resource:observation.resource.clone(),observed_at:observation.observed_at.clone(),admitted_at:receipt.admitted_at.clone()};
                require(receipt==&expected,ErrorCode::IdentityMismatch,"/products/source/accepted")?;
            }
        }
    }
    Ok(json!({"valid":true,"observation_digest":observation.map(canonical_digest).transpose()?}))
}
pub fn check_return(state:&BrowserState,prepared:&PreparedProductSourceRead,observation:&ReadObservationV1,now:&Instant)
    ->Result<Value,DomainError>{let(_,digest)=checked(state,prepared,observation,now)?;Ok(json!({"observation_digest":digest}))}
pub fn accept(state:&BrowserState,prepared:&PreparedProductSourceRead,observation:&ReadObservationV1,evidence_id:EvidenceId,now:&Instant)
    ->Result<Value,DomainError>{
    shape(&evidence_id)?;let(actual,digest)=checked(state,prepared,observation,now)?;
    let accepted=AcceptedReadRef{evidence_id,job_id:observation.job_id.clone(),read_id:observation.read_id.clone(),
        observation_id:observation.observation_id.clone(),digest,resource:observation.resource.clone(),
        observed_at:observation.observed_at.clone(),admitted_at:now.clone()};shape(&accepted)?;
    // Exercise actual native read admission + matching with NO meaning and NO
    // meaning admission. The first real demand line only proves the unresolved
    // consumer route: no source product is mapped to that ingredient here.
    let mut policy=MatchPolicy{version:Revision(1),digest:PayloadDigest("0".repeat(64)),rules:vec![]};policy.digest=policy_digest(&policy)?;
    let context=prepare_match_context(&actual.basis.context,&actual.basis,&actual.derived,&policy,now.clone())?;
    let mut products=Vec::new();
    for target in prepared.targets.iter(){
        let source=ProductReadBasis{request:&prepared.request,expected_reader:&prepared.expected_reader,observation,
            accepted:&accepted,product:target.product.clone()};
        let reviewed=assess_product_meaning(&source,None,None,&context)?;
        let assessment=match_product(&context,&actual.derived.demand.lines[0].line_id,&reviewed)?;
        require(matches!(assessment,MatchAssessment::Unresolved(ref issues) if !issues.is_empty()&&issues.iter().all(|issue|issue.reason==MatchReason::MissingMeaning)),
            ErrorCode::IdentityMismatch,"/products/source/missing_meaning")?;
        products.push(json!({"product":target.product,"status":"unresolved","native_reasons":["missing_meaning"]}));
    }
    Ok(json!({"accepted":accepted,"assessment":{"scope":"source_correlation_only","provenance":"user_witnessed",
        "host_attested":false,"source_truth_assessed":false,"meaning_status":"missing","admission_status":"missing",
        "native_matching":"unresolved","quote_status":"unknown","retailer_status":"unqualified",
        "account_verified":false,"baseline_status":"unknown","effect_enabled":false,"products":products}}))
}
/// A reported check by the exact prepared Codex reader. This is neither a
/// human witness nor host attestation and grants no product/effect authority.
pub fn accept_operator(state:&BrowserState,prepared:&PreparedProductSourceRead,observation:&ReadObservationV1,
    evidence_id:EvidenceId,operator_task_ref:&Text,now:&Instant)->Result<Value,DomainError>{
    shape(operator_task_ref)?;
    require(prepared.expected_reader.provider==Provider::CodexLocal
        && &prepared.expected_reader.native_task_ref==operator_task_ref,
        ErrorCode::IdentityMismatch,"/products/source/operator")?;
    // Keep the existing correlation, state/privacy/time and missing-meaning
    // calculation identical; change only the native receipt's explicit actor.
    let mut result=accept(state,prepared,observation,evidence_id,now)?;
    result["assessment"]["provenance"]=json!("codex_operator_reported");
    result["assessment"]["operator"]=json!(prepared.expected_reader);
    result["assessment"]["human_review_recorded"]=json!(false);
    Ok(result)
}
