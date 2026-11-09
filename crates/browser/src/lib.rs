//! Device-local browser boundary. All IO, clock and transaction ownership belong to the host.
use mstrchf_contracts::*;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use std::{cell::RefCell, collections::BTreeSet};

pub mod catalogue;
pub mod calendar_view;
pub mod maker_packets;
pub mod personal_scope;
pub mod public_catalogue;
pub mod planner_view;
mod portable;
mod shopping_list;
mod product_match_context;
pub mod product_source_reads;
pub mod basket_source_reads;
pub mod basket_addition_review;
pub mod product_meaning_review;
pub mod product_semantic_review;
pub mod product_meaning_admission;
mod booking;
pub mod calendar_owner;
mod calendar_publication;
mod privacy_essentials;

mod legacy_food {
    use mstrchf_contracts::*;
    use mstrchf_core::{food::*, recipes::*};
    fn error(code: ErrorCode, path: &str) -> DomainError {
        DomainError::new(code, path)
    }
    pub(crate) fn validate_premises() -> Result<(), DomainError> {
        static CHECK: std::sync::OnceLock<Result<(), DomainError>> = std::sync::OnceLock::new();
        CHECK
            .get_or_init(|| {
                if bytes_digest(include_bytes!(
                    "../assets/catalogue/engineering-v1/food-premises-v1.rs"
                ))
                .0 != "f1520b18aaa63c7ec3fef4fae0ed012ef67531fb3f05a7cd8d990a1d5b446718"
                    || bytes_digest(include_bytes!(
                        "../assets/catalogue/engineering-v1/engineering-placeholder.png"
                    ))
                    .0 != "ea763c6667d13d84e962dfc3e08ea3c75f9f29cb3543543156e4065d1957b9a0"
                {
                    return Err(error(
                        ErrorCode::DigestMismatch,
                        "/catalogue/legacy_engineering_premises",
                    ));
                }
                Ok(())
            })
            .clone()
    }
    include!(concat!(env!("OUT_DIR"), "/legacy_food_compat.rs"));
}

mod development {
    include!(concat!(env!("OUT_DIR"), "/development_shared.rs"));
    pub fn now() -> Result<Instant, DomainError> {
        crate::CLOCK
            .with(|v| v.borrow().clone())
            .ok_or_else(|| crate::error(ErrorCode::MissingField, "/now"))
    }
}
mod records {
    pub fn storage(_: impl std::fmt::Display) -> mstrchf_contracts::DomainError {
        crate::error(
            mstrchf_contracts::ErrorCode::InvalidValue,
            "/browser/domain",
        )
    }
}
mod planner_state {
    use super::*;
    #[derive(Serialize)]
    pub struct LibraryEntry {
        pub recipe: Option<RecipeRevision>,
        pub candidate: Option<RecipeCandidate>,
        pub reference: RecipeRef,
        pub admission: String,
        pub notices: Vec<String>,
    }
    #[derive(Serialize)]
    pub struct PlannerDetails {
        pub library: Vec<LibraryEntry>,
        pub history: PlanningHistory,
        pub derived: Option<DerivedPlan>,
        pub essentials: Vec<EssentialsConfirmation>,
        pub food_error: Option<ErrorCode>,
        pub catalogue_revision: Revision,
        pub reviewed_sources: Vec<ReviewedSource>,
        pub shopping_list: Option<crate::shopping_list::ShoppingListProjection>,
        pub test_kitchen: bool,
    }
    #[derive(Serialize)]
    pub struct ReviewedSource {
        pub packet: ReviewedRecipePacketRef,
        pub recipe: RecipeRevision,
        pub scope: Vec<String>,
        pub admitted: bool,
    }
}
thread_local! {static CLOCK:RefCell<Option<Instant>>=const {RefCell::new(None)};}
thread_local! {static AUDIT_CACHE:RefCell<Option<(Digest,Instant)>>=const {RefCell::new(None)};}
#[derive(Clone, Default)]
struct ReceiptFoodProof {
    selections: Vec<Vec<u8>>,
    links: Vec<Vec<u8>>,
    demands: Vec<DemandSet>,
}
thread_local! {static RECEIPT_FOOD_CACHE:RefCell<std::collections::VecDeque<(Digest,ReceiptFoodProof)>>=const {RefCell::new(std::collections::VecDeque::new())};}
thread_local! {static ORIGINAL_WEEK_FOOD_CACHE:RefCell<std::collections::VecDeque<(Digest,ReceiptFoodProof)>>=const {RefCell::new(std::collections::VecDeque::new())};}
thread_local! {static CONFLICT_CACHE:RefCell<Option<(Digest,Vec<Conflict>)>>=const {RefCell::new(None)};}
fn error(code: ErrorCode, path: &str) -> DomainError {
    DomainError::new(code, path)
}

fn runtime_catalogue(state: &BrowserState) -> Result<catalogue::RuntimeCatalogue, DomainError> {
    if let Some(pin)=&state.public_catalogue_pin {
        if !state.reviewed_packets.is_empty(){return Err(error(ErrorCode::AuthorityDenied,"/public_beta/legacy_material"));}
        let view=catalogue::RuntimeCatalogue::public_starters()?;
        view.verify_pin(pin)?;
        return Ok(view);
    }
    catalogue::RuntimeCatalogue::from_records(&state.reviewed_packets.0)
}
fn no_reviewed_packets(records: &BoundedVec<maker_packets::ReviewedPacketRecord, 0, 512>) -> bool {
    records.is_empty()
}
fn empty_reviewed_packets() -> BoundedVec<maker_packets::ReviewedPacketRecord, 0, 512> {
    BoundedVec(vec![])
}

#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BrowserState {
    pub format_version: SchemaVersion,
    pub snapshot: Snapshot,
    pub retained_weeks: BoundedVec<Week, 0, 4096>,
    pub history: PlanningHistory,
    pub personal_recipes: BoundedVec<RecipeCandidate, 0, 512>,
    // Exact immutable public maker packet bodies are retained in every backup.
    // Only the compiled maker registry can turn a matching body into evidence.
    #[serde(default = "empty_reviewed_packets", skip_serializing_if = "no_reviewed_packets")]
    pub reviewed_packets: BoundedVec<maker_packets::ReviewedPacketRecord, 0, 512>,
    pub essentials: BoundedVec<EssentialsConfirmation, 0, 4096>,
    pub receipts: BoundedVec<ReceiptRecord, 0, 4096>,
    pub pending_proposal: Option<PendingProposal>,
    pub engineering_demo: bool,
    /// Present only for a newly requested public manual kitchen. Existing
    /// kitchens and all their catalogue/receipt bytes retain the old baseline.
    #[serde(default,skip_serializing_if="Option::is_none")]
    pub public_catalogue_pin: Option<catalogue::CataloguePin>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReceiptRecord {
    pub command: AppCommandV1,
    pub command_digest: Digest,
    pub response_digest: Digest,
    pub snapshot_revision: Revision,
    pub response: Snapshot,
    pub committed_at: Instant,
    // A narrowly optional additive journal extension. Legacy receipts retain
    // every original required field and have no recoverable AI correlation.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_proposal: Option<AcceptedProposal>,
    // Absence explicitly means the original frozen engineering-v1 catalogue.
    // This additive journal field never changes command or Snapshot digests.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub food_catalogue_pin: Option<catalogue::CataloguePin>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub prepared_calendar: Option<calendar_owner::CalendarJournal>,
    // Exact input custody for this additive command only. Legacy sparse
    // pantry ledgers are valid: omission must not become a new stock choice.
    // The audit proves every input against its historical prefix, then derives
    // the output afresh; this is never a claimed transfer or approval marker.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub privacy_essentials_before: Option<BoundedVec<EssentialsConfirmation, 0, 4096>>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct AcceptedProposal {
    pub result: ProposalResult,
    pub result_digest: Digest,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PendingProposal {
    pub proposal_id: Text,
    pub context_digest: Digest,
    pub context: PlanningContext,
    pub mode: PlanMode,
    pub slots: BoundedVec<SlotId, 1, 21>,
    pub allow_repeats: bool,
    pub choices: BoundedVec<ScopedChoices, 1, 21>,
    pub returned: Option<PlanProposal>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ScopedChoices {
    pub slot: SlotId,
    pub choices: BoundedVec<MealSelection, 0, 512>,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct BoardOptions {
    pub overrides: Option<MealOverrides>,
    pub extra_recipients: Option<BoundedVec<PersonRef, 0, 64>>,
    pub make_extra: Option<bool>,
    pub leftover_source: Option<SlotId>,
    pub eat_by_time: Option<LocalTime>,
    pub time_resolution: Option<Resolution>,
    pub portable_cold: Option<PortableColdOptions>,
}
#[derive(Clone, Debug, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct PortableColdOptions {
    pub packing_complete_time: LocalTime,
    pub packing_resolution: Resolution,
    pub depart_time: LocalTime,
    pub departure_resolution: Resolution,
    pub chilling_confirmed: bool,
}
#[derive(Serialize, Deserialize, JsonSchema)]
#[serde(tag = "op", rename_all = "snake_case", deny_unknown_fields)]
enum Request {
    PublicStarterCatalogue {},
    Initialize {
        household_id: HouseholdId,
        diner_id: DinerId,
        zone: ZoneId,
        today: LocalDate,
        now: Instant,
        demo: Option<bool>,
        public_beta: Option<bool>,
    },
    Snapshot {
        state: BrowserState,
        now: Option<Instant>,
    },
    Validate {
        state: BrowserState,
        now: Instant,
    },
    ValidateState {
        state: BrowserState,
        now: Instant,
    },
    PendingProposal {
        state: BrowserState,
        now: Instant,
    },
    RevokePendingProposal {
        state: BrowserState,
        now: Instant,
    },
    Conflicts {
        state: BrowserState,
        offset: u32,
        limit: u16,
        now: Instant,
    },
    Board {
        state: BrowserState,
        week_id: Option<WeekId>,
        slot: Option<SlotId>,
        options: Option<BoardOptions>,
        now: Instant,
    },
    Dispatch {
        state: BrowserState,
        command: AppCommandV1,
        command_json: Option<String>,
        now: Instant,
        today: Option<LocalDate>,
    },
    Lookup {
        state: BrowserState,
        command: AppCommandV1,
        command_json: Option<String>,
        now: Option<Instant>,
    },
    PrepareProposal {
        state: BrowserState,
        week_id: WeekId,
        slots: BoundedVec<SlotId, 1, 21>,
        mode: PlanMode,
        allow_repeats: bool,
        nonce: Option<Text>,
        now: Instant,
    },
    ReturnProposal {
        state: BrowserState,
        result: ProposalResult,
        now: Instant,
    },
    ResolveMoment {
        date: LocalDate,
        time: LocalTime,
        zone: ZoneId,
        resolution: Resolution,
    },
    CalendarPreview {
        state: BrowserState,
        order_id: OrderId,
        destination_id: CalendarDestinationId,
        now: Instant,
    },
    ProductMatchContext {
        state: BrowserState,
        week_id: WeekId,
        now: Instant,
    },
    ProductSourcePrepare {
        state: BrowserState, week_id: WeekId, reader: ReadActor, key: BasketKey,
        delivery: RetailerDeliveryContext, targets: BoundedVec<product_source_reads::ProductSourceTarget,1,8>,
        job_id: JobId, read_id: ReadId, now: Instant, deadline: Instant,
    },
    ProductSourceInspect {state: BrowserState, prepared: product_source_reads::PreparedProductSourceRead, now: Instant},
    ProductSourceCheckReturn {state: BrowserState, prepared: product_source_reads::PreparedProductSourceRead, observation: ReadObservationV1, now: Instant},
    ProductSourceAccept {state: BrowserState, prepared: product_source_reads::PreparedProductSourceRead, observation: ReadObservationV1, evidence_id: EvidenceId, now: Instant},
    ProductSourceOperatorAccept {state: BrowserState, prepared: product_source_reads::PreparedProductSourceRead, observation: ReadObservationV1, evidence_id: EvidenceId, operator_task_ref: Text, now: Instant},
    ProductSourceAudit {prepared: product_source_reads::PreparedProductSourceRead, observation: Nullable<ReadObservationV1>, accepted: Nullable<AcceptedReadRef>},
    BasketAdditionPrepare {state:BrowserState,week_id:WeekId,reader:ReadActor,key:BasketKey,delivery:RetailerDeliveryContext,
        target:product_source_reads::ProductSourceTarget,line_id:DemandLineId,job_id:JobId,read_id:ReadId,review_id:Text,now:Instant,deadline:Instant},
    BasketAdditionCheckSource {state:BrowserState,prepared:basket_addition_review::PreparedBasketAdditionSource,observation:ReadObservationV1,now:Instant},
    BasketAdditionCheckSemantic {state:BrowserState,prepared:basket_addition_review::PreparedBasketAdditionReview,returned:product_semantic_review::PublicSemanticReturn,now:Instant},
    BasketAdditionOperatorReview {state:BrowserState,prepared:basket_addition_review::PreparedBasketAdditionReview,
        returned:product_semantic_review::PublicSemanticReturn,reviewed_at:Instant,decision:basket_addition_review::BasketAdditionOperatorDecision,now:Instant},
    BasketAdditionPrepareV3 {state:BrowserState,week_id:WeekId,reader:ReadActor,key:BasketKey,delivery:RetailerDeliveryContext,
        target:product_source_reads::ProductSourceTarget,line_id:DemandLineId,job_id:JobId,read_id:ReadId,review_id:Text,now:Instant,deadline:Instant},
    BasketAdditionCheckSourceV3 {state:BrowserState,prepared:basket_addition_review::PreparedBasketAdditionSource,observation:ReadObservationV1,now:Instant},
    BasketAdditionCheckSemanticV3 {state:BrowserState,prepared:basket_addition_review::PreparedBasketAdditionReview,returned:product_semantic_review::PublicSemanticReturn,now:Instant},
    BasketAdditionOperatorReviewV3 {state:BrowserState,prepared:basket_addition_review::PreparedBasketAdditionReview,
        returned:product_semantic_review::PublicSemanticReturn,reviewed_at:Instant,decision:basket_addition_review::BasketAdditionOperatorDecision,now:Instant},
    BasketSourcePrepare {state: BrowserState, week_id: WeekId, reader: ReadActor, key: BasketKey,
        delivery: RetailerDeliveryContext, job_id: JobId, read_id: ReadId, now: Instant, deadline: Instant},
    BasketSourceInspect {state: BrowserState, prepared: basket_source_reads::PreparedBasketSourceRead, now: Instant},
    BasketSourceCheckReturn {state: BrowserState, prepared: basket_source_reads::PreparedBasketSourceRead,
        returned: basket_source_reads::PublicBasketSourceReturn, observation_id: ReadObservationId, now: Instant},
    BasketSourceAccept {state: BrowserState, prepared: basket_source_reads::PreparedBasketSourceRead,
        returned: basket_source_reads::PublicBasketSourceReturn, observation: ReadObservationV1, evidence_id: EvidenceId, now: Instant},
    BasketSourceLocalInspect {state: BrowserState, prepared: basket_source_reads::PreparedBasketSourceRead, now: Instant},
    BasketSourceLocalCheckReturn {state: BrowserState, prepared: basket_source_reads::PreparedBasketSourceRead,
        returned: basket_source_reads::PublicBasketSourceReturn, observation_id: ReadObservationId, now: Instant},
    BasketSourceLocalAccept {state: BrowserState, prepared: basket_source_reads::PreparedBasketSourceRead,
        returned: basket_source_reads::PublicBasketSourceReturn, observation: ReadObservationV1, evidence_id: EvidenceId, now: Instant},
    BasketSourceAudit {prepared: basket_source_reads::PreparedBasketSourceRead,
        returned: Nullable<basket_source_reads::PublicBasketSourceReturn>, observation: Nullable<ReadObservationV1>, accepted: Nullable<AcceptedReadRef>},
    ProductMeaningReviewPrepare {state: BrowserState, week_id: WeekId,
        prepared: product_source_reads::PreparedProductSourceRead, observation: ReadObservationV1,
        accepted: AcceptedReadRef, product: ProductRef, line_id: DemandLineId, now: Instant},
    ProductMeaningReviewPrepareV3 {state: BrowserState, week_id: WeekId,
        prepared: product_source_reads::PreparedProductSourceRead, observation: ReadObservationV1,
        accepted: AcceptedReadRef, product: ProductRef, line_id: DemandLineId, now: Instant},
    ProductSemanticPrepare {state: BrowserState, week_id: WeekId,
        source_prepared: product_source_reads::PreparedProductSourceRead, observation: ReadObservationV1,
        accepted: AcceptedReadRef, product: ProductRef, line_id: DemandLineId, request_id: Text, now: Instant, deadline: Instant},
    ProductSemanticPrepareV3 {state: BrowserState, week_id: WeekId,
        source_prepared: product_source_reads::PreparedProductSourceRead, observation: ReadObservationV1,
        accepted: AcceptedReadRef, product: ProductRef, line_id: DemandLineId, request_id: Text, now: Instant, deadline: Instant},
    ProductSemanticInspect {state: BrowserState, prepared: product_semantic_review::PreparedProductSemanticReview, now: Instant},
    ProductSemanticCheckReturn {state: BrowserState, prepared: product_semantic_review::PreparedProductSemanticReview,
        returned: product_semantic_review::PublicSemanticReturn, now: Instant},
    ProductSemanticEvaluate {state: BrowserState, prepared: product_semantic_review::PreparedProductSemanticReview,
        returned: product_semantic_review::PublicSemanticReturn, evidence_id: EvidenceId, now: Instant},
    ProductSemanticAudit {prepared: product_semantic_review::PreparedProductSemanticReview,
        returned: Nullable<product_semantic_review::PublicSemanticReturn>, recorded: Nullable<product_semantic_review::RecordedProductSemanticReview>},
    ProductMeaningQualify {state: BrowserState, prepared: product_semantic_review::PreparedProductSemanticReview,
        recorded: product_semantic_review::RecordedProductSemanticReview, now: Instant},
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ProposalResult {
    pub proposal_id: Text,
    pub context_digest: Digest,
    pub selections: BoundedVec<ReturnedChoice, 1, 21>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize, JsonSchema)]
#[serde(deny_unknown_fields)]
pub struct ReturnedChoice {
    pub slot: SlotId,
    pub recipe: RecipeRef,
}

fn empty_history() -> PlanningHistory {
    PlanningHistory {
        revision: Revision(1),
        selected: BoundedVec(vec![]),
        ordered: BoundedVec(vec![]),
        feedback: BoundedVec(vec![]),
        hidden_recipe_ids: BoundedVec(vec![]),
    }
}
fn initial(id: HouseholdId) -> Snapshot {
    Snapshot {
        schema_version: SchemaVersion,
        snapshot_revision: Revision(1),
        privacy: PrivacyPolicy {
            revision: Revision(1),
            use_ai_features: false,
            include_household_requirements: false,
            confirmed_at: Nullable(None),
        },
        household_draft: HouseholdDraft {
            schema_version: SchemaVersion,
            household_id: id,
            revision: Revision(1),
            completed_stages: BoundedVec(vec![]),
            unresolved_restrictions: BoundedVec(vec![]),
            diners: BoundedVec(vec![]),
            preferences: Nullable(None),
            usual_week: Nullable(None),
            connections: Nullable(None),
        },
        profile: Nullable(None),
        weeks: BoundedVec(vec![]),
        jobs: BoundedVec(vec![]),
        conflicts: BoundedVec(vec![]),
    }
}
fn demo_profile(
    s: &Snapshot,
    diner_id: &DinerId,
    zone: &ZoneId,
) -> Result<HouseholdProfile, DomainError> {
    let mut p: HouseholdProfile = parse_document(include_bytes!(
        "../../../tests/fixtures/households/p03ab/a1a2-profile.json"
    ))?;
    p.household_id = s.household_draft.household_id.clone();
    p.revision = Revision(1);
    p.diners.0.truncate(1);
    p.diners[0].diner_id = diner_id.clone();
    p.diners[0].display_name = Nullable(Some(NameText("Synthetic demo diner".into())));
    p.diners[0].restrictions = RestrictionDeclaration::Confirmed {
        confirmed_at: Instant("2026-01-01T12:00:00Z".into()),
        requirements: BoundedVec(vec![HardRequirement::Diet {
            code: DietCode::Vegetarian,
        }]),
    };
    p.usual_week.zone = zone.clone();
    p.usual_week.suggest_leftovers = false;
    p.preferences.mobility.temperature = MobilityTemperature::Hot;
    for cell in &mut p.usual_week.cells.0 {
        cell.attendee_ids = BoundedVec(vec![diner_id.clone()]);
    }
    p.connections.privacy = s.privacy.clone();
    p.connections.harness = Nullable(None);
    p.connections.retailer = Nullable(None);
    p.connections.calendar_destination = Nullable(None);
    Ok(p)
}
fn open_horizon(
    state: &mut BrowserState,
    at: &Instant,
    today: Option<&LocalDate>,
) -> Result<(), DomainError> {
    let p = state
        .snapshot
        .profile
        .0
        .as_ref()
        .ok_or_else(|| error(ErrorCode::MissingField, "/profile"))?;
    let derived = mstrchf_core::weeks::local_planning_date(at, &p.usual_week.zone)?;
    if today.is_some_and(|v| v != &derived) {
        return Err(error(ErrorCode::IdentityMismatch, "/today"));
    }
    let temporary = [1, 2, 3, 4].map(|n| WeekId(format!("wk_{n:032x}")));
    let ranges = mstrchf_core::weeks::planning_horizon(&derived, &p.usual_week.zone, &temporary)?;
    let mut active = Vec::new();
    for mut range in ranges {
        if let Some(w) = state
            .retained_weeks
            .iter()
            .find(|v| v.range.monday == range.monday && v.range.zone == range.zone)
        {
            active.push(w.clone());
        } else {
            range.week_id = WeekId(planner_view::stable_id(
                "wk",
                &json!([p.household_id, range.monday, range.zone]),
            )?);
            let w = mstrchf_core::weeks::new_week(p, &state.snapshot.privacy, at, &range)?;
            state.retained_weeks.0.push(w.clone());
            active.push(w);
        }
    }
    if state.retained_weeks.len() > 4096 {
        return Err(error(ErrorCode::LimitExceeded, "/weeks/archive"));
    }
    state
        .retained_weeks
        .0
        .sort_by_key(|v| (v.range.monday.0.clone(), v.range.week_id.0.clone()));
    state.snapshot.weeks = BoundedVec(active);
    Ok(())
}
fn conflicts(state: &BrowserState) -> Result<Vec<Conflict>, DomainError> {
    let view = runtime_catalogue(state)?;
    // These are the complete inputs used by the compiled pure conflict projection;
    // no household/history/journal changes are hidden behind a revision-only key.
    let key = canonical_digest(&json!([
        state.snapshot.profile,
        state.snapshot.privacy,
        state.retained_weeks,
        view.pin(),
        CLOCK.with(|v| v.borrow().clone())
    ]))?;
    if let Some(hit) = CONFLICT_CACHE.with(|v| {
        v.borrow()
            .as_ref()
            .filter(|(saved, _)| saved == &key)
            .map(|(_, proof)| proof.clone())
    }) {
        return Ok(hit);
    }
    let out = conflicts_uncached(state, &view)?;
    CONFLICT_CACHE.with(|v| *v.borrow_mut() = Some((key, out.clone())));
    Ok(out)
}
fn conflicts_uncached(state: &BrowserState, view: &catalogue::RuntimeCatalogue) -> Result<Vec<Conflict>, DomainError> {
    let Some(p) = &state.snapshot.profile.0 else {
        return Ok(vec![]);
    };
    let mut out = Vec::new();
    for w in &state.retained_weeks.0 {
        // Retained declarations include disabled cells and archived weeks.
        for c in planner_view::retained_conflicts(&view, p, w)? {
            if !out.contains(&c) {
                out.push(c);
            }
        }
        if let Err(e) = planner_view::derive(&view, &state.snapshot, p, w) {
            let active = mstrchf_core::availability::eligible_slots(w)?;
            let selections = w
                .plan
                .selections
                .iter()
                .filter(|v| active.contains(&v.slot))
                .collect::<Vec<_>>();
            let mut refs = selections
                .iter()
                .map(|v| v.recipe.clone())
                .collect::<Vec<_>>();
            refs.sort_by_key(|v| canonical_bytes(v).unwrap_or_default());
            refs.dedup();
            let c = Conflict {
                code: e.code,
                week_ids: BoundedVec(vec![w.range.week_id.clone()]),
                slot_ids: BoundedVec(selections.iter().map(|v| v.slot.clone()).collect()),
                recipe_refs: BoundedVec(refs),
                evidence_ids: BoundedVec(vec![]),
                blocking: true,
            };
            if !out.contains(&c) {
                out.push(c);
            }
        }
    }
    out.sort_by_key(|v| canonical_bytes(v).unwrap_or_default());
    Ok(out)
}
fn refresh(state: &mut BrowserState) -> Result<(), DomainError> {
    state.snapshot.conflicts = BoundedVec(conflicts(state)?.into_iter().take(100).collect());
    Ok(())
}

fn audit(state: &BrowserState, at: Option<&Instant>) -> Result<(), DomainError> {
    // Cache only a successfully audited complete immutable state. Wire admission
    // still runs on every request. Current library is entirely compiled engineering
    // material: no expiring product, provider, job or production-admission evidence
    // can change without a state change. Backwards clocks always revalidate.
    let digest = canonical_digest(state)?;
    if let Some(at) = at {
        if AUDIT_CACHE.with(|v| {
            v.borrow()
                .as_ref()
                .is_some_and(|(saved, checked)| saved == &digest && checked <= at)
        }) {
            return Ok(());
        }
    }
    audit_uncached(state, at)?;
    if let Some(at) = at {
        AUDIT_CACHE.with(|v| *v.borrow_mut() = Some((digest, at.clone())));
    }
    Ok(())
}
fn audit_uncached(state: &BrowserState, at: Option<&Instant>) -> Result<(), DomainError> {
    let view = runtime_catalogue(state)?;
    let s = &state.snapshot;
    if !s.jobs.is_empty() {
        return Err(error(ErrorCode::UnsupportedCapability, "/jobs"));
    }
    if let Some(at) = at {
        mstrchf_core::household::validate_draft_progress(&s.household_draft, &s.privacy, at)?;
        if let Some(p) = &s.profile.0 {
            mstrchf_core::household::validate_profile(p, &s.privacy, at)?;
        }
    }
    let mut ids = BTreeSet::new();
    let mut ranges = BTreeSet::new();
    for w in &state.retained_weeks.0 {
        if !ids.insert(&w.range.week_id) {
            return Err(error(ErrorCode::DuplicateKey, "/retained_weeks"));
        }
        if !ranges.insert((&w.range.monday, &w.range.zone)) {
            return Err(error(ErrorCode::DuplicateKey, "/retained_weeks/ranges"));
        }
        for selected in &w.plan.selections.0 {
            if !view.recipes()
                .iter()
                .any(|r| development::recipe_reference(r).as_ref() == Ok(&selected.recipe))
            {
                return Err(error(ErrorCode::ForeignReference, "/plan/recipe"));
            }
        }
    }
    for w in &s.weeks.0 {
        if !state.retained_weeks.contains(w) {
            return Err(error(ErrorCode::IdentityMismatch, "/weeks"));
        }
    }
    if s.profile.0.is_none() && !state.retained_weeks.is_empty() {
        return Err(error(ErrorCode::MissingField, "/profile"));
    }
    let mut expected = Vec::new();
    for w in &state.retained_weeks.0 {
        let active = mstrchf_core::availability::eligible_slots(w)?;
        expected.extend(
            w.plan
                .selections
                .iter()
                .filter(|v| active.contains(&v.slot)),
        );
    }
    if expected.len() != state.history.selected.len()
        || state.history.selected.iter().any(|v| {
            !expected.iter().any(|s| {
                s.slot == v.slot
                    && s.recipe == v.recipe
                    && s.servings == v.servings
                    && s.extras == v.extras
                    && s.repeat_authorised == v.repeat_authorised
            })
        })
    {
        return Err(error(ErrorCode::IdentityMismatch, "/history/selected"));
    }
    let mut receipts = BTreeSet::new();
    let mut accepted_ids = BTreeSet::new();
    let mut admitted_selections = BTreeSet::new();
    let mut admitted_links = BTreeSet::new();
    let mut demands = Vec::new();
    let mut essentials_proofs = BTreeSet::new();
    let mut current_essentials = std::collections::BTreeMap::new();
    let mut strict_essentials = false;
    let mut reconstructed = empty_history();
    let mut recorded_weeks = std::collections::BTreeMap::new();
    let mut chronological_packets = Vec::new();
    let mut chronological_view = if state.public_catalogue_pin.is_some(){catalogue::RuntimeCatalogue::public_starters()?}else{catalogue::RuntimeCatalogue::legacy()?};
    let mut previous_response = initial(s.household_draft.household_id.clone());
    if s.snapshot_revision.0 != state.receipts.len() as u64 + 1 {
        return Err(error(
            ErrorCode::IdentityMismatch,
            "/snapshot_revision/receipts",
        ));
    }
    for (index, r) in state.receipts.iter().enumerate() {
        if !receipts.insert(&r.command.command_id)
            || canonical_digest(&r.command)? != r.command_digest
            || r.command.expected_snapshot_revision.checked_increment()? != r.snapshot_revision
            || r.snapshot_revision > s.snapshot_revision
            || r.response.snapshot_revision != r.snapshot_revision
            || r.response.household_draft.household_id != s.household_draft.household_id
            || canonical_digest(&r.response)? != r.response_digest
            || at.is_some_and(|at| &r.committed_at > at)
            || r.snapshot_revision.0 != index as u64 + 2
        {
            return Err(error(ErrorCode::DigestMismatch, "/receipts"));
        }
        if let AppCommand::AdmitReviewedRecipe { packet, expected_catalogue_revision } = &r.command.command {
            if index == 0 || previous_response.profile.0.is_none() {
                return Err(error(ErrorCode::MissingReference, "/receipts/admission_saved_kitchen"));
            }
            if *expected_catalogue_revision != chronological_view.pin().revision {
                return Err(error(ErrorCode::ContextStale, "/receipts/catalogue_revision"));
            }
            let known = maker_packets::known_packet(packet)?;
            if &r.committed_at < known.not_before() {
                return Err(error(ErrorCode::AuthorityDenied, "/receipts/admission_before_review"));
            }
            if chronological_packets.contains(known.record()) {
                return Err(error(ErrorCode::DuplicateKey, "/receipts/admitted_packet"));
            }
            // Every immutable declared extra must already have its own
            // independent admission in this exact historical prefix.
            chronological_view.require_admission_dependencies(known)?;
            chronological_packets.push(known.record().clone());
            chronological_view = catalogue::RuntimeCatalogue::from_records(&chronological_packets)?;
            // Admission changes availability of a known recipe, never old food,
            // profile, privacy or order state. Re-signed imported snapshots do
            // not get permission to smuggle an unrelated effect into this intent.
            let mut expected = previous_response.clone();
            expected.snapshot_revision = r.snapshot_revision;
            if expected != r.response {
                return Err(error(ErrorCode::IdentityMismatch, "/receipts/admission_response"));
            }
        }
        receipt_catalogue(r, &chronological_view)?;
        if let AppCommand::SaveWeek {week,expected_week_revision}=&r.command.command {
            if let (Some(old),Some(profile))=(recorded_weeks.get(&week.week_id),previous_response.profile.0.as_ref()) {
                if planner_view::needs_two_unit_retention(&chronological_view,old,week)? {
                    let (effect,_)=planner_view::prepare_preserving_week_edit(&chronological_view,old,week,*expected_week_revision,profile,&r.committed_at)?;
                    // A re-signed backup may retain only the exact audited old
                    // allocations through this route. Reconstruct its actual
                    // edit, never infer permission from later response bytes.
                    let mut expected=effect.candidate;
                    if let Some(plan)=&week.plan.0 {
                        if plan.revision!=old.plan.revision {return Err(error(ErrorCode::ContextStale,"/receipts/retained_two_unit/plan_revision"));}
                        expected.plan=plan.clone();
                    }
                    expected.revision=old.revision.checked_increment()?;
                    expected.plan.revision=old.plan.revision.checked_increment()?;
                    expected.approved_plan=Nullable(None);
                    expected.confirmed_order=old.confirmed_order.clone();
                    expected.publication=if old.confirmed_order.0.is_some(){PublicationState::NeedsUser}else{old.publication};
                    let actual=r.response.weeks.iter().find(|next|next.range.week_id==week.week_id)
                        .ok_or_else(||error(ErrorCode::MissingReference,"/receipts/retained_two_unit/week"))?;
                    if actual!=&expected {return Err(error(ErrorCode::IdentityMismatch,"/receipts/retained_two_unit/response"));}
                }
            }
        }
        if let AppCommand::CorrectOrderDelivery { order_id, delivery_start, delivery_end } = &r.command.command {
            let week_id = booking::correction_week(&previous_response, &state.receipts.0[..index], order_id, delivery_start, delivery_end, &r.committed_at)?;
            let mut expected = previous_response.clone();
            let week = expected.weeks.iter_mut().find(|week|week.range.week_id==week_id).unwrap();
            week.publication = booking::corrected_publication(week.publication);
            expected.snapshot_revision = r.snapshot_revision;
            if expected != r.response { return Err(error(ErrorCode::IdentityMismatch, "/receipts/delivery_correction_response")); }
        }
        if let AppCommand::ConfirmCalendarPublication { confirmation } = &r.command.command {
            // Validate against the original owner prefix and renderer. Later
            // renderer changes cannot rewrite this reviewed observation.
            let mut prior = state.clone();
            prior.snapshot = previous_response.clone();
            prior.receipts = BoundedVec(state.receipts.iter().take(index).cloned().collect());
            prior.retained_weeks = BoundedVec(recorded_weeks.values().cloned().collect());
            prior.reviewed_packets = BoundedVec(chronological_packets.clone());
            prior.essentials = BoundedVec(current_essentials.values().cloned().collect());
            prior.history = reconstructed.clone();
            prior.pending_proposal = None;
            let week_id = calendar_publication::confirm(&prior, confirmation, &r.committed_at)?;
            let mut expected = previous_response.clone();
            expected.weeks.iter_mut().find(|week|week.range.week_id==week_id).unwrap().publication = PublicationState::Published;
            expected.snapshot_revision = r.snapshot_revision;
            if expected != r.response { return Err(error(ErrorCode::IdentityMismatch, "/receipts/calendar_publication_response")); }
        }
        booking::check_transition(&chronological_view, &previous_response, &recorded_weeks, &r.response, &r.command.command, &r.committed_at)?;
        if matches!(r.command.command, AppCommand::SetPrivacyPreservingEssentials { .. }) {
            // Recompute the transfer from the exact historical owner prefix.
            // Neither a re-signed final digest nor later catalogue admissions
            // can authorise a changed quantity, choice or confirmation time.
            let seed = r.privacy_essentials_before.as_ref()
                .ok_or_else(|| error(ErrorCode::MissingField, "/receipts/privacy_essentials_before"))?;
            if seed.windows(2).any(|pair| pair[0].line_id >= pair[1].line_id)
                || seed.iter().any(|entry| entry.confirmed_at > r.committed_at)
            {
                return Err(error(ErrorCode::InvalidValue, "/receipts/privacy_essentials_before"));
            }
            if strict_essentials {
                if seed.0 != current_essentials.values().cloned().collect::<Vec<_>>() {
                    return Err(error(ErrorCode::IdentityMismatch, "/receipts/privacy_essentials_before"));
                }
            } else {
                // Apply the frozen legacy admission rule to the recorded exact
                // input. Missing or older entries stay missing or older; the
                // receipt never fills them from the full historical proof set.
                for entry in seed.iter() {
                    if !essentials_proofs.contains(&canonical_bytes(entry)?)
                        || !demands.iter().any(|d| validate_essentials_binding(entry, d).is_ok())
                    {
                        return Err(error(ErrorCode::DigestMismatch, "/receipts/privacy_essentials_before"));
                    }
                }
            }
            let mut prior = state.clone();
            prior.snapshot = previous_response.clone();
            prior.receipts = BoundedVec(state.receipts.iter().take(index).cloned().collect());
            prior.retained_weeks = BoundedVec(recorded_weeks.values().cloned().collect());
            prior.reviewed_packets = BoundedVec(chronological_packets.clone());
            prior.essentials = seed.clone();
            prior.history = reconstructed.clone();
            prior.pending_proposal = None;
            let old_clock = CLOCK.with(|clock| clock.replace(Some(r.committed_at.clone())));
            let checked = mutate(&mut prior, &r.command.command, &r.committed_at, None);
            CLOCK.with(|clock| *clock.borrow_mut() = old_clock);
            checked?;
            prior.snapshot.snapshot_revision = r.snapshot_revision;
            if prior.snapshot != r.response {
                return Err(error(ErrorCode::IdentityMismatch, "/receipts/privacy_essentials_response"));
            }
            current_essentials.clear();
            for confirmation in prior.essentials.0 {
                essentials_proofs.insert(canonical_bytes(&confirmation)?);
                current_essentials.insert(confirmation.line_id.clone(), confirmation);
            }
            strict_essentials = true;
        } else if r.privacy_essentials_before.is_some() {
            return Err(error(ErrorCode::IdentityMismatch, "/receipts/privacy_essentials_before_command"));
        }
        if matches!(r.command.command, AppCommand::SuggestPlanningHorizon { .. }) {
            // Reconstruct this local multi-week intent from its exact historical
            // prefix. A re-signed backup cannot use the four-week scope to alter
            // fixed meals, declarations, history, or the original recipe set.
            let mut prior = state.clone();
            prior.snapshot = previous_response.clone();
            prior.receipts = BoundedVec(state.receipts.iter().take(index).cloned().collect());
            prior.retained_weeks = BoundedVec(recorded_weeks.values().cloned().collect());
            prior.reviewed_packets = BoundedVec(chronological_packets.clone());
            prior.essentials = BoundedVec(current_essentials.values().cloned().collect());
            prior.history = reconstructed.clone();
            prior.history.selected.0.sort_by_key(|v| (v.slot.week_id.0.clone(), v.slot.date.0.clone(), canonical_bytes(&v.slot.meal_type).unwrap_or_default()));
            prior.pending_proposal = None;
            let old_clock = CLOCK.with(|clock| clock.replace(Some(r.committed_at.clone())));
            let checked = mutate(&mut prior, &r.command.command, &r.committed_at, None);
            CLOCK.with(|clock| *clock.borrow_mut() = old_clock);
            checked?;
            prior.snapshot.snapshot_revision = r.snapshot_revision;
            if prior.snapshot != r.response {
                return Err(error(ErrorCode::IdentityMismatch, "/receipts/suggest_horizon_response"));
            }
        }
        if let AppCommand::RequestCalendarPublication { order_id, destination_id, event_batch_digest } = &r.command.command {
            // Reconstruct the exact pre-command owner prefix. Later settings,
            // catalogue, stock and exports cannot rewrite original custody.
            let mut prior = state.clone();
            prior.snapshot = previous_response.clone();
            prior.receipts = BoundedVec(state.receipts.iter().take(index).cloned().collect());
            prior.retained_weeks = BoundedVec(recorded_weeks.values().cloned().collect());
            prior.reviewed_packets = BoundedVec(chronological_packets.clone());
            prior.essentials = BoundedVec(current_essentials.values().cloned().collect());
            prior.history = reconstructed.clone();
            prior.pending_proposal = None;
            let renderer = r.prepared_calendar.as_ref().ok_or_else(|| error(ErrorCode::MissingField, "/receipts/prepared_calendar"))?.renderer_version;
            let (expected, _) = calendar_owner::prepare_for_renderer(&prior, order_id, destination_id, &r.committed_at, renderer)?;
            if canonical_digest(&expected.batch)? != *event_batch_digest || r.prepared_calendar.as_ref() != Some(&expected) {
                return Err(error(ErrorCode::IdentityMismatch, "/receipts/prepared_calendar"));
            }
        } else if r.prepared_calendar.is_some() {
            return Err(error(ErrorCode::IdentityMismatch, "/receipts/prepared_calendar_command"));
        }
        if let Some(accepted) = &r.accepted_proposal {
            let AppCommand::ApplyPlanProposal {
                proposal,
                context_stamp,
            } = &r.command.command
            else {
                return Err(error(
                    ErrorCode::IdentityMismatch,
                    "/receipts/accepted_proposal/command",
                ));
            };
            if !accepted_ids.insert(&accepted.result.proposal_id)
                || accepted.result.proposal_id.0.len() != 35
                || !accepted.result.proposal_id.0.starts_with("pr_")
                || !accepted.result.proposal_id.0[3..]
                    .bytes()
                    .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                || canonical_digest(&accepted.result)? != accepted.result_digest
                || canonical_digest(context_stamp)? != accepted.result.context_digest
                || accepted.result.selections.0 != public_selections(proposal)?
            {
                return Err(error(
                    ErrorCode::DigestMismatch,
                    "/receipts/accepted_proposal",
                ));
            }
        }
        let proof = receipt_food_proof(r, &chronological_view)?;
        let original_selections = proof.selections.clone();
        let original_links = proof.links.clone();
        demands.extend(proof.demands);
        admitted_selections.extend(proof.selections);
        admitted_links.extend(proof.links);
        let changed_weeks = match &r.command.command {
            AppCommand::SaveWeek { week, .. } => vec![week.week_id.clone()],
            AppCommand::SuggestMeals { week_id, .. } => vec![week_id.clone()],
            AppCommand::SuggestPlanningHorizon { ordered_week_ids, .. } => ordered_week_ids.0.clone(),
            AppCommand::ApplyPlanProposal { proposal, .. } => {
                proposal.changes.iter().map(|v| v.week_id.clone()).collect()
            }
            _ => vec![],
        };
        for next in &r.response.weeks.0 {
            if !changed_weeks.contains(&next.range.week_id) {
                let mut empty = next.plan.clone();
                empty.selections.0.clear();
                empty.batch_links.0.clear();
                let original = recorded_weeks.get(&next.range.week_id).map(|w: &Week| &w.plan).unwrap_or(&empty);
                if &next.plan != original {
                    return Err(error(ErrorCode::IdentityMismatch, "/receipts/unrequested_plan_change"));
                }
            }
        }
        for id in changed_weeks {
            let next = r
                .response
                .weeks
                .iter()
                .find(|v| v.range.week_id == id)
                .ok_or_else(|| error(ErrorCode::MissingReference, "/receipts/week"))?;
            let mut empty = next.clone();
            empty.plan.selections.0.clear();
            empty.plan.batch_links.0.clear();
            let before = recorded_weeks.get(&id).unwrap_or(&empty);
            for selected in next.plan.selections.iter().filter(|v| !before.plan.selections.contains(v)) {
                if !original_selections.contains(&canonical_bytes(selected)?) {
                    return Err(error(ErrorCode::IdentityMismatch, "/receipts/original_selection_admission"));
                }
            }
            for link in next.plan.batch_links.iter().filter(|v| !before.plan.batch_links.contains(v)) {
                if !original_links.contains(&canonical_bytes(link)?) {
                    return Err(error(ErrorCode::IdentityMismatch, "/receipts/original_batch_link_admission"));
                }
            }
            let delta = mstrchf_core::history::selected_index_delta(
                &[before.clone()],
                &[next.clone()],
                &r.committed_at,
            )?;
            if !delta.remove_slots.is_empty() || !delta.upsert.is_empty() {
                reconstructed.selected.0.retain(|v| {
                    !delta.remove_slots.contains(&v.slot)
                        && !delta.upsert.iter().any(|n| n.slot == v.slot)
                });
                reconstructed.selected.0.extend(delta.upsert);
                reconstructed.revision = reconstructed.revision.checked_increment()?;
            }
        }
        for w in &r.response.weeks.0 {
            recorded_weeks.insert(w.range.week_id.clone(), w.clone());
        }
        previous_response = r.response.clone();
        match &r.command.command {
            AppCommand::RecordFeedback { feedback } => {
                let mut f = feedback.clone();
                f.recorded_at = r.committed_at.clone();
                if f.hide
                    && !reconstructed
                        .hidden_recipe_ids
                        .contains(&f.recipe.recipe_id)
                {
                    reconstructed
                        .hidden_recipe_ids
                        .0
                        .push(f.recipe.recipe_id.clone());
                    reconstructed.hidden_recipe_ids.0.sort();
                }
                reconstructed.feedback.0.push(f);
                reconstructed.revision = reconstructed.revision.checked_increment()?;
            }
            AppCommand::RestoreHiddenRecipe { recipe_id } => {
                reconstructed.hidden_recipe_ids.0.retain(|v| v != recipe_id);
                reconstructed.revision = reconstructed.revision.checked_increment()?;
            }
            AppCommand::ConfirmEssentials { confirmations } => {
                for given in &confirmations.0 {
                    let mut given = given.clone();
                    given.confirmed_at = r.committed_at.clone();
                    essentials_proofs.insert(canonical_bytes(&given)?);
                    current_essentials.insert(given.line_id.clone(), given);
                }
            }
            AppCommand::ConfirmOrder { confirmation } => {
                let week = r.response.weeks.iter().find(|w| w.range.week_id == confirmation.week_id).ok_or_else(|| error(ErrorCode::MissingReference, "/receipts/order_week"))?;
                let order = week.confirmed_order.0.as_ref().ok_or_else(|| error(ErrorCode::MissingReference, "/receipts/order"))?;
                for selection in reconstructed.selected.iter().filter(|s| s.week_id == confirmation.week_id) {
                    reconstructed.ordered.0.push(OrderedHistory { selection: selection.clone(), order_id: order.order_id.clone(), ordered_at: r.committed_at.clone() });
                }
                reconstructed.revision = reconstructed.revision.checked_increment()?;
            }
            _ => {}
        }
    }
    if chronological_packets != state.reviewed_packets.0 || chronological_view.pin() != view.pin() {
        return Err(error(ErrorCode::IdentityMismatch, "/reviewed_packets/admission_journal"));
    }
    if let Some(last) = state.receipts.last() {
        if &last.response != s {
            return Err(error(ErrorCode::IdentityMismatch, "/snapshot/receipt_journal"));
        }
        if state.retained_weeks.len() != recorded_weeks.len() {
            return Err(error(ErrorCode::IdentityMismatch, "/retained_weeks/receipt_membership"));
        }
        for week in &state.retained_weeks.0 {
            if recorded_weeks.get(&week.range.week_id) != Some(week) {
                return Err(error(ErrorCode::IdentityMismatch, "/retained_weeks/receipt_journal"));
            }
        }
    }
    reconstructed.selected.0.sort_by_key(|v| {
        (
            v.slot.week_id.0.clone(),
            v.slot.date.0.clone(),
            canonical_bytes(&v.slot.meal_type).unwrap_or_default(),
        )
    });
    if reconstructed != state.history {
        return Err(error(ErrorCode::IdentityMismatch, "/history/journal"));
    }
    // Preserve legacy audit semantics for histories without the new command.
    // Once used, the final pantry must be the exact latest reconstructed ledger,
    // not any older admitted confirmation chosen from the historical proof set.
    if strict_essentials && current_essentials.values().cloned().collect::<Vec<_>>() != state.essentials.0 {
        return Err(error(ErrorCode::IdentityMismatch, "/essentials/privacy_journal"));
    }
    for v in &state.essentials.0 {
        if !essentials_proofs.contains(&canonical_bytes(v)?)
            || !demands
                .iter()
                .any(|d| validate_essentials_binding(v, d).is_ok())
        {
            return Err(error(ErrorCode::DigestMismatch, "/essentials/admission"));
        }
    }
    for w in &state.retained_weeks.0 {
        for v in &w.plan.selections.0 {
            if !admitted_selections.contains(&canonical_bytes(v)?) {
                return Err(error(
                    ErrorCode::IdentityMismatch,
                    "/plan/selection_admission",
                ));
            }
        }
        for v in &w.plan.batch_links.0 {
            if !admitted_links.contains(&canonical_bytes(v)?) {
                return Err(error(
                    ErrorCode::IdentityMismatch,
                    "/plan/batch_link_admission",
                ));
            }
        }
    }
    for r in &state.personal_recipes.0 {
        let checked = mstrchf_core::recipes::validate_recipe(r)?;
        if r.image.0.is_some() || r.unresolved != checked.capabilities.blocking_issues {
            return Err(error(ErrorCode::IdentityMismatch, "/personal_recipes"));
        }
    }
    if conflicts(state)?.into_iter().take(100).collect::<Vec<_>>() != s.conflicts.0 {
        return Err(error(ErrorCode::IdentityMismatch, "/conflicts"));
    }
    if let Some(pending) = &state.pending_proposal {
        if !s.privacy.use_ai_features {
            return Err(error(ErrorCode::PrivacyDenied, "/pending_proposal"));
        }
        let current = proposal_context(state, &pending.context.weeks[0].range.week_id)?;
        if current != pending.context || canonical_digest(&current.stamp)? != pending.context_digest
        {
            return Err(error(ErrorCode::ContextStale, "/pending_proposal/context"));
        }
        if pending.choices.len() != pending.slots.len() {
            return Err(error(
                ErrorCode::IdentityMismatch,
                "/pending_proposal/choices",
            ));
        }
        for slot in &pending.slots.0 {
            let given = pending
                .choices
                .iter()
                .find(|v| &v.slot == slot)
                .ok_or_else(|| error(ErrorCode::ScopeViolation, "/pending_proposal/choices"))?;
            let expected =
                proposal_choices(state, &current, slot, pending.mode, pending.allow_repeats)?
                    .into_iter()
                    .map(|v| v.selection)
                    .collect::<Vec<_>>();
            if given.choices.0 != expected {
                return Err(error(
                    ErrorCode::IdentityMismatch,
                    "/pending_proposal/choices",
                ));
            }
        }
        if let Some(proposal) = &pending.returned {
            let patch = mstrchf_core::selection::prepare_selection_patch_with_food(
                &current,
                &proposal_intent(pending),
                proposal,
                &view.reviewed()?,
                &[],
                &view.food_evidence(&current.stamp.recipe_refs.0)?,
            )
            .map_err(selection_error)?;
            for w in &patch.context.weeks.0 {
                planner_view::derive(&view, s, &current.household, w)?;
            }
        }
    }
    Ok(())
}

fn receipt_food_proof(r: &ReceiptRecord, view: &catalogue::RuntimeCatalogue) -> Result<ReceiptFoodProof, DomainError> {
    // Authenticate supplied pins before either food-proof cache can be read.
    receipt_catalogue(r, view)?;
    // Cache only immutable original food inputs/results. Receipt digest, exact
    // command binding, chronology and current-state audits remain outside it.
    let key = canonical_digest(&json!([r, view.pin()]))?;
    if let Some(hit) = RECEIPT_FOOD_CACHE.with(|v| {
        v.borrow()
            .iter()
            .find(|(saved, _)| saved == &key)
            .map(|(_, proof)| proof.clone())
    }) {
        return Ok(hit);
    }
    let mut proof = ReceiptFoodProof::default();
    if let Some(p) = &r.response.profile.0 {
        mstrchf_core::household::validate_profile(p, &r.response.privacy, &r.committed_at)?;
        for w in &r.response.weeks.0 {
            for selected in &w.plan.selections.0 {
                view.resolve_recipe(&selected.recipe)?;
                for extra in &selected.extras.0 { view.resolve_recipe(&extra.recipe)?; }
            }
            let week_proof =
                original_week_food_proof(&r.response, p, w, &r.committed_at, view)?;
            proof.demands.extend(week_proof.demands);
            proof.selections.extend(week_proof.selections);
            proof.links.extend(week_proof.links);
        }
    }
    RECEIPT_FOOD_CACHE.with(|v| {
        let mut v = v.borrow_mut();
        if v.len() >= 128 {
            v.pop_front();
        }
        v.push_back((key, proof.clone()));
    });
    Ok(proof)
}

fn receipt_catalogue(
    r: &ReceiptRecord,
    view: &catalogue::RuntimeCatalogue,
) -> Result<(), DomainError> {
    legacy_food::validate_premises()?;
    if let Some(pin) = &r.food_catalogue_pin {
        if pin.revision != view.pin().revision {
            return Err(error(ErrorCode::ContextStale, "/receipts/food_catalogue_pin/revision"));
        }
        if pin != view.pin() {
            return Err(error(ErrorCode::DigestMismatch, "/receipts/food_catalogue_pin"));
        }
    } else if view.pin() != catalogue::legacy_engineering_v1()?.pin() {
        return Err(error(ErrorCode::MissingField, "/receipts/food_catalogue_pin"));
    }
    Ok(())
}

fn original_week_food_proof(
    snapshot: &Snapshot,
    profile: &HouseholdProfile,
    week: &Week,
    committed_at: &Instant,
    view: &catalogue::RuntimeCatalogue,
) -> Result<ReceiptFoodProof, DomainError> {
    // Different receipts commonly contain an identical original week. The shared
    // engineering planner reads only this full household, policy, week and clock;
    // its context contains that one week and compiled immutable recipe material.
    // Never key by revisions, current profile, selected refs or receipt position.
    let key = canonical_digest(&json!([
        profile,
        snapshot.privacy,
        week,
        committed_at,
        view.pin()
    ]))?;
    if let Some(hit) = ORIGINAL_WEEK_FOOD_CACHE.with(|v| {
        v.borrow()
            .iter()
            .find(|(saved, _)| saved == &key)
            .map(|(_, proof)| proof.clone())
    }) {
        return Ok(hit);
    }
    let mut proof = ReceiptFoodProof::default();
    let prior = CLOCK.with(|v| v.replace(Some(committed_at.clone())));
    let derived = planner_view::derive(view, snapshot, profile, week);
    CLOCK.with(|v| *v.borrow_mut() = prior);
    if let Ok(food) = derived {
        if let Some(food) = food {
            proof.demands.push(food.plan.demand);
        }
        let active = mstrchf_core::availability::eligible_slots(week)?;
        for v in week
            .plan
            .selections
            .iter()
            .filter(|v| active.contains(&v.slot))
        {
            proof.selections.push(canonical_bytes(v)?);
        }
        for v in &week.plan.batch_links.0 {
            proof.links.push(canonical_bytes(v)?);
        }
    }
    ORIGINAL_WEEK_FOOD_CACHE.with(|v| {
        let mut v = v.borrow_mut();
        if v.len() >= 256 {
            v.pop_front();
        }
        v.push_back((key, proof.clone()));
    });
    Ok(proof)
}

fn commit_week(state: &mut BrowserState, mut next: Week, at: &Instant) -> Result<(), DomainError> {
    let old = state
        .retained_weeks
        .iter()
        .find(|v| v.range.week_id == next.range.week_id)
        .ok_or_else(|| error(ErrorCode::ForeignReference, "/week_id"))?
        .clone();
    let delta = mstrchf_core::history::selected_index_delta(&[old.clone()], &[next.clone()], at)?;
    if !delta.remove_slots.is_empty() || !delta.upsert.is_empty() {
        state.history.selected.0.retain(|v| {
            !delta.remove_slots.contains(&v.slot) && !delta.upsert.iter().any(|n| n.slot == v.slot)
        });
        state.history.selected.0.extend(delta.upsert);
        state.history.selected.0.sort_by_key(|v| {
            (
                v.slot.week_id.0.clone(),
                v.slot.date.0.clone(),
                canonical_bytes(&v.slot.meal_type).unwrap_or_default(),
            )
        });
        state.history.revision = state.history.revision.checked_increment()?;
    }
    next.revision = old.revision.checked_increment()?;
    next.plan.revision = old.plan.revision.checked_increment()?;
    next.approved_plan = Nullable(None);
    next.confirmed_order = old.confirmed_order.clone();
    next.publication = if old.confirmed_order.0.is_some() { PublicationState::NeedsUser } else { old.publication };
    let next: Week = parse_document(&canonical_bytes(&next)?)?;
    for w in &mut state.retained_weeks.0 {
        if w.range.week_id == next.range.week_id {
            *w = next.clone();
        }
    }
    for w in &mut state.snapshot.weeks.0 {
        if w.range.week_id == next.range.week_id {
            *w = next.clone();
        }
    }
    Ok(())
}
fn selection_error(v: mstrchf_core::selection::SelectionFailure) -> DomainError {
    match v {
        mstrchf_core::selection::SelectionFailure::Invalid(e) => e,
        mstrchf_core::selection::SelectionFailure::Blocked(v) => error(
            v.first().map_or(ErrorCode::DomainConflict, |v| v.code),
            "/proposal/food",
        ),
    }
}

// This private draft is published only after every requested week is checked.
// Each commit updates selected history, so later weeks see both earlier draft
// results and still-retained later weeks/ordered history through existing rules.
fn suggest_planning_horizon(
    state: &mut BrowserState,
    view: &catalogue::RuntimeCatalogue,
    ordered_week_ids: &BoundedVec<WeekId, 4, 4>,
    mode: PlanMode,
    allow_repeats: bool,
    at: &Instant,
) -> Result<(), DomainError> {
    if mode == PlanMode::SwapNamed {
        return Err(error(ErrorCode::ScopeViolation, "/suggest_horizon/mode"));
    }
    let mut current = state.snapshot.weeks.iter().collect::<Vec<_>>();
    current.sort_by_key(|week| (&week.range.monday, &week.range.week_id));
    if current.len() != 4 || ordered_week_ids.len() != 4
        || current.iter().zip(ordered_week_ids.iter()).any(|(week, id)| &week.range.week_id != id)
    {
        return Err(error(ErrorCode::ScopeViolation, "/suggest_horizon/ordered_week_ids"));
    }
    let profile = state.snapshot.profile.0.clone()
        .ok_or_else(|| error(ErrorCode::MissingField, "/profile"))?;
    let mut working = state.clone();
    let mut changed = false;
    for id in ordered_week_ids.iter() {
        let snapshot = working.snapshot.clone();
        let week = snapshot.weeks.iter().find(|week| &week.range.week_id == id).unwrap();
        match planner_view::suggest(view, &snapshot, &profile, week, &mut working.history.clone(), mode, &[], allow_repeats) {
            Ok(next) if next.plan != week.plan => {
                commit_week(&mut working, next, at)?;
                changed = true;
            }
            Ok(_) => {}
            Err(ref error) if error == &crate::error(ErrorCode::DomainConflict, "/suggest/no_fit_or_permission") => {}
            Err(error) => return Err(error),
        }
    }
    if !changed {
        return Err(error(ErrorCode::DomainConflict, "/suggest_horizon/no_fit_or_permission"));
    }
    *state = working;
    Ok(())
}

fn mutate(
    state: &mut BrowserState,
    command: &AppCommand,
    at: &Instant,
    today: Option<&LocalDate>,
) -> Result<(), DomainError> {
    let s = state.snapshot.clone();
    let view = runtime_catalogue(state)?;
    match command {
        AppCommand::OpenPlanningHorizon {} => open_horizon(state, at, today)?,
        AppCommand::SuggestPlanningHorizon { ordered_week_ids, mode, allow_repeats } => {
            suggest_planning_horizon(state, &view, ordered_week_ids, *mode, *allow_repeats, at)?;
        }
        AppCommand::SaveHouseholdDraft {
            draft,
            restriction_issue_edits,
        } => {
            mstrchf_core::setup::validate_setup_draft_transition(
                &s.household_draft,
                draft,
                restriction_issue_edits,
                &s.privacy,
                at,
            )?;
            let mut draft = draft.clone();
            if draft != s.household_draft {
                draft.revision = draft.revision.checked_increment()?;
            }
            state.snapshot.household_draft = draft;
        }
        AppCommand::UpdateProfile {
            profile,
            expected_profile_revision,
        } => {
            if profile.household_id != s.household_draft.household_id {
                return Err(error(
                    ErrorCode::IdentityMismatch,
                    "/command/data/profile/household_id",
                ));
            }
            mstrchf_core::household::validate_profile(profile, &s.privacy, at)?;
            let mut next = profile.clone();
            if let Some(old) = &s.profile.0 {
                for batch in state.retained_weeks.0.chunks(4) {
                    mstrchf_core::household::prepare_profile_update(
                        old,
                        profile,
                        *expected_profile_revision,
                        &s.privacy,
                        at,
                        batch,
                    )?;
                }
                if state.retained_weeks.is_empty() {
                    mstrchf_core::household::prepare_profile_update(
                        old,
                        profile,
                        *expected_profile_revision,
                        &s.privacy,
                        at,
                        &[],
                    )?;
                }
                next.revision = old.revision.checked_increment()?;
            } else {
                if *expected_profile_revision != Revision(1) || profile.revision != Revision(1) {
                    return Err(error(
                        ErrorCode::ContextStale,
                        "/command/data/expected_profile_revision",
                    ));
                }
                mstrchf_core::setup::validate_profile_draft_binding(
                    None,
                    &s.household_draft,
                    profile,
                    &s.privacy,
                    at,
                )?;
            }
            state.snapshot.profile = Nullable(Some(next.clone()));
            state.snapshot.household_draft = mstrchf_core::setup::draft_from_profile(
                &next,
                s.household_draft.revision.checked_increment()?,
                &s.privacy,
                at,
            )?;
            // Active publication is a claim about current household/calendar
            // settings. Retired weeks retain their historical status unchanged.
            let active: BTreeSet<_> = state.snapshot.weeks.iter().map(|week|week.range.week_id.clone()).collect();
            for week in state.retained_weeks.iter_mut().chain(state.snapshot.weeks.iter_mut()) {
                if active.contains(&week.range.week_id) && week.publication == PublicationState::Published {
                    week.publication = PublicationState::NeedsUser;
                }
            }
        }
        AppCommand::SetPrivacyPreservingEssentials { policy, expected_policy_revision } => {
            // Execute the frozen old transition, including proposal revocation,
            // policy/profile/draft revisions and ordinary approval invalidation.
            mutate(state, &AppCommand::SetPrivacy { policy: policy.clone(), expected_policy_revision: *expected_policy_revision }, at, today)?;
            state.essentials = BoundedVec(privacy_essentials::carry(&view, &s, &state.snapshot, &state.essentials.0, at));
        }
        AppCommand::SetPrivacy {
            policy,
            expected_policy_revision,
        } => {
            if *expected_policy_revision != s.privacy.revision
                || policy.revision != s.privacy.revision
            {
                return Err(error(
                    ErrorCode::ContextStale,
                    "/command/data/policy/revision",
                ));
            }
            if policy.confirmed_at.0.as_ref().is_some_and(|v| v > at) {
                return Err(error(
                    ErrorCode::InvalidValue,
                    "/command/data/policy/confirmed_at",
                ));
            }
            let mut next = policy.clone();
            next.revision = next.revision.checked_increment()?;
            state.snapshot.privacy = next.clone();
            if let Some(p) = &mut state.snapshot.profile.0 {
                p.connections.privacy = next.clone();
                p.revision = p.revision.checked_increment()?;
            }
            if let Some(c) = &mut state.snapshot.household_draft.connections.0 {
                c.privacy = next;
                state.snapshot.household_draft.revision = state
                    .snapshot
                    .household_draft
                    .revision
                    .checked_increment()?;
            }
            state.pending_proposal = None;
        }
        AppCommand::SaveWeek {
            week,
            expected_week_revision,
        } => {
            let p = s
                .profile
                .0
                .as_ref()
                .ok_or_else(|| error(ErrorCode::MissingField, "/profile"))?;
            let old = s
                .weeks
                .iter()
                .find(|v| v.range.week_id == week.week_id)
                .ok_or_else(|| error(ErrorCode::ForeignReference, "/week_id"))?;
            let mut next = planner_view::prepare_preserving_week_edit(
                &view,
                old,
                week,
                *expected_week_revision,
                p,
                at,
            )?
            .0.candidate;
            if let Some(plan) = &week.plan.0 {
                if plan.revision != old.plan.revision {
                    return Err(error(
                        ErrorCode::ContextStale,
                        "/command/data/week/plan/revision",
                    ));
                }
                next.plan = plan.clone();
            }
            let active = mstrchf_core::availability::eligible_slots(&next)?;
            for v in next
                .plan
                .selections
                .iter()
                .filter(|v| !active.contains(&v.slot))
            {
                if !old.plan.selections.contains(v) {
                    return Err(error(
                        ErrorCode::AuthorityDenied,
                        "/command/data/week/plan/inactive_selection",
                    ));
                }
            }
            for v in
                next.plan.batch_links.iter().filter(|v| {
                    !active.contains(&v.target_slot) || !active.contains(&v.source_slot)
                })
            {
                if !old.plan.batch_links.contains(v) {
                    return Err(error(
                        ErrorCode::AuthorityDenied,
                        "/command/data/week/plan/inactive_batch_link",
                    ));
                }
            }
            if next.plan != old.plan {
                let removal = next.plan.selections.len() < old.plan.selections.len()
                    && next
                        .plan
                        .selections
                        .iter()
                        .all(|v| old.plan.selections.contains(v))
                    && next
                        .plan
                        .batch_links
                        .iter()
                        .all(|v| old.plan.batch_links.contains(v));
                if !removal {
                    planner_view::derive(&view, &s, p, &next)?;
                }
            }
            commit_week(state, next, at)?;
        }
        AppCommand::SuggestMeals {
            week_id,
            mode,
            slots,
            allow_repeats,
        } => {
            let p = s
                .profile
                .0
                .as_ref()
                .ok_or_else(|| error(ErrorCode::MissingField, "/profile"))?;
            let w = s
                .weeks
                .iter()
                .find(|v| &v.range.week_id == week_id)
                .ok_or_else(|| error(ErrorCode::ForeignReference, "/week_id"))?;
            let next = planner_view::suggest(
                &view,
                &s,
                p,
                w,
                &mut state.history.clone(),
                *mode,
                &slots.0,
                *allow_repeats,
            )?;
            commit_week(state, next, at)?;
        }
        AppCommand::SavePersonalRecipe {
            candidate,
            previous,
        } => {
            let mut r = candidate.clone();
            if let Some(previous) = &previous.0 {
                let old = state
                    .personal_recipes
                    .iter()
                    .filter(|v| v.candidate_id == previous.recipe_id)
                    .max_by_key(|v| v.proposed_revision)
                    .ok_or_else(|| error(ErrorCode::MissingReference, "/recipe/previous"))?;
                if old.proposed_revision != previous.revision
                    || canonical_digest(old)?.0 != previous.digest.0
                {
                    return Err(error(ErrorCode::ContextStale, "/recipe/previous"));
                }
                r.candidate_id = old.candidate_id.clone();
                r.proposed_revision = old.proposed_revision.checked_increment()?;
            } else {
                r.candidate_id = RecipeId(planner_view::stable_id(
                    "rc",
                    &json!([s.household_draft.household_id, s.snapshot_revision, r]),
                )?);
                r.proposed_revision = Revision(1);
            }
            if state.personal_recipes.len() >= 512 {
                return Err(error(ErrorCode::LimitExceeded, "/library"));
            }
            r.image = Nullable(None);
            r.unresolved = mstrchf_core::recipes::validate_recipe(&r)?
                .capabilities
                .blocking_issues;
            state.personal_recipes.0.push(r);
        }
        AppCommand::AdmitReviewedRecipe { packet, expected_catalogue_revision } => {
            if state.public_catalogue_pin.is_some(){return Err(error(ErrorCode::AuthorityDenied,"/public_beta/non_public_admission"));}
            if state.receipts.is_empty() || state.snapshot.profile.0.is_none() {
                return Err(error(ErrorCode::MissingReference, "/reviewed_packets/saved_kitchen"));
            }
            if *expected_catalogue_revision != view.pin().revision {
                return Err(error(ErrorCode::ContextStale, "/expected_catalogue_revision"));
            }
            let known = maker_packets::known_packet(packet)?;
            if at < known.not_before() {
                return Err(error(ErrorCode::AuthorityDenied, "/reviewed_packets/admission_before_review"));
            }
            if state.reviewed_packets.contains(known.record()) {
                return Err(error(ErrorCode::DuplicateKey, "/reviewed_packets"));
            }
            if state.reviewed_packets.len() >= 512 {
                return Err(error(ErrorCode::LimitExceeded, "/reviewed_packets"));
            }
            view.require_admission_dependencies(known)?;
            state.reviewed_packets.0.push(known.record().clone());
            // No food effect is created by admission. It only makes this exact
            // compiled reviewed revision available for scoped future intent.
            runtime_catalogue(state)?.reviewed()?;
        }
        AppCommand::RecordFeedback { feedback } => {
            if !state
                .retained_weeks
                .iter()
                .flat_map(|v| v.plan.selections.iter())
                .any(|v| v.slot == feedback.slot && v.recipe == feedback.recipe)
            {
                return Err(error(ErrorCode::ForeignReference, "/feedback/selection"));
            }
            if feedback.portion_change.0.is_some() || feedback.portion_change_accepted {
                return Err(error(
                    ErrorCode::UnsupportedCapability,
                    "/feedback/portion_change",
                ));
            }
            let mut f = feedback.clone();
            f.recorded_at = at.clone();
            if f.hide
                && !state
                    .history
                    .hidden_recipe_ids
                    .contains(&f.recipe.recipe_id)
            {
                state
                    .history
                    .hidden_recipe_ids
                    .0
                    .push(f.recipe.recipe_id.clone());
                state.history.hidden_recipe_ids.0.sort();
            }
            state.history.feedback.0.push(f);
            state.history.revision = state.history.revision.checked_increment()?;
        }
        AppCommand::RestoreHiddenRecipe { recipe_id } => {
            state.history.hidden_recipe_ids.0.retain(|v| v != recipe_id);
            state.history.revision = state.history.revision.checked_increment()?;
        }
        AppCommand::ConfirmEssentials { confirmations } => {
            let p = s
                .profile
                .0
                .as_ref()
                .ok_or_else(|| error(ErrorCode::MissingField, "/profile"))?;
            for given in &confirmations.0 {
                let mut found = false;
                for w in &s.weeks.0 {
                    // A conflict in another independent week conveys no
                    // authority over this line. Only a successfully derived
                    // matching demand can admit the exact confirmation.
                    if let Ok(Some(food)) = planner_view::derive(&view, &s, p, w) {
                        if food
                            .plan
                            .demand
                            .lines
                            .iter()
                            .any(|v| v.line_id == given.line_id)
                        {
                            validate_essentials_binding(given, &food.plan.demand)?;
                            found = true;
                            break;
                        }
                    }
                }
                if !found {
                    return Err(error(ErrorCode::ForeignReference, "/confirmations/line_id"));
                }
                let mut given = given.clone();
                given.confirmed_at = at.clone();
                state.essentials.0.retain(|v| v.line_id != given.line_id);
                state.essentials.0.push(given);
            }
            state.essentials.0.sort_by_key(|v| v.line_id.clone());
        }
        AppCommand::ApprovePlan { week_id, plan_revision, derived_digest } => {
            let approved = booking::approve(&view, &s, week_id, *plan_revision, derived_digest, at)?;
            for week in state.retained_weeks.iter_mut().chain(state.snapshot.weeks.iter_mut()).filter(|w| &w.range.week_id == week_id) { week.approved_plan = Nullable(Some(approved.clone())); }
        }
        AppCommand::ConfirmOrder { confirmation } => {
            let order = booking::confirm(&s, confirmation, at)?;
            // Its exact historical basis must still verify before recording an
            // external user confirmation, including after a cold restart.
            booking::historical(state, &order)?;
            for week in state.retained_weeks.iter_mut().chain(state.snapshot.weeks.iter_mut()).filter(|w| w.range.week_id == confirmation.week_id) { week.confirmed_order = Nullable(Some(order.clone())); }
            for selection in state.history.selected.iter().filter(|s| s.week_id == confirmation.week_id) {
                state.history.ordered.0.push(OrderedHistory { selection: selection.clone(), order_id: order.order_id.clone(), ordered_at: at.clone() });
            }
            state.history.revision = state.history.revision.checked_increment()?;
        }
        AppCommand::CorrectOrderDelivery { order_id, delivery_start, delivery_end } => {
            let week_id = booking::correction_week(&s, &state.receipts.0, order_id, delivery_start, delivery_end, at)?;
            let order = s.weeks.iter().find(|week|week.range.week_id==week_id).unwrap().confirmed_order.0.as_ref().unwrap();
            booking::historical(state, order)?;
            for week in state.retained_weeks.iter_mut().chain(state.snapshot.weeks.iter_mut()).filter(|week|week.range.week_id==week_id) {
                week.publication = booking::corrected_publication(week.publication);
            }
        }
        AppCommand::RequestCalendarPublication { order_id, .. } => {
            let week_id = s.weeks.iter().find(|w| w.confirmed_order.0.as_ref().is_some_and(|o| &o.order_id == order_id)).map(|w| w.range.week_id.clone()).ok_or_else(|| error(ErrorCode::MissingReference, "/calendar/order"))?;
            for week in state.retained_weeks.iter_mut().chain(state.snapshot.weeks.iter_mut()).filter(|w| w.range.week_id == week_id) { week.publication = PublicationState::Prepared; }
        }
        AppCommand::ConfirmCalendarPublication { confirmation } => {
            let week_id = calendar_publication::confirm(state, confirmation, at)?;
            for week in state.retained_weeks.iter_mut().chain(state.snapshot.weeks.iter_mut()).filter(|week|week.range.week_id==week_id) {
                week.publication = PublicationState::Published;
            }
        }
        AppCommand::ApplyPlanProposal {
            proposal,
            context_stamp,
        } => {
            if !s.privacy.use_ai_features {
                return Err(error(ErrorCode::PrivacyDenied, "/use_ai_features"));
            }
            let pending = state
                .pending_proposal
                .clone()
                .ok_or_else(|| error(ErrorCode::AuthorityDenied, "/proposal/scope"))?;
            if pending.returned.as_ref() != Some(proposal)
                || context_stamp != &pending.context.stamp
            {
                return Err(error(ErrorCode::IdentityMismatch, "/proposal/return"));
            }
            let current = proposal_context(state, &pending.context.weeks[0].range.week_id)?;
            compare_context(context_stamp, &current.stamp)?;
            let intent = proposal_intent(&pending);
            let patch = mstrchf_core::selection::prepare_selection_patch_with_food(
                &current,
                &intent,
                proposal,
                &view.reviewed()?,
                &[],
                &view.food_evidence(&current.stamp.recipe_refs.0)?,
            )
            .map_err(selection_error)?;
            let p = s.profile.0.as_ref().unwrap();
            for w in patch.context.weeks.0 {
                planner_view::derive(&view, &s, p, &w)?;
                commit_week(state, w, at)?;
            }
            state.pending_proposal = None;
        }
        AppCommand::CancelJob { .. } => {
            return Err(error(ErrorCode::MissingReference, "/command/data/job_id"));
        }
        _ => return Err(error(ErrorCode::UnsupportedCapability, "/command")),
    }
    if !matches!(command, AppCommand::ApplyPlanProposal { .. }) {
        state.pending_proposal = None;
    }
    if !matches!(command, AppCommand::ApprovePlan { .. } | AppCommand::ConfirmOrder { .. } | AppCommand::CorrectOrderDelivery { .. } | AppCommand::ConfirmCalendarPublication { .. } | AppCommand::SuggestPlanningHorizon { .. }) {
        for week in state.retained_weeks.iter_mut().chain(state.snapshot.weeks.iter_mut()) {
            if week.confirmed_order.0.is_none() { week.approved_plan = Nullable(None); }
        }
    }
    refresh(state)?;
    Ok(())
}

fn receipt(
    state: &BrowserState,
    r: &ReceiptRecord,
    request: &str,
    replayed: bool,
) -> Result<Value, DomainError> {
    let v = serde_json::to_value(&r.command).map_err(records::storage)?;
    Ok(
        json!({"format_version":1,"disposition":if replayed {"replayed"}else{"applied"},"receipt_profile":"DevelopmentCurrent","request_encoding":"current_json","request_digest":bytes_digest(request.as_bytes()),"command_id":r.command.command_id,"command_kind":v["command"]["kind"],"expected_snapshot_revision":r.command.expected_snapshot_revision,"household_id":state.snapshot.household_draft.household_id,"receipt_snapshot_revision":r.snapshot_revision,"command_digest":r.command_digest,"response_digest":r.response_digest}),
    )
}
fn command_text(command: &AppCommandV1, raw: Option<String>) -> Result<String, DomainError> {
    let raw =
        raw.unwrap_or(String::from_utf8(canonical_bytes(command)?).map_err(records::storage)?);
    let parsed: AppCommandV1 = parse_document(raw.as_bytes())?;
    if &parsed != command {
        return Err(error(ErrorCode::IdentityMismatch, "/command_json"));
    }
    Ok(raw)
}
fn dispatch(
    mut state: BrowserState,
    command: AppCommandV1,
    raw: Option<String>,
    at: &Instant,
    today: Option<&LocalDate>,
) -> Result<Value, DomainError> {
    audit(&state, Some(at))?;
    let text = command_text(&command, raw)?;
    let digest = canonical_digest(&command)?;
    if let Some(r) = state
        .receipts
        .iter()
        .find(|v| v.command.command_id == command.command_id)
    {
        if r.command_digest != digest || r.command != command {
            return Err(error(ErrorCode::ReplayConflict, "/command_id"));
        }
        return Ok(json!({"state":state,"receipt":receipt(&state,r,&text,true)?}));
    }
    if command.expected_snapshot_revision != state.snapshot.snapshot_revision {
        return Err(error(
            ErrorCode::ContextStale,
            "/expected_snapshot_revision",
        ));
    }
    if state.receipts.len() >= 4096 {
        return Err(error(ErrorCode::LimitExceeded, "/receipts"));
    }
    let accepted_proposal = if let AppCommand::ApplyPlanProposal { proposal, .. } = &command.command
    {
        let pending = state
            .pending_proposal
            .as_ref()
            .ok_or_else(|| error(ErrorCode::MissingReference, "/proposal"))?;
        let result = ProposalResult {
            proposal_id: pending.proposal_id.clone(),
            context_digest: pending.context_digest.clone(),
            selections: BoundedVec(public_selections(proposal)?),
        };
        Some(AcceptedProposal {
            result_digest: canonical_digest(&result)?,
            result,
        })
    } else {
        None
    };
    let prepared_calendar = if let AppCommand::RequestCalendarPublication { order_id, destination_id, event_batch_digest } = &command.command {
        let (journal, _) = calendar_owner::prepare(&state, order_id, destination_id, at)?;
        if canonical_digest(&journal.batch)? != *event_batch_digest { return Err(error(ErrorCode::ContextStale, "/calendar/event_batch_digest")); }
        Some(journal)
    } else { None };
    if let AppCommand::ConfirmCalendarPublication { confirmation } = &command.command {
        // New acknowledgements must describe today's exact renderer output.
        // Historical validation deliberately uses the retained renderer instead.
        let (current, _) = calendar_owner::prepare(&state, &confirmation.order_id, &confirmation.destination_id, at)?;
        let prepared = state.receipts.iter().find(|receipt|receipt.command.command_id==confirmation.preparation_command_id)
            .and_then(|receipt|receipt.prepared_calendar.as_ref())
            .ok_or_else(||error(ErrorCode::MissingReference,"/calendar/publication/preparation"))?;
        if current.batch != prepared.batch {
            return Err(error(ErrorCode::ContextStale,"/calendar/publication/current_renderer"));
        }
    }
    let privacy_essentials_before = if matches!(command.command, AppCommand::SetPrivacyPreservingEssentials { .. }) {
        Some(state.essentials.clone())
    } else { None };
    mutate(&mut state, &command.command, at, today)?;
    state.snapshot.snapshot_revision = state.snapshot.snapshot_revision.checked_increment()?;
    let _: Snapshot = parse_document(&canonical_bytes(&state.snapshot)?)?;
    let r = ReceiptRecord {
        command,
        command_digest: digest,
        response_digest: canonical_digest(&state.snapshot)?,
        snapshot_revision: state.snapshot.snapshot_revision,
        response: state.snapshot.clone(),
        committed_at: at.clone(),
        accepted_proposal,
        food_catalogue_pin: Some(runtime_catalogue(&state)?.pin().clone()),
        prepared_calendar,
        privacy_essentials_before,
    };
    let response = receipt(&state, &r, &text, false)?;
    state.receipts.0.push(r);
    let state: BrowserState = parse_document(&canonical_bytes(&state)?)?;
    audit(&state, Some(at))?;
    Ok(json!({"state":state,"receipt":response}))
}
fn library(state: &BrowserState) -> Result<Vec<planner_state::LibraryEntry>, DomainError> {
    let view = runtime_catalogue(state)?;
    let mut entries = view.recipes().iter().map(|r| Ok(planner_state::LibraryEntry {
        reference: development::recipe_reference(r)?, recipe: Some(r.clone()), candidate: None,
        admission: if view.is_public_recipe(&development::recipe_reference(r)?) {"public_reviewed"}else if personal_scope::requires_personal_scope(&r.recipe_id) { "personal_reviewed" } else { "engineering" }.into(),
        notices: if view.is_public_recipe(&development::recipe_reference(r)?) {public_catalogue::notices()}else if personal_scope::requires_personal_scope(&r.recipe_id) { personal_scope::notices_for_recipe(&r.recipe_id) } else { vec!["Synthetic engineering premises. Production food, cooking and rights qualification are not established.".into()] },
    })).collect::<Result<Vec<_>, DomainError>>()?;
    for c in &state.personal_recipes.0 {
        entries.push(planner_state::LibraryEntry{recipe:None,candidate:Some(c.clone()),reference:RecipeRef{recipe_id:c.candidate_id.clone(),revision:c.proposed_revision,digest:RecipeDigest(canonical_digest(c)?.0)},admission:"personal_candidate".into(),notices:vec!["Personal candidate. Quantity, dietary, portion, storage, rights and cooking evidence require review before planning.".into()]});
    }
    Ok(entries)
}
fn reviewed_sources(state: &BrowserState) -> Result<Vec<planner_state::ReviewedSource>, DomainError> {
    if state.public_catalogue_pin.is_some(){return Ok(vec![]);}
    maker_packets::available_packet_refs()?.into_iter().map(|packet| {
        let known = maker_packets::known_packet(&packet)?;
        Ok(planner_state::ReviewedSource {
            recipe: known.recipe().clone(),
            scope: personal_scope::notices_for_recipe(&packet.recipe.recipe_id),
            admitted: state.reviewed_packets.contains(known.record()),
            packet,
        })
    }).collect()
}

fn board(
    state: &BrowserState,
    week_id: Option<&WeekId>,
    slot: Option<&SlotId>,
    options: Option<BoardOptions>,
) -> Result<Value, DomainError> {
    let s = &state.snapshot;
    let view = runtime_catalogue(state)?;
    let w = match week_id {
        Some(id) => s.weeks.iter().find(|v| &v.range.week_id == id),
        None => s.weeks.first(),
    }
    .ok_or_else(|| error(ErrorCode::MissingReference, "/week_id"))?;
    let p = s
        .profile
        .0
        .as_ref()
        .ok_or_else(|| error(ErrorCode::MissingReference, "/profile"))?;
    let slot = slot.cloned().unwrap_or_else(|| w.cells[2].slot.clone());
    let opts = options.unwrap_or_default();
    let opts = planner_view::ChoiceOptions {
        overrides: opts.overrides.unwrap_or(MealOverrides {
            portions: BoundedVec(vec![]),
            eat_by: Nullable(None),
        }),
        extra_recipients: opts.extra_recipients.unwrap_or(BoundedVec(vec![])).0,
        make_extra: opts.make_extra.unwrap_or(false),
        leftover_source: opts.leftover_source,
        eat_by_time: opts.eat_by_time,
        time_resolution: opts.time_resolution.unwrap_or(Resolution::Unambiguous),
        portable_cold: opts.portable_cold,
    };
    let choices = planner_view::custom_choices(&view, s, p, w, &slot, opts)?;
    let (derived, food_error) = match planner_view::derive(&view, s, p, w) {
        Ok(v) => (v.map(|v| v.plan), None),
        Err(e) => (None, Some(e.code)),
    };
    let shopping_list = derived.as_ref().map(|derived| shopping_list::project(state, &view, w, derived, &development::now()?)).transpose()?;
    // Public cooking details use the same admitted exact scaling as demand.
    // An excluded/conflicting/uncommitted meal cannot publish these quantities.
    let selected_recipe_quantities = if view.is_public_baseline() && food_error.is_none()
        && mstrchf_core::availability::eligible_slots(w)?.contains(&slot) {
        if let Some(selected)=w.plan.selections.iter().find(|selected|selected.slot==slot && view.is_public_recipe(&selected.recipe)) {
            let reviewed=view.reviewed()?;
            let accepted=reviewed.iter().find(|recipe|development::recipe_reference(recipe.recipe()).as_ref()==Ok(&selected.recipe))
                .ok_or_else(||error(ErrorCode::MissingReference,"/public_beta/selected_quantities/recipe"))?;
            match selected.batch_use {
                BatchUse::NewBatch{production_servings,..}=>{
                    let scaled=mstrchf_core::portions::scale_recipe(accepted,production_servings)?;
                    let ingredients=scaled.ingredients.iter().map(|ingredient|Ok(json!({
                        "ingredient_id":ingredient.ingredient_id,
                        "name":accepted.recipe().ingredients.iter().find(|source|source.ingredient_id==ingredient.ingredient_id)
                            .ok_or_else(||error(ErrorCode::MissingReference,"/public_beta/selected_quantities/ingredient"))?.name,
                        "quantity":ingredient.quantity
                    }))).collect::<Result<Vec<_>,DomainError>>()?;
                    Some(json!({"slot":slot,"recipe_reference":selected.recipe,"source_servings":accepted.recipe().r#yield.servings,
                        "requested_servings":scaled.requested_servings,"multiplier":scaled.multiplier,"ingredients":ingredients,"equipment":accepted.recipe().equipment}))
                },
                _=>None,
            }
        }else{None}
    }else{None};
    let details = planner_state::PlannerDetails {
        library: library(state)?,
        history: state.history.clone(),
        derived,
        essentials: state.essentials.0.clone(),
        food_error,
        catalogue_revision: view.pin().revision,
        reviewed_sources: reviewed_sources(state)?,
        shopping_list,
        test_kitchen: state.engineering_demo,
    };
    let mut result = json!(development::DevelopmentBoard {
        week: w.clone(),
        choices,
        snapshot_revision: s.snapshot_revision,
        details: Some(details)
    });
    result["details"]["effective_delivery"] = json!(w.confirmed_order.0.as_ref().map(|order|booking::effective_delivery(order, &state.receipts.0)).transpose()?);
    if view.is_public_baseline(){result["details"]["selected_recipe_quantities"]=json!(selected_recipe_quantities);}
    Ok(result)
}

fn proposal_context(
    state: &BrowserState,
    week_id: &WeekId,
) -> Result<PlanningContext, DomainError> {
    let s = &state.snapshot;
    let p = s
        .profile
        .0
        .as_ref()
        .ok_or_else(|| error(ErrorCode::MissingField, "/profile"))?;
    let w = s
        .weeks
        .iter()
        .find(|v| &v.range.week_id == week_id)
        .ok_or_else(|| error(ErrorCode::ForeignReference, "/week_id"))?;
    let view = runtime_catalogue(state)?;
    let mut c = planner_view::context(&view, s, p, w)?;
    c.history = state.history.clone();
    c.stamp.history_revision = c.history.revision;
    Ok(c)
}
fn proposal_intent(p: &PendingProposal) -> mstrchf_core::selection::SelectionIntent {
    mstrchf_core::selection::SelectionIntent {
        mode: p.mode,
        slots: p.slots.0.clone(),
        origin: SelectionOrigin::Suggested,
        repeat_permissions: if p.allow_repeats {
            p.choices
                .iter()
                .flat_map(|v| {
                    v.choices
                        .iter()
                        .map(|c| mstrchf_core::selection::RepeatPermission {
                            slot: c.slot.clone(),
                            recipe: c.recipe.clone(),
                        })
                })
                .collect()
        } else {
            vec![]
        },
    }
}
fn proposal_choices(
    state: &BrowserState,
    context: &PlanningContext,
    slot: &SlotId,
    mode: PlanMode,
    allow_repeats: bool,
) -> Result<Vec<development::DevelopmentChoice>, DomainError> {
    let view = runtime_catalogue(state)?;
    let choices = planner_view::choices(
        &view,
        &state.snapshot,
        &context.household,
        &context.weeks[0],
        slot,
        MealOverrides {
            portions: BoundedVec(vec![]),
            eat_by: Nullable(None),
        },
    )?;
    let mut out = Vec::new();
    for choice in choices {
        let mut selection = choice.selection.clone();
        selection.origin = SelectionOrigin::Suggested;
        selection.repeat_authorised = allow_repeats;
        let intent = mstrchf_core::selection::SelectionIntent {
            mode,
            slots: vec![slot.clone()],
            origin: SelectionOrigin::Suggested,
            repeat_permissions: if allow_repeats {
                vec![mstrchf_core::selection::RepeatPermission {
                    slot: slot.clone(),
                    recipe: selection.recipe.clone(),
                }]
            } else {
                vec![]
            },
        };
        let proposal = PlanProposal {
            changes: BoundedVec(vec![WeekChange {
                week_id: context.weeks[0].range.week_id.clone(),
                base_revision: context.weeks[0].revision,
                replace_slots: BoundedVec(vec![ProposedSelection {
                    slot: slot.clone(),
                    selection: Nullable(Some(selection)),
                }]),
            }]),
        };
        if let Ok(patch) = mstrchf_core::selection::prepare_selection_patch_with_food(
            context,
            &intent,
            &proposal,
            &view.reviewed()?,
            &[],
            &view.food_evidence(&context.stamp.recipe_refs.0)?,
        ) {
            if patch
                .context
                .weeks
                .iter()
                .all(|w| planner_view::derive(&view, &state.snapshot, &context.household, w).is_ok())
            {
                out.push(choice);
            }
        }
    }
    Ok(out)
}
fn prepare_proposal(
    mut state: BrowserState,
    week_id: WeekId,
    slots: BoundedVec<SlotId, 1, 21>,
    mode: PlanMode,
    allow_repeats: bool,
    nonce: Option<Text>,
    at: &Instant,
) -> Result<Value, DomainError> {
    audit(&state, Some(at))?;
    if !state.snapshot.privacy.use_ai_features {
        return Err(error(ErrorCode::PrivacyDenied, "/use_ai_features"));
    }
    let context = proposal_context(&state, &week_id)?;
    let w = &context.weeks[0];
    let active = mstrchf_core::availability::eligible_slots(w)?;
    let mut seen = BTreeSet::new();
    let mut choices = Vec::new();
    let mut outbound = Vec::new();
    for slot in &slots.0 {
        if !seen.insert(canonical_bytes(slot)?) || !active.contains(slot) {
            return Err(error(ErrorCode::ScopeViolation, "/slots"));
        }
        let old = w.plan.selections.iter().find(|v| &v.slot == slot);
        if mode == PlanMode::FillOpen && old.is_some()
            || mode == PlanMode::ReplaceSuggestions
                && !old.is_some_and(|v| v.origin == SelectionOrigin::Suggested)
        {
            return Err(error(ErrorCode::AuthorityDenied, "/slots"));
        }
        let given = proposal_choices(&state, &context, slot, mode, allow_repeats)?;
        outbound.push(json!({"slot":slot,"choices":given.iter().map(|v|json!({"recipe":v.selection.recipe,"title":v.title})).collect::<Vec<_>>()}));
        choices.push(ScopedChoices {
            slot: slot.clone(),
            choices: BoundedVec(given.into_iter().map(|v| v.selection).collect()),
        });
    }
    let context_digest = canonical_digest(&context.stamp)?;
    // The browser supplies a fresh UUID, so cancellation/re-prepare cannot reuse
    // a correlation ID merely because its clock still has the same second.
    // Omitted nonce retains the original direct-caller ID convention.
    let proposal_material = match nonce {
        Some(nonce) => json!([context_digest, slots, mode, allow_repeats, at, nonce]),
        None => json!([context_digest, slots, mode, allow_repeats, at]),
    };
    let proposal_id = planner_view::stable_id("pr", &proposal_material)?;
    state.pending_proposal = Some(PendingProposal {
        proposal_id: Text(proposal_id.clone()),
        context_digest: context_digest.clone(),
        context,
        mode,
        slots,
        allow_repeats,
        choices: BoundedVec(choices),
        returned: None,
    });
    // Public provider text contains no household facts, declarations or person allocations.
    let request = json!({"version":1,"proposal_id":proposal_id,"context_digest":context_digest,"mode":mode,"slots":outbound});
    Ok(json!({"state":state,"request":request}))
}
fn return_proposal(
    mut state: BrowserState,
    result: ProposalResult,
    at: &Instant,
) -> Result<Value, DomainError> {
    audit(&state, Some(at))?;
    let view = runtime_catalogue(&state)?;
    if let Some(accepted) = state
        .receipts
        .iter()
        .filter_map(|r| r.accepted_proposal.as_ref())
        .find(|a| a.result.proposal_id == result.proposal_id)
    {
        if accepted.result != result || accepted.result_digest != canonical_digest(&result)? {
            return Err(error(
                ErrorCode::ReplayConflict,
                "/proposal/accepted_result",
            ));
        }
        // Read-only acknowledgement remains harmless after privacy revocation.
        // It creates no command, plan effect, receipt or replacement scope.
        return Ok(json!({"state":state,"command":null,"status":"already_applied"}));
    }
    if !state.snapshot.privacy.use_ai_features {
        return Err(error(ErrorCode::PrivacyDenied, "/use_ai_features"));
    }
    let mut pending = state
        .pending_proposal
        .clone()
        .ok_or_else(|| error(ErrorCode::MissingReference, "/proposal"))?;
    if result.proposal_id != pending.proposal_id || result.context_digest != pending.context_digest
    {
        return Err(error(ErrorCode::IdentityMismatch, "/proposal/return"));
    }
    let context = proposal_context(&state, &pending.context.weeks[0].range.week_id)?;
    compare_context(&pending.context.stamp, &context.stamp)?;
    let mut selected = Vec::new();
    let mut seen = BTreeSet::new();
    for given in result.selections.0 {
        if !seen.insert(canonical_bytes(&given.slot)?) {
            return Err(error(ErrorCode::DuplicateKey, "/selections"));
        }
        let local = pending
            .choices
            .iter()
            .find(|v| v.slot == given.slot)
            .and_then(|v| v.choices.iter().find(|v| v.recipe == given.recipe))
            .ok_or_else(|| error(ErrorCode::ScopeViolation, "/selections"))?;
        let mut local = local.clone();
        local.origin = SelectionOrigin::Suggested;
        local.repeat_authorised = pending.allow_repeats;
        selected.push(ProposedSelection {
            slot: given.slot,
            selection: Nullable(Some(local)),
        });
    }
    let proposal = PlanProposal {
        changes: BoundedVec(vec![WeekChange {
            week_id: context.weeks[0].range.week_id.clone(),
            base_revision: context.weeks[0].revision,
            replace_slots: BoundedVec(selected),
        }]),
    };
    if pending
        .returned
        .as_ref()
        .is_some_and(|old| old != &proposal)
    {
        return Err(error(ErrorCode::ReplayConflict, "/proposal/returned"));
    }
    let patch = mstrchf_core::selection::prepare_selection_patch_with_food(
        &context,
        &proposal_intent(&pending),
        &proposal,
        &view.reviewed()?,
        &[],
        &view.food_evidence(&context.stamp.recipe_refs.0)?,
    )
    .map_err(selection_error)?;
    for w in &patch.context.weeks.0 {
        planner_view::derive(&view, &state.snapshot, &context.household, w)?;
    }
    pending.returned = Some(proposal.clone());
    let stamp = pending.context.stamp.clone();
    state.pending_proposal = Some(pending);
    Ok(
        json!({"state":state,"command":AppCommand::ApplyPlanProposal{proposal,context_stamp:stamp},"status":"review_ready"}),
    )
}

fn public_selections(proposal: &PlanProposal) -> Result<Vec<ReturnedChoice>, DomainError> {
    proposal
        .changes
        .iter()
        .flat_map(|c| c.replace_slots.iter())
        .map(|v| {
            let selection =
                v.selection.0.as_ref().ok_or_else(|| {
                    error(ErrorCode::IdentityMismatch, "/proposal/public_selection")
                })?;
            Ok(ReturnedChoice {
                slot: v.slot.clone(),
                recipe: selection.recipe.clone(),
            })
        })
        .collect()
}

pub fn execute(bytes: &[u8]) -> Result<Value, DomainError> {
    let request: Request = parse_document(bytes)?;
    let at = match &request {
        Request::Initialize { now, .. }
        | Request::Validate { now, .. }
        | Request::ValidateState { now, .. }
        | Request::PendingProposal { now, .. }
        | Request::RevokePendingProposal { now, .. }
        | Request::Conflicts { now, .. }
        | Request::Board { now, .. }
        | Request::Dispatch { now, .. }
        | Request::PrepareProposal { now, .. }
        | Request::ReturnProposal { now, .. } => Some(now.clone()),
        Request::CalendarPreview { now, .. } | Request::ProductMatchContext { now, .. }
        | Request::ProductSourcePrepare {now,..} | Request::ProductSourceInspect {now,..}
        | Request::ProductSourceCheckReturn {now,..} | Request::ProductSourceAccept {now,..} | Request::ProductSourceOperatorAccept {now,..}
        | Request::BasketAdditionPrepare {now,..} | Request::BasketAdditionCheckSource {now,..} | Request::BasketAdditionCheckSemantic {now,..}
        | Request::BasketAdditionOperatorReview {now,..}
        | Request::BasketAdditionPrepareV3 {now,..} | Request::BasketAdditionCheckSourceV3 {now,..} | Request::BasketAdditionCheckSemanticV3 {now,..}
        | Request::BasketAdditionOperatorReviewV3 {now,..}
        | Request::BasketSourcePrepare {now,..} | Request::BasketSourceInspect {now,..}
        | Request::BasketSourceCheckReturn {now,..} | Request::BasketSourceAccept {now,..}
        | Request::BasketSourceLocalInspect {now,..} | Request::BasketSourceLocalCheckReturn {now,..} | Request::BasketSourceLocalAccept {now,..}
        | Request::ProductMeaningReviewPrepare {now,..} | Request::ProductSemanticPrepare {now,..}
        | Request::ProductMeaningReviewPrepareV3 {now,..} | Request::ProductSemanticPrepareV3 {now,..}
        | Request::ProductSemanticInspect {now,..} | Request::ProductSemanticCheckReturn {now,..}
        | Request::ProductSemanticEvaluate {now,..} | Request::ProductMeaningQualify {now,..} => Some(now.clone()),
        Request::Snapshot { now, .. } | Request::Lookup { now, .. } => now.clone(),
        Request::PublicStarterCatalogue {..} | Request::ResolveMoment { .. } | Request::ProductSourceAudit { .. } | Request::ProductSemanticAudit { .. } | Request::BasketSourceAudit { .. } => None,
    };
    CLOCK.with(|v| *v.borrow_mut() = at.clone());
    let answer = match request {
        Request::PublicStarterCatalogue {} => {
            let view=catalogue::RuntimeCatalogue::public_starters()?;
            let recipes=view.recipes().iter().map(|recipe|planner_state::LibraryEntry{reference:development::recipe_reference(recipe).expect("validated compiled recipe"),recipe:Some(recipe.clone()),candidate:None,admission:"public_reviewed".into(),notices:public_catalogue::notices()}).collect::<Vec<_>>();
            Ok(json!({"format":"mstrchf-public-starter-catalogue","version":1,"catalogue_pin":view.pin(),"recipes":recipes}))
        },
        Request::ProductSourcePrepare {state,week_id,reader,key,delivery,targets,job_id,read_id,now,deadline}=>{
            audit(&state,Some(&now))?;
            Ok(json!(product_source_reads::prepare(&state,&week_id,reader,key,delivery,targets,job_id,read_id,&now,deadline)?))
        },
        Request::ProductSourceInspect {state,prepared,now}=>{audit(&state,Some(&now))?;product_source_reads::inspect(&state,&prepared,&now)},
        Request::ProductSourceCheckReturn {state,prepared,observation,now}=>{audit(&state,Some(&now))?;product_source_reads::check_return(&state,&prepared,&observation,&now)},
        Request::ProductSourceAccept {state,prepared,observation,evidence_id,now}=>{audit(&state,Some(&now))?;product_source_reads::accept(&state,&prepared,&observation,evidence_id,&now)},
        Request::ProductSourceOperatorAccept {state,prepared,observation,evidence_id,operator_task_ref,now}=>{audit(&state,Some(&now))?;product_source_reads::accept_operator(&state,&prepared,&observation,evidence_id,&operator_task_ref,&now)},
        Request::ProductSourceAudit {prepared,observation,accepted}=>product_source_reads::audit(&prepared,observation.0.as_ref(),accepted.0.as_ref()),
        Request::BasketAdditionPrepare {state,week_id,reader,key,delivery,target,line_id,job_id,read_id,review_id,now,deadline}=>{
            audit(&state,Some(&now))?;Ok(json!(basket_addition_review::prepare(&state,&week_id,reader,key,delivery,target,line_id,job_id,read_id,review_id,&now,deadline)?))
        },
        Request::BasketAdditionCheckSource {state,prepared,observation,now}=>{audit(&state,Some(&now))?;Ok(json!(basket_addition_review::check_source(&state,&prepared,&observation,&now)?))},
        Request::BasketAdditionCheckSemantic {state,prepared,returned,now}=>{audit(&state,Some(&now))?;Ok(json!(basket_addition_review::check_semantic(&state,&prepared,&returned,&now)?))},
        Request::BasketAdditionOperatorReview {state,prepared,returned,reviewed_at,decision,now}=>{
            audit(&state,Some(&now))?;Ok(json!(basket_addition_review::operator_review(&state,&prepared,&returned,&reviewed_at,&decision,&now)?))
        },
        Request::BasketAdditionPrepareV3 {state,week_id,reader,key,delivery,target,line_id,job_id,read_id,review_id,now,deadline}=>{
            audit(&state,Some(&now))?;Ok(json!(basket_addition_review::prepare_v3(&state,&week_id,reader,key,delivery,target,line_id,job_id,read_id,review_id,&now,deadline)?))
        },
        Request::BasketAdditionCheckSourceV3 {state,prepared,observation,now}=>{audit(&state,Some(&now))?;Ok(json!(basket_addition_review::check_source_v3(&state,&prepared,&observation,&now)?))},
        Request::BasketAdditionCheckSemanticV3 {state,prepared,returned,now}=>{audit(&state,Some(&now))?;Ok(json!(basket_addition_review::check_semantic_v3(&state,&prepared,&returned,&now)?))},
        Request::BasketAdditionOperatorReviewV3 {state,prepared,returned,reviewed_at,decision,now}=>{
            audit(&state,Some(&now))?;Ok(json!(basket_addition_review::operator_review_v3(&state,&prepared,&returned,&reviewed_at,&decision,&now)?))
        },
        Request::BasketSourcePrepare {state,week_id,reader,key,delivery,job_id,read_id,now,deadline}=>{
            audit(&state,Some(&now))?;Ok(json!(basket_source_reads::prepare(&state,&week_id,reader,key,delivery,job_id,read_id,&now,deadline)?))
        },
        Request::BasketSourceInspect {state,prepared,now}=>{audit(&state,Some(&now))?;basket_source_reads::inspect(&state,&prepared,&now)},
        Request::BasketSourceCheckReturn {state,prepared,returned,observation_id,now}=>{audit(&state,Some(&now))?;Ok(json!(basket_source_reads::check_return(&state,&prepared,&returned,observation_id,&now)?))},
        Request::BasketSourceAccept {state,prepared,returned,observation,evidence_id,now}=>{audit(&state,Some(&now))?;basket_source_reads::accept(&state,&prepared,&returned,&observation,evidence_id,&now)},
        Request::BasketSourceLocalInspect {state,prepared,now}=>{audit(&state,Some(&now))?;basket_source_reads::local_inspect(&state,&prepared,&now)},
        Request::BasketSourceLocalCheckReturn {state,prepared,returned,observation_id,now}=>{audit(&state,Some(&now))?;Ok(json!(basket_source_reads::local_check_return(&state,&prepared,&returned,observation_id,&now)?))},
        Request::BasketSourceLocalAccept {state,prepared,returned,observation,evidence_id,now}=>{audit(&state,Some(&now))?;basket_source_reads::local_accept(&state,&prepared,&returned,&observation,evidence_id,&now)},
        Request::BasketSourceAudit {prepared,returned,observation,accepted}=>Ok(json!(basket_source_reads::audit(&prepared,returned.0.as_ref(),observation.0.as_ref(),accepted.0.as_ref())?)),
        Request::ProductMeaningReviewPrepare {state,week_id,prepared,observation,accepted,product,line_id,now}=>{
            audit(&state,Some(&now))?;
            Ok(json!(product_meaning_review::prepare(&state,&week_id,&prepared,&observation,&accepted,&product,&line_id,&now)?))
        },
        Request::ProductMeaningReviewPrepareV3 {state,week_id,prepared,observation,accepted,product,line_id,now}=>{
            audit(&state,Some(&now))?;
            Ok(json!(product_meaning_review::prepare_v3(&state,&week_id,&prepared,&observation,&accepted,&product,&line_id,&now)?))
        },
        Request::ProductSemanticPrepare {state,week_id,source_prepared,observation,accepted,product,line_id,request_id,now,deadline}=>{
            audit(&state,Some(&now))?;
            Ok(json!(product_semantic_review::prepare(&state,&week_id,&source_prepared,&observation,&accepted,&product,&line_id,request_id,&now,deadline)?))
        },
        Request::ProductSemanticPrepareV3 {state,week_id,source_prepared,observation,accepted,product,line_id,request_id,now,deadline}=>{
            audit(&state,Some(&now))?;
            Ok(json!(product_semantic_review::prepare_v3(&state,&week_id,&source_prepared,&observation,&accepted,&product,&line_id,request_id,&now,deadline)?))
        },
        Request::ProductSemanticInspect {state,prepared,now}=>{audit(&state,Some(&now))?;Ok(json!(product_semantic_review::inspect(&state,&prepared,&now)?))},
        Request::ProductSemanticCheckReturn {state,prepared,returned,now}=>{audit(&state,Some(&now))?;Ok(json!(product_semantic_review::check_return(&state,&prepared,&returned,&now)?))},
        Request::ProductSemanticEvaluate {state,prepared,returned,evidence_id,now}=>{audit(&state,Some(&now))?;Ok(json!(product_semantic_review::evaluate(&state,&prepared,&returned,evidence_id,&now)?))},
        Request::ProductSemanticAudit {prepared,returned,recorded}=>Ok(json!(product_semantic_review::audit(&prepared,returned.0.as_ref(),recorded.0.as_ref())?)),
        Request::ProductMeaningQualify {state,prepared,recorded,now}=>{audit(&state,Some(&now))?;Ok(json!(product_meaning_admission::evaluate(&state,&prepared,&recorded,&now)?))},
        Request::ProductMatchContext {state, week_id, now} => {
            audit(&state, Some(&now))?;
            Ok(json!(product_match_context::project(&state, &week_id, &now)?))
        }
        Request::ResolveMoment { date, time, zone, resolution } => Ok(json!(booking::resolve_moment(&date, &time, &zone, resolution)?)),
        Request::CalendarPreview { state, order_id, destination_id, now } => {
            audit(&state, Some(&now))?;
            let (journal, ics) = calendar_owner::prepare(&state, &order_id, &destination_id, &now)?;
            let publication_ready = state.snapshot.weeks.iter().any(|week|matches!(week.publication,PublicationState::Prepared|PublicationState::Published) && week.confirmed_order.0.as_ref().is_some_and(|order|order.order_id==order_id));
            let prepared = publication_ready && state.receipts.iter().rev().filter_map(|receipt| receipt.prepared_calendar.as_ref()).find(|prior| prior.batch.order_id == order_id && prior.batch.destination_id == destination_id).is_some_and(|prior| prior.batch == journal.batch);
            Ok(json!({"event_batch_digest":canonical_digest(&journal.batch)?,"batch":journal.batch,"ics":ics,"notice":calendar_view::MANUAL_IMPORT_NOTICE,"prepared":prepared}))
        }
        Request::Initialize {
            household_id,
            diner_id,
            zone,
            today,
            now,
            demo,
            public_beta,
        } => {
            let mut state = BrowserState {
                format_version: SchemaVersion,
                snapshot: initial(household_id),
                retained_weeks: BoundedVec(vec![]),
                history: empty_history(),
                personal_recipes: BoundedVec(vec![]),
                reviewed_packets: BoundedVec(vec![]),
                essentials: BoundedVec(vec![]),
                receipts: BoundedVec(vec![]),
                pending_proposal: None,
                engineering_demo: demo.unwrap_or(false),
                public_catalogue_pin: if public_beta.unwrap_or(false){Some(catalogue::RuntimeCatalogue::public_starters()?.pin().clone())}else{None},
            };
            if state.engineering_demo {
                let mut p = demo_profile(&state.snapshot, &diner_id, &zone)?;
                if state.public_catalogue_pin.is_some() {
                    // A fictional demo household may use the separately
                    // qualified public foods. No real household declaration
                    // or legacy fixture is migrated by this explicit mode.
                    p.diners[0].age_band = AgeBand::Adult;
                    p.diners[0].restrictions = RestrictionDeclaration::ConfirmedNone { confirmed_at: now.clone() };
                    p.diners[0].portions = PortionPreference { preset: PortionPresetName::Regular, adjustment: PositiveRational::new(1,1)?, meal_overrides: BoundedVec(vec![]) };
                    p.preferences.equipment = BoundedVec(vec![EquipmentCode::Fridge, EquipmentCode::Hob, EquipmentCode::Kettle, EquipmentCode::Oven, EquipmentCode::Toaster]);
                    p.preferences.preferred_hands_on_minutes = 60;
                    p.usual_week.day_enabled = BoundedVec(vec![true;7]);
                    p.usual_week.enabled_meal_types = BoundedVec(vec![MealType::Breakfast, MealType::Dinner, MealType::Lunch]);
                    for cell in &mut p.usual_week.cells.0 { cell.choice = MealLocation::AtHome; }
                }
                state.snapshot.household_draft = mstrchf_core::setup::draft_from_profile(
                    &p,
                    Revision(1),
                    &state.snapshot.privacy,
                    &now,
                )?;
                state.snapshot.profile = Nullable(Some(p));
                open_horizon(&mut state, &now, Some(&today))?;
            }
            refresh(&mut state)?;
            audit(&state, Some(&now))?;
            Ok(json!(state))
        }
        Request::Snapshot { state, now } => {
            audit(&state, now.as_ref())?;
            Ok(json!(state.snapshot))
        }
        Request::Validate { state, now } | Request::ValidateState { state, now } => {
            audit(&state, Some(&now))?;
            Ok(json!(state))
        }
        Request::RevokePendingProposal { mut state, now } => {
            // Restore/account-context cancellation is a private-journal change.
            // Audit the exact supplied state before removing its unapplied scope;
            // committed history, accepted receipts and every revision are retained.
            audit(&state, Some(&now))?;
            state.pending_proposal = None;
            Ok(json!(state))
        }
        Request::Conflicts {
            state,
            offset,
            limit,
            now,
        } => {
            audit(&state, Some(&now))?;
            if limit == 0 || limit > 100 {
                return Err(error(ErrorCode::LimitExceeded, "/limit"));
            }
            let all = conflicts(&state)?;
            let total = all.len();
            let page = all
                .into_iter()
                .skip(offset as usize)
                .take(limit as usize)
                .collect::<Vec<_>>();
            let next = offset as usize + page.len();
            Ok(
                json!({"conflicts":page,"total":total,"offset":offset,"next_offset":if next<total {Some(next)}else {None}}),
            )
        }
        Request::PendingProposal { state, now } => {
            audit(&state, Some(&now))?;
            let view = runtime_catalogue(&state)?;
            if let Some(p) = &state.pending_proposal {
                let request = json!({"version":1,"proposal_id":p.proposal_id,"context_digest":p.context_digest,"mode":p.mode,"slots":p.choices.iter().map(|v|json!({"slot":v.slot,"choices":v.choices.iter().map(|c|json!({"recipe":c.recipe,"title":view.resolve_recipe(&c.recipe).ok().map(|r|&r.title)})).collect::<Vec<_>>()})).collect::<Vec<_>>()});
                let command = p
                    .returned
                    .as_ref()
                    .map(|proposal| AppCommand::ApplyPlanProposal {
                        proposal: proposal.clone(),
                        context_stamp: p.context.stamp.clone(),
                    });
                Ok(
                    json!({"request":request,"command":command,"status":if p.returned.is_some() {"review_ready"}else {"awaiting_return"}}),
                )
            } else {
                Ok(json!({"request":null,"command":null,"status":"idle"}))
            }
        }
        Request::Board {
            state,
            week_id,
            slot,
            options,
            now,
        } => {
            audit(&state, Some(&now))?;
            board(&state, week_id.as_ref(), slot.as_ref(), options)
        }
        Request::Dispatch {
            state,
            command,
            command_json,
            now,
            today,
        } => dispatch(state, command, command_json, &now, today.as_ref()),
        Request::Lookup {
            state,
            command,
            command_json,
            now,
        } => {
            audit(&state, now.as_ref())?;
            let text = command_text(&command, command_json)?;
            let r = state
                .receipts
                .iter()
                .find(|v| v.command.command_id == command.command_id)
                .ok_or_else(|| error(ErrorCode::MissingReference, "/receipt"))?;
            if r.command != command {
                return Err(error(ErrorCode::ReplayConflict, "/command_id"));
            }
            Ok(json!({"receipt":receipt(&state,r,&text,true)?}))
        }
        Request::PrepareProposal {
            state,
            week_id,
            slots,
            mode,
            allow_repeats,
            nonce,
            now,
        } => prepare_proposal(state, week_id, slots, mode, allow_repeats, nonce, &now),
        Request::ReturnProposal { state, result, now } => return_proposal(state, result, &now),
    };
    CLOCK.with(|v| *v.borrow_mut() = None);
    answer
}
pub fn run_json(bytes: &[u8]) -> Vec<u8> {
    let response = match execute(bytes) {
        Ok(value) => json!({"ok":true,"value":value}),
        Err(error) => json!({"ok":false,"error":error}),
    };
    serde_json::to_vec(&response).expect("JSON response")
}

// No wasm-bindgen runtime or JavaScript imports. Host allocates and frees both buffers.
#[unsafe(no_mangle)]
pub extern "C" fn alloc(len: u32) -> u32 {
    let bytes = vec![0u8; len as usize].into_boxed_slice();
    Box::into_raw(bytes) as *mut u8 as u32
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn dealloc(ptr: u32, len: u32) {
    if ptr != 0 {
        let slice = std::ptr::slice_from_raw_parts_mut(ptr as *mut u8, len as usize);
        unsafe {
            drop(Box::from_raw(slice));
        }
    }
}
#[unsafe(no_mangle)]
pub unsafe extern "C" fn run(ptr: u32, len: u32) -> u64 {
    let input = unsafe { std::slice::from_raw_parts(ptr as *const u8, len as usize) };
    let response = run_json(input).into_boxed_slice();
    let len = response.len() as u32;
    let ptr = Box::into_raw(response) as *mut u8 as u32;
    ((len as u64) << 32) | (ptr as u64)
}
