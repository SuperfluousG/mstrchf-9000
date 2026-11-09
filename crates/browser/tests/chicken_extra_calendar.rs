//! Exact native recipe/calendar custody with synthetic households and user
//! confirmation records. No purchase, delivery, cooking, physical cold chain,
//! product suitability or calendar-client import is observed by these tests.
use mstrchf_browser::{run_json,BrowserState,calendar_owner,
    calendar_view::*,catalogue::RuntimeCatalogue,maker_packets::*,personal_scope::*};
use mstrchf_calendar::*;
use mstrchf_contracts::*;
use mstrchf_core::food::*;
use serde_json::{Value,json};
const NOW:&str="2026-10-13T06:30:00Z";
const LATER:&str="2026-10-13T07:00:00Z";
const DEST:&str="cd_10000000000000000000000000000001";
fn call(input:Value)->Value {serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap()}
fn good(input:Value)->Value {let answer=call(input);assert_eq!(answer["ok"],true,"{answer}");answer["value"].clone()}
fn command(state:&Value,body:Value,id:u32)->Value {json!({"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body})}
fn apply(state:&Value,body:Value,id:u32)->Value {good(json!({"op":"dispatch","state":state,"command":command(state,body,id),"now":NOW}))["state"].clone()}
fn week_body(week:&Value,plan:bool)->Value {json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{"week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],"day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],"cells":week["cells"],"plan":if plan{week["plan"].clone()}else{Value::Null}}}})}
fn vegetarian()->Value {json!({"kind":"diet","data":{"code":"vegetarian"}})}
fn declaration(veg:bool)->Value {if veg {json!({"kind":"confirmed","data":{"confirmed_at":NOW,"requirements":[vegetarian()]}})}else{json!({"kind":"confirmed_none","data":{"confirmed_at":NOW}})}}
fn moment(state:&Value,time:&str)->Value {let w=&state["snapshot"]["weeks"][0];good(json!({"op":"resolve_moment","date":w["cells"][4]["slot"]["date"],"time":time,"zone":w["range"]["zone"],"resolution":"unambiguous"}))}
fn admit(state:&Value,packet:ReviewedRecipePacketRef,id:u32)->Value {let revision=1+state["reviewed_packets"].as_array().map_or(0,Vec::len);apply(state,json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":revision.to_string()}}),id)}
fn recipient(state:&Value,guest:bool,second:bool)->Value {
    if guest&&second {let occasion=&state["snapshot"]["weeks"][0]["cells"][4]["occasion"];json!({"kind":"guest","data":{"occasion_id":occasion["occasion_id"],"guest_id":occasion["guests"][0]["guest_id"]}})}
    else {json!({"kind":"diner","data":{"diner_id":state["snapshot"]["profile"]["diners"][usize::from(second)]["diner_id"]}})}
}
fn board(state:&Value,options:Value)->Value {let w=&state["snapshot"]["weeks"][0];good(json!({"op":"board","state":state,"week_id":w["range"]["week_id"],"slot":w["cells"][4]["slot"],"options":options,"now":NOW}))}
struct Fixture {state:Value,original_view:RuntimeCatalogue,current_view:RuntimeCatalogue,
    basis:SavedFoodBasis,derived:DerivedPlan,approved:ApprovedPlan,order:ConfirmedOrder,
    occupied:OccupancySnapshot,policy:LiveFoodPolicy}
impl Fixture {
    fn snapshot(&self)->Snapshot {serde_json::from_value(self.state["snapshot"].clone()).unwrap()}
    fn inputs<'a>(&'a self,snapshot:&'a Snapshot)->PersonalCalendarInputs<'a> {PersonalCalendarInputs{
        original_catalogue:&self.original_view,current_catalogue:&self.current_view,basis:&self.basis,
        derived:&self.derived,approved:&self.approved,order:&self.order,snapshot,
        profile:snapshot.profile.0.as_ref().unwrap(),week:&snapshot.weeks[0],occupied:&self.occupied,
        essentials:&[],policy:&self.policy,evaluated_at:&self.order.confirmation.confirmed_at}}
}
fn fixture(guest:bool,recipient_second:bool,extra:bool,delivery_end:&str)->Fixture {
    let mut state=serde_json::from_slice::<Value>(include_bytes!("fixtures/legacy-engineering-receipts.json")).unwrap()["state"].clone();
    let mut privacy=state["snapshot"]["privacy"].clone();privacy["use_ai_features"]=json!(false);privacy["include_household_requirements"]=json!(false);
    state=apply(&state,json!({"kind":"set_privacy","data":{"expected_policy_revision":privacy["revision"],"policy":privacy}}),0xcf00);
    let mut p=state["snapshot"]["profile"].clone();p["diners"][0]["age_band"]=json!("adult");p["diners"][0]["display_name"]=json!("Resident One");p["diners"][0]["restrictions"]=declaration(recipient_second);
    p["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});p["preferences"]["equipment"]=json!(["fridge","hob"]);
    if !guest {let mut second=p["diners"][0].clone();second["diner_id"]=json!("dn_80000000000000000000000000000008");second["display_name"]=json!("Resident Two");second["restrictions"]=declaration(!recipient_second);p["diners"].as_array_mut().unwrap().push(second);}
    state=apply(&state,json!({"kind":"update_profile","data":{"expected_profile_revision":p["revision"],"profile":p}}),0xcf01);
    let mut w=state["snapshot"]["weeks"][0].clone();w["plan"]["selections"]=json!([]);w["plan"]["batch_links"]=json!([]);w["meal_type_enabled"]["lunch"]=json!(true);
    if guest {w["cells"][4]["occasion"]=json!({"occasion_id":"oc_80000000000000000000000000000008","host_diner_ids":w["cells"][4]["attendee_ids"],"guests":[{"guest_id":"gu_80000000000000000000000000000008","age_band":"adult","restrictions":declaration(!recipient_second),"portion":{"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]}}]});}
    else {w["cells"][4]["attendee_ids"].as_array_mut().unwrap().push(json!("dn_80000000000000000000000000000008"));}
    state=apply(&state,week_body(&w,true),0xcf02);
    state=admit(&state,chilled_chicken_extra_packet_ref().unwrap(),0xcf03);state=admit(&state,three_bean_two_unit_chicken_extra_packet_ref().unwrap(),0xcf04);
    let options=if extra {json!({"extra_recipients":[recipient(&state,guest,recipient_second)]})}else{Value::Null};let b=board(&state,options);
    let c=b["choices"].as_array().unwrap().iter().find(|choice|choice["selection"]["recipe"]==json!(three_bean_two_unit_chicken_extra_packet_ref().unwrap().recipe)).unwrap();let mut w=state["snapshot"]["weeks"][0].clone();w["plan"]=c["plan"].clone();state=apply(&state,week_body(&w,true),0xcf05);
    let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();let original_view=RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();let(basis,food)=material_at(&original_view,&typed.snapshot,typed.snapshot.profile.0.as_ref().unwrap(),&typed.snapshot.weeks[0],&Instant(NOW.into())).unwrap().unwrap();let derived=food.plan;
    state=apply(&state,json!({"kind":"approve_plan","data":{"week_id":derived.week_id,"plan_revision":derived.plan_revision,"derived_digest":derived.digest}}),0xcf06);
    let approved=serde_json::from_value::<ApprovedPlan>(state["snapshot"]["weeks"][0]["approved_plan"].clone()).unwrap();state=apply(&state,json!({"kind":"confirm_order","data":{"confirmation":{"week_id":derived.week_id,"approval_id":approved.approval_id,"delivery_start":moment(&state,"10:00:00"),"delivery_end":moment(&state,delivery_end),"confirmed_at":NOW,"source":{"kind":"user_confirmation","data":{}}}}}),0xcf07);
    let order=serde_json::from_value(state["snapshot"]["weeks"][0]["confirmed_order"].clone()).unwrap();let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();let current_view=RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();
    // Explicit synthetic empty owner occupancy for this isolated fixture. The
    // owner entrypoint tests below independently construct its real saved view.
    let mut occupied=basis.schedule.occupied.clone();occupied.query.as_mut().unwrap().excluded_subject=Some(owner_pin(&basis,&derived).unwrap());occupied.digest=occupancy_snapshot_digest(&occupied).unwrap();
    let mut policy=LiveFoodPolicy{version:Revision(1),digest:PayloadDigest("0".repeat(64)),general_seconds:172800,rice_seconds:86400,maximum_reheats:1};policy.digest=live_food_policy_digest(&policy).unwrap();
    Fixture{state,original_view,current_view,basis,derived,approved,order,occupied,policy}
}
fn first_update(prepared:&PreparedCalendar)->CalendarUpdateProjection {CalendarUpdateProjection{
    scope:prepared.drafts.iter().map(|draft|ScopedEvent{key:draft.logical_key.clone(),prior:PriorEventState::NeverVersioned,disposition:UpdateDisposition::Upsert}).collect(),
    membership:prepared.drafts.iter().flat_map(|draft|draft.contributors.iter().map(|member|ContributorTransition{member:SessionIdentity{week_id:member.owner.week.week_id.clone(),session_id:member.session_id.clone()},before:None,after:Some(draft.logical_key.clone())})).collect()}}
fn retained_update(snapshot:&CalendarSnapshot)->CalendarUpdateProjection {CalendarUpdateProjection{
    scope:snapshot.events.iter().map(|row|ScopedEvent{key:row.event.logical_key.clone(),prior:PriorEventState::Retained(row.clone()),disposition:UpdateDisposition::Upsert}).collect(),
    membership:snapshot.events.iter().flat_map(|row|row.contributors.iter().map(|member|ContributorTransition{member:SessionIdentity{week_id:member.owner.week.week_id.clone(),session_id:member.session_id.clone()},before:Some(row.event.logical_key.clone()),after:Some(row.event.logical_key.clone())})).collect()}}
fn cold(state:Value)->Value {std::thread::spawn(move||good(json!({"op":"validate_state","state":state,"now":LATER}))).join().unwrap()}
fn assert_invalid(result:Result<PreparedCalendar,CalendarFailure>,path:&str) {
    match result {Err(CalendarFailure::Invalid(error))=>assert!(error.issues.iter().any(|issue|issue.path==path),"{error:?}"),other=>panic!("expected {path}: {other:?}")}
}

#[test]
fn renderer3_exact_pair_keeps_methods_separate_and_names_only_the_real_resident_or_guest() {
    for guest in [false,true] {for second in [false,true] {
        let f=fixture(guest,second,true,"10:59:00");let before=f.state.clone();let pin=saved_basis_digest(&f.basis).unwrap();let snapshot=f.snapshot();let prepared=prepare_personal_calendar_for_renderer(&f.inputs(&snapshot),3).unwrap();assert_eq!(prepared.drafts.len(),3);
        let base=prepared.drafts.iter().find(|draft|draft.start.local_time.0=="11:00:00").unwrap();let extra=prepared.drafts.iter().find(|draft|draft.start.local_time.0=="12:25:00").unwrap();assert_eq!(base.end.local_time.0,"12:30:00");assert_eq!(extra.end,base.end);assert_eq!(base.logical_key.kind,CalendarEventKind::Prep);assert_eq!(extra.logical_key.kind,CalendarEventKind::Prep);assert_ne!(base.logical_key.session_id,extra.logical_key.session_id);
        let expected=if guest&&second {"Diner 2 — Guest 1"}else if second {"Diner 2 — Resident Two (resident)"}else{"Diner 1 — Resident One (resident)"};assert!(extra.body.0.contains(&format!("Named extra recipient: {expected}")),"{}",extra.body.0);assert!(base.body.0.contains(&format!("Linked separate extra: {expected}")));
        for text in ["exactly 50 g edible cooked input","Reserve 5 minutes","unmeasured counter-handling estimate","not a cooking or original-product storage clock","never resets product dates","not a validated safe-exposure interval","dedicated utensils","separate dish","Keep the chicken chilled until retrieval. Fridge access, food scales, dedicated utensils and a separate dish are required. Actual temperature, available space and separation have not been checked.","combined meal is not Vegetarian","05:20:48Z","Codex","PersonalOnly","UnknownProductDate"] {assert!(extra.body.0.contains(text),"missing {text}: {}",extra.body.0);}
        assert!(!extra.body.0.contains("Original storage clock —"));assert!(!extra.body.0.contains("passive refrigeration"));assert!(!extra.body.0.contains("Cooling/storage:"));assert!(base.body.0.contains("Reserve30minutes"));assert!(base.body.0.contains("at least60minutes passive refrigeration"));assert!(base.body.0.contains("Original storage clock — planned first handling"));
        let base_recipe=known_packet(&three_bean_two_unit_chicken_extra_packet_ref().unwrap()).unwrap().recipe();let child=known_packet(&chilled_chicken_extra_packet_ref().unwrap()).unwrap().recipe();
        for step in base_recipe.steps.iter(){assert_eq!(base.body.0.matches(&step.text.0).count(),1);assert!(!extra.body.0.contains(&step.text.0));}for step in child.steps.iter(){assert_eq!(extra.body.0.matches(&step.text.0).count(),1);assert!(!base.body.0.contains(&step.text.0));}
        assert_eq!(f.derived.allocations.len(),2);assert_eq!(f.derived.demand.lines.len(),10);assert_eq!(f.derived.demand.lines.iter().find(|line|line.ingredient_id.0=="ig_80000000000000000000000000000010").unwrap().quantity.amount,PositiveRational::new(50,1).unwrap());assert_eq!(f.state,before);assert_eq!(saved_basis_digest(&f.basis).unwrap(),pin);
    }}
}

#[test]
fn legacy_renderer_wrappers_refuse_extras_and_no_extra_body_bytes_stay_identical() {
    let f=fixture(true,true,true,"10:59:00");let snapshot=f.snapshot();for version in [1,2] {assert_invalid(prepare_personal_calendar_for_renderer(&f.inputs(&snapshot),version),"/calendar/extras");}assert_invalid(prepare_personal_calendar(&f.inputs(&snapshot)),"/calendar/extras");assert_invalid(prepare_personal_calendar_for_renderer(&f.inputs(&snapshot),4),"/calendar/delivery_correction_renderer");
    let f=fixture(true,true,false,"10:59:00");let snapshot=f.snapshot();let legacy=prepare_personal_calendar(&f.inputs(&snapshot)).unwrap();for version in [1,2,3] {let candidate=prepare_personal_calendar_for_renderer(&f.inputs(&snapshot),version).unwrap();assert_eq!(candidate,legacy);}assert_eq!(legacy.drafts.len(),2);assert_eq!(f.snapshot().weeks[0].confirmed_order.0.as_ref(),Some(&f.order));
}

#[test]
fn exact_extra_ics_versions_keep_uid_sequence_bytes_and_do_not_claim_import_or_storage_credit() {
    let f=fixture(true,true,true,"10:59:00");let snapshot=f.snapshot();let prepared=prepare_personal_calendar_for_renderer(&f.inputs(&snapshot),3).unwrap();let first=export_personal_calendar(&prepared,&first_update(&prepared),&snapshot.household_draft.household_id,&Instant(NOW.into())).unwrap();let again=export_personal_calendar(&prepared,&retained_update(&first.snapshot),&snapshot.household_draft.household_id,&Instant(LATER.into())).unwrap();assert_eq!(first.snapshot,again.snapshot);assert_eq!(first.ics,again.ics);
    let bytes=String::from_utf8(first.ics).unwrap();assert_eq!(bytes.matches("BEGIN:VEVENT").count(),3);assert_eq!(bytes.matches("UID:").count(),3);assert!(!bytes.contains("VALARM"));assert!(!bytes.contains("METHOD:"));for line in bytes.split("\r\n"){assert!(line.len()<=75);}assert!(first.notice.contains("Import this file yourself"));assert!(first.notice.contains("does not synchronise"));assert_eq!(f.snapshot(),snapshot);
}

#[test]
fn actual_owner_prepare_receipt_replay_and_cold_restore_preserve_extra_booked_custody() {
    let f=fixture(true,true,true,"10:59:00");let preview=good(json!({"op":"calendar_preview","state":f.state,"order_id":f.order.order_id,"destination_id":DEST,"now":NOW}));assert_eq!(preview["prepared"],false);assert_eq!(preview["batch"]["events"].as_array().unwrap().len(),3);
    for event in preview["batch"]["events"].as_array().unwrap(){assert!(event["title"].as_str().unwrap().starts_with("MasterChef TEST ·"));assert!(event["body"].as_str().unwrap().starts_with("FICTIONAL TEST KITCHEN."));}
    let body=json!({"kind":"request_calendar_publication","data":{"order_id":f.order.order_id,"destination_id":DEST,"event_batch_digest":preview["event_batch_digest"]}});let cmd=command(&f.state,body,0xcf20);let committed=good(json!({"op":"dispatch","state":f.state,"command":cmd,"now":NOW}))["state"].clone();let journal=&committed["receipts"].as_array().unwrap().last().unwrap()["prepared_calendar"];assert_eq!(journal["renderer_version"],3);assert_eq!(journal["saved_events"].as_array().unwrap().len(),3);
    let again=good(json!({"op":"calendar_preview","state":committed,"order_id":f.order.order_id,"destination_id":DEST,"now":LATER}));assert_eq!(again["prepared"],true);assert_eq!(again["batch"],preview["batch"]);assert_eq!(again["ics"],preview["ics"]);assert_eq!(committed["snapshot"]["weeks"][0]["confirmed_order"],f.state["snapshot"]["weeks"][0]["confirmed_order"]);assert_eq!(committed["history"],f.state["history"]);
    let replay=good(json!({"op":"dispatch","state":committed,"command":cmd,"now":LATER}));assert_eq!(replay["receipt"]["disposition"],"replayed");assert_eq!(replay["state"],committed);assert_eq!(cold(committed.clone()),committed);
}

#[test]
fn first_handling_arrival_equality_passes_but_late_delivery_preserves_booked_and_refuses() {
    let exact=fixture(true,true,true,"11:00:00");let snapshot=exact.snapshot();assert!(prepare_personal_calendar_for_renderer(&exact.inputs(&snapshot),3).is_ok());
    let late=fixture(true,true,true,"11:00:01");let snapshot=late.snapshot();let before=late.state.clone();assert!(check_all_prep_arrival(&late.order,&late.derived).is_err());match prepare_personal_calendar_for_renderer(&late.inputs(&snapshot),3) {Err(CalendarFailure::Invalid(error))=>assert!(error.issues.iter().any(|issue|issue.path.contains("delivery_end"))),other=>panic!("unexpected {other:?}")};assert_eq!(late.state,before);assert_eq!(snapshot.weeks[0].confirmed_order.0.as_ref(),Some(&late.order));
}

#[test]
fn changed_or_departed_named_recipient_never_gets_redirected_calendar_authority() {
    let f=fixture(true,true,true,"10:59:00");for change in ["diet","age","portion","departure"] {let mut w=f.state["snapshot"]["weeks"][0].clone();match change {"diet"=>w["cells"][4]["occasion"]["guests"][0]["restrictions"]=declaration(true),"age"=>w["cells"][4]["occasion"]["guests"][0]["age_band"]=json!("teen"),"portion"=>w["cells"][4]["occasion"]["guests"][0]["portion"]["adjustment"]=json!({"numerator":"2","denominator":"1"}),_=>w["cells"][4]["occasion"]["guests"]=json!([])}let changed=apply(&f.state,week_body(&w,false),0xcf21);assert_eq!(changed["snapshot"]["weeks"][0]["confirmed_order"],f.state["snapshot"]["weeks"][0]["confirmed_order"]);assert_eq!(changed["snapshot"]["weeks"][0]["plan"]["selections"],f.state["snapshot"]["weeks"][0]["plan"]["selections"]);assert_eq!(changed["history"]["ordered"],f.state["history"]["ordered"]);let refusal=call(json!({"op":"calendar_preview","state":changed,"order_id":f.order.order_id,"destination_id":DEST,"now":NOW}));assert_eq!(refusal["ok"],false,"{refusal}");assert_eq!(cold(changed.clone()),changed);}
}

#[test]
fn original_extra_scope_resources_and_occupancy_cannot_be_replaced_by_transport_copies() {
    let mut f=fixture(true,true,true,"10:59:00");let snapshot=f.snapshot();let original=f.basis.clone();
    let selection=&f.basis.plan.selections[0];let person=selection.extras[0].recipients[0].person.clone();let meal=f.basis.context.requirements.iter_mut().find(|meal|meal.slot==selection.slot).unwrap();meal.attendees.iter_mut().find(|attendee|attendee.person==person).unwrap().hard_requirements=BoundedVec(vec![HardRequirement::Diet{code:DietCode::Vegetarian}]);assert!(prepare_personal_calendar_for_renderer(&f.inputs(&snapshot),3).is_err());f.basis=original.clone();
    let resource=f.basis.evidence.resources.iter_mut().find(|resource|resource.recipe.recipe_id.0==CHILLED_CHICKEN_EXTRA_SCOPE.recipe_id).unwrap();resource.equipment=vec![EquipmentCode::Fridge];f.basis.evidence.catalogue_digest=food_evidence_digest(&f.basis.evidence).unwrap();assert_invalid(prepare_personal_calendar_for_renderer(&f.inputs(&snapshot),3),"/calendar/original_food");f.basis=original;
    f.occupied.query.as_mut().unwrap().excluded_subject=None;f.occupied.digest=occupancy_snapshot_digest(&f.occupied).unwrap();assert!(prepare_personal_calendar_for_renderer(&f.inputs(&snapshot),3).is_err());assert_eq!(f.snapshot().weeks[0].confirmed_order.0.as_ref(),Some(&f.order));
    let typed:BrowserState=serde_json::from_value(f.state.clone()).unwrap();for version in [1,2] {assert!(calendar_owner::prepare_for_renderer(&typed,&f.order.order_id,&CalendarDestinationId(DEST.into()),&Instant(NOW.into()),version).is_err());}
}
