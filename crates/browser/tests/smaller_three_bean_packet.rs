//! Isolated authored packet/core checks, never runtime food admission or a
//! product, temperature, nutritional, human cook/portion calibration claim.
use mstrchf_contracts::*;
use mstrchf_core::{
    availability::meal_requirements,
    eligibility::{Eligibility, eligible},
    food::*,
    portions,
    recipes::*,
};
use serde_json::{Value, json};
use std::{fs, io::Write, path::PathBuf};
#[path = "../../../tests/fixtures/food/native/support.rs"]
mod native;
const RECIPE: &str = "rc_8000000000000000000000000000000a";
const COMPONENT: &str = "co_8000000000000000000000000000000a";
const AT: &str = "2026-10-13T09:12:24Z";
fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}
fn folder() -> PathBuf {
    repo().join("sites/mstrchf-9000/content/intake/three-bean-smaller-reviewed-v1")
}
fn read(name: &str) -> Value {
    serde_json::from_slice(&fs::read(folder().join(name)).unwrap()).unwrap()
}
fn prior(name: &str) -> Value {
    serde_json::from_slice(
        &fs::read(
            folder()
                .join("../three-bean-personal-reviewed-v1")
                .join(name),
        )
        .unwrap(),
    )
    .unwrap()
}
fn reference(recipe: &RecipeRevision) -> RecipeRef {
    RecipeRef {
        recipe_id: recipe.recipe_id.clone(),
        revision: recipe.revision,
        digest: recipe_digest(recipe).unwrap(),
    }
}
fn one() -> PositiveRational {
    PositiveRational::new(1, 1).unwrap()
}
fn blank() -> PayloadDigest {
    PayloadDigest("0".repeat(64))
}
fn pin(doc: &Value) -> EvidencePin {
    EvidencePin {
        evidence_id: parse_document(&canonical_bytes(&doc["evidence_id"]).unwrap()).unwrap(),
        revision: parse_document(&canonical_bytes(&doc["revision"]).unwrap()).unwrap(),
        digest: canonical_digest(doc).unwrap(),
    }
}
fn pin_value(pin: &EvidencePin) -> Value {
    json!({"evidence_id":pin.evidence_id,"revision":pin.revision,"digest":pin.digest})
}
fn basis(recipe: &RecipeRevision, docs: &Value) -> EvidenceBasis {
    EvidenceBasis::new(
        reference(recipe),
        docs.as_array().unwrap().iter().map(pin).collect(),
        vec![],
    )
    .unwrap()
}
fn classes(recipe: &RecipeRevision) -> ComponentClassView {
    ComponentClassView::new(reference(recipe), vec![]).unwrap()
}
fn assess(recipe: &RecipeRevision, docs: &Value) -> RecipeCapabilities {
    assess_revision(recipe, &basis(recipe, docs), &classes(recipe))
        .unwrap()
        .assessment()
        .capabilities
        .clone()
}
struct Formal {
    packet: Value,
    docs: Value,
    digests: Value,
    recipe: RecipeRevision,
}
fn derive() -> Formal {
    let input = read("authoring-input.json");
    assert_eq!(input["native_jcs_derived"], false);
    assert_eq!(input["capability_seed_unassessed"], true);
    let mut packet = input["packet"].clone();
    let mut docs = input["evidence_documents"].clone();
    let mut recipe: RecipeRevision =
        parse_document(&canonical_bytes(&packet["maker_recipe_revision"]).unwrap()).unwrap();
    recipe.capabilities = assess(&recipe, &docs);
    let rf = reference(&recipe);
    packet["maker_recipe_revision"] = json!(recipe);
    packet["recipe_reference"] = json!(rf);
    packet["compiled_loader_inputs"]["recipe_reference"] = json!(rf);
    packet["operational_scope"]["exact_recipe_reference"] = json!(rf);
    for doc in docs.as_array_mut().unwrap() {
        doc["recipe_reference"] = json!(rf);
    }
    let registry = read("ingredient-registry.json");
    let captures = read("source-captures.json");
    let registry_digest = canonical_digest(&registry).unwrap();
    let captures_digest = canonical_digest(&captures).unwrap();
    docs[0]["source_captures_digest"] = json!(captures_digest);
    docs[1]["ingredient_registry_digest"] = json!(registry_digest);
    docs[2]["registry_digest"] = json!(registry_digest);
    docs[3]["full_method"] = json!(recipe.steps);
    docs[5]["accepts"]["operational_scope"] = packet["operational_scope"].clone();
    docs[5]["accepted_ingredient_registry_digest"] = json!(registry_digest);
    docs[5]["accepted_source_captures_digest"] = json!(captures_digest);
    docs[5]["accepted_source_review_pin"] = pin_value(&pin(&docs[0]));
    docs[5]["accepted_quantity_review_pin"] = pin_value(&pin(&docs[1]));
    docs[5]["accepted_cold_handling_resource_pin"] = pin_value(&pin(&docs[3]));
    docs[5]["accepted_method_pin"] = pin_value(&pin(&docs[4]));
    packet["maker_evidence_pins"] = json!(
        docs.as_array()
            .unwrap()
            .iter()
            .map(|doc| pin_value(&pin(doc)))
            .collect::<Vec<_>>()
    );
    packet["ingredient_registry_digest"] = json!(registry_digest);
    packet["source_captures_digest"] = json!(captures_digest);
    packet["evidence_documents_digest"] = json!(canonical_digest(&docs).unwrap());
    packet["status"] =
        json!("maker_reviewed_native_jcs_ready_not_registry_or_household_runtime_admitted");
    packet["native_contracts_and_core_assessed"] = json!(true);
    assert_eq!(assess(&recipe, &docs), recipe.capabilities);
    let mut clocks = docs
        .as_array()
        .unwrap()
        .iter()
        .map(|doc| parse_document::<Instant>(&canonical_bytes(&doc["at"]).unwrap()).unwrap())
        .collect::<Vec<_>>();
    clocks.extend(
        recipe.reviews.iter().map(|review| {
            parse_document::<Instant>(&canonical_bytes(&review.at).unwrap()).unwrap()
        }),
    );
    let not_before = clocks.into_iter().max().unwrap();
    let digests = json!({"format":"mstrchf-rust-jcs-maker-packet-digests","recipe_reference":rf,"packet_canonical_digest":canonical_digest(&packet).unwrap(),"ingredient_registry_digest":registry_digest,"source_captures_digest":captures_digest,"evidence_documents_digest":canonical_digest(&docs).unwrap(),"source_review_evidence_id":pin(&docs[0]).evidence_id,"source_review_evidence_digest":pin(&docs[0]).digest,"quantity_review_evidence_id":pin(&docs[1]).evidence_id,"quantity_review_evidence_digest":pin(&docs[1]).digest,"ingredient_mapping_evidence_id":pin(&docs[2]).evidence_id,"ingredient_mapping_evidence_digest":pin(&docs[2]).digest,"resource_evidence_id":pin(&docs[3]).evidence_id,"resource_evidence_digest":pin(&docs[3]).digest,"method_evidence_id":pin(&docs[4]).evidence_id,"method_evidence_digest":pin(&docs[4]).digest,"root_policy_evidence_id":pin(&docs[5]).evidence_id,"root_policy_evidence_digest":pin(&docs[5]).digest,"all_evidence_pins":packet["maker_evidence_pins"],"not_before":not_before,"capabilities":recipe.capabilities,"derivation":"Rust mstrchf_contracts recipe_digest/canonical_digest (serde_jcs) and mstrchf_core assess_revision","household_runtime_admission":false,"registry_entry_present":false,"exact_formal_body_root_review_pending":true,"actual_product_or_physical_handling_observed":false});
    Formal {
        packet,
        docs,
        digests,
        recipe,
    }
}
fn preserved() {
    let preservation = read("previous-packet-preservation.json");
    assert_eq!(preservation["prior_packet_count"], 8);
    assert_eq!(preservation["files"].as_array().unwrap().len(), 100);
    for item in preservation["files"].as_array().unwrap() {
        let bytes = fs::read(repo().join(item["path"].as_str().unwrap())).unwrap();
        assert_eq!(json!(bytes.len()), item["bytes"]);
        assert_eq!(
            json!(bytes_digest(&bytes)),
            item["sha256_bytes"],
            "{}",
            item["path"]
        );
    }
    for name in ["ingredient-registry.json", "source-captures.json"] {
        assert_eq!(
            fs::read(folder().join(name)).unwrap(),
            fs::read(
                folder()
                    .join("../three-bean-personal-reviewed-v1")
                    .join(name)
            )
            .unwrap()
        );
    }
    for capture in read("source-captures.json")["captures"].as_array().unwrap() {
        let bytes = fs::read(folder().join(capture["retained_file"].as_str().unwrap())).unwrap();
        assert_eq!(json!(bytes.len()), capture["body_bytes"]);
        assert_eq!(json!(bytes_digest(&bytes)), capture["body_sha256"]);
        assert_eq!(
            bytes,
            fs::read(
                folder()
                    .join("../three-bean-personal-reviewed-v1")
                    .join(capture["retained_file"].as_str().unwrap())
            )
            .unwrap()
        );
    }
}
fn write_new_or_equal(name: &str, value: &Value) {
    let path = folder().join(name);
    if path.exists() {
        assert_eq!(
            read(name),
            *value,
            "Refuse replacing immutable output {name}"
        );
        return;
    }
    let mut file = fs::OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(path)
        .unwrap();
    file.write_all(&serde_json::to_vec_pretty(value).unwrap())
        .unwrap();
    file.write_all(b"\n").unwrap();
}
fn meal() -> MealRequirements {
    let mut m = parse_document::<MealRequirementsDocumentV1>(include_bytes!(
        "../../../tests/fixtures/contracts/meal-requirements.json"
    ))
    .unwrap()
    .meal_requirements;
    m.slot.meal_type = MealType::Lunch;
    m.attendees.0.truncate(1);
    m.attendees[0].age_band = AgeBand::Adult;
    m.attendees[0].hard_requirements = BoundedVec(vec![]);
    m.attendees[0].portion = PortionPreference {
        preset: PortionPresetName::Smaller,
        adjustment: one(),
        meal_overrides: BoundedVec(vec![]),
    };
    m.shared_hard_requirements = BoundedVec(vec![]);
    m.equipment = BoundedVec(vec![EquipmentCode::Fridge]);
    m.location = MealLocation::AtHome;
    m.mobility = Mobility {
        reheating: false,
        cold_storage: false,
        temperature: MobilityTemperature::Either,
    };
    m.enabled = true;
    m
}
fn empty() -> MealOverrides {
    MealOverrides {
        portions: BoundedVec(vec![]),
        eat_by: Nullable(None),
    }
}
fn exact_quantities(ingredients: &[mstrchf_core::portions::ScaledIngredient]) {
    for (suffix, n, d, unit, form) in [
        ("07", 60, 1, Unit::G, IngredientForm::Drained),
        ("08", 60, 1, Unit::G, IngredientForm::Drained),
        ("09", 60, 1, Unit::G, IngredientForm::Drained),
        ("0a", 30, 1, Unit::G, IngredientForm::Raw),
        ("0b", 15, 1, Unit::G, IngredientForm::Raw),
        ("0c", 45, 1, Unit::G, IngredientForm::Raw),
        ("0d", 15, 2, Unit::Ml, IngredientForm::Prepared),
        ("0e", 45, 4, Unit::Ml, IngredientForm::Prepared),
        ("0f", 15, 4, Unit::G, IngredientForm::Prepared),
    ] {
        let ingredient = ingredients
            .iter()
            .find(|ingredient| {
                ingredient.ingredient_id.0 == format!("ig_800000000000000000000000000000{suffix}")
            })
            .unwrap();
        assert_eq!(
            ingredient.quantity,
            Quantity {
                amount: PositiveRational::new(n, d).unwrap(),
                unit,
                form
            }
        );
    }
}

#[test]
fn derive_or_verify_smaller_packet_native_jcs_closure_and_every_prior_packet_byte() {
    preserved();
    let f = derive();
    let rf = reference(&f.recipe);
    assert_eq!(rf.recipe_id.0, RECIPE);
    assert_eq!(f.recipe.components[0].component_id.0, COMPONENT);
    assert_eq!(f.docs.as_array().unwrap().len(), 6);
    for (doc, suffix) in f
        .docs
        .as_array()
        .unwrap()
        .iter()
        .zip(["4e", "4f", "50", "51", "52", "53"])
    {
        assert_eq!(
            doc["evidence_id"],
            format!("ev_800000000000000000000000000000{suffix}")
        );
        assert_eq!(doc["actor"], "automated");
        assert_eq!(doc["at"], AT);
        assert_eq!(doc["recipe_reference"], json!(rf));
    }
    assert_eq!(f.digests["not_before"], AT);
    assert_eq!(
        f.recipe.source.retrieved_at,
        Instant("2026-10-12T20:23:53Z".into())
    );
    assert_eq!(f.docs[0]["fresh_network_retrieval"], false);
    assert_eq!(f.recipe.reviews.len(), 2);
    assert!(
        f.recipe
            .reviews
            .iter()
            .all(|review| review.actor == ReviewActor::Automated
                && review.kind != ReviewKind::CookTest
                && review.at.0 == AT)
    );
    assert_eq!(
        f.packet["operational_scope"]["exact_recipe_reference"],
        json!(rf)
    );
    assert_eq!(
        f.docs[5]["accepts"]["operational_scope"],
        f.packet["operational_scope"]
    );
    assert_eq!(f.docs[5]["accepts"]["exact_quantities"],json!(f.recipe.ingredients.iter().map(|ingredient|json!({"ingredient_id":ingredient.ingredient_id,"quantity":ingredient.quantity})).collect::<Vec<_>>()));
    assert_eq!(
        f.docs[5]["accepted_method_pin"],
        pin_value(&pin(&f.docs[4]))
    );
    assert_eq!(
        f.docs[5]["accepted_cold_handling_resource_pin"],
        pin_value(&pin(&f.docs[3]))
    );
    assert_eq!(f.docs[3]["full_method"], json!(f.recipe.steps));
    assert_eq!(f.docs[4]["full_recipe_steps"], json!(f.recipe.steps));
    assert!(f.recipe.capabilities.local_display && f.recipe.capabilities.shopping);
    assert!(!f.recipe.capabilities.leftovers && !f.recipe.capabilities.starter_distribution);
    for code in [
        CandidateIssueCode::MissingStorage,
        CandidateIssueCode::MissingImage,
    ] {
        assert!(
            f.recipe
                .capabilities
                .blocking_issues
                .iter()
                .any(|issue| issue.code == code)
        );
    }
    assert!(
        f.recipe.image.0.is_none()
            && f.recipe.nutrition.0.is_none()
            && f.recipe.storage.is_empty()
            && f.recipe.extras.is_empty()
            && f.recipe.reused_inputs.is_empty()
            && f.recipe.dietary_evidence.is_empty()
    );
    assert_eq!(
        f.recipe.text_rights.distribution,
        RightsDistribution::Permitted
    );
    assert_eq!(
        f.recipe.text_rights.licence_url.0.as_ref().unwrap().0,
        prior("packet.json")["maker_recipe_revision"]["text_rights"]["licence_url"]
            .as_str()
            .unwrap()
    );
    assert_eq!(f.packet["household_runtime_admission"], false);
    assert_eq!(f.packet["runtime_admission"], false);
    assert_eq!(f.packet["human_cook_test_performed"], false);
    assert_eq!(f.packet["human_calibration_complete"], false);
    assert_eq!(f.packet["exact_formal_body_root_review_pending"], true);
    let compiled = ReviewedRecipePacketRef {
        recipe: rf,
        packet_digest: canonical_digest(&f.packet).unwrap(),
    };
    let trusted = mstrchf_browser::maker_packets::known_packet(&compiled).unwrap();
    assert_eq!(trusted.reference(), &compiled);
    assert_eq!(trusted.not_before(), &Instant(AT.into()));
    assert_eq!(trusted.root_policy_evidence_id().0, "ev_80000000000000000000000000000053");
    if std::env::var("MSTRCHF_DERIVE_SMALLER_THREE_BEAN_PACKET").as_deref() == Ok("1") {
        for (name, value) in [
            ("packet.json", &f.packet),
            ("evidence-documents.json", &f.docs),
            ("digests.json", &f.digests),
        ] {
            write_new_or_equal(name, value);
        }
    }
    for (name, value) in [
        ("packet.json", &f.packet),
        ("evidence-documents.json", &f.docs),
        ("digests.json", &f.digests),
    ] {
        assert_eq!(read(name), *value);
    }
    preserved();
    println!("{}", serde_json::to_string(&f.digests).unwrap());
}

#[test]
fn fixed_smaller_inputs_and_native_portion_precedence_do_not_apply_three_quarters_twice() {
    let f = derive();
    let b = basis(&f.recipe, &f.docs);
    let c = classes(&f.recipe);
    let reviewed = assess_revision(&f.recipe, &b, &c).unwrap();
    let mut m = meal();
    assert_eq!(eligible(&reviewed, &m).unwrap(), Eligibility::Eligible);
    let check = |m: &MealRequirements, o: &MealOverrides| {
        let servings = portions::resolve_servings(&reviewed, m, o).unwrap();
        assert_eq!(servings.len(), 1);
        assert_eq!(servings[0].person, m.attendees[0].person);
        assert_eq!(servings[0].preset_ref.preset, PortionPresetName::Smaller);
        assert_eq!(servings[0].preset_ref.recipe, reference(&f.recipe));
        assert_eq!(servings[0].servings, one());
        let scaled = portions::scale_recipe(&reviewed, one()).unwrap();
        assert_eq!(scaled.multiplier, one());
        assert_eq!(scaled.produced_servings, one());
        assert_eq!(
            scaled.components[0].quantity,
            Quantity {
                amount: one(),
                unit: Unit::Each,
                form: IngredientForm::Prepared
            }
        );
        exact_quantities(&scaled.ingredients);
    };
    check(&m, &empty());
    m.attendees[0].portion.preset = PortionPresetName::Regular;
    m.attendees[0].portion.meal_overrides = BoundedVec(vec![MealPortionOverride {
        meal_type: MealType::Lunch,
        preset: PortionPresetName::Smaller,
        adjustment: one(),
    }]);
    check(&m, &empty());
    m.attendees[0].portion.meal_overrides = BoundedVec(vec![]);
    assert!(portions::resolve_servings(&reviewed, &m, &empty()).is_err());
    let explicit = MealOverrides {
        portions: BoundedVec(vec![PersonPortionOverride {
            person: m.attendees[0].person.clone(),
            preset: PortionPresetName::Smaller,
            adjustment: one(),
        }]),
        eat_by: Nullable(None),
    };
    check(&m, &explicit);
    for total in [
        PositiveRational::new(3, 4).unwrap(),
        PositiveRational::new(1, 2).unwrap(),
        PositiveRational::new(2, 1).unwrap(),
    ] {
        assert_eq!(
            portions::scale_recipe(&reviewed, total).unwrap_err().code,
            ErrorCode::UnsupportedCapability
        );
    }
    let original: RecipeRevision =
        parse_document(&canonical_bytes(&prior("packet.json")["maker_recipe_revision"]).unwrap())
            .unwrap();
    for (new, old) in f.recipe.ingredients.iter().zip(original.ingredients.iter()) {
        assert_eq!(new.ingredient_id, old.ingredient_id);
        let expected = PositiveRational(
            old.quantity
                .0
                .as_ref()
                .unwrap()
                .amount
                .0
                .checked_mul(&PositiveRational::new(3, 4).unwrap().0)
                .unwrap(),
        );
        assert_eq!(new.quantity.0.as_ref().unwrap().amount, expected);
        assert_eq!(
            new.quantity.0.as_ref().unwrap().unit,
            old.quantity.0.as_ref().unwrap().unit
        );
        assert_eq!(
            new.quantity.0.as_ref().unwrap().form,
            old.quantity.0.as_ref().unwrap().form
        );
    }
}

fn at(time: &str) -> ZonedMoment {
    ZonedMoment::new(
        LocalDate("2026-12-28".into()),
        LocalTime(time.into()),
        ZoneId("Europe/London".into()),
        Resolution::Unambiguous,
        Instant(format!("2026-12-28T{time}Z")),
    )
    .unwrap()
}
struct Probe {
    basis: SavedFoodBasis,
    feasibility: FeasibilityInputs,
}
impl Probe {
    fn derive(&self) -> Result<FoodDerivation, FoodFailure> {
        derive_plan(
            &self.basis.plan,
            &self.basis.context,
            &FoodInputs {
                evidence: &self.basis.evidence,
                identities: &self.basis.identities,
                schedule: &self.basis.schedule,
                feasibility: &self.feasibility,
            },
        )
    }
}
fn probe() -> Probe {
    let f = derive();
    let scaffold: Value = serde_json::from_slice(
        &fs::read(repo().join("tests/fixtures/food/native/v4/subject-saved-basis.input.json"))
            .unwrap(),
    )
    .unwrap();
    let mut saved: SavedFoodBasis = native::read(&scaffold["value"]);
    let rf = reference(&f.recipe);
    let first = saved.context.household.diners[0].diner_id.clone();
    saved.context.household.diners.0.truncate(1);
    saved.context.household.diners[0].portions = PortionPreference {
        preset: PortionPresetName::Smaller,
        adjustment: one(),
        meal_overrides: BoundedVec(vec![]),
    };
    saved.context.household.preferences.equipment = BoundedVec(vec![EquipmentCode::Fridge]);
    saved
        .context
        .household
        .preferences
        .meal_times
        .iter_mut()
        .find(|time| time.meal_type == MealType::Lunch)
        .unwrap()
        .time = LocalTime("12:30:00".into());
    let mut selected = saved.plan.selections[0].clone();
    let slot = selected.slot.clone();
    selected.recipe = rf.clone();
    selected.servings = BoundedVec(vec![DinerServing {
        person: PersonRef::Diner { diner_id: first },
        preset_ref: PortionPresetRef {
            recipe: rf.clone(),
            preset: PortionPresetName::Smaller,
            age_band: AgeBand::Adult,
            meal_type: MealType::Lunch,
        },
        servings: one(),
    }]);
    selected.extras = BoundedVec(vec![]);
    selected.batch_use = BatchUse::NewBatch {
        batch_id: BatchId("ba_8000000000000000000000000000000a".into()),
        production_servings: one(),
    };
    selected.meal_overrides = empty();
    saved.plan.selections = BoundedVec(vec![selected]);
    saved.plan.batch_links = BoundedVec(vec![]);
    saved.context.weeks[0].plan = saved.plan.clone();
    saved.context.recipes = BoundedVec(vec![f.recipe.clone()]);
    saved.context.stamp.recipe_refs = BoundedVec(vec![rf.clone()]);
    saved.context.requirements = BoundedVec(
        meal_requirements(&saved.context.household, &saved.context.weeks[0], &[])
            .unwrap()
            .to_vec(),
    );
    saved.evidence = FoodEvidence {
        catalogue_revision: saved.context.stamp.catalogue_revision,
        catalogue_digest: blank(),
        policy_version: Revision(1),
        recipes: vec![basis(&f.recipe, &f.docs)],
        classes: vec![classes(&f.recipe)],
        methods: vec![MethodPhases {
            portable_cold: None,
            recipe: rf.clone(),
            kind: MethodKind::Production,
            prep_ids: vec![1, 2],
            direct_reuse_counts: vec![],
            evidence_ids: vec![EvidenceId("ev_80000000000000000000000000000052".into())],
        }],
        resources: [1, 2]
            .into_iter()
            .map(|prep_id| PhaseResources {
                recipe: rf.clone(),
                prep_id,
                equipment: vec![EquipmentCode::Fridge],
                evidence_ids: vec![EvidenceId("ev_80000000000000000000000000000051".into())],
            })
            .collect(),
        essential_ingredient_ids: vec![],
    };
    saved.evidence.catalogue_digest = food_evidence_digest(&saved.evidence).unwrap();
    let session = SessionKey {
        anchor_slot: slot.clone(),
        role: SessionRole::AdvancePrep,
        ordinal: 1,
    };
    let routes = [1, 2]
        .into_iter()
        .map(|prep_id| PhaseRoute {
            phase: PhaseKey {
                occurrence: OccurrenceKey {
                    slot: slot.clone(),
                    extra_id: None,
                    recipe: rf.clone(),
                },
                prep_id,
            },
            session: session.clone(),
        })
        .collect();
    let mut occupied = OccupancySnapshot {
        query: Some(OccupancyQuery {
            horizon: TimeInterval {
                start: at("11:15:00"),
                end: at("12:30:00"),
            },
            excluded_subject: None,
        }),
        occupancy_epoch: Revision(1),
        owners: vec![],
        digest: blank(),
    };
    occupied.digest = occupancy_snapshot_digest(&occupied).unwrap();
    saved.schedule = ScheduleInputs {
        layout: ScheduleLayout {
            routes,
            fixed_starts: vec![],
            attachments: vec![],
        },
        occupied,
    };
    let keys = required_identity_keys(
        &saved.plan,
        &saved.context,
        &saved.evidence,
        &saved.schedule,
    )
    .unwrap();
    saved.identities.allocations = keys
        .allocations
        .into_iter()
        .enumerate()
        .map(|(i, key)| (key, AllocationId(format!("al_{:032x}", 0xa000 + i))))
        .collect();
    saved.identities.sessions = keys
        .sessions
        .into_iter()
        .enumerate()
        .map(|(i, key)| (key, PrepSessionId(format!("ps_{:032x}", 0xa000 + i))))
        .collect();
    saved.identities.demand_lines = keys
        .demand_lines
        .into_iter()
        .enumerate()
        .map(|(i, key)| (key, DemandLineId(format!("dl_{:032x}", 0xa000 + i))))
        .collect();
    saved.identities.digest = food_identities_digest(&saved.identities).unwrap();
    let mut products = ProductUseSnapshot {
        revision: Revision(1),
        digest: blank(),
        valid_until: None,
        evidence: vec![],
        bindings: vec![],
        facts: vec![],
    };
    products.digest = digest_product_uses(&products).unwrap();
    let mut feasibility = FeasibilityInputs {
        revision: Revision(1),
        digest: blank(),
        delivery: DeliveryInput::Unselected,
        products,
        confirmations: vec![],
        evaluated_at: Instant("2026-12-28T08:00:00Z".into()),
    };
    feasibility.digest = digest_feasibility_inputs(&feasibility).unwrap();
    Probe {
        basis: saved,
        feasibility,
    }
}

#[test]
fn two_phases_nine_once_only_demands_unheated_first_handling_and_arrival_guard() {
    let p = probe();
    let result = p.derive().unwrap();
    assert_eq!(result.plan.demand.lines.len(), 9);
    assert_eq!(result.plan.batches.len(), 1);
    assert_eq!(result.plan.allocations.len(), 1);
    assert_eq!(result.plan.prep_sessions.len(), 1);
    for ingredient in &p.basis.context.recipes[0].ingredients.0 {
        let line = result
            .plan
            .demand
            .lines
            .iter()
            .find(|line| line.ingredient_id == ingredient.ingredient_id)
            .unwrap();
        assert_eq!(
            line.quantity,
            ingredient.quantity.0.as_ref().unwrap().clone()
        );
        assert!(line.origin_extras.is_empty());
        assert_eq!(line.origin_batches.len(), 1);
    }
    let session = &result.plan.prep_sessions[0];
    assert_eq!(
        p.basis.schedule.layout.routes[0].session.role,
        SessionRole::AdvancePrep
    );
    assert_eq!(session.start, at("11:15:00"));
    assert_eq!(session.end, at("12:30:00"));
    assert_eq!(session.ingredient_needed_at, at("11:15:00"));
    assert_eq!(session.tasks.len(), 2);
    assert_eq!(
        session.tasks[0].elapsed,
        TimeInterval {
            start: at("11:15:00"),
            end: at("11:30:00")
        }
    );
    assert_eq!(
        session.tasks[0].active_intervals.0,
        vec![session.tasks[0].elapsed.clone()]
    );
    assert_eq!(
        session.tasks[1].elapsed,
        TimeInterval {
            start: at("11:30:00"),
            end: at("12:30:00")
        }
    );
    assert!(session.tasks[1].active_intervals.is_empty());
    assert!(session.attached_to.0.is_none());
    let component = &result.plan.batches[0].components[0];
    assert_eq!(
        component.original_production,
        ProductionMoment::Unheated {
            preparation_started_at: at("11:15:00")
        }
    );
    assert_eq!(
        component.produced,
        Quantity {
            amount: one(),
            unit: Unit::Each,
            form: IngredientForm::Prepared
        }
    );
    assert_eq!(component.produced, component.allocated);
    assert_eq!(component.unallocated.amount, Rational::new(0, 1).unwrap());
    assert!(
        mstrchf_core::prep::check_delivery(&at("11:15:00"), &session.ingredient_needed_at).is_ok()
    );
    assert_eq!(
        mstrchf_core::prep::check_delivery(&at("11:15:01"), &session.ingredient_needed_at)
            .unwrap_err()
            .issues[0]
            .path,
        "/delivery_end"
    );
    let f = derive();
    let b = basis(&f.recipe, &f.docs);
    let c = classes(&f.recipe);
    let reviewed = assess_revision(&f.recipe, &b, &c).unwrap();
    let mut no_fridge = meal();
    no_fridge.equipment.0.clear();
    assert!(!matches!(
        eligible(&reviewed, &no_fridge).unwrap(),
        Eligibility::Eligible
    ));
    let mut travel = meal();
    travel.location = MealLocation::OnTheMove;
    assert!(!matches!(
        eligible(&reviewed, &travel).unwrap(),
        Eligibility::Eligible
    ));
    assert!(
        result
            .feasibility
            .uses
            .iter()
            .any(|use_check| use_check.issues.contains(&UseIssue::UnknownProductDate))
    );
}

#[test]
fn missing_evidence_diets_and_false_capability_claims_refuse_future_scope_obligations_stay_explicit()
 {
    let f = derive();
    let b = basis(&f.recipe, &f.docs);
    let c = classes(&f.recipe);
    let reviewed = assess_revision(&f.recipe, &b, &c).unwrap();
    let empty = EvidenceBasis::new(reference(&f.recipe), vec![], vec![]).unwrap();
    assert_eq!(
        assess_revision(&f.recipe, &empty, &c).unwrap_err().code,
        ErrorCode::MissingReference
    );
    for suffix in ["4e", "4f", "51", "53"] {
        let mut missing = b.clone();
        missing.admitted.retain(|pin| {
            pin.evidence_id.0 != format!("ev_800000000000000000000000000000{suffix}")
        });
        assert_eq!(
            assess_revision(&f.recipe, &missing, &c).unwrap_err().code,
            ErrorCode::MissingReference
        );
    }
    let mut false_recipe = f.recipe.clone();
    false_recipe.capabilities.starter_distribution = true;
    assert_eq!(
        assess_revision(
            &false_recipe,
            &basis(&false_recipe, &f.docs),
            &classes(&false_recipe)
        )
        .unwrap_err()
        .code,
        ErrorCode::DomainConflict
    );
    for requirement in [
        HardRequirement::Diet {
            code: DietCode::Vegetarian,
        },
        HardRequirement::Diet {
            code: DietCode::Vegan,
        },
        HardRequirement::Diet {
            code: DietCode::GlutenFree,
        },
        HardRequirement::AllergenExclusion {
            code: AllergenCode::Milk,
        },
        HardRequirement::IngredientExclusion {
            ingredient_id: f.recipe.ingredients[0].ingredient_id.clone(),
        },
    ] {
        let mut m = meal();
        m.attendees[0].hard_requirements = BoundedVec(vec![requirement.clone()]);
        m.shared_hard_requirements = BoundedVec(vec![requirement]);
        assert!(!matches!(
            eligible(&reviewed, &m).unwrap(),
            Eligibility::Eligible
        ));
    }
    // Arithmetic/eligibility are not the future exact resident/count/all-layer
    // scope guard. These positive arithmetic controls are deliberately retained.
    let mut guest = meal();
    guest.attendees[0].person = PersonRef::Guest {
        occasion_id: OccasionId("oc_8000000000000000000000000000000a".into()),
        guest_id: GuestId("gu_8000000000000000000000000000000a".into()),
    };
    assert!(portions::resolve_servings(&reviewed, &guest, &empty_overrides()).is_ok());
    let mut compensated = meal();
    compensated.attendees[0].portion.adjustment = PositiveRational::new(2, 1).unwrap();
    let override_one = MealOverrides {
        portions: BoundedVec(vec![PersonPortionOverride {
            person: compensated.attendees[0].person.clone(),
            preset: PortionPresetName::Smaller,
            adjustment: one(),
        }]),
        eat_by: Nullable(None),
    };
    assert_eq!(
        portions::resolve_servings(&reviewed, &compensated, &override_one).unwrap()[0].servings,
        one()
    );
    assert_eq!(f.packet["operational_scope"]["temporary_guest_count"], 0);
    assert_eq!(f.packet["operational_scope"]["resident_count"], 1);
    assert_eq!(
        f.packet["operational_scope"]["every_applicable_adjustment"],
        json!(one())
    );
    assert_eq!(
        f.packet["operational_scope"]["hard_requirements"],
        json!([])
    );
    assert_eq!(
        f.packet["operational_scope"]["timing"]["not_a_guarantee"],
        true
    );
}
fn empty_overrides() -> MealOverrides {
    empty()
}
