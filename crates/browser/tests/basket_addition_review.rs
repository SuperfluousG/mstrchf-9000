//! Actual native boundary, real compiled recipes/commands, synthetic people,
//! source facts and external-chat decisions. No Tesco observation, user witness,
//! human calibration, admitted product, current price or external action.
use mstrchf_browser::{run_json,maker_packets::*,product_semantic_review,basket_addition_review::{self,PreparedBasketAdditionReview}};
use serde_json::{Value,json};
const CREATED:&str="2026-10-13T10:00:00Z";
const OBSERVED:&str="2026-10-13T10:00:01Z";
const NOW:&str="2026-10-13T10:05:00Z";
const LATER:&str="2026-10-13T10:15:00Z";
const DEADLINE:&str="2026-10-13T11:00:00Z";
fn call(v:Value)->Value{serde_json::from_slice(&run_json(&serde_json::to_vec(&v).unwrap())).unwrap()}
fn good(v:Value)->Value{let r=call(v);assert_eq!(r["ok"],true,"{r}");r["value"].clone()}
fn refused(v:Value)->Value{let r=call(v);assert_eq!(r["ok"],false,"{r}");r["error"].clone()}
fn apply(s:&Value,body:Value,id:u32)->Value{good(json!({"op":"dispatch","state":s,"now":CREATED,"command":{
 "schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":s["snapshot"]["snapshot_revision"],"command":body}}))["state"].clone()}
fn save_week(w:&Value)->Value{json!({"kind":"save_week","data":{"expected_week_revision":w["revision"],"week":{
 "week_id":w["range"]["week_id"],"defaults_snapshot":w["defaults_snapshot"],"day_enabled":w["day_enabled"],"meal_type_enabled":w["meal_type_enabled"],"cells":w["cells"],"plan":w["plan"]}}})}
fn setup(two:bool,vegetarian:bool)->Value{
 let mut s=good(json!({"op":"initialize","household_id":"hh_800000000000000000000000000000b0","diner_id":"dn_800000000000000000000000000000b0",
  "zone":"Europe/London","today":"2026-10-13","now":CREATED,"demo":true}));
 let mut p=s["snapshot"]["profile"].clone();p["diners"][0]["age_band"]=json!("adult");p["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":CREATED}});
 p["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});p["preferences"]["equipment"]=json!(["fridge","hob"]);
 if two{let mut second=p["diners"][0].clone();second["diner_id"]=json!("dn_800000000000000000000000000000b1");if vegetarian{second["restrictions"]=json!({"kind":"confirmed","data":{"confirmed_at":CREATED,"requirements":[{"kind":"diet","data":{"code":"vegetarian"}}]}});}p["diners"].as_array_mut().unwrap().push(second);}
 s=apply(&s,json!({"kind":"update_profile","data":{"expected_profile_revision":p["revision"],"profile":p}}),0xeb00);
 let mut w=s["snapshot"]["weeks"][0].clone();w["meal_type_enabled"]=json!({"breakfast":true,"lunch":true,"dinner":true});w["plan"]["selections"]=json!([]);w["plan"]["batch_links"]=json!([]);
 for c in w["cells"].as_array_mut().unwrap(){c["choice"]=json!("at_home");}if two{w["cells"][4]["attendee_ids"].as_array_mut().unwrap().push(json!("dn_800000000000000000000000000000b1"));}
 apply(&s,save_week(&w),0xeb01)
}
fn food(cold:bool,vegetarian:bool)->Value{
 let mut s=setup(vegetarian,vegetarian);let packet=if vegetarian{three_bean_two_unit_vegetarian_packet_ref().unwrap()}else if cold{three_bean_packet_ref().unwrap()}else{porridge_packet_ref().unwrap()};
 s=apply(&s,json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":"1"}}),0xeb02);
 let w=&s["snapshot"]["weeks"][0];let index=if cold{4}else{3};let b=good(json!({"op":"board","state":s,"week_id":w["range"]["week_id"],"slot":w["cells"][index]["slot"],"options":null,"now":CREATED}));
 let choice=b["choices"].as_array().unwrap().iter().find(|v|v["selection"]["recipe"]==json!(packet.recipe)).unwrap();let mut w=w.clone();w["plan"]=choice["plan"].clone();apply(&s,save_week(&w),0xeb03)
}
fn product()->Value{json!({"retailer":"tesco","product_id":"123456789","variant_id":null})}
fn known(v:Value)->Value{json!({"kind":"known","data":{"value":v}})}
fn unknown()->Value{json!({"kind":"unknown","data":{"reason":"not_exposed"}})}
fn quantity(n:&str,unit:&str,form:&str)->Value{json!({"amount":{"numerator":n,"denominator":"1"},"unit":unit,"form":form})}
fn line(s:&Value,suffix:&str)->Value{let c=good(json!({"op":"product_match_context","state":s,"week_id":s["snapshot"]["weeks"][0]["range"]["week_id"],"now":CREATED}));c["lines"].as_array().unwrap().iter().find(|l|l["demand"]["ingredient_id"]==format!("ig_800000000000000000000000000000{suffix}")).unwrap()["demand"]["line_id"].clone()}
fn prepare(s:&Value,suffix:&str)->Value{good(json!({"op":"basket_addition_prepare","state":s,"week_id":s["snapshot"]["weeks"][0]["range"]["week_id"],
 "reader":{"provider":"codex_local","native_task_ref":"synthetic-independent-external-reader"},"key":{"retailer":"tesco","account_ref":"synthetic-declared-account","basket_id":"bk_800000000000000000000000000000b0"},
 "delivery":{"retailer":"tesco","account_ref":"synthetic-declared-account","context_ref":"synthetic-declared-delivery","delivery_date":null,"slot_ref":null},
 "target":{"product":product(),"url":"https://www.tesco.com/shop/en-GB/products/123456789"},"line_id":line(s,suffix),
 "job_id":"jb_800000000000000000000000000000b0","read_id":"rd_800000000000000000000000000000b0","review_id":"sr_800000000000000000000000000000b0","now":CREATED,"deadline":DEADLINE}))}
fn observation(p:&Value,net:Value,drained:Value,storage:Value)->Value{let source=&p["source_prepared"];json!({"schema_version":1,"observation_id":"ob_800000000000000000000000000000b0",
 "read_id":source["request"]["scope"]["data"]["read_scope"]["read_id"],"job_id":source["request"]["job_id"],"input_digest":source["request"]["input_digest"],
 "reader":source["expected_reader"],"resource":source["request"]["scope"]["data"]["read_scope"]["resource"],"observed_at":OBSERVED,
 "body":{"kind":"products","data":{"delivery_context":source["delivery"],"items":[{"product":product(),"url":"https://www.tesco.com/shop/en-GB/products/123456789",
 "title":"Synthetic external product; no actual source observation","pack":{"count":known(json!(1)),"net":net,"drained":drained},
 "price":{"kind":"unknown","data":{"reason":"not_exposed"}},"offers":[],"availability":"unknown","ingredients":known(json!("Synthetic exact plain input label <script>inert()</script>")),"allergens":unknown(),"storage":storage,"delivery_context":source["delivery"],"observed_at":OBSERVED}]}}})}
fn semantic(s:&Value,p:&Value,o:&Value)->Value{good(json!({"op":"basket_addition_check_source","state":s,"prepared":p,"observation":o,"now":NOW}))}
fn returned(q:&Value,state:&str,cold:bool)->Value{let request=&q["semantic_prepared"]["request"];let identity=json!({"state":state,"citations":if state=="unknown"{json!([])}else{json!([{"field":"ingredients","quote":request["product"]["ingredients"]["data"]["value"]}])},"reasoning":"Synthetic external automated inference, conditional on the literal source facts; no actual compliance was witnessed."});
 let mut cooking=identity.clone();if cold&&state!="unknown"{cooking["citations"].as_array_mut().unwrap().push(json!({"field":"storage","quote":request["product"]["storage"]["data"]["value"]}));}
 json!({"format":"mstrchf-product-semantic-return","version":1,"request_id":request["request_id"],"input_digest":request["input_digest"],"identity":identity,
  "cooking":request["methods"].as_array().unwrap().iter().map(|m|json!({"method_handle":m["method_handle"],"decision":cooking})).collect::<Vec<_>>()})
}
fn check(s:&Value,q:&Value,r:&Value,now:&str)->Value{call(json!({"op":"basket_addition_check_semantic","state":s,"prepared":q,"returned":r,"now":now}))}
fn oats()->(Value,Value,Value,Value){let s=food(false,false);let p=prepare(&s,"01");let o=observation(&p,known(quantity("500","g","dry")),unknown(),unknown());let q=semantic(&s,&p,&o);(s,p,o,q)}
fn forbid_private(v:&Value){match v{Value::Object(m)=>for(k,v)in m{assert!(!["person","diner_id","slot","extra_id","occurrence","occurrences","context","context_stamp","week_id","line_id","demand","uses","origins","household_id","account_ref","delivery_context","reader","native_task_ref","accepted_ref","source_prepared"].contains(&k.as_str()),"private public field {k}");forbid_private(v)},Value::Array(items)=>for v in items{forbid_private(v)},_=>{}}}
#[test]
fn ai_off_draft_uses_actual_core_without_a_human_witness_and_changes_no_state(){
 let(s,_,_,q)=oats();let original=s.clone();let r=returned(&q,"supported",false);let out=check(&s,&q,&r,NOW);assert_eq!(out["ok"],true,"{out}");let v=&out["value"];
 assert_eq!(s["snapshot"]["privacy"]["use_ai_features"],false);assert_eq!(s["snapshot"]["privacy"]["include_household_requirements"],false);
 assert_eq!(v["source_status"],"source_witness_pending");assert_eq!(v["provenance"],"external_chat_proposal");assert_eq!(v["qualification"]["status"],"conditional_match");
 assert_eq!(v["qualification"]["pack_basis"],json!({"required":quantity("45","g","dry"),"supply":quantity("500","g","dry")}));
 assert_eq!(v["qualification"]["admission_status"],"missing");assert_eq!(v["qualification"]["freshness_status"],"unqualified");assert_eq!(v["qualification"]["recheck_at"],DEADLINE);
 for field in ["source_truth_assessed","account_verified","human_review_recorded","effect_enabled","reusable_for_intent"]{assert_eq!(v[field],false);}
 for field in ["accepted","admission","policy","match_basis","match_basis_digest","intent","grant_token"]{assert!(v.get(field).is_none());assert!(v["qualification"].get(field).is_none());}
 assert_eq!(good(json!({"op":"validate_state","state":s,"now":LATER})),original);
}
#[test]
fn public_methods_are_minimised_and_old_assisted_endpoints_keep_ai_off_refusals(){
 let(s,p,o,q)=oats();forbid_private(&q["semantic_prepared"]["request"]);assert_eq!(q["semantic_prepared"]["binding"]["draft"]["source"]["provenance"],"external_chat_unwitnessed_correlation");
 for op in ["product_source_inspect","product_source_check_return","product_source_accept"]{
  let mut input=json!({"op":op,"state":s,"prepared":p["source_prepared"],"now":NOW});if op!="product_source_inspect"{input["observation"]=o.clone();}if op=="product_source_accept"{input["evidence_id"]=json!("ev_800000000000000000000000000000b0");}assert_eq!(refused(input)["code"],"authority_denied");
 }
 assert_eq!(refused(json!({"op":"product_semantic_inspect","state":s,"prepared":q["semantic_prepared"],"now":NOW}))["code"],"authority_denied");
 let source=&p["source_prepared"];let legacy=good(json!({"op":"product_source_prepare","state":s,"week_id":source["request"]["week_id"],"reader":source["expected_reader"],"key":source["request"]["scope"]["data"]["key"],"delivery":source["delivery"],"targets":source["targets"],"job_id":source["request"]["job_id"],"read_id":source["request"]["scope"]["data"]["read_scope"]["read_id"],"now":CREATED,"deadline":DEADLINE}));assert_eq!(legacy,*source);
}
#[test]
fn source_refuses_foreign_actor_target_clock_and_transported_authority(){
 let(s,p,o,_)=oats();for edit in 0..5{let mut bad=o.clone();match edit{0=>bad["reader"]["native_task_ref"]=json!("foreign-source-reader"),1=>bad["body"]["data"]["items"][0]["url"]=json!("https://www.tesco.com/shop/en-GB/products/987654321"),2=>bad["observed_at"]=json!(CREATED),3=>bad["body"]["data"]["items"][0]["observed_at"]=json!("2026-10-13T10:06:00Z"),_=>bad["job_id"]=json!("jb_800000000000000000000000000000b1")};refused(json!({"op":"basket_addition_check_source","state":s,"prepared":p,"observation":bad,"now":NOW}));}
 refused(json!({"op":"basket_addition_check_source","state":s,"prepared":p,"observation":o,"accepted":true,"now":NOW}));
}
#[test]
fn unknown_refuted_and_changed_recipe_decisions_remain_native_unresolved_or_unsuitable(){
 let(s,_,_,q)=oats();let unknown=check(&s,&q,&returned(&q,"unknown",false),NOW);assert_eq!(unknown["ok"],true,"{unknown}");assert_eq!(unknown["value"]["qualification"]["status"],"unresolved");assert_eq!(unknown["value"]["qualification"]["pack_basis"],Value::Null);
 let refuted=check(&s,&q,&returned(&q,"refuted",false),NOW);assert_eq!(refuted["ok"],true,"{refuted}");assert_eq!(refuted["value"]["qualification"]["native_assessment"],"unsuitable");assert_eq!(refuted["value"]["qualification"]["pack_basis"],Value::Null);
 let mut r=returned(&q,"supported",false);r["cooking"][0]["decision"]["state"]=json!("refuted");let out=check(&s,&q,&r,NOW);assert_eq!(out["value"]["qualification"]["native_assessment"],"unresolved");assert_eq!(out["value"]["qualification"]["reasons"],json!(["changed_cooking"]));assert_eq!(out["value"]["qualification"]["pack_basis"],Value::Null);
}
#[test]
fn exact_citations_complete_methods_and_internal_pins_cannot_be_replaced(){
 let(s,_,_,q)=oats();for edit in 0..4{let mut r=returned(&q,"supported",false);match edit{0=>r["identity"]["citations"][0]["quote"]=json!("Not the exact label"),1=>r["cooking"]=json!([]),2=>r["input_digest"]=json!("f".repeat(64)),_=>r["admission"]=json!(true)}assert_eq!(check(&s,&q,&r,NOW)["ok"],false);}
 let mut forged=q.clone();forged["semantic_prepared"]["request"]["product"]["ingredients"]=known(json!("Forged source text"));let typed:product_semantic_review::PreparedProductSemanticReview=serde_json::from_value(forged["semantic_prepared"].clone()).unwrap();forged["semantic_prepared"]["prepared_digest"]=json!(product_semantic_review::prepared_digest(&typed).unwrap());let typed:PreparedBasketAdditionReview=serde_json::from_value(forged.clone()).unwrap();forged["prepared_digest"]=json!(basket_addition_review::review_digest(&typed).unwrap());assert_eq!(check(&s,&forged,&returned(&q,"supported",false),NOW)["ok"],false);
}
#[test]
fn original_deadline_and_current_privacy_food_are_not_renewed_by_later_preview(){
 let(s,_,_,q)=oats();let r=returned(&q,"supported",false);let later=check(&s,&q,&r,LATER);assert_eq!(later["ok"],true,"{later}");assert_eq!(later["value"]["qualification"]["recheck_at"],DEADLINE);assert_eq!(check(&s,&q,&r,DEADLINE)["ok"],false);assert_eq!(check(&s,&q,&r,CREATED)["ok"],false);
 let mut policy=s["snapshot"]["privacy"].clone();policy["use_ai_features"]=json!(true);let changed=apply(&s,json!({"kind":"set_privacy","data":{"expected_policy_revision":policy["revision"],"policy":policy}}),0xeb04);assert_eq!(check(&changed,&q,&r,NOW)["ok"],false);
 let mut malformed=q.clone();malformed["source_status"]=json!("accepted");let typed:PreparedBasketAdditionReview=serde_json::from_value(malformed.clone()).unwrap();malformed["prepared_digest"]=json!(basket_addition_review::review_digest(&typed).unwrap());assert_eq!(check(&s,&malformed,&r,NOW)["ok"],false);
}
#[test]
fn exact_ml_pack_is_useful_while_mismatch_unknown_and_multipack_supply_stay_unresolved(){
 let s=food(false,false);for(mode,net,count,expected)in [("ml",known(quantity("1000","ml","raw")),known(json!(1)),None),("unknown",unknown(),known(json!(1)),Some("pack_net_unknown")),("wrong-form",known(quantity("1000","ml","dry")),known(json!(1)),Some("pack_measure_mismatch")),("multipack",known(quantity("1000","ml","raw")),known(json!(2)),Some("pack_count_not_one"))]{let p=prepare(&s,"03");let mut o=observation(&p,net,unknown(),unknown());o["body"]["data"]["items"][0]["pack"]["count"]=count;let q=semantic(&s,&p,&o);let out=check(&s,&q,&returned(&q,"supported",false),NOW);assert_eq!(out["ok"],true,"{mode}: {out}");if let Some(reason)=expected{assert_eq!(out["value"]["qualification"]["pack_basis"],Value::Null);assert!(out["value"]["qualification"]["pack_reasons"].as_array().unwrap().contains(&json!(reason)),"{out}");}else{assert_eq!(out["value"]["qualification"]["pack_basis"],json!({"required":quantity("180","ml","raw"),"supply":quantity("1000","ml","raw")}));}}
}
#[test]
fn cold_drained_demand_requires_storage_citation_and_never_credits_net_weight(){
 let s=food(true,false);let storage=known(json!("Synthetic cooked ready-to-eat canned beans; direct cold use and actual opening/storage labels still need checking."));
 for(drained,expected)in [(known(quantity("240","g","drained")),None),(unknown(),Some("pack_drained_unknown"))]{let p=prepare(&s,"07");let o=observation(&p,known(quantity("999","g","raw")),drained,storage.clone());let q=semantic(&s,&p,&o);assert_eq!(check(&s,&q,&returned(&q,"supported",false),NOW)["ok"],false);let out=check(&s,&q,&returned(&q,"supported",true),NOW);assert_eq!(out["ok"],true,"{out}");if let Some(reason)=expected{assert_eq!(out["value"]["qualification"]["pack_basis"],Value::Null);assert!(out["value"]["qualification"]["pack_reasons"].as_array().unwrap().contains(&json!(reason)));}else{assert_eq!(out["value"]["qualification"]["pack_basis"],json!({"required":quantity("80","g","drained"),"supply":quantity("240","g","drained")}));}}
}
#[test]
fn actual_vegetarian_consumer_has_no_invented_requirement_rule_or_admission(){
 let s=food(true,true);let p=prepare(&s,"07");let storage=known(json!("Synthetic cooked canned ready-to-eat input, cold use and storage text."));let o=observation(&p,known(quantity("400","g","raw")),known(quantity("240","g","drained")),storage);let q=semantic(&s,&p,&o);let out=check(&s,&q,&returned(&q,"supported",true),NOW);assert_eq!(out["ok"],true,"{out}");assert_eq!(out["value"]["qualification"]["status"],"unresolved");assert!(out["value"]["qualification"]["reasons"].as_array().unwrap().contains(&json!("unsupported_policy")));assert_eq!(out["value"]["qualification"]["pack_basis"],Value::Null);
}
