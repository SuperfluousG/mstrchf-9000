//! Synthetic browser-owner checks for the exact compiled portable food pair.
use mstrchf_browser::{maker_packets::available_packet_refs,run_json};
use serde_json::{Value,json};
const NOW:&str="2026-10-14T12:00:00Z";
const HOT:&str="rc_8000000000000000000000000000000d";
const COLD:&str="rc_8000000000000000000000000000000e";
fn call(input:Value)->Value{serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap()}
fn good(input:Value)->Value{let result=call(input);assert_eq!(result["ok"],true,"{result}");result["value"].clone()}
fn command(state:&Value,body:Value,id:u32)->Value{json!({"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body})}
fn apply(state:&Value,body:Value,id:u32)->Value{good(json!({"op":"dispatch","state":state,"command":command(state,body,id),"now":NOW}))["state"].clone()}
fn week_body(week:&Value)->Value{json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{"week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],"day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],"cells":week["cells"],"plan":week["plan"]}}})}
fn pin(id:&str)->Value{json!(available_packet_refs().unwrap().into_iter().find(|reference|reference.recipe.recipe_id.0==id).unwrap())}
fn admission(state:&Value,id:&str)->Value{json!({"kind":"admit_reviewed_recipe","data":{"packet":pin(id),"expected_catalogue_revision":(1+state["reviewed_packets"].as_array().map_or(0,Vec::len)).to_string()}})}
fn setup()->Value{
 let mut state=serde_json::from_slice::<Value>(include_bytes!("fixtures/legacy-engineering-receipts.json")).unwrap()["state"].clone();
 let mut profile=state["snapshot"]["profile"].clone();
 profile["diners"][0]["display_name"]=json!("Fictional portable-lunch resident");
 profile["diners"][0]["age_band"]=json!("adult");profile["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":NOW}});
 profile["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});
 profile["preferences"]["equipment"]=json!(["fridge","hob","oven"]);
 profile["preferences"]["mobility"]=json!({"reheating":false,"cold_storage":true,"temperature":"either"});
 state=apply(&state,json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),0xfe0001);
 let mut week=state["snapshot"]["weeks"][0].clone();week["plan"]["selections"]=json!([]);week["plan"]["batch_links"]=json!([]);week["day_enabled"]=json!(vec![true;7]);week["meal_type_enabled"]=json!({"breakfast":true,"lunch":true,"dinner":true});
 for cell in week["cells"].as_array_mut().unwrap(){cell["choice"]=json!("at_home");cell["attendee_ids"]=json!([profile["diners"][0]["diner_id"]]);cell["occasion"]=Value::Null;}
 week["cells"][7]["choice"]=json!("on_the_move");
 state=apply(&state,week_body(&week),0xfe0002);
 state=apply(&state,admission(&state,HOT),0xfe0003);apply(&state,admission(&state,COLD),0xfe0004)
}
fn raw_board(state:&Value,index:usize,options:Value)->Value{let week=&state["snapshot"]["weeks"][0];call(json!({"op":"board","state":state,"week_id":week["range"]["week_id"],"slot":week["cells"][index]["slot"],"options":options,"now":NOW}))}
fn board(state:&Value,index:usize,options:Value)->Value{let result=raw_board(state,index,options);assert_eq!(result["ok"],true,"{result}");result["value"].clone()}
fn selected(board:&Value,id:&str)->Value{board["choices"].as_array().unwrap().iter().find(|choice|choice["selection"]["recipe"]["recipe_id"]==id).unwrap_or_else(||panic!("Missing {id}: {board}")).clone()}
fn save(state:&Value,choice:&Value,id:u32)->Value{let mut week=state["snapshot"]["weeks"][0].clone();week["plan"]=choice["plan"].clone();apply(state,week_body(&week),id)}
fn dinner()->Value{let state=setup();let choice=selected(&board(&state,5,Value::Null),HOT);save(&state,&choice,0xfe0005)}
fn options(state:&Value)->Value{json!({"leftover_source":state["snapshot"]["weeks"][0]["cells"][5]["slot"],"portable_cold":{"packing_complete_time":"07:30:00","packing_resolution":"unambiguous","depart_time":"07:45:00","departure_resolution":"unambiguous","chilling_confirmed":true}})}
fn has_cold(result:&Value)->bool{result["ok"]==true&&result["value"]["choices"].as_array().unwrap().iter().any(|choice|choice["selection"]["recipe"]["recipe_id"]==COLD)}

#[test]
fn portable_owner_preview_saves_two_sessions_original_clock_and_cold_reloads(){
 let state=dinner();let choice=selected(&board(&state,7,options(&state)),COLD);
 let journey=&choice["selection"]["portable_cold"];assert_eq!(journey["packing_complete_at"]["local_time"],"07:30:00");assert_eq!(journey["depart_at"]["local_time"],"07:45:00");assert_eq!(journey["declaration"]["confirmed_at"],NOW);
 let saved=save(&state,&choice,0xfe0006);let after=board(&saved,7,Value::Null);let derived=&after["details"]["derived"];
 assert!(derived.is_object(),"{after}");assert_eq!(derived["demand"]["lines"].as_array().unwrap().len(),7);assert_eq!(derived["batches"].as_array().unwrap().len(),1);
 assert_eq!(derived["batches"][0]["components"][0]["allocated"]["amount"],json!({"numerator":"2","denominator":"1"}));
 let mut sessions=vec![];
 for session in derived["prep_sessions"].as_array().unwrap(){for task in session["tasks"].as_array().unwrap(){if task["recipe"]["recipe_id"]==COLD{sessions.push(session["session_id"].clone());if task["prep_id"]==1{assert_eq!(task["elapsed"]["start"]["local_time"],"07:25:00");assert_eq!(task["elapsed"]["end"]["local_time"],"07:30:00");}else{assert_eq!(task["prep_id"],2);}}}}
 assert_eq!(sessions.len(),2);assert_ne!(sessions[0],sessions[1]);
 assert_eq!(state["snapshot"]["profile"],saved["snapshot"]["profile"]);assert_eq!(state["snapshot"]["privacy"],saved["snapshot"]["privacy"]);
 assert_eq!(&state["snapshot"]["weeks"].as_array().unwrap()[1..],&saved["snapshot"]["weeks"].as_array().unwrap()[1..]);
 let retained=saved.clone();let reloaded=std::thread::spawn(move||good(json!({"op":"validate_state","state":retained,"now":NOW}))).join().unwrap();assert_eq!(reloaded,saved);
 assert_eq!(board(&reloaded,7,Value::Null)["details"]["derived"],*derived);
}
#[test]
fn owner_confirmation_and_ordering_are_required_for_each_new_portable_choice(){
 let state=dinner();assert!(!has_cold(&raw_board(&state,7,Value::Null)));
 let mut missing=options(&state);missing.as_object_mut().unwrap().remove("portable_cold");assert!(!has_cold(&raw_board(&state,7,missing)));
 for (path,value) in [("chilling_confirmed",json!(false)),("depart_time",json!("07:00:00")),("packing_complete_time",json!("18:00:00"))]{let mut changed=options(&state);changed["portable_cold"][path]=value;assert!(!has_cold(&raw_board(&state,7,changed)));}
 let mut changed=options(&state);changed["make_extra"]=json!(true);assert!(!has_cold(&raw_board(&state,7,changed)));
}
#[test]
fn saved_declaration_cannot_be_reassigned_to_changed_source_or_times(){
 let state=dinner();let choice=selected(&board(&state,7,options(&state)),COLD);
 let mut altered=choice.clone();altered["plan"]["selections"].as_array_mut().unwrap().iter_mut().find(|s|s["recipe"]["recipe_id"]==COLD).unwrap()["portable_cold"]["depart_at"]["instant"]=json!("2026-12-30T08:00:00Z");
 let mut week=state["snapshot"]["weeks"][0].clone();week["plan"]=altered["plan"].clone();let result=call(json!({"op":"dispatch","state":state,"command":command(&state,week_body(&week),0xfe0010),"now":NOW}));assert_eq!(result["ok"],false);
 let saved=save(&state,&choice,0xfe0011);let mut week=saved["snapshot"]["weeks"][0].clone();week["plan"]["selections"].as_array_mut().unwrap().iter_mut().find(|s|s["recipe"]["recipe_id"]==HOT).unwrap()["repeat_authorised"]=json!(true);
 let result=call(json!({"op":"dispatch","state":saved,"command":command(&saved,week_body(&week),0xfe0012),"now":NOW}));assert_eq!(result["ok"],false,"stale owner declaration must not be rewritten silently");
}
