//! Explicitly synthetic, bounded four-week local-planning checks.
use mstrchf_browser::{maker_packets::{porridge_packet_ref,chilli_batch_packet_ref,chilli_wraps_packet_ref}, run_json};
use mstrchf_contracts::canonical_digest;
use serde_json::{Value, json};
const NOW: &str = "2026-10-14T15:00:00Z";
fn call(value: Value) -> Value { serde_json::from_slice(&run_json(&serde_json::to_vec(&value).unwrap())).unwrap() }
fn good(value: Value) -> Value { let out=call(value);assert_eq!(out["ok"],true,"{out}");out["value"].clone() }
fn envelope(state: &Value, command: Value, id: u32) -> Value { json!({"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":command}) }
fn apply(state: &Value, command: Value, id: u32) -> Value { good(json!({"op":"dispatch","state":state,"command":envelope(state,command,id),"now":NOW}))["state"].clone() }
fn edit(state: &Value, week: &Value, id: u32) -> Value { apply(state,json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{"week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],"day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],"cells":week["cells"],"plan":week["plan"]}}}),id) }
fn horizon(state: &Value, mode: &str, repeats: bool) -> Value { json!({"kind":"suggest_planning_horizon","data":{"ordered_week_ids":state["snapshot"]["weeks"].as_array().unwrap().iter().map(|w|w["range"]["week_id"].clone()).collect::<Vec<_>>(),"mode":mode,"allow_repeats":repeats}}) }
fn board(state: &Value, wi: usize, si: usize) -> Value {let w=&state["snapshot"]["weeks"][wi];good(json!({"op":"board","state":state,"week_id":w["range"]["week_id"],"slot":w["cells"][si]["slot"],"now":NOW}))}
fn choose(state: &Value, wi: usize, si: usize, id: u32) -> Value {let b=board(state,wi,si);assert!(!b["choices"].as_array().unwrap().is_empty(),"{b}");let mut w=state["snapshot"]["weeks"][wi].clone();w["plan"]=b["choices"][0]["plan"].clone();edit(state,&w,id)}
fn base() -> Value {
 let mut state=good(json!({"op":"initialize","household_id":"hh_41000000000000000000000000000001","diner_id":"dn_41000000000000000000000000000001","zone":"Europe/London","today":"2026-10-14","now":NOW,"demo":true}));
 for index in 0..4 {let mut week=state["snapshot"]["weeks"][index].clone();week["day_enabled"]=json!([true,index==0,false,false,false,false,false]);week["meal_type_enabled"]=json!({"breakfast":false,"lunch":false,"dinner":true});week["plan"]["selections"]=json!([]);week["plan"]["batch_links"]=json!([]);state=edit(&state,&week,0x4100+index as u32);}
 choose(&state,0,2,0x4110)
}
fn cold(state: &Value) -> Value {let state=state.clone();std::thread::spawn(move||good(json!({"op":"validate_state","state":state,"now":"2026-10-15T15:00:00Z"}))).join().unwrap()}
fn assert_adjacent_distinct(state:&Value) {let weeks=state["snapshot"]["weeks"].as_array().unwrap();for pair in weeks.windows(2){for a in pair[0]["plan"]["selections"].as_array().unwrap(){for b in pair[1]["plan"]["selections"].as_array().unwrap(){if a["slot"]["meal_type"]=="dinner"&&b["slot"]["meal_type"]=="dinner" {assert_ne!(a["recipe"]["recipe_id"],b["recipe"]["recipe_id"]);}}}}}

#[test]
fn four_week_draft_is_one_receipt_with_fixed_food_and_cold_history_preserved() {
 let before=base();let command=envelope(&before,horizon(&before,"fill_open",false),0x4120);
 let result=good(json!({"op":"dispatch","state":before,"command":command,"now":NOW}));let after=&result["state"];
 assert_eq!(after["receipts"].as_array().unwrap().len(),before["receipts"].as_array().unwrap().len()+1);
 assert_eq!(after["snapshot"]["snapshot_revision"].as_str().unwrap().parse::<u64>().unwrap(),before["snapshot"]["snapshot_revision"].as_str().unwrap().parse::<u64>().unwrap()+1);
 assert_eq!(after["snapshot"]["privacy"],before["snapshot"]["privacy"]);assert_eq!(after["snapshot"]["privacy"]["use_ai_features"],false);
 let fixed=&before["snapshot"]["weeks"][0]["plan"]["selections"][0];assert!(after["snapshot"]["weeks"][0]["plan"]["selections"].as_array().unwrap().contains(fixed));
 for index in 0..4 {let old=&before["snapshot"]["weeks"][index];let next=&after["snapshot"]["weeks"][index];for field in ["range","defaults_snapshot","cells","day_enabled","meal_type_enabled"]{assert_eq!(next[field],old[field]);}assert!(!next["plan"]["selections"].as_array().unwrap().is_empty());}
 assert_adjacent_distinct(after);assert_eq!(cold(after),*after);
 if let Ok(directory)=std::env::var("MSTRCHF_HORIZON_FIXTURE_DIR") {std::fs::create_dir_all(&directory).unwrap();for (name,value) in [("horizon-baseline.json",&before),("horizon-command.json",&command),("horizon-expected.json",after)] {std::fs::write(std::path::Path::new(&directory).join(name),serde_json::to_vec_pretty(value).unwrap()).unwrap();}}
}

#[test]
fn horizon_identity_and_mode_are_exact_and_rejected_before_any_state_return() {
 let state=base();let proper=horizon(&state,"fill_open",false);let ids=proper["data"]["ordered_week_ids"].as_array().unwrap();
 let mut variants=vec![];
 let mut reversed=proper.clone();reversed["data"]["ordered_week_ids"]=json!(ids.iter().rev().collect::<Vec<_>>());variants.push(reversed);
 let mut duplicate=proper.clone();duplicate["data"]["ordered_week_ids"][1]=ids[0].clone();variants.push(duplicate);
 let mut foreign=proper.clone();foreign["data"]["ordered_week_ids"][3]=json!("wk_ffffffffffffffffffffffffffffffff");variants.push(foreign);
 let mut missing=proper.clone();missing["data"]["ordered_week_ids"]=json!(&ids[..3]);variants.push(missing);
 let mut swap=proper.clone();swap["data"]["mode"]=json!("swap_named");variants.push(swap);
 for (index,command) in variants.into_iter().enumerate(){let out=call(json!({"op":"dispatch","state":state,"command":envelope(&state,command,0x4200+index as u32),"now":NOW}));assert_eq!(out["ok"],false,"{out}");assert!(out.get("value").is_none());}
 assert_eq!(cold(&state),state);
}

#[test]
fn fill_open_all_no_change_refuses_and_duplicate_stale_conflicting_commands_are_guarded() {
 let before=base();let command=envelope(&before,horizon(&before,"fill_open",false),0x4300);let after=good(json!({"op":"dispatch","state":before,"command":command,"now":NOW}))["state"].clone();
 let replay=good(json!({"op":"dispatch","state":after,"command":command,"now":NOW}));assert_eq!(replay["receipt"]["disposition"],"replayed");assert_eq!(replay["state"],after);
 let out=call(json!({"op":"dispatch","state":after,"command":envelope(&after,horizon(&after,"fill_open",false),0x4301),"now":NOW}));assert_eq!(out["error"]["code"],"domain_conflict");assert_eq!(out["error"]["issues"][0]["path"],"/suggest_horizon/no_fit_or_permission");assert!(out.get("value").is_none());
 let mut stale=command.clone();stale["command_id"]=json!("cm_43000000000000000000000000000002");assert_eq!(call(json!({"op":"dispatch","state":after,"command":stale,"now":NOW}))["error"]["code"],"context_stale");
 let mut changed=command.clone();changed["command"]["data"]["allow_repeats"]=json!(true);assert_eq!(call(json!({"op":"dispatch","state":after,"command":changed,"now":NOW}))["error"]["code"],"replay_conflict");
 assert_eq!(cold(&after),after);
}

#[test]
fn later_fixed_draft_is_consulted_and_explicit_repeats_do_not_replace_fixed_food() {
 let state=base();let mut first=state["snapshot"]["weeks"][0].clone();first["plan"]["selections"]=json!([]);let state=edit(&state,&first,0x4490);let state=choose(&state,1,2,0x4400);let fixed=state["snapshot"]["weeks"][1].clone();
 let normal=apply(&state,horizon(&state,"fill_open",false),0x4401);assert_eq!(normal["snapshot"]["weeks"][1],fixed);assert_adjacent_distinct(&normal);
 let repeats=apply(&state,horizon(&state,"fill_open",true),0x4402);assert_eq!(repeats["snapshot"]["weeks"][1],fixed);
 for week in repeats["snapshot"]["weeks"].as_array().unwrap(){for selection in week["plan"]["selections"].as_array().unwrap(){if selection["origin"]=="suggested"{assert_eq!(selection["repeat_authorised"],true);}}}
 assert_eq!(cold(&normal),normal);assert_eq!(cold(&repeats),repeats);
}

#[test]
fn hidden_recipe_exclusion_can_leave_one_week_unchanged_without_abandoning_other_weeks() {
 let state=base();let mut unavailable=state["snapshot"]["weeks"][1].clone();unavailable["day_enabled"]=json!(vec![false;7]);let state=edit(&state,&unavailable,0x4590);let fixed=state["snapshot"]["weeks"][0]["plan"]["selections"][0].clone();
 let feedback=json!({"kind":"record_feedback","data":{"feedback":{"slot":fixed["slot"],"recipe":fixed["recipe"],"rating":"not_for_me","reasons":[],"hide":true,"portion_change":null,"portion_change_accepted":false,"recorded_at":NOW}}});
 let state=apply(&state,feedback,0x4500);let result=apply(&state,horizon(&state,"fill_open",false),0x4501);
 assert_eq!(result["snapshot"]["weeks"][1],state["snapshot"]["weeks"][1]);
 assert_eq!(result["history"]["hidden_recipe_ids"],state["history"]["hidden_recipe_ids"]);assert_eq!(result["history"]["feedback"],state["history"]["feedback"]);
 for week in result["snapshot"]["weeks"].as_array().unwrap(){for selection in week["plan"]["selections"].as_array().unwrap(){if selection["origin"]=="suggested" {assert_ne!(selection["recipe"]["recipe_id"],fixed["recipe"]["recipe_id"]);}}}
 assert_eq!(cold(&result),result);
}

fn real_breakfasts() -> Value {
 let mut state=base();let mut profile=state["snapshot"]["profile"].clone();profile["diners"][0]["age_band"]=json!("adult");profile["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":NOW}});profile["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});profile["preferences"]["equipment"]=json!(["hob"]);
 state=apply(&state,json!({"kind":"update_profile","data":{"profile":profile,"expected_profile_revision":profile["revision"]}}),0x4600);
 for index in 0..4 {let mut week=state["snapshot"]["weeks"][index].clone();week["day_enabled"]=json!([true,index==0,false,false,false,false,false]);week["meal_type_enabled"]=json!({"breakfast":true,"lunch":false,"dinner":false});week["plan"]["selections"]=json!([]);week["plan"]["batch_links"]=json!([]);state=edit(&state,&week,0x4610+index as u32);}
 state=apply(&state,json!({"kind":"admit_reviewed_recipe","data":{"packet":porridge_packet_ref().unwrap(),"expected_catalogue_revision":"1"}}),0x4620);
 for index in 0..2 {let b=board(&state,index,0);let recipe=json!(porridge_packet_ref().unwrap().recipe);let chosen=b["choices"].as_array().unwrap().iter().find(|c|c["selection"]["recipe"]==recipe).unwrap();let mut w=state["snapshot"]["weeks"][index].clone();w["plan"]=chosen["plan"].clone();state=edit(&state,&w,0x4630+index as u32);}
 for index in 0..2 {let b=board(&state,index,0);let w=&state["snapshot"]["weeks"][index];state=apply(&state,json!({"kind":"approve_plan","data":{"week_id":w["range"]["week_id"],"plan_revision":w["plan"]["revision"],"derived_digest":b["details"]["derived"]["digest"]}}),0x4640+index as u32);}
 let week=&state["snapshot"]["weeks"][0];let date=week["range"]["monday"].as_str().unwrap();let moment=|time:&str|good(json!({"op":"resolve_moment","date":date,"time":time,"zone":"Europe/London","resolution":"unambiguous"}));
 apply(&state,json!({"kind":"confirm_order","data":{"confirmation":{"week_id":week["range"]["week_id"],"approval_id":week["approved_plan"]["approval_id"],"delivery_start":moment("06:00:00"),"delivery_end":moment("07:00:00"),"confirmed_at":NOW,"source":{"kind":"user_confirmation","data":{}}}}}),0x4650)
}

#[test]
fn breakfasts_repeat_and_changed_booked_week_keeps_order_while_untouched_approval_is_exact() {
 let before=real_breakfasts();let after=apply(&before,horizon(&before,"fill_open",false),0x4660);
 assert_eq!(after["snapshot"]["weeks"][0]["confirmed_order"],before["snapshot"]["weeks"][0]["confirmed_order"]);assert_eq!(after["history"]["ordered"],before["history"]["ordered"]);assert_eq!(after["snapshot"]["weeks"][0]["publication"],"needs_user");assert!(after["snapshot"]["weeks"][0]["approved_plan"].is_null());
 assert_eq!(after["snapshot"]["weeks"][1],before["snapshot"]["weeks"][1]);assert!(after["snapshot"]["weeks"][1]["approved_plan"].is_object());
 for index in 0..4 {assert!(!after["snapshot"]["weeks"][index]["plan"]["selections"].as_array().unwrap().is_empty());}assert_eq!(cold(&after),after);
}

#[test]
fn resigned_horizon_receipt_cannot_change_unrequested_cell_declarations() {
 let before=base();let mut after=apply(&before,horizon(&before,"fill_open",false),0x4700);
 let last=after["receipts"].as_array().unwrap().len()-1;after["receipts"][last]["response"]["weeks"][0]["cells"][0]["choice"]=json!("skip");
 after["receipts"][last]["response_digest"]=json!(canonical_digest(&after["receipts"][last]["response"]).unwrap().0);
 after["snapshot"]=after["receipts"][last]["response"].clone();let week=after["snapshot"]["weeks"][0].clone();for retained in after["retained_weeks"].as_array_mut().unwrap(){if retained["range"]["week_id"]==week["range"]["week_id"]{*retained=week.clone();}}
 let result=std::thread::spawn(move||call(json!({"op":"validate_state","state":after,"now":NOW}))).join().unwrap();assert_eq!(result["ok"],false);assert_eq!(result["error"]["issues"][0]["path"],"/receipts/suggest_horizon_response");
}

#[test]
fn replacing_suggestions_keeps_fixed_food_and_later_evidence_cannot_rewrite_old_horizon() {
 let before=base();let fixed=before["snapshot"]["weeks"][0]["plan"]["selections"][0].clone();let filled=apply(&before,horizon(&before,"fill_open",false),0x4750);
 let replaced=apply(&filled,horizon(&filled,"replace_suggestions",true),0x4751);assert!(replaced["snapshot"]["weeks"][0]["plan"]["selections"].as_array().unwrap().contains(&fixed));
 let admitted=apply(&replaced,json!({"kind":"admit_reviewed_recipe","data":{"packet":porridge_packet_ref().unwrap(),"expected_catalogue_revision":"1"}}),0x4752);
 let selection=&admitted["snapshot"]["weeks"][1]["plan"]["selections"][0];
 let later=apply(&admitted,json!({"kind":"record_feedback","data":{"feedback":{"slot":selection["slot"],"recipe":selection["recipe"],"rating":"not_for_me","reasons":[],"hide":true,"portion_change":null,"portion_change_accepted":false,"recorded_at":NOW}}}),0x4753);
 assert_eq!(later["receipts"].as_array().unwrap()[..replaced["receipts"].as_array().unwrap().len()],replaced["receipts"].as_array().unwrap()[..]);assert_eq!(cold(&later),later);
}

#[test]
fn intentional_chilli_source_and_leftover_links_and_ordered_dinner_survive_horizon() {
 let mut state=base();let mut profile=state["snapshot"]["profile"].clone();profile["diners"][0]["age_band"]=json!("adult");profile["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":NOW}});profile["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});profile["preferences"]["equipment"]=json!(["fridge","hob","oven"]);profile["preferences"]["mobility"]=json!({"temperature":"cold","reheating":false,"cold_storage":false});
 state=apply(&state,json!({"kind":"update_profile","data":{"profile":profile,"expected_profile_revision":profile["revision"]}}),0x4900);
 let mut week=state["snapshot"]["weeks"][0].clone();week["plan"]["selections"]=json!([]);week["plan"]["batch_links"]=json!([]);week["meal_type_enabled"]["lunch"]=json!(true);for index in [1,2,4,5]{week["cells"][index]["choice"]=json!(if index==1 {"skip"}else{"at_home"});}state=edit(&state,&week,0x4901);
 for (index,packet) in [chilli_batch_packet_ref().unwrap(),chilli_wraps_packet_ref().unwrap()].into_iter().enumerate(){state=apply(&state,json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":(index+1).to_string()}}),0x4902+index as u32);}
 for (index,packet) in [(2,chilli_batch_packet_ref().unwrap()),(4,chilli_wraps_packet_ref().unwrap())] {let week=&state["snapshot"]["weeks"][0];let options=if index==4 {json!({"leftover_source":week["cells"][2]["slot"]})}else{Value::Null};let b=good(json!({"op":"board","state":state,"week_id":week["range"]["week_id"],"slot":week["cells"][index]["slot"],"options":options,"now":NOW}));let chosen=b["choices"].as_array().unwrap().iter().find(|c|c["selection"]["recipe"]==json!(packet.recipe)).unwrap_or_else(||panic!("Missing retained chilli choice; {b}"));let mut week=week.clone();week["plan"]=chosen["plan"].clone();state=edit(&state,&week,0x4910+index as u32);}
 let b=board(&state,0,2);let week=&state["snapshot"]["weeks"][0];state=apply(&state,json!({"kind":"approve_plan","data":{"week_id":week["range"]["week_id"],"plan_revision":week["plan"]["revision"],"derived_digest":b["details"]["derived"]["digest"]}}),0x4920);
 let week=&state["snapshot"]["weeks"][0];let date=week["range"]["monday"].as_str().unwrap();let moment=|time:&str|good(json!({"op":"resolve_moment","date":date,"time":time,"zone":"Europe/London","resolution":"unambiguous"}));state=apply(&state,json!({"kind":"confirm_order","data":{"confirmation":{"week_id":week["range"]["week_id"],"approval_id":week["approved_plan"]["approval_id"],"delivery_start":moment("06:00:00"),"delivery_end":moment("07:00:00"),"confirmed_at":NOW,"source":{"kind":"user_confirmation","data":{}}}}}),0x4921);
 assert!(!state["snapshot"]["weeks"][0]["plan"]["batch_links"].as_array().unwrap().is_empty());assert!(state["history"]["ordered"].as_array().unwrap().iter().any(|o|o["selection"]["slot"]["meal_type"]=="dinner"));
 let after=apply(&state,horizon(&state,"fill_open",false),0x4922);let old=&state["snapshot"]["weeks"][0];let next=&after["snapshot"]["weeks"][0];assert_eq!(next["plan"]["batch_links"],old["plan"]["batch_links"]);for fixed in old["plan"]["selections"].as_array().unwrap(){assert!(next["plan"]["selections"].as_array().unwrap().contains(fixed));}assert_eq!(next["confirmed_order"],old["confirmed_order"]);assert_eq!(after["history"]["ordered"],state["history"]["ordered"]);assert_eq!(cold(&after),after);
}

#[test]
#[ignore = "bounded explicit timing observation, not a routine regression"]
fn measure_eighty_four_active_slots_and_cold_horizon_audit() {
 let mut state=good(json!({"op":"initialize","household_id":"hh_41000000000000000000000000000002","diner_id":"dn_41000000000000000000000000000002","zone":"Europe/London","today":"2026-10-14","now":NOW,"demo":true}));
 for index in 0..4 {let mut week=state["snapshot"]["weeks"][index].clone();week["day_enabled"]=json!([true,true,true,true,true,true,true]);week["meal_type_enabled"]=json!({"breakfast":true,"lunch":true,"dinner":true});for cell in week["cells"].as_array_mut().unwrap(){cell["choice"]=json!("at_home");}state=edit(&state,&week,0x4800+index as u32);}
 let start=std::time::Instant::now();let after=apply(&state,horizon(&state,"fill_open",false),0x4810);let dispatch_ms=start.elapsed().as_millis();
 let start=std::time::Instant::now();assert_eq!(cold(&after),after);let cold_audit_ms=start.elapsed().as_millis();
 let selected:usize=after["snapshot"]["weeks"].as_array().unwrap().iter().map(|w|w["plan"]["selections"].as_array().unwrap().len()).sum();
 println!("FICTIONAL84 timing: dispatch_ms={dispatch_ms}, cold_audit_ms={cold_audit_ms}, selected={selected}; compiled engineering catalogue only; debug_assertions={}",cfg!(debug_assertions));
}
