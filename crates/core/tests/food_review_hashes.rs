#[cfg(test)]
#[path = "../../../tests/fixtures/food/native/support.rs"]
mod native;

#[cfg(test)]
mod tests {
    use super::native::{self, read, value};
    use mstrchf_contracts::*;
    use mstrchf_core::food::*;

    fn current() -> ContinuationInputs {
        read(&value("known-continuation-inputs"))
    }

    fn confirmations() -> Vec<EssentialsConfirmation> {
        let h = native::historical();
        let line = &h.derived.demand.lines[0];
        // The source hash checks confirmation-set structure, not demand membership.
        let mut rows: Vec<_> = (1..=3)
            .map(|i| EssentialsConfirmation {
                line_id: DemandLineId(format!("dl_{i:032x}")),
                choice: EssentialsChoice::HaveEnough,
                displayed_quantity: line.quantity.clone(),
                demand_digest: h.derived.demand.digest.clone(),
                confirmed_at: Instant("2026-12-27T12:00:00Z".into()),
            })
            .collect();
        rows.sort_by(|a, b| a.line_id.cmp(&b.line_id));
        assert_eq!(rows.len(), 3);
        rows
    }

    fn recipe_ref(state: &LiveRecipeState) -> &RecipeRef {
        match state {
            LiveRecipeState::Admitted { basis, .. } => &basis.recipe,
            LiveRecipeState::Withdrawn { recipe, .. } => recipe,
        }
    }

    #[test]
    fn checked_source_helpers_preserve_valid_expiry_and_confirmation_order() {
        let c = current();
        let h = native::historical();
        let values = confirmations();
        for expiry in [None, Some(Instant("2026-12-29T12:00:00Z".into()))] {
            let mut essentials = c.sources[0].clone();
            essentials.valid_until = expiry.clone();
            assert!(essentials_source_digest(&essentials, &values).is_ok());
            let mut order = c.sources[3].clone();
            order.valid_until = expiry;
            assert!(order_source_digest(&order, &h.order).is_ok());
        }
        assert!(
            food_inputs_digest(
                &h.derived.input_plan_digest,
                &h.basis.evidence,
                &h.basis.identities,
                &h.basis.schedule
            )
            .is_ok()
        );
    }

    #[test]
    fn source_confirmation_duplicates_precede_order_errors() {
        let c = current();
        let rows = confirmations();
        for indices in [[0, 1, 0], [1, 0, 1]] {
            let bad: Vec<_> = indices.into_iter().map(|i| rows[i].clone()).collect();
            assert_eq!(
                essentials_source_digest(&c.sources[0], &bad)
                    .unwrap_err()
                    .code,
                ErrorCode::InvalidValue
            );
        }
        let mut bad = rows;
        bad.swap(0, 1);
        assert_eq!(
            essentials_source_digest(&c.sources[0], &bad)
                .unwrap_err()
                .code,
            ErrorCode::NoncanonicalValue
        );
    }

    #[test]
    fn live_recipe_identity_spans_admitted_and_withdrawn_rows() {
        let mut snapshot = current().evidence;
        let admitted = snapshot.recipes[0].clone();
        let (mut recipe, evidence) = match &admitted {
            LiveRecipeState::Admitted { basis, .. } => {
                (basis.recipe.clone(), basis.admitted.clone())
            }
            _ => unreachable!(),
        };
        recipe.digest = RecipeDigest("f".repeat(64));
        let withdrawn = LiveRecipeState::Withdrawn { recipe, evidence };
        snapshot.recipes = vec![admitted, withdrawn];
        snapshot
            .recipes
            .sort_by(|a, b| recipe_cmp(recipe_ref(a), recipe_ref(b)));
        assert_eq!(
            relevant_evidence_digest(&snapshot).unwrap_err().code,
            ErrorCode::IdentityMismatch
        );
    }

    #[test]
    fn live_recipe_distinct_revisions_remain_supported_but_exact_duplicates_reject() {
        let mut snapshot = current().evidence;
        let (recipe, evidence) = match &snapshot.recipes[0] {
            LiveRecipeState::Admitted { basis, .. } => {
                (basis.recipe.clone(), basis.admitted.clone())
            }
            _ => unreachable!(),
        };
        let first = LiveRecipeState::Withdrawn {
            recipe: recipe.clone(),
            evidence: evidence.clone(),
        };
        let mut next = recipe;
        next.revision = Revision(next.revision.0 + 1);
        next.digest = RecipeDigest("f".repeat(64));
        snapshot.recipes = vec![
            first.clone(),
            LiveRecipeState::Withdrawn {
                recipe: next,
                evidence,
            },
        ];
        snapshot
            .recipes
            .sort_by(|a, b| recipe_cmp(recipe_ref(a), recipe_ref(b)));
        assert!(relevant_evidence_digest(&snapshot).is_ok());
        snapshot.recipes = vec![first.clone(), first];
        assert_eq!(
            relevant_evidence_digest(&snapshot).unwrap_err().code,
            ErrorCode::InvalidValue
        );
    }

    #[test]
    fn control_verified_history_and_literal_continuation() {
        let h = native::historical();
        let checked = verify_historical(&h).unwrap();
        let report = assess_continuation(&checked, &current()).unwrap();
        let expected: ContinuationReport = read(&value("known-continuation-report"));
        assert_eq!(report, expected);
        assert!(report.blocking.is_empty());
    }

    #[test]
    fn food_input_parent_rejects_false_evidence_child_digest() {
        let h = native::historical();
        let mut evidence = h.basis.evidence.clone();
        evidence.catalogue_digest = PayloadDigest("1".repeat(64));
        let result = food_inputs_digest(
            &h.derived.input_plan_digest,
            &evidence,
            &h.basis.identities,
            &h.basis.schedule,
        );
        assert!(
            result.is_err(),
            "false evidence child digest accepted: {result:?}"
        );
    }

    #[test]
    fn food_input_parent_rejects_false_identity_child_digest() {
        let h = native::historical();
        let mut identities = h.basis.identities.clone();
        identities.digest = PayloadDigest("2".repeat(64));
        let result = food_inputs_digest(
            &h.derived.input_plan_digest,
            &h.basis.evidence,
            &identities,
            &h.basis.schedule,
        );
        assert!(
            result.is_err(),
            "false identity child digest accepted: {result:?}"
        );
    }

    #[test]
    fn live_source_helpers_reject_malformed_expiry() {
        let c = current();
        let mut pin = c.sources[0].clone();
        pin.valid_until = Some(Instant("not-an-instant".into()));
        let result = essentials_source_digest(&pin, &c.feasibility.confirmations);
        assert!(
            result.is_err(),
            "malformed expiry accepted by essentials hash: {result:?}"
        );
    }

    #[test]
    fn order_source_helper_rejects_malformed_expiry() {
        let c = current();
        let h = native::historical();
        let mut pin = c.sources[3].clone();
        pin.valid_until = Some(Instant("not-an-instant".into()));
        let result = order_source_digest(&pin, &h.order);
        assert!(
            result.is_err(),
            "malformed expiry accepted by order hash: {result:?}"
        );
    }

    #[test]
    fn live_source_helper_rejects_duplicate_confirmations() {
        let c = current();
        let h = native::historical();
        let confirmation = EssentialsConfirmation {
            line_id: h.derived.demand.lines[0].line_id.clone(),
            choice: EssentialsChoice::HaveEnough,
            displayed_quantity: h.derived.demand.lines[0].quantity.clone(),
            demand_digest: h.derived.demand.digest.clone(),
            confirmed_at: Instant("2026-12-27T12:00:00Z".into()),
        };
        confirmation.validate_shape().unwrap();
        let result = essentials_source_digest(&c.sources[0], &[confirmation.clone(), confirmation]);
        assert!(
            result.is_err(),
            "duplicate confirmations accepted: {result:?}"
        );
    }

    #[test]
    fn current_recipe_snapshot_rejects_same_revision_contradictory_digest() {
        let c = current();
        let (recipe, evidence) = match &c.evidence.recipes[0] {
            LiveRecipeState::Admitted { basis, .. } => {
                (basis.recipe.clone(), basis.admitted.clone())
            }
            _ => panic!("control must be admitted"),
        };
        let mut other = recipe.clone();
        other.digest = RecipeDigest("f".repeat(64));
        let mut recipes = vec![
            LiveRecipeState::Withdrawn {
                recipe,
                evidence: evidence.clone(),
            },
            LiveRecipeState::Withdrawn {
                recipe: other,
                evidence,
            },
        ];
        recipes.sort_by(|a, b| {
            let LiveRecipeState::Withdrawn { recipe: a, .. } = a else {
                unreachable!()
            };
            let LiveRecipeState::Withdrawn { recipe: b, .. } = b else {
                unreachable!()
            };
            recipe_cmp(a, b)
        });
        let snapshot = RelevantEvidenceSnapshot {
            revision: Revision(1),
            digest: PayloadDigest("0".repeat(64)),
            recipes,
        };
        let result = relevant_evidence_digest(&snapshot);
        assert!(
            result.is_err(),
            "same recipe revision with conflicting digests accepted: {result:?}"
        );
    }

    #[test]
    fn complete_facade_defends_against_helper_validation_gaps() {
        let h = native::historical();
        let c = current();
        let mut basis = h.basis.clone();
        basis.evidence.catalogue_digest = PayloadDigest("1".repeat(64));
        assert!(
            derive_plan(
                &basis.plan,
                &basis.context,
                &FoodInputs {
                    evidence: &basis.evidence,
                    identities: &basis.identities,
                    schedule: &basis.schedule,
                    feasibility: &c.feasibility,
                }
            )
            .is_err()
        );
        let mut basis = h.basis.clone();
        basis.identities.digest = PayloadDigest("2".repeat(64));
        assert!(
            derive_plan(
                &basis.plan,
                &basis.context,
                &FoodInputs {
                    evidence: &basis.evidence,
                    identities: &basis.identities,
                    schedule: &basis.schedule,
                    feasibility: &c.feasibility,
                }
            )
            .is_err()
        );
        for index in [0, 3] {
            let mut bad = c.clone();
            bad.sources[index].valid_until = Some(Instant("not-an-instant".into()));
            assert!(continuation_inputs_digest(&bad).is_err());
        }
        let confirmation = EssentialsConfirmation {
            line_id: h.derived.demand.lines[0].line_id.clone(),
            choice: EssentialsChoice::HaveEnough,
            displayed_quantity: h.derived.demand.lines[0].quantity.clone(),
            demand_digest: h.derived.demand.digest.clone(),
            confirmed_at: Instant("2026-12-27T12:00:00Z".into()),
        };
        let mut bad = c.clone();
        bad.feasibility.confirmations = vec![confirmation.clone(), confirmation];
        assert!(digest_feasibility_inputs(&bad.feasibility).is_err());
        let mut bad = c.clone();
        let (mut recipe, evidence) = match &bad.evidence.recipes[0] {
            LiveRecipeState::Admitted { basis, .. } => {
                (basis.recipe.clone(), basis.admitted.clone())
            }
            _ => unreachable!(),
        };
        recipe.digest = RecipeDigest("f".repeat(64));
        bad.evidence
            .recipes
            .insert(1, LiveRecipeState::Withdrawn { recipe, evidence });
        // The corrected public hash boundary now rejects this material before
        // a contradictory source pin can be constructed for continuation.
        assert_eq!(
            relevant_evidence_digest(&bad.evidence).unwrap_err().code,
            ErrorCode::IdentityMismatch
        );
    }
}
