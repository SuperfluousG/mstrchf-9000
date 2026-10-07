// Generated from mstrchf-contracts; run cargo run --locked -p mstrchf-contracts --bin generate-contracts
// Do not edit.

export type Action = "setup" | "plan" | "add_adapt_recipe" | "prepare_shop" | "finish_week" | "check_booking";

export type ActionScope = { "kind": "plan", "data": { mode: PlanMode, slots: Array<SlotId>, } } | { "kind": "recipe", "data": { source_ids: Array<EvidenceId>, } } | { "kind": "shop", "data": { key: BasketKey, phase: ShopPhase, approval_id: ApprovalId | null, read_scope: ReadScope, } } | { "kind": "calendar", "data": { destination_id: CalendarDestinationId, event_ids: Array<CalendarEventId>, approval_id: ApprovalId, read_scope: ReadScope, } } | { "kind": "booking", "data": { week_ids: Array<WeekId>, read_scope: ReadScope, } } | { "kind": "setup", "data": { provider: Provider, } };

export type AgeBand = "adult" | "teen" | "child_1_3" | "child_4_6" | "child_7_10" | "child_11_12";

export type AiFeedback = { recipe: RecipeRef, rating: FeedbackRating, reasons: Array<FeedbackReason>, };

export type AiHistory = { selected_recipe_refs: Array<RecipeRef>, ordered_recipe_refs: Array<RecipeRef>, hidden_recipe_ids: Array<RecipeId>, feedback: Array<AiFeedback>, };

export type AiPreferences = { spending_style: SpendingStyle, higher_protein: boolean, more_variety: boolean, quick_meals: boolean, whole_food_preference: boolean, preferred_hands_on_minutes: number, };

export type AllergenCode = "celery" | "cereals_containing_gluten" | "crustaceans" | "eggs" | "fish" | "lupin" | "milk" | "molluscs" | "mustard" | "peanuts" | "sesame" | "soybeans" | "sulphur_dioxide_sulphites" | "tree_nuts";

export type Allocation = { allocation_id: AllocationId, batch_id: BatchId, component_id: ComponentId, target_slot: SlotId, person: PersonRef, servings: PositiveRational, quantity: Quantity, use_at: ZonedMoment, planned_reheats: number, remix_recipe: RecipeRef | null, };

export type AllocationId = string & { readonly __brand: "AllocationId" };

export type AppCommand = { "kind": "save_household_draft", "data": { draft: HouseholdDraft, restriction_issue_edits: Array<SetupRestrictionEdit>, } } | { "kind": "update_profile", "data": { profile: HouseholdProfile, expected_profile_revision: Revision, } } | { "kind": "set_privacy", "data": { policy: PrivacyPolicy, expected_policy_revision: Revision, } } | { "kind": "save_week", "data": { week: WeekEdit, expected_week_revision: Revision, } } | { "kind": "apply_plan_proposal", "data": { proposal: PlanProposal, context_stamp: ContextStamp, } } | { "kind": "confirm_essentials", "data": { confirmations: Array<EssentialsConfirmation>, } } | { "kind": "approve_plan", "data": { week_id: WeekId, plan_revision: Revision, derived_digest: PayloadDigest, } } | { "kind": "confirm_order", "data": { confirmation: OrderConfirmation, } } | { "kind": "request_job", "data": { action: Action, scope: ActionScope, } } | { "kind": "cancel_job", "data": { job_id: JobId, } } | { "kind": "record_user_stop", "data": { worker: WorkerIdentity, qualification_id: EvidenceId, procedure_version: Revision, visible_status: VisibleTerminalStatus, } } | { "kind": "request_calendar_publication", "data": { order_id: OrderId, destination_id: CalendarDestinationId, event_batch_digest: Digest, } };

export type AppCommandV1 = { schema_version: SchemaVersion, command_id: CommandId, expected_snapshot_revision: Revision, command: AppCommand, };

export type ApprovalId = string & { readonly __brand: "ApprovalId" };

export type ApprovedContributionRef = { approval_id: ApprovalId, derived_digest: PayloadDigest, product_evidence_digest: Digest, price_approval_digest: Digest, lines: Array<ApprovedProductLine>, };

export type ApprovedPlan = { approval_id: ApprovalId, week_id: WeekId, plan_revision: Revision, derived_digest: PayloadDigest, context_stamp: ContextStamp, approved_at: Instant, };

export type ApprovedProductLine = { product: ProductRef, expected_quantity: number, target_quantity: number, removal_approved: boolean, planner_attribution_digest: Digest | null, };

export type ArtefactId = string & { readonly __brand: "ArtefactId" };

export type ArtefactMediaType = "application/json";

export type ArtefactRef = { artefact_id: ArtefactId, relative_name: Text, digest: Digest, byte_length: number, media_type: ArtefactMediaType, };

export type AssetDigest = string & { readonly __brand: "AssetDigest" };

export type AssetId = string & { readonly __brand: "AssetId" };

export type AttendeeRequirement = { person: PersonRef, age_band: AgeBand, hard_requirements: Array<HardRequirement>, portion: PortionPreference, };

export type Availability = "in_stock" | "out_of_stock" | "unknown";

export type BasketId = string & { readonly __brand: "BasketId" };

export type BasketKey = { retailer: Retailer, account_ref: Text, basket_id: BasketId, };

export type BasketLine = { product: ProductRef, quantity: number, unit_price: Price, };

export type BasketSnapshot = { key: BasketKey, delivery_context: RetailerDeliveryContext, observed_at: Instant, lines: Array<BasketLine>, delivery_charge: Price, };

export type BatchId = string & { readonly __brand: "BatchId" };

export type BatchLink = { batch_id: BatchId, source_slot: SlotId, target_slot: SlotId, components: Array<ComponentAllocation>, };

export type BatchUse = { "kind": "new_batch", "data": { batch_id: BatchId, production_servings: PositiveRational, } } | { "kind": "leftover", "data": { batch_id: BatchId, component_ids: Array<ComponentId>, } };

export type BoundedCount = number;

export type CalendarDestinationId = string & { readonly __brand: "CalendarDestinationId" };

export type CalendarEvent = { event_id: CalendarEventId, logical_key: CalendarEventLogicalKey, payload_revision: Revision, start: ZonedMoment, end: ZonedMoment, title: Title, body: LongText, event_payload_digest: PayloadDigest, };

export type CalendarEventId = string & { readonly __brand: "CalendarEventId" };

export type CalendarEventKind = "cooking" | "prep" | "delivery";

export type CalendarEventLogicalKey = { week_id: WeekId, kind: CalendarEventKind, session_id: PrepSessionId | null, order_id: OrderId | null, };

export type CalendarObservation = { event_id: CalendarEventId, provider_event_ref: Text | null, observed_payload_digest: Digest | null, state: CalendarObservationState, observed_at: Instant, user_changed: boolean, };

export type CalendarObservationState = "present" | "absent" | "unknown";

export type CandidateIngredient = { ingredient_id: IngredientId | null, name: Text, quantity: Quantity | null, component_id: ComponentId | null, source_text: Text, };

export type CandidateIssue = { code: CandidateIssueCode, path: Text, evidence_ids: Array<EvidenceId>, };

export type CandidateIssueCode = "missing_quantity" | "missing_yield" | "missing_rights" | "missing_image" | "missing_storage" | "missing_portion_evidence" | "unresolved_ingredient" | "inconsistent_instructions" | "unsupported_requirement";

export type Capability = "scoped_file_read" | "atomic_result_write" | "native_launch" | "browser_read" | "browser_mutate" | "calendar_read" | "calendar_write" | "worker_stop";

export type CapabilityClaim = { capability: Capability, claimed_available: boolean, detail_code: CapabilityDetail, };

export type CapabilityDetail = "available" | "unavailable" | "permission_needed" | "unknown";

export type Cell = { slot: SlotId, choice: MealLocation, attendee_ids: Array<DinerId>, occasion: Occasion | null, };

export type CellDefault = { weekday: Weekday, meal_type: MealType, choice: MealLocation, attendee_ids: Array<DinerId>, };

export type CommandId = string & { readonly __brand: "CommandId" };

export type ComponentAllocation = { component_id: ComponentId, quantity: Quantity, };

export type ComponentId = string & { readonly __brand: "ComponentId" };

export type ComponentProductionSpec = { "kind": "cooked", "data": { completion_prep_id: number, } } | { "kind": "unheated", "data": { first_handling_prep_id: number, } };

export type ComponentSpec = { component_id: ComponentId, title: Title, ingredient_ids: Array<IngredientId>, storage_rule_ids: Array<EvidenceId>, production: ComponentProductionSpec | null, };

export type ComponentYield = { component_id: ComponentId, quantity: Quantity, };

export type ConfirmedOrder = { order_id: OrderId, confirmation: OrderConfirmation, approved_plan: ApprovedPlan, ordered_recipe_refs: Array<RecipeRef>, };

export type Conflict = { code: ErrorCode, week_ids: Array<WeekId>, slot_ids: Array<SlotId>, recipe_refs: Array<RecipeRef>, evidence_ids: Array<EvidenceId>, blocking: boolean, };

export type Connections = { harness: Provider | null, retailer: Retailer | null, calendar_destination: CalendarDestinationId | null, loyalty: LoyaltyStatus, delivery_preferences: Array<DeliveryPreference>, privacy: PrivacyPolicy, reminders: ReminderSettings, };

export type ContextStamp = { household_revision: Revision, settings_revision: Revision, privacy_revision: Revision, history_revision: Revision, catalogue_revision: Revision, ranking_policy_version: Revision, recipe_refs: Array<RecipeRef>, read_weeks: Array<WeekRevision>, write_weeks: Array<WeekRevision>, };

export type ConversionEvidence = { evidence_id: EvidenceId, ingredient_id: IngredientId, from_form: IngredientForm, to_form: IngredientForm, from_unit: Unit, to_unit: Unit, factor: PositiveRational, source: SourceRecord, };

export type Currency = "GBP";

export type DecimalU64 = string;

export type DeliveryApplicable = "yes" | "no" | "unknown";

export type DeliveryPreference = { weekday: Weekday, earliest: LocalTime, latest: LocalTime, };

export type DemandDigest = string & { readonly __brand: "DemandDigest" };

export type DemandLine = { line_id: DemandLineId, ingredient_id: IngredientId, quantity: Quantity, origin_batches: Array<BatchId>, origin_extras: Array<ExtraId>, essential: boolean, conversion_evidence_ids: Array<EvidenceId>, };

export type DemandLineId = string & { readonly __brand: "DemandLineId" };

export type DemandSet = { input_plan_digest: PlanInputDigest, lines: Array<DemandLine>, digest: DemandDigest, };

export type DerivedBatch = { batch_id: BatchId, recipe: RecipeRef, source_slot: SlotId, components: Array<DerivedComponent>, };

export type DerivedComponent = { component_id: ComponentId, produced: Quantity, allocated: Quantity, unallocated: NonnegativeQuantity, original_production: ProductionMoment, storage_rule: StorageRule | null, };

export type DerivedPlan = { schema_version: SchemaVersion, week_id: WeekId, plan_revision: Revision, input_plan_digest: PlanInputDigest, food_inputs_digest: PayloadDigest, context_stamp: ContextStamp, recipe_refs: Array<RecipeRef>, batches: Array<DerivedBatch>, allocations: Array<Allocation>, prep_sessions: Array<PrepSession>, demand: DemandSet, warnings: Array<Conflict>, derivation_policy_version: Revision, digest: PayloadDigest, };

export type DietCode = "vegetarian" | "vegan" | "gluten_free";

export type DietaryEvidence = { evidence_id: EvidenceId, requirement: HardRequirement, verdict: DietaryVerdict, source: SourceRecord, review_id: EvidenceId, };

export type DietaryVerdict = "supported" | "conflicting" | "unknown";

export type Digest = string & { readonly __brand: "Digest" };

export type Diner = { diner_id: DinerId, display_name: NameText | null, age_band: AgeBand, restrictions: RestrictionDeclaration, dislikes: Array<IngredientId>, portions: PortionPreference, };

export type DinerId = string & { readonly __brand: "DinerId" };

export type DinerServing = { person: PersonRef, preset_ref: PortionPresetRef, servings: PositiveRational, };

export type DomainError = { code: ErrorCode, issues: Array<ValidationIssue>, retry: Retry, truncated: boolean, };

export type EquipmentCode = "hob" | "oven" | "microwave" | "fridge" | "kettle" | "toaster" | "blender" | "slow_cooker" | "pressure_cooker";

export type ErrorCode = "malformed_json" | "duplicate_key" | "limit_exceeded" | "unsupported_version" | "unknown_field" | "missing_field" | "invalid_value" | "missing_reference" | "foreign_reference" | "noncanonical_value" | "arithmetic_overflow" | "context_stale" | "identity_mismatch" | "digest_mismatch" | "replay_conflict" | "scope_violation" | "privacy_denied" | "authority_denied" | "unsupported_capability" | "domain_conflict" | "worker_not_stopped" | "reconciliation_required" | "operation_uncertain" | "storage_failure";

export type EssentialsChoice = "have_enough" | "add_to_shop";

export type EssentialsConfirmation = { line_id: DemandLineId, choice: EssentialsChoice, displayed_quantity: Quantity, demand_digest: DemandDigest, confirmed_at: Instant, };

export type EventBatch = { week_id: WeekId, order_id: OrderId, approved_plan_digest: PayloadDigest, destination_id: CalendarDestinationId, events: Array<CalendarEvent>, };

export type EventBatchDocumentV1 = { schema_version: SchemaVersion, event_batch: EventBatch, };

export type EvidenceId = string & { readonly __brand: "EvidenceId" };

export type ExternalAction = { "kind": "basket_set_quantity", "data": { product_ref: ProductRef, expected_observed_quantity: number, target_quantity: number, planner_delta: number, product_evidence_digest: Digest, price_approval_digest: Digest, baseline_digest: Digest, } } | { "kind": "hold_slot", "data": { slot_ref: Text, expires_at: Instant, nonfinancial_evidence_digest: Digest, } } | { "kind": "calendar_create", "data": { event_id: CalendarEventId, payload_digest: PayloadDigest, } } | { "kind": "calendar_update", "data": { event_id: CalendarEventId, provider_event_ref: Text, expected_observed_digest: Digest, payload_digest: PayloadDigest, } } | { "kind": "calendar_cancel", "data": { event_id: CalendarEventId, provider_event_ref: Text, expected_observed_digest: Digest, } };

export type ExternalResource = { "kind": "basket", "data": { key: BasketKey, } } | { "kind": "calendar", "data": { destination_id: CalendarDestinationId, } };

export type ExtraId = string & { readonly __brand: "ExtraId" };

export type ExtraSelection = { extra_id: ExtraId, recipe: RecipeRef, recipients: Array<DinerServing>, };

export type ExtraSpec = { extra_id: ExtraId, title: Title, recipe: RecipeRef, eligible_person_requirements: Array<HardRequirement>, supported_with: Array<ComponentId>, };

export type Fact<T> = { "kind": "known", "data": { value: T, } } | { "kind": "unknown", "data": { reason: UnknownReason, } } | { "kind": "conflicting", "data": { values: Array<T>, } };

export type Feedback = { slot: SlotId, recipe: RecipeRef, rating: FeedbackRating, reasons: Array<FeedbackReason>, hide: boolean, portion_change: PersonPortionOverride | null, portion_change_accepted: boolean, recorded_at: Instant, };

export type FeedbackRating = "liked" | "okay" | "not_for_me";

export type FeedbackReason = "too_spicy" | "too_much" | "too_little" | "too_much_effort";

export type FinancialCommitment = "none" | "unknown";

export type Generation = string;

export type GrantToken = string & { readonly __brand: "GrantToken" };

export type Guest = { guest_id: GuestId, age_band: AgeBand, restrictions: RestrictionDeclaration, portion: PortionPreference, };

export type GuestId = string & { readonly __brand: "GuestId" };

export type HardRequirement = { "kind": "allergen_exclusion", "data": { code: AllergenCode, } } | { "kind": "diet", "data": { code: DietCode, } } | { "kind": "ingredient_exclusion", "data": { ingredient_id: IngredientId, } };

export type HouseholdDraft = { schema_version: SchemaVersion, household_id: HouseholdId, revision: Revision, completed_stages: Array<HouseholdStage>, unresolved_restrictions: Array<SetupRestrictionIssue>, diners: Array<Diner>, preferences: Preferences | null, usual_week: UsualWeek | null, connections: Connections | null, };

export type HouseholdId = string & { readonly __brand: "HouseholdId" };

export type HouseholdProfile = { schema_version: SchemaVersion, household_id: HouseholdId, revision: Revision, diners: Array<Diner>, preferences: Preferences, usual_week: UsualWeek, connections: Connections, };

export type HouseholdStage = "household" | "food_preferences" | "usual_week" | "connections";

export type HttpUrl = string & { readonly __brand: "HttpUrl" };

export type ImageRecord = { asset_id: AssetId, digest: AssetDigest, media_type: RecipeImageMediaType, alt_text: Text, kind: RecipeImageKind, source: SourceRecord, rights: RightsRecord, modified: boolean, };

export type IngredientForm = "raw" | "dry" | "cooked" | "drained" | "whole" | "prepared";

export type IngredientId = string & { readonly __brand: "IngredientId" };

export type InputDigest = string & { readonly __brand: "InputDigest" };

export type Instant = string;

export type InstructionStep = { step_id: number, text: Text, ingredient_ids: Array<IngredientId>, component_ids: Array<ComponentId>, active_minutes: number, elapsed_minutes: number, depends_on: Array<number>, };

export type JobId = string & { readonly __brand: "JobId" };

export type JobPayload = { "kind": "personalised_plan", "data": { requirements: Array<MealRequirements>, recipe_refs: Array<RecipeRef>, preferences: AiPreferences, history: AiHistory, other_weeks: Array<OtherWeekSummary>, } } | { "kind": "generic_catalogue", "data": { public_recipes: Array<RecipeRevision>, meal_types: Array<MealType>, } } | { "kind": "recipe_candidate", "data": { source_id: EvidenceId, source: SourceRecord, source_text: LongText, adaptation: RecipeAdaptation | null, } } | { "kind": "product_observation", "data": { read_scope: ReadScope, products: Array<ProductRef>, approved_contribution: ApprovedContributionRef | null, } } | { "kind": "calendar_publication", "data": { batch: EventBatch, read_scope: ReadScope, } } | { "kind": "booking_check", "data": { week_ids: Array<WeekId>, read_scope: ReadScope, } } | { "kind": "setup_check", "data": { provider: Provider, requested_capabilities: Array<Capability>, } };

export type JobRequestV1 = { schema_version: SchemaVersion, job_id: JobId, action: Action, week_id: WeekId | null, base_plan_revision: Revision | null, context_stamp: ContextStamp, input_digest: InputDigest, privacy_policy_revision: Revision, required_capabilities: Array<Capability>, allowed_artefacts: Array<ArtefactRef>, payload: JobPayload, scope: ActionScope, created_at: Instant, expires_at: Instant, };

export type JobResultV1 = { schema_version: SchemaVersion, result_id: ResultId, job_id: JobId, action: Action, week_id: WeekId | null, base_plan_revision: Revision | null, input_digest: InputDigest, context_stamp: ContextStamp, privacy_policy_revision: Revision, status: ResultStatus, proposal: ResultProposal, observations: Array<ResultObservation>, unresolved: Array<UnresolvedLine>, error: DomainError | null, completed_at: Instant, };

export type JobState = "prepared" | "awaiting_user" | "running" | "needs_user" | "succeeded" | "failed" | "cancelled";

export type JobSummary = { job_id: JobId, action: Action, state: JobState, reason: ErrorCode | null, next_action: RequiredAction | null, };

export type LocalDate = string;

export type LocalTime = string;

export type LongText = string;

export type LoyaltyStatus = "confirmed_member" | "not_member" | "unknown";

export type MealLocation = "at_home" | "on_the_move" | "skip";

export type MealOverrides = { portions: Array<PersonPortionOverride>, eat_by: ZonedMoment | null, };

export type MealPortionOverride = { meal_type: MealType, preset: PortionPresetName, adjustment: PositiveRational, };

export type MealRequirements = { slot: SlotId, enabled: boolean, location: MealLocation, attendees: Array<AttendeeRequirement>, shared_hard_requirements: Array<HardRequirement>, equipment: Array<EquipmentCode>, mobility: Mobility, eat_by: ZonedMoment | null, preferred_hands_on_minutes: number, leftovers_allowed: boolean, };

export type MealRequirementsDocumentV1 = { schema_version: SchemaVersion, meal_requirements: MealRequirements, };

export type MealSelection = { slot: SlotId, recipe: RecipeRef, origin: SelectionOrigin, repeat_authorised: boolean, servings: Array<DinerServing>, extras: Array<ExtraSelection>, batch_use: BatchUse, meal_overrides: MealOverrides, };

export type MealTime = { meal_type: MealType, time: LocalTime, };

export type MealType = "breakfast" | "lunch" | "dinner";

export type MealTypeEnabled = { breakfast: boolean, lunch: boolean, dinner: boolean, };

export type Mobility = { reheating: boolean, cold_storage: boolean, temperature: MobilityTemperature, };

export type MobilityTemperature = "hot" | "cold" | "either";

export type NameText = string;

export type NativeTerminalStatus = "cancelled" | "completed" | "failed";

export type NonnegativeQuantity = { amount: Rational, unit: Unit, form: IngredientForm, };

export type NutritionEvidence = { source: SourceRecord, per_servings: PositiveRational, protein_g: Rational | null, energy_kcal: Rational | null, estimated: boolean, };

export type ObservationRef = { operation_id: OperationId, digest: Digest, };

export type Occasion = { occasion_id: OccasionId, guests: Array<Guest>, host_diner_ids: Array<DinerId>, };

export type OccasionId = string & { readonly __brand: "OccasionId" };

export type OfferKind = "unit_price" | "multibuy";

export type OfferObservation = { offer_ref: Text, kind: OfferKind, minimum_packs: BoundedCount, total_pence: DecimalU64, loyalty_required: boolean, valid_from: LocalDate, valid_to: LocalDate, delivery_applicable: DeliveryApplicable, };

export type OperationEvidence = { "kind": "basket", "data": { live_quantity: number, observed_product_ref: ProductRef, basket_digest: Digest, evidence_id: EvidenceId, } } | { "kind": "calendar", "data": { provider_event_ref: Text | null, logical_event_id: CalendarEventId, observed_payload_digest: Digest | null, evidence_id: EvidenceId, } } | { "kind": "slot", "data": { slot_ref: Text, hold_expires_at: Instant | null, financial_commitment: FinancialCommitment, evidence_id: EvidenceId, } };

export type OperationGrant = { schema_version: SchemaVersion, operation_id: OperationId, worker: WorkerIdentity, resource: ExternalResource, context_stamp: ContextStamp, input_digest: InputDigest, approval_id: ApprovalId, action: ExternalAction, grant_token: GrantToken, granted_at: Instant, expires_at: Instant, };

export type OperationId = string & { readonly __brand: "OperationId" };

export type OperationObservation = { schema_version: SchemaVersion, operation_id: OperationId, worker: WorkerIdentity, grant_token: GrantToken, observed_at: Instant, outcome: OperationOutcome, evidence: OperationEvidence, };

export type OperationOutcome = "observed_success" | "observed_failure" | "uncertain";

export type OperationRequest = { schema_version: SchemaVersion, operation_id: OperationId, worker: WorkerIdentity, resource: ExternalResource, context_stamp: ContextStamp, input_digest: InputDigest, approval_id: ApprovalId, action: ExternalAction, requested_at: Instant, };

export type OrderConfirmation = { week_id: WeekId, approval_id: ApprovalId, delivery_start: ZonedMoment, delivery_end: ZonedMoment, confirmed_at: Instant, source: OrderConfirmationSource, };

export type OrderConfirmationSource = { "kind": "user_confirmation", "data": Record<symbol, never> } | { "kind": "authorised_read", "data": { evidence_id: EvidenceId, } };

export type OrderId = string & { readonly __brand: "OrderId" };

export type OrderedHistory = { selection: SelectionHistory, order_id: OrderId, ordered_at: Instant, };

export type OtherSelection = { slot: SlotId, recipe: RecipeRef, origin: SelectionOrigin, repeat_authorised: boolean, leftover: boolean, };

export type OtherWeekSummary = { week_id: WeekId, revision: Revision, selections: Array<OtherSelection>, };

export type PackObservation = { count: Fact<BoundedCount>, net: Fact<Quantity>, drained: Fact<Quantity>, };

export type PayloadDigest = string & { readonly __brand: "PayloadDigest" };

export type PersonPortionOverride = { person: PersonRef, preset: PortionPresetName, adjustment: PositiveRational, };

export type PersonRef = { "kind": "diner", "data": { diner_id: DinerId, } } | { "kind": "guest", "data": { occasion_id: OccasionId, guest_id: GuestId, } };

export type PlanInputDigest = string & { readonly __brand: "PlanInputDigest" };

export type PlanMode = "fill_open" | "swap_named" | "replace_suggestions";

export type PlanProposal = { changes: Array<WeekChange>, };

export type PlanningContext = { schema_version: SchemaVersion, household: HouseholdProfile, settings_revision: Revision, privacy: PrivacyPolicy, requirements: Array<MealRequirements>, weeks: Array<Week>, recipes: Array<RecipeRevision>, history: PlanningHistory, stamp: ContextStamp, ranking_policy: RankingPolicyRef, };

export type PlanningHistory = { revision: Revision, selected: Array<SelectionHistory>, ordered: Array<OrderedHistory>, feedback: Array<Feedback>, hidden_recipe_ids: Array<RecipeId>, };

export type PortabilityEvidence = { cold_suitable: boolean, reheating_required: boolean, cold_storage_required: boolean, evidence_ids: Array<EvidenceId>, };

export type PortionPreference = { preset: PortionPresetName, adjustment: PositiveRational, meal_overrides: Array<MealPortionOverride>, };

export type PortionPreset = { preset: PortionPresetName, age_band: AgeBand, meal_type: MealType, servings: PositiveRational, evidence_ids: Array<EvidenceId>, };

export type PortionPresetName = "smaller" | "regular" | "larger";

export type PortionPresetRef = { recipe: RecipeRef, preset: PortionPresetName, age_band: AgeBand, meal_type: MealType, };

export type PositiveRational = Rational;

export type Preferences = { spending_style: SpendingStyle, higher_protein: boolean, more_variety: boolean, quick_meals: boolean, whole_food_preference: boolean, dislikes: Array<IngredientId>, equipment: Array<EquipmentCode>, preferred_hands_on_minutes: number, meal_times: Array<MealTime>, mobility: Mobility, };

export type PrepSession = { session_id: PrepSessionId, week_id: WeekId, linked_slots: Array<SlotId>, attached_to: PrepSessionId | null, start: ZonedMoment, end: ZonedMoment, ingredient_needed_at: ZonedMoment, tasks: Array<ScheduledPrepTask>, ingredient_ids: Array<IngredientId>, };

export type PrepSessionId = string & { readonly __brand: "PrepSessionId" };

export type PrepSpec = { prep_id: number, title: Title, step_ids: Array<number>, depends_on: Array<number>, active_minutes: number, elapsed_minutes: number, earliest_offset_seconds: number, latest_offset_seconds: number, ingredient_ids: Array<IngredientId>, can_attach_to_prior_session: boolean, };

export type PrepTaskRef = { session_id: PrepSessionId, task_id: number, };

export type Price = { "kind": "observed", "data": { pence: DecimalU64, currency: Currency, } } | { "kind": "estimated", "data": { pence: DecimalU64, currency: Currency, basis: Text, } } | { "kind": "unknown", "data": { reason: UnknownReason, } };

export type PrivacyPolicy = { revision: Revision, use_ai_features: boolean, include_household_requirements: boolean, confirmed_at: Instant | null, };

export type ProductObservation = { product: ProductRef, url: HttpUrl, title: Text, pack: PackObservation, price: Price, offers: Array<OfferObservation>, availability: Availability, ingredients: Fact<Text>, allergens: Fact<Text>, storage: Fact<Text>, delivery_context: RetailerDeliveryContext, observed_at: Instant, };

export type ProductRef = { retailer: Retailer, product_id: Text, variant_id: Text | null, };

export type ProductionMoment = { "kind": "cooked", "data": { completed_at: ZonedMoment, } } | { "kind": "unheated", "data": { preparation_started_at: ZonedMoment, } };

export type ProposedSelection = { slot: SlotId, selection: MealSelection | null, };

export type Provider = "codex_local" | "claude_cowork";

export type PublicationState = "not_requested" | "prepared" | "exported_manual_import_required" | "publishing" | "published" | "needs_user" | "failed";

export type Quantity = { amount: PositiveRational, unit: Unit, form: IngredientForm, };

export type RankingPolicyRef = { version: Revision, digest: PayloadDigest, };

export type Rational = { numerator: DecimalU64, denominator: DecimalU64, };

export type ReadActor = { provider: Provider, native_task_ref: Text, };

export type ReadBody = { "kind": "products", "data": { delivery_context: RetailerDeliveryContext, items: Array<ProductObservation>, } } | { "kind": "basket", "data": { snapshot: BasketSnapshot, } } | { "kind": "calendar_events", "data": { items: Array<CalendarObservation>, } } | { "kind": "slots", "data": { delivery_context: RetailerDeliveryContext, items: Array<SlotObservation>, } };

export type ReadId = string & { readonly __brand: "ReadId" };

export type ReadKind = "products" | "basket" | "calendar_events" | "slots";

export type ReadObservationId = string & { readonly __brand: "ReadObservationId" };

export type ReadObservationV1 = { schema_version: SchemaVersion, observation_id: ReadObservationId, read_id: ReadId, job_id: JobId, input_digest: InputDigest, reader: ReadActor, resource: ExternalResource, observed_at: Instant, body: ReadBody, };

export type ReadPurpose = "initial" | "reconciliation" | "verification" | "slot_discovery";

export type ReadScope = { read_id: ReadId, resource: ExternalResource, purpose: ReadPurpose, allowed_kinds: Array<ReadKind>, products: Array<ProductRef>, event_ids: Array<CalendarEventId>, not_before: Instant, expires_at: Instant, };

export type RecipeAdaptation = { requirements: Array<HardRequirement>, equipment: Array<EquipmentCode>, portion_requests: Array<PortionPresetRef>, };

export type RecipeCandidate = { schema_version: SchemaVersion, candidate_id: RecipeId, proposed_revision: Revision, title: Title, source: SourceRecord, ingredients: Array<CandidateIngredient>, yield: YieldSpec | null, steps: Array<InstructionStep>, unresolved: Array<CandidateIssue>, image: ImageRecord | null, };

export type RecipeCapabilities = { local_display: boolean, shopping: boolean, leftovers: boolean, starter_distribution: boolean, blocking_issues: Array<CandidateIssue>, };

export type RecipeDigest = string & { readonly __brand: "RecipeDigest" };

export type RecipeId = string & { readonly __brand: "RecipeId" };

export type RecipeImageKind = "photograph" | "illustration" | "generated_illustration";

export type RecipeImageMediaType = "image/jpeg" | "image/png" | "image/webp";

export type RecipeIngredient = { ingredient_id: IngredientId, name: NameText, quantity: Quantity | null, discrete: boolean, conversion_evidence: Array<ConversionEvidence>, component_id: ComponentId, step_ids: Array<number>, };

export type RecipeRef = { recipe_id: RecipeId, revision: Revision, digest: RecipeDigest, };

export type RecipeRevision = { schema_version: SchemaVersion, recipe_id: RecipeId, revision: Revision, title: Title, source: SourceRecord, text_rights: RightsRecord, ingredients: Array<RecipeIngredient>, yield: YieldSpec, scaling: ScalingPolicy, steps: Array<InstructionStep>, equipment: Array<EquipmentCode>, active_minutes: number, elapsed_minutes: number, meal_types: Array<MealType>, portability: PortabilityEvidence, components: Array<ComponentSpec>, extras: Array<ExtraSpec>, reused_inputs: Array<ReusedComponentInput>, dietary_evidence: Array<DietaryEvidence>, storage: Array<StorageRule>, prep: Array<PrepSpec>, portion_presets: Array<PortionPreset>, nutrition: NutritionEvidence | null, image: ImageRecord | null, reviews: Array<ReviewRecord>, capabilities: RecipeCapabilities, };

export type RecipeSourceOrigin = "published" | "user_authored" | "adapted";

export type ReminderChannel = "local" | "calendar";

export type ReminderPrompt = "unfinished_plan" | "booking_opportunity";

export type ReminderSettings = { enabled: boolean, channels: Array<ReminderChannel>, prompts: Array<ReminderPrompt>, };

export type RequiredAction = "choose_alternative" | "inspect_product" | "sign_in" | "stop_worker" | "reconcile" | "retry_read" | "manual_export" | "review_candidate";

export type Resolution = "unambiguous" | "earlier" | "later";

export type RestrictionDeclaration = { "kind": "unconfirmed", "data": Record<symbol, never> } | { "kind": "confirmed_none", "data": { confirmed_at: Instant, } } | { "kind": "confirmed", "data": { confirmed_at: Instant, requirements: Array<HardRequirement>, } };

export type ResultId = string & { readonly __brand: "ResultId" };

export type ResultObservation = { "kind": "read", "data": { observation: ReadObservationV1, } } | { "kind": "operation", "data": { reference: ObservationRef, } };

export type ResultProposal = { "kind": "none", "data": Record<symbol, never> } | { "kind": "plan", "data": { proposal: PlanProposal, } } | { "kind": "recipe", "data": { candidate: RecipeCandidate, } } | { "kind": "catalogue", "data": { recipe_refs: Array<RecipeRef>, } } | { "kind": "setup", "data": { capabilities: Array<CapabilityClaim>, } };

export type ResultStatus = "complete" | "needs_user" | "failed" | "cancelled";

export type Retailer = "tesco";

export type RetailerDeliveryContext = { retailer: Retailer, account_ref: Text, context_ref: Text, delivery_date: LocalDate | null, slot_ref: Text | null, };

export type Retry = "never" | "after_refresh" | "after_user_action" | "read_backoff";

export type ReusedComponentInput = { source_recipe: RecipeRef, source_component_id: ComponentId, quantity_per_recipe_yield: Quantity, step_ids: Array<number>, planned_reheats: number, evidence_ids: Array<EvidenceId>, };

export type ReviewActor = "human" | "automated";

export type ReviewKind = "source_review" | "quantity_review" | "dietary_review" | "cook_test";

export type ReviewRecord = { evidence_id: EvidenceId, kind: ReviewKind, actor: ReviewActor, at: Instant, note: Text, };

export type Revision = string;

export type RightsDistribution = "permitted" | "personal_only" | "unresolved";

export type RightsRecord = { licence_id: Text, licence_url: HttpUrl | null, attribution: Text, adaptations: Array<Text>, distribution: RightsDistribution, evidence_ids: Array<EvidenceId>, };

export type ScalingPolicy = { "kind": "linear", "data": { minimum: PositiveRational, maximum: PositiveRational, } } | { "kind": "discrete_batches", "data": { batch_yield: PositiveRational, maximum_batches: number, } };

export type ScheduledPrepTask = { task_id: number, recipe: RecipeRef, prep_id: number, depends_on: Array<PrepTaskRef>, active_intervals: Array<TimeInterval>, elapsed: TimeInterval, };

export type SchemaVersion = 1;

export type SelectionHistory = { week_id: WeekId, slot: SlotId, recipe: RecipeRef, servings: Array<DinerServing>, extras: Array<ExtraSelection>, repeat_authorised: boolean, selected_at: Instant, };

export type SelectionOrigin = "fixed" | "suggested";

export type SetupRestrictionEdit = { "kind": "replace_text", "data": { diner_id: DinerId, } } | { "kind": "resolve", "data": { diner_id: DinerId, } } | { "kind": "remove_diner", "data": { diner_id: DinerId, } };

export type SetupRestrictionIssue = { diner_id: DinerId, supplied_text: Text, };

export type ShopPhase = "observe" | "reconcile" | "mutate";

export type SlotId = { week_id: WeekId, date: LocalDate, meal_type: MealType, };

export type SlotObservation = { slot_ref: Text, start: ZonedMoment, end: ZonedMoment, available: boolean, charge: Price, hold_expires_at: Instant | null, financial_commitment: FinancialCommitment, };

export type Snapshot = { schema_version: SchemaVersion, snapshot_revision: Revision, privacy: PrivacyPolicy, household_draft: HouseholdDraft, profile: HouseholdProfile | null, weeks: Array<Week>, jobs: Array<JobSummary>, conflicts: Array<Conflict>, };

export type SourceRecord = { url: HttpUrl | null, source_revision: Text | null, retrieved_at: Instant, authors: Array<Text>, adaptation_notice: Text | null, origin: RecipeSourceOrigin, };

export type SpendingStyle = "value" | "balanced" | "flexible";

export type StopSource = { "kind": "native_terminal", "data": { status: NativeTerminalStatus, provider_evidence_id: EvidenceId, observed_at: Instant, } } | { "kind": "user_confirmed", "data": { procedure_version: Revision, visible_status: VisibleTerminalStatus, confirmed_at: Instant, } };

export type StorageRule = { evidence_id: EvidenceId, component_id: ComponentId, max_chilled_seconds: number, cooling_instruction: Text, reheating_instruction: Text, max_reheats: number, source: SourceRecord, };

export type Text = string;

export type TimeInterval = { start: ZonedMoment, end: ZonedMoment, };

export type Title = string;

export type Unit = "g" | "ml" | "each";

export type UnknownReason = "missing" | "unreadable" | "not_exposed";

export type UnresolvedLine = { line_id: DemandLineId | null, code: ErrorCode, required_action: RequiredAction, };

export type UsualWeek = { enabled_meal_types: Array<MealType>, day_enabled: Array<boolean>, cells: Array<CellDefault>, suggest_leftovers: boolean, zone: ZoneId, };

export type ValidationIssue = { path: string, code: ErrorCode, entity_id: Text | null, };

export type VisibleTerminalStatus = "cancelled" | "completed";

export type Week = { schema_version: SchemaVersion, range: WeekRange, revision: Revision, defaults_snapshot: UsualWeek, day_enabled: Array<boolean>, meal_type_enabled: MealTypeEnabled, cells: Array<Cell>, plan: WeekPlan, approved_plan: ApprovedPlan | null, confirmed_order: ConfirmedOrder | null, publication: PublicationState, };

export type WeekChange = { week_id: WeekId, base_revision: Revision, replace_slots: Array<ProposedSelection>, };

export type WeekEdit = { week_id: WeekId, defaults_snapshot: UsualWeek, day_enabled: Array<boolean>, meal_type_enabled: MealTypeEnabled, cells: Array<Cell>, plan: WeekPlan | null, };

export type WeekId = string & { readonly __brand: "WeekId" };

export type WeekPlan = { week_id: WeekId, revision: Revision, selections: Array<MealSelection>, batch_links: Array<BatchLink>, };

export type WeekPlanDocumentV1 = { schema_version: SchemaVersion, week_plan: WeekPlan, };

export type WeekRange = { week_id: WeekId, monday: LocalDate, zone: ZoneId, };

export type WeekRevision = { week_id: WeekId, revision: Revision, };

export type Weekday = "monday" | "tuesday" | "wednesday" | "thursday" | "friday" | "saturday" | "sunday";

export type WorkerIdentity = { provider: Provider, provider_build: Text, native_task_ref: Text, job_id: JobId, generation: Generation, };

export type WorkerStopEvidence = { schema_version: SchemaVersion, worker: WorkerIdentity, qualification_id: EvidenceId, source: StopSource, recorded_at: Instant, };

export type YieldSpec = { servings: PositiveRational, basis: Text, components: Array<ComponentYield>, };

export type ZoneId = string;

export type ZonedMoment = { local_date: LocalDate, local_time: LocalTime, zone: ZoneId, resolution: Resolution, instant: Instant, };
