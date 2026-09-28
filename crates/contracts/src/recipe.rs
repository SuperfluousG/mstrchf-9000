//! Recipe candidates, immutable accepted recipe revisions, and review evidence.

use crate::*;

string_enum! {
    pub enum RecipeSourceOrigin {
        Published,
        UserAuthored,
        Adapted,
    }
}

string_enum! {
    pub enum RightsDistribution {
        Permitted,
        PersonalOnly,
        Unresolved,
    }
}

string_enum! {
    pub enum DietaryVerdict {
        Supported,
        Conflicting,
        Unknown,
    }
}

string_enum! {
    pub enum ReviewKind {
        SourceReview,
        QuantityReview,
        DietaryReview,
        CookTest,
    }
}

string_enum! {
    pub enum ReviewActor {
        Human,
        Automated,
    }
}

string_enum! {
    pub enum CandidateIssueCode {
        MissingQuantity,
        MissingYield,
        MissingRights,
        MissingImage,
        MissingStorage,
        MissingPortionEvidence,
        UnresolvedIngredient,
        InconsistentInstructions,
        UnsupportedRequirement,
    }
}

#[derive(
    Clone,
    Copy,
    Debug,
    PartialEq,
    Eq,
    PartialOrd,
    Ord,
    serde::Serialize,
    serde::Deserialize,
    schemars::JsonSchema,
    ts_rs::TS,
)]
pub enum RecipeImageMediaType {
    #[serde(rename = "image/jpeg")]
    #[ts(rename = "image/jpeg")]
    ImageJpeg,
    #[serde(rename = "image/png")]
    #[ts(rename = "image/png")]
    ImagePng,
    #[serde(rename = "image/webp")]
    #[ts(rename = "image/webp")]
    ImageWebp,
}

string_enum! {
    pub enum RecipeImageKind {
        Photograph,
        Illustration,
        GeneratedIllustration,
    }
}

tagged! {
    pub enum ScalingPolicy {
        Linear {
            minimum: PositiveRational,
            maximum: PositiveRational,
        },
        DiscreteBatches {
            batch_yield: PositiveRational,
            maximum_batches: u16,
        },
    }
}

record! {
    pub struct SourceRecord {
        pub url: Nullable<HttpUrl>,
        pub source_revision: Nullable<Text>,
        pub retrieved_at: Instant,
        pub authors: BoundedVec<Text, 0, 64>,
        pub adaptation_notice: Nullable<Text>,
        pub origin: RecipeSourceOrigin,
    }
}

record! {
    pub struct RightsRecord {
        pub licence_id: Text,
        pub licence_url: Nullable<HttpUrl>,
        pub attribution: Text,
        pub adaptations: BoundedVec<Text, 0, 64>,
        pub distribution: RightsDistribution,
        pub evidence_ids: BoundedVec<EvidenceId, 0, 256>,
    }
}

record! {
    pub struct ConversionEvidence {
        pub evidence_id: EvidenceId,
        pub ingredient_id: IngredientId,
        pub from_form: IngredientForm,
        pub to_form: IngredientForm,
        pub from_unit: Unit,
        pub to_unit: Unit,
        pub factor: PositiveRational,
        pub source: SourceRecord,
    }
}

record! {
    pub struct RecipeIngredient {
        pub ingredient_id: IngredientId,
        pub name: NameText,
        pub quantity: Nullable<Quantity>,
        pub discrete: bool,
        pub conversion_evidence: BoundedVec<ConversionEvidence, 0, 64>,
        pub component_id: ComponentId,
        pub step_ids: BoundedVec<u16, 1, 256>,
    }
}

record! {
    pub struct ComponentYield {
        pub component_id: ComponentId,
        pub quantity: Quantity,
    }
}

record! {
    pub struct YieldSpec {
        pub servings: PositiveRational,
        pub basis: Text,
        pub components: BoundedVec<ComponentYield, 1, 128>,
    }
}

record! {
    pub struct InstructionStep {
        pub step_id: u16,
        pub text: Text,
        pub ingredient_ids: BoundedVec<IngredientId, 0, 256>,
        pub component_ids: BoundedVec<ComponentId, 0, 128>,
        pub active_minutes: u16,
        pub elapsed_minutes: u16,
        pub depends_on: BoundedVec<u16, 0, 256>,
    }
}

record! {
    pub struct ComponentSpec {
        pub component_id: ComponentId,
        pub title: Title,
        pub ingredient_ids: BoundedVec<IngredientId, 0, 256>,
        pub storage_rule_ids: BoundedVec<EvidenceId, 0, 256>,
    }
}

record! {
    pub struct DietaryEvidence {
        pub evidence_id: EvidenceId,
        pub requirement: HardRequirement,
        pub verdict: DietaryVerdict,
        pub source: SourceRecord,
        pub review_id: EvidenceId,
    }
}

record! {
    pub struct NutritionEvidence {
        pub source: SourceRecord,
        pub per_servings: PositiveRational,
        pub protein_g: Nullable<Rational>,
        pub energy_kcal: Nullable<Rational>,
        pub estimated: bool,
    }
}

record! {
    pub struct ReviewRecord {
        pub evidence_id: EvidenceId,
        pub kind: ReviewKind,
        pub actor: ReviewActor,
        pub at: Instant,
        pub note: Text,
    }
}

record! {
    pub struct PortabilityEvidence {
        pub cold_suitable: bool,
        pub reheating_required: bool,
        pub cold_storage_required: bool,
        pub evidence_ids: BoundedVec<EvidenceId, 0, 256>,
    }
}

record! {
    pub struct PortionPreset {
        pub preset: PortionPresetName,
        pub age_band: AgeBand,
        pub meal_type: MealType,
        pub servings: PositiveRational,
        pub evidence_ids: BoundedVec<EvidenceId, 0, 256>,
    }
}

record! {
    pub struct ExtraSpec {
        pub extra_id: ExtraId,
        pub title: Title,
        pub recipe: RecipeRef,
        pub eligible_person_requirements: BoundedVec<HardRequirement, 0, 64>,
        pub supported_with: BoundedVec<ComponentId, 1, 128>,
    }
}

record! {
    pub struct StorageRule {
        pub evidence_id: EvidenceId,
        pub component_id: ComponentId,
        pub max_chilled_seconds: u32,
        pub cooling_instruction: Text,
        pub reheating_instruction: Text,
        pub max_reheats: u8,
        pub source: SourceRecord,
    }
}

record! {
    pub struct PrepSpec {
        pub prep_id: u16,
        pub title: Title,
        pub step_ids: BoundedVec<u16, 1, 256>,
        pub depends_on: BoundedVec<u16, 0, 128>,
        pub active_minutes: u16,
        pub elapsed_minutes: u16,
        pub earliest_offset_seconds: i32,
        pub latest_offset_seconds: i32,
        pub ingredient_ids: BoundedVec<IngredientId, 0, 256>,
        pub can_attach_to_prior_session: bool,
    }
}

record! {
    pub struct ImageRecord {
        pub asset_id: AssetId,
        pub digest: AssetDigest,
        pub media_type: RecipeImageMediaType,
        pub alt_text: Text,
        pub kind: RecipeImageKind,
        pub source: SourceRecord,
        pub rights: RightsRecord,
        pub modified: bool,
    }
}

record! {
    pub struct RecipeCapabilities {
        pub local_display: bool,
        pub shopping: bool,
        pub leftovers: bool,
        pub starter_distribution: bool,
        pub blocking_issues: BoundedVec<CandidateIssue, 0, 256>,
    }
}

record! {
    pub struct RecipeCandidate {
        pub schema_version: SchemaVersion,
        pub candidate_id: RecipeId,
        pub proposed_revision: Revision,
        pub title: Title,
        pub source: SourceRecord,
        pub ingredients: BoundedVec<CandidateIngredient, 0, 256>,
        pub r#yield: Nullable<YieldSpec>,
        pub steps: BoundedVec<InstructionStep, 0, 256>,
        pub unresolved: BoundedVec<CandidateIssue, 0, 256>,
        pub image: Nullable<ImageRecord>,
    }
}

record! {
    pub struct CandidateIngredient {
        pub ingredient_id: Nullable<IngredientId>,
        pub name: Text,
        pub quantity: Nullable<Quantity>,
        pub component_id: Nullable<ComponentId>,
        pub source_text: Text,
    }
}

record! {
    pub struct CandidateIssue {
        pub code: CandidateIssueCode,
        pub path: Text,
        pub evidence_ids: BoundedVec<EvidenceId, 0, 256>,
    }
}

record! {
    pub struct RecipeRevision {
        pub schema_version: SchemaVersion,
        pub recipe_id: RecipeId,
        pub revision: Revision,
        pub title: Title,
        pub source: SourceRecord,
        pub text_rights: RightsRecord,
        pub ingredients: BoundedVec<RecipeIngredient, 1, 256>,
        pub r#yield: YieldSpec,
        pub scaling: ScalingPolicy,
        pub steps: BoundedVec<InstructionStep, 1, 256>,
        pub equipment: BoundedVec<EquipmentCode, 0, 9>,
        pub active_minutes: u16,
        pub elapsed_minutes: u16,
        pub meal_types: BoundedVec<MealType, 1, 3>,
        pub portability: PortabilityEvidence,
        pub components: BoundedVec<ComponentSpec, 1, 128>,
        pub extras: BoundedVec<ExtraSpec, 0, 64>,
        pub dietary_evidence: BoundedVec<DietaryEvidence, 0, 256>,
        pub storage: BoundedVec<StorageRule, 0, 128>,
        pub prep: BoundedVec<PrepSpec, 0, 128>,
        pub portion_presets: BoundedVec<PortionPreset, 1, 192>,
        pub nutrition: Nullable<NutritionEvidence>,
        pub image: Nullable<ImageRecord>,
        pub reviews: BoundedVec<ReviewRecord, 0, 256>,
        pub capabilities: RecipeCapabilities,
    }
}
