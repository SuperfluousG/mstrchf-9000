//! Native JCS and typed core assessment of the Root-adopted rc06 fixture only.
//! No production registry, household, product, cook or incoming authority seam.
use mstrchf_contracts::*;
use mstrchf_core::{eligibility::{eligible, Eligibility}, portions, recipes::*};
use serde_json::{Value,json};
use std::{fs,io::Write,path::PathBuf};
const RECIPE:&str="rc_80000000000000000000000000000006";
const COMPONENT:&str="co_80000000000000000000000000000006";
const DIET:&str="ev_80000000000000000000000000000031";
const POLICY:&str="ev_80000000000000000000000000000032";
const RULE:&str="no animal body-part ingredients, or products/processing aids made from them; no meat/fish/bone stocks, carcass fats, gelatine or isinglass";
fn folder()->PathBuf{PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../../sites/mstrchf-9000/content/intake/three-bean-two-unit-vegetarian-reviewed-v1")}
fn read(file:&str)->Value{serde_json::from_slice(&fs::read(folder().join(file)).unwrap()).unwrap()}
fn reference(recipe:&RecipeRevision)->RecipeRef{RecipeRef{recipe_id:recipe.recipe_id.clone(),revision:recipe.revision,digest:recipe_digest(recipe).unwrap()}}
fn pin(document:&Value)->EvidencePin{EvidencePin{evidence_id:parse_document(&canonical_bytes(&document["evidence_id"]).unwrap()).unwrap(),revision:parse_document(&canonical_bytes(&document["revision"]).unwrap()).unwrap(),digest:canonical_digest(document).unwrap()}}
fn pin_value(value:&EvidencePin)->Value{json!({"evidence_id":value.evidence_id,"revision":value.revision,"digest":value.digest})}
fn basis(recipe:&RecipeRevision,docs:&Value)->EvidenceBasis{EvidenceBasis::new(reference(recipe),docs.as_array().unwrap().iter().map(pin).collect(),vec![]).unwrap()}
fn assess(recipe:&RecipeRevision,docs:&Value)->RecipeCapabilities{
 let basis=basis(recipe,docs);let classes=ComponentClassView::new(reference(recipe),vec![]).unwrap();
 assess_revision(recipe,&basis,&classes).unwrap().assessment().capabilities.clone()
}
fn final_values()->(Value,Value,Value,RecipeRevision){
 let input=read("authoring-input.json");assert_eq!(input["native_jcs_derived"],false);
 let mut packet=input["packet"].clone();let mut docs=input["evidence_documents"].clone();
 let mut recipe:RecipeRevision=parse_document(&serde_json::to_vec(&packet["maker_recipe_revision"]).unwrap()).unwrap();
 recipe.capabilities=assess(&recipe,&docs);let rf=reference(&recipe);
 packet["maker_recipe_revision"]=json!(recipe);packet["recipe_reference"]=json!(rf);
 packet["compiled_loader_inputs"]["recipe_reference"]=json!(rf);
 packet["operational_scope"]["exact_recipe_reference"]=json!(rf);
 packet["intrinsic_input_conditions"]["recipe_reference"]=json!(rf);
 let intrinsic_digest=canonical_digest(&packet["intrinsic_input_conditions"]).unwrap();
 packet["intrinsic_input_conditions_digest"]=json!(intrinsic_digest);
 docs[5]["recipe_reference"]=json!(rf);docs[5]["intrinsic_input_conditions_digest"]=json!(intrinsic_digest);
 let dietary_pin=pin(&docs[5]);
 let registry_digest=canonical_digest(&read("ingredient-registry.json")).unwrap();
 let capture_digest=canonical_digest(&read("source-captures.json")).unwrap();
 docs[6]["recipe_reference"]=json!(rf);docs[6]["accepted_ingredient_registry_digest"]=json!(registry_digest);
 docs[6]["accepted_source_captures_digest"]=json!(capture_digest);
 docs[6]["accepted_intrinsic_input_conditions_digest"]=json!(intrinsic_digest);
 docs[6]["accepted_dietary_review_pin"]=pin_value(&dietary_pin);
 docs[6]["accepts"]["operational_scope"]=packet["operational_scope"].clone();
 let policy_pin=pin(&docs[6]);
 packet["intrinsic_input_custody"]=json!({"recipe_reference":rf,"intrinsic_input_conditions_digest":intrinsic_digest,"dietary_review_pin":pin_value(&dietary_pin),"maker_policy_pin":pin_value(&policy_pin)});
 packet["maker_evidence_pins"]=json!(docs.as_array().unwrap().iter().map(|doc|pin_value(&pin(doc))).collect::<Vec<_>>());
 packet["ingredient_registry_digest"]=json!(registry_digest);packet["source_captures_digest"]=json!(capture_digest);
 packet["evidence_documents_digest"]=json!(canonical_digest(&docs).unwrap());
 packet["status"]=json!("maker_reviewed_native_jcs_ready_not_registry_or_household_runtime_admitted");
 packet["native_contracts_and_core_assessed"]=json!(true);
 assert_eq!(assess(&recipe,&docs),recipe.capabilities);
 let mut clocks=docs.as_array().unwrap().iter().map(|doc|parse_document::<Instant>(&canonical_bytes(&doc["at"]).unwrap()).unwrap()).collect::<Vec<_>>();
 clocks.extend(recipe.reviews.iter().map(|review|parse_document::<Instant>(&canonical_bytes(&review.at).unwrap()).unwrap()));
 let not_before=clocks.into_iter().max().unwrap();
 let digests=json!({"format":"mstrchf-rust-jcs-maker-packet-digests","recipe_reference":rf,"packet_canonical_digest":canonical_digest(&packet).unwrap(),"ingredient_registry_digest":registry_digest,"source_captures_digest":capture_digest,"evidence_documents_digest":canonical_digest(&docs).unwrap(),"intrinsic_input_conditions_digest":intrinsic_digest,"root_policy_evidence_id":POLICY,"root_policy_evidence_digest":policy_pin.digest,"dietary_review_evidence_id":DIET,"dietary_review_evidence_digest":dietary_pin.digest,"method_evidence_id":"ev_80000000000000000000000000000030","method_evidence_digest":pin(&docs[4]).digest,"resource_evidence_id":"ev_8000000000000000000000000000002f","resource_evidence_digest":pin(&docs[3]).digest,"all_evidence_pins":packet["maker_evidence_pins"],"not_before":not_before,"capabilities":recipe.capabilities,"derivation":"mstrchf_contracts::recipe_digest/canonical_digest via Rust serde_jcs; mstrchf_core::recipes::assess_revision","household_runtime_admission":false,"registry_entry_present":false,"actual_product_admission_or_manufacturing_observed":false});
 (packet,docs,digests,recipe)
}
fn require(value:bool,code:ErrorCode,path:&str)->Result<(),DomainError>{if value{Ok(())}else{Err(DomainError::new(code,path))}}
// Fixture-custody checks, not a production loader or generic evidence admission.
fn verify_intrinsic(packet:&Value,docs:&Value,recipe:&RecipeRevision)->Result<(),DomainError>{
 let rf=json!(reference(recipe));let table=&packet["intrinsic_input_conditions"];
 require(packet["recipe_reference"]==rf&&table["recipe_reference"]==rf&&docs[5]["recipe_reference"]==rf&&docs[6]["recipe_reference"]==rf,ErrorCode::IdentityMismatch,"/intrinsic/recipe")?;
 require(table["rule"]==RULE&&table["dietary_review_id"]==DIET&&table["maker_policy_id"]==POLICY&&table["applies_even_when_all_consumers_confirmed_none"]==true,ErrorCode::IdentityMismatch,"/intrinsic/rule")?;
 let rows=table["entries"].as_array().ok_or_else(||DomainError::new(ErrorCode::InvalidValue,"/intrinsic/entries"))?;
 require(rows.len()==recipe.ingredients.len()&&rows.len()==9,ErrorCode::MissingReference,"/intrinsic/coverage")?;
 for (row,ingredient) in rows.iter().zip(recipe.ingredients.iter()){
  require(row["ingredient_id"]==json!(ingredient.ingredient_id)&&row["recipe_input_name"]==json!(ingredient.name)&&row["form"]==json!(ingredient.quantity.0.as_ref().unwrap().form)&&row["unit"]==json!(ingredient.quantity.0.as_ref().unwrap().unit),ErrorCode::IdentityMismatch,"/intrinsic/input")?;
  require(ingredient.name.0.contains("vegetarian-compatible")&&row["input_condition"].as_str().is_some_and(|text|text.contains(RULE)),ErrorCode::IdentityMismatch,"/intrinsic/input_condition")?;
 }
 require(docs[5]["coverage"]["ingredient_ids"]==json!(recipe.ingredients.iter().map(|row|&row.ingredient_id).collect::<Vec<_>>())&&docs[5]["coverage"]["all_ingredient_input_conditions"]==table["entries"]&&docs[5]["full_method"]==json!(recipe.steps)&&docs[4]["full_method"]==json!(recipe.steps),ErrorCode::IdentityMismatch,"/intrinsic/full_method_coverage")?;
 require(docs[5]["coverage"]["step_ids"]==json!([1,2])&&docs[5]["coverage"]["prep_ids"]==json!([1,2]),ErrorCode::IdentityMismatch,"/intrinsic/full_method_coverage")?;
 let digest=json!(canonical_digest(table)?);
 require(packet["intrinsic_input_conditions_digest"]==digest&&docs[5]["intrinsic_input_conditions_digest"]==digest&&docs[6]["accepted_intrinsic_input_conditions_digest"]==digest,ErrorCode::DigestMismatch,"/intrinsic/digest")?;
 require(docs[6]["accepted_dietary_review_pin"]==pin_value(&pin(&docs[5])),ErrorCode::DigestMismatch,"/intrinsic/dietary_pin")?;
 require(packet["intrinsic_input_custody"]==json!({"recipe_reference":rf,"intrinsic_input_conditions_digest":digest,"dietary_review_pin":pin_value(&pin(&docs[5])),"maker_policy_pin":pin_value(&pin(&docs[6]))}),ErrorCode::DigestMismatch,"/intrinsic/custody")?;
 require(docs[5]["actual_products_and_processing_observed"]==false&&docs[5]["trademark_GMO_animal_testing_allergen_or_other_diet_supported"]==false&&table["product_and_operational_compliance_observed"]==false,ErrorCode::AuthorityDenied,"/intrinsic/product_authority")?;
 Ok(())
}
fn preserved(){
 let preservation=read("previous-packet-preservation.json");let items=preservation["files"].as_array().unwrap();assert_eq!(items.len(),16);
 for item in items{let bytes=fs::read(folder().join("..").join(item["directory"].as_str().unwrap()).join(item["filename"].as_str().unwrap())).unwrap();assert_eq!(json!(bytes.len()),item["bytes"]);assert_eq!(json!(bytes_digest(&bytes)),item["sha256_bytes"]);}
 assert_eq!(fs::read(folder().join("ingredient-registry.json")).unwrap(),fs::read(folder().join("../three-bean-two-unit-reviewed-v1/ingredient-registry.json")).unwrap());
}
fn write_new_or_equal(file:&str,value:&Value){let path=folder().join(file);if path.exists(){assert_eq!(read(file),*value,"Refuse differing immutable output {file}");return;}let mut out=fs::OpenOptions::new().create_new(true).write(true).open(path).unwrap();out.write_all(&serde_json::to_vec_pretty(value).unwrap()).unwrap();out.write_all(b"\n").unwrap();}
fn meal(requirements:&[Option<DietCode>;2])->MealRequirements{
 let mut meal=parse_document::<MealRequirementsDocumentV1>(include_bytes!("../../../tests/fixtures/contracts/meal-requirements.json")).unwrap().meal_requirements;
 let mut diner=meal.attendees[0].clone();diner.age_band=AgeBand::Adult;diner.portion=PortionPreference{preset:PortionPresetName::Regular,adjustment:PositiveRational::new(1,1).unwrap(),meal_overrides:BoundedVec(vec![])};
 let mut guest=diner.clone();guest.person=PersonRef::Guest{occasion_id:OccasionId("oc_80000000000000000000000000000006".into()),guest_id:GuestId("gu_80000000000000000000000000000006".into())};
 meal.attendees=BoundedVec(vec![diner,guest]);for (attendee,code) in meal.attendees.iter_mut().zip(requirements){attendee.hard_requirements=BoundedVec(code.as_ref().map(|code|HardRequirement::Diet{code:*code}).into_iter().collect());}
 let mut shared=meal.attendees.iter().flat_map(|a|a.hard_requirements.0.clone()).collect::<Vec<_>>();shared.sort_by_key(|r|canonical_bytes(r).unwrap());shared.dedup();meal.shared_hard_requirements=BoundedVec(shared);
 meal.slot.meal_type=MealType::Lunch;meal.location=MealLocation::AtHome;meal.enabled=true;meal.equipment=BoundedVec(vec![EquipmentCode::Fridge]);meal.mobility=Mobility{reheating:false,cold_storage:false,temperature:MobilityTemperature::Either};meal
}

#[test]
fn derive_or_verify_exact_conditional_vegetarian_packet_native_jcs_and_core(){
 preserved();let(packet,docs,digests,recipe)=final_values();let rf=reference(&recipe);assert_eq!(rf.recipe_id.0,RECIPE);assert_eq!(recipe.components[0].component_id.0,COMPONENT);verify_intrinsic(&packet,&docs,&recipe).unwrap();
 assert_eq!(recipe.reviews.len(),3);assert!(recipe.reviews.iter().all(|review|review.actor==ReviewActor::Automated&&review.kind!=ReviewKind::CookTest));assert_eq!(docs.as_array().unwrap().len(),7);
 for(doc,suffix)in docs.as_array().unwrap().iter().zip(["2c","2d","2e","2f","30","31","32"]){assert_eq!(doc["evidence_id"],format!("ev_800000000000000000000000000000{suffix}"));assert_eq!(doc["actor"],"automated");let _:Instant=parse_document(&canonical_bytes(&doc["at"]).unwrap()).unwrap();}
 assert_eq!(packet["operational_scope"]["attendee_count"],2);assert_eq!(packet["operational_scope"]["resident_count_minimum"],1);assert_eq!(packet["operational_scope"]["temporary_guest_count_maximum"],1);assert_eq!(packet["operational_scope"]["timing"]["first_handling_offset_seconds"],-5400);assert_eq!(packet["operational_scope"]["thermal_completion_guaranteed"],false);
 assert_eq!(recipe.dietary_evidence.len(),1);assert_eq!(recipe.dietary_evidence[0].requirement,HardRequirement::Diet{code:DietCode::Vegetarian});assert_eq!(recipe.dietary_evidence[0].verdict,DietaryVerdict::Supported);
 assert!(recipe.image.0.is_none()&&recipe.nutrition.0.is_none()&&recipe.storage.is_empty()&&recipe.extras.is_empty()&&recipe.reused_inputs.is_empty());assert_eq!(recipe.components[0].production,Nullable(Some(ComponentProductionSpec::Unheated{first_handling_prep_id:1})));
 assert_eq!(recipe.prep.len(),2);assert_eq!((recipe.prep[0].active_minutes,recipe.prep[0].elapsed_minutes,recipe.prep[0].latest_offset_seconds),(30,30,-5400));assert_eq!((recipe.prep[1].active_minutes,recipe.prep[1].elapsed_minutes,recipe.prep[1].latest_offset_seconds),(0,60,-3600));assert_eq!(recipe.prep[1].depends_on.0,vec![1]);
 let b=basis(&recipe,&docs);let classes=ComponentClassView::new(rf.clone(),vec![]).unwrap();let reviewed=assess_revision(&recipe,&b,&classes).unwrap();assert!(recipe.capabilities.local_display&&recipe.capabilities.shopping);assert!(!recipe.capabilities.leftovers&&!recipe.capabilities.starter_distribution);assert_eq!(recipe.capabilities,reviewed.assessment().capabilities);
 for requirement in [[None,None],[Some(DietCode::Vegetarian),None],[None,Some(DietCode::Vegetarian)],[Some(DietCode::Vegetarian),Some(DietCode::Vegetarian)]]{let m=meal(&requirement);assert_eq!(eligible(&reviewed,&m).unwrap(),Eligibility::Eligible);let servings=portions::resolve_servings(&reviewed,&m,&MealOverrides{portions:BoundedVec(vec![]),eat_by:Nullable(None)}).unwrap();assert_eq!(servings.len(),2);assert!(servings.iter().all(|row|row.servings==PositiveRational::new(1,1).unwrap()));}
 let scaled=portions::scale_recipe(&reviewed,PositiveRational::new(2,1).unwrap()).unwrap();assert_eq!(scaled.multiplier,PositiveRational::new(1,1).unwrap());assert_eq!(scaled.components[0].quantity.amount,PositiveRational::new(2,1).unwrap());assert_eq!(scaled.ingredients.len(),9);
 for(suffix,amount,unit,form)in[("07",160,Unit::G,IngredientForm::Drained),("08",160,Unit::G,IngredientForm::Drained),("09",160,Unit::G,IngredientForm::Drained),("0a",80,Unit::G,IngredientForm::Raw),("0b",40,Unit::G,IngredientForm::Raw),("0c",120,Unit::G,IngredientForm::Raw),("0d",20,Unit::Ml,IngredientForm::Prepared),("0e",30,Unit::Ml,IngredientForm::Prepared),("0f",10,Unit::G,IngredientForm::Prepared)]{let row=scaled.ingredients.iter().find(|row|row.ingredient_id.0==format!("ig_800000000000000000000000000000{suffix}")).unwrap();assert_eq!(row.quantity,Quantity{amount:PositiveRational::new(amount,1).unwrap(),unit,form});}
 for amount in [1,3]{assert_eq!(portions::scale_recipe(&reviewed,PositiveRational::new(amount,1).unwrap()).unwrap_err().code,ErrorCode::UnsupportedCapability);}
 for capture in read("source-captures.json")["captures"].as_array().unwrap(){let bytes=fs::read(folder().join(capture["retained_file"].as_str().unwrap())).unwrap();assert_eq!(json!(bytes_digest(&bytes)),capture["body_sha256"]);assert_eq!(json!(bytes.len()),capture["body_bytes"]);}
 let registered=mstrchf_browser::maker_packets::three_bean_two_unit_vegetarian_packet_ref().unwrap();
 assert_eq!(registered.recipe,rf);assert_eq!(json!(registered.packet_digest),digests["packet_canonical_digest"]);
 let trusted=mstrchf_browser::maker_packets::known_packet(&registered).unwrap();
 assert_eq!(trusted.record().packet,packet);assert_eq!(trusted.record().evidence_documents,docs);
 assert_eq!(json!(trusted.not_before()),digests["not_before"]);
 assert_eq!(trusted.intrinsic_input_conditions(),packet.get("intrinsic_input_conditions"));
 if std::env::var("MSTRCHF_DERIVE_TWO_UNIT_VEGETARIAN_PACKET").as_deref()==Ok("1"){for(file,value)in[("packet.json",&packet),("evidence-documents.json",&docs),("digests.json",&digests)]{write_new_or_equal(file,value);}}
 assert_eq!(read("packet.json"),packet);assert_eq!(read("evidence-documents.json"),docs);assert_eq!(read("digests.json"),digests);preserved();println!("{}",serde_json::to_string(&digests).unwrap());
}

#[test]
fn exact_vegetarian_review_does_not_support_other_diets_exclusions_or_missing_evidence(){
 let(_,docs,_,recipe)=final_values();let b=basis(&recipe,&docs);let classes=ComponentClassView::new(reference(&recipe),vec![]).unwrap();let reviewed=assess_revision(&recipe,&b,&classes).unwrap();
 for code in [DietCode::Vegan,DietCode::GlutenFree]{assert!(!matches!(eligible(&reviewed,&meal(&[Some(code),None])).unwrap(),Eligibility::Eligible));}
 let mut excluded=meal(&[None,None]);let requirement=HardRequirement::IngredientExclusion{ingredient_id:recipe.ingredients[0].ingredient_id.clone()};excluded.attendees[0].hard_requirements=BoundedVec(vec![requirement.clone()]);excluded.shared_hard_requirements=BoundedVec(vec![requirement]);assert!(!matches!(eligible(&reviewed,&excluded).unwrap(),Eligibility::Eligible));
 let mut missing=b.clone();missing.admitted.retain(|pin|pin.evidence_id.0!=DIET);assert_eq!(assess_revision(&recipe,&missing,&classes).unwrap_err().code,ErrorCode::MissingReference);
 let mut wrong=recipe.clone();wrong.dietary_evidence[0].review_id=EvidenceId("ev_8000000000000000000000000000002c".into());let wrong_basis=basis(&wrong,&docs);let wrong_classes=ComponentClassView::new(reference(&wrong),vec![]).unwrap();assert!(assess_revision(&wrong,&wrong_basis,&wrong_classes).is_err());
}

#[test]
fn intrinsic_table_rejects_missing_inputs_wrong_ref_or_self_reported_product_authority(){
 let(packet,docs,_,recipe)=final_values();verify_intrinsic(&packet,&docs,&recipe).unwrap();
 let mut missing=packet.clone();missing["intrinsic_input_conditions"]["entries"].as_array_mut().unwrap().pop();assert_eq!(verify_intrinsic(&missing,&docs,&recipe).unwrap_err().code,ErrorCode::MissingReference);
 let mut wrong=packet.clone();wrong["intrinsic_input_conditions"]["recipe_reference"]["digest"]=json!("a".repeat(64));assert_eq!(verify_intrinsic(&wrong,&docs,&recipe).unwrap_err().code,ErrorCode::IdentityMismatch);
 let mut wrong=packet.clone();wrong["intrinsic_input_custody"]["dietary_review_pin"]["digest"]=json!("a".repeat(64));assert_eq!(verify_intrinsic(&wrong,&docs,&recipe).unwrap_err().code,ErrorCode::DigestMismatch);
 let mut authority=packet.clone();authority["intrinsic_input_conditions"]["product_and_operational_compliance_observed"]=json!(true);let error=verify_intrinsic(&authority,&docs,&recipe).unwrap_err();assert!(matches!(error.code,ErrorCode::DigestMismatch|ErrorCode::AuthorityDenied));
}
