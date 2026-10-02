#![allow(dead_code)]
use mstrchf_calendar::*;
use mstrchf_contracts::*;
use mstrchf_core::{food::*, portions::*, recipes::*};

pub fn pin(h: &HistoricalApproval) -> PlanOwnerPin {
    PlanOwnerPin {
        week: WeekRevision {
            week_id: h.derived.week_id.clone(),
            revision: h.derived.plan_revision,
        },
        derived_digest: h.derived.digest.clone(),
        basis_digest: saved_basis_digest(&h.basis).unwrap(),
    }
}
pub fn assemblies(h: &HistoricalApproval) -> Vec<AssemblyEvidence> {
    h.derived
        .prep_sessions
        .iter()
        .map(|s| {
            let member = SessionContribution {
                owner: pin(h),
                session_id: s.session_id.clone(),
            };
            AssemblyEvidence {
                root: member.clone(),
                contributors: vec![member],
                evidence: h.basis.evidence.recipes[0].admitted.clone(),
            }
        })
        .collect()
}
pub fn direct_text(h: &HistoricalApproval) -> Vec<OccurrenceText> {
    let r = &h.basis.context.recipes[0];
    let reviewed = assess_revision(
        r,
        &h.basis.evidence.recipes[0],
        &h.basis.evidence.classes[0],
    )
    .unwrap();
    h.basis
        .plan
        .selections
        .iter()
        .enumerate()
        .map(|(i, s)| {
            let (method, prep_id, scaled) = if i == 0 {
                (
                    MethodKind::Production,
                    1,
                    scale_recipe(&reviewed, PositiveRational::new(2, 1).unwrap()).unwrap(),
                )
            } else {
                (
                    MethodKind::Reuse,
                    2,
                    ScaledRecipe {
                        recipe: s.recipe.clone(),
                        multiplier: PositiveRational::new(1, 2).unwrap(),
                        requested_servings: PositiveRational::new(1, 1).unwrap(),
                        produced_servings: PositiveRational::new(1, 1).unwrap(),
                        unallocated_servings: Rational::new(0, 1).unwrap(),
                        discrete_batches: None,
                        ingredients: vec![],
                        components: vec![ComponentYield {
                            component_id: r.components[0].component_id.clone(),
                            quantity: Quantity {
                                amount: PositiveRational::new(200, 1).unwrap(),
                                unit: Unit::G,
                                form: IngredientForm::Cooked,
                            },
                        }],
                    },
                )
            };
            OccurrenceText {
                owner: pin(h),
                occurrence: OccurrenceKey {
                    slot: s.slot.clone(),
                    extra_id: None,
                    recipe: s.recipe.clone(),
                },
                method,
                scaled,
                steps: vec![PresentedStep {
                    prep_id,
                    step_id: prep_id,
                    text: r
                        .steps
                        .iter()
                        .find(|s| s.step_id == prep_id)
                        .unwrap()
                        .text
                        .clone(),
                }],
                evidence: h.basis.evidence.recipes[0].admitted.clone(),
            }
        })
        .collect()
}
pub fn prepare(
    h: &HistoricalApproval,
    c: &ContinuationInputs,
    a: &[AssemblyEvidence],
    t: &[OccurrenceText],
) -> Result<PreparedCalendar, CalendarFailure> {
    prepare_events(&CalendarInputs {
        subject: OwnerInput {
            historical: h,
            current: c,
        },
        linked_owners: &[],
        assemblies: a,
        instruction_text: t,
        evaluated_at: &c.feasibility.evaluated_at,
    })
}

pub fn production_text(h: &HistoricalApproval) -> Vec<OccurrenceText> {
    let mut rows = Vec::new();
    for s in h.basis.plan.selections.iter() {
        let base_amount = match s.batch_use {
            BatchUse::NewBatch {
                production_servings,
                ..
            } => production_servings,
            _ => PositiveRational::new(1, 1).unwrap(),
        };
        let mut entries = vec![(s.recipe.clone(), None, base_amount)];
        entries.extend(s.extras.iter().map(|e| {
            (
                e.recipe.clone(),
                Some(e.extra_id.clone()),
                e.recipients[0].servings,
            )
        }));
        for (reference, extra_id, amount) in entries {
            let r = h
                .basis
                .context
                .recipes
                .iter()
                .find(|r| r.recipe_id == reference.recipe_id)
                .unwrap();
            let b = h
                .basis
                .evidence
                .recipes
                .iter()
                .find(|b| b.recipe == reference)
                .unwrap();
            let c = h
                .basis
                .evidence
                .classes
                .iter()
                .find(|b| b.recipe == reference)
                .unwrap();
            let m = h
                .basis
                .evidence
                .methods
                .iter()
                .find(|m| m.recipe == reference && m.kind == MethodKind::Production)
                .unwrap();
            let steps = m
                .prep_ids
                .iter()
                .flat_map(|id| {
                    let p = r.prep.iter().find(|p| p.prep_id == *id).unwrap();
                    r.steps
                        .iter()
                        .filter(|step| p.step_ids.contains(&step.step_id))
                        .map(|step| PresentedStep {
                            prep_id: *id,
                            step_id: step.step_id,
                            text: step.text.clone(),
                        })
                })
                .collect();
            rows.push(OccurrenceText {
                owner: pin(h),
                occurrence: OccurrenceKey {
                    slot: s.slot.clone(),
                    extra_id,
                    recipe: reference,
                },
                method: MethodKind::Production,
                scaled: scale_recipe(&assess_revision(r, b, c).unwrap(), amount).unwrap(),
                steps,
                evidence: b.admitted.clone(),
            });
        }
    }
    rows
}
pub fn repin_current(c: &mut ContinuationInputs, h: &HistoricalApproval) {
    c.requirements.digest = live_requirements_digest(&c.requirements).unwrap();
    c.evidence.digest = relevant_evidence_digest(&c.evidence).unwrap();
    c.policy.digest = live_food_policy_digest(&c.policy).unwrap();
    c.occupied.digest = occupancy_snapshot_digest(&c.occupied).unwrap();
    c.feasibility.products.digest = digest_product_uses(&c.feasibility.products).unwrap();
    c.feasibility.digest = digest_feasibility_inputs(&c.feasibility).unwrap();
    c.sources[0].digest =
        essentials_source_digest(&c.sources[0], &c.feasibility.confirmations).unwrap();
    c.sources[1].digest = c.policy.digest.clone();
    c.sources[2].digest = c.occupied.digest.clone();
    c.sources[3].digest = order_source_digest(&c.sources[3], &h.order).unwrap();
    c.sources[4].digest = c.feasibility.products.digest.clone();
    c.sources[5].digest = c.evidence.digest.clone();
    c.sources[6].digest = c.requirements.digest.clone();
    c.digest = continuation_inputs_digest(c).unwrap();
}
