//! Real packet integration after Root adoption; household and handling are synthetic.
//! Empty pins are deliberately not replaced by invented test admissions.
use mstrchf_browser::{
    catalogue::RuntimeCatalogue, maker_packets::*, personal_scope::*, run_json, BoardOptions,
    BrowserState,
};
use mstrchf_contracts::*;
use serde_json::{json, Value};
use std::sync::OnceLock;

const NOW: &str = "2026-10-14T15:00:00Z";
const SOURCE: usize = 5;
const FIRST: usize = 7;
const SECOND: usize = 10;
fn call(input: Value) -> Value {
    serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap()
}
fn good(input: Value) -> Value {
    let answer = call(input);
    assert_eq!(answer["ok"], true, "{answer}");
    answer["value"].clone()
}
fn command(state: &Value, body: Value, id: u32) -> Value {
    json!({"schema_version":1,
    "command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":body})
}
fn apply(state: &Value, body: Value, id: u32) -> Value {
    good(json!({"op":"dispatch","state":state,"command":command(state,body,id),"now":NOW}))["state"]
        .clone()
}
fn week_body(week: &Value, plan: bool) -> Value {
    json!({"kind":"save_week","data":{
    "expected_week_revision":week["revision"],"week":{"week_id":week["range"]["week_id"],
    "defaults_snapshot":week["defaults_snapshot"],"day_enabled":week["day_enabled"],
    "meal_type_enabled":week["meal_type_enabled"],"cells":week["cells"],
    "plan":if plan {week["plan"].clone()} else {Value::Null}}}})
}
fn pin(scope: &PersonalUseScope) -> ReviewedRecipePacketRef {
    available_packet_refs()
        .unwrap()
        .into_iter()
        .find(|pin| pin.recipe.recipe_id.0 == scope.recipe_id)
        .expect(
            "Root must adopt and bind the exact chilli family before this integration suite runs",
        )
}
fn admission(state: &Value, scope: &PersonalUseScope) -> Value {
    json!({"kind":"admit_reviewed_recipe","data":{
    "packet":pin(scope),"expected_catalogue_revision":(1+state["reviewed_packets"].as_array().map_or(0,Vec::len)).to_string()}})
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
    profile["preferences"]["equipment"] = json!(["fridge", "hob", "oven"]);
    profile["preferences"]["mobility"] = json!({"reheating":false,"cold_storage":false,"temperature":"cold"});
    state = apply(
        &state,
        json!({"kind":"update_profile","data":{
        "expected_profile_revision":profile["revision"],"profile":profile}}),
        0xfa1100,
    );
    let mut week = state["snapshot"]["weeks"][0].clone();
    week["plan"]["selections"] = json!([]);
    week["plan"]["batch_links"] = json!([]);
    week["meal_type_enabled"]["lunch"] = json!(true);
    week["meal_type_enabled"]["dinner"] = json!(true);
    for index in [SOURCE, FIRST, SECOND] {
        week["cells"][index]["choice"] = json!("at_home");
    }
    apply(&state, week_body(&week, true), 0xfa1101)
}
fn ready() -> Value {
    static READY: OnceLock<Value> = OnceLock::new();
    READY
        .get_or_init(|| {
            let mut state = setup();
            for (index, scope) in [
                &CHILLI_BATCH_SCOPE,
                &CHILLI_WRAPS_SCOPE,
                &CHILLI_POTATO_SCOPE,
            ]
            .iter()
            .enumerate()
            {
                state = apply(&state, admission(&state, scope), 0xfa1102 + index as u32);
            }
            state
        })
        .clone()
}
fn board(state: &Value, index: usize, options: Value) -> Value {
    let week = &state["snapshot"]["weeks"][0];
    good(
        json!({"op":"board","state":state,"week_id":week["range"]["week_id"],
        "slot":week["cells"][index]["slot"],"options":options,"now":NOW}),
    )
}
fn offered<'a>(board: &'a Value, scope: &PersonalUseScope) -> Vec<&'a Value> {
    board["choices"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|choice| choice["selection"]["recipe"] == json!(pin(scope).recipe))
        .collect()
}
fn source_options(state: &Value) -> Value {
    json!({"leftover_source":state["snapshot"]["weeks"][0]["cells"][SOURCE]["slot"]})
}
fn select(state: &Value, index: usize, scope: &PersonalUseScope, options: Value, id: u32) -> Value {
    let b = board(state, index, options);
    let choices = offered(&b, scope);
    assert_eq!(choices.len(), 1, "Missing exact scope {}; total choices {}; food error {}",
        scope.scope_id, b["choices"].as_array().map_or(0,Vec::len), b["details"]["food_error"]);
    let mut week = state["snapshot"]["weeks"][0].clone();
    week["plan"] = choices[0]["plan"].clone();
    apply(state, week_body(&week, true), id)
}
fn dinner() -> Value {
    select(&ready(), SOURCE, &CHILLI_BATCH_SCOPE, Value::Null, 0xfa1105)
}
fn saved() -> Value {
    static SAVED: OnceLock<Value> = OnceLock::new();
    SAVED
        .get_or_init(|| {
            let state = dinner();
            let state = select(
                &state,
                FIRST,
                &CHILLI_WRAPS_SCOPE,
                source_options(&state),
                0xfa1106,
            );
            select(
                &state,
                SECOND,
                &CHILLI_POTATO_SCOPE,
                source_options(&state),
                0xfa1107,
            )
        })
        .clone()
}
fn cold(state: Value) -> Value {
    std::thread::spawn(move || good(json!({"op":"validate_state","state":state,"now":NOW})))
        .join()
        .unwrap()
}

#[test]
fn services_require_separate_source_first_admission_and_an_explicit_saved_batch() {
    let state = setup();
    for (index, scope) in [&CHILLI_WRAPS_SCOPE, &CHILLI_POTATO_SCOPE]
        .iter()
        .enumerate()
    {
        let denied = call(json!({"op":"dispatch","state":state,
            "command":command(&state,admission(&state,scope),0xfa1110+index as u32),"now":NOW}));
        assert_eq!(denied["ok"], false, "{denied}");
    }
    let state = dinner();
    for scope in [&CHILLI_WRAPS_SCOPE, &CHILLI_POTATO_SCOPE] {
        assert!(offered(&board(&state, FIRST, Value::Null), scope).is_empty());
        assert_eq!(
            offered(&board(&state, FIRST, source_options(&state)), scope).len(),
            1
        );
    }
    assert_eq!(cold(state.clone()), state);
}

#[test]
fn three_meals_keep_one_original_batch_fourteen_demands_and_one_unallocated_share() {
    let before = ready();
    let state = saved();
    assert_eq!(state["snapshot"]["profile"]["preferences"]["mobility"],
        json!({"reheating":false,"cold_storage":false,"temperature":"cold"}),
        "Home meals must not require claims about access or temperature on the move");
    let b = board(&state, SECOND, source_options(&state));
    let derived = &b["details"]["derived"];
    assert!(derived.is_object(), "Missing derivation: {}", b["details"]["food_error"]);
    assert_eq!(derived["batches"].as_array().unwrap().len(), 1);
    let batch = &derived["batches"][0];
    assert_eq!(batch["recipe"], json!(pin(&CHILLI_BATCH_SCOPE).recipe));
    assert_eq!(
        batch["components"][0]["produced"]["amount"],
        json!({"numerator":"4","denominator":"1"})
    );
    assert_eq!(
        batch["components"][0]["unallocated"]["amount"],
        json!({"numerator":"1","denominator":"1"})
    );
    let allocations = derived["allocations"].as_array().unwrap();
    assert_eq!(allocations.len(), 3);
    assert_eq!(
        allocations
            .iter()
            .filter(|row| row["planned_reheats"] == 1)
            .count(),
        2
    );
    assert!(allocations.iter().all(|row|row["component_id"]==CHILLI_COMPONENT_ID
        && row["quantity"]==json!({"amount":{"numerator":"1","denominator":"1"},"unit":"each","form":"cooked"})));
    let demands = derived["demand"]["lines"].as_array().unwrap();
    assert_eq!(demands.len(), 14);
    let source_packet = known_packet(&pin(&CHILLI_BATCH_SCOPE)).unwrap();
    for ingredient in source_packet.recipe().ingredients.iter() {
        let lines = demands
            .iter()
            .filter(|line| line["ingredient_id"] == json!(ingredient.ingredient_id))
            .collect::<Vec<_>>();
        assert_eq!(lines.len(), 1);
        assert_eq!(
            lines[0]["quantity"],
            json!(ingredient.quantity.0.as_ref().unwrap())
        );
    }
    for scope in [&CHILLI_WRAPS_SCOPE, &CHILLI_POTATO_SCOPE] {
        let packet = known_packet(&pin(scope)).unwrap();
        let ingredient = &packet.recipe().ingredients[0];
        let line = demands
            .iter()
            .find(|line| line["ingredient_id"] == json!(ingredient.ingredient_id))
            .unwrap();
        assert_eq!(
            line["quantity"],
            json!(ingredient.quantity.0.as_ref().unwrap())
        );
    }
    let shopping = &b["details"]["shopping_list"];
    let rows = shopping["lines"].as_array().unwrap();
    assert_eq!(rows.len(), 14);
    assert_eq!(shopping["demand_digest"], derived["demand"]["digest"]);
    let cells = &state["snapshot"]["weeks"][0]["cells"];
    for (scope,index) in [(&CHILLI_BATCH_SCOPE,SOURCE),(&CHILLI_WRAPS_SCOPE,FIRST),(&CHILLI_POTATO_SCOPE,SECOND)] {
        let packet = known_packet(&pin(scope)).unwrap();
        for ingredient in packet.recipe().ingredients.iter() {
            let matching = rows.iter().filter(|row| row["demand"]["ingredient_id"] == json!(ingredient.ingredient_id)).collect::<Vec<_>>();
            assert_eq!(matching.len(),1,"Each fresh input appears once in the shopping projection");
            let row = matching[0];
            assert_eq!(row["name"],json!(ingredient.name));
            assert_eq!(row["demand"],*demands.iter().find(|line| line["ingredient_id"] == json!(ingredient.ingredient_id)).unwrap());
            let origins = row["origins"].as_array().unwrap();
            assert_eq!(origins.len(),1,"Do not attribute a remix input to the original producer");
            assert_eq!(origins[0]["recipe"],json!(packet.reference().recipe));
            assert_eq!(origins[0]["batch_id"],batch["batch_id"]);
            assert_eq!(origins[0]["source_slot"],cells[index]["slot"]);
            assert_eq!(origins[0]["target_slots"],if index == SOURCE {
                json!([cells[SOURCE]["slot"],cells[FIRST]["slot"],cells[SECOND]["slot"]])
            } else {json!([cells[index]["slot"]])});
        }
    }
    let tasks = derived["prep_sessions"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|session| session["tasks"].as_array().unwrap())
        .collect::<Vec<_>>();
    let endpoint = tasks
        .iter()
        .find(|task| {
            task["recipe"] == json!(pin(&CHILLI_BATCH_SCOPE).recipe) && task["prep_id"] == 3
        })
        .unwrap();
    assert_eq!(
        batch["components"][0]["original_production"]["data"]["completed_at"],
        endpoint["elapsed"]["end"]
    );
    assert_eq!(
        state["snapshot"]["weeks"][0]["plan"]["batch_links"]
            .as_array()
            .unwrap()
            .len(),
        2
    );
    assert_eq!(
        &state["snapshot"]["weeks"].as_array().unwrap()[1..],
        &before["snapshot"]["weeks"].as_array().unwrap()[1..]
    );
    assert_eq!(state["snapshot"]["privacy"], before["snapshot"]["privacy"]);
    assert_eq!(
        &state["receipts"].as_array().unwrap()[..before["receipts"].as_array().unwrap().len()],
        before["receipts"].as_array().unwrap()
    );
    assert_eq!(cold(state.clone()), state);
}

#[test]
fn either_order_is_supported_but_duplicate_services_chains_and_a_third_day_are_refused() {
    let state = dinner();
    let state = select(
        &state,
        FIRST,
        &CHILLI_POTATO_SCOPE,
        source_options(&state),
        0xfa1120,
    );
    assert!(offered(
        &board(&state, SECOND, source_options(&state)),
        &CHILLI_POTATO_SCOPE
    )
    .is_empty());
    let state = select(
        &state,
        SECOND,
        &CHILLI_WRAPS_SCOPE,
        source_options(&state),
        0xfa1121,
    );
    let wrong_source =
        json!({"leftover_source":state["snapshot"]["weeks"][0]["cells"][FIRST]["slot"]});
    assert!(offered(&board(&state, SECOND, wrong_source), &CHILLI_WRAPS_SCOPE).is_empty());
    let original = dinner();
    for index in [4, 13] {
        let b = board(&original, index, source_options(&original));
        assert!(offered(&b, &CHILLI_WRAPS_SCOPE).is_empty());
        assert!(offered(&b, &CHILLI_POTATO_SCOPE).is_empty());
    }
    assert!(offered(
        &board(&original, SOURCE, json!({"make_extra":true})),
        &CHILLI_BATCH_SCOPE
    )
    .is_empty());
    assert_eq!(cold(state.clone()), state);
}

#[test]
fn second_day_service_uses_original_thermal_deadline_not_dinner_or_remix_completion() {
    let state = dinner();
    let b = board(&state, SOURCE, Value::Null);
    let component = &b["details"]["derived"]["batches"][0]["components"][0];
    let time = component["original_production"]["data"]["completed_at"]["local_time"]
        .as_str()
        .unwrap();
    let mut options = source_options(&state);
    options["eat_by_time"] = json!(time);
    options["time_resolution"] = json!("unambiguous");
    assert_eq!(
        offered(&board(&state, SECOND, options.clone()), &CHILLI_WRAPS_SCOPE).len(),
        1
    );
    let late =
        chrono::NaiveTime::parse_from_str(time, "%H:%M:%S").unwrap() + chrono::Duration::seconds(1);
    options["eat_by_time"] = json!(late.format("%H:%M:%S").to_string());
    assert!(offered(&board(&state, SECOND, options), &CHILLI_WRAPS_SCOPE).is_empty());
}

#[test]
fn exact_scope_refuses_changed_cohorts_equipment_portions_and_source_links() {
    let typed: BrowserState = serde_json::from_value(saved()).unwrap();
    let view = RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();
    let week = &typed.snapshot.weeks[0];
    let context = mstrchf_browser::planner_view::context(
        &view,
        &typed.snapshot,
        typed.snapshot.profile.0.as_ref().unwrap(),
        week,
    )
    .unwrap();
    for selected in week.plan.selections.iter() {
        let recipe = view.resolve_recipe(&selected.recipe).unwrap();
        let meal = context
            .requirements
            .iter()
            .find(|meal| meal.slot == selected.slot)
            .unwrap();
        let check = |meal: &MealRequirements,
                     selection: &MealSelection,
                     plan: &WeekPlan,
                     options: &BoardOptions| {
            check_personal_use_scope(recipe, &selected.recipe, meal, selection, plan, options)
        };
        assert!(check(meal, selected, &week.plan, &Default::default()).is_ok());
        let mut bad = meal.clone();
        bad.attendees[0].age_band = AgeBand::Child4To6;
        assert!(check(&bad, selected, &week.plan, &Default::default()).is_err());
        let mut bad = meal.clone();
        bad.attendees[0].hard_requirements = BoundedVec(vec![HardRequirement::Diet {
            code: DietCode::Vegetarian,
        }]);
        bad.shared_hard_requirements = bad.attendees[0].hard_requirements.clone();
        assert!(check(&bad, selected, &week.plan, &Default::default()).is_err());
        let mut bad = meal.clone();
        bad.location = MealLocation::OnTheMove;
        assert!(check(&bad, selected, &week.plan, &Default::default()).is_err());
        let mut required = vec![EquipmentCode::Fridge, EquipmentCode::Hob];
        if selected.recipe.recipe_id.0 == CHILLI_POTATO_SCOPE.recipe_id {
            required.push(EquipmentCode::Oven);
        }
        for missing in required {
            let mut bad = meal.clone();
            bad.equipment.0.retain(|equipment| *equipment != missing);
            assert!(check(&bad, selected, &week.plan, &Default::default()).is_err());
        }
        let mut bad = meal.clone();
        bad.attendees[0].portion.adjustment = PositiveRational::new(2, 1).unwrap();
        let mut portion = selected.clone();
        portion.meal_overrides.portions = BoundedVec(vec![PersonPortionOverride {
            person: meal.attendees[0].person.clone(),
            preset: PortionPresetName::Regular,
            adjustment: PositiveRational::new(1, 2).unwrap(),
        }]);
        assert!(check(&bad, &portion, &week.plan, &Default::default()).is_err());
        let mut plan = week.plan.clone();
        plan.batch_links[0].components[0].quantity.amount = PositiveRational::new(2, 1).unwrap();
        assert!(check(meal, selected, &plan, &Default::default()).is_err());
        let mut plan = week.plan.clone();
        plan.batch_links[0].components[0].component_id =
            ComponentId("co_80000000000000000000000000000012".into());
        assert!(check(meal, selected, &plan, &Default::default()).is_err());
        let mut plan = week.plan.clone();
        if let BatchUse::NewBatch {
            production_servings,
            ..
        } = &mut plan.selections[0].batch_use
        {
            *production_servings = PositiveRational::new(3, 1).unwrap();
        }
        assert!(check(meal, selected, &plan, &Default::default()).is_err());
    }
}

#[test]
fn changed_current_requirements_retain_exact_saved_meals_and_report_conflicts() {
    let state = saved();
    let mut profile = state["snapshot"]["profile"].clone();
    profile["diners"][0]["restrictions"] = json!({"kind":"confirmed","data":{"confirmed_at":NOW,
        "requirements":[{"kind":"diet","data":{"code":"vegetarian"}}]}});
    let changed = apply(
        &state,
        json!({"kind":"update_profile","data":{
        "expected_profile_revision":profile["revision"],"profile":profile}}),
        0xfa1130,
    );
    assert_eq!(
        changed["snapshot"]["weeks"][0]["plan"],
        state["snapshot"]["weeks"][0]["plan"]
    );
    assert!(!changed["snapshot"]["conflicts"]
        .as_array()
        .unwrap()
        .is_empty());
    assert!(board(&changed, SECOND, source_options(&changed))["details"]["derived"].is_null());
    assert_eq!(changed["history"], state["history"]);
    assert_eq!(cold(changed.clone()), changed);
}
