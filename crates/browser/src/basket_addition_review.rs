//! Owner-private, ephemeral hypothetical product additions. External chat facts
//! are correlated, never witnessed or attested here. The core is evaluated only
//! under the explicit premise below. Its temporary admission/candidate stays
//! inside this calculation; no authority, freshness policy or intent is returned.
use crate::{BrowserState,calendar_view::material_at,product_source_reads::{self,PreparedProductSourceRead,ProductSourceTarget},
    product_semantic_review::{self,PreparedProductSemanticReview,PublicSemanticReturn}};
use mstrchf_contracts::*;
use mstrchf_core::{recipes::EvidencePin,products::{MatchPolicy,ProductMeaningAdmission,ProductReadBasis,
    MatchAssessment,MatchReason,SupplyRelation,policy_digest,meaning_digest,prepare_match_context,assess_product_meaning,match_product}};
use schemars::JsonSchema;
use serde::{Deserialize,Serialize};
use serde_json::json;

const PREMISE:&str="This is a hypothetical calculation if the displayed external source facts and automated interpretation are correct. The source has not been witnessed, freshness is unqualified, and no product admission or permission to change a basket is granted.";
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreparedBasketAdditionSource {
    pub format:String,pub version:u8,pub source_prepared:PreparedProductSourceRead,
    pub line_id:DemandLineId,pub product:ProductRef,pub review_id:Text,pub prepared_digest:Digest,
}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PreparedBasketAdditionReview {
    pub format:String,pub version:u8,pub semantic_prepared:PreparedProductSemanticReview,
    pub source_status:String,pub prepared_digest:Digest,
}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,JsonSchema)]
pub struct DraftPackBasis {pub required:Quantity,pub supply:Quantity}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,JsonSchema)]
pub struct BasketAdditionAssessment {
    pub status:String,pub native_assessment:String,pub reasons:Vec<String>,pub evaluated_at:Instant,
    pub source_observed_at:Instant,pub recheck_at:Instant,pub pack_basis:Option<DraftPackBasis>,pub pack_reasons:Vec<String>,
    pub assumption:String,pub freshness_status:String,pub admission_status:String,pub effect_enabled:bool,
}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,JsonSchema)]
pub struct BasketAdditionDraft {
    pub format:String,pub version:u8,pub source_status:String,pub provenance:String,pub product:ProductRef,pub line_id:DemandLineId,
    pub source_observation_digest:Digest,pub prepared_digest:Digest,pub return_digest:Digest,pub qualification:BasketAdditionAssessment,
    pub source_truth_assessed:bool,pub account_verified:bool,pub human_review_recorded:bool,pub effect_enabled:bool,
    pub reusable_for_intent:bool,pub draft_digest:Digest,
}
/// The host alone establishes that a genuine operator action supplied these
/// declarations. JSON validation, signatures and digests do not establish it.
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BasketAdditionOperatorDecision {
    pub review_digest:Digest,pub decided_at:Instant,
    pub source_and_interpretation_accepted:bool,pub exact_addition_approved:bool,
}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,JsonSchema)]
pub struct BasketAdditionOperatorReview {
    pub format:String,pub version:u8,pub product:ProductRef,pub line_id:DemandLineId,
    pub required:Quantity,pub supply:Quantity,pub additional_packs:u8,
    pub review_digest:Digest,pub decision_digest:Digest,pub decided_at:Instant,
    pub source_observation_digest:Digest,pub interpretation_digest:Digest,pub native_assessment:String,
    pub source_truth_assessed:bool,pub account_verified:bool,pub price_assessed:bool,
    pub effect_enabled:bool,pub reusable_for_intent:bool,
}
fn error(code:ErrorCode,path:&str)->DomainError{DomainError::new(code,path)}
fn require(ok:bool,code:ErrorCode,path:&str)->Result<(),DomainError>{if ok{Ok(())}else{Err(error(code,path))}}
fn digest_without(value:&impl Serialize,field:&str)->Result<Digest,DomainError>{
    let mut v=serde_json::to_value(value).map_err(|_|error(ErrorCode::InvalidValue,"/basket/addition/encoding"))?;
    v.as_object_mut().ok_or_else(||error(ErrorCode::InvalidValue,"/basket/addition/encoding"))?.remove(field);canonical_digest(&v)
}
fn size(value:&impl Serialize,max:usize)->Result<(),DomainError>{
    let bytes=serde_json::to_vec(value).map_err(|_|error(ErrorCode::InvalidValue,"/basket/addition/encoding"))?;
    require(bytes.len()<=max,ErrorCode::LimitExceeded,"/basket/addition/size")
}
pub fn source_digest(value:&PreparedBasketAdditionSource)->Result<Digest,DomainError>{digest_without(value,"prepared_digest")}
pub fn review_digest(value:&PreparedBasketAdditionReview)->Result<Digest,DomainError>{digest_without(value,"prepared_digest")}
fn evidence(role:&str,material:&impl Serialize)->Result<EvidenceId,DomainError>{
    let pin=canonical_digest(&json!({"format":"mstrchf-basket-addition-calculation-proof","role":role,"material":material}))?;
    Ok(EvidenceId(format!("ev_{}",&pin.0[..32])))
}
fn selected_line(state:&BrowserState,week_id:&WeekId,line_id:&DemandLineId,now:&Instant)->Result<(),DomainError>{
    let view=crate::runtime_catalogue(state)?;let profile=state.snapshot.profile.0.as_ref().ok_or_else(||error(ErrorCode::MissingField,"/basket/addition/profile"))?;
    let week=state.snapshot.weeks.iter().find(|w|&w.range.week_id==week_id).ok_or_else(||error(ErrorCode::MissingReference,"/basket/addition/week"))?;
    let(_,food)=material_at(&view,&state.snapshot,profile,week,now)?.ok_or_else(||error(ErrorCode::MissingReference,"/basket/addition/food"))?;
    let list=crate::shopping_list::project(state,&view,week,&food.plan,now)?;
    let line=list.lines.iter().find(|l|&l.demand.line_id==line_id).ok_or_else(||error(ErrorCode::MissingReference,"/basket/addition/line"))?;
    require(line.action=="add_to_shop",ErrorCode::DomainConflict,"/basket/addition/line_already_covered")
}
pub fn prepare(state:&BrowserState,week_id:&WeekId,reader:ReadActor,key:BasketKey,delivery:RetailerDeliveryContext,
    target:ProductSourceTarget,line_id:DemandLineId,job_id:JobId,read_id:ReadId,review_id:Text,now:&Instant,deadline:Instant)
    ->Result<PreparedBasketAdditionSource,DomainError>{
    prepare_version(state,week_id,reader,key,delivery,target,line_id,job_id,read_id,review_id,now,deadline,1)
}
pub fn prepare_v3(state:&BrowserState,week_id:&WeekId,reader:ReadActor,key:BasketKey,delivery:RetailerDeliveryContext,
    target:ProductSourceTarget,line_id:DemandLineId,job_id:JobId,read_id:ReadId,review_id:Text,now:&Instant,deadline:Instant)
    ->Result<PreparedBasketAdditionSource,DomainError>{
    prepare_version(state,week_id,reader,key,delivery,target,line_id,job_id,read_id,review_id,now,deadline,2)
}
fn prepare_version(state:&BrowserState,week_id:&WeekId,reader:ReadActor,key:BasketKey,delivery:RetailerDeliveryContext,
    target:ProductSourceTarget,line_id:DemandLineId,job_id:JobId,read_id:ReadId,review_id:Text,now:&Instant,deadline:Instant,version:u8)
    ->Result<PreparedBasketAdditionSource,DomainError>{
    require(!review_id.0.trim().is_empty()&&review_id.0.len()<=128,ErrorCode::InvalidValue,"/basket/addition/review_id")?;
    selected_line(state,week_id,&line_id,now)?;let product=target.product.clone();
    let source_prepared=product_source_reads::prepare(state,week_id,reader,key,delivery,BoundedVec(vec![target]),job_id,read_id,now,deadline)?;
    let mut result=PreparedBasketAdditionSource{format:"mstrchf-basket-addition-source-prepared".into(),version,source_prepared,line_id,product,review_id,prepared_digest:Digest("0".repeat(64))};
    result.prepared_digest=source_digest(&result)?;size(&result,1_000_000)?;Ok(result)
}
pub fn check_source(state:&BrowserState,prepared:&PreparedBasketAdditionSource,observation:&ReadObservationV1,now:&Instant)
    ->Result<PreparedBasketAdditionReview,DomainError>{
    check_source_version(state,prepared,observation,now,1)
}
pub fn check_source_v3(state:&BrowserState,prepared:&PreparedBasketAdditionSource,observation:&ReadObservationV1,now:&Instant)
    ->Result<PreparedBasketAdditionReview,DomainError>{
    check_source_version(state,prepared,observation,now,2)
}
fn check_source_version(state:&BrowserState,prepared:&PreparedBasketAdditionSource,observation:&ReadObservationV1,now:&Instant,version:u8)
    ->Result<PreparedBasketAdditionReview,DomainError>{
    size(prepared,1_000_000)?;
    require(prepared.format=="mstrchf-basket-addition-source-prepared"&&prepared.version==version,ErrorCode::UnsupportedVersion,"/basket/addition/source")?;
    require(source_digest(prepared)?==prepared.prepared_digest,ErrorCode::DigestMismatch,"/basket/addition/source_digest")?;
    let source=&prepared.source_prepared;
    require(source.targets.len()==1&&source.targets[0].product==prepared.product,ErrorCode::IdentityMismatch,"/basket/addition/product")?;
    let week=source.request.week_id.0.as_ref().ok_or_else(||error(ErrorCode::MissingReference,"/basket/addition/week"))?;
    selected_line(state,week,&prepared.line_id,now)?;
    let proof=product_source_reads::unwitnessed_correlation(state,source,observation,now)?;
    let semantic_prepared=if version==2{product_semantic_review::prepare_unwitnessed_v3(state,week,source,observation,&proof,&prepared.product,
        &prepared.line_id,prepared.review_id.clone(),now,source.request.expires_at.clone())?}
        else{product_semantic_review::prepare_unwitnessed(state,week,source,observation,&proof,&prepared.product,
        &prepared.line_id,prepared.review_id.clone(),now,source.request.expires_at.clone())?};
    let mut result=PreparedBasketAdditionReview{format:"mstrchf-basket-addition-semantic-prepared".into(),version,semantic_prepared,
        source_status:"source_witness_pending".into(),prepared_digest:Digest("0".repeat(64))};
    result.prepared_digest=review_digest(&result)?;size(&result,4_100_000)?;Ok(result)
}
fn reason(reason:MatchReason)->&'static str{match reason{
    MatchReason::MissingMeaning=>"missing_meaning",MatchReason::MissingAdmission=>"missing_admission",MatchReason::StaleMeaning=>"stale_meaning",
    MatchReason::UnknownIdentity=>"unknown_identity",MatchReason::ContradictoryIdentity=>"contradictory_identity",MatchReason::RefutedIdentity=>"refuted_identity",
    MatchReason::UnknownRequirement=>"unknown_requirement",MatchReason::ContradictoryRequirement=>"contradictory_requirement",MatchReason::RefutedRequirement=>"refuted_requirement",
    MatchReason::UnsupportedPolicy=>"unsupported_policy",MatchReason::UnknownCooking=>"unknown_cooking",MatchReason::ContradictoryCooking=>"contradictory_cooking",
    MatchReason::ChangedCooking=>"changed_cooking",MatchReason::UnsupportedSupplyRelation=>"unsupported_supply_relation",MatchReason::ConflictingSupplyRelation=>"conflicting_supply_relation",
    MatchReason::PackMeasureMismatch=>"pack_measure_mismatch",
}}
pub fn check_semantic(state:&BrowserState,prepared:&PreparedBasketAdditionReview,returned:&PublicSemanticReturn,now:&Instant)
    ->Result<BasketAdditionDraft,DomainError>{
    evaluate_semantic(state,prepared,returned,now,None,1)
}
pub fn check_semantic_v3(state:&BrowserState,prepared:&PreparedBasketAdditionReview,returned:&PublicSemanticReturn,now:&Instant)
    ->Result<BasketAdditionDraft,DomainError>{
    evaluate_semantic(state,prepared,returned,now,None,2)
}
fn evaluate_semantic(state:&BrowserState,prepared:&PreparedBasketAdditionReview,returned:&PublicSemanticReturn,now:&Instant,
    operator:Option<(&BasketAdditionOperatorDecision,&Instant)>,version:u8)->Result<BasketAdditionDraft,DomainError>{
    size(prepared,4_100_000)?;
    require(prepared.format=="mstrchf-basket-addition-semantic-prepared"&&prepared.version==version&&prepared.source_status=="source_witness_pending",
        ErrorCode::UnsupportedVersion,"/basket/addition/semantic")?;
    require(review_digest(prepared)?==prepared.prepared_digest,ErrorCode::DigestMismatch,"/basket/addition/semantic_digest")?;
    let semantic=&prepared.semantic_prepared;let b=&semantic.binding;
    require(semantic.version==if version==2{3}else{2},ErrorCode::UnsupportedVersion,"/basket/addition/semantic_version")?;
    selected_line(state,&b.week_id,&b.line_id,now)?;
    let review_id=evidence("external_automated_interpretation",returned)?;
    let(recorded,meaning)=if version==2{product_semantic_review::unwitnessed_record_and_meaning_v3(state,semantic,returned,review_id,now)?}
        else{product_semantic_review::unwitnessed_record_and_meaning(state,semantic,returned,review_id,now)?};
    let view=crate::runtime_catalogue(state)?;let profile=state.snapshot.profile.0.as_ref().ok_or_else(||error(ErrorCode::MissingField,"/basket/addition/profile"))?;
    let week=state.snapshot.weeks.iter().find(|w|w.range.week_id==b.week_id).ok_or_else(||error(ErrorCode::MissingReference,"/basket/addition/week"))?;
    let(basis,food)=material_at(&view,&state.snapshot,profile,week,now)?.ok_or_else(||error(ErrorCode::MissingReference,"/basket/addition/food"))?;
    let mut policy=MatchPolicy{version:Revision(1),digest:PayloadDigest("0".repeat(64)),rules:vec![]};policy.digest=policy_digest(&policy)?;
    let context=prepare_match_context(&basis.context,&basis,&food.plan,&policy,now.clone())?;
    let source=ProductReadBasis{request:&b.source_prepared.request,expected_reader:&b.source_prepared.expected_reader,
        observation:&b.observation,accepted:&b.accepted,product:b.product.clone()};
    // A second check WITHOUT any admission proves that actual matching remains
    // unresolved. The subsequent candidate is conditional calculus, not truth.
    let actual=assess_product_meaning(&source,Some(&meaning),None,&context)?;
    require(matches!(match_product(&context,&b.line_id,&actual)?,MatchAssessment::Unresolved(ref issues)if issues.iter().any(|i|i.reason==MatchReason::MissingAdmission)),
        ErrorCode::AuthorityDenied,"/basket/addition/unadmitted")?;
    let review=EvidencePin{evidence_id:recorded.review_pin.evidence_id.clone(),revision:recorded.review_pin.revision,digest:recorded.review_pin.digest.clone()};
    let premise_doc=if let Some((decision,reviewed_at))=operator{json!({
        "format":"mstrchf-basket-addition-declared-operator-premise","version":version,"decision":decision,"reviewed_at":reviewed_at,
        "source_observation_digest":b.accepted.digest,"return_digest":recorded.document.return_digest,"operational_deadline":semantic.deadline,
        "freshness_status":"unqualified","human_action_verified":false,"source_truth_assessed":false,"account_verified":false,
        "effect_enabled":false,"reusable_for_intent":false
    })}else{json!({"format":"mstrchf-hypothetical-basket-calculation-premise","version":version,"assumption":PREMISE,
        "source_observation_digest":b.accepted.digest,"return_digest":recorded.document.return_digest,"operational_deadline":semantic.deadline,
        "freshness_status":"unqualified","human_review_recorded":false,"reusable_for_intent":false})};
    let premise=EvidencePin{evidence_id:evidence(if operator.is_some(){"declared_operator_premise"}else{"hypothetical_premise"},&premise_doc)?,revision:Revision(1),digest:canonical_digest(&premise_doc)?};
    require(review.evidence_id!=premise.evidence_id,ErrorCode::DuplicateKey,"/basket/addition/proof")?;
    let mut pins=vec![review.clone(),premise.clone()];pins.sort_by(|a,b|a.evidence_id.cmp(&b.evidence_id));
    let hypothetical=ProductMeaningAdmission{review,evidence:pins,meaning_digest:meaning_digest(&meaning)?,policy_digest:policy.digest.clone(),
        freshness_rule:premise,meaning_admitted_at:now.clone(),valid_until:semantic.deadline.clone()};
    let hypothetical_review=assess_product_meaning(&source,Some(&meaning),Some(&hypothetical),&context)?;
    let assessment=match_product(&context,&b.line_id,&hypothetical_review)?;
    let(native_assessment,mut reasons,candidate)=match assessment{
        MatchAssessment::Suitable(candidate)=>("suitable",vec![],Some(candidate)),
        MatchAssessment::Unsuitable(issues)=>("unsuitable",issues.into_iter().map(|i|reason(i.reason).to_owned()).collect(),None),
        MatchAssessment::Unresolved(issues)=>("unresolved",issues.into_iter().map(|i|reason(i.reason).to_owned()).collect(),None),
    };
    reasons.sort();reasons.dedup();
    let list=crate::shopping_list::project(state,&view,week,&food.plan,now)?;
    let line=list.lines.iter().find(|line|line.demand.line_id==b.line_id).ok_or_else(||error(ErrorCode::MissingReference,"/basket/addition/line"))?;
    let item=&semantic.request.product;let mut pack_reasons=Vec::<String>::new();
    if candidate.is_none(){pack_reasons.push("not_conditional_match".into());}
    if !line.demand.conversion_evidence_ids.is_empty(){pack_reasons.push("pack_conversion_unsupported".into());}
    if let Some(candidate)=&candidate{
        require(candidate.basis().line==line.demand&&candidate.observation().product==b.product&&candidate.observation().pack==item.pack,
            ErrorCode::IdentityMismatch,"/basket/addition/pack_binding")?;
        if candidate.basis().origins.iter().any(|origin|!matches!(origin.relation,SupplyRelation::Exact)){pack_reasons.push("pack_conversion_unsupported".into());}
    }
    match &item.pack.count{Fact::Known{value}if value.0==1=>{},Fact::Known{..}=>pack_reasons.push("pack_count_not_one".into()),
        Fact::Unknown{..}=>pack_reasons.push("pack_count_unknown".into()),Fact::Conflicting{..}=>pack_reasons.push("pack_count_conflicting".into())}
    let drained=line.demand.quantity.form==IngredientForm::Drained;
    let supply=match if drained{&item.pack.drained}else{&item.pack.net}{
        Fact::Known{value}if value.unit==line.demand.quantity.unit&&value.form==line.demand.quantity.form=>Some(value.clone()),
        Fact::Known{..}=>{pack_reasons.push("pack_measure_mismatch".into());None},
        Fact::Unknown{..}=>{pack_reasons.push(if drained{"pack_drained_unknown"}else{"pack_net_unknown"}.into());None},
        Fact::Conflicting{..}=>{pack_reasons.push(if drained{"pack_drained_conflicting"}else{"pack_net_conflicting"}.into());None},
    };
    pack_reasons.sort();pack_reasons.dedup();
    let pack_basis=if pack_reasons.is_empty(){Some(DraftPackBasis{required:line.demand.quantity.clone(),supply:supply.ok_or_else(||error(ErrorCode::MissingReference,"/basket/addition/supply"))?})}else{None};
    let qualification=BasketAdditionAssessment{status:if native_assessment=="suitable"{"conditional_match"}else{"unresolved"}.into(),native_assessment:native_assessment.into(),reasons,
        evaluated_at:now.clone(),source_observed_at:item.observed_at.clone(),recheck_at:semantic.deadline.clone(),pack_basis,pack_reasons,
        assumption:PREMISE.into(),freshness_status:"unqualified".into(),admission_status:"missing".into(),effect_enabled:false};
    let mut result=BasketAdditionDraft{format:"mstrchf-basket-addition-review".into(),version,source_status:"source_witness_pending".into(),
        provenance:"external_chat_proposal".into(),product:b.product.clone(),line_id:b.line_id.clone(),source_observation_digest:b.accepted.digest.clone(),
        prepared_digest:prepared.prepared_digest.clone(),return_digest:recorded.document.return_digest.clone(),qualification,
        source_truth_assessed:false,account_verified:false,human_review_recorded:false,effect_enabled:false,reusable_for_intent:false,draft_digest:Digest("0".repeat(64))};
    result.draft_digest=digest_without(&result,"draft_digest")?;Ok(result)
}

/// Reconstruct the exact earlier display, then assess the currently selected
/// line under the separately declared operator decision. No acceptance,
/// admission, candidate, grant or external-write permission leaves this call.
pub fn operator_review(state:&BrowserState,prepared:&PreparedBasketAdditionReview,returned:&PublicSemanticReturn,
    reviewed_at:&Instant,decision:&BasketAdditionOperatorDecision,now:&Instant)->Result<BasketAdditionOperatorReview,DomainError>{
    operator_review_version(state,prepared,returned,reviewed_at,decision,now,1)
}
pub fn operator_review_v3(state:&BrowserState,prepared:&PreparedBasketAdditionReview,returned:&PublicSemanticReturn,
    reviewed_at:&Instant,decision:&BasketAdditionOperatorDecision,now:&Instant)->Result<BasketAdditionOperatorReview,DomainError>{
    operator_review_version(state,prepared,returned,reviewed_at,decision,now,2)
}
fn operator_review_version(state:&BrowserState,prepared:&PreparedBasketAdditionReview,returned:&PublicSemanticReturn,
    reviewed_at:&Instant,decision:&BasketAdditionOperatorDecision,now:&Instant,version:u8)->Result<BasketAdditionOperatorReview,DomainError>{
    for issues in [decision.validate_shape(),reviewed_at.validate_shape(),now.validate_shape()]{
        issues.map_err(|issues|DomainError{code:issues.first().map_or(ErrorCode::InvalidValue,|issue|issue.code),
            truncated:issues.len()>100,issues:BoundedVec(issues.into_iter().take(100).collect()),retry:Retry::Never})?;
    }
    require(decision.source_and_interpretation_accepted&&decision.exact_addition_approved,
        ErrorCode::AuthorityDenied,"/basket/addition/operator_decision")?;
    require(reviewed_at<=&decision.decided_at&&&decision.decided_at==now,
        ErrorCode::ContextStale,"/basket/addition/operator_clock")?;
    let prior=evaluate_semantic(state,prepared,returned,reviewed_at,None,version)?;
    require(prior.draft_digest==decision.review_digest,ErrorCode::DigestMismatch,"/basket/addition/review_digest")?;
    let current=evaluate_semantic(state,prepared,returned,now,Some((decision,reviewed_at)),version)?;
    require(current.qualification.native_assessment=="suitable"&&current.qualification.status=="conditional_match",
        ErrorCode::DomainConflict,"/basket/addition/operator_suitability")?;
    let pack=current.qualification.pack_basis.as_ref().ok_or_else(||error(ErrorCode::DomainConflict,"/basket/addition/operator_pack"))?;
    require(pack.required.unit==pack.supply.unit&&pack.required.form==pack.supply.form,
        ErrorCode::DomainConflict,"/basket/addition/operator_pack")?;
    // This bounded inequality uses the canonical positive rationals produced
    // by the core. One pack covers the positive demand iff required <= supply;
    // no conversion, rounding or second pack-allocation calculation is added.
    let required=pack.required.amount.0;let supply=pack.supply.amount.0;
    let left=u128::from(required.numerator.0).checked_mul(u128::from(supply.denominator.0));
    let right=u128::from(supply.numerator.0).checked_mul(u128::from(required.denominator.0));
    let (left,right)=left.zip(right).ok_or_else(||error(ErrorCode::ArithmeticOverflow,"/basket/addition/operator_pack"))?;
    require(left<=right,ErrorCode::DomainConflict,"/basket/addition/operator_one_pack")?;
    let mut result=BasketAdditionOperatorReview{format:"mstrchf-basket-addition-operator-review".into(),version,
        product:current.product,line_id:current.line_id,required:pack.required.clone(),supply:pack.supply.clone(),additional_packs:1,
        review_digest:decision.review_digest.clone(),decision_digest:Digest("0".repeat(64)),decided_at:decision.decided_at.clone(),
        source_observation_digest:current.source_observation_digest,interpretation_digest:current.return_digest,native_assessment:"suitable".into(),
        source_truth_assessed:false,account_verified:false,price_assessed:false,effect_enabled:false,reusable_for_intent:false};
    let projection=serde_json::to_value(&result).map_err(|_|error(ErrorCode::InvalidValue,"/basket/addition/encoding"))?;
    let mut projection=projection;projection.as_object_mut().ok_or_else(||error(ErrorCode::InvalidValue,"/basket/addition/encoding"))?.remove("decision_digest");
    result.decision_digest=canonical_digest(&json!({"format":"mstrchf-basket-addition-operator-decision-material","version":version,
        "projection":projection,"decision":decision,"reviewed_at":reviewed_at,"prepared_digest":prepared.prepared_digest}))?;
    Ok(result)
}
