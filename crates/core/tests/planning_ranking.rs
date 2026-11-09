use mstrchf_contracts::*;
use mstrchf_core::{ranking::*, recipes::*, selection::*};
use serde_json::Value;
use std::{fs, path::PathBuf};
fn load(path: &str) -> Value {
    serde_json::from_slice(
        &fs::read(
            PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                .join("../../tests/fixtures/households/p03cd/ranking")
                .join(path),
        )
        .unwrap(),
    )
    .unwrap()
}
macro_rules! field {
    ($v:expr,$k:expr) => {
        serde_json::from_value($v[$k].clone()).unwrap()
    };
}
fn pins(v: &Value) -> Vec<EvidencePin> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|p| EvidencePin {
            evidence_id: field!(p, "evidence_id"),
            revision: field!(p, "revision"),
            digest: field!(p, "digest"),
        })
        .collect()
}
fn evidence(v: &Value) -> RankingEvidence {
    RankingEvidence {
        revision: field!(v, "revision"),
        digest: field!(v, "digest"),
        catalogue_revision: field!(v, "catalogue_revision"),
        categories: if v["categories"].is_null() {
            None
        } else {
            let c = &v["categories"];
            Some(CategoryRegistry {
                revision: field!(c, "revision"),
                digest: field!(c, "digest"),
                keys: field!(c, "keys"),
            })
        },
        recipes: v["recipes"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| RecipeRankFacts {
                recipe: field!(r, "recipe"),
                nutrition_digest: if r["nutrition_digest"].is_null() {
                    None
                } else {
                    Some(Attested {
                        value: field!(&r["nutrition_digest"], "value"),
                        pins: pins(&r["nutrition_digest"]["pins"]),
                    })
                },
                categories: if r["categories"].is_null() {
                    None
                } else {
                    Some(Attested {
                        value: field!(&r["categories"], "value"),
                        pins: pins(&r["categories"]["pins"]),
                    })
                },
                whole_food: if r["whole_food"].is_null() {
                    None
                } else {
                    Some(Attested {
                        value: field!(&r["whole_food"], "value"),
                        pins: pins(&r["whole_food"]["pins"]),
                    })
                },
            })
            .collect(),
        purchase: v["purchase"]
            .as_array()
            .unwrap()
            .iter()
            .map(|p| PurchaseComparison {
                choice_digest: field!(p, "choice_digest"),
                context_stamp: field!(p, "context_stamp"),
                scenario_plan_digest: field!(p, "scenario_plan_digest"),
                comparison_key: field!(p, "comparison_key"),
                kind: if p["kind"] == "estimated_purchase" {
                    PurchaseKind::EstimatedPurchase
                } else {
                    PurchaseKind::ObservedPurchase
                },
                observed_at: field!(p, "observed_at"),
                valid_until: field!(p, "valid_until"),
                total_purchase_pence: field!(p, "total_purchase_pence"),
                unused_purchase_value_pence: field!(p, "unused_purchase_value_pence"),
                pins: pins(&p["pins"]),
            })
            .collect(),
    }
}
fn intent(v: &Value) -> SelectionIntent {
    SelectionIntent {
        mode: field!(v, "mode"),
        slots: field!(v, "slots"),
        origin: field!(v, "origin"),
        repeat_permissions: v["repeat_permissions"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| RepeatPermission {
                slot: field!(r, "slot"),
                recipe: field!(r, "recipe"),
            })
            .collect(),
    }
}
fn choices(v: &Value) -> Vec<ChoiceSpec> {
    v.as_array()
        .unwrap()
        .iter()
        .map(|c| ChoiceSpec {
            portable_cold: None,
            recipe: field!(c, "recipe"),
            batch_use: field!(c, "batch_use"),
            meal_overrides: field!(c, "meal_overrides"),
            extras: c["extras"]
                .as_array()
                .unwrap()
                .iter()
                .map(|e| ExtraChoice {
                    extra_id: field!(e, "extra_id"),
                    recipe: field!(e, "recipe"),
                    recipients: field!(e, "recipients"),
                })
                .collect(),
        })
        .collect()
}
fn admitted(v: &Value) -> (Vec<EvidenceBasis>, Vec<ComponentClassView>) {
    let mut b = vec![];
    let mut c = vec![];
    for r in v.as_array().unwrap() {
        let rf: RecipeRef = field!(r, "recipe");
        b.push(
            EvidenceBasis::new(
                rf.clone(),
                pins(&r["admitted"]),
                field!(r, "verified_asset_digests"),
            )
            .unwrap(),
        );
        c.push(
            ComponentClassView::new(
                rf,
                r["components"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|a| ComponentClassEvidence {
                        recipe: field!(a, "recipe"),
                        component_id: field!(a, "component_id"),
                        class: ComponentClass::General,
                        evidence_ids: field!(a, "evidence_ids"),
                    })
                    .collect(),
            )
            .unwrap(),
        );
    }
    (b, c)
}
#[test]
fn five_independent_worked_orders_and_pool_preimages() {
    let expected = load("bound-score-expectations.json");
    for case in expected.as_array().unwrap() {
        let bind = &case["binding"];
        let ctx: PlanningContext =
            serde_json::from_value(load(bind["context"].as_str().unwrap())).unwrap();
        let ev = evidence(&load(bind["evidence"].as_str().unwrap()));
        let pool = load(bind["pool"].as_str().unwrap());
        let i = intent(&pool["intent"]);
        let ch = choices(&load("inputs/choices.json"));
        let (b, c) = admitted(&load("inputs/admission.json"));
        let reviewed: Vec<_> = ctx
            .recipes
            .iter()
            .map(|r| {
                let idx = b
                    .iter()
                    .position(|b| {
                        b.recipe.recipe_id == r.recipe_id && b.recipe.revision == r.revision
                    })
                    .unwrap();
                assess_revision(r, &b[idx], &c[idx]).unwrap()
            })
            .collect();
        let result = rank_candidates(
            &ctx,
            &i.slots[0],
            &i,
            &ch,
            &reviewed,
            &ev,
            &field!(bind, "evaluated_at"),
        )
        .unwrap();
        assert_eq!(
            result.pool_digest,
            literal_digest(bind["pool"].as_str().unwrap())
        );
        assert_eq!(result.refused.len(), 1);
        assert_eq!(result.ranked.len(), 3);
        // Independent original five worked orders (A=1, B=2, C=3).
        let expected_order = match case["id"].as_str().unwrap() {
            "SK01" => vec!['2', '3', '1'],
            "SK02-balanced-protein" | "SK02-flexible-protein" => vec!['3', '2', '1'],
            "SK02-value-only" => vec!['1', '2', '3'],
            "SK03" => vec!['2', '1', '3'],
            _ => panic!("unexpected independent score vector"),
        };
        assert_eq!(
            result
                .ranked
                .iter()
                .map(|r| r.selection.recipe.recipe_id.0.chars().last().unwrap())
                .collect::<Vec<_>>(),
            expected_order
        );
        assert_eq!(result.refused[0].index, 3);
        for r in &result.ranked {
            assert_eq!(
                r.score as u64,
                case["scores"][&r.choice_digest.0].as_u64().unwrap(),
                "{}",
                case["id"]
            );
            assert_eq!(r.metrics.len(), 10);
            assert!(r.compromises.len() <= 3);
            assert_eq!(
                serde_json::json!(
                    r.compromises
                        .iter()
                        .map(compromise_json)
                        .collect::<Vec<_>>()
                ),
                case["compromises"][&r.choice_digest.0]
            );
        }
        let mut rev = ch.clone();
        rev.reverse();
        let perm = rank_candidates(
            &ctx,
            &i.slots[0],
            &i,
            &rev,
            &reviewed,
            &ev,
            &field!(bind, "evaluated_at"),
        )
        .unwrap();
        assert_eq!(result.pool_digest, perm.pool_digest);
        assert_eq!(result.ranked, perm.ranked);
        assert_eq!(perm.refused[0].index, 0);
    }
}
fn metric_name(m: RankMetric) -> &'static str {
    match m {
        RankMetric::PurchaseCost => "purchase_cost",
        RankMetric::UnusedPurchaseValue => "unused_purchase_value",
        RankMetric::IngredientReuse => "ingredient_reuse",
        RankMetric::HandsOnMinutes => "hands_on_minutes",
        RankMetric::ProteinGrams => "protein_grams",
        RankMetric::RecipeRecencyCount => "recipe_recency_count",
        RankMetric::CategoryRecencyCount => "category_recency_count",
        RankMetric::WholeFoodTag => "whole_food_tag",
        RankMetric::Feedback => "feedback",
        RankMetric::DislikedIngredients => "disliked_ingredients",
    }
}
fn mv(v: &MetricValue) -> Value {
    match v {
        MetricValue::Amount(a) => serde_json::json!({"kind":"amount","data":a}),
        MetricValue::Count(a) => serde_json::json!({"kind":"count","data":a}),
        MetricValue::Flag(a) => serde_json::json!({"kind":"flag","data":a}),
        MetricValue::Rating(a) => serde_json::json!({"kind":"rating","data":a}),
    }
}
fn compromise_json(c: &Compromise) -> Value {
    serde_json::json!({"metric":metric_name(c.metric),"chosen":mv(&c.chosen),"alternative":mv(&c.alternative),"alternative_recipe":c.alternative_recipe,"alternative_choice":c.alternative_choice,"evidence_digest":c.evidence_digest})
}
fn unavailable_json(u: &MetricUnavailable) -> Value {
    serde_json::json!({"metric":metric_name(u.metric),"causes":u.causes.iter().map(|c|serde_json::json!({"reason":match c.reason{MetricUnavailableReason::MissingEvidence=>"missing_evidence",MetricUnavailableReason::ExpiredEvidence=>"expired_evidence",MetricUnavailableReason::NoncomparableEvidence=>"noncomparable_evidence"},"choices":c.choices})).collect::<Vec<_>>()})
}
fn run(
    ctx: &PlanningContext,
    i: &SelectionIntent,
    ch: &[ChoiceSpec],
    ev: &RankingEvidence,
    at: &Instant,
) -> Result<CandidateSet, DomainError> {
    let (b, c) = admitted(&load("inputs/admission.json"));
    let reviewed: Vec<_> = ctx
        .recipes
        .iter()
        .map(|r| {
            let idx = b
                .iter()
                .position(|b| b.recipe.recipe_id == r.recipe_id && b.recipe.revision == r.revision)
                .unwrap();
            assess_revision(r, &b[idx], &c[idx]).unwrap()
        })
        .collect();
    rank_candidates(ctx, &i.slots[0], i, ch, &reviewed, ev, at)
}
#[test]
fn seven_bound_controls() {
    for case in load("bound-control-expectations.json").as_array().unwrap() {
        let ctx: PlanningContext =
            serde_json::from_value(load(case["context"].as_str().unwrap())).unwrap();
        let pool = load(case["pool"].as_str().unwrap());
        let i = intent(&pool["intent"]);
        let ch = choices(&pool["choices"]);
        let ev = evidence(&load(case["evidence"].as_str().unwrap()));
        let result = run(&ctx, &i, &ch, &ev, &field!(case, "evaluated_at")).unwrap();
        assert_eq!(
            result.pool_digest,
            literal_digest(case["pool"].as_str().unwrap()),
            "{}",
            case["id"]
        );
        let un: Vec<_> = result.unavailable.iter().map(unavailable_json).collect();
        assert_eq!(
            serde_json::json!(un),
            case["expected_unavailable"],
            "{}",
            case["id"]
        );
        for r in result.ranked {
            for m in &r.metrics {
                if let Some(n) = case["expected_metric_readings"][metric_name(m.metric)]
                    [&r.choice_digest.0]
                    .as_u64()
                {
                    assert_eq!(
                        m.value,
                        Some(MetricValue::Count(n as u32)),
                        "{}",
                        case["id"]
                    );
                }
            }
            if let Some(n) = case["expected_scores"][&r.choice_digest.0].as_u64() {
                assert_eq!(r.score as u64, n, "{}", case["id"]);
            }
        }
    }
}
fn fixture_bytes(path: &str) -> Vec<u8> {
    fs::read(
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../../tests/fixtures/households/p03cd/ranking")
            .join(path),
    )
    .unwrap()
}
fn literal_digest(path: &str) -> Digest {
    let all = load("expected-hashes.json");
    let entry = all
        .as_object()
        .unwrap()
        .values()
        .find(|v| v["path"] == path)
        .unwrap();
    Digest(entry["sha256"].as_str().unwrap().into())
}
#[test]
fn native_evidence_material_matches_literal_bytes() {
    for case in load("bound-score-expectations.json").as_array().unwrap() {
        let path = case["binding"]["evidence"].as_str().unwrap();
        let raw = load(path);
        let ev = evidence(&raw);
        let canonical = path
            .replace("inputs/", "canonical/")
            .replace(".json", ".jcs.json");
        assert_eq!(
            canonical_bytes(&ranking_evidence_material(&ev).unwrap()).unwrap(),
            fixture_bytes(&canonical)
        );
        assert_eq!(ev.digest, literal_digest(&canonical));
    }
}

fn base() -> (
    PlanningContext,
    SelectionIntent,
    Vec<ChoiceSpec>,
    RankingEvidence,
    Instant,
) {
    let ctx = serde_json::from_value(load("canonical/context-base.jcs.json")).unwrap();
    let i = intent(&load("inputs/intent.json"));
    let ch = choices(&load("inputs/choices.json"));
    let ev = evidence(&load("inputs/ranking-evidence-known-zero.json"));
    (ctx, i, ch, ev, Instant("2026-12-21T12:00:00Z".into()))
}
fn rehash(ev: &mut RankingEvidence) {
    ev.digest = canonical_digest(&ranking_evidence_material(ev).unwrap()).unwrap();
}

#[test]
fn repeated_context_never_reuses_time_dependent_ranking_admission() {
    let (ctx,i,ch,e,at)=base();assert!(run(&ctx,&i,&ch,&e,&at).is_ok());
    let earlier=Instant("2020-01-01T00:00:00Z".into());
    let (cold_context,cold_intent,cold_choices,cold_evidence,cold_time)=(ctx.clone(),i.clone(),ch.clone(),e.clone(),earlier.clone());
    let cold=std::thread::spawn(move||run(&cold_context,&cold_intent,&cold_choices,&cold_evidence,&cold_time)).join().unwrap();
    assert!(cold.is_err());assert_eq!(run(&ctx,&i,&ch,&e,&earlier),cold);
    assert!(run(&ctx,&i,&ch,&e,&at).is_ok());
}

#[test]
fn local_repeat_authorisation_preserves_full_pool_scoring_and_strict_intent() {
    let (ctx, mut i, ch, mut e, at) = base();
    i.slots.truncate(1);
    i.repeat_permissions.clear();
    e.purchase.clear();
    rehash(&mut e);
    let (bases, classes) = admitted(&load("inputs/admission.json"));
    let reviewed: Vec<_> = ctx.recipes.iter().map(|r| {
        let n = bases.iter().position(|b| b.recipe.recipe_id == r.recipe_id && b.recipe.revision == r.revision).unwrap();
        assess_revision(r, &bases[n], &classes[n]).unwrap()
    }).collect();
    let ordinary = rank_candidates(&ctx,&i.slots[0],&i,&ch,&reviewed,&e,&at).unwrap();
    let explicit = rank_candidates_with_repeat_authorisation(&ctx,&i.slots[0],&i,&ch,&reviewed,&e,&at).unwrap();
    assert!(explicit.ranked.len() > 1);
    assert_ne!(ordinary.pool_digest, explicit.pool_digest);
    for (old,new) in ordinary.ranked.iter().zip(&explicit.ranked) {
        assert_eq!(old.selection.recipe,new.selection.recipe);
        assert_eq!(old.score,new.score);
        assert_eq!(old.metrics,new.metrics);
        assert!(new.selection.repeat_authorised);
    }
    let mut reversed=ch.clone();reversed.reverse();
    let reordered=rank_candidates_with_repeat_authorisation(&ctx,&i.slots[0],&i,&reversed,&reviewed,&e,&at).unwrap();
    assert_eq!(explicit.ranked,reordered.ranked);assert_eq!(explicit.pool_digest,reordered.pool_digest);
    let mut invalid=i.clone();invalid.repeat_permissions=ch[..2].iter().map(|c|RepeatPermission{slot:i.slots[0].clone(),recipe:c.recipe.clone()}).collect();
    assert_eq!(rank_candidates(&ctx,&i.slots[0],&invalid,&ch,&reviewed,&e,&at).unwrap_err(),DomainError::new(ErrorCode::InvalidValue,"/intent/repeat_permissions"));
    assert!(rank_candidates_with_repeat_authorisation(&ctx,&i.slots[0],&invalid,&ch,&reviewed,&e,&at).is_err());
}

#[test]
fn local_repeat_authorisation_consults_ordered_dinner_and_never_overrides_hidden_food() {
    let (mut ctx, mut i, ch, mut e, at)=base();i.slots.truncate(1);i.repeat_permissions.clear();e.purchase.clear();rehash(&mut e);
    let a:MealSelection=serde_json::from_value(load("canonical/choice-A2.jcs.json")).unwrap();
    let mut slot=i.slots[0].clone();slot.week_id=WeekId("wk_00000000000000000000000000000099".into());slot.date=LocalDate("2026-12-21".into());
    ctx.history.ordered.0.push(OrderedHistory{selection:SelectionHistory{week_id:slot.week_id.clone(),slot,recipe:a.recipe.clone(),servings:a.servings.clone(),extras:a.extras.clone(),repeat_authorised:false,selected_at:Instant("2026-12-13T12:00:00Z".into())},order_id:OrderId("or_00000000000000000000000000000099".into()),ordered_at:Instant("2026-12-13T13:00:00Z".into())});
    let normal=run(&ctx,&i,&ch,&e,&at).unwrap();assert!(normal.withheld.iter().any(|w|w.selection.recipe==a.recipe&&w.reason==WithheldReason::RepeatNeedsPermission));
    let (bases,classes)=admitted(&load("inputs/admission.json"));
    let reviewed:Vec<_>=ctx.recipes.iter().map(|r|{let n=bases.iter().position(|b|b.recipe.recipe_id==r.recipe_id&&b.recipe.revision==r.revision).unwrap();assess_revision(r,&bases[n],&classes[n]).unwrap()}).collect();
    let explicit=rank_candidates_with_repeat_authorisation(&ctx,&i.slots[0],&i,&ch,&reviewed,&e,&at).unwrap();assert!(explicit.ranked.iter().any(|r|r.selection.recipe==a.recipe&&r.selection.repeat_authorised));
    ctx.history.hidden_recipe_ids.0.push(a.recipe.recipe_id.clone());
    let hidden=rank_candidates_with_repeat_authorisation(&ctx,&i.slots[0],&i,&ch,&reviewed,&e,&at).unwrap();assert!(!hidden.ranked.iter().any(|r|r.selection.recipe==a.recipe));assert!(hidden.withheld.iter().any(|w|w.selection.recipe==a.recipe&&w.reason==WithheldReason::Hidden));
}
#[test]
fn malformed_native_evidence_cannot_be_downgraded_to_unknown() {
    let (ctx, i, ch, original, at) = base();
    let mutations: Vec<(&str, fn(&mut RankingEvidence))> = vec![
        ("future observation", |e| {
            e.purchase[0].observed_at = Instant("2027-01-01T00:00:00Z".into())
        }),
        ("reversed validity", |e| {
            e.purchase[0].valid_until = Instant("2020-01-01T00:00:00Z".into())
        }),
        ("scenario mismatch", |e| {
            e.purchase[0].scenario_plan_digest = PlanInputDigest("a".repeat(64))
        }),
        ("foreign choice", |e| {
            e.purchase[0].choice_digest = Digest("a".repeat(64))
        }),
        ("noncanonical fact order", |e| e.recipes.swap(0, 1)),
        ("duplicate fact", |e| e.recipes[1] = e.recipes[0].clone()),
        ("wrong nutrition bytes", |e| {
            e.recipes[0].nutrition_digest.as_mut().unwrap().value = Digest("a".repeat(64))
        }),
        ("empty attestation", |e| {
            e.recipes[0].nutrition_digest.as_mut().unwrap().pins.clear()
        }),
        ("noncanonical pins", |e| {
            e.recipes[0].categories.as_mut().unwrap().pins.reverse()
        }),
        ("unknown category", |e| {
            e.recipes[0].categories.as_mut().unwrap().value = vec![Text("unknown".into())]
        }),
        ("duplicate category", |e| {
            let a = e.recipes[0].categories.as_mut().unwrap();
            a.value.push(a.value[0].clone());
        }),
        ("foreign recipe", |e| {
            e.recipes[0].recipe.digest = RecipeDigest("a".repeat(64))
        }),
        ("malformed rational", |e| {
            e.purchase[0].total_purchase_pence.denominator = DecimalU64(0)
        }),
        ("waste above purchase", |e| {
            e.purchase[0].unused_purchase_value_pence = Some(Rational::new(1000000, 1).unwrap())
        }),
        ("conflicting pin", |e| {
            e.recipes[1].nutrition_digest.as_mut().unwrap().pins[0].evidence_id =
                e.recipes[0].nutrition_digest.as_ref().unwrap().pins[0]
                    .evidence_id
                    .clone();
        }),
        ("foreign stamp", |e| {
            e.purchase[0].context_stamp.history_revision = Revision(2)
        }),
    ];
    for (name, mutate) in mutations {
        let mut e = original.clone();
        mutate(&mut e);
        rehash(&mut e);
        assert!(run(&ctx, &i, &ch, &e, &at).is_err(), "{name}");
    }
}
#[test]
fn unknown_policy_and_duplicate_configurations_reject() {
    let (mut ctx, i, ch, mut e, at) = base();
    ctx.ranking_policy.digest = PayloadDigest("a".repeat(64));
    assert!(run(&ctx, &i, &ch, &e, &at).is_err());
    ctx.ranking_policy.digest = PayloadDigest(POLICY_DIGEST.into());
    let mut duplicate = ch.clone();
    duplicate.push(ch[0].clone());
    assert_eq!(
        run(&ctx, &i, &duplicate, &e, &at).unwrap_err().code,
        ErrorCode::DuplicateKey
    );
    let too_many = vec![ch[0].clone(); 513];
    assert_eq!(
        run(&ctx, &i, &too_many, &e, &at).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
    e.purchase.clear();
    rehash(&mut e);
    let mut extra = ch[0].clone();
    extra.extras = vec![
        ExtraChoice {
            extra_id: ExtraId("ex_00000000000000000000000000000001".into()),
            recipe: extra.recipe.clone(),
            recipients: vec![]
        };
        65
    ];
    assert_eq!(
        run(&ctx, &i, &[extra], &e, &at).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
}
#[test]
fn hidden_and_empty_pools_never_invent_candidates() {
    let (mut ctx, i, ch, mut e, at) = base();
    e.purchase.clear();
    rehash(&mut e);
    ctx.history.hidden_recipe_ids = BoundedVec(vec![
        ch[0].recipe.recipe_id.clone(),
        ch[1].recipe.recipe_id.clone(),
        ch[2].recipe.recipe_id.clone(),
    ]);
    let out = run(&ctx, &i, &ch, &e, &at).unwrap();
    assert!(out.ranked.is_empty());
    assert!(out.unavailable.is_empty());
    assert_eq!(out.withheld.len(), 3);
    assert_eq!(out.refused.len(), 1);
    let mut manual = i.clone();
    manual.origin = SelectionOrigin::Fixed;
    let out = run(&ctx, &manual, &ch, &e, &at).unwrap();
    assert_eq!(out.ranked.len(), 3);
    assert!(out.withheld.is_empty());
    let empty = run(&ctx, &i, &[], &e, &at).unwrap();
    assert!(empty.ranked.is_empty() && empty.unavailable.is_empty() && empty.refused.is_empty());
}
#[test]
fn evidence_encoder_preflights_nested_native_bounds() {
    let (_, _, _, mut e, _) = base();
    let pin = e.recipes[0].nutrition_digest.as_ref().unwrap().pins[0].clone();
    e.recipes[0].nutrition_digest.as_mut().unwrap().pins = vec![pin; 257];
    assert_eq!(
        ranking_evidence_material(&e).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
}
#[test]
fn unknown_protein_omits_dimension_for_whole_pool_without_imputation() {
    let (ctx, i, ch, mut e, at) = base();
    e.recipes
        .iter_mut()
        .find(|f| f.recipe == ch[1].recipe)
        .unwrap()
        .nutrition_digest = None;
    rehash(&mut e);
    let out = run(&ctx, &i, &ch, &e, &at).unwrap();
    let missing = out
        .unavailable
        .iter()
        .find(|u| u.metric == RankMetric::ProteinGrams)
        .unwrap();
    assert_eq!(
        missing.causes[0].choices,
        vec![Digest(
            "9e45c4556204c533296365c11b9af9072cf8a541a010dd4a6d7cbec85ab6a887".into()
        )]
    );
    for r in out.ranked {
        let symbol = r.selection.recipe.recipe_id.0.chars().last().unwrap();
        assert_eq!(
            r.score,
            match symbol {
                '1' => 8,
                '2' => 12,
                '3' => 4,
                _ => panic!(),
            }
        );
        if symbol == '2' {
            assert_eq!(r.metrics[4].value, None);
        } else {
            assert!(r.metrics[4].value.is_some());
        }
    }
}
#[test]
fn whole_food_true_false_unknown_is_an_independent_dimension() {
    let (mut ctx, i, ch, mut e, at) = base();
    ctx.household.preferences.whole_food_preference = true;
    e.purchase.clear();
    e.recipes
        .iter_mut()
        .find(|f| f.recipe == ch[1].recipe)
        .unwrap()
        .whole_food
        .as_mut()
        .unwrap()
        .value = false;
    rehash(&mut e);
    let known = run(&ctx, &i, &ch, &e, &at).unwrap();
    assert!(
        known
            .unavailable
            .iter()
            .all(|u| u.metric != RankMetric::WholeFoodTag)
    );
    assert_eq!(
        known
            .ranked
            .iter()
            .find(|r| r.selection.recipe == ch[1].recipe)
            .unwrap()
            .metrics[7]
            .value,
        Some(MetricValue::Flag(false))
    );
    e.recipes
        .iter_mut()
        .find(|f| f.recipe == ch[2].recipe)
        .unwrap()
        .whole_food = None;
    rehash(&mut e);
    let unknown = run(&ctx, &i, &ch, &e, &at).unwrap();
    assert_eq!(
        unknown
            .unavailable
            .iter()
            .find(|u| u.metric == RankMetric::WholeFoodTag)
            .unwrap()
            .causes[0]
            .choices
            .len(),
        1
    );
    assert_eq!(unknown.ranked.len(), 3);
}
#[test]
fn latest_exact_recipe_feedback_and_independent_hide() {
    let (mut ctx, i, ch, mut e, at) = base();
    e.purchase.clear();
    rehash(&mut e);
    let feedback = |recipe: RecipeRef, when: &str, rating| Feedback {
        slot: i.slots[0].clone(),
        recipe,
        rating,
        reasons: BoundedVec(vec![]),
        hide: true,
        portion_change: Nullable(None),
        portion_change_accepted: false,
        recorded_at: Instant(when.into()),
    };
    ctx.history.feedback.0 = vec![
        feedback(
            ch[0].recipe.clone(),
            "2026-12-19T12:00:00Z",
            FeedbackRating::Liked,
        ),
        feedback(
            ch[0].recipe.clone(),
            "2026-12-20T12:00:00Z",
            FeedbackRating::NotForMe,
        ),
    ];
    let out = run(&ctx, &i, &ch, &e, &at).unwrap();
    let a = out
        .ranked
        .iter()
        .find(|r| r.selection.recipe == ch[0].recipe)
        .unwrap();
    assert_eq!(a.metrics[8].value, Some(MetricValue::Rating(-1)));
    assert!(out.withheld.is_empty());
    let mut different = ch[0].recipe.clone();
    different.revision = Revision(10);
    different.digest =
        RecipeDigest("98394a2ce55022e6bf2bb89d837d770f9b2abe774cce166ccf695fd21ed2e65b".into());
    ctx.history.feedback.0.push(feedback(
        different,
        "2026-12-21T11:00:00Z",
        FeedbackRating::Liked,
    ));
    let out = run(&ctx, &i, &ch, &e, &at).unwrap();
    assert_eq!(
        out.ranked
            .iter()
            .find(|r| r.selection.recipe == ch[0].recipe)
            .unwrap()
            .metrics[8]
            .value,
        Some(MetricValue::Rating(-1))
    );
    let mut contradiction = feedback(
        ch[0].recipe.clone(),
        "2026-12-20T12:00:00Z",
        FeedbackRating::Liked,
    );
    contradiction.slot = i.slots[1].clone();
    ctx.history.feedback.0.push(contradiction);
    assert!(run(&ctx, &i, &ch, &e, &at).is_err());
}
#[test]
fn variety_occurrences_are_deduplicated_and_missing_history_tags_omit_category() {
    let (mut ctx, i, ch, mut e, at) = base();
    ctx.household.preferences.more_variety = true;
    e.purchase.clear();
    rehash(&mut e);
    let a: MealSelection = serde_json::from_value(load("canonical/choice-A2.jcs.json")).unwrap();
    let b: MealSelection = serde_json::from_value(load("canonical/choice-B.jcs.json")).unwrap();
    for (day, s) in [(1, &a), (2, &a), (3, &a), (4, &b)] {
        let mut slot = i.slots[0].clone();
        slot.week_id = WeekId("wk_00000000000000000000000000000009".into());
        slot.date = LocalDate(format!("2026-12-{day:02}"));
        ctx.history.selected.0.push(SelectionHistory {
            week_id: slot.week_id.clone(),
            slot,
            recipe: s.recipe.clone(),
            servings: s.servings.clone(),
            extras: s.extras.clone(),
            repeat_authorised: false,
            selected_at: Instant("2026-11-30T12:00:00Z".into()),
        });
    }
    ctx.history.ordered.0.push(OrderedHistory {
        selection: ctx.history.selected[0].clone(),
        order_id: OrderId("or_00000000000000000000000000000001".into()),
        ordered_at: Instant("2026-11-30T13:00:00Z".into()),
    });
    let out = run(&ctx, &i, &ch, &e, &at).unwrap();
    for r in &out.ranked {
        let id = r.selection.recipe.recipe_id.0.chars().last().unwrap();
        assert_eq!(
            r.metrics[5].value,
            Some(MetricValue::Count(match id {
                '1' => 3,
                '2' => 1,
                '3' => 0,
                _ => panic!(),
            }))
        );
        assert_eq!(r.metrics[6].value, Some(MetricValue::Count(4)));
    }
    e.recipes
        .iter_mut()
        .find(|f| f.recipe == ch[0].recipe)
        .unwrap()
        .categories = None;
    rehash(&mut e);
    let out = run(&ctx, &i, &ch, &e, &at).unwrap();
    assert_eq!(
        out.unavailable
            .iter()
            .find(|u| u.metric == RankMetric::CategoryRecencyCount)
            .unwrap()
            .causes[0]
            .choices
            .len(),
        3
    );
    assert!(out.ranked.iter().all(|r| r.metrics[5].value.is_some()));
}
#[test]
fn dislikes_are_soft_and_one_suitable_candidate_has_zero_score() {
    let (mut ctx, i, ch, mut e, at) = base();
    e.purchase.clear();
    rehash(&mut e);
    let ingredient = ctx
        .recipes
        .iter()
        .find(|r| r.recipe_id == ch[0].recipe.recipe_id && r.revision == ch[0].recipe.revision)
        .unwrap()
        .ingredients[0]
        .ingredient_id
        .clone();
    ctx.household.preferences.dislikes = BoundedVec(vec![ingredient]);
    let out = run(&ctx, &i, &ch[..1], &e, &at).unwrap();
    assert_eq!(out.ranked.len(), 1);
    assert_eq!(out.ranked[0].score, 0);
    assert_eq!(out.ranked[0].metrics[9].value, Some(MetricValue::Count(1)));
}
#[test]
fn future_history_is_an_error_not_soft_data() {
    let (mut ctx, i, ch, mut e, at) = base();
    e.purchase.clear();
    rehash(&mut e);
    ctx.history.feedback.0.push(Feedback {
        slot: i.slots[0].clone(),
        recipe: ch[0].recipe.clone(),
        rating: FeedbackRating::Liked,
        reasons: BoundedVec(vec![]),
        hide: false,
        portion_change: Nullable(None),
        portion_change_accepted: false,
        recorded_at: Instant("2026-12-22T00:00:00Z".into()),
    });
    assert_eq!(
        run(&ctx, &i, &ch, &e, &at).unwrap_err().code,
        ErrorCode::InvalidValue
    );
}
#[test]
fn carried_repeat_permission_is_exact_slot_and_ref() {
    let (mut ctx, i, ch, mut e, at) = base();
    e.purchase.clear();
    rehash(&mut e);
    let mut a: MealSelection =
        serde_json::from_value(load("canonical/choice-A2.jcs.json")).unwrap();
    a.repeat_authorised = true;
    ctx.weeks[0].plan.selections.0.push(a.clone());
    let mut adjacent = a.clone();
    adjacent.slot = SlotId {
        week_id: ctx.weeks[1].range.week_id.clone(),
        date: ctx.weeks[1].range.monday.clone(),
        meal_type: MealType::Dinner,
    };
    adjacent.batch_use = BatchUse::NewBatch {
        batch_id: BatchId("ba_00000000000000000000000000000099".into()),
        production_servings: PositiveRational::new(2, 1).unwrap(),
    };
    ctx.weeks[1].plan.selections.0.push(adjacent);
    let carried = run(&ctx, &i, &ch[..1], &e, &at).unwrap();
    assert_eq!(carried.ranked.len(), 1);
    assert!(carried.ranked[0].selection.repeat_authorised);
    ctx.weeks[0].plan.selections[0].repeat_authorised = false;
    let unpermitted = run(&ctx, &i, &ch[..1], &e, &at).unwrap();
    assert_eq!(
        unpermitted.withheld[0].reason,
        WithheldReason::RepeatNeedsPermission
    );
    ctx.weeks[0].plan.selections[0].repeat_authorised = true;
    let mut different = ch[0].clone();
    different.recipe.revision = Revision(10);
    different.recipe.digest =
        RecipeDigest("98394a2ce55022e6bf2bb89d837d770f9b2abe774cce166ccf695fd21ed2e65b".into());
    assert_eq!(
        run(&ctx, &i, &[different], &e, &at).unwrap().withheld[0].reason,
        WithheldReason::RepeatNeedsPermission
    );
    let mut other = i.clone();
    other.slots.remove(0);
    assert_eq!(
        run(&ctx, &other, &ch[..1], &e, &at).unwrap().withheld[0].reason,
        WithheldReason::RepeatNeedsPermission
    );
}
#[test]
fn empty_pool_still_rejects_inactive_target() {
    let (mut ctx, i, _, mut e, at) = base();
    e.purchase.clear();
    rehash(&mut e);
    ctx.weeks[0].day_enabled[0] = false;
    for r in &mut ctx.requirements.0 {
        if r.slot.week_id == i.slots[0].week_id && r.slot.date == i.slots[0].date {
            r.enabled = false;
            r.attendees.0.clear();
            r.shared_hard_requirements.0.clear();
            r.eat_by = Nullable(None);
        }
    }
    assert!(run(&ctx, &i, &[], &e, &at).is_err());
}
#[test]
fn actual_portions_scale_nutrition_and_numeric_revision_breaks_ties() {
    let (ctx, i, ch, mut e, at) = base();
    e.purchase.clear();
    rehash(&mut e);
    let mut half = ch[0].clone();
    half.meal_overrides.portions.0.push(PersonPortionOverride {
        person: PersonRef::Diner {
            diner_id: ctx.household.diners[0].diner_id.clone(),
        },
        preset: PortionPresetName::Regular,
        adjustment: PositiveRational::new(1, 2).unwrap(),
    });
    let out = run(&ctx, &i, &[half], &e, &at).unwrap();
    assert_eq!(
        out.ranked[0].metrics[4].value,
        Some(MetricValue::Amount(Rational::new(15, 2).unwrap()))
    );
    let mut a10 = ch[0].clone();
    a10.recipe.revision = Revision(10);
    a10.recipe.digest =
        RecipeDigest("98394a2ce55022e6bf2bb89d837d770f9b2abe774cce166ccf695fd21ed2e65b".into());
    let tied = run(&ctx, &i, &[a10, ch[0].clone()], &e, &at).unwrap();
    assert_eq!(
        tied.ranked
            .iter()
            .map(|r| r.selection.recipe.revision.0)
            .collect::<Vec<_>>(),
        vec![2, 10]
    );
    assert!(tied.ranked.iter().all(|r| r.score == 0));
}
#[test]
fn occupied_scenario_keeps_unrelated_selection_order() {
    let (mut ctx, i, ch, mut e, at) = base();
    e.purchase.clear();
    let a: MealSelection = serde_json::from_value(load("canonical/choice-A2.jcs.json")).unwrap();
    let b: MealSelection = serde_json::from_value(load("canonical/choice-B.jcs.json")).unwrap();
    let mut last = a.clone();
    last.slot.date = LocalDate("2026-12-30".into());
    last.batch_use = BatchUse::NewBatch {
        batch_id: BatchId("ba_00000000000000000000000000000090".into()),
        production_servings: PositiveRational::new(2, 1).unwrap(),
    };
    ctx.weeks[0].plan.selections.0 = vec![last, a];
    let mut scenario = ctx.weeks[0].plan.clone();
    scenario.selections[1] = b;
    let expected = plan_input_digest(&PlanInputMaterial {
        schema_version: ctx.schema_version,
        week_plan: scenario,
        context_stamp: ctx.stamp.clone(),
    })
    .unwrap();
    let original = evidence(&load("inputs/ranking-evidence-known-zero.json"));
    let mut quote = original
        .purchase
        .iter()
        .find(|p| {
            p.choice_digest.0 == "9e45c4556204c533296365c11b9af9072cf8a541a010dd4a6d7cbec85ab6a887"
        })
        .unwrap()
        .clone();
    quote.scenario_plan_digest = expected;
    e.purchase.push(quote);
    rehash(&mut e);
    let result = run(&ctx, &i, &ch[1..2], &e, &at).unwrap();
    assert_eq!(result.ranked.len(), 1);
}
// Reviewer-owned assertions. Reuse literal fixture/admission readers only.

#[test]
fn review_all_24_raw_permutations_bind_same_pool_and_refusal_locator() {
    let (ctx, i, ch, e, at) = base();
    let expected = run(&ctx, &i, &ch, &e, &at).unwrap();
    let mut count = 0;
    for a in 0..4 {
        for b in 0..4 {
            for c in 0..4 {
                for d in 0..4 {
                    let indices = [a, b, c, d];
                    if indices
                        .iter()
                        .collect::<std::collections::BTreeSet<_>>()
                        .len()
                        != 4
                    {
                        continue;
                    }
                    let perm: Vec<_> = indices.iter().map(|j| ch[*j].clone()).collect();
                    let out = run(&ctx, &i, &perm, &e, &at).unwrap();
                    assert_eq!(out.pool_digest, expected.pool_digest);
                    assert_eq!(out.ranked, expected.ranked);
                    assert_eq!(
                        out.refused[0].index as usize,
                        indices.iter().position(|j| *j == 3).unwrap()
                    );
                    count += 1;
                }
            }
        }
    }
    assert_eq!(count, 24);
}

#[test]
fn review_intent_permutation_keeps_readings_but_changes_exact_pool_binding() {
    let (ctx, mut i, ch, e, at) = base();
    let expected = run(&ctx, &i, &ch, &e, &at).unwrap();
    let slot = i.slots[0].clone();
    i.slots.reverse();
    let (b, k) = admitted(&load("inputs/admission.json"));
    let reviewed: Vec<_> = ctx
        .recipes
        .iter()
        .map(|r| {
            let j = b
                .iter()
                .position(|x| x.recipe.recipe_id == r.recipe_id && x.recipe.revision == r.revision)
                .unwrap();
            assess_revision(r, &b[j], &k[j]).unwrap()
        })
        .collect();
    let actual = rank_candidates(&ctx, &slot, &i, &ch, &reviewed, &e, &at).unwrap();
    assert_eq!(actual.ranked, expected.ranked);
    assert_ne!(actual.pool_digest, expected.pool_digest);
}

#[test]
fn review_all_authoritative_purchase_and_registry_permutations_refuse() {
    let (ctx, i, ch, original, at) = base();
    let mut ev = original.clone();
    ev.purchase.reverse();
    rehash(&mut ev);
    assert_eq!(
        run(&ctx, &i, &ch, &ev, &at).unwrap_err().code,
        ErrorCode::NoncanonicalValue
    );
    let mut ev = original;
    let reg = ev.categories.as_mut().unwrap();
    reg.keys.push(reg.keys[0].clone());
    reg.digest =
        canonical_digest(&serde_json::json!({"revision":reg.revision,"keys":reg.keys})).unwrap();
    rehash(&mut ev);
    assert_eq!(
        run(&ctx, &i, &ch, &ev, &at).unwrap_err().code,
        ErrorCode::NoncanonicalValue
    );
}

#[test]
fn review_mixed_purchase_kinds_mark_entire_ranked_quote_cohort() {
    let (ctx, i, ch, mut ev, at) = base();
    ev.purchase[0].kind = PurchaseKind::ObservedPurchase;
    rehash(&mut ev);
    let out = run(&ctx, &i, &ch, &ev, &at).unwrap();
    let mut digests: Vec<_> = out.ranked.iter().map(|r| r.choice_digest.clone()).collect();
    digests.sort();
    for metric in [RankMetric::PurchaseCost, RankMetric::UnusedPurchaseValue] {
        let u = out.unavailable.iter().find(|u| u.metric == metric).unwrap();
        assert_eq!(u.causes.len(), 1);
        assert_eq!(
            u.causes[0].reason,
            MetricUnavailableReason::NoncomparableEvidence
        );
        assert_eq!(u.causes[0].choices, digests);
    }
}

#[test]
fn review_public_malformed_choice_primitives_refuse_without_panic() {
    let (ctx, i, ch, mut ev, at) = base();
    ev.purchase.clear();
    rehash(&mut ev);
    for field in 0..4 {
        let mut mutated = ch[0].clone();
        match field {
            0 => mutated.recipe.revision = Revision(0),
            1 => mutated.recipe.recipe_id = RecipeId("invalid".into()),
            2 => {
                if let BatchUse::NewBatch {
                    production_servings,
                    ..
                } = &mut mutated.batch_use
                {
                    production_servings.0.denominator = DecimalU64(0)
                }
            }
            _ => {
                if let BatchUse::NewBatch {
                    production_servings,
                    ..
                } = &mut mutated.batch_use
                {
                    production_servings.0 = Rational {
                        numerator: DecimalU64(2),
                        denominator: DecimalU64(4),
                    }
                }
            }
        }
        assert!(run(&ctx, &i, &[mutated], &ev, &at).is_err());
    }
}

#[test]
fn review_native_null_key_and_public_collection_bounds() {
    let (_, _, _, mut ev, _) = base();
    ev.purchase[0].unused_purchase_value_pence = None;
    let material = ranking_evidence_material(&ev).unwrap();
    assert!(
        material["purchase"][0]
            .as_object()
            .unwrap()
            .contains_key("unused_purchase_value_pence")
    );
    assert!(material["purchase"][0]["unused_purchase_value_pence"].is_null());
    let rf = ev.purchase[0].context_stamp.recipe_refs[0].clone();
    ev.purchase[0].context_stamp.recipe_refs.0.resize(513, rf);
    assert_eq!(
        ranking_evidence_material(&ev).unwrap_err().code,
        ErrorCode::LimitExceeded
    );
}
