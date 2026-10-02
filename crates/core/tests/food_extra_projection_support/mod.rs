//! Frozen synthetic Food fixture builders; no matcher runtime dependency.
#![allow(dead_code)]
use mstrchf_contracts::*;
use mstrchf_core::food::*;
use serde_json::Value;
#[path = "../../../../tests/fixtures/food/native/support.rs"]
pub mod native;
pub fn now() -> Instant {
    Instant("2026-12-28T12:05:00Z".into())
}
pub fn saved_value() -> Value {
    native::value("subject-saved-basis")
}
fn replace_refs(v: &mut Value, refs: &[RecipeRef]) {
    match v {
        Value::Object(o) => {
            if o.len() == 3
                && o.contains_key("recipe_id")
                && o.contains_key("revision")
                && o.contains_key("digest")
                && let Some(r) = refs.iter().find(|r| o["recipe_id"] == r.recipe_id.0)
            {
                *v = serde_json::to_value(r).unwrap();
                return;
            }
            for x in o.values_mut() {
                replace_refs(x, refs);
            }
        }
        Value::Array(a) => {
            for x in a {
                replace_refs(x, refs)
            }
        }
        _ => {}
    }
}
pub fn rebuild_inputs(mut v: Value) -> SavedFoodBasis {
    for _ in 0..3 {
        let refs: Vec<RecipeRef> = v["context"]["recipes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|j| {
                let r: RecipeRevision = serde_json::from_value(j.clone()).unwrap();
                RecipeRef {
                    recipe_id: r.recipe_id.clone(),
                    revision: r.revision,
                    digest: recipe_digest(&r).unwrap(),
                }
            })
            .collect();
        replace_refs(&mut v, &refs);
        v["context"]["stamp"]["recipe_refs"] = serde_json::to_value(&refs).unwrap();
    }
    v["context"]["weeks"][0]["plan"] = v["plan"].clone();
    v["context"]["weeks"][0]["defaults_snapshot"]["suggest_leftovers"] = serde_json::json!(true);
    let mut s: SavedFoodBasis = native::read(&v);
    s.schedule
        .layout
        .routes
        .sort_by(|a, b| phase_cmp(&a.phase, &b.phase));
    s.schedule
        .layout
        .fixed_starts
        .sort_by(|a, b| phase_cmp(&a.phase, &b.phase));
    s.context.requirements = BoundedVec(
        mstrchf_core::availability::meal_requirements(
            &s.context.household,
            &s.context.weeks[0],
            &[],
        )
        .unwrap()
        .to_vec(),
    );
    s.evidence.catalogue_digest = food_evidence_digest(&s.evidence).unwrap();
    for o in &mut s.schedule.occupied.owners {
        o.projection_digest = occupied_owner_digest(o).unwrap();
    }
    s.schedule.occupied.digest = occupancy_snapshot_digest(&s.schedule.occupied).unwrap();
    s
}
pub fn rebuild(v: Value) -> (SavedFoodBasis, DerivedPlan) {
    let mut s = rebuild_inputs(v);
    let keys = required_identity_keys(&s.plan, &s.context, &s.evidence, &s.schedule).unwrap();
    s.identities.allocations = keys
        .allocations
        .into_iter()
        .enumerate()
        .map(|(i, k)| (k, AllocationId(format!("al_{:032x}", i + 1))))
        .collect();
    s.identities.sessions = keys
        .sessions
        .into_iter()
        .enumerate()
        .map(|(i, k)| (k, PrepSessionId(format!("ps_{:032x}", i + 1))))
        .collect();
    s.identities.demand_lines = keys
        .demand_lines
        .into_iter()
        .enumerate()
        .map(|(i, k)| (k, DemandLineId(format!("dl_{:032x}", i + 1))))
        .collect();
    s.identities.digest = food_identities_digest(&s.identities).unwrap();
    let mut products = ProductUseSnapshot {
        revision: Revision(1),
        digest: PayloadDigest("0".repeat(64)),
        valid_until: None,
        evidence: vec![],
        bindings: vec![],
        facts: vec![],
    };
    products.digest = digest_product_uses(&products).unwrap();
    let mut feasibility = FeasibilityInputs {
        revision: Revision(1),
        digest: PayloadDigest("0".repeat(64)),
        delivery: DeliveryInput::Unselected,
        products,
        confirmations: vec![],
        evaluated_at: now(),
    };
    feasibility.digest = digest_feasibility_inputs(&feasibility).unwrap();
    let d = derive_plan(
        &s.plan,
        &s.context,
        &FoodInputs {
            evidence: &s.evidence,
            identities: &s.identities,
            schedule: &s.schedule,
            feasibility: &feasibility,
        },
    )
    .unwrap();
    (s, d.plan)
}
pub fn require(v: &mut Value, reqs: &[HardRequirement]) {
    v["context"]["household"]["diners"][0]["restrictions"] = serde_json::json!({"kind":"confirmed","data":{"confirmed_at":"2026-12-27T08:00:00Z","requirements":reqs}});
    let r = &mut v["context"]["recipes"][0];
    let source = r["source"].clone();
    r["dietary_evidence"]=Value::Array(reqs.iter().map(|q|serde_json::json!({"evidence_id":"ev_0000000000000000000000000000012d","requirement":q,"verdict":"supported","source":source,"review_id":"ev_0000000000000000000000000000012e"})).collect());
}
pub fn add_diet_review(v: &mut Value) {
    v["context"]["recipes"][0]["reviews"].as_array_mut().unwrap().push(serde_json::json!({"evidence_id":"ev_0000000000000000000000000000012e","kind":"dietary_review","actor":"automated","at":"2026-12-27T10:00:00Z","note":"SYNTHETIC reviewed dietary premise"}));
    v["evidence"]["recipes"][0]["admitted"].as_array_mut().unwrap().push(serde_json::json!({"evidence_id":"ev_0000000000000000000000000000012e","revision":"1","digest":format!("{:064x}",302)}));
}
fn replace_recipe_id(v: &mut Value, qr: &Value) {
    match v {
        Value::Object(o) => {
            if o.len() == 3
                && o.contains_key("recipe_id")
                && o.contains_key("revision")
                && o.contains_key("digest")
            {
                *v = qr.clone();
                return;
            }
            for x in o.values_mut() {
                replace_recipe_id(x, qr)
            }
        }
        Value::Array(a) => {
            for x in a {
                replace_recipe_id(x, qr)
            }
        }
        _ => {}
    }
}
pub fn extra_value() -> Value {
    let mut v = saved_value();
    let veg = HardRequirement::Diet {
        code: DietCode::Vegetarian,
    };
    require(&mut v, &[veg]);
    add_diet_review(&mut v);
    let mut b = v["context"]["household"]["diners"][0].clone();
    b["diner_id"] = serde_json::json!("dn_00000000000000000000000000000002");
    b["restrictions"] =
        serde_json::json!({"kind":"confirmed_none","data":{"confirmed_at":"2026-12-27T08:00:00Z"}});
    v["context"]["household"]["diners"]
        .as_array_mut()
        .unwrap()
        .push(b);
    v["context"]["weeks"][0]["cells"][1]["attendee_ids"]
        .as_array_mut()
        .unwrap()
        .push(serde_json::json!("dn_00000000000000000000000000000002"));
    let mut serving = v["plan"]["selections"][0]["servings"][0].clone();
    serving["person"]["data"]["diner_id"] =
        serde_json::json!("dn_00000000000000000000000000000002");
    v["plan"]["selections"][0]["servings"]
        .as_array_mut()
        .unwrap()
        .push(serving.clone());
    v["plan"]["selections"][0]["batch_use"]["data"]["production_servings"]["numerator"] =
        serde_json::json!("3");
    let mut q = v["context"]["recipes"][0].clone();
    q["recipe_id"] = serde_json::json!("rc_0000000000000000000000000000012e");
    replace_scalar(
        &mut q,
        "ig_0000000000000000000000000000012d",
        "ig_0000000000000000000000000000012e",
    );
    replace_scalar(
        &mut q,
        "co_0000000000000000000000000000012d",
        "co_0000000000000000000000000000012e",
    );
    q["dietary_evidence"] = serde_json::json!([]);
    q["ingredients"][0]["quantity"]["amount"]["numerator"] = serde_json::json!("100");
    q["prep"][0]["earliest_offset_seconds"] = serde_json::json!(-1200);
    q["prep"][0]["latest_offset_seconds"] = serde_json::json!(-1200);
    let mut qr = v["plan"]["selections"][0]["recipe"].clone();
    qr["recipe_id"] = q["recipe_id"].clone();
    serving["preset_ref"]["recipe"] = qr.clone();
    v["context"]["recipes"][0]["extras"] = serde_json::json!([{"extra_id":"ex_00000000000000000000000000000001","title":"SYNTHETIC B-only extra","recipe":qr,"eligible_person_requirements":[],"supported_with":["co_0000000000000000000000000000012d"]}]);
    v["context"]["recipes"].as_array_mut().unwrap().push(q);
    v["plan"]["selections"][0]["extras"] = serde_json::json!([{"extra_id":"ex_00000000000000000000000000000001","recipe":qr,"recipients":[serving]}]);
    for key in ["recipes", "classes"] {
        let mut x = v["evidence"][key][0].clone();
        replace_recipe_id(&mut x, &qr);
        if key == "classes" {
            replace_scalar(
                &mut x,
                "co_0000000000000000000000000000012d",
                "co_0000000000000000000000000000012e",
            );
        }
        v["evidence"][key].as_array_mut().unwrap().push(x);
    }
    for key in ["methods", "resources"] {
        let old = v["evidence"][key].as_array().unwrap().clone();
        for mut x in old {
            replace_recipe_id(&mut x, &qr);
            replace_scalar(
                &mut x,
                "co_0000000000000000000000000000012d",
                "co_0000000000000000000000000000012e",
            );
            v["evidence"][key].as_array_mut().unwrap().push(x);
        }
    }
    let mut route = v["schedule"]["layout"]["routes"][0].clone();
    route["phase"]["occurrence"]["extra_id"] =
        serde_json::json!("ex_00000000000000000000000000000001");
    route["phase"]["occurrence"]["recipe"] = qr;
    route["session"]["ordinal"] = serde_json::json!(2);
    let mut start = v["schedule"]["layout"]["fixed_starts"][0].clone();
    start["phase"] = route["phase"].clone();
    start["start"]["local_time"] = serde_json::json!("11:50:00");
    start["start"]["instant"] = serde_json::json!("2026-12-28T11:50:00Z");
    v["schedule"]["occupied"]["query"]["horizon"]["start"] = start["start"].clone();
    v["schedule"]["layout"]["routes"]
        .as_array_mut()
        .unwrap()
        .push(route);
    v["schedule"]["layout"]["fixed_starts"]
        .as_array_mut()
        .unwrap()
        .push(start);
    v["schedule"]["occupied"]["owners"] = serde_json::json!([]);
    v
}
fn replace_scalar(v: &mut Value, a: &str, b: &str) {
    match v {
        Value::String(s) if s == a => *s = b.into(),
        Value::Object(o) => {
            for x in o.values_mut() {
                replace_scalar(x, a, b)
            }
        }
        Value::Array(ar) => {
            for x in ar {
                replace_scalar(x, a, b)
            }
        }
        _ => {}
    }
}
