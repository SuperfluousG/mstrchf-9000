//! F01 regression: every explicitly cited requirement source is relevant.
//! Synthetic native claims; no real dietary or storage policy is inferred.
use mstrchf_contracts::*;
use mstrchf_core::products::*;
mod shopping_matching_support;
use shopping_matching_support::*;

fn storage_requirement(storage: Fact<Text>, fields: Vec<SourceField>) -> MatchAssessment {
    let (saved, derived) = gf_base();
    let policy = policy();
    let mut read = ReadFixture::new();
    read.item_mut().storage = storage;
    let mut meaning = current_meaning(&saved, &derived, &read);
    let claim = meaning
        .requirement_claims
        .iter_mut()
        .find(|c| c.requirement == gf())
        .unwrap();
    claim.source_fields = fields;
    assert_eq!(claim.state, ClaimState::Supported);
    let mut admission = admission();
    read.refresh(&mut meaning, &mut admission);
    // Full producer preparation and source/meaning admission must succeed.
    assess(
        &saved,
        &derived,
        &policy,
        &read,
        Some(&meaning),
        Some(&admission),
        now(),
    )
    .unwrap()
}

fn all_fields() -> Vec<SourceField> {
    vec![
        SourceField::Ingredients,
        SourceField::Allergens,
        SourceField::Storage,
    ]
}
fn required_fields() -> Vec<SourceField> {
    vec![SourceField::Ingredients, SourceField::Allergens]
}
fn unknown_storage() -> Fact<Text> {
    Fact::Unknown {
        reason: UnknownReason::Missing,
    }
}
fn conflicting_storage() -> Fact<Text> {
    Fact::Conflicting {
        values: BoundedVec(vec![
            Text("synthetic storage A".into()),
            Text("synthetic storage B".into()),
        ]),
    }
}
fn known_storage() -> Fact<Text> {
    Fact::Known {
        value: Text("synthetic reviewed storage".into()),
    }
}
fn assert_unresolved_requirement(result: MatchAssessment, reason: MatchReason) {
    let MatchAssessment::Unresolved(issues) = result else {
        panic!("expected unresolved requirement, got {result:?}");
    };
    assert!(
        issues
            .iter()
            .any(|i| i.requirement == Some(gf()) && i.reason == reason)
    );
}

#[test]
fn cited_unknown_requirement_source_blocks_support() {
    assert_unresolved_requirement(
        storage_requirement(unknown_storage(), all_fields()),
        MatchReason::UnknownRequirement,
    );
}
#[test]
fn cited_conflicting_requirement_source_blocks_support() {
    assert_unresolved_requirement(
        storage_requirement(conflicting_storage(), all_fields()),
        MatchReason::ContradictoryRequirement,
    );
}
#[test]
fn cited_known_requirement_source_allows_support() {
    assert_eq!(
        outcome(&storage_requirement(known_storage(), all_fields())),
        "Suitable"
    );
}
#[test]
fn uncited_unrequired_unknown_source_does_not_block_support() {
    assert_eq!(
        outcome(&storage_requirement(unknown_storage(), required_fields())),
        "Suitable"
    );
}
#[test]
fn uncited_unrequired_conflicting_source_does_not_block_support() {
    assert_eq!(
        outcome(&storage_requirement(
            conflicting_storage(),
            required_fields()
        )),
        "Suitable"
    );
}
#[test]
fn omission_of_policy_required_field_still_blocks_support() {
    assert_unresolved_requirement(
        storage_requirement(
            known_storage(),
            vec![SourceField::Ingredients, SourceField::Storage],
        ),
        MatchReason::UnknownRequirement,
    );
}
