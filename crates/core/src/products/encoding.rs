use super::*;
use serde_json::{Value, json};

macro_rules! record { ($t:ty {$($f:ident),*}) => {impl NativeValue for $t {fn native(&self)->Value {json!({$(stringify!($f):self.$f.native()),*})}}}; }
macro_rules! units { ($t:ident {$($v:ident:$s:literal),*}) => {impl NativeValue for $t {fn native(&self)->Value {json!({"kind":match self {$(Self::$v=>$s),*}})}}}; }
// Existing wire values retain their frozen encodings; native records are explicit.
macro_rules! wire {($($t:ty),*)=>{$(impl NativeValue for $t{fn native(&self)->Value{json!(self)}})*};}
wire!(
    HardRequirement,
    Quantity,
    ConversionEvidence,
    DemandLine,
    DemandDigest,
    ContextStamp
);
units!(SourceField {
    Ingredients: "ingredients",
    Allergens: "allergens",
    Storage: "storage"
});
units!(ClaimState {
    Supported: "supported",
    Refuted: "refuted",
    Unknown: "unknown",
    Contradictory: "contradictory"
});
record!(RuleBinding {
    requirement,
    required_fields,
    review
});
record!(IdentityClaim {
    supply_key,
    state,
    evidence_ids,
    source_fields
});
record!(RequirementClaim {
    requirement,
    state,
    evidence_ids,
    source_fields
});
record!(CookingClaim {
    recipe,
    occurrence,
    state,
    evidence_ids,
    source_fields
});
record!(ProductMeaning {
    product,
    observation_digest,
    key_claim,
    requirement_claims,
    cooking_claims
});
record!(SupplyOrigin {
    occurrence,
    batch_id,
    component_id,
    ingredient_id,
    original_quantity,
    relation
});
record!(MatchUse {
    use_key,
    at,
    requirements,
    cooking
});
record!(MatchBasis {
    saved_basis_digest,
    derived_digest,
    demand_digest,
    context,
    policy_digest,
    line_id,
    line,
    product,
    observation_digest,
    meaning_digest,
    review,
    evidence,
    freshness_rule,
    item_observed_at,
    read_observed_at,
    read_admitted_at,
    meaning_admitted_at,
    valid_until,
    evaluated_at,
    uses,
    supply_key,
    origins
});
impl NativeValue for SupplyRelation {
    fn native(&self) -> Value {
        match self {
            Self::Exact => json!({"kind":"exact"}),
            Self::ForwardConversion { evidence } => {
                json!({"kind":"forward_conversion","data":{"evidence":evidence.native()}})
            }
        }
    }
}
pub fn policy_digest(v: &MatchPolicy) -> Result<PayloadDigest, DomainError> {
    validation::policy(v)?;
    hash_native(
        "mstrchf-product-match-policy-v1",
        json!({"version":v.version,"rules":v.rules.native()}),
    )
}
pub fn meaning_digest(v: &ProductMeaning) -> Result<PayloadDigest, DomainError> {
    validation::meaning(v)?;
    hash_native("mstrchf-product-meaning-v1", v.native())
}
pub fn match_basis_material(v: &MatchBasis) -> Result<Value, DomainError> {
    validation::basis(v)?;
    Ok(v.native())
}
pub fn match_basis_digest(v: &MatchBasis) -> Result<PayloadDigest, DomainError> {
    hash_native("mstrchf-product-match-v1", match_basis_material(v)?)
}
