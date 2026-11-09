//! Native calendar integration from explicitly SYNTHETIC food premises.
//! No recipe admission, measured cooking/cooling, product or browser authority.
use mstrchf_calendar::*;
use mstrchf_contracts::*;
use mstrchf_core::food::*;

#[path = "../../core/tests/support/original_component.rs"]
mod fixture;
mod support;

fn prepared_case(split_cooling: bool) -> (HistoricalApproval, ContinuationInputs) {
    prepared_timing_case(split_cooling,false)
}
fn prepared_timing_case(split_cooling: bool, earlier: bool) -> (HistoricalApproval, ContinuationInputs) {
    let mut basis = fixture::cooling_case();
    if earlier {
        let mut source=basis.context.recipes[0].clone();
        for phase in source.prep.iter_mut().filter(|phase|matches!(phase.prep_id,1|3|4)) {
            phase.earliest_offset_seconds-=300;phase.latest_offset_seconds-=300;
        }
        fixture::refresh_claims(&mut source);fixture::replace_source(&mut basis,source);
        for fixed in basis.schedule.layout.fixed_starts.iter_mut()
            .filter(|fixed|fixed.phase.occurrence.slot==basis.plan.selections[0].slot) {
            fixed.start=fixture::shifted(&fixed.start,-300);
        }
        fixture::canonicalize_schedule(&mut basis);fixture::exact_fixed_occupancy(&mut basis);fixture::assign_identities(&mut basis);
    }
    // Calendar continuation uses actual household/week requirements rather than
    // the portable core probe's deliberately empty destination-equipment input.
    basis.context.requirements = BoundedVec(
        mstrchf_core::availability::meal_requirements(
            &basis.context.household,
            &basis.context.weeks[0],
            &[],
        )
        .unwrap()
        .into(),
    );
    if split_cooling {
        let source_slot = basis.plan.selections[0].slot.clone();
        for route in &mut basis.schedule.layout.routes {
            if route.phase.occurrence.slot == source_slot && matches!(route.phase.prep_id, 3 | 4) {
                route.session = SessionKey {
                    anchor_slot: source_slot.clone(),
                    role: SessionRole::AdvancePrep,
                    ordinal: 1,
                };
            }
        }
        fixture::assign_identities(&mut basis);
    }
    let derived = fixture::derive(&basis)
        .expect("synthetic real core derivation")
        .plan;
    let approved = ApprovedPlan {
        approval_id: ApprovalId(format!("au_{:032x}", 0xf011)),
        week_id: derived.week_id.clone(),
        plan_revision: derived.plan_revision,
        derived_digest: derived.digest.clone(),
        context_stamp: derived.context_stamp.clone(),
        approved_at: Instant("2026-10-18T09:00:00Z".into()),
    };
    let order = ConfirmedOrder {
        order_id: OrderId(format!("or_{:032x}", 0xf011)),
        confirmation: OrderConfirmation {
            week_id: derived.week_id.clone(),
            approval_id: approved.approval_id.clone(),
            delivery_start: fixture::at(18, "10:00:00"),
            delivery_end: fixture::at(18, "11:00:00"),
            confirmed_at: Instant("2026-10-18T09:30:00Z".into()),
            source: OrderConfirmationSource::UserConfirmation {},
        },
        approved_plan: approved.clone(),
        ordered_recipe_refs: derived.recipe_refs.clone(),
    };
    let zero = || PayloadDigest("0".repeat(64));
    let mut products = ProductUseSnapshot {
        revision: Revision(1),
        digest: zero(),
        valid_until: None,
        evidence: vec![],
        bindings: vec![],
        facts: vec![],
    };
    products.digest = digest_product_uses(&products).unwrap();
    let mut current = ContinuationInputs {
        revision: Revision(1),
        digest: zero(),
        sources: vec![],
        requirements: LiveRequirements {
            revision: Revision(1),
            digest: zero(),
            meals: basis
                .context
                .requirements
                .iter()
                .filter(|meal| {
                    basis
                        .plan
                        .selections
                        .iter()
                        .any(|selection| selection.slot == meal.slot)
                })
                .cloned()
                .collect(),
            unconfirmed: vec![],
        },
        evidence: RelevantEvidenceSnapshot {
            revision: Revision(1),
            digest: zero(),
            recipes: basis
                .evidence
                .recipes
                .iter()
                .map(|e| LiveRecipeState::Admitted {
                    basis: e.clone(),
                    classes: basis
                        .evidence
                        .classes
                        .iter()
                        .find(|c| c.recipe == e.recipe)
                        .unwrap()
                        .clone(),
                    methods: basis
                        .evidence
                        .methods
                        .iter()
                        .filter(|m| m.recipe == e.recipe)
                        .cloned()
                        .collect(),
                    resources: basis
                        .evidence
                        .resources
                        .iter()
                        .filter(|r| r.recipe == e.recipe)
                        .cloned()
                        .collect(),
                })
                .collect(),
        },
        policy: LiveFoodPolicy {
            version: Revision(1),
            digest: zero(),
            general_seconds: 172800,
            rice_seconds: 86400,
            maximum_reheats: 1,
        },
        occupied: basis.schedule.occupied.clone(),
        feasibility: FeasibilityInputs {
            revision: Revision(1),
            digest: zero(),
            delivery: DeliveryInput::Confirmed {
                order: order.clone(),
            },
            products,
            confirmations: vec![],
            evaluated_at: Instant("2026-10-18T12:00:00Z".into()),
        },
    };
    let historical = HistoricalApproval {
        basis,
        derived,
        approved,
        order,
    };
    current.occupied.query.as_mut().unwrap().excluded_subject = Some(support::pin(&historical));
    for source in [
        LiveSource::Essentials,
        LiveSource::FoodPolicy,
        LiveSource::Occupancy,
        LiveSource::Order,
        LiveSource::ProductEvidence,
        LiveSource::RecipeEvidence,
        LiveSource::Requirements,
    ] {
        current.sources.push(LiveSourcePin {
            source,
            revision: Revision(1),
            digest: zero(),
            valid_until: None,
        });
    }
    support::repin_current(&mut current, &historical);
    let checked = verify_historical(&historical).expect("exact historical native proof");
    let report = assess_continuation(&checked, &current).unwrap();
    assert!(
        report.blocking.is_empty(),
        "synthetic current premise: {report:?}"
    );
    (historical, current)
}

#[test]
fn zero_offset_calendar_bytes_remain_stable() {
    let (historical, current) = prepared_case(true);
    let prepared = support::prepare(&historical, &current,
        &support::assemblies(&historical), &support::production_text(&historical)).unwrap();
    let drafts=prepared.drafts.iter().map(|draft|serde_json::json!({
        "logical_key":draft.logical_key,"start":draft.start,"end":draft.end,"title":draft.title,"body":draft.body,
        "contributors":draft.contributors.iter().map(|row|serde_json::json!({"owner":{"week":row.owner.week,"derived_digest":row.owner.derived_digest,"basis_digest":row.owner.basis_digest},"session_id":row.session_id})).collect::<Vec<_>>()
    })).collect::<Vec<_>>();
    assert_eq!(bytes_digest(&canonical_bytes(&drafts).unwrap()).0,
        "882ded60b64431be713b28ae0d83341893ce7dd0742f728e8d20c07b8219104c");
}

#[test]
fn earlier_cooking_calendar_distinguishes_original_endpoint_portioning_and_hot_service() {
    let (historical,current)=prepared_timing_case(true,true);
    let prepared=support::prepare(&historical,&current,
        &support::assemblies(&historical),&support::production_text(&historical)).unwrap();
    let cooking=prepared.drafts.iter().find(|draft|draft.logical_key.kind==CalendarEventKind::Cooking).unwrap();
    assert_eq!(cooking.end,fixture::at(19,"17:55:00"));
    assert!(cooking.body.0.contains("original cooking ends earlier"));
    assert!(cooking.body.0.contains("Remaining-portion handling/cooling scheduled from original cooking"));
    let cooling=prepared.drafts.iter().find(|draft|draft.logical_key.kind==CalendarEventKind::Prep).unwrap();
    assert_eq!(cooling.start,fixture::at(19,"17:55:00"));
    assert_eq!(cooling.end,fixture::at(19,"19:55:00"));
    assert!(cooling.body.0.contains("Portioning/cooling/put-away after original cooking"));
    assert!(cooling.body.0.contains("within 120 minutes of that cooking endpoint"));
    assert!(!prepared.drafts.iter().any(|draft|draft.body.0.contains("After-service cooling/put-away")
        ||draft.body.0.contains("Remaining-portion cooling scheduled after hot service")));
}

#[test]
fn separate_after_service_event_keeps_original_clock_and_all_source_prerequisites() {
    let (historical, current) = prepared_case(true);
    let before = historical.clone();
    let prepared = support::prepare(
        &historical,
        &current,
        &support::assemblies(&historical),
        &support::production_text(&historical),
    )
    .unwrap();
    assert_eq!(prepared.drafts.len(), 4); // delivery, cooking, cooling, cold service
    let cooling = prepared
        .drafts
        .iter()
        .find(|d| d.start == fixture::at(19, "18:00:00"))
        .unwrap();
    assert_eq!(cooling.end, fixture::at(19, "20:00:00"));
    assert_eq!(cooling.logical_key.kind, CalendarEventKind::Prep);
    assert!(cooling.body.0.contains("After-service cooling/put-away"));
    assert!(
        cooling
            .body
            .0
            .contains("Cool and refrigerate the remaining portions within 120 minutes of cooking")
    );
    assert!(
        cooling
            .body
            .0
            .contains("These times are planned; follow the recipe handling instructions")
    );
    assert!(
        cooling
            .body
            .0
            .contains("Hot meal service (cooling of remaining shares follows when scheduled)")
    );
    for step in historical.basis.context.recipes[0]
        .steps
        .iter()
        .filter(|s| matches!(s.step_id, 3 | 4))
    {
        assert_eq!(cooling.body.0.matches(&step.text.0).count(), 1);
    }
    let cooking = prepared
        .drafts
        .iter()
        .find(|d| d.logical_key.kind == CalendarEventKind::Cooking)
        .unwrap();
    assert_eq!(cooking.end, fixture::at(19, "18:00:00"));
    assert!(
        cooking
            .body
            .0
            .contains("Remaining-portion cooling scheduled after hot service")
    );
    assert!(
        !cooking.body.0.lines().any(|line| (line.contains("phase 3,")
            || line.contains("phase 4,"))
            && line.contains("required by"))
    );
    let cold = prepared
        .drafts
        .iter()
        .find(|d| d.start == fixture::at(20, "11:55:00"))
        .unwrap();
    for phase in [1, 3, 4] {
        assert!(
            cold.body.0.contains(&format!("phase {phase},")),
            "missing exact source phase {phase}"
        );
    }
    assert_eq!(
        cold.body
            .0
            .matches("Original storage clock — planned cooking completion")
            .count(),
        1
    );
    assert!(
        cold.body
            .0
            .contains("planned cooking completion: 2026-10-19 18:00:00")
    );
    assert_eq!(historical, before);
}

#[test]
fn missing_presented_cooling_instruction_refuses_without_a_partial_calendar() {
    let (historical, current) = prepared_case(false);
    let mut text = support::production_text(&historical);
    assert!(text[0].steps.iter().any(|step| step.prep_id == 4));
    text[0].steps.retain(|step| step.prep_id != 4);
    assert!(matches!(
        support::prepare(
            &historical,
            &current,
            &support::assemblies(&historical),
            &text
        ),
        Err(CalendarFailure::Invalid(_))
    ));
}
