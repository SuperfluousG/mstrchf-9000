//! Bounded public manual-beta food authority. These are new, original recipes,
//! never relabelled engineering fixtures or broadened personal-use packets.
//! Only the exact compiled catalogue, evidence and local image bytes qualify.
use crate::BoardOptions;
use mstrchf_contracts::*;
use mstrchf_core::recipes::*;
use serde_json::{Value, json};
use std::sync::OnceLock;
mod pins;

const DOCUMENT: &[u8] =
    include_bytes!("../../../sites/mstrchf-9000/content/intake/public-beta-v1/catalogue.json");

#[derive(Debug)]
pub struct PublicRecipe {
    pub recipe: RecipeRevision,
    pub reference: RecipeRef,
    pub packet_digest: Digest,
    pub basis: EvidenceBasis,
    pub classes: ComponentClassView,
    pub method_id: EvidenceId,
    pub relevant_essentials: Vec<IngredientId>,
}
fn fail(code: ErrorCode, field: &str) -> DomainError {
    DomainError::new(code, &format!("/public_beta/{field}"))
}
fn unsupported(field: &str) -> DomainError {
    fail(ErrorCode::UnsupportedCapability, field)
}

fn load() -> Result<Vec<PublicRecipe>, DomainError> {
    if bytes_digest(DOCUMENT).0 != pins::CATALOGUE_BYTES_SHA256 {
        return Err(fail(ErrorCode::DigestMismatch, "catalogue_bytes"));
    }
    let document: Value = parse_document(DOCUMENT)?;
    if document["format"] != "mstrchf-public-beta-starter-catalogue"
        || document["version"] != 1
        || document["status"] != "compiled_candidate_ready_for_independent_review"
    {
        return Err(fail(ErrorCode::AuthorityDenied, "catalogue_qualification"));
    }
    let packets = document["starter_packets"]
        .as_array()
        .ok_or_else(|| fail(ErrorCode::MissingField, "recipes"))?;
    if packets.len() != 13 {
        return Err(fail(
            ErrorCode::MissingReference,
            "distinct_recipe_threshold",
        ));
    }
    let mut result = Vec::new();
    for (index, packet) in packets.iter().enumerate() {
        let recipe: RecipeRevision = parse_document(&canonical_bytes(&packet["recipe"])?)?;
        let reference: RecipeRef = parse_document(&canonical_bytes(&packet["recipe_reference"])?)?;
        if reference
            != (RecipeRef {
                recipe_id: recipe.recipe_id.clone(),
                revision: recipe.revision,
                digest: recipe_digest(&recipe)?,
            })
        {
            return Err(fail(ErrorCode::IdentityMismatch, "recipe_reference"));
        }
        if result
            .iter()
            .any(|prior: &PublicRecipe| prior.reference.recipe_id == reference.recipe_id)
            || recipe.components.len() != 1
            || recipe.prep.len() != 1
            || recipe.meal_types.len() != 1
            || recipe.scaling
                != (ScalingPolicy::Linear {
                    minimum: PositiveRational::new(1, 2)?,
                    maximum: PositiveRational::new(1, 1)?,
                })
            || recipe.r#yield.servings != PositiveRational::new(2, 1)?
            || recipe.portion_presets.len() != 1
            || recipe.portion_presets[0].age_band != AgeBand::Adult
            || recipe.portion_presets[0].preset != PortionPresetName::Regular
            || recipe.portion_presets[0].meal_type != recipe.meal_types[0]
            || recipe.portion_presets[0].servings != PositiveRational::new(1, 1)?
            || !recipe.storage.is_empty()
            || !recipe.extras.is_empty()
            || !recipe.reused_inputs.is_empty()
            || !recipe.dietary_evidence.is_empty()
            || recipe.nutrition.0.is_some()
            || recipe
                .reviews
                .iter()
                .any(|r| r.actor != ReviewActor::Automated || r.kind == ReviewKind::CookTest)
            || recipe.source.origin != RecipeSourceOrigin::UserAuthored
            || recipe.text_rights.distribution != RightsDistribution::Permitted
        {
            return Err(fail(
                ErrorCode::AuthorityDenied,
                "bounded_recipe_definition",
            ));
        }
        let image = recipe
            .image
            .0
            .as_ref()
            .ok_or_else(|| fail(ErrorCode::MissingField, "starter_image"))?;
        let (slug, asset) = pins::ASSETS[index];
        if image.kind != RecipeImageKind::GeneratedIllustration
            || image.media_type != RecipeImageMediaType::ImageWebp
            || image.rights.distribution != RightsDistribution::Permitted
            || bytes_digest(asset).0 != image.digest.0
            || packet["image_asset"]["sha256"] != image.digest.0
            || packet["image_asset"]["src"] != format!("/recipe-artwork/public-{slug}-v1.webp")
            || packet["image_asset"]["bytes"] != asset.len()
            || packet["image_asset"]["width"] != 960
            || packet["image_asset"]["height"] != 640
        {
            return Err(fail(ErrorCode::DigestMismatch, "starter_image"));
        }
        let documents = packet["evidence_documents"]
            .as_array()
            .ok_or_else(|| fail(ErrorCode::MissingReference, "evidence"))?;
        if documents.len() != 4
            || documents.iter().any(|d| {
                d["recipe_reference"] != packet["recipe_reference"]
                    || d["revision"] != "1"
                    || d["actor"] != "automated"
            })
            || documents[1]["ingredients"] != json!(recipe.ingredients)
            || documents[1]["yield"] != json!(recipe.r#yield)
            || documents[2]["full_method"] != json!(recipe.steps)
            || documents[2]["prep"] != json!(recipe.prep)
            || documents[2]["human_cook_test"] != false
            || documents[2]["timing_measured"] != false
            || documents[3]["sha256"] != image.digest.0
            || documents[3]["physical_cooking_test"] != false
        {
            return Err(fail(ErrorCode::IdentityMismatch, "evidence_closure"));
        }
        let admitted = documents
            .iter()
            .map(|document| {
                Ok(EvidencePin {
                    evidence_id: parse_document(&canonical_bytes(&document["evidence_id"])?)?,
                    revision: Revision(1),
                    digest: canonical_digest(document)?,
                })
            })
            .collect::<Result<Vec<_>, DomainError>>()?;
        let method_id = admitted[2].evidence_id.clone();
        let basis = EvidenceBasis::new(reference.clone(), admitted, vec![image.digest.clone()])?;
        let classes = ComponentClassView::new(reference.clone(), vec![])?;
        let assessment = assess_revision(&recipe, &basis, &classes)?;
        if !assessment.assessment().capabilities.shopping
            || !assessment.assessment().capabilities.starter_distribution
            || assessment.assessment().capabilities.leftovers
        {
            return Err(fail(ErrorCode::AuthorityDenied, "starter_capabilities"));
        }
        let relevant_essentials = recipe
            .ingredients
            .iter()
            .map(|i| i.ingredient_id.clone())
            .collect();
        result.push(PublicRecipe {
            recipe,
            reference,
            packet_digest: canonical_digest(packet)?,
            basis,
            classes,
            method_id,
            relevant_essentials,
        });
    }
    for (meal, count) in [
        (MealType::Dinner, 7),
        (MealType::Breakfast, 3),
        (MealType::Lunch, 3),
    ] {
        if result
            .iter()
            .filter(|p| p.recipe.meal_types.0 == vec![meal])
            .count()
            != count
        {
            return Err(fail(
                ErrorCode::MissingReference,
                "distinct_recipe_threshold",
            ));
        }
    }
    Ok(result)
}
pub fn registry() -> Result<&'static Vec<PublicRecipe>, DomainError> {
    static REGISTRY: OnceLock<Result<Vec<PublicRecipe>, DomainError>> = OnceLock::new();
    REGISTRY.get_or_init(load).as_ref().map_err(Clone::clone)
}
pub fn resolve(reference: &RecipeRef) -> Result<&'static PublicRecipe, DomainError> {
    registry()?
        .iter()
        .find(|p| &p.reference == reference)
        .ok_or_else(|| fail(ErrorCode::MissingReference, "recipe"))
}
pub fn notices() -> Vec<String> {
    vec![
    "Public manual beta: fresh at-home food for one or two explicitly confirmed unrestricted adults, each at Adult Regular ×1. One serving is the authored convention, not nutrition advice or human portion calibration.".into(),
    "Ingredients and complete methods are original authored recipes. The planner reserves the full estimated duration as continuously supervised preparation/cooking time; actual hands-on effort may be less. Check doneness and actual pack instructions and allow longer when needed. No human cooking test is claimed.".into(),
    "No child, dietary/allergy clearance, guest, extra, non-unit portion, advance preparation, portable, storage, reheating or leftover path is qualified. Check actual products, dates and handling. Images are labelled generated serving illustrations, not measured portions.".into(),
]
}
fn unit(value: PositiveRational) -> bool {
    value == PositiveRational::new(1, 1).expect("one")
}
fn portions(values: &MealOverrides, meal: &MealRequirements) -> Result<(), DomainError> {
    if values.portions.iter().any(|p| {
        p.preset != PortionPresetName::Regular
            || !unit(p.adjustment)
            || !meal.attendees.iter().any(|a| a.person == p.person)
    }) {
        return Err(unsupported("portion_override"));
    }
    Ok(())
}
pub(crate) fn check_use(
    recipe: &RecipeRevision,
    reference: &RecipeRef,
    meal: &MealRequirements,
    selection: &MealSelection,
    plan: &WeekPlan,
    options: &BoardOptions,
) -> Result<(), DomainError> {
    let admitted = resolve(reference)?;
    if &admitted.recipe != recipe
        || &selection.recipe != reference
        || meal.slot != selection.slot
        || plan.week_id != meal.slot.week_id
    {
        return Err(fail(ErrorCode::IdentityMismatch, "recipe_or_slot"));
    }
    if !meal.enabled
        || meal.location != MealLocation::AtHome
        || !recipe.meal_types.contains(&meal.slot.meal_type)
        || !(1..=2).contains(&meal.attendees.len())
        || !meal.shared_hard_requirements.is_empty()
    {
        return Err(unsupported("supported_meal"));
    }
    for attendee in meal.attendees.iter() {
        if !matches!(attendee.person, PersonRef::Diner { .. })
            || attendee.age_band != AgeBand::Adult
            || !attendee.hard_requirements.is_empty()
            || attendee.portion.preset != PortionPresetName::Regular
            || !unit(attendee.portion.adjustment)
            || attendee.portion.meal_overrides.iter().any(|p| {
                p.meal_type == meal.slot.meal_type
                    && (p.preset != PortionPresetName::Regular || !unit(p.adjustment))
            })
        {
            return Err(unsupported("adult_regular_unrestricted"));
        }
        let serving = selection
            .servings
            .iter()
            .filter(|s| s.person == attendee.person)
            .collect::<Vec<_>>();
        if serving.len() != 1
            || !unit(serving[0].servings)
            || serving[0].preset_ref.recipe != *reference
            || serving[0].preset_ref.age_band != AgeBand::Adult
            || serving[0].preset_ref.preset != PortionPresetName::Regular
            || serving[0].preset_ref.meal_type != meal.slot.meal_type
        {
            return Err(unsupported("serving_convention"));
        }
    }
    if selection.servings.len() != meal.attendees.len()
        || !selection.extras.is_empty()
        || selection.portable_cold.is_some()
        || options.make_extra == Some(true)
        || options.leftover_source.is_some()
        || options.portable_cold.is_some()
        || options
            .extra_recipients
            .as_ref()
            .is_some_and(|r| !r.is_empty())
        || plan
            .batch_links
            .iter()
            .any(|l| l.source_slot == selection.slot || l.target_slot == selection.slot)
    {
        return Err(unsupported("extras_leftovers_or_portability"));
    }
    match selection.batch_use {
        BatchUse::NewBatch {
            production_servings,
            ..
        } if production_servings == PositiveRational::new(meal.attendees.len() as u64, 1)? => {}
        _ => return Err(unsupported("fresh_servings")),
    }
    portions(&selection.meal_overrides, meal)?;
    if let Some(overrides) = &options.overrides {
        portions(overrides, meal)?;
    }
    Ok(())
}
