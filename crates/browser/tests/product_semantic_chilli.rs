//! Actual compiled food and native owner, with wholly synthetic household,
//! source observations and model decisions. No retailer/handling/effect proof.
use mstrchf_browser::{maker_packets::*, run_json};
use mstrchf_contracts::*;
use serde_json::{Value, json};
use std::sync::OnceLock;

const CREATED: &str = "2026-10-14T15:00:00Z";
const OBSERVED: &str = "2026-10-14T15:00:01Z";
const NOW: &str = "2026-10-14T15:05:00Z";
const DEADLINE: &str = "2026-10-14T16:00:00Z";
fn call(input: Value) -> Value {
    serde_json::from_slice(&run_json(&serde_json::to_vec(&input).unwrap())).unwrap()
}
fn good(input: Value) -> Value {
    let out = call(input);
    assert_eq!(out["ok"], true, "{out}");
    out["value"].clone()
}
fn apply(state: &Value, command: Value, id: u32) -> Value {
    good(
        json!({"op":"dispatch","state":state,"now":CREATED,"command":{
        "schema_version":1,"command_id":format!("cm_{id:032x}"),
        "expected_snapshot_revision":state["snapshot"]["snapshot_revision"],"command":command}}),
    )["state"]
        .clone()
}
fn save(state: &Value, week: &Value, id: u32) -> Value {
    apply(
        state,
        json!({"kind":"save_week","data":{"expected_week_revision":week["revision"],"week":{
        "week_id":week["range"]["week_id"],"defaults_snapshot":week["defaults_snapshot"],
        "day_enabled":week["day_enabled"],"meal_type_enabled":week["meal_type_enabled"],
        "cells":week["cells"],"plan":week["plan"]}}}),
        id,
    )
}
fn admit(state: &Value, packet: ReviewedRecipePacketRef, id: u32) -> Value {
    apply(
        state,
        json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,
        "expected_catalogue_revision":(state["reviewed_packets"].as_array().map_or(0,Vec::len)+1).to_string()}}),
        id,
    )
}
fn select(state: &Value, index: usize, recipe: RecipeRef, options: Value, id: u32) -> Value {
    let week = &state["snapshot"]["weeks"][0];
    let board = good(
        json!({"op":"board","state":state,"week_id":week["range"]["week_id"],
        "slot":week["cells"][index]["slot"],"options":options,"now":CREATED}),
    );
    let choice = board["choices"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["selection"]["recipe"] == json!(recipe))
        .unwrap_or_else(|| panic!("Missing synthetic choice {recipe:?}: {}", board["details"]));
    let mut week = week.clone();
    week["plan"] = choice["plan"].clone();
    save(state, &week, id)
}
fn fixture() -> &'static Value {
    static STATE: OnceLock<Value> = OnceLock::new();
    STATE.get_or_init(|| {
        let mut s = good(json!({"op":"initialize","household_id":"hh_800000000000000000000000000000b3",
            "diner_id":"dn_800000000000000000000000000000b3","zone":"Europe/London",
            "today":"2026-10-14","now":CREATED,"demo":true}));
        let mut p = s["snapshot"]["profile"].clone();
        p["diners"][0]["age_band"] = json!("adult");
        p["diners"][0]["restrictions"] = json!({"kind":"confirmed_none","data":{"confirmed_at":CREATED}});
        p["diners"][0]["portions"] = json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});
        p["preferences"]["equipment"] = json!(["fridge","hob","oven"]);
        s = apply(&s, json!({"kind":"update_profile","data":{"expected_profile_revision":p["revision"],"profile":p}}), 0xb300);
        let mut w = s["snapshot"]["weeks"][0].clone();
        w["plan"]["selections"] = json!([]); w["plan"]["batch_links"] = json!([]);
        w["day_enabled"] = json!([true,true,true,true,true,true,true]);
        w["meal_type_enabled"] = json!({"breakfast":true,"lunch":true,"dinner":true});
        for c in w["cells"].as_array_mut().unwrap() { c["choice"] = json!("at_home"); }
        s = save(&s, &w, 0xb301);
        let mut privacy = s["snapshot"]["privacy"].clone(); privacy["use_ai_features"] = json!(true);
        s = apply(&s, json!({"kind":"set_privacy","data":{"expected_policy_revision":privacy["revision"],"policy":privacy}}), 0xb302);
        for (i,p) in [chilli_batch_packet_ref().unwrap(),chilli_wraps_packet_ref().unwrap(),chilli_potato_packet_ref().unwrap()].into_iter().enumerate() {
            s = admit(&s,p,0xb303+i as u32);
        }
        s = select(&s,8,chilli_batch_packet_ref().unwrap().recipe,Value::Null,0xb306);
        let source = s["snapshot"]["weeks"][0]["cells"][8]["slot"].clone();
        s = select(&s,10,chilli_wraps_packet_ref().unwrap().recipe,json!({"leftover_source":source}),0xb307);
        select(&s,13,chilli_potato_packet_ref().unwrap().recipe,json!({"leftover_source":source}),0xb308)
    })
}
fn context(s: &Value) -> Value {
    good(
        json!({"op":"product_match_context","state":s,"week_id":s["snapshot"]["weeks"][0]["range"]["week_id"],"now":NOW}),
    )
}
fn line(s: &Value, suffix: &str) -> Value {
    context(s)["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| {
            l["demand"]["ingredient_id"] == format!("ig_800000000000000000000000000000{suffix}")
        })
        .unwrap()["demand"]["line_id"]
        .clone()
}
fn unknown() -> Value {
    json!({"kind":"unknown","data":{"reason":"not_exposed"}})
}
fn known(value: Value) -> Value {
    json!({"kind":"known","data":{"value":value}})
}
fn quantity(n: &str, unit: &str, form: &str) -> Value {
    json!({"amount":{"numerator":n,"denominator":"1"},"unit":unit,"form":form})
}
fn product() -> Value {
    json!({"retailer":"tesco","product_id":"123456789","variant_id":null})
}
fn source(
    s: &Value,
    net: Value,
    drained: Value,
    ingredients: Value,
    storage: Value,
) -> (Value, Value, Value) {
    let p = good(json!({"op":"product_source_prepare","state":s,
        "week_id":s["snapshot"]["weeks"][0]["range"]["week_id"],
        "reader":{"provider":"codex_local","native_task_ref":"synthetic-chilli-test-not-a-live-reader"},
        "key":{"retailer":"tesco","account_ref":"synthetic-account","basket_id":"bk_800000000000000000000000000000b3"},
        "delivery":{"retailer":"tesco","account_ref":"synthetic-account","context_ref":"synthetic-context","delivery_date":null,"slot_ref":null},
        "targets":[{"product":product(),"url":"https://www.tesco.com/shop/en-GB/products/123456789"}],
        "job_id":"jb_800000000000000000000000000000b3","read_id":"rd_800000000000000000000000000000b3",
        "now":CREATED,"deadline":DEADLINE}));
    let o = json!({"schema_version":1,"observation_id":"ob_800000000000000000000000000000b3",
        "read_id":p["request"]["scope"]["data"]["read_scope"]["read_id"],"job_id":p["request"]["job_id"],
        "input_digest":p["request"]["input_digest"],"reader":p["expected_reader"],
        "resource":p["request"]["scope"]["data"]["read_scope"]["resource"],"observed_at":OBSERVED,
        "body":{"kind":"products","data":{"delivery_context":p["delivery"],"items":[{
            "product":product(),"url":"https://www.tesco.com/shop/en-GB/products/123456789",
            "title":"Synthetic public-label fixture; no retailer read","pack":{"count":known(json!(1)),"net":net,"drained":drained},
            "price":unknown(),"offers":[],"availability":"unknown","ingredients":ingredients,"allergens":unknown(),
            "storage":storage,"delivery_context":p["delivery"],"observed_at":OBSERVED}]}}});
    let a = good(
        json!({"op":"product_source_accept","state":s,"prepared":p,"observation":o,
        "evidence_id":"ev_800000000000000000000000000000b3","now":NOW}),
    )["accepted"]
        .clone();
    (p, o, a)
}
fn prepared(
    s: &Value,
    suffix: &str,
    net: Value,
    drained: Value,
    ingredients: Value,
    storage: Value,
) -> Value {
    let (p, o, a) = source(s, net, drained, ingredients, storage);
    good(json!({"op":"product_semantic_prepare_v3","state":s,
        "week_id":s["snapshot"]["weeks"][0]["range"]["week_id"],"source_prepared":p,"observation":o,
        "accepted":a,"product":product(),"line_id":line(s,suffix),"request_id":"sr_800000000000000000000000000000b3",
        "now":NOW,"deadline":DEADLINE}))
}
fn proposed(q: &Value, state: &str) -> Value {
    let mut citations = vec![];
    if q["request"]["product"]["ingredients"]["kind"] == "known" {
        citations.push(json!({"field":"ingredients","quote":q["request"]["product"]["ingredients"]["data"]["value"]}));
    }
    let identity = json!({"state":state,"citations":citations,"reasoning":"Synthetic exact ingredient interpretation; no physical product or source-truth proof."});
    if q["request"]["product"]["storage"]["kind"] == "known" {
        citations.push(
            json!({"field":"storage","quote":q["request"]["product"]["storage"]["data"]["value"]}),
        );
    }
    json!({"format":"mstrchf-product-semantic-return","version":1,"request_id":q["request"]["request_id"],
        "input_digest":q["request"]["input_digest"],"identity":identity,
        "cooking":q["request"]["methods"].as_array().unwrap().iter().map(|m|json!({"method_handle":m["method_handle"],
            "decision":{"state":state,"citations":citations,"reasoning":"Synthetic review covers the full pinned method, original batch and input conditions; handling remains unobserved."}})).collect::<Vec<_>>()})
}
fn recorded(s: &Value, q: &Value, r: &Value) -> Value {
    good(
        json!({"op":"product_semantic_evaluate","state":s,"prepared":q,"returned":r,
        "evidence_id":"ev_800000000000000000000000000000b4","now":NOW}),
    )
}
fn qualify(s: &Value, q: &Value, r: &Value, now: &str) -> Value {
    good(json!({"op":"product_meaning_qualify","state":s,"prepared":q,"recorded":r,"now":now}))
}
fn resign(q: &mut Value) {
    let typed = serde_json::from_value(q.clone()).unwrap();
    q["prepared_digest"] =
        json!(mstrchf_browser::product_semantic_review::prepared_digest(&typed).unwrap());
}
fn audit(q: &Value) -> Value {
    call(json!({"op":"product_semantic_audit","prepared":q,"returned":null,"recorded":null}))
}
fn forbid_private(v: &Value) {
    match v {
        Value::Object(map) => {
            for (key, value) in map {
                assert!(
                    ![
                        "household_id",
                        "diner_id",
                        "person",
                        "slot",
                        "source_slot",
                        "target_slot",
                        "batch_id",
                        "week_id",
                        "line_id",
                        "occurrence",
                        "occurrences",
                        "uses",
                        "origins",
                        "allocations",
                        "account_ref",
                        "delivery_context"
                    ]
                    .contains(&key.as_str()),
                    "Private {key}"
                );
                forbid_private(value);
            }
        }
        Value::Array(values) => {
            for value in values {
                forbid_private(value)
            }
        }
        _ => {}
    }
}

#[test]
fn v3_chilli_entire_producer_and_both_remixes_keep_exact_demand_and_private_batch_trace() {
    let s = fixture();
    let c = context(s);
    assert_eq!(c["lines"].as_array().unwrap().len(), 14);
    let refs = [
        chilli_batch_packet_ref().unwrap(),
        chilli_wraps_packet_ref().unwrap(),
        chilli_potato_packet_ref().unwrap(),
    ];
    for rf in &refs {
        for ingredient in known_packet(rf).unwrap().recipe().ingredients.iter() {
            let demand = &c["lines"]
                .as_array()
                .unwrap()
                .iter()
                .find(|l| l["demand"]["ingredient_id"] == json!(ingredient.ingredient_id))
                .unwrap()["demand"];
            assert_eq!(
                demand["quantity"],
                json!(ingredient.quantity),
                "No inherited purchase duplication"
            );
        }
    }
    for (suffix, net, drained, required, count) in [
        (
            "22",
            quantity("400", "g", "prepared"),
            known(quantity("240", "g", "drained")),
            quantity("480", "g", "drained"),
            3,
        ),
        (
            "2e",
            quantity("8", "each", "prepared"),
            unknown(),
            quantity("2", "each", "prepared"),
            1,
        ),
        (
            "2f",
            quantity("1000", "g", "raw"),
            unknown(),
            quantity("250", "g", "raw"),
            1,
        ),
    ] {
        let q = prepared(
            s,
            suffix,
            known(net),
            drained,
            known(json!(
                "Synthetic exact label ingredients for the pinned input."
            )),
            known(json!(
                "Synthetic known storage, direct-use and handling instructions."
            )),
        );
        assert_eq!(q["version"], 3);
        assert_eq!(q["request"]["version"], 3);
        assert_eq!(q["request"]["methods"].as_array().unwrap().len(), count);
        forbid_private(&q["request"]);
        assert_eq!(audit(&q)["ok"], true);
        let r = recorded(s, &q, &proposed(&q, "supported"));
        assert_eq!(r["version"], 3);
        assert_eq!(r["document"]["meaning"]["requirement_claims"], json!([]));
        let value = qualify(s, &q, &r, NOW);
        assert_eq!(value["status"], "conditional_match");
        assert_eq!(value["pack_basis"]["required"], required);
        assert_eq!(value["effect_enabled"], false);
        assert_eq!(value["recheck_at"], "2026-10-14T16:00:01Z");
    }
    assert_eq!(good(json!({"op":"validate_state","state":s,"now":NOW})), *s);
}

fn beans() -> Value {
    prepared(
        fixture(),
        "22",
        known(quantity("400", "g", "prepared")),
        known(quantity("240", "g", "drained")),
        known(json!(
            "Synthetic kidney beans, water; no actual product was read."
        )),
        known(json!(
            "Synthetic already cooked canned beans; follow original cooling, storage, use-by and reheating conditions."
        )),
    )
}

#[test]
fn v3_resigned_wrong_producer_cross_batch_origin_clock_and_method_omissions_fail() {
    let q = beans();
    for attack in [
        "producer",
        "cross_batch",
        "origin",
        "quantity",
        "clock",
        "plan",
        "context",
        "allocation",
        "condition",
        "method",
        "intrinsic",
        "inheritance",
        "missing_proof",
    ] {
        let mut bad = q.clone();
        match attack {
            "producer" => {
                bad["binding"]["draft"]["line"]["origins"][0]["occurrence"]["recipe"] =
                    json!(aglio_packet_ref().unwrap().recipe)
            }
            "cross_batch" => {
                bad["binding"]["draft"]["line"]["origins"][0]["batch_id"] =
                    json!("ba_800000000000000000000000000000b9")
            }
            "origin" => {
                bad["binding"]["draft"]["line"]["origins"][0]["component_id"] =
                    json!("co_80000000000000000000000000000012")
            }
            "quantity" => {
                bad["binding"]["draft"]["line"]["origins"][0]["original_quantity"]["amount"]["numerator"] =
                    json!("240")
            }
            "clock" => {
                bad["binding"]["draft"]["food_proof"]["derived"]["batches"][0]["components"][0]["original_production"]
                    ["data"]["completed_at"] = good(
                    json!({"op":"resolve_moment","date":"2026-10-14","time":"18:26:00","zone":"Europe/London","resolution":"unambiguous"}),
                )
            }
            "plan" => {
                bad["binding"]["draft"]["food_proof"]["plan"]["batch_links"][0]["source_slot"]["date"] =
                    json!("2026-10-13")
            }
            "context" => bad["binding"]["draft"]["context"]["profile_revision"] = json!("999"),
            "allocation" => {
                let allocation = bad["binding"]["draft"]["food_proof"]["derived"]["allocations"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|a| !a["remix_recipe"].is_null() && a["planned_reheats"] == 1)
                    .unwrap();
                allocation["planned_reheats"] = json!(0);
            }
            "condition" => bad["binding"]["draft"]["occurrences"][0]["condition_text"] = json!([]),
            "method" => {
                bad["binding"]["draft"]["occurrences"][1]["recipe"]["steps"][0]["text"] =
                    json!("Invented clock reset")
            }
            "intrinsic" => {
                bad["binding"]["draft"]["occurrences"][0]["intrinsic_conditions"] =
                    json!({"entry":{"input_condition":"Invented supported input"}})
            }
            "inheritance" => {
                let row = bad["binding"]["draft"]["occurrences"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|r| r.get("inherited_input").is_some())
                    .unwrap();
                row["inherited_input"]["producer_packet"] = json!(aglio_packet_ref().unwrap());
            }
            _ => {
                bad["binding"]["draft"]
                    .as_object_mut()
                    .unwrap()
                    .remove("food_proof");
            }
        }
        resign(&mut bad);
        assert_eq!(audit(&bad)["ok"], false, "{attack}");
    }
    // Coherently remove a complete consumer from every self-authored graph and
    // re-sign both digests. The immutable native allocation still requires it.
    let mut omitted = q.clone();
    let omitted_recipe = json!(chilli_potato_packet_ref().unwrap().recipe);
    omitted["binding"]["draft"]["occurrences"]
        .as_array_mut()
        .unwrap()
        .retain(|r| r["occurrence"]["recipe"] != omitted_recipe);
    omitted["binding"]["draft"]["line"]["uses"]
        .as_array_mut()
        .unwrap()
        .retain(|u| {
            let occurrence = if u["use_key"]["kind"] == "preparation" {
                &u["use_key"]["data"]["phase"]["occurrence"]
            } else {
                &u["use_key"]["data"]["occurrence"]
            };
            occurrence["recipe"] != omitted_recipe
        });
    let handle = omitted["request"]["methods"]
        .as_array()
        .unwrap()
        .iter()
        .find(|m| m["recipe"] == omitted_recipe)
        .unwrap()["method_handle"]
        .clone();
    omitted["request"]["methods"]
        .as_array_mut()
        .unwrap()
        .retain(|m| m["method_handle"] != handle);
    omitted["binding"]["method_map"]
        .as_array_mut()
        .unwrap()
        .retain(|m| m["method_handle"] != handle);
    let typed = serde_json::from_value(omitted["request"].clone()).unwrap();
    omitted["request"]["input_digest"] =
        json!(mstrchf_browser::product_semantic_review::request_digest(&typed).unwrap());
    resign(&mut omitted);
    let refusal = audit(&omitted);
    assert_eq!(refusal["ok"], false);
    assert_eq!(
        refusal["error"]["issues"][0]["path"],
        "/products/semantic/v3/inherited_graph_closure"
    );
    let r = proposed(&q, "supported");
    for attack in [
        "missing",
        "duplicate",
        "authority",
        "ingredients",
        "storage",
    ] {
        let mut bad = r.clone();
        match attack {
            "missing" => {
                bad["cooking"].as_array_mut().unwrap().pop();
            }
            "duplicate" => bad["cooking"][1] = bad["cooking"][0].clone(),
            "authority" => bad["effect_enabled"] = json!(true),
            "ingredients" => {
                bad["identity"]["citations"][0]["quote"] = json!("Not retained source text")
            }
            _ => {
                bad["cooking"][0]["decision"]["citations"]
                    .as_array_mut()
                    .unwrap()
                    .retain(|c| c["field"] != "storage");
            }
        }
        assert_eq!(
            call(
                json!({"op":"product_semantic_audit","prepared":q,"returned":bad,"recorded":null})
            )["ok"],
            false,
            "{attack}"
        );
    }
    let rec = recorded(fixture(), &q, &r);
    let cold: Value = serde_json::from_slice(
        &serde_json::to_vec(&json!({"prepared":q,"returned":r,"recorded":rec})).unwrap(),
    )
    .unwrap();
    assert_eq!(
        call(
            json!({"op":"product_semantic_audit","prepared":cold["prepared"],"returned":cold["returned"],"recorded":cold["recorded"]})
        )["ok"],
        true
    );
    assert_eq!(
        qualify(fixture(), &q, &rec, "2026-10-14T16:00:01Z")["status"],
        "recheck_required"
    );
    assert_eq!(
        q["binding"]["draft"]["food_proof"],
        cold["prepared"]["binding"]["draft"]["food_proof"]
    );
}

#[test]
fn v3_unknown_input_storage_and_drained_measure_stay_unresolved() {
    let s = fixture();
    for (ingredients, storage, identity, cooking, reason) in [
        (
            unknown(),
            unknown(),
            "unknown",
            "unknown",
            "unknown_identity",
        ),
        (
            known(json!("Synthetic plain kidney bean ingredients.")),
            unknown(),
            "supported",
            "unknown",
            "unknown_cooking",
        ),
        (
            known(json!("Synthetic plain kidney bean ingredients.")),
            json!({"kind":"conflicting","data":{"values":["Synthetic direct use","Synthetic do not use directly"]}}),
            "supported",
            "contradictory",
            "contradictory_cooking",
        ),
    ] {
        let q = prepared(
            s,
            "22",
            known(quantity("400", "g", "prepared")),
            known(quantity("240", "g", "drained")),
            ingredients,
            storage,
        );
        let mut r = proposed(&q, identity);
        for c in r["cooking"].as_array_mut().unwrap() {
            c["decision"]["state"] = json!(cooking);
        }
        let rec = recorded(s, &q, &r);
        let value = qualify(s, &q, &rec, NOW);
        assert_eq!(value["status"], "unresolved");
        assert_eq!(value["pack_basis"], Value::Null);
        assert!(
            value["reasons"]
                .as_array()
                .unwrap()
                .contains(&json!(reason))
        );
        assert_eq!(value["effect_enabled"], false);
    }
    let q = prepared(
        s,
        "22",
        known(quantity("999", "g", "prepared")),
        unknown(),
        known(json!("Synthetic kidney beans.")),
        known(json!("Synthetic known storage/direct-use instructions.")),
    );
    let rec = recorded(s, &q, &proposed(&q, "supported"));
    let value = qualify(s, &q, &rec, NOW);
    assert_eq!(value["pack_basis"], Value::Null);
    assert!(
        value["pack_reasons"]
            .as_array()
            .unwrap()
            .contains(&json!("pack_drained_unknown"))
    );
}

#[test]
fn v3_mixed_legacy_food_retains_v2_route_and_separate_full_pinned_policy() {
    let s = admit(fixture(), aglio_packet_ref().unwrap(), 0xb320);
    let s = select(
        &s,
        17,
        aglio_packet_ref().unwrap().recipe,
        Value::Null,
        0xb321,
    );
    let (p, o, a) = source(
        &s,
        known(quantity("500", "g", "dry")),
        unknown(),
        known(json!("Synthetic dry unfilled wheat pasta.")),
        unknown(),
    );
    let mut request = json!({"op":"product_semantic_prepare_v3","state":s,"week_id":s["snapshot"]["weeks"][0]["range"]["week_id"],
        "source_prepared":p,"observation":o,"accepted":a,"product":product(),"line_id":line(&s,"04"),
        "request_id":"sr_800000000000000000000000000000b3","now":NOW,"deadline":DEADLINE});
    let q = good(request.clone());
    let rec = recorded(&s, &q, &proposed(&q, "supported"));
    let v3 = qualify(&s, &q, &rec, NOW);
    assert_eq!(v3["status"], "conditional_match");
    assert_eq!(
        v3["policy"]["id"],
        "tesco-chilli-and-pinned-input-retained-interpretation-v1"
    );
    assert!(q["request"]["methods"][0].get("inherited_input").is_none());
    request["op"] = json!("product_semantic_prepare");
    let legacy = good(request);
    assert_eq!(legacy["version"], 2);
    assert!(legacy["binding"]["draft"].get("food_proof").is_none());
    let rec = recorded(&s, &legacy, &proposed(&legacy, "supported"));
    let v2 = qualify(&s, &legacy, &rec, NOW);
    assert_eq!(
        v2["policy"]["id"],
        "tesco-pinned-input-retained-interpretation-v1"
    );
    assert_eq!(v2["pack_basis"], v3["pack_basis"]);
}

#[test]
fn v3_literal_policy_has_twelve_full_pins_and_original_observation_budget() {
    let policy = mstrchf_browser::product_meaning_admission::chilli_policy_document().unwrap();
    assert_eq!(policy["packets"].as_array().unwrap().len(), 12);
    assert_eq!(policy["max_age_seconds"], 3600);
    assert_eq!(policy["source_truth_guarantee"], false);
    assert_eq!(
        canonical_digest(&policy).unwrap().0,
        "42c2ccd11de411d1e2a12eef701a9a83472e73abb408e9819f21a61a5454c73b"
    );
}
