use mstrchf_contracts::*;
use serde_json::{Value, json};

fn fixture(id: &str) -> Value {
    serde_json::from_slice::<Value>(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/contracts/ruling004-v4")
                .join(format!("{id}.input.json")),
        )
        .unwrap(),
    )
    .unwrap()["value"]
        .clone()
}
fn parse<T: serde::de::DeserializeOwned + ValidateShape>(v: &Value) -> Result<T, DomainError> {
    parse_document(&serde_json::to_vec(v).unwrap())
}

fn reuse_only_recipe() -> Value {
    let mut value = fixture("wire-addition-only-recipe");
    value["ingredients"] = json!([]);
    for component in value["components"].as_array_mut().unwrap() {
        component["ingredient_ids"] = json!([]);
    }
    for step in value["steps"].as_array_mut().unwrap() {
        step["ingredient_ids"] = json!([]);
    }
    for prep in value["prep"].as_array_mut().unwrap() {
        prep["ingredient_ids"] = json!([]);
    }
    value
}

#[test]
fn exact_original_component_can_be_served_without_new_ingredients() {
    let value = reuse_only_recipe();
    let recipe: RecipeRevision = parse(&value).unwrap();
    assert!(recipe.ingredients.is_empty());
    assert_eq!(recipe.reused_inputs.len(), 1);
    assert_eq!(serde_json::to_value(&recipe).unwrap(), value);
    assert_eq!(
        parse::<RecipeRevision>(&serde_json::to_value(recipe).unwrap())
            .unwrap()
            .reused_inputs,
        parse::<RecipeRevision>(&fixture("wire-addition-only-recipe"))
            .unwrap()
            .reused_inputs
    );
}

#[test]
fn empty_recipe_cannot_claim_a_shopping_capability() {
    let mut value = reuse_only_recipe();
    value["reused_inputs"] = json!([]);
    let error = parse::<RecipeRevision>(&value).unwrap_err();
    assert!(
        error
            .issues
            .iter()
            .any(|issue| issue.path == "/ingredients")
    );
    value["capabilities"]["shopping"] = json!(false);
    assert!(parse::<RecipeRevision>(&value).is_err());
}

#[test]
fn reuse_only_recipe_preserves_original_input_reference_guards() {
    for mutation in 0..5 {
        let mut value = reuse_only_recipe();
        match mutation {
            0 => {
                value["reused_inputs"][0]["source_recipe"] = json!({
                "recipe_id":value["recipe_id"], "revision":value["revision"], "digest":"a".repeat(64)})
            }
            1 => {
                value["reused_inputs"][0]["quantity_per_recipe_yield"]["amount"]["numerator"] =
                    json!("0")
            }
            2 => value["reused_inputs"][0]["step_ids"] = json!([256]),
            3 => value["reused_inputs"][0]["evidence_ids"] = json!([]),
            _ => value["reused_inputs"][0]["source_recipe"]["digest"] = json!("unbound"),
        }
        assert!(
            parse::<RecipeRevision>(&value).is_err(),
            "mutation {mutation}"
        );
    }
}

#[test]
fn generated_recipe_schema_requires_fresh_or_reused_food_and_keeps_closed_fields() {
    let schema = mstrchf_contracts::schema::schema_for::<RecipeRevision>();
    let validator = jsonschema::validator_for(&schema).unwrap();
    assert!(validator.is_valid(&fixture("recipe")));
    assert!(validator.is_valid(&fixture("wire-addition-only-recipe")));
    let mut value = reuse_only_recipe();
    assert!(validator.is_valid(&value));
    value["reused_inputs"] = json!([]);
    assert!(!validator.is_valid(&value));
    assert!(parse::<RecipeRevision>(&value).is_err());
    for mutation in 0..4 {
        let mut invalid = reuse_only_recipe();
        match mutation {
            0 => invalid["extra_field"] = json!(true),
            1 => {
                invalid.as_object_mut().unwrap().remove("title");
            }
            2 => {
                invalid["reused_inputs"][0]["quantity_per_recipe_yield"]["amount"]["numerator"] =
                    json!("0")
            }
            _ => invalid["ingredients"] = Value::Null,
        }
        assert!(!validator.is_valid(&invalid), "schema mutation {mutation}");
        assert!(
            parse::<RecipeRevision>(&invalid).is_err(),
            "native mutation {mutation}"
        );
    }
    // The same type-local constraint is present when a recipe is nested.
    let context_schema = mstrchf_contracts::schema::schema_for::<PlanningContext>();
    assert_eq!(
        context_schema["$defs"]["RecipeRevision"]["allOf"],
        schema["allOf"]
    );
}

#[test]
fn complete_independent_wire_vectors_decode_and_hash() {
    let source: RecipeRevision = parse(&fixture("recipe")).unwrap();
    assert_eq!(
        recipe_digest(&source).unwrap().0,
        "0fb683dff9176716c54b5757a2c299237825db24d1c104e5c7307b27ce5ead76"
    );
    let remix: RecipeRevision = parse(&fixture("wire-addition-only-recipe")).unwrap();
    assert_eq!(
        recipe_digest(&remix).unwrap().0,
        "ddcd57fd2454400a3ba8ff9bcd601a2d8e07b84d9492c8f9b068f2414b540090"
    );
    parse::<DerivedBatch>(&fixture("wire-derived-batch")).unwrap();
    parse::<PlanInputMaterial>(&fixture("subject-plan-input")).unwrap();
    let plan: DerivedPlan = parse(&fixture("wire-derived-plan")).unwrap();
    let material: DerivedMaterial = parse(&fixture("subject-derived")).unwrap();
    assert_eq!(DerivedMaterial::from(&plan), material);
    assert_eq!(
        derived_digest(&material).unwrap().0,
        "418aa9d41640f1bd7f4b4c2d202a48e7580eca1b0afb09eb6deb848fd95e55ff"
    );
}

#[test]
fn accepted_v4_historical_contexts_are_complete_and_omissions_refuse() {
    for (id, active) in [("prior-saved-basis", 1), ("subject-saved-basis", 2)] {
        // Exact accepted R3 fields: no compatibility stripping, repair or rehashing.
        let context = fixture(id)["context"].clone();
        let complete: PlanningContext = parse(&context).unwrap();
        assert_eq!(complete.requirements.len(), 21);
        assert_eq!(
            complete.requirements.iter().filter(|r| r.enabled).count(),
            active
        );
        assert!(
            complete
                .requirements
                .iter()
                .filter(|r| !r.enabled)
                .all(|r| r.eat_by.0.is_none())
        );
        validate_context(&complete).unwrap();
        let mut omitted = context;
        omitted["requirements"]
            .as_array_mut()
            .unwrap()
            .retain(|r| r["enabled"] == true);
        let error = parse::<PlanningContext>(&omitted).unwrap_err();
        assert!(error.issues.iter().any(
            |issue| issue.code == ErrorCode::MissingReference && issue.path == "/requirements"
        ));
        let omitted: PlanningContext = serde_json::from_value(omitted).unwrap();
        let issues = validate_context(&omitted).unwrap_err();
        assert!(issues.iter().any(
            |issue| issue.code == ErrorCode::MissingReference && issue.path == "/requirements"
        ));
    }
}

#[test]
fn production_commitment_is_required_positive_and_strict() {
    let mut batch =
        fixture("subject-plan-input")["week_plan"]["selections"][0]["batch_use"].clone();
    parse::<BatchUse>(&batch).unwrap();
    batch["data"]
        .as_object_mut()
        .unwrap()
        .remove("production_servings");
    assert!(parse::<BatchUse>(&batch).is_err());
    batch["data"]["production_servings"] = Value::Null;
    assert!(parse::<BatchUse>(&batch).is_err());
    batch["data"]["production_servings"] = json!({"numerator":"0","denominator":"1"});
    assert!(parse::<BatchUse>(&batch).is_err());
}

#[test]
fn recipe_new_keys_are_required_and_external_components_are_explicit() {
    let recipe = fixture("wire-addition-only-recipe");
    parse::<RecipeRevision>(&recipe).unwrap();
    let mut missing_reuse = recipe.clone();
    missing_reuse
        .as_object_mut()
        .unwrap()
        .remove("reused_inputs");
    assert!(parse::<RecipeRevision>(&missing_reuse).is_err());
    let mut bad = recipe.clone();
    bad["components"][0]
        .as_object_mut()
        .unwrap()
        .remove("production");
    assert!(parse::<RecipeRevision>(&bad).is_err());
    let mut unknown = recipe.clone();
    unknown["components"][0]["production"] = Value::Null;
    parse::<RecipeRevision>(&unknown).unwrap();
    let mut bad = recipe.clone();
    bad["reused_inputs"] = json!([]);
    assert!(parse::<RecipeRevision>(&bad).is_err());
    let mut bad = recipe.clone();
    bad["reused_inputs"][0]["planned_reheats"] = json!(2);
    assert!(parse::<RecipeRevision>(&bad).is_err());
    let mut bad = recipe.clone();
    bad["reused_inputs"][0]["step_ids"] = json!([256]);
    assert!(parse::<RecipeRevision>(&bad).is_err());
    let mut bad = recipe.clone();
    bad["components"][0]["production"]["data"]["completion_prep_id"] = json!(128);
    assert!(parse::<RecipeRevision>(&bad).is_err());
}

#[test]
fn component_clock_replaces_old_batch_clock_and_storage_null_is_explicit() {
    let batch = fixture("wire-derived-batch");
    parse::<DerivedBatch>(&batch).unwrap();
    let mut old = batch.clone();
    old["original_cook"] =
        batch["components"][0]["original_production"]["data"]["completed_at"].clone();
    assert!(parse::<DerivedBatch>(&old).is_err());
    for field in ["original_production", "storage_rule"] {
        let mut bad = batch.clone();
        bad["components"][0].as_object_mut().unwrap().remove(field);
        assert!(parse::<DerivedBatch>(&bad).is_err());
    }
    let mut fresh = batch.clone();
    fresh["components"][0]["storage_rule"] = Value::Null;
    parse::<DerivedBatch>(&fresh).unwrap();
    fresh["components"][0]["original_production"] = json!({"kind":"unheated","data":{"preparation_started_at":batch["components"][0]["original_production"]["data"]["completed_at"]}});
    parse::<DerivedBatch>(&fresh).unwrap();
}

#[test]
fn food_input_digest_is_required_and_hash_bound() {
    let plan = fixture("wire-derived-plan");
    let mut missing = plan.clone();
    missing
        .as_object_mut()
        .unwrap()
        .remove("food_inputs_digest");
    assert!(parse::<DerivedPlan>(&missing).is_err());
    let mut changed = plan.clone();
    changed["food_inputs_digest"] = json!("0".repeat(64));
    assert_eq!(
        parse::<DerivedPlan>(&changed).unwrap_err().code,
        ErrorCode::DigestMismatch
    );
    let mut material = fixture("subject-derived");
    material
        .as_object_mut()
        .unwrap()
        .remove("food_inputs_digest");
    assert!(parse::<DerivedMaterial>(&material).is_err());
}

#[test]
fn new_helper_bounds_identity_and_phase_membership_reject() {
    let recipe = fixture("wire-addition-only-recipe");
    for pointer in ["/reused_inputs/0/evidence_ids", "/reused_inputs/0/step_ids"] {
        let mut bad = recipe.clone();
        *bad.pointer_mut(pointer).unwrap() = json!([]);
        assert!(parse::<RecipeRevision>(&bad).is_err());
    }
    for field in [
        "planned_reheats",
        "quantity_per_recipe_yield",
        "source_recipe",
    ] {
        let mut bad = recipe.clone();
        bad["reused_inputs"][0]
            .as_object_mut()
            .unwrap()
            .remove(field);
        assert!(parse::<RecipeRevision>(&bad).is_err());
    }
    let mut duplicate = recipe.clone();
    let mut row = duplicate["reused_inputs"][0].clone();
    row["planned_reheats"] = json!(0);
    duplicate["reused_inputs"].as_array_mut().unwrap().push(row);
    assert_eq!(
        parse::<RecipeRevision>(&duplicate).unwrap_err().code,
        ErrorCode::InvalidValue
    );
    let mut foreign_evidence = recipe.clone();
    foreign_evidence["reused_inputs"][0]["evidence_ids"] =
        json!(["ev_00000000000000000000000000000999"]);
    assert!(parse::<RecipeRevision>(&foreign_evidence).is_err());
    let mut overlap = recipe.clone();
    overlap["reused_inputs"][0]["source_component_id"] =
        recipe["components"][0]["component_id"].clone();
    assert!(parse::<RecipeRevision>(&overlap).is_err());
    let mut unrelated_phase = recipe.clone();
    unrelated_phase["steps"][0]["component_ids"] =
        json!([recipe["reused_inputs"][0]["source_component_id"]]);
    assert!(parse::<RecipeRevision>(&unrelated_phase).is_err());
    for id in [0, 129] {
        assert!(
            parse::<ComponentProductionSpec>(
                &json!({"kind":"cooked","data":{"completion_prep_id":id}})
            )
            .is_err()
        );
        assert!(
            parse::<ComponentProductionSpec>(
                &json!({"kind":"unheated","data":{"first_handling_prep_id":id}})
            )
            .is_err()
        );
    }
    let mut cold = recipe.clone();
    cold["components"][0]["production"] =
        json!({"kind":"unheated","data":{"first_handling_prep_id":1}});
    cold["reused_inputs"][0]["planned_reheats"] = json!(0);
    parse::<RecipeRevision>(&cold).unwrap();
    let mut oversized = recipe;
    oversized["reused_inputs"] = json!(vec![oversized["reused_inputs"][0].clone(); 129]);
    assert_eq!(
        parse::<RecipeRevision>(&oversized).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
}

fn catalogue(recipes: Vec<Value>) -> Value {
    let mut context: Value = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/contracts/ruling004-current--planning-context.json"),
        )
        .unwrap(),
    )
    .unwrap();
    context["recipes"] = json!(recipes);
    rebind(&mut context);
    context
}
fn rebind(context: &mut Value) {
    context["stamp"]["recipe_refs"] = Value::Array(context["recipes"].as_array().unwrap().iter().map(|recipe| {
        json!({"recipe_id":recipe["recipe_id"],"revision":recipe["revision"],"digest":canonical_digest(recipe).unwrap().0})
    }).collect());
}

fn invalid_local_recipe_fields() -> [(&'static str, Value); 4] {
    [
        ("/reused_inputs/0/planned_reheats", json!(2)),
        (
            "/components/0/production/data/completion_prep_id",
            json!(128),
        ),
        (
            "/reused_inputs/0/evidence_ids",
            json!(["ev_00000000000000000000000000000999"]),
        ),
        ("/reused_inputs/0/step_ids", json!([256])),
    ]
}

#[test]
fn fcr_r1_direct_context_checks_recipe_children_before_cross_record_validation() {
    let good = catalogue(vec![
        fixture("recipe"),
        fixture("wire-addition-only-recipe"),
    ]);
    let typed: PlanningContext = parse(&good).unwrap();
    validate_context(&typed).unwrap();
    let mut accepted = Vec::new();
    for (path, value) in invalid_local_recipe_fields() {
        let mut bad = good.clone();
        *bad["recipes"][1].pointer_mut(path).unwrap() = value;
        rebind(&mut bad);
        let typed: PlanningContext = serde_json::from_value(bad.clone()).unwrap();
        for (recipe, reference) in typed.recipes.iter().zip(typed.stamp.recipe_refs.iter()) {
            assert_eq!(recipe_digest(recipe).unwrap(), reference.digest);
        }
        let child_errors = typed.recipes.validate_shape().unwrap_err();
        assert!(parse::<PlanningContext>(&bad).is_err(), "{path}");
        match validate_context(&typed) {
            Ok(()) => accepted.push(path),
            Err(errors) => assert_eq!(errors, child_errors, "{path}"),
        }
    }
    assert!(
        accepted.is_empty(),
        "direct context accepted invalid recipes: {accepted:?}"
    );
}

fn bind_catalogue_request(request: &mut Value) {
    request["context_stamp"]["recipe_refs"] = Value::Array(request["payload"]["data"]["public_recipes"].as_array().unwrap().iter().map(|recipe| {
        json!({"recipe_id":recipe["recipe_id"],"revision":recipe["revision"],"digest":canonical_digest(recipe).unwrap().0})
    }).collect());
    let typed: JobRequestV1 = serde_json::from_value(request.clone()).unwrap();
    request["input_digest"] = json!(input_digest(&RequestMaterial::from(&typed)).unwrap().0);
}

#[test]
fn fcr_r1_direct_catalogue_request_checks_recipe_children_before_hashing() {
    let mut good: Value = serde_json::from_slice(
        &std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/contracts/ruling004-current--job-request.json"),
        )
        .unwrap(),
    )
    .unwrap();
    good["payload"]["data"]["public_recipes"] =
        json!([fixture("recipe"), fixture("wire-addition-only-recipe")]);
    bind_catalogue_request(&mut good);
    let valid: JobRequestV1 = parse(&good).unwrap();
    validate_request_shape(&valid).unwrap();
    let mut accepted = Vec::new();
    for (path, value) in invalid_local_recipe_fields() {
        let mut bad = good.clone();
        *bad["payload"]["data"]["public_recipes"][1]
            .pointer_mut(path)
            .unwrap() = value;
        bind_catalogue_request(&mut bad);
        let typed: JobRequestV1 = serde_json::from_value(bad.clone()).unwrap();
        assert_eq!(
            typed.input_digest,
            input_digest(&RequestMaterial::from(&typed)).unwrap()
        );
        let JobPayload::GenericCatalogue { public_recipes, .. } = &typed.payload else {
            panic!("catalogue fixture")
        };
        for (recipe, reference) in public_recipes
            .iter()
            .zip(typed.context_stamp.recipe_refs.iter())
        {
            assert_eq!(recipe_digest(recipe).unwrap(), reference.digest);
        }
        let child_errors = public_recipes.validate_shape().unwrap_err();
        assert!(parse::<JobRequestV1>(&bad).is_err(), "{path}");
        match validate_request_shape(&typed) {
            Ok(()) => accepted.push(path),
            Err(error) => assert_eq!(error.issues.0, child_errors, "{path}"),
        }
    }
    assert!(
        accepted.is_empty(),
        "direct catalogue accepted invalid recipes: {accepted:?}"
    );
}

#[test]
fn catalogue_resolves_exact_sources_components_and_dependency_closure() {
    let base = catalogue(vec![
        fixture("recipe"),
        fixture("wire-addition-only-recipe"),
    ]);
    parse::<PlanningContext>(&base).unwrap();
    let mut missing = base.clone();
    missing["recipes"].as_array_mut().unwrap().remove(0);
    rebind(&mut missing);
    assert!(parse::<PlanningContext>(&missing).is_err());
    let mut digest = base.clone();
    digest["recipes"][1]["reused_inputs"][0]["source_recipe"]["digest"] = json!("0".repeat(64));
    rebind(&mut digest);
    assert!(parse::<PlanningContext>(&digest).is_err());
    let mut component = base.clone();
    component["recipes"][0] = serde_json::from_str(
        &serde_json::to_string(&component["recipes"][0])
            .unwrap()
            .replace(
                "co_0000000000000000000000000000012d",
                "co_00000000000000000000000000000999",
            ),
    )
    .unwrap();
    parse::<RecipeRevision>(&component["recipes"][0]).unwrap();
    component["recipes"][1]["reused_inputs"][0]["source_recipe"]["digest"] =
        json!(canonical_digest(&component["recipes"][0]).unwrap().0);
    rebind(&mut component);
    assert_eq!(
        parse::<PlanningContext>(&component).unwrap_err().code,
        ErrorCode::MissingReference
    );
    // Each recipe is locally valid with exact hashes. Only the direct-original
    // dependency rule rejects this three-recipe source -> remix -> remix chain.
    let mut third: Value = serde_json::from_str(
        &serde_json::to_string(&base["recipes"][1])
            .unwrap()
            .replace(
                "rc_0000000000000000000000000000012f",
                "rc_00000000000000000000000000000130",
            )
            .replace(
                "co_0000000000000000000000000000012f",
                "co_00000000000000000000000000000130",
            )
            .replace(
                "co_0000000000000000000000000000012d",
                "co_0000000000000000000000000000012f",
            ),
    )
    .unwrap();
    third["reused_inputs"][0]["source_recipe"] = base["stamp"]["recipe_refs"][1].clone();
    parse::<RecipeRevision>(&third).unwrap();
    let chained = catalogue(vec![
        fixture("recipe"),
        fixture("wire-addition-only-recipe"),
        third,
    ]);
    assert_eq!(
        parse::<PlanningContext>(&chained).unwrap_err().code,
        ErrorCode::DomainConflict
    );
}

#[test]
fn generated_schema_preserves_required_null_and_closed_union_shapes() {
    let recipe_schema =
        jsonschema::validator_for(&mstrchf_contracts::schema::schema_for::<RecipeRevision>())
            .unwrap();
    let recipe = fixture("wire-addition-only-recipe");
    assert!(recipe_schema.is_valid(&recipe));
    let mut fresh = recipe.clone();
    fresh["components"][0]["production"] = Value::Null;
    assert!(recipe_schema.is_valid(&fresh));
    fresh["components"][0]
        .as_object_mut()
        .unwrap()
        .remove("production");
    assert!(!recipe_schema.is_valid(&fresh));
    let mut missing = recipe;
    missing.as_object_mut().unwrap().remove("reused_inputs");
    assert!(!recipe_schema.is_valid(&missing));
    let batch_schema =
        jsonschema::validator_for(&mstrchf_contracts::schema::schema_for::<DerivedBatch>())
            .unwrap();
    let mut batch = fixture("wire-derived-batch");
    assert!(batch_schema.is_valid(&batch));
    batch["components"][0]["storage_rule"] = Value::Null;
    assert!(batch_schema.is_valid(&batch));
    batch["original_cook"] = Value::Null;
    assert!(!batch_schema.is_valid(&batch));
    assert_eq!(mstrchf_contracts::schema::ROOT_NAMES.len(), 19);
}

#[test]
fn addition_only_recipe_cannot_be_selected_as_an_original_batch() {
    let mut context = catalogue(vec![
        fixture("recipe"),
        fixture("wire-addition-only-recipe"),
    ]);
    for week in context["weeks"].as_array_mut().unwrap() {
        week["day_enabled"] = json!(vec![false; 7]);
    }
    for req in context["requirements"].as_array_mut().unwrap() {
        req["enabled"] = json!(false);
        req["eat_by"] = Value::Null;
    }
    let mut selection = fixture("subject-plan-input")["week_plan"]["selections"][0].clone();
    selection["slot"] = context["weeks"][0]["cells"][2]["slot"].clone();
    selection["servings"][0]["person"] =
        json!({"kind":"diner","data":{"diner_id":context["household"]["diners"][0]["diner_id"]}});
    context["weeks"][0]["plan"]["selections"] = json!([selection]);
    parse::<PlanningContext>(&context).unwrap();
    let remix = context["stamp"]["recipe_refs"][1].clone();
    context["weeks"][0]["plan"]["selections"][0]["recipe"] = remix.clone();
    context["weeks"][0]["plan"]["selections"][0]["servings"][0]["preset_ref"]["recipe"] = remix;
    assert!(parse::<PlanningContext>(&context).is_err());
}

#[test]
fn independent_preimages_and_original_fixture_bytes_remain_pinned() {
    let root = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../");
    let fixtures = root.join("tests/fixtures/contracts");
    let manifest: Value = serde_json::from_slice(
        &std::fs::read(fixtures.join("ruling004-v4/provenance.json")).unwrap(),
    )
    .unwrap();
    for row in manifest["vectors"].as_array().unwrap() {
        let bytes = std::fs::read(
            fixtures
                .join("ruling004-v4")
                .join(row["preimage"].as_str().unwrap()),
        )
        .unwrap();
        assert_eq!(bytes_digest(&bytes).0, row["sha256"]);
        let value = fixture(row["id"].as_str().unwrap());
        let mut material = value;
        if let Some(key) = row["omit_outer_field"].as_str() {
            material.as_object_mut().unwrap().remove(key);
        }
        if let Some(domain) = row["domain"].as_str() {
            material = json!({"domain":domain,"material":material});
        }
        assert_eq!(canonical_bytes(&material).unwrap(), bytes);
    }
    let adaptations: Value = serde_json::from_slice(
        &std::fs::read(fixtures.join("ruling004-adaptations.json")).unwrap(),
    )
    .unwrap();
    for row in adaptations["files"].as_array().unwrap() {
        for (field, pin) in [
            ("original", "original_lf_sha256"),
            ("current", "current_lf_sha256"),
        ] {
            let bytes = std::fs::read(root.join(row[field].as_str().unwrap())).unwrap();
            // Git normalises fixture line endings across hosts; the explicit LF pin
            // verifies content while provenance retains the original local byte pin.
            let lf = String::from_utf8(bytes).unwrap().replace("\r\n", "\n");
            assert_eq!(bytes_digest(lf.as_bytes()).0, row[pin], "{}", row[field]);
        }
    }
}
