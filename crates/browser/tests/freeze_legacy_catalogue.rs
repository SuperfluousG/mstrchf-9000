//! Explicit one-time freeze of actual browser/core engineering declarations.
//! Canonical recipe bytes and references come from the existing public API.
use mstrchf_browser::run_json;
use mstrchf_contracts::*;
use serde_json::{Value, json};
use std::{fs::OpenOptions, io::Write};

fn call(request: Value) -> Value {
    let out: Value =
        serde_json::from_slice(&run_json(&serde_json::to_vec(&request).unwrap())).unwrap();
    assert_eq!(out["ok"], true, "{out}");
    out["value"].clone()
}

#[test]
#[ignore = "one-time exclusive-create export; never replaces frozen engineering bytes"]
fn freeze_current_engineering_v1_from_actual_browser_api_and_core() {
    let now = "2026-10-12T12:00:00Z";
    let state = call(
        json!({"op":"initialize","household_id":"hh_00000000000000000000000000000004","diner_id":"dn_00000000000000000000000000000004","zone":"Europe/London","today":"2026-10-12","now":now,"demo":true}),
    );
    let board = call(json!({"op":"board","state":state,"now":now}));
    let library = board["details"]["library"].as_array().unwrap();
    assert_eq!(library.len(), 5);
    let folder = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("assets/catalogue/engineering-v1");
    std::fs::create_dir_all(&folder).unwrap();
    assert!(
        !folder.join("manifest.json").exists(),
        "the frozen manifest already exists"
    );
    let mut entries = Vec::new();
    for entry in library {
        assert_eq!(entry["admission"], "engineering");
        let recipe: RecipeRevision =
            parse_document(&serde_json::to_vec(&entry["recipe"]).unwrap()).unwrap();
        let reference = RecipeRef {
            recipe_id: recipe.recipe_id.clone(),
            revision: recipe.revision,
            digest: recipe_digest(&recipe).unwrap(),
        };
        assert_eq!(json!(reference), entry["reference"]);
        let body = canonical_bytes(&recipe).unwrap();
        assert_eq!(bytes_digest(&body).0, reference.digest.0);
        let packet_digest = canonical_digest(
            &json!({"schema_version":1,"status":"legacy_engineering_v1","recipe":recipe}),
        )
        .unwrap();
        entries.push(json!({"recipe":reference,"packet_digest":packet_digest}));
        let filename = format!(
            "{}-r{}.canonical.json",
            recipe.recipe_id.0, recipe.revision.0
        );
        OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(folder.join(filename))
            .unwrap()
            .write_all(&body)
            .unwrap();
    }
    let material = json!({"schema_version":1,"revision":"1","entries":entries});
    let manifest = json!({"schema_version":1,"status":"legacy_engineering_v1","catalogue":{"revision":"1","manifest_digest":canonical_digest(&material).unwrap(),"entries":entries}});
    OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(folder.join("manifest.json"))
        .unwrap()
        .write_all(&canonical_bytes(&manifest).unwrap())
        .unwrap();
    println!("{}", serde_json::to_string_pretty(&manifest).unwrap());
}
