use mstrchf_contracts::*;
use serde::{Deserialize, Serialize};
#[derive(Debug, Serialize, Deserialize, schemars::JsonSchema)]
struct Probe {
    values: Vec<u8>,
}
#[test]
fn raw_duplicate_key() {
    let e = parse_document::<Probe>(br#"{"values":[],"values":[]}"#).unwrap_err();
    assert_eq!(e.code, ErrorCode::DuplicateKey);
}
#[test]
fn raw_oversize_and_depth() {
    let bytes = vec![b' '; MAX_DOCUMENT_BYTES + 1];
    assert_eq!(
        parse_document::<Probe>(&bytes).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
}
#[test]
fn empty_input() {
    assert_eq!(
        parse_document::<Probe>(b"").unwrap_err().code,
        ErrorCode::MalformedJson
    );
}

#[test]
fn manifest_fixtures_run_and_schema_runtime_agree() {
    let base =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/contracts");
    let raw = std::fs::read(base.join("manifest.json")).expect("required fixture manifest");
    let manifest: serde_json::Value = serde_json::from_slice(&raw).unwrap();
    let cases = manifest["cases"].as_array().expect("cases array");
    assert!(!cases.is_empty(), "empty fixture corpus");
    let generated = mstrchf_contracts::schema::render_contracts().unwrap();
    let mut seen = std::collections::BTreeSet::new();
    let mut counts = std::collections::BTreeMap::new();
    let mut failures = Vec::new();
    for c in cases {
        let id = c["id"].as_str().unwrap();
        let sub = c["subcase"].as_str().unwrap_or("default");
        assert!(seen.insert((id, sub)), "duplicate fixture {id}/{sub}");
        let stage = c["stage"].as_str().unwrap();
        assert!(
            [
                "wire",
                "semantic",
                "raw",
                "binding",
                "privacy",
                "context_compare",
                "read_binding",
                "resume",
                "operation_approval",
                "operation_shape",
                "arithmetic",
                "conversion",
                "essentials",
                "command",
                "canonical",
                "primitive"
            ]
            .contains(&stage),
            "unknown stage {stage}"
        );
        let file = c["file"].as_str().unwrap();
        assert!(
            !file.contains('/') && !file.contains('\\'),
            "fixture path must be flat"
        );
        let bytes = std::fs::read(base.join(file)).unwrap_or_else(|e| panic!("{file}: {e}"));
        if !["wire", "semantic", "raw"].contains(&stage) {
            assert!(!bytes.is_empty());
            continue;
        }
        let root = c["root"].as_str().unwrap();
        let expected = c["expected"].as_str().unwrap();
        let got = mstrchf_contracts::schema::parse_root(root, &bytes);
        let actual = got
            .as_ref()
            .err()
            .map(|e| {
                serde_json::to_value(e.code)
                    .unwrap()
                    .as_str()
                    .unwrap()
                    .to_owned()
            })
            .unwrap_or("ok".into());
        if actual != expected {
            failures.push(format!(
                "{id}/{sub}: expected {expected}; got {actual} {got:?}"
            ));
        }
        if stage == "wire" || expected == "ok" {
            let s: serde_json::Value = serde_json::from_slice(
                generated
                    .get(&format!("contracts/v1/{root}.schema.json"))
                    .unwrap(),
            )
            .unwrap();
            let validator = jsonschema::draft202012::options().build(&s).unwrap();
            let schema_ok = serde_json::from_slice::<serde_json::Value>(&bytes)
                .is_ok_and(|v| validator.is_valid(&v));
            if schema_ok != (expected == "ok") {
                failures.push(format!(
                    "{id}/{sub}: schema acceptance {schema_ok}; expected {expected}"
                ));
            }
        }
        *counts
            .entry(id.split('_').next().unwrap())
            .or_insert(0usize) += 1;
    }
    println!("fixture subcases={} categories={counts:?}", seen.len());
    let required: Vec<String> =
        serde_json::from_slice(&std::fs::read(base.join("required-case-ids.json")).unwrap())
            .unwrap();
    assert_eq!(required.len(), 90);
    for id in required {
        assert!(
            seen.iter().any(|(present, _)| *present == id),
            "missing required matrix row {id}"
        );
    }
    assert!(failures.is_empty(), "{}", failures.join("\n"));
}
