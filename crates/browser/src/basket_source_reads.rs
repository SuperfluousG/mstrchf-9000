//! Basket-only source correlation. The host retains prepared requests locally
//! and supplies the real witness; hashes and aliases establish neither retailer
//! identity, completeness/truth, price currentness nor controlled-basket authority.
//! These pure functions never change jobs, meals, history or external goods.
use crate::{BrowserState,calendar_view::material_at,product_source_reads::ProductSourceFood};
use mstrchf_contracts::*;
use mstrchf_core::food::saved_basis_digest;
use schemars::JsonSchema;
use serde::{Deserialize,Serialize};
use serde_json::{Value,json};

macro_rules! record {($name:ident {$($field:ident:$ty:ty),* $(,)?}) => {
    #[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct $name {$(pub $field:$ty),*}
};}
record!(PreparedBasketSourceRead {format:String,version:u8,request:JobRequestV1,
    expected_reader:ReadActor,delivery:RetailerDeliveryContext,food:ProductSourceFood,prepared_digest:Digest});
record!(BasketSourcePrice {pence:DecimalU64,currency:Currency,label:String});
record!(BasketSourceLine {line_ref:String,product:Option<ProductRef>,title:String,
    quantity:Fact<BoundedCount>,unit_price:Fact<BasketSourcePrice>});
#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(rename_all="snake_case")]
pub enum BasketCompleteness {Complete,Incomplete}
record!(PublicBasketSourceReturn {format:String,version:u8,read_id:ReadId,job_id:JobId,
    input_digest:InputDigest,observed_at:Instant,completeness:BasketCompleteness,
    observed_lines:Vec<BasketSourceLine>,delivery_charge:Fact<BasketSourcePrice>,notes:Vec<String>});
#[derive(Clone,Debug,PartialEq,Eq,Serialize,JsonSchema)]
pub struct BasketReturnCheck {pub payload_digest:Digest,pub observation:Option<ReadObservationV1>,pub observation_digest:Option<Digest>}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,JsonSchema)]
pub struct BasketSourceAudit {pub valid:bool,pub payload_digest:Option<Digest>,pub observation_digest:Option<Digest>}

fn error(code:ErrorCode,path:&str)->DomainError{DomainError::new(code,path)}
fn require(ok:bool,code:ErrorCode,path:&str)->Result<(),DomainError>{if ok{Ok(())}else{Err(error(code,path))}}
fn shape(v:&impl ValidateShape)->Result<(),DomainError>{v.validate_shape().map_err(|_|error(ErrorCode::InvalidValue,"/basket/source/shape"))}
fn bounded(v:&str,max:usize,nonempty:bool,path:&str)->Result<(),DomainError>{require(v.len()<=max&&(!nonempty||!v.trim().is_empty()),ErrorCode::InvalidValue,path)}
fn fact<T:PartialEq>(v:&Fact<T>,check:impl Fn(&T)->Result<(),DomainError>,path:&str)->Result<(),DomainError>{
    match v{
        Fact::Known{value}=>check(value),Fact::Unknown{..}=>Ok(()),
        Fact::Conflicting{values}=>{require((2..=8).contains(&values.len()),ErrorCode::LimitExceeded,path)?;
            for(index,value)in values.iter().enumerate(){require(!values[..index].contains(value),ErrorCode::DuplicateKey,path)?;check(value)?;}Ok(())},
    }
}
fn price(v:&BasketSourcePrice)->Result<(),DomainError>{
    require(v.pence.0<=1_000_000_000_000,ErrorCode::LimitExceeded,"/basket/source/price")?;
    bounded(&v.label,4096,true,"/basket/source/price_label")
}
fn product(v:&ProductRef)->Result<(),DomainError>{
    shape(v)?;require(v.retailer==Retailer::Tesco&&v.variant_id.0.is_none()
        &&!v.product_id.0.is_empty()&&v.product_id.0.len()<=32&&v.product_id.0.bytes().all(|b|b.is_ascii_digit()),ErrorCode::ScopeViolation,"/basket/source/product")
}
fn returned_shape(v:&PublicBasketSourceReturn)->Result<(),DomainError>{
    require(v.format=="mstrchf-basket-source-return"&&v.version==1,ErrorCode::UnsupportedVersion,"/basket/source/return")?;
    shape(&v.read_id)?;shape(&v.job_id)?;shape(&v.input_digest)?;shape(&v.observed_at)?;
    require(v.observed_lines.len()<=256&&v.notes.len()<=16,ErrorCode::LimitExceeded,"/basket/source/return_bounds")?;
    require(serde_json::to_vec(v).map_err(|_|error(ErrorCode::InvalidValue,"/basket/source/encoding"))?.len()<=2_000_000,ErrorCode::LimitExceeded,"/basket/source/return_bytes")?;
    for(index,line)in v.observed_lines.iter().enumerate(){
        bounded(&line.line_ref,80,true,"/basket/source/line_ref")?;bounded(&line.title,1024,true,"/basket/source/title")?;
        require(!v.observed_lines[..index].iter().any(|prior|prior.line_ref==line.line_ref),ErrorCode::DuplicateKey,"/basket/source/line_ref")?;
        if let Some(p)=&line.product{product(p)?;}
        fact(&line.quantity,|count|require((1..=10000).contains(&count.0),ErrorCode::LimitExceeded,"/basket/source/quantity"),"/basket/source/quantity")?;
        fact(&line.unit_price,price,"/basket/source/price")?;
        if v.completeness==BasketCompleteness::Complete{
            require(line.product.is_some(),ErrorCode::MissingReference,"/basket/source/complete/product")?;
            require(matches!(line.quantity,Fact::Known{..}),ErrorCode::MissingField,"/basket/source/complete/quantity")?;
            require(!v.observed_lines[..index].iter().any(|prior|prior.product==line.product),ErrorCode::DuplicateKey,"/basket/source/complete/product")?;
        }
    }
    fact(&v.delivery_charge,price,"/basket/source/delivery_charge")?;
    for note in &v.notes{bounded(note,1024,true,"/basket/source/notes")?;}
    Ok(())
}
pub fn prepared_digest(v:&PreparedBasketSourceRead)->Result<Digest,DomainError>{
    let mut encoded=serde_json::to_value(v).map_err(|_|error(ErrorCode::InvalidValue,"/basket/source/prepared"))?;
    encoded.as_object_mut().ok_or_else(||error(ErrorCode::InvalidValue,"/basket/source/prepared"))?.remove("prepared_digest");canonical_digest(&encoded)
}
fn material(state:&BrowserState,week_id:&WeekId,at:&Instant)->Result<(ProductSourceFood,ContextStamp,Revision),DomainError>{
    let view=crate::runtime_catalogue(state)?;
    let profile=state.snapshot.profile.0.as_ref().ok_or_else(||error(ErrorCode::MissingField,"/basket/source/profile"))?;
    let week=state.snapshot.weeks.iter().find(|w|&w.range.week_id==week_id).ok_or_else(||error(ErrorCode::MissingReference,"/basket/source/week"))?;
    let(basis,food)=material_at(&view,&state.snapshot,profile,week,at)?.ok_or_else(||error(ErrorCode::MissingReference,"/basket/source/food"))?;
    require(!food.plan.demand.lines.is_empty(),ErrorCode::MissingReference,"/basket/source/food")?;
    Ok((ProductSourceFood{snapshot_revision:state.snapshot.snapshot_revision,week_revision:week.revision,
        saved_basis_digest:saved_basis_digest(&basis)?,derived_digest:food.plan.digest.clone(),demand_digest:food.plan.demand.digest.clone(),
        catalogue_pin:view.pin().clone(),contains_engineering:food.plan.recipe_refs.iter().any(|r|!view.is_personal_recipe(r))},basis.context.stamp,basis.plan.revision))
}
fn inputs(reader:&ReadActor,key:&BasketKey,delivery:&RetailerDeliveryContext,at:&Instant,deadline:&Instant)->Result<(),DomainError>{
    shape(reader)?;shape(key)?;shape(delivery)?;shape(at)?;shape(deadline)?;
    require(reader.provider==Provider::CodexLocal,ErrorCode::UnsupportedCapability,"/basket/source/reader")?;
    require(key.retailer==Retailer::Tesco&&delivery.retailer==key.retailer&&delivery.account_ref==key.account_ref,ErrorCode::IdentityMismatch,"/basket/source/delivery")?;
    require((1..=3600).contains(&(deadline.timestamp_seconds()-at.timestamp_seconds())),ErrorCode::InvalidValue,"/basket/source/deadline")
}
fn request_for(week_id:&WeekId,plan_revision:Revision,mut stamp:ContextStamp,privacy_revision:Revision,key:BasketKey,
    job_id:JobId,read_id:ReadId,at:&Instant,deadline:Instant)->Result<JobRequestV1,DomainError>{
    stamp.write_weeks.0.clear();
    let scope=ReadScope{read_id,resource:ExternalResource::Basket{key:key.clone()},purpose:ReadPurpose::Initial,
        allowed_kinds:BoundedVec(vec![ReadKind::Basket]),products:BoundedVec(vec![]),event_ids:BoundedVec(vec![]),not_before:at.clone(),expires_at:deadline.clone()};
    let mut request=JobRequestV1{schema_version:SchemaVersion,job_id,action:Action::PrepareShop,week_id:Nullable(Some(week_id.clone())),base_plan_revision:Nullable(Some(plan_revision)),
        context_stamp:stamp,privacy_policy_revision:privacy_revision,input_digest:InputDigest("0".repeat(64)),required_capabilities:BoundedVec(vec![Capability::BrowserRead]),allowed_artefacts:BoundedVec(vec![]),
        payload:JobPayload::ProductObservation{read_scope:scope.clone(),products:BoundedVec(vec![]),approved_contribution:Nullable(None)},
        scope:ActionScope::Shop{key,phase:ShopPhase::Observe,approval_id:Nullable(None),read_scope:scope},created_at:at.clone(),expires_at:deadline};
    request.input_digest=input_digest(&RequestMaterial::from(&request))?;shape(&request)?;validate_request_shape(&request)?;Ok(request)
}
pub fn prepare(state:&BrowserState,week_id:&WeekId,reader:ReadActor,key:BasketKey,delivery:RetailerDeliveryContext,
    job_id:JobId,read_id:ReadId,now:&Instant,deadline:Instant)->Result<PreparedBasketSourceRead,DomainError>{
    inputs(&reader,&key,&delivery,now,&deadline)?;shape(week_id)?;shape(&job_id)?;shape(&read_id)?;
    let(food,stamp,revision)=material(state,week_id,now)?;
    let request=request_for(week_id,revision,stamp,state.snapshot.privacy.revision,key,job_id,read_id,now,deadline)?;
    let mut prepared=PreparedBasketSourceRead{format:"mstrchf-basket-source-read".into(),version:1,request,expected_reader:reader,delivery,food,prepared_digest:Digest("0".repeat(64))};
    prepared.prepared_digest=prepared_digest(&prepared)?;Ok(prepared)
}
fn internal(prepared:&PreparedBasketSourceRead)->Result<(),DomainError>{
    require(prepared.format=="mstrchf-basket-source-read"&&prepared.version==1,ErrorCode::UnsupportedVersion,"/basket/source/prepared")?;
    shape(&prepared.prepared_digest)?;require(prepared.prepared_digest==prepared_digest(prepared)?,ErrorCode::DigestMismatch,"/basket/source/prepared_digest")?;
    let request=&prepared.request;shape(request)?;validate_request_shape(request)?;
    let ActionScope::Shop{key,read_scope,..}=&request.scope else{return Err(error(ErrorCode::ScopeViolation,"/basket/source/request"))};
    inputs(&prepared.expected_reader,key,&prepared.delivery,&request.created_at,&request.expires_at)?;
    let week_id=request.week_id.0.as_ref().ok_or_else(||error(ErrorCode::MissingReference,"/basket/source/week"))?;
    let revision=request.base_plan_revision.0.ok_or_else(||error(ErrorCode::MissingReference,"/basket/source/plan"))?;
    shape(&prepared.delivery)?;shape(&prepared.food.snapshot_revision)?;shape(&prepared.food.week_revision)?;shape(&prepared.food.saved_basis_digest)?;shape(&prepared.food.derived_digest)?;shape(&prepared.food.demand_digest)?;
    require(request.context_stamp.read_weeks.iter().any(|w|&w.week_id==week_id&&w.revision==prepared.food.week_revision)
        &&request.context_stamp.privacy_revision==request.privacy_policy_revision&&request.context_stamp.catalogue_revision==prepared.food.catalogue_pin.revision,ErrorCode::IdentityMismatch,"/basket/source/food")?;
    let expected=request_for(week_id,revision,request.context_stamp.clone(),request.privacy_policy_revision,key.clone(),request.job_id.clone(),read_scope.read_id.clone(),&request.created_at,request.expires_at.clone())?;
    require(request==&expected,ErrorCode::ScopeViolation,"/basket/source/request")
}
// Local capture records a separately commissioned read. It cannot establish
// that commissioning, start a reader, change privacy or qualify an account.
// The owner retains the mode and supplies the real explicit witness.
#[derive(Clone,Copy)]
enum SourceAccess {Assistant,ExternalCapture}
fn current(state:&BrowserState,prepared:&PreparedBasketSourceRead,at:&Instant,access:SourceAccess)->Result<(),DomainError>{
    internal(prepared)?;shape(at)?;
    require(state.snapshot.privacy.revision==prepared.request.privacy_policy_revision,ErrorCode::ContextStale,"/basket/source/privacy")?;
    if matches!(access,SourceAccess::Assistant){require(state.snapshot.privacy.use_ai_features,ErrorCode::AuthorityDenied,"/basket/source/ai_features")?;}
    require(prepared.request.created_at<=*at&&*at<prepared.request.expires_at,ErrorCode::ContextStale,"/basket/source/deadline")?;
    let ActionScope::Shop{key,read_scope,..}=&prepared.request.scope else{unreachable!()};let week=prepared.request.week_id.0.as_ref().ok_or_else(||error(ErrorCode::MissingReference,"/basket/source/week"))?;
    let rebuilt=prepare(state,week,prepared.expected_reader.clone(),key.clone(),prepared.delivery.clone(),prepared.request.job_id.clone(),read_scope.read_id.clone(),&prepared.request.created_at,prepared.request.expires_at.clone())?;
    require(&rebuilt==prepared&&material(state,week,at)?.0==prepared.food,ErrorCode::ContextStale,"/basket/source/current")
}
pub fn inspect(state:&BrowserState,prepared:&PreparedBasketSourceRead,now:&Instant)->Result<Value,DomainError>{current(state,prepared,now,SourceAccess::Assistant)?;Ok(json!({"current":true}))}
pub fn local_inspect(state:&BrowserState,prepared:&PreparedBasketSourceRead,now:&Instant)->Result<Value,DomainError>{current(state,prepared,now,SourceAccess::ExternalCapture)?;Ok(json!({"current":true}))}
fn correlate(prepared:&PreparedBasketSourceRead,returned:&PublicBasketSourceReturn,at:&Instant)->Result<(),DomainError>{
    returned_shape(returned)?;shape(at)?;
    let ActionScope::Shop{read_scope,..}=&prepared.request.scope else{unreachable!()};
    require(returned.read_id==read_scope.read_id&&returned.job_id==prepared.request.job_id,ErrorCode::IdentityMismatch,"/basket/source/read")?;
    require(returned.input_digest==prepared.request.input_digest,ErrorCode::DigestMismatch,"/basket/source/input_digest")?;
    require(prepared.request.created_at<returned.observed_at&&returned.observed_at<=*at&&*at<prepared.request.expires_at,ErrorCode::InvalidValue,"/basket/source/observed_at")
}
fn observed_price(fact:&Fact<BasketSourcePrice>)->Price{match fact{
    // The declared field/label role is witnessed by the host; this conversion
    // does not independently verify role, currentness or checkout applicability.
    Fact::Known{value}=>Price::Observed{pence:value.pence,currency:value.currency},
    Fact::Unknown{reason}=>Price::Unknown{reason:*reason},
    Fact::Conflicting{..}=>Price::Unknown{reason:UnknownReason::Unreadable},
}}
fn observation(prepared:&PreparedBasketSourceRead,returned:&PublicBasketSourceReturn,id:&ReadObservationId)->Result<ReadObservationV1,DomainError>{
    shape(id)?;
    require(returned.completeness==BasketCompleteness::Complete,ErrorCode::MissingField,"/basket/source/completeness")?;
    let ActionScope::Shop{key,read_scope,..}=&prepared.request.scope else{unreachable!()};
    let lines=returned.observed_lines.iter().map(|line|{
        let Fact::Known{value}=&line.quantity else{return Err(error(ErrorCode::MissingField,"/basket/source/quantity"))};
        Ok(BasketLine{product:line.product.clone().ok_or_else(||error(ErrorCode::MissingReference,"/basket/source/product"))?,quantity:u32::from(value.0),unit_price:observed_price(&line.unit_price)})
    }).collect::<Result<Vec<_>,DomainError>>()?;
    Ok(ReadObservationV1{schema_version:SchemaVersion,observation_id:id.clone(),read_id:returned.read_id.clone(),job_id:returned.job_id.clone(),input_digest:returned.input_digest.clone(),
        reader:prepared.expected_reader.clone(),resource:read_scope.resource.clone(),observed_at:returned.observed_at.clone(),
        body:ReadBody::Basket{snapshot:BasketSnapshot{key:key.clone(),delivery_context:prepared.delivery.clone(),observed_at:returned.observed_at.clone(),lines:BoundedVec(lines),delivery_charge:observed_price(&returned.delivery_charge)}}})
}
fn checked_observation(prepared:&PreparedBasketSourceRead,returned:&PublicBasketSourceReturn,observed:&ReadObservationV1,at:&Instant)->Result<Digest,DomainError>{
    correlate(prepared,returned,at)?;shape(observed)?;
    require(&observation(prepared,returned,&observed.observation_id)?==observed,ErrorCode::IdentityMismatch,"/basket/source/observation")?;
    validate_read_binding(observed,&prepared.request,&prepared.expected_reader,at.clone())?;canonical_digest(observed)
}
fn check_return_with_access(state:&BrowserState,prepared:&PreparedBasketSourceRead,returned:&PublicBasketSourceReturn,id:ReadObservationId,now:&Instant,access:SourceAccess)->Result<BasketReturnCheck,DomainError>{
    current(state,prepared,now,access)?;correlate(prepared,returned,now)?;shape(&id)?;
    let payload_digest=canonical_digest(returned)?;
    if returned.completeness==BasketCompleteness::Incomplete{return Ok(BasketReturnCheck{payload_digest,observation:None,observation_digest:None});}
    let observed=observation(prepared,returned,&id)?;let digest=checked_observation(prepared,returned,&observed,now)?;
    Ok(BasketReturnCheck{payload_digest,observation:Some(observed),observation_digest:Some(digest)})
}
pub fn check_return(state:&BrowserState,prepared:&PreparedBasketSourceRead,returned:&PublicBasketSourceReturn,id:ReadObservationId,now:&Instant)->Result<BasketReturnCheck,DomainError>{check_return_with_access(state,prepared,returned,id,now,SourceAccess::Assistant)}
pub fn local_check_return(state:&BrowserState,prepared:&PreparedBasketSourceRead,returned:&PublicBasketSourceReturn,id:ReadObservationId,now:&Instant)->Result<BasketReturnCheck,DomainError>{check_return_with_access(state,prepared,returned,id,now,SourceAccess::ExternalCapture)}
fn accept_with_access(state:&BrowserState,prepared:&PreparedBasketSourceRead,returned:&PublicBasketSourceReturn,observed:&ReadObservationV1,evidence_id:EvidenceId,now:&Instant,access:SourceAccess)->Result<Value,DomainError>{
    current(state,prepared,now,access)?;shape(&evidence_id)?;let digest=checked_observation(prepared,returned,observed,now)?;
    let accepted=AcceptedReadRef{evidence_id,job_id:observed.job_id.clone(),read_id:observed.read_id.clone(),observation_id:observed.observation_id.clone(),digest,resource:observed.resource.clone(),observed_at:observed.observed_at.clone(),admitted_at:now.clone()};
    shape(&accepted)?;Ok(json!({"accepted":accepted,"assessment":{"source_truth_assessed":false,"account_verified":false,"execution_baseline_status":"unqualified","effect_enabled":false,"price_currentness":"unknown"}}))
}
pub fn accept(state:&BrowserState,prepared:&PreparedBasketSourceRead,returned:&PublicBasketSourceReturn,observed:&ReadObservationV1,evidence_id:EvidenceId,now:&Instant)->Result<Value,DomainError>{accept_with_access(state,prepared,returned,observed,evidence_id,now,SourceAccess::Assistant)}
pub fn local_accept(state:&BrowserState,prepared:&PreparedBasketSourceRead,returned:&PublicBasketSourceReturn,observed:&ReadObservationV1,evidence_id:EvidenceId,now:&Instant)->Result<Value,DomainError>{accept_with_access(state,prepared,returned,observed,evidence_id,now,SourceAccess::ExternalCapture)}
/// Inactive historical custody only. Owner comparison of the returned payload
/// digest also binds sidecar title/label/completeness bytes not in native reads.
/// Imported history needs quarantine; a checksum or AcceptedReadRef never grants
/// a qualified baseline, existing-goods credit, approval, intent or effect.
pub fn audit(prepared:&PreparedBasketSourceRead,returned:Option<&PublicBasketSourceReturn>,observed:Option<&ReadObservationV1>,accepted:Option<&AcceptedReadRef>)->Result<BasketSourceAudit,DomainError>{
    internal(prepared)?;
    let(payload_digest,observation_digest)=match(returned,observed,accepted){
        (None,None,None)=>(None,None),
        (Some(returned),None,None)if returned.completeness==BasketCompleteness::Incomplete=>{correlate(prepared,returned,&returned.observed_at)?;(Some(canonical_digest(returned)?),None)},
        (Some(returned),Some(observed),accepted)=>{
            let at=accepted.map_or(&returned.observed_at,|receipt|&receipt.admitted_at);let digest=checked_observation(prepared,returned,observed,at)?;
            if let Some(receipt)=accepted{
                shape(receipt)?;let expected=AcceptedReadRef{evidence_id:receipt.evidence_id.clone(),job_id:observed.job_id.clone(),read_id:observed.read_id.clone(),observation_id:observed.observation_id.clone(),digest:digest.clone(),resource:observed.resource.clone(),observed_at:observed.observed_at.clone(),admitted_at:receipt.admitted_at.clone()};
                require(receipt==&expected,ErrorCode::IdentityMismatch,"/basket/source/accepted")?;
            }
            (Some(canonical_digest(returned)?),Some(digest))
        },
        _=>return Err(error(ErrorCode::MissingReference,"/basket/source/history")),
    };
    Ok(BasketSourceAudit{valid:true,payload_digest,observation_digest})
}
