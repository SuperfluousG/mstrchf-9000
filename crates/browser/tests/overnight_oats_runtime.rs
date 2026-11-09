//! Exact packet integration after adoption; household, timing and handling are synthetic.
use mstrchf_browser::{
    BoardOptions, BrowserState, catalogue::RuntimeCatalogue, maker_packets::*, personal_scope::*,
    run_json,
};
use mstrchf_contracts::*;
use mstrchf_core::food::{FoodDerivation, SavedFoodBasis};
use serde_json::{Value, json};
use std::sync::OnceLock;

const NOW: &str = "2026-10-15T12:00:00Z";
fn call(input: Value) -> Value {
    serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap()
}
fn good(input: Value) -> Value {
    let answer = call(input);
    assert_eq!(answer["ok"], true, "{answer}");
    answer["value"].clone()
}
fn command(state: &Value, body: Value, id: u32) -> Value {
    json!({"schema_version":1,"command_id":format!("cm_{id:032x}"),
        "expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body})
}
fn apply(state: &Value, body: Value, id: u32) -> Value {
    good(json!({"op":"dispatch","state":state,"command":command(state,body,id),"now":NOW}))["state"]
        .clone()
}
fn week_body(week: &Value) -> Value {
    json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{
        "week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],
        "day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],
        "cells":week["cells"],"plan":week["plan"]}}})
}
fn pin() -> ReviewedRecipePacketRef {
    available_packet_refs()
        .unwrap()
        .into_iter()
        .find(|value| value.recipe.recipe_id.0 == OVERNIGHT_OATS_SCOPE.recipe_id)
        .expect("Root must adopt the exact overnight oats packet before this suite runs")
}
fn setup() -> Value {
    let mut state = serde_json::from_slice::<Value>(include_bytes!(
        "fixtures/legacy-engineering-receipts.json"
    ))
    .unwrap()["state"]
        .clone();
    let mut profile = state["snapshot"]["profile"].clone();
    profile["diners"][0]["age_band"] = json!("adult");
    profile["diners"][0]["restrictions"] =
        json!({"kind":"confirmed_none","data":{"confirmed_at":NOW}});
    profile["diners"][0]["portions"] = json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});
    profile["preferences"]["equipment"] = json!(["fridge"]);
    profile["preferences"]["mobility"] =
        json!({"reheating":false,"cold_storage":false,"temperature":"cold"});
    state = apply(
        &state,
        json!({"kind":"update_profile","data":{
        "expected_profile_revision":profile["revision"],"profile":profile}}),
        0xfb1400,
    );
    let mut week = state["snapshot"]["weeks"][0].clone();
    week["plan"]["selections"] = json!([]);
    week["plan"]["batch_links"] = json!([]);
    week["meal_type_enabled"]["breakfast"] = json!(true);
    week["cells"][0]["choice"] = json!("at_home");
    apply(&state, week_body(&week), 0xfb1401)
}
fn ready() -> Value {
    static READY: OnceLock<Value> = OnceLock::new();
    READY.get_or_init(|| {
        let state = setup();
        apply(&state, json!({"kind":"admit_reviewed_recipe","data":{"packet":pin(),
            "expected_catalogue_revision":(1 + state["reviewed_packets"].as_array().map_or(0,Vec::len)).to_string()}}), 0xfb1402)
    }).clone()
}
fn board(state: &Value, index: usize, options: Value) -> Value {
    let week = &state["snapshot"]["weeks"][0];
    good(
        json!({"op":"board","state":state,"week_id":week["range"]["week_id"],
        "slot":week["cells"][index]["slot"],"options":options,"now":NOW}),
    )
}
fn choices(board: &Value) -> Vec<&Value> {
    board["choices"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|choice| choice["selection"]["recipe"] == json!(pin().recipe))
        .collect()
}
fn selected(state: &Value, options: Value, id: u32) -> Value {
    let b = board(state, 0, options);
    let offered = choices(&b);
    assert_eq!(
        offered.len(),
        1,
        "Missing scope {}; total choices {}; food error {}",
        OVERNIGHT_OATS_SCOPE.scope_id,
        b["choices"].as_array().map_or(0, Vec::len),
        b["details"]["food_error"]
    );
    let mut week = state["snapshot"]["weeks"][0].clone();
    week["plan"] = offered[0]["plan"].clone();
    apply(state, week_body(&week), id)
}
fn saved() -> Value {
    static SAVED: OnceLock<Value> = OnceLock::new();
    SAVED
        .get_or_init(|| selected(&ready(), Value::Null, 0xfb1403))
        .clone()
}
fn cold(state: Value) -> Value {
    std::thread::spawn(move || good(json!({"op":"validate_state","state":state,"now":NOW})))
        .join()
        .unwrap()
}
fn material() -> (SavedFoodBasis, FoodDerivation) {
    let state: BrowserState = serde_json::from_value(saved()).unwrap();
    let view = RuntimeCatalogue::from_records(&state.reviewed_packets.0).unwrap();
    mstrchf_browser::calendar_view::material_at(
        &view,
        &state.snapshot,
        state.snapshot.profile.0.as_ref().unwrap(),
        &state.snapshot.weeks[0],
        &Instant(NOW.into()),
    )
    .unwrap()
    .unwrap()
}
fn moment(date: &str, time: &str, zone: &str) -> ZonedMoment {
    use chrono::TimeZone;
    let tz: chrono_tz::Tz = zone.parse().unwrap();
    let local =
        chrono::NaiveDateTime::parse_from_str(&format!("{date}T{time}"), "%Y-%m-%dT%H:%M:%S")
            .unwrap();
    let instant = tz
        .from_local_datetime(&local)
        .single()
        .unwrap()
        .with_timezone(&chrono::Utc)
        .to_rfc3339_opts(chrono::SecondsFormat::Secs, true);
    ZonedMoment::new(
        LocalDate(date.into()),
        LocalTime(time.into()),
        ZoneId(zone.into()),
        Resolution::Unambiguous,
        Instant(instant),
    )
    .unwrap()
}

#[test]
fn exact_admission_retains_old_receipts_and_reloads_one_monday_unit() {
    let before = setup();
    assert!(choices(&board(&before, 0, Value::Null)).is_empty());
    let admitted = ready();
    assert_eq!(
        &admitted["receipts"].as_array().unwrap()[..before["receipts"].as_array().unwrap().len()],
        before["receipts"].as_array().unwrap()
    );
    let state = saved();
    assert_eq!(state["snapshot"]["profile"], before["snapshot"]["profile"]);
    assert_eq!(state["snapshot"]["privacy"], before["snapshot"]["privacy"]);
    let selections = state["snapshot"]["weeks"][0]["plan"]["selections"]
        .as_array()
        .unwrap();
    assert_eq!(selections.len(), 1);
    assert_eq!(selections[0]["slot"]["date"], "2026-10-19");
    assert_eq!(selections[0]["recipe"], json!(pin().recipe));
    assert_eq!(selections[0]["batch_use"]["kind"], "new_batch");
    assert!(is_overnight_oats(&pin().recipe));
    let mut stale = pin().recipe;
    stale.revision = Revision(2);
    assert!(!is_overnight_oats(&stale));
    assert_eq!(cold(state.clone()), state);
}

#[test]
fn native_clock_and_two_sessions_start_on_prior_sunday_without_duplicate_demand() {
    let (basis, food) = material();
    let derived = &food.plan;
    assert_eq!(derived.batches.len(), 1);
    assert_eq!(derived.allocations.len(), 1);
    assert_eq!(derived.prep_sessions.len(), 2);
    let mut sessions = derived.prep_sessions.iter().collect::<Vec<_>>();
    sessions.sort_by_key(|session| &session.start.instant);
    let prep = sessions[0];
    let service = sessions[1];
    assert_eq!(
        prep.start,
        moment("2026-10-18", "19:55:00", "Europe/London")
    );
    assert_eq!(prep.end, moment("2026-10-18", "20:00:00", "Europe/London"));
    assert_eq!(
        service.start,
        moment("2026-10-19", "07:55:00", "Europe/London")
    );
    assert_eq!(
        service.end,
        moment("2026-10-19", "08:00:00", "Europe/London")
    );
    assert_eq!(prep.ingredient_needed_at, prep.start);
    assert!(prep.attached_to.0.is_none());
    assert!(service.attached_to.0.is_none());
    assert_eq!(
        prep.linked_slots.0,
        vec![basis.plan.selections[0].slot.clone()]
    );
    assert_eq!(prep.tasks.len(), 1);
    assert_eq!(prep.tasks[0].prep_id, 1);
    assert_eq!(service.tasks.len(), 1);
    assert_eq!(service.tasks[0].prep_id, 2);
    assert_eq!(service.tasks[0].depends_on[0].session_id, prep.session_id);
    let component = &derived.batches[0].components[0];
    assert_eq!(
        component.original_production,
        ProductionMoment::Unheated {
            preparation_started_at: prep.start.clone()
        }
    );
    assert_eq!(
        component
            .storage_rule
            .0
            .as_ref()
            .unwrap()
            .max_chilled_seconds,
        86_400
    );
    assert_eq!(component.storage_rule.0.as_ref().unwrap().max_reheats, 0);
    assert_eq!(
        component.produced.amount,
        PositiveRational::new(1, 1).unwrap()
    );
    assert_eq!(component.produced.unit, Unit::Each);
    assert_eq!(component.produced.form, IngredientForm::Prepared);
    assert_eq!(derived.allocations[0].planned_reheats, 0);
    assert_eq!(derived.demand.lines.len(), 5);
    let packet = known_packet(&pin()).unwrap();
    assert!(!packet.recipe().capabilities.leftovers);
    for ingredient in packet.recipe().ingredients.iter() {
        let lines = derived
            .demand
            .lines
            .iter()
            .filter(|line| line.ingredient_id == ingredient.ingredient_id)
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 1);
        assert_eq!(&lines[0].quantity, ingredient.quantity.0.as_ref().unwrap());
    }
    mstrchf_core::prep::validate_schedule(
        &basis.plan,
        &basis.context,
        &basis.evidence,
        &basis.identities,
        &basis.schedule,
        &derived.prep_sessions,
    )
    .unwrap();
    let mut shortened = derived.prep_sessions.0.clone();
    let changed = shortened
        .iter_mut()
        .find(|session| session.tasks[0].prep_id == 1)
        .unwrap();
    changed.start = moment("2026-10-19", "05:00:00", "Europe/London");
    assert!(
        mstrchf_core::prep::validate_schedule(
            &basis.plan,
            &basis.context,
            &basis.evidence,
            &basis.identities,
            &basis.schedule,
            &shortened
        )
        .is_err()
    );
    mstrchf_core::prep::check_delivery(&prep.start, &prep.ingredient_needed_at).unwrap();
    assert!(
        mstrchf_core::prep::check_delivery(
            &moment("2026-10-18", "19:55:01", "Europe/London"),
            &prep.ingredient_needed_at
        )
        .is_err()
    );
    assert!(
        mstrchf_core::prep::check_delivery(&service.start, &prep.ingredient_needed_at).is_err()
    );
}

#[test]
fn monday_window_boundary_requests_and_dst_local_dates_are_checked() {
    let state = ready();
    for time in ["06:00:00", "10:00:00"] {
        assert_eq!(
            choices(&board(&state, 0, json!({"eat_by_time":time}))).len(),
            1,
            "{time}"
        );
    }
    for time in ["05:59:59", "10:00:01"] {
        assert!(
            choices(&board(&state, 0, json!({"eat_by_time":time}))).is_empty(),
            "{time}"
        );
    }
    assert!(
        choices(&board(&state, 3, Value::Null)).is_empty(),
        "Tuesday is outside this definition"
    );
    let (basis, _) = material();
    let selected = &basis.plan.selections[0];
    let meal = basis
        .context
        .requirements
        .iter()
        .find(|meal| meal.slot == selected.slot)
        .unwrap();
    let packet = known_packet(&pin()).unwrap();
    for date in ["2026-10-26", "2027-03-29"] {
        let mut m = meal.clone();
        let mut s = selected.clone();
        m.slot.date = LocalDate(date.into());
        s.slot = m.slot.clone();
        let time = moment(date, "06:00:00", "Europe/London");
        m.eat_by = Nullable(Some(time.clone()));
        s.meal_overrides.eat_by = Nullable(Some(time));
        let mut plan = basis.plan.clone();
        plan.selections = BoundedVec(vec![s.clone()]);
        assert!(
            check_personal_use_scope(
                packet.recipe(),
                &s.recipe,
                &m,
                &s,
                &plan,
                &BoardOptions::default()
            )
            .is_ok(),
            "{date}"
        );
    }
    let options = BoardOptions {
        eat_by_time: Some(LocalTime("09:00:00".into())),
        ..Default::default()
    };
    assert!(
        check_personal_use_scope(
            packet.recipe(),
            &selected.recipe,
            meal,
            selected,
            &basis.plan,
            &options
        )
        .is_err()
    );
}

#[test]
fn home_fridge_resident_unit_and_fresh_only_guards_do_not_use_travel_flags() {
    let (basis, _) = material();
    let selected = &basis.plan.selections[0];
    let meal = basis
        .context
        .requirements
        .iter()
        .find(|meal| meal.slot == selected.slot)
        .unwrap();
    let packet = known_packet(&pin()).unwrap();
    let check =
        |m: &MealRequirements, s: &MealSelection, plan: &WeekPlan, options: &BoardOptions| {
            check_personal_use_scope(packet.recipe(), &s.recipe, m, s, plan, options)
        };
    assert!(check(meal, selected, &basis.plan, &Default::default()).is_ok());
    for mutate in 0..7 {
        let mut m = meal.clone();
        let mut s = selected.clone();
        match mutate {
            0 => m.equipment.0.clear(),
            1 => m.location = MealLocation::OnTheMove,
            2 => m.attendees[0].age_band = AgeBand::Child4To6,
            3 => {
                m.attendees[0].hard_requirements = BoundedVec(vec![HardRequirement::Diet {
                    code: DietCode::Vegetarian,
                }])
            }
            4 => m.attendees[0].portion.adjustment = PositiveRational::new(2, 1).unwrap(),
            5 => {
                m.attendees[0].person = PersonRef::Guest {
                    occasion_id: OccasionId("oc_80000000000000000000000000000014".into()),
                    guest_id: GuestId("gu_80000000000000000000000000000014".into()),
                };
                s.servings[0].person = m.attendees[0].person.clone();
            }
            _ => {
                s.batch_use = BatchUse::Leftover {
                    batch_id: match &selected.batch_use {
                        BatchUse::NewBatch { batch_id, .. } => batch_id.clone(),
                        _ => unreachable!(),
                    },
                    component_ids: BoundedVec(vec![ComponentId(
                        "co_80000000000000000000000000000014".into(),
                    )]),
                }
            }
        }
        assert!(
            check(&m, &s, &basis.plan, &Default::default()).is_err(),
            "mutation {mutate}"
        );
    }
    for options in [
        BoardOptions {
            make_extra: Some(true),
            ..Default::default()
        },
        BoardOptions {
            leftover_source: Some(selected.slot.clone()),
            ..Default::default()
        },
    ] {
        assert!(check(meal, selected, &basis.plan, &options).is_err());
    }
    let mut plan = basis.plan.clone();
    let mut other = selected.clone();
    other.slot.date = LocalDate("2026-10-20".into());
    plan.selections.0.push(other);
    assert!(check(meal, selected, &plan, &Default::default()).is_err());
    let notices = notices_for_recipe(&selected.recipe.recipe_id).join(" ");
    assert!(notices.contains("not observed"));
    assert!(notices.contains("Unknown or failed"));
    assert!(notices.contains("by the start of Sunday mixing"));
}

#[test]
fn changed_restrictions_keep_saved_food_and_prior_sunday_work_as_a_conflict() {
    let state = saved();
    let mut profile = state["snapshot"]["profile"].clone();
    profile["diners"][0]["restrictions"] = json!({"kind":"confirmed","data":{"confirmed_at":NOW,
        "requirements":[{"kind":"allergen_exclusion","data":{"code":"milk"}}]}});
    let changed = apply(
        &state,
        json!({"kind":"update_profile","data":{
        "expected_profile_revision":profile["revision"],"profile":profile}}),
        0xfb1404,
    );
    assert_eq!(
        changed["snapshot"]["weeks"][0]["plan"],
        state["snapshot"]["weeks"][0]["plan"]
    );
    assert_eq!(changed["history"], state["history"]);
    assert!(
        !changed["snapshot"]["conflicts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(board(&changed, 0, Value::Null)["details"]["derived"].is_null());
    assert_eq!(cold(changed.clone()), changed);
}
