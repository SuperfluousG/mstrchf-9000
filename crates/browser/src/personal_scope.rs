//! Scope predicates for the exact maker-reviewed personal source packets.
//!
//! This module constructs no EvidenceBasis, ReviewedRecipe or library admission.
//! Root must call it at candidate selection, dispatch/result application and state
//! validation boundaries. An Ok result only establishes these use predicates for
//! the supplied immutable context; ordinary core validation is still required.
use crate::BoardOptions;
use mstrchf_contracts::*;
mod courgette;
mod portable;
mod gf_chicken;
mod chilli;
mod overnight;
pub use overnight::{OVERNIGHT_OATS_SCOPE, is_overnight_oats};
pub use gf_chicken::{THREE_BEAN_GF_CHICKEN_SCOPE, CHILLED_GF_CHICKEN_SCOPE, GF_CHICKEN_EXTRA_ID};
pub(crate) use gf_chicken::{check_gf_chicken_pair_allocation, check_gf_chicken_extra_scope};
pub use chilli::{CHILLI_BATCH_SCOPE, CHILLI_WRAPS_SCOPE, CHILLI_POTATO_SCOPE, CHILLI_COMPONENT_ID};
pub(crate) use chilli::{is_chilli_service, chilli_service_input};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct PersonalUseScope {
    pub scope_id: &'static str,
    pub recipe_id: &'static str,
    pub recipe_revision: u64,
    pub recipe_digest: &'static str,
    pub meal_type: MealType,
    pub source_url: &'static str,
    pub packet_canonical_digest: &'static str,
    pub attention_reservation_minutes: u16,
    pub timing_is_estimate: bool,
    pub portion_convention: &'static str,
}

/// Root adopted the packet-local ingredient meanings, unchanged source-serving
/// counting convention and labelled reservation estimate. No cooking, nutrition,
/// age, dietary, product or runtime-admission authority is supplied by this value.
pub const PORRIDGE_PERSONAL_SCOPE: PersonalUseScope = PersonalUseScope {
    scope_id: "porridge-personal-source-serving-v1",
    recipe_id: "rc_80000000000000000000000000000001",
    recipe_revision: 3,
    recipe_digest: "dd867c017a9b591f57546246518cd93c94e9f1ab50c54bfe4f288a1054de6ce0",
    meal_type: MealType::Breakfast,
    source_url: "https://en.wikibooks.org/w/index.php?title=Cookbook:Oat_Porridge&oldid=4606890",
    packet_canonical_digest: "edc4239b57366c96ef4b91198e4cc54de62db44d3d6346574e7318a852e2d726",
    attention_reservation_minutes: 15,
    timing_is_estimate: true,
    portion_convention: "Adult Regular counts one published source serving; not nutrition or age advice.",
};

pub const AGLIO_PERSONAL_SCOPE: PersonalUseScope = PersonalUseScope {
    scope_id: "aglio-personal-two-clove-dinner-v1",
    recipe_id: "rc_80000000000000000000000000000003",
    recipe_revision: 1,
    recipe_digest: "58914b613960257e33ce294636bcd417baccb631f61379a22aa4977b14cc14f3",
    meal_type: MealType::Dinner,
    source_url: "https://github.com/ronaldl29/public-domain-recipes/blob/da84378b36bd5b2e3cb35f610d64630bf1bd899d/content/aglio-e-olio.md",
    packet_canonical_digest: "62949a9b43fb50ed7385271a22c09f9b5d703ff08ce865041b66c6d6f2d14dee",
    attention_reservation_minutes: 25,
    timing_is_estimate: true,
    portion_convention: "Adult Regular counts one adapted plate; not nutrition or age advice.",
};

pub const THREE_BEAN_PERSONAL_SCOPE: PersonalUseScope = PersonalUseScope {
    scope_id: "three-bean-personal-cold-home-lunch-v1",
    recipe_id: "rc_80000000000000000000000000000004",
    recipe_revision: 1,
    recipe_digest: "247de874dc0348b4afdc06fe6b5aab31cdebf2869ace5098393d61dae9a80377",
    meal_type: MealType::Lunch,
    source_url: "https://github.com/ronaldl29/public-domain-recipes/blob/da84378b36bd5b2e3cb35f610d64630bf1bd899d/content/bean-salad.md",
    packet_canonical_digest: "ec3894c5071806462224a3ba282467b159b8a84ad05df8ae12995489784802ac",
    attention_reservation_minutes: 15,
    timing_is_estimate: true,
    portion_convention: "Adult Regular counts one complete authored cold-home salad unit; not source scaling, nutrition or human calibration.",
};

pub const THREE_BEAN_TWO_UNIT_SCOPE: PersonalUseScope = PersonalUseScope {
    scope_id: "three-bean-personal-two-unit-cold-home-lunch-v1",
    recipe_id: "rc_80000000000000000000000000000005",
    recipe_revision: 1,
    recipe_digest: "bea391c42fe3a7207d4ac04104f5fac6d839a36970a197be490585c89fb0cf7c",
    meal_type: MealType::Lunch,
    source_url: THREE_BEAN_PERSONAL_SCOPE.source_url,
    packet_canonical_digest: "e18d259dfa7941b893997dee7b378616457025ccac92021d54a2a43e25173932",
    attention_reservation_minutes: 30,
    timing_is_estimate: true,
    portion_convention: "Exactly two separate containers, one complete authored Adult Regular unit each; not source scaling, nutrition or human calibration.",
};

pub const THREE_BEAN_TWO_UNIT_VEGETARIAN_SCOPE: PersonalUseScope = PersonalUseScope {
    scope_id: "three-bean-personal-two-unit-vegetarian-cold-home-lunch-v1",
    recipe_id: "rc_80000000000000000000000000000006",
    recipe_revision: 1,
    recipe_digest: "9e5495c21e6da79685364e4069c62485db521652f5b184f955ff5ae27a6f1632",
    meal_type: MealType::Lunch,
    source_url: THREE_BEAN_PERSONAL_SCOPE.source_url,
    packet_canonical_digest: "61aa9bc1b586ff82a1ceab2198f5cf88e7113ef82e37bdfa2b60cd4c7915de6d",
    attention_reservation_minutes: 30,
    timing_is_estimate: true,
    portion_convention: "Exactly two separate vegetarian-input containers, one authored Adult Regular unit each; not product qualification, nutrition or human calibration.",
};

pub const PORRIDGE_LARGER_SCOPE: PersonalUseScope = PersonalUseScope {
    scope_id: "porridge-personal-fixed-larger-unit-v1",
    recipe_id: "rc_80000000000000000000000000000007",
    recipe_revision: 1,
    recipe_digest: "8e73018765acca1f0265082f1c95601359274014259a306f4621ed805a133b97",
    meal_type: MealType::Breakfast,
    source_url: PORRIDGE_PERSONAL_SCOPE.source_url,
    packet_canonical_digest: "93df7d839d9076d377fb1c08b201c3048aff4a428093a1cf75fcf6cf919cebfd",
    attention_reservation_minutes: 20,
    timing_is_estimate: true,
    portion_convention: "Adult Larger counts one whole authored fixed-quantity unit; not nutrition, measured yield, age advice or human portion calibration.",
};

pub const THREE_BEAN_TWO_UNIT_CHICKEN_EXTRA_SCOPE: PersonalUseScope = PersonalUseScope {
    scope_id: "three-bean-two-unit-vegetarian-base-optional-chicken-cold-home-lunch-v1",
    recipe_id: "rc_80000000000000000000000000000008", recipe_revision: 1,
    recipe_digest: "fa50f509872b19c8b986dff36ec91397e5268a60afbb93a7728badfde8661a71",
    meal_type: MealType::Lunch, source_url: THREE_BEAN_PERSONAL_SCOPE.source_url,
    packet_canonical_digest: "a6a189f045a8fe21617efb5fed082a43773071c6250c2b06bdda943e32395407",
    attention_reservation_minutes: 30, timing_is_estimate: true,
    portion_convention: "Two separate conditional Vegetarian-input Adult Regular units; optional separately reviewed extra only for one named None adult. No product, nutrition or human calibration claim.",
};
pub const CHILLED_CHICKEN_EXTRA_SCOPE: PersonalUseScope = PersonalUseScope {
    scope_id: "chilled-chicken-named-none-adult-extra-only-v1",
    recipe_id: "rc_80000000000000000000000000000009", recipe_revision: 1,
    recipe_digest: "ef0cbaba1c1698ccaaa8de4b0f6ee4526d33150645532a3ca822f65ba9cf97c9",
    // Exact original generated definition has no published recipe URL. This
    // internal sentinel changes neither SourceRecord nor any older source rule.
    meal_type: MealType::Lunch, source_url: "",
    packet_canonical_digest: "e9675ef1aec183cf4d540b5c2c23856a1666f02bcf6b57080e8b845f1551209b",
    attention_reservation_minutes: 5, timing_is_estimate: true,
    portion_convention: "One whole fixed 50 g edible cooked chicken extra for one named None Adult Regular recipient; never a standalone meal, measured yield or calibrated portion.",
};
pub const CHICKEN_EXTRA_ID: &str = "ex_80000000000000000000000000000002";

pub const THREE_BEAN_SMALLER_SCOPE: PersonalUseScope = PersonalUseScope {
    scope_id: "three-bean-personal-fixed-smaller-cold-home-lunch-v1",
    recipe_id: "rc_8000000000000000000000000000000a", recipe_revision: 1,
    recipe_digest: "152c886cd7d50aae387f3d469059a57f2c21b651943e82823f33103df35f300d",
    meal_type: MealType::Lunch, source_url: THREE_BEAN_PERSONAL_SCOPE.source_url,
    packet_canonical_digest: "8414f28d6683d7d4024911686034166de3de9d278b0759f1acfa29e6a3bed663",
    attention_reservation_minutes: 15, timing_is_estimate: true,
    portion_convention: "Adult Smaller counts one whole fixed salad unit with three quarters of the authored Regular inputs; not a published serving, nutrition or human portion calibration.",
};

pub const COURGETTE_HOT_SCOPE: PersonalUseScope = PersonalUseScope {
    scope_id: "courgette-four-share-hot-home-dinner-v1",
    recipe_id: "rc_8000000000000000000000000000000b", recipe_revision: 1,
    recipe_digest: "e1a034ab1ab6f10bd1df2c28f7feca5152b4d108895c6fd9ca61ee99dc6aaa2c",
    meal_type: MealType::Dinner,
    source_url: "https://en.wikibooks.org/w/index.php?title=Cookbook:Zucchini_Pasta_Bake&oldid=4525784",
    packet_canonical_digest: "b2c3301c26c57e5e0c12e3dfe467acacb5a3a0a87c6c4c4b46e6b427269a404e",
    attention_reservation_minutes: 42, timing_is_estimate: true,
    portion_convention: "Cook one complete four-share bake. Each unrestricted Adult Regular diner has one share; only the original remaining shares can supply the separately reviewed cold home lunch. Shares are a recipe convention, not measured nutritional portions.",
};
pub const COURGETTE_COLD_SCOPE: PersonalUseScope = PersonalUseScope {
    scope_id: "courgette-original-share-cold-home-lunch-v1",
    recipe_id: "rc_8000000000000000000000000000000c", recipe_revision: 1,
    recipe_digest: "0217e936a282f0e20381a1be98bfd501518bf6d7205fdf3deb0f21de53996026",
    meal_type: MealType::Lunch, source_url: COURGETTE_HOT_SCOPE.source_url,
    packet_canonical_digest: "1ccb51875f250c48fe01456465f7ee3c5c2150cb0a05e25026d5ee9a232a1e16",
    attention_reservation_minutes: 5, timing_is_estimate: true,
    portion_convention: "Serve one original cooked share per unrestricted Adult Regular diner, for one to three people within the original batch remainder. No new ingredients or new storage time.",
};

pub const PORTABLE_HOT_SCOPE: PersonalUseScope = PersonalUseScope {
    scope_id: "courgette-four-share-hot-dinner-portable-source-v1",
    recipe_id: "rc_8000000000000000000000000000000d", recipe_revision: 1,
    recipe_digest: "722273c11760561ed99eb0f4487a12d38b10ed8d38400507898953c9f0565dc8",
    meal_type: MealType::Dinner, source_url: COURGETTE_HOT_SCOPE.source_url,
    packet_canonical_digest: "3252af039f9ea3265ce74a9e3acc307dbe7707ddd8fd69d623c2c6598c403f68",
    attention_reservation_minutes: 42, timing_is_estimate: true,
    portion_convention: "Cook one complete four-share bake. Each unrestricted Adult Regular diner has one share; only the original remaining shares can supply the separately reviewed portable cold lunch. Shares are a recipe convention, not measured nutritional portions.",
};
pub const PORTABLE_COLD_SCOPE: PersonalUseScope = PersonalUseScope {
    scope_id: "courgette-original-share-portable-cold-lunch-v1",
    recipe_id: "rc_8000000000000000000000000000000e", recipe_revision: 1,
    recipe_digest: "d31d0d42e1ae7be0ebdd7290947440cf7ddd0ca48a49be82c6d0660743c6f436",
    meal_type: MealType::Lunch, source_url: COURGETTE_HOT_SCOPE.source_url,
    packet_canonical_digest: "6132e23c06146fd706049de946b55c851c86b83fa7ca04e860da5d4ad4e80772",
    attention_reservation_minutes: 10, timing_is_estimate: true,
    portion_convention: "Serve one original cooked share per unrestricted Adult Regular diner, for one to three people within the original batch remainder. No new ingredients or new storage time.",
};

fn courgette_reference(scope: &PersonalUseScope) -> RecipeRef {
    RecipeRef {recipe_id:RecipeId(scope.recipe_id.into()),revision:Revision(scope.recipe_revision),
        digest:RecipeDigest(scope.recipe_digest.into())}
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct ColdHomeTiming {
    pub first_handling_seconds: i64,
    pub assembly_minutes: u16,
    pub chilling_minutes: u16,
}
/// Timing predicates for exact compiled references only; not evidence authority.
pub fn cold_home_timing(reference: &RecipeRef) -> Result<Option<ColdHomeTiming>, DomainError> {
    let Some(scope)=scope_for_recipe_id(&reference.recipe_id) else {return Ok(None);};
    let timing=if scope.recipe_id==THREE_BEAN_PERSONAL_SCOPE.recipe_id || scope.recipe_id==THREE_BEAN_SMALLER_SCOPE.recipe_id {
        ColdHomeTiming{first_handling_seconds:4500,assembly_minutes:15,chilling_minutes:60}
    } else if is_two_unit_scope(scope) {
        ColdHomeTiming{first_handling_seconds:5400,assembly_minutes:30,chilling_minutes:60}
    } else if scope.recipe_id==CHILLED_CHICKEN_EXTRA_SCOPE.recipe_id || scope.recipe_id==CHILLED_GF_CHICKEN_SCOPE.recipe_id {
        ColdHomeTiming{first_handling_seconds:300,assembly_minutes:5,chilling_minutes:0}
    } else {return Ok(None);};
    if !pinned(reference,scope) {return Err(refusal(ErrorCode::IdentityMismatch,"recipe_reference"));}
    Ok(Some(timing))
}
#[derive(Clone, Copy, PartialEq, Eq)]
enum Cohort { SingleAdult, SingleResidentAdult, TwoAdultsAtLeastOneResident }
fn cohort(scope:&PersonalUseScope)->Cohort {
    if is_two_unit_scope(scope) {Cohort::TwoAdultsAtLeastOneResident}
    else if scope.recipe_id==THREE_BEAN_PERSONAL_SCOPE.recipe_id || scope.recipe_id==PORRIDGE_LARGER_SCOPE.recipe_id || scope.recipe_id==THREE_BEAN_SMALLER_SCOPE.recipe_id || scope.recipe_id==OVERNIGHT_OATS_SCOPE.recipe_id {Cohort::SingleResidentAdult}
    else {Cohort::SingleAdult}
}

pub fn scope_for_recipe_id(recipe_id: &RecipeId) -> Option<&'static PersonalUseScope> {
    if recipe_id.0 == OVERNIGHT_OATS_SCOPE.recipe_id { return Some(&OVERNIGHT_OATS_SCOPE); }
    [&PORRIDGE_PERSONAL_SCOPE, &AGLIO_PERSONAL_SCOPE, &THREE_BEAN_PERSONAL_SCOPE, &THREE_BEAN_TWO_UNIT_SCOPE, &THREE_BEAN_TWO_UNIT_VEGETARIAN_SCOPE, &PORRIDGE_LARGER_SCOPE, &THREE_BEAN_TWO_UNIT_CHICKEN_EXTRA_SCOPE, &CHILLED_CHICKEN_EXTRA_SCOPE, &THREE_BEAN_SMALLER_SCOPE, &COURGETTE_HOT_SCOPE, &COURGETTE_COLD_SCOPE, &PORTABLE_HOT_SCOPE, &PORTABLE_COLD_SCOPE, &THREE_BEAN_GF_CHICKEN_SCOPE, &CHILLED_GF_CHICKEN_SCOPE, &CHILLI_BATCH_SCOPE, &CHILLI_WRAPS_SCOPE, &CHILLI_POTATO_SCOPE].into_iter().find(|scope| scope.recipe_id == recipe_id.0)
}

/// Stable exact-recipe notices for the library/source UI; no authority object.
pub fn notices_for_recipe(recipe_id: &RecipeId) -> Vec<String> {
    let Some(scope) = scope_for_recipe_id(recipe_id) else { return vec![]; };
    if scope.recipe_id == OVERNIGHT_OATS_SCOPE.recipe_id { return overnight::notices(); }
    if chilli::is_family(scope) {return chilli::notices(scope);}
    if scope.recipe_id==THREE_BEAN_GF_CHICKEN_SCOPE.recipe_id || scope.recipe_id==CHILLED_GF_CHICKEN_SCOPE.recipe_id {
        return gf_chicken::notices(scope);
    }
    if scope.recipe_id==PORTABLE_HOT_SCOPE.recipe_id || scope.recipe_id==PORTABLE_COLD_SCOPE.recipe_id {
        let hot=scope.recipe_id==PORTABLE_HOT_SCOPE.recipe_id;
        return vec![scope.portion_convention.into(),
            if hot {"At-home Dinner for one to four adults with no declared dietary restrictions. Requires a hob, oven and fridge. Cook the full four-share batch; reserve only actual remaining original shares for the paired next-day portable lunch.".into()}
            else {"Admit the paired original dinner first, then choose its saved batch. Portable Lunch for one to three adults with no declared dietary restrictions, on the next local day and within 24 hours of original cooking, or sooner if a label requires it.".into()},
            if hot {"Allow an estimated 62 minutes to hot service, including 32 active minutes, then conditional cooling and handling for any remaining shares. Refrigerate sooner if cool; monitor and retime within two hours if still warm. The schedule does not prove readiness or cooling.".into()}
            else {"Plan five active minutes to pack at home and five to serve cold, in separate sessions. Set packing completion and departure times and confirm chilling for this lunch. Keep the food at a checked 0–5°C with an insulated bag and frozen ice packs during travel, then promptly use a working fridge on arrival until lunch.".into()},
            "Use only food with known proper original cooking, cooling and uninterrupted chilling. Unknown or failed handling means do not use the food. No bag-only full-day route, guaranteed journey duration, freezing, reheating, fresh lunch ingredients, extras or further carry-over. Handling times are unmeasured estimates.".into(),
            "Adapted from Wikibooks revision 4525784 under CC BY-SA 4.0. No dietary guarantee, observed cooking or travel handling, measured yield, nutrition or calibrated portion claim. Actual labels and conditions still need checking.".into()];
    }
    if scope.recipe_id==COURGETTE_HOT_SCOPE.recipe_id || scope.recipe_id==COURGETTE_COLD_SCOPE.recipe_id {
        let hot=scope.recipe_id==COURGETTE_HOT_SCOPE.recipe_id;
        return vec![scope.portion_convention.into(),
            if hot {"At-home Dinner for one to four adults with no declared dietary restrictions. Requires a hob, oven and fridge. The full batch uses 400 g dry wheat pasta, 250 g grated hard dairy cheese, two eggs, 200 ml milk, 55 g butter, three medium courgettes and 4 litres of cooking water, which is drained.".into()}
            else {"Admit the original dinner recipe first, then select its saved batch in this lunch's leftover controls. At-home Lunch on the next local day only, within 24 hours of original cooking completion. Requires a fridge and one to three adults with no declared dietary restrictions.".into()},
            if hot {"Allow an estimated 62 minutes to hot service: 32 active minutes and 30 minutes baking. If shares remain, allow separate handling and cooling afterwards: five active minutes, a nominal 50-minute cooling allowance, then five active minutes for checking and refrigeration. Refrigerate sooner if cool; the schedule does not prove readiness or cooling.".into()}
            else {"Allow an estimated five active minutes to check the original food history, retrieve and plate the remaining shares. Original cooking, cooling within two hours and continuous correct refrigeration must be known. Unknown or failed handling means do not use these leftovers.".into()},
            "No reheating, freezing, travel, extra batch or further carry-over. No dietary guarantee, measured cooked yield, nutrition, cook test or calibrated portion claim. Actual ingredient labels and handling still need checking.".into(),
            "Adapted from Wikibooks revision 4525784 under CC BY-SA 4.0. Stock, salt and pepper are omitted; cooking water and dish-greasing butter are explicit authored quantities. Timing estimates and the stricter 24-hour home-leftover limit are adaptations.".into()];
    }
    if scope.recipe_id==CHILLED_CHICKEN_EXTRA_SCOPE.recipe_id {
        return vec![scope.portion_convention.into(),
            "Original operational text generated by Codex under owner commission, not a human-authored published recipe. PersonalOnly rights; no published recipe or licence URL, image or distribution grant.".into(),
            "Admit this exact definition independently before its paired base. Use only as that base's declared extra for one explicitly selected current confirmed-None Adult Regular ×1 person; no standalone choice, second recipient, compensation, travel, batch link or reuse.".into(),
            "Commercially fully cooked chilled plain edible ready-to-eat pieces must have credible actual cold-use, ingredient, date and storage label support. Correct prior refrigeration, clean scales, dedicated utensils/separate dish and protection of both salads remain required and unobserved.".into(),
            "Retrieve at T−5, weigh exactly 50 g and serve separately at T. Five minutes of full attention is an unmeasured estimate, not a safe-exposure interval or completion guarantee. No continuously reserved EquipmentCode during countertop weighing/plating; brief Fridge access, scales, utensils and correct prior refrigeration remain required conditions.".into(),
            "Chicken conflicts Vegetarian and Vegan; no hard requirement is supported. No actual product, temperature, capacity, separation, nutrition, CookTest, calibration, storage life or calendar qualification is supplied.".into()];
    }
    if scope.recipe_id==THREE_BEAN_TWO_UNIT_CHICKEN_EXTRA_SCOPE.recipe_id {
        return vec![scope.portion_convention.into(),
            "Exactly two confirmed Adult Regular ×1 attendees, at least one resident and at most one temporary guest, at-home Lunch with Fridge. Each base attendee may have None or exactly Vegetarian; all nine intrinsic input conditions still apply even None.".into(),
            "Two unchanged separate base units: aggregate 160 g each drained ready-to-eat compatible kidney beans/chickpeas/black beans, 80 g carrot, 40 g onion, 120 g pepper, 20 ml olive oil, 30 ml cider vinegar and 10 g sugar. Reserve unmeasured 30-minute assembly plus at least 60-minute passive covered home-fridge chilling, same-day first handling T−90. Continue chilling and delay/refuse if needed.".into(),
            "Independently admit the exact chicken definition first, even for a base with no selected extra. Zero or one separate fixed 50 g extra may go only to one explicitly named current confirmed-None Adult Regular ×1 attendee. Chicken never enters either base, shares utensils or becomes Vegetarian combined-meal support.".into(),
            "No automatic scaling, make-extra, multiple recipients, unknown/additional hard requirements, linked/shared batch, travel or reuse. Changed/departed people retain exact saved food/history as conflict until explicit removal or eligible replacement; never redirect an extra.".into(),
            "Actual input/manufacturing/processing, cold chain, labels/dates, physical separation, temperature and completion remain unobserved. No image, nutrition, CookTest, calibration or calendar qualification is supplied.".into()];
    }
    let vegetarian = scope.recipe_id == THREE_BEAN_TWO_UNIT_VEGETARIAN_SCOPE.recipe_id;
    let mut notices = if scope.recipe_id == PORRIDGE_LARGER_SCOPE.recipe_id {
        vec![
            "Fresh at-home Breakfast for exactly one confirmed unrestricted resident adult with Hob. Adult Larger counts one whole authored unit: 135/2 g plain rolled oats, 270 ml water and 270 ml plain dairy milk; cooked mass and volume remain unknown.".into(),
            "Use effective Larger from saved Breakfast preference or an explicit same-meal override. Every applicable adjustment must be1, including earlier overridden preferences; no compensation, guest, extra unit, linked batch or reuse. Native scaling stays1.".into(),
            "Reserve20minutes of full attention as an unmeasured estimate. Preserve the source five-minute simmer procedure; it is not measured completion for the Larger quantity. Delay service if preparation needs longer. Raw dairy milk means liquid input, not unpasteurised milk.".into(),
        ]
    } else if scope.recipe_id == THREE_BEAN_SMALLER_SCOPE.recipe_id {
        vec![
            "Cold at-home Lunch for one confirmed unrestricted resident adult. Fixed Smaller inputs: 60 g each newly opened ready-to-eat canned kidney beans, chickpeas and black beans, weighed after rinsing and draining; 30 g carrot, 15 g onion, 45 g pepper, 15/2 ml olive oil, 45/4 ml cider vinegar and 15/4 g sugar.".into(),
            "Use Smaller from a saved Lunch preference or an explicit same-meal choice. Every applicable multiplier must remain 1, including earlier overridden preferences. The complete salad is one unit; native scaling stays 1.".into(),
            "Reserve 15 minutes of full attention for assembly, then at least 60 minutes covered home-fridge chilling at 0–5 °C. First handling starts 75 minutes before same-day service. Continue chilling and delay service if needed; the schedule is an estimate, not observed completion.".into(),
            "Actual cooked ready-to-eat cold-use, date and storage labels remain required. No guest, child, hard dietary requirement, travel, extra, previous-day handling or reuse is supported. Unused can contents follow their actual labels separately and confer no stock credit.".into(),
        ]
    } else if vegetarian {
        vec![
            "Cold at-home lunch for exactly two confirmed adults at Regular ×1, at least one resident and at most one temporary guest. Each person's requirements must be None or exactly Vegetarian; prepare two separately measured covered containers, one whole operational unit each.".into(),
            "Aggregate inputs: 160 g each vegetarian-compatible commercially canned cooked ready-to-eat kidney beans, chickpeas and black beans weighed after rinse/drain; 80 g plain carrot, 40 g plain onion, 120 g plain pepper, 20 ml plain olive oil, 30 ml plain cider vinegar and 10 g plain sugar. Every input must meet the exact intrinsic conditions even when consumers confirm None.".into(),
            "No animal body-part ingredients, or products/processing aids made from them; no meat/fish/bone stocks, carcass fats, gelatine or isinglass. Actual product/manufacturer processing and labels remain unknown; no brand, certification, manufacturing or allergen compliance is inferred.".into(),
            "Reserve 30 minutes of full attention as an unmeasured assembly estimate, followed by at least 60 minutes covered home-fridge chilling at 0–5 °C. Begin first handling 90 minutes before same-day service; continue chilling and delay/refuse service if either container is not properly chilled.".into(),
            "No automatic halving/scaling, extras, travel or reuse. Removing a person retains the exact saved meal as a conflict; explicitly remove it or choose a genuinely eligible replacement. An unrestricted one-unit recipe is not a Vegetarian-compatible replacement simply because the count fits.".into(),
        ]
    } else if scope.recipe_id == THREE_BEAN_TWO_UNIT_SCOPE.recipe_id {
        vec![
            "Cold at-home lunch for exactly two confirmed unrestricted adults at Regular ×1, at least one resident and at most one temporary guest. Prepare two separately measured covered containers; one whole operational unit each.".into(),
            "Aggregate inputs:160g each cooked ready-to-eat canned kidney beans, chickpeas and black beans weighed after rinse/drain;80g carrot,40g onion,120g pepper,20ml olive oil,30ml cider vinegar and10g sugar. These are authored fixed quantities, not source serving, can/cup or density conversions.".into(),
            "Reserve30minutes of full attention as an unmeasured assembly estimate, followed by at least60minutes covered home-fridge chilling at0-5C. Begin first handling90minutes before service on the same local day; delay/refuse service and continue chilling if either container is not properly chilled.".into(),
            "No automatic halving or scaling, extras, travel or reuse. Removing a person retains this saved two-unit meal as a conflict; explicitly replace it with the one-unit recipe or remove it. Actual ready-to-eat labels, dates and handling remain required; no product suitability or measured completion is inferred.".into(),
        ]
    } else if scope.recipe_id == THREE_BEAN_PERSONAL_SCOPE.recipe_id {
        vec![
            "Cold at-home lunch for one unrestricted resident adult:80g each cooked ready-to-eat canned kidney beans, chickpeas and black beans weighed after rinse/drain;40g carrot,20g onion,60g pepper,10ml olive oil,15ml cider vinegar and5g sugar.".into(),
            "These are authored amounts, not one eighth of the source's eight servings or cup/can/density conversions. Newly opened ready-to-eat cans and actual label instructions are required; no product suitability is inferred.".into(),
            "Reserve15minutes of full attention for assembly, then at least60minutes of passive covered home-fridge chilling at0-5C. The75minute interval is nominal; continue chilling and delay/refuse service if not properly chilled.".into(),
            "First handling starts75minutes before the meal in the same local day. No travel, previous-day handling, leftovers or reuse; unused can contents are outside this unit and follow their actual labels.".into(),
        ]
    } else if scope.recipe_id == AGLIO_PERSONAL_SCOPE.recipe_id {
        vec![
            "Fresh at-home dinner for one unrestricted adult:125g dry wheat spaghetti,55/2g extra virgin olive oil,2 peeled whole garlic cloves and1500ml water.".into(),
            "Retain/use60ml of that same cooking water; it adds no extra water demand. Parsley, chilli, unmeasured seasoning and linked butter/herbs are omitted.".into(),
            "Pasta and olive oil are relevant pantry essentials; deduct only an explicit exact current-demand stock confirmation. No stock or product suitability is inferred.".into(),
        ]
    } else {
        vec!["Fresh at-home breakfast for one unrestricted adult:45g plain rolled oats,180ml water and180ml dairy milk.".into(),
            "Raw dairy milk form denotes input state, not unpasteurised milk; brand/fat level and actual product suitability are unverified.".into()]
    };
    notices.push(scope.portion_convention.into());
    notices.push(format!("{}-minute full-attention reservation is an unmeasured estimate, not a completion guarantee.",scope.attention_reservation_minutes));
    notices.push(if vegetarian {
        "Only the exact conditional Vegetarian input definition is reviewed; Vegan, allergens, ingredient exclusions and any other/additional hard requirement remain unsupported. No actual product, other ages/scales, substitutions, extras, leftovers, reuse, image, nutrition, human cook test or portion calibration is qualified."
    } else {
        "No hard dietary requirements, other ages/scales, substitutions, extras, leftovers, reuse, image or nutrition are qualified. No human cook test or portion calibration is claimed."
    }.into());
    notices
}

/// Apply the scope by recipe identity, including stale or forged revisions and
/// digests. Matching only the permitted reference would let altered references
/// avoid the scope. Catalogue admission must separately refuse unregistered IDs.
pub fn requires_personal_scope(recipe_id: &RecipeId) -> bool {
    scope_for_recipe_id(recipe_id).is_some()
}

fn is_two_unit_scope(scope: &PersonalUseScope) -> bool {
    scope.recipe_id == THREE_BEAN_TWO_UNIT_SCOPE.recipe_id
        || scope.recipe_id == THREE_BEAN_TWO_UNIT_VEGETARIAN_SCOPE.recipe_id
        || scope.recipe_id == THREE_BEAN_TWO_UNIT_CHICKEN_EXTRA_SCOPE.recipe_id
        || scope.recipe_id == THREE_BEAN_GF_CHICKEN_SCOPE.recipe_id
}
fn permitted_requirements(scope: &PersonalUseScope, requirements: &[HardRequirement]) -> bool {
    if scope.recipe_id==THREE_BEAN_GF_CHICKEN_SCOPE.recipe_id {return gf_chicken::base_requirements(requirements);}
    requirements.is_empty() || ((scope.recipe_id == THREE_BEAN_TWO_UNIT_VEGETARIAN_SCOPE.recipe_id
        || scope.recipe_id == THREE_BEAN_TWO_UNIT_CHICKEN_EXTRA_SCOPE.recipe_id)
        && matches!(requirements, [HardRequirement::Diet { code: DietCode::Vegetarian }]))
}

fn refusal(code: ErrorCode, field: &str) -> DomainError {
    DomainError::new(code, &format!("/personal_use_scope/{field}"))
}
fn unsupported(field: &str) -> DomainError {
    refusal(ErrorCode::UnsupportedCapability, field)
}
fn pinned(reference: &RecipeRef, scope: &PersonalUseScope) -> bool {
    reference.recipe_id.0 == scope.recipe_id
        && reference.revision.0 == scope.recipe_revision
        && reference.digest.0 == scope.recipe_digest
}
fn one(value: PositiveRational) -> bool {
    value == PositiveRational::new(1, 1).expect("constant unit rational")
}
fn batch_id(selection: &MealSelection) -> &BatchId {
    match &selection.batch_use {
        BatchUse::NewBatch { batch_id, .. } | BatchUse::Leftover { batch_id, .. } => batch_id,
    }
}
fn overrides_are_unit(
    overrides: &MealOverrides,
    attendees: &[AttendeeRequirement],
    field: &str,
    required_preset: Option<PortionPresetName>,
) -> Result<(), DomainError> {
    if overrides.portions.len() > attendees.len() {
        return Err(unsupported(field));
    }
    for value in overrides.portions.iter() {
        if !attendees.iter().any(|attendee| attendee.person==value.person) {
            return Err(refusal(ErrorCode::IdentityMismatch, field));
        }
        if required_preset.is_some_and(|preset|value.preset != preset) || !one(value.adjustment) {
            return Err(unsupported(field));
        }
    }
    Ok(())
}

/// Use the same effective immutable deadline as core food derivation. Compare
/// the first-handling instant in that IANA zone, rather than subtracting wall
/// time or comparing UTC dates. Physical label/fridge/readiness compliance is
/// an operational instruction, never a sensor or product observation here.
fn check_cold_home_day(
    meal: &MealRequirements,
    selection: &MealSelection,
    options: &BoardOptions,
    timing: ColdHomeTiming,
) -> Result<(), DomainError> {
    let deadline = selection.meal_overrides.eat_by.0.as_ref()
        .or(meal.eat_by.0.as_ref())
        .ok_or_else(|| unsupported("first_handling_time"))?;
    let validated = ZonedMoment::new(deadline.local_date.clone(), deadline.local_time.clone(),
        deadline.zone.clone(), deadline.resolution, deadline.instant.clone())
        .map_err(|_| refusal(ErrorCode::InvalidValue,"first_handling_time"))?;
    if validated.local_date != meal.slot.date {
        return Err(refusal(ErrorCode::IdentityMismatch,"first_handling_time"));
    }
    if options.overrides.as_ref().and_then(|value|value.eat_by.0.as_ref()).is_some_and(|value|value!=deadline)
        || options.eat_by_time.as_ref().is_some_and(|value|value!=&deadline.local_time)
        || options.time_resolution.is_some_and(|value|value!=deadline.resolution)
    {
        return Err(refusal(ErrorCode::IdentityMismatch,"request_meal_time"));
    }
    let zone: chrono_tz::Tz = validated.zone.0.parse()
        .map_err(|_| refusal(ErrorCode::InvalidValue,"first_handling_time"))?;
    let use_at = chrono::DateTime::parse_from_rfc3339(&validated.instant.0)
        .map_err(|_| refusal(ErrorCode::InvalidValue,"first_handling_time"))?;
    let first = use_at.checked_sub_signed(chrono::Duration::seconds(timing.first_handling_seconds))
        .ok_or_else(|| refusal(ErrorCode::InvalidValue,"first_handling_time"))?;
    if first.with_timezone(&zone).format("%Y-%m-%d").to_string() != meal.slot.date.0 {
        return Err(unsupported("same_day_first_handling"));
    }
    Ok(())
}

/// Reject use outside the exact fresh operational unit and adult cohort of its
/// compiled personal recipe. The plan is the authoritative same-week context;
/// BoardOptions must be the actual request options, not a reconstructed default.
/// Relevant existing links and batch owners must not be omitted by the caller.
pub fn check_personal_use_scope(
    recipe: &RecipeRevision,
    reference: &RecipeRef,
    meal: &MealRequirements,
    selection: &MealSelection,
    plan: &WeekPlan,
    options: &BoardOptions,
) -> Result<(), DomainError> {
    let scope = scope_for_recipe_id(&reference.recipe_id).ok_or_else(||refusal(ErrorCode::IdentityMismatch,"recipe_reference"))?;
    if scope.recipe_id==CHILLED_CHICKEN_EXTRA_SCOPE.recipe_id || scope.recipe_id==CHILLED_GF_CHICKEN_SCOPE.recipe_id {
        return Err(unsupported("extra_only_never_standalone"));
    }
    let gf_pair=scope.recipe_id==THREE_BEAN_GF_CHICKEN_SCOPE.recipe_id;
    let paired_base=scope.recipe_id==THREE_BEAN_TWO_UNIT_CHICKEN_EXTRA_SCOPE.recipe_id || gf_pair;
    let larger = scope.recipe_id == PORRIDGE_LARGER_SCOPE.recipe_id;
    let smaller = scope.recipe_id == THREE_BEAN_SMALLER_SCOPE.recipe_id;
    let fixed_variant = larger || smaller;
    let required_preset = if larger {PortionPresetName::Larger} else if smaller {PortionPresetName::Smaller} else {PortionPresetName::Regular};
    let meal_field = match scope.meal_type { MealType::Breakfast=>"breakfast", MealType::Lunch=>"lunch", MealType::Dinner=>"dinner" };
    let portion_field = match scope.meal_type { MealType::Breakfast=>"breakfast_portion_override", MealType::Lunch=>"lunch_portion_override", MealType::Dinner=>"dinner_portion_override" };
    // Exact content binding prevents a forged recipe body from retaining the
    // permitted reference while changing amounts, forms, method or reuse inputs.
    if !pinned(reference, scope)
        || recipe.recipe_id != reference.recipe_id
        || recipe.revision != reference.revision
        || selection.recipe != *reference
    {
        return Err(refusal(ErrorCode::IdentityMismatch, "recipe_reference"));
    }
    if recipe_digest(recipe).map_err(|_| refusal(ErrorCode::InvalidValue, "recipe"))?
        != reference.digest
    {
        return Err(refusal(ErrorCode::DigestMismatch, "recipe"));
    }
    // Keep malformed inputs bounded and private. This is not a replacement for
    // core context/eligibility/quantity validation at the authoritative boundary.
    if meal.validate_shape().is_err()
        || selection.validate_shape().is_err()
        || plan.validate_shape().is_err()
    {
        return Err(refusal(ErrorCode::InvalidValue, "context"));
    }
    if meal.slot != selection.slot || plan.week_id != meal.slot.week_id {
        return Err(refusal(ErrorCode::IdentityMismatch, "slot"));
    }
    if !meal.enabled || meal.slot.meal_type != scope.meal_type {
        return Err(unsupported(meal_field));
    }
    if chilli::is_family(scope) {
        return chilli::check(recipe,reference,meal,selection,plan,options,
            &courgette_reference(&CHILLI_BATCH_SCOPE),&courgette_reference(&CHILLI_WRAPS_SCOPE),
            &courgette_reference(&CHILLI_POTATO_SCOPE));
    }
    if scope.recipe_id==COURGETTE_HOT_SCOPE.recipe_id || scope.recipe_id==COURGETTE_COLD_SCOPE.recipe_id {
        return courgette::check(recipe,reference,meal,selection,plan,options,
            &courgette_reference(&COURGETTE_HOT_SCOPE),&courgette_reference(&COURGETTE_COLD_SCOPE));
    }
    if scope.recipe_id==PORTABLE_HOT_SCOPE.recipe_id || scope.recipe_id==PORTABLE_COLD_SCOPE.recipe_id {
        return portable::check(recipe,reference,meal,selection,plan,options,
            &courgette_reference(&PORTABLE_HOT_SCOPE),&courgette_reference(&PORTABLE_COLD_SCOPE));
    }
    let timing=cold_home_timing(reference)?;
    let cold_home = timing.is_some();
    if scope.recipe_id == OVERNIGHT_OATS_SCOPE.recipe_id {
        overnight::check_timing(meal, selection, options)?;
    } else if cold_home {
        if meal.location != MealLocation::AtHome || !meal.equipment.contains(&EquipmentCode::Fridge) {
            return Err(unsupported("at_home_cold_preparation"));
        }
        check_cold_home_day(meal,selection,options,timing.unwrap())?;
    } else if meal.location != MealLocation::AtHome
        || meal.mobility.temperature == MobilityTemperature::Cold
        || !meal.equipment.contains(&EquipmentCode::Hob)
    {
        return Err(unsupported("at_home_fresh_preparation"));
    }
    let count=if cohort(scope)==Cohort::TwoAdultsAtLeastOneResident {2}else{1};
    if meal.attendees.len() != count {
        return Err(unsupported(if count==2 {"two_attendees"} else {"single_attendee"}));
    }
    let residents=meal.attendees.iter().filter(|attendee|matches!(attendee.person,PersonRef::Diner{..})).count();
    if (cohort(scope)==Cohort::SingleResidentAdult && residents!=1)
        || (cohort(scope)==Cohort::TwoAdultsAtLeastOneResident && residents==0) {
        return Err(unsupported("resident_diner"));
    }
    for attendee in meal.attendees.iter() {
    if attendee.age_band != AgeBand::Adult {
        return Err(unsupported("adult_source_serving"));
    }
    if !permitted_requirements(scope, &meal.shared_hard_requirements.0)
        || !permitted_requirements(scope, &attendee.hard_requirements.0) {
        return Err(unsupported("hard_requirements"));
    }
    // Require every applicable layer to be unchanged. A later override cannot
    // launder an unsupported earlier portion into an apparently unit total.
    if (!fixed_variant && attendee.portion.preset != PortionPresetName::Regular) || !one(attendee.portion.adjustment) {
        return Err(unsupported("portion_preference"));
    }
    let applicable: Vec<_> = attendee
        .portion
        .meal_overrides
        .iter()
        .filter(|value| value.meal_type == scope.meal_type)
        .collect();
    if applicable.len() > 1
        || applicable
            .iter()
            .any(|value| (!fixed_variant && value.preset != PortionPresetName::Regular) || !one(value.adjustment))
    {
        return Err(unsupported(portion_field));
    }
    }
    overrides_are_unit(
        &selection.meal_overrides,
        &meal.attendees.0,
        "selection_portion_override",
        if fixed_variant {None} else {Some(PortionPresetName::Regular)},
    )?;
    if let Some(value) = &options.overrides {
        if value.validate_shape().is_err() {
            return Err(refusal(ErrorCode::InvalidValue, "request_portion_override"));
        }
        overrides_are_unit(value, &meal.attendees.0, "request_portion_override",
            if fixed_variant {None} else {Some(PortionPresetName::Regular)})?;
    }
    if fixed_variant {
        // Preset precedence follows native resolve_servings. This predicate
        // performs no serving arithmetic and never accepts a transported preset
        // reference in place of the actual current preference/override layers.
        for attendee in meal.attendees.iter() {
            let mut effective = attendee.portion.preset;
            if let Some(value)=attendee.portion.meal_overrides.iter().find(|value|value.meal_type==scope.meal_type) {effective=value.preset;}
            if let Some(value)=selection.meal_overrides.portions.iter().find(|value|value.person==attendee.person) {effective=value.preset;}
            if let Some(value)=options.overrides.as_ref().and_then(|overrides|overrides.portions.iter().find(|value|value.person==attendee.person)) {effective=value.preset;}
            if effective!=required_preset {return Err(unsupported(if larger {"effective_larger_preset"} else {"effective_smaller_preset"}));}
        }
    }
    if selection.servings.len() != count {
        return Err(unsupported(if count==2 {"two_serving_allocations"} else {"single_serving_allocation"}));
    }
    for serving in selection.servings.iter() {
    if !meal.attendees.iter().any(|attendee|attendee.person==serving.person)
        || serving.preset_ref.recipe != *reference
        || serving.preset_ref.age_band != AgeBand::Adult
        || serving.preset_ref.meal_type != scope.meal_type
    {
        return Err(refusal(ErrorCode::IdentityMismatch, "serving_identity"));
    }
    if serving.preset_ref.preset != required_preset || !one(serving.servings) {
        return Err(unsupported("unchanged_source_serving"));
    }
    }
    if meal.attendees.iter().any(|attendee|selection.servings.iter().filter(|serving|serving.person==attendee.person).count()!=1) {
        return Err(refusal(ErrorCode::IdentityMismatch,"serving_identity"));
    }
    if options.make_extra == Some(true)
        || options.leftover_source.is_some()
        || (gf_pair && (selection.portable_cold.is_some() || options.portable_cold.is_some()))
        || (!paired_base && (options
            .extra_recipients
            .as_ref()
            .is_some_and(|value| !value.is_empty())
            || !selection.extras.is_empty()))
        || !recipe.reused_inputs.is_empty()
    {
        return Err(unsupported("extras_or_reuse"));
    }
    if paired_base {
        if gf_pair {check_gf_chicken_pair_allocation(selection,options)?;}
        else {check_chicken_pair_allocation(selection,options)?;}
    }
    match &selection.batch_use {
        BatchUse::NewBatch {
            production_servings,
            ..
        } if *production_servings==PositiveRational::new(count as u64,1).expect("fixed operational count") => {}
        _ => return Err(unsupported("fresh_unit_batch")),
    }
    let selected_batch = batch_id(selection);
    if plan.batch_links.iter().any(|link| {
        link.source_slot == selection.slot
            || link.target_slot == selection.slot
            || &link.batch_id == selected_batch
    }) {
        return Err(unsupported("batch_reuse_link"));
    }
    if plan
        .selections
        .iter()
        .any(|other| other.slot != selection.slot && batch_id(other) == selected_batch)
    {
        return Err(unsupported("batch_already_used_elsewhere"));
    }
    // leftovers_allowed/reheating/cold_storage are availability permissions,
    // not evidence that the selected batch is reused. Only the actual batch use,
    // allocations, links and request options establish planned fresh-only use.
    Ok(())
}

/// Exact saved allocation shape only. Current eligibility is checked against
/// actual meal attendees below; this does not create admitted food authority.
pub(crate) fn check_chicken_pair_allocation(selection:&MealSelection,options:&BoardOptions)->Result<(),DomainError>{
    if !pinned(&selection.recipe,&THREE_BEAN_TWO_UNIT_CHICKEN_EXTRA_SCOPE) || selection.extras.len()>1 {
        return Err(unsupported("one_declared_chicken_extra"));
    }
    let requested=options.extra_recipients.as_ref();
    if requested.is_some_and(|people|people.len()>1) {return Err(unsupported("one_named_extra_recipient"));}
    if let Some(extra)=selection.extras.first(){
        if extra.extra_id.0!=CHICKEN_EXTRA_ID || !pinned(&extra.recipe,&CHILLED_CHICKEN_EXTRA_SCOPE)
            || extra.recipients.len()!=1 {return Err(unsupported("one_declared_chicken_extra"));}
        let serving=&extra.recipients[0];
        if serving.preset_ref.recipe!=extra.recipe || serving.preset_ref.age_band!=AgeBand::Adult
            || serving.preset_ref.meal_type!=MealType::Lunch || serving.preset_ref.preset!=PortionPresetName::Regular
            || !one(serving.servings) || !selection.servings.iter().any(|base|base.person==serving.person) {
            return Err(refusal(ErrorCode::IdentityMismatch,"extra_serving_identity"));
        }
        if requested.is_some_and(|people|people.0!=vec![serving.person.clone()]) {
            return Err(refusal(ErrorCode::IdentityMismatch,"request_extra_recipient"));
        }
    } else if requested.is_some_and(|people|!people.is_empty()) {
        return Err(unsupported("requested_extra_not_selected"));
    }
    Ok(())
}

/// Validate the sole permitted placement against genuine current requirements.
/// The core separately resolves the exact recipient-only quantity and method.
pub(crate) fn check_chicken_extra_scope(recipe:&RecipeRevision,meal:&MealRequirements,
    selection:&MealSelection,plan:&WeekPlan,options:&BoardOptions)->Result<(),DomainError>{
    check_chicken_pair_allocation(selection,options)?;
    let Some(extra)=selection.extras.first() else {return Ok(());};
    if !pinned(&extra.recipe,&CHILLED_CHICKEN_EXTRA_SCOPE) || recipe.recipe_id!=extra.recipe.recipe_id
        || recipe.revision!=extra.recipe.revision || recipe_digest(recipe)?!=extra.recipe.digest {
        return Err(refusal(ErrorCode::IdentityMismatch,"extra_recipe_reference"));
    }
    if recipe.source.origin!=RecipeSourceOrigin::UserAuthored || recipe.source.url.0.is_some()
        || recipe.text_rights.licence_url.0.is_some() {return Err(refusal(ErrorCode::IdentityMismatch,"extra_original_source"));}
    if !meal.enabled || meal.location!=MealLocation::AtHome || meal.slot.meal_type!=MealType::Lunch
        || !meal.equipment.contains(&EquipmentCode::Fridge) || selection.slot!=meal.slot
        || plan.week_id!=meal.slot.week_id {return Err(unsupported("at_home_chicken_extra"));}
    let person=&extra.recipients[0].person;
    let attendee=meal.attendees.iter().find(|attendee|&attendee.person==person)
        .ok_or_else(||refusal(ErrorCode::ForeignReference,"extra_current_recipient"))?;
    // Confirmed requirements are nonempty and Unconfirmed is refused by the
    // ordinary availability projection. Empty genuine requirements mean None.
    if attendee.age_band!=AgeBand::Adult || !attendee.hard_requirements.is_empty()
        || attendee.portion.preset!=PortionPresetName::Regular || !one(attendee.portion.adjustment)
        || attendee.portion.meal_overrides.iter().filter(|value|value.meal_type==MealType::Lunch)
            .any(|value|value.preset!=PortionPresetName::Regular||!one(value.adjustment)) {
        return Err(unsupported("none_adult_unit_extra_recipient"));
    }
    check_cold_home_day(meal,selection,options,ColdHomeTiming{first_handling_seconds:300,assembly_minutes:5,chilling_minutes:0})
}

#[cfg(test)]
mod tests {
    use super::*;
    use mstrchf_core::recipes::{EvidenceBasis, EvidencePin, ComponentClassView, assess_revision};
    use serde_json::Value;

    struct Input { recipe:RecipeRevision, reference:RecipeRef, meal:MealRequirements,
        selection:MealSelection, plan:WeekPlan, options:BoardOptions, packet:Value }
    impl Input {
        fn check(&self)->Result<(),DomainError>{check_personal_use_scope(&self.recipe,&self.reference,
            &self.meal,&self.selection,&self.plan,&self.options)}
        fn refuses(&self,field:&str){let e=self.check().unwrap_err();assert_eq!(e.retry,Retry::Never);
            assert_eq!(e.issues.len(),1);assert_eq!(e.issues[0].path,format!("/personal_use_scope/{field}"));}
    }
    fn moment(date:&str,time:&str,zone:&str,resolution:Option<Resolution>)->ZonedMoment{
        mstrchf_core::availability::resolve_meal_time(&LocalDate(date.into()),&LocalTime(time.into()),
            &ZoneId(zone.into()),resolution).unwrap()
    }
    fn unit()->PositiveRational{PositiveRational::new(1,1).unwrap()}
    fn fixture(scope:&PersonalUseScope)->Input{
        let bytes=match scope.recipe_id {
            "rc_80000000000000000000000000000001"=>include_bytes!("../../../sites/mstrchf-9000/content/intake/porridge-personal-reviewed-v1/packet.json").as_slice(),
            "rc_80000000000000000000000000000003"=>include_bytes!("../../../sites/mstrchf-9000/content/intake/aglio-personal-reviewed-v1/packet.json").as_slice(),
            _=>include_bytes!("../../../sites/mstrchf-9000/content/intake/three-bean-personal-reviewed-v1/packet.json").as_slice(),
        };
        let packet:Value=serde_json::from_slice(bytes).unwrap();
        let recipe=parse_document(&serde_json::to_vec(&packet["maker_recipe_revision"]).unwrap()).unwrap();
        let reference:RecipeRef=serde_json::from_value(packet["recipe_reference"].clone()).unwrap();
        let mut meal=parse_document::<MealRequirementsDocumentV1>(include_bytes!("../../../tests/fixtures/contracts/meal-requirements.json")).unwrap().meal_requirements;
        meal.slot.meal_type=scope.meal_type;meal.attendees.0.truncate(1);
        meal.attendees[0].hard_requirements.0.clear();meal.shared_hard_requirements.0.clear();
        meal.equipment=BoundedVec(vec![if scope.recipe_id==THREE_BEAN_PERSONAL_SCOPE.recipe_id{EquipmentCode::Fridge}else{EquipmentCode::Hob}]);
        meal.eat_by=Nullable(Some(moment("2026-12-28","12:00:00","Europe/London",None)));
        let selection=MealSelection{portable_cold: None, slot:meal.slot.clone(),recipe:reference.clone(),origin:SelectionOrigin::Fixed,
            repeat_authorised:false,servings:BoundedVec(vec![DinerServing{person:meal.attendees[0].person.clone(),
                preset_ref:PortionPresetRef{recipe:reference.clone(),preset:PortionPresetName::Regular,age_band:AgeBand::Adult,meal_type:scope.meal_type},servings:unit()}]),
            extras:BoundedVec(vec![]),batch_use:BatchUse::NewBatch{batch_id:BatchId("ba_80000000000000000000000000000004".into()),production_servings:unit()},
            meal_overrides:MealOverrides{portions:BoundedVec(vec![]),eat_by:Nullable(None)}};
        let plan=WeekPlan{week_id:meal.slot.week_id.clone(),revision:Revision(1),selections:BoundedVec(vec![]),batch_links:BoundedVec(vec![])};
        Input{recipe,reference,meal,selection,plan,options:BoardOptions::default(),packet}
    }
    fn reviewed<'a>(input:&'a Input,basis:&'a EvidenceBasis,classes:&'a ComponentClassView)->mstrchf_core::recipes::ReviewedRecipe<'a>{
        assess_revision(&input.recipe,basis,classes).unwrap()
    }
    fn diagnostic_inputs(input:&Input)->(EvidenceBasis,ComponentClassView){
        let pins=input.packet["maker_evidence_pins"].as_array().unwrap().iter().map(|pin|EvidencePin{
            evidence_id:serde_json::from_value(pin["evidence_id"].clone()).unwrap(),revision:Revision(1),
            digest:serde_json::from_value(pin["digest"].clone()).unwrap()}).collect();
        (EvidenceBasis::new(input.reference.clone(),pins,vec![]).unwrap(),ComponentClassView::new(input.reference.clone(),vec![]).unwrap())
    }
    #[test]
    fn three_bean_home_fridge_accepts_only_the_disclosed_cold_home_route(){
        let mut input=fixture(&THREE_BEAN_PERSONAL_SCOPE);
        for temperature in [MobilityTemperature::Either,MobilityTemperature::Hot,MobilityTemperature::Cold]{
            input.meal.mobility.temperature=temperature;assert!(input.check().is_ok());
        }
        input.meal.equipment=BoundedVec(vec![EquipmentCode::Hob]);input.refuses("at_home_cold_preparation");
        input.meal.equipment=BoundedVec(vec![EquipmentCode::Fridge]);input.meal.location=MealLocation::OnTheMove;
        input.meal.mobility.cold_storage=true;input.refuses("at_home_cold_preparation");
        input.meal.location=MealLocation::AtHome;input.meal.attendees[0].person=PersonRef::Guest{occasion_id:OccasionId("oc_80000000000000000000000000000001".into()),guest_id:GuestId("gu_80000000000000000000000000000001".into())};
        input.selection.servings[0].person=input.meal.attendees[0].person.clone();input.refuses("resident_diner");
        let notices=notices_for_recipe(&input.reference.recipe_id).join(" ");
        assert!(notices.contains("15minutes of full attention")&&notices.contains("60minutes of passive"));
        assert!(!notices.contains("75-minute full-attention"));
    }
    #[test]
    fn three_bean_first_handling_is_same_local_day_at_the_exact_75minute_boundary(){
        let mut input=fixture(&THREE_BEAN_PERSONAL_SCOPE);
        input.meal.eat_by=Nullable(Some(moment("2026-12-28","01:15:00","Europe/London",None)));assert!(input.check().is_ok());
        input.meal.eat_by=Nullable(Some(moment("2026-12-28","01:14:59","Europe/London",None)));input.refuses("same_day_first_handling");
        input.meal.slot.date=LocalDate("2026-07-01".into());input.selection.slot=input.meal.slot.clone();
        input.meal.eat_by=Nullable(Some(moment("2026-07-01","01:15:00","Europe/London",None)));
        assert_eq!(input.meal.eat_by.0.as_ref().unwrap().instant.0,"2026-07-01T00:15:00Z");assert!(input.check().is_ok(),"First handling is prior UTC day but same local day");
        input.meal.slot.date=LocalDate("2026-12-28".into());input.selection.slot=input.meal.slot.clone();
        input.meal.eat_by=Nullable(Some(moment("2026-12-28","01:14:59","America/Los_Angeles",None)));input.refuses("same_day_first_handling");
        input.meal.eat_by=Nullable(None);input.refuses("context");
    }
    #[test]
    fn three_bean_effective_override_deadline_and_dst_instants_cannot_hide_prior_day_handling(){
        let mut input=fixture(&THREE_BEAN_PERSONAL_SCOPE);
        input.selection.meal_overrides.eat_by=Nullable(Some(moment("2026-12-28","01:14:59","Europe/London",None)));
        input.refuses("same_day_first_handling");
        input.selection.meal_overrides.eat_by=Nullable(Some(moment("2026-12-28","01:15:00","Europe/London",None)));
        input.options.overrides=Some(input.selection.meal_overrides.clone());assert!(input.check().is_ok());
        input.options.overrides.as_mut().unwrap().eat_by=Nullable(Some(moment("2026-12-28","12:00:00","Europe/London",None)));input.refuses("request_meal_time");
        input.options=BoardOptions::default();input.meal.slot.date=LocalDate("2026-03-29".into());input.selection.slot=input.meal.slot.clone();
        input.selection.meal_overrides.eat_by=Nullable(Some(moment("2026-03-29","02:15:00","Europe/London",None)));assert!(input.check().is_ok(),"Actual75minutes traverses spring gap");
        input.meal.slot.date=LocalDate("2026-10-25".into());input.selection.slot=input.meal.slot.clone();
        input.selection.meal_overrides.eat_by=Nullable(Some(moment("2026-10-25","01:15:00","Europe/London",Some(Resolution::Earlier))));assert!(input.check().is_ok());
        input.selection.meal_overrides.eat_by=Nullable(Some(moment("2026-10-25","01:14:59","Europe/London",Some(Resolution::Earlier))));input.refuses("same_day_first_handling");
        input.selection.meal_overrides.eat_by=Nullable(Some(moment("2026-10-25","01:15:00","Europe/London",Some(Resolution::Later))));assert!(input.check().is_ok());
    }
    #[test]
    fn three_bean_core_unit_total_cannot_admit_multiple_or_compensating_portions(){
        let mut input=fixture(&THREE_BEAN_PERSONAL_SCOPE);let (basis,classes)=diagnostic_inputs(&input);
        let mut second=input.meal.attendees[0].clone();second.person=PersonRef::Diner{diner_id:DinerId("dn_80000000000000000000000000000002".into())};
        input.meal.attendees[0].portion.adjustment=PositiveRational::new(1,2).unwrap();second.portion.adjustment=PositiveRational::new(1,2).unwrap();input.meal.attendees.0.push(second);
        let r=reviewed(&input,&basis,&classes);let portions=mstrchf_core::portions::resolve_servings(&r,&input.meal,&input.selection.meal_overrides).unwrap();
        assert_eq!(portions.len(),2);assert!(mstrchf_core::portions::scale_recipe(&r,unit()).is_ok());input.selection.servings=BoundedVec(portions);input.refuses("single_attendee");
        let mut input=fixture(&THREE_BEAN_PERSONAL_SCOPE);let (basis,classes)=diagnostic_inputs(&input);
        input.meal.attendees[0].portion.adjustment=PositiveRational::new(2,1).unwrap();input.selection.meal_overrides.portions=BoundedVec(vec![PersonPortionOverride{
            person:input.meal.attendees[0].person.clone(),preset:PortionPresetName::Regular,adjustment:unit()}]);
        let r=reviewed(&input,&basis,&classes);assert_eq!(mstrchf_core::portions::resolve_servings(&r,&input.meal,&input.selection.meal_overrides).unwrap()[0].servings,unit());input.refuses("portion_preference");
        input.meal.attendees[0].portion.adjustment=unit();input.meal.attendees[0].portion.meal_overrides=BoundedVec(vec![MealPortionOverride{meal_type:MealType::Lunch,preset:PortionPresetName::Regular,adjustment:PositiveRational::new(1,2).unwrap()}]);input.refuses("lunch_portion_override");
        input.meal.attendees[0].portion.meal_overrides.0.clear();input.selection.meal_overrides.portions[0].adjustment=PositiveRational::new(1,2).unwrap();input.refuses("selection_portion_override");
        input.selection.meal_overrides.portions[0].adjustment=unit();input.options.overrides=Some(input.selection.meal_overrides.clone());input.options.overrides.as_mut().unwrap().portions[0].adjustment=PositiveRational::new(2,1).unwrap();input.refuses("request_portion_override");
    }
    #[test]
    fn three_bean_identity_diet_and_reuse_changes_are_refused_independently(){
        let mut input=fixture(&THREE_BEAN_PERSONAL_SCOPE);input.recipe.ingredients[0].quantity.0.as_mut().unwrap().amount=PositiveRational::new(81,1).unwrap();input.refuses("recipe");
        let mut input=fixture(&THREE_BEAN_PERSONAL_SCOPE);input.meal.attendees[0].age_band=AgeBand::Child4To6;input.refuses("adult_source_serving");
        let mut input=fixture(&THREE_BEAN_PERSONAL_SCOPE);input.meal.shared_hard_requirements=BoundedVec(vec![HardRequirement::Diet{code:DietCode::Vegan}]);input.meal.attendees[0].hard_requirements=input.meal.shared_hard_requirements.clone();input.refuses("hard_requirements");
        let mut input=fixture(&THREE_BEAN_PERSONAL_SCOPE);input.options.make_extra=Some(true);input.refuses("extras_or_reuse");input.options.make_extra=None;
        input.options.leftover_source=Some(input.selection.slot.clone());input.refuses("extras_or_reuse");input.options.leftover_source=None;
        input.selection.batch_use=BatchUse::Leftover{batch_id:BatchId("ba_80000000000000000000000000000004".into()),component_ids:BoundedVec(vec![input.recipe.components[0].component_id.clone()])};input.refuses("fresh_unit_batch");
        let mut input=fixture(&THREE_BEAN_PERSONAL_SCOPE);let mut duplicate=input.selection.clone();duplicate.slot.date=LocalDate("2026-12-29".into());input.plan.selections=BoundedVec(vec![duplicate]);input.refuses("batch_already_used_elsewhere");
    }
    #[test]
    fn three_bean_registration_does_not_relax_existing_hot_personal_predicates(){
        for scope in [&PORRIDGE_PERSONAL_SCOPE,&AGLIO_PERSONAL_SCOPE]{let mut input=fixture(scope);assert!(input.check().is_ok());
            input.meal.mobility.temperature=MobilityTemperature::Cold;input.refuses("at_home_fresh_preparation");
            input.meal.mobility.temperature=MobilityTemperature::Either;input.meal.equipment=BoundedVec(vec![EquipmentCode::Fridge]);input.refuses("at_home_fresh_preparation");
            input.meal.equipment=BoundedVec(vec![EquipmentCode::Hob]);input.meal.eat_by=Nullable(None);input.refuses("context");
        }
    }
}
