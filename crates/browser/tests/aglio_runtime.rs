//! Accepted Aglio through audited public commands, exact lists and proposal IO.
use mstrchf_browser::{maker_packets::*,run_json};
use mstrchf_contracts::*;
use serde_json::{Value,json};
const NOW:&str="2026-10-12T20:00:00Z";
const EARLY:&str="2026-10-12T19:02:01Z";
fn call(request:Value)->Value{serde_json::from_slice(&run_json(&serde_json::to_vec(&request).unwrap())).unwrap()}
fn good(request:Value)->Value{let out=call(request);assert_eq!(out["ok"],true,"{out}");out["value"].clone()}
fn envelope(state:&Value,body:Value,id:u32)->Value{json!({"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body})}
fn apply_at(state:&Value,body:Value,id:u32,now:&str)->Value{let command=envelope(state,body,id);good(json!({"op":"dispatch","state":state,"command":command,"now":now}))["state"].clone()}
fn apply(state:&Value,body:Value,id:u32)->Value{apply_at(state,body,id,NOW)}
fn admission(state:&Value,packet:ReviewedRecipePacketRef,id:u32)->Value{let revision=(state["reviewed_packets"].as_array().map_or(0,Vec::len)+1).to_string();apply(state,json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":revision}}),id)}
fn edit(state:&Value,week:Value,id:u32)->Value{apply(state,json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{"week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],"day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],"cells":week["cells"],"plan":week["plan"]}}}),id)}
fn base()->Value{
    let mut state:Value=serde_json::from_slice::<Value>(include_bytes!("fixtures/legacy-engineering-receipts.json")).unwrap()["state"].clone();
    let mut profile=state["snapshot"]["profile"].clone();profile["diners"][0]["age_band"]=json!("adult");profile["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":"2026-10-12T18:00:00Z"}});
    profile["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});
    state=apply_at(&state,json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),100,"2026-10-12T18:00:00Z");
    let mut week=state["snapshot"]["weeks"][0].clone();week["plan"]["selections"]=json!([]);week["plan"]["batch_links"]=json!([]);week["meal_type_enabled"]["breakfast"]=json!(true);week["meal_type_enabled"]["dinner"]=json!(true);
    // Backdated ordinary saved context for the evidence-clock rejection case.
    let command=envelope(&state,json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{"week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],"day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],"cells":week["cells"],"plan":week["plan"]}}}),101);
    good(json!({"op":"dispatch","state":state,"command":command,"now":"2026-10-12T18:00:00Z"}))["state"].clone()
}
fn board(state:&Value,index:usize)->Value{let week=&state["snapshot"]["weeks"][0];good(json!({"op":"board","state":state,"week_id":week["range"]["week_id"],"slot":week["cells"][index]["slot"],"now":NOW}))}
fn save(state:&Value,index:usize,reference:&RecipeRef,id:u32)->Value{let out=board(state,index);let choice=out["choices"].as_array().unwrap().iter().find(|value|value["selection"]["recipe"]==serde_json::to_value(reference).unwrap()).expect("expected exact admitted eligible choice");let mut week=state["snapshot"]["weeks"][0].clone();week["plan"]=choice["plan"].clone();edit(state,week,id)}
fn cold(state:Value)->Value{std::thread::spawn(move||good(json!({"op":"validate_state","state":state,"now":NOW}))).join().unwrap()}
fn list(state:&Value)->Value{board(state,8)["details"]["shopping_list"].clone()}

#[test]
fn clock_refusal_then_real_admission_and_exact_demand_preserve_legacy_receipts() {
    let before=base();let packet=aglio_packet_ref().unwrap();
    assert_eq!(known_packet(&packet).unwrap().not_before().0,"2026-10-12T19:02:02Z");
    assert!(!board(&before,8)["choices"].as_array().unwrap().iter().any(|choice|choice["selection"]["recipe"]==serde_json::to_value(&packet.recipe).unwrap()));
    let command=envelope(&before,json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":"1"}}),102);
    assert_eq!(call(json!({"op":"dispatch","state":before,"command":command,"now":EARLY}))["error"]["code"],"authority_denied");
    let admitted=admission(&before,packet.clone(),102);let saved=save(&admitted,8,&packet.recipe,103);
    assert_eq!(cold(saved.clone()),saved);
    for index in 0..before["receipts"].as_array().unwrap().len(){assert_eq!(saved["receipts"][index],before["receipts"][index]);}
    let projected=list(&saved);assert_eq!(projected["contains_engineering"],false);assert_eq!(projected["lines"].as_array().unwrap().len(),4);
    for row in projected["lines"].as_array().unwrap(){let id=row["demand"]["ingredient_id"].as_str().unwrap();assert_eq!(row["demand"]["essential"],id.ends_with("04")||id.ends_with("05"));assert_eq!(row["origins"][0]["recipe"],serde_json::to_value(&packet.recipe).unwrap());}
    let mut too_early=saved;let receipt=&mut too_early["receipts"][6];receipt["committed_at"]=json!(EARLY);
    assert_eq!(call(json!({"op":"validate_state","state":too_early,"now":NOW}))["error"]["code"],"authority_denied");
}

#[test]
fn exact_pasta_stock_confirmation_deducts_then_stales_after_new_current_demand() {
    let state=save(&admission(&base(),aglio_packet_ref().unwrap(),110),8,&aglio_packet_ref().unwrap().recipe,111);
    let projected=list(&state);let row=projected["lines"].as_array().unwrap().iter().find(|row|row["demand"]["ingredient_id"]=="ig_80000000000000000000000000000004").unwrap();
    let confirmation=json!({"line_id":row["demand"]["line_id"],"choice":"have_enough","displayed_quantity":row["demand"]["quantity"],"demand_digest":projected["demand_digest"],"confirmed_at":NOW});
    let confirmed=apply(&state,json!({"kind":"confirm_essentials","data":{"confirmations":[confirmation]}}),112);
    let current=list(&confirmed);assert_eq!(current["lines"].as_array().unwrap().iter().find(|row|row["demand"]["ingredient_id"]=="ig_80000000000000000000000000000004").unwrap()["confirmation_status"],"current");
    assert_eq!(board(&confirmed,8)["details"]["derived"],board(&state,8)["details"]["derived"]);
    let changed=save(&confirmed,11,&aglio_packet_ref().unwrap().recipe,113);let newer=list(&changed);
    let pasta=newer["lines"].as_array().unwrap().iter().find(|row|row["demand"]["ingredient_id"]=="ig_80000000000000000000000000000004").unwrap();
    assert_eq!(pasta["confirmation_status"],"stale");assert_eq!(pasta["action"],"add_to_shop");assert_eq!(pasta["demand"]["quantity"]["amount"]["numerator"],"250");
    let stale=envelope(&changed,json!({"kind":"confirm_essentials","data":{"confirmations":[confirmation]}}),114);
    assert_eq!(call(json!({"op":"dispatch","state":changed,"command":stale,"now":NOW}))["ok"],false);
    assert_eq!(cold(changed.clone()),changed);
}

#[test]
fn ordinary_dual_admission_order_keeps_scope_and_original_acknowledgements() {
    for reverse in[false,true]{
        let before=base();let first=if reverse{aglio_packet_ref().unwrap()}else{porridge_packet_ref().unwrap()};let second=if reverse{porridge_packet_ref().unwrap()}else{aglio_packet_ref().unwrap()};
        let once=admission(&before,first,120);let mut both=admission(&once,second,121);
        assert_eq!(both["receipts"].as_array().unwrap()[6],once["receipts"].as_array().unwrap()[6]);
        both=save(&both,3,&porridge_packet_ref().unwrap().recipe,122);both=save(&both,8,&aglio_packet_ref().unwrap().recipe,123);
        assert_eq!(cold(both.clone()),both);assert_eq!(board(&both,8)["details"]["catalogue_revision"],"3");
        let envelope=&once["receipts"][6]["command"];let replay=good(json!({"op":"dispatch","state":both,"command":envelope,"now":NOW}));
        assert_eq!(replay["state"],both);assert_eq!(replay["receipt"]["response_digest"],once["receipts"][6]["response_digest"]);
        let water=list(&both)["lines"].as_array().unwrap().iter().find(|row|row["demand"]["ingredient_id"]=="ig_80000000000000000000000000000002").unwrap().clone();
        assert_eq!(water["demand"]["quantity"]["amount"]["numerator"],"1680");assert_eq!(water["demand"]["essential"],false);
    }
}

#[test]
fn exact_aglio_proposal_roundtrip_rejects_cross_recipe_return_and_requires_explicit_apply() {
    let state=admission(&base(),aglio_packet_ref().unwrap(),130);let week=&state["snapshot"]["weeks"][0];let slot=week["cells"][8]["slot"].clone();
    let prepared=good(json!({"op":"prepare_proposal","state":state,"week_id":week["range"]["week_id"],"slots":[slot],"mode":"fill_open","allow_repeats":false,"nonce":"aglio-boundary-return","now":NOW}));
    let request=&prepared["request"];assert_eq!(request["slots"][0]["choices"].as_array().unwrap().len(),1);
    let result=json!({"proposal_id":request["proposal_id"],"context_digest":request["context_digest"],"selections":[{"slot":slot,"recipe":aglio_packet_ref().unwrap().recipe}]});
    let mut wrong=result.clone();wrong["selections"][0]["recipe"]=serde_json::to_value(porridge_packet_ref().unwrap().recipe).unwrap();
    assert_eq!(call(json!({"op":"return_proposal","state":prepared["state"],"result":wrong,"now":NOW}))["ok"],false);
    let returned=good(json!({"op":"return_proposal","state":prepared["state"],"result":result,"now":NOW}));
    assert_eq!(returned["status"],"review_ready");assert_eq!(returned["state"]["snapshot"],state["snapshot"]);
    let saved=apply(&returned["state"],returned["command"].clone(),131);assert_eq!(cold(saved.clone()),saved);
    let duplicate=good(json!({"op":"return_proposal","state":saved,"result":result,"now":NOW}));assert_eq!(duplicate["status"],"already_applied");assert_eq!(duplicate["state"],saved);assert!(duplicate["command"].is_null());
}

#[test]
fn compiled_records_and_current_hard_requirements_cannot_be_forged_into_scope() {
    let admitted=admission(&base(),aglio_packet_ref().unwrap(),140);
    let mut forged=admitted.clone();forged["reviewed_packets"][0]["packet"]["maker_relevant_essential_ingredient_ids"]=json!(["ig_80000000000000000000000000000002"]);
    assert_eq!(call(json!({"op":"validate_state","state":forged,"now":NOW}))["ok"],false);
    let mut forged=admitted.clone();forged["reviewed_packets"][0]["evidence_basis"]=json!({"admitted":[]});
    assert_eq!(call(json!({"op":"validate_state","state":forged,"now":NOW}))["error"]["code"],"unknown_field");
    let saved=save(&admitted,8,&aglio_packet_ref().unwrap().recipe,141);let original=saved["snapshot"]["weeks"][0]["plan"].clone();
    let mut profile=saved["snapshot"]["profile"].clone();profile["diners"][0]["restrictions"]=json!({"kind":"confirmed","data":{"confirmed_at":NOW,"requirements":[{"kind":"diet","data":{"code":"vegan"}}]}});
    let changed=apply(&saved,json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),142);
    assert_eq!(changed["snapshot"]["weeks"][0]["plan"],original);
    assert!(!changed["snapshot"]["conflicts"].as_array().unwrap().is_empty());assert!(board(&changed,8)["choices"].as_array().unwrap().is_empty());assert!(board(&changed,8)["details"]["derived"].is_null());assert_eq!(cold(changed.clone()),changed);
}
