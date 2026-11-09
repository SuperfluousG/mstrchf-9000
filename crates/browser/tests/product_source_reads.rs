//! Actual native read/food contracts with synthetic local actors, aliases and
//! label observations. No Tesco page, account, price, source truth, witness act,
//! product meaning/admission or external effect is qualified by these fixtures.
use mstrchf_browser::{run_json,maker_packets::porridge_packet_ref,
    product_source_reads::{PreparedProductSourceRead,prepared_digest}};
use mstrchf_contracts::*;
use serde_json::{Value,json};
const CREATED:&str="2026-10-13T07:00:00Z";
const OBSERVED:&str="2026-10-13T07:00:01Z";
const NOW:&str="2026-10-13T07:05:00Z";
const DEADLINE:&str="2026-10-13T08:00:00Z";
fn call(input:Value)->Value {serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap()}
fn good(input:Value)->Value {let result=call(input);assert_eq!(result["ok"],true,"{result}");result["value"].clone()}
fn refuse(input:Value)->Value {let result=call(input);assert_eq!(result["ok"],false,"{result}");result["error"].clone()}
fn apply(state:&Value,body:Value,id:u32)->Value {good(json!({"op":"dispatch","state":state,"now":CREATED,"command":{"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body}}))["state"].clone()}
fn week_body(w:&Value)->Value {json!({"kind":"save_week","data":{"expected_week_revision":w["revision"],"week":{"week_id":w["range"]["week_id"],"defaults_snapshot":w["defaults_snapshot"],"day_enabled":w["day_enabled"],"meal_type_enabled":w["meal_type_enabled"],"cells":w["cells"],"plan":w["plan"]}}})}
fn privacy(state:&Value,enabled:bool,id:u32)->Value {let mut policy=state["snapshot"]["privacy"].clone();policy["use_ai_features"]=json!(enabled);apply(state,json!({"kind":"set_privacy","data":{"expected_policy_revision":policy["revision"],"policy":policy}}),id)}
fn saved(ai:bool)->Value {
    let mut state=good(json!({"op":"initialize","household_id":"hh_80000000000000000000000000000080","diner_id":"dn_80000000000000000000000000000080","zone":"Europe/London","today":"2026-10-13","now":CREATED,"demo":true}));
    let mut p=state["snapshot"]["profile"].clone();p["diners"][0]["age_band"]=json!("adult");p["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":CREATED}});p["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});p["preferences"]["equipment"]=json!(["hob"]);
    state=apply(&state,json!({"kind":"update_profile","data":{"expected_profile_revision":p["revision"],"profile":p}}),0xd000);
    let mut w=state["snapshot"]["weeks"][0].clone();w["meal_type_enabled"]["breakfast"]=json!(true);w["cells"][3]["choice"]=json!("at_home");state=apply(&state,week_body(&w),0xd001);
    state=apply(&state,json!({"kind":"admit_reviewed_recipe","data":{"packet":porridge_packet_ref().unwrap(),"expected_catalogue_revision":"1"}}),0xd002);
    let w=&state["snapshot"]["weeks"][0];let board=good(json!({"op":"board","state":state,"week_id":w["range"]["week_id"],"slot":w["cells"][3]["slot"],"now":CREATED}));let choice=board["choices"].as_array().unwrap().iter().find(|choice|choice["selection"]["recipe"]==json!(porridge_packet_ref().unwrap().recipe)).unwrap();let mut w=w.clone();w["plan"]=choice["plan"].clone();state=apply(&state,week_body(&w),0xd003);
    if ai {state=privacy(&state,true,0xd004);}state
}
fn target()->Value {json!({"product":{"retailer":"tesco","product_id":"123456789","variant_id":null},"url":"https://www.tesco.com/shop/en-GB/products/123456789"})}
fn prepare_input(state:&Value)->Value {json!({"op":"product_source_prepare","state":state,"week_id":state["snapshot"]["weeks"][0]["range"]["week_id"],"reader":{"provider":"codex_local","native_task_ref":"synthetic-independently-bound-codex-task"},"key":{"retailer":"tesco","account_ref":"synthetic-local-account-alias","basket_id":"bk_80000000000000000000000000000080"},"delivery":{"retailer":"tesco","account_ref":"synthetic-local-account-alias","context_ref":"synthetic-public-source-context","delivery_date":null,"slot_ref":null},"targets":[target()],"job_id":"jb_80000000000000000000000000000080","read_id":"rd_80000000000000000000000000000080","now":CREATED,"deadline":DEADLINE})}
fn prepare(state:&Value)->Value {good(prepare_input(state))}
fn unknown()->Value {json!({"kind":"unknown","data":{"reason":"not_exposed"}})}
fn observation(prepared:&Value)->Value {
    let scope=&prepared["request"]["scope"]["data"]["read_scope"];
    json!({"schema_version":1,"observation_id":"ob_80000000000000000000000000000080","read_id":scope["read_id"],"job_id":prepared["request"]["job_id"],"input_digest":prepared["request"]["input_digest"],"reader":prepared["expected_reader"],"resource":scope["resource"],"observed_at":OBSERVED,"body":{"kind":"products","data":{"delivery_context":prepared["delivery"],"items":[{"product":prepared["targets"][0]["product"],"url":prepared["targets"][0]["url"],"title":"Synthetic label fixture; no real product page was read","pack":{"count":{"kind":"known","data":{"value":1}},"net":{"kind":"known","data":{"value":{"amount":{"numerator":"500","denominator":"1"},"unit":"g","form":"dry"}}},"drained":unknown()},"price":{"kind":"observed","data":{"pence":"123","currency":"GBP"}},"offers":[],"availability":"unknown","ingredients":{"kind":"known","data":{"value":"Synthetic oats; unverified source text."}},"allergens":unknown(),"storage":unknown(),"delivery_context":prepared["delivery"],"observed_at":OBSERVED}]}}})
}
fn check_input(state:&Value,prepared:&Value,observed:&Value)->Value {json!({"op":"product_source_check_return","state":state,"prepared":prepared,"observation":observed,"now":NOW})}
fn accept_input(state:&Value,prepared:&Value,observed:&Value)->Value {json!({"op":"product_source_accept","state":state,"prepared":prepared,"observation":observed,"evidence_id":"ev_80000000000000000000000000000080","now":NOW})}
fn inspect_input(state:&Value,prepared:&Value)->Value {json!({"op":"product_source_inspect","state":state,"prepared":prepared,"now":NOW})}
fn resign(prepared:&mut Value){let mut request:JobRequestV1=serde_json::from_value(prepared["request"].clone()).unwrap();request.input_digest=input_digest(&RequestMaterial::from(&request)).unwrap();prepared["request"]=json!(request);let typed:PreparedProductSourceRead=serde_json::from_value(prepared.clone()).unwrap();prepared["prepared_digest"]=json!(prepared_digest(&typed).unwrap());}

#[test]
fn private_prepare_uses_native_digests_products_only_no_approval_writes_or_job_mutation() {
    let state=saved(false);let before=state.clone();let prepared=prepare(&state);let typed:PreparedProductSourceRead=parse_document(&canonical_bytes(&prepared).unwrap()).unwrap();assert_eq!(typed.prepared_digest,prepared_digest(&typed).unwrap());assert_eq!(typed.request.input_digest,input_digest(&RequestMaterial::from(&typed.request)).unwrap());validate_request_shape(&typed.request).unwrap();
    assert_eq!(typed.request.action,Action::PrepareShop);assert_eq!(typed.request.required_capabilities.0,vec![Capability::BrowserRead]);assert!(typed.request.allowed_artefacts.is_empty());assert!(typed.request.context_stamp.write_weeks.is_empty());assert_eq!(typed.request.privacy_policy_revision,state["snapshot"]["privacy"]["revision"].as_str().unwrap().parse::<u64>().map(Revision).unwrap());
    let ActionScope::Shop{phase,approval_id,read_scope,..}=&typed.request.scope else{panic!("shop only")};assert_eq!(*phase,ShopPhase::Observe);assert!(approval_id.0.is_none());assert_eq!(read_scope.allowed_kinds.0,vec![ReadKind::Products]);assert!(read_scope.event_ids.is_empty());
    let JobPayload::ProductObservation{approved_contribution,..}=&typed.request.payload else{panic!("product observation only")};assert!(approved_contribution.0.is_none());assert_eq!(prepared["food"]["contains_engineering"],false);assert_eq!(state,before);assert!(state["snapshot"]["jobs"].as_array().unwrap().is_empty());let refused=refuse(inspect_input(&state,&prepared));assert_eq!(refused["code"],"authority_denied");
}

#[test]
fn real_native_read_basis_is_usable_but_missing_meaning_admission_and_effect_authority_stay_explicit() {
    let state=saved(true);let before=state.clone();let prepared=prepare(&state);assert_eq!(good(inspect_input(&state,&prepared)),json!({"current":true}));let observed=observation(&prepared);let checked=good(check_input(&state,&prepared,&observed));let accepted=good(accept_input(&state,&prepared,&observed));assert_eq!(checked["observation_digest"],json!(canonical_digest(&observed).unwrap()));assert_eq!(accepted["accepted"]["digest"],checked["observation_digest"]);
    let typed:AcceptedReadRef=parse_document(&canonical_bytes(&accepted["accepted"]).unwrap()).unwrap();assert_eq!(typed.observed_at,Instant(OBSERVED.into()));assert_eq!(typed.admitted_at,Instant(NOW.into()));assert_eq!(typed.read_id.0,"rd_80000000000000000000000000000080");
    let assessment=&accepted["assessment"];assert_eq!(assessment["scope"],"source_correlation_only");assert_eq!(assessment["provenance"],"user_witnessed");for key in ["host_attested","source_truth_assessed","account_verified","effect_enabled"]{assert_eq!(assessment[key],false);}assert_eq!(assessment["meaning_status"],"missing");assert_eq!(assessment["admission_status"],"missing");assert_eq!(assessment["native_matching"],"unresolved");assert_eq!(assessment["quote_status"],"unknown");assert_eq!(assessment["retailer_status"],"unqualified");assert_eq!(assessment["products"][0]["native_reasons"],json!(["missing_meaning"]));assert!(!accepted.to_string().contains("grant_token"));assert!(!accepted.to_string().contains("match_basis"));assert_eq!(good(accept_input(&state,&prepared,&observed)),accepted,"Pure identical-input determinism, not the TS owner's duplicate journal proof");assert_eq!(state,before);
}

#[test]
fn freshness_is_strict_after_creation_not_future_and_before_the_one_hour_deadline() {
    let state=saved(true);let prepared=prepare(&state);for at in [CREATED,"2026-10-13T07:05:01Z"]{let mut observed=observation(&prepared);observed["observed_at"]=json!(at);observed["body"]["data"]["items"][0]["observed_at"]=json!(at);refuse(check_input(&state,&prepared,&observed));}
    let mut observed=observation(&prepared);observed["body"]["data"]["items"][0]["observed_at"]=json!(CREATED);refuse(check_input(&state,&prepared,&observed));
    let mut late=check_input(&state,&prepared,&observation(&prepared));late["now"]=json!(DEADLINE);assert_eq!(refuse(late)["code"],"context_stale");let mut early=inspect_input(&state,&prepared);early["now"]=json!("2026-10-13T06:59:59Z");refuse(early);
    for deadline in [CREATED,"2026-10-13T08:00:01Z"]{let mut input=prepare_input(&state);input["deadline"]=json!(deadline);refuse(input);}let mut equal=check_input(&state,&prepared,&observation(&prepared));equal["now"]=json!(OBSERVED);assert!(good(equal)["observation_digest"].is_string());
}

#[test]
fn exact_independent_reader_resource_targets_delivery_and_complete_item_set_are_checked() {
    let state=saved(true);let prepared=prepare(&state);let observed=observation(&prepared);
    for field in ["job","read","digest","reader","resource","delivery","item_delivery","product","url","missing","duplicate"] {let mut changed=observed.clone();match field{
        "job"=>changed["job_id"]=json!("jb_80000000000000000000000000000081"),"read"=>changed["read_id"]=json!("rd_80000000000000000000000000000081"),"digest"=>changed["input_digest"]=json!("a".repeat(64)),"reader"=>changed["reader"]["native_task_ref"]=json!("synthetic-foreign-task"),"resource"=>changed["resource"]["data"]["key"]["basket_id"]=json!("bk_80000000000000000000000000000081"),"delivery"=>changed["body"]["data"]["delivery_context"]["context_ref"]=json!("synthetic-foreign-context"),"item_delivery"=>changed["body"]["data"]["items"][0]["delivery_context"]["context_ref"]=json!("synthetic-foreign-context"),"product"=>changed["body"]["data"]["items"][0]["product"]["product_id"]=json!("123456780"),"url"=>changed["body"]["data"]["items"][0]["url"]=json!("https://www.tesco.com/shop/en-GB/products/123456780"),"missing"=>changed["body"]["data"]["items"]=json!([]),_=>{let item=changed["body"]["data"]["items"][0].clone();changed["body"]["data"]["items"].as_array_mut().unwrap().push(item);}}
        refuse(check_input(&state,&prepared,&changed));refuse(accept_input(&state,&prepared,&changed));
    }
}

#[test]
fn food_profile_removal_and_privacy_revocation_or_reenable_stale_existing_reads() {
    let state=saved(true);let prepared=prepare(&state);let observed=observation(&prepared);let mut p=state["snapshot"]["profile"].clone();p["diners"][0]["display_name"]=json!("Changed synthetic owner");let changed=apply(&state,json!({"kind":"update_profile","data":{"expected_profile_revision":p["revision"],"profile":p}}),0xd010);refuse(inspect_input(&changed,&prepared));refuse(accept_input(&changed,&prepared,&observed));
    let off=privacy(&state,false,0xd011);refuse(inspect_input(&off,&prepared));refuse(check_input(&off,&prepared,&observed));let on=privacy(&off,true,0xd012);refuse(accept_input(&on,&prepared,&observed));
    let private=saved(false);let private_prepared=prepare(&private);let on=privacy(&private,true,0xd013);refuse(inspect_input(&on,&private_prepared));let mut w=state["snapshot"]["weeks"][0].clone();w["plan"]["selections"]=json!([]);let empty=apply(&state,week_body(&w),0xd014);refuse(prepare_input(&empty));refuse(inspect_input(&empty,&prepared));
}

#[test]
fn tampered_or_resigned_prepared_scope_cannot_enable_more_reads_writes_approvals_or_capabilities() {
    let state=saved(true);let prepared=prepare(&state);
    let mut changed=prepared.clone();changed["food"]["contains_engineering"]=json!(true);assert_eq!(refuse(inspect_input(&state,&changed))["code"],"digest_mismatch");
    for field in ["kind","capability","write","artefact"] {let mut changed=prepared.clone();match field{
        "kind"=>{changed["request"]["scope"]["data"]["read_scope"]["allowed_kinds"]=json!(["products","basket"]);changed["request"]["payload"]["data"]["read_scope"]["allowed_kinds"]=json!(["products","basket"]);},
        "capability"=>changed["request"]["required_capabilities"]=json!(["browser_read","browser_mutate"]),
        "write"=>changed["request"]["context_stamp"]["write_weeks"]=changed["request"]["context_stamp"]["read_weeks"].clone(),
        _=>changed["request"]["allowed_artefacts"]=json!([{"artefact_id":"ar_80000000000000000000000000000080","relative_name":"synthetic-unwanted.json","digest":"a".repeat(64),"byte_length":1,"media_type":"application/json"}])}
        resign(&mut changed);refuse(inspect_input(&state,&changed));
    }
}

#[test]
fn malformed_bounds_targets_actor_and_audited_state_refuse_without_source_truth_upgrade() {
    let state=saved(true);for change in ["provider","variant","url","count","duplicate","delivery"]{let mut input=prepare_input(&state);match change{
        "provider"=>input["reader"]["provider"]=json!("claude_cowork"),"variant"=>input["targets"][0]["product"]["variant_id"]=json!("unsupported-variant"),"url"=>input["targets"][0]["url"]=json!("https://example.invalid/synthetic-source"),"count"=>input["targets"]=json!(vec![target();9]),"duplicate"=>input["targets"]=json!([target(),target()]),_=>input["delivery"]["account_ref"]=json!("different-local-alias")};refuse(input);}
    let prepared=prepare(&state);let mut observed=observation(&prepared);observed["body"]["data"]["items"][0]["ingredients"]=json!({"kind":"known","data":{"value":"<script>synthetic inert text, never execute</script>"}});let accepted=good(accept_input(&state,&prepared,&observed));assert_eq!(accepted["assessment"]["native_matching"],"unresolved");assert_eq!(accepted["assessment"]["source_truth_assessed"],false);
    let mut extra=check_input(&state,&prepared,&observed);extra["observation"]["accepted"]=json!(true);refuse(extra);let mut broken=state.clone();broken["receipts"][0]["command_digest"]=json!("a".repeat(64));refuse(prepare_input(&broken));
}

#[test]
fn inactive_history_audit_checks_native_custody_without_current_state_time_or_ai_authority() {
    let state=saved(true);let prepared=prepare(&state);let observed=observation(&prepared);
    let accepted=good(accept_input(&state,&prepared,&observed))["accepted"].clone();
    let audit=|observation:Value,accepted:Value|json!({"op":"product_source_audit","prepared":prepared,"observation":observation,"accepted":accepted});
    assert_eq!(good(audit(Value::Null,Value::Null)),json!({"valid":true,"observation_digest":null}));
    for input in [audit(observed.clone(),Value::Null),audit(observed.clone(),accepted.clone())]{assert_eq!(good(input),json!({"valid":true,"observation_digest":canonical_digest(&observed).unwrap()}));}
    let off=privacy(&state,false,0xd020);let mut live=accept_input(&off,&prepared,&observed);live["now"]=json!("2026-10-14T08:00:00Z");refuse(live);
    assert_eq!(good(audit(observed.clone(),accepted.clone())),json!({"valid":true,"observation_digest":canonical_digest(&observed).unwrap()}),"Historical custody remains readable after live scope expires/revokes; audit takes neither state nor now");
    refuse(audit(Value::Null,accepted.clone()));
    for field in ["digest","actor","time","unknown_fact","count","price","url"]{let mut item=observed.clone();let mut receipt=accepted.clone();match field{
        "digest"=>receipt["digest"]=json!("a".repeat(64)),"actor"=>item["reader"]["native_task_ref"]=json!("different-independent-task"),
        "time"=>receipt["admitted_at"]=json!(DEADLINE),"unknown_fact"=>item["body"]["data"]["items"][0]["storage"]=json!({"kind":"unknown","data":{"reason":"invented_reason"}}),
        "count"=>item["body"]["data"]["items"][0]["pack"]["count"]=json!({"kind":"known","data":{"value":0}}),
        "price"=>item["body"]["data"]["items"][0]["price"]=json!({"kind":"observed","data":{"pence":"123","currency":"invented_currency"}}),
        _=>item["body"]["data"]["items"][0]["url"]=json!("https://www.tesco.com/shop/en-GB/products/123456780")}
        refuse(audit(item,receipt));
    }
    let mut changed=prepared.clone();changed["request"]["scope"]["data"]["read_scope"]["allowed_kinds"]=json!(["products","basket"]);changed["request"]["payload"]["data"]["read_scope"]["allowed_kinds"]=json!(["products","basket"]);resign(&mut changed);
    refuse(json!({"op":"product_source_audit","prepared":changed,"observation":null,"accepted":null}));
}
