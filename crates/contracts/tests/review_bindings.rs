use mstrchf_contracts::*;
use serde_json::{Value, json};
fn load(name: &str) -> Value {
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
fn decode<T: serde::de::DeserializeOwned + ValidateShape>(v: &Value) -> T {
    parse_document(&serde_json::to_vec(v).unwrap()).unwrap()
}
fn bind_digest(r: &mut JobRequestV1) {
    r.input_digest = input_digest(&RequestMaterial::from(&*r)).unwrap();
}
fn fragment(request: &JobRequestV1, value: Value) -> Result<(), DomainError> {
    let mut r = request.clone();
    let bytes = serde_json::to_vec(&value).unwrap();
    let id = ArtefactId("ar_00000000000000000000000000000099".into());
    r.allowed_artefacts = BoundedVec(vec![ArtefactRef {
        artefact_id: id.clone(),
        relative_name: Text(format!("input/{}.json", id.0)),
        digest: bytes_digest(&bytes),
        byte_length: bytes.len() as u32,
        media_type: ArtefactMediaType::Json,
    }]);
    bind_digest(&mut r);
    validate_outbound(
        &OutboundBundle {
            request: r,
            artefacts: vec![AllowedArtefactBytes {
                artefact_id: id,
                bytes,
            }],
        },
        &PrivacyPolicy {
            revision: Revision(1),
            use_ai_features: true,
            include_household_requirements: true,
            confirmed_at: Nullable(None),
        },
    )
}
fn result_pair() -> (JobRequestV1, Value) {
    let reqv = load("job-request-personalised.json");
    let req = decode(&reqv);
    let mut result = load("job-result-complete-noop.json");
    for name in [
        "job_id",
        "action",
        "week_id",
        "base_plan_revision",
        "context_stamp",
        "privacy_policy_revision",
        "input_digest",
    ] {
        result[name] = reqv[name].clone();
    }
    let mut s = load("week-disabled-retained.json")["plan"]["selections"][0].clone();
    let slot = reqv["scope"]["data"]["slots"][0].clone();
    s["slot"] = slot.clone();
    s["origin"] = json!("suggested");
    let serving = s["servings"][0].clone();
    s["servings"] = json!(
        reqv["payload"]["data"]["requirements"][0]["attendees"]
            .as_array()
            .unwrap()
            .iter()
            .map(|a| {
                let mut p = serving.clone();
                p["person"] = a["person"].clone();
                p["preset_ref"]["age_band"] = a["age_band"].clone();
                p
            })
            .collect::<Vec<_>>()
    );
    result["proposal"]["data"]["proposal"]["changes"] = json!([{"week_id":reqv["week_id"],"base_revision":"1","replace_slots":[{"slot":slot,"selection":s}]}]);
    (req, result)
}
fn selection(v: &mut Value) -> &mut Value {
    &mut v["proposal"]["data"]["proposal"]["changes"][0]["replace_slots"][0]["selection"]
}
#[test]
fn f01_parent_payload_recipe_references_must_also_be_closed() {
    let mut req: JobRequestV1 = decode(&load("job-request-personalised.json"));
    let mut foreign = req.context_stamp.recipe_refs[0].clone();
    foreign.recipe_id = RecipeId("rc_00000000000000000000000000000099".into());
    if let JobPayload::PersonalisedPlan { history, .. } = &mut req.payload {
        history.selected_recipe_refs.0.push(foreign.clone());
    }
    bind_digest(&mut req);
    assert!(
        req.validate_shape().is_err(),
        "consulted AI history must be stamped"
    );
    let mut req: JobRequestV1 = decode(&load("job-request-recipe-candidate.json"));
    if let JobPayload::RecipeCandidate { adaptation, .. } = &mut req.payload {
        adaptation.0 = Some(RecipeAdaptation {
            requirements: BoundedVec(vec![]),
            equipment: BoundedVec(vec![]),
            portion_requests: BoundedVec(vec![PortionPresetRef {
                recipe: foreign,
                preset: PortionPresetName::Regular,
                age_band: AgeBand::Adult,
                meal_type: MealType::Dinner,
            }]),
        });
    }
    bind_digest(&mut req);
    assert!(
        req.validate_shape().is_err(),
        "adaptation preset must be stamped"
    );
}
#[test]
fn f02_allowed_extra_and_override_are_preserved() {
    let (mut req, mut result) = result_pair();
    if let JobPayload::PersonalisedPlan { recipe_refs, .. } = &mut req.payload {
        recipe_refs.0.push(req.context_stamp.recipe_refs[1].clone());
    }
    bind_digest(&mut req);
    result["input_digest"] = json!(req.input_digest);
    let s = selection(&mut result);
    let recipe = serde_json::to_value(&req.context_stamp.recipe_refs[1]).unwrap();
    let mut recipient = s["servings"][0].clone();
    recipient["preset_ref"]["recipe"] = recipe.clone();
    s["extras"] = json!([{"extra_id":"ex_00000000000000000000000000000001","recipe":recipe,"recipients":[recipient]}]);
    s["meal_overrides"]["portions"] = json!([{"person":s["servings"][0]["person"],"preset":"regular","adjustment":{"numerator":"1","denominator":"1"}}]);
    validate_result_binding(&decode(&result), &req, &req.context_stamp).unwrap();
}
#[test]
fn f01_parent_requirements_and_permitted_recipe_set_are_not_replaced() {
    let req: JobRequestV1 = decode(&load("job-request-personalised.json"));
    let payload = serde_json::to_value(&req.payload).unwrap();
    fragment(&req, payload.clone()).unwrap();
    for (label, changed) in ["recipes", "person", "restriction", "preferences", "history"]
        .into_iter()
        .map(|label| {
            let mut f = payload.clone();
            match label {
                "recipes" => {
                    f["data"]["recipe_refs"] =
                        serde_json::to_value(&req.context_stamp.recipe_refs).unwrap()
                }
                "person" => {
                    f["data"]["requirements"][0]["attendees"][0]["person"]["data"]["diner_id"] =
                        json!("dn_00000000000000000000000000000099")
                }
                "restriction" => {
                    f["data"]["requirements"][0]["attendees"][0]["hard_requirements"] = json!([]);
                    f["data"]["requirements"][0]["shared_hard_requirements"] = json!([
                        f["data"]["requirements"][0]["attendees"][1]["hard_requirements"][0]
                            .clone()
                    ]);
                }
                "preferences" => f["data"]["preferences"]["quick_meals"] = json!(true),
                "history" => {
                    f["data"]["history"]["selected_recipe_refs"] =
                        json!([f["data"]["recipe_refs"][0].clone()])
                }
                _ => unreachable!(),
            };
            (label, f)
        })
    {
        assert_eq!(
            fragment(&req, changed).unwrap_err().code,
            ErrorCode::ScopeViolation,
            "{label}"
        );
    }
}
#[test]
fn f01_all_payload_variants_keep_unchanged_fragments_valid() {
    for file in [
        "job-request-personalised.json",
        "job-request.json",
        "job-request-recipe-candidate.json",
        "job-request-product-observation.json",
        "job-request-calendar-publication.json",
        "job-request-booking-check.json",
        "job-request-setup.json",
    ] {
        let req: JobRequestV1 = decode(&load(file));
        fragment(&req, serde_json::to_value(&req.payload).unwrap())
            .unwrap_or_else(|e| panic!("{file}: {e:?}"));
    }
}
#[test]
fn f01_complete_record_subset_is_valid() {
    let mut req: JobRequestV1 = decode(&load("job-request-personalised.json"));
    if let JobPayload::PersonalisedPlan {
        requirements,
        recipe_refs,
        history,
        ..
    } = &mut req.payload
    {
        let mut extra = requirements[0].clone();
        extra.slot.date = LocalDate("2026-12-29".into());
        let moment = extra
            .eat_by
            .0
            .as_mut()
            .expect("active fixture has a deadline");
        moment.local_date = extra.slot.date.clone();
        moment.instant = Instant("2026-12-29T18:00:00Z".into());
        requirements.0.push(extra.clone());
        recipe_refs.0.push(req.context_stamp.recipe_refs[1].clone());
        history.selected_recipe_refs = recipe_refs.clone();
        if let ActionScope::Plan { slots, .. } = &mut req.scope {
            slots.0.push(extra.slot);
        }
    }
    bind_digest(&mut req);
    let mut f = serde_json::to_value(&req.payload).unwrap();
    f["data"]["requirements"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    f["data"]["recipe_refs"].as_array_mut().unwrap().truncate(1);
    f["data"]["history"]["selected_recipe_refs"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    fragment(&req, f).unwrap();
}
#[test]
fn f02_selection_recipe_and_people_are_bound_to_request() {
    let (req, control) = result_pair();
    validate_result_binding(&decode(&control), &req, &req.context_stamp).unwrap();
    for label in [
        "recipe",
        "person",
        "preset_recipe",
        "preset_age",
        "extra_recipe",
        "extra_person",
        "override_person",
    ] {
        let mut bad = control.clone();
        let s = selection(&mut bad);
        match label {
            "recipe" => {
                s["recipe"]["recipe_id"] = json!("rc_00000000000000000000000000000099");
                let foreign_recipe = s["recipe"].clone();
                for p in s["servings"].as_array_mut().unwrap() {
                    p["preset_ref"]["recipe"] = foreign_recipe.clone();
                }
            }
            "person" => {
                s["servings"][0]["person"]["data"]["diner_id"] =
                    json!("dn_00000000000000000000000000000099")
            }
            "preset_recipe" => {
                s["servings"][0]["preset_ref"]["recipe"]["recipe_id"] =
                    json!("rc_00000000000000000000000000000099")
            }
            "preset_age" => s["servings"][0]["preset_ref"]["age_band"] = json!("child_1_3"),
            "extra_recipe" | "extra_person" => {
                let mut recipient = s["servings"][0].clone();
                let mut recipe = s["recipe"].clone();
                if label == "extra_recipe" {
                    recipe["recipe_id"] = json!("rc_00000000000000000000000000000099");
                    recipient["preset_ref"]["recipe"] = recipe.clone();
                } else {
                    recipient["person"]["data"]["diner_id"] =
                        json!("dn_00000000000000000000000000000099");
                }
                s["extras"] = json!([{"extra_id":"ex_00000000000000000000000000000001","recipe":recipe,"recipients":[recipient]}]);
            }
            "override_person" => {
                s["meal_overrides"]["portions"] = json!([{"person":{"kind":"diner","data":{"diner_id":"dn_00000000000000000000000000000099"}},"preset":"regular","adjustment":{"numerator":"1","denominator":"1"}}])
            }
            _ => unreachable!(),
        }
        let parsed: JobResultV1 = decode(&bad);
        let rejected = validate_result_binding(&parsed, &req, &req.context_stamp);
        assert!(rejected.is_err(), "{label} was admitted");
    }
}
