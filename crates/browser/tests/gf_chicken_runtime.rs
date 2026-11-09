//! Synthetic owner-flow checks for the separately adopted conditional GF pair.
//! Requires Root's real packet binding; no test substitutes invented admissions.
//! These checks do not establish actual product suitability or physical handling.
use mstrchf_browser::{
    BoardOptions, BrowserState, catalogue::RuntimeCatalogue, maker_packets::*, personal_scope::*,
    run_json,
};
use mstrchf_contracts::*;
use serde_json::{Value, json};
use std::sync::OnceLock;

const NOW: &str = "2026-10-18T12:00:00Z";
const CELL: usize = 4;

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
fn week_body(week: &Value, plan: bool) -> Value {
    json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{
        "week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],
        "day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],
        "cells":week["cells"],"plan":if plan {week["plan"].clone()} else {Value::Null}}}})
}
fn edit(state: &Value, week: &Value, id: u32, plan: bool) -> Value {
    apply(state, week_body(week, plan), id)
}
fn diet(code: &str) -> Value {
    json!({"kind":"diet","data":{"code":code}})
}
fn declaration(requirements: &[Value]) -> Value {
    if requirements.is_empty() {
        json!({"kind":"confirmed_none","data":{"confirmed_at":NOW}})
    } else {
        json!({"kind":"confirmed","data":{"confirmed_at":NOW,"requirements":requirements}})
    }
}
fn profile(state: &Value, profile: Value, id: u32) -> Value {
    apply(
        state,
        json!({"kind":"update_profile","data":{
        "expected_profile_revision":profile["revision"],"profile":profile}}),
        id,
    )
}
fn pin(scope: &PersonalUseScope) -> ReviewedRecipePacketRef {
    available_packet_refs()
        .unwrap()
        .into_iter()
        .find(|reference| reference.recipe.recipe_id.0 == scope.recipe_id)
        .expect("Root must bind the reviewed GF pair before running this integration suite")
}
fn admit(state: &Value, packet: ReviewedRecipePacketRef, id: u32) -> Value {
    apply(
        state,
        json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,
        "expected_catalogue_revision":(1+state["reviewed_packets"].as_array().map_or(0,Vec::len)).to_string()}}),
        id,
    )
}
fn setup(guest: bool, first: &[Value], second: &[Value]) -> Value {
    let mut state = serde_json::from_slice::<Value>(include_bytes!(
        "fixtures/legacy-engineering-receipts.json"
    ))
    .unwrap()["state"]
        .clone();
    let mut privacy = state["snapshot"]["privacy"].clone();
    privacy["use_ai_features"] = json!(false);
    privacy["include_household_requirements"] = json!(false);
    state = apply(
        &state,
        json!({"kind":"set_privacy","data":{
        "expected_policy_revision":privacy["revision"],"policy":privacy}}),
        0xff1000,
    );
    let mut p = state["snapshot"]["profile"].clone();
    p["diners"][0]["age_band"] = json!("adult");
    p["diners"][0]["restrictions"] = declaration(first);
    p["diners"][0]["portions"] = json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});
    p["preferences"]["equipment"] = json!(["fridge", "hob"]);
    if !guest {
        let mut other = p["diners"][0].clone();
        other["diner_id"] = json!("dn_8000000000000000000000000000000f");
        other["restrictions"] = declaration(second);
        p["diners"].as_array_mut().unwrap().push(other);
    }
    state = profile(&state, p, 0xff1001);
    let mut w = state["snapshot"]["weeks"][0].clone();
    w["plan"]["selections"] = json!([]);
    w["plan"]["batch_links"] = json!([]);
    w["meal_type_enabled"]["lunch"] = json!(true);
    w["cells"][CELL]["choice"] = json!("at_home");
    if guest {
        w["cells"][CELL]["occasion"] = json!({"occasion_id":"oc_8000000000000000000000000000000f",
            "host_diner_ids":w["cells"][CELL]["attendee_ids"],"guests":[{
            "guest_id":"gu_8000000000000000000000000000000f","age_band":"adult",
            "restrictions":declaration(second),"portion":{"preset":"regular",
            "adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]}}]});
    } else {
        w["cells"][CELL]["attendee_ids"]
            .as_array_mut()
            .unwrap()
            .push(json!("dn_8000000000000000000000000000000f"));
    }
    state = edit(&state, &w, 0xff1002, true);
    state = admit(&state, pin(&CHILLED_GF_CHICKEN_SCOPE), 0xff1003);
    admit(&state, pin(&THREE_BEAN_GF_CHICKEN_SCOPE), 0xff1004)
}
fn ready() -> Value {
    static STATE: OnceLock<Value> = OnceLock::new();
    STATE
        .get_or_init(|| setup(true, &[diet("vegetarian")], &[diet("gluten_free")]))
        .clone()
}
fn person(state: &Value, second: bool) -> Value {
    let cell = &state["snapshot"]["weeks"][0]["cells"][CELL];
    if second && cell["occasion"].is_object() {
        json!({"kind":"guest","data":{"occasion_id":cell["occasion"]["occasion_id"],
            "guest_id":cell["occasion"]["guests"][0]["guest_id"]}})
    } else {
        json!({"kind":"diner","data":{"diner_id":state["snapshot"]["profile"]["diners"][usize::from(second)]["diner_id"]}})
    }
}
fn board(state: &Value, options: Value) -> Value {
    let w = &state["snapshot"]["weeks"][0];
    good(
        json!({"op":"board","state":state,"week_id":w["range"]["week_id"],
        "slot":w["cells"][CELL]["slot"],"options":options,"now":NOW}),
    )
}
fn offered(board: &Value, scope: &PersonalUseScope) -> bool {
    board["choices"]
        .as_array()
        .unwrap()
        .iter()
        .any(|choice| choice["selection"]["recipe"] == json!(pin(scope).recipe))
}
fn select(state: &Value, options: Value, id: u32) -> Value {
    let b = board(state, options);
    let choice = b["choices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|choice| {
            choice["selection"]["recipe"] == json!(pin(&THREE_BEAN_GF_CHICKEN_SCOPE).recipe)
        })
        .unwrap_or_else(|| panic!("missing exact GF choice: {b}"));
    let mut w = state["snapshot"]["weeks"][0].clone();
    w["plan"] = choice["plan"].clone();
    edit(state, &w, id, true)
}
fn saved() -> Value {
    static STATE: OnceLock<Value> = OnceLock::new();
    STATE
        .get_or_init(|| {
            let s = ready();
            select(&s, json!({"extra_recipients":[person(&s,true)]}), 0xff1005)
        })
        .clone()
}
fn cold(state: Value) -> Value {
    std::thread::spawn(move || good(json!({"op":"validate_state","state":state,"now":NOW})))
        .join()
        .unwrap()
}

#[test]
fn shared_two_diet_base_keeps_named_chicken_quantity_and_separate_method_once() {
    let state = saved();
    let b = board(&state, Value::Null);
    let d = &b["details"]["derived"];
    assert!(d.is_object(), "{b}");
    let selection = &state["snapshot"]["weeks"][0]["plan"]["selections"][0];
    assert_eq!(selection["servings"].as_array().unwrap().len(), 2);
    assert_eq!(selection["extras"].as_array().unwrap().len(), 1);
    assert_eq!(
        selection["extras"][0]["recipients"]
            .as_array()
            .unwrap()
            .len(),
        1
    );
    assert_eq!(
        selection["extras"][0]["recipients"][0]["person"],
        person(&state, true)
    );
    let extra = known_packet(&pin(&CHILLED_GF_CHICKEN_SCOPE)).unwrap();
    let ingredient = &extra.recipe().ingredients[0].ingredient_id;
    let lines = d["demand"]["lines"].as_array().unwrap();
    assert_eq!(lines.len(), 10);
    let chicken = lines
        .iter()
        .find(|line| line["ingredient_id"] == json!(ingredient))
        .unwrap();
    assert_eq!(
        chicken["quantity"],
        json!({"amount":{"numerator":"50","denominator":"1"},"unit":"g","form":"cooked"})
    );
    assert_eq!(chicken["origin_extras"], json!([GF_CHICKEN_EXTRA_ID]));
    assert_eq!(d["prep_sessions"].as_array().unwrap().len(), 2);
    let tasks: Vec<_> = d["prep_sessions"]
        .as_array()
        .unwrap()
        .iter()
        .flat_map(|s| s["tasks"].as_array().unwrap())
        .collect();
    for (scope, expected) in [
        (
            &THREE_BEAN_GF_CHICKEN_SCOPE,
            vec![(1, "11:00:00", "11:30:00"), (2, "11:30:00", "12:30:00")],
        ),
        (&CHILLED_GF_CHICKEN_SCOPE, vec![(1, "12:25:00", "12:30:00")]),
    ] {
        let actual: Vec<_> = tasks
            .iter()
            .filter(|task| task["recipe"]["recipe_id"] == scope.recipe_id)
            .collect();
        assert_eq!(actual.len(), expected.len());
        for (id, start, end) in expected {
            let task = actual.iter().find(|task| task["prep_id"] == id).unwrap();
            assert_eq!(task["elapsed"]["start"]["local_time"], start);
            assert_eq!(task["elapsed"]["end"]["local_time"], end);
        }
    }
    assert_eq!(cold(state.clone()), state);
}

#[test]
fn none_recipient_and_both_diets_base_work_without_broadening_the_old_pair() {
    let state = setup(false, &[diet("vegetarian"), diet("gluten_free")], &[]);
    let base = select(&state, Value::Null, 0xff1010);
    assert_eq!(
        board(&base, Value::Null)["details"]["derived"]["demand"]["lines"]
            .as_array()
            .unwrap()
            .len(),
        9
    );
    let extra = select(
        &base,
        json!({"extra_recipients":[person(&base,true)]}),
        0xff1011,
    );
    assert_eq!(
        extra["snapshot"]["weeks"][0]["plan"]["selections"][0]["extras"][0]["recipients"][0]["person"],
        person(&base, true)
    );
    assert!(!offered(
        &board(&state, Value::Null),
        &CHILLED_GF_CHICKEN_SCOPE
    ));
    let old = admit(
        &state,
        chilled_chicken_extra_packet_ref().unwrap(),
        0xff1012,
    );
    let old = admit(
        &old,
        three_bean_two_unit_chicken_extra_packet_ref().unwrap(),
        0xff1013,
    );
    assert!(!offered(
        &board(&old, json!({"extra_recipients":[person(&old,true)]})),
        &THREE_BEAN_TWO_UNIT_CHICKEN_EXTRA_SCOPE
    ));
}

#[test]
fn both_none_diners_keep_conditional_food_and_unverified_product_handling_notices() {
    let state = setup(false, &[], &[]);
    let base = select(&state, Value::Null, 0xff1050);
    let pair = select(
        &base,
        json!({"extra_recipients":[person(&base,true)]}),
        0xff1051,
    );
    for (state, expected_demands) in [(&base, 9), (&pair, 10)] {
        let selected = &state["snapshot"]["weeks"][0]["plan"]["selections"][0];
        assert_eq!(
            selected["recipe"],
            json!(pin(&THREE_BEAN_GF_CHICKEN_SCOPE).recipe)
        );
        let b = board(state, Value::Null);
        assert_eq!(
            b["details"]["derived"]["demand"]["lines"]
                .as_array()
                .unwrap()
                .len(),
            expected_demands
        );
        let library = b["details"]["library"].as_array().unwrap();
        for scope in [&THREE_BEAN_GF_CHICKEN_SCOPE, &CHILLED_GF_CHICKEN_SCOPE] {
            let entry = library
                .iter()
                .find(|entry| entry["reference"] == json!(pin(scope).recipe))
                .unwrap();
            let notices = entry["notices"]
                .as_array()
                .unwrap()
                .iter()
                .map(|notice| notice.as_str().unwrap())
                .collect::<Vec<_>>()
                .join("\n");
            assert!(
                notices.contains("Recipe conditions do not qualify an actual pack"),
                "{notices}"
            );
            assert!(notices.contains("If required suitability or handling conditions cannot be established, do not use the affected food"), "{notices}");
            assert!(
                notices.contains("no observed temperature, completion"),
                "{notices}"
            );
            if scope.recipe_id == THREE_BEAN_GF_CHICKEN_SCOPE.recipe_id {
                assert!(
                    notices.contains("All intrinsic input conditions apply even to None consumers"),
                    "{notices}"
                );
            } else {
                assert!(notices.contains("Establish actual cooked cold-ready-to-eat product suitability, labels, dates and proper prior chilling"), "{notices}");
                assert!(
                    notices.contains("These conditions are not observations"),
                    "{notices}"
                );
            }
        }
    }
    assert_eq!(
        pair["snapshot"]["weeks"][0]["plan"]["selections"][0]["extras"][0]["recipients"][0]["person"],
        person(&pair, true)
    );
    assert_eq!(cold(pair.clone()), pair);
}

#[test]
fn vegetarian_duplicate_foreign_and_cross_pair_extra_allocations_are_refused() {
    let state = ready();
    for people in [
        vec![person(&state, false)],
        vec![person(&state, false), person(&state, true)],
    ] {
        assert!(!offered(
            &board(&state, json!({"extra_recipients":people})),
            &THREE_BEAN_GF_CHICKEN_SCOPE
        ));
    }
    let state = saved();
    for attack in 0..4 {
        let mut w = state["snapshot"]["weeks"][0].clone();
        let extra = &mut w["plan"]["selections"][0]["extras"][0];
        match attack {
            0 => extra["recipients"][0]["person"] = person(&state, false),
            1 => {
                let other = extra["recipients"][0].clone();
                extra["recipients"].as_array_mut().unwrap().push(other);
            }
            2 => {
                extra["recipients"][0]["person"]["data"]["guest_id"] =
                    json!("gu_80000000000000000000000000000010")
            }
            _ => {
                extra["recipe"] = json!(chilled_chicken_extra_packet_ref().unwrap().recipe);
                extra["extra_id"] = json!(CHICKEN_EXTRA_ID);
            }
        }
        let response = call(
            json!({"op":"dispatch","state":state,"command":command(&state,week_body(&w,true),0xff1020+attack),"now":NOW}),
        );
        assert_eq!(response["ok"], false, "{response}");
    }
    assert_eq!(cold(state.clone()), state);
}

#[test]
fn exact_scope_rejects_other_requirements_compensated_portions_and_missing_home_fridge() {
    let state = saved();
    let typed: BrowserState = serde_json::from_value(state).unwrap();
    let view = RuntimeCatalogue::from_records(&typed.reviewed_packets.0).unwrap();
    let (basis, _) = mstrchf_browser::calendar_view::material_at(
        &view,
        &typed.snapshot,
        typed.snapshot.profile.0.as_ref().unwrap(),
        &typed.snapshot.weeks[0],
        &Instant(NOW.into()),
    )
    .unwrap()
    .unwrap();
    let recipe = known_packet(&pin(&THREE_BEAN_GF_CHICKEN_SCOPE))
        .unwrap()
        .recipe();
    let selected = &basis.plan.selections[0];
    let meal = basis
        .context
        .requirements
        .iter()
        .find(|meal| meal.slot == selected.slot)
        .unwrap();
    let check = |m: &MealRequirements, s: &MealSelection, o: &BoardOptions| {
        check_personal_use_scope(recipe, &selected.recipe, m, s, &basis.plan, o)
    };
    assert!(check(meal, selected, &Default::default()).is_ok());
    for requirement in [
        HardRequirement::Diet {
            code: DietCode::Vegan,
        },
        HardRequirement::AllergenExclusion {
            code: AllergenCode::CerealsContainingGluten,
        },
        HardRequirement::AllergenExclusion {
            code: AllergenCode::Milk,
        },
        HardRequirement::IngredientExclusion {
            ingredient_id: recipe.ingredients[0].ingredient_id.clone(),
        },
    ] {
        let mut m = meal.clone();
        m.attendees[1].hard_requirements = BoundedVec(vec![requirement.clone()]);
        m.shared_hard_requirements = BoundedVec(vec![
            HardRequirement::Diet {
                code: DietCode::Vegetarian,
            },
            requirement,
        ]);
        assert!(m.validate_shape().is_ok());
        assert!(check(&m, selected, &Default::default()).is_err());
    }
    for age in [AgeBand::Child4To6, AgeBand::Teen] {
        let mut m = meal.clone();
        m.attendees[1].age_band = age;
        assert!(m.validate_shape().is_ok());
        assert!(check(&m, selected, &Default::default()).is_err());
    }
    let mut m = meal.clone();
    m.attendees[1].portion.adjustment = PositiveRational::new(2, 1).unwrap();
    let mut s = selected.clone();
    s.meal_overrides.portions = BoundedVec(vec![PersonPortionOverride {
        person: m.attendees[1].person.clone(),
        preset: PortionPresetName::Regular,
        adjustment: PositiveRational::new(1, 2).unwrap(),
    }]);
    assert!(check(&m, &s, &Default::default()).is_err());
    let mut m = meal.clone();
    m.equipment = BoundedVec(vec![EquipmentCode::Hob]);
    assert!(check(&m, selected, &Default::default()).is_err());
    let mut m = meal.clone();
    m.location = MealLocation::OnTheMove;
    assert!(check(&m, selected, &Default::default()).is_err());
    let mut m = meal.clone();
    m.attendees.0.truncate(1);
    m.shared_hard_requirements = m.attendees[0].hard_requirements.clone();
    assert!(m.validate_shape().is_ok());
    assert!(check(&m, selected, &Default::default()).is_err());
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
        assert!(check(meal, selected, &options).is_err());
    }
    let state = ready();
    assert!(!offered(
        &board(
            &state,
            json!({"eat_by_time":"01:29:59","time_resolution":"unambiguous"})
        ),
        &THREE_BEAN_GF_CHICKEN_SCOPE
    ));
    assert!(offered(
        &board(
            &state,
            json!({"eat_by_time":"01:30:00","time_resolution":"unambiguous"})
        ),
        &THREE_BEAN_GF_CHICKEN_SCOPE
    ));
}

#[test]
fn changed_or_departed_guest_retains_exact_extra_and_conflict_until_owner_removes_it() {
    let state = saved();
    let original = state["snapshot"]["weeks"][0]["plan"]["selections"].clone();
    let mut w = state["snapshot"]["weeks"][0].clone();
    w["cells"][CELL]["occasion"]["guests"][0]["restrictions"] =
        declaration(&[diet("vegetarian"), diet("gluten_free")]);
    let changed = edit(&state, &w, 0xff1030, false);
    assert_eq!(
        changed["snapshot"]["weeks"][0]["plan"]["selections"],
        original
    );
    assert!(
        !changed["snapshot"]["conflicts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(board(&changed, Value::Null)["details"]["derived"].is_null());
    let without_extra = select(&changed, Value::Null, 0xff1031);
    assert_eq!(
        without_extra["snapshot"]["weeks"][0]["plan"]["selections"][0]["extras"],
        json!([])
    );
    assert!(
        without_extra["snapshot"]["conflicts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    let mut w = state["snapshot"]["weeks"][0].clone();
    w["cells"][CELL]["occasion"]["guests"] = json!([]);
    let departed = edit(&state, &w, 0xff1032, false);
    assert_eq!(
        departed["snapshot"]["weeks"][0]["plan"]["selections"],
        original
    );
    assert_eq!(departed["history"], state["history"]);
    assert!(
        !departed["snapshot"]["conflicts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(board(&departed, Value::Null)["details"]["derived"].is_null());
    assert_eq!(
        &departed["receipts"].as_array().unwrap()[..state["receipts"].as_array().unwrap().len()],
        state["receipts"].as_array().unwrap()
    );
    assert_eq!(cold(changed.clone()), changed);
    assert_eq!(cold(departed.clone()), departed);
    assert_eq!(cold(without_extra.clone()), without_extra);
}

#[test]
fn unconfirmed_guest_refuses_and_deselected_resident_keeps_exact_saved_allocation() {
    let state = ready();
    let mut w = state["snapshot"]["weeks"][0].clone();
    w["cells"][CELL]["occasion"]["guests"][0]["restrictions"] =
        json!({"kind":"unconfirmed","data":{}});
    let unconfirmed = edit(&state, &w, 0xff1040, false);
    let w = &unconfirmed["snapshot"]["weeks"][0];
    let response = call(
        json!({"op":"board","state":unconfirmed,"week_id":w["range"]["week_id"],
        "slot":w["cells"][CELL]["slot"],"options":null,"now":NOW}),
    );
    assert!(response["ok"] == false || !offered(&response["value"], &THREE_BEAN_GF_CHICKEN_SCOPE));
    let state = setup(false, &[diet("vegetarian")], &[diet("gluten_free")]);
    let state = select(
        &state,
        json!({"extra_recipients":[person(&state,true)]}),
        0xff1041,
    );
    let mut w = state["snapshot"]["weeks"][0].clone();
    w["cells"][CELL]["attendee_ids"]
        .as_array_mut()
        .unwrap()
        .pop();
    let removed = edit(&state, &w, 0xff1042, false);
    assert_eq!(
        removed["snapshot"]["weeks"][0]["plan"]["selections"],
        state["snapshot"]["weeks"][0]["plan"]["selections"]
    );
    assert!(
        !removed["snapshot"]["conflicts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(board(&removed, Value::Null)["details"]["derived"].is_null());
    assert_eq!(cold(removed.clone()), removed);
    let mut w = removed["snapshot"]["weeks"][0].clone();
    w["plan"]["selections"] = json!([]);
    let cleared = edit(&removed, &w, 0xff1043, true);
    assert!(
        cleared["snapshot"]["conflicts"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert_eq!(cold(cleared.clone()), cleared);
}
