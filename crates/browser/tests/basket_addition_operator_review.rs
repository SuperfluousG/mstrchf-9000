//! Actual pure native boundary with compiled food and ordinary commands.
//! People, source facts, external inference and declared operator actions are
//! synthetic. These tests do not establish a real human click, source truth,
//! account identity, current price or permission to perform a retailer effect.
use mstrchf_browser::{run_json,maker_packets::*};
use serde_json::{Value,json};

const CREATED:&str="2026-10-13T10:00:00Z";
const OBSERVED:&str="2026-10-13T10:00:01Z";
const REVIEWED:&str="2026-10-13T10:05:00Z";
const DECIDED:&str="2026-10-13T10:06:00Z";
const DEADLINE:&str="2026-10-13T11:00:00Z";
fn call(v:Value)->Value{serde_json::from_slice(&run_json(&serde_json::to_vec(&v).unwrap())).unwrap()}
fn good(v:Value)->Value{let r=call(v);assert_eq!(r["ok"],true,"{r}");r["value"].clone()}
fn refused(v:Value)->Value{let r=call(v);assert_eq!(r["ok"],false,"{r}");let mut e=r["error"].clone();e["path"]=e["issues"][0]["path"].clone();e}
fn apply(s:&Value,body:Value,id:u32,at:&str)->Value{good(json!({"op":"dispatch","state":s,"now":at,"command":{
 "schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":s["snapshot"]["snapshot_revision"],"command":body}}))["state"].clone()}
fn save_week(w:&Value)->Value{json!({"kind":"save_week","data":{"expected_week_revision":w["revision"],"week":{
 "week_id":w["range"]["week_id"],"defaults_snapshot":w["defaults_snapshot"],"day_enabled":w["day_enabled"],"meal_type_enabled":w["meal_type_enabled"],"cells":w["cells"],"plan":w["plan"]}}})}
fn food(cold:bool)->Value{
 let mut s=good(json!({"op":"initialize","household_id":"hh_800000000000000000000000000000c0","diner_id":"dn_800000000000000000000000000000c0",
  "zone":"Europe/London","today":"2026-10-13","now":CREATED,"demo":true}));
 let mut p=s["snapshot"]["profile"].clone();p["diners"][0]["age_band"]=json!("adult");p["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":CREATED}});
 p["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});p["preferences"]["equipment"]=json!(["fridge","hob"]);
 s=apply(&s,json!({"kind":"update_profile","data":{"expected_profile_revision":p["revision"],"profile":p}}),0xec00,CREATED);
 let mut w=s["snapshot"]["weeks"][0].clone();w["meal_type_enabled"]=json!({"breakfast":true,"lunch":true,"dinner":true});w["plan"]["selections"]=json!([]);w["plan"]["batch_links"]=json!([]);
 for c in w["cells"].as_array_mut().unwrap(){c["choice"]=json!("at_home");}s=apply(&s,save_week(&w),0xec01,CREATED);
 let packet=if cold{three_bean_packet_ref().unwrap()}else{porridge_packet_ref().unwrap()};
 s=apply(&s,json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":"1"}}),0xec02,CREATED);
 let w=&s["snapshot"]["weeks"][0];let index=if cold{4}else{3};let b=good(json!({"op":"board","state":s,"week_id":w["range"]["week_id"],"slot":w["cells"][index]["slot"],"options":null,"now":CREATED}));
 let choice=b["choices"].as_array().unwrap().iter().find(|v|v["selection"]["recipe"]==json!(packet.recipe)).unwrap();let mut w=w.clone();w["plan"]=choice["plan"].clone();apply(&s,save_week(&w),0xec03,CREATED)
}
fn product()->Value{json!({"retailer":"tesco","product_id":"123456789","variant_id":null})}
fn known(v:Value)->Value{json!({"kind":"known","data":{"value":v}})}
fn unknown()->Value{json!({"kind":"unknown","data":{"reason":"not_exposed"}})}
fn quantity(n:&str,d:&str,unit:&str,form:&str)->Value{json!({"amount":{"numerator":n,"denominator":d},"unit":unit,"form":form})}
fn prepare(s:&Value,suffix:&str)->Value{
 let c=good(json!({"op":"product_match_context","state":s,"week_id":s["snapshot"]["weeks"][0]["range"]["week_id"],"now":CREATED}));
 let line=c["lines"].as_array().unwrap().iter().find(|l|l["demand"]["ingredient_id"]==format!("ig_800000000000000000000000000000{suffix}")).unwrap()["demand"]["line_id"].clone();
 good(json!({"op":"basket_addition_prepare","state":s,"week_id":s["snapshot"]["weeks"][0]["range"]["week_id"],
 "reader":{"provider":"codex_local","native_task_ref":"synthetic-independent-external-reader"},"key":{"retailer":"tesco","account_ref":"synthetic-declared-account","basket_id":"bk_800000000000000000000000000000c0"},
 "delivery":{"retailer":"tesco","account_ref":"synthetic-declared-account","context_ref":"synthetic-declared-delivery","delivery_date":null,"slot_ref":null},
 "target":{"product":product(),"url":"https://www.tesco.com/shop/en-GB/products/123456789"},"line_id":line,
 "job_id":"jb_800000000000000000000000000000c0","read_id":"rd_800000000000000000000000000000c0","review_id":"sr_800000000000000000000000000000c0","now":CREATED,"deadline":DEADLINE}))
}
fn semantic(s:&Value,p:&Value,net:Value,drained:Value,count:Value,cold:bool)->Value{
 let source=&p["source_prepared"];let observation=json!({"schema_version":1,"observation_id":"ob_800000000000000000000000000000c0",
 "read_id":source["request"]["scope"]["data"]["read_scope"]["read_id"],"job_id":source["request"]["job_id"],"input_digest":source["request"]["input_digest"],
 "reader":source["expected_reader"],"resource":source["request"]["scope"]["data"]["read_scope"]["resource"],"observed_at":OBSERVED,
 "body":{"kind":"products","data":{"delivery_context":source["delivery"],"items":[{"product":product(),"url":"https://www.tesco.com/shop/en-GB/products/123456789",
 "title":"Synthetic external product; no actual source observation","pack":{"count":count,"net":net,"drained":drained},
 "price":{"kind":"unknown","data":{"reason":"not_exposed"}},"offers":[],"availability":"unknown","ingredients":known(json!("Synthetic exact plain input label <script>inert()</script>")),"allergens":unknown(),
 "storage":if cold{known(json!("Synthetic cooked ready-to-eat canned beans, direct cold use and storage label."))}else{unknown()},"delivery_context":source["delivery"],"observed_at":OBSERVED}]}}});
 good(json!({"op":"basket_addition_check_source","state":s,"prepared":p,"observation":observation,"now":REVIEWED}))
}
fn returned(q:&Value,state:&str,cold:bool)->Value{
 let request=&q["semantic_prepared"]["request"];let identity=json!({"state":state,"citations":if state=="unknown"{json!([])}else{json!([{"field":"ingredients","quote":request["product"]["ingredients"]["data"]["value"]}])},"reasoning":"Synthetic external inference, conditional on displayed facts; no real compliance observation."});
 let mut cooking=identity.clone();if cold&&state!="unknown"{cooking["citations"].as_array_mut().unwrap().push(json!({"field":"storage","quote":request["product"]["storage"]["data"]["value"]}));}
 json!({"format":"mstrchf-product-semantic-return","version":1,"request_id":request["request_id"],"input_digest":request["input_digest"],"identity":identity,
  "cooking":request["methods"].as_array().unwrap().iter().map(|m|json!({"method_handle":m["method_handle"],"decision":cooking})).collect::<Vec<_>>()})
}
fn draft(s:&Value,q:&Value,r:&Value)->Value{good(json!({"op":"basket_addition_check_semantic","state":s,"prepared":q,"returned":r,"now":REVIEWED}))}
fn operator(s:&Value,q:&Value,r:&Value,d:&Value)->Value{json!({"op":"basket_addition_operator_review","state":s,"prepared":q,"returned":r,"reviewed_at":REVIEWED,
 "decision":{"review_digest":d["draft_digest"],"decided_at":DECIDED,"source_and_interpretation_accepted":true,"exact_addition_approved":true},"now":DECIDED})}
fn oats()->(Value,Value,Value,Value,Value){let s=food(false);let p=prepare(&s,"01");let q=semantic(&s,&p,known(quantity("500","1","g","dry")),unknown(),known(json!(1)),false);let r=returned(&q,"supported",false);let d=draft(&s,&q,&r);(s,p,q,r,d)}

#[test]
fn frozen_v55_legacy_addition_source_semantic_draft_and_operator_remain_exact(){
 let fixture:Value=serde_json::from_slice(include_bytes!("fixtures/basket-addition-v1-v55.json")).unwrap();
 let(s,p,q,r,d)=oats();let op=good(operator(&s,&q,&r,&d));
 for(key,value)in [("state",s),("source",p),("semantic",q),("returned",r),("draft",d),("operator",op)]{
  assert_eq!(value,fixture[key],"{key}");assert_eq!(serde_json::to_vec(&value).unwrap(),serde_json::to_vec(&fixture[key]).unwrap(),"{key}");
  assert_eq!(json!(mstrchf_contracts::canonical_digest(&value).unwrap()),fixture["expected"][key],"{key}");
 }
 let s=&fixture["state"];let p=&fixture["source"];let q=&fixture["semantic"];let r=&fixture["returned"];let d=&fixture["draft"];
 assert_eq!(p["version"],1);assert_eq!(q["version"],1);assert_eq!(q["semantic_prepared"]["version"],2);assert!(q["semantic_prepared"]["binding"]["draft"].get("food_proof").is_none());
 assert_eq!(good(json!({"op":"validate_state","state":s,"now":DECIDED})),*s);
 assert_eq!(good(json!({"op":"basket_addition_check_source","state":s,"prepared":p,"observation":q["semantic_prepared"]["binding"]["observation"],"now":REVIEWED})),*q);
 assert_eq!(draft(s,q,r),*d);assert_eq!(good(operator(s,q,r,d)),fixture["operator"]);
}

#[test]
fn operator_declared_acceptance_is_current_deterministic_and_not_a_permission_or_state_write(){
 let(s,_,q,r,d)=oats();let input=operator(&s,&q,&r,&d);let result=good(input.clone());
 assert_eq!(s["snapshot"]["privacy"]["use_ai_features"],false);assert_eq!(s["snapshot"]["privacy"]["include_household_requirements"],false);
 assert_eq!(result["format"],"mstrchf-basket-addition-operator-review");assert_eq!(result["native_assessment"],"suitable");assert_eq!(result["additional_packs"],1);
 assert_eq!(result["required"],quantity("45","1","g","dry"));assert_eq!(result["supply"],quantity("500","1","g","dry"));assert_eq!(result["review_digest"],d["draft_digest"]);
 assert_eq!(result["source_observation_digest"],d["source_observation_digest"]);assert_eq!(result["interpretation_digest"],d["return_digest"]);assert_eq!(result["decided_at"],DECIDED);
 for field in ["source_truth_assessed","account_verified","price_assessed","effect_enabled","reusable_for_intent"]{assert_eq!(result[field],false);}
 for field in ["accepted","admission","candidate","match_basis","grant","grant_token","intent","human_action_verified","state"]{assert!(result.get(field).is_none(),"{field}");}
 assert_eq!(good(input),result);assert_eq!(draft(&s,&q,&r),d);assert_eq!(good(json!({"op":"validate_state","state":s,"now":DECIDED})),s);
 // An explicit declared decision has separate custody from the hypothetical
 // display. Price is still unknown: the host must check a standard price.
 assert_ne!(result["decision_digest"],d["draft_digest"]);assert_eq!(q["semantic_prepared"]["request"]["product"].get("price"),None);
}
#[test]
fn operator_requires_both_declarations_and_refuses_transported_extra_authority(){
 let(s,_,q,r,d)=oats();let input=operator(&s,&q,&r,&d);
 for field in ["source_and_interpretation_accepted","exact_addition_approved"]{let mut v=input.clone();v["decision"][field]=json!(false);let e=refused(v);assert_eq!(e["code"],"authority_denied");assert_eq!(e["path"],"/basket/addition/operator_decision");}
 for field in ["admission","grant_token","quantity"]{let mut v=input.clone();v["decision"][field]=json!(true);refused(v);}
 let mut v=input.clone();v["accepted"]=json!(true);refused(v);
}
#[test]
fn operator_binds_exact_prior_digest_and_review_clock_without_extending_the_window(){
 let(s,_,q,r,d)=oats();let input=operator(&s,&q,&r,&d);
 let mut wrong=input.clone();wrong["decision"]["review_digest"]=json!("f".repeat(64));assert_eq!(refused(wrong)["path"],"/basket/addition/review_digest");
 let mut wrong=input.clone();wrong["reviewed_at"]=json!("2026-10-13T10:05:01Z");assert_eq!(refused(wrong)["path"],"/basket/addition/review_digest");
 for(field,time)in [("decided_at","2026-10-13T10:06:01Z"),("decided_at",REVIEWED)]{let mut wrong=input.clone();wrong["decision"][field]=json!(time);assert_eq!(refused(wrong)["path"],"/basket/addition/operator_clock");}
 let mut wrong=input.clone();wrong["reviewed_at"]=json!("2026-10-13T10:07:00Z");assert_eq!(refused(wrong)["path"],"/basket/addition/operator_clock");
 let mut expired=input.clone();expired["decision"]["decided_at"]=json!(DEADLINE);expired["now"]=json!(DEADLINE);refused(expired);
 let mut changed_return=input;changed_return["returned"]["identity"]["reasoning"]=json!("Different external inference text must be reviewed afresh.");assert_eq!(refused(changed_return)["path"],"/basket/addition/review_digest");
}
#[test]
fn operator_accepts_exact_or_fractional_one_pack_but_refuses_insufficient_or_unknown_supply(){
 let s=food(false);let p=prepare(&s,"01");
 for(net,count,accepted,path)in [
  (known(quantity("45","1","g","dry")),known(json!(1)),true,""),
  (known(quantity("91","2","g","dry")),known(json!(1)),true,""),
  (known(quantity("44","1","g","dry")),known(json!(1)),false,"/basket/addition/operator_one_pack"),
  (unknown(),known(json!(1)),false,"/basket/addition/operator_pack"),
  (known(quantity("500","1","g","raw")),known(json!(1)),false,"/basket/addition/operator_suitability"),
  (known(quantity("500","1","g","dry")),known(json!(2)),false,"/basket/addition/operator_pack")
 ]{let q=semantic(&s,&p,net,unknown(),count,false);let r=returned(&q,"supported",false);let d=draft(&s,&q,&r);let input=operator(&s,&q,&r,&d);
   if accepted{assert_eq!(good(input)["additional_packs"],1);}else{assert_eq!(refused(input)["path"],path);}}
}
#[test]
fn operator_cannot_adopt_unknown_refuted_or_changed_recipe_semantics(){
 let(s,_,q,_,_)=oats();
 for status in ["unknown","refuted"]{let r=returned(&q,status,false);let d=draft(&s,&q,&r);assert_eq!(refused(operator(&s,&q,&r,&d))["path"],"/basket/addition/operator_suitability");}
 let mut r=returned(&q,"supported",false);r["cooking"][0]["decision"]["state"]=json!("refuted");let d=draft(&s,&q,&r);assert_eq!(refused(operator(&s,&q,&r,&d))["path"],"/basket/addition/operator_suitability");
 let mut r=returned(&q,"supported",false);r["identity"]["citations"][0]["quote"]=json!("Unobserved replacement facts");refused(operator(&s,&q,&r,&d));
}
#[test]
fn operator_rechecks_current_food_and_privacy_and_preserves_original_receipts(){
 let(s,_,q,r,d)=oats();let receipt_prefix=s["receipts"].clone();
 let mut policy=s["snapshot"]["privacy"].clone();policy["use_ai_features"]=json!(true);
 let changed=apply(&s,json!({"kind":"set_privacy","data":{"expected_policy_revision":policy["revision"],"policy":policy}}),0xec04,DECIDED);refused(operator(&changed,&q,&r,&d));
 let mut w=s["snapshot"]["weeks"][0].clone();w["plan"]["selections"]=json!([]);w["plan"]["batch_links"]=json!([]);let changed=apply(&s,save_week(&w),0xec05,DECIDED);refused(operator(&changed,&q,&r,&d));
 assert_eq!(changed["receipts"].as_array().unwrap()[..receipt_prefix.as_array().unwrap().len()],receipt_prefix.as_array().unwrap()[..]);
 assert_eq!(good(json!({"op":"validate_state","state":changed,"now":DECIDED})),changed);
}
#[test]
fn operator_cold_input_requires_real_native_storage_citation_and_exact_drained_measure(){
 let s=food(true);let p=prepare(&s,"07");let q=semantic(&s,&p,known(quantity("400","1","g","raw")),known(quantity("80","1","g","drained")),known(json!(1)),true);
 let r=returned(&q,"supported",true);let d=draft(&s,&q,&r);let out=good(operator(&s,&q,&r,&d));assert_eq!(out["required"],quantity("80","1","g","drained"));assert_eq!(out["supply"],quantity("80","1","g","drained"));
 let mut wrong=r;wrong["cooking"][0]["decision"]["citations"].as_array_mut().unwrap().retain(|c|c["field"]!="storage");refused(operator(&s,&q,&wrong,&d));
 let q=semantic(&s,&p,known(quantity("400","1","g","raw")),unknown(),known(json!(1)),true);let r=returned(&q,"supported",true);let d=draft(&s,&q,&r);assert_eq!(refused(operator(&s,&q,&r,&d))["path"],"/basket/addition/operator_pack");
}
