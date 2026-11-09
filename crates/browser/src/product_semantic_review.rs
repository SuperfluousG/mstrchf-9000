//! Bounded, automated semantic proposals. Public methods and label facts are
//! separated from the retained owner-private occurrence graph. Correlation and
//! exact citations establish custody, not source truth, freshness, suitability
//! or ProductMeaningAdmission. Imported records require host quarantine.
use crate::{BrowserState, maker_packets, product_meaning_review,
    product_source_reads::{self, PreparedProductSourceRead, ProductSourceFood}};
use mstrchf_contracts::*;
use mstrchf_core::{food::{DemandKey, OccurrenceKey, occurrence_cmp},
    products::{ClaimState, CookingClaim, IdentityClaim, ProductMeaning, SourceField, meaning_digest,
        MatchPolicy,policy_digest,prepare_match_context,ProductReadBasis,assess_product_meaning,
        match_product,MatchAssessment,MatchReason}};
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

macro_rules! record {($name:ident {$($field:ident:$ty:ty),* $(,)?}) => {
    #[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
    #[serde(deny_unknown_fields)]
    pub struct $name {$(pub $field:$ty),*}
};}
record!(PublicSemanticProduct {product:ProductRef,url:HttpUrl,title:Text,pack:PackObservation,
    ingredients:Fact<Text>,allergens:Fact<Text>,storage:Fact<Text>,observed_at:Instant});
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PublicSemanticMethod {
    pub method_handle:String,pub recipe:RecipeRef,pub title:Title,pub source_url:Option<HttpUrl>,
    pub attribution:String,pub ingredient:Value,pub input_definition:Value,pub condition_text:Vec<String>,
    pub steps:Value,pub prep:Value,pub equipment:Value,pub component_production:Value,
    // Absent in all v1 bytes. V2 preserves the exact recipe-local input row and
    // its compiled table/custody, never a transported product approval.
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub intrinsic_conditions:Option<Value>,
    // Only v3 original chilli inputs inherited by exact remix methods.
    #[serde(default,skip_serializing_if="Option::is_none",deserialize_with="inherited_input")]
    pub inherited_input:Option<product_meaning_review::InheritedProductInput>,
}
fn inherited_input<'de,D:serde::Deserializer<'de>>(d:D)->Result<Option<product_meaning_review::InheritedProductInput>,D::Error>{
    product_meaning_review::InheritedProductInput::deserialize(d).map(Some)
}
record!(PublicSemanticRequest {format:String,version:u8,request_id:Text,input_digest:Digest,
    created_at:Instant,deadline:Instant,product:PublicSemanticProduct,methods:Vec<PublicSemanticMethod>});
#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(rename_all="snake_case")]
pub enum SemanticState {Supported,Refuted,Unknown,Contradictory}
#[derive(Clone,Copy,Debug,PartialEq,Eq,Serialize,Deserialize,JsonSchema)]
#[serde(rename_all="snake_case")]
pub enum CitationField {Ingredients,Allergens,Storage}
record!(SemanticCitation {field:CitationField,quote:String});
record!(SemanticDecision {state:SemanticState,citations:Vec<SemanticCitation>,reasoning:String});
record!(SemanticCookingDecision {method_handle:String,decision:SemanticDecision});
record!(PublicSemanticReturn {format:String,version:u8,request_id:Text,input_digest:Digest,
    identity:SemanticDecision,cooking:Vec<SemanticCookingDecision>});
record!(ProductMatchOccurrence {slot:SlotId,extra_id:Option<ExtraId>,recipe:RecipeRef});
impl ProductMatchOccurrence {
    fn native(&self)->OccurrenceKey {OccurrenceKey{slot:self.slot.clone(),extra_id:self.extra_id.clone(),recipe:self.recipe.clone()}}
}
record!(SemanticMethodMap {method_handle:String,occurrences:Vec<ProductMatchOccurrence>});
record!(SemanticBinding {week_id:WeekId,source_prepared:PreparedProductSourceRead,
    observation:ReadObservationV1,accepted:AcceptedReadRef,product:ProductRef,line_id:DemandLineId,
    draft:Value,method_map:Vec<SemanticMethodMap>});
record!(PreparedProductSemanticReview {format:String,version:u8,prepared_at:Instant,deadline:Instant,
    request:PublicSemanticRequest,binding:SemanticBinding,prepared_digest:Digest});
record!(SemanticReviewDocument {request_id:Text,input_digest:Digest,prepared_digest:Digest,
    source_observation_digest:Digest,product:ProductRef,line_id:DemandLineId,food_binding:ProductSourceFood,
    return_digest:Digest,returned:PublicSemanticReturn,reviewed_at:Instant,provenance:String,
    host_attested:bool,meaning:Value,meaning_digest:PayloadDigest});
record!(SemanticReviewPin {evidence_id:EvidenceId,revision:Revision,digest:Digest});
record!(SemanticAssessment {meaning_status:String,admission_status:String,native_reasons:Vec<String>,
    freshness_status:String,effect_enabled:bool});
record!(RecordedProductSemanticReview {format:String,version:u8,document:SemanticReviewDocument,
    review_pin:SemanticReviewPin,assessment:SemanticAssessment});
record!(SemanticInspection {current:bool});
record!(SemanticReturnCheck {return_digest:Digest});
record!(SemanticAudit {valid:bool,return_digest:Option<Digest>});

fn error(code:ErrorCode,path:&str)->DomainError {DomainError::new(code,path)}
fn require(ok:bool,code:ErrorCode,path:&str)->Result<(),DomainError>{if ok{Ok(())}else{Err(error(code,path))}}
fn shape(v:&impl ValidateShape)->Result<(),DomainError>{v.validate_shape().map_err(|_|error(ErrorCode::InvalidValue,"/products/semantic/shape"))}
fn encoded(v:&impl Serialize)->Result<Value,DomainError>{serde_json::to_value(v).map_err(|_|error(ErrorCode::InvalidValue,"/products/semantic/encoding"))}
fn parsed<T:serde::de::DeserializeOwned>(v:Value)->Result<T,DomainError>{serde_json::from_value(v).map_err(|_|error(ErrorCode::InvalidValue,"/products/semantic/retained"))}
fn size(v:&impl Serialize,max:usize)->Result<(),DomainError>{require(serde_json::to_vec(v).map_err(|_|error(ErrorCode::InvalidValue,"/products/semantic/encoding"))?.len()<=max,ErrorCode::LimitExceeded,"/products/semantic/size")}
fn digest_without(v:&impl Serialize,key:&str)->Result<Digest,DomainError>{let mut v=encoded(v)?;v.as_object_mut().ok_or_else(||error(ErrorCode::InvalidValue,"/products/semantic/digest"))?.remove(key);canonical_digest(&v)}
pub fn prepared_digest(v:&PreparedProductSemanticReview)->Result<Digest,DomainError>{digest_without(v,"prepared_digest")}
pub fn request_digest(v:&PublicSemanticRequest)->Result<Digest,DomainError>{digest_without(v,"input_digest")}
fn opaque(id:&Text)->bool {id.0.len()==35&&id.0.starts_with("sr_")&&id.0[3..].bytes().all(|b|b.is_ascii_hexdigit()&&!b.is_ascii_uppercase())}
fn handle(id:&Text,recipe:&RecipeRef)->Result<String,DomainError>{Ok(format!("mh_{}",&canonical_digest(&json!({"request_id":id,"recipe":recipe}))?.0[..32]))}
pub(crate) fn supported_packet_refs(version:u8)->Result<Vec<ReviewedRecipePacketRef>,DomainError>{
    match version {
        1=>Ok(vec![maker_packets::porridge_packet_ref()?,maker_packets::larger_porridge_packet_ref()?]),
        // Deliberately enumerated. A future registry append does not silently
        // gain semantic or maker recheck policy coverage.
        2=>Ok(vec![maker_packets::porridge_packet_ref()?,maker_packets::aglio_packet_ref()?,
            maker_packets::three_bean_packet_ref()?,maker_packets::three_bean_two_unit_packet_ref()?,
            maker_packets::three_bean_two_unit_vegetarian_packet_ref()?,maker_packets::larger_porridge_packet_ref()?,
            maker_packets::three_bean_two_unit_chicken_extra_packet_ref()?,maker_packets::chilled_chicken_extra_packet_ref()?,
            maker_packets::three_bean_smaller_packet_ref()?]),
        3=>{let mut refs=supported_packet_refs(2)?;refs.extend([maker_packets::chilli_batch_packet_ref()?,
            maker_packets::chilli_wraps_packet_ref()?,maker_packets::chilli_potato_packet_ref()?]);Ok(refs)},
        _=>Err(error(ErrorCode::UnsupportedVersion,"/products/semantic/version")),
    }
}
fn cold_input(method:&PublicSemanticMethod)->bool{
    matches!(method.ingredient["quantity"]["form"].as_str(),Some("drained"|"cooked"))
}

/// Rebuild public material from retained private native output and exact maker
/// bodies. This does not make a hash or imported draft into owner authority.
fn project(binding:&SemanticBinding,id:&Text,at:&Instant,deadline:&Instant,version:u8)
    ->Result<(PublicSemanticRequest,Vec<SemanticMethodMap>),DomainError>{
    shape(&binding.week_id)?;shape(&binding.product)?;shape(&binding.line_id)?;shape(id)?;shape(at)?;shape(deadline)?;
    require(opaque(id),ErrorCode::InvalidValue,"/products/semantic/request_id")?;
    require((1..=3600).contains(&(deadline.timestamp_seconds()-at.timestamp_seconds())),ErrorCode::InvalidValue,"/products/semantic/deadline")?;
    product_source_reads::audit(&binding.source_prepared,Some(&binding.observation),Some(&binding.accepted))?;
    require(binding.accepted.admitted_at<=*at,ErrorCode::InvalidValue,"/products/semantic/chronology")?;
    require(binding.source_prepared.request.week_id.0.as_ref()==Some(&binding.week_id),ErrorCode::IdentityMismatch,"/products/semantic/week")?;
    let ReadBody::Products{items,..}=&binding.observation.body else{return Err(error(ErrorCode::ScopeViolation,"/products/semantic/source"))};
    let item=items.iter().find(|i|i.product==binding.product).ok_or_else(||error(ErrorCode::MissingReference,"/products/semantic/product"))?;
    let d=&binding.draft;
    require(d["format"]=="mstrchf-product-meaning-review-draft"&&d["version"]==if version==3{2}else{1}
        &&d["food_binding"]==encoded(&binding.source_prepared.food)?&&d["source"]["item"]==encoded(item)?
        &&d["source"]["accepted_ref"]==encoded(&binding.accepted)?&&d["source"]["observation_digest"]==json!(binding.accepted.digest),
        ErrorCode::IdentityMismatch,"/products/semantic/draft")?;
    let demand:DemandLine=parsed(d["line"]["demand"].clone())?;shape(&demand)?;
    require(demand.line_id==binding.line_id&&(version>=2||(demand.ingredient_id.0=="ig_80000000000000000000000000000001"
        &&demand.quantity.unit==Unit::G&&demand.quantity.form==IngredientForm::Dry)),
        ErrorCode::UnsupportedCapability,"/products/semantic/input_scope")?;
    let food_proof=if version==3 {
        let proof:product_meaning_review::MeaningFoodProof=parsed(d["food_proof"].clone())?;
        product_meaning_review::validate_food_proof(&proof,&binding.source_prepared.food,&binding.week_id,&demand)?;
        require(d["context"]==encoded(&proof.derived.context_stamp)?,ErrorCode::IdentityMismatch,"/products/semantic/v3/context")?;Some(proof)
    } else {require(d.get("food_proof").is_none(),ErrorCode::ScopeViolation,"/products/semantic/legacy_food_proof")?;None};
    let uses=d["line"]["uses"].as_array().ok_or_else(||error(ErrorCode::MissingReference,"/products/semantic/uses"))?;
    let origins=d["line"]["origins"].as_array().ok_or_else(||error(ErrorCode::MissingReference,"/products/semantic/origins"))?;
    require(!uses.is_empty()&&!origins.is_empty()&&uses.len()<=4096&&origins.len()<=512,ErrorCode::LimitExceeded,"/products/semantic/graph")?;
    let mut graph=Vec::<ProductMatchOccurrence>::new();
    for use_ in uses {
        if version==1{require(use_["requirements"]==json!([]),ErrorCode::UnsupportedCapability,"/products/semantic/requirements")?;}
        else{let requirements:Vec<HardRequirement>=parsed(use_["requirements"].clone())?;require(requirements.len()<=64,ErrorCode::LimitExceeded,"/products/semantic/requirements")?;for requirement in requirements{shape(&requirement)?;}}
        let o=match use_["use_key"]["kind"].as_str(){Some("preparation")=>use_["use_key"]["data"]["phase"]["occurrence"].clone(),Some("consumption")=>use_["use_key"]["data"]["occurrence"].clone(),_=>return Err(error(ErrorCode::InvalidValue,"/products/semantic/use"))};
        graph.push(parsed(o)?);
    }
    for origin in origins {require(origin["ingredient_id"]==json!(demand.ingredient_id),ErrorCode::IdentityMismatch,"/products/semantic/origin")?;graph.push(parsed(origin["occurrence"].clone())?);}
    graph.sort_by(|a,b|occurrence_cmp(&a.native(),&b.native()));graph.dedup();
    let rows=d["occurrences"].as_array().ok_or_else(||error(ErrorCode::MissingReference,"/products/semantic/occurrences"))?;
    require(!rows.is_empty()&&rows.len()<=512,ErrorCode::LimitExceeded,"/products/semantic/occurrences")?;
    let allowed=supported_packet_refs(version)?;let mut methods=Vec::<PublicSemanticMethod>::new();let mut maps=Vec::<SemanticMethodMap>::new();let mut seen=Vec::new();
    for row in rows {
        let occurrence:ProductMatchOccurrence=parsed(row["occurrence"].clone())?;shape(&occurrence.slot)?;shape(&occurrence.recipe)?;
        require((version>=2||occurrence.extra_id.is_none())&&!seen.contains(&occurrence),ErrorCode::ScopeViolation,"/products/semantic/occurrence")?;seen.push(occurrence.clone());
        let packet_ref:ReviewedRecipePacketRef=parsed(row["packet_ref"].clone())?;
        require(allowed.contains(&packet_ref)&&occurrence.recipe==packet_ref.recipe,ErrorCode::UnsupportedCapability,"/products/semantic/recipe_scope")?;
        let packet=maker_packets::known_packet(&packet_ref)?;let recipe=packet.recipe();
        let (ingredient,inherited)=if let Some(proof)=&food_proof {
            product_meaning_review::v3_input(recipe,&packet_ref,&demand,&encoded(&occurrence)?,&d["line"],proof)?
        } else {require(row.get("inherited_input").is_none(),ErrorCode::ScopeViolation,"/products/semantic/legacy_inherited_input")?;
            (recipe.ingredients.iter().find(|i|i.ingredient_id==demand.ingredient_id).ok_or_else(||error(ErrorCode::MissingReference,"/products/semantic/ingredient"))?.clone(),None)};
        if version==3 {require(match &inherited{Some(v)=>row.get("inherited_input")==Some(&encoded(v)?),None=>row.get("inherited_input").is_none()},
            ErrorCode::IdentityMismatch,"/products/semantic/inherited_input")?;}
        let definitions:Vec<_>=packet.record().ingredient_registry["entries"].as_array().ok_or_else(||error(ErrorCode::MissingReference,"/products/semantic/definition"))?.iter()
            .filter(|v|v["ingredient_id"]==json!(demand.ingredient_id)&&v["unit"]==json!(demand.quantity.unit)&&v["form"]==json!(demand.quantity.form)).collect();
        require(definitions.len()==1,ErrorCode::MissingReference,"/products/semantic/definition")?;let definition=definitions[0];
        require(row["recipe"]==encoded(recipe)?&&row["ingredient"]==encoded(&ingredient)?&&row["input_definition"]==*definition,
            ErrorCode::DigestMismatch,"/products/semantic/method")?;
        let conditions=vec![definition["meaning"].as_str().ok_or_else(||error(ErrorCode::MissingReference,"/products/semantic/conditions"))?.to_owned()];
        let mut conditions=conditions;
        if let Some(v)=definition["product_constraints"].as_array(){for v in v{conditions.push(v.as_str().ok_or_else(||error(ErrorCode::InvalidValue,"/products/semantic/conditions"))?.to_owned());}}
        let mut intrinsic_conditions=None;
        if version>=2{
            if let Some(values)=packet.record().packet["operational_scope"]["actual_ingredient_conditions"].as_object(){
                for v in values.values(){conditions.push(v.as_str().ok_or_else(||error(ErrorCode::InvalidValue,"/products/semantic/conditions"))?.to_owned());}
            }
            if let Some(table)=packet.intrinsic_input_conditions(){
                let entries:Vec<_>=table["entries"].as_array().ok_or_else(||error(ErrorCode::MissingReference,"/products/semantic/intrinsic"))?.iter()
                    .filter(|v|v["ingredient_id"]==json!(demand.ingredient_id)&&v["unit"]==json!(demand.quantity.unit)&&v["form"]==json!(demand.quantity.form)).collect();
                require(entries.len()==1,ErrorCode::MissingReference,"/products/semantic/intrinsic")?;
                conditions.push(entries[0]["input_condition"].as_str().ok_or_else(||error(ErrorCode::MissingReference,"/products/semantic/conditions"))?.to_owned());
                intrinsic_conditions=Some(json!({"table":table,"entry":entries[0],"custody":packet.record().packet["intrinsic_input_custody"]}));
            }
            require(row["intrinsic_conditions"]==encoded(&intrinsic_conditions)?,ErrorCode::DigestMismatch,"/products/semantic/intrinsic")?;
            require(row["checklist_status"]==if intrinsic_conditions.is_some(){"pinned_structured_conditions"}else{"prose_only_requires_semantic_review"},ErrorCode::DigestMismatch,"/products/semantic/checklist")?;
        }
        // Both frozen oats packets retain their exact v1 prose. V2 separately
        // binds the current compiled method/input table without diet inference.
        require(row["condition_text"]==json!(conditions),ErrorCode::DigestMismatch,"/products/semantic/conditions")?;
        if version>=2&&matches!(demand.quantity.form,IngredientForm::Cooked|IngredientForm::Drained){
            conditions.push("Semantic review rule: a supported cold ready-to-eat input decision must cite exact known ingredients and storage/direct-use text. Unknown or conflicting storage stays unresolved; actual prior refrigeration, handling and separation are not observed.".into());
        }
        let mh=handle(id,&occurrence.recipe)?;
        if let Some(map)=maps.iter_mut().find(|m|m.method_handle==mh){map.occurrences.push(occurrence);}else{
            methods.push(PublicSemanticMethod{method_handle:mh.clone(),recipe:occurrence.recipe.clone(),title:recipe.title.clone(),source_url:recipe.source.url.0.clone(),
                attribution:recipe.text_rights.attribution.0.clone(),ingredient:encoded(&ingredient)?,input_definition:definition.clone(),condition_text:conditions,
                steps:encoded(&recipe.steps)?,prep:encoded(&recipe.prep)?,equipment:encoded(&recipe.equipment)?,component_production:encoded(&recipe.components)?,intrinsic_conditions,inherited_input:inherited});
            maps.push(SemanticMethodMap{method_handle:mh,occurrences:vec![occurrence]});
        }
    }
    seen.sort_by(|a,b|occurrence_cmp(&a.native(),&b.native()));
    require(seen==graph,ErrorCode::MissingReference,"/products/semantic/graph_closure")?;
    if let Some(proof)=&food_proof {if let Some(expected)=product_meaning_review::v3_original_occurrences(proof,&demand,&d["line"])? {
        let mut expected:Vec<ProductMatchOccurrence>=expected.into_iter().map(parsed).collect::<Result<_,_>>()?;
        expected.sort_by(|a,b|occurrence_cmp(&a.native(),&b.native()));
        require(seen==expected,ErrorCode::MissingReference,"/products/semantic/v3/inherited_graph_closure")?;
    }}
    for map in &mut maps {map.occurrences.sort_by(|a,b|occurrence_cmp(&a.native(),&b.native()));}
    methods.sort_by(|a,b|a.method_handle.cmp(&b.method_handle));maps.sort_by(|a,b|a.method_handle.cmp(&b.method_handle));
    require(methods.len()<=match version{1=>2,2=>9,3=>12,_=>0},ErrorCode::LimitExceeded,"/products/semantic/methods")?;
    let mut request=PublicSemanticRequest{format:"mstrchf-product-semantic-request".into(),version,request_id:id.clone(),input_digest:Digest("0".repeat(64)),
        created_at:at.clone(),deadline:deadline.clone(),product:PublicSemanticProduct{product:item.product.clone(),url:item.url.clone(),title:item.title.clone(),pack:item.pack.clone(),
            ingredients:item.ingredients.clone(),allergens:item.allergens.clone(),storage:item.storage.clone(),observed_at:item.observed_at.clone()},methods};
    request.input_digest=request_digest(&request)?;size(&request,512_000)?;Ok((request,maps))
}

pub fn prepare(state:&BrowserState,week_id:&WeekId,source_prepared:&PreparedProductSourceRead,
    observation:&ReadObservationV1,accepted:&AcceptedReadRef,product:&ProductRef,line_id:&DemandLineId,
    request_id:Text,now:&Instant,deadline:Instant)->Result<PreparedProductSemanticReview,DomainError>{
    let draft=encoded(&product_meaning_review::prepare(state,week_id,source_prepared,observation,accepted,product,line_id,now)?)?;
    prepare_with_draft(week_id,source_prepared,observation,accepted,product,line_id,request_id,now,deadline,draft,2)
}
pub fn prepare_v3(state:&BrowserState,week_id:&WeekId,source_prepared:&PreparedProductSourceRead,
    observation:&ReadObservationV1,accepted:&AcceptedReadRef,product:&ProductRef,line_id:&DemandLineId,
    request_id:Text,now:&Instant,deadline:Instant)->Result<PreparedProductSemanticReview,DomainError>{
    let draft=encoded(&product_meaning_review::prepare_v3(state,week_id,source_prepared,observation,accepted,product,line_id,now)?)?;
    prepare_with_draft(week_id,source_prepared,observation,accepted,product,line_id,request_id,now,deadline,draft,3)
}
pub(crate) fn prepare_unwitnessed(state:&BrowserState,week_id:&WeekId,source_prepared:&PreparedProductSourceRead,
    observation:&ReadObservationV1,correlation:&AcceptedReadRef,product:&ProductRef,line_id:&DemandLineId,
    request_id:Text,now:&Instant,deadline:Instant)->Result<PreparedProductSemanticReview,DomainError>{
    let draft=encoded(&product_meaning_review::prepare_unwitnessed(state,week_id,source_prepared,observation,correlation,product,line_id,now)?)?;
    prepare_with_draft(week_id,source_prepared,observation,correlation,product,line_id,request_id,now,deadline,draft,2)
}
pub(crate) fn prepare_unwitnessed_v3(state:&BrowserState,week_id:&WeekId,source_prepared:&PreparedProductSourceRead,
    observation:&ReadObservationV1,correlation:&AcceptedReadRef,product:&ProductRef,line_id:&DemandLineId,
    request_id:Text,now:&Instant,deadline:Instant)->Result<PreparedProductSemanticReview,DomainError>{
    let draft=encoded(&product_meaning_review::prepare_unwitnessed_v3(state,week_id,source_prepared,observation,correlation,product,line_id,now)?)?;
    prepare_with_draft(week_id,source_prepared,observation,correlation,product,line_id,request_id,now,deadline,draft,3)
}
fn prepare_with_draft(week_id:&WeekId,source_prepared:&PreparedProductSourceRead,observation:&ReadObservationV1,
    accepted:&AcceptedReadRef,product:&ProductRef,line_id:&DemandLineId,request_id:Text,now:&Instant,deadline:Instant,draft:Value,version:u8)
    ->Result<PreparedProductSemanticReview,DomainError>{
    let mut binding=SemanticBinding{week_id:week_id.clone(),source_prepared:source_prepared.clone(),observation:observation.clone(),accepted:accepted.clone(),product:product.clone(),line_id:line_id.clone(),draft,method_map:vec![]};
    let(request,map)=project(&binding,&request_id,now,&deadline,version)?;binding.method_map=map;
    let mut result=PreparedProductSemanticReview{format:"mstrchf-product-semantic-prepared".into(),version,prepared_at:now.clone(),deadline,request,binding,prepared_digest:Digest("0".repeat(64))};
    result.prepared_digest=prepared_digest(&result)?;size(&result,4_000_000)?;Ok(result)
}
fn internal(prepared:&PreparedProductSemanticReview)->Result<(),DomainError>{
    size(prepared,4_000_000)?;shape(&prepared.prepared_digest)?;
    require(prepared.format=="mstrchf-product-semantic-prepared"&&[1,2,3].contains(&prepared.version),ErrorCode::UnsupportedVersion,"/products/semantic/prepared")?;
    require(prepared.prepared_digest==prepared_digest(prepared)?,ErrorCode::DigestMismatch,"/products/semantic/prepared_digest")?;
    let(request,map)=project(&prepared.binding,&prepared.request.request_id,&prepared.prepared_at,&prepared.deadline,prepared.version)?;
    require(request==prepared.request&&map==prepared.binding.method_map,ErrorCode::DigestMismatch,"/products/semantic/request")
}
pub fn inspect(state:&BrowserState,prepared:&PreparedProductSemanticReview,now:&Instant)->Result<SemanticInspection,DomainError>{
    internal(prepared)?;shape(now)?;
    require(state.snapshot.privacy.use_ai_features,ErrorCode::AuthorityDenied,"/products/semantic/ai_features")?;
    require(prepared.prepared_at<=*now&&*now<prepared.deadline,ErrorCode::ContextStale,"/products/semantic/task_deadline")?;
    let b=&prepared.binding;let current=current_draft(state,prepared,now)?;
    require(current==b.draft,ErrorCode::ContextStale,"/products/semantic/food")?;Ok(SemanticInspection{current:true})
}
/// Local hypothetical draft validation; never an assisted handoff capability.
pub(crate) fn inspect_unwitnessed(state:&BrowserState,prepared:&PreparedProductSemanticReview,now:&Instant)->Result<(),DomainError>{
    inspect_unwitnessed_version(state,prepared,now,2)
}
pub(crate) fn inspect_unwitnessed_v3(state:&BrowserState,prepared:&PreparedProductSemanticReview,now:&Instant)->Result<(),DomainError>{
    inspect_unwitnessed_version(state,prepared,now,3)
}
fn inspect_unwitnessed_version(state:&BrowserState,prepared:&PreparedProductSemanticReview,now:&Instant,version:u8)->Result<(),DomainError>{
    internal(prepared)?;shape(now)?;
    require(prepared.version==version&&prepared.prepared_at<=*now&&*now<prepared.deadline,ErrorCode::ContextStale,"/products/draft/deadline")?;
    let b=&prepared.binding;
    require(prepared.deadline==b.source_prepared.request.expires_at&&b.accepted.admitted_at==prepared.prepared_at,
        ErrorCode::IdentityMismatch,"/products/draft/chronology")?;
    let expected=product_source_reads::unwitnessed_correlation(state,&b.source_prepared,&b.observation,&b.accepted.admitted_at)?;
    require(expected==b.accepted,ErrorCode::IdentityMismatch,"/products/draft/correlation")?;
    let current=encoded(&if version==3{product_meaning_review::prepare_unwitnessed_v3(state,&b.week_id,&b.source_prepared,&b.observation,&b.accepted,&b.product,&b.line_id,now)?}
        else{product_meaning_review::prepare_unwitnessed(state,&b.week_id,&b.source_prepared,&b.observation,&b.accepted,&b.product,&b.line_id,now)?})?;
    require(current==b.draft,ErrorCode::ContextStale,"/products/draft/food")?;Ok(())
}
pub(crate) fn unwitnessed_record_and_meaning(state:&BrowserState,prepared:&PreparedProductSemanticReview,r:&PublicSemanticReturn,
    evidence_id:EvidenceId,now:&Instant)->Result<(RecordedProductSemanticReview,ProductMeaning),DomainError>{
    inspect_unwitnessed(state,prepared,now)?;
    let saved=record(prepared,r,evidence_id.clone(),now)?;
    let interpreted=meaning(prepared,r,&evidence_id)?;
    Ok((saved,interpreted))
}
pub(crate) fn unwitnessed_record_and_meaning_v3(state:&BrowserState,prepared:&PreparedProductSemanticReview,r:&PublicSemanticReturn,
    evidence_id:EvidenceId,now:&Instant)->Result<(RecordedProductSemanticReview,ProductMeaning),DomainError>{
    inspect_unwitnessed_v3(state,prepared,now)?;
    let saved=record(prepared,r,evidence_id.clone(),now)?;
    let interpreted=meaning(prepared,r,&evidence_id)?;
    Ok((saved,interpreted))
}
fn fact<'a>(p:&'a PublicSemanticProduct,f:CitationField)->&'a Fact<Text>{match f{CitationField::Ingredients=>&p.ingredients,CitationField::Allergens=>&p.allergens,CitationField::Storage=>&p.storage}}
fn field(f:CitationField)->SourceField{match f{CitationField::Ingredients=>SourceField::Ingredients,CitationField::Allergens=>SourceField::Allergens,CitationField::Storage=>SourceField::Storage}}
fn state(s:SemanticState)->ClaimState{match s{SemanticState::Supported=>ClaimState::Supported,SemanticState::Refuted=>ClaimState::Refuted,SemanticState::Unknown=>ClaimState::Unknown,SemanticState::Contradictory=>ClaimState::Contradictory}}
fn decision(d:&SemanticDecision,p:&PublicSemanticProduct)->Result<Vec<SourceField>,DomainError>{
    require(!d.reasoning.trim().is_empty()&&d.reasoning.len()<=4096&&d.citations.len()<=8,ErrorCode::LimitExceeded,"/products/semantic/decision")?;
    let mut fields=vec![SourceField::Ingredients];let mut cited=Vec::new();
    for c in &d.citations {
        require(!c.quote.is_empty()&&c.quote.len()<=16384&&!cited.contains(&c.field),ErrorCode::InvalidValue,"/products/semantic/citation")?;cited.push(c.field);
        require(matches!(fact(p,c.field),Fact::Known{value} if value.0==c.quote),ErrorCode::IdentityMismatch,"/products/semantic/known_citation")?;fields.push(field(c.field));
    }
    if matches!(d.state,SemanticState::Supported|SemanticState::Refuted){require(cited.contains(&CitationField::Ingredients),ErrorCode::MissingReference,"/products/semantic/ingredients_citation")?;}
    if d.state==SemanticState::Contradictory {
        let mut conflicting=false;for f in [CitationField::Ingredients,CitationField::Allergens,CitationField::Storage]{if matches!(fact(p,f),Fact::Conflicting{..}){conflicting=true;fields.push(field(f));}}
        require(conflicting||!cited.is_empty(),ErrorCode::MissingReference,"/products/semantic/contradictory_dependency")?;
    }
    fields.sort();fields.dedup();Ok(fields)
}
fn returned(prepared:&PreparedProductSemanticReview,r:&PublicSemanticReturn)->Result<Digest,DomainError>{
    size(r,250_000)?;require(r.format=="mstrchf-product-semantic-return"&&r.version==1,ErrorCode::UnsupportedVersion,"/products/semantic/return")?;
    require(r.request_id==prepared.request.request_id&&r.input_digest==prepared.request.input_digest,ErrorCode::IdentityMismatch,"/products/semantic/correlation")?;
    decision(&r.identity,&prepared.request.product)?;
    require(r.cooking.len()==prepared.request.methods.len(),ErrorCode::MissingReference,"/products/semantic/cooking_closure")?;
    let mut seen=Vec::new();for c in &r.cooking {require(!seen.contains(&c.method_handle)&&prepared.request.methods.iter().any(|m|m.method_handle==c.method_handle),ErrorCode::IdentityMismatch,"/products/semantic/method_handle")?;seen.push(c.method_handle.clone());decision(&c.decision,&prepared.request.product)?;
        if prepared.version>=2&&prepared.request.methods.iter().any(|method|method.method_handle==c.method_handle&&cold_input(method))&&matches!(c.decision.state,SemanticState::Supported|SemanticState::Refuted){require(c.decision.citations.iter().any(|citation|citation.field==CitationField::Storage),ErrorCode::MissingReference,"/products/semantic/cold_input_storage_citation")?;}
    }
    canonical_digest(r)
}
pub fn check_return(state:&BrowserState,prepared:&PreparedProductSemanticReview,r:&PublicSemanticReturn,now:&Instant)->Result<SemanticReturnCheck,DomainError>{inspect(state,prepared,now)?;Ok(SemanticReturnCheck{return_digest:returned(prepared,r)?})}

fn meaning(prepared:&PreparedProductSemanticReview,r:&PublicSemanticReturn,id:&EvidenceId)->Result<ProductMeaning,DomainError>{
    shape(id)?;let demand:DemandLine=parsed(prepared.binding.draft["line"]["demand"].clone())?;
    let mut cooking=Vec::new();for map in &prepared.binding.method_map {
        let d=&r.cooking.iter().find(|c|c.method_handle==map.method_handle).ok_or_else(||error(ErrorCode::MissingReference,"/products/semantic/method_handle"))?.decision;
        let mut fields=decision(d,&prepared.request.product)?;
        if prepared.version>=2&&prepared.request.methods.iter().any(|method|method.method_handle==map.method_handle&&cold_input(method)){
            if matches!(d.state,SemanticState::Supported|SemanticState::Refuted){require(d.citations.iter().any(|c|c.field==CitationField::Storage),ErrorCode::MissingReference,"/products/semantic/cold_input_storage_citation")?;}
            fields.push(SourceField::Storage);fields.sort();fields.dedup();
        }
        for o in &map.occurrences {cooking.push(CookingClaim{recipe:o.recipe.clone(),occurrence:o.native(),state:state(d.state),evidence_ids:vec![id.clone()],source_fields:fields.clone()});}
    }
    cooking.sort_by(|a,b|occurrence_cmp(&a.occurrence,&b.occurrence));
    let result=ProductMeaning{product:prepared.binding.product.clone(),observation_digest:prepared.binding.accepted.digest.clone(),key_claim:IdentityClaim{
        supply_key:DemandKey{ingredient_id:demand.ingredient_id,unit:demand.quantity.unit,form:demand.quantity.form},state:state(r.identity.state),
        evidence_ids:vec![id.clone()],source_fields:decision(&r.identity,&prepared.request.product)?},requirement_claims:vec![],cooking_claims:cooking};
    meaning_digest(&result)?;Ok(result)
}
fn material(m:&ProductMeaning)->Value{
    fn cs(s:ClaimState)->Value{json!({"kind":match s{ClaimState::Supported=>"supported",ClaimState::Refuted=>"refuted",ClaimState::Unknown=>"unknown",ClaimState::Contradictory=>"contradictory"}})}
    fn fs(f:&[SourceField])->Value{json!(f.iter().map(|f|json!({"kind":match f{SourceField::Ingredients=>"ingredients",SourceField::Allergens=>"allergens",SourceField::Storage=>"storage"}})).collect::<Vec<_>>())}
    json!({"product":m.product,"observation_digest":m.observation_digest,"key_claim":{"supply_key":{"ingredient_id":m.key_claim.supply_key.ingredient_id,"unit":m.key_claim.supply_key.unit,"form":m.key_claim.supply_key.form},"state":cs(m.key_claim.state),"evidence_ids":m.key_claim.evidence_ids,"source_fields":fs(&m.key_claim.source_fields)},
        "requirement_claims":[],"cooking_claims":m.cooking_claims.iter().map(|c|json!({"recipe":c.recipe,"occurrence":{"slot":c.occurrence.slot,"extra_id":c.occurrence.extra_id,"recipe":c.occurrence.recipe},"state":cs(c.state),"evidence_ids":c.evidence_ids,"source_fields":fs(&c.source_fields)})).collect::<Vec<_>>()})
}
fn record(prepared:&PreparedProductSemanticReview,r:&PublicSemanticReturn,id:EvidenceId,at:&Instant)->Result<RecordedProductSemanticReview,DomainError>{
    shape(at)?;require(prepared.prepared_at<=*at&&*at<prepared.deadline,ErrorCode::ContextStale,"/products/semantic/review_chronology")?;
    let rd=returned(prepared,r)?;let m=meaning(prepared,r,&id)?;
    let document=SemanticReviewDocument{request_id:prepared.request.request_id.clone(),input_digest:prepared.request.input_digest.clone(),prepared_digest:prepared.prepared_digest.clone(),
        source_observation_digest:prepared.binding.accepted.digest.clone(),product:prepared.binding.product.clone(),line_id:prepared.binding.line_id.clone(),food_binding:prepared.binding.source_prepared.food.clone(),return_digest:rd,
        returned:r.clone(),reviewed_at:at.clone(),provenance:"automated_model_proposal".into(),host_attested:false,meaning:material(&m),meaning_digest:meaning_digest(&m)?};
    let review_pin=SemanticReviewPin{evidence_id:id,revision:Revision(1),digest:canonical_digest(&document)?};
    Ok(RecordedProductSemanticReview{format:"mstrchf-product-semantic-review-record".into(),version:prepared.version,document,review_pin,
        assessment:SemanticAssessment{meaning_status:"proposed".into(),admission_status:"missing".into(),native_reasons:vec!["missing_admission".into()],freshness_status:"unknown".into(),effect_enabled:false}})
}
pub fn evaluate(state:&BrowserState,prepared:&PreparedProductSemanticReview,r:&PublicSemanticReturn,evidence_id:EvidenceId,now:&Instant)->Result<RecordedProductSemanticReview,DomainError>{
    check_return(state,prepared,r,now)?;
    let b=&prepared.binding;let view=crate::runtime_catalogue(state)?;
    let profile=state.snapshot.profile.0.as_ref().ok_or_else(||error(ErrorCode::MissingField,"/products/semantic/profile"))?;
    let week=state.snapshot.weeks.iter().find(|w|w.range.week_id==b.week_id).ok_or_else(||error(ErrorCode::MissingReference,"/products/semantic/week"))?;
    let (basis,food)=crate::calendar_view::material_at(&view,&state.snapshot,profile,week,now)?.ok_or_else(||error(ErrorCode::MissingReference,"/products/semantic/food"))?;
    let mut policy=MatchPolicy{version:Revision(1),digest:PayloadDigest("0".repeat(64)),rules:vec![]};policy.digest=policy_digest(&policy)?;
    let context=prepare_match_context(&basis.context,&basis,&food.plan,&policy,now.clone())?;
    let source=ProductReadBasis{request:&b.source_prepared.request,expected_reader:&b.source_prepared.expected_reader,observation:&b.observation,accepted:&b.accepted,product:b.product.clone()};
    let m=meaning(prepared,r,&evidence_id)?;
    let reviewed=assess_product_meaning(&source,Some(&m),None,&context)?;
    require(matches!(match_product(&context,&b.line_id,&reviewed)?,MatchAssessment::Unresolved(issues) if issues.iter().any(|i|i.reason==MatchReason::MissingAdmission)),ErrorCode::AuthorityDenied,"/products/semantic/unadmitted")?;
    record(prepared,r,evidence_id,now)
}
/// Historical custody only. No current state/clock test, host attestation or
/// revived approval is implied. The owner must retain/quarantine this record.
pub fn audit(prepared:&PreparedProductSemanticReview,r:Option<&PublicSemanticReturn>,recorded:Option<&RecordedProductSemanticReview>)->Result<SemanticAudit,DomainError>{
    internal(prepared)?;let digest=r.map(|r|returned(prepared,r)).transpose()?;
    match (r,recorded){(None,Some(_))=>return Err(error(ErrorCode::MissingReference,"/products/semantic/return")),(Some(r),Some(saved))=>{
        let expected=record(prepared,r,saved.review_pin.evidence_id.clone(),&saved.document.reviewed_at)?;
        require(&expected==saved,ErrorCode::DigestMismatch,"/products/semantic/record")?;
    },_=>{}}
    Ok(SemanticAudit{valid:true,return_digest:digest})
}

/// Local conditional assessment may happen after the completed model task's
/// deadline. Historical review custody and the complete CURRENT food binding
/// still apply. This helper supplies no source freshness or owner acceptance.
pub(crate) fn recorded_meaning(state:&BrowserState,prepared:&PreparedProductSemanticReview,
    recorded:&RecordedProductSemanticReview,now:&Instant)->Result<ProductMeaning,DomainError>{
    audit(prepared,Some(&recorded.document.returned),Some(recorded))?;shape(now)?;
    require(recorded.document.reviewed_at<=*now,ErrorCode::InvalidValue,"/products/semantic/review_chronology")?;
    let b=&prepared.binding;
    let current=current_draft(state,prepared,now)?;
    require(current==b.draft,ErrorCode::ContextStale,"/products/semantic/food")?;
    meaning(prepared,&recorded.document.returned,&recorded.review_pin.evidence_id)
}
fn current_draft(state:&BrowserState,prepared:&PreparedProductSemanticReview,now:&Instant)->Result<Value,DomainError>{
    let b=&prepared.binding;
    encoded(&if prepared.version==3 {product_meaning_review::prepare_v3(state,&b.week_id,&b.source_prepared,&b.observation,&b.accepted,&b.product,&b.line_id,now)?}
        else {product_meaning_review::prepare(state,&b.week_id,&b.source_prepared,&b.observation,&b.accepted,&b.product,&b.line_id,now)?})
}
