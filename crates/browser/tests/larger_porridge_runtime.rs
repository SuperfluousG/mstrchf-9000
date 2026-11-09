//! Exact native owner commands with synthetic households/orders only.
//! The fixed Larger unit supplies no cook, calibration, product or account proof.
use mstrchf_browser::{run_json, BrowserState, BoardOptions, catalogue::RuntimeCatalogue,
    maker_packets::*, personal_scope::*};
use mstrchf_contracts::*;
use serde_json::{json, Value};

const NOW:&str="2026-10-13T04:00:00Z";
const OLD:&str="2026-10-12T18:00:00Z";
const BEFORE:&str="2026-10-13T02:40:30Z";
const DEST:&str="cd_10000000000000000000000000000001";
fn call(input:Value)->Value {serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap()}
fn good(input:Value)->Value {let r=call(input);assert_eq!(r["ok"],true,"{r}");r["value"].clone()}
fn command(state:&Value,body:Value,id:u32)->Value {json!({"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body})}
fn apply_at(state:&Value,body:Value,id:u32,at:&str)->Value {good(json!({"op":"dispatch","state":state,"command":command(state,body,id),"now":at}))["state"].clone()}
fn apply(state:&Value,body:Value,id:u32)->Value {apply_at(state,body,id,NOW)}
fn week_body(w:&Value)->Value {json!({"kind":"save_week","data":{"expected_week_revision":w["revision"],"week":{"week_id":w["range"]["week_id"],"defaults_snapshot":w["defaults_snapshot"],"day_enabled":w["day_enabled"],"meal_type_enabled":w["meal_type_enabled"],"cells":w["cells"],"plan":w["plan"]}}})}
fn setup_at(at:&str)->Value {
    let mut s=serde_json::from_slice::<Value>(include_bytes!("fixtures/legacy-engineering-receipts.json")).unwrap()["state"].clone();
    let mut p=s["snapshot"]["profile"].clone();p["diners"][0]["age_band"]=json!("adult");
    p["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":at}});
    p["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});p["preferences"]["equipment"]=json!(["fridge","hob"]);
    s=apply_at(&s,json!({"kind":"update_profile","data":{"expected_profile_revision":p["revision"],"profile":p}}),0xf700,at);
    let mut w=s["snapshot"]["weeks"][0].clone();w["plan"]["selections"]=json!([]);w["plan"]["batch_links"]=json!([]);w["meal_type_enabled"]["breakfast"]=json!(true);
    apply_at(&s,week_body(&w),0xf701,at)
}
fn setup()->Value {setup_at(NOW)}
fn admit_at(state:&Value,packet:ReviewedRecipePacketRef,id:u32,at:&str)->Value {
    let revision=1+state["reviewed_packets"].as_array().map_or(0,Vec::len);
    apply_at(state,json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":revision.to_string()}}),id,at)
}
fn ready()->Value {admit_at(&setup(),larger_porridge_packet_ref().unwrap(),0xf702,NOW)}
fn board(state:&Value,options:Value)->Value {let w=&state["snapshot"]["weeks"][0];good(json!({"op":"board","state":state,"week_id":w["range"]["week_id"],"slot":w["cells"][3]["slot"],"options":options,"now":NOW}))}
fn offered(b:&Value,rf:&RecipeRef)->bool {b["choices"].as_array().unwrap().iter().any(|c|c["selection"]["recipe"]==json!(rf))}
fn options(state:&Value,preset:&str,n:&str,d:&str)->Value {json!({"overrides":{"portions":[{"person":{"kind":"diner","data":{"diner_id":state["snapshot"]["profile"]["diners"][0]["diner_id"]}},"preset":preset,"adjustment":{"numerator":n,"denominator":d}}],"eat_by":null}})}
fn select_at(state:&Value,rf:&RecipeRef,opts:Value,id:u32,at:&str)->Value {
    let w=&state["snapshot"]["weeks"][0];let b=good(json!({"op":"board","state":state,"week_id":w["range"]["week_id"],"slot":w["cells"][3]["slot"],"options":opts,"now":at}));
    let choice=b["choices"].as_array().unwrap().iter().find(|c|c["selection"]["recipe"]==json!(rf)).expect("eligible exact pinned Larger or original R3 choice");
    let mut w=w.clone();w["plan"]=choice["plan"].clone();apply_at(state,week_body(&w),id,at)
}
fn saved()->Value {let s=ready();select_at(&s,&larger_porridge_packet_ref().unwrap().recipe,options(&s,"larger","1","1"),0xf703,NOW)}
fn profile(state:&Value,p:Value,id:u32)->Value {apply(state,json!({"kind":"update_profile","data":{"expected_profile_revision":p["revision"],"profile":p}}),id)}
fn cold(s:Value)->Value {std::thread::spawn(move||good(json!({"op":"validate_state","state":s,"now":NOW}))).join().unwrap()}
fn qty(n:&str,d:&str,unit:&str,form:&str)->Value {json!({"amount":{"numerator":n,"denominator":d},"unit":unit,"form":form})}
fn frozen_previous_refs()->Value {json!([
    {"recipe":{"recipe_id":"rc_80000000000000000000000000000001","revision":"3","digest":"dd867c017a9b591f57546246518cd93c94e9f1ab50c54bfe4f288a1054de6ce0"},"packet_digest":"edc4239b57366c96ef4b91198e4cc54de62db44d3d6346574e7318a852e2d726"},
    {"recipe":{"recipe_id":"rc_80000000000000000000000000000003","revision":"1","digest":"58914b613960257e33ce294636bcd417baccb631f61379a22aa4977b14cc14f3"},"packet_digest":"62949a9b43fb50ed7385271a22c09f9b5d703ff08ce865041b66c6d6f2d14dee"},
    {"recipe":{"recipe_id":"rc_80000000000000000000000000000004","revision":"1","digest":"247de874dc0348b4afdc06fe6b5aab31cdebf2869ace5098393d61dae9a80377"},"packet_digest":"ec3894c5071806462224a3ba282467b159b8a84ad05df8ae12995489784802ac"},
    {"recipe":{"recipe_id":"rc_80000000000000000000000000000005","revision":"1","digest":"bea391c42fe3a7207d4ac04104f5fac6d839a36970a197be490585c89fb0cf7c"},"packet_digest":"e18d259dfa7941b893997dee7b378616457025ccac92021d54a2a43e25173932"},
    {"recipe":{"recipe_id":"rc_80000000000000000000000000000006","revision":"1","digest":"9e5495c21e6da79685364e4069c62485db521652f5b184f955ff5ae27a6f1632"},"packet_digest":"61aa9bc1b586ff82a1ceab2198f5cf88e7113ef82e37bdfa2b60cd4c7915de6d"}
])}

#[test]
fn exact_append_only_registry_clock_and_same_source_variants() {
    let refs=available_packet_refs().unwrap();assert_eq!(refs.len(),11);
    assert_eq!(refs[..5],[porridge_packet_ref().unwrap(),aglio_packet_ref().unwrap(),three_bean_packet_ref().unwrap(),three_bean_two_unit_packet_ref().unwrap(),three_bean_two_unit_vegetarian_packet_ref().unwrap()]);
    assert_eq!(json!(&refs[..5]),frozen_previous_refs(),"Existing full recipe and packet pins stay literal and append-only");
    let rf=larger_porridge_packet_ref().unwrap();assert_eq!(refs[5],rf);let known=known_packet(&rf).unwrap();
    assert_eq!(known.not_before(),&Instant("2026-10-13T02:40:31Z".into()));assert_eq!(known.root_policy_evidence_id().0,"ev_8000000000000000000000000000003d");
    assert_eq!(known.record().packet["maker_evidence_pins"].as_array().unwrap().len(),11);
    let s=setup_at(BEFORE);let r=call(json!({"op":"dispatch","state":s,"command":command(&s,json!({"kind":"admit_reviewed_recipe","data":{"packet":rf,"expected_catalogue_revision":"1"}}),0xf710),"now":BEFORE}));assert_eq!(r["ok"],false);assert_eq!(r["error"]["code"],"authority_denied");
    let s=ready();let sources=board(&s,Value::Null)["details"]["reviewed_sources"].clone();let old=sources.as_array().unwrap().iter().find(|x|x["packet"]["recipe"]==json!(porridge_packet_ref().unwrap().recipe)).unwrap();let new=sources.as_array().unwrap().iter().find(|x|x["packet"]["recipe"]==json!(rf.recipe)).unwrap();
    assert_eq!(old["recipe"]["source"]["url"],new["recipe"]["source"]["url"]);assert_ne!(old["packet"],new["packet"]);assert_ne!(old["scope"],new["scope"]);assert_eq!(cold(s.clone()),s);
}

#[test]
fn regular_default_refuses_explicit_larger_is_saved_without_profile_or_other_meal_edits() {
    let s=ready();let rf=larger_porridge_packet_ref().unwrap().recipe;assert!(!offered(&board(&s,Value::Null),&rf));
    assert!(offered(&board(&s,options(&s,"larger","1","1")),&rf));let saved=select_at(&s,&rf,options(&s,"larger","1","1"),0xf711,NOW);
    assert_eq!(saved["snapshot"]["profile"],s["snapshot"]["profile"]);assert_eq!(saved["snapshot"]["privacy"],s["snapshot"]["privacy"]);
    assert_eq!(saved["snapshot"]["weeks"][0]["cells"],s["snapshot"]["weeks"][0]["cells"]);assert_eq!(saved["snapshot"]["weeks"][0]["defaults_snapshot"],s["snapshot"]["weeks"][0]["defaults_snapshot"]);
    let selection=&saved["snapshot"]["weeks"][0]["plan"]["selections"][0];assert_eq!(selection["servings"][0]["preset_ref"]["preset"],"larger");assert_eq!(selection["meal_overrides"]["portions"][0]["preset"],"larger");assert_eq!(selection["meal_overrides"]["portions"][0]["person"],selection["servings"][0]["person"]);
    assert_eq!(saved["snapshot"]["weeks"][0]["plan"]["selections"].as_array().unwrap().len(),1);assert_eq!(cold(saved.clone()),saved);
}

#[test]
fn saved_breakfast_larger_and_earlier_different_unit_names_follow_native_precedence() {
    let s=ready();let mut p=s["snapshot"]["profile"].clone();p["diners"][0]["portions"]["preset"]=json!("smaller");
    p["diners"][0]["portions"]["meal_overrides"]=json!([{"meal_type":"breakfast","preset":"larger","adjustment":{"numerator":"1","denominator":"1"}},{"meal_type":"lunch","preset":"regular","adjustment":{"numerator":"1","denominator":"1"}},{"meal_type":"dinner","preset":"regular","adjustment":{"numerator":"1","denominator":"1"}}]);
    let changed=profile(&s,p,0xf712);let rf=larger_porridge_packet_ref().unwrap().recipe;assert!(offered(&board(&changed,Value::Null),&rf));
    let saved=select_at(&changed,&rf,Value::Null,0xf713,NOW);assert_eq!(saved["snapshot"]["profile"],changed["snapshot"]["profile"]);assert_eq!(saved["snapshot"]["weeks"][0]["plan"]["selections"][0]["meal_overrides"]["portions"],json!([]));
    assert!(!offered(&board(&changed,options(&changed,"regular","1","1")),&rf));assert_eq!(cold(saved.clone()),saved);
}

#[test]
fn fixed_quantities_one_allocation_and_one_twenty_minute_hob_session_are_native_derived() {
    let s=saved();let b=board(&s,Value::Null);let d=&b["details"]["derived"];assert_eq!(d["allocations"].as_array().unwrap().len(),1);
    let selection=&s["snapshot"]["weeks"][0]["plan"]["selections"][0];assert_eq!(selection["batch_use"]["data"]["production_servings"],json!({"numerator":"1","denominator":"1"}));
    assert_eq!(d["allocations"][0]["person"],selection["servings"][0]["person"]);assert_eq!(d["allocations"][0]["quantity"],qty("1","1","each","cooked"));assert_eq!(d["allocations"][0]["component_id"],"co_80000000000000000000000000000007");
    assert_eq!(d["demand"]["lines"].as_array().unwrap().len(),3);
    for (suffix,n,den,unit,form) in [("01","135","2","g","dry"),("02","270","1","ml","raw"),("03","270","1","ml","raw")] {let line=d["demand"]["lines"].as_array().unwrap().iter().find(|x|x["ingredient_id"].as_str().unwrap().ends_with(suffix)).unwrap();assert_eq!(line["quantity"],qty(n,den,unit,form));assert_eq!(line["essential"],false);}
    let sessions=d["prep_sessions"].as_array().unwrap();assert_eq!(sessions.len(),1);assert_eq!(sessions[0]["tasks"].as_array().unwrap().len(),1);assert_eq!(sessions[0]["start"]["local_time"],"07:40:00");assert_eq!(sessions[0]["end"]["local_time"],"08:00:00");assert_eq!(sessions[0]["ingredient_needed_at"],sessions[0]["start"]);
    let t:BrowserState=serde_json::from_value(s.clone()).unwrap();let view=RuntimeCatalogue::from_records(&t.reviewed_packets.0).unwrap();let (basis,_)=mstrchf_browser::calendar_view::material_at(&view,&t.snapshot,t.snapshot.profile.0.as_ref().unwrap(),&t.snapshot.weeks[0],&Instant(NOW.into())).unwrap().unwrap();
    assert_eq!(basis.evidence.methods[0].prep_ids,vec![1]);assert_eq!(basis.evidence.methods[0].evidence_ids,vec![EvidenceId("ev_80000000000000000000000000000037".into())]);assert_eq!(basis.evidence.resources.len(),1);assert_eq!(basis.evidence.resources[0].equipment,vec![EquipmentCode::Hob]);assert_eq!(basis.evidence.resources[0].evidence_ids,basis.evidence.methods[0].evidence_ids);assert_eq!(cold(s.clone()),s);
}

#[test]
fn every_applicable_nonunit_layer_count_guest_age_diet_and_reuse_bypass_refuses() {
    let s=saved();let t:BrowserState=serde_json::from_value(s).unwrap();let view=RuntimeCatalogue::from_records(&t.reviewed_packets.0).unwrap();let(basis,_)=mstrchf_browser::calendar_view::material_at(&view,&t.snapshot,t.snapshot.profile.0.as_ref().unwrap(),&t.snapshot.weeks[0],&Instant(NOW.into())).unwrap().unwrap();let selected=basis.plan.selections[0].clone();let meal=basis.context.requirements.iter().find(|m|m.slot==selected.slot).unwrap().clone();let recipe=known_packet(&larger_porridge_packet_ref().unwrap()).unwrap().recipe();
    let check=|m:&MealRequirements,s:&MealSelection,p:&WeekPlan,o:&BoardOptions|check_personal_use_scope(recipe,&selected.recipe,m,s,p,o);
    assert!(check(&meal,&selected,&basis.plan,&Default::default()).is_ok());
    for layer in 0..4 {let mut m=meal.clone();let mut s=selected.clone();let mut o=BoardOptions::default();let nonunit=PositiveRational::new(3,2).unwrap();
        match layer {0=>m.attendees[0].portion.adjustment=nonunit,1=>m.attendees[0].portion.meal_overrides=BoundedVec(vec![MealPortionOverride{meal_type:MealType::Breakfast,preset:PortionPresetName::Regular,adjustment:nonunit}]),2=>s.meal_overrides.portions[0].adjustment=nonunit,_=>o.overrides=Some(MealOverrides{portions:BoundedVec(vec![PersonPortionOverride{person:m.attendees[0].person.clone(),preset:PortionPresetName::Larger,adjustment:nonunit}]),eat_by:Nullable(None)})}
        assert!(check(&m,&s,&basis.plan,&o).is_err());}
    let mut m=meal.clone();let mut s=selected.clone();m.attendees[0].portion.adjustment=PositiveRational::new(1,2).unwrap();let mut second=m.attendees[0].clone();second.person=PersonRef::Diner{diner_id:DinerId("dn_80000000000000000000000000000007".into())};m.attendees.0.push(second.clone());s.servings[0].servings=PositiveRational::new(1,2).unwrap();let mut allocation=s.servings[0].clone();allocation.person=second.person;s.servings.0.push(allocation);assert!(check(&m,&s,&basis.plan,&Default::default()).is_err());
    let mut m=meal.clone();m.attendees[0].person=PersonRef::Guest{occasion_id:OccasionId("oc_80000000000000000000000000000007".into()),guest_id:GuestId("gu_80000000000000000000000000000007".into())};assert!(check(&m,&selected,&basis.plan,&Default::default()).is_err());
    for age in [AgeBand::Teen,AgeBand::Child4To6] {let mut m=meal.clone();m.attendees[0].age_band=age;assert!(check(&m,&selected,&basis.plan,&Default::default()).is_err());}
    for requirement in [HardRequirement::Diet{code:DietCode::Vegetarian},HardRequirement::Diet{code:DietCode::Vegan},HardRequirement::Diet{code:DietCode::GlutenFree},HardRequirement::AllergenExclusion{code:AllergenCode::Milk}] {let mut m=meal.clone();m.attendees[0].hard_requirements=BoundedVec(vec![requirement.clone()]);m.shared_hard_requirements=BoundedVec(vec![requirement]);assert!(check(&m,&selected,&basis.plan,&Default::default()).is_err());}
    for o in [BoardOptions{make_extra:Some(true),..Default::default()},BoardOptions{leftover_source:Some(selected.slot.clone()),..Default::default()}] {assert!(check(&meal,&selected,&basis.plan,&o).is_err());}
    let mut m=meal.clone();m.location=MealLocation::OnTheMove;assert!(check(&m,&selected,&basis.plan,&Default::default()).is_err());let mut m=meal.clone();m.equipment=BoundedVec(vec![EquipmentCode::Fridge]);assert!(check(&m,&selected,&basis.plan,&Default::default()).is_err());
    let mut plan=basis.plan.clone();let mut other=selected.clone();other.slot.date=LocalDate("2026-10-21".into());plan.selections.0.push(other);assert!(check(&meal,&selected,&plan,&Default::default()).is_err());
    let mut foreign=selected.clone();foreign.servings[0].person=PersonRef::Diner{diner_id:DinerId("dn_800000000000000000000000000000ff".into())};assert!(check(&meal,&foreign,&basis.plan,&Default::default()).is_err());
    let mut wrong=selected.clone();wrong.servings[0].preset_ref.preset=PortionPresetName::Regular;assert!(check(&meal,&wrong,&basis.plan,&Default::default()).is_err());
    let mut wrong=selected.clone();wrong.meal_overrides.portions.0.clear();assert!(check(&meal,&wrong,&basis.plan,&Default::default()).is_err(),"allocation alone cannot manufacture effective Larger");
}

#[test]
fn actual_current_profile_adjustment_and_saved_diet_refuse_new_offers_and_forged_plan() {
    let s=ready();let rf=larger_porridge_packet_ref().unwrap().recipe;
    for (n,d) in [("3","2"),("1","2"),("2","1")] {assert!(!offered(&board(&s,options(&s,"larger",n,d)),&rf));}
    let mut p=s["snapshot"]["profile"].clone();p["diners"][0]["portions"]["adjustment"]=json!({"numerator":"2","denominator":"1"});let changed=profile(&s,p,0xf714);assert!(!offered(&board(&changed,options(&changed,"larger","1","1")),&rf));
    let mut p=s["snapshot"]["profile"].clone();p["diners"][0]["restrictions"]=json!({"kind":"confirmed","data":{"confirmed_at":NOW,"requirements":[{"kind":"diet","data":{"code":"vegetarian"}}]}});let changed=profile(&s,p,0xf715);assert!(!offered(&board(&changed,options(&changed,"larger","1","1")),&rf));
    let saved=saved();let mut w=saved["snapshot"]["weeks"][0].clone();w["plan"]["selections"][0]["meal_overrides"]["portions"]=json!([]);
    let r=call(json!({"op":"dispatch","state":saved,"command":command(&saved,week_body(&w),0xf716),"now":NOW}));assert_eq!(r["ok"],false);assert_eq!(cold(saved.clone()),saved);
}

#[test]
fn original_r3_receipt_pin_response_and_ack_survive_later_larger_admission_and_replacement() {
    let s=admit_at(&setup_at(OLD),porridge_packet_ref().unwrap(),0xf720,OLD);let original=select_at(&s,&porridge_packet_ref().unwrap().recipe,Value::Null,0xf721,OLD);
    let original_receipts=original["receipts"].clone();let index=original_receipts.as_array().unwrap().len()-1;let receipt=&original_receipts[index];assert_eq!(receipt["food_catalogue_pin"]["revision"],"2");
    let original_food=good(json!({"op":"board","state":original,"now":OLD}))["details"]["derived"].clone();let admitted=admit_at(&original,larger_porridge_packet_ref().unwrap(),0xf722,NOW);
    assert_eq!(&admitted["receipts"].as_array().unwrap()[..original_receipts.as_array().unwrap().len()],original_receipts.as_array().unwrap());assert_eq!(admitted["snapshot"]["weeks"],original["snapshot"]["weeks"]);
    let replay=good(json!({"op":"dispatch","state":admitted,"command":receipt["command"],"now":NOW}));assert_eq!(replay["state"],admitted);assert_eq!(replay["receipt"]["disposition"],"replayed");assert_eq!(replay["receipt"]["response_digest"],receipt["response_digest"]);
    let replaced=select_at(&admitted,&larger_porridge_packet_ref().unwrap().recipe,options(&admitted,"larger","1","1"),0xf723,NOW);assert_eq!(replaced["snapshot"]["profile"],original["snapshot"]["profile"]);assert_eq!(&replaced["receipts"].as_array().unwrap()[..original_receipts.as_array().unwrap().len()],original_receipts.as_array().unwrap());assert_eq!(cold(replaced.clone()),replaced);
    assert_eq!(good(json!({"op":"board","state":original,"now":OLD}))["details"]["derived"],original_food);
    let mut forged=replaced.clone();forged["receipts"][index]["food_catalogue_pin"]=replaced["receipts"].as_array().unwrap().last().unwrap()["food_catalogue_pin"].clone();let refusal=call(json!({"op":"validate_state","state":forged,"now":NOW}));assert_eq!(refusal["ok"],false);
}

#[test]
fn public_checksums_cannot_admit_changed_quantity_or_cook_authority() {
    let rf=larger_porridge_packet_ref().unwrap();let original=known_packet(&rf).unwrap();
    for cook in [false,true] {let mut fake=original.record().clone();if cook {fake.evidence_documents[10]["limits"]["human_cook_test"]=json!(true);} else {fake.packet["maker_recipe_revision"]["ingredients"][0]["quantity"]["amount"]["numerator"]=json!("136");}
        fake.packet["maker_evidence_pins"]=json!(fake.evidence_documents.as_array().unwrap().iter().map(|d|json!({"evidence_id":d["evidence_id"],"revision":"1","digest":canonical_digest(d).unwrap()})).collect::<Vec<_>>());fake.packet["evidence_documents_digest"]=json!(canonical_digest(&fake.evidence_documents).unwrap());assert_eq!(known_record(&fake).unwrap_err().code,ErrorCode::DigestMismatch);
    }
    assert_eq!(known_record(original.record()).unwrap().reference(),&rf);
}

#[test]
fn synthetic_booked_late_delivery_retains_order_and_refuses_calendar_before_all_prep() {
    let s=saved();let b=board(&s,Value::Null);let w=&s["snapshot"]["weeks"][0];let approved=apply(&s,json!({"kind":"approve_plan","data":{"week_id":w["range"]["week_id"],"plan_revision":w["plan"]["revision"],"derived_digest":b["details"]["derived"]["digest"]}}),0xf730);
    for (late,id) in [(false,0xf731),(true,0xf732)] {let w=&approved["snapshot"]["weeks"][0];let moment=|time:&str|good(json!({"op":"resolve_moment","date":w["cells"][3]["slot"]["date"],"time":time,"zone":w["range"]["zone"],"resolution":"unambiguous"}));
        let booked=apply(&approved,json!({"kind":"confirm_order","data":{"confirmation":{"week_id":w["range"]["week_id"],"approval_id":w["approved_plan"]["approval_id"],"delivery_start":moment("07:00:00"),"delivery_end":moment(if late {"07:40:01"} else {"07:39:00"}),"confirmed_at":NOW,"source":{"kind":"user_confirmation","data":{}}}}}),id);
        let order=&booked["snapshot"]["weeks"][0]["confirmed_order"];let preview=call(json!({"op":"calendar_preview","state":booked,"order_id":order["order_id"],"destination_id":DEST,"now":NOW}));assert_eq!(preview["ok"],!late);
        if late {assert!(preview["error"]["issues"].to_string().contains("/delivery_end"));} else {let body=preview["value"]["batch"]["events"].as_array().unwrap().iter().find(|event|event["body"].as_str().unwrap().contains("five-minute")).unwrap()["body"].as_str().unwrap();assert!(body.contains("135/2")&&body.contains("270 ml")&&body.contains("unmeasured estimate")&&body.contains("delay service"));}
        assert!(!order.is_null());assert!(!booked["history"]["ordered"].as_array().unwrap().is_empty());assert_eq!(cold(booked.clone()),booked);
    }
}

#[test]
fn booked_current_portion_or_diet_change_retains_exact_food_order_history_and_original_proof() {
    let s=saved();let b=board(&s,Value::Null);let w=&s["snapshot"]["weeks"][0];
    let approved=apply(&s,json!({"kind":"approve_plan","data":{"week_id":w["range"]["week_id"],"plan_revision":w["plan"]["revision"],"derived_digest":b["details"]["derived"]["digest"]}}),0xf740);
    let w=&approved["snapshot"]["weeks"][0];let moment=|time:&str|good(json!({"op":"resolve_moment","date":w["cells"][3]["slot"]["date"],"time":time,"zone":w["range"]["zone"],"resolution":"unambiguous"}));
    let booked=apply(&approved,json!({"kind":"confirm_order","data":{"confirmation":{"week_id":w["range"]["week_id"],"approval_id":w["approved_plan"]["approval_id"],"delivery_start":moment("07:00:00"),"delivery_end":moment("07:39:00"),"confirmed_at":NOW,"source":{"kind":"user_confirmation","data":{}}}}}),0xf741);
    let order=booked["snapshot"]["weeks"][0]["confirmed_order"].clone();let old_receipts=booked["receipts"].as_array().unwrap();
    for vegetarian in [false,true] {
        let mut p=booked["snapshot"]["profile"].clone();
        if vegetarian {p["diners"][0]["restrictions"]=json!({"kind":"confirmed","data":{"confirmed_at":NOW,"requirements":[{"kind":"diet","data":{"code":"vegetarian"}}]}});}
        else {p["diners"][0]["portions"]["adjustment"]=json!({"numerator":"2","denominator":"1"});}
        let changed=profile(&booked,p,0xf742+u32::from(vegetarian));
        assert_eq!(changed["snapshot"]["weeks"][0]["plan"]["selections"],booked["snapshot"]["weeks"][0]["plan"]["selections"]);
        assert_eq!(changed["snapshot"]["weeks"][0]["confirmed_order"],order);assert_eq!(changed["history"],booked["history"]);
        // No calendar was prepared/published. A profile edit preserves that
        // historical status; current food/calendar guards must still refuse.
        assert_eq!(changed["snapshot"]["weeks"][0]["publication"],booked["snapshot"]["weeks"][0]["publication"]);
        assert!(!changed["snapshot"]["conflicts"].as_array().unwrap().is_empty());
        assert_eq!(&changed["receipts"].as_array().unwrap()[..old_receipts.len()],old_receipts.as_slice());
        let current=board(&changed,Value::Null);assert!(current["details"]["derived"].is_null());assert!(!current["details"]["food_error"].is_null());
        let refusal=call(json!({"op":"calendar_preview","state":changed,"order_id":order["order_id"],"destination_id":DEST,"now":NOW}));assert_eq!(refusal["ok"],false);
        assert_eq!(cold(changed.clone()),changed);
        let replay=good(json!({"op":"dispatch","state":changed,"command":old_receipts.last().unwrap()["command"],"now":NOW}));
        assert_eq!(replay["receipt"]["disposition"],"replayed");assert_eq!(replay["receipt"]["response_digest"],old_receipts.last().unwrap()["response_digest"]);assert_eq!(replay["state"],changed);
    }
}
