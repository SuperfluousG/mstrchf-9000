#![allow(non_camel_case_types)] // Names match the frozen schema inventory.
//! Frozen closed graph from 2f6473c0b1d9623fc1d2699910e67b2ab3311c86.
//! Mechanical schema projection; semantic decoding is private and mandatory.
use super::{Error, ReceiptBinding, codec};
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
enum Never {}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum Action {
    #[serde(rename = "setup")]
    Setup,
    #[serde(rename = "plan")]
    Plan,
    #[serde(rename = "add_adapt_recipe")]
    AddAdaptRecipe,
    #[serde(rename = "prepare_shop")]
    PrepareShop,
    #[serde(rename = "finish_week")]
    FinishWeek,
    #[serde(rename = "check_booking")]
    CheckBooking,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum AgeBand {
    #[serde(rename = "adult")]
    Adult,
    #[serde(rename = "teen")]
    Teen,
    #[serde(rename = "child_1_3")]
    Child13,
    #[serde(rename = "child_4_6")]
    Child46,
    #[serde(rename = "child_7_10")]
    Child710,
    #[serde(rename = "child_11_12")]
    Child1112,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum AllergenCode {
    #[serde(rename = "celery")]
    Celery,
    #[serde(rename = "cereals_containing_gluten")]
    CerealsContainingGluten,
    #[serde(rename = "crustaceans")]
    Crustaceans,
    #[serde(rename = "eggs")]
    Eggs,
    #[serde(rename = "fish")]
    Fish,
    #[serde(rename = "lupin")]
    Lupin,
    #[serde(rename = "milk")]
    Milk,
    #[serde(rename = "molluscs")]
    Molluscs,
    #[serde(rename = "mustard")]
    Mustard,
    #[serde(rename = "peanuts")]
    Peanuts,
    #[serde(rename = "sesame")]
    Sesame,
    #[serde(rename = "soybeans")]
    Soybeans,
    #[serde(rename = "sulphur_dioxide_sulphites")]
    SulphurDioxideSulphites,
    #[serde(rename = "tree_nuts")]
    TreeNuts,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "data", deny_unknown_fields)]
pub(super) enum AppCommand {
    #[serde(rename = "save_household_draft")]
    SaveHouseholdDraft { draft: HouseholdDraft },
    #[serde(rename = "set_privacy")]
    SetPrivacy {
        expected_policy_revision: Revision,
        policy: PrivacyPolicy,
    },
    #[serde(rename = "cancel_job")]
    CancelJob { job_id: JobId },
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct AppCommandV1 {
    command: AppCommand,
    command_id: CommandId,
    expected_snapshot_revision: Revision,
    schema_version: SchemaVersion,
}
type Array_CellDefault_21_21 = Vec<CellDefault>;
type Array_DeliveryPreference_0_7 = Vec<DeliveryPreference>;
type Array_DinerId_0_64 = Vec<DinerId>;
type Array_Diner_0_64 = Vec<Diner>;
type Array_Diner_1_64 = Vec<Diner>;
type Array_EquipmentCode_0_9 = Vec<EquipmentCode>;
type Array_HardRequirement_1_64 = Vec<HardRequirement>;
type Array_HouseholdStage_0_4 = Vec<HouseholdStage>;
type Array_IngredientId_0_256 = Vec<IngredientId>;
type Array_JobSummary_0_1024 = Vec<JobSummary>;
type Array_MealPortionOverride_0_3 = Vec<MealPortionOverride>;
type Array_MealTime_3_3 = Vec<MealTime>;
type Array_MealType_0_3 = Vec<MealType>;
type Array_ReminderChannel_0_2 = Vec<ReminderChannel>;
type Array_ReminderPrompt_0_2 = Vec<ReminderPrompt>;
type Array_boolean_7_7 = Vec<bool>;
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub(super) struct CalendarDestinationId(String);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct CellDefault {
    attendee_ids: Array_DinerId_0_64,
    choice: MealLocation,
    meal_type: MealType,
    weekday: Weekday,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub(super) struct CommandId(String);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Connections {
    calendar_destination: Nullable_CalendarDestinationId,
    delivery_preferences: Array_DeliveryPreference_0_7,
    harness: Nullable_Provider,
    loyalty: LoyaltyStatus,
    privacy: PrivacyPolicy,
    reminders: ReminderSettings,
    retailer: Nullable_Retailer,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct DeliveryPreference {
    earliest: LocalTime,
    latest: LocalTime,
    weekday: Weekday,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum DietCode {
    #[serde(rename = "vegetarian")]
    Vegetarian,
    #[serde(rename = "vegan")]
    Vegan,
    #[serde(rename = "gluten_free")]
    GlutenFree,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Diner {
    age_band: AgeBand,
    diner_id: DinerId,
    dislikes: Array_IngredientId_0_256,
    display_name: Nullable_NameText,
    portions: PortionPreference,
    restrictions: RestrictionDeclaration,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub(super) struct DinerId(String);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum EquipmentCode {
    #[serde(rename = "hob")]
    Hob,
    #[serde(rename = "oven")]
    Oven,
    #[serde(rename = "microwave")]
    Microwave,
    #[serde(rename = "fridge")]
    Fridge,
    #[serde(rename = "kettle")]
    Kettle,
    #[serde(rename = "toaster")]
    Toaster,
    #[serde(rename = "blender")]
    Blender,
    #[serde(rename = "slow_cooker")]
    SlowCooker,
    #[serde(rename = "pressure_cooker")]
    PressureCooker,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum ErrorCode {
    #[serde(rename = "malformed_json")]
    MalformedJson,
    #[serde(rename = "duplicate_key")]
    DuplicateKey,
    #[serde(rename = "limit_exceeded")]
    LimitExceeded,
    #[serde(rename = "unsupported_version")]
    UnsupportedVersion,
    #[serde(rename = "unknown_field")]
    UnknownField,
    #[serde(rename = "missing_field")]
    MissingField,
    #[serde(rename = "invalid_value")]
    InvalidValue,
    #[serde(rename = "missing_reference")]
    MissingReference,
    #[serde(rename = "foreign_reference")]
    ForeignReference,
    #[serde(rename = "noncanonical_value")]
    NoncanonicalValue,
    #[serde(rename = "arithmetic_overflow")]
    ArithmeticOverflow,
    #[serde(rename = "context_stale")]
    ContextStale,
    #[serde(rename = "identity_mismatch")]
    IdentityMismatch,
    #[serde(rename = "digest_mismatch")]
    DigestMismatch,
    #[serde(rename = "replay_conflict")]
    ReplayConflict,
    #[serde(rename = "scope_violation")]
    ScopeViolation,
    #[serde(rename = "privacy_denied")]
    PrivacyDenied,
    #[serde(rename = "authority_denied")]
    AuthorityDenied,
    #[serde(rename = "unsupported_capability")]
    UnsupportedCapability,
    #[serde(rename = "domain_conflict")]
    DomainConflict,
    #[serde(rename = "worker_not_stopped")]
    WorkerNotStopped,
    #[serde(rename = "reconciliation_required")]
    ReconciliationRequired,
    #[serde(rename = "operation_uncertain")]
    OperationUncertain,
    #[serde(rename = "storage_failure")]
    StorageFailure,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "data", deny_unknown_fields)]
pub(super) enum HardRequirement {
    #[serde(rename = "allergen_exclusion")]
    AllergenExclusion { code: AllergenCode },
    #[serde(rename = "diet")]
    Diet { code: DietCode },
    #[serde(rename = "ingredient_exclusion")]
    IngredientExclusion { ingredient_id: IngredientId },
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HouseholdDraft {
    completed_stages: Array_HouseholdStage_0_4,
    connections: Nullable_Connections,
    diners: Array_Diner_0_64,
    household_id: HouseholdId,
    preferences: Nullable_Preferences,
    revision: Revision,
    schema_version: SchemaVersion,
    usual_week: Nullable_UsualWeek,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub(super) struct HouseholdId(String);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct HouseholdProfile {
    connections: Connections,
    diners: Array_Diner_1_64,
    household_id: HouseholdId,
    preferences: Preferences,
    revision: Revision,
    schema_version: SchemaVersion,
    usual_week: UsualWeek,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum HouseholdStage {
    #[serde(rename = "household")]
    Household,
    #[serde(rename = "food_preferences")]
    FoodPreferences,
    #[serde(rename = "usual_week")]
    UsualWeek,
    #[serde(rename = "connections")]
    Connections,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub(super) struct IngredientId(String);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub(super) struct Instant(String);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub(super) struct JobId(String);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum JobState {
    #[serde(rename = "prepared")]
    Prepared,
    #[serde(rename = "awaiting_user")]
    AwaitingUser,
    #[serde(rename = "running")]
    Running,
    #[serde(rename = "needs_user")]
    NeedsUser,
    #[serde(rename = "succeeded")]
    Succeeded,
    #[serde(rename = "failed")]
    Failed,
    #[serde(rename = "cancelled")]
    Cancelled,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct JobSummary {
    action: Action,
    job_id: JobId,
    next_action: Nullable_RequiredAction,
    reason: Nullable_ErrorCode,
    state: JobState,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub(super) struct LocalTime(String);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum LoyaltyStatus {
    #[serde(rename = "confirmed_member")]
    ConfirmedMember,
    #[serde(rename = "not_member")]
    NotMember,
    #[serde(rename = "unknown")]
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum MealLocation {
    #[serde(rename = "at_home")]
    AtHome,
    #[serde(rename = "on_the_move")]
    OnTheMove,
    #[serde(rename = "skip")]
    Skip,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MealPortionOverride {
    adjustment: PositiveRational,
    meal_type: MealType,
    preset: PortionPresetName,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct MealTime {
    meal_type: MealType,
    time: LocalTime,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum MealType {
    #[serde(rename = "breakfast")]
    Breakfast,
    #[serde(rename = "lunch")]
    Lunch,
    #[serde(rename = "dinner")]
    Dinner,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Mobility {
    cold_storage: bool,
    reheating: bool,
    temperature: MobilityTemperature,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum MobilityTemperature {
    #[serde(rename = "hot")]
    Hot,
    #[serde(rename = "cold")]
    Cold,
    #[serde(rename = "either")]
    Either,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub(super) struct NameText(String);
type Nullable_CalendarDestinationId = Option<CalendarDestinationId>;
type Nullable_Connections = Option<Connections>;
type Nullable_ErrorCode = Option<ErrorCode>;
type Nullable_HouseholdProfile = Option<HouseholdProfile>;
type Nullable_Instant = Option<Instant>;
type Nullable_NameText = Option<NameText>;
type Nullable_Preferences = Option<Preferences>;
type Nullable_Provider = Option<Provider>;
type Nullable_RequiredAction = Option<RequiredAction>;
type Nullable_Retailer = Option<Retailer>;
type Nullable_UsualWeek = Option<UsualWeek>;
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PortionPreference {
    adjustment: PositiveRational,
    meal_overrides: Array_MealPortionOverride_0_3,
    preset: PortionPresetName,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum PortionPresetName {
    #[serde(rename = "smaller")]
    Smaller,
    #[serde(rename = "regular")]
    Regular,
    #[serde(rename = "larger")]
    Larger,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PositiveRational {
    denominator: String,
    numerator: String,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct Preferences {
    dislikes: Array_IngredientId_0_256,
    equipment: Array_EquipmentCode_0_9,
    higher_protein: bool,
    meal_times: Array_MealTime_3_3,
    mobility: Mobility,
    more_variety: bool,
    preferred_hands_on_minutes: u64,
    quick_meals: bool,
    spending_style: SpendingStyle,
    whole_food_preference: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct PrivacyPolicy {
    confirmed_at: Nullable_Instant,
    include_household_requirements: bool,
    revision: Revision,
    use_ai_features: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum Provider {
    #[serde(rename = "codex_local")]
    CodexLocal,
    #[serde(rename = "claude_cowork")]
    ClaudeCowork,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum ReminderChannel {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "calendar")]
    Calendar,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum ReminderPrompt {
    #[serde(rename = "unfinished_plan")]
    UnfinishedPlan,
    #[serde(rename = "booking_opportunity")]
    BookingOpportunity,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct ReminderSettings {
    channels: Array_ReminderChannel_0_2,
    enabled: bool,
    prompts: Array_ReminderPrompt_0_2,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum RequiredAction {
    #[serde(rename = "choose_alternative")]
    ChooseAlternative,
    #[serde(rename = "inspect_product")]
    InspectProduct,
    #[serde(rename = "sign_in")]
    SignIn,
    #[serde(rename = "stop_worker")]
    StopWorker,
    #[serde(rename = "reconcile")]
    Reconcile,
    #[serde(rename = "retry_read")]
    RetryRead,
    #[serde(rename = "manual_export")]
    ManualExport,
    #[serde(rename = "review_candidate")]
    ReviewCandidate,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "data", deny_unknown_fields)]
pub(super) enum RestrictionDeclaration {
    #[serde(rename = "unconfirmed")]
    Unconfirmed {},
    #[serde(rename = "confirmed_none")]
    ConfirmedNone { confirmed_at: Instant },
    #[serde(rename = "confirmed")]
    Confirmed {
        confirmed_at: Instant,
        requirements: Array_HardRequirement_1_64,
    },
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum Retailer {
    #[serde(rename = "tesco")]
    Tesco,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub(super) struct Revision(String);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub(super) struct SchemaVersion(u64);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct Snapshot {
    conflicts: Vec<Never>,
    household_draft: HouseholdDraft,
    jobs: Array_JobSummary_0_1024,
    privacy: PrivacyPolicy,
    profile: Nullable_HouseholdProfile,
    schema_version: SchemaVersion,
    snapshot_revision: Revision,
    weeks: Vec<Never>,
}
/// Immutable inspected view; deliberately has no Deserialize constructor.
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize)]
#[serde(transparent)]
pub(super) struct B1RetainedV1(Snapshot);
impl B1RetainedV1 {
    pub(super) fn household_draft(&self) -> &HouseholdDraft {
        &self.0.household_draft
    }
    pub(super) fn privacy(&self) -> &PrivacyPolicy {
        &self.0.privacy
    }
    pub(super) fn profile(&self) -> Option<&HouseholdProfile> {
        self.0.profile.as_ref()
    }
    pub(super) fn jobs(&self) -> &[JobSummary] {
        &self.0.jobs
    }
    pub(super) fn snapshot_revision(&self) -> &Revision {
        &self.0.snapshot_revision
    }
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum SpendingStyle {
    #[serde(rename = "value")]
    Value,
    #[serde(rename = "balanced")]
    Balanced,
    #[serde(rename = "flexible")]
    Flexible,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct UsualWeek {
    cells: Array_CellDefault_21_21,
    day_enabled: Array_boolean_7_7,
    enabled_meal_types: Array_MealType_0_3,
    suggest_leftovers: bool,
    zone: ZoneId,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub(super) enum Weekday {
    #[serde(rename = "monday")]
    Monday,
    #[serde(rename = "tuesday")]
    Tuesday,
    #[serde(rename = "wednesday")]
    Wednesday,
    #[serde(rename = "thursday")]
    Thursday,
    #[serde(rename = "friday")]
    Friday,
    #[serde(rename = "saturday")]
    Saturday,
    #[serde(rename = "sunday")]
    Sunday,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub(super) struct ZoneId(String);
const SCHEMA: &str = include_str!("b1_v1.schema.json");
pub(super) fn snapshot(bytes: &[u8]) -> Result<B1RetainedV1, Error> {
    codec::decode(bytes, SCHEMA, "Snapshot").map(B1RetainedV1)
}
pub(super) fn command(bytes: &[u8]) -> Result<AppCommandV1, Error> {
    codec::decode(bytes, SCHEMA, "AppCommandV1")
}
pub(super) fn draft(bytes: &[u8]) -> Result<HouseholdDraft, Error> {
    codec::decode(bytes, SCHEMA, "HouseholdDraft")
}
pub(super) fn profile(bytes: &[u8]) -> Result<HouseholdProfile, Error> {
    codec::decode(bytes, SCHEMA, "HouseholdProfile")
}
pub(super) fn privacy(bytes: &[u8]) -> Result<PrivacyPolicy, Error> {
    codec::decode(bytes, SCHEMA, "PrivacyPolicy")
}
pub(super) fn job(bytes: &[u8]) -> Result<JobSummary, Error> {
    codec::decode(bytes, SCHEMA, "JobSummary")
}

/// A pure pair inspection, not a catalogue/DB audit or replay authority.
#[derive(Debug)]
pub(super) struct Receipt {
    command: AppCommandV1,
    response: B1RetainedV1,
    command_bytes: Vec<u8>,
    response_bytes: Vec<u8>,
    command_digest: String,
    response_digest: String,
}
impl Receipt {
    pub(super) fn original_command_bytes(&self) -> &[u8] {
        &self.command_bytes
    }
    pub(super) fn original_response_bytes(&self) -> &[u8] {
        &self.response_bytes
    }
    pub(super) fn command_digest(&self) -> &str {
        &self.command_digest
    }
    pub(super) fn response_digest(&self) -> &str {
        &self.response_digest
    }
    pub(super) fn response(&self) -> &B1RetainedV1 {
        &self.response
    }
}
pub(super) fn receipt(
    command_bytes: &[u8],
    response_bytes: &[u8],
    binding: &ReceiptBinding<'_>,
) -> Result<Receipt, Error> {
    let command = command(command_bytes)?;
    let response = snapshot(response_bytes)?;
    // Both objects have passed their concrete frozen codec. The common pure
    // predicate never chooses a profile or grants authority from JSON fields.
    codec::receipt_predicates(&command, &response, command_bytes, response_bytes, binding)?;
    Ok(Receipt {
        command,
        response,
        command_bytes: command_bytes.to_vec(),
        response_bytes: response_bytes.to_vec(),
        command_digest: binding.command_digest.into(),
        response_digest: binding.response_digest.into(),
    })
}
