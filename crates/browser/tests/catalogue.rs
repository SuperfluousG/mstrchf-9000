use mstrchf_browser::{catalogue::*, run_json};
use mstrchf_contracts::*;
use serde_json::{Value, json};

fn packet(recipe: RecipeRevision) -> NonAdmittedRecipePacket {
    NonAdmittedRecipePacket {
        schema_version: SchemaVersion,
        status: NonAdmittedStatus::NonAdmitted,
        recipe,
    }
}
fn bytes(manifest: &NonAdmittedRecipeManifest) -> Vec<u8> {
    canonical_bytes(manifest).unwrap()
}

#[test]
fn frozen_five_match_current_public_api_and_exact_canonical_bytes() {
    let view = legacy_engineering_v1().unwrap();
    assert_eq!(view.origin(), CatalogueOrigin::FrozenLegacyEngineeringV1);
    assert_eq!(view.declared_recipes().len(), 5);
    assert_eq!(
        view.pin().manifest_digest.0,
        LEGACY_ENGINEERING_V1_MANIFEST_DIGEST
    );
    let call = |request: Value| {
        let response: Value =
            serde_json::from_slice(&run_json(&serde_json::to_vec(&request).unwrap())).unwrap();
        assert_eq!(response["ok"], true, "{response}");
        response["value"].clone()
    };
    let now = "2026-10-12T12:00:00Z";
    let state = call(
        json!({"op":"initialize","household_id":"hh_00000000000000000000000000000004","diner_id":"dn_00000000000000000000000000000004","zone":"Europe/London","today":"2026-10-12","now":now,"demo":true}),
    );
    let board = call(json!({"op":"board","state":state,"now":now}));
    let entries = board["details"]["library"].as_array().unwrap();
    let engineering = entries
        .iter()
        .filter(|entry| entry["admission"] == "engineering")
        .collect::<Vec<_>>();
    assert_eq!(engineering.len(), 5);
    for entry in engineering {
        let reference: RecipeRef = serde_json::from_value(entry["reference"].clone()).unwrap();
        let live: RecipeRevision =
            parse_document(&serde_json::to_vec(&entry["recipe"]).unwrap()).unwrap();
        let frozen = view.resolve_recipe(&reference).unwrap();
        assert_eq!(
            canonical_bytes(frozen).unwrap(),
            canonical_bytes(&live).unwrap()
        );
        assert_eq!(recipe_digest(frozen).unwrap(), reference.digest);
    }
    view.verify_catalogue_pin(view.pin()).unwrap();
}

#[test]
fn exact_resolution_distinguishes_revisions_and_declared_digests() {
    let legacy = legacy_engineering_v1().unwrap();
    let first = legacy.declared_recipes()[0].clone();
    let mut second = first.clone();
    second.revision = Revision(2);
    second.title = Title("Synthetic second declaration revision; no food admission".into());
    second.ingredients[0].quantity.0.as_mut().unwrap().amount =
        PositiveRational::new(500, 1).unwrap();
    let manifest = non_admitted_manifest(
        Revision(2),
        vec![packet(second.clone()), packet(first.clone())],
    )
    .unwrap();
    let view = non_admitted_view(&bytes(&manifest)).unwrap();
    assert_eq!(view.origin(), CatalogueOrigin::NonAdmittedDeclarations);
    assert_eq!(
        view.resolve_packet(&manifest.catalogue.entries[0]).unwrap(),
        &first
    );
    assert_eq!(
        view.resolve_packet(&manifest.catalogue.entries[1]).unwrap(),
        &second
    );
    assert_ne!(
        manifest.catalogue.entries[0].recipe.digest,
        manifest.catalogue.entries[1].recipe.digest
    );
    let mut crossed = manifest.catalogue.entries[0].recipe.clone();
    crossed.digest = manifest.catalogue.entries[1].recipe.digest.clone();
    assert_eq!(
        view.resolve_recipe(&crossed).unwrap_err().code,
        ErrorCode::DigestMismatch
    );
    let mut unknown = crossed;
    unknown.revision = Revision(3);
    assert_eq!(
        view.resolve_recipe(&unknown).unwrap_err().code,
        ErrorCode::MissingReference
    );
    let mut wrong_packet = manifest.catalogue.entries[0].clone();
    wrong_packet.packet_digest = Digest("a".repeat(64));
    assert_eq!(
        view.resolve_packet(&wrong_packet).unwrap_err().code,
        ErrorCode::DigestMismatch
    );
    assert_eq!(
        legacy
            .resolve_packet(&manifest.catalogue.entries[0])
            .unwrap_err()
            .code,
        ErrorCode::DigestMismatch
    );
}

#[test]
fn tampered_manifests_bodies_and_known_catalogue_pins_are_refused() {
    let legacy = legacy_engineering_v1().unwrap();
    let manifest = non_admitted_manifest(
        Revision(1),
        vec![packet(legacy.declared_recipes()[0].clone())],
    )
    .unwrap();
    let mut wrong = manifest.clone();
    wrong.catalogue.manifest_digest = Digest("a".repeat(64));
    assert_eq!(
        non_admitted_view(&bytes(&wrong)).unwrap_err().code,
        ErrorCode::DigestMismatch
    );
    let mut wrong = manifest.clone();
    wrong.catalogue.entries[0].recipe.digest = RecipeDigest("b".repeat(64));
    assert_eq!(
        non_admitted_view(&bytes(&wrong)).unwrap_err().code,
        ErrorCode::DigestMismatch
    );
    let mut wrong = manifest;
    wrong.packets[0].recipe.title = Title("Changed body without a new pin".into());
    assert_eq!(
        non_admitted_view(&bytes(&wrong)).unwrap_err().code,
        ErrorCode::DigestMismatch
    );
    let mut pin = legacy.pin().clone();
    pin.entries[0].packet_digest = Digest("a".repeat(64));
    assert_eq!(
        legacy.verify_catalogue_pin(&pin).unwrap_err().code,
        ErrorCode::DigestMismatch
    );
    pin = legacy.pin().clone();
    pin.revision = Revision(2);
    assert_eq!(
        legacy.verify_catalogue_pin(&pin).unwrap_err().code,
        ErrorCode::ContextStale
    );
}

#[test]
fn duplicate_or_unordered_revision_sets_and_authority_transport_are_refused() {
    let legacy = legacy_engineering_v1().unwrap();
    let first = legacy.declared_recipes()[0].clone();
    let mut same_identity = first.clone();
    same_identity.title = Title("Conflicting bytes under the same ID and revision".into());
    assert_eq!(
        non_admitted_manifest(
            Revision(1),
            vec![packet(first.clone()), packet(same_identity)]
        )
        .unwrap_err()
        .code,
        ErrorCode::DuplicateKey
    );
    let mut manifest = non_admitted_manifest(
        Revision(1),
        vec![packet(first), packet(legacy.declared_recipes()[1].clone())],
    )
    .unwrap();
    manifest.packets.0.reverse();
    assert_eq!(
        non_admitted_view(&bytes(&manifest)).unwrap_err().code,
        ErrorCode::NoncanonicalValue
    );
    let mut supplied: Value = serde_json::to_value(
        non_admitted_manifest(
            Revision(1),
            vec![packet(legacy.declared_recipes()[0].clone())],
        )
        .unwrap(),
    )
    .unwrap();
    supplied["evidence_basis"] = json!({"admitted":[],"verified_asset_digests":[]});
    assert_eq!(
        non_admitted_view(&serde_json::to_vec(&supplied).unwrap())
            .unwrap_err()
            .code,
        ErrorCode::UnknownField
    );
    supplied.as_object_mut().unwrap().remove("evidence_basis");
    supplied["status"] = json!("admitted_real");
    assert!(non_admitted_view(&serde_json::to_vec(&supplied).unwrap()).is_err());
    supplied["status"] = json!("legacy_engineering_v1");
    assert!(non_admitted_view(&serde_json::to_vec(&supplied).unwrap()).is_err());
}

#[test]
fn declaration_manifest_has_exact_explicit_bounds() {
    let empty = non_admitted_manifest(Revision(1), vec![]).unwrap();
    assert!(
        non_admitted_view(&bytes(&empty))
            .unwrap()
            .declared_recipes()
            .is_empty()
    );
    let recipe = legacy_engineering_v1().unwrap().declared_recipes()[0].clone();
    let mut packets = Vec::new();
    for revision in 1..=513 {
        let mut recipe = recipe.clone();
        recipe.revision = Revision(revision);
        packets.push(packet(recipe));
    }
    assert_eq!(
        non_admitted_manifest(Revision(1), packets)
            .unwrap_err()
            .code,
        ErrorCode::LimitExceeded
    );
}
