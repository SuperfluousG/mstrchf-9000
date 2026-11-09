//! Native owner commands using synthetic households and fictional orders only.
//! These checks qualify exact allocation/scope mechanics, never actual chicken,
//! labels, refrigeration, physical separation, capacity or human cooking.
use mstrchf_browser::{run_json, BrowserState, BoardOptions, catalogue::RuntimeCatalogue,
    maker_packets::*, personal_scope::*};
use mstrchf_contracts::*;
use serde_json::{Value, json};

const NOW: &str = "2026-10-13T06:00:00Z";
const DEST: &str = "cd_10000000000000000000000000000001";
fn call(input: Value) -> Value { serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap() }
fn good(input: Value) -> Value { let answer=call(input);assert_eq!(answer["ok"],true,"{answer}");answer["value"].clone() }
fn command(state:&Value,body:Value,id:u32)->Value {json!({"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body})}
fn apply(state:&Value,body:Value,id:u32)->Value {good(json!({"op":"dispatch","state":state,"command":command(state,body,id),"now":NOW}))["state"].clone()}
fn week_body(week:Value,plan:bool)->Value {json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{"week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],"day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],"cells":week["cells"],"plan":if plan{week["plan"].clone()}else{Value::Null}}}})}
fn edit(state:&Value,week:Value,id:u32,plan:bool)->Value {apply(state,week_body(week,plan),id)}
fn diet(code:&str)->Value {json!({"kind":"diet","data":{"code":code}})}
fn declaration(requirement:&Value)->Value {if requirement.is_null(){json!({"kind":"confirmed_none","data":{"confirmed_at":NOW}})}else{json!({"kind":"confirmed","data":{"confirmed_at":NOW,"requirements":[requirement]}})}}
fn profile(state:&Value,p:Value,id:u32)->Value {apply(state,json!({"kind":"update_profile","data":{"expected_profile_revision":p["revision"],"profile":p}}),id)}
fn setup()->Value {
    // Retain a genuine old receipt prefix rather than inventing imported proof.
    let mut state=serde_json::from_slice::<Value>(include_bytes!("fixtures/legacy-engineering-receipts.json")).unwrap()["state"].clone();
    let mut policy=state["snapshot"]["privacy"].clone();policy["use_ai_features"]=json!(false);policy["include_household_requirements"]=json!(false);
    state=apply(&state,json!({"kind":"set_privacy","data":{"expected_policy_revision":policy["revision"],"policy":policy}}),0xf800);
    let mut p=state["snapshot"]["profile"].clone();p["diners"][0]["age_band"]=json!("adult");p["diners"][0]["restrictions"]=declaration(&Value::Null);
    p["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});p["preferences"]["equipment"]=json!(["fridge","hob"]);
    state=profile(&state,p,0xf801);let mut w=state["snapshot"]["weeks"][0].clone();w["plan"]["selections"]=json!([]);w["plan"]["batch_links"]=json!([]);w["meal_type_enabled"]["lunch"]=json!(true);
    edit(&state,w,0xf802,true)
}
fn group(state:&Value,guest:bool,first:&Value,second:&Value)->Value {
    let mut p=state["snapshot"]["profile"].clone();p["diners"][0]["restrictions"]=declaration(first);
    if !guest {let mut second_diner=p["diners"][0].clone();second_diner["diner_id"]=json!("dn_80000000000000000000000000000008");second_diner["restrictions"]=declaration(second);p["diners"].as_array_mut().unwrap().push(second_diner);}
    let state=profile(state,p,0xf803);let mut w=state["snapshot"]["weeks"][0].clone();
    if guest {w["cells"][4]["occasion"]=json!({"occasion_id":"oc_80000000000000000000000000000008","host_diner_ids":w["cells"][4]["attendee_ids"],"guests":[{"guest_id":"gu_80000000000000000000000000000008","age_band":"adult","restrictions":declaration(second),"portion":{"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]}}]});}
    else {w["cells"][4]["attendee_ids"].as_array_mut().unwrap().push(json!("dn_80000000000000000000000000000008"));}
    edit(&state,w,0xf804,false)
}
fn admit(state:&Value,packet:ReviewedRecipePacketRef,id:u32)->Value {let revision=1+state["reviewed_packets"].as_array().map_or(0,Vec::len);apply(state,json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":revision.to_string()}}),id)}
fn ready(guest:bool,first:&Value,second:&Value)->Value {
    let state=group(&setup(),guest,first,second);
    let state=admit(&state,three_bean_packet_ref().unwrap(),0xf805);
    let state=admit(&state,three_bean_two_unit_vegetarian_packet_ref().unwrap(),0xf806);
    let state=admit(&state,chilled_chicken_extra_packet_ref().unwrap(),0xf807);
    admit(&state,three_bean_two_unit_chicken_extra_packet_ref().unwrap(),0xf808)
}
fn person(state:&Value,guest:bool,second:bool)->Value {
    if guest&&second {let occasion=&state["snapshot"]["weeks"][0]["cells"][4]["occasion"];json!({"kind":"guest","data":{"occasion_id":occasion["occasion_id"],"guest_id":occasion["guests"][0]["guest_id"]}})}
    else {json!({"kind":"diner","data":{"diner_id":state["snapshot"]["profile"]["diners"][usize::from(second)]["diner_id"]}})}
}
fn board_slot(state:&Value,index:usize,options:Value)->Value {let w=&state["snapshot"]["weeks"][0];good(json!({"op":"board","state":state,"week_id":w["range"]["week_id"],"slot":w["cells"][index]["slot"],"options":options,"now":NOW}))}
fn board(state:&Value,options:Value)->Value {board_slot(state,4,options)}
fn offered(b:&Value,reference:&RecipeRef)->bool {b["choices"].as_array().unwrap().iter().any(|c|c["selection"]["recipe"]==json!(reference))}
fn chosen(b:&Value,reference:&RecipeRef)->Value {b["choices"].as_array().unwrap().iter().find(|c|c["selection"]["recipe"]==json!(reference)).expect("actual exact eligible choice").clone()}
fn select(state:&Value,options:Value,id:u32)->Value {let c=chosen(&board(state,options),&three_bean_two_unit_chicken_extra_packet_ref().unwrap().recipe);let mut w=state["snapshot"]["weeks"][0].clone();w["plan"]=c["plan"].clone();edit(state,w,id,true)}
fn saved(guest:bool,first:&Value,second:&Value,recipient_second:bool)->Value {let state=ready(guest,first,second);select(&state,json!({"extra_recipients":[person(&state,guest,recipient_second)]}),0xf809)}
fn cold(state:Value)->Value {std::thread::spawn(move||good(json!({"op":"validate_state","state":state,"now":NOW}))).join().unwrap()}
fn qty(amount:u64,unit:&str,form:&str)->Value {json!({"amount":{"numerator":amount.to_string(),"denominator":"1"},"unit":unit,"form":form})}
fn refuse_plan(state:&Value,week:Value,id:u32)->Value {let result=call(json!({"op":"dispatch","state":state,"command":command(state,week_body(week,true),id),"now":NOW}));assert_eq!(result["ok"],false,"{result}");result}
fn book(state:&Value)->Value {
    let w=&state["snapshot"]["weeks"][0];let b=board(state,Value::Null);
    let state=apply(state,json!({"kind":"approve_plan","data":{"week_id":w["range"]["week_id"],"plan_revision":w["plan"]["revision"],"derived_digest":b["details"]["derived"]["digest"]}}),0xf840);
    let w=&state["snapshot"]["weeks"][0];let moment=|time:&str|good(json!({"op":"resolve_moment","date":w["cells"][4]["slot"]["date"],"time":time,"zone":w["range"]["zone"],"resolution":"unambiguous"}));
    apply(&state,json!({"kind":"confirm_order","data":{"confirmation":{"week_id":w["range"]["week_id"],"approval_id":w["approved_plan"]["approval_id"],"delivery_start":moment("10:00:00"),"delivery_end":moment("10:59:00"),"confirmed_at":NOW,"source":{"kind":"user_confirmation","data":{}}}}}),0xf841)
}

#[test]
fn zero_extra_two_vegetarian_base_needs_no_chicken_demand_but_keeps_both_independent_admissions() {
    let ready=ready(true,&diet("vegetarian"),&diet("vegetarian"));let state=select(&ready,Value::Null,0xf810);
    let b=board(&state,Value::Null);let d=&b["details"]["derived"];
    assert_eq!(state["reviewed_packets"].as_array().unwrap().len(),4);
    assert_eq!(d["demand"]["lines"].as_array().unwrap().len(),9);assert_eq!(d["allocations"].as_array().unwrap().len(),2);
    assert_eq!(d["prep_sessions"].as_array().unwrap().len(),1);assert_eq!(state["snapshot"]["weeks"][0]["plan"]["selections"][0]["extras"],json!([]));
    assert!(!offered(&b,&three_bean_packet_ref().unwrap().recipe));assert!(!offered(&b,&chilled_chicken_extra_packet_ref().unwrap().recipe));
    assert_eq!(cold(state.clone()),state);
}

#[test]
fn exact_named_resident_or_guest_gets_one_extra_serving_and_once_only_quantities_and_clocks() {
    let vegetarian=diet("vegetarian");
    for guest in [false,true] {for recipient_second in [false,true] {
        let state=saved(guest,if recipient_second{&vegetarian}else{&Value::Null},if recipient_second{&Value::Null}else{&vegetarian},recipient_second);
        let recipient=person(&state,guest,recipient_second);let b=board(&state,Value::Null);let d=&b["details"]["derived"];
        let selection=&state["snapshot"]["weeks"][0]["plan"]["selections"][0];assert_eq!(selection["recipe"],json!(three_bean_two_unit_chicken_extra_packet_ref().unwrap().recipe));
        assert_eq!(selection["servings"].as_array().unwrap().len(),2);assert_eq!(selection["extras"].as_array().unwrap().len(),1);assert_eq!(selection["extras"][0]["recipients"].as_array().unwrap().len(),1);assert_eq!(selection["extras"][0]["recipients"][0]["person"],recipient);
        // Derived allocations conserve original reusable base components only.
        // Fresh extras retain their recipient serving in ExtraSelection and
        // their own occurrence/demand/session, never an invented stored batch.
        let allocations=d["allocations"].as_array().unwrap();assert_eq!(allocations.len(),2);
        for serving in selection["servings"].as_array().unwrap() {let allocated=allocations.iter().filter(|row|row["person"]==serving["person"]).collect::<Vec<_>>();assert_eq!(allocated.len(),1);assert_eq!(allocated[0]["component_id"],"co_80000000000000000000000000000008");assert_eq!(allocated[0]["quantity"],qty(1,"each","prepared"));}
        let selected_extra=&selection["extras"][0]["recipients"][0];assert_eq!(selected_extra["servings"],json!({"numerator":"1","denominator":"1"}));assert_eq!(selected_extra["preset_ref"],json!({"recipe":chilled_chicken_extra_packet_ref().unwrap().recipe,"preset":"regular","age_band":"adult","meal_type":"lunch"}));
        assert_eq!(d["batches"].as_array().unwrap().len(),1);assert_eq!(d["batches"][0]["components"].as_array().unwrap().len(),1);assert_eq!(d["batches"][0]["components"][0]["component_id"],"co_80000000000000000000000000000008");
        let lines=d["demand"]["lines"].as_array().unwrap();assert_eq!(lines.len(),10);
        for (suffix,amount,unit,form) in [("07",160,"g","drained"),("08",160,"g","drained"),("09",160,"g","drained"),("0a",80,"g","raw"),("0b",40,"g","raw"),("0c",120,"g","raw"),("0d",20,"ml","prepared"),("0e",30,"ml","prepared"),("0f",10,"g","prepared"),("10",50,"g","cooked")] {let line=lines.iter().find(|row|row["ingredient_id"].as_str().unwrap().ends_with(suffix)).unwrap();assert_eq!(line["quantity"],qty(amount,unit,form));assert_eq!(line["essential"],false);}
        let chicken=lines.iter().find(|line|line["ingredient_id"]=="ig_80000000000000000000000000000010").unwrap();assert_eq!(chicken["origin_extras"],json!([CHICKEN_EXTRA_ID]));assert_eq!(chicken["origin_batches"],json!([]));
        let sessions=d["prep_sessions"].as_array().unwrap();assert_eq!(sessions.len(),2);
        let base=sessions.iter().find(|row|row["start"]["local_time"]=="11:00:00").unwrap();let extra=sessions.iter().find(|row|row["start"]["local_time"]=="12:25:00").unwrap();
        assert_eq!(base["tasks"].as_array().unwrap().len(),2);assert_eq!(extra["tasks"].as_array().unwrap().len(),1);
        for session in [base,extra] {assert_eq!(session["end"]["local_time"],"12:30:00");assert_eq!(session["ingredient_needed_at"],session["start"]);}
        let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();let view=RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();let (basis,_)=mstrchf_browser::calendar_view::material_at(&view,&typed.snapshot,typed.snapshot.profile.0.as_ref().unwrap(),&typed.snapshot.weeks[0],&Instant(NOW.into())).unwrap().unwrap();
        let occurrences=basis.schedule.layout.routes.iter().filter(|row|row.phase.occurrence.recipe.recipe_id.0==CHILLED_CHICKEN_EXTRA_SCOPE.recipe_id).collect::<Vec<_>>();assert_eq!(occurrences.len(),1);assert_eq!(occurrences[0].phase.occurrence.extra_id,Some(ExtraId(CHICKEN_EXTRA_ID.into())));assert_eq!(occurrences[0].phase.occurrence.recipe,chilled_chicken_extra_packet_ref().unwrap().recipe);
        let resources=basis.evidence.resources.iter().filter(|row|row.recipe.recipe_id.0==CHILLED_CHICKEN_EXTRA_SCOPE.recipe_id).collect::<Vec<_>>();assert_eq!(resources.len(),1);assert_eq!(resources[0].prep_id,1);assert!(resources[0].equipment.is_empty());assert_eq!(resources[0].evidence_ids,vec![EvidenceId("ev_80000000000000000000000000000042".into())]);
        assert!(basis.evidence.resources.iter().filter(|row|row.recipe.recipe_id.0==THREE_BEAN_TWO_UNIT_CHICKEN_EXTRA_SCOPE.recipe_id).all(|row|row.equipment==vec![EquipmentCode::Fridge]));
        assert_eq!(state["snapshot"]["privacy"]["use_ai_features"],false);assert_eq!(state["snapshot"]["privacy"]["include_household_requirements"],false);assert_eq!(cold(state.clone()),state);
    }}
}

#[test]
fn two_genuinely_none_recipients_are_refused_without_a_dietary_confounder() {
    let state=ready(false,&Value::Null,&Value::Null);let people=json!([person(&state,false,false),person(&state,false,true)]);
    assert!(!offered(&board(&state,json!({"extra_recipients":people})),&three_bean_two_unit_chicken_extra_packet_ref().unwrap().recipe));
    let saved=select(&state,json!({"extra_recipients":[person(&state,false,false)]}),0xf811);let mut w=saved["snapshot"]["weeks"][0].clone();let mut second=w["plan"]["selections"][0]["extras"][0]["recipients"][0].clone();second["person"]=person(&saved,false,true);w["plan"]["selections"][0]["extras"][0]["recipients"].as_array_mut().unwrap().push(second);
    let refusal=refuse_plan(&saved,w,0xf812);assert!(refusal["error"]["issues"].to_string().contains("/personal_use_scope/one_declared_chicken_extra"));assert_eq!(cold(saved.clone()),saved);
}

#[test]
fn otherwise_valid_admitted_extra_cannot_be_a_standalone_meal_or_another_base_extra() {
    let state=admit(&setup(),chilled_chicken_extra_packet_ref().unwrap(),0xf820);let reference=chilled_chicken_extra_packet_ref().unwrap().recipe;
    let b=board(&state,Value::Null);assert!(!offered(&b,&reference));let mut w=state["snapshot"]["weeks"][0].clone();
    // One real current None Adult Regular person, a valid full-ref preset and
    // fresh unit batch: refusal must be placement, not count or missing admission.
    w["plan"]["selections"]=json!([{"slot":w["cells"][4]["slot"],"recipe":reference,"origin":"fixed","repeat_authorised":false,"servings":[{"person":person(&state,false,false),"preset_ref":{"recipe":reference,"preset":"regular","age_band":"adult","meal_type":"lunch"},"servings":{"numerator":"1","denominator":"1"}}],"extras":[],"batch_use":{"kind":"new_batch","data":{"batch_id":"ba_80000000000000000000000000000009","production_servings":{"numerator":"1","denominator":"1"}}},"meal_overrides":{"portions":[],"eat_by":null}}]);
    let refusal=refuse_plan(&state,w,0xf821);assert!(refusal["error"]["issues"].to_string().contains("/personal_use_scope/extra_only_never_standalone"));
    let state=saved(true,&diet("vegetarian"),&Value::Null,true);let mut w=state["snapshot"]["weeks"][0].clone();let old=three_bean_two_unit_vegetarian_packet_ref().unwrap().recipe;w["plan"]["selections"][0]["recipe"]=json!(old);for serving in w["plan"]["selections"][0]["servings"].as_array_mut().unwrap(){serving["preset_ref"]["recipe"]=json!(old);}
    let refusal=refuse_plan(&state,w,0xf822);assert!(refusal["error"]["issues"].to_string().contains("/personal_use_scope/extra_placement"));assert_eq!(cold(state.clone()),state);
}

#[test]
fn explicit_current_none_identity_and_all_layer_unit_guards_are_not_replaced_by_compensation() {
    let state=saved(true,&diet("vegetarian"),&Value::Null,true);let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();let view=RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();let (basis,_)=mstrchf_browser::calendar_view::material_at(&view,&typed.snapshot,typed.snapshot.profile.0.as_ref().unwrap(),&typed.snapshot.weeks[0],&Instant(NOW.into())).unwrap().unwrap();
    let recipe=known_packet(&three_bean_two_unit_chicken_extra_packet_ref().unwrap()).unwrap().recipe();let selected=&basis.plan.selections[0];let meal=basis.context.requirements.iter().find(|row|row.slot==selected.slot).unwrap();
    let check=|m:&MealRequirements,s:&MealSelection,o:&BoardOptions|check_personal_use_scope(recipe,&selected.recipe,m,s,&basis.plan,o);
    assert!(check(meal,selected,&Default::default()).is_ok());
    let mut m=meal.clone();m.attendees[1].portion.adjustment=PositiveRational::new(2,1).unwrap();let mut selection=selected.clone();selection.meal_overrides.portions=BoundedVec(vec![PersonPortionOverride{person:m.attendees[1].person.clone(),preset:PortionPresetName::Regular,adjustment:PositiveRational::new(1,2).unwrap()}]);assert!(check(&m,&selection,&Default::default()).is_err());
    let mut m=meal.clone();m.attendees[1].portion.meal_overrides=BoundedVec(vec![MealPortionOverride{meal_type:MealType::Lunch,preset:PortionPresetName::Regular,adjustment:PositiveRational::new(2,1).unwrap()}]);assert!(check(&m,selected,&Default::default()).is_err());
    let mut selection=selected.clone();selection.extras[0].recipients[0].servings=PositiveRational::new(2,1).unwrap();assert!(check(meal,&selection,&Default::default()).is_err());
    assert!(check(meal,selected,&BoardOptions{extra_recipients:Some(BoundedVec(vec![meal.attendees[0].person.clone()])),..Default::default()}).is_err());
    for changed in ["vegan","gluten_free"] {let ready=ready(true,&Value::Null,&diet(changed));assert!(!offered(&board(&ready,json!({"extra_recipients":[person(&ready,true,true)]})),&three_bean_two_unit_chicken_extra_packet_ref().unwrap().recipe));}
    let ready=ready(true,&diet("vegetarian"),&Value::Null);assert!(!offered(&board(&ready,json!({"extra_recipients":[person(&ready,true,false)]})),&three_bean_two_unit_chicken_extra_packet_ref().unwrap().recipe));
    for field in ["servings","person"] {let mut w=state["snapshot"]["weeks"][0].clone();if field=="servings"{w["plan"]["selections"][0]["extras"][0]["recipients"][0]["servings"]=json!({"numerator":"2","denominator":"1"});}else{w["plan"]["selections"][0]["extras"][0]["recipients"][0]["person"]["data"]["guest_id"]=json!("gu_80000000000000000000000000000009");}refuse_plan(&state,w,0xf823+u32::from(field=="person"));}
    assert_eq!(cold(state.clone()),state);
}

#[test]
fn no_make_extra_reuse_unknown_recipient_travel_or_missing_fridge_is_offered() {
    let state=ready(true,&diet("vegetarian"),&Value::Null);let recipient=person(&state,true,true);let rf=three_bean_two_unit_chicken_extra_packet_ref().unwrap().recipe;
    for options in [json!({"extra_recipients":[recipient],"make_extra":true}),json!({"extra_recipients":[recipient],"leftover_source":state["snapshot"]["weeks"][0]["cells"][3]["slot"]})] {assert!(!offered(&board(&state,options),&rf));}
    let w=&state["snapshot"]["weeks"][0];let foreign=call(json!({"op":"board","state":state,"week_id":w["range"]["week_id"],"slot":w["cells"][4]["slot"],"options":{"extra_recipients":[{"kind":"guest","data":{"occasion_id":"oc_80000000000000000000000000000008","guest_id":"gu_80000000000000000000000000000009"}}]},"now":NOW}));assert!(foreign["ok"]==false||!offered(&foreign["value"],&rf),"{foreign}");
    let mut p=state["snapshot"]["profile"].clone();p["preferences"]["equipment"]=json!(["hob"]);let absent=profile(&state,p,0xf825);assert!(!offered(&board(&absent,json!({"extra_recipients":[recipient]})),&rf));
    let mut w=state["snapshot"]["weeks"][0].clone();w["cells"][4]["choice"]=json!("on_the_move");let moved=edit(&state,w,0xf826,false);assert!(!offered(&board(&moved,json!({"extra_recipients":[recipient]})),&rf));
    assert!(!offered(&board(&state,json!({"extra_recipients":[recipient],"eat_by_time":"00:30:00","time_resolution":"unambiguous"})),&rf));assert_eq!(cold(state.clone()),state);
}

#[test]
fn changed_named_recipient_stays_allocated_and_conflicted_until_explicit_extra_removal() {
    let state=saved(true,&diet("vegetarian"),&Value::Null,true);let original=state["snapshot"]["weeks"][0]["plan"]["selections"].clone();let mut w=state["snapshot"]["weeks"][0].clone();w["cells"][4]["occasion"]["guests"][0]["restrictions"]=declaration(&diet("vegetarian"));let changed=edit(&state,w,0xf830,false);
    assert_eq!(changed["snapshot"]["weeks"][0]["plan"]["selections"],original);assert_eq!(changed["history"],state["history"]);assert!(!changed["snapshot"]["conflicts"].as_array().unwrap().is_empty());let b=board(&changed,Value::Null);assert!(b["details"]["derived"].is_null());assert!(!b["details"]["food_error"].is_null());
    assert!(!offered(&board(&changed,json!({"extra_recipients":[person(&changed,true,true)]})),&three_bean_two_unit_chicken_extra_packet_ref().unwrap().recipe));
    let removed=select(&changed,Value::Null,0xf831);assert_eq!(removed["snapshot"]["weeks"][0]["plan"]["selections"][0]["extras"],json!([]));assert!(removed["snapshot"]["conflicts"].as_array().unwrap().is_empty());assert_eq!(board(&removed,Value::Null)["details"]["derived"]["demand"]["lines"].as_array().unwrap().len(),9);
    assert_eq!(&removed["receipts"].as_array().unwrap()[..state["receipts"].as_array().unwrap().len()],state["receipts"].as_array().unwrap());assert_eq!(cold(changed.clone()),changed);assert_eq!(cold(removed.clone()),removed);
}

#[test]
fn departed_named_guest_retains_booked_order_history_exact_food_and_replay_but_no_current_calendar() {
    let state=book(&saved(true,&diet("vegetarian"),&Value::Null,true));let order=state["snapshot"]["weeks"][0]["confirmed_order"].clone();let mut w=state["snapshot"]["weeks"][0].clone();w["cells"][4]["occasion"]["guests"]=json!([]);let body=week_body(w,false);let cmd=command(&state,body.clone(),0xf832);let removed=apply(&state,body,0xf832);
    let mut expected_plan=state["snapshot"]["weeks"][0]["plan"].clone();expected_plan["revision"]=removed["snapshot"]["weeks"][0]["plan"]["revision"].clone();assert_eq!(removed["snapshot"]["weeks"][0]["plan"],expected_plan);
    assert_eq!(removed["snapshot"]["weeks"][0]["plan"]["selections"],state["snapshot"]["weeks"][0]["plan"]["selections"]);assert_eq!(removed["history"],state["history"]);assert_eq!(removed["snapshot"]["weeks"][0]["confirmed_order"],order);assert_eq!(removed["snapshot"]["weeks"][0]["publication"],"needs_user");assert!(!removed["snapshot"]["conflicts"].as_array().unwrap().is_empty());
    let b=board(&removed,Value::Null);assert!(b["details"]["derived"].is_null());assert!(!offered(&b,&three_bean_packet_ref().unwrap().recipe));assert!(!offered(&b,&three_bean_two_unit_chicken_extra_packet_ref().unwrap().recipe));
    let refusal=call(json!({"op":"calendar_preview","state":removed,"order_id":order["order_id"],"destination_id":DEST,"now":NOW}));assert_eq!(refusal["ok"],false);
    let replay=good(json!({"op":"dispatch","state":removed,"command":cmd,"now":NOW}));assert_eq!(replay["state"],removed);assert_eq!(replay["receipt"]["disposition"],"replayed");assert_eq!(cold(removed.clone()),removed);
    let mut w=removed["snapshot"]["weeks"][0].clone();w["plan"]["selections"]=json!([]);w["plan"]["batch_links"]=json!([]);let cleared=edit(&removed,w,0xf833,true);assert!(cleared["snapshot"]["conflicts"].as_array().unwrap().is_empty());assert_eq!(cleared["snapshot"]["weeks"][0]["confirmed_order"],order);assert_eq!(cleared["history"]["ordered"],removed["history"]["ordered"]);assert_eq!(cold(cleared.clone()),cleared);
}

#[test]
fn deselected_named_resident_is_not_redirected_and_changed_plan_or_resigned_retention_cannot_launder_authority() {
    let state=saved(false,&diet("vegetarian"),&Value::Null,true);let mut w=state["snapshot"]["weeks"][0].clone();w["cells"][4]["attendee_ids"].as_array_mut().unwrap().pop();let removed=edit(&state,w.clone(),0xf834,false);
    assert_eq!(removed["snapshot"]["weeks"][0]["plan"]["selections"],state["snapshot"]["weeks"][0]["plan"]["selections"]);assert_eq!(removed["history"],state["history"]);assert!(!removed["snapshot"]["conflicts"].as_array().unwrap().is_empty());assert!(!offered(&board(&removed,Value::Null),&three_bean_packet_ref().unwrap().recipe));assert_eq!(cold(removed.clone()),removed);
    let mut supplied=w.clone();supplied["plan"]["selections"][0]["extras"][0]["recipients"][0]["person"]=person(&state,false,false);refuse_plan(&state,supplied,0xf835);
    let mut forged=removed.clone();let receipt=forged["receipts"].as_array_mut().unwrap().last_mut().unwrap();receipt["command"]["command"]["data"]["week"]["cells"][4]["choice"]=json!("on_the_move");receipt["command_digest"]=json!(canonical_digest(&receipt["command"]).unwrap());let refusal=call(json!({"op":"validate_state","state":forged,"now":NOW}));assert_eq!(refusal["ok"],false);assert!(refusal["error"]["issues"].to_string().contains("/receipts/retained_two_unit/response"));
    let mut w=removed["snapshot"]["weeks"][0].clone();w["plan"]["selections"]=json!([]);let cleared=edit(&removed,w,0xf836,true);assert!(cleared["snapshot"]["conflicts"].as_array().unwrap().is_empty());assert_eq!(cold(cleared.clone()),cleared);
}

#[test]
fn current_extra_calendar_support_keeps_legacy_refusal_and_booking_custody() {
    let state=book(&saved(true,&diet("vegetarian"),&Value::Null,true));let order=&state["snapshot"]["weeks"][0]["confirmed_order"];
    let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();let order_id:OrderId=serde_json::from_value(order["order_id"].clone()).unwrap();
    for version in [1,2] {let refusal=mstrchf_browser::calendar_owner::prepare_for_renderer(&typed,&order_id,&CalendarDestinationId(DEST.into()),&Instant(NOW.into()),version).unwrap_err();assert!(refusal.issues.iter().any(|issue|issue.path=="/calendar/extras"));}
    let preview=good(json!({"op":"calendar_preview","state":state,"order_id":order["order_id"],"destination_id":DEST,"now":NOW}));assert_eq!(preview["batch"]["events"].as_array().unwrap().len(),3);assert_eq!(preview["prepared"],false);assert!(!order.is_null());assert_eq!(cold(state.clone()),state);
}

#[test]
fn empty_continuous_equipment_does_not_remove_real_concurrent_attention_guard() {
    let state=saved(true,&Value::Null,&diet("vegetarian"),false);let state=admit(&state,porridge_packet_ref().unwrap(),0xf850);let b=board_slot(&state,3,json!({"eat_by_time":"12:30:00","time_resolution":"unambiguous"}));assert!(!offered(&b,&porridge_packet_ref().unwrap().recipe),"Hob preparation still overlaps the named extra's five minutes of full attention");
    assert!(offered(&board_slot(&state,3,Value::Null),&porridge_packet_ref().unwrap().recipe));assert_eq!(cold(state.clone()),state);
}

#[test]
fn target_recovery_projection_does_not_hide_a_second_departed_person_selection() {
    let state=saved(true,&diet("vegetarian"),&Value::Null,true);let mut w=state["snapshot"]["weeks"][0].clone();let mut occasion=w["cells"][4]["occasion"].clone();occasion["occasion_id"]=json!("oc_80000000000000000000000000000009");occasion["guests"][0]["guest_id"]=json!("gu_80000000000000000000000000000009");w["cells"][7]["occasion"]=occasion;let state=edit(&state,w,0xf851,false);
    let occasion=&state["snapshot"]["weeks"][0]["cells"][7]["occasion"];let b=board_slot(&state,7,json!({"extra_recipients":[{"kind":"guest","data":{"occasion_id":occasion["occasion_id"],"guest_id":occasion["guests"][0]["guest_id"]}}]}));let c=chosen(&b,&three_bean_two_unit_chicken_extra_packet_ref().unwrap().recipe);let mut w=state["snapshot"]["weeks"][0].clone();w["plan"]=c["plan"].clone();let state=edit(&state,w,0xf852,true);
    let mut w=state["snapshot"]["weeks"][0].clone();w["cells"][4]["occasion"]["guests"]=json!([]);w["cells"][7]["occasion"]["guests"]=json!([]);let removed=edit(&state,w,0xf853,false);let w=&removed["snapshot"]["weeks"][0];let refusal=call(json!({"op":"board","state":removed,"week_id":w["range"]["week_id"],"slot":w["cells"][4]["slot"],"now":NOW}));assert_eq!(refusal["ok"],false);assert_eq!(refusal["error"]["code"],"foreign_reference");assert_eq!(cold(removed.clone()),removed);
}
