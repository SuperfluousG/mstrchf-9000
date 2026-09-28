use crate::*;
use serde::Serialize;
use sha2::{Digest as ShaDigest, Sha256};
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
    Ok(RecipeDigest(canonical_digest(v)?.0))
}
pub fn result_digest(v: &JobResultV1) -> Result<ResultDigest, DomainError> {
    Ok(ResultDigest(canonical_digest(v)?.0))
}
record! {pub struct PlanInputMaterial {pub schema_version:SchemaVersion,pub week_plan:WeekPlan,pub context_stamp:ContextStamp}}
record! {pub struct DemandMaterial {pub schema_version:SchemaVersion,pub input_plan_digest:PlanInputDigest,pub lines:BoundedVec<DemandLine,0,4096>}}
record! {pub struct DerivedMaterial {pub schema_version:SchemaVersion,pub week_id:WeekId,pub plan_revision:Revision,pub input_plan_digest:PlanInputDigest,pub context_stamp:ContextStamp,pub recipe_refs:BoundedVec<RecipeRef,0,512>,pub batches:BoundedVec<DerivedBatch,0,256>,pub allocations:BoundedVec<Allocation,0,4096>,pub prep_sessions:BoundedVec<PrepSession,0,256>,pub demand:DemandSet,pub warnings:BoundedVec<Conflict,0,100>,pub derivation_policy_version:Revision}}
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
