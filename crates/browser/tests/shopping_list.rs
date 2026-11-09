//! Manual lists through the actual audited public boundary. Every positive
//! state/confirmation comes from ordinary commands; no receipt is spoofed.
use mstrchf_browser::{maker_packets::porridge_packet_ref, run_json};
use serde_json::{Value, json};
const NOW: &str = "2026-10-12T20:00:00Z";
const QUICK: &str = "rc_00000000000000000000000000000005";

fn call(request: Value) -> Value { serde_json::from_slice(&run_json(&serde_json::to_vec(&request).unwrap())).unwrap() }
fn good(request: Value) -> Value { let result=call(request); assert_eq!(result["ok"],true,"{result}"); result["value"].clone() }
fn init() -> Value { good(json!({"op":"initialize","demo":true,"today":"2026-10-12","now":NOW,"zone":"Europe/London","household_id":"hh_00000000000000000000000000000501","diner_id":"dn_00000000000000000000000000000501"})) }
fn envelope(state:&Value,body:Value,id:u32)->Value { json!({"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body}) }
fn dispatch(state:&Value,body:Value,id:u32)->Value { let command=envelope(state,body,id);good(json!({"op":"dispatch","state":state,"command_json":serde_json::to_string(&command).unwrap(),"command":command,"now":NOW}))["state"].clone() }
fn edit(state:&Value,wi:usize,week:Value,id:u32)->Value { assert_eq!(week["range"]["week_id"],state["snapshot"]["weeks"][wi]["range"]["week_id"]);dispatch(state,json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{"week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],"day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],"cells":week["cells"],"plan":week["plan"]}}}),id) }
fn board(state:&Value,wi:usize,si:usize,options:Value)->Value { let week=&state["snapshot"]["weeks"][wi];good(json!({"op":"board","state":state,"week_id":week["range"]["week_id"],"slot":week["cells"][si]["slot"],"options":options,"now":NOW})) }
fn save(state:&Value,wi:usize,si:usize,recipe:&str,options:Value,id:u32)->Value { let offers=board(state,wi,si,options);let chosen=offers["choices"].as_array().unwrap().iter().find(|choice|choice["selection"]["recipe"]["recipe_id"]==recipe).expect("expected an actual eligible core choice");let mut week=state["snapshot"]["weeks"][wi].clone();week["plan"]=chosen["plan"].clone();edit(state,wi,week,id) }
fn unrestricted(state:&Value,id:u32)->Value { let mut profile=state["snapshot"]["profile"].clone();profile["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":NOW}});dispatch(state,json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),id) }
fn enable(state:&Value,wi:usize,indices:&[usize],id:u32)->Value { let mut week=state["snapshot"]["weeks"][wi].clone();for index in indices {let meal=week["cells"][*index]["slot"]["meal_type"].as_str().unwrap().to_owned();week["meal_type_enabled"][meal]=json!(true);week["cells"][*index]["choice"]=json!("at_home");}edit(state,wi,week,id) }
fn admit(state:&Value,id:u32)->Value {dispatch(state,json!({"kind":"admit_reviewed_recipe","data":{"packet":porridge_packet_ref().unwrap(),"expected_catalogue_revision":"1"}}),id)}
fn list(state:&Value,wi:usize)->Value {board(state,wi,2,Value::Null)["details"]["shopping_list"].clone()}
fn confirmation(row:&Value,digest:&Value,choice:&str)->Value {json!({"line_id":row["demand"]["line_id"],"displayed_quantity":row["demand"]["quantity"],"demand_digest":digest,"choice":choice,"confirmed_at":NOW})}
fn confirm(state:&Value,list:&Value,choice:&str,id:u32)->Value {let row=list["lines"].as_array().unwrap().iter().find(|row|row["demand"]["essential"]==true).unwrap();dispatch(state,json!({"kind":"confirm_essentials","data":{"confirmations":[confirmation(row,&list["demand_digest"],choice)]}}),id)}
fn cold(state:Value)->Value {std::thread::spawn(move||good(json!({"op":"validate_state","state":state,"now":NOW}))).join().unwrap()}

#[test]
fn two_real_servings_have_exact_forms_origins_and_honest_unqualified_stock_status() {
    let state=enable(&unrestricted(&init(),1),0,&[0,3],2);
    let state=admit(&state,3);let real=porridge_packet_ref().unwrap().recipe.recipe_id.0;
    let state=save(&state,0,0,&real,Value::Null,4);let state=save(&state,0,3,&real,Value::Null,5);
    let before=state.clone();let projected=list(&state,0);
    assert_eq!(projected["format"],"mstrchf-manual-shopping-list");
    assert_eq!(projected["contains_engineering"],false);
    assert_eq!(projected["catalogue_pin"]["revision"],"2");
    assert_eq!(projected["retailer_status"],"unqualified");assert_eq!(projected["price_status"],"unavailable");
    assert!(!projected["open_slots"].as_array().unwrap().is_empty(),"Open meals cannot be described as a complete weekly shop");
    assert_eq!(projected["lines"].as_array().unwrap().len(),3);
    for row in projected["lines"].as_array().unwrap() {
        let quantity=&row["demand"]["quantity"];
        assert_eq!(quantity["amount"]["denominator"],"1");
        assert_eq!(quantity["amount"]["numerator"],if quantity["unit"]=="g" {"90"}else{"360"});
        assert!(matches!(quantity["form"].as_str().unwrap(),"dry"|"raw"|"prepared"));
        assert_eq!(row["confirmation_status"],"not_offered");assert_eq!(row["action"],"add_to_shop");
        assert_eq!(row["origins"].as_array().unwrap().len(),2);
        for origin in row["origins"].as_array().unwrap(){assert_eq!(origin["admission"],"personal_reviewed");assert_eq!(origin["recipe"],serde_json::to_value(porridge_packet_ref().unwrap().recipe).unwrap());assert_eq!(origin["target_slots"].as_array().unwrap().len(),1);}
    }
    assert_eq!(state,before,"Projection has no owner or receipt effect");assert_eq!(cold(state.clone()),before);
    // A direct caller can submit an exact non-essential confirmation, but its
    // stored checkbox intent must not create an unsupported pantry deduction.
    let row=&projected["lines"][0];let attempted=dispatch(&state,json!({"kind":"confirm_essentials","data":{"confirmations":[confirmation(row,&projected["demand_digest"],"have_enough")]}}),6);
    let unchanged=list(&attempted,0);assert!(unchanged["lines"].as_array().unwrap().iter().all(|row|row["action"]=="add_to_shop"&&row["confirmation_status"]=="not_offered"));
    assert_eq!(unchanged["derived_digest"],projected["derived_digest"]);
}

#[test]
fn bound_pantry_deduction_renews_after_demand_change_without_changing_food_derivation() {
    let state=save(&init(),0,2,QUICK,Value::Null,10);let before=board(&state,0,2,Value::Null);
    let state=confirm(&state,&before["details"]["shopping_list"],"have_enough",11);
    let after=board(&state,0,2,Value::Null);assert_eq!(after["details"]["derived"],before["details"]["derived"]);
    assert_eq!(after["details"]["shopping_list"]["lines"][0]["action"],"have_enough");
    assert_eq!(after["details"]["shopping_list"]["lines"][0]["confirmation_status"],"current");
    let state=save(&state,0,5,QUICK,Value::Null,12);let stale=list(&state,0);
    assert_eq!(stale["lines"][0]["action"],"add_to_shop");assert_eq!(stale["lines"][0]["confirmation_status"],"stale");
    assert_eq!(stale["lines"][0]["demand"]["quantity"]["amount"]["numerator"],"400");
    let state=confirm(&state,&stale,"add_to_shop",13);assert_eq!(list(&state,0)["lines"][0]["confirmation_status"],"current");
    assert_eq!(cold(state.clone()),state);
}

#[test]
fn original_batch_is_counted_once_and_trace_retains_both_consumption_meals() {
    let state=enable(&init(),0,&[4],20);
    let state=save(&state,0,2,QUICK,json!({"make_extra":true}),21);
    let source=state["snapshot"]["weeks"][0]["cells"][2]["slot"].clone();
    let state=save(&state,0,4,QUICK,json!({"leftover_source":source}),22);
    let projected=list(&state,0);assert_eq!(projected["lines"].as_array().unwrap().len(),1);
    let row=&projected["lines"][0];assert_eq!(row["demand"]["quantity"]["amount"]["numerator"],"400");
    assert_eq!(row["origins"].as_array().unwrap().len(),1);
    assert_eq!(row["origins"][0]["source_slot"],source);assert_eq!(row["origins"][0]["target_slots"].as_array().unwrap().len(),2);
    let mut week=state["snapshot"]["weeks"][0].clone();week["cells"][2]["choice"]=json!("skip");
    let excluded=edit(&state,0,week,23);let blocked=board(&excluded,0,4,Value::Null);
    assert!(!blocked["details"]["food_error"].is_null());assert!(blocked["details"]["shopping_list"].is_null(),"An excluded source must not leave a misleading complete list for its dependent leftover");
}

#[test]
fn exclusion_removes_only_its_demand_and_mixed_food_keeps_engineering_provenance() {
    let state=save(&init(),0,2,QUICK,Value::Null,30);
    let state=enable(&unrestricted(&state,31),0,&[0],32);let state=admit(&state,33);
    let real=porridge_packet_ref().unwrap().recipe.recipe_id.0;let state=save(&state,0,0,&real,Value::Null,34);
    let mixed=list(&state,0);assert_eq!(mixed["lines"].as_array().unwrap().len(),4);assert_eq!(mixed["contains_engineering"],true);
    assert!(mixed["lines"].as_array().unwrap().iter().any(|row|row["origins"][0]["admission"]=="engineering"));
    let mut week=state["snapshot"]["weeks"][0].clone();week["meal_type_enabled"]["dinner"]=json!(false);
    let excluded=edit(&state,0,week,35);let only_real=list(&excluded,0);
    assert_eq!(only_real["lines"].as_array().unwrap().len(),3);assert_eq!(only_real["contains_engineering"],false);
    assert_eq!(excluded["snapshot"]["weeks"][0]["plan"]["selections"],state["snapshot"]["weeks"][0]["plan"]["selections"]);
}

#[test]
fn conflicted_first_week_cannot_abort_exact_confirmation_in_a_valid_independent_week() {
    let state=save(&unrestricted(&init(),40),1,2,QUICK,Value::Null,41);
    let state=enable(&state,0,&[0],42);let state=admit(&state,43);
    let real=porridge_packet_ref().unwrap().recipe.recipe_id.0;let state=save(&state,0,0,&real,Value::Null,44);
    let mut profile=state["snapshot"]["profile"].clone();profile["diners"][0]["restrictions"]=json!({"kind":"confirmed","data":{"confirmed_at":NOW,"requirements":[{"kind":"diet","data":{"code":"vegetarian"}}]}});
    let state=dispatch(&state,json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),45);
    assert!(list(&state,0).is_null());let valid=list(&state,1);assert!(valid.is_object());
    let state=confirm(&state,&valid,"have_enough",46);
    assert_eq!(list(&state,1)["lines"][0]["action"],"have_enough");assert_eq!(cold(state.clone()),state);
    let mut wrong=confirmation(&valid["lines"][0],&valid["demand_digest"],"have_enough");wrong["displayed_quantity"]["amount"]["numerator"]=json!("201");
    let command=envelope(&state,json!({"kind":"confirm_essentials","data":{"confirmations":[wrong]}}),47);
    assert_eq!(call(json!({"op":"dispatch","state":state,"command":command,"now":NOW}))["error"]["code"],"digest_mismatch");
}

#[test]
fn new_list_preserves_exact_legacy_receipt_food_and_accepted_results() {
    let fixture:Value=serde_json::from_slice(include_bytes!("fixtures/legacy-engineering-receipts.json")).unwrap();
    let state=&fixture["state"];let result=board(state,0,2,Value::Null);
    assert_eq!(result["details"]["derived"],fixture["derived"]);assert_eq!(result["details"]["essentials"],state["essentials"]);
    assert_eq!(cold(state.clone()),*state);assert_eq!(list(state,0)["contains_engineering"],true);
    let replay=good(json!({"op":"return_proposal","state":state,"result":fixture["accepted_result"],"now":NOW}));
    assert_eq!(replay["status"],"already_applied");assert_eq!(replay["state"],*state);
}
