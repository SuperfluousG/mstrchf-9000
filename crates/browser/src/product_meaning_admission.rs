//! A maker-adopted LOCAL operational recheck rule for retained plain-oats
//! interpretation. It is not Tesco expiry, a host attestation, proof of unchanged
//! source facts, usable pack coverage or shopping permission. Native admission
//! exists only during this pure evaluation; no admission/basis is imported or
//! persisted. The owner independently selects accepted, nonquarantined custody.
use crate::{BrowserState, calendar_view::material_at, product_semantic_review::{self,
    PreparedProductSemanticReview,RecordedProductSemanticReview}};
use mstrchf_contracts::*;
use mstrchf_core::{recipes::EvidencePin,products::{MatchPolicy,ProductMeaningAdmission,
    ProductReadBasis,MatchAssessment,MatchReason,policy_digest,meaning_digest,
    prepare_match_context,assess_product_meaning,match_product,SupplyRelation}};
use schemars::JsonSchema;
use serde::Serialize;
use serde_json::{Value,json};

const POLICY_ID:&str="tesco-plain-oats-retained-interpretation-v1";
const SCOPE:&str="tesco_en_gb_plain_rolled_oats_r3_rc07_confirmed_none";
const AGE:i64=3600;
const NOTICE:&str="The maker's local rule requires rechecking retained Tesco plain-oats composition and cooking interpretation after 3600 seconds from the product observation. This is not Tesco's expiry or proof that the page is unchanged. Pack coverage, prices, stock, dates, handling and basket changes remain unqualified.";
const DOCUMENT:&str=r#"{
 "format":"mstrchf-product-meaning-recheck-policy","version":1,
 "id":"tesco-plain-oats-retained-interpretation-v1","revision":"1",
 "evidence_id":"ev_80000000000000000000000000000054",
 "scope":"tesco_en_gb_plain_rolled_oats_r3_rc07_confirmed_none",
 "maker_status":"adopted_local_operational_recheck_policy",
 "review_actor":"automated","source_truth_guarantee":false,
 "source":"https://www.tesco.com/shop/en-GB/products/{retained_product_id}",
 "ingredient_id":"ig_80000000000000000000000000000001","unit":"g","form":"dry",
 "recipe_refs":[
  {"recipe_id":"rc_80000000000000000000000000000001","revision":"3","digest":"dd867c017a9b591f57546246518cd93c94e9f1ab50c54bfe4f288a1054de6ce0"},
  {"recipe_id":"rc_80000000000000000000000000000007","revision":"1","digest":"8e73018765acca1f0265082f1c95601359274014259a306f4621ed805a133b97"}
 ],
 "max_age_seconds":3600,"anchor":"item.observed_at","expiry_exclusive":true,
 "scope_conditions":["Exact independently retained accepted source and automated semantic review","Exact current pinned R3 or rc07 plain dry rolled-oat uses and production origins","No current hard requirements, substitution or conversion","Actual known cited ingredient text; unknown or conflicting dependencies stay unresolved"],
 "limitations":["Local maker recheck budget, not vendor TTL","No proof source remains unchanged","No host-model or retailer attestation","No actual pack coverage, price, stock, dates or handling qualification","No persisted admission, shopping approval or external effect","Fresh accepted observation and semantic record required after age limit; old records are not rewritten"],
 "notice":"The maker's local rule requires rechecking retained Tesco plain-oats composition and cooking interpretation after 3600 seconds from the product observation. This is not Tesco's expiry or proof that the page is unchanged. Pack coverage, prices, stock, dates, handling and basket changes remain unqualified."
}"#;

const EXPANDED_POLICY_ID:&str="tesco-pinned-input-retained-interpretation-v1";
const EXPANDED_SCOPE:&str="tesco_en_gb_exact_nine_maker_packets_no_hard_requirement_admission";
const EXPANDED_NOTICE:&str="The maker's local rule requires rechecking retained Tesco ingredient and exact-method interpretation after 3600 seconds from the original product observation. This is not Tesco's expiry or proof that the page is unchanged. Hard dietary requirements remain unqualified. Cold ready-to-eat inputs need known cited storage/direct-use text. Actual processing, cold storage, handling, separation, prices, stock, dates and basket changes are not established by this check.";
// Literal full recipe, packet and registry pins prevent future catalogue entries
// or changed input definitions from silently acquiring this policy. The original
// oats document above remains byte-for-byte unchanged for v1 custody.
const EXPANDED_DOCUMENT:&str=r#"{
 "format":"mstrchf-product-meaning-recheck-policy","version":1,
 "id":"tesco-pinned-input-retained-interpretation-v1","revision":"1",
 "evidence_id":"ev_80000000000000000000000000000055",
 "scope":"tesco_en_gb_exact_nine_maker_packets_no_hard_requirement_admission",
 "maker_status":"adopted_local_operational_recheck_policy","review_actor":"automated","source_truth_guarantee":false,
 "source":"https://www.tesco.com/shop/en-GB/products/{retained_product_id}",
 "packets":[
  {"recipe_id":"rc_80000000000000000000000000000001","revision":"3","recipe_digest":"dd867c017a9b591f57546246518cd93c94e9f1ab50c54bfe4f288a1054de6ce0","packet_digest":"edc4239b57366c96ef4b91198e4cc54de62db44d3d6346574e7318a852e2d726","registry_digest":"054556ac105133e16b373dd139d24f47fa361787ed1709cd18b2ace043801f59"},
  {"recipe_id":"rc_80000000000000000000000000000003","revision":"1","recipe_digest":"58914b613960257e33ce294636bcd417baccb631f61379a22aa4977b14cc14f3","packet_digest":"62949a9b43fb50ed7385271a22c09f9b5d703ff08ce865041b66c6d6f2d14dee","registry_digest":"478ab6a465d2b61a069d8437349e61c5d3e32065b5c602a5860899c9dfda0390"},
  {"recipe_id":"rc_80000000000000000000000000000004","revision":"1","recipe_digest":"247de874dc0348b4afdc06fe6b5aab31cdebf2869ace5098393d61dae9a80377","packet_digest":"ec3894c5071806462224a3ba282467b159b8a84ad05df8ae12995489784802ac","registry_digest":"2a1cd41ed445c0454842d30ed219009e4dcb0bc1e0092a01f38c4cf5687623b1"},
  {"recipe_id":"rc_80000000000000000000000000000005","revision":"1","recipe_digest":"bea391c42fe3a7207d4ac04104f5fac6d839a36970a197be490585c89fb0cf7c","packet_digest":"e18d259dfa7941b893997dee7b378616457025ccac92021d54a2a43e25173932","registry_digest":"f5f1156cd891ab266ed7c085985b6c9e48466f55356cb673de1740bd6215992f"},
  {"recipe_id":"rc_80000000000000000000000000000006","revision":"1","recipe_digest":"9e5495c21e6da79685364e4069c62485db521652f5b184f955ff5ae27a6f1632","packet_digest":"61aa9bc1b586ff82a1ceab2198f5cf88e7113ef82e37bdfa2b60cd4c7915de6d","registry_digest":"f5f1156cd891ab266ed7c085985b6c9e48466f55356cb673de1740bd6215992f"},
  {"recipe_id":"rc_80000000000000000000000000000007","revision":"1","recipe_digest":"8e73018765acca1f0265082f1c95601359274014259a306f4621ed805a133b97","packet_digest":"93df7d839d9076d377fb1c08b201c3048aff4a428093a1cf75fcf6cf919cebfd","registry_digest":"0782d6aa6877ec07e3d5217a61a49dc50e4857b7744e394cb5991a1b583a2579"},
  {"recipe_id":"rc_80000000000000000000000000000008","revision":"1","recipe_digest":"fa50f509872b19c8b986dff36ec91397e5268a60afbb93a7728badfde8661a71","packet_digest":"a6a189f045a8fe21617efb5fed082a43773071c6250c2b06bdda943e32395407","registry_digest":"204605d56020595ef9bb702c07d89d4a7125f7b8173fef6c4a71536d0248306f"},
  {"recipe_id":"rc_80000000000000000000000000000009","revision":"1","recipe_digest":"ef0cbaba1c1698ccaaa8de4b0f6ee4526d33150645532a3ca822f65ba9cf97c9","packet_digest":"e9675ef1aec183cf4d540b5c2c23856a1666f02bcf6b57080e8b845f1551209b","registry_digest":"30d530a39d08b30e5475c2c4bf5b35705ccfba6d7263c4e28ca96df9b21bf621"},
  {"recipe_id":"rc_8000000000000000000000000000000a","revision":"1","recipe_digest":"152c886cd7d50aae387f3d469059a57f2c21b651943e82823f33103df35f300d","packet_digest":"8414f28d6683d7d4024911686034166de3de9d278b0759f1acfa29e6a3bed663","registry_digest":"2a1cd41ed445c0454842d30ed219009e4dcb0bc1e0092a01f38c4cf5687623b1"}
 ],
 "max_age_seconds":3600,"anchor":"item.observed_at","expiry_exclusive":true,
 "scope_conditions":["Exact accepted source and immutable automated semantic review","Every current use and supply origin resolves to an enumerated full packet and exact input definition","Cooking decisions cover the complete method and all pinned intrinsic conditions, including for None consumers","Known cited ingredients required; cooked/drained cold-ready-to-eat inputs also require known cited storage/direct-use text","No hard-requirement claims or policy rules; current dietary requirements remain unresolved in core","Exact g/ml/each and same-form single sold pack only; drained demand uses drained measure, never net"],
 "limitations":["Local maker recheck budget, not vendor TTL or unchanged-source proof","No title/brand/recipe inference of product compliance","No observed processing/cross-contact, actual prior cold storage, handling, separation or date compliance","No substitutions, conversions, multipack or actual available stock assumption","No price currentness or promotions eligibility","No persisted admission, shopping approval or external effect"],
 "notice":"The maker's local rule requires rechecking retained Tesco ingredient and exact-method interpretation after 3600 seconds from the original product observation. This is not Tesco's expiry or proof that the page is unchanged. Hard dietary requirements remain unqualified. Cold ready-to-eat inputs need known cited storage/direct-use text. Actual processing, cold storage, handling, separation, prices, stock, dates and basket changes are not established by this check."
}"#;
const CHILLI_POLICY_ID:&str="tesco-chilli-and-pinned-input-retained-interpretation-v1";
const CHILLI_SCOPE:&str="tesco_en_gb_exact_twelve_maker_packets_chilli_original_batch_no_hard_requirement_admission";
// Independent immutable v3 policy; old v1/v2 literals above stay unchanged.
const CHILLI_DOCUMENT:&str=r#"{
 "format":"mstrchf-product-meaning-recheck-policy","version":1,
 "id":"tesco-chilli-and-pinned-input-retained-interpretation-v1","revision":"1",
 "evidence_id":"ev_80000000000000000000000000000200",
 "scope":"tesco_en_gb_exact_twelve_maker_packets_chilli_original_batch_no_hard_requirement_admission",
 "maker_status":"adopted_local_operational_recheck_policy","review_actor":"automated","source_truth_guarantee":false,
 "source":"https://www.tesco.com/shop/en-GB/products/{retained_product_id}",
 "packets":[
  {"recipe_id":"rc_80000000000000000000000000000001","revision":"3","recipe_digest":"dd867c017a9b591f57546246518cd93c94e9f1ab50c54bfe4f288a1054de6ce0","packet_digest":"edc4239b57366c96ef4b91198e4cc54de62db44d3d6346574e7318a852e2d726","registry_digest":"054556ac105133e16b373dd139d24f47fa361787ed1709cd18b2ace043801f59"},
  {"recipe_id":"rc_80000000000000000000000000000003","revision":"1","recipe_digest":"58914b613960257e33ce294636bcd417baccb631f61379a22aa4977b14cc14f3","packet_digest":"62949a9b43fb50ed7385271a22c09f9b5d703ff08ce865041b66c6d6f2d14dee","registry_digest":"478ab6a465d2b61a069d8437349e61c5d3e32065b5c602a5860899c9dfda0390"},
  {"recipe_id":"rc_80000000000000000000000000000004","revision":"1","recipe_digest":"247de874dc0348b4afdc06fe6b5aab31cdebf2869ace5098393d61dae9a80377","packet_digest":"ec3894c5071806462224a3ba282467b159b8a84ad05df8ae12995489784802ac","registry_digest":"2a1cd41ed445c0454842d30ed219009e4dcb0bc1e0092a01f38c4cf5687623b1"},
  {"recipe_id":"rc_80000000000000000000000000000005","revision":"1","recipe_digest":"bea391c42fe3a7207d4ac04104f5fac6d839a36970a197be490585c89fb0cf7c","packet_digest":"e18d259dfa7941b893997dee7b378616457025ccac92021d54a2a43e25173932","registry_digest":"f5f1156cd891ab266ed7c085985b6c9e48466f55356cb673de1740bd6215992f"},
  {"recipe_id":"rc_80000000000000000000000000000006","revision":"1","recipe_digest":"9e5495c21e6da79685364e4069c62485db521652f5b184f955ff5ae27a6f1632","packet_digest":"61aa9bc1b586ff82a1ceab2198f5cf88e7113ef82e37bdfa2b60cd4c7915de6d","registry_digest":"f5f1156cd891ab266ed7c085985b6c9e48466f55356cb673de1740bd6215992f"},
  {"recipe_id":"rc_80000000000000000000000000000007","revision":"1","recipe_digest":"8e73018765acca1f0265082f1c95601359274014259a306f4621ed805a133b97","packet_digest":"93df7d839d9076d377fb1c08b201c3048aff4a428093a1cf75fcf6cf919cebfd","registry_digest":"0782d6aa6877ec07e3d5217a61a49dc50e4857b7744e394cb5991a1b583a2579"},
  {"recipe_id":"rc_80000000000000000000000000000008","revision":"1","recipe_digest":"fa50f509872b19c8b986dff36ec91397e5268a60afbb93a7728badfde8661a71","packet_digest":"a6a189f045a8fe21617efb5fed082a43773071c6250c2b06bdda943e32395407","registry_digest":"204605d56020595ef9bb702c07d89d4a7125f7b8173fef6c4a71536d0248306f"},
  {"recipe_id":"rc_80000000000000000000000000000009","revision":"1","recipe_digest":"ef0cbaba1c1698ccaaa8de4b0f6ee4526d33150645532a3ca822f65ba9cf97c9","packet_digest":"e9675ef1aec183cf4d540b5c2c23856a1666f02bcf6b57080e8b845f1551209b","registry_digest":"30d530a39d08b30e5475c2c4bf5b35705ccfba6d7263c4e28ca96df9b21bf621"},
  {"recipe_id":"rc_8000000000000000000000000000000a","revision":"1","recipe_digest":"152c886cd7d50aae387f3d469059a57f2c21b651943e82823f33103df35f300d","packet_digest":"8414f28d6683d7d4024911686034166de3de9d278b0759f1acfa29e6a3bed663","registry_digest":"2a1cd41ed445c0454842d30ed219009e4dcb0bc1e0092a01f38c4cf5687623b1"}
,
  {"recipe_id":"rc_80000000000000000000000000000011","revision":"1","recipe_digest":"1dfb0d625718a3eb890701e34e69e2f7c3ae75a3958dab6d35636c393eaece3b","packet_digest":"99bdfba2526d24f6c6d2ca78751cfc3464e21bd52386b1abc6e7d6436b89cf80","registry_digest":"84d054764fee84b80b4de59f85006d01e5b3135ee16ddf172279305257cd2d52"},
  {"recipe_id":"rc_80000000000000000000000000000012","revision":"1","recipe_digest":"dafaef4f8f417e2ca7f05643aaa060c76be69e6c0a3ff0b18e5969c1c83ec41f","packet_digest":"6cd211674048a6860513db53c0146e11aa938394ef9ba5fe88c5ebd842671aa9","registry_digest":"84d054764fee84b80b4de59f85006d01e5b3135ee16ddf172279305257cd2d52"},
  {"recipe_id":"rc_80000000000000000000000000000013","revision":"1","recipe_digest":"5ad6ce59c15710de3ce8c629cc352151e97f201af5e95670869c997507d9dec3","packet_digest":"6f5fa887ae1c57a883d7139d0ad7c01bd37424004623fba5846a57ad17a150af","registry_digest":"84d054764fee84b80b4de59f85006d01e5b3135ee16ddf172279305257cd2d52"}
 ],
 "max_age_seconds":3600,"anchor":"item.observed_at","expiry_exclusive":true,
 "scope_conditions":["Exact accepted source and immutable automated semantic review","Every current use and supply origin resolves to an enumerated full packet and exact input definition","Only the exact chilli producer and two pinned original-share remixes may inherit inputs: saved leftover, source new batch, batch link, allocation and supply origin must agree exactly; no ingredient-only ancestry","Original production, preparation, storage deadline, tighter product limits and once-only reheating remain controlling; source recheck never renews food clocks or duplicates purchase demand","Cooking decisions cover the complete method and all pinned intrinsic conditions, including for None consumers","Known cited ingredients required; cooked/drained cold-ready-to-eat inputs also require known cited storage/direct-use text","No hard-requirement claims or policy rules; current dietary requirements remain unresolved in core","Exact g/ml/each and same-form single sold pack only; drained demand uses drained measure, never net"],
 "limitations":["Local maker recheck budget, not vendor TTL or unchanged-source proof","No title/brand/recipe inference of product compliance","No observed processing/cross-contact, actual prior cold storage, handling, separation or date compliance","No substitutions, conversions, multipack or actual available stock assumption","No price currentness or promotions eligibility","No persisted admission, shopping approval or external effect"],
 "notice":"The maker's local rule requires rechecking retained Tesco ingredient and exact-method interpretation after 3600 seconds from the original product observation. This is not Tesco's expiry or proof that the page is unchanged. Hard dietary requirements remain unqualified. Cold ready-to-eat inputs need known cited storage/direct-use text. Actual processing, cold storage, handling, separation, prices, stock, dates and basket changes are not established by this check."
}"#;


#[derive(Clone,Debug,PartialEq,Eq,Serialize,JsonSchema)]
pub struct MeaningRecheckPolicy {pub id:String,pub revision:Revision,pub digest:Digest,
    pub max_age_seconds:u32,pub scope:String,pub notice:String}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,JsonSchema)]
pub struct ProductPackBasis {pub required:Quantity,pub supply:Quantity}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,JsonSchema)]
pub struct ProductMeaningQualification {
    pub format:String,pub version:u8,pub request_id:Text,pub product:ProductRef,pub line_id:DemandLineId,
    pub evaluated_at:Instant,pub source_observed_at:Instant,pub recheck_at:Instant,
    pub policy:MeaningRecheckPolicy,pub status:String,pub native_assessment:String,pub reasons:Vec<String>,
    pub match_basis_digest:Option<PayloadDigest>,pub pack_basis:Option<ProductPackBasis>,
    pub pack_reasons:Vec<String>,pub effect_enabled:bool,
}
fn error(code:ErrorCode,path:&str)->DomainError{DomainError::new(code,path)}
fn require(ok:bool,code:ErrorCode,path:&str)->Result<(),DomainError>{if ok{Ok(())}else{Err(error(code,path))}}
/// Readable evidence document only; callers cannot use these bytes to construct
/// or replace the private compiled policy selected by evaluate.
pub fn policy_document()->Result<Value,DomainError>{serde_json::from_str(DOCUMENT).map_err(|_|error(ErrorCode::InvalidValue,"/products/qualification/policy"))}
pub fn expanded_policy_document()->Result<Value,DomainError>{serde_json::from_str(EXPANDED_DOCUMENT).map_err(|_|error(ErrorCode::InvalidValue,"/products/qualification/policy"))}
pub fn chilli_policy_document()->Result<Value,DomainError>{serde_json::from_str(CHILLI_DOCUMENT).map_err(|_|error(ErrorCode::InvalidValue,"/products/qualification/policy"))}
struct TrustedPolicy {metadata:MeaningRecheckPolicy,pin:EvidencePin}
fn trusted_policy(version:u8)->Result<TrustedPolicy,DomainError>{
    let(doc,id,scope,notice,evidence_id)=match version{
        1=>{let doc=policy_document()?;require(doc["recipe_refs"]==json!([crate::maker_packets::porridge_packet_ref()?.recipe,crate::maker_packets::larger_porridge_packet_ref()?.recipe]),ErrorCode::DigestMismatch,"/products/qualification/recipe_refs")?;(doc,POLICY_ID,SCOPE,NOTICE,"ev_80000000000000000000000000000054")},
        2=>{let doc=expanded_policy_document()?;let actual=product_semantic_review::supported_packet_refs(2)?.iter().map(|rf|{
                let packet=crate::maker_packets::known_packet(rf)?;Ok(json!({"recipe_id":rf.recipe.recipe_id,"revision":rf.recipe.revision,"recipe_digest":rf.recipe.digest,"packet_digest":rf.packet_digest,"registry_digest":canonical_digest(&packet.record().ingredient_registry)?}))
            }).collect::<Result<Vec<_>,DomainError>>()?;
            require(doc["packets"]==json!(actual),ErrorCode::DigestMismatch,"/products/qualification/packet_registry_pins")?;(doc,EXPANDED_POLICY_ID,EXPANDED_SCOPE,EXPANDED_NOTICE,"ev_80000000000000000000000000000055")},
        3=>{let doc=chilli_policy_document()?;let actual=product_semantic_review::supported_packet_refs(3)?.iter().map(|rf|{
                let packet=crate::maker_packets::known_packet(rf)?;Ok(json!({"recipe_id":rf.recipe.recipe_id,"revision":rf.recipe.revision,"recipe_digest":rf.recipe.digest,"packet_digest":rf.packet_digest,"registry_digest":canonical_digest(&packet.record().ingredient_registry)?}))
            }).collect::<Result<Vec<_>,DomainError>>()?;
            require(doc["packets"]==json!(actual),ErrorCode::DigestMismatch,"/products/qualification/packet_registry_pins")?;(doc,CHILLI_POLICY_ID,CHILLI_SCOPE,EXPANDED_NOTICE,"ev_80000000000000000000000000000200")},
        _=>return Err(error(ErrorCode::UnsupportedVersion,"/products/qualification/policy")),
    };
    let digest=canonical_digest(&doc)?;
    Ok(TrustedPolicy{metadata:MeaningRecheckPolicy{id:id.into(),revision:Revision(1),digest:digest.clone(),max_age_seconds:AGE as u32,scope:scope.into(),notice:notice.into()},
        pin:EvidencePin{evidence_id:EvidenceId(evidence_id.into()),revision:Revision(1),digest}})
}
fn reason(r:MatchReason)->&'static str{match r{
    MatchReason::MissingMeaning=>"missing_meaning",MatchReason::MissingAdmission=>"missing_admission",MatchReason::StaleMeaning=>"stale_meaning",
    MatchReason::UnknownIdentity=>"unknown_identity",MatchReason::ContradictoryIdentity=>"contradictory_identity",MatchReason::RefutedIdentity=>"refuted_identity",
    MatchReason::UnknownRequirement=>"unknown_requirement",MatchReason::ContradictoryRequirement=>"contradictory_requirement",MatchReason::RefutedRequirement=>"refuted_requirement",
    MatchReason::UnsupportedPolicy=>"unsupported_policy",MatchReason::UnknownCooking=>"unknown_cooking",MatchReason::ContradictoryCooking=>"contradictory_cooking",
    MatchReason::ChangedCooking=>"changed_cooking",MatchReason::UnsupportedSupplyRelation=>"unsupported_supply_relation",MatchReason::ConflictingSupplyRelation=>"conflicting_supply_relation",
    MatchReason::PackMeasureMismatch=>"pack_measure_mismatch",
}}

pub fn evaluate(state:&BrowserState,prepared:&PreparedProductSemanticReview,recorded:&RecordedProductSemanticReview,now:&Instant)
    ->Result<ProductMeaningQualification,DomainError>{
    // Exact historical document/pin plus exact current line/use/origin/recipe
    // closure; completed model task expiry and AI settings are not new gates.
    let m=product_semantic_review::recorded_meaning(state,prepared,recorded,now)?;
    let trusted=trusted_policy(prepared.version)?;let b=&prepared.binding;let item=&prepared.request.product;
    require(item.product.retailer==Retailer::Tesco&&item.product.variant_id.0.is_none()
        &&item.url.0==format!("https://www.tesco.com/shop/en-GB/products/{}",item.product.product_id.0),ErrorCode::ScopeViolation,"/products/qualification/source")?;
    require(item.observed_at<=*now,ErrorCode::InvalidValue,"/products/qualification/observation_chronology")?;
    let expiry=item.observed_at.timestamp_seconds().checked_add(AGE).ok_or_else(||error(ErrorCode::ArithmeticOverflow,"/products/qualification/recheck_at"))?;
    let at=chrono::DateTime::from_timestamp(expiry,0).ok_or_else(||error(ErrorCode::InvalidValue,"/products/qualification/recheck_at"))?;
    let recheck_at=Instant(at.format("%Y-%m-%dT%H:%M:%SZ").to_string());
    recheck_at.validate_shape().map_err(|_|error(ErrorCode::InvalidValue,"/products/qualification/recheck_at"))?;
    let expired=now>=&recheck_at;
    let view=crate::runtime_catalogue(state)?;let profile=state.snapshot.profile.0.as_ref().ok_or_else(||error(ErrorCode::MissingField,"/products/qualification/profile"))?;
    let week=state.snapshot.weeks.iter().find(|w|w.range.week_id==b.week_id).ok_or_else(||error(ErrorCode::MissingReference,"/products/qualification/week"))?;
    let (basis,food)=material_at(&view,&state.snapshot,profile,week,now)?.ok_or_else(||error(ErrorCode::MissingReference,"/products/qualification/food"))?;
    let mut policy=MatchPolicy{version:Revision(1),digest:PayloadDigest("0".repeat(64)),rules:vec![]};policy.digest=policy_digest(&policy)?;
    let context=prepare_match_context(&basis.context,&basis,&food.plan,&policy,now.clone())?;
    // Do not create an expired admission by backdating a newly adopted meaning.
    // The core still assesses expired retained material with no admission.
    let admission=if expired{None}else{
        require(recorded.review_pin.evidence_id!=trusted.pin.evidence_id,ErrorCode::DuplicateKey,"/products/qualification/evidence")?;
        let review=EvidencePin{evidence_id:recorded.review_pin.evidence_id.clone(),revision:recorded.review_pin.revision,digest:recorded.review_pin.digest.clone()};
        let mut evidence=vec![review.clone(),trusted.pin.clone()];evidence.sort_by(|a,b|a.evidence_id.cmp(&b.evidence_id));
        Some(ProductMeaningAdmission{review,evidence,meaning_digest:meaning_digest(&m)?,policy_digest:policy.digest.clone(),freshness_rule:trusted.pin,
            meaning_admitted_at:now.clone(),valid_until:recheck_at.clone()})
    };
    let source=ProductReadBasis{request:&b.source_prepared.request,expected_reader:&b.source_prepared.expected_reader,observation:&b.observation,accepted:&b.accepted,product:b.product.clone()};
    let reviewed=assess_product_meaning(&source,Some(&m),admission.as_ref(),&context)?;
    let assessment=match_product(&context,&b.line_id,&reviewed)?;
    let(native_assessment,mut reasons,match_basis_digest,candidate)=match assessment{
        MatchAssessment::Suitable(candidate)=>("suitable",vec![],Some(candidate.digest().clone()),Some(candidate)),
        MatchAssessment::Unsuitable(issues)=>("unsuitable",issues.into_iter().map(|i|reason(i.reason).to_owned()).collect(),None,None),
        MatchAssessment::Unresolved(issues)=>("unresolved",issues.into_iter().map(|i|reason(i.reason).to_owned()).collect(),None,None),
    };
    if expired{reasons.push("source_recheck_required".into());}
    reasons.sort();reasons.dedup();
    let status=if expired{"recheck_required"}else if native_assessment=="suitable"{"conditional_match"}else{"unresolved"};
    // Same current food projection, not a model/mapping DTO. Pantry action does
    // not change the full food demand; covered lines cannot suggest purchases.
    let shopping=crate::shopping_list::project(state,&view,week,&food.plan,now)?;
    let line=shopping.lines.iter().find(|line|line.demand.line_id==b.line_id)
        .ok_or_else(||error(ErrorCode::MissingReference,"/products/qualification/shopping_line"))?;
    let mut pack_reasons=Vec::<String>::new();
    if candidate.is_none(){pack_reasons.push("not_conditional_match".into());}
    if expired{pack_reasons.push("source_recheck_required".into());}
    if line.action!="add_to_shop"{pack_reasons.push(if line.action=="have_enough"{"line_already_covered"}else{"line_not_add_to_shop"}.into());}
    if !line.demand.conversion_evidence_ids.is_empty(){pack_reasons.push("pack_conversion_unsupported".into());}
    let observed=if let Some(candidate)=&candidate {
        require(candidate.basis().line==line.demand&&candidate.observation().product==b.product
            &&candidate.observation().pack==item.pack,ErrorCode::IdentityMismatch,"/products/qualification/pack_binding")?;
        if candidate.basis().origins.iter().any(|o|!matches!(o.relation,SupplyRelation::Exact)){pack_reasons.push("pack_conversion_unsupported".into());}
        &candidate.observation().pack
    }else{&item.pack};
    match &observed.count {
        Fact::Known{value} if value.0==1=>{},Fact::Known{..}=>pack_reasons.push("pack_count_not_one".into()),
        Fact::Unknown{..}=>pack_reasons.push("pack_count_unknown".into()),Fact::Conflicting{..}=>pack_reasons.push("pack_count_conflicting".into()),
    }
    let drained=line.demand.quantity.form==IngredientForm::Drained;
    let measure=if drained{&observed.drained}else{&observed.net};
    let supply=match measure {
        Fact::Known{value} if value.unit==line.demand.quantity.unit&&value.form==line.demand.quantity.form=>Some(value.clone()),
        Fact::Known{..}=>{pack_reasons.push("pack_measure_mismatch".into());None},
        Fact::Unknown{..}=>{pack_reasons.push(if drained{"pack_drained_unknown"}else{"pack_net_unknown"}.into());None},
        Fact::Conflicting{..}=>{pack_reasons.push(if drained{"pack_drained_conflicting"}else{"pack_net_conflicting"}.into());None},
    };
    pack_reasons.sort();pack_reasons.dedup();
    let pack_basis=if pack_reasons.is_empty(){
        let candidate=candidate.as_ref().ok_or_else(||error(ErrorCode::MissingReference,"/products/qualification/pack_candidate"))?;
        Some(ProductPackBasis{required:candidate.basis().line.quantity.clone(),supply:supply.ok_or_else(||error(ErrorCode::MissingReference,"/products/qualification/pack_supply"))?})
    }else{None};
    Ok(ProductMeaningQualification{format:"mstrchf-product-meaning-qualification".into(),version:1,request_id:prepared.request.request_id.clone(),product:b.product.clone(),line_id:b.line_id.clone(),
        evaluated_at:now.clone(),source_observed_at:item.observed_at.clone(),recheck_at,policy:trusted.metadata,status:status.into(),native_assessment:native_assessment.into(),reasons,match_basis_digest,pack_basis,pack_reasons,effect_enabled:false})
}
