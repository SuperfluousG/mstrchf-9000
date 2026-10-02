#![allow(non_camel_case_types)] // Names match the frozen schema inventory.
//! Frozen closed graph from 4b7854ca76ce7b5b86cc5cd71eba5520dd8e7fc1.
//! Mechanical schema projection; semantic decoding is private and mandatory.
use super::{Error, ReceiptBinding, codec};
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Never {}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Action {
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
pub enum AgeBand {
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
pub enum AllergenCode {
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
// Preserve the independently reviewed frozen value representation.
#[allow(clippy::large_enum_variant)]
pub enum AppCommand {
    #[serde(rename = "save_household_draft")]
    SaveHouseholdDraft {
        draft: HouseholdDraft,
        restriction_issue_edits: Array_SetupRestrictionEdit_0_64,
    },
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
pub struct AppCommandV1 {
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
type Array_SetupRestrictionEdit_0_64 = Vec<SetupRestrictionEdit>;
type Array_SetupRestrictionIssue_0_64 = Vec<SetupRestrictionIssue>;
type Array_boolean_7_7 = Vec<bool>;
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct CalendarDestinationId(String);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CellDefault {
    attendee_ids: Array_DinerId_0_64,
    choice: MealLocation,
    meal_type: MealType,
    weekday: Weekday,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct CommandId(String);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Connections {
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
pub struct DeliveryPreference {
    earliest: LocalTime,
    latest: LocalTime,
    weekday: Weekday,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum DietCode {
    #[serde(rename = "vegetarian")]
    Vegetarian,
    #[serde(rename = "vegan")]
    Vegan,
    #[serde(rename = "gluten_free")]
    GlutenFree,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Diner {
    age_band: AgeBand,
    diner_id: DinerId,
    dislikes: Array_IngredientId_0_256,
    display_name: Nullable_NameText,
    portions: PortionPreference,
    restrictions: RestrictionDeclaration,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct DinerId(String);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum EquipmentCode {
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
pub enum ErrorCode {
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
pub enum HardRequirement {
    #[serde(rename = "allergen_exclusion")]
    AllergenExclusion { code: AllergenCode },
    #[serde(rename = "diet")]
    Diet { code: DietCode },
    #[serde(rename = "ingredient_exclusion")]
    IngredientExclusion { ingredient_id: IngredientId },
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HouseholdDraft {
    completed_stages: Array_HouseholdStage_0_4,
    connections: Nullable_Connections,
    diners: Array_Diner_0_64,
    household_id: HouseholdId,
    preferences: Nullable_Preferences,
    revision: Revision,
    schema_version: SchemaVersion,
    unresolved_restrictions: Array_SetupRestrictionIssue_0_64,
    usual_week: Nullable_UsualWeek,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct HouseholdId(String);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HouseholdProfile {
    connections: Connections,
    diners: Array_Diner_1_64,
    household_id: HouseholdId,
    preferences: Preferences,
    revision: Revision,
    schema_version: SchemaVersion,
    usual_week: UsualWeek,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum HouseholdStage {
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
pub struct IngredientId(String);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Instant(String);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct JobId(String);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum JobState {
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
pub struct JobSummary {
    action: Action,
    job_id: JobId,
    next_action: Nullable_RequiredAction,
    reason: Nullable_ErrorCode,
    state: JobState,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct LocalTime(String);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum LoyaltyStatus {
    #[serde(rename = "confirmed_member")]
    ConfirmedMember,
    #[serde(rename = "not_member")]
    NotMember,
    #[serde(rename = "unknown")]
    Unknown,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MealLocation {
    #[serde(rename = "at_home")]
    AtHome,
    #[serde(rename = "on_the_move")]
    OnTheMove,
    #[serde(rename = "skip")]
    Skip,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MealPortionOverride {
    adjustment: PositiveRational,
    meal_type: MealType,
    preset: PortionPresetName,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct MealTime {
    meal_type: MealType,
    time: LocalTime,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MealType {
    #[serde(rename = "breakfast")]
    Breakfast,
    #[serde(rename = "lunch")]
    Lunch,
    #[serde(rename = "dinner")]
    Dinner,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Mobility {
    cold_storage: bool,
    reheating: bool,
    temperature: MobilityTemperature,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum MobilityTemperature {
    #[serde(rename = "hot")]
    Hot,
    #[serde(rename = "cold")]
    Cold,
    #[serde(rename = "either")]
    Either,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct NameText(String);
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
pub struct PortionPreference {
    adjustment: PositiveRational,
    meal_overrides: Array_MealPortionOverride_0_3,
    preset: PortionPresetName,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum PortionPresetName {
    #[serde(rename = "smaller")]
    Smaller,
    #[serde(rename = "regular")]
    Regular,
    #[serde(rename = "larger")]
    Larger,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct PositiveRational {
    denominator: String,
    numerator: String,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Preferences {
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
pub struct PrivacyPolicy {
    confirmed_at: Nullable_Instant,
    include_household_requirements: bool,
    revision: Revision,
    use_ai_features: bool,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Provider {
    #[serde(rename = "codex_local")]
    CodexLocal,
    #[serde(rename = "claude_cowork")]
    ClaudeCowork,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ReminderChannel {
    #[serde(rename = "local")]
    Local,
    #[serde(rename = "calendar")]
    Calendar,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum ReminderPrompt {
    #[serde(rename = "unfinished_plan")]
    UnfinishedPlan,
    #[serde(rename = "booking_opportunity")]
    BookingOpportunity,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReminderSettings {
    channels: Array_ReminderChannel_0_2,
    enabled: bool,
    prompts: Array_ReminderPrompt_0_2,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum RequiredAction {
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
pub enum RestrictionDeclaration {
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
pub enum Retailer {
    #[serde(rename = "tesco")]
    Tesco,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Revision(String);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct SchemaVersion(u64);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(tag = "kind", content = "data", deny_unknown_fields)]
pub enum SetupRestrictionEdit {
    #[serde(rename = "replace_text")]
    ReplaceText { diner_id: DinerId },
    #[serde(rename = "resolve")]
    Resolve { diner_id: DinerId },
    #[serde(rename = "remove_diner")]
    RemoveDiner { diner_id: DinerId },
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct SetupRestrictionIssue {
    diner_id: DinerId,
    supplied_text: Text,
}
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
pub struct B1Draft005(Snapshot);
impl B1Draft005 {
    pub fn schema_version(&self) -> &SchemaVersion {
        &self.0.schema_version
    }
    pub fn weeks(&self) -> &[Never] {
        &self.0.weeks
    }
    pub fn conflicts(&self) -> &[Never] {
        &self.0.conflicts
    }
    pub fn household_draft(&self) -> &HouseholdDraft {
        &self.0.household_draft
    }
    pub fn privacy(&self) -> &PrivacyPolicy {
        &self.0.privacy
    }
    pub fn profile(&self) -> Option<&HouseholdProfile> {
        self.0.profile.as_ref()
    }
    pub fn jobs(&self) -> &[JobSummary] {
        &self.0.jobs
    }
    pub fn snapshot_revision(&self) -> &Revision {
        &self.0.snapshot_revision
    }
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum SpendingStyle {
    #[serde(rename = "value")]
    Value,
    #[serde(rename = "balanced")]
    Balanced,
    #[serde(rename = "flexible")]
    Flexible,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct Text(String);
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(deny_unknown_fields)]
pub struct UsualWeek {
    cells: Array_CellDefault_21_21,
    day_enabled: Array_boolean_7_7,
    enabled_meal_types: Array_MealType_0_3,
    suggest_leftovers: bool,
    zone: ZoneId,
}
#[derive(Clone, Debug, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
pub enum Weekday {
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
pub struct ZoneId(String);
const SCHEMA: &str = include_str!("draft005.schema.json");
pub(super) fn snapshot(bytes: &[u8]) -> Result<B1Draft005, Error> {
    codec::decode(bytes, SCHEMA, "Snapshot").map(B1Draft005)
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
pub struct Receipt {
    command: AppCommandV1,
    response: B1Draft005,
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
    pub(super) fn response(&self) -> &B1Draft005 {
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

// Read-only navigation of the frozen receipt graph.
impl AppCommandV1 {
    pub fn command(&self) -> &AppCommand {
        &self.command
    }
    pub fn command_id(&self) -> &CommandId {
        &self.command_id
    }
    pub fn expected_snapshot_revision(&self) -> &Revision {
        &self.expected_snapshot_revision
    }
    pub fn schema_version(&self) -> &SchemaVersion {
        &self.schema_version
    }
}
impl CellDefault {
    pub fn attendee_ids(&self) -> &Array_DinerId_0_64 {
        &self.attendee_ids
    }
    pub fn choice(&self) -> &MealLocation {
        &self.choice
    }
    pub fn meal_type(&self) -> &MealType {
        &self.meal_type
    }
    pub fn weekday(&self) -> &Weekday {
        &self.weekday
    }
}
impl Connections {
    pub fn calendar_destination(&self) -> &Nullable_CalendarDestinationId {
        &self.calendar_destination
    }
    pub fn delivery_preferences(&self) -> &Array_DeliveryPreference_0_7 {
        &self.delivery_preferences
    }
    pub fn harness(&self) -> &Nullable_Provider {
        &self.harness
    }
    pub fn loyalty(&self) -> &LoyaltyStatus {
        &self.loyalty
    }
    pub fn privacy(&self) -> &PrivacyPolicy {
        &self.privacy
    }
    pub fn reminders(&self) -> &ReminderSettings {
        &self.reminders
    }
    pub fn retailer(&self) -> &Nullable_Retailer {
        &self.retailer
    }
}
impl DeliveryPreference {
    pub fn earliest(&self) -> &LocalTime {
        &self.earliest
    }
    pub fn latest(&self) -> &LocalTime {
        &self.latest
    }
    pub fn weekday(&self) -> &Weekday {
        &self.weekday
    }
}
impl Diner {
    pub fn age_band(&self) -> &AgeBand {
        &self.age_band
    }
    pub fn diner_id(&self) -> &DinerId {
        &self.diner_id
    }
    pub fn dislikes(&self) -> &Array_IngredientId_0_256 {
        &self.dislikes
    }
    pub fn display_name(&self) -> &Nullable_NameText {
        &self.display_name
    }
    pub fn portions(&self) -> &PortionPreference {
        &self.portions
    }
    pub fn restrictions(&self) -> &RestrictionDeclaration {
        &self.restrictions
    }
}
impl HouseholdDraft {
    pub fn completed_stages(&self) -> &Array_HouseholdStage_0_4 {
        &self.completed_stages
    }
    pub fn connections(&self) -> &Nullable_Connections {
        &self.connections
    }
    pub fn diners(&self) -> &Array_Diner_0_64 {
        &self.diners
    }
    pub fn household_id(&self) -> &HouseholdId {
        &self.household_id
    }
    pub fn preferences(&self) -> &Nullable_Preferences {
        &self.preferences
    }
    pub fn revision(&self) -> &Revision {
        &self.revision
    }
    pub fn schema_version(&self) -> &SchemaVersion {
        &self.schema_version
    }
    pub fn unresolved_restrictions(&self) -> &Array_SetupRestrictionIssue_0_64 {
        &self.unresolved_restrictions
    }
    pub fn usual_week(&self) -> &Nullable_UsualWeek {
        &self.usual_week
    }
}
impl HouseholdProfile {
    pub fn connections(&self) -> &Connections {
        &self.connections
    }
    pub fn diners(&self) -> &Array_Diner_1_64 {
        &self.diners
    }
    pub fn household_id(&self) -> &HouseholdId {
        &self.household_id
    }
    pub fn preferences(&self) -> &Preferences {
        &self.preferences
    }
    pub fn revision(&self) -> &Revision {
        &self.revision
    }
    pub fn schema_version(&self) -> &SchemaVersion {
        &self.schema_version
    }
    pub fn usual_week(&self) -> &UsualWeek {
        &self.usual_week
    }
}
impl JobSummary {
    pub fn action(&self) -> &Action {
        &self.action
    }
    pub fn job_id(&self) -> &JobId {
        &self.job_id
    }
    pub fn next_action(&self) -> &Nullable_RequiredAction {
        &self.next_action
    }
    pub fn reason(&self) -> &Nullable_ErrorCode {
        &self.reason
    }
    pub fn state(&self) -> &JobState {
        &self.state
    }
}
impl MealPortionOverride {
    pub fn adjustment(&self) -> &PositiveRational {
        &self.adjustment
    }
    pub fn meal_type(&self) -> &MealType {
        &self.meal_type
    }
    pub fn preset(&self) -> &PortionPresetName {
        &self.preset
    }
}
impl MealTime {
    pub fn meal_type(&self) -> &MealType {
        &self.meal_type
    }
    pub fn time(&self) -> &LocalTime {
        &self.time
    }
}
impl Mobility {
    pub fn cold_storage(&self) -> &bool {
        &self.cold_storage
    }
    pub fn reheating(&self) -> &bool {
        &self.reheating
    }
    pub fn temperature(&self) -> &MobilityTemperature {
        &self.temperature
    }
}
impl PortionPreference {
    pub fn adjustment(&self) -> &PositiveRational {
        &self.adjustment
    }
    pub fn meal_overrides(&self) -> &Array_MealPortionOverride_0_3 {
        &self.meal_overrides
    }
    pub fn preset(&self) -> &PortionPresetName {
        &self.preset
    }
}
impl PositiveRational {
    pub fn denominator(&self) -> &String {
        &self.denominator
    }
    pub fn numerator(&self) -> &String {
        &self.numerator
    }
}
impl Preferences {
    pub fn dislikes(&self) -> &Array_IngredientId_0_256 {
        &self.dislikes
    }
    pub fn equipment(&self) -> &Array_EquipmentCode_0_9 {
        &self.equipment
    }
    pub fn higher_protein(&self) -> &bool {
        &self.higher_protein
    }
    pub fn meal_times(&self) -> &Array_MealTime_3_3 {
        &self.meal_times
    }
    pub fn mobility(&self) -> &Mobility {
        &self.mobility
    }
    pub fn more_variety(&self) -> &bool {
        &self.more_variety
    }
    pub fn preferred_hands_on_minutes(&self) -> &u64 {
        &self.preferred_hands_on_minutes
    }
    pub fn quick_meals(&self) -> &bool {
        &self.quick_meals
    }
    pub fn spending_style(&self) -> &SpendingStyle {
        &self.spending_style
    }
    pub fn whole_food_preference(&self) -> &bool {
        &self.whole_food_preference
    }
}
impl PrivacyPolicy {
    pub fn confirmed_at(&self) -> &Nullable_Instant {
        &self.confirmed_at
    }
    pub fn include_household_requirements(&self) -> &bool {
        &self.include_household_requirements
    }
    pub fn revision(&self) -> &Revision {
        &self.revision
    }
    pub fn use_ai_features(&self) -> &bool {
        &self.use_ai_features
    }
}
impl ReminderSettings {
    pub fn channels(&self) -> &Array_ReminderChannel_0_2 {
        &self.channels
    }
    pub fn enabled(&self) -> &bool {
        &self.enabled
    }
    pub fn prompts(&self) -> &Array_ReminderPrompt_0_2 {
        &self.prompts
    }
}
impl SetupRestrictionIssue {
    pub fn diner_id(&self) -> &DinerId {
        &self.diner_id
    }
    pub fn supplied_text(&self) -> &Text {
        &self.supplied_text
    }
}
impl UsualWeek {
    pub fn cells(&self) -> &Array_CellDefault_21_21 {
        &self.cells
    }
    pub fn day_enabled(&self) -> &Array_boolean_7_7 {
        &self.day_enabled
    }
    pub fn enabled_meal_types(&self) -> &Array_MealType_0_3 {
        &self.enabled_meal_types
    }
    pub fn suggest_leftovers(&self) -> &bool {
        &self.suggest_leftovers
    }
    pub fn zone(&self) -> &ZoneId {
        &self.zone
    }
}
impl CalendarDestinationId {
    pub fn value(&self) -> &String {
        &self.0
    }
}
impl CommandId {
    pub fn value(&self) -> &String {
        &self.0
    }
}
impl DinerId {
    pub fn value(&self) -> &String {
        &self.0
    }
}
impl HouseholdId {
    pub fn value(&self) -> &String {
        &self.0
    }
}
impl IngredientId {
    pub fn value(&self) -> &String {
        &self.0
    }
}
impl Instant {
    pub fn value(&self) -> &String {
        &self.0
    }
}
impl JobId {
    pub fn value(&self) -> &String {
        &self.0
    }
}
impl LocalTime {
    pub fn value(&self) -> &String {
        &self.0
    }
}
impl NameText {
    pub fn value(&self) -> &String {
        &self.0
    }
}
impl Revision {
    pub fn value(&self) -> &String {
        &self.0
    }
}
impl SchemaVersion {
    pub fn value(&self) -> &u64 {
        &self.0
    }
}
impl Text {
    pub fn value(&self) -> &String {
        &self.0
    }
}
impl ZoneId {
    pub fn value(&self) -> &String {
        &self.0
    }
}
