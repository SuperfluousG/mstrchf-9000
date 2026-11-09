//! Rust JCS preparation/verification for the maker-reviewed R3 packet.
//! No household state, runtime catalogue or EvidenceBasis wire admission occurs.
use mstrchf_contracts::*;
use mstrchf_core::{eligibility::Eligibility, recipes::*};
use serde_json::{Value, json};
use std::path::PathBuf;

fn folder() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../../sites/mstrchf-9000/content/intake/porridge-personal-reviewed-v1")
}
fn read(name: &str) -> Value {
    serde_json::from_slice(&std::fs::read(folder().join(name)).unwrap()).unwrap()
}
fn prior(name: &str) -> Value {
    serde_json::from_slice(
        &std::fs::read(folder().join("../porridge-personal-v1").join(name)).unwrap(),
    )
    .unwrap()
}
fn finalized() -> (Value, Value, Value, RecipeRevision) {
    let mut packet = read("packet.json");
    let mut documents = read("evidence-documents.json");
    let registry = read("ingredient-registry.json");
    let captures = read("source-captures.json");
    let recipe: RecipeRevision =
        parse_document(&serde_json::to_vec(&packet["maker_recipe_revision"]).unwrap()).unwrap();
    let reference = RecipeRef {
        recipe_id: recipe.recipe_id.clone(),
        revision: recipe.revision,
        digest: recipe_digest(&recipe).unwrap(),
    };
    packet["recipe_reference"] = serde_json::to_value(&reference).unwrap();
    let policy = documents.as_array_mut().unwrap().last_mut().unwrap();
    assert_eq!(policy["evidence_id"], "ev_8000000000000000000000000000000b");
    policy["recipe_reference"] = serde_json::to_value(&reference).unwrap();
    policy["accepted_ingredient_registry_digest"] = json!(canonical_digest(&registry).unwrap().0);
    let pins: Vec<Value> = documents
        .as_array()
        .unwrap()
        .iter()
        .map(|document| {
            json!({
                "evidence_id":document["evidence_id"],"revision":document["revision"],
                "digest":canonical_digest(document).unwrap().0,
            })
        })
        .collect();
    packet["maker_evidence_pins"] = json!(pins);
    packet["prior_packet_canonical_digest"] =
        json!(canonical_digest(&prior("packet.json")).unwrap().0);
    packet["ingredient_registry_digest"] = json!(canonical_digest(&registry).unwrap().0);
    packet["source_captures_digest"] = json!(canonical_digest(&captures).unwrap().0);
    packet["evidence_documents_digest"] = json!(canonical_digest(&documents).unwrap().0);
    let digests = json!({
        "format":"mstrchf-rust-jcs-maker-packet-digests",
        "recipe_reference":reference,
        "packet_canonical_digest":canonical_digest(&packet).unwrap().0,
        "ingredient_registry_digest":canonical_digest(&registry).unwrap().0,
        "source_captures_digest":canonical_digest(&captures).unwrap().0,
        "evidence_documents_digest":canonical_digest(&documents).unwrap().0,
        "root_policy_evidence_id":"ev_8000000000000000000000000000000b",
        "root_policy_evidence_digest":pins.last().unwrap()["digest"],
        "derivation":"mstrchf_contracts::recipe_digest and canonical_digest, Rust serde_jcs",
        "household_runtime_admission":false,
    });
    (packet, documents, digests, recipe)
}
fn evidence_basis(packet: &Value, recipe: &RecipeRevision) -> EvidenceBasis {
    let pins = packet["maker_evidence_pins"]
        .as_array()
        .unwrap()
        .iter()
        .map(|pin| EvidencePin {
            evidence_id: serde_json::from_value(pin["evidence_id"].clone()).unwrap(),
            revision: serde_json::from_value(pin["revision"].clone()).unwrap(),
            digest: serde_json::from_value(pin["digest"].clone()).unwrap(),
        })
        .collect();
    EvidenceBasis::new(
        RecipeRef {
            recipe_id: recipe.recipe_id.clone(),
            revision: recipe.revision,
            digest: recipe_digest(recipe).unwrap(),
        },
        pins,
        vec![],
    )
    .unwrap()
}

#[test]
fn derive_or_verify_exact_r3_pins_in_rust_jcs() {
    let (packet, documents, digests, recipe) = finalized();
    if std::env::var("MSTRCHF_DERIVE_REVIEWED_PACKET").as_deref() == Ok("1") {
        // The destination is hard-coded to this new packet. Caller supplies no path.
        for (name, value) in [
            ("packet.json", &packet),
            ("evidence-documents.json", &documents),
            ("digests.json", &digests),
        ] {
            std::fs::write(
                folder().join(name),
                serde_json::to_vec_pretty(value).unwrap(),
            )
            .unwrap();
        }
    } else {
        assert_eq!(read("packet.json"), packet);
        assert_eq!(read("evidence-documents.json"), documents);
        assert_eq!(read("digests.json"), digests);
    }
    assert_eq!(recipe.revision, Revision(3));
    assert_ne!(
        packet["recipe_reference"],
        prior("packet.json")["recipe_reference"]
    );
    assert_eq!(packet["household_runtime_admission"], false);
    assert_eq!(packet["maker_evidence_pins"].as_array().unwrap().len(), 11);
    println!("{}", serde_json::to_string_pretty(&digests).unwrap());
}

#[test]
fn prior_evidence_and_capture_dates_survive_without_new_observation_claims() {
    let (packet, documents, _, recipe) = finalized();
    let original_docs = prior("evidence-documents.json");
    assert_eq!(
        &documents.as_array().unwrap()[..10],
        original_docs.as_array().unwrap().as_slice()
    );
    assert_eq!(read("source-captures.json"), prior("source-captures.json"));
    assert_eq!(
        recipe.source.retrieved_at,
        serde_json::from_value::<Instant>(
            prior("packet.json")["proposed_recipe_revision"]["source"]["retrieved_at"].clone()
        )
        .unwrap()
    );
    assert!(
        recipe
            .reviews
            .iter()
            .all(|r| r.actor == ReviewActor::Automated && r.kind != ReviewKind::CookTest)
    );
    assert_eq!(
        serde_json::to_value(&recipe.reviews).unwrap(),
        prior("packet.json")["proposed_recipe_revision"]["reviews"]
    );
    let notice = recipe
        .source
        .adaptation_notice
        .0
        .as_ref()
        .unwrap()
        .0
        .to_lowercase();
    assert!(
        !notice.contains("pending")
            && !notice.contains("not runtime")
            && !notice.contains("not-runtime")
            && !notice.contains("not admitted")
    );
    assert!(notice.contains("estimate") && notice.contains("not nutritional adequacy"));
    assert!(
        recipe.image.0.is_none()
            && recipe.nutrition.0.is_none()
            && recipe.storage.is_empty()
            && recipe.reused_inputs.is_empty()
    );
    assert_eq!(packet["human_cook_test_performed"], false);
    assert_eq!(packet["human_calibration_complete"], false);
}

#[test]
fn policy_document_binds_full_r3_reference_and_never_claims_cooking_or_transported_authority() {
    let (packet, documents, _, recipe) = finalized();
    let policy = documents.as_array().unwrap().last().unwrap();
    assert_eq!(policy["recipe_reference"], packet["recipe_reference"]);
    assert_eq!(policy["actor"], "automated");
    assert_eq!(
        policy["accepts"]["attention_reservation"]["estimated"],
        true
    );
    assert_eq!(
        policy["accepts"]["adult_regular"]["servings"],
        json!({"numerator":"1","denominator":"1"})
    );
    for key in [
        "human_cook_test",
        "human_calibration",
        "nutrition",
        "image",
        "storage_or_reuse",
        "hard_dietary_requirements_supported",
        "evidence_basis_from_request_permitted",
    ] {
        assert_eq!(policy["limits"][key], false);
    }
    let policy_id: EvidenceId = serde_json::from_value(policy["evidence_id"].clone()).unwrap();
    assert!(recipe.portion_presets[0].evidence_ids.contains(&policy_id));
    let mut changed = recipe.clone();
    changed.ingredients[0].quantity.0.as_mut().unwrap().amount =
        PositiveRational::new(90, 1).unwrap();
    assert_ne!(
        recipe_digest(&changed).unwrap().0,
        policy["recipe_reference"]["digest"]
    );
    let pin = packet["maker_evidence_pins"]
        .as_array()
        .unwrap()
        .last()
        .unwrap();
    let mut forged = policy.clone();
    forged["recipe_reference"]["revision"] = json!("2");
    assert_ne!(canonical_digest(&forged).unwrap().0, pin["digest"]);
    assert!(packet.get("evidence_basis").is_none());
    assert!(
        packet["compiled_loader_inputs"]["construction"]
            .as_str()
            .unwrap()
            .contains("Never deserialize EvidenceBasis")
    );
}

#[test]
fn maker_pins_support_only_unchanged_quantity_mechanics_and_unrestricted_source_scope() {
    let (packet, _, _, recipe) = finalized();
    let basis = evidence_basis(&packet, &recipe);
    let reference: RecipeRef = serde_json::from_value(packet["recipe_reference"].clone()).unwrap();
    let classes = ComponentClassView::new(reference.clone(), vec![]).unwrap();
    let empty = EvidenceBasis::new(reference, vec![], vec![]).unwrap();
    assert_eq!(
        assess_revision(&recipe, &empty, &classes).unwrap_err().code,
        ErrorCode::MissingReference
    );
    let reviewed = assess_revision(&recipe, &basis, &classes).unwrap();
    let scaled =
        mstrchf_core::portions::scale_recipe(&reviewed, PositiveRational::new(1, 1).unwrap())
            .unwrap();
    let amounts: Vec<_> = scaled
        .ingredients
        .iter()
        .map(|i| {
            (
                i.quantity.amount.0.numerator.0,
                i.quantity.unit,
                i.quantity.form,
            )
        })
        .collect();
    assert_eq!(
        amounts,
        vec![
            (45, Unit::G, IngredientForm::Dry),
            (180, Unit::Ml, IngredientForm::Raw),
            (180, Unit::Ml, IngredientForm::Raw)
        ]
    );
    assert!(
        !reviewed.assessment().capabilities.leftovers
            && !reviewed.assessment().capabilities.starter_distribution
    );
    assert!(
        mstrchf_core::portions::scale_recipe(&reviewed, PositiveRational::new(2, 1).unwrap())
            .is_err()
    );
    let mut meal = parse_document::<MealRequirementsDocumentV1>(include_bytes!(
        "../../../tests/fixtures/contracts/meal-requirements.json"
    ))
    .unwrap()
    .meal_requirements;
    meal.slot.meal_type = MealType::Breakfast;
    meal.attendees.0.truncate(1);
    meal.attendees[0].hard_requirements = BoundedVec(vec![]);
    meal.shared_hard_requirements = BoundedVec(vec![]);
    assert_eq!(
        mstrchf_core::eligibility::eligible(&reviewed, &meal).unwrap(),
        Eligibility::Eligible
    );
    let requirement = HardRequirement::Diet {
        code: DietCode::Vegetarian,
    };
    meal.attendees[0].hard_requirements = BoundedVec(vec![requirement.clone()]);
    meal.shared_hard_requirements = BoundedVec(vec![requirement]);
    assert!(matches!(
        mstrchf_core::eligibility::eligible(&reviewed, &meal).unwrap(),
        Eligibility::Unresolved(_)
    ));
}
