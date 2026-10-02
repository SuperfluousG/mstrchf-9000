use mstrchf_contracts::*;
use serde_json::{Value, json};
fn request(name: &str) -> JobRequestV1 {
    let base =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../tests/fixtures/contracts");
    parse_document(&std::fs::read(base.join(name)).unwrap()).unwrap()
}
fn bundle(mut r: JobRequestV1, fragment: Value) -> OutboundBundle {
    let bytes = serde_json::to_vec(&fragment).unwrap();
    let id = ArtefactId("ar_00000000000000000000000000000001".into());
    r.allowed_artefacts = BoundedVec(vec![ArtefactRef {
        artefact_id: id.clone(),
        relative_name: Text(format!("input/{}.json", id.0)),
        digest: bytes_digest(&bytes),
        byte_length: bytes.len() as u32,
        media_type: ArtefactMediaType::Json,
    }]);
    r.input_digest = input_digest(&RequestMaterial::from(&r)).unwrap();
    OutboundBundle {
        request: r,
        artefacts: vec![AllowedArtefactBytes {
            artefact_id: id,
            bytes,
        }],
    }
}
fn policy(share: bool) -> PrivacyPolicy {
    PrivacyPolicy {
        revision: Revision(1),
        use_ai_features: true,
        include_household_requirements: share,
        confirmed_at: Nullable(None),
    }
}
#[test]
fn same_variant_scoped_fragment() {
    let r = request("job-request-personalised.json");
    let b = bundle(r.clone(), serde_json::to_value(&r.payload).unwrap());
    assert_eq!(validate_outbound(&b, &policy(true)), Ok(()));
}
#[test]
fn matching_hash_wrong_variant_refused() {
    let r = request("job-request-setup.json");
    let other = request("job-request-recipe-candidate.json");
    let b = bundle(r, serde_json::to_value(other.payload).unwrap());
    assert_eq!(
        validate_outbound(&b, &policy(true)).unwrap_err().code,
        ErrorCode::ScopeViolation
    );
}
#[test]
fn matching_hash_out_of_scope_slot_refused() {
    let r = request("job-request-personalised.json");
    let mut f = serde_json::to_value(&r.payload).unwrap();
    f["data"]["requirements"][0]["slot"]["week_id"] = json!("wk_00000000000000000000000000000009");
    let b = bundle(r, f);
    assert_eq!(
        validate_outbound(&b, &policy(true)).unwrap_err().code,
        ErrorCode::ScopeViolation
    );
}
#[test]
fn sharing_off_fragment_leak_refused() {
    let r = request("job-request-setup.json");
    let other = request("job-request-personalised.json");
    let b = bundle(r, serde_json::to_value(other.payload).unwrap());
    assert_eq!(
        validate_outbound(&b, &policy(false)).unwrap_err().code,
        ErrorCode::PrivacyDenied
    );
}
#[test]
fn unknown_fragment_field_refused() {
    let r = request("job-request-setup.json");
    let mut f = serde_json::to_value(&r.payload).unwrap();
    f["data"]["private_notes"] = json!("synthetic");
    let b = bundle(r, f);
    assert_eq!(
        validate_outbound(&b, &policy(true)).unwrap_err().code,
        ErrorCode::UnknownField
    );
}
#[test]
fn undeclared_fragment_refused() {
    let r = request("job-request-setup.json");
    let mut b = bundle(r.clone(), serde_json::to_value(&r.payload).unwrap());
    b.artefacts[0].artefact_id = ArtefactId("ar_00000000000000000000000000000002".into());
    assert_eq!(
        validate_outbound(&b, &policy(true)).unwrap_err().code,
        ErrorCode::MissingReference
    );
}
#[test]
fn missing_declared_fragment_refused() {
    let r = request("job-request-setup.json");
    let mut b = bundle(r.clone(), serde_json::to_value(&r.payload).unwrap());
    b.artefacts.clear();
    assert_eq!(
        validate_outbound(&b, &policy(true)).unwrap_err().code,
        ErrorCode::MissingReference
    );
}
#[test]
fn malformed_fragment_refused() {
    let r = request("job-request-setup.json");
    let mut b = bundle(r.clone(), serde_json::to_value(&r.payload).unwrap());
    b.artefacts[0].bytes = b"{".to_vec();
    b.request.allowed_artefacts.0[0].byte_length = 1;
    b.request.allowed_artefacts.0[0].digest = bytes_digest(b"{");
    b.request.input_digest = input_digest(&RequestMaterial::from(&b.request)).unwrap();
    assert_eq!(
        validate_outbound(&b, &policy(true)).unwrap_err().code,
        ErrorCode::MalformedJson
    );
}
#[test]
fn diet_exact_code() {
    assert!(
        parse_document::<HardRequirement>(br#"{"kind":"diet","data":{"code":"vegan"}}"#).is_ok()
    );
    assert_eq!(
        parse_document::<HardRequirement>(br#"{"kind":"diet","data":{}}"#)
            .unwrap_err()
            .code,
        ErrorCode::MissingField
    );
    assert_eq!(
        parse_document::<HardRequirement>(
            br#"{"kind":"diet","data":{"code":"vegan","extra":true}}"#
        )
        .unwrap_err()
        .code,
        ErrorCode::UnknownField
    );
}
#[test]
fn inactive_exact_fragment_and_both_privacy_gates() {
    let r = request("positive_inactive_request.json");
    let mut fragment = serde_json::to_value(&r.payload).unwrap();
    fragment["data"]["requirements"]
        .as_array_mut()
        .unwrap()
        .truncate(1);
    let b = bundle(r, fragment);
    validate_outbound(&b, &policy(true)).unwrap();
    assert_eq!(
        validate_outbound(&b, &policy(false)).unwrap_err().code,
        ErrorCode::PrivacyDenied
    );
    let mut ai_off = policy(true);
    ai_off.use_ai_features = false;
    assert_eq!(
        validate_outbound(&b, &ai_off).unwrap_err().code,
        ErrorCode::PrivacyDenied
    );
    let mut out_of_scope = b.clone();
    if let ActionScope::Plan { slots, .. } = &mut out_of_scope.request.scope {
        slots.0.remove(0);
    }
    out_of_scope.request.input_digest =
        input_digest(&RequestMaterial::from(&out_of_scope.request)).unwrap();
    assert_eq!(
        validate_outbound(&out_of_scope, &policy(true))
            .unwrap_err()
            .code,
        ErrorCode::ScopeViolation
    );
}
#[test]
fn rehashed_deadline_and_activity_changes_are_not_exact_fragments() {
    let active = request("job-request-complete-noop.json");
    for mode in 0..3 {
        let mut parent = active.clone();
        let mut fragment = serde_json::to_value(&active.payload).unwrap();
        let requirement = &mut fragment["data"]["requirements"][0];
        match mode {
            0 => {
                requirement["eat_by"]["local_time"] = json!("19:00:00");
                requirement["eat_by"]["instant"] = json!("2026-12-28T19:00:00Z");
            }
            1 => {
                requirement["enabled"] = json!(false);
                requirement["eat_by"] = Value::Null;
            }
            _ => {
                let JobPayload::PersonalisedPlan { requirements, .. } = &mut parent.payload else {
                    panic!("fixture premise")
                };
                requirements.0[0].enabled = false;
                requirements.0[0].eat_by = Nullable(None);
            }
        }
        // Both fragments have legal shape/semantics in isolation. Exact record
        // equality, not merely the slot or freshly recomputed hash, grants scope.
        parse_document::<JobPayload>(&serde_json::to_vec(&fragment).unwrap()).unwrap();
        let b = bundle(parent, fragment);
        assert_eq!(
            validate_outbound(&b, &policy(true)).unwrap_err().code,
            ErrorCode::ScopeViolation,
            "mode={mode}"
        );
    }
}
