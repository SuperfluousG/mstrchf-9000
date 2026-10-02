use mstrchf_contracts::*;
use serde_json::{Value, json};

fn fixture(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/contracts")
        .join(name);
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn parse<T: serde::de::DeserializeOwned + ValidateShape>(value: &Value) -> Result<T, DomainError> {
    parse_document(&serde_json::to_vec(value).unwrap())
}

fn rejected<T: serde::de::DeserializeOwned + ValidateShape + std::fmt::Debug>(
    value: &Value,
    expected: ErrorCode,
) {
    let actual = parse::<T>(value).err().map(|e| e.code);
    assert_eq!(actual, Some(expected));
}

#[test]
fn f03_recipe_components_resolve_ingredient_and_storage_ids() {
    let recipe = fixture("recipe-revision-minimal-unrestricted.json");
    parse::<RecipeRevision>(&recipe).unwrap();

    let mut missing_ingredient = recipe.clone();
    missing_ingredient["components"][0]["ingredient_ids"] =
        json!(["ig_00000000000000000000000000000099"]);
    rejected::<RecipeRevision>(&missing_ingredient, ErrorCode::MissingReference);

    let mut missing_storage = recipe.clone();
    missing_storage["components"][0]["storage_rule_ids"] =
        json!(["ev_00000000000000000000000000000099"]);
    rejected::<RecipeRevision>(&missing_storage, ErrorCode::MissingReference);

    let mut linked_storage = recipe.clone();
    let rule_id = "ev_00000000000000000000000000000099";
    linked_storage["components"][0]["storage_rule_ids"] = json!([rule_id]);
    linked_storage["storage"] = json!([{
        "evidence_id":rule_id,
        "component_id":recipe["components"][0]["component_id"],
        "max_chilled_seconds":3600,
        "cooling_instruction":"Chill.",
        "reheating_instruction":"Reheat.",
        "max_reheats":1,
        "source":recipe["source"]
    }]);
    parse::<RecipeRevision>(&linked_storage).unwrap();
    linked_storage["storage"][0]["component_id"] = json!("co_00000000000000000000000000000099");
    rejected::<RecipeRevision>(&linked_storage, ErrorCode::MissingReference);

    let mut unsupported_extra = fixture("recipe-revision.json");
    parse::<RecipeRevision>(&unsupported_extra).unwrap();
    unsupported_extra["extras"][0]["supported_with"] =
        json!(["co_00000000000000000000000000000099"]);
    rejected::<RecipeRevision>(&unsupported_extra, ErrorCode::MissingReference);
}

fn disabled_retained_context() -> Value {
    let mut context = fixture("planning-context.json");
    context["weeks"][0]["plan"]["selections"] =
        fixture("week-disabled-retained.json")["plan"]["selections"].clone();
    let week_id = context["weeks"][0]["range"]["week_id"].clone();
    context["weeks"][0]["day_enabled"] = json!([false, false, false, false, false, false, false]);
    for requirement in context["requirements"].as_array_mut().unwrap() {
        if requirement["slot"]["week_id"] == week_id {
            requirement["enabled"] = json!(false);
            requirement["eat_by"] = Value::Null;
        }
    }
    context
}

#[test]
fn f04_disabled_selection_keeps_resolved_person_identity() {
    let mut context = disabled_retained_context();
    parse::<PlanningContext>(&context).unwrap();
    context["weeks"][0]["plan"]["selections"][0]["servings"][0]["person"]["data"]["diner_id"] =
        json!("dn_00000000000000000000000000000099");
    rejected::<PlanningContext>(&context, ErrorCode::ForeignReference);
}

fn retained_guest_context() -> (Value, usize, Value) {
    let mut context = disabled_retained_context();
    let week_id = context["weeks"][0]["range"]["week_id"].clone();
    for requirement in context["requirements"].as_array_mut().unwrap() {
        if requirement["slot"]["week_id"] == week_id {
            requirement["attendees"] = json!([]);
            requirement["shared_hard_requirements"] = json!([]);
        }
    }
    let person = json!({
        "kind":"guest",
        "data":{
            "occasion_id":"oc_00000000000000000000000000000099",
            "guest_id":"gu_00000000000000000000000000000099"
        }
    });
    let guest = json!({
        "guest_id":person["data"]["guest_id"],
        "age_band":"adult",
        "restrictions":{
            "kind":"confirmed_none",
            "data":{"confirmed_at":"2026-12-28T10:00:00Z"}
        },
        "portion":{
            "preset":"regular",
            "adjustment":{"numerator":"1","denominator":"1"},
            "meal_overrides":[]
        }
    });
    let selected_slot = context["weeks"][0]["plan"]["selections"][0]["slot"].clone();
    let cells = context["weeks"][0]["cells"].as_array().unwrap();
    let target_index = cells
        .iter()
        .position(|cell| cell["slot"] == selected_slot)
        .unwrap();
    let hosts = context["weeks"][0]["cells"][target_index]["attendee_ids"].clone();
    context["weeks"][0]["cells"][target_index]["occasion"] = json!({
        "occasion_id":person["data"]["occasion_id"],
        "guests":[guest],
        "host_diner_ids":hosts
    });
    context["weeks"][0]["plan"]["selections"][0]["servings"][0]["person"] = person.clone();
    (context, target_index, person)
}

fn move_guest_occasion(context: &mut Value, from: usize, week: usize, to: usize) {
    let occasion = context["weeks"][0]["cells"][from]["occasion"].clone();
    context["weeks"][0]["cells"][from]["occasion"] = Value::Null;
    context["weeks"][week]["cells"][to]["occasion"] = occasion;
    context["weeks"][week]["cells"][to]["occasion"]["host_diner_ids"] =
        context["weeks"][week]["cells"][to]["attendee_ids"].clone();
}

#[test]
fn r2_inactive_guest_resolves_only_in_selected_cell_occasion() {
    let (control, target, _) = retained_guest_context();
    parse::<PlanningContext>(&control).unwrap();
    let mut other_cell = control.clone();
    move_guest_occasion(&mut other_cell, target, 0, (target + 3) % 21);
    rejected::<PlanningContext>(&other_cell, ErrorCode::ForeignReference);
    let mut other_week = control;
    move_guest_occasion(&mut other_week, target, 1, 0);
    rejected::<PlanningContext>(&other_week, ErrorCode::ForeignReference);
}

#[test]
fn r2_inactive_extra_recipient_and_override_keep_scoped_guest_identity() {
    let (mut extra_control, target, person) = retained_guest_context();
    let extra_recipe = extra_control["stamp"]["recipe_refs"][1].clone();
    let mut recipient = extra_control["weeks"][0]["plan"]["selections"][0]["servings"][0].clone();
    recipient["preset_ref"]["recipe"] = extra_recipe.clone();
    extra_control["weeks"][0]["plan"]["selections"][0]["extras"] = json!([{
        "extra_id":"ex_00000000000000000000000000000001",
        "recipe":extra_recipe,
        "recipients":[recipient]
    }]);
    parse::<PlanningContext>(&extra_control).unwrap();
    let mut foreign_extra = extra_control;
    move_guest_occasion(&mut foreign_extra, target, 0, (target + 3) % 21);
    rejected::<PlanningContext>(&foreign_extra, ErrorCode::ForeignReference);

    let (mut override_control, target, _) = retained_guest_context();
    override_control["weeks"][0]["plan"]["selections"][0]["servings"][0]["person"] =
        fixture("week-disabled-retained.json")["plan"]["selections"][0]["servings"][0]["person"]
            .clone();
    override_control["weeks"][0]["plan"]["selections"][0]["meal_overrides"]["portions"] = json!([{
        "person":person,
        "preset":"regular",
        "adjustment":{"numerator":"1","denominator":"1"}
    }]);
    parse::<PlanningContext>(&override_control).unwrap();
    move_guest_occasion(&mut override_control, target, 0, (target + 3) % 21);
    rejected::<PlanningContext>(&override_control, ErrorCode::ForeignReference);
}

#[test]
fn r2_retained_diner_remains_valid_with_zero_effective_attendees() {
    let (mut context, target, _) = retained_guest_context();
    context["weeks"][0]["plan"]["selections"][0]["servings"][0]["person"] =
        fixture("week-disabled-retained.json")["plan"]["selections"][0]["servings"][0]["person"]
            .clone();
    move_guest_occasion(&mut context, target, 0, (target + 3) % 21);
    parse::<PlanningContext>(&context).unwrap();
}

#[test]
fn f05_disabled_requirements_may_omit_attendees_but_supplied_people_resolve() {
    let mut context = fixture("planning-context.json");
    parse::<PlanningContext>(&context).unwrap();
    let week_id = context["weeks"][0]["range"]["week_id"].clone();
    context["weeks"][0]["day_enabled"] = json!([false, false, false, false, false, false, false]);
    for requirement in context["requirements"].as_array_mut().unwrap() {
        if requirement["slot"]["week_id"] == week_id {
            requirement["enabled"] = json!(false);
            requirement["eat_by"] = Value::Null;
            requirement["attendees"] = json!([]);
            requirement["shared_hard_requirements"] = json!([]);
        }
    }
    parse::<PlanningContext>(&context).unwrap();

    let foreign = json!({
        "person": {"kind": "diner", "data": {"diner_id": "dn_00000000000000000000000000000099"}},
        "age_band": "adult",
        "hard_requirements": [],
        "portion": {"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]}
    });
    let first = context["requirements"]
        .as_array_mut()
        .unwrap()
        .iter_mut()
        .find(|r| r["slot"]["week_id"] == week_id)
        .unwrap();
    first["attendees"] = json!([foreign]);
    rejected::<PlanningContext>(&context, ErrorCode::ForeignReference);
}

#[test]
fn f07_candidate_known_ids_are_unique_even_across_components() {
    let mut candidate = fixture("recipe-candidate.json");
    parse::<RecipeCandidate>(&candidate).unwrap();
    candidate["ingredients"] = json!([
        {"ingredient_id":"ig_00000000000000000000000000000001",
         "component_id":"co_00000000000000000000000000000001",
         "name":"One","quantity":null,"source_text":"Synthetic"},
        {"ingredient_id":"ig_00000000000000000000000000000001",
         "component_id":"co_00000000000000000000000000000002",
         "name":"One again","quantity":null,"source_text":"Synthetic"}
    ]);
    rejected::<RecipeCandidate>(&candidate, ErrorCode::InvalidValue);
    candidate["ingredients"][0]["ingredient_id"] = Value::Null;
    candidate["ingredients"][1]["ingredient_id"] = Value::Null;
    parse::<RecipeCandidate>(&candidate).unwrap();
}

#[test]
fn f08_every_delivery_event_belongs_to_batch_order() {
    let batch = fixture("event-batch.json");
    parse::<EventBatchDocumentV1>(&batch).unwrap();
    let mut foreign = batch.clone();
    let mut extra = foreign["event_batch"]["events"][0].clone();
    extra["event_id"] = json!("ce_00000000000000000000000000000002");
    extra["logical_key"]["order_id"] = json!("or_00000000000000000000000000000002");
    let mut material = extra.clone();
    material
        .as_object_mut()
        .unwrap()
        .remove("event_payload_digest");
    material["schema_version"] = json!(1);
    extra["event_payload_digest"] = json!(canonical_digest(&material).unwrap().0);
    foreign["event_batch"]["events"]
        .as_array_mut()
        .unwrap()
        .push(extra);
    rejected::<EventBatchDocumentV1>(&foreign, ErrorCode::IdentityMismatch);

    let mut request = fixture("job-request-calendar-publication.json");
    request["payload"]["data"]["batch"] = foreign["event_batch"].clone();
    let event_ids = json!([
        "ce_00000000000000000000000000000001",
        "ce_00000000000000000000000000000002"
    ]);
    request["scope"]["data"]["event_ids"] = event_ids.clone();
    request["scope"]["data"]["read_scope"]["event_ids"] = event_ids.clone();
    request["payload"]["data"]["read_scope"]["event_ids"] = event_ids;
    let parsed: JobRequestV1 = serde_json::from_value(request.clone()).unwrap();
    request["input_digest"] = json!(input_digest(&RequestMaterial::from(&parsed)).unwrap().0);
    rejected::<JobRequestV1>(&request, ErrorCode::IdentityMismatch);
}

fn old_history_context() -> Value {
    let mut context = fixture("planning-context.json");
    let selection = fixture("week-disabled-retained.json")["plan"]["selections"][0].clone();
    let mut selected = json!({
        "week_id":"wk_00000000000000000000000000000099",
        "slot":{
            "week_id":"wk_00000000000000000000000000000099",
            "date":"2026-12-21",
            "meal_type":"dinner"
        },
        "recipe": selection["recipe"],
        "servings": selection["servings"],
        "extras":[],
        "repeat_authorised":false,
        "selected_at":"2026-12-22T12:00:00Z"
    });
    selected["servings"][0]["person"]["data"]["diner_id"] =
        json!("dn_00000000000000000000000000000099");
    selected["extras"] = json!([{
        "extra_id":"ex_00000000000000000000000000000099",
        "recipe": selected["recipe"],
        "recipients":selected["servings"]
    }]);
    context["history"]["selected"] = json!([selected.clone()]);
    context["history"]["ordered"] = json!([{
        "selection":selected,
        "order_id":"or_00000000000000000000000000000099",
        "ordered_at":"2026-12-23T12:00:00Z"
    }]);
    context["history"]["feedback"] = json!([{
        "slot":context["history"]["selected"][0]["slot"],
        "recipe":context["history"]["selected"][0]["recipe"],
        "rating":"liked","reasons":[],"hide":false,
        "portion_change":null,"portion_change_accepted":false,
        "recorded_at":"2026-12-24T12:00:00Z"
    }]);
    context["history"]["hidden_recipe_ids"] = json!(["rc_00000000000000000000000000000099"]);
    context
}

#[test]
fn current_extra_recipient_preset_resolves_and_belongs_to_extra_recipe() {
    let mut context = disabled_retained_context();
    let extra_recipe = context["stamp"]["recipe_refs"][1].clone();
    let selection = &mut context["weeks"][0]["plan"]["selections"][0];
    let mut recipient = selection["servings"][0].clone();
    recipient["preset_ref"]["recipe"] = extra_recipe.clone();
    selection["extras"] = json!([{
        "extra_id":"ex_00000000000000000000000000000001",
        "recipe":extra_recipe,
        "recipients":[recipient]
    }]);
    parse::<PlanningContext>(&context).unwrap();

    let mut wrong_recipe = context.clone();
    wrong_recipe["weeks"][0]["plan"]["selections"][0]["extras"][0]["recipients"][0]["preset_ref"]
        ["recipe"] = wrong_recipe["weeks"][0]["plan"]["selections"][0]["recipe"].clone();
    rejected::<PlanningContext>(&wrong_recipe, ErrorCode::ForeignReference);

    let mut missing_recipe = context.clone();
    missing_recipe["weeks"][0]["plan"]["selections"][0]["extras"][0]["recipients"][0]["preset_ref"]
        ["recipe"]["recipe_id"] = json!("rc_00000000000000000000000000000098");
    rejected::<PlanningContext>(&missing_recipe, ErrorCode::MissingReference);

    let mut missing_extra_preset = context.clone();
    missing_extra_preset["weeks"][0]["plan"]["selections"][0]["extras"][0]["recipients"][0]["preset_ref"]
        ["preset"] = json!("smaller");
    rejected::<PlanningContext>(&missing_extra_preset, ErrorCode::MissingReference);

    let mut missing_base_preset = context.clone();
    missing_base_preset["weeks"][0]["plan"]["selections"][0]["servings"][0]["preset_ref"]["preset"] =
        json!("smaller");
    rejected::<PlanningContext>(&missing_base_preset, ErrorCode::MissingReference);
}

#[test]
fn consulted_history_presets_resolve_without_current_people_or_weeks() {
    let control = old_history_context();
    parse::<PlanningContext>(&control).unwrap();
    for path in [
        "/history/selected/0/servings/0/preset_ref/preset",
        "/history/selected/0/extras/0/recipients/0/preset_ref/preset",
        "/history/ordered/0/selection/servings/0/preset_ref/preset",
        "/history/ordered/0/selection/extras/0/recipients/0/preset_ref/preset",
    ] {
        let mut changed = control.clone();
        *changed.pointer_mut(path).unwrap() = json!("smaller");
        rejected::<PlanningContext>(&changed, ErrorCode::MissingReference);
    }
    let mut wrong_base = control.clone();
    wrong_base["history"]["selected"][0]["servings"][0]["preset_ref"]["recipe"] =
        wrong_base["stamp"]["recipe_refs"][1].clone();
    rejected::<PlanningContext>(&wrong_base, ErrorCode::ForeignReference);
    let mut wrong_extra = control.clone();
    wrong_extra["history"]["selected"][0]["extras"][0]["recipients"][0]["preset_ref"]["recipe"] =
        wrong_extra["stamp"]["recipe_refs"][1].clone();
    rejected::<PlanningContext>(&wrong_extra, ErrorCode::ForeignReference);
}

#[test]
fn consulted_history_requires_every_exact_recipe_ref_but_allows_old_people_and_weeks() {
    let control = old_history_context();
    parse::<PlanningContext>(&control).unwrap();
    let paths = [
        "/history/selected/0/recipe",
        "/history/selected/0/servings/0/preset_ref/recipe",
        "/history/selected/0/extras/0/recipe",
        "/history/selected/0/extras/0/recipients/0/preset_ref/recipe",
        "/history/ordered/0/selection/recipe",
        "/history/ordered/0/selection/servings/0/preset_ref/recipe",
        "/history/ordered/0/selection/extras/0/recipe",
        "/history/ordered/0/selection/extras/0/recipients/0/preset_ref/recipe",
        "/history/feedback/0/recipe",
    ];
    for path in paths {
        let mut changed = control.clone();
        changed.pointer_mut(path).unwrap()["recipe_id"] =
            json!("rc_00000000000000000000000000000098");
        rejected::<PlanningContext>(&changed, ErrorCode::MissingReference);
    }
    let mut wrong_digest = control.clone();
    wrong_digest["history"]["feedback"][0]["recipe"]["digest"] = json!("a".repeat(64));
    rejected::<PlanningContext>(&wrong_digest, ErrorCode::MissingReference);
}
