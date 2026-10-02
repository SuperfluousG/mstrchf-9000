use crate::*;
record! {pub struct RecipeRef {pub recipe_id:RecipeId,pub revision:Revision,pub digest:RecipeDigest}}
record! {pub struct WeekRevision {pub week_id:WeekId,pub revision:Revision}}
record! {pub struct ContextStamp {pub household_revision:Revision,pub settings_revision:Revision,pub privacy_revision:Revision,pub history_revision:Revision,pub catalogue_revision:Revision,pub ranking_policy_version:Revision,pub recipe_refs:BoundedVec<RecipeRef,0,512>,pub read_weeks:BoundedVec<WeekRevision,0,4>,pub write_weeks:BoundedVec<WeekRevision,0,4>}}
record! {pub struct PlanningContext {pub schema_version:SchemaVersion,pub household:HouseholdProfile,pub settings_revision:Revision,pub privacy:PrivacyPolicy,pub requirements:BoundedVec<MealRequirements,0,84>,pub weeks:BoundedVec<Week,0,4>,pub recipes:BoundedVec<RecipeRevision,0,512>,pub history:PlanningHistory,pub stamp:ContextStamp,pub ranking_policy:RankingPolicyRef}}
pub fn compare_context(expected: &ContextStamp, current: &ContextStamp) -> Result<(), DomainError> {
    if expected != current {
        Err(DomainError::new(ErrorCode::ContextStale, "/context_stamp"))
    } else {
        Ok(())
    }
}
use serde_json::Value;
use std::collections::{BTreeMap, BTreeSet};
fn err(code: ErrorCode, path: &str) -> DomainError {
    DomainError::new(code, path)
}
fn decoded<T: serde::de::DeserializeOwned>(v: &Value) -> Result<T, DomainError> {
    serde_json::from_value(v.clone()).map_err(|e| crate::parse::decode_error(&e.to_string()))
}
fn key<T: serde::Serialize>(v: &T) -> String {
    serde_jcs::to_string(v).expect("typed contract serialisation")
}
fn require(ok: bool, code: ErrorCode, path: &str) -> Result<(), DomainError> {
    if ok { Ok(()) } else { Err(err(code, path)) }
}
fn unique<T: serde::Serialize>(items: &[T], path: &str) -> Result<(), DomainError> {
    let mut seen = BTreeSet::new();
    require(
        items.iter().all(|i| seen.insert(key(i))),
        ErrorCode::InvalidValue,
        path,
    )
}
pub(crate) fn validate_local(name: &str, v: &Value) -> Result<(), DomainError> {
    if let Some(obj) = v.as_object() {
        for (field, val) in obj {
            if let Some(a) = val.as_array() {
                if field != "day_enabled" {
                    let mut seen = BTreeSet::new();
                    for x in a {
                        if field == "ingredients" && x.get("ingredient_id") == Some(&Value::Null) {
                            continue;
                        }
                        let identity = if name == "RecipeCandidate" && field == "ingredients" {
                            key(&x.get("ingredient_id"))
                        } else {
                            collection_identity(field, x)
                        };
                        if !seen.insert(identity) {
                            return Err(err(ErrorCode::InvalidValue, &format!("/{field}")));
                        }
                    }
                }
                if matches!(
                    field.as_str(),
                    "ingredient_ids"
                        | "component_ids"
                        | "evidence_ids"
                        | "conversion_evidence_ids"
                        | "storage_rule_ids"
                        | "attendee_ids"
                        | "host_diner_ids"
                        | "week_ids"
                        | "event_ids"
                        | "hidden_recipe_ids"
                        | "dislikes"
                        | "equipment"
                        | "required_capabilities"
                        | "requested_capabilities"
                        | "enabled_meal_types"
                        | "completed_stages"
                        | "reasons"
                        | "channels"
                        | "prompts"
                ) {
                    require(
                        a.windows(2).all(|w| key(&w[0]) < key(&w[1])),
                        ErrorCode::NoncanonicalValue,
                        &format!("/{field}"),
                    )?;
                }
            }
        }
    }
    match name {
        "ContextStamp" => validate_stamp(&decoded(v)?),
        "Snapshot" => {
            let snapshot: Snapshot = decoded(v)?;
            if let Some(connections) = &snapshot.household_draft.connections.0 {
                require(
                    connections.privacy == snapshot.privacy,
                    ErrorCode::ContextStale,
                    "/household_draft/connections/privacy",
                )?;
            }
            if let Some(profile) = &snapshot.profile.0 {
                require(
                    profile.connections.privacy == snapshot.privacy,
                    ErrorCode::ContextStale,
                    "/profile/connections/privacy",
                )?;
            }
            Ok(())
        }
        "HouseholdDraft" => {
            let h: HouseholdDraft = decoded(v)?;
            let confirmed = !h.diners.is_empty()
                && h.diners
                    .iter()
                    .all(|d| !matches!(d.restrictions, RestrictionDeclaration::Unconfirmed {}));
            for (s, complete) in [
                (HouseholdStage::Household, confirmed),
                (HouseholdStage::FoodPreferences, h.preferences.0.is_some()),
                (HouseholdStage::UsualWeek, h.usual_week.0.is_some()),
                (HouseholdStage::Connections, h.connections.0.is_some()),
            ] {
                require(
                    h.completed_stages.contains(&s) == complete,
                    ErrorCode::InvalidValue,
                    "/completed_stages",
                )?;
            }
            Ok(())
        }
        "HouseholdProfile" => {
            let h: HouseholdProfile = decoded(v)?;
            require(
                h.diners
                    .iter()
                    .all(|d| !matches!(d.restrictions, RestrictionDeclaration::Unconfirmed {})),
                ErrorCode::InvalidValue,
                "/diners",
            )
        }
        "Preferences" => {
            let p: Preferences = decoded(v)?;
            require(
                p.preferred_hands_on_minutes <= 1440,
                ErrorCode::LimitExceeded,
                "/preferred_hands_on_minutes",
            )
        }
        "AiPreferences" => {
            let p: AiPreferences = decoded(v)?;
            require(
                p.preferred_hands_on_minutes <= 1440,
                ErrorCode::LimitExceeded,
                "/preferred_hands_on_minutes",
            )
        }
        "ReminderSettings" => {
            let r: ReminderSettings = decoded(v)?;
            require(
                !r.enabled || (!r.channels.is_empty() && !r.prompts.is_empty()),
                ErrorCode::InvalidValue,
                "/enabled",
            )
        }
        "DeliveryPreference" => {
            let d: DeliveryPreference = decoded(v)?;
            require(
                d.earliest.0 < d.latest.0,
                ErrorCode::InvalidValue,
                "/latest",
            )
        }
        "WeekRange" => {
            let w: WeekRange = decoded(v)?;
            use chrono::Datelike;
            require(
                chrono::NaiveDate::parse_from_str(&w.monday.0, "%Y-%m-%d")
                    .map(|d| d.weekday() == chrono::Weekday::Mon)
                    .unwrap_or(false),
                ErrorCode::InvalidValue,
                "/monday",
            )
        }
        "Week" => validate_week(&decoded(v)?),
        "WeekEdit" => {
            let w: WeekEdit = decoded(v)?;
            require(
                w.cells.iter().all(|c| c.slot.week_id == w.week_id),
                ErrorCode::ForeignReference,
                "/cells",
            )?;
            unique_slots(&w.cells)
        }
        "WeekPlan" => {
            let p: WeekPlan = decoded(v)?;
            require(
                p.selections.iter().all(|s| s.slot.week_id == p.week_id)
                    && p.batch_links.iter().all(|b| {
                        b.source_slot.week_id == p.week_id && b.target_slot.week_id == p.week_id
                    }),
                ErrorCode::IdentityMismatch,
                "/selections",
            )
        }
        "MealRequirements" => validate_requirements(&decoded(v)?),
        "ProposedSelection" => {
            let p: ProposedSelection = decoded(v)?;
            require(
                p.selection.0.as_ref().is_none_or(|s| s.slot == p.slot),
                ErrorCode::IdentityMismatch,
                "/selection/slot",
            )
        }
        "WeekChange" => {
            let w: WeekChange = decoded(v)?;
            require(
                w.replace_slots.iter().all(|s| s.slot.week_id == w.week_id),
                ErrorCode::ForeignReference,
                "/replace_slots",
            )
        }
        "Feedback" => {
            let f: Feedback = decoded(v)?;
            require(
                f.portion_change.0.is_some() || !f.portion_change_accepted,
                ErrorCode::InvalidValue,
                "/portion_change_accepted",
            )
        }
        "RecipeRevision" => validate_recipe(&decoded(v)?),
        "ComponentProductionSpec" => {
            let anchor: ComponentProductionSpec = decoded(v)?;
            let id = match anchor {
                ComponentProductionSpec::Cooked { completion_prep_id } => completion_prep_id,
                ComponentProductionSpec::Unheated {
                    first_handling_prep_id,
                } => first_handling_prep_id,
            };
            require((1..=128).contains(&id), ErrorCode::InvalidValue, "/data")
        }
        "ReusedComponentInput" => {
            let input: ReusedComponentInput = decoded(v)?;
            require(
                input.planned_reheats <= 1
                    && input.step_ids.iter().all(|id| (1..=256).contains(id)),
                ErrorCode::InvalidValue,
                "/planned_reheats",
            )
        }
        "DerivedComponent" => {
            let component: DerivedComponent = decoded(v)?;
            require(
                component
                    .storage_rule
                    .0
                    .as_ref()
                    .is_none_or(|rule| rule.component_id == component.component_id),
                ErrorCode::ForeignReference,
                "/storage_rule/component_id",
            )
        }
        "ReviewRecord" => {
            let r: ReviewRecord = decoded(v)?;
            require(
                r.kind != ReviewKind::CookTest || r.actor == ReviewActor::Human,
                ErrorCode::AuthorityDenied,
                "/actor",
            )
        }
        "InstructionStep" => {
            let s: InstructionStep = decoded(v)?;
            require(
                s.step_id > 0
                    && s.step_id <= 256
                    && s.active_minutes <= s.elapsed_minutes
                    && s.elapsed_minutes <= 10080,
                ErrorCode::InvalidValue,
                "/step_id",
            )
        }
        "PrepSpec" => {
            let p: PrepSpec = decoded(v)?;
            require(
                p.prep_id > 0
                    && p.prep_id <= 128
                    && p.active_minutes <= p.elapsed_minutes
                    && p.elapsed_minutes <= 10080
                    && (-604800..=604800).contains(&p.earliest_offset_seconds)
                    && (-604800..=604800).contains(&p.latest_offset_seconds)
                    && p.earliest_offset_seconds <= p.latest_offset_seconds,
                ErrorCode::InvalidValue,
                "/prep_id",
            )
        }
        "StorageRule" => {
            let s: StorageRule = decoded(v)?;
            require(
                (1..=172800).contains(&s.max_chilled_seconds) && s.max_reheats <= 1,
                ErrorCode::InvalidValue,
                "/max_chilled_seconds",
            )
        }
        "ScalingPolicy" => {
            let s: ScalingPolicy = decoded(v)?;
            match s {
                ScalingPolicy::Linear { minimum, maximum } => require(
                    rat_le(&minimum.0, &maximum.0),
                    ErrorCode::InvalidValue,
                    "/data/maximum",
                ),
                ScalingPolicy::DiscreteBatches {
                    maximum_batches, ..
                } => require(
                    maximum_batches > 0,
                    ErrorCode::InvalidValue,
                    "/data/maximum_batches",
                ),
            }
        }
        "TimeInterval" => {
            let t: TimeInterval = decoded(v)?;
            require(
                t.start.instant.0 <= t.end.instant.0,
                ErrorCode::InvalidValue,
                "/end",
            )
        }
        "ScheduledPrepTask" => {
            let t: ScheduledPrepTask = decoded(v)?;
            require(
                t.task_id > 0 && t.task_id <= 256 && t.prep_id > 0 && t.prep_id <= 128,
                ErrorCode::InvalidValue,
                "/task_id",
            )?;
            require(
                t.active_intervals.iter().all(|i| {
                    i.start.instant.0 >= t.elapsed.start.instant.0
                        && i.end.instant.0 <= t.elapsed.end.instant.0
                }) && t
                    .active_intervals
                    .windows(2)
                    .all(|w| w[0].end.instant.0 <= w[1].start.instant.0),
                ErrorCode::InvalidValue,
                "/active_intervals",
            )
        }
        "DemandLine" => {
            let l: DemandLine = decoded(v)?;
            require(
                !l.origin_batches.is_empty() || !l.origin_extras.is_empty(),
                ErrorCode::InvalidValue,
                "/origin_batches",
            )
        }
        "DemandSet" => {
            let d: DemandSet = decoded(v)?;
            require(
                demand_digest(&DemandMaterial {
                    schema_version: SchemaVersion,
                    input_plan_digest: d.input_plan_digest,
                    lines: d.lines,
                })? == d.digest,
                ErrorCode::DigestMismatch,
                "/digest",
            )
        }
        "DerivedPlan" => {
            let d: DerivedPlan = decoded(v)?;
            require(
                d.input_plan_digest == d.demand.input_plan_digest,
                ErrorCode::DigestMismatch,
                "/demand/input_plan_digest",
            )?;
            require(
                derived_digest(&DerivedMaterial::from(&d))? == d.digest,
                ErrorCode::DigestMismatch,
                "/digest",
            )?;
            require(
                !d.demand.lines.is_empty() || d.batches.is_empty(),
                ErrorCode::InvalidValue,
                "/demand/lines",
            )
        }
        "OrderConfirmation" => {
            let o: OrderConfirmation = decoded(v)?;
            require(
                o.delivery_start.instant.0 < o.delivery_end.instant.0,
                ErrorCode::InvalidValue,
                "/delivery_end",
            )
        }
        "ConfirmedOrder" => {
            let o: ConfirmedOrder = decoded(v)?;
            require(
                o.confirmation.week_id == o.approved_plan.week_id
                    && o.confirmation.approval_id == o.approved_plan.approval_id,
                ErrorCode::IdentityMismatch,
                "/approved_plan",
            )
        }
        "CalendarEventLogicalKey" => {
            let k: CalendarEventLogicalKey = decoded(v)?;
            require(
                match k.kind {
                    CalendarEventKind::Cooking | CalendarEventKind::Prep => {
                        k.session_id.0.is_some() && k.order_id.0.is_none()
                    }
                    CalendarEventKind::Delivery => {
                        k.order_id.0.is_some() && k.session_id.0.is_none()
                    }
                },
                ErrorCode::InvalidValue,
                "/kind",
            )
        }
        "CalendarEvent" => validate_event(&decoded(v)?),
        "EventBatch" => {
            let b: EventBatch = decoded(v)?;
            require(
                b.events.iter().all(|e| {
                    e.logical_key.week_id == b.week_id
                        && (e.logical_key.kind != CalendarEventKind::Delivery
                            || e.logical_key.order_id.0.as_ref() == Some(&b.order_id))
                }),
                ErrorCode::IdentityMismatch,
                "/events",
            )?;
            require(
                b.events
                    .iter()
                    .any(|e| e.logical_key.kind == CalendarEventKind::Delivery),
                ErrorCode::InvalidValue,
                "/events",
            )?;
            unique(
                &b.events.iter().map(|e| &e.logical_key).collect::<Vec<_>>(),
                "/events",
            )
        }
        "AppCommandV1" => validate_request_job_command(&decoded(v)?),
        "ReadScope" => validate_read_scope(&decoded(v)?),
        "ReadObservationV1" => validate_read_shape(&decoded(v)?),
        "OperationRequest" => validate_operation_shape(&decoded(v)?),
        "OperationGrant" => validate_grant_shape(&decoded(v)?),
        "OperationObservation" => validate_operation_observation_shape(&decoded(v)?),
        "ApprovedContributionRef" => validate_approved_contribution_shape(&decoded(v)?),
        "JobRequestV1" => validate_request_shape(&decoded(v)?),
        "JobResultV1" => validate_result_shape(&decoded(v)?),
        "ApprovedProductLine" => {
            let l: ApprovedProductLine = decoded(v)?;
            require(
                l.expected_quantity <= 10000 && l.target_quantity <= 10000,
                ErrorCode::LimitExceeded,
                "/target_quantity",
            )?;
            require(
                l.target_quantity >= l.expected_quantity
                    || (l.removal_approved && l.planner_attribution_digest.0.is_some()),
                ErrorCode::AuthorityDenied,
                "/removal_approved",
            )
        }
        "ArtefactRef" => {
            let a: ArtefactRef = decoded(v)?;
            require(
                a.relative_name.0 == format!("input/{}.json", a.artefact_id.0),
                ErrorCode::InvalidValue,
                "/relative_name",
            )?;
            require(
                a.byte_length as usize <= MAX_DOCUMENT_BYTES,
                ErrorCode::LimitExceeded,
                "/byte_length",
            )
        }
        "PlanningContext" => validate_context(&decoded(v)?).map_err(crate::parse::shape_error),
        _ => Ok(()),
    }
}
fn collection_identity(field: &str, x: &Value) -> String {
    if field == "weeks" {
        return key(&x.pointer("/range/week_id"));
    }
    if field == "ordered" {
        return key(&(x.get("order_id"), x.pointer("/selection/slot")));
    }
    if field == "observations" {
        return key(&(
            x.get("kind"),
            x.pointer("/data/observation/observation_id")
                .or_else(|| x.pointer("/data/reference/operation_id")),
        ));
    }
    let fields: &[&str] = match field {
        "diners" => &["diner_id"],
        "guests" => &["guest_id"],
        "cells" => {
            if x.get("slot").is_some() {
                &["slot"]
            } else {
                &["weekday", "meal_type"]
            }
        }
        "selections" | "replace_slots" | "requirements" => {
            if x.get("slot").is_some() {
                &["slot"]
            } else {
                &[]
            }
        }
        "read_weeks" | "write_weeks" | "changes" | "other_weeks" => &["week_id"],
        "weeks" => &["range"],
        "recipes"
        | "public_recipes"
        | "recipe_refs"
        | "ordered_recipe_refs"
        | "selected_recipe_refs" => &["recipe_id", "revision"],
        "meal_times" | "meal_overrides" => &["meal_type"],
        "delivery_preferences" => &["weekday"],
        "servings" | "recipients" | "portions" | "attendees" => &["person"],
        "ingredients" => &["component_id", "ingredient_id"],
        "components" => &["component_id"],
        "steps" => &["step_id"],
        "prep" => &["prep_id"],
        "tasks" => &["task_id"],
        "extras" => &["extra_id"],
        "reused_inputs" => &["source_recipe", "source_component_id"],
        "dietary_evidence" | "storage" | "reviews" | "conversion_evidence" => &["evidence_id"],
        "portion_presets" => &["age_band", "meal_type", "preset"],
        "unresolved" | "blocking_issues" => {
            if x.get("path").is_some() {
                &["path", "code"]
            } else {
                &["line_id", "code", "required_action"]
            }
        }
        "batch_links" => &["batch_id", "source_slot", "target_slot"],
        "batches" => &["batch_id"],
        "allocations" => &["allocation_id"],
        "prep_sessions" => &["session_id"],
        "allowed_artefacts" => &["artefact_id"],
        "events" => &["event_id"],
        "jobs" => &["job_id"],
        "confirmations" => &["line_id"],
        "offers" => &["offer_ref"],
        "lines" => {
            if x.get("line_id").is_some() {
                &["line_id"]
            } else {
                &["product"]
            }
        }
        "capabilities" => &["capability"],
        "selected" => &["slot", "selected_at"],
        "ordered" => &["order_id", "selection"],
        "feedback" => {
            if x.get("slot").is_some() {
                &["slot", "recipe", "recorded_at"]
            } else {
                &["recipe", "rating", "reasons"]
            }
        }
        _ => &[],
    };
    if fields.is_empty() {
        key(x)
    } else {
        key(&fields
            .iter()
            .map(|f| x.get(*f).unwrap_or(&Value::Null))
            .collect::<Vec<_>>())
    }
}
fn rat_le(a: &Rational, b: &Rational) -> bool {
    u128::from(a.numerator.0) * u128::from(b.denominator.0)
        <= u128::from(b.numerator.0) * u128::from(a.denominator.0)
}
pub fn validate_stamp(s: &ContextStamp) -> Result<(), DomainError> {
    for (a, p) in [
        (&s.read_weeks.0, "/read_weeks"),
        (&s.write_weeks.0, "/write_weeks"),
    ] {
        require(
            a.windows(2).all(|w| w[0].week_id < w[1].week_id),
            ErrorCode::InvalidValue,
            p,
        )?;
    }
    require(
        s.recipe_refs
            .windows(2)
            .all(|w| (&w[0].recipe_id, w[0].revision.0) < (&w[1].recipe_id, w[1].revision.0)),
        ErrorCode::InvalidValue,
        "/recipe_refs",
    )?;
    require(
        s.write_weeks.iter().all(|w| s.read_weeks.contains(w)),
        ErrorCode::MissingReference,
        "/write_weeks",
    )
}
fn unique_slots(cells: &[Cell]) -> Result<(), DomainError> {
    unique(&cells.iter().map(|c| &c.slot).collect::<Vec<_>>(), "/cells")
}
/// Active cells, including unoccupied slots, in civil date and meal order.
pub fn active_slots(week: &Week) -> Result<Vec<SlotId>, DomainError> {
    week.validate_shape().map_err(crate::parse::shape_error)?;
    let mut slots: Vec<_> = week
        .cells
        .iter()
        .filter(|cell| cell_enabled(week, cell))
        .map(|cell| cell.slot.clone())
        .collect();
    slots.sort_by_key(|slot| {
        (
            slot.date.clone(),
            match slot.meal_type {
                MealType::Breakfast => 0,
                MealType::Lunch => 1,
                MealType::Dinner => 2,
            },
        )
    });
    Ok(slots)
}

pub fn active_selections(w: &Week) -> Vec<&MealSelection> {
    w.plan
        .selections
        .iter()
        .filter(|s| {
            w.cells
                .iter()
                .any(|c| c.slot == s.slot && cell_enabled(w, c))
        })
        .collect()
}
fn cell_enabled(w: &Week, c: &Cell) -> bool {
    let date = chrono::NaiveDate::parse_from_str(&c.slot.date.0, "%Y-%m-%d").ok();
    let monday = chrono::NaiveDate::parse_from_str(&w.range.monday.0, "%Y-%m-%d").ok();
    let day = date
        .zip(monday)
        .map(|(d, m)| (d - m).num_days())
        .unwrap_or(-1);
    (0..7).contains(&day)
        && w.day_enabled[day as usize]
        && match c.slot.meal_type {
            MealType::Breakfast => w.meal_type_enabled.breakfast,
            MealType::Lunch => w.meal_type_enabled.lunch,
            MealType::Dinner => w.meal_type_enabled.dinner,
        }
        && c.choice != MealLocation::Skip
}
fn validate_week(w: &Week) -> Result<(), DomainError> {
    require(
        w.plan.week_id == w.range.week_id,
        ErrorCode::IdentityMismatch,
        "/plan/week_id",
    )?;
    require(
        w.plan.revision == w.revision,
        ErrorCode::InvalidValue,
        "/plan/revision",
    )?;
    unique_slots(&w.cells)?;
    let monday = chrono::NaiveDate::parse_from_str(&w.range.monday.0, "%Y-%m-%d")
        .map_err(|_| err(ErrorCode::InvalidValue, "/range/monday"))?;
    for c in w.cells.iter() {
        let date = chrono::NaiveDate::parse_from_str(&c.slot.date.0, "%Y-%m-%d")
            .map_err(|_| err(ErrorCode::InvalidValue, "/cells"))?;
        require(
            c.slot.week_id == w.range.week_id && (0..7).contains(&(date - monday).num_days()),
            ErrorCode::ForeignReference,
            "/cells",
        )?;
        let guests = c.occasion.0.as_ref().map_or(0, |o| o.guests.len());
        require(
            c.attendee_ids.len() + guests <= 64
                && (!cell_enabled(w, c) || c.attendee_ids.len() + guests > 0),
            ErrorCode::InvalidValue,
            "/cells/attendee_ids",
        )?;
        if let Some(o) = &c.occasion.0 {
            require(
                o.host_diner_ids == c.attendee_ids,
                ErrorCode::IdentityMismatch,
                "/cells/occasion/host_diner_ids",
            )?;
        }
    }
    require(
        w.plan
            .selections
            .iter()
            .all(|s| w.cells.iter().any(|c| c.slot == s.slot)),
        ErrorCode::ForeignReference,
        "/plan/selections",
    )
}
fn hard(d: &RestrictionDeclaration) -> Option<Vec<HardRequirement>> {
    match d {
        RestrictionDeclaration::Unconfirmed {} => None,
        RestrictionDeclaration::ConfirmedNone { .. } => Some(vec![]),
        RestrictionDeclaration::Confirmed { requirements, .. } => Some(requirements.0.clone()),
    }
}
fn union(items: impl Iterator<Item = HardRequirement>) -> Vec<String> {
    items
        .map(|x| key(&x))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}
fn validate_requirements(r: &MealRequirements) -> Result<(), DomainError> {
    require(
        r.enabled == r.eat_by.0.is_some(),
        ErrorCode::InvalidValue,
        "/eat_by",
    )?;
    if let Some(moment) = &r.eat_by.0 {
        // Validate the child, never recursively validate this requirement here.
        moment.validate_shape().map_err(|mut issues| {
            for issue in &mut issues {
                issue.path = format!("/eat_by{}", issue.path);
            }
            crate::parse::shape_error(issues)
        })?;
    }
    require(
        !r.enabled || r.location == MealLocation::Skip || !r.attendees.is_empty(),
        ErrorCode::InvalidValue,
        "/attendees",
    )?;
    require(
        r.preferred_hands_on_minutes <= 1440,
        ErrorCode::LimitExceeded,
        "/preferred_hands_on_minutes",
    )?;
    require(
        union(
            r.attendees
                .iter()
                .flat_map(|a| a.hard_requirements.0.clone()),
        ) == union(r.shared_hard_requirements.0.clone().into_iter()),
        ErrorCode::DomainConflict,
        "/shared_hard_requirements",
    )
}
pub fn validate_context(c: &PlanningContext) -> Result<(), Vec<ValidationIssue>> {
    validate_context_inner(c).map_err(|e| e.issues.0)
}
fn validate_context_inner(c: &PlanningContext) -> Result<(), DomainError> {
    // Public callers may construct malformed primitives directly. Check children
    // before cross-record lookup/activity checks, without recursing into context.
    c.weeks
        .validate_shape()
        .map_err(crate::parse::shape_error)?;
    c.requirements
        .validate_shape()
        .map_err(crate::parse::shape_error)?;
    validate_stamp(&c.stamp)?;
    require(
        c.stamp.household_revision == c.household.revision
            && c.stamp.settings_revision == c.settings_revision
            && c.stamp.privacy_revision == c.privacy.revision
            && c.stamp.history_revision == c.history.revision
            && c.stamp.ranking_policy_version == c.ranking_policy.version,
        ErrorCode::ContextStale,
        "/stamp",
    )?;
    require(
        c.household.connections.privacy == c.privacy,
        ErrorCode::ContextStale,
        "/privacy",
    )?;
    require(
        c.weeks.len() == c.stamp.read_weeks.len()
            && c.weeks.iter().all(|w| {
                c.stamp
                    .read_weeks
                    .iter()
                    .any(|r| r.week_id == w.range.week_id && r.revision == w.revision)
            }),
        ErrorCode::ContextStale,
        "/weeks",
    )?;
    require(
        c.recipes.len() == c.stamp.recipe_refs.len(),
        ErrorCode::MissingReference,
        "/recipes",
    )?;
    for rf in c.stamp.recipe_refs.iter() {
        let r = c
            .recipes
            .iter()
            .find(|r| r.recipe_id == rf.recipe_id && r.revision == rf.revision)
            .ok_or_else(|| err(ErrorCode::MissingReference, "/recipes"))?;
        require(
            recipe_digest(r)? == rf.digest,
            ErrorCode::DigestMismatch,
            "/recipes",
        )?;
    }
    require(
        c.requirements.len() == c.weeks.len() * 21,
        ErrorCode::MissingReference,
        "/requirements",
    )?;
    let diners: BTreeMap<_, _> = c
        .household
        .diners
        .iter()
        .map(|d| (&d.diner_id, d))
        .collect();
    for w in c.weeks.iter() {
        validate_week(w)?;
        for cell in w.cells.iter() {
            let r = c
                .requirements
                .iter()
                .find(|r| r.slot == cell.slot)
                .ok_or_else(|| err(ErrorCode::MissingReference, "/requirements"))?;
            require(
                r.enabled == cell_enabled(w, cell) && r.location == cell.choice,
                ErrorCode::DomainConflict,
                "/requirements/enabled",
            )?;
            let mut expected = Vec::new();
            for id in cell.attendee_ids.iter() {
                let d = diners
                    .get(id)
                    .ok_or_else(|| err(ErrorCode::ForeignReference, "/cells/attendee_ids"))?;
                expected.push((
                    PersonRef::Diner {
                        diner_id: id.clone(),
                    },
                    d.age_band,
                    hard(&d.restrictions).ok_or_else(|| {
                        err(ErrorCode::DomainConflict, "/household/diners/restrictions")
                    })?,
                ));
            }
            if let Some(o) = &cell.occasion.0 {
                for g in o.guests.iter() {
                    expected.push((
                        PersonRef::Guest {
                            occasion_id: o.occasion_id.clone(),
                            guest_id: g.guest_id.clone(),
                        },
                        g.age_band,
                        hard(&g.restrictions).ok_or_else(|| {
                            err(ErrorCode::DomainConflict, "/occasion/guests/restrictions")
                        })?,
                    ));
                }
            }
            if r.enabled {
                require(
                    r.attendees.len() == expected.len(),
                    ErrorCode::ForeignReference,
                    "/requirements/attendees",
                )?;
            }
            for a in r.attendees.iter() {
                let (_, age, req) = expected
                    .iter()
                    .find(|(p, _, _)| p == &a.person)
                    .ok_or_else(|| err(ErrorCode::ForeignReference, "/requirements/attendees"))?;
                require(
                    a.age_band == *age
                        && union(a.hard_requirements.0.clone().into_iter())
                            == union(req.clone().into_iter()),
                    ErrorCode::DomainConflict,
                    "/requirements/attendees/hard_requirements",
                )?;
            }
            validate_requirements(r)?;
        }
        for s in w.plan.selections.iter() {
            check_ref(&s.recipe, c)?;
            let selected_cell = w
                .cells
                .iter()
                .find(|cell| cell.slot == s.slot)
                .ok_or_else(|| err(ErrorCode::ForeignReference, "/plan/selections"))?;
            for serving in s.servings.iter() {
                require(
                    person_known_for_cell(&serving.person, c, selected_cell),
                    ErrorCode::ForeignReference,
                    "/selections/servings/person",
                )?;
            }
            for portion in s.meal_overrides.portions.iter() {
                require(
                    person_known_for_cell(&portion.person, c, selected_cell),
                    ErrorCode::ForeignReference,
                    "/selections/meal_overrides/portions/person",
                )?;
            }
            let recipe = c
                .recipes
                .iter()
                .find(|r| r.recipe_id == s.recipe.recipe_id && r.revision == s.recipe.revision)
                .ok_or_else(|| err(ErrorCode::MissingReference, "/selections/recipe"))?;
            require(
                !matches!(s.batch_use, BatchUse::NewBatch { .. })
                    || recipe.reused_inputs.is_empty(),
                ErrorCode::DomainConflict,
                "/selections/batch_use",
            )?;
            for e in s.extras.iter() {
                check_ref(&e.recipe, c)?;
                for recipient in e.recipients.iter() {
                    check_preset_ref(
                        &recipient.preset_ref,
                        &e.recipe,
                        c,
                        "/selections/extras/recipients/preset_ref",
                    )?;
                    require(
                        person_known_for_cell(&recipient.person, c, selected_cell),
                        ErrorCode::ForeignReference,
                        "/selections/extras/recipients/person",
                    )?;
                }
                require(
                    recipe
                        .extras
                        .iter()
                        .any(|x| x.extra_id == e.extra_id && x.recipe == e.recipe),
                    ErrorCode::ForeignReference,
                    "/selections/extras",
                )?;
                require(
                    e.recipients
                        .iter()
                        .all(|p| s.servings.iter().any(|x| x.person == p.person)),
                    ErrorCode::ForeignReference,
                    "/selections/extras/recipients",
                )?;
            }
            for p in s.servings.iter() {
                check_preset_ref(
                    &p.preset_ref,
                    &s.recipe,
                    c,
                    "/selections/servings/preset_ref",
                )?;
            }
            if active_selections(w).contains(&s) {
                let req = c
                    .requirements
                    .iter()
                    .find(|r| r.slot == s.slot)
                    .ok_or_else(|| err(ErrorCode::MissingReference, "/requirements"))?;
                require(
                    s.servings.len() == req.attendees.len()
                        && s.servings
                            .iter()
                            .all(|p| req.attendees.iter().any(|a| a.person == p.person)),
                    ErrorCode::ForeignReference,
                    "/selections/servings",
                )?;
            }
        }
    }
    for r in c.recipes.iter() {
        for e in r.extras.iter() {
            check_ref(&e.recipe, c)?;
        }
        for input in r.reused_inputs.iter() {
            check_ref(&input.source_recipe, c)?;
            let source = c
                .recipes
                .iter()
                .find(|source| {
                    source.recipe_id == input.source_recipe.recipe_id
                        && source.revision == input.source_recipe.revision
                })
                .ok_or_else(|| {
                    err(
                        ErrorCode::MissingReference,
                        "/recipes/reused_inputs/source_recipe",
                    )
                })?;
            require(
                source
                    .components
                    .iter()
                    .any(|component| component.component_id == input.source_component_id),
                ErrorCode::MissingReference,
                "/recipes/reused_inputs/source_component_id",
            )?;
            require(
                source.reused_inputs.is_empty(),
                ErrorCode::DomainConflict,
                "/recipes/reused_inputs/source_recipe",
            )?;
        }
    }
    for selected in c
        .history
        .selected
        .iter()
        .chain(c.history.ordered.iter().map(|ordered| &ordered.selection))
    {
        check_ref(&selected.recipe, c)?;
        for serving in selected.servings.iter() {
            check_preset_ref(
                &serving.preset_ref,
                &selected.recipe,
                c,
                "/history/servings/preset_ref",
            )?;
        }
        for extra in selected.extras.iter() {
            check_ref(&extra.recipe, c)?;
            for recipient in extra.recipients.iter() {
                check_preset_ref(
                    &recipient.preset_ref,
                    &extra.recipe,
                    c,
                    "/history/extras/recipients/preset_ref",
                )?;
            }
        }
    }
    for feedback in c.history.feedback.iter() {
        check_ref(&feedback.recipe, c)?;
    }
    let edges: cxx::Edges = c
        .recipes
        .iter()
        .map(|r| {
            (
                format!("{}:{}", r.recipe_id.0, r.revision.0),
                r.extras
                    .iter()
                    .map(|e| format!("{}:{}", e.recipe.recipe_id.0, e.recipe.revision.0))
                    .chain(r.reused_inputs.iter().map(|input| {
                        format!(
                            "{}:{}",
                            input.source_recipe.recipe_id.0, input.source_recipe.revision.0
                        )
                    }))
                    .collect(),
            )
        })
        .collect();
    acyclic(&edges, "/recipes/extras")
}
fn person_known_for_cell(person: &PersonRef, context: &PlanningContext, cell: &Cell) -> bool {
    match person {
        PersonRef::Diner { diner_id } => context
            .household
            .diners
            .iter()
            .any(|diner| diner.diner_id == *diner_id),
        PersonRef::Guest {
            occasion_id,
            guest_id,
        } => cell.occasion.0.as_ref().is_some_and(|occasion| {
            occasion.occasion_id == *occasion_id
                && occasion
                    .guests
                    .iter()
                    .any(|guest| guest.guest_id == *guest_id)
        }),
    }
}
mod cxx {
    pub type Edges = std::collections::BTreeMap<String, Vec<String>>;
}
fn check_ref(r: &RecipeRef, c: &PlanningContext) -> Result<(), DomainError> {
    require(
        c.stamp.recipe_refs.contains(r),
        ErrorCode::MissingReference,
        "/recipe_refs",
    )
}
fn check_preset_ref(
    preset_ref: &PortionPresetRef,
    serving_recipe: &RecipeRef,
    context: &PlanningContext,
    path: &str,
) -> Result<(), DomainError> {
    check_ref(&preset_ref.recipe, context)?;
    require(
        &preset_ref.recipe == serving_recipe,
        ErrorCode::ForeignReference,
        path,
    )?;
    let recipe = context
        .recipes
        .iter()
        .find(|recipe| {
            recipe.recipe_id == preset_ref.recipe.recipe_id
                && recipe.revision == preset_ref.recipe.revision
        })
        .ok_or_else(|| err(ErrorCode::MissingReference, path))?;
    require(
        recipe.portion_presets.iter().any(|preset| {
            preset.preset == preset_ref.preset
                && preset.age_band == preset_ref.age_band
                && preset.meal_type == preset_ref.meal_type
        }),
        ErrorCode::MissingReference,
        path,
    )
}
fn acyclic(edges: &cxx::Edges, path: &str) -> Result<(), DomainError> {
    fn visit(
        n: &str,
        e: &cxx::Edges,
        active: &mut BTreeSet<String>,
        done: &mut BTreeSet<String>,
    ) -> bool {
        if done.contains(n) {
            return true;
        }
        if !active.insert(n.into()) {
            return false;
        }
        if !e
            .get(n)
            .is_some_and(|a| a.iter().all(|x| visit(x, e, active, done)))
        {
            return false;
        }
        active.remove(n);
        done.insert(n.into());
        true
    }
    let mut done = BTreeSet::new();
    for n in edges.keys() {
        if !visit(n, edges, &mut BTreeSet::new(), &mut done) {
            return Err(err(ErrorCode::DomainConflict, path));
        }
    }
    Ok(())
}
fn validate_recipe(r: &RecipeRevision) -> Result<(), DomainError> {
    require(
        r.active_minutes <= r.elapsed_minutes && r.elapsed_minutes <= 10080,
        ErrorCode::InvalidValue,
        "/elapsed_minutes",
    )?;
    let components: BTreeSet<_> = r.components.iter().map(|c| &c.component_id).collect();
    let yielded: BTreeSet<_> = r
        .r#yield
        .components
        .iter()
        .map(|c| &c.component_id)
        .collect();
    require(
        components == yielded,
        ErrorCode::MissingReference,
        "/yield/components",
    )?;
    let ingredients: BTreeSet<_> = r.ingredients.iter().map(|i| &i.ingredient_id).collect();
    let steps: BTreeSet<_> = r.steps.iter().map(|s| s.step_id).collect();
    let external: BTreeSet<_> = r
        .reused_inputs
        .iter()
        .map(|input| &input.source_component_id)
        .collect();
    require(
        components.is_disjoint(&external),
        ErrorCode::ForeignReference,
        "/reused_inputs/source_component_id",
    )?;
    for input in r.reused_inputs.iter() {
        require(
            input.source_recipe.recipe_id != r.recipe_id
                || input.source_recipe.revision != r.revision,
            ErrorCode::DomainConflict,
            "/reused_inputs/source_recipe",
        )?;
        require(
            input.step_ids.iter().all(|id| {
                r.steps.iter().any(|step| {
                    step.step_id == *id && step.component_ids.contains(&input.source_component_id)
                })
            }),
            ErrorCode::MissingReference,
            "/reused_inputs/step_ids",
        )?;
        require(
            input.evidence_ids.iter().all(|id| {
                r.reviews.iter().any(|review| review.evidence_id == *id)
                    || r.dietary_evidence
                        .iter()
                        .any(|evidence| evidence.evidence_id == *id)
                    || r.storage.iter().any(|rule| rule.evidence_id == *id)
                    || r.ingredients.iter().any(|ingredient| {
                        ingredient
                            .conversion_evidence
                            .iter()
                            .any(|evidence| evidence.evidence_id == *id)
                    })
            }),
            ErrorCode::MissingReference,
            "/reused_inputs/evidence_ids",
        )?;
    }
    for component in r.components.iter() {
        if let Some(anchor) = &component.production.0 {
            let id = match anchor {
                ComponentProductionSpec::Cooked { completion_prep_id } => completion_prep_id,
                ComponentProductionSpec::Unheated {
                    first_handling_prep_id,
                } => first_handling_prep_id,
            };
            require(
                r.prep.iter().any(|phase| {
                    phase.prep_id == *id
                        && phase.step_ids.iter().any(|step_id| {
                            r.steps.iter().any(|step| {
                                step.step_id == *step_id
                                    && step.component_ids.contains(&component.component_id)
                            })
                        })
                }),
                ErrorCode::MissingReference,
                "/components/production",
            )?;
        }
        require(
            component
                .ingredient_ids
                .iter()
                .all(|ingredient| ingredients.contains(ingredient)),
            ErrorCode::MissingReference,
            "/components/ingredient_ids",
        )?;
        require(
            component.storage_rule_ids.iter().all(|rule_id| {
                r.storage.iter().any(|rule| {
                    rule.evidence_id == *rule_id && rule.component_id == component.component_id
                })
            }),
            ErrorCode::MissingReference,
            "/components/storage_rule_ids",
        )?;
    }
    for storage in r.storage.iter() {
        require(
            components.contains(&storage.component_id),
            ErrorCode::MissingReference,
            "/storage/component_id",
        )?;
    }
    for extra in r.extras.iter() {
        require(
            extra
                .supported_with
                .iter()
                .all(|component_id| components.contains(component_id)),
            ErrorCode::MissingReference,
            "/extras/supported_with",
        )?;
    }
    for i in r.ingredients.iter() {
        require(
            components.contains(&i.component_id) && i.step_ids.iter().all(|s| steps.contains(s)),
            ErrorCode::MissingReference,
            "/ingredients",
        )?;
        for e in i.conversion_evidence.iter() {
            require(
                e.ingredient_id == i.ingredient_id,
                ErrorCode::ForeignReference,
                "/ingredients/conversion_evidence",
            )?;
        }
    }
    for s in r.steps.iter() {
        require(
            s.ingredient_ids.iter().all(|i| ingredients.contains(i))
                && s.component_ids.iter().all(|i| {
                    components.contains(i)
                        || r.reused_inputs.iter().any(|input| {
                            input.source_component_id == *i && input.step_ids.contains(&s.step_id)
                        })
                }),
            ErrorCode::MissingReference,
            "/steps",
        )?;
    }
    let graph = r
        .steps
        .iter()
        .map(|s| {
            (
                s.step_id.to_string(),
                s.depends_on.iter().map(ToString::to_string).collect(),
            )
        })
        .collect();
    acyclic(&graph, "/steps/depends_on")?;
    let prep = r
        .prep
        .iter()
        .map(|s| {
            (
                s.prep_id.to_string(),
                s.depends_on.iter().map(ToString::to_string).collect(),
            )
        })
        .collect();
    acyclic(&prep, "/prep/depends_on")?;
    for p in r.prep.iter() {
        require(
            p.step_ids.iter().all(|s| steps.contains(s))
                && p.ingredient_ids.iter().all(|i| ingredients.contains(i)),
            ErrorCode::MissingReference,
            "/prep",
        )?;
    }
    if r.capabilities.shopping {
        require(
            r.ingredients.iter().all(|i| i.quantity.0.is_some())
                && r.reviews
                    .iter()
                    .any(|x| x.kind == ReviewKind::QuantityReview),
            ErrorCode::DomainConflict,
            "/capabilities/shopping",
        )?;
    }
    if r.capabilities.leftovers {
        require(
            components
                .iter()
                .all(|id| r.storage.iter().any(|s| &s.component_id == *id)),
            ErrorCode::DomainConflict,
            "/capabilities/leftovers",
        )?;
    }
    if r.capabilities.starter_distribution {
        require(
            r.text_rights.distribution == RightsDistribution::Permitted
                && !r.text_rights.evidence_ids.is_empty()
                && r.image.0.as_ref().is_some_and(|i| {
                    i.rights.distribution == RightsDistribution::Permitted
                        && !i.rights.evidence_ids.is_empty()
                }),
            ErrorCode::DomainConflict,
            "/capabilities/starter_distribution",
        )?;
    }
    for p in r.portion_presets.iter() {
        require(
            !p.evidence_ids.is_empty(),
            ErrorCode::MissingReference,
            "/portion_presets/evidence_ids",
        )?;
    }
    Ok(())
}
pub fn validate_conversion(
    e: &ConversionEvidence,
    ingredient: &IngredientId,
    from_unit: Unit,
    from_form: IngredientForm,
    to_unit: Unit,
    to_form: IngredientForm,
) -> Result<(), DomainError> {
    require(
        &e.ingredient_id == ingredient
            && e.from_unit == from_unit
            && e.from_form == from_form
            && e.to_unit == to_unit
            && e.to_form == to_form,
        ErrorCode::DomainConflict,
        "/conversion_evidence",
    )
}
fn validate_event(e: &CalendarEvent) -> Result<(), DomainError> {
    require(
        e.start.instant.0 <= e.end.instant.0,
        ErrorCode::InvalidValue,
        "/end",
    )?;
    let mut material = serde_json::to_value(e).map_err(|_| err(ErrorCode::InvalidValue, ""))?;
    material
        .as_object_mut()
        .ok_or_else(|| err(ErrorCode::InvalidValue, ""))?
        .remove("event_payload_digest");
    material["schema_version"] = Value::from(1);
    require(
        canonical_digest(&material)?.0 == e.event_payload_digest.0,
        ErrorCode::DigestMismatch,
        "/event_payload_digest",
    )
}
