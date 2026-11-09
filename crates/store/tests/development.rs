//! Real current-schema durability and disposable-root boundaries.
use super::*;
use crate::database::Database;
fn root() -> PathBuf {
    std::env::temp_dir().join(format!(
        "mstrchf-m1-controls-{}-{}",
        std::process::id(),
        records::random_id().unwrap()
    ))
}
fn profile_command(s: &Snapshot, id: u64) -> AppCommandV1 {
    AppCommandV1 {
        schema_version: SchemaVersion,
        command_id: CommandId(format!("cm_{id:032x}")),
        expected_snapshot_revision: s.snapshot_revision,
        command: AppCommand::UpdateProfile {
            profile: development::seed_profile(s).unwrap(),
            expected_profile_revision: Revision(1),
        },
    }
}
fn save_command(s: &Snapshot, w: Week, id: u64) -> AppCommandV1 {
    AppCommandV1 {
        schema_version: SchemaVersion,
        command_id: CommandId(format!("cm_{id:032x}")),
        expected_snapshot_revision: s.snapshot_revision,
        command: AppCommand::SaveWeek {
            expected_week_revision: w.revision,
            week: WeekEdit {
                week_id: w.range.week_id,
                defaults_snapshot: w.defaults_snapshot,
                day_enabled: w.day_enabled,
                meal_type_enabled: w.meal_type_enabled,
                cells: w.cells,
                plan: Nullable(Some(w.plan)),
            },
        },
    }
}
fn seeded() -> (PathBuf, Store) {
    let p = root();
    let store = Store::open_development(StoreConfig {
        app_data_root: p.clone(),
    })
    .unwrap();
    development::seed_runtime(&store).unwrap();
    (p, store)
}

#[test]
fn m2_manual_all_meal_types_and_two_weeks_survive_reopen() {
    let (path,store)=seeded();
    let s=store.snapshot().unwrap();
    store.dispatch_ui(AppCommandV1{schema_version:SchemaVersion,command_id:CommandId(format!("cm_{:032x}",8100)),expected_snapshot_revision:s.snapshot_revision,command:AppCommand::OpenPlanningHorizon{}}).unwrap();
    for (n,(week_index,cell_index)) in [(0,2),(0,3),(0,4),(1,5)].into_iter().enumerate(){
        let s=store.snapshot().unwrap();let mut w=s.weeks[week_index].clone();let slot=w.cells[cell_index].slot.clone();
        let mut board=development::board_for(&s,Some(&w.range.week_id),Some(&slot)).unwrap();
        assert!(!board.choices.is_empty(),"native choice required for {:?}",slot);
        let chosen=board.choices.remove(0).selection;
        assert_eq!(chosen.slot,slot);
        w.plan.selections.0.push(chosen);
        store.dispatch_ui(save_command(&s,w,8101+n as u64)).unwrap();
    }
    let saved=store.snapshot().unwrap();assert_eq!(saved.weeks[0].plan.selections.len(),3);assert_eq!(saved.weeks[1].plan.selections.len(),1);
    let food=development::validate_saved_plan(&saved,saved.profile.0.as_ref().unwrap(),&saved.weeks[0]).unwrap().unwrap();
    assert_eq!(food.plan.batches.len(),3);assert_eq!(food.plan.prep_sessions.len(),3);
    assert_eq!(food.plan.demand.lines[0].quantity.amount,PositiveRational::new(600,1).unwrap());
    drop(store);assert_eq!(Store::open_development(StoreConfig{app_data_root:path}).unwrap().snapshot().unwrap(),saved);
}

#[test]
fn m2_reviewed_large_portion_is_native_and_exact() {
    let (_,store)=seeded();let s=store.snapshot().unwrap();let mut p=s.profile.0.clone().unwrap();p.diners[0].portions.preset=PortionPresetName::Larger;
    store.dispatch_ui(AppCommandV1{schema_version:SchemaVersion,command_id:CommandId(format!("cm_{:032x}",8110)),expected_snapshot_revision:s.snapshot_revision,command:AppCommand::UpdateProfile{expected_profile_revision:p.revision,profile:p}}).unwrap();
    let s=store.snapshot().unwrap();let mut w=s.weeks[0].clone();let board=development::board(&s).unwrap();
    assert!(!board.choices.is_empty(),"a recipe with its own reviewed large premise is required");
    let choice=board.choices[0].selection.clone();assert_eq!(choice.servings[0].servings,PositiveRational::new(3,2).unwrap());
    w.plan.selections=BoundedVec(vec![choice]);store.dispatch_ui(save_command(&s,w,8111)).unwrap();
    let s=store.snapshot().unwrap();let food=development::validate_saved_plan(&s,s.profile.0.as_ref().unwrap(),&s.weeks[0]).unwrap().unwrap();
    assert_eq!(food.plan.demand.lines[0].quantity.amount,PositiveRational::new(300,1).unwrap());
}

#[test]
fn m2_native_suggestion_keeps_a_fixed_meal_and_saves_an_open_slot(){
    let (_,store)=seeded();let s=store.snapshot().unwrap();let mut w=s.weeks[0].clone();
    w.plan.selections=BoundedVec(vec![development::board(&s).unwrap().choices.remove(0).selection]);
    store.dispatch_ui(save_command(&s,w,8120)).unwrap();let before=store.snapshot().unwrap();
    let command:AppCommandV1=parse_document(&serde_json::to_vec(&serde_json::json!({"schema_version":1,"command_id":format!("cm_{:032x}",8121),"expected_snapshot_revision":before.snapshot_revision,"command":{"kind":"suggest_meals","data":{"week_id":before.weeks[0].range.week_id,"mode":"fill_open","slots":[before.weeks[0].cells[3].slot],"allow_repeats":false}}})).unwrap()).unwrap();
    store.dispatch_ui(command).unwrap();let saved=store.snapshot().unwrap();
    assert_eq!(saved.weeks[0].plan.selections[0],before.weeks[0].plan.selections[0]);
    assert!(saved.weeks[0].plan.selections.iter().any(|v|v.slot==before.weeks[0].cells[3].slot&&v.origin==SelectionOrigin::Suggested));
}

#[test]
fn m2_manual_leftover_keeps_original_batch_and_demand_once(){
    let (_,store)=seeded();let s=store.snapshot().unwrap();let p=s.profile.0.as_ref().unwrap();let mut w=s.weeks[0].clone();
    assert!(!w.defaults_snapshot.suggest_leftovers);
    let source=crate::planner::custom_choices(&s,p,&w,&w.cells[2].slot,crate::planner::ChoiceOptions{make_extra:true,..Default::default()}).unwrap().remove(0).selection;
    let batch=match &source.batch_use{BatchUse::NewBatch{batch_id,production_servings}=>{assert_eq!(*production_servings,PositiveRational::new(2,1).unwrap());batch_id.clone()},_=>panic!("new batch")};
    let mut target=development::board_for(&s,Some(&w.range.week_id),Some(&w.cells[5].slot)).unwrap().choices.remove(0).selection;
    target.batch_use=BatchUse::Leftover{batch_id:batch.clone(),component_ids:BoundedVec(vec![ComponentId(format!("co_{:032x}",1))])};
    w.plan.batch_links=BoundedVec(vec![BatchLink{batch_id:batch,source_slot:source.slot.clone(),target_slot:target.slot.clone(),components:BoundedVec(vec![ComponentAllocation{component_id:ComponentId(format!("co_{:032x}",1)),quantity:Quantity{amount:PositiveRational::new(200,1).unwrap(),unit:Unit::G,form:IngredientForm::Cooked}}])}]);
    w.plan.selections=BoundedVec(vec![source,target]);
    store.dispatch_ui(save_command(&s,w,8130)).unwrap();let saved=store.snapshot().unwrap();let food=development::validate_saved_plan(&saved,saved.profile.0.as_ref().unwrap(),&saved.weeks[0]).unwrap().unwrap();
    assert_eq!(food.plan.batches.len(),1);assert_eq!(food.plan.demand.lines[0].quantity.amount,PositiveRational::new(400,1).unwrap());assert_eq!(saved.weeks[0].plan.batch_links.len(),1);
}

#[test]
fn m2_personal_recipe_is_an_immutable_candidate_without_invented_quantity(){
    let (path,store)=seeded();let s=store.snapshot().unwrap();
    let mut candidate=RecipeCandidate{schema_version:SchemaVersion,candidate_id:RecipeId(format!("rc_{:032x}",999)),proposed_revision:Revision(1),title:Title("Personal pasted recipe".into()),source:crate::planner::catalogue().unwrap()[0].source.clone(),ingredients:BoundedVec(vec![CandidateIngredient{ingredient_id:Nullable(None),name:Text("A personal ingredient".into()),quantity:Nullable(None),component_id:Nullable(None),source_text:Text("A personal ingredient".into())}]),r#yield:Nullable(None),steps:BoundedVec(vec![]),unresolved:BoundedVec(vec![]),image:Nullable(None)};
    let raw=serde_json::json!({"schema_version":1,"command_id":format!("cm_{:032x}",8140),"expected_snapshot_revision":s.snapshot_revision,"command":{"kind":"save_personal_recipe","data":{"candidate":candidate,"previous":null}}});
    store.dispatch_ui(parse_document(&serde_json::to_vec(&raw).unwrap()).unwrap()).unwrap();
    let board=store.planner_board(None,None,Default::default()).unwrap();let saved=board.details.unwrap().library.into_iter().find(|v|v.admission=="personal_candidate").unwrap();
    let bytes=canonical_bytes(&saved).unwrap();drop(store);
    let store=Store::open_development(StoreConfig{app_data_root:path}).unwrap();let s=store.snapshot().unwrap();
    candidate.title=Title("Personal edit".into());
    let raw=serde_json::json!({"schema_version":1,"command_id":format!("cm_{:032x}",8141),"expected_snapshot_revision":s.snapshot_revision,"command":{"kind":"save_personal_recipe","data":{"candidate":candidate,"previous":saved.reference}}});
    store.dispatch_ui(parse_document(&serde_json::to_vec(&raw).unwrap()).unwrap()).unwrap();
    let board=store.planner_board(None,None,Default::default()).unwrap();let entries=board.details.unwrap().library;
    assert!(entries.iter().any(|v|canonical_bytes(v).unwrap()==bytes));
    assert_eq!(entries.iter().filter(|v|v.admission=="personal_candidate").count(),2);
    assert!(board.choices.iter().all(|v|v.selection.recipe.recipe_id!=saved.reference.recipe_id));
}

#[test]
fn m2_excluded_choice_still_reports_a_new_hard_restriction(){
    let (_,store)=seeded();let s=store.snapshot().unwrap();let mut w=s.weeks[0].clone();w.plan.selections=BoundedVec(vec![development::board(&s).unwrap().choices.remove(0).selection]);
    store.dispatch_ui(save_command(&s,w,8150)).unwrap();let s=store.snapshot().unwrap();let mut w=s.weeks[0].clone();w.day_enabled[0]=false;store.dispatch_ui(save_command(&s,w,8151)).unwrap();
    let s=store.snapshot().unwrap();let mut p=s.profile.0.clone().unwrap();p.diners[0].restrictions=RestrictionDeclaration::Confirmed{confirmed_at:Instant("2026-01-01T12:00:00Z".into()),requirements:BoundedVec(vec![HardRequirement::Diet{code:DietCode::Vegan}])};
    store.dispatch_ui(AppCommandV1{schema_version:SchemaVersion,command_id:CommandId(format!("cm_{:032x}",8152)),expected_snapshot_revision:s.snapshot_revision,command:AppCommand::UpdateProfile{expected_profile_revision:p.revision,profile:p}}).unwrap();let saved=store.snapshot().unwrap();
    assert_eq!(saved.weeks[0].plan.selections,s.weeks[0].plan.selections);
    assert!(!saved.conflicts.is_empty(),"excluded choices still require hard restriction review");
    assert!(development::validate_saved_plan(&saved,saved.profile.0.as_ref().unwrap(),&saved.weeks[0]).unwrap().unwrap().plan.demand.lines.is_empty());
}

#[test]
fn m2_editable_profile_survives_reopen_without_replacing_saved_week() {
    let (p, store) = seeded();
    let before = store.snapshot().unwrap();
    let mut profile = before.profile.0.clone().unwrap();
    profile.preferences.spending_style = SpendingStyle::Flexible;
    profile.preferences.higher_protein = true;
    profile.diners[0].display_name = Nullable(Some(NameText("Engineering diner edit".into())));
    store.dispatch_ui(AppCommandV1 {
        schema_version: SchemaVersion, command_id: CommandId(format!("cm_{:032x}",8001)),
        expected_snapshot_revision: before.snapshot_revision,
        command: AppCommand::UpdateProfile { expected_profile_revision: profile.revision, profile },
    }).unwrap();
    let saved = store.snapshot().unwrap();
    assert_eq!(saved.weeks, before.weeks);
    assert_eq!(saved.profile.0.as_ref().unwrap().revision, Revision(2));
    drop(store);
    let reopened = Store::open_development(StoreConfig {app_data_root:p}).unwrap();
    assert_eq!(reopened.snapshot().unwrap(), saved);
}

#[test]
fn m2_four_native_weeks_survive_reopen_and_preserve_m1_selection() {
    let (p, store) = seeded();
    let before = store.snapshot().unwrap();
    let choice = development::board(&before).unwrap().choices.remove(0).selection;
    let mut week = before.weeks[0].clone();
    week.plan.selections = BoundedVec(vec![choice.clone()]);
    store.dispatch_ui(save_command(&before,week,8002)).unwrap();
    let before = store.snapshot().unwrap();
    let command:AppCommandV1 = parse_document(&serde_json::to_vec(&serde_json::json!({
        "schema_version":1,"command_id":format!("cm_{:032x}",8003),
        "expected_snapshot_revision":before.snapshot_revision,
        "command":{"kind":"open_planning_horizon","data":{}}
    })).unwrap()).unwrap();
    store.dispatch_ui(command).unwrap();
    let saved = store.snapshot().unwrap();
    assert_eq!(saved.weeks.len(),4);
    assert_eq!(saved.weeks[0],before.weeks[0]);
    assert_eq!(saved.weeks[0].plan.selections[0],choice);
    for pair in saved.weeks.windows(2) {
        let a=chrono::NaiveDate::parse_from_str(&pair[0].range.monday.0,"%Y-%m-%d").unwrap();
        let b=chrono::NaiveDate::parse_from_str(&pair[1].range.monday.0,"%Y-%m-%d").unwrap();
        assert_eq!(b-a,chrono::Duration::days(7));
        assert_ne!(pair[0].range.week_id,pair[1].range.week_id);
    }
    drop(store);
    let reopened=Store::open_development(StoreConfig{app_data_root:p}).unwrap();
    assert_eq!(reopened.snapshot().unwrap(),saved);
}

#[test]
fn m2_restriction_change_preserves_meal_and_reports_current_conflict() {
    let (_,store)=seeded();
    let before=store.snapshot().unwrap();
    let mut w=before.weeks[0].clone();
    w.plan.selections=BoundedVec(vec![development::board(&before).unwrap().choices.remove(0).selection]);
    store.dispatch_ui(save_command(&before,w,8010)).unwrap();
    let before=store.snapshot().unwrap();
    let mut p=before.profile.0.clone().unwrap();
    p.diners[0].restrictions=RestrictionDeclaration::Confirmed{confirmed_at:development::now().unwrap(),requirements:BoundedVec(vec![HardRequirement::AllergenExclusion{code:AllergenCode::Milk}])};
    store.dispatch_ui(AppCommandV1{schema_version:SchemaVersion,command_id:CommandId(format!("cm_{:032x}",8011)),expected_snapshot_revision:before.snapshot_revision,command:AppCommand::UpdateProfile{expected_profile_revision:p.revision,profile:p}}).unwrap();
    let after=store.snapshot().unwrap();
    assert_eq!(after.weeks,before.weeks);
    assert!(!after.conflicts.is_empty(),"unknown milk evidence must remain a visible blocking conflict");
    assert!(after.conflicts.iter().all(|c|c.blocking));
}

#[test]
fn m2_overlapping_exclusions_retain_selection_and_restore_in_either_order(){
    for day_first in [true,false]{
        let (p,store)=seeded();let s=store.snapshot().unwrap();let mut w=s.weeks[0].clone();
        w.plan.selections=BoundedVec(vec![development::board(&s).unwrap().choices.remove(0).selection]);
        store.dispatch_ui(save_command(&s,w,8020)).unwrap();
        let original=store.snapshot().unwrap().weeks[0].plan.selections.clone();
        for (n,enabled) in [false,false,true,true].into_iter().enumerate(){
            let s=store.snapshot().unwrap();let mut w=s.weeks[0].clone();
            if (n%2==0)==day_first {w.day_enabled[0]=enabled;}else{w.meal_type_enabled.dinner=enabled;}
            store.dispatch_ui(save_command(&s,w,8021+n as u64)).unwrap();
            let s=store.snapshot().unwrap();assert_eq!(s.weeks[0].plan.selections,original);
            let active=mstrchf_core::availability::eligible_slots(&s.weeks[0]).unwrap();
            assert_eq!(active.contains(&original[0].slot),n==3);
        }
        let saved=store.snapshot().unwrap();drop(store);
        assert_eq!(Store::open_development(StoreConfig{app_data_root:p}).unwrap().snapshot().unwrap(),saved);
    }
}
#[test]
fn m2_initial_profile_requires_a_bound_draft_then_allows_valid_settings_edits() {
    let p = root();
    let store = Store::open_development(StoreConfig { app_data_root: p }).unwrap();
    let before = store.snapshot().unwrap();
    let mut wrong = profile_command(&before, 1);
    if let AppCommand::UpdateProfile { profile, .. } = &mut wrong.command {
        profile.diners[0].restrictions = RestrictionDeclaration::ConfirmedNone {
            confirmed_at: Instant("2026-01-01T12:00:00Z".into()),
        };
    }
    assert_eq!(
        store.dispatch_ui(wrong).unwrap_err().code,
        ErrorCode::MissingField
    );
    assert_eq!(store.snapshot().unwrap(), before);
    store.dispatch_ui(profile_command(&before, 2)).unwrap();
    let before = store.snapshot().unwrap();
    let mut wrong = profile_command(&before, 3);
    if let AppCommand::UpdateProfile { profile, .. } = &mut wrong.command {
        profile.diners[0].restrictions = RestrictionDeclaration::ConfirmedNone {
            confirmed_at: Instant("2026-01-01T12:00:00Z".into()),
        };
    }
    store.dispatch_ui(wrong).unwrap();
    let saved=store.snapshot().unwrap();
    assert_eq!(saved.profile.0.as_ref().unwrap().revision,Revision(2));
    assert_eq!(development::board(&saved).unwrap().choices.len(),5);
}
#[test]
fn development_reopen_refuses_an_altered_synthetic_profile() {
    let (p, store) = seeded();
    let mut profile = store.snapshot().unwrap().profile.0.unwrap();
    drop(store);
    profile.diners[0].restrictions = RestrictionDeclaration::ConfirmedNone {
        confirmed_at: Instant("2026-01-01T12:00:00Z".into()),
    };
    let path = p.join("planner.sqlite");
    let c = rusqlite::Connection::open(&path).unwrap();
    // Preserve the M1-only source admission rule before the additive M2 upgrade.
    c.execute_batch("DROP TABLE essentials; DROP TABLE recipe_library; DROP TABLE planner_history; DROP TABLE local_assets; DROP TABLE active_weeks; PRAGMA user_version=1001;").unwrap();
    c.execute(
        "UPDATE household SET profile=?1",
        [canonical_bytes(&profile).unwrap()],
    )
    .unwrap();
    drop(c);
    let before = std::fs::read(&path).unwrap();
    assert!(Store::open_development(StoreConfig { app_data_root: p }).is_err());
    assert_eq!(std::fs::read(path).unwrap(), before);
}
#[test]
fn development_reopen_refuses_altered_week_structure_without_writing() {
    for variant in 0..5 {
        let (p, store) = seeded();
        let state = store.snapshot().unwrap();
        let mut week = state.weeks[0].clone();
        let diner = &state.profile.0.as_ref().unwrap().diners[0];
        drop(store);
        match variant {
            0 => {
                week.cells[2].attendee_ids = BoundedVec(vec![]);
                week.cells[2].occasion = Nullable(Some(Occasion {
                    occasion_id: OccasionId(format!("oc_{:032x}", 1)),
                    host_diner_ids: BoundedVec(vec![]),
                    guests: BoundedVec(vec![Guest {
                        guest_id: GuestId(format!("gu_{:032x}", 1)),
                        age_band: diner.age_band,
                        restrictions: RestrictionDeclaration::ConfirmedNone {
                            confirmed_at: Instant("2026-01-01T12:00:00Z".into()),
                        },
                        portion: diner.portions.clone(),
                    }]),
                }));
            }
            1 => week.cells.0.swap(0, 2),
            2 => week.day_enabled[0] = false,
            3 => week.meal_type_enabled.dinner = false,
            _ => week.defaults_snapshot.cells.0.swap(0, 2),
        }
        let body = canonical_bytes(&week).unwrap();
        assert_eq!(parse_document::<Week>(&body).unwrap(), week);
        let path = p.join("planner.sqlite");
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute_batch("DROP TABLE essentials; DROP TABLE recipe_library; DROP TABLE planner_history; DROP TABLE local_assets; DROP TABLE active_weeks; PRAGMA user_version=1001;").unwrap();
        c.execute("UPDATE weeks SET body=?1", [body]).unwrap();
        drop(c);
        let before = std::fs::read(&path).unwrap();
        assert!(
            Store::open_development(StoreConfig { app_data_root: p }).is_err(),
            "altered non-plan variant {variant} must be refused"
        );
        assert_eq!(std::fs::read(path).unwrap(), before, "variant {variant}");
    }
}
#[test]
fn development_reopen_keeps_a_valid_saved_range_without_reseeding() {
    let (p, store) = seeded();
    let mut saved = store.snapshot().unwrap();
    saved.weeks = BoundedVec(vec![]);
    // Simulate a week originally created by the native seed at this earlier time.
    let prior = development::seed_week(&saved, &Instant("2026-01-05T12:00:00Z".into())).unwrap();
    assert_eq!(prior.range.monday.0, "2026-01-12");
    saved.weeks = BoundedVec(vec![prior.clone()]);
    drop(store);
    let c = rusqlite::Connection::open(p.join("planner.sqlite")).unwrap();
    c.execute(
        "UPDATE weeks SET body=?1",
        [canonical_bytes(&prior).unwrap()],
    )
    .unwrap();
    drop(c);
    let store = Store::open_development(StoreConfig { app_data_root: p }).unwrap();
    assert_eq!(store.snapshot().unwrap(), saved);
}
#[test]
fn m2_admits_scoped_guest_and_availability_edits_without_replacing_choices() {
    let (_, store) = seeded();
    for variant in 0..4 {
        let before = store.snapshot().unwrap();
        let mut w = before.weeks[0].clone();
        match variant {
            0 => {
                let diner = &before.profile.0.as_ref().unwrap().diners[0];
                w.cells[2].attendee_ids = BoundedVec(vec![]);
                w.cells[2].occasion = Nullable(Some(Occasion {
                    occasion_id: OccasionId(format!("oc_{:032x}", 1)),
                    host_diner_ids: BoundedVec(vec![]),
                    guests: BoundedVec(vec![Guest {
                        guest_id: GuestId(format!("gu_{:032x}", 1)),
                        age_band: diner.age_band,
                        restrictions: RestrictionDeclaration::ConfirmedNone {
                            confirmed_at: Instant("2026-01-01T12:00:00Z".into()),
                        },
                        portion: diner.portions.clone(),
                    }]),
                }));
            }
            1 => w.cells.0.swap(0, 2),
            2 => w.day_enabled[0] = false,
            _ => w.meal_type_enabled.dinner = false,
        }
        store.dispatch_ui(save_command(&before,w.clone(),10+variant)).unwrap();
        let saved=store.snapshot().unwrap();
        assert_eq!(saved.weeks[0].cells,w.cells);
        assert_eq!(saved.weeks[0].plan.selections,before.weeks[0].plan.selections);
    }
}
#[test]
fn development_refuses_dangling_batch_links_even_with_no_selections() {
    let (_, store) = seeded();
    let before = store.snapshot().unwrap();
    let mut w = before.weeks[0].clone();
    w.plan.batch_links = BoundedVec(vec![BatchLink {
        batch_id: BatchId(format!("ba_{:032x}", 1)),
        source_slot: w.cells[2].slot.clone(),
        target_slot: w.cells[5].slot.clone(),
        components: BoundedVec(vec![ComponentAllocation {
            component_id: ComponentId(format!("co_{:032x}", 1)),
            quantity: Quantity {
                amount: PositiveRational::new(1, 1).unwrap(),
                unit: Unit::G,
                form: IngredientForm::Cooked,
            },
        }]),
    }]);
    assert_eq!(
        store
            .dispatch_ui(save_command(&before, w, 1))
            .unwrap_err()
            .code,
        ErrorCode::MissingReference
    );
    assert_eq!(store.snapshot().unwrap(), before);
    assert_eq!(development::board(&before).unwrap().choices.len(), 4);
}
#[test]
fn development_privacy_change_keeps_the_exact_saved_meal_after_reopen() {
    let (p, store) = seeded();
    let s = store.snapshot().unwrap();
    let mut w = s.weeks[0].clone();
    w.plan.selections = BoundedVec(vec![
        development::board(&s).unwrap().choices.remove(0).selection,
    ]);
    store.dispatch_ui(save_command(&s, w, 1)).unwrap();
    let s = store.snapshot().unwrap();
    store
        .dispatch_ui(AppCommandV1 {
            schema_version: SchemaVersion,
            command_id: CommandId(format!("cm_{:032x}", 2)),
            expected_snapshot_revision: s.snapshot_revision,
            command: AppCommand::SetPrivacy {
                policy: s.privacy.clone(),
                expected_policy_revision: s.privacy.revision,
            },
        })
        .unwrap();
    let saved = store.snapshot().unwrap();
    assert_eq!(saved.privacy.revision, Revision(2));
    assert_eq!(saved.profile.0.as_ref().unwrap().revision, Revision(2));
    assert_eq!(saved.weeks[0].plan.selections, s.weeks[0].plan.selections);
    drop(store);
    let store = Store::open_development(StoreConfig { app_data_root: p }).unwrap();
    assert_eq!(store.snapshot().unwrap(), saved);
    assert_eq!(development::board(&saved).unwrap().choices.len(), 4);
}
#[test]
fn development_refuses_existing_user_state_without_changing_it() {
    let p = root();
    let store = Store::open(StoreConfig {
        app_data_root: p.clone(),
    })
    .unwrap();
    let mut s = store.snapshot().unwrap();
    let mut draft = s.household_draft.clone();
    draft.preferences = Nullable(Some(development::seed_profile(&s).unwrap().preferences));
    draft.completed_stages = BoundedVec(vec![HouseholdStage::FoodPreferences]);
    let cmd = AppCommandV1 {
        schema_version: SchemaVersion,
        command_id: CommandId(format!("cm_{:032x}", 1)),
        expected_snapshot_revision: s.snapshot_revision,
        command: AppCommand::SaveHouseholdDraft {
            draft,
            restriction_issue_edits: BoundedVec(vec![]),
        },
    };
    store.dispatch_ui(cmd).unwrap();
    s = store.snapshot().unwrap();
    drop(store);
    assert_eq!(
        Store::open_development(StoreConfig {
            app_data_root: p.clone()
        })
        .unwrap_err()
        .code,
        ErrorCode::AuthorityDenied
    );
    assert_eq!(
        Store::open(StoreConfig { app_data_root: p })
            .unwrap()
            .snapshot()
            .unwrap(),
        s
    );
}
#[test]
fn current_write_rollback_and_commit_return_error_preserve_exactly_once_state() {
    for fault in ["before_commit_error", "after_commit_error"] {
        let p = root();
        let mut db = Database::open_development(&p).unwrap();
        let before = db.snapshot().unwrap();
        let command = profile_command(&before, 1);
        let bytes = canonical_bytes(&command).unwrap();
        crate::tests::inject(fault);
        let result = db.dispatch_outcome(&command, &bytes);
        assert!(!crate::tests::fault(fault), "fault path must be reached");
        if fault == "before_commit_error" {
            assert!(result.is_err());
            assert_eq!(db.snapshot().unwrap(), before);
            assert!(db.replay(&command.command_id.0, &bytes).is_err());
        } else {
            assert!(result.is_ok());
            assert_eq!(db.snapshot().unwrap().snapshot_revision, Revision(2));
            let again = db.dispatch_outcome(&command, &bytes).unwrap();
            assert_eq!(again.disposition(), CommandDisposition::Replayed);
            assert_eq!(db.snapshot().unwrap().snapshot_revision, Revision(2));
        }
        let saved = db.snapshot().unwrap();
        drop(db);
        assert_eq!(
            Database::open_development(&p).unwrap().snapshot().unwrap(),
            saved
        );
    }
}
#[test]
fn normal_store_cannot_open_current_development_schema() {
    let p = root();
    let store = Store::open_development(StoreConfig {
        app_data_root: p.clone(),
    })
    .unwrap();
    drop(store);
    assert!(
        Store::open(StoreConfig {
            app_data_root: p.clone()
        })
        .is_err()
    );
    assert!(Store::open_development(StoreConfig { app_data_root: p }).is_ok());
}
#[test]
fn unknown_development_schema_is_refused_before_writable_access() {
    let p = root();
    let store = Store::open_development(StoreConfig {
        app_data_root: p.clone(),
    })
    .unwrap();
    drop(store);
    let path = p.join("planner.sqlite");
    let c = rusqlite::Connection::open(&path).unwrap();
    c.execute_batch("CREATE TABLE unrelated(value TEXT);")
        .unwrap();
    drop(c);
    let before = std::fs::read(&path).unwrap();
    assert!(Store::open_development(StoreConfig { app_data_root: p }).is_err());
    assert_eq!(std::fs::read(path).unwrap(), before);
}
#[test]
fn inconsistent_base_metadata_is_refused_without_writing() {
    for sql in [
        "UPDATE schema_migrations SET checksum='changed' WHERE version=1;",
        "UPDATE app_meta SET settings_revision='2' WHERE singleton=1;",
    ] {
        let p = root();
        let store = Store::open_development(StoreConfig {
            app_data_root: p.clone(),
        })
        .unwrap();
        drop(store);
        let path = p.join("planner.sqlite");
        let c = rusqlite::Connection::open(&path).unwrap();
        c.execute_batch(sql).unwrap();
        drop(c);
        let before = std::fs::read(&path).unwrap();
        assert!(
            Store::open_development(StoreConfig { app_data_root: p }).is_err(),
            "{sql}"
        );
        assert_eq!(std::fs::read(path).unwrap(), before);
    }
}
