//! Real compiled recipe/domain commands with explicitly simulated order custody.
//! These tests do not assert actual payment, cooking, delivery or calendar import.
use mstrchf_browser::{BrowserState,run_json,calendar_view::*,catalogue::RuntimeCatalogue,maker_packets::*};
use mstrchf_calendar::*;
use mstrchf_contracts::*;
use mstrchf_core::food::*;
use serde_json::{Value,json};
const NOW:&str="2026-10-12T20:00:00Z";
fn good(request:Value)->Value{let out:Value=serde_json::from_slice(&run_json(&serde_json::to_vec(&request).unwrap())).unwrap();assert_eq!(out["ok"],true,"{out}");out["value"].clone()}
fn apply(state:&Value,body:Value,id:u32)->Value{good(json!({"op":"dispatch","state":state,"now":NOW,"command":{"schema_version":1,"command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body}}))["state"].clone()}
fn save(state:&Value,week:Value,id:u32)->Value{apply(state,json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{"week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],"day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],"cells":week["cells"],"plan":week["plan"]}}}),id)}
fn select(state:&Value,reference:&RecipeRef,index:usize,id:u32)->Value{let week=&state["snapshot"]["weeks"][0];let board=good(json!({"op":"board","state":state,"week_id":week["range"]["week_id"],"slot":week["cells"][index]["slot"],"now":NOW}));let choice=board["choices"].as_array().unwrap().iter().find(|choice|choice["selection"]["recipe"]==serde_json::to_value(reference).unwrap()).unwrap();let mut week=week.clone();week["plan"]=choice["plan"].clone();save(state,week,id)}
fn moment(date:&str,local:&str,utc:&str)->ZonedMoment{parse_document(&serde_json::to_vec(&json!({"local_date":date,"local_time":local,"zone":"Europe/London","resolution":"unambiguous","instant":utc})).unwrap()).unwrap()}
fn shifted(at:&ZonedMoment,seconds:i64)->ZonedMoment{let utc=chrono::DateTime::parse_from_rfc3339(&at.instant.0).unwrap()+chrono::Duration::seconds(seconds);let local=utc.with_timezone(&at.zone.0.parse::<chrono_tz::Tz>().unwrap());moment(&local.format("%Y-%m-%d").to_string(),&local.format("%H:%M:%S").to_string(),&utc.with_timezone(&chrono::Utc).format("%Y-%m-%dT%H:%M:%SZ").to_string())}
struct Fixture{state:Value,original_view:RuntimeCatalogue,current_view:RuntimeCatalogue,basis:SavedFoodBasis,derived:DerivedPlan,approved:ApprovedPlan,order:ConfirmedOrder,occupied:OccupancySnapshot,policy:LiveFoodPolicy}
impl Fixture{
 fn inputs<'a>(&'a self,snapshot:&'a Snapshot)->PersonalCalendarInputs<'a>{PersonalCalendarInputs{original_catalogue:&self.original_view,current_catalogue:&self.current_view,basis:&self.basis,derived:&self.derived,approved:&self.approved,order:&self.order,snapshot,profile:snapshot.profile.0.as_ref().unwrap(),week:&snapshot.weeks[0],occupied:&self.occupied,essentials:&[],policy:&self.policy,evaluated_at:&self.order.confirmation.confirmed_at}}
 fn snapshot(&self)->Snapshot{serde_json::from_value(self.state["snapshot"].clone()).unwrap()}
}
fn fixture(both:bool,late:bool)->Fixture{
 let mut state:Value=serde_json::from_slice::<Value>(include_bytes!("fixtures/legacy-engineering-receipts.json")).unwrap()["state"].clone();let mut profile=state["snapshot"]["profile"].clone();profile["diners"][0]["age_band"]=json!("adult");profile["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":NOW}});profile["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});
 state=apply(&state,json!({"kind":"update_profile","data":{"expected_profile_revision":profile["revision"],"profile":profile}}),0xc01);let mut week=state["snapshot"]["weeks"][0].clone();week["plan"]["selections"]=json!([]);week["plan"]["batch_links"]=json!([]);week["meal_type_enabled"]["breakfast"]=json!(true);week["meal_type_enabled"]["dinner"]=json!(true);state=save(&state,week,0xc02);
 state=apply(&state,json!({"kind":"admit_reviewed_recipe","data":{"packet":porridge_packet_ref().unwrap(),"expected_catalogue_revision":"1"}}),0xc03);
 if both{state=apply(&state,json!({"kind":"admit_reviewed_recipe","data":{"packet":aglio_packet_ref().unwrap(),"expected_catalogue_revision":"2"}}),0xc04);}
 state=select(&state,&porridge_packet_ref().unwrap().recipe,3,0xc05);if both{state=select(&state,&aglio_packet_ref().unwrap().recipe,8,0xc06);}
 let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();let original_view=RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();
 let (basis,food)=material_at(&original_view,&typed.snapshot,typed.snapshot.profile.0.as_ref().unwrap(),&typed.snapshot.weeks[0],&Instant(NOW.into())).unwrap().unwrap();let derived=food.plan;
 state=apply(&state,json!({"kind":"approve_plan","data":{"week_id":derived.week_id,"plan_revision":derived.plan_revision,"derived_digest":derived.digest}}),0xc07);
 let approved:ApprovedPlan=serde_json::from_value(state["snapshot"]["weeks"][0]["approved_plan"].clone()).unwrap();
 let (start,end)=if late{let first=derived.prep_sessions.iter().map(|session|&session.ingredient_needed_at).min_by_key(|at|&at.instant).unwrap();(shifted(first,3600),shifted(first,7200))}else{(moment("2026-10-12","16:00:00","2026-10-12T15:00:00Z"),moment("2026-10-12","17:00:00","2026-10-12T16:00:00Z"))};
 state=apply(&state,json!({"kind":"confirm_order","data":{"confirmation":{"week_id":derived.week_id,"approval_id":approved.approval_id,"delivery_start":start,"delivery_end":end,"confirmed_at":NOW,"source":{"kind":"user_confirmation","data":{}}}}}),0xc08);
 let order:ConfirmedOrder=serde_json::from_value(state["snapshot"]["weeks"][0]["confirmed_order"].clone()).unwrap();
 let mut occupied=basis.schedule.occupied.clone();occupied.query.as_mut().unwrap().excluded_subject=Some(owner_pin(&basis,&derived).unwrap());occupied.digest=occupancy_snapshot_digest(&occupied).unwrap();
 let mut policy=LiveFoodPolicy{version:Revision(1),digest:PayloadDigest("0".repeat(64)),general_seconds:172800,rice_seconds:86400,maximum_reheats:1};policy.digest=live_food_policy_digest(&policy).unwrap();
 let typed:BrowserState=serde_json::from_value(state.clone()).unwrap();let current_view=RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();
 Fixture{state,original_view,current_view,basis,derived,approved,order,occupied,policy}
}
fn first_update(prepared:&PreparedCalendar)->CalendarUpdateProjection{CalendarUpdateProjection{scope:prepared.drafts.iter().map(|draft|ScopedEvent{key:draft.logical_key.clone(),prior:PriorEventState::NeverVersioned,disposition:UpdateDisposition::Upsert}).collect(),membership:prepared.drafts.iter().flat_map(|draft|draft.contributors.iter().map(|member|ContributorTransition{member:SessionIdentity{week_id:member.owner.week.week_id.clone(),session_id:member.session_id.clone()},before:None,after:Some(draft.logical_key.clone())})).collect()}}
fn retained_update(snapshot:&CalendarSnapshot)->CalendarUpdateProjection{CalendarUpdateProjection{scope:snapshot.events.iter().map(|row|ScopedEvent{key:row.event.logical_key.clone(),prior:PriorEventState::Retained(row.clone()),disposition:UpdateDisposition::Upsert}).collect(),membership:snapshot.events.iter().flat_map(|row|row.contributors.iter().map(|member|ContributorTransition{member:SessionIdentity{week_id:member.owner.week.week_id.clone(),session_id:member.session_id.clone()},before:Some(row.event.logical_key.clone()),after:Some(row.event.logical_key.clone())})).collect()}}

#[test]
fn compiled_unit_recipes_prepare_full_exact_bodies_with_unknown_product_warnings_and_no_order_effect(){
 let f=fixture(true,false);let snapshot=f.snapshot();let saved=f.state.clone();let digest=saved_basis_digest(&f.basis).unwrap();let prepared=prepare_personal_calendar(&f.inputs(&snapshot)).unwrap();assert_eq!(prepared.drafts.len(),3);assert!(prepared.reports.iter().all(|report|report.blocking.is_empty()));assert!(prepared.reports.iter().any(|report|!report.warnings.is_empty()));
 let porridge=prepared.drafts.iter().find(|draft|draft.body.0.contains("45 g")).unwrap();assert!(porridge.body.0.contains("180 ml"));assert!(porridge.body.0.contains("Reserve15minutes"));assert!(porridge.body.0.contains("CC-BY-SA-4.0"));assert!(porridge.body.0.contains("Diner 1: 1 servings"));
 let dinner=prepared.drafts.iter().find(|draft|draft.body.0.contains("125 g")).unwrap();for text in["55/2 g","2 each","1500 ml","60 ml","Reserve25minutes","robert5800","unknown"]{assert!(dinner.body.0.to_lowercase().contains(&text.to_lowercase()),"missing {text}");}
 for draft in[porridge,dinner]{assert!(draft.body.0.contains("full attention as an unmeasured estimate"));assert!(draft.body.0.contains("Actual completion is not guaranteed."));assert!(!draft.body.0.contains("includes active and unattended phases"));assert!(!draft.body.0.contains("not continuous hands-on work"));}
 assert_eq!(saved_basis_digest(&f.basis).unwrap(),digest);assert_eq!(f.state,saved);assert!(snapshot.weeks[0].confirmed_order.0.is_some());
}
#[test]
fn native_versions_keep_stable_uid_bytes_and_require_explicit_import_without_changing_booked_state(){
 let f=fixture(true,false);let snapshot=f.snapshot();let before=snapshot.clone();let prepared=prepare_personal_calendar(&f.inputs(&snapshot)).unwrap();let first=export_personal_calendar(&prepared,&first_update(&prepared),&snapshot.household_draft.household_id,&Instant(NOW.into())).unwrap();
 let again=export_personal_calendar(&prepared,&retained_update(&first.snapshot),&snapshot.household_draft.household_id,&Instant("2026-10-12T21:00:00Z".into())).unwrap();assert_eq!(first.ics,again.ics);assert_eq!(first.snapshot.events,again.snapshot.events);assert!(first.notice.contains("Import this file yourself"));assert!(first.notice.contains("does not synchronise"));
 let text=String::from_utf8(first.ics.clone()).unwrap();assert_eq!(text.matches("BEGIN:VEVENT").count(),3);assert!(!text.contains("VALARM"));assert!(!text.contains("METHOD:"));assert!(text.ends_with("END:VCALENDAR\r\n"));for line in text.split("\r\n"){assert!(line.len()<=75);}
 assert_eq!(snapshot,before);assert_eq!(snapshot.weeks[0].publication,PublicationState::NotRequested);
 let mut unavailable=retained_update(&first.snapshot);unavailable.scope[0].prior=PriorEventState::Unavailable;assert!(matches!(export_personal_calendar(&prepared,&unavailable,&snapshot.household_draft.household_id,&Instant(NOW.into())),Err(CalendarFailure::ReconciliationRequired{..})));
}
#[test]
fn later_aglio_catalogue_cannot_rewrite_the_original_r3_calendar_food_or_instruction_pins(){
 let mut f=fixture(false,false);let original=saved_basis_digest(&f.basis).unwrap();f.state=apply(&f.state,json!({"kind":"admit_reviewed_recipe","data":{"packet":aglio_packet_ref().unwrap(),"expected_catalogue_revision":"2"}}),0xc09);let typed:BrowserState=serde_json::from_value(f.state.clone()).unwrap();f.current_view=RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();let snapshot=f.snapshot();
 assert_eq!(f.original_view.pin().revision,Revision(2));assert_eq!(f.current_view.pin().revision,Revision(3));let prepared=prepare_personal_calendar(&f.inputs(&snapshot)).unwrap();assert_eq!(prepared.drafts.len(),2);assert!(prepared.drafts.iter().all(|draft|!draft.body.0.contains("125 g")));assert_eq!(saved_basis_digest(&f.basis).unwrap(),original);
}
#[test]
fn late_arrival_refuses_calendar_preparation_but_preserves_the_exact_confirmed_order(){
 let f=fixture(false,true);let snapshot=f.snapshot();let before=f.state.clone();assert!(check_all_prep_arrival(&f.order,&f.derived).is_err());assert!(prepare_personal_calendar(&f.inputs(&snapshot)).is_err());assert_eq!(f.state,before);assert_eq!(snapshot.weeks[0].confirmed_order.0.as_ref(),Some(&f.order));
 let mut equality=f.order.clone();equality.confirmation.delivery_end=f.derived.prep_sessions[0].ingredient_needed_at.clone();equality.confirmation.delivery_start=shifted(&equality.confirmation.delivery_end,-3600);assert!(check_all_prep_arrival(&equality,&f.derived).is_ok());
}
#[test]
fn forged_original_food_missing_occupancy_and_current_age_changes_do_not_gain_calendar_authority(){
 let mut f=fixture(false,false);let snapshot=f.snapshot();let original=f.basis.clone();f.basis.evidence.methods[0].evidence_ids=vec![EvidenceId("ev_00000000000000000000000000000099".into())];assert!(prepare_personal_calendar(&f.inputs(&snapshot)).is_err());f.basis=original;
 let occupied=f.occupied.clone();f.occupied.query.as_mut().unwrap().excluded_subject=None;f.occupied.digest=occupancy_snapshot_digest(&f.occupied).unwrap();assert!(prepare_personal_calendar(&f.inputs(&snapshot)).is_err());f.occupied=occupied;
 let mut profile=f.state["snapshot"]["profile"].clone();profile["diners"][0]["age_band"]=json!("teen");f.state=apply(&f.state,json!({"kind":"update_profile","data":{"profile":profile,"expected_profile_revision":profile["revision"]}}),0xc10);let changed=f.snapshot();assert!(changed.weeks[0].confirmed_order.0.is_some());assert!(prepare_personal_calendar(&f.inputs(&changed)).is_err());
}
#[test]
fn unqualified_attachments_or_changed_policy_refuse_before_creating_any_calendar_snapshot(){
 let mut f=fixture(false,false);let snapshot=f.snapshot();let key=f.basis.identities.sessions[0].0.clone();let basis=f.basis.clone();f.basis.schedule.layout.attachments.push(AttachmentChoice{session:key,target_owner:owner_pin(&basis,&f.derived).unwrap(),target_session:f.derived.prep_sessions[0].session_id.clone()});
 match prepare_personal_calendar(&f.inputs(&snapshot)){Err(CalendarFailure::Invalid(error))=>{assert_eq!(error.code,ErrorCode::UnsupportedCapability);assert_eq!(error.issues[0].path,"/calendar/attachments");},other=>panic!("unexpected {other:?}")};f.basis=basis;
 f.policy.general_seconds=172799;f.policy.digest=live_food_policy_digest(&f.policy).unwrap();match prepare_personal_calendar(&f.inputs(&snapshot)){Err(CalendarFailure::Invalid(error))=>assert_eq!(error.issues[0].path,"/calendar/current_policy"),other=>panic!("unexpected {other:?}")};
 assert_eq!(f.snapshot().weeks[0].confirmed_order.0.as_ref(),Some(&f.order));
}
