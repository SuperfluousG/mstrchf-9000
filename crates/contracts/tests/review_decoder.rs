use mstrchf_contracts::{
    BoundedVec, ErrorCode, Fact, RecipeRevision, SourceRecord, Text, parse_document,
};
use serde_json::{Value, json};
use std::path::Path;

fn authors(count: usize) -> String {
    (0..count)
        .map(|index| format!("\"author{index}\""))
        .collect::<Vec<_>>()
        .join(",")
}

#[test]
fn source_authors_stop_before_over_limit_element_is_decoded() {
    for raw in [
        format!("{{\"authors\":[{},BROKEN]}}", authors(64)),
        format!("{{\"authors\":[{},BROKEN]}}", authors(65)),
    ] {
        let error = parse_document::<SourceRecord>(raw.as_bytes()).unwrap_err();
        assert_eq!(error.code, ErrorCode::LimitExceeded, "{error:?}");
    }
}

#[test]
fn nested_authors_limit_precedes_later_malformed_root_content() {
    let raw = format!(
        "{{\"source\":{{\"authors\":[{},BROKEN]}},\"title\":BROKEN}}",
        authors(64)
    );
    let error = parse_document::<RecipeRevision>(raw.as_bytes()).unwrap_err();
    assert_eq!(error.code, ErrorCode::LimitExceeded, "{error:?}");
}

#[test]
fn tagged_union_data_before_kind_enforces_its_inner_collection_cap() {
    let eight = "\"a\",\"b\",\"c\",\"d\",\"e\",\"f\",\"g\",\"h\"";
    for raw in [
        format!("{{\"data\":{{\"values\":[{eight},BROKEN]}},\"kind\":\"conflicting\"}}"),
        format!("{{\"kind\":\"conflicting\",\"data\":{{\"values\":[{eight},BROKEN]}}}}"),
    ] {
        let error = parse_document::<Fact<Text>>(raw.as_bytes()).unwrap_err();
        assert_eq!(error.code, ErrorCode::LimitExceeded, "{error:?}");
    }
}

#[test]
fn exact_bound_and_valid_field_orders_remain_accepted() {
    let fixture_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/contracts");
    let mut recipe: Value =
        serde_json::from_slice(&std::fs::read(fixture_root.join("recipe-revision.json")).unwrap())
            .unwrap();
    recipe["source"]["authors"] = json!((0..64).map(|i| format!("author{i}")).collect::<Vec<_>>());
    parse_document::<RecipeRevision>(&serde_json::to_vec(&recipe).unwrap()).unwrap();
    let values = "\"a\",\"b\",\"c\",\"d\",\"e\",\"f\",\"g\",\"h\"";
    for raw in [
        format!("{{\"data\":{{\"values\":[{values}]}},\"kind\":\"conflicting\"}}"),
        format!("{{\"kind\":\"conflicting\",\"data\":{{\"values\":[{values}]}}}}"),
    ] {
        parse_document::<Fact<Text>>(raw.as_bytes()).unwrap();
    }
}

#[test]
fn bounded_vec_direct_decode_refuses_before_the_next_element() {
    let error = serde_json::from_str::<BoundedVec<Text, 0, 2>>("[\"a\",\"b\",BROKEN]").unwrap_err();
    assert!(error.to_string().contains("CODE:limit_exceeded"), "{error}");
    let value = serde_json::from_str::<BoundedVec<Text, 0, 2>>("[\"a\",\"b\"]").unwrap();
    assert_eq!(value.len(), 2);
}
