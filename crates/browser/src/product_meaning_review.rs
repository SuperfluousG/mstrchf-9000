//! Owner-private preparation for a later semantic review. Source correlation,
//! exact native uses and pinned recipe conditions are readable inputs, not a
//! ProductMeaningAdmission, freshness rule, matched candidate or permission.
//! The host must select an actually accepted, nonquarantined canonical record;
//! these pure functions cannot establish that a human witnessed its facts.
use crate::{BrowserState, maker_packets, personal_scope, product_match_context,
    product_source_reads::{self, PreparedProductSourceRead, ProductSourceFood}};
use mstrchf_contracts::*;
use serde::{Deserialize,Serialize};
use serde_json::{Value, json};

#[derive(Serialize)]
pub struct ProductMeaningReviewDraft {
    format: &'static str,
    version: u8,
    food_binding: ProductSourceFood,
    context: Value,
    source: Source,
    line: Value,
    occurrences: Vec<OccurrenceReview>,
    field_dependencies: Vec<FieldDependency>,
    pack_comparison: PackComparison,
    meaning_status: &'static str,
    freshness: Freshness,
    admission_status: &'static str,
    match_status: &'static str,
    purchase_readiness: &'static str,
    effect_enabled: bool,
    // Version 2 private draft only. Never sent in the public method request.
    #[serde(skip_serializing_if="Option::is_none")]
    food_proof: Option<MeaningFoodProof>,
}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize)]
#[serde(deny_unknown_fields)]
pub(crate) struct MeaningFoodProof {pub plan:WeekPlan,pub derived:DerivedPlan}
#[derive(Clone,Debug,PartialEq,Eq,Serialize,Deserialize,schemars::JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct InheritedProductInput {pub producer_packet:ReviewedRecipePacketRef,pub component_id:ComponentId}
#[derive(Serialize)]
struct Source {
    read_id: ReadId,
    observation_digest: Digest,
    accepted_ref: AcceptedReadRef,
    item: ProductObservation,
    provenance: &'static str,
    host_attested: bool,
    source_truth_assessed: bool,
    currentness: &'static str,
}
#[derive(Serialize)]
struct OccurrenceReview {
    occurrence: Value,
    recipe: RecipeRevision,
    ingredient: Value,
    packet_ref: ReviewedRecipePacketRef,
    input_definition: Option<Value>,
    intrinsic_conditions: Option<Value>,
    condition_text: Vec<String>,
    checklist_status: &'static str,
    scope_notices: Vec<String>,
    evidence_refs: Vec<EvidenceRef>,
    #[serde(skip_serializing_if="Option::is_none")]
    inherited_input: Option<InheritedProductInput>,
}
#[derive(Serialize)]
struct EvidenceRef { evidence_id: EvidenceId, revision: Revision, digest: Digest }
#[derive(Serialize)]
struct FieldDependency { field: &'static str, fact: Fact<Text>, status: &'static str }
#[derive(Serialize)]
struct PackComparison { status: &'static str, measure: &'static str, observed: Fact<Quantity> }
#[derive(Serialize)]
struct Freshness { status: &'static str, valid_until: Option<Instant> }

fn error(code: ErrorCode, path: &str) -> DomainError { DomainError::new(code, path) }
fn require(ok: bool, code: ErrorCode, path: &str) -> Result<(), DomainError> {
    if ok { Ok(()) } else { Err(error(code,path)) }
}
fn shape(value: &impl ValidateShape) -> Result<(), DomainError> {
    value.validate_shape().map_err(|_|error(ErrorCode::InvalidValue,"/products/review/selector"))
}
fn encoded(value: &impl Serialize) -> Result<Value, DomainError> {
    serde_json::to_value(value).map_err(|_|error(ErrorCode::InvalidValue,"/products/review/encoding"))
}
fn dependency(field: &'static str, fact: &Fact<Text>) -> FieldDependency {
    let status=match fact { Fact::Known{..}=>"known", Fact::Unknown{..}=>"unknown", Fact::Conflicting{..}=>"conflicting" };
    FieldDependency {field,fact:fact.clone(),status}
}
fn prose(definition: Option<&Value>, scope: &Value) -> Vec<String> {
    let mut lines=Vec::new();
    if let Some(definition)=definition {
        if let Some(meaning)=definition.get("meaning").and_then(Value::as_str) {lines.push(meaning.into());}
        if let Some(constraints)=definition.get("product_constraints").and_then(Value::as_array) {
            lines.extend(constraints.iter().filter_map(Value::as_str).map(String::from));
        }
    }
    // Retain adopted operational prose; never infer predicates or a Supported
    // claim from it. Full steps/prep and scope notices remain beside these lines.
    if let Some(conditions)=scope.get("actual_ingredient_conditions").and_then(Value::as_object) {
        lines.extend(conditions.values().filter_map(Value::as_str).map(String::from));
    }
    lines
}

/// The caller passes selectors and an independently retained source trio only.
/// Current quantities, consumers, occurrences, origins and recipe evidence are
/// reconstructed here, never admitted from a presentation DTO or source note.
pub fn prepare(state: &BrowserState, week_id: &WeekId,
    prepared: &PreparedProductSourceRead, observation: &ReadObservationV1,
    accepted: &AcceptedReadRef, product: &ProductRef, line_id: &DemandLineId,
    now: &Instant) -> Result<ProductMeaningReviewDraft, DomainError> {
    prepare_with_provenance(state,week_id,prepared,observation,accepted,product,line_id,now,"retained_source_correlation_only",false)
}
pub fn prepare_v3(state:&BrowserState,week_id:&WeekId,prepared:&PreparedProductSourceRead,
    observation:&ReadObservationV1,accepted:&AcceptedReadRef,product:&ProductRef,line_id:&DemandLineId,now:&Instant)
    ->Result<ProductMeaningReviewDraft,DomainError>{
    prepare_with_provenance(state,week_id,prepared,observation,accepted,product,line_id,now,"retained_source_correlation_only",true)
}
/// Same native graph/recipe authority, but expressly unwitnessed source facts.
pub(crate) fn prepare_unwitnessed(state:&BrowserState,week_id:&WeekId,prepared:&PreparedProductSourceRead,
    observation:&ReadObservationV1,correlation:&AcceptedReadRef,product:&ProductRef,line_id:&DemandLineId,now:&Instant)
    ->Result<ProductMeaningReviewDraft,DomainError>{
    prepare_with_provenance(state,week_id,prepared,observation,correlation,product,line_id,now,"external_chat_unwitnessed_correlation",false)
}
/// Explicit v3 hypothetical route: retain the original-input proof without
/// converting a correlated external report into witnessed source history.
pub(crate) fn prepare_unwitnessed_v3(state:&BrowserState,week_id:&WeekId,prepared:&PreparedProductSourceRead,
    observation:&ReadObservationV1,correlation:&AcceptedReadRef,product:&ProductRef,line_id:&DemandLineId,now:&Instant)
    ->Result<ProductMeaningReviewDraft,DomainError>{
    prepare_with_provenance(state,week_id,prepared,observation,correlation,product,line_id,now,"external_chat_unwitnessed_correlation",true)
}
fn prepare_with_provenance(state:&BrowserState,week_id:&WeekId,prepared:&PreparedProductSourceRead,
    observation:&ReadObservationV1,accepted:&AcceptedReadRef,product:&ProductRef,line_id:&DemandLineId,now:&Instant,provenance:&'static str,v3:bool)
    ->Result<ProductMeaningReviewDraft,DomainError>{
    shape(week_id)?;shape(product)?;shape(line_id)?;shape(now)?;
    require(prepared.request.week_id.0.as_ref()==Some(week_id),ErrorCode::IdentityMismatch,"/products/review/week")?;
    // Audit at the original acceptance chronology. A request deadline governs
    // returning/witnessing the read; it is never product or meaning expiry.
    product_source_reads::audit(prepared,Some(observation),Some(accepted))?;
    require(&accepted.admitted_at<=now,ErrorCode::InvalidValue,"/products/review/chronology")?;
    let ReadBody::Products{items,..}=&observation.body else {
        return Err(error(ErrorCode::ScopeViolation,"/products/review/source"));
    };
    let item=items.iter().find(|item|&item.product==product)
        .ok_or_else(||error(ErrorCode::MissingReference,"/products/review/product"))?;

    // Reuse the existing strict native preflight and its exact graph projection.
    // This Value originates solely inside the native owner, not deserialised IO.
    let old_clock=crate::CLOCK.with(|clock|clock.replace(Some(now.clone())));
    let projected=product_match_context::project(state,week_id,now);
    crate::CLOCK.with(|clock|*clock.borrow_mut()=old_clock);
    let context=encoded(&projected?)?;
    let current_food=json!({"snapshot_revision":context["snapshot_revision"],"week_revision":context["week_revision"],
        "saved_basis_digest":context["saved_basis_digest"],"derived_digest":context["derived_digest"],
        "demand_digest":context["demand_digest"],"catalogue_pin":context["catalogue_pin"],
        "contains_engineering":context["contains_engineering"]});
    require(current_food==encoded(&prepared.food)?,ErrorCode::ContextStale,"/products/review/food")?;
    let line=context["lines"].as_array().and_then(|lines|lines.iter().find(|line|line["demand"]["line_id"]==json!(line_id)))
        .ok_or_else(||error(ErrorCode::MissingReference,"/products/review/line"))?;
    let demand: DemandLine=serde_json::from_value(line["demand"].clone())
        .map_err(|_|error(ErrorCode::InvalidValue,"/products/review/native_line"))?;
    let view=crate::runtime_catalogue(state)?;
    let food_proof=if v3 {
        let week=state.snapshot.weeks.iter().find(|week|&week.range.week_id==week_id)
            .ok_or_else(||error(ErrorCode::MissingReference,"/products/review/week"))?;
        let profile=state.snapshot.profile.0.as_ref().ok_or_else(||error(ErrorCode::MissingField,"/products/review/profile"))?;
        let (_,food)=crate::calendar_view::material_at(&view,&state.snapshot,profile,week,now)?
            .ok_or_else(||error(ErrorCode::MissingReference,"/products/review/food"))?;
        let proof=MeaningFoodProof{plan:week.plan.clone(),derived:food.plan};
        validate_food_proof(&proof,&prepared.food,week_id,&demand)?;Some(proof)
    } else {None};
    let mut occurrences=Vec::new();
    // The preflight already takes the union of production origins and all
    // preparation/consumption occurrences, with native ordering/deduplication.
    for occurrence in line["cooking_occurrences"].as_array()
        .ok_or_else(||error(ErrorCode::MissingReference,"/products/review/occurrences"))? {
        let reference: RecipeRef=serde_json::from_value(occurrence["recipe"].clone())
            .map_err(|_|error(ErrorCode::InvalidValue,"/products/review/native_recipe"))?;
        let recipe=view.resolve_recipe(&reference)?;
        // Synthetic engineering material stays outside production review. This
        // is a bounded compiled-personal-recipe adapter, not matcher admission.
        require(view.is_personal_recipe(&reference),ErrorCode::UnsupportedCapability,"/products/review/recipe_scope")?;
        let entry=view.pin().entries.iter().find(|entry|entry.recipe==reference)
            .ok_or_else(||error(ErrorCode::MissingReference,"/products/review/packet"))?;
        let packet_ref=ReviewedRecipePacketRef{recipe:entry.recipe.clone(),packet_digest:entry.packet_digest.clone()};
        let packet=maker_packets::known_packet(&packet_ref)?;
        let (ingredient,inherited_input)=if let Some(proof)=&food_proof {
            v3_input(recipe,&packet_ref,&demand,occurrence,line,proof)?
        } else {
            let ingredients:Vec<_>=recipe.ingredients.iter().filter(|ingredient|ingredient.ingredient_id==demand.ingredient_id).collect();
            require(ingredients.len()==1,ErrorCode::MissingReference,"/products/review/ingredient" )?;
            (ingredients[0].clone(),None)
        };
        let definitions=packet.record().ingredient_registry["entries"].as_array()
            .ok_or_else(||error(ErrorCode::MissingReference,"/products/review/registry"))?;
        let definitions:Vec<_>=definitions.iter().filter(|entry|entry["ingredient_id"]==json!(demand.ingredient_id)
            && entry["unit"]==json!(demand.quantity.unit)&&entry["form"]==json!(demand.quantity.form)).collect();
        require(definitions.len()==1,ErrorCode::MissingReference,"/products/review/definition" )?;
        let definition=definitions[0];
        let mut condition_text=prose(Some(definition),&packet.record().packet["operational_scope"]);
        let intrinsic=if let Some(table)=packet.intrinsic_input_conditions() {
            let matches:Vec<_>=table["entries"].as_array()
                .ok_or_else(||error(ErrorCode::MissingReference,"/products/review/intrinsic"))?.iter()
                .filter(|entry|entry["ingredient_id"]==json!(demand.ingredient_id)
                    && entry["unit"]==json!(demand.quantity.unit)&&entry["form"]==json!(demand.quantity.form)).collect();
            require(matches.len()==1,ErrorCode::MissingReference,"/products/review/intrinsic" )?;
            if let Some(text)=matches[0]["input_condition"].as_str() {condition_text.push(text.into());}
            Some(json!({"table":table,"entry":matches[0],"custody":packet.record().packet["intrinsic_input_custody"]}))
        } else {None};
        let checklist_status=if intrinsic.is_some(){"pinned_structured_conditions"}else{"prose_only_requires_semantic_review"};
        occurrences.push(OccurrenceReview {occurrence:occurrence.clone(),recipe:recipe.clone(),ingredient:encoded(&ingredient)?,
            packet_ref,input_definition:Some(definition.clone()),intrinsic_conditions:intrinsic,condition_text,checklist_status,
            scope_notices:personal_scope::notices_for_recipe(&reference.recipe_id),evidence_refs:packet.basis().admitted.iter()
                .map(|pin|EvidenceRef{evidence_id:pin.evidence_id.clone(),revision:pin.revision,digest:pin.digest.clone()}).collect(),inherited_input});
    }
    require(!occurrences.is_empty(),ErrorCode::MissingReference,"/products/review/occurrences")?;
    let (measure,observed)=if demand.quantity.form==IngredientForm::Drained {("drained",&item.pack.drained)}else{("net",&item.pack.net)};
    let pack_status=match observed {
        Fact::Known{value} if value.unit==demand.quantity.unit&&value.form==demand.quantity.form=>"comparable_declared_measure",
        Fact::Known{..}=>"known_measure_mismatch", Fact::Unknown{..}=>"unknown", Fact::Conflicting{..}=>"conflicting",
    };
    Ok(ProductMeaningReviewDraft {format:"mstrchf-product-meaning-review-draft",version:if v3{2}else{1},food_binding:prepared.food.clone(),
        context:context["context"].clone(),source:Source{read_id:accepted.read_id.clone(),observation_digest:accepted.digest.clone(),
            accepted_ref:accepted.clone(),item:item.clone(),provenance,host_attested:false,
            source_truth_assessed:false,currentness:"not_assessed"},
        line:json!({"demand":line["demand"],"shopping_action":line["shopping_action"],"uses":line["uses"],"origins":line["origins"]}),
        occurrences,field_dependencies:vec![dependency("ingredients",&item.ingredients),dependency("allergens",&item.allergens),dependency("storage",&item.storage)],
        pack_comparison:PackComparison{status:pack_status,measure,observed:observed.clone()},meaning_status:"not_reviewed",
        freshness:Freshness{status:"unknown",valid_until:None},admission_status:"missing",match_status:"not_assessed",
        purchase_readiness:"not_assessed",effect_enabled:false,food_proof})
}

/// Retained custody is checked against the original source binding. Current
/// acceptance additionally reconstructs this entire proof from audited state.
pub(crate) fn validate_food_proof(proof:&MeaningFoodProof,food:&ProductSourceFood,week:&WeekId,demand:&DemandLine)
    ->Result<(),DomainError>{
    shape(&proof.plan)?;shape(&proof.derived)?;
    let d=&proof.derived;
    require(&proof.plan.week_id==week&&&d.week_id==week&&proof.plan.revision==d.plan_revision,
        ErrorCode::IdentityMismatch,"/products/review/v3/plan")?;
    require(plan_input_digest(&PlanInputMaterial{schema_version:SchemaVersion,week_plan:proof.plan.clone(),context_stamp:d.context_stamp.clone()})?==d.input_plan_digest,
        ErrorCode::DigestMismatch,"/products/review/v3/plan_digest")?;
    require(derived_digest(&DerivedMaterial::from(d))?==d.digest&&d.digest==food.derived_digest
        &&d.demand.digest==food.demand_digest&&d.demand.input_plan_digest==d.input_plan_digest,
        ErrorCode::DigestMismatch,"/products/review/v3/derived_digest")?;
    require(d.demand.lines.iter().filter(|line|line.line_id==demand.line_id).count()==1
        &&d.demand.lines.iter().any(|line|line==demand),ErrorCode::IdentityMismatch,"/products/review/v3/demand")
}

/// Only the two fixed chilli remixes inherit an original input. Never search
/// all batches by ingredient identity: a line may aggregate several sources.
pub(crate) fn v3_input(recipe:&RecipeRevision,packet_ref:&ReviewedRecipePacketRef,demand:&DemandLine,
    occurrence:&Value,line:&Value,proof:&MeaningFoodProof)->Result<(RecipeIngredient,Option<InheritedProductInput>),DomainError>{
    let direct:Vec<_>=recipe.ingredients.iter().filter(|i|i.ingredient_id==demand.ingredient_id).collect();
    if direct.len()==1{return Ok((direct[0].clone(),None));}
    require(direct.is_empty()&&occurrence["extra_id"].is_null()
        &&[maker_packets::chilli_wraps_packet_ref()?,maker_packets::chilli_potato_packet_ref()?].contains(packet_ref),
        ErrorCode::UnsupportedCapability,"/products/review/v3/inherited_scope")?;
    let slot:SlotId=serde_json::from_value(occurrence["slot"].clone()).map_err(|_|error(ErrorCode::InvalidValue,"/products/review/v3/slot"))?;
    let consumer:Vec<_>=proof.plan.selections.iter().filter(|s|s.slot==slot&&s.recipe==packet_ref.recipe).collect();
    require(consumer.len()==1,ErrorCode::MissingReference,"/products/review/v3/consumer")?;
    let BatchUse::Leftover{batch_id,component_ids}=&consumer[0].batch_use else{return Err(error(ErrorCode::IdentityMismatch,"/products/review/v3/leftover"))};
    let source_ref=maker_packets::chilli_batch_packet_ref()?;
    let source=maker_packets::known_packet(&source_ref)?;
    require(recipe.reused_inputs.len()==1,ErrorCode::IdentityMismatch,"/products/review/v3/reuse")?;
    let reuse=&recipe.reused_inputs[0];
    require(reuse.source_recipe==source_ref.recipe&&component_ids.len()==1&&component_ids[0]==reuse.source_component_id
        &&reuse.planned_reheats==1,ErrorCode::IdentityMismatch,"/products/review/v3/reuse")?;
    let sources:Vec<_>=proof.plan.selections.iter().filter(|s|s.recipe==source_ref.recipe
        &&matches!(&s.batch_use,BatchUse::NewBatch{batch_id:id,..} if id==batch_id)).collect();
    require(sources.len()==1,ErrorCode::MissingReference,"/products/review/v3/producer")?;
    let producer=sources[0];
    let links:Vec<_>=proof.plan.batch_links.iter().filter(|l|&l.batch_id==batch_id&&l.source_slot==producer.slot&&l.target_slot==slot).collect();
    require(links.len()==1&&links[0].components.len()==1&&links[0].components[0].component_id==reuse.source_component_id
        &&links[0].components[0].quantity==reuse.quantity_per_recipe_yield,ErrorCode::IdentityMismatch,"/products/review/v3/batch_link")?;
    let batches:Vec<_>=proof.derived.batches.iter().filter(|b|&b.batch_id==batch_id&&b.source_slot==producer.slot&&b.recipe==source_ref.recipe).collect();
    require(batches.len()==1&&batches[0].components.iter().filter(|c|c.component_id==reuse.source_component_id).count()==1,
        ErrorCode::MissingReference,"/products/review/v3/batch")?;
    let allocations:Vec<_>=proof.derived.allocations.iter().filter(|a|a.target_slot==slot&&&a.batch_id==batch_id&&a.component_id==reuse.source_component_id).collect();
    require(allocations.len()==1&&allocations[0].remix_recipe.0.as_ref()==Some(&packet_ref.recipe)
        &&allocations[0].quantity==reuse.quantity_per_recipe_yield&&allocations[0].planned_reheats==reuse.planned_reheats,
        ErrorCode::IdentityMismatch,"/products/review/v3/allocation")?;
    let ingredients:Vec<_>=source.recipe().ingredients.iter().filter(|i|i.ingredient_id==demand.ingredient_id&&i.component_id==reuse.source_component_id).collect();
    require(ingredients.len()==1,ErrorCode::MissingReference,"/products/review/v3/source_ingredient")?;
    let ingredient=ingredients[0];
    require(ingredient.quantity.0.as_ref().is_some_and(|q|q.unit==demand.quantity.unit&&q.form==demand.quantity.form),
        ErrorCode::IdentityMismatch,"/products/review/v3/source_key")?;
    let source_occurrence=json!({"slot":producer.slot,"extra_id":null,"recipe":source_ref.recipe});
    let origins=line["origins"].as_array().ok_or_else(||error(ErrorCode::MissingReference,"/products/review/v3/origins"))?;
    let origins:Vec<_>=origins.iter().filter(|o|o["batch_id"]==json!(batch_id)&&o["component_id"]==json!(reuse.source_component_id)).collect();
    require(origins.len()==1&&origins[0]["occurrence"]==source_occurrence&&origins[0]["ingredient_id"]==json!(demand.ingredient_id)
        &&origins[0]["original_quantity"]==encoded(&ingredient.quantity)?,ErrorCode::IdentityMismatch,"/products/review/v3/origin")?;
    // The two compiled registries contain the original definitions as well as
    // their single fresh input. A remix cannot substitute a different meaning.
    let local=maker_packets::known_packet(packet_ref)?;
    let entries=|v:&Value|->Result<Vec<Value>,DomainError>{Ok(v["entries"].as_array().ok_or_else(||error(ErrorCode::MissingReference,"/products/review/v3/registry"))?.iter()
        .filter(|e|e["ingredient_id"]==json!(demand.ingredient_id)&&e["unit"]==json!(demand.quantity.unit)&&e["form"]==json!(demand.quantity.form)).cloned().collect())};
    let original=entries(&source.record().ingredient_registry)?;let inherited=entries(&local.record().ingredient_registry)?;
    require(original.len()==1&&inherited==original,ErrorCode::IdentityMismatch,"/products/review/v3/definition")?;
    Ok((ingredient.clone(),Some(InheritedProductInput{producer_packet:source_ref,component_id:reuse.source_component_id.clone()})))
}

/// The retained line cannot omit a remix and then re-sign its own smaller
/// method graph. For the twelve original inputs derive closure independently
/// from the original digest-bound batches and their actual allocations.
pub(crate) fn v3_original_occurrences(proof:&MeaningFoodProof,demand:&DemandLine,line:&Value)
    ->Result<Option<Vec<Value>>,DomainError>{
    let producer_ref=maker_packets::chilli_batch_packet_ref()?;let producer=maker_packets::known_packet(&producer_ref)?;
    let Some(ingredient)=producer.recipe().ingredients.iter().find(|i|i.ingredient_id==demand.ingredient_id) else{return Ok(None)};
    require(ingredient.quantity.0.as_ref().is_some_and(|q|q.unit==demand.quantity.unit&&q.form==demand.quantity.form),
        ErrorCode::IdentityMismatch,"/products/review/v3/graph_key")?;
    let mut origins=Vec::new();let mut occurrences=Vec::new();
    for batch in proof.derived.batches.iter().filter(|b|b.recipe==producer_ref.recipe) {
        let source=json!({"slot":batch.source_slot,"extra_id":null,"recipe":producer_ref.recipe});
        origins.push(json!({"occurrence":source,"batch_id":batch.batch_id,"component_id":ingredient.component_id,
            "ingredient_id":ingredient.ingredient_id,"original_quantity":ingredient.quantity}));
        if !occurrences.contains(&source){occurrences.push(source);}
        for allocation in proof.derived.allocations.iter().filter(|a|a.batch_id==batch.batch_id&&a.component_id==ingredient.component_id) {
            let selections:Vec<_>=proof.plan.selections.iter().filter(|s|s.slot==allocation.target_slot).collect();
            require(selections.len()==1,ErrorCode::MissingReference,"/products/review/v3/graph_selection")?;
            let selected=selections[0];
            require(if allocation.target_slot==batch.source_slot {selected.recipe==producer_ref.recipe&&allocation.remix_recipe.0.is_none()}
                else {[maker_packets::chilli_wraps_packet_ref()?.recipe,maker_packets::chilli_potato_packet_ref()?.recipe].contains(&selected.recipe)
                    &&allocation.remix_recipe.0.as_ref()==Some(&selected.recipe)},
                ErrorCode::IdentityMismatch,"/products/review/v3/graph_consumer")?;
            let occurrence=json!({"slot":selected.slot,"extra_id":null,"recipe":selected.recipe});
            if !occurrences.contains(&occurrence){occurrences.push(occurrence);}
        }
    }
    let retained=line["origins"].as_array().ok_or_else(||error(ErrorCode::MissingReference,"/products/review/v3/origins"))?;
    require(!origins.is_empty()&&retained.len()==origins.len()&&origins.iter().all(|o|retained.contains(o)),
        ErrorCode::IdentityMismatch,"/products/review/v3/origin_closure")?;
    Ok(Some(occurrences))
}
