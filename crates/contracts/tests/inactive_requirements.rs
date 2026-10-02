use mstrchf_contracts::*;
use serde_json::{Value, json};

fn fixture(name: &str) -> Value {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../tests/fixtures/contracts")
        .join(name);
    serde_json::from_slice(&std::fs::read(path).unwrap()).unwrap()
}
fn parse<T: serde::de::DeserializeOwned + ValidateShape>(v: &Value) -> Result<T, DomainError> {
    parse_document(&serde_json::to_vec(v).unwrap())
}
fn inactive(r: &mut Value) {
    r["enabled"] = json!(false);
    r["eat_by"] = Value::Null;
    r["attendees"] = json!([]);
    r["shared_hard_requirements"] = json!([]);
}
#[test]
fn required_nullable_activity_pairs_and_missing_key() {
    let active = fixture("meal-requirements.json")["meal_requirements"].clone();
    assert_eq!(
        serde_json::to_value(parse::<MealRequirements>(&active).unwrap()).unwrap(),
        active
    );
    let mut disabled = active.clone();
    inactive(&mut disabled);
    let r = parse::<MealRequirements>(&disabled).unwrap();
    assert_eq!(serde_json::to_value(r).unwrap(), disabled);
    for mut contrary in [active.clone(), disabled.clone()] {
        contrary["enabled"] = json!(!contrary["enabled"].as_bool().unwrap());
        let e = parse::<MealRequirements>(&contrary).unwrap_err();
        assert_eq!(e.code, ErrorCode::InvalidValue);
        assert!(e.issues.iter().any(|i| i.path == "/eat_by"));
    }
    for mut absent in [active, disabled] {
        absent.as_object_mut().unwrap().remove("eat_by");
        assert_eq!(
            parse::<MealRequirements>(&absent).unwrap_err().code,
            ErrorCode::MissingField
        );
    }
}
#[test]
fn direct_entrypoints_reject_disabled_present_deadline() {
    let mut c: PlanningContext =
        parse(&fixture("ruling004-current--planning-context.json")).unwrap();
    c.weeks.0[0].day_enabled.0[0] = false;
    for r in &mut c.requirements.0 {
        if r.slot.week_id == c.weeks[0].range.week_id && r.slot.date == c.weeks[0].range.monday {
            r.enabled = false;
            r.attendees.0.clear();
            r.shared_hard_requirements.0.clear();
        }
    }
    let issues = validate_context(&c).unwrap_err();
    assert!(
        issues
            .iter()
            .any(|i| i.code == ErrorCode::InvalidValue && i.path.ends_with("eat_by"))
    );
}
#[test]
fn direct_request_rejects_disabled_present_deadline() {
    let mut r: JobRequestV1 =
        parse(&fixture("ruling004-current--job-request-personalised.json")).unwrap();
    if let JobPayload::PersonalisedPlan { requirements, .. } = &mut r.payload {
        requirements.0[0].enabled = false;
        requirements.0[0].attendees.0.clear();
        requirements.0[0].shared_hard_requirements.0.clear();
    } else {
        panic!("fixture premise");
    }
    r.input_digest = input_digest(&RequestMaterial::from(&r)).unwrap();
    assert_eq!(
        validate_request_shape(&r).unwrap_err().code,
        ErrorCode::InvalidValue
    );
}

#[test]
fn direct_context_rejects_malformed_nested_requirement() {
    let mut c: PlanningContext =
        parse(&fixture("ruling004-current--planning-context.json")).unwrap();
    c.requirements.0[0].attendees.0[0]
        .portion
        .adjustment
        .0
        .denominator
        .0 = 0;
    assert!(validate_context(&c).is_err());
}

#[test]
fn generic_schema_describes_nullable_shape_but_semantics_refuse_contrary_pairs() {
    let schema = schema::schema_for::<MealRequirementsDocumentV1>();
    let validator = jsonschema::draft202012::options().build(&schema).unwrap();
    for name in [
        "meal-requirements.json",
        "positive_inactive_requirement.json",
        "negative_activity_deadline_disabled_moment.json",
        "negative_activity_deadline_enabled_null.json",
    ] {
        let v = fixture(name);
        assert!(validator.is_valid(&v), "{name}");
        assert_eq!(
            parse::<MealRequirementsDocumentV1>(&v).is_ok(),
            !name.starts_with("negative")
        );
    }
    for enabled in ["false", "true"] {
        let v = fixture(&format!("missing_activity_deadline_{enabled}.json"));
        assert!(!validator.is_valid(&v));
    }
}
fn request_requirements(r: &mut JobRequestV1) -> &mut Vec<MealRequirements> {
    let JobPayload::PersonalisedPlan { requirements, .. } = &mut r.payload else {
        panic!("fixture premise")
    };
    &mut requirements.0
}
#[test]
fn direct_context_and_request_reject_both_contradictions_and_malformed_children() {
    for mode in 0..11 {
        let mut context: PlanningContext =
            parse(&fixture("ruling004-current--planning-context.json")).unwrap();
        let mut request: JobRequestV1 =
            parse(&fixture("ruling004-current--job-request-personalised.json")).unwrap();
        let mutate = |r: &mut MealRequirements| match mode {
            0 => r.enabled = false,
            1 => r.eat_by = Nullable(None),
            2 => r.eat_by.0.as_mut().unwrap().instant.0 = "not-an-instant".into(),
            3 => r.eat_by.0.as_mut().unwrap().instant.0 = "2026-02-30T18:00:00Z".into(),
            4 => r.eat_by.0.as_mut().unwrap().instant.0 = "2026-12-28T17:00:00Z".into(),
            5 => r.eat_by.0.as_mut().unwrap().local_date.0 = "2026-02-30".into(),
            6 => r.eat_by.0.as_mut().unwrap().local_time.0 = "25:00:00".into(),
            7 => r.eat_by.0.as_mut().unwrap().zone.0 = "Bad/Zone".into(),
            8 => r.attendees.0[0].portion.adjustment.0.denominator.0 = 0,
            9 => r.shared_hard_requirements.0.clear(),
            _ => r.preferred_hands_on_minutes = 1441,
        };
        mutate(&mut context.requirements.0[0]);
        mutate(&mut request_requirements(&mut request)[0]);
        request.input_digest = input_digest(&RequestMaterial::from(&request)).unwrap();
        assert!(
            context.requirements[0].validate_shape().is_err(),
            "shape mode={mode}"
        );
        assert!(validate_context(&context).is_err(), "context mode={mode}");
        assert!(
            validate_request_shape(&request).is_err(),
            "request mode={mode}"
        );
        assert!(parse::<PlanningContext>(&serde_json::to_value(&context).unwrap()).is_err());
        assert!(parse::<JobRequestV1>(&serde_json::to_value(&request).unwrap()).is_err());
    }
}
#[test]
fn mixed_context_activity_membership_and_all_inactive_closure() {
    let c: PlanningContext = parse(&fixture(
        "ruling004-current--positive_mixed_activity_context.json",
    ))
    .unwrap();
    validate_context(&c).unwrap();
    for mode in 0..7 {
        let mut bad = c.clone();
        match mode {
            0 => bad.weeks.0[0].day_enabled.0[0] = true,
            1 => bad.weeks.0[0].day_enabled.0[1] = false,
            2 => {
                bad.requirements.0.pop();
            }
            3 => bad.requirements.0[1] = bad.requirements[0].clone(),
            4 => {
                let r = bad.requirements.0.iter_mut().find(|r| r.enabled).unwrap();
                r.attendees.0[0].person = PersonRef::Guest {
                    occasion_id: OccasionId("oc_00000000000000000000000000000099".into()),
                    guest_id: GuestId("gu_00000000000000000000000000000099".into()),
                };
            }
            5 => bad.requirements.0[0].location = MealLocation::OnTheMove,
            _ => bad.weeks.0[0].day_enabled.0.clear(),
        }
        let issues = validate_context(&bad).unwrap_err();
        if mode < 2 || mode == 5 {
            assert_eq!(issues[0].code, ErrorCode::DomainConflict);
        }
    }
    let mut value = serde_json::to_value(&c).unwrap();
    for week in value["weeks"].as_array_mut().unwrap() {
        week["day_enabled"] = json!(vec![false; 7]);
    }
    for requirement in value["requirements"].as_array_mut().unwrap() {
        inactive(requirement);
    }
    let all: PlanningContext = parse(&value).unwrap();
    validate_context(&all).unwrap();
    assert_eq!(all.requirements.len(), all.weeks.len() * 21);
    for (w, original) in all.weeks.iter().zip(c.weeks.iter()) {
        assert!(active_slots(w).unwrap().is_empty());
        assert_eq!(w.plan, original.plan);
    }
    // Inactivity must not turn an unanswered guest restriction into confirmation.
    let mut unconfirmed = all.clone();
    let cell = &mut unconfirmed.weeks.0[0].cells.0[0];
    cell.occasion = Nullable(Some(Occasion {
        occasion_id: OccasionId("oc_00000000000000000000000000000099".into()),
        host_diner_ids: cell.attendee_ids.clone(),
        guests: BoundedVec(vec![Guest {
            guest_id: GuestId("gu_00000000000000000000000000000099".into()),
            age_band: AgeBand::Adult,
            restrictions: RestrictionDeclaration::Unconfirmed {},
            portion: unconfirmed.household.diners[0].portions.clone(),
        }]),
    }));
    assert_eq!(
        validate_context(&unconfirmed).unwrap_err()[0].code,
        ErrorCode::DomainConflict
    );
}
#[test]
fn independently_derived_null_bytes_and_digest_vectors() {
    for (name, pointer) in [
        ("requirement", ""),
        ("context", "/requirements/0"),
        ("request", "/payload/data/requirements/0"),
    ] {
        let vector = fixture(&format!("positive_inactive_{name}_canonical.json"));
        let value = &vector["input"];
        assert_eq!(
            canonical_bytes(value).unwrap(),
            vector["expected_canonical"].as_str().unwrap().as_bytes()
        );
        assert_eq!(
            canonical_digest(value).unwrap().0,
            vector["expected_sha256"].as_str().unwrap()
        );
        for variant in vector["variants"].as_array().unwrap() {
            let mut v = value.clone();
            let record = v.pointer_mut(pointer).unwrap();
            if variant["variant"] == "omitted" {
                record.as_object_mut().unwrap().remove("eat_by");
            } else {
                record["eat_by"] = variant["eat_by"].clone();
            }
            assert_eq!(
                canonical_digest(&v).unwrap().0,
                variant["expected_sha256"].as_str().unwrap()
            );
        }
    }
    let r: JobRequestV1 = parse(&fixture(
        "ruling004-current--positive_inactive_request.json",
    ))
    .unwrap();
    let vector = fixture("ruling004-current--positive_inactive_request_canonical.json");
    assert_eq!(
        canonical_bytes(&RequestMaterial::from(&r)).unwrap(),
        vector["expected_canonical"].as_str().unwrap().as_bytes()
    );
    assert_eq!(
        r.input_digest.0,
        vector["expected_sha256"].as_str().unwrap()
    );
    let mut result: JobResultV1 =
        parse(&fixture("ruling004-current--job-result-complete-noop.json")).unwrap();
    assert_eq!(
        validate_result_binding(&result, &r, &r.context_stamp)
            .unwrap_err()
            .code,
        ErrorCode::DigestMismatch
    );
    result.input_digest = r.input_digest.clone();
    validate_result_binding(&result, &r, &r.context_stamp).unwrap();
}
