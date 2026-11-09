//! Native owner integration with synthetic households/orders only.
//! A fixed Smaller recipe is an authored unit, not human portion calibration.
use mstrchf_browser::{run_json, BrowserState, BoardOptions, catalogue::RuntimeCatalogue,
    maker_packets::*, personal_scope::*};
use mstrchf_contracts::*;
use serde_json::{json, Value};

const NOW:&str="2026-10-13T12:00:00Z";
const DEST:&str="cd_10000000000000000000000000000001";
fn call(input:Value)->Value {serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap()}
fn good(input:Value)->Value {let r=call(input);assert_eq!(r["ok"],true,"{r}");r["value"].clone()}
fn command(s:&Value,body:Value,id:u32)->Value {json!({"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":s["snapshot"]["snapshot_revision"],"command":body})}
fn apply(s:&Value,body:Value,id:u32)->Value {good(json!({"op":"dispatch","state":s,"command":command(s,body,id),"now":NOW}))["state"].clone()}
fn week_body(w:&Value)->Value {json!({"kind":"save_week","data":{"expected_week_revision":w["revision"],"week":{"week_id":w["range"]["week_id"],"defaults_snapshot":w["defaults_snapshot"],"day_enabled":w["day_enabled"],"meal_type_enabled":w["meal_type_enabled"],"cells":w["cells"],"plan":w["plan"]}}})}
fn profile(s:&Value,p:Value,id:u32)->Value {apply(s,json!({"kind":"update_profile","data":{"expected_profile_revision":p["revision"],"profile":p}}),id)}
fn setup()->Value {
    let mut s=serde_json::from_slice::<Value>(include_bytes!("fixtures/legacy-engineering-receipts.json")).unwrap()["state"].clone();
    let mut p=s["snapshot"]["profile"].clone();p["diners"][0]["age_band"]=json!("adult");
    p["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":NOW}});
    p["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});p["preferences"]["equipment"]=json!(["fridge","hob"]);
    s=profile(&s,p,0xfa00);let mut w=s["snapshot"]["weeks"][0].clone();
    w["plan"]["selections"]=json!([]);w["plan"]["batch_links"]=json!([]);w["meal_type_enabled"]["breakfast"]=json!(true);w["meal_type_enabled"]["lunch"]=json!(true);
    apply(&s,week_body(&w),0xfa01)
}
fn admit(s:&Value,packet:ReviewedRecipePacketRef,id:u32)->Value {apply(s,json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":(1+s["reviewed_packets"].as_array().map_or(0,Vec::len)).to_string()}}),id)}
fn ready()->Value {admit(&admit(&setup(),larger_porridge_packet_ref().unwrap(),0xfa02),three_bean_smaller_packet_ref().unwrap(),0xfa03)}
fn opts(s:&Value,preset:&str)->Value {json!({"overrides":{"portions":[{"person":{"kind":"diner","data":{"diner_id":s["snapshot"]["profile"]["diners"][0]["diner_id"]}},"preset":preset,"adjustment":{"numerator":"1","denominator":"1"}}],"eat_by":null}})}
fn board(s:&Value,index:usize,options:Value)->Value {let w=&s["snapshot"]["weeks"][0];good(json!({"op":"board","state":s,"week_id":w["range"]["week_id"],"slot":w["cells"][index]["slot"],"options":options,"now":NOW}))}
fn offered(b:&Value,rf:&RecipeRef)->bool {b["choices"].as_array().unwrap().iter().any(|c|c["selection"]["recipe"]==json!(rf))}
fn select(s:&Value,index:usize,rf:&RecipeRef,options:Value,id:u32)->Value {let b=board(s,index,options);let choice=b["choices"].as_array().unwrap().iter().find(|c|c["selection"]["recipe"]==json!(rf)).expect("exact eligible portion choice");let mut w=s["snapshot"]["weeks"][0].clone();w["plan"]=choice["plan"].clone();apply(s,week_body(&w),id)}
fn saved()->Value {let s=ready();let s=select(&s,3,&larger_porridge_packet_ref().unwrap().recipe,opts(&s,"larger"),0xfa04);select(&s,4,&three_bean_smaller_packet_ref().unwrap().recipe,opts(&s,"smaller"),0xfa05)}
fn cold(s:Value)->Value {std::thread::spawn(move||good(json!({"op":"validate_state","state":s,"now":NOW}))).join().unwrap()}
fn qty(n:&str,d:&str,unit:&str,form:&str)->Value {json!({"amount":{"numerator":n,"denominator":d},"unit":unit,"form":form})}

#[test]
fn separate_breakfast_larger_and_lunch_smaller_preserve_usual_profile_other_weeks_and_receipts() {
    let original=ready();let small=three_bean_smaller_packet_ref().unwrap().recipe;
    assert!(!offered(&board(&original,4,Value::Null),&small));assert!(offered(&board(&original,4,opts(&original,"smaller")),&small));
    assert!(!offered(&board(&original,4,opts(&original,"larger")),&small));
    let breakfast=select(&original,3,&larger_porridge_packet_ref().unwrap().recipe,opts(&original,"larger"),0xfa04);
    let s=select(&breakfast,4,&small,opts(&breakfast,"smaller"),0xfa05);
    assert_eq!(s["snapshot"]["profile"],original["snapshot"]["profile"]);assert_eq!(s["snapshot"]["privacy"],original["snapshot"]["privacy"]);
    assert_eq!(s["snapshot"]["weeks"][0]["cells"],original["snapshot"]["weeks"][0]["cells"]);
    assert_eq!(&s["snapshot"]["weeks"].as_array().unwrap()[1..],&original["snapshot"]["weeks"].as_array().unwrap()[1..]);
    assert_eq!(&s["receipts"].as_array().unwrap()[..breakfast["receipts"].as_array().unwrap().len()],breakfast["receipts"].as_array().unwrap());
    let meals=s["snapshot"]["weeks"][0]["plan"]["selections"].as_array().unwrap();assert_eq!(meals.len(),2);
    assert_eq!(meals[0],breakfast["snapshot"]["weeks"][0]["plan"]["selections"][0]);
    assert_eq!(meals[0]["servings"][0]["person"],meals[1]["servings"][0]["person"]);
    assert_eq!(meals[1]["servings"][0]["preset_ref"]["preset"],"smaller");assert_eq!(meals[1]["servings"][0]["servings"],json!({"numerator":"1","denominator":"1"}));
    assert_eq!(cold(s.clone()),s);
}

#[test]
fn nine_smaller_demands_two_cold_phases_and_original_handling_clock_remain_exact() {
    let s=saved();let b=board(&s,4,Value::Null);let d=&b["details"]["derived"];
    assert_eq!(d["allocations"].as_array().unwrap().len(),2);assert_eq!(d["demand"]["lines"].as_array().unwrap().len(),12);
    let recipe=known_packet(&three_bean_smaller_packet_ref().unwrap()).unwrap().recipe();
    let expected=[("60","1","g","drained"),("60","1","g","drained"),("60","1","g","drained"),("30","1","g","raw"),("15","1","g","raw"),("45","1","g","raw"),("15","2","ml","prepared"),("45","4","ml","prepared"),("15","4","g","prepared")];
    for (input,(n,den,unit,form)) in recipe.ingredients.iter().zip(expected) {let line=d["demand"]["lines"].as_array().unwrap().iter().find(|l|l["ingredient_id"]==json!(input.ingredient_id)).unwrap();assert_eq!(line["quantity"],qty(n,den,unit,form));}
    let sessions=d["prep_sessions"].as_array().unwrap();assert_eq!(sessions.len(),2);let cold=sessions.iter().find(|session|session["tasks"].as_array().unwrap().len()==2).unwrap();
    assert_eq!(cold["start"]["local_time"],"11:15:00");assert_eq!(cold["end"]["local_time"],"12:30:00");assert_eq!(cold["ingredient_needed_at"],cold["start"]);
    let batch=d["batches"].as_array().unwrap().iter().find(|v|v["recipe"]==json!(three_bean_smaller_packet_ref().unwrap().recipe)).unwrap();
    assert_eq!(batch["components"][0]["original_production"]["kind"],"unheated");assert_eq!(batch["components"][0]["original_production"]["data"]["preparation_started_at"],cold["start"]);
    let t:BrowserState=serde_json::from_value(s).unwrap();let view=RuntimeCatalogue::from_records(&t.reviewed_packets.0).unwrap();let (basis,_)=mstrchf_browser::calendar_view::material_at(&view,&t.snapshot,t.snapshot.profile.0.as_ref().unwrap(),&t.snapshot.weeks[0],&Instant(NOW.into())).unwrap().unwrap();
    let method=basis.evidence.methods.iter().find(|m|m.recipe==three_bean_smaller_packet_ref().unwrap().recipe).unwrap();assert_eq!(method.prep_ids,vec![1,2]);assert_eq!(method.evidence_ids,vec![EvidenceId("ev_80000000000000000000000000000052".into())]);
}

#[test]
fn saved_lunch_preference_and_same_meal_override_follow_precedence_without_compensation() {
    let s=ready();let mut p=s["snapshot"]["profile"].clone();p["diners"][0]["portions"]["meal_overrides"]=json!([{"meal_type":"breakfast","preset":"larger","adjustment":{"numerator":"1","denominator":"1"}},{"meal_type":"lunch","preset":"smaller","adjustment":{"numerator":"1","denominator":"1"}}]);
    let s=profile(&s,p,0xfa10);let rf=three_bean_smaller_packet_ref().unwrap().recipe;assert!(offered(&board(&s,4,Value::Null),&rf));assert!(!offered(&board(&s,4,opts(&s,"regular")),&rf));
    let saved=select(&s,4,&rf,Value::Null,0xfa11);assert_eq!(saved["snapshot"]["profile"],s["snapshot"]["profile"]);assert_eq!(cold(saved.clone()),saved);
    let mut p=s["snapshot"]["profile"].clone();p["diners"][0]["portions"]["adjustment"]=json!({"numerator":"4","denominator":"3"});let changed=profile(&s,p,0xfa12);assert!(!offered(&board(&changed,4,opts(&changed,"smaller")),&rf));
}

#[test]
fn nonunit_layers_guests_children_diets_travel_reuse_and_prior_day_handling_refuse() {
    let t:BrowserState=serde_json::from_value(saved()).unwrap();let view=RuntimeCatalogue::from_records(&t.reviewed_packets.0).unwrap();let(basis,_)=mstrchf_browser::calendar_view::material_at(&view,&t.snapshot,t.snapshot.profile.0.as_ref().unwrap(),&t.snapshot.weeks[0],&Instant(NOW.into())).unwrap().unwrap();
    let selected=basis.plan.selections.iter().find(|s|s.recipe==three_bean_smaller_packet_ref().unwrap().recipe).unwrap().clone();let meal=basis.context.requirements.iter().find(|m|m.slot==selected.slot).unwrap().clone();let recipe=known_packet(&three_bean_smaller_packet_ref().unwrap()).unwrap().recipe();
    let check=|m:&MealRequirements,s:&MealSelection,o:&BoardOptions|check_personal_use_scope(recipe,&selected.recipe,m,s,&basis.plan,o);
    assert!(check(&meal,&selected,&Default::default()).is_ok());
    for layer in 0..4 {let mut m=meal.clone();let mut s=selected.clone();let mut o=BoardOptions::default();let nonunit=PositiveRational::new(4,3).unwrap();match layer {0=>m.attendees[0].portion.adjustment=nonunit,1=>m.attendees[0].portion.meal_overrides=BoundedVec(vec![MealPortionOverride{meal_type:MealType::Lunch,preset:PortionPresetName::Regular,adjustment:nonunit}]),2=>s.meal_overrides.portions[0].adjustment=nonunit,_=>o.overrides=Some(MealOverrides{portions:BoundedVec(vec![PersonPortionOverride{person:m.attendees[0].person.clone(),preset:PortionPresetName::Smaller,adjustment:nonunit}]),eat_by:Nullable(None)})};assert!(check(&m,&s,&o).is_err());}
    let mut m=meal.clone();m.attendees[0].person=PersonRef::Guest{occasion_id:OccasionId("oc_8000000000000000000000000000000a".into()),guest_id:GuestId("gu_8000000000000000000000000000000a".into())};assert!(check(&m,&selected,&Default::default()).is_err());
    let mut m=meal.clone();m.attendees[0].age_band=AgeBand::Child4To6;assert!(check(&m,&selected,&Default::default()).is_err());
    let mut m=meal.clone();m.attendees[0].hard_requirements=BoundedVec(vec![HardRequirement::Diet{code:DietCode::Vegetarian}]);assert!(check(&m,&selected,&Default::default()).is_err());
    let mut m=meal.clone();m.location=MealLocation::OnTheMove;assert!(check(&m,&selected,&Default::default()).is_err());
    let mut m=meal.clone();m.equipment=BoundedVec(vec![EquipmentCode::Hob]);assert!(check(&m,&selected,&Default::default()).is_err());
    for o in [BoardOptions{make_extra:Some(true),..Default::default()},BoardOptions{leftover_source:Some(selected.slot.clone()),..Default::default()}] {assert!(check(&meal,&selected,&o).is_err());}
    let s=ready();let mut o=opts(&s,"smaller");o["eat_by_time"]=json!("01:14:59");o["time_resolution"]=json!("unambiguous");assert!(!offered(&board(&s,4,o),&three_bean_smaller_packet_ref().unwrap().recipe));
}

#[test]
fn admission_preserves_original_regular_food_receipt_and_rejects_resigned_recipe_change() {
    let s=admit(&setup(),three_bean_packet_ref().unwrap(),0xfa20);let s=select(&s,4,&three_bean_packet_ref().unwrap().recipe,Value::Null,0xfa21);let receipts=s["receipts"].as_array().unwrap();
    let admitted=admit(&s,three_bean_smaller_packet_ref().unwrap(),0xfa22);assert_eq!(admitted["snapshot"]["weeks"],s["snapshot"]["weeks"]);assert_eq!(&admitted["receipts"].as_array().unwrap()[..receipts.len()],receipts.as_slice());
    let replay=good(json!({"op":"dispatch","state":admitted,"command":receipts.last().unwrap()["command"],"now":NOW}));assert_eq!(replay["state"],admitted);assert_eq!(replay["receipt"]["disposition"],"replayed");
    let replaced=select(&admitted,4,&three_bean_smaller_packet_ref().unwrap().recipe,opts(&admitted,"smaller"),0xfa23);assert_eq!(cold(replaced.clone()),replaced);
    let mut fake=known_packet(&three_bean_smaller_packet_ref().unwrap()).unwrap().record().clone();fake.packet["maker_recipe_revision"]["ingredients"][0]["quantity"]["amount"]["numerator"]=json!("61");assert!(known_record(&fake).is_err());
}

#[test]
fn current_incompatible_profile_preserves_food_as_conflict_and_cold_reloads() {
    let s=saved();let mut p=s["snapshot"]["profile"].clone();p["diners"][0]["restrictions"]=json!({"kind":"confirmed","data":{"confirmed_at":NOW,"requirements":[{"kind":"diet","data":{"code":"vegetarian"}}]}});let changed=profile(&s,p,0xfa30);
    assert_eq!(changed["snapshot"]["weeks"][0]["plan"]["selections"],s["snapshot"]["weeks"][0]["plan"]["selections"]);assert!(!changed["snapshot"]["conflicts"].as_array().unwrap().is_empty());assert!(board(&changed,4,Value::Null)["details"]["derived"].is_null());assert_eq!(cold(changed.clone()),changed);
}

#[test]
fn calendar_renders_smaller_fractions_both_phases_and_keeps_booked_after_late_delivery_refusal() {
    let s=admit(&setup(),three_bean_smaller_packet_ref().unwrap(),0xfa40);let s=select(&s,4,&three_bean_smaller_packet_ref().unwrap().recipe,opts(&s,"smaller"),0xfa41);let b=board(&s,4,Value::Null);let w=&s["snapshot"]["weeks"][0];
    let approved=apply(&s,json!({"kind":"approve_plan","data":{"week_id":w["range"]["week_id"],"plan_revision":w["plan"]["revision"],"derived_digest":b["details"]["derived"]["digest"]}}),0xfa42);
    for (late,id) in [(false,0xfa43),(true,0xfa44)] {let w=&approved["snapshot"]["weeks"][0];let moment=|time:&str|good(json!({"op":"resolve_moment","date":w["cells"][4]["slot"]["date"],"time":time,"zone":w["range"]["zone"],"resolution":"unambiguous"}));let booked=apply(&approved,json!({"kind":"confirm_order","data":{"confirmation":{"week_id":w["range"]["week_id"],"approval_id":w["approved_plan"]["approval_id"],"delivery_start":moment("10:00:00"),"delivery_end":moment(if late {"11:15:01"} else {"11:14:59"}),"confirmed_at":NOW,"source":{"kind":"user_confirmation","data":{}}}}}),id);
        let preview=call(json!({"op":"calendar_preview","state":booked,"order_id":booked["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],"destination_id":DEST,"now":NOW}));assert_eq!(preview["ok"],!late);
        if !late {let bodies=preview["value"]["batch"]["events"].as_array().unwrap().iter().map(|e|e["body"].as_str().unwrap()).collect::<Vec<_>>().join("\n");for expected in ["15/2 ml","45/4 ml","15/4 g","11:15:00","Requires phase 1","joel-maxuel"] {assert!(bodies.contains(expected),"missing {expected}: {bodies}");}}
        assert!(!booked["snapshot"]["weeks"][0]["confirmed_order"].is_null());assert_eq!(cold(booked.clone()),booked);
    }
}
