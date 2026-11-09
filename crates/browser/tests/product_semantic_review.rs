//! Synthetic source correlation and actual compiled oats/native owner fixtures;
//! actors, source text, products and households are explicitly synthetic.
//! These fixtures prove no human witness, Tesco read, currentness, suitability,
//! ProductMeaningAdmission, pack quote or external permission.
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
    // Explicit legacy-wire fixture seam. The real prepare operation now returns
    // v2; old tests continue exercising exact v1 reconstruction, never silently
    // replacing the historical policy/projection expectations.
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
#[test]
fn public_minimisation_keeps_label_and_pinned_methods_without_usage_or_account_graph(){
    let(s,_,o,_,q)=fixture();let req=&q["request"];forbid_keys(req);assert_eq!(req["methods"].as_array().unwrap().len(),2);
    assert_eq!(req["product"]["ingredients"],o["body"]["data"]["items"][0]["ingredients"]);assert_eq!(req["product"]["allergens"],o["body"]["data"]["items"][0]["allergens"]);assert_eq!(req["product"]["storage"],unknown());
    let text=req.to_string();for private in ["synthetic-local-account","synthetic-public-source-context","synthetic-owner-bound-task-not-a-live-host-attestation",s["snapshot"]["profile"]["diners"][0]["diner_id"].as_str().unwrap(),q["binding"]["line_id"].as_str().unwrap()] {assert!(!text.contains(private),"outbound private value {private}");}
    for method in req["methods"].as_array().unwrap(){let r:RecipeRef=serde_json::from_value(method["recipe"].clone()).unwrap();let p=[porridge_packet_ref().unwrap(),larger_porridge_packet_ref().unwrap()].into_iter().find(|p|p.recipe==r).unwrap();let trusted=known_packet(&p).unwrap();assert_eq!(method["steps"],json!(trusted.recipe().steps));assert_eq!(method["prep"],json!(trusted.recipe().prep));assert_eq!(method["attribution"],json!(trusted.recipe().text_rights.attribution));assert!(method["condition_text"].to_string().contains("steel-cut"));assert!(method["method_handle"].as_str().unwrap().starts_with("mh_"));}
    assert_eq!(q["binding"]["draft"]["occurrences"].as_array().unwrap().len(),2);assert_eq!(audit(&q,Value::Null,Value::Null)["ok"],true);
}
#[test]
fn automated_meaning_covers_every_private_occurrence_but_native_matching_stays_missing_admission(){
    let(s,_,_,a,q)=fixture();let original=s.clone();let r=returned(&q,"supported");let record=evaluate(&s,&q,&r,NOW);
    assert_eq!(record["assessment"],json!({"meaning_status":"proposed","admission_status":"missing","native_reasons":["missing_admission"],"freshness_status":"unknown","effect_enabled":false}));
    assert_eq!(record["document"]["provenance"],"automated_model_proposal");assert_eq!(record["document"]["host_attested"],false);assert_eq!(record["document"]["source_observation_digest"],a["digest"]);
    let claims=record["document"]["meaning"]["cooking_claims"].as_array().unwrap();let occurrences=q["binding"]["draft"]["occurrences"].as_array().unwrap();assert_eq!(claims.len(),occurrences.len());
    for row in occurrences {let claim=claims.iter().find(|c|c["occurrence"]==row["occurrence"]).unwrap();assert_eq!(claim["recipe"],row["occurrence"]["recipe"]);assert_eq!(claim["evidence_ids"],json!(["ev_80000000000000000000000000000092"]));assert_ne!(claim["evidence_ids"],json!([a["evidence_id"]]));}
    assert_eq!(record["document"]["meaning"]["requirement_claims"],json!([]));assert_eq!(audit(&q,r.clone(),record.clone())["ok"],true);
    assert_eq!(good(json!({"op":"validate_state","state":s,"now":LATER})),original);
    for forbidden in ["admission","freshness_rule","valid_until","match_basis","matched_candidate","quote","intent","grant_token"]{assert!(record.get(forbidden).is_none());assert!(record["document"].get(forbidden).is_none());}
    let mut forged=record.clone();forged["document"]["meaning"]["cooking_claims"][0]["state"]=json!({"kind":"refuted"});assert_eq!(audit(&q,r.clone(),forged)["ok"],false);
    let mut forged=record;forged["assessment"]["effect_enabled"]=json!(true);assert_eq!(audit(&q,r,forged)["ok"],false);
}
#[test]
fn normalized_returns_refuse_missing_foreign_duplicate_methods_and_native_authority(){
    let(s,_,_,_,q)=fixture();let r=returned(&q,"supported");for attack in ["missing","duplicate","foreign","citation","request","native","quantity","actor","conversion","decision"] {let mut bad=r.clone();match attack{
        "missing"=>{bad["cooking"].as_array_mut().unwrap().pop();},"duplicate"=>bad["cooking"][1]=bad["cooking"][0].clone(),"foreign"=>bad["cooking"][0]["method_handle"]=json!("mh_ffffffffffffffffffffffffffffffff"),
        "citation"=>bad["identity"]["citations"][0]["quote"]=json!("Synthetic unobserved assertion"),"request"=>bad["request_id"]=json!("sr_ffffffffffffffffffffffffffffffff"),"native"=>bad["meaning_digest"]=json!("a".repeat(64)),"quantity"=>bad["identity"]["supply_quantity"]=quantity("100","g","dry"),"actor"=>bad["review_actor"]=json!("human"),"conversion"=>bad["conversion_evidence"]=json!([]),_=>bad["identity"]["state"]=json!("suitable")};assert_eq!(current(&s,&q,&bad,NOW)["ok"],false,"{attack}");}
    let mut too_big=r.clone();too_big["identity"]["reasoning"]=json!("x".repeat(4097));assert_eq!(current(&s,&q,&too_big,NOW)["ok"],false);
    let mut inert=r;inert["identity"]["reasoning"]=json!("<script>inert synthetic reasoning</script>");assert_eq!(current(&s,&q,&inert,NOW)["ok"],true);
}
#[test]
fn unknown_and_conflicting_dependencies_cannot_become_supported_known_citations(){
    let(s,p,mut o,_,_)=fixture();for fact in [unknown(),json!({"kind":"conflicting","data":{"values":["Synthetic rolled oats","Synthetic oat-free product"]}})]{
        o["body"]["data"]["items"][0]["ingredients"]=fact.clone();let a=accepted(&s,&p,&o);let q=semantic(&s,&p,&o,&a,line(&s,"01"),NOW,DEADLINE);
        let mut bad=returned(&q,"supported");bad["identity"]["citations"][0]["quote"]=json!("Synthetic rolled oats");assert_eq!(current(&s,&q,&bad,NOW)["ok"],false);
        let mut r=returned(&q,"unknown");r["identity"]["citations"]=json!([]);for c in r["cooking"].as_array_mut().unwrap(){c["decision"]["citations"]=json!([]);}
        let record=evaluate(&s,&q,&r,NOW);assert_eq!(record["document"]["meaning"]["key_claim"]["state"],json!({"kind":"unknown"}));assert_eq!(record["assessment"]["admission_status"],"missing");
        if fact["kind"]=="conflicting"{r["identity"]["state"]=json!("contradictory");assert_eq!(evaluate(&s,&q,&r,NOW)["document"]["meaning"]["key_claim"]["state"],json!({"kind":"contradictory"}));}
    }
    let(_,_,_,_,q)=fixture();let mut bad=returned(&q,"supported");bad["identity"]["citations"].as_array_mut().unwrap().push(json!({"field":"storage","quote":"Keep dry"}));assert_eq!(current(&s,&q,&bad,NOW)["ok"],false);
}
#[test]
fn resigned_prepared_copies_cannot_change_methods_graph_scope_or_source_custody(){
    let(_,_,_,_,q)=fixture();for attack in ["method","ingredient","condition","map","omission","foreign","hard_requirement","recipe"]{let mut bad=q.clone();match attack{
        "method"=>bad["request"]["methods"][0]["steps"][0]["text"]=json!("Synthetic forged instruction"),"ingredient"=>bad["binding"]["draft"]["occurrences"][0]["ingredient"]["quantity"]=quantity("99","g","dry"),
        "condition"=>bad["binding"]["draft"]["occurrences"][0]["condition_text"]=json!(["Synthetic arbitrary ingredient is allowed"]),"map"=>{bad["binding"]["method_map"][0]["occurrences"].as_array_mut().unwrap().clear();},"omission"=>{bad["binding"]["draft"]["occurrences"].as_array_mut().unwrap().pop();},
        "foreign"=>bad["binding"]["draft"]["line"]["origins"][0]["occurrence"]["recipe"]["digest"]=json!("a".repeat(64)),"hard_requirement"=>bad["binding"]["draft"]["line"]["uses"][0]["requirements"]=json!([{"kind":"diet","data":{"code":"vegan"}}]),_=>bad["binding"]["draft"]["occurrences"][0]["recipe"]["title"]=json!("Synthetic forged recipe")};resign(&mut bad);assert_eq!(audit(&bad,Value::Null,Value::Null)["ok"],false,"{attack}");}
}
#[test]
fn current_food_privacy_and_task_lifetime_stale_but_historical_audit_remains_exact(){
    let(s,_,_,_,q)=fixture();let r=returned(&q,"supported");let record=evaluate(&s,&q,&r,NOW);
    let changed=privacy(&s,false,0xda10);assert_eq!(current(&changed,&q,&r,NOW)["ok"],false);assert_eq!(current(&s,&q,&r,DEADLINE)["ok"],false);assert_eq!(current(&s,&q,&r,"2026-10-13T10:04:59Z")["ok"],false);
    let mut profile=s["snapshot"]["profile"].clone();profile["diners"][0]["display_name"]=json!("Synthetic current context changed");let changed=apply(&s,json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),0xda11);assert_eq!(current(&changed,&q,&r,NOW)["ok"],false);
    assert_eq!(audit(&q,r.clone(),record.clone())["ok"],true);let mut late=record;late["document"]["reviewed_at"]=json!(DEADLINE);assert_eq!(audit(&q,r,late)["ok"],false);
}
#[test]
fn source_request_expiry_is_not_semantic_or_product_freshness(){
    let(s,p,o,a,_)=fixture();let q=semantic(&s,&p,&o,&a,line(&s,"01"),LATER,"2026-10-13T12:00:00Z");let r=returned(&q,"supported");let record=evaluate(&s,&q,&r,LATER);
    assert_eq!(record["assessment"]["freshness_status"],"unknown");assert_eq!(record["assessment"]["native_reasons"],json!(["missing_admission"]));assert_eq!(q["request"]["product"]["observed_at"],OBSERVED);assert_eq!(q["binding"]["source_prepared"]["request"]["expires_at"],DEADLINE);
    assert_eq!(audit(&q,r,record)["ok"],true);
}
#[test]
fn legacy_oats_scope_stays_bounded_while_v2_explicitly_supports_other_pinned_inputs(){
    let(s,p,o,a,_)=fixture();let q=semantic_v2(&s,&p,&o,&a,line(&s,"02"),NOW,DEADLINE);assert_eq!(q["version"],2);
    let legacy=semantic(&s,&p,&o,&a,line(&s,"02"),NOW,DEADLINE);let refused=audit(&legacy,Value::Null,Value::Null);assert_eq!(refused["ok"],false);assert_eq!(refused["error"]["code"],"unsupported_capability");
    let s=privacy(&s,false,0xda20);let p=prepare(&s);let o=observation(&p,unknown(),unknown());let typed:ReadObservationV1=serde_json::from_value(o.clone()).unwrap();
    // Explicit synthetic correlation double: this is not a real accepted owner
    // source, witness, AI-off source-return bypass or external read authority.
    let a=json!(AcceptedReadRef{evidence_id:EvidenceId("ev_80000000000000000000000000000093".into()),job_id:typed.job_id.clone(),read_id:typed.read_id.clone(),observation_id:typed.observation_id.clone(),digest:canonical_digest(&typed).unwrap(),resource:typed.resource.clone(),observed_at:typed.observed_at.clone(),admitted_at:Instant(NOW.into())});
    let q=semantic(&s,&p,&o,&a,line(&s,"01"),NOW,DEADLINE);assert_eq!(audit(&q,Value::Null,Value::Null)["ok"],true);let r=returned(&q,"unknown");let result=current(&s,&q,&r,NOW);assert_eq!(result["ok"],false);assert_eq!(result["error"]["code"],"authority_denied");
}

#[test]
fn repeated_recipe_uses_one_public_method_but_every_private_occurrence_gets_a_cooking_claim(){
    let s=select(&porridge(),9,porridge_packet_ref().unwrap().recipe,None,0xda30);
    let p=prepare(&s);let o=observation(&p,known(quantity("500","g","dry")),unknown());let a=accepted(&s,&p,&o);
    let q=semantic(&s,&p,&o,&a,line(&s,"01"),NOW,DEADLINE);
    assert_eq!(q["request"]["methods"].as_array().unwrap().len(),2);forbid_keys(&q["request"]);
    assert_eq!(q["binding"]["draft"]["occurrences"].as_array().unwrap().len(),3);
    let r3=json!(porridge_packet_ref().unwrap().recipe);
    let m=q["request"]["methods"].as_array().unwrap().iter().find(|m|m["recipe"]==r3).unwrap();
    let map=q["binding"]["method_map"].as_array().unwrap().iter().find(|map|map["method_handle"]==m["method_handle"]).unwrap();
    assert_eq!(map["occurrences"].as_array().unwrap().len(),2);
    let r=returned(&q,"supported");assert_eq!(r["cooking"].as_array().unwrap().len(),2);
    let record=evaluate(&s,&q,&r,NOW);let claims=record["document"]["meaning"]["cooking_claims"].as_array().unwrap();assert_eq!(claims.len(),3);
    for occurrence in map["occurrences"].as_array().unwrap(){assert_eq!(claims.iter().filter(|c|&c["occurrence"]==occurrence).count(),1);}
    assert_eq!(audit(&q,r,record)["ok"],true);
}

fn choose_options(s:&Value,index:usize,rf:RecipeRef,options:Value,id:u32)->Value{
 let w=&s["snapshot"]["weeks"][0];let board=good(json!({"op":"board","state":s,"week_id":w["range"]["week_id"],"slot":w["cells"][index]["slot"],"options":options,"now":CREATED}));
 let choice=board["choices"].as_array().unwrap().iter().find(|c|c["selection"]["recipe"]==json!(rf)).expect("exact qualified synthetic choice");let mut w=w.clone();w["plan"]=choice["plan"].clone();apply(s,week_body(&w),id)
}
fn expanded_food(two:bool,vegetarian:bool,extra:bool)->Value{
 let mut s=admit(&porridge(),aglio_packet_ref().unwrap(),0xde01);s=select(&s,5,aglio_packet_ref().unwrap().recipe,None,0xde02);
 if two{
  let mut p=s["snapshot"]["profile"].clone();let mut second=p["diners"][0].clone();second["diner_id"]=json!("dn_80000000000000000000000000000091");
  if vegetarian{second["restrictions"]=json!({"kind":"confirmed","data":{"confirmed_at":CREATED,"requirements":[{"kind":"diet","data":{"code":"vegetarian"}}]}});}
  p["diners"].as_array_mut().unwrap().push(second);s=apply(&s,json!({"kind":"update_profile","data":{"expected_profile_revision":p["revision"],"profile":p}}),0xde03);
  let mut w=s["snapshot"]["weeks"][0].clone();w["cells"][4]["attendee_ids"].as_array_mut().unwrap().push(json!("dn_80000000000000000000000000000091"));s=apply(&s,week_body(&w),0xde04);
 }
 let rf=if extra{
  s=admit(&s,chilled_chicken_extra_packet_ref().unwrap(),0xde05);s=admit(&s,three_bean_two_unit_chicken_extra_packet_ref().unwrap(),0xde06);three_bean_two_unit_chicken_extra_packet_ref().unwrap()
 }else if two{s=admit(&s,three_bean_two_unit_vegetarian_packet_ref().unwrap(),0xde07);three_bean_two_unit_vegetarian_packet_ref().unwrap()}
 else{s=admit(&s,three_bean_packet_ref().unwrap(),0xde08);three_bean_packet_ref().unwrap()};
 let options=if extra{json!({"extra_recipients":[{"kind":"diner","data":{"diner_id":s["snapshot"]["profile"]["diners"][0]["diner_id"]}}]})}else{Value::Null};
 choose_options(&s,4,rf.recipe,options,0xde09)
}
fn expanded_fixture(s:&Value,suffix:&str,net:Value,drained:Value,storage:Value)->(Value,Value,Value){
 let p=prepare(s);let mut o=observation(&p,net,drained);o["body"]["data"]["items"][0]["ingredients"]=known(json!("Synthetic exact plain ingredient label and recipe-compatible composition; no actual Tesco observation occurred."));o["body"]["data"]["items"][0]["storage"]=storage;
 let a=accepted(s,&p,&o);let q=semantic_v2(s,&p,&o,&a,line(s,suffix),NOW,DEADLINE);(o,a,q)
}
fn storage_return(q:&Value,claim:&str)->Value{
 let mut r=returned(q,claim);for row in r["cooking"].as_array_mut().unwrap(){row["decision"]["citations"].as_array_mut().unwrap().push(json!({"field":"storage","quote":q["request"]["product"]["storage"]["data"]["value"]}));}r
}
fn qualify_v2(s:&Value,q:&Value,r:&Value)->Value{let record=evaluate(s,q,r,NOW);good(json!({"op":"product_meaning_qualify","state":s,"prepared":q,"recorded":record,"now":NOW}))}

#[test]
fn exact_saved_v24_v1_prepared_and_record_reencode_without_byte_changes_and_still_audit(){
 let fixture:Value=serde_json::from_slice(include_bytes!("fixtures/product-semantic-v1-v24.json")).unwrap();
 let prepared:mstrchf_browser::product_semantic_review::PreparedProductSemanticReview=serde_json::from_value(fixture["prepared"].clone()).unwrap();
 let returned:mstrchf_browser::product_semantic_review::PublicSemanticReturn=serde_json::from_value(fixture["returned"].clone()).unwrap();
 let recorded:mstrchf_browser::product_semantic_review::RecordedProductSemanticReview=serde_json::from_value(fixture["recorded"].clone()).unwrap();
 for(key,encoded)in[("prepared",json!(prepared)),("returned",json!(returned)),("recorded",json!(recorded))]{assert_eq!(encoded,fixture[key]);assert_eq!(serde_json::to_vec(&encoded).unwrap(),serde_json::to_vec(&fixture[key]).unwrap());assert_eq!(canonical_digest(&encoded).unwrap().0,fixture["expected"][key].as_str().unwrap());}
 assert_eq!(prepared.version,1);assert_eq!(recorded.version,1);assert_eq!(audit(&json!(prepared),json!(returned),json!(recorded))["ok"],true);
 assert_eq!(canonical_digest(&mstrchf_browser::product_meaning_admission::policy_document().unwrap()).unwrap().0,"10328a1e4c7532c9c4d8084f22bf6d813cf3ddc6c485dda680c99b281c47d4d0");
}

#[test]
fn new_v2_public_requests_cover_exact_milk_pasta_garlic_and_three_water_methods_without_private_graph(){
 let s=expanded_food(false,false,false);let original=s.clone();
 for(suffix,net,required)in[("03",quantity("500","ml","raw"),"450"),("04",quantity("500","g","dry"),"125"),("06",quantity("1","each","whole"),"2")]{
  let(_,_,q)=expanded_fixture(&s,suffix,known(net.clone()),unknown(),unknown());assert_eq!(q["version"],2);assert_eq!(q["request"]["version"],2);forbid_keys(&q["request"]);
  let record=evaluate(&s,&q,&returned(&q,"supported"),NOW);assert_eq!(record["version"],2);assert_eq!(record["document"]["returned"]["version"],1);assert_eq!(record["document"]["meaning"]["requirement_claims"],json!([]));
  let value=good(json!({"op":"product_meaning_qualify","state":s,"prepared":q,"recorded":record,"now":NOW}));assert_eq!(value["status"],"conditional_match");assert_eq!(value["native_assessment"],"suitable");assert_eq!(value["pack_basis"]["supply"],net);assert_eq!(value["pack_basis"]["required"]["amount"]["numerator"],required);assert_eq!(value["effect_enabled"],false);assert_eq!(value["policy"]["id"],"tesco-pinned-input-retained-interpretation-v1");
 }
 let(_,_,q)=expanded_fixture(&s,"02",known(quantity("2000","ml","raw")),unknown(),unknown());assert_eq!(q["request"]["methods"].as_array().unwrap().len(),3);forbid_keys(&q["request"]);assert_eq!(q["binding"]["method_map"].as_array().unwrap().len(),3);
 assert_eq!(good(json!({"op":"validate_state","state":s,"now":NOW})),original);
}

#[test]
fn v2_pinned_intrinsic_conditions_are_present_and_resigned_omissions_or_foreign_inputs_do_not_audit(){
 let s=expanded_food(true,false,false);let(_,_,q)=expanded_fixture(&s,"07",known(quantity("400","g","raw")),known(quantity("240","g","drained")),known(json!("Synthetic ready-to-eat canned input; serve directly cold; follow refrigeration/opening labels.")));
 let method=&q["request"]["methods"][0];assert!(method["condition_text"].to_string().contains("processing aids"));assert!(method["condition_text"].to_string().contains("exact known ingredients and storage"));
 assert_eq!(method["intrinsic_conditions"],q["binding"]["draft"]["occurrences"][0]["intrinsic_conditions"]);assert_eq!(method["intrinsic_conditions"]["table"]["entries"].as_array().unwrap().len(),9);forbid_keys(&q["request"]);
 for attack in ["table","condition","definition","recipe","extra"]{let mut bad=q.clone();match attack{
  "table"=>bad["binding"]["draft"]["occurrences"][0]["intrinsic_conditions"]["entry"]["input_condition"]=json!("Any product is fine"),
  "condition"=>bad["binding"]["draft"]["occurrences"][0]["condition_text"]=json!([]),"definition"=>bad["binding"]["draft"]["occurrences"][0]["input_definition"]["form"]=json!("raw"),
  "recipe"=>bad["binding"]["draft"]["occurrences"][0]["recipe"]["steps"][0]["text"]=json!("Forged method"),_=>bad["binding"]["draft"]["occurrences"][0]["occurrence"]["extra_id"]=json!("ex_80000000000000000000000000000099")};resign(&mut bad);assert_eq!(audit(&bad,Value::Null,Value::Null)["ok"],false,"{attack}");}
}

#[test]
fn cold_input_unknown_or_conflicting_storage_cannot_gain_conditional_match_from_known_ingredients(){
 let s=expanded_food(false,false,false);
 for(storage,claim,reason)in[(unknown(),"unknown","unknown_cooking"),(json!({"kind":"conflicting","data":{"values":["Synthetic ready-to-eat label","Synthetic cook-before-eating label"]}}),"contradictory","contradictory_cooking")]{
  let(_,_,q)=expanded_fixture(&s,"07",known(quantity("400","g","raw")),known(quantity("240","g","drained")),storage);
  let mut proposed=returned(&q,"supported");for cooking in proposed["cooking"].as_array_mut().unwrap(){cooking["decision"]["state"]=json!(claim);}
  let value=qualify_v2(&s,&q,&proposed);assert_eq!(value["status"],"unresolved");assert_eq!(value["pack_basis"],Value::Null);assert!(value["reasons"].as_array().unwrap().contains(&json!(reason)));
  let positive=returned(&q,"supported");assert_eq!(current(&s,&q,&positive,NOW)["ok"],false);assert_eq!(call(json!({"op":"product_semantic_evaluate","state":s,"prepared":q,"returned":positive,"evidence_id":"ev_80000000000000000000000000000092","now":NOW}))["ok"],false);
 }
}

#[test]
fn drained_pack_uses_only_drained_measure_and_true_vegetarian_consumers_remain_native_unresolved(){
 let storage=known(json!("Synthetic ready-to-eat canned input; direct cold use and label storage instructions."));let s=expanded_food(true,false,false);
 for(drained,reason)in[(known(quantity("240","g","drained")),None),(unknown(),Some("pack_drained_unknown")),(json!({"kind":"conflicting","data":{"values":[quantity("240","g","drained"),quantity("250","g","drained")]}}),Some("pack_drained_conflicting")),(known(quantity("240","g","raw")),Some("pack_measure_mismatch"))]{
  let(_,_,q)=expanded_fixture(&s,"07",known(quantity("999","g","raw")),drained.clone(),storage.clone());let value=qualify_v2(&s,&q,&storage_return(&q,"supported"));
  if let Some(reason)=reason{assert_eq!(value["pack_basis"],Value::Null);assert!(value["pack_reasons"].as_array().unwrap().contains(&json!(reason)));}else{assert_eq!(value["status"],"conditional_match");assert_eq!(value["pack_basis"],json!({"required":quantity("160","g","drained"),"supply":quantity("240","g","drained")}));}
  assert_eq!(q["request"]["product"]["pack"]["net"],known(quantity("999","g","raw")));assert_eq!(q["request"]["product"]["pack"]["drained"],drained);
 }
 let vegetarian=expanded_food(true,true,false);let(_,_,q)=expanded_fixture(&vegetarian,"07",known(quantity("400","g","raw")),known(quantity("240","g","drained")),storage);
 assert!(q["binding"]["draft"]["line"]["uses"].to_string().contains("vegetarian"));forbid_keys(&q["request"]);
 let record=evaluate(&vegetarian,&q,&storage_return(&q,"supported"),NOW);assert_eq!(record["document"]["meaning"]["requirement_claims"],json!([]));let value=good(json!({"op":"product_meaning_qualify","state":vegetarian,"prepared":q,"recorded":record,"now":NOW}));assert_eq!(value["status"],"unresolved");assert_eq!(value["native_assessment"],"unresolved");assert!(value["reasons"].as_array().unwrap().contains(&json!("unsupported_policy")));assert_eq!(value["pack_basis"],Value::Null);
}

#[test]
fn exact_extra_occurrence_retains_private_recipient_graph_and_null_source_with_no_standalone_authority(){
 let s=expanded_food(true,true,true);let original=s.clone();let(_,_,q)=expanded_fixture(&s,"10",known(quantity("100","g","cooked")),unknown(),known(json!("Synthetic fully cooked chilled ready-to-eat plain chicken. Direct cold use permitted; keep refrigerated and comply with actual dates/labels.")));
 assert_eq!(q["request"]["methods"].as_array().unwrap().len(),1);assert_eq!(q["request"]["methods"][0]["source_url"],Value::Null);assert_eq!(q["request"]["methods"][0]["recipe"],json!(chilled_chicken_extra_packet_ref().unwrap().recipe));forbid_keys(&q["request"]);
 let record=evaluate(&s,&q,&storage_return(&q,"supported"),NOW);let claims=record["document"]["meaning"]["cooking_claims"].as_array().unwrap();assert_eq!(claims.len(),1);assert_eq!(claims[0]["occurrence"]["extra_id"],"ex_80000000000000000000000000000002");assert_eq!(record["document"]["meaning"]["requirement_claims"],json!([]));
 let value=good(json!({"op":"product_meaning_qualify","state":s,"prepared":q,"recorded":record,"now":NOW}));assert_eq!(value["status"],"conditional_match");assert_eq!(value["pack_basis"],json!({"required":quantity("50","g","cooked"),"supply":quantity("100","g","cooked")}));assert_eq!(value["effect_enabled"],false);assert_eq!(good(json!({"op":"validate_state","state":s,"now":NOW})),original);
}

#[test]
fn exact_saved_v54_v2_prepared_and_record_reencode_without_byte_changes_and_still_audit(){
 let fixture:Value=serde_json::from_slice(include_bytes!("fixtures/product-semantic-v2-v54.json")).unwrap();
 let prepared:mstrchf_browser::product_semantic_review::PreparedProductSemanticReview=serde_json::from_value(fixture["prepared"].clone()).unwrap();
 let returned:mstrchf_browser::product_semantic_review::PublicSemanticReturn=serde_json::from_value(fixture["returned"].clone()).unwrap();
 let recorded:mstrchf_browser::product_semantic_review::RecordedProductSemanticReview=serde_json::from_value(fixture["recorded"].clone()).unwrap();
 for(key,encoded)in[("prepared",json!(prepared)),("returned",json!(returned)),("recorded",json!(recorded))]{assert_eq!(encoded,fixture[key]);assert_eq!(serde_json::to_vec(&encoded).unwrap(),serde_json::to_vec(&fixture[key]).unwrap());assert_eq!(canonical_digest(&encoded).unwrap().0,fixture["expected"][key].as_str().unwrap());}
 assert_eq!(prepared.version,2);assert_eq!(recorded.version,2);assert_eq!(audit(&json!(prepared),json!(returned),json!(recorded))["ok"],true);
 assert_eq!(canonical_digest(&mstrchf_browser::product_meaning_admission::expanded_policy_document().unwrap()).unwrap().0,"de88bc73090de4e6ec151ea3061b2bdd3be5dbf80daf6bbb0941ef95b740d129");
}
