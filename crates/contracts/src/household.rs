//! Household, preference and setup wire records.

use crate::*;

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
#[serde(rename_all = "snake_case")]
pub enum AgeBand {
    #[ts(rename = "adult")]
    Adult,
    #[ts(rename = "teen")]
    Teen,
    #[serde(rename = "child_1_3")]
    #[ts(rename = "child_1_3")]
    Child1To3,
    #[serde(rename = "child_4_6")]
    #[ts(rename = "child_4_6")]
    Child4To6,
    #[serde(rename = "child_7_10")]
    #[ts(rename = "child_7_10")]
    Child7To10,
    #[serde(rename = "child_11_12")]
    #[ts(rename = "child_11_12")]
    Child11To12,
}

string_enum! {
    pub enum PortionPresetName {
        Smaller,
        Regular,
        Larger,
    }
}

string_enum! {
    pub enum AllergenCode {
        Celery,
        CerealsContainingGluten,
        Crustaceans,
        Eggs,
        Fish,
        Lupin,
        Milk,
        Molluscs,
        Mustard,
        Peanuts,
        Sesame,
        Soybeans,
        SulphurDioxideSulphites,
        TreeNuts,
    }
}

string_enum! {
    pub enum DietCode {
        Vegetarian,
        Vegan,
        GlutenFree,
    }
}

string_enum! {
    pub enum SpendingStyle {
        Value,
        Balanced,
        Flexible,
    }
}

string_enum! {
    pub enum MobilityTemperature {
        Hot,
        Cold,
        Either,
    }
}

string_enum! {
    pub enum LoyaltyStatus {
        ConfirmedMember,
        NotMember,
        Unknown,
    }
}

string_enum! {
    pub enum ReminderChannel {
        Local,
        Calendar,
    }
}

string_enum! {
    pub enum ReminderPrompt {
        UnfinishedPlan,
        BookingOpportunity,
    }
}

string_enum! {
    pub enum HouseholdStage {
        Household,
        FoodPreferences,
        UsualWeek,
        Connections,
    }
}

tagged! {
    pub enum HardRequirement {
        AllergenExclusion {
            code: AllergenCode,
        },
        Diet {
            code: DietCode,
        },
        IngredientExclusion {
            ingredient_id: IngredientId,
        },
    }
}

tagged! {
    pub enum RestrictionDeclaration {
        Unconfirmed {},
        ConfirmedNone {
            confirmed_at: Instant,
        },
        Confirmed {
            confirmed_at: Instant,
            requirements: BoundedVec<HardRequirement, 1, 64>,
        },
    }
}

record! {
    pub struct MealPortionOverride {
        pub meal_type: MealType,
        pub preset: PortionPresetName,
        pub adjustment: PositiveRational,
    }
}

record! {
    pub struct PortionPreference {
        pub preset: PortionPresetName,
        pub adjustment: PositiveRational,
        pub meal_overrides: BoundedVec<MealPortionOverride, 0, 3>,
    }
}

record! {
    pub struct Diner {
        pub diner_id: DinerId,
        pub display_name: Nullable<NameText>,
        pub age_band: AgeBand,
        pub restrictions: RestrictionDeclaration,
        pub dislikes: BoundedVec<IngredientId, 0, 256>,
        pub portions: PortionPreference,
    }
}

record! {
    pub struct MealTime {
        pub meal_type: MealType,
        pub time: LocalTime,
    }
}

record! {
    pub struct Mobility {
        pub reheating: bool,
        pub cold_storage: bool,
        pub temperature: MobilityTemperature,
    }
}

record! {
    pub struct Preferences {
        pub spending_style: SpendingStyle,
        pub higher_protein: bool,
        pub more_variety: bool,
        pub quick_meals: bool,
        pub whole_food_preference: bool,
        pub dislikes: BoundedVec<IngredientId, 0, 256>,
        pub equipment: BoundedVec<EquipmentCode, 0, 9>,
        pub preferred_hands_on_minutes: u16,
        pub meal_times: BoundedVec<MealTime, 3, 3>,
        pub mobility: Mobility,
    }
}

record! {
    pub struct CellDefault {
        pub weekday: Weekday,
        pub meal_type: MealType,
        pub choice: MealLocation,
        pub attendee_ids: BoundedVec<DinerId, 0, 64>,
    }
}

record! {
    pub struct UsualWeek {
        pub enabled_meal_types: BoundedVec<MealType, 0, 3>,
        pub day_enabled: DayEnabled,
        pub cells: BoundedVec<CellDefault, 21, 21>,
        pub suggest_leftovers: bool,
        pub zone: ZoneId,
    }
}

record! {
    pub struct DeliveryPreference {
        pub weekday: Weekday,
        pub earliest: LocalTime,
        pub latest: LocalTime,
    }
}

record! {
    pub struct ReminderSettings {
        pub enabled: bool,
        pub channels: BoundedVec<ReminderChannel, 0, 2>,
        pub prompts: BoundedVec<ReminderPrompt, 0, 2>,
    }
}

record! {
    pub struct Connections {
        pub harness: Nullable<Provider>,
        pub retailer: Nullable<Retailer>,
        pub calendar_destination: Nullable<CalendarDestinationId>,
        pub loyalty: LoyaltyStatus,
        pub delivery_preferences: BoundedVec<DeliveryPreference, 0, 7>,
        pub privacy: PrivacyPolicy,
        pub reminders: ReminderSettings,
    }
}

record! {
    pub struct HouseholdDraft {
        pub schema_version: SchemaVersion,
        pub household_id: HouseholdId,
        pub revision: Revision,
        pub completed_stages: BoundedVec<HouseholdStage, 0, 4>,
        pub diners: BoundedVec<Diner, 0, 64>,
        pub preferences: Nullable<Preferences>,
        pub usual_week: Nullable<UsualWeek>,
        pub connections: Nullable<Connections>,
    }
}

record! {
    pub struct HouseholdProfile {
        pub schema_version: SchemaVersion,
        pub household_id: HouseholdId,
        pub revision: Revision,
        pub diners: BoundedVec<Diner, 1, 64>,
        pub preferences: Preferences,
        pub usual_week: UsualWeek,
        pub connections: Connections,
    }
}
string_enum! {pub enum EquipmentCode {Hob,Oven,Microwave,Fridge,Kettle,Toaster,Blender,SlowCooker,PressureCooker}}
