//! Immutable catalogue views. Incoming declarations are never food authority.
//! Runtime food authority resolves only compiled frozen engineering material or
//! the exact separately maker-reviewed packet, with mandatory use-scope checks.
use mstrchf_contracts::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::json;
use std::{collections::BTreeSet, sync::OnceLock};
use crate::maker_packets::{ReviewedPacketRecord, TrustedPacket, known_record};
use mstrchf_core::{food::*, recipes::*};

pub const LEGACY_ENGINEERING_V1_MANIFEST_DIGEST: &str =
    "1a48cedfb36fd00affbc21615dac3d80219aa5d235385d4ce23ec0297c8b0512";
const LEGACY_MANIFEST: &[u8] = include_bytes!("../assets/catalogue/engineering-v1/manifest.json");
const LEGACY_RECIPES: [&[u8]; 5] = [
    include_bytes!(
        "../assets/catalogue/engineering-v1/rc_00000000000000000000000000000001-r1.canonical.json"
    ),
    include_bytes!(
        "../assets/catalogue/engineering-v1/rc_00000000000000000000000000000002-r1.canonical.json"
    ),
    include_bytes!(
        "../assets/catalogue/engineering-v1/rc_00000000000000000000000000000003-r1.canonical.json"
    ),
    include_bytes!(
        "../assets/catalogue/engineering-v1/rc_00000000000000000000000000000004-r1.canonical.json"
    ),
    include_bytes!(
        "../assets/catalogue/engineering-v1/rc_00000000000000000000000000000005-r1.canonical.json"
    ),
];

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct RecipePacketPin {
    pub recipe: RecipeRef,
    pub packet_digest: Digest,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct CataloguePin {
    pub revision: Revision,
    pub manifest_digest: Digest,
    pub entries: BoundedVec<RecipePacketPin, 0, 512>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
pub enum NonAdmittedStatus {
    NonAdmitted,
}

/// Structurally checked immutable input data only. In particular, capability
/// bits and review actors inside this recipe remain declarations, not authority.
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NonAdmittedRecipePacket {
    pub schema_version: SchemaVersion,
    pub status: NonAdmittedStatus,
    pub recipe: RecipeRevision,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct NonAdmittedRecipeManifest {
    pub schema_version: SchemaVersion,
    pub status: NonAdmittedStatus,
    pub catalogue: CataloguePin,
    pub packets: BoundedVec<NonAdmittedRecipePacket, 0, 512>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CatalogueOrigin {
    FrozenLegacyEngineeringV1,
    NonAdmittedDeclarations,
}

/// Its contents and pin cannot be mutated through this view. This is deliberately
/// not a collection of core ReviewedRecipe values or an admission constructor.
#[derive(Debug)]
pub struct ImmutableCatalogueView {
    origin: CatalogueOrigin,
    pin: CataloguePin,
    recipes: Box<[RecipeRevision]>,
}

impl ImmutableCatalogueView {
    pub fn origin(&self) -> CatalogueOrigin {
        self.origin
    }
    pub fn pin(&self) -> &CataloguePin {
        &self.pin
    }
    pub fn declared_recipes(&self) -> &[RecipeRevision] {
        &self.recipes
    }
    /// Full ID/revision/digest resolution. Never falls back to a newer revision.
    pub fn resolve_recipe(&self, reference: &RecipeRef) -> Result<&RecipeRevision, DomainError> {
        shape(reference, "/recipe")?;
        let index = self
            .pin
            .entries
            .iter()
            .position(|entry| {
                entry.recipe.recipe_id == reference.recipe_id
                    && entry.recipe.revision == reference.revision
            })
            .ok_or_else(|| fail(ErrorCode::MissingReference, "/recipe"))?;
        if self.pin.entries[index].recipe.digest != reference.digest {
            return Err(fail(ErrorCode::DigestMismatch, "/recipe/digest"));
        }
        Ok(&self.recipes[index])
    }
    pub fn resolve_packet(&self, pin: &RecipePacketPin) -> Result<&RecipeRevision, DomainError> {
        shape(pin, "/packet")?;
        let recipe = self.resolve_recipe(&pin.recipe)?;
        let entry = self
            .pin
            .entries
            .iter()
            .find(|entry| entry.recipe == pin.recipe)
            .ok_or_else(|| fail(ErrorCode::MissingReference, "/packet/recipe"))?;
        if entry.packet_digest != pin.packet_digest {
            return Err(fail(ErrorCode::DigestMismatch, "/packet/packet_digest"));
        }
        Ok(recipe)
    }
    /// Checks this exact catalogue, not any self-consistent imported manifest.
    pub fn verify_catalogue_pin(&self, pin: &CataloguePin) -> Result<(), DomainError> {
        shape(pin, "/catalogue")?;
        if pin.revision != self.pin.revision {
            return Err(fail(ErrorCode::ContextStale, "/catalogue/revision"));
        }
        if pin != &self.pin {
            return Err(fail(ErrorCode::DigestMismatch, "/catalogue"));
        }
        Ok(())
    }
}

fn fail(code: ErrorCode, path: &str) -> DomainError {
    DomainError::new(code, path)
}
fn shape<T: ValidateShape>(value: &T, path: &str) -> Result<(), DomainError> {
    value.validate_shape().map_err(|issues| {
        fail(
            issues
                .first()
                .map_or(ErrorCode::InvalidValue, |issue| issue.code),
            path,
        )
    })
}
fn recipe_ref(recipe: &RecipeRevision) -> Result<RecipeRef, DomainError> {
    Ok(RecipeRef {
        recipe_id: recipe.recipe_id.clone(),
        revision: recipe.revision,
        digest: recipe_digest(recipe)?,
    })
}
fn packet_pin(recipe: &RecipeRevision, status: &str) -> Result<RecipePacketPin, DomainError> {
    Ok(RecipePacketPin {
        recipe: recipe_ref(recipe)?,
        packet_digest: canonical_digest(
            &json!({"schema_version":1,"status":status,"recipe":recipe}),
        )?,
    })
}
fn catalogue_pin(
    revision: Revision,
    entries: Vec<RecipePacketPin>,
) -> Result<CataloguePin, DomainError> {
    shape(&revision, "/catalogue/revision")?;
    let entries = BoundedVec(entries);
    let pin = CataloguePin {
        revision,
        manifest_digest: canonical_digest(
            &json!({"schema_version":1,"revision":revision,"entries":entries}),
        )?,
        entries,
    };
    shape(&pin, "/catalogue")?;
    Ok(pin)
}
fn check_recipes(recipes: &[RecipeRevision]) -> Result<(), DomainError> {
    if recipes.len() > 512 {
        return Err(fail(ErrorCode::LimitExceeded, "/packets"));
    }
    let mut seen = BTreeSet::new();
    for recipe in recipes {
        shape(recipe, "/packets/recipe")?;
        if !seen.insert((&recipe.recipe_id, recipe.revision)) {
            return Err(fail(ErrorCode::DuplicateKey, "/packets/recipe"));
        }
    }
    if recipes
        .windows(2)
        .any(|pair| (&pair[0].recipe_id, pair[0].revision) > (&pair[1].recipe_id, pair[1].revision))
    {
        return Err(fail(ErrorCode::NoncanonicalValue, "/packets"));
    }
    Ok(())
}

/// Builds a declaration manifest; this helper neither reviews nor admits food.
pub fn non_admitted_manifest(
    revision: Revision,
    mut packets: Vec<NonAdmittedRecipePacket>,
) -> Result<NonAdmittedRecipeManifest, DomainError> {
    if packets.len() > 512 {
        return Err(fail(ErrorCode::LimitExceeded, "/packets"));
    }
    packets.sort_by(|a, b| {
        (&a.recipe.recipe_id, a.recipe.revision).cmp(&(&b.recipe.recipe_id, b.recipe.revision))
    });
    let recipes = packets
        .iter()
        .map(|packet| packet.recipe.clone())
        .collect::<Vec<_>>();
    check_recipes(&recipes)?;
    for packet in &packets {
        shape(packet, "/packets")?;
    }
    let entries = recipes
        .iter()
        .map(|recipe| packet_pin(recipe, "non_admitted"))
        .collect::<Result<Vec<_>, _>>()?;
    let manifest = NonAdmittedRecipeManifest {
        schema_version: SchemaVersion,
        status: NonAdmittedStatus::NonAdmitted,
        catalogue: catalogue_pin(revision, entries)?,
        packets: BoundedVec(packets),
    };
    shape(&manifest, "/manifest")?;
    Ok(manifest)
}

/// Strict untrusted-wire admission into an immutable DECLARATION view only.
/// No EvidenceBasis transport or claim of real source/cooking authority exists.
pub fn non_admitted_view(bytes: &[u8]) -> Result<ImmutableCatalogueView, DomainError> {
    let manifest: NonAdmittedRecipeManifest = parse_document(bytes)?;
    let recipes = manifest
        .packets
        .iter()
        .map(|packet| packet.recipe.clone())
        .collect::<Vec<_>>();
    check_recipes(&recipes)?;
    let entries = recipes
        .iter()
        .map(|recipe| packet_pin(recipe, "non_admitted"))
        .collect::<Result<Vec<_>, _>>()?;
    let expected = catalogue_pin(manifest.catalogue.revision, entries)?;
    if manifest.catalogue != expected {
        return Err(fail(ErrorCode::DigestMismatch, "/catalogue"));
    }
    Ok(ImmutableCatalogueView {
        origin: CatalogueOrigin::NonAdmittedDeclarations,
        pin: expected,
        recipes: recipes.into_boxed_slice(),
    })
}

#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum LegacyStatus {
    LegacyEngineeringV1,
}
#[derive(Deserialize, Serialize, JsonSchema)]
#[serde(deny_unknown_fields)]
struct LegacyManifest {
    schema_version: SchemaVersion,
    status: LegacyStatus,
    catalogue: CataloguePin,
}

fn load_legacy() -> Result<ImmutableCatalogueView, DomainError> {
    let manifest: LegacyManifest = parse_document(LEGACY_MANIFEST)?;
    if canonical_bytes(&manifest)? != LEGACY_MANIFEST {
        return Err(fail(ErrorCode::NoncanonicalValue, "/legacy/manifest"));
    }
    let mut recipes = Vec::new();
    for bytes in LEGACY_RECIPES {
        let recipe: RecipeRevision = parse_document(bytes)?;
        if canonical_bytes(&recipe)? != bytes {
            return Err(fail(ErrorCode::NoncanonicalValue, "/legacy/recipe"));
        }
        recipes.push(recipe);
    }
    check_recipes(&recipes)?;
    let entries = recipes
        .iter()
        .map(|recipe| packet_pin(recipe, "legacy_engineering_v1"))
        .collect::<Result<Vec<_>, _>>()?;
    let expected = catalogue_pin(Revision(1), entries)?;
    if expected.manifest_digest.0 != LEGACY_ENGINEERING_V1_MANIFEST_DIGEST
        || manifest.catalogue != expected
    {
        return Err(fail(ErrorCode::DigestMismatch, "/legacy/catalogue"));
    }
    Ok(ImmutableCatalogueView {
        origin: CatalogueOrigin::FrozenLegacyEngineeringV1,
        pin: expected,
        recipes: recipes.into_boxed_slice(),
    })
}

/// Frozen original five declarations. Still synthetic engineering material.
/// Later receipt integration can select this exact view for legacy absent pins.
pub fn legacy_engineering_v1() -> Result<&'static ImmutableCatalogueView, DomainError> {
    static LEGACY: OnceLock<Result<ImmutableCatalogueView, DomainError>> = OnceLock::new();
    LEGACY
        .get_or_init(load_legacy)
        .as_ref()
        .map_err(Clone::clone)
}

/// Immutable runtime view. Admitted records are copied packet custody; all food
/// evidence below originates from the compiled registry, never these Values.
#[derive(Debug)]
pub struct RuntimeCatalogue {
    pin: CataloguePin,
    recipes: Vec<RecipeRevision>,
    inputs: Vec<(EvidenceBasis, ComponentClassView)>,
    maker_packets: Vec<&'static TrustedPacket>,
    public_packets: Vec<&'static crate::public_catalogue::PublicRecipe>,
}
impl RuntimeCatalogue {
    pub fn legacy() -> Result<Self, DomainError> { Self::from_records(&[]) }
    pub fn from_records(records: &[ReviewedPacketRecord]) -> Result<Self, DomainError> {
        if records.len() > 512 { return Err(fail(ErrorCode::LimitExceeded,"/catalogue/records")); }
        crate::legacy_food::validate_premises()?;
        let legacy = legacy_engineering_v1()?;
        let mut packets = Vec::new();
        let mut seen = BTreeSet::new();
        for record in records {
            let packet = known_record(record)?;
            if !seen.insert((&packet.reference().recipe.recipe_id,packet.reference().recipe.revision)) {
                return Err(fail(ErrorCode::DuplicateKey,"/catalogue/records"));
            }
            packets.push(packet);
        }
        packets.sort_by_key(|packet| (packet.reference().recipe.recipe_id.clone(),packet.reference().recipe.revision));
        let mut recipes = legacy.declared_recipes().to_vec();
        let mut inputs = recipes.iter().map(crate::legacy_food::evidence_inputs).collect::<Result<Vec<_>,_>>()?;
        let mut entries = legacy.pin().entries.0.clone();
        for packet in &packets {
            recipes.push(packet.recipe().clone());
            inputs.push((packet.basis().clone(),packet.classes().clone()));
            entries.push(RecipePacketPin {recipe:packet.reference().recipe.clone(),packet_digest:packet.reference().packet_digest.clone()});
        }
        // This first registry has one immutable real packet after the five
        // engineering IDs. Reject a future registry edit unless ordering remains
        // explicit, rather than relying on ID-only matching or silent fallback.
        check_recipes(&recipes)?;
        let revision = Revision(1 + packets.len() as u64);
        let pin = catalogue_pin(revision, entries)?;
        if packets.is_empty() && &pin != legacy.pin() { return Err(fail(ErrorCode::DigestMismatch,"/catalogue/legacy")); }
        let view=Self {pin,recipes,inputs,maker_packets:packets,public_packets:vec![]};
        // A cold-service declaration cannot stand in for its original food.
        // Receipt validation separately proves the source-first admission order.
        for packet in &view.maker_packets {
            if !packet.recipe().reused_inputs.is_empty() {
                view.require_admission_dependencies(packet)?;
            }
        }
        Ok(view)
    }
    /// A separate exact public qualification baseline, never the legacy
    /// engineering catalogue or an automatic migration of personal records.
    pub fn public_starters() -> Result<Self,DomainError> {
        let public_packets=crate::public_catalogue::registry()?.iter().collect::<Vec<_>>();
        let recipes=public_packets.iter().map(|p|p.recipe.clone()).collect::<Vec<_>>();
        check_recipes(&recipes)?;
        let inputs=public_packets.iter().map(|p|(p.basis.clone(),p.classes.clone())).collect();
        let entries=public_packets.iter().map(|p|RecipePacketPin{recipe:p.reference.clone(),packet_digest:p.packet_digest.clone()}).collect();
        Ok(Self{pin:catalogue_pin(Revision(1),entries)?,recipes,inputs,maker_packets:vec![],public_packets})
    }
    pub fn is_public_baseline(&self)->bool{!self.public_packets.is_empty()}
    pub fn is_public_recipe(&self,reference:&RecipeRef)->bool{self.public_packets.iter().any(|p|&p.reference==reference)}
    pub fn pin(&self) -> &CataloguePin { &self.pin }
    pub fn recipes(&self) -> &[RecipeRevision] { &self.recipes }
    pub fn admitted_packets(&self) -> &[&'static TrustedPacket] { &self.maker_packets }
    /// Admission is independent and explicit. Declared extras and original
    /// reused components must already resolve in the current admitted view.
    pub(crate) fn require_admission_dependencies(&self, packet:&TrustedPacket)->Result<(),DomainError>{
        for extra in packet.recipe().extras.iter() {
            self.resolve_recipe(&extra.recipe)
                .map_err(|failure|fail(failure.code,"/catalogue/extra_dependency"))?;
        }
        for input in packet.recipe().reused_inputs.iter() {
            let source=self.resolve_recipe(&input.source_recipe)
                .map_err(|failure|fail(failure.code,"/catalogue/source_dependency"))?;
            if !self.maker_packets.iter().any(|original|original.reference().recipe==input.source_recipe)
                || !source.reused_inputs.is_empty()
                || !source.components.iter().any(|component|component.component_id==input.source_component_id) {
                return Err(fail(ErrorCode::MissingReference,"/catalogue/source_component_dependency"));
            }
        }
        Ok(())
    }
    pub fn has_real_entries(&self) -> bool { !self.maker_packets.is_empty() || !self.public_packets.is_empty() }
    pub fn resolve_recipe(&self, reference: &RecipeRef) -> Result<&RecipeRevision, DomainError> {
        shape(reference,"/recipe")?;
        let index = self.pin.entries.iter().position(|entry| entry.recipe.recipe_id == reference.recipe_id && entry.recipe.revision == reference.revision)
            .ok_or_else(||fail(ErrorCode::MissingReference,"/recipe"))?;
        if self.pin.entries[index].recipe.digest != reference.digest { return Err(fail(ErrorCode::DigestMismatch,"/recipe/digest")); }
        Ok(&self.recipes[index])
    }
    pub fn verify_pin(&self, pin: &CataloguePin) -> Result<(), DomainError> {
        shape(pin,"/catalogue")?;
        if pin.revision != self.pin.revision { return Err(fail(ErrorCode::ContextStale,"/catalogue/revision")); }
        if pin != &self.pin { return Err(fail(ErrorCode::DigestMismatch,"/catalogue")); }
        Ok(())
    }
    pub fn is_personal_recipe(&self, reference: &RecipeRef) -> bool {
        crate::personal_scope::requires_personal_scope(&reference.recipe_id)
    }
    pub fn offers_new_choice(&self, reference: &RecipeRef) -> Result<bool, DomainError> {
        self.resolve_recipe(reference)?;
        if self.is_public_baseline(){return Ok(self.is_public_recipe(reference));}
        if [crate::personal_scope::CHILLED_CHICKEN_EXTRA_SCOPE.recipe_id,
            crate::personal_scope::CHILLED_GF_CHICKEN_SCOPE.recipe_id].contains(&reference.recipe_id.0.as_str()) {return Ok(false);}
        Ok(!self.has_real_entries() || self.maker_packets.iter().any(|packet| &packet.reference().recipe == reference))
    }
    pub fn reviewed(&self) -> Result<Vec<ReviewedRecipe<'_>>, DomainError> {
        self.recipes.iter().zip(&self.inputs).map(|(recipe,(basis,classes))|assess_revision(recipe,basis,classes)).collect()
    }
    /// Exact selected subset of the compiled food premises. The real packet
    /// gets production-only phases with its own accepted policy pin, no class,
    /// asset, storage, reheat or engineering essential-ingredient assumptions.
    pub(crate) fn food_evidence(&self, references: &[RecipeRef]) -> Result<FoodEvidence, DomainError> {
        let mut legacy_refs = Vec::new();
        let mut real_refs = Vec::new();
        for reference in references {
            self.resolve_recipe(reference)?;
            if self.is_personal_recipe(reference) || self.is_public_recipe(reference) { real_refs.push(reference.clone()); }
            else { legacy_refs.push(reference.clone()); }
        }
        let legacy = legacy_engineering_v1()?;
        let mut evidence = crate::legacy_food::evidence_for_refs(&legacy_refs, legacy.declared_recipes(), &self.inputs[..5])?;
        evidence.catalogue_revision = self.pin.revision;
        for reference in real_refs {
            let index = self.pin.entries.iter().position(|entry| entry.recipe == reference).ok_or_else(||fail(ErrorCode::MissingReference,"/food/recipe"))?;
            let recipe = &self.recipes[index];
            evidence.recipes.push(self.inputs[index].0.clone());
            evidence.classes.push(self.inputs[index].1.clone());
            if let Some(packet)=self.public_packets.iter().find(|packet|packet.reference==reference) {
                evidence.essential_ingredient_ids.extend_from_slice(&packet.relevant_essentials);
                let evidence_ids=vec![packet.method_id.clone()];
                evidence.methods.push(MethodPhases{portable_cold:None,recipe:reference.clone(),kind:MethodKind::Production,prep_ids:recipe.prep.iter().map(|p|p.prep_id).collect(),direct_reuse_counts:vec![],evidence_ids:evidence_ids.clone()});
                for prep in recipe.prep.iter(){evidence.resources.push(PhaseResources{recipe:reference.clone(),prep_id:prep.prep_id,equipment:recipe.equipment.0.clone(),evidence_ids:evidence_ids.clone()});}
                continue;
            }
            let packet = self.maker_packets.iter().find(|packet|packet.reference().recipe==reference)
                .ok_or_else(||fail(ErrorCode::MissingReference,"/food/maker_packet"))?;
            let method_ids = vec![packet.production_method_evidence_id().clone()];
            let resource_ids = vec![packet.phase_resource_evidence_id().clone()];
            // Pantry presentation is owned by this selected exact compiled
            // packet. Shared IDs and earlier catalogue proofs remain unchanged.
            evidence.essential_ingredient_ids.extend_from_slice(packet.relevant_essential_ingredient_ids());
            evidence.methods.push(MethodPhases {portable_cold: packet.portable_cold_method(), recipe:reference.clone(),kind:MethodKind::Production,prep_ids:recipe.prep.iter().map(|p|p.prep_id).collect(),direct_reuse_counts:vec![],evidence_ids:method_ids});
            for prep in recipe.prep.iter() {
                evidence.resources.push(PhaseResources {recipe:reference.clone(),prep_id:prep.prep_id,equipment:packet.phase_equipment(prep.prep_id)?.to_vec(),evidence_ids:resource_ids.clone()});
            }
        }
        evidence.essential_ingredient_ids.sort();
        evidence.essential_ingredient_ids.dedup();
        evidence.catalogue_digest = food_evidence_digest(&evidence)?;
        Ok(evidence)
    }
}
