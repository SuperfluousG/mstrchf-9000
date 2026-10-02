use mstrchf_contracts::*;
use serde_json::{Value, json};
fn fixture(name: &str) -> Value {
    serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/contracts")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
#[test]
fn diner_names_use_the_eighty_scalar_limit() {
    let mut v = fixture("household-profile.json");
    v["diners"][0]["display_name"] = json!("a".repeat(81));
    assert!(parse_document::<HouseholdProfile>(&serde_json::to_vec(&v).unwrap()).is_err());
}
#[test]
fn unresolved_candidate_ingredients_have_no_fabricated_identity() {
    let mut r = fixture("recipe-candidate.json");
    r["ingredients"] = json!([
 {"ingredient_id":null,"component_id":null,"name":"First unknown ingredient","quantity":null,"source_text":"Synthetic first"},
 {"ingredient_id":null,"component_id":null,"name":"Second unknown ingredient","quantity":null,"source_text":"Synthetic second"}]);
    parse_document::<RecipeCandidate>(&serde_json::to_vec(&r).unwrap()).unwrap();
}
#[test]
fn ordered_history_identity_is_order_and_slot() {
    let w = fixture("ruling004-current--week-disabled-retained.json");
    let s = &w["plan"]["selections"][0];
    let selected = json!({"week_id":s["slot"]["week_id"],"slot":s["slot"],"recipe":s["recipe"],"servings":s["servings"],"extras":s["extras"],"repeat_authorised":s["repeat_authorised"],"selected_at":"2026-12-28T10:00:00Z"});
    let first = json!({"selection":selected,"order_id":"or_00000000000000000000000000000001","ordered_at":"2026-12-28T12:00:00Z"});
    let mut second = first.clone();
    second["selection"]["selected_at"] = json!("2026-12-28T11:00:00Z");
    let base = json!({"revision":"1","selected":[],"ordered":[first],"feedback":[],"hidden_recipe_ids":[]});
    parse_document::<PlanningHistory>(&serde_json::to_vec(&base).unwrap()).unwrap();
    let mut bad = base;
    bad["ordered"].as_array_mut().unwrap().push(second);
    assert_eq!(
        parse_document::<PlanningHistory>(&serde_json::to_vec(&bad).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::InvalidValue
    );
}
#[test]
fn snapshot_week_identity_does_not_include_zone() {
    let mut v = fixture("ruling005-current--snapshot.json");
    v["weeks"].as_array_mut().unwrap().truncate(1);
    let mut w = v["weeks"][0].clone();
    w["range"]["zone"] = json!("Etc/UTC");
    v["weeks"].as_array_mut().unwrap().push(w);
    assert_eq!(
        parse_document::<Snapshot>(&serde_json::to_vec(&v).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::InvalidValue
    );
}
