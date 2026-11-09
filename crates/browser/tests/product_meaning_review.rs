//! Real public native boundary, source correlation and maker packet custody;
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
fn inert(d:&Value) {
    assert_eq!(d["format"],"mstrchf-product-meaning-review-draft");assert_eq!(d["version"],1);
    for(key,value)in [("meaning_status","not_reviewed"),("admission_status","missing"),("match_status","not_assessed"),("purchase_readiness","not_assessed")] {assert_eq!(d[key],value);}
    assert_eq!(d["freshness"],json!({"status":"unknown","valid_until":null}));assert_eq!(d["effect_enabled"],false);
    assert_eq!(d["source"]["provenance"],"retained_source_correlation_only");assert_eq!(d["source"]["host_attested"],false);assert_eq!(d["source"]["source_truth_assessed"],false);
    for forbidden in ["match_basis","meaning_digest","matched_candidate","freshness_rule","policy_digest","intent","quote","grant_token"] {assert!(d.get(forbidden).is_none(),"unexpected authority field {forbidden}");}
}

#[test]
fn exact_r3_and_larger_uses_origins_and_methods_are_readable_without_meaning_or_freshness_admission() {
    let s=porridge();let before=s.clone();let p=prepare(&s);let o=observation(&p,known(quantity("500","g","dry")),unknown());let a=accepted(&s,&p,&o);let id=line(&s,"01");
    let d=good(input(&s,&p,&o,&a,id.clone(),NOW));inert(&d);assert_eq!(d["food_binding"],p["food"]);
    let ctx=context(&s);let native=ctx["lines"].as_array().unwrap().iter().find(|l|l["demand"]["line_id"]==id).unwrap();for key in ["demand","shopping_action","uses","origins"] {assert_eq!(d["line"][key],native[key]);}
    assert_eq!(d["occurrences"].as_array().unwrap().len(),2);let refs=[porridge_packet_ref().unwrap(),larger_porridge_packet_ref().unwrap()];
    for r in refs {let row=d["occurrences"].as_array().unwrap().iter().find(|row|row["occurrence"]["recipe"]==json!(r.recipe)).unwrap();let packet=known_packet(&r).unwrap();assert_eq!(row["recipe"],json!(packet.recipe()));assert_eq!(row["packet_ref"],json!(r));assert_eq!(row["checklist_status"],"prose_only_requires_semantic_review");assert!(row["intrinsic_conditions"].is_null());assert_eq!(row["input_definition"],packet.record().ingredient_registry["entries"][0]);assert!(!row["evidence_refs"].as_array().unwrap().is_empty());assert!(row["condition_text"].to_string().contains("steel-cut"));}
    assert_eq!(d["source"]["item"],o["body"]["data"]["items"][0]);assert_eq!(d["field_dependencies"][0]["status"],"known");assert_eq!(d["field_dependencies"][1]["status"],"conflicting");assert_eq!(d["field_dependencies"][2]["status"],"unknown");assert_eq!(d["pack_comparison"]["status"],"comparable_declared_measure");assert_eq!(s,before);
    assert_eq!(good(json!({"op":"validate_state","state":s,"now":NOW})),before);
}

#[test]
fn source_and_current_selectors_refuse_tampering_stale_food_and_incoming_authority_fields() {
    let s=porridge();let p=prepare(&s);let o=observation(&p,known(quantity("500","g","dry")),unknown());let a=accepted(&s,&p,&o);let request=input(&s,&p,&o,&a,line(&s,"01"),NOW);
    for change in ["product","line","digest","reader","week","authority"] {let mut bad=request.clone();match change {
        "product"=>bad["product"]["product_id"]=json!("987654321"),"line"=>{let old=bad["line_id"].as_str().unwrap();let last=if old.ends_with('f'){'e'}else{'f'};bad["line_id"]=json!(format!("{}{last}",&old[..old.len()-1]));},
        "digest"=>bad["accepted"]["digest"]=json!("a".repeat(64)),"reader"=>bad["observation"]["reader"]["native_task_ref"]=json!("synthetic-foreign-task"),
        "week"=>bad["week_id"]=s["snapshot"]["weeks"][1]["range"]["week_id"].clone(),_=>bad["meaning_admission"]=json!({"valid_until":DEADLINE,"supported":true})};refuse(bad);}
    let mut profile=s["snapshot"]["profile"].clone();profile["diners"][0]["display_name"]=json!("Synthetic changed context");let changed=apply(&s,json!({"kind":"update_profile","data":{"profile":profile,"expected_profile_revision":profile["revision"]}}),0xd910);
    assert_eq!(refuse(input(&changed,&p,&o,&a,line(&changed,"01"),NOW))["code"],"context_stale");
}

fn vegetarian()->Value {
    let s=admit(&setup(),three_bean_two_unit_vegetarian_packet_ref().unwrap(),0xd920);let mut w=s["snapshot"]["weeks"][0].clone();
    w["cells"][4]["occasion"]=json!({"occasion_id":"oc_80000000000000000000000000000090","host_diner_ids":w["cells"][4]["attendee_ids"],"guests":[{
        "guest_id":"gu_80000000000000000000000000000090","age_band":"adult","portion":{"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]},
        "restrictions":{"kind":"confirmed","data":{"confirmed_at":CREATED,"requirements":[{"kind":"diet","data":{"code":"vegetarian"}}]}}} ]});
    let s=apply(&s,week_body(&w),0xd921);select(&s,4,three_bean_two_unit_vegetarian_packet_ref().unwrap().recipe,None,0xd922)
}
#[test]
fn intrinsic_input_conditions_remain_exact_for_none_resident_and_vegetarian_guest() {
    let s=vegetarian();let p=prepare(&s);let o=observation(&p,known(quantity("400","g","cooked")),known(quantity("240","g","drained")));let a=accepted(&s,&p,&o);
    let d=good(input(&s,&p,&o,&a,line(&s,"07"),NOW));inert(&d);let row=&d["occurrences"][0];let packet=known_packet(&three_bean_two_unit_vegetarian_packet_ref().unwrap()).unwrap();
    assert_eq!(row["intrinsic_conditions"]["table"],packet.record().packet["intrinsic_input_conditions"]);assert_eq!(row["intrinsic_conditions"]["entry"],packet.record().packet["intrinsic_input_conditions"]["entries"][0]);assert_eq!(row["checklist_status"],"pinned_structured_conditions");assert!(row["condition_text"].to_string().contains("processing aids"));
    let consumers:Vec<_>=d["line"]["uses"].as_array().unwrap().iter().filter(|u|u["use_key"]["kind"]=="consumption").collect();assert_eq!(consumers.len(),2);
    assert!(consumers.iter().any(|u|u["use_key"]["data"]["person"]["kind"]=="diner"&&u["requirements"].as_array().unwrap().is_empty()));
    assert!(consumers.iter().any(|u|u["use_key"]["data"]["person"]["kind"]=="guest"&&u["requirements"]==json!([{"kind":"diet","data":{"code":"vegetarian"}}])));
    assert_eq!(d["line"]["origins"].as_array().unwrap().len(),1);assert_eq!(d["pack_comparison"]["measure"],"drained");
    // Dropping the guest's Vegetarian declaration does not drop the recipe's
    // all-nine adopted input definition. Conditions are readable text, not
    // hidden opaque JSON or assumed actual manufacturer/product compliance.
    let mut w=s["snapshot"]["weeks"][0].clone();w["cells"][4]["occasion"]["guests"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":CREATED}});
    let all_none=apply(&s,week_body(&w),0xd923);let p=prepare(&all_none);let o=observation(&p,unknown(),unknown());let a=accepted(&all_none,&p,&o);
    for suffix in ["07","08","09","0a","0b","0c","0d","0e","0f"] {
        let draft=good(input(&all_none,&p,&o,&a,line(&all_none,suffix),NOW));inert(&draft);
        let row=&draft["occurrences"][0];let exact=packet.record().packet["intrinsic_input_conditions"]["entries"].as_array().unwrap().iter()
            .find(|entry|entry["ingredient_id"]==format!("ig_800000000000000000000000000000{suffix}")).unwrap();
        let text=row["condition_text"].as_array().unwrap();assert!(!text.is_empty());assert!(text.iter().any(|text|text==&exact["input_condition"]),"exact readable input condition for {suffix}");
        assert!(text.iter().all(|text|text.as_str().is_some_and(|text|!text.trim().is_empty())));
        assert_eq!(row["intrinsic_conditions"]["table"]["applies_even_when_all_consumers_confirmed_none"],true);
        assert!(draft["line"]["uses"].as_array().unwrap().iter().all(|use_|use_["requirements"].as_array().unwrap().is_empty()));
        for (index,field) in ["ingredients","allergens","storage"].into_iter().enumerate() {assert_eq!(draft["field_dependencies"][index]["fact"],o["body"]["data"]["items"][0][field]);}
    }
}

#[test]
fn drained_pack_comparison_never_uses_gross_net_as_edible_coverage_or_converts_forms() {
    let mut s=admit(&setup(),three_bean_packet_ref().unwrap(),0xd930);for (id,index) in (0xd931..).zip([1,4,7,10,13,16]) {s=select(&s,index,three_bean_packet_ref().unwrap().recipe,None,id);}
    let p=prepare(&s);let id=line(&s,"07");for (drained,status) in [(known(quantity("240","g","drained")),"comparable_declared_measure"),(unknown(),"unknown"),
        (json!({"kind":"conflicting","data":{"values":[quantity("240","g","drained"),quantity("250","g","drained")]}}),"conflicting"),(known(quantity("240","g","cooked")),"known_measure_mismatch")] {
        // Each is a separate pure synthetic fixture, not rewriting an accepted
        // host journal. Owner duplicate/import authority is tested separately.
        let o=observation(&p,known(quantity("400","g","cooked")),drained.clone());let a=accepted(&s,&p,&o);let d=good(input(&s,&p,&o,&a,id.clone(),NOW));inert(&d);
        assert_eq!(d["line"]["demand"]["quantity"],quantity("480","g","drained"));assert_eq!(d["pack_comparison"],json!({"measure":"drained","observed":drained,"status":status}));assert_eq!(d["occurrences"].as_array().unwrap().len(),6);
    }
}

#[test]
fn expired_request_does_not_invent_meaning_freshness_and_changed_privacy_is_still_stale() {
    let s=porridge();let p=prepare(&s);let o=observation(&p,unknown(),unknown());let a=accepted(&s,&p,&o);let id=line(&s,"01");
    let d=good(input(&s,&p,&o,&a,id.clone(),LATER));inert(&d);assert_eq!(d["source"]["accepted_ref"],a);assert_eq!(d["source"]["currentness"],"not_assessed");assert_eq!(d["pack_comparison"]["status"],"unknown");
    let changed=privacy(&s,false,0xd940);assert_eq!(refuse(input(&changed,&p,&o,&a,line(&changed,"01"),LATER))["code"],"context_stale");
    // This deliberately supplies only a synthetic correlation receipt for an
    // AI-off private prepared scope. It is NOT a production user-witnessed
    // acceptance: the browser owner must reject any absent/quarantined record.
    // The pure readonly projection itself performs no model call or AI bypass.
    let p=prepare(&changed);let o=observation(&p,unknown(),unknown());let typed:ReadObservationV1=serde_json::from_value(o.clone()).unwrap();
    let a=json!(AcceptedReadRef{evidence_id:EvidenceId("ev_80000000000000000000000000000091".into()),job_id:typed.job_id.clone(),read_id:typed.read_id.clone(),
        observation_id:typed.observation_id.clone(),digest:canonical_digest(&typed).unwrap(),resource:typed.resource.clone(),observed_at:typed.observed_at.clone(),admitted_at:Instant(NOW.into())});
    inert(&good(input(&changed,&p,&o,&a,line(&changed,"01"),LATER)));
}
