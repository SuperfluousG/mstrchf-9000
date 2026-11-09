//! Actual native commands with synthetic households and confirmations. The
//! conditional recipe definition never observes products, cook tests or chilling.
use mstrchf_browser::{run_json, BrowserState, BoardOptions, catalogue::RuntimeCatalogue,
    maker_packets::*, personal_scope::*};
use mstrchf_contracts::*;
use serde_json::{json, Value};

const NOW: &str = "2026-10-13T04:00:00Z";
const BEFORE: &str = "2026-10-13T02:28:11Z";
const DEST: &str = "cd_10000000000000000000000000000001";
fn call(input: Value) -> Value { serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap() }
fn good(input: Value) -> Value { let result=call(input); assert_eq!(result["ok"],true,"{result}"); result["value"].clone() }
fn command(state:&Value, body:Value, id:u32) -> Value { json!({"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body}) }
fn apply_at(state:&Value, body:Value, id:u32, at:&str) -> Value { good(json!({"op":"dispatch","state":state,"command":command(state,body,id),"now":at}))["state"].clone() }
fn apply(state:&Value, body:Value, id:u32) -> Value { apply_at(state,body,id,NOW) }
fn week_edit(week:Value, plan:bool) -> Value { json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{"week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],"day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],"cells":week["cells"],"plan":if plan {week["plan"].clone()} else {Value::Null}}}}) }
fn edit(state:&Value, week:Value, id:u32, plan:bool) -> Value { apply(state,week_edit(week,plan),id) }
fn diet(code:&str) -> Value { json!({"kind":"diet","data":{"code":code}}) }
fn declaration(requirement:&Value) -> Value { if requirement.is_null() {json!({"kind":"confirmed_none","data":{"confirmed_at":NOW}})} else {json!({"kind":"confirmed","data":{"confirmed_at":NOW,"requirements":[requirement]}})} }
fn setup() -> Value {
    let mut state=serde_json::from_slice::<Value>(include_bytes!("fixtures/legacy-engineering-receipts.json")).unwrap()["state"].clone();
    let mut policy=state["snapshot"]["privacy"].clone();policy["use_ai_features"]=json!(false);policy["include_household_requirements"]=json!(false);
    state=apply_at(&state,json!({"kind":"set_privacy","data":{"expected_policy_revision":policy["revision"],"policy":policy}}),0xf600,BEFORE);
    let mut profile=state["snapshot"]["profile"].clone();profile["diners"][0]["age_band"]=json!("adult");profile["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":BEFORE}});
    profile["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});profile["preferences"]["equipment"]=json!(["fridge","hob"]);
    state=apply_at(&state,json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),0xf601,BEFORE);
    let mut week=state["snapshot"]["weeks"][0].clone();week["plan"]["selections"]=json!([]);week["plan"]["batch_links"]=json!([]);week["meal_type_enabled"]["lunch"]=json!(true);
    apply_at(&state,week_edit(week,true),0xf602,BEFORE)
}
fn group(state:&Value, guest:bool, first:&Value, second:&Value, id:u32) -> Value {
    let mut profile=state["snapshot"]["profile"].clone();profile["diners"][0]["restrictions"]=declaration(first);
    if !guest {let mut diner=profile["diners"][0].clone();diner["diner_id"]=json!("dn_80000000000000000000000000000006");diner["restrictions"]=declaration(second);profile["diners"].as_array_mut().unwrap().push(diner);}
    let state=apply(state,json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),id);
    let mut week=state["snapshot"]["weeks"][0].clone();
    if guest {week["cells"][4]["occasion"]=json!({"occasion_id":"oc_80000000000000000000000000000006","host_diner_ids":week["cells"][4]["attendee_ids"],"guests":[{"guest_id":"gu_80000000000000000000000000000006","age_band":"adult","restrictions":declaration(second),"portion":{"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]}}]});}
    else {week["cells"][4]["attendee_ids"].as_array_mut().unwrap().push(json!("dn_80000000000000000000000000000006"));}
    edit(&state,week,id+1,false)
}
fn admit(state:&Value, packet:ReviewedRecipePacketRef, id:u32) -> Value { let revision=1+state["reviewed_packets"].as_array().map_or(0,Vec::len);apply(state,json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":revision.to_string()}}),id) }
fn ready(guest:bool, first:&Value, second:&Value) -> Value { let state=group(&setup(),guest,first,second,0xf603);let state=admit(&state,three_bean_packet_ref().unwrap(),0xf605);let state=admit(&state,three_bean_two_unit_packet_ref().unwrap(),0xf606);admit(&state,three_bean_two_unit_vegetarian_packet_ref().unwrap(),0xf607) }
fn board_slot(state:&Value, index:usize, options:Value) -> Value { let week=&state["snapshot"]["weeks"][0];good(json!({"op":"board","state":state,"week_id":week["range"]["week_id"],"slot":week["cells"][index]["slot"],"options":options,"now":NOW})) }
fn board(state:&Value, options:Value) -> Value { board_slot(state,4,options) }
fn offered(board:&Value, reference:&RecipeRef) -> bool { board["choices"].as_array().unwrap().iter().any(|choice|choice["selection"]["recipe"]==json!(reference)) }
fn select_slot(state:&Value, reference:&RecipeRef, index:usize, id:u32) -> Value { let b=board_slot(state,index,Value::Null);let choice=b["choices"].as_array().unwrap().iter().find(|choice|choice["selection"]["recipe"]==json!(reference)).expect("actual eligible exact choice");let mut week=state["snapshot"]["weeks"][0].clone();week["plan"]=choice["plan"].clone();edit(state,week,id,true) }
fn saved(guest:bool, first:&Value, second:&Value) -> Value {select_slot(&ready(guest,first,second),&three_bean_two_unit_vegetarian_packet_ref().unwrap().recipe,4,0xf608)}
fn cold(state:Value) -> Value {std::thread::spawn(move||good(json!({"op":"validate_state","state":state,"now":NOW}))).join().unwrap()}
fn qty(amount:u64, unit:&str, form:&str) -> Value {json!({"amount":{"numerator":amount.to_string(),"denominator":"1"},"unit":unit,"form":form})}
fn book(state:&Value, late:bool) -> Value {let b=board(state,Value::Null);let week=&state["snapshot"]["weeks"][0];let state=apply(state,json!({"kind":"approve_plan","data":{"week_id":week["range"]["week_id"],"plan_revision":week["plan"]["revision"],"derived_digest":b["details"]["derived"]["digest"]}}),0xf640);let week=&state["snapshot"]["weeks"][0];let moment=|time:&str|good(json!({"op":"resolve_moment","date":week["cells"][4]["slot"]["date"],"time":time,"zone":week["range"]["zone"],"resolution":"unambiguous"}));apply(&state,json!({"kind":"confirm_order","data":{"confirmation":{"week_id":week["range"]["week_id"],"approval_id":week["approved_plan"]["approval_id"],"delivery_start":moment("10:00:00"),"delivery_end":moment(if late {"11:00:01"} else {"10:59:00"}),"confirmed_at":NOW,"source":{"kind":"user_confirmation","data":{}}}}}),0xf641)}

#[test]
fn exact_registration_closure_clock_and_same_source_prefix_are_preserved() {
    let refs=available_packet_refs().unwrap();assert_eq!(refs.len(),11);assert_eq!(refs[..5],[porridge_packet_ref().unwrap(),aglio_packet_ref().unwrap(),three_bean_packet_ref().unwrap(),three_bean_two_unit_packet_ref().unwrap(),three_bean_two_unit_vegetarian_packet_ref().unwrap()]);
    assert_eq!(json!(&refs[..5]),json!([
        {"recipe":{"recipe_id":"rc_80000000000000000000000000000001","revision":"3","digest":"dd867c017a9b591f57546246518cd93c94e9f1ab50c54bfe4f288a1054de6ce0"},"packet_digest":"edc4239b57366c96ef4b91198e4cc54de62db44d3d6346574e7318a852e2d726"},
        {"recipe":{"recipe_id":"rc_80000000000000000000000000000003","revision":"1","digest":"58914b613960257e33ce294636bcd417baccb631f61379a22aa4977b14cc14f3"},"packet_digest":"62949a9b43fb50ed7385271a22c09f9b5d703ff08ce865041b66c6d6f2d14dee"},
        {"recipe":{"recipe_id":"rc_80000000000000000000000000000004","revision":"1","digest":"247de874dc0348b4afdc06fe6b5aab31cdebf2869ace5098393d61dae9a80377"},"packet_digest":"ec3894c5071806462224a3ba282467b159b8a84ad05df8ae12995489784802ac"},
        {"recipe":{"recipe_id":"rc_80000000000000000000000000000005","revision":"1","digest":"bea391c42fe3a7207d4ac04104f5fac6d839a36970a197be490585c89fb0cf7c"},"packet_digest":"e18d259dfa7941b893997dee7b378616457025ccac92021d54a2a43e25173932"},
        {"recipe":{"recipe_id":"rc_80000000000000000000000000000006","revision":"1","digest":"9e5495c21e6da79685364e4069c62485db521652f5b184f955ff5ae27a6f1632"},"packet_digest":"61aa9bc1b586ff82a1ceab2198f5cf88e7113ef82e37bdfa2b60cd4c7915de6d"}
    ]));
    let reference=three_bean_two_unit_vegetarian_packet_ref().unwrap();let packet=known_packet(&reference).unwrap();assert_eq!(refs[4],reference);
    assert_eq!(packet.not_before(),&Instant("2026-10-13T02:28:12Z".into()));assert_eq!(packet.root_policy_evidence_id().0,"ev_80000000000000000000000000000032");
    let table=packet.intrinsic_input_conditions().unwrap();assert_eq!(table["entries"].as_array().unwrap().len(),9);assert_eq!(table["recipe_reference"],json!(reference.recipe));assert_eq!(table["product_and_operational_compliance_observed"],false);assert_eq!(table["applies_even_when_all_consumers_confirmed_none"],true);
    let state=setup();let early=call(json!({"op":"dispatch","state":state,"command":command(&state,json!({"kind":"admit_reviewed_recipe","data":{"packet":reference,"expected_catalogue_revision":"1"}}),0xf610),"now":BEFORE}));assert_eq!(early["ok"],false);assert_eq!(early["error"]["code"],"authority_denied");
    let old=admit(&admit(&state,three_bean_packet_ref().unwrap(),0xf611),three_bean_two_unit_packet_ref().unwrap(),0xf612);let added=admit(&old,reference,0xf613);
    for field in ["profile","privacy","weeks","household_draft"] {assert_eq!(added["snapshot"][field],old["snapshot"][field]);}
    assert_eq!(added["history"],old["history"]);assert_eq!(&added["receipts"].as_array().unwrap()[..old["receipts"].as_array().unwrap().len()],old["receipts"].as_array().unwrap());
    let sources=board(&added,Value::Null)["details"]["reviewed_sources"].clone();let variants=sources.as_array().unwrap().iter().filter(|row|["rc_80000000000000000000000000000004","rc_80000000000000000000000000000005","rc_80000000000000000000000000000006"].contains(&row["packet"]["recipe"]["recipe_id"].as_str().unwrap())).collect::<Vec<_>>();assert_eq!(variants.len(),3);assert!(variants.windows(2).all(|rows|rows[0]["recipe"]["source"]["url"]==rows[1]["recipe"]["source"]["url"]&&rows[0]["packet"]!=rows[1]["packet"]));
    assert_eq!(cold(added.clone()),added);
}

#[test]
fn all_none_vegetarian_requirement_combinations_offer_exact_two_units_for_both_cohorts() {
    let vegetarian=diet("vegetarian");
    for guest in [false,true] {for (first,second) in [(&Value::Null,&Value::Null),(&vegetarian,&Value::Null),(&Value::Null,&vegetarian),(&vegetarian,&vegetarian)] {
        let state=saved(guest,first,second);let b=board(&state,Value::Null);let d=&b["details"]["derived"];
        assert_eq!(d["allocations"].as_array().unwrap().len(),2);assert_eq!(d["batches"][0]["components"][0]["produced"],qty(2,"each","prepared"));assert_eq!(d["batches"][0]["components"][0]["allocated"],qty(2,"each","prepared"));
        for serving in state["snapshot"]["weeks"][0]["plan"]["selections"][0]["servings"].as_array().unwrap() {assert_eq!(serving["servings"],json!({"numerator":"1","denominator":"1"}));let allocations=d["allocations"].as_array().unwrap().iter().filter(|row|row["person"]==serving["person"]).collect::<Vec<_>>();assert_eq!(allocations.len(),1);assert_eq!(allocations[0]["quantity"],qty(1,"each","prepared"));assert_eq!(allocations[0]["component_id"],"co_80000000000000000000000000000006");}
        assert_eq!(d["demand"]["lines"].as_array().unwrap().len(),9);
        for (suffix,amount,unit,form) in [("07",160,"g","drained"),("08",160,"g","drained"),("09",160,"g","drained"),("0a",80,"g","raw"),("0b",40,"g","raw"),("0c",120,"g","raw"),("0d",20,"ml","prepared"),("0e",30,"ml","prepared"),("0f",10,"g","prepared")] {let line=d["demand"]["lines"].as_array().unwrap().iter().find(|row|row["ingredient_id"].as_str().unwrap().ends_with(suffix)).unwrap();assert_eq!(line["quantity"],qty(amount,unit,form));assert_eq!(line["essential"],false);}
        let sessions=d["prep_sessions"].as_array().unwrap();assert_eq!(sessions.len(),1);assert_eq!(sessions[0]["tasks"].as_array().unwrap().len(),2);assert_eq!(sessions[0]["start"]["local_time"],"11:00:00");assert_eq!(sessions[0]["end"]["local_time"],"12:30:00");assert_eq!(sessions[0]["ingredient_needed_at"],sessions[0]["start"]);assert_eq!(d["batches"][0]["components"][0]["original_production"]["data"]["preparation_started_at"],sessions[0]["start"]);
        assert_eq!(state["snapshot"]["privacy"]["use_ai_features"],false);assert_eq!(state["snapshot"]["privacy"]["include_household_requirements"],false);
        if !first.is_null()||!second.is_null() {assert!(!offered(&b,&three_bean_two_unit_packet_ref().unwrap().recipe));assert!(!offered(&b,&three_bean_packet_ref().unwrap().recipe));}
        assert_eq!(cold(state.clone()),state);
    }}
}

#[test]
fn vegan_gluten_allergen_exclusion_additional_and_unconfirmed_requirements_get_no_offer() {
    let restricted=[diet("vegan"),diet("gluten_free"),json!({"kind":"allergen_exclusion","data":{"code":"milk"}}),json!({"kind":"ingredient_exclusion","data":{"ingredient_id":"ig_80000000000000000000000000000007"}})];
    for requirement in restricted {for guest in [false,true] {let state=ready(guest,&Value::Null,&requirement);assert!(!offered(&board(&state,Value::Null),&three_bean_two_unit_vegetarian_packet_ref().unwrap().recipe));assert_eq!(cold(state.clone()),state);}}
    let state=ready(true,&diet("vegetarian"),&Value::Null);let mut week=state["snapshot"]["weeks"][0].clone();week["cells"][4]["occasion"]["guests"][0]["restrictions"]=json!({"kind":"confirmed","data":{"confirmed_at":NOW,"requirements":[diet("vegetarian"),diet("vegan")]}});let changed=edit(&state,week,0xf620,false);assert!(!offered(&board(&changed,Value::Null),&three_bean_two_unit_vegetarian_packet_ref().unwrap().recipe));
    let mut week=state["snapshot"]["weeks"][0].clone();week["cells"][4]["occasion"]["guests"][0]["restrictions"]=json!({"kind":"unconfirmed","data":{}});let changed=edit(&state,week,0xf621,false);let week=&changed["snapshot"]["weeks"][0];let refusal=call(json!({"op":"board","state":changed,"week_id":week["range"]["week_id"],"slot":week["cells"][4]["slot"],"now":NOW}));assert_eq!(refusal["ok"],false);assert_eq!(refusal["error"]["code"],"domain_conflict");assert_eq!(cold(changed.clone()),changed);
}

#[test]
fn direct_scope_keeps_every_identity_portion_location_and_reuse_guard() {
    let state=saved(true,&diet("vegetarian"),&Value::Null);let state:BrowserState=serde_json::from_value(state).unwrap();let view=RuntimeCatalogue::from_records(&state.reviewed_packets.0).unwrap();let (basis,_)=mstrchf_browser::calendar_view::material_at(&view,&state.snapshot,state.snapshot.profile.0.as_ref().unwrap(),&state.snapshot.weeks[0],&Instant(NOW.into())).unwrap().unwrap();let selection=basis.plan.selections[0].clone();let meal=basis.context.requirements.iter().find(|row|row.slot==selection.slot).unwrap().clone();let recipe=known_packet(&three_bean_two_unit_vegetarian_packet_ref().unwrap()).unwrap().recipe();let check=|meal:&MealRequirements,selected:&MealSelection,options:&BoardOptions|check_personal_use_scope(recipe,&selection.recipe,meal,selected,&basis.plan,options);
    assert!(check(&meal,&selection,&Default::default()).is_ok());
    for count in [0,1,3] {let mut changed=meal.clone();changed.attendees.0.resize(count,meal.attendees[0].clone());assert!(check(&changed,&selection,&Default::default()).is_err());}
    for age in [AgeBand::Child4To6,AgeBand::Teen] {let mut changed=meal.clone();changed.attendees[0].age_band=age;assert!(check(&changed,&selection,&Default::default()).is_err());}
    let mut changed=meal.clone();changed.attendees[0].portion.adjustment=PositiveRational::new(2,1).unwrap();let mut selected=selection.clone();selected.meal_overrides.portions=BoundedVec(vec![PersonPortionOverride{person:changed.attendees[0].person.clone(),preset:PortionPresetName::Regular,adjustment:PositiveRational::new(1,2).unwrap()}]);assert!(check(&changed,&selected,&Default::default()).is_err());
    let mut selected=selection.clone();selected.servings[0].servings=PositiveRational::new(1,2).unwrap();selected.servings[1].servings=PositiveRational::new(3,2).unwrap();assert!(check(&meal,&selected,&Default::default()).is_err());
    let mut selected=selection.clone();selected.servings[1].person=selected.servings[0].person.clone();assert!(check(&meal,&selected,&Default::default()).is_err());
    let mut changed=meal.clone();changed.attendees.iter_mut().enumerate().for_each(|(index,row)|row.person=PersonRef::Guest{occasion_id:OccasionId("oc_80000000000000000000000000000006".into()),guest_id:GuestId(format!("gu_{index:032x}"))});assert!(check(&changed,&selection,&Default::default()).is_err());
    for options in [BoardOptions{make_extra:Some(true),..Default::default()},BoardOptions{leftover_source:Some(selection.slot.clone()),..Default::default()}] {assert!(check(&meal,&selection,&options).is_err());}
    let mut changed=meal.clone();changed.location=MealLocation::OnTheMove;changed.mobility.cold_storage=true;assert!(check(&changed,&selection,&Default::default()).is_err());let mut changed=meal.clone();changed.equipment=BoundedVec(vec![EquipmentCode::Hob]);assert!(check(&changed,&selection,&Default::default()).is_err());
    let mut changed=meal.clone();changed.shared_hard_requirements=BoundedVec(vec![HardRequirement::Diet{code:DietCode::Vegetarian},HardRequirement::Diet{code:DietCode::Vegan}]);assert!(check(&changed,&selection,&Default::default()).is_err());
    assert!(board(&serde_json::to_value(&state).unwrap(),json!({"eat_by_time":"00:30:00","time_resolution":"unambiguous"}))["choices"].as_array().unwrap().is_empty());
}

#[test]
fn departed_guest_retains_exact_vegetarian_selection_and_does_not_offer_unrestricted_single_unit() {
    let state=saved(true,&diet("vegetarian"),&Value::Null);let selections=state["snapshot"]["weeks"][0]["plan"]["selections"].clone();let mut week=state["snapshot"]["weeks"][0].clone();week["cells"][4]["occasion"]["guests"]=json!([]);let removed=edit(&state,week,0xf630,false);
    assert_eq!(removed["snapshot"]["weeks"][0]["plan"]["selections"],selections);assert_eq!(removed["history"],state["history"]);assert_eq!(removed["reviewed_packets"],state["reviewed_packets"]);assert!(!removed["snapshot"]["conflicts"].as_array().unwrap().is_empty());
    let b=board(&removed,Value::Null);assert!(!offered(&b,&three_bean_packet_ref().unwrap().recipe));assert!(!offered(&b,&three_bean_two_unit_vegetarian_packet_ref().unwrap().recipe));assert!(b["details"]["derived"].is_null());assert!(!b["details"]["food_error"].is_null());assert_eq!(cold(removed.clone()),removed);
    let mut week=removed["snapshot"]["weeks"][0].clone();week["plan"]["selections"]=json!([]);week["plan"]["batch_links"]=json!([]);let cleared=edit(&removed,week,0xf631,true);assert!(cleared["snapshot"]["conflicts"].as_array().unwrap().is_empty());
    assert!(cleared["history"]["selected"].as_array().unwrap().is_empty());
    for field in ["ordered","feedback","hidden_recipe_ids"] {assert_eq!(cleared["history"][field],removed["history"][field]);}
    assert_eq!(&cleared["receipts"].as_array().unwrap()[..removed["receipts"].as_array().unwrap().len()],removed["receipts"].as_array().unwrap());assert_eq!(cold(cleared.clone()),cleared);
}

#[test]
fn departed_guest_with_unrestricted_resident_can_explicitly_replace_with_actual_single_unit_fit() {
    let state=saved(true,&Value::Null,&diet("vegetarian"));let mut week=state["snapshot"]["weeks"][0].clone();week["cells"][4]["occasion"]["guests"]=json!([]);let removed=edit(&state,week,0xf632,false);assert!(offered(&board(&removed,Value::Null),&three_bean_packet_ref().unwrap().recipe));let replaced=select_slot(&removed,&three_bean_packet_ref().unwrap().recipe,4,0xf633);assert!(replaced["snapshot"]["conflicts"].as_array().unwrap().is_empty());assert_eq!(board(&replaced,Value::Null)["details"]["derived"]["allocations"].as_array().unwrap().len(),1);assert_eq!(cold(replaced.clone()),replaced);
}

#[test]
fn supplied_changed_plan_foreign_people_and_re_signed_retention_command_refuse() {
    let state=saved(true,&diet("vegetarian"),&Value::Null);let mut week=state["snapshot"]["weeks"][0].clone();week["cells"][4]["occasion"]["guests"]=json!([]);let body=week_edit(week.clone(),false);let removed=apply(&state,body.clone(),0xf650);let replay=good(json!({"op":"dispatch","state":removed,"command":command(&state,body,0xf650),"now":NOW}));assert_eq!(replay["state"],removed);assert_eq!(replay["receipt"]["disposition"],"replayed");
    for guest in [false,true] {let mut supplied=week.clone();let person=supplied["plan"]["selections"][0]["servings"].as_array_mut().unwrap().iter_mut().find(|serving|(serving["person"]["kind"]=="guest")==guest).unwrap();person["person"]["data"][if guest {"guest_id"} else {"diner_id"}]=json!(if guest {"gu_80000000000000000000000000000009"} else {"dn_80000000000000000000000000000009"});let refusal=call(json!({"op":"dispatch","state":state,"command":command(&state,week_edit(supplied,true),0xf651+u32::from(guest)),"now":NOW}));assert_eq!(refusal["ok"],false,"{refusal}");}
    let mut forged=removed.clone();let receipt=forged["receipts"].as_array_mut().unwrap().last_mut().unwrap();receipt["command"]["command"]["data"]["week"]["cells"][4]["choice"]=json!("on_the_move");receipt["command_digest"]=json!(canonical_digest(&receipt["command"]).unwrap());let refusal=call(json!({"op":"validate_state","state":forged,"now":NOW}));assert_eq!(refusal["ok"],false);assert!(refusal["error"]["issues"].to_string().contains("/receipts/retained_two_unit/response"));assert_eq!(cold(removed.clone()),removed);
}

#[test]
fn self_re_signed_packet_cannot_claim_observed_product_authority_or_change_intrinsic_inputs() {
    let state=ready(true,&diet("vegetarian"),&Value::Null);let reference=three_bean_two_unit_vegetarian_packet_ref().unwrap();let original=known_packet(&reference).unwrap();
    for observed in [false,true] {let mut forged=original.record().clone();if observed {forged.packet["intrinsic_input_conditions"]["product_and_operational_compliance_observed"]=json!(true);forged.evidence_documents[5]["actual_products_and_processing_observed"]=json!(true);} else {forged.packet["intrinsic_input_conditions"]["entries"][0]["cooked_ready_to_eat_canned_required"]=json!(false);}
        let table_digest=canonical_digest(&forged.packet["intrinsic_input_conditions"]).unwrap();forged.packet["intrinsic_input_conditions_digest"]=json!(table_digest);forged.evidence_documents[5]["intrinsic_input_conditions_digest"]=json!(table_digest);forged.evidence_documents[6]["accepted_intrinsic_input_conditions_digest"]=json!(table_digest);let dietary_pin=json!({"evidence_id":forged.evidence_documents[5]["evidence_id"],"revision":"1","digest":canonical_digest(&forged.evidence_documents[5]).unwrap()});forged.evidence_documents[6]["accepted_dietary_review_pin"]=dietary_pin.clone();let policy_pin=json!({"evidence_id":forged.evidence_documents[6]["evidence_id"],"revision":"1","digest":canonical_digest(&forged.evidence_documents[6]).unwrap()});forged.packet["intrinsic_input_custody"]=json!({"recipe_reference":reference.recipe,"intrinsic_input_conditions_digest":table_digest,"dietary_review_pin":dietary_pin,"maker_policy_pin":policy_pin});forged.packet["maker_evidence_pins"]=json!(forged.evidence_documents.as_array().unwrap().iter().map(|doc|json!({"evidence_id":doc["evidence_id"],"revision":"1","digest":canonical_digest(doc).unwrap()})).collect::<Vec<_>>());forged.packet["evidence_documents_digest"]=json!(canonical_digest(&forged.evidence_documents).unwrap());
        assert_eq!(known_record(&forged).unwrap_err().code,ErrorCode::DigestMismatch);let mut changed=state.clone();changed["reviewed_packets"].as_array_mut().unwrap()[2]=json!(forged);let refusal=call(json!({"op":"validate_state","state":changed,"now":NOW}));assert_eq!(refusal["ok"],false);assert_eq!(refusal["error"]["code"],"digest_mismatch");
    }
    assert_eq!(known_record(original.record()).unwrap().reference(),&reference);assert_eq!(cold(state.clone()),state);
}

#[test]
fn target_projection_does_not_hide_a_second_stale_vegetarian_selection() {
    let state=saved(true,&diet("vegetarian"),&Value::Null);let mut week=state["snapshot"]["weeks"][0].clone();let mut occasion=week["cells"][4]["occasion"].clone();occasion["occasion_id"]=json!("oc_80000000000000000000000000000009");occasion["guests"][0]["guest_id"]=json!("gu_80000000000000000000000000000009");week["cells"][7]["occasion"]=occasion;let state=edit(&state,week,0xf660,false);let state=select_slot(&state,&three_bean_two_unit_vegetarian_packet_ref().unwrap().recipe,7,0xf661);let mut week=state["snapshot"]["weeks"][0].clone();week["cells"][4]["occasion"]["guests"]=json!([]);week["cells"][7]["occasion"]["guests"]=json!([]);let removed=edit(&state,week,0xf662,false);let week=&removed["snapshot"]["weeks"][0];let refusal=call(json!({"op":"board","state":removed,"week_id":week["range"]["week_id"],"slot":week["cells"][4]["slot"],"now":NOW}));assert_eq!(refusal["ok"],false);assert_eq!(refusal["error"]["code"],"foreign_reference");assert_eq!(cold(removed.clone()),removed);
}

#[test]
fn selected_packet_owns_methods_resources_calendar_clock_and_conditional_once_only_text() {
    let state=book(&saved(true,&diet("vegetarian"),&Value::Null),false);let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();let view=RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();let(basis,_)=mstrchf_browser::calendar_view::material_at(&view,&typed.snapshot,typed.snapshot.profile.0.as_ref().unwrap(),&typed.snapshot.weeks[0],&Instant(NOW.into())).unwrap().unwrap();assert_eq!(basis.evidence.methods[0].evidence_ids,vec![EvidenceId("ev_80000000000000000000000000000030".into())]);assert!(basis.evidence.resources.iter().all(|row|row.equipment==vec![EquipmentCode::Fridge]&&row.evidence_ids==vec![EvidenceId("ev_8000000000000000000000000000002f".into())]));
    let order=&state["snapshot"]["weeks"][0]["confirmed_order"];let first=good(json!({"op":"calendar_preview","state":state,"order_id":order["order_id"],"destination_id":DEST,"now":NOW}));assert_eq!(first["batch"]["events"].as_array().unwrap().len(),2);let body=first["batch"]["events"].as_array().unwrap().iter().find(|event|event["body"].as_str().unwrap().contains("Reserve30minutes")).unwrap()["body"].as_str().unwrap();
    for text in ["at least60minutes passive refrigeration","11:00:00","Original storage clock","160 g","80 g","40 g","120 g","20 ml","30 ml","10 g","delay or refuse service","no animal body-part ingredients","actual product suitability"] {assert!(body.contains(text),"missing {text}: {body}");}assert_eq!(body.matches("For EACH container independently").count(),1);assert!(!body.contains("90minutes of full attention"));
    let prepared=apply(&state,json!({"kind":"request_calendar_publication","data":{"order_id":order["order_id"],"destination_id":DEST,"event_batch_digest":first["event_batch_digest"]}}),0xf642);let again=good(json!({"op":"calendar_preview","state":prepared,"order_id":order["order_id"],"destination_id":DEST,"now":NOW}));assert_eq!(again["prepared"],true);assert_eq!(again["batch"],first["batch"]);assert_eq!(again["ics"],first["ics"]);assert_eq!(cold(prepared.clone()),prepared);
}

#[test]
fn booked_guest_removal_preserves_original_order_history_and_receipt_pins() {
    let state=book(&saved(true,&diet("vegetarian"),&Value::Null),false);let order=state["snapshot"]["weeks"][0]["confirmed_order"].clone();let mut week=state["snapshot"]["weeks"][0].clone();week["cells"][4]["occasion"]["guests"]=json!([]);let removed=edit(&state,week,0xf670,false);assert_eq!(removed["snapshot"]["weeks"][0]["confirmed_order"],order);assert_eq!(removed["history"]["ordered"],state["history"]["ordered"]);assert_eq!(removed["snapshot"]["weeks"][0]["publication"],"needs_user");assert_eq!(&removed["receipts"].as_array().unwrap()[..state["receipts"].as_array().unwrap().len()],state["receipts"].as_array().unwrap());let refusal=call(json!({"op":"calendar_preview","state":removed,"order_id":order["order_id"],"destination_id":DEST,"now":NOW}));assert_eq!(refusal["ok"],false);assert_eq!(cold(removed.clone()),removed);
    let late=book(&saved(true,&diet("vegetarian"),&Value::Null),true);let refusal=call(json!({"op":"calendar_preview","state":late,"order_id":late["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],"destination_id":DEST,"now":NOW}));assert_eq!(refusal["ok"],false);assert!(refusal["error"]["issues"].to_string().contains("/delivery_end"));assert!(!late["snapshot"]["weeks"][0]["confirmed_order"].is_null());assert_eq!(cold(late.clone()),late);
}
