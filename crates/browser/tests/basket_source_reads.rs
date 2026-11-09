//! Actual native basket/read contracts and pinned R3 food with fictional actors,
//! aliases, basket rows and prices. No Tesco access, user witness, current basket,
//! retailer account, price role/currentness or controlled execution is proved.
use mstrchf_browser::{run_json,maker_packets::porridge_packet_ref,
    basket_source_reads::{PreparedBasketSourceRead,prepared_digest}};
use mstrchf_contracts::*;
use serde_json::{Value,json};
const CREATED:&str="2026-10-13T07:00:00Z";
const OBSERVED:&str="2026-10-13T07:00:01Z";
const NOW:&str="2026-10-13T07:05:00Z";
const DEADLINE:&str="2026-10-13T08:00:00Z";
fn call(input:Value)->Value{serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap()}
fn good(input:Value)->Value{let out=call(input);assert_eq!(out["ok"],true,"{out}");out["value"].clone()}
fn refuse(input:Value)->Value{let out=call(input);assert_eq!(out["ok"],false,"{out}");out["error"].clone()}
fn apply(s:&Value,body:Value,id:u32)->Value{good(json!({"op":"dispatch","state":s,"now":CREATED,"command":{"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":s["snapshot"]["snapshot_revision"],"command":body}}))["state"].clone()}
fn week_body(w:&Value)->Value{json!({"kind":"save_week","data":{"expected_week_revision":w["revision"],"week":{"week_id":w["range"]["week_id"],"defaults_snapshot":w["defaults_snapshot"],"day_enabled":w["day_enabled"],"meal_type_enabled":w["meal_type_enabled"],"cells":w["cells"],"plan":w["plan"]}}})}
fn privacy(s:&Value,on:bool,id:u32)->Value{let mut p=s["snapshot"]["privacy"].clone();p["use_ai_features"]=json!(on);p["include_household_requirements"]=json!(false);apply(s,json!({"kind":"set_privacy","data":{"expected_policy_revision":p["revision"],"policy":p}}),id)}
fn saved(ai:bool)->Value{
 let mut s=good(json!({"op":"initialize","household_id":"hh_800000000000000000000000000000a0","diner_id":"dn_800000000000000000000000000000a0","zone":"Europe/London","today":"2026-10-13","now":CREATED,"demo":true}));
 let mut p=s["snapshot"]["profile"].clone();p["diners"][0]["age_band"]=json!("adult");p["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":CREATED}});p["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});p["preferences"]["equipment"]=json!(["hob"]);
 s=apply(&s,json!({"kind":"update_profile","data":{"expected_profile_revision":p["revision"],"profile":p}}),0xea00);let mut w=s["snapshot"]["weeks"][0].clone();w["meal_type_enabled"]["breakfast"]=json!(true);w["cells"][3]["choice"]=json!("at_home");s=apply(&s,week_body(&w),0xea01);
 s=apply(&s,json!({"kind":"admit_reviewed_recipe","data":{"packet":porridge_packet_ref().unwrap(),"expected_catalogue_revision":"1"}}),0xea02);let w=&s["snapshot"]["weeks"][0];let board=good(json!({"op":"board","state":s,"week_id":w["range"]["week_id"],"slot":w["cells"][3]["slot"],"now":CREATED}));let choice=board["choices"].as_array().unwrap().iter().find(|c|c["selection"]["recipe"]==json!(porridge_packet_ref().unwrap().recipe)).unwrap();let mut w=w.clone();w["plan"]=choice["plan"].clone();s=apply(&s,week_body(&w),0xea03);if ai{s=privacy(&s,true,0xea04);}s
}
fn prepare_input(s:&Value)->Value{json!({"op":"basket_source_prepare","state":s,"week_id":s["snapshot"]["weeks"][0]["range"]["week_id"],"reader":{"provider":"codex_local","native_task_ref":"synthetic-locally-correlated-reader-not-host-attested"},"key":{"retailer":"tesco","account_ref":"synthetic-local-account-alias","basket_id":"bk_800000000000000000000000000000a0"},"delivery":{"retailer":"tesco","account_ref":"synthetic-local-account-alias","context_ref":"synthetic-local-delivery-alias","delivery_date":null,"slot_ref":null},"job_id":"jb_800000000000000000000000000000a0","read_id":"rd_800000000000000000000000000000a0","now":CREATED,"deadline":DEADLINE})}
fn prepare(s:&Value)->Value{good(prepare_input(s))}
fn unknown()->Value{json!({"kind":"unknown","data":{"reason":"not_exposed"}})}
fn known(value:Value)->Value{json!({"kind":"known","data":{"value":value}})}
fn price(pence:&str,label:&str)->Value{json!({"pence":pence,"currency":"GBP","label":label})}
fn product(last:&str)->Value{json!({"retailer":"tesco","product_id":format!("12345678{last}"),"variant_id":null})}
fn payload(p:&Value)->Value{json!({"format":"mstrchf-basket-source-return","version":1,"read_id":p["request"]["scope"]["data"]["read_scope"]["read_id"],"job_id":p["request"]["job_id"],"input_digest":p["request"]["input_digest"],"observed_at":OBSERVED,"completeness":"complete","observed_lines":[
 {"line_ref":"synthetic-row-A","product":product("1"),"title":"Synthetic oats row","quantity":known(json!(2)),"unit_price":known(price("210","Synthetic explicitly declared price for one sold pack"))},
 {"line_ref":"synthetic-unrelated-row-B","product":product("2"),"title":"Synthetic unrelated goods <script>inert</script>","quantity":known(json!(3)),"unit_price":unknown()}],"delivery_charge":unknown(),"notes":["Synthetic source capture; no actual basket read or account verification occurred."]})}
fn check(s:&Value,p:&Value,r:&Value)->Value{json!({"op":"basket_source_check_return","state":s,"prepared":p,"returned":r,"observation_id":"ob_800000000000000000000000000000a0","now":NOW})}
fn accept(s:&Value,p:&Value,r:&Value,o:&Value)->Value{json!({"op":"basket_source_accept","state":s,"prepared":p,"returned":r,"observation":o,"evidence_id":"ev_800000000000000000000000000000a0","now":NOW})}
fn audit(p:&Value,r:Value,o:Value,a:Value)->Value{json!({"op":"basket_source_audit","prepared":p,"returned":r,"observation":o,"accepted":a})}
fn inspect(s:&Value,p:&Value)->Value{json!({"op":"basket_source_inspect","state":s,"prepared":p,"now":NOW})}
fn resign(p:&mut Value){let mut req:JobRequestV1=serde_json::from_value(p["request"].clone()).unwrap();req.input_digest=input_digest(&RequestMaterial::from(&req)).unwrap();p["request"]=json!(req);let typed:PreparedBasketSourceRead=serde_json::from_value(p.clone()).unwrap();p["prepared_digest"]=json!(prepared_digest(&typed).unwrap());}

#[test]
fn private_prepare_is_basket_only_has_no_approval_contribution_effect_or_native_job_and_ai_off_cannot_inspect(){
 let s=saved(false);let before=s.clone();let p=prepare(&s);let typed:PreparedBasketSourceRead=serde_json::from_value(p.clone()).unwrap();validate_request_shape(&typed.request).unwrap();assert_eq!(typed.prepared_digest,prepared_digest(&typed).unwrap());assert_eq!(typed.request.input_digest,input_digest(&RequestMaterial::from(&typed.request)).unwrap());
 assert_eq!(typed.request.action,Action::PrepareShop);assert_eq!(typed.request.required_capabilities.0,vec![Capability::BrowserRead]);assert!(typed.request.allowed_artefacts.is_empty()&&typed.request.context_stamp.write_weeks.is_empty());
 let ActionScope::Shop{phase,approval_id,read_scope,..}=&typed.request.scope else{panic!("shop")};assert_eq!(*phase,ShopPhase::Observe);assert!(approval_id.0.is_none());assert_eq!(read_scope.allowed_kinds.0,vec![ReadKind::Basket]);assert!(read_scope.products.is_empty()&&read_scope.event_ids.is_empty());
 let JobPayload::ProductObservation{products,approved_contribution,..}=&typed.request.payload else{panic!("read payload")};assert!(products.is_empty()&&approved_contribution.0.is_none());assert_eq!(s,before);assert_eq!(s["snapshot"]["jobs"],json!([]));assert_eq!(refuse(inspect(&s,&p))["code"],"authority_denied");assert_eq!(good(audit(&p,Value::Null,Value::Null,Value::Null))["valid"],true);
}

#[test]
fn complete_read_builds_exact_independent_snapshot_preserving_unrelated_goods_but_never_execution_authority(){
 let s=saved(true);let before=s.clone();let p=prepare(&s);let r=payload(&p);let checked=good(check(&s,&p,&r));assert_eq!(checked["payload_digest"],json!(canonical_digest(&r).unwrap()));let o=&checked["observation"];assert_eq!(o["reader"],p["expected_reader"]);assert_eq!(o["resource"],p["request"]["scope"]["data"]["read_scope"]["resource"]);assert_eq!(o["body"]["kind"],"basket");
 let b=&o["body"]["data"]["snapshot"];assert_eq!(b["delivery_context"],p["delivery"]);assert_eq!(b["lines"].as_array().unwrap().len(),2);assert_eq!(b["lines"][1]["product"],product("2"));assert_eq!(b["lines"][1]["quantity"],3);assert_eq!(b["delivery_charge"],unknown());assert_eq!(b["delivery_context"]["delivery_date"],Value::Null);assert_eq!(b["delivery_context"]["slot_ref"],Value::Null);assert!(b["lines"][0].get("title").is_none());
 let accepted=good(accept(&s,&p,&r,o));assert_eq!(accepted["accepted"]["digest"],checked["observation_digest"]);assert_eq!(accepted["assessment"],json!({"source_truth_assessed":false,"account_verified":false,"execution_baseline_status":"unqualified","effect_enabled":false,"price_currentness":"unknown"}));for key in["baseline","intent","approval","grant_token","existing_credit","stock","match_basis"]{assert!(accepted.get(key).is_none());}
 let a=good(audit(&p,r.clone(),o.clone(),accepted["accepted"].clone()));assert_eq!(a["payload_digest"],checked["payload_digest"]);assert_eq!(a["observation_digest"],checked["observation_digest"]);assert_eq!(good(json!({"op":"validate_state","state":s,"now":NOW})),before);
}

#[test]
fn complete_empty_is_explicit_and_unknown_fees_never_become_free_stock_or_a_qualified_baseline(){
 let s=saved(true);let p=prepare(&s);let mut r=payload(&p);r["observed_lines"]=json!([]);let checked=good(check(&s,&p,&r));assert_eq!(checked["observation"]["body"]["data"]["snapshot"]["lines"],json!([]));assert_eq!(checked["observation"]["body"]["data"]["snapshot"]["delivery_charge"],unknown());
 let receipt=good(accept(&s,&p,&r,&checked["observation"]));assert_eq!(receipt["assessment"]["execution_baseline_status"],"unqualified");assert_eq!(receipt["assessment"]["effect_enabled"],false);
 r["completeness"]=json!("incomplete");let incomplete=good(check(&s,&p,&r));assert_eq!(incomplete["observation"],Value::Null);assert_eq!(incomplete["observation_digest"],Value::Null);refuse(accept(&s,&p,&r,&checked["observation"]));
}

#[test]
fn incomplete_unknown_conflicting_and_unresolved_products_are_retained_without_dummy_observation_or_acceptance(){
 let s=saved(true);let p=prepare(&s);let mut r=payload(&p);r["completeness"]=json!("incomplete");r["observed_lines"][0]["product"]=Value::Null;r["observed_lines"][0]["quantity"]=unknown();r["observed_lines"][1]["quantity"]=json!({"kind":"conflicting","data":{"values":[2,3]}});r["observed_lines"][1]["unit_price"]=json!({"kind":"conflicting","data":{"values":[price("210","Synthetic role label A"),price("220","Synthetic role label B")]}});
 let checked=good(check(&s,&p,&r));assert_eq!(checked["observation"],Value::Null);assert_eq!(checked["observation_digest"],Value::Null);assert_eq!(checked["payload_digest"],json!(canonical_digest(&r).unwrap()));let audited=good(audit(&p,r.clone(),Value::Null,Value::Null));assert_eq!(audited["payload_digest"],checked["payload_digest"]);
 let complete=good(check(&s,&p,&payload(&p)));refuse(accept(&s,&p,&r,&complete["observation"]));refuse(audit(&p,r,complete["observation"].clone(),Value::Null));
}

#[test]
fn complete_requires_every_exact_quantity_and_product_and_never_merges_duplicate_or_ambiguous_rows(){
 let s=saved(true);let p=prepare(&s);let r=payload(&p);
 for attack in["unknown_quantity","conflict_quantity","unknown_product","duplicate_product","duplicate_row","foreign_product","variant","zero_quantity","float_quantity"]{let mut bad=r.clone();match attack{
  "unknown_quantity"=>bad["observed_lines"][0]["quantity"]=unknown(),"conflict_quantity"=>bad["observed_lines"][0]["quantity"]=json!({"kind":"conflicting","data":{"values":[1,2]}}),"unknown_product"=>bad["observed_lines"][0]["product"]=Value::Null,
  "duplicate_product"=>bad["observed_lines"][1]["product"]=bad["observed_lines"][0]["product"].clone(),"duplicate_row"=>bad["observed_lines"][1]["line_ref"]=bad["observed_lines"][0]["line_ref"].clone(),"foreign_product"=>bad["observed_lines"][0]["product"]["product_id"]=json!("not-a-tesco-number"),"variant"=>bad["observed_lines"][0]["product"]["variant_id"]=json!("unqualified-variant"),"zero_quantity"=>bad["observed_lines"][0]["quantity"]=known(json!(0)),_=>bad["observed_lines"][0]["quantity"]=known(json!(1.5))};refuse(check(&s,&p,&bad));}
 let mut duplicates=r;duplicates["completeness"]=json!("incomplete");duplicates["observed_lines"][1]["product"]=duplicates["observed_lines"][0]["product"].clone();
 // Separate source rows stay intact when incomplete: no inferred merging,
 // exact-product closure or native BasketSnapshot is produced.
 let ambiguous=good(check(&s,&p,&duplicates));assert_eq!(ambiguous["observation"],Value::Null);assert_eq!(ambiguous["payload_digest"],json!(canonical_digest(&duplicates).unwrap()));
 duplicates["observed_lines"][0]["product"]=Value::Null;duplicates["observed_lines"][1]["product"]=Value::Null;duplicates["notes"]=json!(["Synthetic ambiguous rows: both exact product identities unresolved; no merge or complete-basket claim."]);
 let unresolved=good(check(&s,&p,&duplicates));assert_eq!(unresolved["observation"],Value::Null);assert_eq!(unresolved["payload_digest"],json!(canonical_digest(&duplicates).unwrap()));
 assert_eq!(good(audit(&p,duplicates.clone(),Value::Null,Value::Null))["valid"],true);duplicates["completeness"]=json!("complete");refuse(check(&s,&p,&duplicates));
}

#[test]
fn price_conflicts_remain_sidecar_facts_and_exact_labels_have_separate_digest_from_native_observation(){
 let s=saved(true);let p=prepare(&s);let mut r=payload(&p);r["observed_lines"][0]["unit_price"]=json!({"kind":"conflicting","data":{"values":[price("210","Synthetic pack price"),price("220","Synthetic conflicting pack price")]}});r["delivery_charge"]=json!({"kind":"conflicting","data":{"values":[price("100","Synthetic fee A"),price("200","Synthetic fee B")]}});let checked=good(check(&s,&p,&r));let b=&checked["observation"]["body"]["data"]["snapshot"];assert_eq!(b["lines"][0]["unit_price"],json!({"kind":"unknown","data":{"reason":"unreadable"}}));assert_eq!(b["delivery_charge"],json!({"kind":"unknown","data":{"reason":"unreadable"}}));assert_eq!(checked["payload_digest"],json!(canonical_digest(&r).unwrap()));
 let original=payload(&p);let a=good(check(&s,&p,&original));let mut labelled=original.clone();labelled["observed_lines"][0]["unit_price"]["data"]["value"]["label"]=json!("<script>inert source label</script> Synthetic one-pack price");labelled["observed_lines"][1]["title"]=json!("Changed inert source title");let b=good(check(&s,&p,&labelled));assert_eq!(a["observation_digest"],b["observation_digest"]);assert_ne!(a["payload_digest"],b["payload_digest"],"Owner must retain/check this whole-return witness digest too.");assert_eq!(labelled["observed_lines"][0]["unit_price"]["data"]["value"]["label"],"<script>inert source label</script> Synthetic one-pack price");
}

#[test]
fn foreign_correlation_actor_resource_delivery_payload_and_incoming_authority_never_replace_local_bindings(){
 let s=saved(true);let p=prepare(&s);let r=payload(&p);let checked=good(check(&s,&p,&r));
 for field in["read_id","job_id","input_digest","reader","account","approval"]{let mut bad=r.clone();match field{
  "read_id"=>bad[field]=json!("rd_800000000000000000000000000000a1"),"job_id"=>bad[field]=json!("jb_800000000000000000000000000000a1"),"input_digest"=>bad[field]=json!("a".repeat(64)),_=>bad[field]=json!("supplied external authority")};refuse(check(&s,&p,&bad));}
 for attack in["reader","resource","delivery","quantity","price","products_body"]{let mut o=checked["observation"].clone();match attack{
  "reader"=>o["reader"]["native_task_ref"]=json!("synthetic-foreign-task"),"resource"=>o["resource"]["data"]["key"]["basket_id"]=json!("bk_800000000000000000000000000000a1"),"delivery"=>o["body"]["data"]["snapshot"]["delivery_context"]["context_ref"]=json!("synthetic-other-context"),"quantity"=>o["body"]["data"]["snapshot"]["lines"][0]["quantity"]=json!(9),"price"=>o["body"]["data"]["snapshot"]["lines"][0]["unit_price"]["data"]["pence"]=json!("999"),_=>o["body"]=json!({"kind":"products","data":{"delivery_context":p["delivery"],"items":[]}})};refuse(accept(&s,&p,&r,&o));refuse(audit(&p,r.clone(),o,Value::Null));}
}

#[test]
fn strict_task_chronology_food_privacy_and_acceptance_guards_do_not_turn_history_into_fresh_basket_authority(){
 let s=saved(true);let p=prepare(&s);let r=payload(&p);let checked=good(check(&s,&p,&r));let receipt=good(accept(&s,&p,&r,&checked["observation"]));
 for at in[CREATED,"2026-10-13T07:05:01Z",DEADLINE]{let mut bad=r.clone();bad["observed_at"]=json!(at);refuse(check(&s,&p,&bad));}
 let mut late=check(&s,&p,&r);late["now"]=json!(DEADLINE);refuse(late);let mut back=inspect(&s,&p);back["now"]=json!("2026-10-13T06:59:59Z");refuse(back);
 let off=privacy(&s,false,0xeb01);refuse(inspect(&off,&p));refuse(check(&off,&p,&r));let on=privacy(&off,true,0xeb02);refuse(accept(&on,&p,&r,&checked["observation"]));
 let mut profile=s["snapshot"]["profile"].clone();profile["diners"][0]["display_name"]=json!("Changed fictional context");let changed=apply(&s,json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),0xeb03);refuse(inspect(&changed,&p));
 let mut w=s["snapshot"]["weeks"][0].clone();w["plan"]["selections"]=json!([]);let empty=apply(&s,week_body(&w),0xeb04);refuse(prepare_input(&empty));refuse(check(&empty,&p,&r));
 assert_eq!(good(audit(&p,r.clone(),checked["observation"].clone(),receipt["accepted"].clone()))["valid"],true);
 for field in["digest","read_id","observed_at","admitted_at"]{let mut bad=receipt["accepted"].clone();bad[field]=match field{"digest"=>json!("f".repeat(64)),"read_id"=>json!("rd_800000000000000000000000000000a1"),_=>json!(DEADLINE)};refuse(audit(&p,r.clone(),checked["observation"].clone(),bad));}
}

#[test]
fn even_resigned_prepared_transport_cannot_add_other_read_kinds_mutation_contribution_or_write_scope(){
 let s=saved(true);let p=prepare(&s);
 for attack in["kinds","capability","approval","products","purpose","writes","food"]{let mut bad=p.clone();match attack{
  "kinds"=>{bad["request"]["scope"]["data"]["read_scope"]["allowed_kinds"]=json!(["basket","products"]);bad["request"]["payload"]["data"]["read_scope"]=bad["request"]["scope"]["data"]["read_scope"].clone();},
  "capability"=>bad["request"]["required_capabilities"]=json!(["browser_read","browser_mutate"]),"approval"=>bad["request"]["scope"]["data"]["approval_id"]=json!("au_800000000000000000000000000000a0"),
  "products"=>bad["request"]["payload"]["data"]["products"]=json!([product("1")]),"purpose"=>{bad["request"]["scope"]["data"]["read_scope"]["purpose"]=json!("reconciliation");bad["request"]["payload"]["data"]["read_scope"]=bad["request"]["scope"]["data"]["read_scope"].clone();},"writes"=>bad["request"]["context_stamp"]["write_weeks"]=bad["request"]["context_stamp"]["read_weeks"].clone(),_=>bad["food"]["derived_digest"]=json!("f".repeat(64))};resign(&mut bad);refuse(inspect(&s,&bad));
  if attack!="food"{refuse(audit(&bad,Value::Null,Value::Null,Value::Null));}
 }
}

#[test]
fn strict_scalars_bounds_utf8_and_two_megabyte_payload_refuse_without_a_source_observation(){
 let s=saved(true);let p=prepare(&s);let r=payload(&p);
 for attack in["pence_padding","pence_float","price_limit","currency","label","title_utf8","empty_title","blank_note","ref","notes","lines"]{let mut bad=r.clone();match attack{
  "pence_padding"=>bad["observed_lines"][0]["unit_price"]["data"]["value"]["pence"]=json!("0210"),"pence_float"=>bad["observed_lines"][0]["unit_price"]["data"]["value"]["pence"]=json!(2.1),"price_limit"=>bad["observed_lines"][0]["unit_price"]["data"]["value"]["pence"]=json!("1000000000001"),"currency"=>bad["observed_lines"][0]["unit_price"]["data"]["value"]["currency"]=json!("USD"),"label"=>bad["observed_lines"][0]["unit_price"]["data"]["value"]["label"]=json!(" "),"title_utf8"=>bad["observed_lines"][0]["title"]=json!("é".repeat(513)),"empty_title"=>bad["observed_lines"][0]["title"]=json!(""),"blank_note"=>bad["notes"]=json!([" "]),"ref"=>bad["observed_lines"][0]["line_ref"]=json!("x".repeat(81)),"notes"=>bad["notes"]=json!(vec!["note";17]),_=>bad["observed_lines"]=json!(vec![r["observed_lines"][0].clone();257])};refuse(check(&s,&p,&bad));}
 let mut too_big=r;too_big["completeness"]=json!("incomplete");too_big["observed_lines"]=json!((0..256).map(|i|json!({"line_ref":format!("synthetic-row-{i}"),"product":null,"title":"Synthetic incomplete row","quantity":unknown(),"unit_price":{"kind":"conflicting","data":{"values":[price("1",&("A".repeat(4096))),price("2",&("B".repeat(4096)))]}}})).collect::<Vec<_>>());assert!(serde_json::to_vec(&too_big).unwrap().len()>2_000_000);refuse(check(&s,&p,&too_big));
}
