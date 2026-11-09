//! Isolated formal authoring/JCS/core checks; no compiled registration, runtime
//! household admission, real product, reader, human cook or handling authority.
use mstrchf_contracts::*;
use mstrchf_core::{availability::meal_requirements, food::*, recipes::*};
use serde_json::{Value,json};
use std::{fs,io::Write,path::PathBuf};

const BASE:&str="rc_80000000000000000000000000000008";
const EXTRA:&str="rc_80000000000000000000000000000009";
const CHICKEN:&str="ig_80000000000000000000000000000010";
const OPTIONAL:&str="ex_80000000000000000000000000000002";
const SECOND:&str="dn_00000000000000000000000000000002";
const REPO:&str="C:/Users/Gregg/Bayes Ltd/Builds/MSTRCHF-9000";
const COUNTER:&str="No continuously reserved EquipmentCode during countertop weighing/plating; brief Fridge access, scales, utensils and correct prior refrigeration remain required conditions.";
#[path="../../../tests/fixtures/food/native/support.rs"] mod native;
fn repo()->PathBuf{PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")}
fn folder(extra:bool)->PathBuf{repo().join("sites/mstrchf-9000/content/intake").join(if extra{"chilled-chicken-extra-reviewed-v1"}else{"three-bean-two-unit-chicken-extra-reviewed-v1"})}
fn read(extra:bool,name:&str)->Value{serde_json::from_slice(&fs::read(folder(extra).join(name)).unwrap()).unwrap()}
fn load(path:&str)->Value{serde_json::from_slice(&fs::read(format!("{REPO}/{path}")).unwrap()).unwrap()}
fn reference(r:&RecipeRevision)->RecipeRef{RecipeRef{recipe_id:r.recipe_id.clone(),revision:r.revision,digest:recipe_digest(r).unwrap()}}
fn pin(d:&Value)->EvidencePin{EvidencePin{evidence_id:parse_document(&canonical_bytes(&d["evidence_id"]).unwrap()).unwrap(),revision:parse_document(&canonical_bytes(&d["revision"]).unwrap()).unwrap(),digest:canonical_digest(d).unwrap()}}
fn pin_value(p:&EvidencePin)->Value{json!({"evidence_id":p.evidence_id,"revision":p.revision,"digest":p.digest})}
fn basis(r:&RecipeRevision,docs:&Value)->EvidenceBasis{EvidenceBasis::new(reference(r),docs.as_array().unwrap().iter().map(pin).collect(),vec![]).unwrap()}
fn classes(r:&RecipeRevision)->ComponentClassView{ComponentClassView::new(reference(r),vec![]).unwrap()}
fn assess(r:&RecipeRevision,docs:&Value)->RecipeCapabilities{assess_revision(r,&basis(r,docs),&classes(r)).unwrap().assessment().capabilities.clone()}
struct Formal {packet:Value,docs:Value,digests:Value,recipe:RecipeRevision}
fn derive(extra:bool,child:Option<&Formal>)->Formal{
 let input=read(extra,"authoring-input.json");assert_eq!(input["native_jcs_derived"],false);
 let mut packet=input["packet"].clone();let mut docs=input["evidence_documents"].clone();
 let mut recipe:RecipeRevision=parse_document(&canonical_bytes(&packet["maker_recipe_revision"]).unwrap()).unwrap();
 if let Some(child)=child {recipe.extras[0].recipe=reference(&child.recipe);}
 recipe.capabilities=assess(&recipe,&docs);let rf=reference(&recipe);
 packet["maker_recipe_revision"]=json!(recipe);packet["recipe_reference"]=json!(rf);
 packet["compiled_loader_inputs"]["recipe_reference"]=json!(rf);packet["operational_scope"]["exact_recipe_reference"]=json!(rf);
 packet["intrinsic_input_conditions"]["recipe_reference"]=json!(rf);packet["phase_resource_map"]["recipe_reference"]=json!(rf);
 if let Some(child)=child {
  let child_ref=json!(reference(&child.recipe));packet["operational_scope"]["optional_extra"]["recipe_reference"]=child_ref.clone();
  packet["declared_extra_dependency"]["extra_reference"]=child_ref.clone();packet["declared_extra_dependency"]["extra_packet_digest"]=child.digests["packet_canonical_digest"].clone();
  docs[6]["accepted_extra_reference"]=child_ref;docs[6]["accepted_extra_packet_digest"]=child.digests["packet_canonical_digest"].clone();
 }
 for doc in docs.as_array_mut().unwrap(){doc["recipe_reference"]=json!(rf);}
 let table_digest=canonical_digest(&packet["intrinsic_input_conditions"]).unwrap();
 let map_digest=canonical_digest(&packet["phase_resource_map"]).unwrap();
 packet["intrinsic_input_conditions_digest"]=json!(table_digest);packet["phase_resource_map_digest"]=json!(map_digest);
 let (resource,diet,policy)=if extra{(4,6,7)}else{(3,5,6)};
 docs[resource]["phase_resource_map_digest"]=json!(map_digest);docs[diet]["intrinsic_input_conditions_digest"]=json!(table_digest);
 let dietary_pin=pin(&docs[diet]);
 let registry_digest=canonical_digest(&read(extra,"ingredient-registry.json")).unwrap();let captures_digest=canonical_digest(&read(extra,"source-captures.json")).unwrap();
 docs[policy]["accepted_ingredient_registry_digest"]=json!(registry_digest);docs[policy]["accepted_source_captures_digest"]=json!(captures_digest);
 docs[policy]["accepted_intrinsic_input_conditions_digest"]=json!(table_digest);docs[policy]["accepted_dietary_review_pin"]=pin_value(&dietary_pin);
 docs[policy]["accepted_phase_resource_map_digest"]=json!(map_digest);docs[policy]["accepts"]["operational_scope"]=packet["operational_scope"].clone();
 let policy_pin=pin(&docs[policy]);packet["intrinsic_input_custody"]=json!({"recipe_reference":rf,"intrinsic_input_conditions_digest":table_digest,"phase_resource_map_digest":map_digest,"dietary_review_pin":pin_value(&dietary_pin),"maker_policy_pin":pin_value(&policy_pin)});
 packet["maker_evidence_pins"]=json!(docs.as_array().unwrap().iter().map(|d|pin_value(&pin(d))).collect::<Vec<_>>());
 packet["ingredient_registry_digest"]=json!(registry_digest);packet["source_captures_digest"]=json!(captures_digest);packet["evidence_documents_digest"]=json!(canonical_digest(&docs).unwrap());
 packet["status"]=json!("maker_reviewed_native_jcs_ready_not_registry_or_household_runtime_admitted");packet["native_contracts_and_core_assessed"]=json!(true);
 assert_eq!(assess(&recipe,&docs),recipe.capabilities);
 let mut clocks=docs.as_array().unwrap().iter().map(|d|parse_document::<Instant>(&canonical_bytes(&d["at"]).unwrap()).unwrap()).collect::<Vec<_>>();
 clocks.extend(recipe.reviews.iter().map(|r|r.at.clone()));let not_before=clocks.into_iter().max().unwrap();
 let method=if extra{5}else{4};let digests=json!({"format":"mstrchf-rust-jcs-maker-packet-digests","recipe_reference":rf,"packet_canonical_digest":canonical_digest(&packet).unwrap(),"ingredient_registry_digest":registry_digest,"source_captures_digest":captures_digest,"evidence_documents_digest":canonical_digest(&docs).unwrap(),"intrinsic_input_conditions_digest":table_digest,"phase_resource_map_digest":map_digest,"root_policy_evidence_id":policy_pin.evidence_id,"root_policy_evidence_digest":policy_pin.digest,"dietary_review_evidence_id":dietary_pin.evidence_id,"dietary_review_evidence_digest":dietary_pin.digest,"method_evidence_id":pin(&docs[method]).evidence_id,"method_evidence_digest":pin(&docs[method]).digest,"resource_evidence_id":pin(&docs[resource]).evidence_id,"resource_evidence_digest":pin(&docs[resource]).digest,"all_evidence_pins":packet["maker_evidence_pins"],"not_before":not_before,"capabilities":recipe.capabilities,"derivation":"Rust mstrchf_contracts::recipe_digest/canonical_digest (serde_jcs) and mstrchf_core::recipes::assess_revision","household_runtime_admission":false,"registry_entry_present":false,"actual_product_or_physical_handling_observed":false});
 Formal{packet,docs,digests,recipe}
}
fn pair()->(Formal,Formal){let extra=derive(true,None);let base=derive(false,Some(&extra));(base,extra)}
fn write_new_or_equal(extra:bool,name:&str,v:&Value){let p=folder(extra).join(name);if p.exists(){assert_eq!(read(extra,name),*v,"Refuse changing immutable {name}");return;}let mut f=fs::OpenOptions::new().write(true).create_new(true).open(p).unwrap();f.write_all(&serde_json::to_vec_pretty(v).unwrap()).unwrap();f.write_all(b"\n").unwrap();}
fn preserved(){for item in read(false,"previous-packet-preservation.json")["files"].as_array().unwrap(){let b=fs::read(repo().join(item["path"].as_str().unwrap())).unwrap();assert_eq!(json!(b.len()),item["bytes"]);assert_eq!(json!(bytes_digest(&b)),item["sha256_bytes"],"{}",item["path"]);}}

#[test]
fn native_formal_pair_derivation_exact_closure_and_old_byte_preservation(){
 preserved();let(base,extra)=pair();assert_eq!(base.recipe.recipe_id.0,BASE);assert_eq!(extra.recipe.recipe_id.0,EXTRA);
 assert_eq!(base.recipe.extras[0].recipe,reference(&extra.recipe));assert_eq!(base.recipe.extras[0].extra_id.0,OPTIONAL);assert_eq!(base.recipe.extras[0].supported_with.0,vec![base.recipe.components[0].component_id.clone()]);
 assert_eq!(base.packet["declared_extra_dependency"]["extra_reference"],json!(reference(&extra.recipe)));assert_eq!(base.packet["declared_extra_dependency"]["base_without_selected_extra_still_requires_dependency"],true);
 for (is_extra,f,n) in [(false,&base,7),(true,&extra,9)]{
  assert_eq!(f.docs.as_array().unwrap().len(),n);assert_eq!(f.digests["not_before"],"2026-10-13T05:20:48Z");
  assert!(f.recipe.reviews.iter().all(|r|r.actor==ReviewActor::Automated&&r.kind!=ReviewKind::CookTest));
  assert!(f.recipe.capabilities.local_display&&f.recipe.capabilities.shopping);assert!(!f.recipe.capabilities.leftovers&&!f.recipe.capabilities.starter_distribution);
  for code in [CandidateIssueCode::MissingStorage,CandidateIssueCode::MissingImage]{assert!(f.recipe.capabilities.blocking_issues.iter().any(|i|i.code==code));}
  assert_eq!(f.packet["runtime_admission"],false);assert_eq!(f.packet["household_runtime_admission"],false);assert!(f.recipe.image.0.is_none()&&f.recipe.nutrition.0.is_none()&&f.recipe.storage.is_empty()&&f.recipe.reused_inputs.is_empty());
  for d in f.docs.as_array().unwrap(){assert_eq!(d["actor"],"automated");assert_eq!(d["recipe_reference"],json!(reference(&f.recipe)));}
  assert_eq!(f.packet["intrinsic_input_conditions"]["product_and_operational_compliance_observed"],false);
  let empty=EvidenceBasis::new(reference(&f.recipe),vec![],vec![]).unwrap();assert_eq!(assess_revision(&f.recipe,&empty,&classes(&f.recipe)).unwrap_err().code,ErrorCode::MissingReference);
  if std::env::var("MSTRCHF_DERIVE_CHICKEN_PAIR").as_deref()==Ok("1"){for(name,v)in[("packet.json",&f.packet),("evidence-documents.json",&f.docs),("digests.json",&f.digests)]{write_new_or_equal(is_extra,name,v);}}
  for(name,v)in[("packet.json",&f.packet),("evidence-documents.json",&f.docs),("digests.json",&f.digests)]{assert_eq!(read(is_extra,name),*v);}
  println!("{}",serde_json::to_string(&f.digests).unwrap());
 }
 assert_eq!(extra.recipe.source.origin,RecipeSourceOrigin::UserAuthored);assert!(extra.recipe.source.url.0.is_none()&&extra.recipe.text_rights.licence_url.0.is_none());assert_eq!(extra.recipe.text_rights.distribution,RightsDistribution::PersonalOnly);
 assert!(extra.recipe.source.authors[0].0.contains("Codex"));assert!(extra.recipe.capabilities.blocking_issues.iter().any(|i|i.code==CandidateIssueCode::MissingRights));
 assert_eq!(extra.packet["phase_resource_map"]["entries"],json!([{"prep_id":1,"equipment":[]}]));assert_eq!(extra.packet["phase_resource_map"]["interpretation"],COUNTER);
 assert_eq!(base.packet["phase_resource_map"]["entries"],json!([{"prep_id":1,"equipment":["fridge"]},{"prep_id":2,"equipment":["fridge"]}]));
 let old=load("sites/mstrchf-9000/content/intake/three-bean-two-unit-vegetarian-reviewed-v1/packet.json");let oldr:RecipeRevision=parse_document(&canonical_bytes(&old["maker_recipe_revision"]).unwrap()).unwrap();
 assert_eq!(base.recipe.ingredients.iter().map(|i|(&i.ingredient_id,&i.quantity)).collect::<Vec<_>>(),oldr.ingredients.iter().map(|i|(&i.ingredient_id,&i.quantity)).collect::<Vec<_>>());assert_eq!(base.recipe.prep,oldr.prep);assert_eq!(base.recipe.equipment,oldr.equipment);
 let oldreg=load("sites/mstrchf-9000/content/intake/three-bean-two-unit-vegetarian-reviewed-v1/ingredient-registry.json");assert_eq!(read(false,"ingredient-registry.json")["entries"].as_array().unwrap()[..9],oldreg["entries"].as_array().unwrap()[..]);assert_eq!(read(false,"ingredient-registry.json")["entries"][9],read(true,"ingredient-registry.json")["entries"][0]);
 preserved();
}

fn one()->PositiveRational{PositiveRational::new(1,1).unwrap()}
fn blank()->PayloadDigest{PayloadDigest("0".repeat(64))}
fn at(time:&str)->ZonedMoment{ZonedMoment::new(LocalDate("2026-12-28".into()),LocalTime(time.into()),ZoneId("Europe/London".into()),Resolution::Unambiguous,Instant(format!("2026-12-28T{time}Z"))).unwrap()}
struct Probe {basis:SavedFoodBasis,feasibility:FeasibilityInputs}
impl Probe {
    fn derive(&self)->Result<FoodDerivation,FoodFailure> {derive_plan(&self.basis.plan,&self.basis.context,&FoodInputs{evidence:&self.basis.evidence,identities:&self.basis.identities,schedule:&self.basis.schedule,feasibility:&self.feasibility})}
    fn identities(&mut self) {
        let keys=required_identity_keys(&self.basis.plan,&self.basis.context,&self.basis.evidence,&self.basis.schedule).unwrap();
        self.basis.identities.allocations=keys.allocations.into_iter().enumerate().map(|(i,key)|(key,AllocationId(format!("al_{:032x}",0xf100+i)))).collect();
        self.basis.identities.sessions=keys.sessions.into_iter().enumerate().map(|(i,key)|(key,PrepSessionId(format!("ps_{:032x}",0xf100+i)))).collect();
        self.basis.identities.demand_lines=keys.demand_lines.into_iter().enumerate().map(|(i,key)|(key,DemandLineId(format!("dl_{:032x}",0xf100+i)))).collect();
        self.basis.identities.digest=food_identities_digest(&self.basis.identities).unwrap();
    }
}

fn fixture(extra_offset:i32,selected:bool,counter:bool)->Probe {
    let scaffold=load("tests/fixtures/food/native/v4/subject-saved-basis.input.json");
    let mut saved:SavedFoodBasis=native::read(&scaffold["value"]);
    assert_eq!(extra_offset,-300,"Only exact formal T-5 is adopted");
    let (base_formal,extra_formal)=pair();let base=base_formal.recipe;let extra=extra_formal.recipe;
    let base_ref=reference(&base);let extra_ref=reference(&extra);
    let base_basis=basis(&base,&base_formal.docs);let base_classes=classes(&base);
    let mut extra_basis=basis(&extra,&extra_formal.docs);let extra_classes=classes(&extra);
    // Negative control only: distinct SYNTHETIC all-Fridge premise, never a
    // formal resource review or packet authority.
    let extra_resource=EvidenceId(if counter {"ev_80000000000000000000000000000042"} else {"ev_ffffffffffffffffffffffffffffff13"}.into());
    if !counter {extra_basis.admitted.push(EvidencePin{evidence_id:extra_resource.clone(),revision:Revision(1),digest:Digest("a".repeat(64))});extra_basis.admitted.sort_by(|a,b|a.evidence_id.cmp(&b.evidence_id));}
    let first=saved.context.household.diners[0].diner_id.clone();
    let mut second=saved.context.household.diners[0].clone();second.diner_id=DinerId(SECOND.into());second.display_name=Nullable(Some(NameText("SYNTHETIC None adult".into())));saved.context.household.diners.0.push(second);
    saved.context.household.diners[0].restrictions=serde_json::from_value(serde_json::json!({"kind":"confirmed","data":{"confirmed_at":"2026-12-27T08:00:00Z","requirements":[{"kind":"diet","data":{"code":"vegetarian"}}]}})).unwrap();
    saved.context.household.preferences.equipment=BoundedVec(vec![EquipmentCode::Fridge]);
    saved.context.household.preferences.meal_times.iter_mut().find(|time|time.meal_type==MealType::Lunch).unwrap().time=LocalTime("12:30:00".into());
    let slot=saved.plan.selections[0].slot.clone();assert_eq!(slot.meal_type,MealType::Lunch);
    let week=&mut saved.context.weeks[0];week.plan=saved.plan.clone();
    let cell=week.cells.iter_mut().find(|cell|cell.slot==slot).unwrap();cell.attendee_ids=BoundedVec(vec![first.clone(),DinerId(SECOND.into())]);
    let mut chosen=saved.plan.selections[0].clone();chosen.recipe=base_ref.clone();chosen.extras=BoundedVec(vec![]);chosen.meal_overrides=MealOverrides{portions:BoundedVec(vec![]),eat_by:Nullable(None)};
    chosen.servings=BoundedVec([first.clone(),DinerId(SECOND.into())].into_iter().map(|diner_id|DinerServing{person:PersonRef::Diner{diner_id},preset_ref:PortionPresetRef{recipe:base_ref.clone(),preset:PortionPresetName::Regular,age_band:AgeBand::Adult,meal_type:MealType::Lunch},servings:one()}).collect());
    chosen.batch_use=BatchUse::NewBatch{batch_id:BatchId("ba_ffffffffffffffffffffffffffffff11".into()),production_servings:PositiveRational::new(2,1).unwrap()};
    if selected {chosen.extras=BoundedVec(vec![ExtraSelection{extra_id:ExtraId(OPTIONAL.into()),recipe:extra_ref.clone(),recipients:BoundedVec(vec![DinerServing{person:PersonRef::Diner{diner_id:DinerId(SECOND.into())},preset_ref:PortionPresetRef{recipe:extra_ref.clone(),preset:PortionPresetName::Regular,age_band:AgeBand::Adult,meal_type:MealType::Lunch},servings:one()}])}]);}
    saved.plan.selections=BoundedVec(vec![chosen]);saved.plan.batch_links=BoundedVec(vec![]);saved.context.weeks[0].plan=saved.plan.clone();
    saved.context.recipes=BoundedVec(vec![base.clone(),extra.clone()]);saved.context.stamp.recipe_refs=BoundedVec(vec![base_ref.clone(),extra_ref.clone()]);
    saved.context.requirements=BoundedVec(meal_requirements(&saved.context.household,&saved.context.weeks[0],&[]).unwrap().to_vec());
    let eat=saved.context.requirements.iter().find(|meal|meal.slot==slot).unwrap().eat_by.0.clone().unwrap();assert_eq!(eat.local_time.0,"12:30:00");
    let resources=vec![PhaseResources{recipe:base_ref.clone(),prep_id:1,equipment:vec![EquipmentCode::Fridge],evidence_ids:vec![EvidenceId("ev_80000000000000000000000000000049".into())]},PhaseResources{recipe:base_ref.clone(),prep_id:2,equipment:vec![EquipmentCode::Fridge],evidence_ids:vec![EvidenceId("ev_80000000000000000000000000000049".into())]},PhaseResources{recipe:extra_ref.clone(),prep_id:1,equipment:if counter {vec![]} else {vec![EquipmentCode::Fridge]},evidence_ids:vec![extra_resource.clone()]}];
    saved.evidence=FoodEvidence{catalogue_revision:saved.context.stamp.catalogue_revision,catalogue_digest:blank(),policy_version:Revision(1),recipes:vec![base_basis,extra_basis],classes:vec![base_classes,extra_classes],methods:vec![MethodPhases{portable_cold: None, recipe:base_ref.clone(),kind:MethodKind::Production,prep_ids:vec![1,2],direct_reuse_counts:vec![],evidence_ids:vec![EvidenceId("ev_80000000000000000000000000000049".into())]},MethodPhases{portable_cold: None, recipe:extra_ref.clone(),kind:MethodKind::Production,prep_ids:vec![1],direct_reuse_counts:vec![],evidence_ids:vec![extra_resource]}],resources,essential_ingredient_ids:vec![]};
    if !selected {saved.evidence.recipes.retain(|row|row.recipe.recipe_id.0==BASE);saved.evidence.classes.retain(|row|row.recipe.recipe_id.0==BASE);saved.evidence.methods.retain(|row|row.recipe.recipe_id.0==BASE);saved.evidence.resources.retain(|row|row.recipe.recipe_id.0==BASE);}
    saved.evidence.catalogue_digest=food_evidence_digest(&saved.evidence).unwrap();
    let base_session=SessionKey{anchor_slot:slot.clone(),role:SessionRole::AdvancePrep,ordinal:1};let extra_session=SessionKey{anchor_slot:slot.clone(),role:SessionRole::AdvancePrep,ordinal:2};
    let mut routes=vec![PhaseRoute{phase:PhaseKey{occurrence:OccurrenceKey{slot:slot.clone(),extra_id:None,recipe:base_ref.clone()},prep_id:1},session:base_session.clone()},PhaseRoute{phase:PhaseKey{occurrence:OccurrenceKey{slot:slot.clone(),extra_id:None,recipe:base_ref},prep_id:2},session:base_session}];
    if selected {routes.push(PhaseRoute{phase:PhaseKey{occurrence:OccurrenceKey{slot:slot.clone(),extra_id:Some(ExtraId(OPTIONAL.into())),recipe:extra_ref},prep_id:1},session:extra_session});}routes.sort_by(|a,b|phase_cmp(&a.phase,&b.phase));
    let mut occupied=OccupancySnapshot{query:Some(OccupancyQuery{horizon:TimeInterval{start:at(if selected&&extra_offset==-5700 {"10:55:00"} else {"11:00:00"}),end:eat},excluded_subject:None}),occupancy_epoch:Revision(1),owners:vec![],digest:blank()};occupied.digest=occupancy_snapshot_digest(&occupied).unwrap();
    // Explicit empty occupancy is a complete synthetic household premise only.
    saved.schedule=ScheduleInputs{layout:ScheduleLayout{routes,fixed_starts:vec![],attachments:vec![]},occupied};
    let mut products=ProductUseSnapshot{revision:Revision(1),digest:blank(),valid_until:None,evidence:vec![],bindings:vec![],facts:vec![]};products.digest=digest_product_uses(&products).unwrap();
    let mut feasibility=FeasibilityInputs{revision:Revision(1),digest:blank(),delivery:DeliveryInput::Unselected,products,confirmations:vec![],evaluated_at:Instant("2026-12-28T08:00:00Z".into())};feasibility.digest=digest_feasibility_inputs(&feasibility).unwrap();
    let mut probe=Probe{basis:saved,feasibility};probe.identities();probe
}

fn exact_base_quantities(probe:&Probe,result:&FoodDerivation) {
    let base=probe.basis.context.recipes.iter().find(|recipe|recipe.recipe_id.0==BASE).unwrap();
    assert_eq!(base.ingredients.len(),9);
    for ingredient in base.ingredients.iter() {
        let line=result.plan.demand.lines.iter().find(|line|line.ingredient_id==ingredient.ingredient_id).unwrap();
        assert_eq!(line.quantity,ingredient.quantity.0.clone().unwrap());assert!(line.origin_extras.is_empty());
    }
    assert_eq!(result.plan.batches.len(),1);
    let component=&result.plan.batches[0].components[0];let units=Quantity{amount:PositiveRational::new(2,1).unwrap(),unit:Unit::Each,form:IngredientForm::Prepared};assert_eq!(component.produced,units);assert_eq!(component.allocated,units);
    assert!(probe.basis.evidence.resources.iter().filter(|phase|phase.recipe.recipe_id.0==BASE).all(|phase|phase.equipment==vec![EquipmentCode::Fridge]));
}

fn named_none(probe:&Probe)->DinerServing {
    probe.basis.plan.selections[0].extras[0].recipients[0].clone()
}
fn update_plan_context(probe:&mut Probe) {probe.basis.context.weeks[0].plan=probe.basis.plan.clone();}
fn extra_fit(probe:&Probe,equipment:Vec<EquipmentCode>)->mstrchf_core::eligibility::Eligibility {
    let recipe=probe.basis.context.recipes.iter().find(|recipe|recipe.recipe_id.0==EXTRA).unwrap();
    let reference=reference(recipe);
    let basis=probe.basis.evidence.recipes.iter().find(|row|row.recipe==reference).unwrap();
    let classes=probe.basis.evidence.classes.iter().find(|row|row.recipe==reference).unwrap();
    let reviewed=assess_revision(recipe,basis,classes).unwrap();
    let mut meal=probe.basis.context.requirements.iter().find(|meal|meal.slot==probe.basis.plan.selections[0].slot).unwrap().clone();
    meal.attendees.0.retain(|attendee|attendee.person==named_none(probe).person);
    assert_eq!(meal.attendees.len(),1);assert!(meal.attendees[0].hard_requirements.is_empty());
    // Exact recipient-only projection from this actual synthetic meal. Native
    // validate_extra itself uses this same union rule; no transported authority.
    meal.shared_hard_requirements=BoundedVec(vec![]);meal.equipment=BoundedVec(equipment);
    mstrchf_core::eligibility::eligible(&reviewed,&meal).unwrap()
}

#[test]
fn adopted_counter_phase_derives_t5_extra_with_exact_base_and_cold_prerequisite() {
    let probe=fixture(-300,true,true);let result=probe.derive().unwrap();
    exact_base_quantities(&probe,&result);
    let recipe=probe.basis.context.recipes.iter().find(|recipe|recipe.recipe_id.0==EXTRA).unwrap();
    assert_eq!(recipe.equipment.0,vec![EquipmentCode::Fridge]);assert!(recipe.portability.cold_storage_required);assert!(!recipe.portability.reheating_required);
    let resource=probe.basis.evidence.resources.iter().find(|row|row.recipe.recipe_id.0==EXTRA).unwrap();assert!(resource.equipment.is_empty());assert_eq!(resource.prep_id,1);assert_eq!(resource.evidence_ids,vec![EvidenceId("ev_80000000000000000000000000000042".into())]);
    assert_eq!(result.plan.demand.lines.len(),10);assert_eq!(result.plan.allocations.len(),2);assert_eq!(result.plan.prep_sessions.len(),2);
    let base=result.plan.prep_sessions.iter().find(|session|session.tasks.iter().any(|task|task.recipe.recipe_id.0==BASE)).unwrap();
    assert_eq!(base.start,at("11:00:00"));assert_eq!(base.end,at("12:30:00"));assert_eq!(base.ingredient_needed_at,at("11:00:00"));assert_eq!(base.tasks.len(),2);assert_eq!(base.tasks[1].elapsed.start,at("11:30:00"));assert!(base.tasks[1].active_intervals.is_empty());
    let extra=result.plan.prep_sessions.iter().find(|session|session.tasks.iter().any(|task|task.recipe.recipe_id.0==EXTRA)).unwrap();
    assert_eq!(extra.start,at("12:25:00"));assert_eq!(extra.end,at("12:30:00"));assert_eq!(extra.ingredient_needed_at,at("12:25:00"));assert_eq!(extra.tasks.len(),1);assert_eq!(extra.tasks[0].active_intervals.0,vec![extra.tasks[0].elapsed.clone()]);assert!(extra.attached_to.0.is_none());
    let recipients=&probe.basis.plan.selections[0].extras[0].recipients;assert_eq!(recipients.len(),1);assert_eq!(recipients[0].person,PersonRef::Diner{diner_id:DinerId(SECOND.into())});assert_eq!(recipients[0].servings,one());
    let chicken=result.plan.demand.lines.iter().find(|line|line.ingredient_id.0==CHICKEN).unwrap();assert_eq!(chicken.quantity,Quantity{amount:PositiveRational::new(50,1).unwrap(),unit:Unit::G,form:IngredientForm::Cooked});assert_eq!(chicken.origin_extras.0,vec![ExtraId(OPTIONAL.into())]);
    println!("FORMAL INPUT/SYNTHETIC HOUSEHOLD DERIVATION: base T-90/T-60 +counter extra T-5; exact9base rows/2units/2base allocations +50g cooked/one named None extra recipient; recipe Fridge still required. No actual product/thermal/separation/service claim.");
}

#[test]
fn recipe_fridge_remains_mandatory_even_when_counter_resource_assignment_is_empty() {
    let probe=fixture(-300,true,true);
    assert_eq!(extra_fit(&probe,vec![EquipmentCode::Fridge]),mstrchf_core::eligibility::Eligibility::Eligible);
    let fit=extra_fit(&probe,vec![]);println!("SYNTHETIC NO FRIDGE EXTRA PREDICATE: {fit:?}");
    assert!(matches!(fit,mstrchf_core::eligibility::Eligibility::Ineligible(ref conflicts) if conflicts.iter().any(|conflict|conflict.code==ErrorCode::DomainConflict)));
    let mut whole=fixture(-300,true,true);whole.basis.context.household.preferences.equipment=BoundedVec(vec![]);
    whole.basis.context.requirements=BoundedVec(meal_requirements(&whole.basis.context.household,&whole.basis.context.weeks[0],&[]).unwrap().to_vec());
    assert!(whole.derive().is_err());
}

#[test]
fn vegetarian_recipient_is_refused_without_silently_dropping_or_redirecting_extra() {
    let mut probe=fixture(-300,true,true);
    let vegetarian=probe.basis.plan.selections[0].servings[0].person.clone();
    probe.basis.plan.selections[0].extras[0].recipients[0].person=vegetarian.clone();update_plan_context(&mut probe);
    let plan=probe.basis.plan.clone();let error=probe.derive().unwrap_err();println!("SYNTHETIC VEGETARIAN EXTRA RECIPIENT RESULT: {error:?}");
    assert!(matches!(error,FoodFailure::Blocked{ref conflicts,..} if conflicts.iter().any(|conflict|conflict.code==ErrorCode::DomainConflict)));
    assert_eq!(probe.basis.plan,plan);assert_eq!(probe.basis.plan.selections[0].extras[0].recipients[0].person,vegetarian);
}

#[test]
fn foreign_recipient_refuses_without_remapping_to_a_current_attendee() {
    let mut probe=fixture(-300,true,true);let foreign=PersonRef::Diner{diner_id:DinerId("dn_fffffffffffffffffffffffffffffff1".into())};
    probe.basis.plan.selections[0].extras[0].recipients[0].person=foreign.clone();update_plan_context(&mut probe);
    let plan=probe.basis.plan.clone();let error=probe.derive().unwrap_err();println!("SYNTHETIC FOREIGN EXTRA RECIPIENT RESULT: {error:?}");
    let FoodFailure::Invalid(error)=error else {panic!("foreign recipient must fail validation")};assert_eq!(error.code,ErrorCode::ForeignReference);
    assert_eq!(probe.basis.plan,plan);assert_eq!(probe.basis.plan.selections[0].extras[0].recipients[0].person,foreign);
}

#[test]
fn distinct_all_fridge_control_still_refuses_the_exact_t5_schedule() {
    let probe=fixture(-300,true,false);
    assert!(probe.basis.evidence.resources.iter().all(|resource|resource.equipment==vec![EquipmentCode::Fridge]));
    let extra=probe.basis.evidence.resources.iter().find(|row|row.recipe.recipe_id.0==EXTRA).unwrap();assert_eq!(extra.evidence_ids,vec![EvidenceId("ev_ffffffffffffffffffffffffffffff13".into())]);
    let error=probe.derive().unwrap_err();println!("SYNTHETIC DISTINCT ALL-FRIDGE CONTROL: {error:?}");
    let FoodFailure::Invalid(error)=error else {panic!("expected placement error")};assert_eq!(error.code,ErrorCode::DomainConflict);assert_eq!(error.issues[0].path,"/placement/no_schedule");
}

#[test]
fn unselected_extra_preserves_nine_base_demands_but_declared_reference_still_needs_closure(){
 let probe=fixture(-300,false,true);let result=probe.derive().unwrap();exact_base_quantities(&probe,&result);
 assert_eq!(result.plan.demand.lines.len(),9);assert_eq!(result.plan.allocations.len(),2);assert_eq!(result.plan.prep_sessions.len(),1);
 assert!(!result.plan.demand.lines.iter().any(|line|line.ingredient_id.0==CHICKEN));
 let mut unresolved=fixture(-300,false,true);unresolved.basis.context.recipes.0.retain(|r|r.recipe_id.0!=EXTRA);unresolved.basis.context.stamp.recipe_refs.0.retain(|r|r.recipe_id.0!=EXTRA);
 let error=unresolved.derive().unwrap_err();println!("FORMAL base-zero-extra missing child closure: {error:?}");
 assert!(matches!(error,FoodFailure::Invalid(ref e) if e.code==ErrorCode::MissingReference));
}

#[test]
fn doubled_extra_recipient_unknown_requirement_and_changed_resource_custody_refuse(){
 let mut both=fixture(-300,true,true);let mut second=both.basis.plan.selections[0].extras[0].recipients[0].clone();second.person=both.basis.plan.selections[0].servings[0].person.clone();both.basis.plan.selections[0].extras[0].recipients.0.push(second);update_plan_context(&mut both);
 assert!(both.derive().is_err());assert_eq!(both.basis.plan.selections[0].extras[0].recipients.len(),2);
 let mut unknown=fixture(-300,true,true);unknown.basis.context.household.diners[1].restrictions=serde_json::from_value(json!({"kind":"confirmed","data":{"confirmed_at":"2026-12-27T08:00:00Z","requirements":[{"kind":"allergen_exclusion","data":{"code":"milk"}}]}})).unwrap();
 unknown.basis.context.requirements=BoundedVec(meal_requirements(&unknown.basis.context.household,&unknown.basis.context.weeks[0],&[]).unwrap().to_vec());assert!(unknown.derive().is_err());
 let mut missing=fixture(-300,true,true);missing.basis.evidence.resources.iter_mut().find(|r|r.recipe.recipe_id.0==EXTRA).unwrap().evidence_ids.clear();assert_eq!(food_evidence_digest(&missing.basis.evidence).unwrap_err().code,ErrorCode::MissingReference);
 let mut extra_as_base=fixture(-300,true,true);let rf=reference(extra_as_base.basis.context.recipes.iter().find(|r|r.recipe_id.0==EXTRA).unwrap());extra_as_base.basis.plan.selections[0].recipe=rf;update_plan_context(&mut extra_as_base);assert!(extra_as_base.derive().is_err());
 // The source/table/policy and exact map are immutable fixture inputs; a
 // self-signed unknown document is not used to derive any accepted packet.
 let(base,extra)=pair();assert_ne!(base.packet["recipe_reference"],extra.packet["recipe_reference"]);
 for (f,i) in [(&base,3),(&extra,4)]{assert_eq!(f.docs[i]["phase_resources"],f.packet["phase_resource_map"]["entries"]);assert_eq!(f.docs[i]["full_method"],json!(f.recipe.steps));assert_eq!(f.docs[i]["phase_resource_map_digest"],f.packet["phase_resource_map_digest"]);}
}

