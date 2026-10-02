//! FN-R1-01: first service retains the original component's age and storage limits.
//! Synthetic reviewed premises; no claim about actual cooking or food safety.
//! Null-storage in-cap cases prove age-limit behaviour only. These native records
//! do not attest the fresh-service versus within-cap holding transition.
use mstrchf_contracts::*;
use mstrchf_core::{batches::validate_allocations, food::*, recipes::ComponentClass};

#[path = "../../../tests/fixtures/food/batches/f11_fixture.rs"]
mod f11;

// Adapt the independent FOOD-C-01 probe without changing its original evidence.
fn source_only(
    class: ComponentClass,
    day: u8,
    start: &str,
    end: &str,
    storage: Option<u32>,
    unheated: bool,
) -> SavedFoodBasis {
    let mut saved = f11::fixture();
    saved.plan.selections.0.truncate(1);
    saved.plan.batch_links.0.clear();
    saved.context.recipes.0.truncate(1);
    saved.context.stamp.recipe_refs.0.truncate(1);
    saved.evidence.recipes.truncate(1);
    saved.evidence.classes.truncate(1);
    let old_ref = saved.plan.selections[0].recipe.clone();
    saved.evidence.methods.retain(|m| m.recipe == old_ref);
    saved.evidence.resources.retain(|m| m.recipe == old_ref);
    let recipe = &mut saved.context.recipes[0];
    recipe.prep[0].earliest_offset_seconds = -604800;
    if unheated {
        recipe.components[0].production = Nullable(Some(ComponentProductionSpec::Unheated {
            first_handling_prep_id: 1,
        }));
    }
    if let Some(seconds) = storage {
        recipe.storage[0].max_chilled_seconds = seconds;
        // First service is still explicit zero, even with a zero source reheat cap.
        recipe.storage[0].max_reheats = 0;
    } else {
        recipe.storage.0.clear();
        recipe.components[0].storage_rule_ids.0.clear();
        recipe.capabilities.leftovers = false;
    }
    if class == ComponentClass::Unknown {
        recipe.capabilities.leftovers = false;
    }
    let new_ref = RecipeRef {
        recipe_id: old_ref.recipe_id,
        revision: old_ref.revision,
        digest: recipe_digest(recipe).unwrap(),
    };
    saved.plan.selections[0].recipe = new_ref.clone();
    for serving in &mut saved.plan.selections[0].servings.0 {
        serving.preset_ref.recipe = new_ref.clone();
    }
    saved.context.stamp.recipe_refs[0] = new_ref.clone();
    saved.context.weeks[0].plan = saved.plan.clone();
    saved.evidence.recipes[0].recipe = new_ref.clone();
    saved.evidence.classes[0].recipe = new_ref.clone();
    saved.evidence.classes[0].components[0].recipe = new_ref.clone();
    saved.evidence.classes[0].components[0].class = class;
    for method in &mut saved.evidence.methods {
        method.recipe = new_ref.clone();
        for count in &mut method.direct_reuse_counts {
            count.recipe = new_ref.clone();
        }
    }
    for resource in &mut saved.evidence.resources {
        resource.recipe = new_ref.clone();
    }
    saved.evidence.catalogue_digest = food_evidence_digest(&saved.evidence).unwrap();
    saved.identities.allocations.truncate(1);
    saved.identities.sessions.truncate(1);
    saved.identities.demand_lines.truncate(4);
    saved.identities.digest = food_identities_digest(&saved.identities).unwrap();
    saved.schedule.layout.routes.truncate(1);
    saved.schedule.layout.routes[0].phase.occurrence.recipe = new_ref.clone();
    saved.schedule.layout.fixed_starts.truncate(1);
    saved.schedule.layout.fixed_starts[0]
        .phase
        .occurrence
        .recipe = new_ref;
    saved.schedule.layout.fixed_starts[0].start = f11::at(day, start);
    saved.schedule.occupied.query.as_mut().unwrap().horizon = TimeInterval {
        start: f11::at(day, start),
        end: f11::at(day, end),
    };
    saved.schedule.occupied.digest = occupancy_snapshot_digest(&saved.schedule.occupied).unwrap();
    saved
}

#[allow(clippy::result_large_err)] // R3's complete by-value failure report.
fn derive(saved: &SavedFoodBasis) -> Result<FoodDerivation, FoodFailure> {
    derive_plan(
        &saved.plan,
        &saved.context,
        &FoodInputs {
            evidence: &saved.evidence,
            identities: &saved.identities,
            schedule: &saved.schedule,
            feasibility: &f11::feasibility(),
        },
    )
}

fn assert_age(saved: &SavedFoodBasis, seconds: i64) -> DerivedPlan {
    let d = derive(saved).unwrap().plan;
    assert_eq!(d.batches.len(), 1);
    assert_eq!(d.allocations.len(), 1);
    let component = &d.batches[0].components[0];
    let original = match &component.original_production {
        ProductionMoment::Cooked { completed_at } => completed_at,
        ProductionMoment::Unheated {
            preparation_started_at,
        } => preparation_started_at,
    };
    assert_eq!(d.allocations[0].target_slot, d.batches[0].source_slot);
    assert_eq!(d.allocations[0].planned_reheats, 0);
    assert_eq!(
        d.allocations[0].use_at.instant.timestamp_seconds() - original.instant.timestamp_seconds(),
        seconds
    );
    assert!(validate_allocations(&d.batches, &d.allocations).is_ok());
    d
}

fn assert_blocked(saved: &SavedFoodBasis) {
    let before = saved.clone();
    assert!(
        matches!(derive(saved), Err(FoodFailure::Blocked { conflicts, .. })
        if conflicts.iter().any(|c| c.code == ErrorCode::DomainConflict)),
        "the full facade must reject the first source's excessive original age"
    );
    assert_eq!(*saved, before);
}

#[test]
fn first_source_rice_inclusive_86400_and_rejected_86401() {
    assert_age(
        &source_only(
            ComponentClass::Rice,
            4,
            "17:50:00",
            "18:00:00",
            Some(172800),
            false,
        ),
        86400,
    );
    assert_blocked(&source_only(
        ComponentClass::Rice,
        4,
        "17:49:59",
        "17:59:59",
        Some(172800),
        false,
    ));
    // Preserve the reviewer's original 114600-second counterexample as well.
    assert_blocked(&source_only(
        ComponentClass::Rice,
        4,
        "10:00:00",
        "10:10:00",
        Some(172800),
        false,
    ));
}

#[test]
fn first_source_general_inclusive_172800_and_rejected_172801() {
    assert_age(
        &source_only(
            ComponentClass::General,
            3,
            "17:50:00",
            "18:00:00",
            Some(172800),
            false,
        ),
        172800,
    );
    assert_blocked(&source_only(
        ComponentClass::General,
        3,
        "17:49:59",
        "17:59:59",
        Some(172800),
        false,
    ));
    // Preserve the reviewer's original 287400-second counterexample as well.
    assert_blocked(&source_only(
        ComponentClass::General,
        2,
        "10:00:00",
        "10:10:00",
        Some(172800),
        false,
    ));
}

#[test]
fn null_storage_does_not_exempt_first_source_from_known_class_ceiling() {
    for (class, day, limit) in [
        (ComponentClass::Rice, 4, 86400),
        (ComponentClass::General, 3, 172800),
    ] {
        assert_age(
            &source_only(class, day, "17:50:00", "18:00:00", None, false),
            limit,
        );
        assert_blocked(&source_only(
            class, day, "17:49:59", "17:59:59", None, false,
        ));
    }
}

#[test]
fn unheated_source_class_age_starts_at_first_handling_not_completion() {
    assert_age(
        &source_only(ComponentClass::Rice, 4, "18:00:00", "18:10:00", None, true),
        86400,
    );
    assert_blocked(&source_only(
        ComponentClass::Rice,
        4,
        "17:59:59",
        "18:09:59",
        None,
        true,
    ));
}

#[test]
fn first_source_preserves_shorter_rule_inclusive_boundary() {
    assert_age(
        &source_only(
            ComponentClass::General,
            5,
            "16:50:00",
            "17:00:00",
            Some(3600),
            false,
        ),
        3600,
    );
    assert_blocked(&source_only(
        ComponentClass::General,
        5,
        "16:49:59",
        "16:59:59",
        Some(3600),
        false,
    ));
}

#[test]
fn first_source_cooked_immediate_fresh_only_remains_supported() {
    let immediate = source_only(
        ComponentClass::General,
        5,
        "17:50:00",
        "18:00:00",
        None,
        false,
    );
    let d = assert_age(&immediate, 0);
    assert!(d.batches[0].components[0].storage_rule.0.is_none());
}

#[test]
fn unheated_fresh_only_allows_positive_handling_duration() {
    let immediate = source_only(
        ComponentClass::General,
        5,
        "17:50:00",
        "18:00:00",
        None,
        true,
    );
    let d = assert_age(&immediate, 600);
    assert!(matches!(
        d.batches[0].components[0].original_production,
        ProductionMoment::Unheated { .. }
    ));
}

fn unclassified_fresh_sources() -> Vec<SavedFoodBasis> {
    let mut cases = vec![];
    for storage in [None, Some(172800)] {
        for absent_class in [false, true] {
            let mut saved = source_only(
                ComponentClass::Unknown,
                5,
                "17:50:00",
                "18:00:00",
                storage,
                false,
            );
            if absent_class {
                saved.evidence.classes[0].components.clear();
                saved.evidence.catalogue_digest = food_evidence_digest(&saved.evidence).unwrap();
            }
            cases.push(saved);
        }
    }
    cases
}

#[test]
fn unknown_or_absent_class_immediate_source_preserves_optional_storage_rule_support() {
    for saved in unclassified_fresh_sources() {
        assert_age(&saved, 0);
    }
}

#[test]
fn unknown_class_source_still_obeys_represented_shorter_rule() {
    assert_age(
        &source_only(
            ComponentClass::Unknown,
            5,
            "16:50:00",
            "17:00:00",
            Some(3600),
            false,
        ),
        3600,
    );
    assert_blocked(&source_only(
        ComponentClass::Unknown,
        5,
        "16:49:59",
        "16:59:59",
        Some(3600),
        false,
    ));
}

#[test]
fn supplied_source_output_checks_represented_storage() {
    let d = assert_age(
        &source_only(
            ComponentClass::General,
            5,
            "17:50:00",
            "18:00:00",
            Some(86400),
            false,
        ),
        0,
    );
    let mut batches = d.batches.clone();
    batches[0].components[0].original_production = ProductionMoment::Cooked {
        completed_at: f11::at(4, "18:00:00"),
    };
    assert!(validate_allocations(&batches, &d.allocations).is_ok());
    batches[0].components[0].original_production = ProductionMoment::Cooked {
        completed_at: f11::at(4, "17:59:59"),
    };
    assert!(validate_allocations(&batches, &d.allocations).is_err());
    batches[0].components[0].original_production = ProductionMoment::Cooked {
        completed_at: d.allocations[0].use_at.clone(),
    };
    assert!(validate_allocations(&batches, &d.allocations).is_ok());
}

#[test]
fn supplied_null_source_output_still_checks_universal_maximum_age() {
    let d = assert_age(
        &source_only(
            ComponentClass::General,
            5,
            "17:50:00",
            "18:00:00",
            None,
            false,
        ),
        0,
    );
    let mut batches = d.batches.clone();
    batches[0].components[0].original_production = ProductionMoment::Cooked {
        completed_at: f11::at(3, "18:00:00"),
    };
    assert!(validate_allocations(&batches, &d.allocations).is_ok());
    batches[0].components[0].original_production = ProductionMoment::Cooked {
        completed_at: f11::at(3, "17:59:59"),
    };
    assert!(validate_allocations(&batches, &d.allocations).is_err());
}

fn continuation_fixture(saved: SavedFoodBasis) -> (HistoricalApproval, ContinuationInputs) {
    let derived = derive(&saved).unwrap().plan;
    let approved = ApprovedPlan {
        approval_id: ApprovalId(format!("au_{:032x}", 401)),
        week_id: derived.week_id.clone(),
        plan_revision: derived.plan_revision,
        derived_digest: derived.digest.clone(),
        context_stamp: derived.context_stamp.clone(),
        approved_at: f11::at(3, "09:00:00").instant,
    };
    let order = ConfirmedOrder {
        order_id: OrderId(format!("or_{:032x}", 401)),
        confirmation: OrderConfirmation {
            week_id: approved.week_id.clone(),
            approval_id: approved.approval_id.clone(),
            delivery_start: f11::at(3, "10:00:00"),
            delivery_end: f11::at(3, "11:00:00"),
            confirmed_at: f11::at(3, "09:30:00").instant,
            source: OrderConfirmationSource::UserConfirmation {},
        },
        approved_plan: approved.clone(),
        ordered_recipe_refs: derived.recipe_refs.clone(),
    };
    let mut current = ContinuationInputs {
        revision: Revision(1),
        digest: PayloadDigest("0".repeat(64)),
        sources: vec![],
        requirements: LiveRequirements {
            revision: Revision(1),
            digest: PayloadDigest("0".repeat(64)),
            meals: saved
                .context
                .requirements
                .iter()
                .filter(|m| m.slot == saved.plan.selections[0].slot)
                .cloned()
                .collect(),
            unconfirmed: vec![],
        },
        evidence: RelevantEvidenceSnapshot {
            revision: Revision(1),
            digest: PayloadDigest("0".repeat(64)),
            recipes: vec![LiveRecipeState::Admitted {
                basis: saved.evidence.recipes[0].clone(),
                classes: saved.evidence.classes[0].clone(),
                methods: saved.evidence.methods.clone(),
                resources: saved.evidence.resources.clone(),
            }],
        },
        policy: LiveFoodPolicy {
            version: Revision(1),
            digest: PayloadDigest("0".repeat(64)),
            general_seconds: 172800,
            rice_seconds: 86400,
            maximum_reheats: 0,
        },
        occupied: saved.schedule.occupied.clone(),
        feasibility: f11::feasibility(),
    };
    current.occupied.query.as_mut().unwrap().excluded_subject = Some(PlanOwnerPin {
        week: WeekRevision {
            week_id: saved.plan.week_id.clone(),
            revision: saved.plan.revision,
        },
        derived_digest: derived.digest.clone(),
        basis_digest: saved_basis_digest(&saved).unwrap(),
    });
    current.feasibility.delivery = DeliveryInput::Confirmed {
        order: order.clone(),
    };
    current.feasibility.evaluated_at = f11::at(3, "12:00:00").instant;
    current.feasibility.digest = digest_feasibility_inputs(&current.feasibility).unwrap();
    current.requirements.digest = live_requirements_digest(&current.requirements).unwrap();
    current.evidence.digest = relevant_evidence_digest(&current.evidence).unwrap();
    current.policy.digest = live_food_policy_digest(&current.policy).unwrap();
    current.occupied.digest = occupancy_snapshot_digest(&current.occupied).unwrap();
    for (source, revision, digest) in [
        (
            LiveSource::Essentials,
            Revision(1),
            PayloadDigest("0".repeat(64)),
        ),
        (
            LiveSource::FoodPolicy,
            current.policy.version,
            current.policy.digest.clone(),
        ),
        (
            LiveSource::Occupancy,
            current.occupied.occupancy_epoch,
            current.occupied.digest.clone(),
        ),
        (
            LiveSource::Order,
            Revision(1),
            PayloadDigest("0".repeat(64)),
        ),
        (
            LiveSource::ProductEvidence,
            current.feasibility.products.revision,
            current.feasibility.products.digest.clone(),
        ),
        (
            LiveSource::RecipeEvidence,
            current.evidence.revision,
            current.evidence.digest.clone(),
        ),
        (
            LiveSource::Requirements,
            current.requirements.revision,
            current.requirements.digest.clone(),
        ),
    ] {
        current.sources.push(LiveSourcePin {
            source,
            revision,
            digest,
            valid_until: None,
        });
    }
    current.sources[0].digest =
        essentials_source_digest(&current.sources[0], &current.feasibility.confirmations).unwrap();
    current.sources[3].digest = order_source_digest(&current.sources[3], &order).unwrap();
    current.digest = continuation_inputs_digest(&current).unwrap();
    (
        HistoricalApproval {
            basis: saved,
            derived,
            approved,
            order,
        },
        current,
    )
}

fn assert_tighter_current_policy(class: ComponentClass, storage: Option<u32>) {
    let saved = source_only(class, 5, "16:50:00", "17:00:00", storage, false);
    let (historical, mut current) = continuation_fixture(saved);
    let before = historical.clone();
    let checked = verify_historical(&historical).unwrap();
    assert_eq!(historical.derived.allocations.len(), 1);
    assert_eq!(historical.derived.allocations[0].planned_reheats, 0);
    assert!(
        assess_continuation(&checked, &current)
            .unwrap()
            .blocking
            .is_empty()
    );
    current.policy.general_seconds = 3600;
    current.policy.rice_seconds = 3600;
    current.policy.digest = live_food_policy_digest(&current.policy).unwrap();
    current.sources[1].digest = current.policy.digest.clone();
    current.digest = continuation_inputs_digest(&current).unwrap();
    assert!(
        assess_continuation(&checked, &current)
            .unwrap()
            .blocking
            .is_empty()
    );
    current.policy.general_seconds = 3599;
    current.policy.rice_seconds = 3599;
    current.policy.digest = live_food_policy_digest(&current.policy).unwrap();
    current.sources[1].digest = current.policy.digest.clone();
    current.digest = continuation_inputs_digest(&current).unwrap();
    let report = assess_continuation(&checked, &current).unwrap();
    assert!(
        !report.blocking.is_empty(),
        "source-only history must honour a tighter current age limit"
    );
    assert!(
        report
            .other_checks
            .iter()
            .any(|c| c.code == ErrorCode::DomainConflict
                && c.slot_ids.0 == vec![historical.derived.batches[0].source_slot.clone()])
    );
    assert_eq!(historical, before);
}

#[test]
fn source_only_continuation_blocks_tighter_policy_without_mutating_history() {
    assert_tighter_current_policy(ComponentClass::General, Some(172800));
}

#[test]
fn null_source_only_continuation_still_checks_tighter_known_class_policy() {
    assert_tighter_current_policy(ComponentClass::General, None);
}

#[test]
fn unknown_class_source_continuation_obeys_current_general_ceiling() {
    for storage in [None, Some(172800)] {
        assert_tighter_current_policy(ComponentClass::Unknown, storage);
    }
}

#[test]
fn immediate_fresh_only_continuation_remains_supported() {
    for unheated in [false, true] {
        let saved = source_only(
            ComponentClass::General,
            5,
            "17:50:00",
            "18:00:00",
            None,
            unheated,
        );
        let (historical, current) = continuation_fixture(saved);
        let checked = verify_historical(&historical).unwrap();
        assert!(
            assess_continuation(&checked, &current)
                .unwrap()
                .blocking
                .is_empty()
        );
    }
}

#[test]
fn unknown_or_absent_class_immediate_source_continuation_remains_supported() {
    for saved in unclassified_fresh_sources() {
        let (historical, current) = continuation_fixture(saved);
        let checked = verify_historical(&historical).unwrap();
        assert!(
            assess_continuation(&checked, &current)
                .unwrap()
                .blocking
                .is_empty()
        );
    }
}
