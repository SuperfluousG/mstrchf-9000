//! Query-bound mechanics with exact compiled phase evidence. The isolated lunch
//! and placeholder declaration digest do not claim a valid complete food plan;
//! native derivation remains the authority after an owner enters this horizon.
use super::*;
use crate::maker_packets::{available_packet_refs, known_packet};

fn moment(day: u8, time: &str, resolution: Resolution) -> ZonedMoment {
    mstrchf_core::availability::resolve_meal_time(
        &LocalDate(format!("2026-10-{day:02}")),
        &LocalTime(time.into()),
        &ZoneId("Europe/London".into()),
        Some(resolution),
    )
    .unwrap()
}

fn fixture(day: u8) -> (RuntimeCatalogue, HouseholdProfile, Week) {
    let packets = available_packet_refs().unwrap();
    let hot = packets
        .iter()
        .find(|packet| {
            packet.recipe.recipe_id.0 == crate::personal_scope::PORTABLE_HOT_SCOPE.recipe_id
        })
        .unwrap();
    let cold = packets
        .iter()
        .find(|packet| {
            packet.recipe.recipe_id.0 == crate::personal_scope::PORTABLE_COLD_SCOPE.recipe_id
        })
        .unwrap();
    let view = RuntimeCatalogue::from_records(&[
        known_packet(hot).unwrap().record().clone(),
        known_packet(cold).unwrap().record().clone(),
    ])
    .unwrap();
    let state: BrowserState = serde_json::from_value(
        serde_json::from_slice::<serde_json::Value>(include_bytes!(
            "../fixtures/legacy-engineering-receipts.json"
        ))
        .unwrap()["state"]
            .clone(),
    )
    .unwrap();
    let profile = state.snapshot.profile.0.unwrap();
    let mut week = state.snapshot.weeks[0].clone();
    let mut selected = week.plan.selections[0].clone();
    let index = usize::from(day - 19) * 3 + 1;
    for cell in week.cells.iter_mut() {
        cell.choice = MealLocation::Skip;
    }
    week.day_enabled = BoundedVec(vec![true; 7]);
    week.meal_type_enabled.lunch = true;
    week.cells[index].choice = MealLocation::OnTheMove;
    selected.slot = week.cells[index].slot.clone();
    selected.recipe = cold.recipe.clone();
    selected.origin = SelectionOrigin::Fixed;
    selected.extras.0.clear();
    for serving in selected.servings.iter_mut() {
        serving.preset_ref.recipe = selected.recipe.clone();
        serving.preset_ref.meal_type = MealType::Lunch;
    }
    selected.batch_use = BatchUse::Leftover {
        batch_id: BatchId(format!("ba_{:032x}", 0xfd99)),
        component_ids: BoundedVec(vec![
            view.resolve_recipe(&cold.recipe).unwrap().reused_inputs[0]
                .source_component_id
                .clone(),
        ]),
    };
    selected.meal_overrides.eat_by =
        Nullable(Some(moment(day, "12:30:00", Resolution::Unambiguous)));
    selected.portable_cold = Some(PortableColdJourney {
        packing_complete_at: moment(day, "07:30:00", Resolution::Unambiguous),
        depart_at: moment(day, "08:00:00", Resolution::Unambiguous),
        declaration: PortableChillingDeclaration {
            schema_version: SchemaVersion,
            confirmed_at: Instant("2026-10-18T12:00:00Z".into()),
            journey_digest: PayloadDigest("0".repeat(64)),
        },
    });
    week.plan.selections = BoundedVec(vec![selected]);
    week.plan.batch_links.0.clear();
    (view, profile, week)
}

#[test]
fn packing_only_overlap_enters_the_query_and_departure_does_not_become_work() {
    let (view, profile, mut week) = fixture(20);
    let span = declared_span(&view, &profile, &week).unwrap().unwrap();
    assert_eq!(
        span,
        (
            seconds(&moment(20, "07:25:00", Resolution::Unambiguous).instant).unwrap(),
            seconds(&moment(20, "12:30:00", Resolution::Unambiguous).instant).unwrap()
        )
    );
    let morning = (span.0, span.0 + 60);
    assert!(intersects(span, morning));
    week.plan.selections[0]
        .portable_cold
        .as_mut()
        .unwrap()
        .depart_at = moment(20, "10:00:00", Resolution::Unambiguous);
    assert_eq!(declared_span(&view, &profile, &week).unwrap(), Some(span));
    week.plan.selections[0].portable_cold = None;
    let ordinary = declared_span(&view, &profile, &week).unwrap().unwrap();
    assert_eq!(ordinary, (span.1 - 300, span.1));
    assert!(
        !intersects(ordinary, morning),
        "This is the overlap the old meal-only anchor omitted"
    );
}

#[test]
fn packing_uses_its_resolved_fold_instant_while_unresolved_service_keeps_both_candidates() {
    let (view, mut profile, mut week) = fixture(25);
    week.plan.selections[0]
        .portable_cold
        .as_mut()
        .unwrap()
        .packing_complete_at = moment(25, "01:30:00", Resolution::Earlier);
    let earlier = declared_span(&view, &profile, &week).unwrap().unwrap();
    week.plan.selections[0]
        .portable_cold
        .as_mut()
        .unwrap()
        .packing_complete_at = moment(25, "01:30:00", Resolution::Later);
    let later = declared_span(&view, &profile, &week).unwrap().unwrap();
    assert_eq!(later.0 - earlier.0, 3600);
    assert_eq!(later.1, earlier.1);
    week.plan.selections[0].meal_overrides.eat_by = Nullable(None);
    let journey = week.plan.selections[0].portable_cold.as_mut().unwrap();
    journey.packing_complete_at = moment(25, "00:30:00", Resolution::Unambiguous);
    journey.depart_at = moment(25, "00:45:00", Resolution::Unambiguous);
    profile
        .preferences
        .meal_times
        .iter_mut()
        .find(|time| time.meal_type == MealType::Lunch)
        .unwrap()
        .time = LocalTime("01:30:00".into());
    let unresolved = declared_span(&view, &profile, &week).unwrap().unwrap();
    assert_eq!(
        unresolved.0,
        seconds(&moment(25, "00:25:00", Resolution::Unambiguous).instant).unwrap()
    );
    assert_eq!(
        unresolved.1,
        seconds(&moment(25, "01:30:00", Resolution::Later).instant).unwrap()
    );
}

#[test]
fn active_declaration_without_exact_native_method_is_unknown_but_inactive_work_is_ignored() {
    let (view, profile, mut week) = fixture(20);
    let ordinary = view
        .recipes()
        .iter()
        .find(|recipe| recipe.reused_inputs.is_empty())
        .unwrap();
    let reference = RecipeRef {
        recipe_id: ordinary.recipe_id.clone(),
        revision: ordinary.revision,
        digest: recipe_digest(ordinary).unwrap(),
    };
    week.plan.selections[0].recipe = reference.clone();
    for serving in week.plan.selections[0].servings.iter_mut() {
        serving.preset_ref.recipe = reference.clone();
    }
    let error = declared_span(&view, &profile, &week).unwrap_err();
    assert_eq!(error.code, ErrorCode::MissingReference);
    assert!(
        error
            .issues
            .iter()
            .any(|issue| issue.path == "/portable_cold/method")
    );
    week.day_enabled[1] = false;
    assert_eq!(declared_span(&view, &profile, &week).unwrap(), None);
}
