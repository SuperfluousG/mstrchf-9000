//! Public manual starter qualification through the real owner boundary.
//! Household identities, dates and declarations here are explicit fixtures.
use mstrchf_browser::{catalogue::RuntimeCatalogue, maker_packets, public_catalogue, run_json};
use mstrchf_contracts::*;
use serde_json::{Value, json};
const NOW: &str = "2026-12-18T12:00:00Z";
fn call(request: Value) -> Value {
    serde_json::from_slice(&run_json(&serde_json::to_vec(&request).unwrap())).unwrap()
}
fn good(request: Value) -> Value {
    let result = call(request);
    assert_eq!(result["ok"], true, "{result}");
    result["value"].clone()
}
fn envelope(state: &Value, command: Value, n: u64) -> Value {
    json!({"schema_version":1,"command_id":format!("cm_a000000000000000{n:016x}"),"expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":command})
}
fn apply(state: &Value, command: Value, n: u64) -> Value {
    good(json!({"op":"dispatch","state":state,"command":envelope(state,command,n),"now":NOW,"today":"2026-12-18"}))["state"].clone()
}
fn cold(state: &Value) -> Value {
    good(json!({"op":"validate_state","state":state,"now":NOW}))
}
fn base(adults: usize) -> Value {
    let mut state = good(
        json!({"op":"initialize","household_id":"hh_a0000000000000000000000000000001","diner_id":"dn_a0000000000000000000000000000001","zone":"Europe/London","today":"2026-12-18","now":NOW,"demo":false,"public_beta":true}),
    );
    let mut profile: HouseholdProfile = parse_document(include_bytes!(
        "../../../tests/fixtures/households/p03ab/a1a2-profile.json"
    ))
    .unwrap();
    profile.household_id =
        serde_json::from_value(state["snapshot"]["household_draft"]["household_id"].clone())
            .unwrap();
    profile.revision = Revision(1);
    profile.diners.0.truncate(adults);
    for (index, diner) in profile.diners.iter_mut().enumerate() {
        diner.diner_id = DinerId(format!("dn_a00000000000000000000000{:08x}", index + 1));
        diner.age_band = AgeBand::Adult;
        diner.restrictions = RestrictionDeclaration::ConfirmedNone {
            confirmed_at: Instant(NOW.into()),
        };
        diner.portions = PortionPreference {
            preset: PortionPresetName::Regular,
            adjustment: PositiveRational::new(1, 1).unwrap(),
            meal_overrides: BoundedVec(vec![]),
        };
    }
    profile.preferences.equipment = BoundedVec(vec![
        EquipmentCode::Fridge,
        EquipmentCode::Hob,
        EquipmentCode::Kettle,
        EquipmentCode::Oven,
        EquipmentCode::Toaster,
    ]);
    profile.preferences.preferred_hands_on_minutes = 60;
    profile.preferences.mobility.temperature = MobilityTemperature::Hot;
    profile.usual_week.day_enabled = BoundedVec(vec![true; 7]);
    profile.usual_week.enabled_meal_types =
        BoundedVec(vec![MealType::Breakfast, MealType::Dinner, MealType::Lunch]);
    profile.usual_week.suggest_leftovers = false;
    profile.usual_week.zone = ZoneId("Europe/London".into());
    for cell in profile.usual_week.cells.iter_mut() {
        cell.choice = MealLocation::AtHome;
        cell.attendee_ids = BoundedVec(profile.diners.iter().map(|d| d.diner_id.clone()).collect());
    }
    profile.connections.harness = Nullable(None);
    profile.connections.retailer = Nullable(None);
    profile.connections.calendar_destination = Nullable(None);
    profile.connections.privacy =
        serde_json::from_value(state["snapshot"]["privacy"].clone()).unwrap();
    let draft = mstrchf_core::setup::draft_from_profile(
        &profile,
        Revision(1),
        &profile.connections.privacy,
        &Instant(NOW.into()),
    )
    .unwrap();
    state = apply(
        &state,
        json!({"kind":"save_household_draft","data":{"draft":draft,"restriction_issue_edits":[]}}),
        1,
    );
    state = apply(
        &state,
        json!({"kind":"update_profile","data":{"profile":profile,"expected_profile_revision":"1"}}),
        2,
    );
    apply(&state, json!({"kind":"open_planning_horizon","data":{}}), 3)
}
fn board(state: &Value, week: usize, cell: usize) -> Value {
    good(
        json!({"op":"board","state":state,"week_id":state["snapshot"]["weeks"][week]["range"]["week_id"],"slot":state["snapshot"]["weeks"][week]["cells"][cell]["slot"],"now":NOW}),
    )
}
fn edit(state: &Value, week: Value, n: u64) -> Value {
    apply(
        state,
        json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{"week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],"day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],"cells":week["cells"],"plan":week["plan"]}}}),
        n,
    )
}
fn save(state: &Value, week_index: usize, cell: usize, recipe_index: usize, n: u64) -> Value {
    let reference = &public_catalogue::registry().unwrap()[recipe_index].reference;
    let choices = board(state, week_index, cell);
    let selected = choices["choices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["selection"]["recipe"] == json!(reference))
        .unwrap_or_else(|| {
            panic!(
                "missing public choice {recipe_index}; offered={:?}; food_error={}",
                choices["choices"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|c| &c["title"])
                    .collect::<Vec<_>>(),
                choices["details"]["food_error"]
            )
        });
    let mut week = state["snapshot"]["weeks"][week_index].clone();
    week["plan"] = selected["plan"].clone();
    edit(state, week, n)
}

#[test]
fn thirteen_distinct_public_starters_have_exact_image_and_food_authority() {
    let registry = public_catalogue::registry().unwrap();
    assert_eq!(registry.len(), 13);
    let mut names = std::collections::BTreeSet::new();
    for entry in registry {
        assert!(names.insert(&entry.recipe.title));
        assert!(entry.recipe.capabilities.starter_distribution);
        assert!(entry.recipe.capabilities.shopping);
        assert!(!entry.recipe.capabilities.leftovers);
        assert!(entry.recipe.image.0.is_some());
        assert!(entry.recipe.text_rights.distribution == RightsDistribution::Permitted);
        assert!(entry.recipe.steps.len() >= 3);
        assert!(
            entry
                .recipe
                .reviews
                .iter()
                .all(|review| review.kind != ReviewKind::CookTest)
        );
    }
    let inventory = good(json!({"op":"public_starter_catalogue"}));
    assert_eq!(inventory["recipes"].as_array().unwrap().len(), 13);
    assert!(
        inventory["recipes"]
            .as_array()
            .unwrap()
            .iter()
            .all(|entry| entry["admission"] == "public_reviewed")
    );
    let view = RuntimeCatalogue::public_starters().unwrap();
    assert_eq!(view.pin().entries.len(), 13);
    assert!(
        view.resolve_recipe(&maker_packets::porridge_packet_ref().unwrap().recipe)
            .is_err()
    );
    assert!(
        view.resolve_recipe(&RuntimeCatalogue::legacy().unwrap().pin().entries[0].recipe)
            .is_err()
    );
}

#[test]
fn new_public_kitchen_has_public_pin_without_mutating_old_bootstrap() {
    let state = base(1);
    assert!(state["public_catalogue_pin"].is_object());
    assert!(state.get("reviewed_packets").is_none());
    assert_eq!(cold(&state), state);
    assert_eq!(state["snapshot"]["privacy"]["use_ai_features"], false);
    assert_eq!(
        state["snapshot"]["privacy"]["include_household_requirements"],
        false
    );
    let legacy: Value =
        serde_json::from_slice(include_bytes!("fixtures/legacy-engineering-receipts.json"))
            .unwrap();
    assert_eq!(cold(&legacy["state"]), legacy["state"]);
    assert!(legacy["state"].get("public_catalogue_pin").is_none());
    let mut forged = state.clone();
    forged["public_catalogue_pin"]["entries"][0]["recipe"]["digest"] = json!("f".repeat(64));
    assert_eq!(
        call(json!({"op":"validate_state","state":forged,"now":NOW}))["ok"],
        false
    );
    let denied = envelope(
        &state,
        json!({"kind":"admit_reviewed_recipe","data":{"packet":maker_packets::porridge_packet_ref().unwrap(),"expected_catalogue_revision":"1"}}),
        90,
    );
    assert_eq!(
        call(json!({"op":"dispatch","state":state,"command":denied,"now":NOW}))["error"]["code"],
        "authority_denied"
    );
}

#[test]
fn explicit_public_demo_has_fictional_household_and_real_starter_choices() {
    let mut state = good(
        json!({"op":"initialize","household_id":"hh_a0000000000000000000000000000002","diner_id":"dn_a0000000000000000000000000000002","zone":"Europe/London","today":"2026-12-18","now":NOW,"demo":true,"public_beta":true}),
    );
    assert_eq!(state["engineering_demo"], true);
    assert!(state["public_catalogue_pin"].is_object());
    assert_eq!(
        state["snapshot"]["profile"]["diners"][0]["restrictions"]["kind"],
        "confirmed_none"
    );
    assert_eq!(
        state["snapshot"]["profile"]["connections"]["harness"],
        Value::Null
    );
    for (cell, index) in [(0, 7), (1, 10), (2, 0)] {
        let before = board(&state, 0, cell);
        assert_eq!(before["details"]["test_kitchen"], true);
        assert!(
            before["details"]["library"]
                .as_array()
                .unwrap()
                .iter()
                .all(|entry| entry["admission"] == "public_reviewed")
        );
        state = save(&state, 0, cell, index, 300 + index as u64);
    }
    assert_eq!(cold(&state), state);
    assert_eq!(
        board(&state, 0, 2)["details"]["shopping_list"]["contains_engineering"],
        false
    );
}

#[test]
fn one_and_two_adult_exact_servings_and_unsupported_scope() {
    for adults in [1, 2] {
        let state = base(adults);
        for (index, cell) in [(0, 2), (7, 0), (10, 1)] {
            let saved = save(&state, 0, cell, index, 100 + index as u64);
            let projection = board(&saved, 0, cell);
            let quantities = &projection["details"]["selected_recipe_quantities"];
            assert_eq!(
                quantities["requested_servings"],
                json!({"numerator":adults.to_string(),"denominator":"1"})
            );
            assert_eq!(
                quantities["multiplier"],
                if adults == 1 {
                    json!({"numerator":"1","denominator":"2"})
                } else {
                    json!({"numerator":"1","denominator":"1"})
                }
            );
            for ingredient in quantities["ingredients"].as_array().unwrap() {
                let demand = projection["details"]["shopping_list"]["lines"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .find(|line| line["demand"]["ingredient_id"] == ingredient["ingredient_id"])
                    .unwrap();
                assert_eq!(ingredient["quantity"], demand["demand"]["quantity"]);
            }
            assert_eq!(
                projection["details"]["shopping_list"]["contains_engineering"],
                false
            );
            assert_eq!(cold(&saved), saved);
            assert_eq!(
                saved["snapshot"]["weeks"][0]["plan"]["selections"][0]["servings"]
                    .as_array()
                    .unwrap()
                    .len(),
                adults
            );
        }
        let mut profile = state["snapshot"]["profile"].clone();
        profile["diners"][0]["age_band"] = json!("teen");
        let changed = apply(
            &state,
            json!({"kind":"update_profile","data":{"profile":profile,"expected_profile_revision":state["snapshot"]["profile"]["revision"]}}),
            120,
        );
        assert!(
            board(&changed, 0, 2)["choices"]
                .as_array()
                .unwrap()
                .is_empty()
        );
    }
    let state = base(1);
    let mut profile = state["snapshot"]["profile"].clone();
    profile["diners"][0]["restrictions"] = json!({"kind":"confirmed","data":{"confirmed_at":NOW,"requirements":[{"kind":"diet","data":{"code":"vegetarian"}}]}});
    let changed = apply(
        &state,
        json!({"kind":"update_profile","data":{"profile":profile,"expected_profile_revision":state["snapshot"]["profile"]["revision"]}}),
        130,
    );
    assert!(
        board(&changed, 0, 2)["choices"]
            .as_array()
            .unwrap()
            .is_empty()
    );
    assert!(board(&state, 0, 2)["choices"].as_array().unwrap().len() >= 7);
    let slot = state["snapshot"]["weeks"][0]["cells"][2]["slot"].clone();
    let extras = good(
        json!({"op":"board","state":state,"slot":slot,"options":{"make_extra":true},"now":NOW}),
    );
    assert!(extras["choices"].as_array().unwrap().is_empty());
}

#[test]
fn complete_real_week_four_week_independence_exclusion_pantry_and_reopen() {
    let mut state = base(2);
    assert_eq!(
        state["snapshot"]["weeks"][0]["range"]["monday"],
        "2026-12-28"
    );
    assert_eq!(
        state["snapshot"]["weeks"][1]["range"]["monday"],
        "2027-01-04"
    );
    let other = state["snapshot"]["weeks"][1].clone();
    let mut drafted = state["snapshot"]["weeks"][0].clone();
    // Build one atomic week edit from native choices against the same current
    // snapshot. The final save rechecks the complete combined plan; no receipt
    // or authoritative state is synthesised between these read-only choices.
    for day in 0..7 {
        for (cell, index) in [
            (day * 3, 7 + day % 3),
            (day * 3 + 1, 10 + day % 3),
            (day * 3 + 2, day),
        ] {
            let offered = board(&state, 0, cell);
            let reference = &public_catalogue::registry().unwrap()[index].reference;
            let choice = offered["choices"]
                .as_array()
                .unwrap()
                .iter()
                .find(|choice| choice["selection"]["recipe"] == json!(reference))
                .unwrap();
            drafted["plan"]["selections"]
                .as_array_mut()
                .unwrap()
                .push(choice["selection"].clone());
        }
    }
    state = edit(&state, drafted, 200);
    assert_eq!(state["snapshot"]["weeks"][1], other);
    assert_eq!(
        state["snapshot"]["weeks"][0]["plan"]["selections"]
            .as_array()
            .unwrap()
            .len(),
        21
    );
    assert_eq!(cold(&state), state);
    let projection = board(&state, 0, 2);
    let list = &projection["details"]["shopping_list"];
    assert_eq!(list["contains_engineering"], false);
    assert!(list["open_slots"].as_array().unwrap().is_empty());
    assert!(list["lines"].as_array().unwrap().iter().all(|line| {
        line["origins"]
            .as_array()
            .unwrap()
            .iter()
            .all(|origin| origin["admission"] == "public_reviewed")
    }));
    let row = &list["lines"][0];
    let confirmation = json!({"line_id":row["demand"]["line_id"],"choice":"have_enough","displayed_quantity":row["demand"]["quantity"],"demand_digest":list["demand_digest"],"confirmed_at":NOW});
    let pantry = apply(
        &state,
        json!({"kind":"confirm_essentials","data":{"confirmations":[confirmation]}}),
        250,
    );
    let current = board(&pantry, 0, 2);
    assert!(
        current["details"]["shopping_list"]["lines"]
            .as_array()
            .unwrap()
            .iter()
            .any(|line| line["action"] == "have_enough")
    );
    let mut week = pantry["snapshot"]["weeks"][0].clone();
    week["day_enabled"][0] = json!(false);
    let excluded = edit(&pantry, week, 251);
    assert_eq!(
        excluded["snapshot"]["weeks"][0]["plan"]["selections"],
        pantry["snapshot"]["weeks"][0]["plan"]["selections"]
    );
    let excluded_list = board(&excluded, 0, 5);
    assert!(
        excluded_list["details"]["shopping_list"]["lines"]
            .as_array()
            .unwrap()
            .iter()
            .all(|line| line["origins"]
                .as_array()
                .unwrap()
                .iter()
                .all(|origin| origin["source_slot"]["date"] != "2026-12-28"))
    );
    assert!(board(&excluded, 0, 2)["details"]["selected_recipe_quantities"].is_null());
    let mut restore = excluded["snapshot"]["weeks"][0].clone();
    restore["day_enabled"][0] = json!(true);
    let restored = edit(&excluded, restore, 252);
    assert_eq!(
        restored["snapshot"]["weeks"][0]["plan"]["selections"],
        state["snapshot"]["weeks"][0]["plan"]["selections"]
    );
    assert_eq!(cold(&restored), restored);
}
