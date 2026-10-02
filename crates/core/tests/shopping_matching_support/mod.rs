//! Independent synthetic native fixture decoder; no product production encoder.
#![allow(dead_code)]
use mstrchf_contracts::*;
use mstrchf_core::{food::*, products::*, recipes::*};
use serde_json::Value;
#[path = "../../../../tests/fixtures/food/native/support.rs"]
pub mod native;
use native::{Read, read};
macro_rules! wire {($($t:ty),*)=>{$(impl Read for $t{fn read(v:&Value)->Self{serde_json::from_value(v.clone()).unwrap()}})*};}
wire!(
    HardRequirement,
    Quantity,
    DemandDigest,
    DemandLine,
    ContextStamp,
    ConversionEvidence
);
macro_rules! record {($t:ident{$($f:ident),*})=>{impl Read for $t{fn read(v:&Value)->Self{Self{$($f:read(&v[stringify!($f)])),*}}}};}
impl Read for ClaimState {
    fn read(v: &Value) -> Self {
        match v["kind"].as_str().unwrap() {
            "supported" => Self::Supported,
            "refuted" => Self::Refuted,
            "unknown" => Self::Unknown,
            "contradictory" => Self::Contradictory,
            _ => panic!(),
        }
    }
}
impl Read for SourceField {
    fn read(v: &Value) -> Self {
        match v["kind"].as_str().unwrap() {
            "ingredients" => Self::Ingredients,
            "allergens" => Self::Allergens,
            "storage" => Self::Storage,
            _ => panic!(),
        }
    }
}
record!(RuleBinding {
    requirement,
    required_fields,
    review
});
record!(MatchPolicy {
    version,
    digest,
    rules
});
record!(IdentityClaim {
    supply_key,
    state,
    evidence_ids,
    source_fields
});
record!(RequirementClaim {
    requirement,
    state,
    evidence_ids,
    source_fields
});
record!(CookingClaim {
    recipe,
    occurrence,
    state,
    evidence_ids,
    source_fields
});
record!(ProductMeaning {
    product,
    observation_digest,
    key_claim,
    requirement_claims,
    cooking_claims
});
record!(ProductMeaningAdmission {
    review,
    evidence,
    meaning_digest,
    policy_digest,
    freshness_rule,
    meaning_admitted_at,
    valid_until
});
record!(SupplyOrigin {
    occurrence,
    batch_id,
    component_id,
    ingredient_id,
    original_quantity,
    relation
});
record!(MatchUse {
    use_key,
    at,
    requirements,
    cooking
});
record!(MatchBasis {
    saved_basis_digest,
    derived_digest,
    demand_digest,
    context,
    policy_digest,
    line_id,
    line,
    product,
    observation_digest,
    meaning_digest,
    review,
    evidence,
    freshness_rule,
    item_observed_at,
    read_observed_at,
    read_admitted_at,
    meaning_admitted_at,
    valid_until,
    evaluated_at,
    uses,
    supply_key,
    origins
});
impl Read for SupplyRelation {
    fn read(v: &Value) -> Self {
        match v["kind"].as_str().unwrap() {
            "exact" => Self::Exact,
            "forward_conversion" => Self::ForwardConversion {
                evidence: read(&v["data"]["evidence"]),
            },
            _ => panic!(),
        }
    }
}
pub fn vector() -> Value {
    serde_json::from_slice(include_bytes!(
        "../../../../tests/fixtures/shopping/matching/vector-input.json"
    ))
    .unwrap()
}
pub fn base() -> (SavedFoodBasis, DerivedPlan) {
    rebuild(saved_value())
}
pub fn policy() -> MatchPolicy {
    read(&vector()["policy"])
}
pub fn meaning() -> ProductMeaning {
    read(&vector()["meaning"])
}
pub fn admission() -> ProductMeaningAdmission {
    read(&vector()["admission"])
}
pub fn basis() -> MatchBasis {
    read(&vector()["basis"])
}
pub fn now() -> Instant {
    Instant("2026-12-28T12:05:00Z".into())
}
pub struct ReadFixture {
    pub request: JobRequestV1,
    pub reader: ReadActor,
    pub observation: ReadObservationV1,
    pub accepted: AcceptedReadRef,
}
impl ReadFixture {
    pub fn new() -> Self {
        let v = vector();
        let request = serde_json::from_value(v["read_pair"]["request"].clone()).unwrap();
        let reader = serde_json::from_value(v["read_pair"]["expected_reader"].clone()).unwrap();
        let observation: ReadObservationV1 =
            serde_json::from_value(v["read_pair"]["observation"].clone()).unwrap();
        let accepted = AcceptedReadRef {
            evidence_id: EvidenceId(format!("ev_{:032x}", 104)),
            job_id: observation.job_id.clone(),
            read_id: observation.read_id.clone(),
            observation_id: observation.observation_id.clone(),
            digest: canonical_digest(&observation).unwrap(),
            resource: observation.resource.clone(),
            observed_at: observation.observed_at.clone(),
            admitted_at: Instant("2026-12-28T12:01:00Z".into()),
        };
        Self {
            request,
            reader,
            observation,
            accepted,
        }
    }
    pub fn item(&self) -> &ProductObservation {
        match &self.observation.body {
            ReadBody::Products { items, .. } => &items[0],
            _ => panic!(),
        }
    }
    pub fn item_mut(&mut self) -> &mut ProductObservation {
        match &mut self.observation.body {
            ReadBody::Products { items, .. } => &mut items[0],
            _ => panic!(),
        }
    }
    pub fn source(&self) -> ProductReadBasis<'_> {
        ProductReadBasis {
            request: &self.request,
            expected_reader: &self.reader,
            observation: &self.observation,
            accepted: &self.accepted,
            product: self.item().product.clone(),
        }
    }
    pub fn refresh(&mut self, m: &mut ProductMeaning, a: &mut ProductMeaningAdmission) {
        self.accepted.digest = canonical_digest(&self.observation).unwrap();
        m.observation_digest = self.accepted.digest.clone();
        a.meaning_digest = meaning_digest(m).unwrap();
    }
}
pub fn outcome(a: &MatchAssessment) -> &'static str {
    match a {
        MatchAssessment::Suitable(_) => "Suitable",
        MatchAssessment::Unsuitable(_) => "Unsuitable",
        MatchAssessment::Unresolved(_) => "Unresolved",
    }
}
pub fn gf() -> HardRequirement {
    HardRequirement::Diet {
        code: DietCode::GlutenFree,
    }
}
pub fn milk() -> HardRequirement {
    HardRequirement::AllergenExclusion {
        code: AllergenCode::Milk,
    }
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
pub fn gf_base() -> (SavedFoodBasis, DerivedPlan) {
    let mut v = saved_value();
    require(&mut v, &[gf()]);
    add_diet_review(&mut v);
    rebuild(v)
}
pub fn current_meaning(s: &SavedFoodBasis, d: &DerivedPlan, f: &ReadFixture) -> ProductMeaning {
    let graph = mstrchf_core::demand::reconstruct_use_graph(d, s).unwrap();
    let mut m = meaning();
    m.observation_digest = f.accepted.digest.clone();
    m.key_claim.supply_key = s.identities.demand_lines[0].0.clone();
    let mut os: Vec<OccurrenceKey> = graph
        .uses
        .iter()
        .map(|u| match &u.use_key {
            UseKey::Consumption { occurrence, .. } => occurrence.clone(),
            UseKey::Preparation { phase } => phase.occurrence.clone(),
        })
        .collect();
    os.sort_by(occurrence_cmp);
    os.dedup();
    let template = m.cooking_claims[0].clone();
    m.cooking_claims = os
        .into_iter()
        .map(|o| {
            let mut c = template.clone();
            c.recipe = o.recipe.clone();
            c.occurrence = o;
            c
        })
        .collect();
    m
}
pub fn assess(
    s: &SavedFoodBasis,
    d: &DerivedPlan,
    p: &MatchPolicy,
    f: &ReadFixture,
    m: Option<&ProductMeaning>,
    a: Option<&ProductMeaningAdmission>,
    at: Instant,
) -> Result<MatchAssessment, DomainError> {
    let c = prepare_match_context(&s.context, s, d, p, at)?;
    let source = f.source();
    let r = assess_product_meaning(&source, m, a, &c)?;
    match_product(&c, &d.demand.lines[0].line_id, &r)
}
pub fn pin(n: u32) -> EvidencePin {
    EvidencePin {
        evidence_id: EvidenceId(format!("ev_{n:032x}")),
        revision: Revision(1),
        digest: Digest(format!("{n:064x}")),
    }
}
pub fn conversion(v: &mut Value, factors: &[u64], reverse: bool) {
    let r = &mut v["context"]["recipes"][0];
    r["ingredients"][0]["quantity"]["form"] = serde_json::json!("cooked");
    r["ingredients"][0]["quantity"]["amount"]["numerator"] = serde_json::json!("150");
    let source = r["source"].clone();
    let iid = r["ingredients"][0]["ingredient_id"].clone();
    r["ingredients"][0]["conversion_evidence"]=Value::Array(factors.iter().enumerate().map(|(i,f)|serde_json::json!({"evidence_id":format!("ev_{:032x}",501+i),"ingredient_id":iid,"from_form":if reverse{"cooked"}else{"dry"},"to_form":if reverse{"dry"}else{"cooked"},"from_unit":"g","to_unit":"g","factor":{"numerator":if reverse{"1".to_owned()}else{f.to_string()},"denominator":if reverse{f.to_string()}else{"1".to_owned()}},"source":source})).collect());
    for i in 0..factors.len() {
        v["evidence"]["recipes"][0]["admitted"].as_array_mut().unwrap().push(serde_json::json!({"evidence_id":format!("ev_{:032x}",501+i),"revision":"1","digest":format!("{:064x}",501+i)}));
    }
}
pub fn guest_value() -> Value {
    let mut v = saved_value();
    require(&mut v, &[gf()]);
    add_diet_review(&mut v);
    let source = v["context"]["recipes"][0]["source"].clone();
    v["context"]["recipes"][0]["dietary_evidence"].as_array_mut().unwrap().push(serde_json::json!({"evidence_id":"ev_0000000000000000000000000000012f","requirement":milk(),"verdict":"supported","source":source,"review_id":"ev_0000000000000000000000000000012e"}));
    v["evidence"]["recipes"][0]["admitted"].as_array_mut().unwrap().push(serde_json::json!({"evidence_id":"ev_0000000000000000000000000000012f","revision":"1","digest":format!("{:064x}",303)}));
    let guest = serde_json::json!({"kind":"guest","data":{"occasion_id":"oc_00000000000000000000000000000001","guest_id":"gu_00000000000000000000000000000001"}});
    v["plan"]["selections"][1]["servings"][0]["person"] = guest;
    let cell = &mut v["context"]["weeks"][0]["cells"][2];
    cell["attendee_ids"] = serde_json::json!([]);
    cell["occasion"] = serde_json::json!({"occasion_id":"oc_00000000000000000000000000000001","host_diner_ids":[],"guests":[{"guest_id":"gu_00000000000000000000000000000001","age_band":"adult","restrictions":{"kind":"confirmed","data":{"confirmed_at":"2026-12-27T08:00:00Z","requirements":[milk()]}},"portion":{"preset":"regular","adjustment":{"numerator":"1","denominator":"1"},"meal_overrides":[]}}]});
    v
}
pub fn mixed_value(second_conversion: bool) -> Value {
    let mut v = saved_value();
    conversion(&mut v, &[3], false);
    let mut q = v["context"]["recipes"][0].clone();
    q["recipe_id"] = serde_json::json!("rc_0000000000000000000000000000012e");
    q["ingredients"][0]["quantity"]["amount"]["numerator"] = serde_json::json!("200");
    if second_conversion {
        q["ingredients"][0]["conversion_evidence"][0]["factor"]["numerator"] =
            serde_json::json!("2");
    } else {
        q["ingredients"][0]["conversion_evidence"] = serde_json::json!([]);
    }
    let mut qr = v["plan"]["selections"][0]["recipe"].clone();
    qr["recipe_id"] = q["recipe_id"].clone();
    v["context"]["recipes"].as_array_mut().unwrap().push(q);
    v["plan"]["selections"][1]["recipe"] = qr.clone();
    v["plan"]["selections"][1]["batch_use"] = serde_json::json!({"kind":"new_batch","data":{"batch_id":"ba_00000000000000000000000000000002","production_servings":{"numerator":"1","denominator":"1"}}});
    v["plan"]["selections"][1]["servings"][0]["preset_ref"]["recipe"] = qr.clone();
    v["plan"]["batch_links"] = serde_json::json!([]);
    for key in ["recipes", "classes"] {
        let mut x = v["evidence"][key][0].clone();
        replace_recipe_id(&mut x, &qr);
        v["evidence"][key].as_array_mut().unwrap().push(x);
    }
    for key in ["methods", "resources"] {
        let old = v["evidence"][key].as_array().unwrap().clone();
        for mut x in old {
            replace_recipe_id(&mut x, &qr);
            v["evidence"][key].as_array_mut().unwrap().push(x);
        }
    }
    for key in ["routes", "fixed_starts"] {
        let rows = v["schedule"]["layout"][key].as_array_mut().unwrap();
        for x in rows {
            if x["phase"]["occurrence"]["slot"]["meal_type"] == "dinner" {
                x["phase"]["occurrence"]["recipe"] = qr.clone();
                x["phase"]["prep_id"] = serde_json::json!(1);
            }
        }
    }
    v["schedule"]["occupied"]["owners"] = serde_json::json!([]);
    v
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
