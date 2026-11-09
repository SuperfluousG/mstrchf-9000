//! Synthetic source correlation and actual compiled oats/native owner fixtures;
//! actors, source text, products and households are explicitly synthetic.
//! Conditional core matching uses transient maker admission. These fixtures
//! prove no real witness, Tesco read, unchanged source, usable pack coverage,
//! commercial qualification, persisted approval or external permission.
use mstrchf_browser::{run_json, maker_packets::*};
use mstrchf_contracts::*;
use serde_json::{Value,json};

const CREATED:&str="2026-10-13T10:00:00Z";
const OBSERVED:&str="2026-10-13T10:00:01Z";
const NOW:&str="2026-10-13T10:05:00Z";
const LATER:&str="2026-10-13T11:30:00Z";
const DEADLINE:&str="2026-10-13T11:00:00Z";
fn call(input:Value)->Value {serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap()}
fn good(input:Value)->Value {let out=call(input);assert_eq!(out["ok"],true,"{out}");out["value"].clone()}
fn refuse(input:Value)->Value {let out=call(input);assert_eq!(out["ok"],false,"{out}");out["error"].clone()}
fn apply(state:&Value,body:Value,id:u32)->Value {good(json!({"op":"dispatch","state":state,"now":CREATED,"command":{
    "schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body}}))["state"].clone()}
fn week_body(w:&Value)->Value {json!({"kind":"save_week","data":{"expected_week_revision":w["revision"],"week":{
    "week_id":w["range"]["week_id"],"defaults_snapshot":w["defaults_snapshot"],"day_enabled":w["day_enabled"],
    "meal_type_enabled":w["meal_type_enabled"],"cells":w["cells"],"plan":w["plan"]}}})}
fn privacy(s:&Value,on:bool,id:u32)->Value {let mut p=s["snapshot"]["privacy"].clone();p["use_ai_features"]=json!(on);p["include_household_requirements"]=json!(false);apply(s,json!({"kind":"set_privacy","data":{"expected_policy_revision":p["revision"],"policy":p}}),id)}
fn setup()->Value {
    let mut s=good(json!({"op":"initialize","household_id":"hh_80000000000000000000000000000090","diner_id":"dn_80000000000000000000000000000090","zone":"Europe/London","today":"2026-10-13","now":CREATED,"demo":true}));
    let mut p=s["snapshot"]["profile"].clone();p["diners"][0]["age_band"]=json!("adult");p["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":CREATED}});
    p["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});p["preferences"]["equipment"]=json!(["fridge","hob"]);
    s=apply(&s,json!({"kind":"update_profile","data":{"expected_profile_revision":p["revision"],"profile":p}}),0xd900);
    let mut w=s["snapshot"]["weeks"][0].clone();w["meal_type_enabled"]=json!({"breakfast":true,"lunch":true,"dinner":true});w["plan"]["selections"]=json!([]);w["plan"]["batch_links"]=json!([]);for cell in w["cells"].as_array_mut().unwrap(){cell["choice"]=json!("at_home");}
    s=apply(&s,week_body(&w),0xd901);privacy(&s,true,0xd902)
}
fn admit(s:&Value,p:ReviewedRecipePacketRef,id:u32)->Value {apply(s,json!({"kind":"admit_reviewed_recipe","data":{"packet":p,"expected_catalogue_revision":(1+s["reviewed_packets"].as_array().map_or(0,Vec::len)).to_string()}}),id)}
fn select(s:&Value,index:usize,rf:RecipeRef,preset:Option<&str>,id:u32)->Value {
    let w=&s["snapshot"]["weeks"][0];let options=preset.map(|preset|json!({"overrides":{"portions":[{"person":{"kind":"diner","data":{"diner_id":s["snapshot"]["profile"]["diners"][0]["diner_id"]}},"preset":preset,"adjustment":{"numerator":"1","denominator":"1"}}],"eat_by":null}}));
    let b=good(json!({"op":"board","state":s,"week_id":w["range"]["week_id"],"slot":w["cells"][index]["slot"],"options":options,"now":CREATED}));
    let c=b["choices"].as_array().unwrap().iter().find(|c|c["selection"]["recipe"]==json!(rf)).expect("eligible actual compiled recipe");let mut w=w.clone();w["plan"]=c["plan"].clone();apply(s,week_body(&w),id)
}
fn porridge()->Value {let s=admit(&setup(),porridge_packet_ref().unwrap(),0xd903);let s=admit(&s,larger_porridge_packet_ref().unwrap(),0xd904);let s=select(&s,3,porridge_packet_ref().unwrap().recipe,None,0xd905);select(&s,6,larger_porridge_packet_ref().unwrap().recipe,Some("larger"),0xd906)}
fn product()->Value {json!({"retailer":"tesco","product_id":"123456789","variant_id":null})}
fn unknown()->Value {json!({"kind":"unknown","data":{"reason":"not_exposed"}})}
fn known(v:Value)->Value {json!({"kind":"known","data":{"value":v}})}
fn quantity(n:&str,unit:&str,form:&str)->Value {json!({"amount":{"numerator":n,"denominator":"1"},"unit":unit,"form":form})}
fn prepare(s:&Value)->Value {good(json!({"op":"product_source_prepare","state":s,"week_id":s["snapshot"]["weeks"][0]["range"]["week_id"],
    "reader":{"provider":"codex_local","native_task_ref":"synthetic-owner-bound-task-not-a-live-host-attestation"},
    "key":{"retailer":"tesco","account_ref":"synthetic-local-account","basket_id":"bk_80000000000000000000000000000090"},
    "delivery":{"retailer":"tesco","account_ref":"synthetic-local-account","context_ref":"synthetic-public-source-context","delivery_date":null,"slot_ref":null},
    "targets":[{"product":product(),"url":"https://www.tesco.com/shop/en-GB/products/123456789"}],
    "job_id":"jb_80000000000000000000000000000090","read_id":"rd_80000000000000000000000000000090","now":CREATED,"deadline":DEADLINE}))}
fn observation(p:&Value,net:Value,drained:Value)->Value {json!({"schema_version":1,"observation_id":"ob_80000000000000000000000000000090",
    "read_id":p["request"]["scope"]["data"]["read_scope"]["read_id"],"job_id":p["request"]["job_id"],"input_digest":p["request"]["input_digest"],
    "reader":p["expected_reader"],"resource":p["request"]["scope"]["data"]["read_scope"]["resource"],"observed_at":OBSERVED,
    "body":{"kind":"products","data":{"delivery_context":p["delivery"],"items":[{"product":product(),"url":"https://www.tesco.com/shop/en-GB/products/123456789",
        "title":"Synthetic source fixture; no Tesco read occurred","pack":{"count":known(json!(1)),"net":net,"drained":drained},
        "price":{"kind":"unknown","data":{"reason":"not_exposed"}},"offers":[],"availability":"unknown",
        "ingredients":known(json!("<script>synthetic inert ingredients text; no product meaning reviewed</script>")),
        "allergens":{"kind":"conflicting","data":{"values":["Synthetic statement A","Synthetic statement B"]}},"storage":unknown(),
        "delivery_context":p["delivery"],"observed_at":OBSERVED}]}}})}
fn accepted(s:&Value,p:&Value,o:&Value)->Value {good(json!({"op":"product_source_accept","state":s,"prepared":p,"observation":o,"evidence_id":"ev_80000000000000000000000000000090","now":NOW}))["accepted"].clone()}
fn context(s:&Value)->Value {good(json!({"op":"product_match_context","state":s,"week_id":s["snapshot"]["weeks"][0]["range"]["week_id"],"now":NOW}))}
fn line(s:&Value,suffix:&str)->Value {context(s)["lines"].as_array().unwrap().iter().find(|l|l["demand"]["ingredient_id"]==format!("ig_800000000000000000000000000000{suffix}")).unwrap()["demand"]["line_id"].clone()}
fn input(s:&Value,p:&Value,o:&Value,a:&Value,line:Value,at:&str)->Value {json!({"op":"product_meaning_review_prepare","state":s,"week_id":s["snapshot"]["weeks"][0]["range"]["week_id"],"prepared":p,"observation":o,"accepted":a,"product":product(),"line_id":line,"now":at})}
fn semantic(s:&Value,p:&Value,o:&Value,a:&Value,id:Value,at:&str,deadline:&str)->Value {
    // Test-only legacy-wire preparation preserves the old policy assertions.
    let mut q=semantic_v2(s,p,o,a,id,at,deadline);q["version"]=json!(1);q["request"]["version"]=json!(1);
    let request:mstrchf_browser::product_semantic_review::PublicSemanticRequest=serde_json::from_value(q["request"].clone()).unwrap();q["request"]["input_digest"]=json!(mstrchf_browser::product_semantic_review::request_digest(&request).unwrap());resign(&mut q);q
}
fn semantic_v2(s:&Value,p:&Value,o:&Value,a:&Value,id:Value,at:&str,deadline:&str)->Value {
    good(json!({"op":"product_semantic_prepare","state":s,"week_id":s["snapshot"]["weeks"][0]["range"]["week_id"],
        "source_prepared":p,"observation":o,"accepted":a,"product":product(),"line_id":id,
        "request_id":"sr_80000000000000000000000000000090","now":at,"deadline":deadline}))
}
fn fixture()->(Value,Value,Value,Value,Value){
    let s=porridge();let p=prepare(&s);let mut o=observation(&p,known(quantity("500","g","dry")),unknown());
    o["body"]["data"]["items"][0]["ingredients"]=known(json!("Synthetic label: plain rolled oats (100%)."));
    let a=accepted(&s,&p,&o);let q=semantic(&s,&p,&o,&a,line(&s,"01"),NOW,DEADLINE);(s,p,o,a,q)
}
fn decision(q:&Value,state:&str)->Value {json!({"state":state,"citations":[{"field":"ingredients","quote":q["request"]["product"]["ingredients"]["data"]["value"]}],"reasoning":"Synthetic automated proposal: the exact known ingredient text describes the pinned plain dry rolled-oat input; no actual product suitability or freshness is asserted."})}
fn returned(q:&Value,state:&str)->Value {json!({"format":"mstrchf-product-semantic-return","version":1,"request_id":q["request"]["request_id"],"input_digest":q["request"]["input_digest"],"identity":decision(q,state),
    "cooking":q["request"]["methods"].as_array().unwrap().iter().map(|m|json!({"method_handle":m["method_handle"],"decision":decision(q,state)})).collect::<Vec<_>>()})}
fn evaluate(s:&Value,q:&Value,r:&Value,at:&str)->Value {good(json!({"op":"product_semantic_evaluate","state":s,"prepared":q,"returned":r,"evidence_id":"ev_80000000000000000000000000000092","now":at}))}
fn audit(q:&Value,r:Value,record:Value)->Value {call(json!({"op":"product_semantic_audit","prepared":q,"returned":r,"recorded":record}))}
fn resign(q:&mut Value){let typed:mstrchf_browser::product_semantic_review::PreparedProductSemanticReview=serde_json::from_value(q.clone()).unwrap();q["prepared_digest"]=json!(mstrchf_browser::product_semantic_review::prepared_digest(&typed).unwrap());}
fn current(s:&Value,q:&Value,r:&Value,at:&str)->Value {call(json!({"op":"product_semantic_check_return","state":s,"prepared":q,"returned":r,"now":at}))}
fn forbid_keys(v:&Value){match v{Value::Object(m)=>for(k,v)in m{assert!(!["person","person_id","diner_id","slot","extra_id","occurrence","occurrences","context","context_stamp","week_id","line_id","demand","uses","origins","household_id","privacy_policy_revision","account_ref","delivery_context","reader","native_task_ref","accepted_ref","source_prepared"].contains(&k.as_str()),"outbound private key {k}");forbid_keys(v);},Value::Array(v)=>for v in v{forbid_keys(v)},_=>{}}}
const POLICY_DIGEST:&str="10328a1e4c7532c9c4d8084f22bf6d813cf3ddc6c485dda680c99b281c47d4d0";
fn qualify(s:&Value,q:&Value,recorded:&Value,at:&str)->Value {good(json!({"op":"product_meaning_qualify","state":s,"prepared":q,"recorded":recorded,"now":at}))}
fn qualification_call(s:&Value,q:&Value,recorded:&Value,at:&str)->Value {call(json!({"op":"product_meaning_qualify","state":s,"prepared":q,"recorded":recorded,"now":at}))}
fn recorded(s:&Value,q:&Value,claim:&str,at:&str)->Value {evaluate(s,q,&returned(q,claim),at)}
fn inert_qualification(value:&Value){assert_eq!(value["format"],"mstrchf-product-meaning-qualification");assert_eq!(value["version"],1);assert_eq!(value["effect_enabled"],false);assert_eq!(value["policy"]["digest"],POLICY_DIGEST);assert_eq!(value["policy"]["max_age_seconds"],3600);assert_eq!(value["policy"]["revision"],"1");assert!(value["policy"]["notice"].as_str().unwrap().contains("not Tesco's expiry or proof"));for forbidden in ["admission","match_basis","matched_candidate","quote","pack_coverage","valid_until","permission","intent","grant_token","host_attested"]{assert!(value.get(forbidden).is_none(),"unexpected field {forbidden}");}}

#[test]
fn exact_compiled_policy_and_both_pinned_oats_methods_produce_real_readonly_conditional_candidate(){
 let(s,_,_,_,q)=fixture();let original=s.clone();let record=recorded(&s,&q,"supported",NOW);let value=qualify(&s,&q,&record,NOW);inert_qualification(&value);
 assert_eq!(value["status"],"conditional_match");assert_eq!(value["native_assessment"],"suitable");assert_eq!(value["reasons"],json!([]));assert!(value["match_basis_digest"].as_str().unwrap().len()==64);
 let ctx=context(&s);let actual=ctx["lines"].as_array().unwrap().iter().find(|line|line["demand"]["line_id"]==value["line_id"]).unwrap();
 assert_eq!(value["pack_basis"],json!({"required":actual["demand"]["quantity"],"supply":quantity("500","g","dry")}));assert_eq!(value["pack_reasons"],json!([]));
 assert_eq!(value["pack_basis"]["required"],json!({"amount":{"numerator":"225","denominator":"2"},"unit":"g","form":"dry"}));
 assert_eq!(value["source_observed_at"],OBSERVED);assert_eq!(value["recheck_at"],"2026-10-13T11:00:01Z");assert_eq!(value["evaluated_at"],NOW);assert_eq!(value["request_id"],q["request"]["request_id"]);assert_eq!(value["product"],product());assert_eq!(value["line_id"],q["binding"]["line_id"]);
 let doc=mstrchf_browser::product_meaning_admission::policy_document().unwrap();assert_eq!(canonical_digest(&doc).unwrap().0,POLICY_DIGEST);assert_eq!(doc["recipe_refs"],json!([porridge_packet_ref().unwrap().recipe,larger_porridge_packet_ref().unwrap().recipe]));assert_eq!(doc["evidence_id"],"ev_80000000000000000000000000000054");assert_eq!(doc["source_truth_guarantee"],false);
 assert_eq!(good(json!({"op":"validate_state","state":s,"now":NOW})),original);assert_eq!(audit(&q,json!(record["document"]["returned"]),record)["ok"],true);
}
#[test]
fn real_core_keeps_unknown_refuted_contradictory_and_changed_cooking_unqualified(){
 let(s,_,_,_,q)=fixture();for(claim,native,reason)in[("unknown","unresolved","unknown_identity"),("refuted","unsuitable","refuted_identity"),("contradictory","unresolved","contradictory_identity")]{
  let r=recorded(&s,&q,claim,NOW);let value=qualify(&s,&q,&r,NOW);inert_qualification(&value);assert_eq!(value["status"],"unresolved");assert_eq!(value["native_assessment"],native);assert!(value["reasons"].as_array().unwrap().contains(&json!(reason)));assert_eq!(value["match_basis_digest"],Value::Null);
  assert_eq!(value["pack_basis"],Value::Null);assert!(value["pack_reasons"].as_array().unwrap().contains(&json!("not_conditional_match")));
 }
 let mut returned=returned(&q,"supported");returned["cooking"][0]["decision"]["state"]=json!("refuted");let r=evaluate(&s,&q,&returned,NOW);let value=qualify(&s,&q,&r,NOW);assert_eq!(value["status"],"unresolved");assert!(value["reasons"].as_array().unwrap().contains(&json!("changed_cooking")));
}
#[test]
fn original_observation_exclusive_age_limit_is_independent_of_task_expiry_and_later_review_time(){
 let(s,_,_,_,q)=fixture();let r=recorded(&s,&q,"supported",NOW);
 // The source AND semantic task deadline is11:00; retained local policy lasts
 // only until original item observation10:00:01+3600, not a new admission+3600.
 let before=qualify(&s,&q,&r,"2026-10-13T11:00:00Z");assert_eq!(before["status"],"conditional_match");assert_eq!(before["recheck_at"],"2026-10-13T11:00:01Z");
 for at in ["2026-10-13T11:00:01Z",LATER]{let expired=qualify(&s,&q,&r,at);inert_qualification(&expired);assert_eq!(expired["status"],"recheck_required");assert_eq!(expired["native_assessment"],"unresolved");assert_eq!(expired["match_basis_digest"],Value::Null);assert!(expired["reasons"].as_array().unwrap().contains(&json!("source_recheck_required")));assert!(expired["reasons"].as_array().unwrap().contains(&json!("missing_admission")));assert_eq!(expired["pack_basis"],Value::Null);assert!(expired["pack_reasons"].as_array().unwrap().contains(&json!("source_recheck_required")));}
 let later=recorded(&s,&q,"supported","2026-10-13T10:59:50Z");assert_eq!(qualify(&s,&q,&later,"2026-10-13T10:59:55Z")["recheck_at"],"2026-10-13T11:00:01Z");
 assert_eq!(qualification_call(&s,&q,&later,NOW)["ok"],false);assert_eq!(qualification_call(&s,&q,&r,"2026-10-13T09:59:59Z")["ok"],false);
}
#[test]
fn known_raw_pack_is_mismatch_unknown_pack_survives_without_becoming_coverage(){
 let(s,p,mut o,_,_)=fixture();for(net,status,reason)in[(known(quantity("500","g","raw")),"unresolved",Some("pack_measure_mismatch")),(unknown(),"conditional_match",None)]{
  o["body"]["data"]["items"][0]["pack"]["net"]=net.clone();let a=accepted(&s,&p,&o);let q=semantic(&s,&p,&o,&a,line(&s,"01"),NOW,DEADLINE);let r=recorded(&s,&q,"supported",NOW);let value=qualify(&s,&q,&r,NOW);inert_qualification(&value);assert_eq!(value["status"],status);assert_eq!(q["request"]["product"]["pack"]["net"],net);assert_eq!(value["pack_basis"],Value::Null);if let Some(reason)=reason{assert!(value["reasons"].as_array().unwrap().contains(&json!(reason)));assert!(value["pack_reasons"].as_array().unwrap().contains(&json!(reason)));assert_eq!(value["match_basis_digest"],Value::Null);}else{assert!(value["match_basis_digest"].as_str().is_some());assert_eq!(value["pack_reasons"],json!(["pack_net_unknown"]));}
 }
}
#[test]
fn unknown_or_conflicting_ingredient_dependencies_never_gain_positive_admission(){
 let(s,p,mut o,_,_)=fixture();for(fact,claim,reason)in[(unknown(),"unknown","unknown_identity"),(json!({"kind":"conflicting","data":{"values":["Synthetic rolled oats","Synthetic wheat cereal"]}}),"contradictory","contradictory_identity")]{
  o["body"]["data"]["items"][0]["ingredients"]=fact.clone();let a=accepted(&s,&p,&o);let q=semantic(&s,&p,&o,&a,line(&s,"01"),NOW,DEADLINE);let mut payload=returned(&q,claim);payload["identity"]["citations"]=json!([]);for c in payload["cooking"].as_array_mut().unwrap(){c["decision"]["citations"]=json!([]);}let r=evaluate(&s,&q,&payload,NOW);let value=qualify(&s,&q,&r,NOW);assert_eq!(value["status"],"unresolved");assert_eq!(value["match_basis_digest"],Value::Null);assert!(value["reasons"].as_array().unwrap().contains(&json!(reason)));assert_eq!(q["request"]["product"]["ingredients"],fact);
 }
}
#[test]
fn changed_food_privacy_or_altered_retained_source_custody_does_not_requalify_history(){
 let(s,_,_,_,q)=fixture();let record=recorded(&s,&q,"supported",NOW);let changed=privacy(&s,false,0xdb10);assert_eq!(qualification_call(&changed,&q,&record,NOW)["ok"],false);
 let mut p=s["snapshot"]["profile"].clone();p["diners"][0]["display_name"]=json!("Synthetic changed current context");let changed=apply(&s,json!({"kind":"update_profile","data":{"expected_profile_revision":p["revision"],"profile":p}}),0xdb11);assert_eq!(qualification_call(&changed,&q,&record,NOW)["ok"],false);
 let mut w=s["snapshot"]["weeks"][0].clone();w["plan"]["selections"].as_array_mut().unwrap().clear();let changed=apply(&s,week_body(&w),0xdb12);assert_eq!(qualification_call(&changed,&q,&record,NOW)["ok"],false);
 // Pure native validation cannot manufacture a host human witness/canonical
 // source selection: that remains the owner gate. Altering retained custody
 // does not pass these native correlation/historical checks either.
 let mut bad=q.clone();bad["binding"]["accepted"]["digest"]=json!("a".repeat(64));resign(&mut bad);assert_eq!(qualification_call(&s,&bad,&record,NOW)["ok"],false);
}
#[test]
fn policy_allowance_or_incoming_admission_cannot_override_exact_recorded_document_and_citations(){
 let(s,_,_,_,q)=fixture();let record=recorded(&s,&q,"supported",NOW);
 for attack in ["meaning","review_pin","source","reasoning","future_review"]{let mut bad=record.clone();match attack{
  "meaning"=>bad["document"]["meaning"]["key_claim"]["state"]=json!({"kind":"refuted"}),"review_pin"=>bad["review_pin"]["digest"]=json!("f".repeat(64)),"source"=>bad["document"]["source_observation_digest"]=json!("f".repeat(64)),"reasoning"=>bad["document"]["returned"]["identity"]["reasoning"]=json!("Synthetic changed decision"),_=>bad["document"]["reviewed_at"]=json!("2026-10-13T10:55:00Z")};assert_eq!(qualification_call(&s,&q,&bad,NOW)["ok"],false,"{attack}");}
 for(key,value)in[("admission",json!({"valid_until":"2099-01-01T00:00:00Z"})),("policy",json!({"max_age_seconds":999999})),("time_allowance",json!(999999))]{let mut input=json!({"op":"product_meaning_qualify","state":s,"prepared":q,"recorded":record,"now":LATER});input[key]=value;assert_eq!(call(input)["ok"],false);}
}
#[test]
fn repeated_recipe_occurrences_keep_complete_native_cooking_closure(){
 let s=select(&porridge(),9,porridge_packet_ref().unwrap().recipe,None,0xdb30);let p=prepare(&s);let o=observation(&p,known(quantity("500","g","dry")),unknown());let a=accepted(&s,&p,&o);let q=semantic(&s,&p,&o,&a,line(&s,"01"),NOW,DEADLINE);let r=recorded(&s,&q,"supported",NOW);
 assert_eq!(q["request"]["methods"].as_array().unwrap().len(),2);assert_eq!(r["document"]["meaning"]["cooking_claims"].as_array().unwrap().len(),3);let value=qualify(&s,&q,&r,NOW);inert_qualification(&value);assert_eq!(value["status"],"conditional_match");assert_eq!(value["native_assessment"],"suitable");assert!(value["match_basis_digest"].as_str().is_some());
 let mut changed=r.clone();changed["document"]["meaning"]["cooking_claims"].as_array_mut().unwrap().pop();assert_eq!(qualification_call(&s,&q,&changed,NOW)["ok"],false);
}

#[test]
fn pack_inputs_require_known_single_count_and_unambiguous_net_without_multipack_or_drained_guessing(){
 let(s,p,original,_,_)=fixture();
 let cases=[(unknown(),known(quantity("500","g","dry")),"pack_count_unknown"),
  (json!({"kind":"conflicting","data":{"values":[1,2]}}),known(quantity("500","g","dry")),"pack_count_conflicting"),
  (known(json!(2)),known(quantity("500","g","dry")),"pack_count_not_one"),
  (known(json!(1)),json!({"kind":"conflicting","data":{"values":[quantity("500","g","dry"),quantity("600","g","dry")]}}),"pack_net_conflicting"),
  (known(json!(1)),known(quantity("500","ml","dry")),"pack_measure_mismatch")];
 for(count,net,reason)in cases {
  let mut o=original.clone();o["body"]["data"]["items"][0]["pack"]["count"]=count.clone();o["body"]["data"]["items"][0]["pack"]["net"]=net.clone();
  // A known drained amount cannot repair unknown/ambiguous/wrong net supply.
  o["body"]["data"]["items"][0]["pack"]["drained"]=known(quantity("400","g","drained"));
  let a=accepted(&s,&p,&o);let q=semantic(&s,&p,&o,&a,line(&s,"01"),NOW,DEADLINE);let record=recorded(&s,&q,"supported",NOW);let value=qualify(&s,&q,&record,NOW);inert_qualification(&value);
  assert_eq!(value["pack_basis"],Value::Null);assert!(value["pack_reasons"].as_array().unwrap().contains(&json!(reason)),"{reason}: {value}");
  assert_eq!(q["request"]["product"]["pack"]["count"],count);assert_eq!(q["request"]["product"]["pack"]["net"],net);
 }
}

#[test]
fn unsupported_nonessential_confirmation_cannot_suppress_or_reduce_the_actual_oat_pack_demand(){
 let(s,_,_,_,_)=fixture();let ctx=context(&s);let demand=ctx["lines"].as_array().unwrap().iter().find(|line|line["demand"]["ingredient_id"]=="ig_80000000000000000000000000000001").unwrap()["demand"].clone();
 assert_eq!(demand["essential"],false);
 let state=apply(&s,json!({"kind":"confirm_essentials","data":{"confirmations":[{"line_id":demand["line_id"],"displayed_quantity":demand["quantity"],"demand_digest":ctx["demand_digest"],"choice":"have_enough","confirmed_at":CREATED}]}}),0xdc40);
 let current=context(&state);let actual=current["lines"].as_array().unwrap().iter().find(|line|line["demand"]["line_id"]==demand["line_id"]).unwrap();
 assert_eq!(actual["shopping_action"],"add_to_shop");assert_eq!(actual["demand"],demand);
 let p=prepare(&state);let mut o=observation(&p,known(quantity("500","g","dry")),unknown());o["body"]["data"]["items"][0]["ingredients"]=known(json!("Synthetic label: plain rolled oats (100%)."));
 let a=accepted(&state,&p,&o);let q=semantic(&state,&p,&o,&a,demand["line_id"].clone(),NOW,DEADLINE);let record=recorded(&state,&q,"supported",NOW);let value=qualify(&state,&q,&record,NOW);
 assert_eq!(value["pack_basis"],json!({"required":demand["quantity"],"supply":quantity("500","g","dry")}));assert_eq!(value["pack_reasons"],json!([]));
 // Frozen oats intentionally cannot reach have_enough. No synthetic pantry
 // capability or private fake candidate/action is introduced to test that arm.
 assert!(known_packet(&porridge_packet_ref().unwrap()).unwrap().record().packet.get("maker_relevant_essential_ingredient_ids").is_none());
}

#[test]
fn broader_policy_literal_locks_all_nine_recipe_packet_and_registry_pins_without_replacing_oats_policy(){
 let old=mstrchf_browser::product_meaning_admission::policy_document().unwrap();assert_eq!(canonical_digest(&old).unwrap().0,POLICY_DIGEST);
 let doc=mstrchf_browser::product_meaning_admission::expanded_policy_document().unwrap();assert_eq!(canonical_digest(&doc).unwrap().0,"de88bc73090de4e6ec151ea3061b2bdd3be5dbf80daf6bbb0941ef95b740d129");assert_eq!(doc["evidence_id"],"ev_80000000000000000000000000000055");assert_eq!(doc["max_age_seconds"],3600);assert_eq!(doc["anchor"],"item.observed_at");assert_eq!(doc["source_truth_guarantee"],false);
 let packets=doc["packets"].as_array().unwrap();assert_eq!(packets.len(),9);let expected=[porridge_packet_ref().unwrap(),aglio_packet_ref().unwrap(),three_bean_packet_ref().unwrap(),three_bean_two_unit_packet_ref().unwrap(),three_bean_two_unit_vegetarian_packet_ref().unwrap(),larger_porridge_packet_ref().unwrap(),three_bean_two_unit_chicken_extra_packet_ref().unwrap(),chilled_chicken_extra_packet_ref().unwrap(),three_bean_smaller_packet_ref().unwrap()];
 for(row,rf)in packets.iter().zip(expected){assert_eq!(*row,json!({"recipe_id":rf.recipe.recipe_id,"revision":rf.recipe.revision,"recipe_digest":rf.recipe.digest,"packet_digest":rf.packet_digest,"registry_digest":canonical_digest(&known_packet(&rf).unwrap().record().ingredient_registry).unwrap()}));}
 let(s,p,mut o,_,_)=fixture();o["body"]["data"]["items"][0]["pack"]["net"]=known(quantity("500","ml","raw"));o["body"]["data"]["items"][0]["ingredients"]=known(json!("Synthetic plain dairy milk source; not an actual product read."));let a=accepted(&s,&p,&o);let q=semantic_v2(&s,&p,&o,&a,line(&s,"03"),NOW,DEADLINE);let record=recorded(&s,&q,"supported",NOW);
 let fresh=qualify(&s,&q,&record,NOW);assert_eq!(fresh["policy"]["digest"],"de88bc73090de4e6ec151ea3061b2bdd3be5dbf80daf6bbb0941ef95b740d129");assert_eq!(fresh["pack_basis"],json!({"required":quantity("450","ml","raw"),"supply":quantity("500","ml","raw")}));assert_eq!(fresh["recheck_at"],"2026-10-13T11:00:01Z");
 let expired=qualify(&s,&q,&record,"2026-10-13T11:00:01Z");assert_eq!(expired["status"],"recheck_required");assert_eq!(expired["pack_basis"],Value::Null);assert_eq!(expired["match_basis_digest"],Value::Null);assert_eq!(expired["policy"],fresh["policy"]);assert_eq!(record["version"],2);assert_eq!(audit(&q,record["document"]["returned"].clone(),record)["ok"],true);
}

#[test]
fn real_current_aglio_essential_confirmation_blocks_purchases_without_reducing_full_demand(){
 let mut s=admit(&setup(),aglio_packet_ref().unwrap(),0xdf01);s=select(&s,5,aglio_packet_ref().unwrap().recipe,None,0xdf02);
 let ctx=context(&s);let demand=ctx["lines"].as_array().unwrap().iter().find(|l|l["demand"]["ingredient_id"]=="ig_80000000000000000000000000000005").unwrap()["demand"].clone();assert_eq!(demand["essential"],true);
 s=apply(&s,json!({"kind":"confirm_essentials","data":{"confirmations":[{"line_id":demand["line_id"],"displayed_quantity":demand["quantity"],"demand_digest":ctx["demand_digest"],"choice":"have_enough","confirmed_at":CREATED}]}}),0xdf03);
 let p=prepare(&s);let mut o=observation(&p,known(quantity("500","g","raw")),unknown());o["body"]["data"]["items"][0]["ingredients"]=known(json!("Synthetic plain extra virgin olive oil source; no actual retailer observation."));let a=accepted(&s,&p,&o);let q=semantic_v2(&s,&p,&o,&a,line(&s,"05"),NOW,DEADLINE);let record=recorded(&s,&q,"supported",NOW);let value=qualify(&s,&q,&record,NOW);
 assert_eq!(value["status"],"conditional_match");assert_eq!(value["native_assessment"],"suitable");assert_eq!(value["pack_basis"],Value::Null);assert_eq!(value["pack_reasons"],json!(["line_already_covered"]));let current=context(&s);let line=current["lines"].as_array().unwrap().iter().find(|l|l["demand"]["line_id"]==demand["line_id"]).unwrap();assert_eq!(line["demand"],demand);assert_eq!(line["shopping_action"],"have_enough");assert_eq!(value["effect_enabled"],false);
}
