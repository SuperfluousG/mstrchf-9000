use mstrchf_contracts::*;
use serde_json::{Value, json};

fn fixture(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/contracts")
        .join(name);
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}

fn decode<T: serde::de::DeserializeOwned + ValidateShape>(value: &Value) -> T {
    parse_document(&serde_json::to_vec(value).unwrap()).unwrap()
}

fn bind_digest(request: &mut JobRequestV1) {
    request.input_digest = input_digest(&RequestMaterial::from(&*request)).unwrap();
}

fn fragment(request: &JobRequestV1, value: Value) -> Result<(), DomainError> {
    let mut request = request.clone();
    let bytes = serde_json::to_vec(&value).unwrap();
    let artefact_id = ArtefactId("ar_00000000000000000000000000000099".into());
    request.allowed_artefacts = BoundedVec(vec![ArtefactRef {
        artefact_id: artefact_id.clone(),
        relative_name: Text(format!("input/{}.json", artefact_id.0)),
        digest: bytes_digest(&bytes),
        byte_length: bytes.len() as u32,
        media_type: ArtefactMediaType::Json,
    }]);
    bind_digest(&mut request);
    validate_outbound(
        &OutboundBundle {
            request,
            artefacts: vec![AllowedArtefactBytes { artefact_id, bytes }],
        },
        &PrivacyPolicy {
            revision: Revision(1),
            use_ai_features: true,
            include_household_requirements: true,
            confirmed_at: Nullable(None),
        },
    )
}

fn payload(request: &JobRequestV1) -> Value {
    serde_json::to_value(&request.payload).unwrap()
}

fn scope_violation(request: &JobRequestV1, value: Value, label: &str) {
    let error = fragment(request, value).expect_err(label);
    assert_eq!(error.code, ErrorCode::ScopeViolation, "{label}: {error:?}");
}

#[test]
fn generic_catalogue_fragment_cannot_replace_or_expand_authorised_content() {
    let request: JobRequestV1 = decode(&fixture("job-request.json"));
    let original = payload(&request);
    fragment(&request, original.clone()).unwrap();

    let mut replaced = original.clone();
    replaced["data"]["public_recipes"][0]["recipe_id"] =
        json!("rc_00000000000000000000000000000002");
    scope_violation(&request, replaced, "recipe identity substitution");

    let mut expanded = original.clone();
    expanded["data"]["meal_types"] = json!(["breakfast"]);
    scope_violation(&request, expanded, "meal-type authority substitution");

    // Empty meal_types is a legal narrower fragment; the recipe record remains intact.
    let mut narrowed = original;
    narrowed["data"]["meal_types"] = json!([]);
    fragment(&request, narrowed).unwrap();
}

#[test]
fn recipe_candidate_fragment_cannot_add_drop_or_replace_adaptation_authority() {
    let request: JobRequestV1 = decode(&fixture("job-request-recipe-candidate.json"));
    let original = payload(&request);
    fragment(&request, original.clone()).unwrap();

    let mut newly_adapted = original.clone();
    newly_adapted["data"]["adaptation"] = json!({
        "requirements": [],
        "equipment": [],
        "portion_requests": []
    });
    scope_violation(&request, newly_adapted, "None adaptation changed to Some");

    let mut replaced = original.clone();
    replaced["data"]["source_id"] = json!("ev_00000000000000000000000000000002");
    scope_violation(&request, replaced, "source identity substitution");

    // A parent with an adaptation may narrow its equipment list, but may not erase
    // the adaptation record or acquire additional equipment.
    let mut adapted_parent = request.clone();
    if let JobPayload::RecipeCandidate { adaptation, .. } = &mut adapted_parent.payload {
        adaptation.0 = Some(RecipeAdaptation {
            requirements: BoundedVec(vec![]),
            equipment: BoundedVec(vec![EquipmentCode::Hob]),
            portion_requests: BoundedVec(vec![]),
        });
    } else {
        unreachable!();
    }
    bind_digest(&mut adapted_parent);
    let mut omitted = payload(&adapted_parent);
    omitted["data"]["adaptation"] = Value::Null;
    scope_violation(&adapted_parent, omitted, "Some adaptation weakened to None");

    let mut widened = payload(&adapted_parent);
    widened["data"]["adaptation"]["equipment"] = json!(["hob", "oven"]);
    scope_violation(&adapted_parent, widened, "adaptation equipment expanded");

    let mut narrowed = payload(&adapted_parent);
    narrowed["data"]["adaptation"]["equipment"] = json!([]);
    fragment(&adapted_parent, narrowed).unwrap();
}

#[test]
fn product_observation_fragment_cannot_add_products_or_retarget_read_scope() {
    let request: JobRequestV1 = decode(&fixture("job-request-product-observation.json"));
    let original = payload(&request);
    fragment(&request, original.clone()).unwrap();

    let mut expanded = original.clone();
    expanded["data"]["products"] = json!([{
        "retailer": "tesco",
        "product_id": "synthetic-product-1",
        "variant_id": null
    }]);
    scope_violation(&request, expanded, "product collection expanded");

    let mut retargeted = original;
    retargeted["data"]["read_scope"]["resource"]["data"]["key"]["basket_id"] =
        json!("bk_00000000000000000000000000000002");
    scope_violation(&request, retargeted, "basket read scope replaced");
}

#[test]
fn calendar_fragment_cannot_substitute_an_event_with_a_recomputed_digest() {
    let request: JobRequestV1 = decode(&fixture("job-request-calendar-publication.json"));
    let original = payload(&request);
    fragment(&request, original.clone()).unwrap();

    let mut substituted = original;
    let event = &mut substituted["data"]["batch"]["events"][0];
    event["title"] = json!("Different synthetic reminder");
    let mut material = event.clone();
    material
        .as_object_mut()
        .unwrap()
        .remove("event_payload_digest");
    material["schema_version"] = json!(1);
    event["event_payload_digest"] = json!(canonical_digest(&material).unwrap().0);
    scope_violation(
        &request,
        substituted,
        "event changed with a valid recomputed digest",
    );
}

#[test]
fn booking_fragment_cannot_expand_the_week_set() {
    let request: JobRequestV1 = decode(&fixture("job-request-booking-check.json"));
    let original = payload(&request);
    fragment(&request, original.clone()).unwrap();

    let mut expanded = original;
    expanded["data"]["week_ids"] = json!([
        "wk_00000000000000000000000000000001",
        "wk_00000000000000000000000000000002"
    ]);
    scope_violation(&request, expanded, "booking week set expanded");
}

#[test]
fn setup_fragment_cannot_replace_provider_or_capability() {
    let request: JobRequestV1 = decode(&fixture("job-request-setup.json"));
    let original = payload(&request);
    fragment(&request, original.clone()).unwrap();

    let mut provider = original.clone();
    provider["data"]["provider"] = json!("claude_cowork");
    scope_violation(&request, provider, "setup provider replaced");

    let mut capability = original;
    capability["data"]["requested_capabilities"] = json!(["browser_read"]);
    scope_violation(&request, capability, "setup capability replaced");

    // Setup can legally retain a subset when the parent requested multiple capabilities.
    let mut expanded_parent = request;
    if let JobPayload::SetupCheck {
        requested_capabilities,
        ..
    } = &mut expanded_parent.payload
    {
        requested_capabilities.0 = vec![Capability::BrowserRead, Capability::ScopedFileRead];
    } else {
        unreachable!();
    }
    expanded_parent.required_capabilities =
        BoundedVec(vec![Capability::BrowserRead, Capability::ScopedFileRead]);
    bind_digest(&mut expanded_parent);
    let narrowed = json!({
        "kind": "setup_check",
        "data": {
            "provider": "codex_local",
            "requested_capabilities": ["scoped_file_read"]
        }
    });
    fragment(&expanded_parent, narrowed).unwrap();
}
