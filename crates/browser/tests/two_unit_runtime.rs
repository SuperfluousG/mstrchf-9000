//! Exact compiled two-unit food through ordinary commands. Households, guests
//! and checkout confirmations are synthetic; no product/fridge observation.
use mstrchf_browser::{BrowserState,run_json,maker_packets::*,catalogue::RuntimeCatalogue,personal_scope::*};
use mstrchf_contracts::*;
use serde_json::{Value,json};
const NOW:&str="2026-10-13T01:00:00Z";
const BEFORE:&str="2026-10-13T00:18:39Z";
const DEST:&str="cd_10000000000000000000000000000001";
fn call(input:Value)->Value{serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap()}
fn good(input:Value)->Value{let out=call(input);assert_eq!(out["ok"],true,"{out}");out["value"].clone()}
fn command(state:&Value,body:Value,id:u32)->Value{json!({"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body})}
fn apply_at(state:&Value,body:Value,id:u32,at:&str)->Value{good(json!({"op":"dispatch","state":state,"command":command(state,body,id),"now":at}))["state"].clone()}
fn apply(state:&Value,body:Value,id:u32)->Value{apply_at(state,body,id,NOW)}
fn week_edit(week:Value,plan:bool)->Value{json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{
 "week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],"day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],"cells":week["cells"],"plan":if plan{week["plan"].clone()}else{Value::Null}}}})}
fn edit(state:&Value,week:Value,id:u32,plan:bool)->Value{apply(state,week_edit(week,plan),id)}
fn setup()->Value{
 let mut state=serde_json::from_slice::<Value>(include_bytes!("fixtures/legacy-engineering-receipts.json")).unwrap()["state"].clone();
 let mut p=state["snapshot"]["profile"].clone();p["diners"][0]["age_band"]=json!("adult");p["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":BEFORE}});
 p["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});p["preferences"]["equipment"]=json!(["fridge","hob"]);
 state=apply_at(&state,json!({"kind":"update_profile","data":{"expected_profile_revision":p["revision"],"profile":p}}),0xee01,BEFORE);
 let mut week=state["snapshot"]["weeks"][0].clone();week["plan"]["selections"]=json!([]);week["plan"]["batch_links"]=json!([]);week["meal_type_enabled"]["lunch"]=json!(true);
 apply_at(&state,week_edit(week,true),0xee02,BEFORE)
}
fn admit(state:&Value,packet:ReviewedRecipePacketRef,id:u32)->Value{let revision=1+state["reviewed_packets"].as_array().map_or(0,Vec::len);apply(state,json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":revision.to_string()}}),id)}
fn group(state:&Value,guest:bool,id:u32)->Value{
 let mut state=state.clone();let mut week=state["snapshot"]["weeks"][0].clone();let host=week["cells"][4]["attendee_ids"].clone();
 if guest{week["cells"][4]["occasion"]=json!({"occasion_id":"oc_80000000000000000000000000000005","host_diner_ids":host,"guests":[{
  "guest_id":"gu_80000000000000000000000000000005","age_band":"adult","restrictions":{"kind":"confirmed_none","data":{"confirmed_at":NOW}},
  "portion":{"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]}}]});}
 else{let mut p=state["snapshot"]["profile"].clone();let mut second=p["diners"][0].clone();second["diner_id"]=json!("dn_80000000000000000000000000000005");p["diners"].as_array_mut().unwrap().push(second);
  state=apply(&state,json!({"kind":"update_profile","data":{"expected_profile_revision":p["revision"],"profile":p}}),id);
  week=state["snapshot"]["weeks"][0].clone();week["cells"][4]["attendee_ids"].as_array_mut().unwrap().push(json!("dn_80000000000000000000000000000005"));}
 edit(&state,week,id+1,false)
}
fn board(state:&Value,options:Value)->Value{let week=&state["snapshot"]["weeks"][0];good(json!({"op":"board","state":state,"week_id":week["range"]["week_id"],"slot":week["cells"][4]["slot"],"options":options,"now":NOW}))}
fn select_slot(state:&Value,reference:&RecipeRef,index:usize,id:u32)->Value{let week=&state["snapshot"]["weeks"][0];let b=good(json!({"op":"board","state":state,"week_id":week["range"]["week_id"],"slot":week["cells"][index]["slot"],"now":NOW}));let choice=b["choices"].as_array().unwrap().iter().find(|c|c["selection"]["recipe"]==json!(reference)).expect("exact eligible compiled choice");let mut w=week.clone();w["plan"]=choice["plan"].clone();edit(state,w,id,true)}
fn select(state:&Value,reference:&RecipeRef,id:u32)->Value{select_slot(state,reference,4,id)}
fn ready(guest:bool)->Value{let state=group(&setup(),guest,0xee03);let state=admit(&state,three_bean_packet_ref().unwrap(),0xee05);admit(&state,three_bean_two_unit_packet_ref().unwrap(),0xee06)}
fn saved(guest:bool)->Value{select(&ready(guest),&three_bean_two_unit_packet_ref().unwrap().recipe,0xee07)}
fn cold(state:Value)->Value{std::thread::spawn(move||good(json!({"op":"validate_state","state":state,"now":NOW}))).join().unwrap()}
fn qty(n:u64,unit:&str,form:&str)->Value{json!({"amount":{"numerator":n.to_string(),"denominator":"1"},"unit":unit,"form":form})}
fn book(state:&Value,late:bool)->Value{let b=board(state,Value::Null);let week=&state["snapshot"]["weeks"][0];let mut state=apply(state,json!({"kind":"approve_plan","data":{"week_id":week["range"]["week_id"],"plan_revision":week["plan"]["revision"],"derived_digest":b["details"]["derived"]["digest"]}}),0xee40);
 let week=&state["snapshot"]["weeks"][0];let date=week["cells"][4]["slot"]["date"].as_str().unwrap();let zone=week["range"]["zone"].as_str().unwrap();
 let moment=|time:&str|good(json!({"op":"resolve_moment","date":date,"time":time,"zone":zone,"resolution":"unambiguous"}));
 state=apply(&state,json!({"kind":"confirm_order","data":{"confirmation":{"week_id":week["range"]["week_id"],"approval_id":week["approved_plan"]["approval_id"],"delivery_start":moment("10:00:00"),"delivery_end":moment(if late{"11:00:01"}else{"10:59:00"}),"confirmed_at":NOW,"source":{"kind":"user_confirmation","data":{}}}}}),0xee41);state
}
fn preview(state:&Value)->Value{good(json!({"op":"calendar_preview","state":state,"order_id":state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],"destination_id":DEST,"now":NOW}))}

#[test]
fn admission_clock_exact_bundle_and_both_same_source_variants_are_distinct(){
 let state=setup();let packet=three_bean_two_unit_packet_ref().unwrap();let known=known_packet(&packet).unwrap();
 assert_eq!(known.not_before(),&Instant("2026-10-13T00:18:40Z".into()));assert_eq!(known.root_policy_evidence_id().0,"ev_8000000000000000000000000000002b");
 let early=call(json!({"op":"dispatch","state":state,"command":command(&state,json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":"1"}}),0xee10),"now":BEFORE}));assert_eq!(early["ok"],false);assert_eq!(early["error"]["code"],"authority_denied");
 let available=board(&state,Value::Null)["details"]["reviewed_sources"].clone();let old=available.as_array().unwrap().iter().find(|source|source["packet"]["recipe"]==json!(three_bean_packet_ref().unwrap().recipe)).unwrap();let new=available.as_array().unwrap().iter().find(|source|source["packet"]["recipe"]==json!(packet.recipe)).unwrap();
 assert_eq!(old["recipe"]["source"]["url"],new["recipe"]["source"]["url"]);assert_ne!(old["scope"],new["scope"]);assert_ne!(old["packet"],new["packet"]);
 let mut forged=known.record().clone();forged.packet["operational_scope"]["attendee_count"]=json!(1);assert!(known_record(&forged).is_err());
 let mut forged=packet;forged.recipe.digest=RecipeDigest("a".repeat(64));assert!(known_packet(&forged).is_err());assert_eq!(cold(state.clone()),state);
}

#[test]
fn both_allowed_cohorts_derive_two_exact_allocations_nine_double_inputs_and_one_two_phase_session(){
 for guest in [false,true]{let state=saved(guest);let b=board(&state,Value::Null);let d=&b["details"]["derived"];
  assert_eq!(d["allocations"].as_array().unwrap().len(),2);let selected=&state["snapshot"]["weeks"][0]["plan"]["selections"][0];
  assert_eq!(selected["batch_use"]["data"]["production_servings"],json!({"numerator":"2","denominator":"1"}));
  let people=selected["servings"].as_array().unwrap();for serving in people{assert_eq!(serving["servings"],json!({"numerator":"1","denominator":"1"}));let matches=d["allocations"].as_array().unwrap().iter().filter(|allocation|allocation["person"]==serving["person"]).collect::<Vec<_>>();assert_eq!(matches.len(),1);assert_eq!(matches[0]["quantity"],qty(1,"each","prepared"));assert_eq!(matches[0]["component_id"],"co_80000000000000000000000000000005");}
  assert_eq!(d["batches"][0]["components"][0]["produced"],qty(2,"each","prepared"));assert_eq!(d["batches"][0]["components"][0]["allocated"],qty(2,"each","prepared"));
  assert_eq!(d["demand"]["lines"].as_array().unwrap().len(),9);
  for (suffix,n,unit,form) in [("07",160,"g","drained"),("08",160,"g","drained"),("09",160,"g","drained"),("0a",80,"g","raw"),("0b",40,"g","raw"),("0c",120,"g","raw"),("0d",20,"ml","prepared"),("0e",30,"ml","prepared"),("0f",10,"g","prepared")]{let line=d["demand"]["lines"].as_array().unwrap().iter().find(|line|line["ingredient_id"].as_str().unwrap().ends_with(suffix)).unwrap();assert_eq!(line["quantity"],qty(n,unit,form));assert_eq!(line["essential"],false);}
  let sessions=d["prep_sessions"].as_array().unwrap();assert_eq!(sessions.len(),1);assert_eq!(sessions[0]["tasks"].as_array().unwrap().len(),2);assert_eq!(sessions[0]["start"]["local_time"],"11:00:00");assert_eq!(sessions[0]["end"]["local_time"],"12:30:00");assert_eq!(sessions[0]["ingredient_needed_at"],sessions[0]["start"]);
  assert_eq!(d["batches"][0]["components"][0]["original_production"]["data"]["preparation_started_at"],sessions[0]["start"]);
  let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();let view=RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();let(basis,_)=mstrchf_browser::calendar_view::material_at(&view,&typed.snapshot,typed.snapshot.profile.0.as_ref().unwrap(),&typed.snapshot.weeks[0],&Instant(NOW.into())).unwrap().unwrap();
  assert_eq!(basis.evidence.methods[0].evidence_ids,vec![EvidenceId("ev_8000000000000000000000000000002a".into())]);assert_eq!(basis.evidence.resources.len(),2);assert!(basis.evidence.resources.iter().all(|row|row.equipment==vec![EquipmentCode::Fridge]&&row.evidence_ids==vec![EvidenceId("ev_80000000000000000000000000000029".into())]));assert_eq!(cold(state.clone()),state);
 }
}

#[test]
fn exact_scope_refuses_person_count_identity_portion_cohort_and_reuse_bypasses(){
 let state=saved(true);let typed:BrowserState=serde_json::from_value(state).unwrap();let view=RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();let(basis,_)=mstrchf_browser::calendar_view::material_at(&view,&typed.snapshot,typed.snapshot.profile.0.as_ref().unwrap(),&typed.snapshot.weeks[0],&Instant(NOW.into())).unwrap().unwrap();let selection=basis.plan.selections[0].clone();let meal=basis.context.requirements.iter().find(|meal|meal.slot==selection.slot).unwrap().clone();let recipe=known_packet(&three_bean_two_unit_packet_ref().unwrap()).unwrap().recipe();
 let check=|m:&MealRequirements,s:&MealSelection,o:&mstrchf_browser::BoardOptions|check_personal_use_scope(recipe,&selection.recipe,m,s,&basis.plan,o);
 assert!(check(&meal,&selection,&Default::default()).is_ok());
 let mut m=meal.clone();m.attendees.0.truncate(1);assert!(check(&m,&selection,&Default::default()).is_err());
 let mut m=meal.clone();m.attendees[0].age_band=AgeBand::Teen;assert!(check(&m,&selection,&Default::default()).is_err());
 let mut m=meal.clone();m.attendees[0].hard_requirements=BoundedVec(vec![HardRequirement::Diet{code:DietCode::Vegetarian}]);assert!(check(&m,&selection,&Default::default()).is_err());
 let mut m=meal.clone();m.attendees[0].person=PersonRef::Guest{occasion_id:OccasionId("oc_80000000000000000000000000000006".into()),guest_id:GuestId("gu_80000000000000000000000000000006".into())};assert!(check(&m,&selection,&Default::default()).is_err());
 let mut s=selection.clone();s.servings[1].person=s.servings[0].person.clone();assert!(check(&meal,&s,&Default::default()).is_err());
 let mut s=selection.clone();s.servings[0].servings=PositiveRational::new(1,2).unwrap();s.servings[1].servings=PositiveRational::new(3,2).unwrap();assert!(check(&meal,&s,&Default::default()).is_err());
 let mut m=meal.clone();m.attendees[0].portion.adjustment=PositiveRational::new(2,1).unwrap();let mut s=selection.clone();s.meal_overrides.portions=BoundedVec(vec![PersonPortionOverride{person:m.attendees[0].person.clone(),preset:PortionPresetName::Regular,adjustment:PositiveRational::new(1,2).unwrap()}]);assert!(check(&m,&s,&Default::default()).is_err());
 for o in [mstrchf_browser::BoardOptions{make_extra:Some(true),..Default::default()},mstrchf_browser::BoardOptions{leftover_source:Some(selection.slot.clone()),..Default::default()}]{assert!(check(&meal,&selection,&o).is_err());}
 let mut m=meal.clone();m.location=MealLocation::OnTheMove;m.mobility.cold_storage=true;assert!(check(&m,&selection,&Default::default()).is_err());
 let mut m=meal.clone();m.equipment=BoundedVec(vec![EquipmentCode::Hob]);assert!(check(&m,&selection,&Default::default()).is_err());
 let mut s=selection.clone();if let BatchUse::NewBatch{production_servings,..}=&mut s.batch_use{*production_servings=PositiveRational::new(1,1).unwrap();}assert!(check(&meal,&s,&Default::default()).is_err());
 let mut wrong=selection.recipe.clone();wrong.digest=RecipeDigest("a".repeat(64));assert!(cold_home_timing(&wrong).is_err());
}

#[test]
fn two_unit_autumn_fold_uses_actual_90minutes_and_preserves_same_local_day_boundary(){
 let state=saved(true);let typed:BrowserState=serde_json::from_value(state).unwrap();let view=RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();
 let(basis,_)=mstrchf_browser::calendar_view::material_at(&view,&typed.snapshot,typed.snapshot.profile.0.as_ref().unwrap(),&typed.snapshot.weeks[0],&Instant(NOW.into())).unwrap().unwrap();
 let mut selection=basis.plan.selections[0].clone();let mut meal=basis.context.requirements.iter().find(|meal|meal.slot==selection.slot).unwrap().clone();
 meal.slot.date=LocalDate("2026-10-25".into());selection.slot=meal.slot.clone();let mut plan=basis.plan.clone();plan.selections=BoundedVec(vec![selection.clone()]);
 let recipe=known_packet(&three_bean_two_unit_packet_ref().unwrap()).unwrap().recipe();
 let moment=|time:&str,resolution:Option<Resolution>|mstrchf_core::availability::resolve_meal_time(&meal.slot.date,&LocalTime(time.into()),&ZoneId("Europe/London".into()),resolution).unwrap();
 let earlier=moment("01:00:00",Some(Resolution::Earlier));let later=moment("01:00:00",Some(Resolution::Later));let first=moment("00:30:00",None);
 assert_eq!(earlier.instant.0,"2026-10-25T00:00:00Z");assert_eq!(later.instant.0,"2026-10-25T01:00:00Z");assert_eq!(first.instant.0,"2026-10-24T23:30:00Z");
 assert_eq!((chrono::DateTime::parse_from_rfc3339(&later.instant.0).unwrap()-chrono::DateTime::parse_from_rfc3339(&first.instant.0).unwrap()).num_seconds(),5400);
 meal.eat_by=Nullable(Some(earlier));let refusal=check_personal_use_scope(recipe,&selection.recipe,&meal,&selection,&plan,&Default::default()).unwrap_err();
 assert_eq!(refusal.issues[0].path,"/personal_use_scope/same_day_first_handling");
 meal.eat_by=Nullable(Some(later));assert!(check_personal_use_scope(recipe,&selection.recipe,&meal,&selection,&plan,&Default::default()).is_ok());
}

#[test]
fn guest_removal_retains_two_unit_selection_and_history_as_conflict_then_direct_one_unit_replace_resolves(){
 let state=saved(true);let previous=state["snapshot"]["weeks"][0]["plan"]["selections"].clone();let history=state["history"].clone();let mut week=state["snapshot"]["weeks"][0].clone();week["cells"][4]["occasion"]["guests"]=json!([]);
 let removed=edit(&state,week,0xee20,false);assert_eq!(removed["snapshot"]["weeks"][0]["plan"]["selections"],previous);assert_eq!(removed["history"],history);assert!(!removed["snapshot"]["conflicts"].as_array().unwrap().is_empty());assert_eq!(cold(removed.clone()),removed);
 assert_eq!(&removed["receipts"].as_array().unwrap()[..state["receipts"].as_array().unwrap().len()],state["receipts"].as_array().unwrap());assert_eq!(removed["reviewed_packets"],state["reviewed_packets"]);
 let replaced=select(&removed,&three_bean_packet_ref().unwrap().recipe,0xee21);let b=board(&replaced,Value::Null);assert_eq!(b["details"]["derived"]["allocations"].as_array().unwrap().len(),1);assert!(replaced["snapshot"]["conflicts"].as_array().unwrap().is_empty());assert_eq!(b["details"]["derived"]["prep_sessions"][0]["start"]["local_time"],"11:15:00");assert_eq!(cold(replaced.clone()),replaced);
 let mut week=removed["snapshot"]["weeks"][0].clone();week["plan"]["selections"]=json!([]);week["plan"]["batch_links"]=json!([]);let cleared=edit(&removed,week,0xee22,true);assert!(cleared["snapshot"]["weeks"][0]["plan"]["selections"].as_array().unwrap().is_empty());assert!(cleared["snapshot"]["conflicts"].as_array().unwrap().is_empty());assert_eq!(cold(cleared.clone()),cleared);
}

#[test]
fn removal_cannot_admit_new_foreign_allocations_or_reinterpret_the_original_command(){
 let state=saved(true);let mut week=state["snapshot"]["weeks"][0].clone();week["cells"][4]["occasion"]["guests"]=json!([]);
 let removal=week_edit(week.clone(),false);let removed=apply(&state,removal.clone(),0xee50);
 let replay=good(json!({"op":"dispatch","state":removed,"command":command(&state,removal,0xee50),"now":NOW}));assert_eq!(replay["state"],removed);assert_eq!(replay["receipt"]["disposition"],"replayed");
 for guest in [true,false]{let mut changed=week.clone();let servings=changed["plan"]["selections"][0]["servings"].as_array_mut().unwrap();let selected=servings.iter_mut().find(|serving|(serving["person"]["kind"]=="guest")==guest).unwrap();
  selected["person"]["data"][if guest{"guest_id"}else{"diner_id"}]=json!(if guest{"gu_80000000000000000000000000000009"}else{"dn_80000000000000000000000000000009"});
  let refused=call(json!({"op":"dispatch","state":state,"command":command(&state,week_edit(changed,true),0xee51+u32::from(guest)),"now":NOW}));assert_eq!(refused["ok"],false,"{refused}");
 }
 let mut forged=removed.clone();let last=forged["receipts"].as_array_mut().unwrap().last_mut().unwrap();last["command"]["command"]["data"]["week"]["cells"][4]["choice"]=json!("on_the_move");last["command_digest"]=json!(canonical_digest(&last["command"]).unwrap());
 let refused=call(json!({"op":"validate_state","state":forged,"now":NOW}));assert_eq!(refused["ok"],false);assert!(refused["error"]["issues"].to_string().contains("/receipts/retained_two_unit/response"),"{refused}");assert_eq!(cold(removed.clone()),removed);
}

#[test]
fn a_requested_target_projection_does_not_hide_a_second_stale_saved_meal(){
 let state=saved(true);let mut week=state["snapshot"]["weeks"][0].clone();let mut occasion=week["cells"][4]["occasion"].clone();occasion["occasion_id"]=json!("oc_80000000000000000000000000000006");occasion["guests"][0]["guest_id"]=json!("gu_80000000000000000000000000000006");week["cells"][7]["occasion"]=occasion;let state=edit(&state,week,0xee60,false);
 let state=select_slot(&state,&three_bean_two_unit_packet_ref().unwrap().recipe,7,0xee61);let mut week=state["snapshot"]["weeks"][0].clone();week["cells"][4]["occasion"]["guests"]=json!([]);week["cells"][7]["occasion"]["guests"]=json!([]);let removed=edit(&state,week,0xee62,false);let week=&removed["snapshot"]["weeks"][0];
 let refused=call(json!({"op":"board","state":removed,"week_id":week["range"]["week_id"],"slot":week["cells"][4]["slot"],"now":NOW}));assert_eq!(refused["ok"],false,"{refused}");assert_eq!(refused["error"]["code"],"foreign_reference");assert_eq!(removed["snapshot"]["weeks"][0]["plan"]["selections"],state["snapshot"]["weeks"][0]["plan"]["selections"]);assert_eq!(cold(removed.clone()),removed);
}

#[test]
fn booked_two_unit_guest_removal_preserves_original_order_and_history_but_refuses_current_calendar(){
 let state=book(&saved(true),false);let ordered=state["history"]["ordered"].clone();let order=state["snapshot"]["weeks"][0]["confirmed_order"].clone();let mut week=state["snapshot"]["weeks"][0].clone();week["cells"][4]["occasion"]["guests"]=json!([]);let removed=edit(&state,week,0xee70,false);
 assert_eq!(removed["snapshot"]["weeks"][0]["confirmed_order"],order);assert_eq!(removed["history"]["ordered"],ordered);assert_eq!(removed["snapshot"]["weeks"][0]["publication"],"needs_user");assert!(!removed["snapshot"]["conflicts"].as_array().unwrap().is_empty());
 let refused=call(json!({"op":"calendar_preview","state":removed,"order_id":order["order_id"],"destination_id":DEST,"now":NOW}));assert_eq!(refused["ok"],false,"{refused}");assert_eq!(cold(removed.clone()),removed);
}

#[test]
fn unconfirmed_guest_board_refuses_and_prior_day_or_changed_portion_requests_have_no_native_offer(){
 let state=ready(true);let mut week=state["snapshot"]["weeks"][0].clone();week["cells"][4]["occasion"]["guests"][0]["restrictions"]=json!({"kind":"unconfirmed","data":{}});
 let unconfirmed=edit(&state,week,0xee30,false);let week=&unconfirmed["snapshot"]["weeks"][0];
 let refused=call(json!({"op":"board","state":unconfirmed,"week_id":week["range"]["week_id"],"slot":week["cells"][4]["slot"],"now":NOW}));assert_eq!(refused["ok"],false);assert_eq!(refused["error"]["code"],"domain_conflict");assert!(refused["error"]["issues"].to_string().contains("/occasion/guests/restrictions"));assert_eq!(cold(unconfirmed.clone()),unconfirmed);
 assert!(board(&state,json!({"eat_by_time":"00:30:00","time_resolution":"unambiguous"}))["choices"].as_array().unwrap().is_empty());
 let person=json!({"kind":"guest","data":{"occasion_id":"oc_80000000000000000000000000000005","guest_id":"gu_80000000000000000000000000000005"}});
 assert!(board(&state,json!({"overrides":{"portions":[{"person":person,"preset":"regular","adjustment":{"numerator":"2","denominator":"1"}}],"eat_by":null}}))["choices"].as_array().unwrap().is_empty());
}

#[test]
fn two_unit_calendar_keeps_two_phase_once_full_inputs_two_people_t90_and_repeat_custody(){
 let state=book(&saved(true),false);let first=preview(&state);assert_eq!(first["batch"]["events"].as_array().unwrap().len(),2);let body=first["batch"]["events"].as_array().unwrap().iter().find(|event|event["body"].as_str().unwrap().contains("Reserve30minutes")).unwrap()["body"].as_str().unwrap();
 for text in ["at least60minutes passive refrigeration","11:00:00","Original storage clock","160 g","80 g","40 g","120 g","20 ml","30 ml","10 g","delay or refuse service","Requires phase 1"]{assert!(body.contains(text),"missing {text}: {body}");}assert!(!body.contains("90minutes of full attention"));
 let prepared=apply(&state,json!({"kind":"request_calendar_publication","data":{"order_id":state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],"destination_id":DEST,"event_batch_digest":first["event_batch_digest"]}}),0xee42);let again=preview(&prepared);assert_eq!(again["prepared"],true);assert_eq!(again["ics"],first["ics"]);assert_eq!(again["batch"],first["batch"]);assert_eq!(cold(prepared.clone()),prepared);assert_eq!(prepared["snapshot"]["weeks"][0]["confirmed_order"],state["snapshot"]["weeks"][0]["confirmed_order"]);
}

#[test]
fn delivery_after_first_handling_refuses_calendar_without_erasing_booked_order(){
 let state=book(&saved(true),true);let result=call(json!({"op":"calendar_preview","state":state,"order_id":state["snapshot"]["weeks"][0]["confirmed_order"]["order_id"],"destination_id":DEST,"now":NOW}));assert_eq!(result["ok"],false);assert_eq!(result["error"]["code"],"domain_conflict");assert!(result["error"]["issues"].to_string().contains("/delivery_end"));assert!(!state["snapshot"]["weeks"][0]["confirmed_order"].is_null());assert_eq!(cold(state.clone()),state);
}
