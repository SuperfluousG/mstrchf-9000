//! Synthetic people, product facts, external decisions and operator declarations.
//! Real compiled recipes and native state/audit; no source truth or account effect.
use mstrchf_browser::{
    basket_addition_review, maker_packets::*, product_semantic_review, run_json,
};
use serde_json::{Value, json};
use std::sync::OnceLock;

const CREATED: &str = "2026-10-14T15:00:00Z";
const OBSERVED: &str = "2026-10-14T15:00:01Z";
const REVIEWED: &str = "2026-10-14T15:05:00Z";
const DECIDED: &str = "2026-10-14T15:06:00Z";
const DEADLINE: &str = "2026-10-14T16:00:00Z";
fn call(v: Value) -> Value {
    serde_json::from_slice(&run_json(&serde_json::to_vec(&v).unwrap())).unwrap()
}
fn good(v: Value) -> Value {
    let r = call(v);
    assert_eq!(r["ok"], true, "{r}");
    r["value"].clone()
}
fn refused(v: Value) -> Value {
    let r = call(v);
    assert_eq!(r["ok"], false, "{r}");
    let mut e = r["error"].clone();
    e["path"] = e["issues"][0]["path"].clone();
    e
}
fn apply(s: &Value, command: Value, id: u32) -> Value {
    good(json!({"op":"dispatch","state":s,"now":CREATED,"command":{"schema_version":1,
        "command_id":format!("cm_{id:032x}"),"expected_snapshot_revision":s["snapshot"]["snapshot_revision"],"command":command}}))["state"].clone()
}
fn save(s: &Value, w: &Value, id: u32) -> Value {
    apply(
        s,
        json!({"kind":"save_week","data":{"expected_week_revision":w["revision"],"week":{
    "week_id":w["range"]["week_id"],"defaults_snapshot":w["defaults_snapshot"],"day_enabled":w["day_enabled"],
    "meal_type_enabled":w["meal_type_enabled"],"cells":w["cells"],"plan":w["plan"]}}}),
        id,
    )
}
fn fixture() -> &'static Value {
    static STATE: OnceLock<Value> = OnceLock::new();
    STATE.get_or_init(||{
        let mut s=good(json!({"op":"initialize","household_id":"hh_800000000000000000000000000000b6",
            "diner_id":"dn_800000000000000000000000000000b6","zone":"Europe/London","today":"2026-10-14","now":CREATED,"demo":true}));
        let mut p=s["snapshot"]["profile"].clone();p["diners"][0]["age_band"]=json!("adult");
        p["diners"][0]["restrictions"]=json!({"kind":"confirmed_none","data":{"confirmed_at":CREATED}});
        p["diners"][0]["portions"]=json!({"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]});p["preferences"]["equipment"]=json!(["fridge","hob","oven"]);
        s=apply(&s,json!({"kind":"update_profile","data":{"expected_profile_revision":p["revision"],"profile":p}}),0xb600);
        let mut w=s["snapshot"]["weeks"][0].clone();w["plan"]["selections"]=json!([]);w["plan"]["batch_links"]=json!([]);
        w["day_enabled"]=json!([true,true,true,true,true,true,true]);w["meal_type_enabled"]=json!({"breakfast":true,"lunch":true,"dinner":true});for c in w["cells"].as_array_mut().unwrap(){c["choice"]=json!("at_home");}s=save(&s,&w,0xb601);
        let refs=[chilli_batch_packet_ref().unwrap(),chilli_wraps_packet_ref().unwrap(),chilli_potato_packet_ref().unwrap()];
        for(i,packet)in refs.iter().enumerate(){s=apply(&s,json!({"kind":"admit_reviewed_recipe","data":{"packet":packet,"expected_catalogue_revision":(i+1).to_string()}}),0xb602+i as u32);}
        let source=s["snapshot"]["weeks"][0]["cells"][8]["slot"].clone();
        for(i,(index,packet))in [8,10,13].into_iter().zip(refs).enumerate(){let mut w=s["snapshot"]["weeks"][0].clone();
            let options=if i==0{Value::Null}else{json!({"leftover_source":source})};
            let board=good(json!({"op":"board","state":s,"week_id":w["range"]["week_id"],"slot":w["cells"][index]["slot"],"options":options,"now":CREATED}));
            let choice=board["choices"].as_array().unwrap().iter().find(|v|v["selection"]["recipe"]==json!(packet.recipe)).unwrap();w["plan"]=choice["plan"].clone();s=save(&s,&w,0xb605+i as u32);
        }s
    })
}
fn context(s: &Value) -> Value {
    good(
        json!({"op":"product_match_context","state":s,"week_id":s["snapshot"]["weeks"][0]["range"]["week_id"],"now":REVIEWED}),
    )
}
fn known(v: Value) -> Value {
    json!({"kind":"known","data":{"value":v}})
}
fn unknown() -> Value {
    json!({"kind":"unknown","data":{"reason":"not_exposed"}})
}
fn quantity(n: &str, unit: &str, form: &str) -> Value {
    json!({"amount":{"numerator":n,"denominator":"1"},"unit":unit,"form":form})
}
fn product() -> Value {
    json!({"retailer":"tesco","product_id":"123456789","variant_id":null})
}
fn prepared(s: &Value, suffix: &str) -> Value {
    let c = context(s);
    let line = &c["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| {
            l["demand"]["ingredient_id"] == format!("ig_800000000000000000000000000000{suffix}")
        })
        .unwrap()["demand"]["line_id"];
    good(
        json!({"op":"basket_addition_prepare_v3","state":s,"week_id":s["snapshot"]["weeks"][0]["range"]["week_id"],
        "reader":{"provider":"codex_local","native_task_ref":"synthetic-chilli-external-reader"},
        "key":{"retailer":"tesco","account_ref":"synthetic-account-unverified","basket_id":"bk_800000000000000000000000000000b6"},
        "delivery":{"retailer":"tesco","account_ref":"synthetic-account-unverified","context_ref":"synthetic-no-held-slot","delivery_date":null,"slot_ref":null},
        "target":{"product":product(),"url":"https://www.tesco.com/shop/en-GB/products/123456789"},"line_id":line,
        "job_id":"jb_800000000000000000000000000000b6","read_id":"rd_800000000000000000000000000000b6","review_id":"sr_800000000000000000000000000000b6","now":CREATED,"deadline":DEADLINE}),
    )
}
fn observation(p: &Value, net: Value, drained: Value) -> Value {
    let source = &p["source_prepared"];
    json!({"schema_version":1,"observation_id":"ob_800000000000000000000000000000b6",
    "read_id":source["request"]["scope"]["data"]["read_scope"]["read_id"],"job_id":source["request"]["job_id"],"input_digest":source["request"]["input_digest"],
    "reader":source["expected_reader"],"resource":source["request"]["scope"]["data"]["read_scope"]["resource"],"observed_at":OBSERVED,
    "body":{"kind":"products","data":{"delivery_context":source["delivery"],"items":[{"product":product(),"url":"https://www.tesco.com/shop/en-GB/products/123456789",
    "title":"Synthetic product facts; no retailer read","pack":{"count":known(json!(1)),"net":net,"drained":drained},"price":unknown(),"offers":[],"availability":"unknown",
    "ingredients":known(json!("Synthetic exact input label; all recipe conditions still require review.")),"allergens":unknown(),
    "storage":known(json!("Synthetic cooked canned or fresh input as required; follow original cooling, storage, use-by and reheating conditions.")),
    "delivery_context":source["delivery"],"observed_at":OBSERVED}]}}})
}
fn semantic(s: &Value, p: &Value, o: &Value) -> Value {
    good(
        json!({"op":"basket_addition_check_source_v3","state":s,"prepared":p,"observation":o,"now":REVIEWED}),
    )
}
fn returned(q: &Value, status: &str) -> Value {
    let request = &q["semantic_prepared"]["request"];
    let mut citations = vec![];
    if status != "unknown" {
        citations.push(json!({"field":"ingredients","quote":request["product"]["ingredients"]["data"]["value"]}));
    }
    let identity = json!({"state":status,"citations":citations,"reasoning":"Synthetic conditional interpretation only; no observed physical compliance."});
    if status != "unknown" && request["product"]["storage"]["kind"] == "known" {
        citations.push(
            json!({"field":"storage","quote":request["product"]["storage"]["data"]["value"]}),
        );
    }
    json!({"format":"mstrchf-product-semantic-return","version":1,"request_id":request["request_id"],"input_digest":request["input_digest"],"identity":identity,
        "cooking":request["methods"].as_array().unwrap().iter().map(|m|json!({"method_handle":m["method_handle"],"decision":{"state":status,"citations":citations,"reasoning":"Synthetic complete-method interpretation; retain original batch clock and conditions, no handling proof."}})).collect::<Vec<_>>()})
}
fn check(s: &Value, q: &Value, r: &Value, at: &str) -> Value {
    json!({"op":"basket_addition_check_semantic_v3","state":s,"prepared":q,"returned":r,"now":at})
}
fn operator(s: &Value, q: &Value, r: &Value, d: &Value) -> Value {
    json!({"op":"basket_addition_operator_review_v3","state":s,"prepared":q,"returned":r,"reviewed_at":REVIEWED,
    "decision":{"review_digest":d["draft_digest"],"decided_at":DECIDED,"source_and_interpretation_accepted":true,"exact_addition_approved":true},"now":DECIDED})
}
fn beans() -> (Value, Value, Value, Value) {
    let s = fixture();
    let p = prepared(s, "22");
    let o = observation(
        &p,
        known(quantity("400", "g", "prepared")),
        known(quantity("240", "g", "drained")),
    );
    let q = semantic(s, &p, &o);
    let r = returned(&q, "supported");
    (p, o, q, r)
}
fn resign(q: &mut Value) {
    let typed = serde_json::from_value(q["semantic_prepared"].clone()).unwrap();
    q["semantic_prepared"]["prepared_digest"] =
        json!(product_semantic_review::prepared_digest(&typed).unwrap());
    let typed = serde_json::from_value(q.clone()).unwrap();
    q["prepared_digest"] = json!(basket_addition_review::review_digest(&typed).unwrap());
}
fn no_private(v: &Value) {
    match v {
        Value::Object(m) => {
            for (k, v) in m {
                assert!(
                    ![
                        "food_proof",
                        "person",
                        "diner_id",
                        "household_id",
                        "slot",
                        "source_slot",
                        "target_slot",
                        "batch_id",
                        "week_id",
                        "line_id",
                        "occurrences",
                        "uses",
                        "origins",
                        "allocations",
                        "account_ref",
                        "delivery_context",
                        "native_task_ref"
                    ]
                    .contains(&k.as_str()),
                    "{k}"
                );
                no_private(v)
            }
        }
        Value::Array(a) => {
            for v in a {
                no_private(v)
            }
        }
        _ => {}
    }
}

#[test]
fn v3_ai_off_chilli_proposal_preserves_fourteen_exact_demands_and_three_methods_without_authority()
{
    let s = fixture();
    assert_eq!(s["snapshot"]["privacy"]["use_ai_features"], false);
    assert_eq!(
        s["snapshot"]["privacy"]["include_household_requirements"],
        false
    );
    let c = context(s);
    assert_eq!(c["lines"].as_array().unwrap().len(), 14);
    for packet in [
        chilli_batch_packet_ref().unwrap(),
        chilli_wraps_packet_ref().unwrap(),
        chilli_potato_packet_ref().unwrap(),
    ] {
        for input in known_packet(&packet).unwrap().recipe().ingredients.iter() {
            let line = c["lines"]
                .as_array()
                .unwrap()
                .iter()
                .find(|l| l["demand"]["ingredient_id"] == json!(input.ingredient_id))
                .unwrap();
            assert_eq!(line["demand"]["quantity"], json!(input.quantity));
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
        let p = prepared(s, suffix);
        let o = observation(&p, known(net), drained);
        let q = semantic(s, &p, &o);
        let r = returned(&q, "supported");
        let d = good(check(s, &q, &r, REVIEWED));
        assert_eq!(p["version"], 2);
        assert_eq!(q["version"], 2);
        assert_eq!(q["semantic_prepared"]["version"], 3);
        assert_eq!(q["semantic_prepared"]["binding"]["draft"]["version"], 2);
        assert_eq!(
            q["semantic_prepared"]["binding"]["draft"]["source"]["provenance"],
            "external_chat_unwitnessed_correlation"
        );
        assert!(q["semantic_prepared"]["binding"]["draft"]["food_proof"].is_object());
        let request = &q["semantic_prepared"]["request"];
        assert_eq!(request["version"], 3);
        assert_eq!(request["methods"].as_array().unwrap().len(), count);
        no_private(request);
        let inherited: Vec<_> = request["methods"]
            .as_array()
            .unwrap()
            .iter()
            .filter(|m| m.get("inherited_input").is_some())
            .collect();
        assert_eq!(inherited.len(), if count == 3 { 2 } else { 0 });
        for method in inherited {
            assert_eq!(
                method["inherited_input"]["producer_packet"],
                json!(chilli_batch_packet_ref().unwrap())
            );
            assert_eq!(method["inherited_input"].as_object().unwrap().len(), 2);
        }
        assert_eq!(d["version"], 2);
        assert_eq!(d["qualification"]["status"], "conditional_match");
        assert_eq!(d["qualification"]["pack_basis"]["required"], required);
        assert_eq!(d["qualification"]["recheck_at"], DEADLINE);
        assert_eq!(d["source_status"], "source_witness_pending");
        assert_eq!(d["provenance"], "external_chat_proposal");
        assert_eq!(d["qualification"]["admission_status"], "missing");
        assert_eq!(d["qualification"]["freshness_status"], "unqualified");
        for field in [
            "source_truth_assessed",
            "account_verified",
            "human_review_recorded",
            "effect_enabled",
            "reusable_for_intent",
        ] {
            assert_eq!(d[field], false);
        }
        for field in [
            "accepted",
            "admission",
            "policy",
            "candidate",
            "match_basis",
            "grant_token",
            "intent",
            "state",
        ] {
            assert!(d.get(field).is_none());
        }
        assert_eq!(good(check(s, &q, &r, REVIEWED)), d);
        good(
            json!({"op":"product_semantic_audit","prepared":q["semantic_prepared"],"returned":null,"recorded":null}),
        );
        let assisted = call(
            json!({"op":"product_semantic_inspect","state":s,"prepared":q["semantic_prepared"],"now":REVIEWED}),
        );
        assert_eq!(assisted["ok"], false);
    }
    assert_eq!(
        good(json!({"op":"validate_state","state":s,"now":DECIDED})),
        *s
    );
}

#[test]
fn v3_operator_keeps_exact_one_pack_limit_and_separate_review_decision() {
    let s = fixture();
    let (_, _, q, r) = beans();
    let d = good(check(s, &q, &r, REVIEWED));
    assert_eq!(
        d["qualification"]["pack_basis"],
        json!({"required":quantity("480","g","drained"),"supply":quantity("240","g","drained")})
    );
    assert_eq!(
        refused(operator(s, &q, &r, &d))["path"],
        "/basket/addition/operator_one_pack"
    );
    for (suffix, net) in [
        ("2e", quantity("8", "each", "prepared")),
        ("2f", quantity("1000", "g", "raw")),
    ] {
        let p = prepared(s, suffix);
        let q = semantic(s, &p, &observation(&p, known(net), unknown()));
        let r = returned(&q, "supported");
        let d = good(check(s, &q, &r, REVIEWED));
        let input = operator(s, &q, &r, &d);
        let out = good(input.clone());
        assert_eq!(out["version"], 2);
        assert_eq!(out["additional_packs"], 1);
        assert_eq!(out["review_digest"], d["draft_digest"]);
        assert_ne!(out["decision_digest"], d["draft_digest"]);
        assert_eq!(
            out["source_observation_digest"],
            d["source_observation_digest"]
        );
        assert_eq!(out["interpretation_digest"], d["return_digest"]);
        for field in [
            "source_truth_assessed",
            "account_verified",
            "price_assessed",
            "effect_enabled",
            "reusable_for_intent",
        ] {
            assert_eq!(out[field], false);
        }
        assert_eq!(good(input.clone()), out);
        for field in [
            "source_and_interpretation_accepted",
            "exact_addition_approved",
        ] {
            let mut wrong = input.clone();
            wrong["decision"][field] = json!(false);
            assert_eq!(refused(wrong)["path"], "/basket/addition/operator_decision");
        }
        for field in ["quantity", "grant_token", "human_action_verified"] {
            let mut wrong = input.clone();
            wrong["decision"][field] = json!(true);
            refused(wrong);
        }
        let mut wrong = input.clone();
        wrong["decision"]["review_digest"] = json!("f".repeat(64));
        assert_eq!(refused(wrong)["path"], "/basket/addition/review_digest");
        let mut wrong = input.clone();
        wrong["returned"]["identity"]["reasoning"] = json!("Changed external interpretation");
        assert_eq!(refused(wrong)["path"], "/basket/addition/review_digest");
        let mut wrong = input;
        wrong["decision"]["decided_at"] = json!(REVIEWED);
        assert_eq!(refused(wrong)["path"], "/basket/addition/operator_clock");
    }
}

#[test]
fn v3_unknown_and_drained_or_unit_mismatch_never_become_a_pack_or_effect() {
    let s = fixture();
    let (p, o, _, _) = beans();
    for (mode, field, value, status) in [
        ("ingredients", "ingredients", unknown(), "unknown"),
        ("storage", "storage", unknown(), "unknown"),
        ("drained", "drained", unknown(), "supported"),
        (
            "conflicting-drained",
            "drained",
            json!({"kind":"conflicting","data":{"values":[quantity("240","g","drained"),quantity("400","g","drained")]}}),
            "supported",
        ),
        (
            "gross-only",
            "drained",
            known(quantity("400", "g", "prepared")),
            "supported",
        ),
    ] {
        let mut observation = o.clone();
        if field == "drained" {
            observation["body"]["data"]["items"][0]["pack"][field] = value
        } else {
            observation["body"]["data"]["items"][0][field] = value
        }
        let q = semantic(s, &p, &observation);
        let r = returned(&q, status);
        let d = good(check(s, &q, &r, REVIEWED));
        assert_eq!(d["qualification"]["pack_basis"], Value::Null, "{mode}");
        assert_eq!(d["effect_enabled"], false);
        refused(operator(s, &q, &r, &d));
    }
    // Original spices are measured in ml; a gram-labelled pack is not a conversion.
    let c = context(s);
    let spice = c["lines"]
        .as_array()
        .unwrap()
        .iter()
        .find(|l| l["demand"]["quantity"]["unit"] == "ml")
        .unwrap();
    let id = spice["demand"]["ingredient_id"].as_str().unwrap();
    let p = prepared(s, &id[id.len() - 2..]);
    let q = semantic(
        s,
        &p,
        &observation(&p, known(quantity("40", "g", "prepared")), unknown()),
    );
    let r = returned(&q, "supported");
    let d = good(check(s, &q, &r, REVIEWED));
    assert_eq!(d["qualification"]["pack_basis"], Value::Null);
    refused(operator(s, &q, &r, &d));
}

#[test]
fn v3_exact_versions_clocks_privacy_and_unwitnessed_custody_cannot_be_substituted() {
    let s = fixture();
    let (p, o, q, r) = beans();
    let d = good(check(s, &q, &r, REVIEWED));
    assert_eq!(
        refused(
            json!({"op":"basket_addition_check_source","state":s,"prepared":p,"observation":o,"now":REVIEWED})
        )["path"],
        "/basket/addition/source"
    );
    let mut old = check(s, &q, &r, REVIEWED);
    old["op"] = json!("basket_addition_check_semantic");
    assert_eq!(refused(old)["path"], "/basket/addition/semantic");
    let mut old = operator(s, &q, &r, &d);
    old["op"] = json!("basket_addition_operator_review");
    assert_eq!(refused(old)["path"], "/basket/addition/semantic");
    let fixture: Value =
        serde_json::from_slice(include_bytes!("fixtures/basket-addition-v1-v55.json")).unwrap();
    let legacy = &fixture["semantic"];
    let old_s = &fixture["state"];
    let at = "2026-10-13T10:05:00Z";
    refused(
        json!({"op":"basket_addition_check_source_v3","state":old_s,"prepared":fixture["source"],"observation":legacy["semantic_prepared"]["binding"]["observation"],"now":at}),
    );
    refused(check(old_s, legacy, &fixture["returned"], at));
    let mut hybrid = legacy.clone();
    hybrid["version"] = json!(2);
    resign(&mut hybrid);
    assert_eq!(
        refused(check(old_s, &hybrid, &fixture["returned"], at))["path"],
        "/basket/addition/semantic_version"
    );
    let mut hybrid = q.clone();
    hybrid["version"] = json!(1);
    resign(&mut hybrid);
    let mut input = check(s, &hybrid, &r, REVIEWED);
    input["op"] = json!("basket_addition_check_semantic");
    assert_eq!(refused(input)["path"], "/basket/addition/semantic_version");
    assert_eq!(
        good(check(s, &q, &r, "2026-10-14T15:15:00Z"))["qualification"]["recheck_at"],
        DEADLINE
    );
    for at in [CREATED, DEADLINE, "2026-10-14T16:00:01Z"] {
        refused(check(s, &q, &r, at));
    }
    for field in ["reader", "job_id"] {
        let mut changed = o.clone();
        if field == "reader" {
            changed[field]["native_task_ref"] = json!("foreign-operator")
        } else {
            changed[field] = json!("jb_800000000000000000000000000000b7")
        };
        refused(
            json!({"op":"basket_addition_check_source_v3","state":s,"prepared":p,"observation":changed,"now":REVIEWED}),
        );
    }
    let mut changed = q.clone();
    changed["source_status"] = json!("accepted");
    resign(&mut changed);
    refused(check(s, &changed, &r, REVIEWED));
    let mut changed = q.clone();
    changed["semantic_prepared"]["binding"]["draft"]["source"]["provenance"] =
        json!("retained_source_correlation_only");
    resign(&mut changed);
    refused(check(s, &changed, &r, REVIEWED));
    let mut policy = s["snapshot"]["privacy"].clone();
    policy["use_ai_features"] = json!(true);
    let changed = apply(
        s,
        json!({"kind":"set_privacy","data":{"expected_policy_revision":policy["revision"],"policy":policy}}),
        0xb610,
    );
    refused(check(&changed, &q, &r, REVIEWED));
    let mut w = s["snapshot"]["weeks"][0].clone();
    w["plan"]["selections"] = json!([]);
    w["plan"]["batch_links"] = json!([]);
    let changed = save(s, &w, 0xb611);
    refused(check(&changed, &q, &r, REVIEWED));
}

#[test]
fn v3_resigned_batch_origin_clock_method_and_unknown_conditions_cannot_escape_reconstruction() {
    let s = fixture();
    let (_, _, q, r) = beans();
    for attack in [
        "batch",
        "origin",
        "clock",
        "plan",
        "prose",
        "inherited",
        "proof",
    ] {
        let mut changed = q.clone();
        let draft = &mut changed["semantic_prepared"]["binding"]["draft"];
        match attack {
            "batch" => {
                draft["food_proof"]["derived"]["allocations"][0]["batch_id"] =
                    json!("ba_800000000000000000000000000000ff")
            }
            "origin" => {
                draft["line"]["origins"][0]["original_quantity"] = quantity("999", "g", "drained")
            }
            "clock" => {
                draft["food_proof"]["derived"]["batches"][0]["components"][0]["original_production"]
                    ["data"]["completed_at"] = good(
                    json!({"op":"resolve_moment","date":"2026-10-14","time":"18:26:00","zone":"Europe/London","resolution":"unambiguous"}),
                )
            }
            "plan" => draft["food_proof"]["plan"]["batch_links"] = json!([]),
            "prose" => draft["occurrences"][0]["condition_text"] = json!([]),
            "inherited" => {
                let row = draft["occurrences"]
                    .as_array_mut()
                    .unwrap()
                    .iter_mut()
                    .find(|row| row.get("inherited_input").is_some())
                    .unwrap();
                row["inherited_input"]["producer_packet"]["recipe"]["digest"] =
                    json!("f".repeat(64));
            }
            _ => {
                draft.as_object_mut().unwrap().remove("food_proof");
            }
        }
        resign(&mut changed);
        refused(check(s, &changed, &r, REVIEWED));
    }
    for attack in [
        "missing",
        "duplicate",
        "storage",
        "foreign-citation",
        "authority",
    ] {
        let mut changed = r.clone();
        match attack {
            "missing" => {
                changed["cooking"].as_array_mut().unwrap().pop();
            }
            "duplicate" => {
                let first = changed["cooking"][0].clone();
                changed["cooking"].as_array_mut().unwrap().push(first);
            }
            "storage" => changed["cooking"][1]["decision"]["citations"]
                .as_array_mut()
                .unwrap()
                .retain(|v| v["field"] != "storage"),
            "foreign-citation" => {
                changed["identity"]["citations"][0]["quote"] = json!("Unobserved source text")
            }
            _ => changed["effect_enabled"] = json!(true),
        }
        refused(check(s, &q, &changed, REVIEWED));
    }
    assert_eq!(
        good(json!({"op":"validate_state","state":s,"now":DECIDED})),
        *s
    );
}
