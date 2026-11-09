//! Read-only public/native context preflight. Households are synthetic; real
//! maker packets enter only through their ordinary audited admission command.
use mstrchf_browser::{BrowserState, catalogue::RuntimeCatalogue, maker_packets::*, run_json};
use mstrchf_contracts::*;
use mstrchf_core::{food::saved_basis_digest, products::{MatchPolicy, policy_digest, prepare_match_context}};
use serde_json::{Value, json};

const NOW: &str = "2026-10-13T00:30:00Z";
const LATER: &str = "2026-10-13T01:00:00Z";
fn call(request: Value) -> Value { serde_json::from_slice(&run_json(&serde_json::to_vec(&request).unwrap())).unwrap() }
fn good(request: Value) -> Value {let result=call(request);assert_eq!(result["ok"],true,"{result}");result["value"].clone()}
fn dispatch(state: &Value, body: Value, id: u32) -> Value {
    let command=json!({"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body});
    good(json!({"op":"dispatch","state":state,"command_json":serde_json::to_string(&command).unwrap(),"command":command,"now":NOW}))["state"].clone()
}
fn edit(state: &Value, week: Value, id: u32) -> Value {
    dispatch(state,json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{
        "week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],"day_enabled":week["day_enabled"],
        "meal_type_enabled":week["meal_type_enabled"],"cells":week["cells"],"plan":week["plan"]}}}),id)
}
fn setup() -> Value {
    let mut state=serde_json::from_slice::<Value>(include_bytes!("fixtures/legacy-engineering-receipts.json")).unwrap()["state"].clone();
    let mut profile=state["snapshot"]["profile"].clone();profile["diners"][0]["age_band"]=json!("adult");
    profile["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":NOW}});
    profile["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});
    profile["preferences"]["equipment"]=json!(["fridge","hob"]);
    state=dispatch(&state,json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),0xe001);
    // The frozen legacy fixture deliberately has its old privacy flags enabled.
    // Establish the current off/off kitchen through the ordinary command.
    let mut privacy=state["snapshot"]["privacy"].clone();privacy["use_ai_features"]=json!(false);
    privacy["include_household_requirements"]=json!(false);privacy["confirmed_at"]=json!(NOW);
    state=dispatch(&state,json!({"kind":"set_privacy","data":{"expected_policy_revision":privacy["revision"],"policy":privacy}}),0xe005);
    let mut week=state["snapshot"]["weeks"][0].clone();week["plan"]["selections"]=json!([]);week["plan"]["batch_links"]=json!([]);
    week["meal_type_enabled"]=json!({"breakfast":true,"lunch":true,"dinner":true});
    for cell in week["cells"].as_array_mut().unwrap(){cell["choice"]=json!("at_home");}
    edit(&state,week,0xe002)
}
fn admit(state: &Value, packet: ReviewedRecipePacketRef, id: u32) -> Value {
    let revision=1+state["reviewed_packets"].as_array().map_or(0,Vec::len);
    dispatch(state,json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":revision.to_string()}}),id)
}
fn select(state: &Value, week_index: usize, cell: usize, recipe: &RecipeRef, id: u32) -> Value {
    let week=&state["snapshot"]["weeks"][week_index];
    let board=good(json!({"op":"board","state":state,"week_id":week["range"]["week_id"],"slot":week["cells"][cell]["slot"],"now":NOW}));
    let reference=serde_json::to_value(recipe).unwrap();
    let choice=board["choices"].as_array().unwrap().iter().find(|choice|choice["selection"]["recipe"]==reference).expect("actual exact eligible native choice");
    let mut week=week.clone();week["plan"]=choice["plan"].clone();edit(state,week,id)
}
fn real() -> Value {let state=admit(&setup(),porridge_packet_ref().unwrap(),0xe003);select(&state,0,3,&porridge_packet_ref().unwrap().recipe,0xe004)}
fn quick_ref() -> RecipeRef {
    RuntimeCatalogue::legacy().unwrap().pin().entries.iter()
        .find(|entry| entry.recipe.recipe_id.0 == "rc_00000000000000000000000000000005")
        .unwrap().recipe.clone()
}
fn preflight(state: &Value, week: usize) -> Value {good(json!({"op":"product_match_context","state":state,"week_id":state["snapshot"]["weeks"][week]["range"]["week_id"],"now":NOW}))}
fn policy() -> MatchPolicy {let mut policy=MatchPolicy{version:Revision(1),digest:PayloadDigest("0".repeat(64)),rules:vec![]};policy.digest=policy_digest(&policy).unwrap();policy}
fn quantity(n: u64, unit: &str, form: &str) -> Value {json!({"amount":{"numerator":n.to_string(),"denominator":"1"},"unit":unit,"form":form})}
fn material(state: &Value) -> (mstrchf_core::food::SavedFoodBasis,mstrchf_core::food::FoodDerivation) {
    let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();let view=RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();
    mstrchf_browser::calendar_view::material_at(&view,&typed.snapshot,typed.snapshot.profile.0.as_ref().unwrap(),&typed.snapshot.weeks[0],&Instant(NOW.into())).unwrap().unwrap()
}
fn no_authority(value: &Value) {
    match value {Value::Object(fields)=>{for (key,value) in fields {assert!(!["suitable","accepted_read","product_meaning","meaning_admission","match_basis","matched_candidate","policy_digest","intent","effect"].contains(&key.as_str()),"unexpected authority field {key}");no_authority(value);}},Value::Array(values)=>for value in values {no_authority(value)},_=>()}
}

#[test]
fn exact_r3_context_is_readonly_complete_and_has_no_product_authority() {
    let state=real();let before=canonical_bytes(&state).unwrap();let projected=preflight(&state,0);
    assert_eq!(projected["format"],"mstrchf-product-match-context");assert_eq!(projected["version"],1);
    assert_eq!(projected["match_status"],"not_assessed");assert_eq!(projected["retailer_status"],"unqualified");assert_eq!(projected["effect_enabled"],false);
    assert_eq!(projected["contains_engineering"],false);assert_eq!(projected["snapshot_revision"],state["snapshot"]["snapshot_revision"]);
    assert_eq!(projected["plan_revision"],state["snapshot"]["weeks"][0]["plan"]["revision"]);assert!(!projected["open_slots"].as_array().unwrap().is_empty());
    let (basis,food)=material(&state);assert_eq!(projected["saved_basis_digest"],json!(saved_basis_digest(&basis).unwrap()));
    assert_eq!(projected["derived_digest"],json!(food.plan.digest));assert_eq!(projected["demand_digest"],json!(food.plan.demand.digest));assert_eq!(projected["context"],json!(basis.context.stamp));
    assert_eq!(projected["lines"].as_array().unwrap().len(),3);
    for line in projected["lines"].as_array().unwrap(){let id=line["demand"]["ingredient_id"].as_str().unwrap();assert_eq!(line["demand"]["quantity"],if id.ends_with("01"){quantity(45,"g","dry")}else{quantity(180,"ml","raw")});
        assert_eq!(line["shopping_action"],"add_to_shop");assert_eq!(line["cooking_occurrences"].as_array().unwrap().len(),1);assert_eq!(line["cooking_occurrences"][0]["recipe"],json!(porridge_packet_ref().unwrap().recipe));
        assert_eq!(line["origins"].as_array().unwrap().len(),1);assert_eq!(line["origins"][0]["original_quantity"],line["demand"]["quantity"]);
        assert!(line["uses"].as_array().unwrap().iter().any(|item|item["use_key"]["kind"]=="preparation"));
        assert!(line["uses"].as_array().unwrap().iter().any(|item|item["use_key"]["kind"]=="consumption"));
        for item in line["uses"].as_array().unwrap(){assert_eq!(item["requirements"],json!([]));assert!(item["at"]["instant"].is_string());}
    }
    no_authority(&projected);assert_eq!(preflight(&state,0),projected);assert_eq!(canonical_bytes(&state).unwrap(),before);
    assert_eq!(good(json!({"op":"validate_state","state":state,"now":NOW})),state);assert_eq!(state["snapshot"]["privacy"]["use_ai_features"],false);assert_eq!(state["snapshot"]["privacy"]["include_household_requirements"],false);
}

#[test]
fn aggregate_context_keeps_each_occurrence_then_exclusion_removes_only_active_demand() {
    let state=select(&real(),0,6,&porridge_packet_ref().unwrap().recipe,0xe010);let projected=preflight(&state,0);
    for line in projected["lines"].as_array().unwrap(){let unit=line["demand"]["quantity"]["unit"].as_str().unwrap();assert_eq!(line["demand"]["quantity"]["amount"]["numerator"],if unit=="g"{"90"}else{"360"});assert_eq!(line["cooking_occurrences"].as_array().unwrap().len(),2);assert_eq!(line["origins"].as_array().unwrap().len(),2);}
    let mut week=state["snapshot"]["weeks"][0].clone();week["cells"][3]["choice"]=json!("skip");let excluded=edit(&state,week,0xe011);let smaller=preflight(&excluded,0);
    assert_eq!(excluded["snapshot"]["weeks"][0]["plan"]["selections"],state["snapshot"]["weeks"][0]["plan"]["selections"]);
    for line in smaller["lines"].as_array().unwrap(){assert_eq!(line["origins"].as_array().unwrap().len(),1);assert_eq!(line["cooking_occurrences"].as_array().unwrap().len(),1);assert_eq!(line["cooking_occurrences"][0]["slot"],excluded["snapshot"]["weeks"][0]["cells"][6]["slot"]);}
}

#[test]
fn core_accessor_resolves_actual_line_and_refuses_signed_quantity_forgery_and_stale_current_context() {
    let state=real();let (basis,food)=material(&state);let policy=policy();let prepared=prepare_match_context(&basis.context,&basis,&food.plan,&policy,Instant(NOW.into())).unwrap();
    let id=food.plan.demand.lines[0].line_id.clone();let (mut cloned,_,origins)=prepared.line_context(&id).unwrap();let original=cloned.quantity.clone();cloned.quantity.amount=PositiveRational::new(999,1).unwrap();
    assert_eq!(prepared.line_context(&cloned.line_id).unwrap().0.quantity,original,"Caller quantities cannot replace a native derived line");assert!(!origins.is_empty());
    let unknown=DemandLineId(format!("{}{}",&id.0[..3],"f".repeat(32)));assert_eq!(prepared.line_context(&unknown).unwrap_err().code,ErrorCode::MissingReference);
    let mut forged=food.plan.clone();forged.demand.lines[0].quantity.amount=PositiveRational::new(999,1).unwrap();forged.demand.digest=demand_digest(&DemandMaterial{schema_version:SchemaVersion,input_plan_digest:forged.input_plan_digest.clone(),lines:forged.demand.lines.clone()}).unwrap();forged.digest=derived_digest(&DerivedMaterial::from(&forged)).unwrap();
    assert!(prepare_match_context(&basis.context,&basis,&forged,&policy,Instant(NOW.into())).is_err(),"Re-signing a changed demand cannot satisfy original-batch quantity closure");
    let mut current=basis.context.clone();current.household.preferences.quick_meals=!current.household.preferences.quick_meals;
    assert_eq!(prepare_match_context(&current,&basis,&food.plan,&policy,Instant(NOW.into())).unwrap_err().code,ErrorCode::ContextStale);
}

#[test]
fn cold_home_context_retains_both_atomic_preparation_phases_and_nine_original_demand_lines() {
    let state=admit(&setup(),three_bean_packet_ref().unwrap(),0xe020);let state=select(&state,0,4,&three_bean_packet_ref().unwrap().recipe,0xe021);let projected=preflight(&state,0);
    assert_eq!(projected["lines"].as_array().unwrap().len(),9);let phases:std::collections::BTreeSet<_>=projected["lines"].as_array().unwrap().iter().flat_map(|line|line["uses"].as_array().unwrap()).filter(|item|item["use_key"]["kind"]=="preparation").map(|item|item["use_key"]["data"]["phase"]["prep_id"].as_u64().unwrap()).collect();assert_eq!(phases.len(),2);
    assert!(projected["lines"].as_array().unwrap().iter().flat_map(|line|line["uses"].as_array().unwrap()).any(|item|item["use_key"]["kind"]=="preparation"&&item["at"]["local_time"]=="11:15:00"));
    for line in projected["lines"].as_array().unwrap(){assert_eq!(line["cooking_occurrences"].as_array().unwrap().len(),1);assert_eq!(line["cooking_occurrences"][0]["recipe"],json!(three_bean_packet_ref().unwrap().recipe));assert_eq!(line["origins"][0]["original_quantity"],line["demand"]["quantity"]);}
    no_authority(&projected);
}

#[test]
fn food_conflicts_unknown_week_and_transport_authority_are_refused_without_fallback() {
    let state=real();let week_id=state["snapshot"]["weeks"][0]["range"]["week_id"].clone();let mut profile=state["snapshot"]["profile"].clone();profile["diners"][0]["restrictions"]=json!({"kind":"confirmed","data":{"confirmed_at":NOW,"requirements":[{"kind":"allergen_exclusion","data":{"code":"milk"}}]}});
    let changed=dispatch(&state,json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),0xe030);let failed=call(json!({"op":"product_match_context","state":changed,"week_id":week_id,"now":NOW}));assert_eq!(failed["ok"],false,"{failed}");assert!(failed.get("value").is_none());
    let unknown=call(json!({"op":"product_match_context","state":state,"week_id":"wk_ffffffffffffffffffffffffffffffff","now":NOW}));assert_eq!(unknown["ok"],false);assert_eq!(unknown["error"]["issues"][0]["path"],"/products/preflight/week");
    for key in ["derived","lines","meaning_admission","accepted_read","policy"]{let mut request=json!({"op":"product_match_context","state":state,"week_id":week_id,"now":NOW});request[key]=json!({"supported":true,"digest":"a".repeat(64)});assert_eq!(call(request)["ok"],false,"Transport {key} must not be a context input");}
    let mut forged=state.clone();forged["snapshot"]["profile"]["preferences"]["quick_meals"]=json!(true);assert_eq!(call(json!({"op":"product_match_context","state":forged,"week_id":week_id,"now":NOW}))["ok"],false,"Same-counter source bytes cannot bypass receipt audit");
    assert_eq!(good(json!({"op":"validate_state","state":state,"now":NOW})),state);
}

#[test]
fn later_catalogue_preflight_preserves_original_r3_packet_and_receipt_pins_on_cold_audit() {
    let state=real();let before=state["receipts"].clone();let packet=state["reviewed_packets"][0].clone();let original=preflight(&state,0);
    let later=admit(&state,aglio_packet_ref().unwrap(),0xe040);let projected=preflight(&later,0);assert_eq!(later["reviewed_packets"][0],packet);assert_eq!(later["receipts"].as_array().unwrap()[..before.as_array().unwrap().len()],before.as_array().unwrap()[..]);
    assert_eq!(projected["lines"],original["lines"]);assert_ne!(projected["catalogue_pin"],original["catalogue_pin"]);
    let input=json!({"op":"product_match_context","state":later,"week_id":later["snapshot"]["weeks"][0]["range"]["week_id"],"now":NOW});let cold=std::thread::spawn(move||good(input)).join().unwrap();assert_eq!(cold,projected);
    let replay=good(json!({"op":"dispatch","state":later,"command":state["receipts"].as_array().unwrap().last().unwrap()["command"],"now":LATER}));assert_eq!(replay["receipt"]["disposition"],"replayed");assert_eq!(replay["state"],later);
}

#[test]
fn empty_plan_refuses_food_preflight_and_all_excluded_selections_project_no_active_demand() {
    let empty=setup();let result=call(json!({"op":"product_match_context","state":empty,
        "week_id":empty["snapshot"]["weeks"][0]["range"]["week_id"],"now":NOW}));
    assert_eq!(result["ok"],false);assert_eq!(result["error"]["code"],"missing_reference");
    assert_eq!(result["error"]["issues"][0]["path"],"/products/preflight/food");
    assert!(result.get("value").is_none(),"No manufactured empty authority for an absent food basis");
    assert_eq!(good(json!({"op":"validate_state","state":empty,"now":NOW})),empty);

    let active=real();let mut week=active["snapshot"]["weeks"][0].clone();
    week["cells"][3]["choice"]=json!("skip");let excluded=edit(&active,week,0xe050);
    assert_eq!(excluded["snapshot"]["weeks"][0]["plan"]["selections"],active["snapshot"]["weeks"][0]["plan"]["selections"]);
    let projected=preflight(&excluded,0);assert_eq!(projected["lines"],json!([]));
    assert_eq!(projected["contains_engineering"],false);assert_eq!(projected["effect_enabled"],false);
    assert_eq!(projected["match_status"],"not_assessed");assert!(!projected["open_slots"].as_array().unwrap().is_empty());
    assert_ne!(projected["demand_digest"],preflight(&active,0)["demand_digest"]);
    no_authority(&projected);assert_eq!(good(json!({"op":"validate_state","state":excluded,"now":NOW})),excluded);
}

#[test]
fn active_retained_engineering_is_explicitly_flagged_and_never_qualified_for_effects() {
    // Choose the synthetic recipe before any real admission; later admission
    // retains it for inspection but does not offer it as a new real-food choice.
    let engineering=select(&setup(),0,2,&quick_ref(),0xe060);
    let state=admit(&engineering,porridge_packet_ref().unwrap(),0xe061);
    let projected=preflight(&state,0);assert_eq!(projected["contains_engineering"],true);
    assert_eq!(projected["lines"].as_array().unwrap().len(),1);
    let line=&projected["lines"][0];assert_eq!(line["demand"]["quantity"],quantity(200,"g","raw"));
    assert_eq!(line["cooking_occurrences"][0]["recipe"],json!(quick_ref()));
    assert_eq!(line["origins"][0]["original_quantity"],line["demand"]["quantity"]);
    assert_eq!(projected["effect_enabled"],false);assert_eq!(projected["retailer_status"],"unqualified");
    assert_eq!(projected["match_status"],"not_assessed");no_authority(&projected);
    assert_eq!(good(json!({"op":"validate_state","state":state,"now":NOW})),state);
}

#[test]
fn nonempty_current_hard_requirements_are_projected_exactly_without_product_policy_support() {
    // This is an owner-private DTO even with sharing off. Synthetic recipe
    // eligibility does not assert that any retailer product meets the diet.
    let initial=setup();let requirement=json!({"kind":"diet","data":{"code":"vegetarian"}});
    let mut profile=initial["snapshot"]["profile"].clone();profile["diners"][0]["restrictions"]=json!({
        "kind":"confirmed","data":{"confirmed_at":NOW,"requirements":[requirement]}});
    let state=dispatch(&initial,json!({"kind":"update_profile","data":{
        "expected_profile_revision":profile["revision"],"profile":profile}}),0xe070);
    let state=select(&state,0,2,&quick_ref(),0xe071);let projected=preflight(&state,0);
    let uses=projected["lines"][0]["uses"].as_array().unwrap();assert_eq!(uses.len(),2);
    for item in uses {assert_eq!(item["requirements"],if item["use_key"]["kind"]=="consumption"{
        json!([requirement])}else{json!([])},"Native requirements belong to the actual consumer, not an invented preparation attendee");}
    let consumer=uses.iter().find(|item|item["use_key"]["kind"]=="consumption").unwrap();
    assert_eq!(consumer["use_key"]["data"]["person"]["data"]["diner_id"],state["snapshot"]["profile"]["diners"][0]["diner_id"]);
    assert_eq!(projected["contains_engineering"],true);assert_eq!(projected["match_status"],"not_assessed");
    assert_eq!(projected["effect_enabled"],false);no_authority(&projected);
    assert_eq!(state["snapshot"]["privacy"]["use_ai_features"],false);
    assert_eq!(state["snapshot"]["privacy"]["include_household_requirements"],false);
    assert_eq!(good(json!({"op":"validate_state","state":state,"now":NOW})),state);
}

#[test]
fn current_have_enough_and_stale_renewal_use_native_action_without_reducing_full_food_demand() {
    let initial=select(&setup(),0,2,&quick_ref(),0xe080);let before=preflight(&initial,0);
    let original=&before["lines"][0];assert_eq!(original["demand"]["essential"],true);
    let state=dispatch(&initial,json!({"kind":"confirm_essentials","data":{"confirmations":[{
        "line_id":original["demand"]["line_id"],"displayed_quantity":original["demand"]["quantity"],
        "demand_digest":before["demand_digest"],"choice":"have_enough","confirmed_at":NOW}]}}),0xe081);
    let current=preflight(&state,0);assert_eq!(current["lines"][0]["shopping_action"],"have_enough");
    assert_eq!(current["lines"][0]["demand"],original["demand"]);
    assert_eq!(current["lines"][0]["origins"],original["origins"]);
    assert_eq!(current["lines"][0]["uses"],original["uses"]);
    assert_eq!(current["derived_digest"],before["derived_digest"]);

    let changed=select(&state,0,5,&quick_ref(),0xe082);let stale=preflight(&changed,0);
    assert_eq!(stale["lines"][0]["demand"]["line_id"],original["demand"]["line_id"]);
    assert_eq!(stale["lines"][0]["shopping_action"],"add_to_shop");
    assert_eq!(stale["lines"][0]["demand"]["quantity"],quantity(400,"g","raw"));
    assert_eq!(stale["lines"][0]["origins"].as_array().unwrap().len(),2);
    assert_eq!(stale["lines"][0]["cooking_occurrences"].as_array().unwrap().len(),2);
    assert_eq!(changed["essentials"],state["essentials"],"The old human confirmation remains history, not fresh stock evidence");
    let board=good(json!({"op":"board","state":changed,"week_id":changed["snapshot"]["weeks"][0]["range"]["week_id"],"now":NOW}));
    let native=&board["details"]["shopping_list"];assert_eq!(native["lines"][0]["confirmation_status"],"stale");
    assert_eq!(stale["lines"][0]["shopping_action"],native["lines"][0]["action"]);
    assert_eq!(stale["lines"][0]["demand"],native["lines"][0]["demand"]);
    assert_eq!(stale["effect_enabled"],false);no_authority(&stale);
    assert_eq!(good(json!({"op":"validate_state","state":changed,"now":NOW})),changed);
}
