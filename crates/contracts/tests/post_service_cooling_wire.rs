use mstrchf_contracts::*;
use serde_json::{Value, json};

fn cooling_recipe() -> Value {
    let mut recipe: Value = serde_json::from_slice::<Value>(include_bytes!(
        "../../../tests/fixtures/contracts/ruling004-v4/recipe.input.json"
    ))
    .unwrap()["value"]
        .clone();
    recipe["components"][0]["production"] = json!({
        "kind": "cooked_with_cooling",
        "data": {"completion_prep_id": 1, "cooling_prep_ids": [3,4], "max_cooling_seconds": 7200}
    });
    let component = recipe["components"][0]["component_id"].clone();
    for (id, dependency, active, duration, offset) in [(3, 1, 5, 5, 0), (4, 3, 0, 115, 300)] {
        recipe["steps"].as_array_mut().unwrap().push(json!({
            "step_id":id,"text":"Synthetic conditional cooling test only.",
            "ingredient_ids":[],"component_ids":[component],"active_minutes":active,
            "elapsed_minutes":duration,"depends_on":[dependency]
        }));
        recipe["prep"].as_array_mut().unwrap().push(json!({
            "prep_id":id,"title":"Synthetic after-service handling",
            "step_ids":[id],"depends_on":[dependency],"active_minutes":active,
            "elapsed_minutes":duration,"earliest_offset_seconds":offset,
            "latest_offset_seconds":offset,"ingredient_ids":[],"can_attach_to_prior_session":false
        }));
    }
    recipe["active_minutes"] = json!(25);
    recipe["elapsed_minutes"] = json!(140);
    recipe
}

fn parse(value: &Value) -> Result<RecipeRevision, DomainError> {
    parse_document(&serde_json::to_vec(value).unwrap())
}

#[test]
fn explicit_cooling_wire_roundtrips_without_changing_old_cooked_encoding() {
    let recipe = cooling_recipe();
    assert_eq!(
        serde_json::to_value(parse(&recipe).unwrap()).unwrap(),
        recipe
    );
    let schema = schema::schema_for::<RecipeRevision>();
    assert!(
        jsonschema::validator_for(&schema)
            .unwrap()
            .is_valid(&recipe)
    );
    assert_eq!(
        serde_json::to_value(ComponentProductionSpec::Cooked {
            completion_prep_id: 1
        })
        .unwrap(),
        json!({"kind":"cooked","data":{"completion_prep_id":1}})
    );
}

fn earlier_recipe(seconds: i64) -> Value {
    let mut value=cooling_recipe();
    for index in [0,2,3] {
        for field in ["earliest_offset_seconds","latest_offset_seconds"] {
            value["prep"][index][field]=json!(value["prep"][index][field].as_i64().unwrap()-seconds);
        }
    }
    value
}

#[test]
fn earlier_fixed_endpoint_bounds_cooling_from_cooking_not_service() {
    let earlier=earlier_recipe(300);
    assert!(parse(&earlier).is_ok());
    assert_eq!(serde_json::to_value(parse(&earlier).unwrap()).unwrap(),earlier);
    // Last phase: start T+0 plus115min = T+6900 = E+7200.
    assert_eq!(earlier["prep"][3]["latest_offset_seconds"],0);
    let mut late=earlier.clone();late["prep"][3]["latest_offset_seconds"]=json!(1);
    assert!(parse(&late).is_err(),"one extra second cannot borrow from service time");
    let mut before=earlier.clone();before["prep"][2]["earliest_offset_seconds"]=json!(-301);
    assert!(parse(&before).is_err(),"cooling cannot begin before thermal completion");
    let mut flexible=earlier;flexible["prep"][0]["earliest_offset_seconds"]=json!(-901);
    assert!(parse(&flexible).is_err(),"the endpoint must remain fixed");
    assert!(parse(&earlier_recipe(7200)).is_ok());
    assert!(parse(&earlier_recipe(7201)).is_err(),"no unbounded pre-service holding");
    assert!(parse(&earlier_recipe(-1)).is_err(),"thermal completion cannot follow meal time");
}

#[test]
fn cooling_variant_requires_positive_bounded_canonical_distinct_phases() {
    for mutation in 0..8 {
        let mut value = cooling_recipe();
        let data = &mut value["components"][0]["production"]["data"];
        match mutation {
            0 => data["max_cooling_seconds"] = json!(0),
            1 => data["max_cooling_seconds"] = json!(7201),
            2 => data["cooling_prep_ids"] = json!([]),
            3 => data["cooling_prep_ids"] = json!([3, 3]),
            4 => data["cooling_prep_ids"] = json!([4, 3]),
            5 => data["cooling_prep_ids"] = json!([1, 3]),
            6 => data["cooling_prep_ids"] = json!([129]),
            _ => {
                data.as_object_mut().unwrap().remove("max_cooling_seconds");
            }
        }
        assert!(parse(&value).is_err(), "mutation {mutation}");
    }
}

#[test]
fn cooling_cannot_relabel_fresh_preparation_or_an_unbound_completion() {
    for mutation in 0..11 {
        let mut value = cooling_recipe();
        match mutation {
            0 => value["prep"][0]["earliest_offset_seconds"] = json!(-601),
            1 => value["prep"][0]["can_attach_to_prior_session"] = json!(true),
            2 => value["prep"][2]["earliest_offset_seconds"] = json!(-1),
            3 => value["prep"][3]["latest_offset_seconds"] = json!(301),
            4 => value["prep"][2]["ingredient_ids"] = value["prep"][0]["ingredient_ids"].clone(),
            5 => value["steps"][2]["ingredient_ids"] = value["steps"][0]["ingredient_ids"].clone(),
            6 => value["steps"][2]["component_ids"] = json!([]),
            7 => value["prep"][2]["depends_on"] = json!([]),
            8 => value["prep"][2]["depends_on"] = json!([2]),
            9 => value["components"][0]["storage_rule_ids"] = json!([]),
            _ => value["components"][0]["production"]["data"]["cooling_prep_ids"] = json!([3, 5]),
        }
        assert!(parse(&value).is_err(), "mutation {mutation}");
    }
}

#[test]
fn cooling_steps_and_dependencies_cannot_be_shared_with_ordinary_phases() {
    for mutation in 0..3 {
        let mut value = cooling_recipe();
        match mutation {
            0 => value["prep"][1]["depends_on"] = json!([3]),
            1 => value["prep"][1]["step_ids"] = json!([3]),
            _ => value["prep"][3]["step_ids"] = json!([3]),
        }
        assert!(parse(&value).is_err(), "mutation {mutation}");
    }
}
