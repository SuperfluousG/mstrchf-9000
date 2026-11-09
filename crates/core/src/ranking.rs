//! Bounded native soft ranking. Source-owned evidence pins bind bytes, not truth.
//! No result here grants selection, plan feasibility, persistence or effect authority.
use crate::recipes::{EvidencePin, ReviewedRecipe};
use crate::selection::{
    ChoiceAssessment, ChoiceSpec, SelectionIntent, assess_choice_in_validated_context,
    repeat_needs_permission, validate_current_selected, validate_intent,
    validate_selection_context,
};
use mstrchf_contracts::*;
use serde_json::{Value, json};
use std::cmp::Ordering;
use std::collections::{BTreeMap, BTreeSet};

pub const POLICY_DIGEST: &str = "cff488e1f4729bc788b1a13daece0ace72b714768de10b0446d9bd049b214daa";
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attested<T> {
    pub value: T,
    pub pins: Vec<EvidencePin>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CategoryRegistry {
    pub revision: Revision,
    pub digest: Digest,
    pub keys: Vec<Text>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RecipeRankFacts {
    pub recipe: RecipeRef,
    pub nutrition_digest: Option<Attested<Digest>>,
    pub categories: Option<Attested<Vec<Text>>>,
    pub whole_food: Option<Attested<bool>>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PurchaseKind {
    ObservedPurchase,
    EstimatedPurchase,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PurchaseComparison {
    pub choice_digest: Digest,
    pub context_stamp: ContextStamp,
    pub scenario_plan_digest: PlanInputDigest,
    pub comparison_key: Digest,
    pub kind: PurchaseKind,
    pub observed_at: Instant,
    pub valid_until: Instant,
    pub total_purchase_pence: Rational,
    pub unused_purchase_value_pence: Option<Rational>,
    pub pins: Vec<EvidencePin>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankingEvidence {
    pub revision: Revision,
    pub digest: Digest,
    pub catalogue_revision: Revision,
    pub categories: Option<CategoryRegistry>,
    pub recipes: Vec<RecipeRankFacts>,
    pub purchase: Vec<PurchaseComparison>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RankMetric {
    PurchaseCost,
    UnusedPurchaseValue,
    IngredientReuse,
    HandsOnMinutes,
    ProteinGrams,
    RecipeRecencyCount,
    CategoryRecencyCount,
    WholeFoodTag,
    Feedback,
    DislikedIngredients,
}
const METRICS: [RankMetric; 10] = [
    RankMetric::PurchaseCost,
    RankMetric::UnusedPurchaseValue,
    RankMetric::IngredientReuse,
    RankMetric::HandsOnMinutes,
    RankMetric::ProteinGrams,
    RankMetric::RecipeRecencyCount,
    RankMetric::CategoryRecencyCount,
    RankMetric::WholeFoodTag,
    RankMetric::Feedback,
    RankMetric::DislikedIngredients,
];
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetricValue {
    Amount(Rational),
    Count(u32),
    Flag(bool),
    Rating(i8),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetricReading {
    pub metric: RankMetric,
    pub value: Option<MetricValue>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum MetricUnavailableReason {
    MissingEvidence,
    ExpiredEvidence,
    NoncomparableEvidence,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetricUnavailableCause {
    pub reason: MetricUnavailableReason,
    pub choices: Vec<Digest>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetricUnavailable {
    pub metric: RankMetric,
    pub causes: Vec<MetricUnavailableCause>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Compromise {
    pub metric: RankMetric,
    pub chosen: MetricValue,
    pub alternative: MetricValue,
    pub alternative_recipe: RecipeRef,
    pub alternative_choice: Digest,
    pub evidence_digest: Digest,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RankedChoice {
    pub choice_digest: Digest,
    pub selection: MealSelection,
    pub score: u32,
    pub metrics: Vec<MetricReading>,
    pub compromises: Vec<Compromise>,
}
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum WithheldReason {
    Hidden,
    RepeatNeedsPermission,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WithheldChoice {
    pub choice_digest: Digest,
    pub selection: MealSelection,
    pub reason: WithheldReason,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ChoiceRefusal {
    pub index: u16,
    pub outcome: ChoiceAssessment,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct CandidateSet {
    pub slot: SlotId,
    pub context_stamp: ContextStamp,
    pub evidence_revision: Revision,
    pub evidence_digest: Digest,
    pub pool_digest: Digest,
    pub evaluated_at: Instant,
    pub ranked: Vec<RankedChoice>,
    pub withheld: Vec<WithheldChoice>,
    pub refused: Vec<ChoiceRefusal>,
    pub unavailable: Vec<MetricUnavailable>,
}
fn fail(code: ErrorCode, path: &str) -> DomainError {
    DomainError::new(code, path)
}
fn require(ok: bool, code: ErrorCode, path: &str) -> Result<(), DomainError> {
    if ok { Ok(()) } else { Err(fail(code, path)) }
}
fn bound(n: usize, max: usize, path: &str) -> Result<(), DomainError> {
    require(n <= max, ErrorCode::LimitExceeded, path)
}
fn shape(v: &impl ValidateShape, path: &str) -> Result<(), DomainError> {
    v.validate_shape().map_err(|e| {
        fail(
            e.first().map(|i| i.code).unwrap_or(ErrorCode::InvalidValue),
            path,
        )
    })
}
fn pin_json(p: &EvidencePin) -> Value {
    json!({"evidence_id":p.evidence_id,"revision":p.revision,"digest":p.digest})
}
fn attested_json(v: Value, p: &[EvidencePin]) -> Value {
    json!({"value":v,"pins":p.iter().map(pin_json).collect::<Vec<_>>()})
}
fn registry_json(c: &CategoryRegistry) -> Value {
    json!({"revision":c.revision,"digest":c.digest,"keys":c.keys})
}
/// Exact adopted self-digest-omitted native material, with required nulls.
pub fn ranking_evidence_material(e: &RankingEvidence) -> Result<Value, DomainError> {
    evidence_bounds(e)?;
    Ok(evidence_material_unchecked(e))
}
fn evidence_material_unchecked(e: &RankingEvidence) -> Value {
    json!({"revision":e.revision,"catalogue_revision":e.catalogue_revision,"categories":e.categories.as_ref().map(registry_json),"recipes":e.recipes.iter().map(|r|json!({"recipe":r.recipe,"nutrition_digest":r.nutrition_digest.as_ref().map(|a|attested_json(json!(a.value),&a.pins)),"categories":r.categories.as_ref().map(|a|attested_json(json!(a.value),&a.pins)),"whole_food":r.whole_food.as_ref().map(|a|attested_json(json!(a.value),&a.pins))})).collect::<Vec<_>>(),"purchase":e.purchase.iter().map(|p|json!({"choice_digest":p.choice_digest,"context_stamp":p.context_stamp,"scenario_plan_digest":p.scenario_plan_digest,"comparison_key":p.comparison_key,"kind":match p.kind{PurchaseKind::ObservedPurchase=>"observed_purchase",PurchaseKind::EstimatedPurchase=>"estimated_purchase"},"observed_at":p.observed_at,"valid_until":p.valid_until,"total_purchase_pence":p.total_purchase_pence,"unused_purchase_value_pence":p.unused_purchase_value_pence,"pins":p.pins.iter().map(pin_json).collect::<Vec<_>>()})).collect::<Vec<_>>()})
}
fn choice_json(c: &ChoiceSpec) -> Value {
    json!({"recipe":c.recipe,"extras":c.extras.iter().map(|e|json!({"extra_id":e.extra_id,"recipe":e.recipe,"recipients":e.recipients})).collect::<Vec<_>>(),"meal_overrides":c.meal_overrides,"batch_use":c.batch_use})
}
fn pool_digest(
    c: &PlanningContext,
    s: &SlotId,
    i: &SelectionIntent,
    choices: &[ChoiceSpec],
) -> Result<Digest, DomainError> {
    let mut encoded = choices
        .iter()
        .map(|c| {
            let v = choice_json(c);
            Ok((canonical_bytes(&v)?, v))
        })
        .collect::<Result<Vec<_>, DomainError>>()?;
    encoded.sort_by(|a, b| a.0.cmp(&b.0));
    require(
        encoded.windows(2).all(|w| w[0].0 != w[1].0),
        ErrorCode::DuplicateKey,
        "/choices",
    )?;
    canonical_digest(
        &json!({"context_digest":canonical_digest(c)?,"slot":s,"intent":{"mode":i.mode,"slots":i.slots,"origin":i.origin,"repeat_permissions":i.repeat_permissions.iter().map(|r|json!({"slot":r.slot,"recipe":r.recipe})).collect::<Vec<_>>()},"choices":encoded.into_iter().map(|(_,v)|v).collect::<Vec<_>>()}),
    )
}
fn ref_key(r: &RecipeRef) -> (&RecipeId, u64, &RecipeDigest) {
    (&r.recipe_id, r.revision.0, &r.digest)
}
fn recipe<'a>(c: &'a PlanningContext, r: &RecipeRef) -> Result<&'a RecipeRevision, DomainError> {
    c.recipes
        .iter()
        .find(|a| {
            a.recipe_id == r.recipe_id
                && a.revision == r.revision
                && recipe_digest(a).is_ok_and(|d| d == r.digest)
        })
        .ok_or_else(|| fail(ErrorCode::ForeignReference, "/recipe"))
}
fn rational_cmp(a: &Rational, b: &Rational) -> Ordering {
    ((a.numerator.0 as u128) * (b.denominator.0 as u128))
        .cmp(&((b.numerator.0 as u128) * (a.denominator.0 as u128)))
}
fn metric_cmp(a: &MetricValue, b: &MetricValue) -> Ordering {
    match (a, b) {
        (MetricValue::Amount(a), MetricValue::Amount(b)) => rational_cmp(a, b),
        (MetricValue::Count(a), MetricValue::Count(b)) => a.cmp(b),
        (MetricValue::Flag(a), MetricValue::Flag(b)) => a.cmp(b),
        (MetricValue::Rating(a), MetricValue::Rating(b)) => a.cmp(b),
        _ => unreachable!("internally fixed metric types"),
    }
}
fn better(k: usize, a: &MetricValue, b: &MetricValue) -> bool {
    metric_cmp(a, b)
        == if [2, 4, 7, 8].contains(&k) {
            Ordering::Greater
        } else {
            Ordering::Less
        }
}
fn check_pins(
    pins: &[EvidencePin],
    seen: &mut BTreeMap<EvidenceId, (Revision, Digest)>,
) -> Result<(), DomainError> {
    bound(pins.len(), 256, "/evidence/pins")?;
    require(
        !pins.is_empty(),
        ErrorCode::MissingReference,
        "/evidence/pins",
    )?;
    require(
        pins.windows(2).all(|w| w[0].evidence_id < w[1].evidence_id),
        ErrorCode::NoncanonicalValue,
        "/evidence/pins",
    )?;
    for p in pins {
        shape(&p.evidence_id, "/evidence/pins/evidence_id")?;
        shape(&p.revision, "/evidence/pins/revision")?;
        shape(&p.digest, "/evidence/pins/digest")?;
        if let Some(old) = seen.insert(p.evidence_id.clone(), (p.revision, p.digest.clone())) {
            require(
                old == (p.revision, p.digest.clone()),
                ErrorCode::InvalidValue,
                "/evidence/pins",
            )?;
        }
    }
    Ok(())
}
// All native collection bounds precede serde/JCS expansion, including stamps in quotes.
fn evidence_bounds(e: &RankingEvidence) -> Result<(), DomainError> {
    bound(e.recipes.len(), 512, "/evidence/recipes")?;
    bound(e.purchase.len(), 512, "/evidence/purchase")?;
    if let Some(c) = &e.categories {
        bound(c.keys.len(), 256, "/evidence/categories/keys")?;
    }
    for r in &e.recipes {
        if let Some(a) = &r.nutrition_digest {
            bound(a.pins.len(), 256, "/evidence/pins")?;
        }
        if let Some(a) = &r.whole_food {
            bound(a.pins.len(), 256, "/evidence/pins")?;
        }
        if let Some(a) = &r.categories {
            bound(a.pins.len(), 256, "/evidence/pins")?;
            bound(a.value.len(), 16, "/evidence/categories")?;
        }
    }
    for p in &e.purchase {
        bound(p.pins.len(), 256, "/evidence/pins")?;
        bound(
            p.context_stamp.recipe_refs.len(),
            512,
            "/evidence/context_stamp/recipe_refs",
        )?;
        bound(
            p.context_stamp.read_weeks.len(),
            4,
            "/evidence/context_stamp/read_weeks",
        )?;
        bound(
            p.context_stamp.write_weeks.len(),
            4,
            "/evidence/context_stamp/write_weeks",
        )?;
    }
    Ok(())
}
fn check_evidence(
    c: &PlanningContext,
    e: &RankingEvidence,
    at: &Instant,
) -> Result<(), DomainError> {
    bound(e.recipes.len(), 512, "/evidence/recipes")?;
    bound(e.purchase.len(), 512, "/evidence/purchase")?;
    shape(&e.revision, "/evidence/revision")?;
    shape(&e.digest, "/evidence/digest")?;
    shape(&e.catalogue_revision, "/evidence/catalogue_revision")?;
    require(
        e.catalogue_revision == c.stamp.catalogue_revision,
        ErrorCode::ContextStale,
        "/evidence/catalogue_revision",
    )?;
    let mut pins = BTreeMap::new();
    if let Some(reg) = &e.categories {
        bound(reg.keys.len(), 256, "/evidence/categories/keys")?;
        shape(&reg.revision, "/evidence/categories/revision")?;
        shape(&reg.digest, "/evidence/categories/digest")?;
        for key in &reg.keys {
            shape(key, "/evidence/categories/keys")?;
        }
        let encoded = reg
            .keys
            .iter()
            .map(canonical_bytes)
            .collect::<Result<Vec<_>, _>>()?;
        require(
            encoded.windows(2).all(|w| w[0] < w[1]),
            ErrorCode::NoncanonicalValue,
            "/evidence/categories/keys",
        )?;
        require(
            reg.digest == canonical_digest(&json!({"revision":reg.revision,"keys":reg.keys}))?,
            ErrorCode::InvalidValue,
            "/evidence/categories/digest",
        )?;
    }
    require(
        e.recipes
            .windows(2)
            .all(|w| ref_key(&w[0].recipe) < ref_key(&w[1].recipe)),
        ErrorCode::NoncanonicalValue,
        "/evidence/recipes",
    )?;
    for fact in &e.recipes {
        shape(&fact.recipe, "/evidence/recipes/recipe")?;
        let actual = recipe(c, &fact.recipe)?;
        if let Some(a) = &fact.nutrition_digest {
            shape(&a.value, "/evidence/recipes/nutrition_digest")?;
            check_pins(&a.pins, &mut pins)?;
            let nutrition = actual.nutrition.0.as_ref().ok_or_else(|| {
                fail(
                    ErrorCode::ForeignReference,
                    "/evidence/recipes/nutrition_digest",
                )
            })?;
            require(
                a.value == canonical_digest(nutrition)?,
                ErrorCode::InvalidValue,
                "/evidence/recipes/nutrition_digest",
            )?;
        }
        if let Some(a) = &fact.whole_food {
            check_pins(&a.pins, &mut pins)?;
        }
        if let Some(a) = &fact.categories {
            bound(a.value.len(), 16, "/evidence/recipes/categories")?;
            check_pins(&a.pins, &mut pins)?;
            let registry = e
                .categories
                .as_ref()
                .ok_or_else(|| fail(ErrorCode::MissingReference, "/evidence/categories"))?;
            let mut prior = None;
            for tag in &a.value {
                shape(tag, "/evidence/recipes/categories")?;
                let key = canonical_bytes(tag)?;
                require(
                    prior.as_ref().is_none_or(|p| p < &key),
                    ErrorCode::NoncanonicalValue,
                    "/evidence/recipes/categories",
                )?;
                require(
                    registry.keys.contains(tag),
                    ErrorCode::ForeignReference,
                    "/evidence/recipes/categories",
                )?;
                prior = Some(key);
            }
        }
    }
    require(
        e.purchase
            .windows(2)
            .all(|w| w[0].choice_digest < w[1].choice_digest),
        ErrorCode::NoncanonicalValue,
        "/evidence/purchase",
    )?;
    for p in &e.purchase {
        shape(&p.choice_digest, "/evidence/purchase/choice_digest")?;
        shape(&p.context_stamp, "/evidence/purchase/context_stamp")?;
        shape(
            &p.scenario_plan_digest,
            "/evidence/purchase/scenario_plan_digest",
        )?;
        shape(&p.comparison_key, "/evidence/purchase/comparison_key")?;
        shape(&p.observed_at, "/evidence/purchase/observed_at")?;
        shape(&p.valid_until, "/evidence/purchase/valid_until")?;
        shape(
            &p.total_purchase_pence,
            "/evidence/purchase/total_purchase_pence",
        )?;
        check_pins(&p.pins, &mut pins)?;
        require(
            p.observed_at.0 <= at.0 && p.observed_at.0 <= p.valid_until.0,
            ErrorCode::InvalidValue,
            "/evidence/purchase/observed_at",
        )?;
        require(
            p.context_stamp == c.stamp,
            ErrorCode::ContextStale,
            "/evidence/purchase/context_stamp",
        )?;
        if let Some(w) = p.unused_purchase_value_pence {
            shape(&w, "/evidence/purchase/unused_purchase_value_pence")?;
            require(
                rational_cmp(&w, &p.total_purchase_pence) != Ordering::Greater,
                ErrorCode::InvalidValue,
                "/evidence/purchase/unused_purchase_value_pence",
            )?;
        }
    }
    require(
        e.digest == canonical_digest(&evidence_material_unchecked(e))?,
        ErrorCode::InvalidValue,
        "/evidence/digest",
    )
}
fn fact<'a>(e: &'a RankingEvidence, r: &RecipeRef) -> Option<&'a RecipeRankFacts> {
    e.recipes.iter().find(|f| f.recipe == *r)
}
fn active(c: &PlanningContext) -> Vec<&MealSelection> {
    c.weeks
        .iter()
        .flat_map(|w| w.plan.selections.iter())
        .filter(|s| {
            c.requirements.iter().any(|m| {
                m.slot == s.slot
                    && m.enabled
                    && m.location != MealLocation::Skip
                    && !m.attendees.is_empty()
            })
        })
        .collect()
}
fn ingredients(
    c: &PlanningContext,
    s: &MealSelection,
) -> Result<BTreeSet<IngredientId>, DomainError> {
    let mut ids = BTreeSet::new();
    for r in std::iter::once(&s.recipe).chain(s.extras.iter().map(|e| &e.recipe)) {
        for i in &recipe(c, r)?.ingredients.0 {
            ids.insert(i.ingredient_id.clone());
        }
    }
    Ok(ids)
}
fn occurrences(
    c: &PlanningContext,
    slot: &SlotId,
) -> Result<Vec<(SlotId, RecipeRef)>, DomainError> {
    let target = chrono::NaiveDate::parse_from_str(&slot.date.0, "%Y-%m-%d")
        .map_err(|_| fail(ErrorCode::InvalidValue, "/slot/date"))?;
    let in_window = |s: &SlotId| {
        chrono::NaiveDate::parse_from_str(&s.date.0, "%Y-%m-%d").is_ok_and(|d| {
            let days = (target - d).num_days();
            days > 0 && days <= 28
        })
    };
    let mut rows = vec![];
    for s in active(c) {
        if s.slot != *slot {
            rows.push((s.slot.clone(), s.recipe.clone()));
        }
    }
    for s in &c.history.selected.0 {
        if s.slot != *slot
            && in_window(&s.slot)
            && !c.weeks.iter().any(|w| w.range.week_id == s.slot.week_id)
        {
            rows.push((s.slot.clone(), s.recipe.clone()));
        }
    }
    for o in &c.history.ordered.0 {
        let s = &o.selection;
        if s.slot != *slot && in_window(&s.slot) {
            rows.push((s.slot.clone(), s.recipe.clone()));
        }
    }
    let mut seen = BTreeSet::new();
    rows.retain(|(s, r)| {
        seen.insert((
            canonical_bytes(s).expect("validated slot"),
            r.recipe_id.clone(),
        ))
    });
    Ok(rows)
}
fn history_times(c: &PlanningContext, at: &Instant) -> Result<(), DomainError> {
    for row in &c.history.selected.0 {
        require(
            row.selected_at.0 <= at.0,
            ErrorCode::InvalidValue,
            "/history/selected/selected_at",
        )?;
    }
    for row in &c.history.ordered.0 {
        require(
            row.selection.selected_at.0 <= row.ordered_at.0 && row.ordered_at.0 <= at.0,
            ErrorCode::InvalidValue,
            "/history/ordered/ordered_at",
        )?;
    }
    let mut latest: BTreeMap<Vec<u8>, (&Instant, FeedbackRating, bool)> = BTreeMap::new();
    for row in &c.history.feedback.0 {
        require(
            row.recorded_at.0 <= at.0,
            ErrorCode::InvalidValue,
            "/history/feedback/recorded_at",
        )?;
        let entry = latest.entry(canonical_bytes(&row.recipe)?).or_insert((
            &row.recorded_at,
            row.rating,
            false,
        ));
        if row.recorded_at.0 > entry.0.0 {
            *entry = (&row.recorded_at, row.rating, false);
        } else if row.recorded_at.0 == entry.0.0 && row.rating != entry.1 {
            entry.2 = true;
        }
    }
    require(
        latest.values().all(|(_, _, contradiction)| !*contradiction),
        ErrorCode::InvalidValue,
        "/history/feedback",
    )
}
fn feedback(c: &PlanningContext, r: &RecipeRef) -> Result<i8, DomainError> {
    let rows: Vec<_> = c
        .history
        .feedback
        .iter()
        .filter(|f| f.recipe == *r)
        .collect();
    let Some(latest) = rows.iter().map(|f| &f.recorded_at.0).max() else {
        return Ok(0);
    };
    let newest: Vec<_> = rows.iter().filter(|f| &f.recorded_at.0 == latest).collect();
    require(
        newest.iter().all(|f| f.rating == newest[0].rating),
        ErrorCode::InvalidValue,
        "/history/feedback",
    )?;
    Ok(match newest[0].rating {
        FeedbackRating::Liked => 1,
        FeedbackRating::Okay => 0,
        FeedbackRating::NotForMe => -1,
    })
}
fn scale_protein(
    protein: Rational,
    servings: Rational,
    per: Rational,
) -> Result<Rational, DomainError> {
    // Contract inputs are validated first. Cancel the complete expression before
    // bounding its result; an unrepresentable reciprocal is not itself an error.
    let n = (protein.numerator.0 as u128)
        .checked_mul(servings.numerator.0 as u128)
        .and_then(|x| x.checked_mul(per.denominator.0 as u128))
        .ok_or_else(|| fail(ErrorCode::ArithmeticOverflow, "/nutrition"))?;
    let d = (protein.denominator.0 as u128)
        .checked_mul(servings.denominator.0 as u128)
        .and_then(|x| x.checked_mul(per.numerator.0 as u128))
        .ok_or_else(|| fail(ErrorCode::ArithmeticOverflow, "/nutrition"))?;
    require(d > 0, ErrorCode::InvalidValue, "/nutrition")?;
    let (mut a, mut b) = (n, d);
    while b != 0 {
        let rem = a % b;
        a = b;
        b = rem;
    }
    let (n, d) = (n / a, d / a);
    require(
        n <= MAX_NUMERATOR as u128 && d <= MAX_DENOMINATOR as u128,
        ErrorCode::LimitExceeded,
        "/nutrition",
    )?;
    Rational::new(n as u64, d as u64)
}
fn values(
    c: &PlanningContext,
    s: &MealSelection,
    e: &RankingEvidence,
    history: &[(SlotId, RecipeRef)],
    retained: &BTreeSet<IngredientId>,
) -> Result<Vec<Option<MetricValue>>, DomainError> {
    let ids = ingredients(c, s)?;
    let mut minutes = 0u32;
    let mut protein = Some(Rational::new(0, 1)?);
    let mut tags = vec![];
    for (rf, servings) in std::iter::once((&s.recipe, &s.servings))
        .chain(s.extras.iter().map(|e| (&e.recipe, &e.recipients)))
    {
        let r = recipe(c, rf)?;
        minutes += u32::from(r.active_minutes);
        let f = fact(e, rf);
        tags.push(f.and_then(|f| f.whole_food.as_ref()).map(|a| a.value));
        let p = match (
            r.nutrition.0.as_ref(),
            f.and_then(|f| f.nutrition_digest.as_ref()),
        ) {
            (Some(n), Some(_)) => {
                if let Some(p) = n.protein_g.0 {
                    let amount = servings
                        .iter()
                        .try_fold(Rational::new(0, 1)?, |a, s| a.checked_add(&s.servings.0))?;
                    Some(scale_protein(p, amount, n.per_servings.0)?)
                } else {
                    None
                }
            }
            _ => None,
        };
        protein = match (protein, p) {
            (Some(a), Some(b)) => Some(a.checked_add(&b)?),
            _ => None,
        };
    }
    let whole = if tags.contains(&Some(false)) {
        Some(false)
    } else if tags.iter().all(|x| *x == Some(true)) {
        Some(true)
    } else {
        None
    };
    let categories = fact(e, &s.recipe).and_then(|f| f.categories.as_ref());
    let category = if let Some(own) = categories {
        let mut count = 0;
        let mut complete = true;
        for (_, r) in history {
            if let Some(other) = fact(e, r).and_then(|f| f.categories.as_ref()) {
                if own.value.iter().any(|t| other.value.contains(t)) {
                    count += 1;
                }
            } else {
                complete = false;
            }
        }
        if complete { Some(count) } else { None }
    } else {
        None
    };
    let mut dislikes: BTreeSet<_> = c.household.preferences.dislikes.iter().cloned().collect();
    for d in &c.household.diners.0 {
        if s.servings.iter().any(|a| {
            a.person
                == PersonRef::Diner {
                    diner_id: d.diner_id.clone(),
                }
        }) {
            dislikes.extend(d.dislikes.iter().cloned());
        }
    }
    Ok(vec![
        None,
        None,
        Some(MetricValue::Count(ids.intersection(retained).count() as u32)),
        Some(MetricValue::Count(minutes)),
        protein.map(MetricValue::Amount),
        Some(MetricValue::Count(
            history
                .iter()
                .filter(|(_, r)| r.recipe_id == s.recipe.recipe_id)
                .count() as u32,
        )),
        category.map(MetricValue::Count),
        whole.map(MetricValue::Flag),
        Some(MetricValue::Rating(feedback(c, &s.recipe)?)),
        Some(MetricValue::Count(
            ids.intersection(&dislikes).count() as u32
        )),
    ])
}
fn scenario(c: &PlanningContext, s: &MealSelection) -> Result<PlanInputDigest, DomainError> {
    let mut w = c
        .weeks
        .iter()
        .find(|w| w.range.week_id == s.slot.week_id)
        .ok_or_else(|| fail(ErrorCode::MissingReference, "/slot"))?
        .plan
        .clone();
    // Replace in place: unrelated selection order is part of the supplied material.
    if let Some(existing) = w.selections.0.iter_mut().find(|x| x.slot == s.slot) {
        *existing = s.clone();
    } else {
        w.selections.0.push(s.clone());
    }
    plan_input_digest(&PlanInputMaterial {
        schema_version: c.schema_version,
        week_plan: w,
        context_stamp: c.stamp.clone(),
    })
}

pub fn rank_candidates(
    c: &PlanningContext,
    slot: &SlotId,
    intent: &SelectionIntent,
    choices: &[ChoiceSpec],
    reviewed: &[ReviewedRecipe<'_>],
    e: &RankingEvidence,
    at: &Instant,
) -> Result<CandidateSet, DomainError> {
    rank_candidate_pool(c, slot, intent, choices, reviewed, e, at, false)
}

/// Local explicit repeat authorisation applies independently to each candidate.
/// Final selection still needs one permission for the chosen recipe and slot.
/// Assessment and scoring retain the complete pool; no per-candidate ranking
/// scores are combined, and the ordinary intent guard remains unchanged.
pub fn rank_candidates_with_repeat_authorisation(
    c: &PlanningContext,
    slot: &SlotId,
    intent: &SelectionIntent,
    choices: &[ChoiceSpec],
    reviewed: &[ReviewedRecipe<'_>],
    e: &RankingEvidence,
    at: &Instant,
) -> Result<CandidateSet, DomainError> {
    require(intent.origin == SelectionOrigin::Suggested
        && intent.slots.as_slice() == std::slice::from_ref(slot)
        && intent.repeat_permissions.is_empty(),
        ErrorCode::InvalidValue, "/intent/repeat_permissions")?;
    rank_candidate_pool(c, slot, intent, choices, reviewed, e, at, true)
}

fn rank_candidate_pool(
    c: &PlanningContext,
    slot: &SlotId,
    intent: &SelectionIntent,
    choices: &[ChoiceSpec],
    reviewed: &[ReviewedRecipe<'_>],
    e: &RankingEvidence,
    at: &Instant,
    authorise_each_candidate: bool,
) -> Result<CandidateSet, DomainError> {
    bound(choices.len(), 512, "/choices")?;
    bound(reviewed.len(), 512, "/reviewed")?;
    bound(c.weeks.len(), 4, "/weeks")?;
    bound(c.recipes.len(), 512, "/recipes")?;
    bound(c.requirements.len(), 84, "/requirements")?;
    bound(c.history.selected.len(), 4096, "/history/selected")?;
    bound(c.history.ordered.len(), 4096, "/history/ordered")?;
    bound(c.history.feedback.len(), 4096, "/history/feedback")?;
    validate_selection_context(c)?;
    validate_intent(c, intent)?;
    validate_current_selected(&c.history)?;
    shape(slot, "/slot")?;
    shape(at, "/evaluated_at")?;
    history_times(c, at)?;
    let target_week = c
        .weeks
        .iter()
        .find(|w| w.range.week_id == slot.week_id)
        .ok_or_else(|| fail(ErrorCode::ForeignReference, "/slot"))?;
    require(
        active_slots(target_week)?.contains(slot),
        ErrorCode::DomainConflict,
        "/slot",
    )?;
    require(
        intent.slots.contains(slot),
        ErrorCode::ForeignReference,
        "/slot",
    )?;
    require(
        c.ranking_policy.version.0 == 1 && c.ranking_policy.digest.0 == POLICY_DIGEST,
        ErrorCode::UnsupportedCapability,
        "/ranking_policy",
    )?;
    evidence_bounds(e)?;
    check_evidence(c, e, at)?;
    for ch in choices {
        if let BatchUse::Leftover { component_ids, .. } = &ch.batch_use {
            bound(component_ids.len(), 128, "/choices/batch_use/component_ids")?;
        }
        bound(ch.extras.len(), 64, "/choices/extras")?;
        for ex in &ch.extras {
            bound(ex.recipients.len(), 64, "/choices/extras/recipients")?;
        }
        bound(
            ch.meal_overrides.portions.len(),
            64,
            "/choices/meal_overrides",
        )?;
    }
    let mut pool = pool_digest(c, slot, intent, choices)?;
    if authorise_each_candidate {
        pool = canonical_digest(&json!({"pool_digest":pool,"repeat_authorisation":"each_candidate"}))?;
    }
    let mut out = CandidateSet {
        slot: slot.clone(),
        context_stamp: c.stamp.clone(),
        evidence_revision: e.revision,
        evidence_digest: e.digest.clone(),
        pool_digest: pool,
        evaluated_at: at.clone(),
        ranked: vec![],
        withheld: vec![],
        refused: vec![],
        unavailable: vec![],
    };
    let mut all = BTreeMap::new();
    for (index, ch) in choices.iter().enumerate() {
        if authorise_each_candidate {
            let mut candidate_intent = intent.clone();
            candidate_intent.repeat_permissions = vec![crate::selection::RepeatPermission {
                slot: slot.clone(), recipe: ch.recipe.clone(),
            }];
            validate_intent(c, &candidate_intent)?;
        }
        let permit = authorise_each_candidate || intent
            .repeat_permissions
            .iter()
            .any(|p| p.slot == *slot && p.recipe == ch.recipe)
            || c.weeks
                .iter()
                .flat_map(|w| w.plan.selections.iter())
                .any(|s| s.slot == *slot && s.recipe == ch.recipe && s.repeat_authorised);
        match assess_choice_in_validated_context(c, slot, ch, intent.origin, permit, reviewed)? {
            ChoiceAssessment::Suitable(s) => {
                let digest = canonical_digest(&s)?;
                all.insert(digest.clone(), s.clone());
                let reason = if intent.origin == SelectionOrigin::Suggested
                    && c.history.hidden_recipe_ids.contains(&s.recipe.recipe_id)
                {
                    Some(WithheldReason::Hidden)
                } else if !permit && repeat_needs_permission(c, &s)? {
                    Some(WithheldReason::RepeatNeedsPermission)
                } else {
                    None
                };
                if let Some(reason) = reason {
                    out.withheld.push(WithheldChoice {
                        choice_digest: digest,
                        selection: s,
                        reason,
                    });
                } else {
                    out.ranked.push(RankedChoice {
                        choice_digest: digest,
                        selection: s,
                        score: 0,
                        metrics: vec![],
                        compromises: vec![],
                    });
                }
            }
            outcome => out.refused.push(ChoiceRefusal {
                index: index as u16,
                outcome,
            }),
        }
    }
    for p in &e.purchase {
        let s = all.get(&p.choice_digest).ok_or_else(|| {
            fail(
                ErrorCode::ForeignReference,
                "/evidence/purchase/choice_digest",
            )
        })?;
        require(
            p.scenario_plan_digest == scenario(c, s)?,
            ErrorCode::InvalidValue,
            "/evidence/purchase/scenario_plan_digest",
        )?;
    }
    let history = occurrences(c, slot)?;
    let mut retained = BTreeSet::new();
    for s in active(c) {
        if s.slot.week_id == slot.week_id && s.slot != *slot {
            retained.extend(ingredients(c, s)?);
        }
    }
    for r in &mut out.ranked {
        r.metrics = values(c, &r.selection, e, &history, &retained)?
            .into_iter()
            .enumerate()
            .map(|(k, value)| MetricReading {
                metric: METRICS[k],
                value,
            })
            .collect();
        if let Some(p) = e
            .purchase
            .iter()
            .find(|p| p.choice_digest == r.choice_digest)
        {
            r.metrics[0].value = Some(MetricValue::Amount(p.total_purchase_pence));
            r.metrics[1].value = p.unused_purchase_value_pence.map(MetricValue::Amount);
        }
    }
    let prefs = &c.household.preferences;
    let weights: [u32; 10] = [
        match prefs.spending_style {
            SpendingStyle::Value => 4,
            SpendingStyle::Balanced => 2,
            SpendingStyle::Flexible => 0,
        },
        1,
        if prefs.spending_style == SpendingStyle::Value {
            2
        } else {
            1
        },
        if prefs.quick_meals { 4 } else { 0 },
        if prefs.higher_protein { 4 } else { 0 },
        if prefs.more_variety { 4 } else { 0 },
        if prefs.more_variety { 2 } else { 0 },
        if prefs.whole_food_preference { 3 } else { 0 },
        1,
        1,
    ];
    let mut enabled = [true; 10];
    for k in 0..10 {
        if weights[k] == 0 {
            enabled[k] = false;
            continue;
        }
        let mut causes: [BTreeSet<Digest>; 3] = std::array::from_fn(|_| BTreeSet::new());
        for r in &out.ranked {
            if r.metrics[k].value.is_none() {
                causes[0].insert(r.choice_digest.clone());
            }
            if k < 2
                && let Some(p) = e
                    .purchase
                    .iter()
                    .find(|p| p.choice_digest == r.choice_digest)
                && at.0 > p.valid_until.0
            {
                causes[1].insert(r.choice_digest.clone());
            }
        }
        if k < 2 {
            let quotes: Vec<_> = out
                .ranked
                .iter()
                .filter_map(|r| {
                    e.purchase
                        .iter()
                        .find(|p| p.choice_digest == r.choice_digest)
                })
                .collect();
            if let Some(first) = quotes.first()
                && quotes
                    .iter()
                    .any(|p| p.comparison_key != first.comparison_key || p.kind != first.kind)
            {
                for r in &out.ranked {
                    if quotes.iter().any(|q| q.choice_digest == r.choice_digest) {
                        causes[2].insert(r.choice_digest.clone());
                    }
                }
            }
        }
        if causes.iter().any(|c| !c.is_empty()) {
            enabled[k] = false;
            out.unavailable.push(MetricUnavailable {
                metric: METRICS[k],
                causes: causes
                    .into_iter()
                    .enumerate()
                    .filter(|(_, s)| !s.is_empty())
                    .map(|(i, s)| MetricUnavailableCause {
                        reason: [
                            MetricUnavailableReason::MissingEvidence,
                            MetricUnavailableReason::ExpiredEvidence,
                            MetricUnavailableReason::NoncomparableEvidence,
                        ][i],
                        choices: s.into_iter().collect(),
                    })
                    .collect(),
            });
        }
    }
    // At most 511 strict wins on each of ten metrics, every weight <=4:
    // score <=20_440; u32 sums and products cannot overflow these admitted bounds.
    for i in 0..out.ranked.len() {
        let score = (0..10).filter(|k| enabled[*k]).try_fold(0u32, |sum, k| {
            let wins = out
                .ranked
                .iter()
                .filter(|other| {
                    better(
                        k,
                        out.ranked[i].metrics[k].value.as_ref().unwrap(),
                        other.metrics[k].value.as_ref().unwrap(),
                    )
                })
                .count() as u32;
            let contribution = weights[k]
                .checked_mul(wins)
                .ok_or_else(|| fail(ErrorCode::ArithmeticOverflow, "/ranking/score"))?;
            sum.checked_add(contribution)
                .ok_or_else(|| fail(ErrorCode::ArithmeticOverflow, "/ranking/score"))
        })?;
        out.ranked[i].score = score;
    }
    out.ranked.sort_by(|a, b| {
        b.score
            .cmp(&a.score)
            .then(
                ref_key(&a.selection.recipe)
                    .0
                    .cmp(ref_key(&b.selection.recipe).0),
            )
            .then(
                a.selection
                    .recipe
                    .revision
                    .0
                    .cmp(&b.selection.recipe.revision.0),
            )
            .then(a.choice_digest.cmp(&b.choice_digest))
    });
    for i in 0..out.ranked.len() {
        let mut compromises = vec![];
        for k in 0..10 {
            if !enabled[k] {
                continue;
            }
            let chosen = out.ranked[i].metrics[k].value.as_ref().unwrap();
            if let Some(alt) = out
                .ranked
                .iter()
                .find(|r| better(k, r.metrics[k].value.as_ref().unwrap(), chosen))
            {
                let alternative_value = alt.metrics[k].value.as_ref().unwrap();
                let alternative_wins = out
                    .ranked
                    .iter()
                    .filter(|r| better(k, alternative_value, r.metrics[k].value.as_ref().unwrap()))
                    .count();
                let chosen_wins = out
                    .ranked
                    .iter()
                    .filter(|r| better(k, chosen, r.metrics[k].value.as_ref().unwrap()))
                    .count();
                let difference = alternative_wins
                    .checked_sub(chosen_wins)
                    .ok_or_else(|| fail(ErrorCode::ArithmeticOverflow, "/ranking/compromises"))?;
                let lost = weights[k]
                    .checked_mul(difference as u32)
                    .ok_or_else(|| fail(ErrorCode::ArithmeticOverflow, "/ranking/compromises"))?;
                compromises.push((
                    lost,
                    k,
                    Compromise {
                        metric: METRICS[k],
                        chosen: chosen.clone(),
                        alternative: alt.metrics[k].value.clone().unwrap(),
                        alternative_recipe: alt.selection.recipe.clone(),
                        alternative_choice: alt.choice_digest.clone(),
                        evidence_digest: e.digest.clone(),
                    },
                ));
            }
        }
        compromises.sort_by(|a, b| b.0.cmp(&a.0).then(a.1.cmp(&b.1)));
        out.ranked[i].compromises = compromises.into_iter().take(3).map(|(_, _, c)| c).collect();
    }
    out.withheld
        .sort_by(|a, b| a.choice_digest.cmp(&b.choice_digest));
    Ok(out)
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn nutrition_cancels_before_bounded_rational_construction() {
        assert_eq!(
            scale_protein(
                Rational::new(1_000_001, 1).unwrap(),
                Rational::new(2, 1).unwrap(),
                Rational::new(1_000_001, 1).unwrap()
            )
            .unwrap(),
            Rational::new(2, 1).unwrap()
        );
        assert_eq!(
            scale_protein(
                Rational::new(1, 3).unwrap(),
                Rational::new(3, 2).unwrap(),
                Rational::new(2, 1).unwrap()
            )
            .unwrap(),
            Rational::new(1, 4).unwrap()
        );
    }
}
