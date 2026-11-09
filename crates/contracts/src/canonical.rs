use crate::*;
use serde::Serialize;
use sha2::{Digest as ShaDigest, Sha256};
use std::{cell::RefCell, collections::VecDeque};

const RECIPE_DIGEST_MEMO_ENTRIES: usize = 64;
const RECIPE_DIGEST_MEMO_BYTES: usize = 1024 * 1024;
struct RecipeDigestMemo {
    entries: VecDeque<(RecipeRevision, RecipeDigest, usize)>,
    canonical_bytes: usize,
}
thread_local! {
    // Pure hash reuse, never recipe admission. Every field is compared with an
    // owned prior value; an ID, supplied digest or reused address is not a key.
    // The byte budget counts canonical bytes, not exact allocator heap usage.
    static RECIPE_DIGEST_MEMO: RefCell<RecipeDigestMemo> = const { RefCell::new(RecipeDigestMemo {
        entries: VecDeque::new(), canonical_bytes: 0,
    }) };
}
pub fn bytes_digest(bytes: &[u8]) -> Digest {
    Digest(
        Sha256::digest(bytes)
            .iter()
            .map(|b| format!("{b:02x}"))
            .collect(),
    )
}
pub fn canonical_bytes<T: Serialize>(value: &T) -> Result<Vec<u8>, DomainError> {
    serde_jcs::to_vec(value).map_err(|_| DomainError::new(ErrorCode::InvalidValue, ""))
}
pub fn canonical_digest<T: Serialize>(value: &T) -> Result<Digest, DomainError> {
    Ok(bytes_digest(&canonical_bytes(value)?))
}
pub fn input_digest(v: &RequestMaterial) -> Result<InputDigest, DomainError> {
    Ok(InputDigest(canonical_digest(v)?.0))
}
pub fn recipe_digest(v: &RecipeRevision) -> Result<RecipeDigest, DomainError> {
    if let Some(digest) = RECIPE_DIGEST_MEMO.with(|memo| {
        let mut memo = memo.borrow_mut();
        let position = memo.entries.iter().position(|(recipe, _, _)| recipe == v)?;
        let entry = memo.entries.remove(position)?;
        let digest = entry.1.clone();
        memo.entries.push_back(entry);
        Some(digest)
    }) {
        return Ok(digest);
    }
    // No memo borrow crosses serialisation. Failure has exactly the original
    // result and cannot install a reusable entry. Shape/evidence checks remain
    // the callers' responsibility, exactly as for canonical_digest itself.
    let bytes = canonical_bytes(v)?;
    let digest = RecipeDigest(bytes_digest(&bytes).0);
    if bytes.len() <= RECIPE_DIGEST_MEMO_BYTES {
        RECIPE_DIGEST_MEMO.with(|memo| {
            let mut memo = memo.borrow_mut();
            while memo.entries.len() >= RECIPE_DIGEST_MEMO_ENTRIES
                || memo.canonical_bytes + bytes.len() > RECIPE_DIGEST_MEMO_BYTES
            {
                if let Some((_, _, size)) = memo.entries.pop_front() {
                    memo.canonical_bytes -= size;
                } else {
                    break;
                }
            }
            memo.canonical_bytes += bytes.len();
            memo.entries
                .push_back((v.clone(), digest.clone(), bytes.len()));
        });
    }
    Ok(digest)
}

#[cfg(test)]
mod recipe_digest_memo_tests {
    use super::*;
    fn recipe() -> RecipeRevision {
        parse_document(include_bytes!(
            "../../../tests/fixtures/food/admission/ruling004-current--complete-synthetic.json"
        ))
        .unwrap()
    }
    fn clear() {
        RECIPE_DIGEST_MEMO.with(|memo| {
            *memo.borrow_mut() = RecipeDigestMemo {
                entries: VecDeque::new(),
                canonical_bytes: 0,
            }
        });
    }
    fn unchanged_hash(value: &RecipeRevision) -> RecipeDigest {
        RecipeDigest(canonical_digest(value).unwrap().0)
    }
    #[test]
    fn same_identity_changed_food_or_provenance_never_reuses_an_old_digest() {
        clear();
        let original = recipe();
        let expected = unchanged_hash(&original);
        assert_eq!(recipe_digest(&original).unwrap(), expected);
        let mut cases = vec![];
        let mut changed = original.clone();
        changed.title.0.push_str(" changed");
        cases.push(changed);
        let mut changed = original.clone();
        changed.ingredients.0[0].quantity.0.as_mut().unwrap().amount =
            PositiveRational::new(2, 1).unwrap();
        cases.push(changed);
        let mut changed = original.clone();
        changed.source.retrieved_at = Instant("2026-12-29T12:00:00Z".into());
        cases.push(changed);
        let mut changed = original.clone();
        changed.text_rights.attribution.0.push_str(" changed");
        cases.push(changed);
        let mut changed = original.clone();
        changed.elapsed_minutes += 1;
        cases.push(changed);
        for changed in cases {
            assert_eq!(changed.recipe_id, original.recipe_id);
            assert_eq!(changed.revision, original.revision);
            let actual = recipe_digest(&changed).unwrap();
            assert_ne!(actual, expected);
            assert_eq!(actual, unchanged_hash(&changed));
            assert_eq!(recipe_digest(&changed.clone()).unwrap(), actual);
        }
        // Caller mutation and cache churn cannot mutate the owned prior value.
        assert_eq!(recipe_digest(&original).unwrap(), expected);
    }
    #[test]
    fn retention_is_bounded_and_evicted_values_keep_their_exact_hash() {
        clear();
        let original = recipe();
        let expected = unchanged_hash(&original);
        for i in 0..RECIPE_DIGEST_MEMO_ENTRIES + 8 {
            let mut value = original.clone();
            value.title.0 = format!("Synthetic digest variant {i}");
            assert_eq!(recipe_digest(&value).unwrap(), unchanged_hash(&value));
            RECIPE_DIGEST_MEMO.with(|memo| {
                let memo = memo.borrow();
                assert!(memo.entries.len() <= RECIPE_DIGEST_MEMO_ENTRIES);
                assert!(memo.canonical_bytes <= RECIPE_DIGEST_MEMO_BYTES);
                assert_eq!(
                    memo.canonical_bytes,
                    memo.entries.iter().map(|entry| entry.2).sum::<usize>()
                );
            });
        }
        assert_eq!(recipe_digest(&original).unwrap(), expected);
        let mut oversized = original.clone();
        oversized.title.0 = "x".repeat(RECIPE_DIGEST_MEMO_BYTES + 1);
        let before = RECIPE_DIGEST_MEMO.with(|memo| memo.borrow().entries.len());
        assert_eq!(
            recipe_digest(&oversized).unwrap(),
            unchanged_hash(&oversized)
        );
        RECIPE_DIGEST_MEMO.with(|memo| assert_eq!(memo.borrow().entries.len(), before));
        // A digest, cached or uncached, does not confer valid wire shape.
        assert!(oversized.validate_shape().is_err());
    }
    #[test]
    fn callers_cannot_mutate_owned_cache_entries_and_byte_budget_evicts() {
        clear();
        let mut value = recipe();
        let original = value.clone();
        let expected = unchanged_hash(&original);
        let mut returned = recipe_digest(&value).unwrap();
        returned.0.clear();
        value.title.0.push_str(" mutated in the same allocation");
        assert_eq!(recipe_digest(&value).unwrap(), unchanged_hash(&value));
        assert_eq!(recipe_digest(&original).unwrap(), expected);
        clear();
        for i in 0..5 {
            let mut large = original.clone();
            large.title.0 = format!("{i}{}", "x".repeat(RECIPE_DIGEST_MEMO_BYTES / 3));
            assert_eq!(recipe_digest(&large).unwrap(), unchanged_hash(&large));
        }
        RECIPE_DIGEST_MEMO.with(|memo| {
            let memo = memo.borrow();
            assert!(memo.entries.len() < 5);
            assert!(memo.canonical_bytes <= RECIPE_DIGEST_MEMO_BYTES);
        });
        assert_eq!(recipe_digest(&original).unwrap(), expected);
    }
}
pub fn result_digest(v: &JobResultV1) -> Result<ResultDigest, DomainError> {
    Ok(ResultDigest(canonical_digest(v)?.0))
}
record! {pub struct PlanInputMaterial {pub schema_version:SchemaVersion,pub week_plan:WeekPlan,pub context_stamp:ContextStamp}}
record! {pub struct DemandMaterial {pub schema_version:SchemaVersion,pub input_plan_digest:PlanInputDigest,pub lines:BoundedVec<DemandLine,0,4096>}}
record! {pub struct DerivedMaterial {pub schema_version:SchemaVersion,pub week_id:WeekId,pub plan_revision:Revision,pub input_plan_digest:PlanInputDigest,pub food_inputs_digest:PayloadDigest,pub context_stamp:ContextStamp,pub recipe_refs:BoundedVec<RecipeRef,0,512>,pub batches:BoundedVec<DerivedBatch,0,256>,pub allocations:BoundedVec<Allocation,0,4096>,pub prep_sessions:BoundedVec<PrepSession,0,256>,pub demand:DemandSet,pub warnings:BoundedVec<Conflict,0,100>,pub derivation_policy_version:Revision}}
pub fn plan_input_digest(v: &PlanInputMaterial) -> Result<PlanInputDigest, DomainError> {
    Ok(PlanInputDigest(canonical_digest(v)?.0))
}
pub fn demand_digest(v: &DemandMaterial) -> Result<DemandDigest, DomainError> {
    Ok(DemandDigest(canonical_digest(v)?.0))
}
pub fn derived_digest(v: &DerivedMaterial) -> Result<PayloadDigest, DomainError> {
    Ok(PayloadDigest(canonical_digest(v)?.0))
}
impl From<&DerivedPlan> for DerivedMaterial {
    fn from(v: &DerivedPlan) -> Self {
        Self {
            schema_version: v.schema_version,
            week_id: v.week_id.clone(),
            plan_revision: v.plan_revision,
            input_plan_digest: v.input_plan_digest.clone(),
            food_inputs_digest: v.food_inputs_digest.clone(),
            context_stamp: v.context_stamp.clone(),
            recipe_refs: v.recipe_refs.clone(),
            batches: v.batches.clone(),
            allocations: v.allocations.clone(),
            prep_sessions: v.prep_sessions.clone(),
            demand: v.demand.clone(),
            warnings: v.warnings.clone(),
            derivation_policy_version: v.derivation_policy_version,
        }
    }
}
