//! The existing pure native planner, parameterised by an explicit trusted view.
use crate::catalogue::RuntimeCatalogue;
include!(concat!(env!("OUT_DIR"), "/planner_view_shared.rs"));

const HOME_ORIGINAL_SOURCE_ID: &str = "rc_8000000000000000000000000000000b";
const HOME_ORIGINAL_COLD_ID: &str = "rc_8000000000000000000000000000000c";

fn is_home_original_cold(reference: &RecipeRef) -> bool {
    reference.recipe_id.0 == HOME_ORIGINAL_COLD_ID || reference.recipe_id.0 == crate::portable::COLD_ID
}

fn is_original_component_service(reference: &RecipeRef) -> bool {
    is_home_original_cold(reference) || crate::personal_scope::is_chilli_service(reference)
}

fn original_service_input(recipe: &RecipeRevision) -> Result<&ReusedComponentInput, DomainError> {
    if [crate::personal_scope::CHILLI_WRAPS_SCOPE.recipe_id, crate::personal_scope::CHILLI_POTATO_SCOPE.recipe_id]
        .contains(&recipe.recipe_id.0.as_str()) {
        crate::personal_scope::chilli_service_input(recipe)
    } else { original_cold_input(recipe) }
}

/// The compiled view and actual source selection establish full-reference
/// custody. This branch assembles a candidate only; complete native derivation
/// still establishes portions, credit, original clocks and cooling dependencies.
fn choice_source<'a>(recipe: &RecipeRevision, reference: &RecipeRef, week: &'a Week,
    target: &SlotId, options: &ChoiceOptions) -> Result<Option<&'a MealSelection>, DomainError> {
    if !is_original_component_service(reference) {
        return Ok(options.leftover_source.as_ref().and_then(|slot|week.plan.selections.iter()
            .find(|selection|&selection.slot==slot && selection.recipe==*reference
                && matches!(selection.batch_use,BatchUse::NewBatch{..}))));
    }
    let input = original_service_input(recipe)?;
    if options.make_extra || !options.extra_recipients.is_empty() || options.leftover_source.is_none() {
        return Ok(None);
    }
    let source = week.plan.selections.iter().find(|selection|
        options.leftover_source.as_ref()==Some(&selection.slot)
            && &selection.slot!=target && selection.recipe==input.source_recipe
            && selection.extras.is_empty() && matches!(selection.batch_use,BatchUse::NewBatch{..}));
    Ok(source)
}

fn original_cold_input(recipe: &RecipeRevision) -> Result<&ReusedComponentInput, DomainError> {
    let source_id = if recipe.recipe_id.0 == crate::portable::COLD_ID {
        crate::portable::SOURCE_ID
    } else {HOME_ORIGINAL_SOURCE_ID};
    if ![HOME_ORIGINAL_COLD_ID,crate::portable::COLD_ID].contains(&recipe.recipe_id.0.as_str()) || !recipe.ingredients.is_empty()
        || !recipe.extras.is_empty() || recipe.reused_inputs.len()!=1
        || recipe.reused_inputs[0].source_recipe.recipe_id.0!=source_id
        || recipe.reused_inputs[0].planned_reheats!=0 {
        return Err(error(ErrorCode::UnsupportedCapability,"/original_component/cold_definition"));
    }
    Ok(&recipe.reused_inputs[0])
}

fn choice_component_ids(recipe: &RecipeRevision) -> Result<BoundedVec<ComponentId,1,128>,DomainError> {
    if [HOME_ORIGINAL_COLD_ID,crate::portable::COLD_ID,crate::personal_scope::CHILLI_WRAPS_SCOPE.recipe_id,
        crate::personal_scope::CHILLI_POTATO_SCOPE.recipe_id].contains(&recipe.recipe_id.0.as_str()) {
        return Ok(BoundedVec(vec![original_service_input(recipe)?.source_component_id.clone()]));
    }
    Ok(BoundedVec(recipe.components.iter().map(|component|component.component_id.clone()).collect()))
}

fn choice_link_components(recipe: &RecipeRevision, scaled: &mstrchf_core::portions::ScaledRecipe)
    ->Result<BoundedVec<ComponentAllocation,1,128>,DomainError> {
    if [HOME_ORIGINAL_COLD_ID,crate::portable::COLD_ID,crate::personal_scope::CHILLI_WRAPS_SCOPE.recipe_id,
        crate::personal_scope::CHILLI_POTATO_SCOPE.recipe_id].contains(&recipe.recipe_id.0.as_str()) {
        let input=original_service_input(recipe)?;
        return Ok(BoundedVec(vec![ComponentAllocation {
            component_id:input.source_component_id.clone(),
            quantity:Quantity {
                amount:PositiveRational(input.quantity_per_recipe_yield.amount.0.checked_mul(&scaled.multiplier.0)?),
                ..input.quantity_per_recipe_yield.clone()
            },
        }]));
    }
    Ok(BoundedVec(scaled.components.iter().map(|component|ComponentAllocation {
        component_id:component.component_id.clone(),quantity:component.quantity.clone(),
    }).collect()))
}

fn phase_session_role(recipe: &RecipeRevision, reference: &RecipeRef, prep: &PrepSpec)
    ->Result<SessionRole,DomainError> {
    if recipe.recipe_id.0 == crate::personal_scope::OVERNIGHT_OATS_SCOPE.recipe_id {
        if !crate::personal_scope::is_overnight_oats(reference) {
            return Err(error(ErrorCode::IdentityMismatch, "/overnight_oats/recipe"));
        }
        return match prep.prep_id {
            1 => Ok(SessionRole::AdvancePrep),
            2 => Ok(SessionRole::Cooking),
            _ => Err(error(ErrorCode::UnsupportedCapability, "/overnight_oats/phase")),
        };
    }
    if recipe.recipe_id.0 == crate::personal_scope::CHILLI_BATCH_SCOPE.recipe_id {
        if recipe.components.len()!=1 || !matches!(&recipe.components[0].production.0,
            Some(ComponentProductionSpec::CookedWithCooling {completion_prep_id:3,cooling_prep_ids,..})
                if cooling_prep_ids.0==vec![4,5,6]) {
            return Err(error(ErrorCode::UnsupportedCapability,"/original_component/chilli_cooling"));
        }
        return match prep.prep_id {
            1..=3=>Ok(SessionRole::Cooking),
            4..=6=>Ok(SessionRole::AdvancePrep),
            _=>Err(error(ErrorCode::UnsupportedCapability,"/original_component/chilli_phase")),
        };
    }
    if crate::personal_scope::is_chilli_service(reference) {
        let maximum=if recipe.recipe_id.0==crate::personal_scope::CHILLI_WRAPS_SCOPE.recipe_id {3}else{5};
        return if (1..=maximum).contains(&prep.prep_id) {Ok(SessionRole::Cooking)}
            else {Err(error(ErrorCode::UnsupportedCapability,"/original_component/chilli_phase"))};
    }
    if [HOME_ORIGINAL_SOURCE_ID,crate::portable::SOURCE_ID].contains(&recipe.recipe_id.0.as_str()) {
        let mut cooling = Vec::new();
        for component in recipe.components.iter() {
            if let Some(ComponentProductionSpec::CookedWithCooling {cooling_prep_ids,..})=&component.production.0 {
                cooling.extend(cooling_prep_ids.iter().copied());
            }
        }
        if cooling.is_empty() {return Err(error(ErrorCode::UnsupportedCapability,"/original_component/source_cooling"));}
        return Ok(if cooling.contains(&prep.prep_id) {SessionRole::AdvancePrep} else {SessionRole::Cooking});
    }
    if recipe.recipe_id.0 == crate::portable::COLD_ID {
        return match prep.prep_id {
            1 => Ok(SessionRole::AdvancePrep),
            2 => Ok(SessionRole::Cooking),
            _ => Err(error(ErrorCode::UnsupportedCapability,"/portable_cold/phase")),
        };
    }
    Ok(if crate::personal_scope::cold_home_timing(reference)?.is_some() || prep.latest_offset_seconds < -3600 {
        SessionRole::AdvancePrep
    } else {SessionRole::Cooking})
}

#[cfg(test)]
#[path = "../tests/support/original_component_choices.rs"]
mod original_component_choices;

#[cfg(test)]
#[path = "../tests/support/chilli_component_choices.rs"]
mod chilli_component_choices;

/// A saved, previously admitted two-unit meal may retain its exact allocations
/// after its temporary guest leaves. The new exact extra-capable pair may also
/// retain a still-defined resident deselected from this meal. This projection supplies no food authority:
/// callers retain the full saved plan, report a conflict, and derive every new
/// selection against the complete replacement candidate.
fn two_unit_guest_projection(view: &RuntimeCatalogue, week: &Week,
    edited_cells: &[Cell], target: Option<&SlotId>) -> Result<Option<Week>, DomainError> {
    let mut omitted = Vec::new();
    for selection in week.plan.selections.iter().filter(|selection|target.is_none_or(|slot|&selection.slot==slot)) {
        let gf_pair=selection.recipe.recipe_id.0==crate::personal_scope::THREE_BEAN_GF_CHICKEN_SCOPE.recipe_id;
        let paired=selection.recipe.recipe_id.0==crate::personal_scope::THREE_BEAN_TWO_UNIT_CHICKEN_EXTRA_SCOPE.recipe_id || gf_pair;
        let exact = if selection.recipe.recipe_id.0 == crate::personal_scope::THREE_BEAN_TWO_UNIT_SCOPE.recipe_id {
            crate::maker_packets::three_bean_two_unit_packet_ref()?.recipe
        } else if selection.recipe.recipe_id.0 == crate::personal_scope::THREE_BEAN_TWO_UNIT_VEGETARIAN_SCOPE.recipe_id {
            crate::maker_packets::three_bean_two_unit_vegetarian_packet_ref()?.recipe
        } else if gf_pair {
            let scope=&crate::personal_scope::THREE_BEAN_GF_CHICKEN_SCOPE;
            RecipeRef{recipe_id:RecipeId(scope.recipe_id.into()),revision:Revision(scope.recipe_revision),digest:RecipeDigest(scope.recipe_digest.into())}
        } else if paired {
            RecipeRef{recipe_id:RecipeId(crate::personal_scope::THREE_BEAN_TWO_UNIT_CHICKEN_EXTRA_SCOPE.recipe_id.into()),revision:Revision(1),digest:RecipeDigest(crate::personal_scope::THREE_BEAN_TWO_UNIT_CHICKEN_EXTRA_SCOPE.recipe_digest.into())}
        } else {continue;};
        if selection.recipe != exact {return Err(error(ErrorCode::IdentityMismatch,"/personal_use_scope/recipe_reference"));}
        view.resolve_recipe(&selection.recipe)?;
        let BatchUse::NewBatch {batch_id,..} = &selection.batch_use else {continue;};
        if paired {
            if gf_pair {crate::personal_scope::check_gf_chicken_pair_allocation(selection,&crate::BoardOptions::default())?;}
            else {crate::personal_scope::check_chicken_pair_allocation(selection,&crate::BoardOptions::default())?;}
            for extra in selection.extras.iter(){view.resolve_recipe(&extra.recipe)?;}
        }
        if (!paired && !selection.extras.is_empty())
            || week.plan.selections.iter().any(|other|other.slot!=selection.slot && match &other.batch_use {
                BatchUse::NewBatch {batch_id:other,..}|BatchUse::Leftover {batch_id:other,..}=>other==batch_id })
            || week.plan.batch_links.iter().any(|link|&link.batch_id==batch_id||link.source_slot==selection.slot||link.target_slot==selection.slot) {continue;}
        let current = week.cells.iter().find(|cell|cell.slot==selection.slot)
            .ok_or_else(||error(ErrorCode::ForeignReference,"/personal_use_scope/slot"))?;
        let edited = edited_cells.iter().find(|cell|cell.slot==selection.slot)
            .ok_or_else(||error(ErrorCode::ForeignReference,"/personal_use_scope/slot"))?;
        let absent = |person:&PersonRef,cell:&Cell|match person {
            PersonRef::Guest {occasion_id,guest_id}=>!cell.occasion.0.as_ref().is_some_and(|occasion|
                &occasion.occasion_id==occasion_id&&occasion.guests.iter().any(|guest|&guest.guest_id==guest_id)),
            // Only the new exact pair permits retaining a still-defined
            // resident who was deselected from this meal. Other cell/profile
            // references retain their ordinary strict checks.
            PersonRef::Diner {diner_id}=>paired&&!cell.attendee_ids.contains(diner_id),
        };
        if selection.servings.iter().any(|serving|absent(&serving.person,current)||absent(&serving.person,edited)) {
            omitted.push(selection.slot.clone());
        }
    }
    if omitted.is_empty() {return Ok(None);}
    let mut projected=week.clone();projected.plan.selections.0.retain(|selection|!omitted.contains(&selection.slot));
    Ok(Some(projected))
}

pub(crate) fn prepare_preserving_week_edit(view:&RuntimeCatalogue,current:&Week,edit:&WeekEdit,
    expected:Revision,profile:&HouseholdProfile,at:&Instant)
    ->Result<(mstrchf_core::availability::WeekEditEffect,bool),DomainError> {
    let projection=two_unit_guest_projection(view,current,&edit.cells.0,None)?;
    let mut effect=mstrchf_core::availability::prepare_week_edit(projection.as_ref().unwrap_or(current),edit,expected,profile,at)?;
    if projection.is_some() {effect.candidate.plan=current.plan.clone();}
    Ok((effect,projection.is_some()))
}

pub(crate) fn needs_two_unit_retention(view:&RuntimeCatalogue,current:&Week,edit:&WeekEdit)
    ->Result<bool,DomainError> {
    Ok(two_unit_guest_projection(view,current,&edit.cells.0,None)?.is_some())
}

fn choice_context(view:&RuntimeCatalogue,snapshot:&Snapshot,profile:&HouseholdProfile,
    week:&Week,target:&SlotId)->Result<PlanningContext,DomainError> {
    let projection=two_unit_guest_projection(view,week,&week.cells.0,Some(target))?;
    context(view,snapshot,profile,projection.as_ref().unwrap_or(week))
}

fn actual_board_options(options: &ChoiceOptions) -> crate::BoardOptions {
    crate::BoardOptions {
        overrides: Some(options.overrides.clone()),
        extra_recipients: Some(BoundedVec(options.extra_recipients.clone())),
        make_extra: Some(options.make_extra), leftover_source: options.leftover_source.clone(),
        eat_by_time: options.eat_by_time.clone(), time_resolution: Some(options.time_resolution),
        portable_cold: options.portable_cold.clone(),
    }
}
fn choice_scope(view: &RuntimeCatalogue, context: &PlanningContext, week: &Week,
    selection: &MealSelection, options: &crate::BoardOptions) -> Result<(), DomainError> {
    let recipe = view.resolve_recipe(&selection.recipe)?;
    if view.is_public_recipe(&selection.recipe) {
        let meal=context.requirements.iter().find(|meal|meal.slot==selection.slot)
            .ok_or_else(||error(ErrorCode::MissingReference,"/public_beta/slot"))?;
        return crate::public_catalogue::check_use(recipe,&selection.recipe,meal,selection,&week.plan,options);
    }
    for extra in selection.extras.iter() {
        view.resolve_recipe(&extra.recipe)?;
        if view.is_personal_recipe(&extra.recipe)
            && selection.recipe.recipe_id.0!=crate::personal_scope::THREE_BEAN_TWO_UNIT_CHICKEN_EXTRA_SCOPE.recipe_id
            && selection.recipe.recipe_id.0!=crate::personal_scope::THREE_BEAN_GF_CHICKEN_SCOPE.recipe_id {
            return Err(error(ErrorCode::UnsupportedCapability,"/personal_use_scope/extra_placement"));
        }
    }
    if !view.is_personal_recipe(&selection.recipe) { return Ok(()); }
    let meal = context.requirements.iter().find(|meal|meal.slot == selection.slot)
        .ok_or_else(||error(ErrorCode::MissingReference,"/personal_use_scope/slot"))?;
    crate::personal_scope::check_personal_use_scope(recipe,&selection.recipe,meal,selection,&week.plan,options)?;
    if selection.recipe.recipe_id.0==crate::personal_scope::THREE_BEAN_TWO_UNIT_CHICKEN_EXTRA_SCOPE.recipe_id {
        for extra in selection.extras.iter(){crate::personal_scope::check_chicken_extra_scope(view.resolve_recipe(&extra.recipe)?,meal,selection,&week.plan,options)?;}
    }
    if selection.recipe.recipe_id.0==crate::personal_scope::THREE_BEAN_GF_CHICKEN_SCOPE.recipe_id {
        for extra in selection.extras.iter(){crate::personal_scope::check_gf_chicken_extra_scope(view.resolve_recipe(&extra.recipe)?,meal,selection,&week.plan,options)?;}
    }
    Ok(())
}
fn check_context_scope(view: &RuntimeCatalogue, context: &PlanningContext, week: &Week) -> Result<(), DomainError> {
    let active = mstrchf_core::availability::eligible_slots(week)?;
    for selection in week.plan.selections.iter() {
        // Exclusion retains exact references, but creates no active demand or
        // new scope authority. Current incompatibility is a retained conflict.
        view.resolve_recipe(&selection.recipe)?;
        for extra in selection.extras.iter() { view.resolve_recipe(&extra.recipe)?; }
        if active.contains(&selection.slot) { choice_scope(view,context,week,selection,&crate::BoardOptions::default())?; }
    }
    Ok(())
}
fn retained_personal_scope(view: &RuntimeCatalogue, profile: &HouseholdProfile,
    week: &Week, selection: &MealSelection) -> Result<(), DomainError> {
    let recipe = view.resolve_recipe(&selection.recipe)?;
    // Query the actual current cell without asking stale subject allocations to
    // establish current attendees. Other saved selections and links still pass
    // the strict core reference check in this query.
    let mut current_cell=week.clone();current_cell.plan.selections.0.retain(|saved|saved.slot!=selection.slot);
    let hard = mstrchf_core::availability::retained_hard_requirements(profile,&current_cell,&selection.slot,None)?;
    // A read-only inspection week makes just the retained intrinsic cell active.
    // This ignores exclusion switches, never mutates saved availability and
    // never derives food or manufactures old attendees from saved allocations.
    let mut inspect = week.clone();
    inspect.day_enabled = BoundedVec(vec![true;7]);
    inspect.meal_type_enabled = MealTypeEnabled {breakfast:true,lunch:true,dinner:true};
    inspect.plan.selections = BoundedVec(vec![]);
    inspect.plan.batch_links = BoundedVec(vec![]);
    for cell in inspect.cells.iter_mut() {
        if cell.slot == selection.slot {
            if cell.choice == MealLocation::Skip { cell.choice = MealLocation::AtHome; }
        } else { cell.choice = MealLocation::Skip; cell.occasion = Nullable(None); }
    }
    // Preserve a validated saved deadline through the subject-less read, also
    // when its explicit fold resolution differs from the household usual time.
    let mut inspect_profile=profile.clone();let mut resolutions=Vec::new();
    if let Some(eat_by)=&selection.meal_overrides.eat_by.0 {
        let checked=ZonedMoment::new(eat_by.local_date.clone(),eat_by.local_time.clone(),eat_by.zone.clone(),eat_by.resolution,eat_by.instant.clone())?;
        if checked.local_date!=selection.slot.date||checked.zone!=week.range.zone {return Err(error(ErrorCode::IdentityMismatch,"/personal_use_scope/first_handling_time"));}
        let usual=inspect_profile.preferences.meal_times.iter_mut().find(|time|time.meal_type==selection.slot.meal_type)
            .ok_or_else(||error(ErrorCode::MissingField,"/preferences/meal_times"))?;
        usual.time=checked.local_time;resolutions.push(mstrchf_core::availability::SlotResolution{slot:selection.slot.clone(),resolution:checked.resolution});
    }
    let mut meal = mstrchf_core::availability::meal_requirements(&inspect_profile,&inspect,&resolutions)?.into_iter()
        .find(|meal|meal.slot == selection.slot).ok_or_else(||error(ErrorCode::ForeignReference,"/personal_use_scope/slot"))?;
    if selection.meal_overrides.eat_by.0.as_ref().is_some_and(|eat_by|meal.eat_by.0.as_ref()!=Some(eat_by)) {return Err(error(ErrorCode::IdentityMismatch,"/personal_use_scope/first_handling_time"));}
    meal.shared_hard_requirements = BoundedVec(hard);
    if view.is_public_recipe(&selection.recipe){return crate::public_catalogue::check_use(recipe,&selection.recipe,&meal,selection,&week.plan,&crate::BoardOptions::default());}
    crate::personal_scope::check_personal_use_scope(recipe,&selection.recipe,&meal,selection,&week.plan,&crate::BoardOptions::default())?;
    if selection.recipe.recipe_id.0==crate::personal_scope::THREE_BEAN_TWO_UNIT_CHICKEN_EXTRA_SCOPE.recipe_id {
        for extra in selection.extras.iter(){crate::personal_scope::check_chicken_extra_scope(view.resolve_recipe(&extra.recipe)?,&meal,selection,&week.plan,&crate::BoardOptions::default())?;}
    }
    if selection.recipe.recipe_id.0==crate::personal_scope::THREE_BEAN_GF_CHICKEN_SCOPE.recipe_id {
        for extra in selection.extras.iter(){crate::personal_scope::check_gf_chicken_extra_scope(view.resolve_recipe(&extra.recipe)?,&meal,selection,&week.plan,&crate::BoardOptions::default())?;}
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::maker_packets::*;
    use serde_json::{Value, json};
    const NOW: &str = "2026-10-12T18:00:00Z";
    const BREAKFAST: usize = 3; // Future Tuesday breakfast in the current week.
    fn clock(at: &str) { crate::CLOCK.with(|v| *v.borrow_mut() = Some(Instant(at.into()))); }
    fn demo() -> (Snapshot, HouseholdProfile, Week) {
        let response: Value = serde_json::from_slice(&crate::run_json(&serde_json::to_vec(&json!({"op":"initialize","household_id":"hh_00000000000000000000000000000006","diner_id":"dn_00000000000000000000000000000006","zone":"Europe/London","today":"2026-10-12","now":NOW,"demo":true})).unwrap())).unwrap();
        assert_eq!(response["ok"], true, "{response}");
        let mut snapshot: Snapshot = parse_document(&canonical_bytes(&response["value"]["snapshot"]).unwrap()).unwrap();
        let mut profile = snapshot.profile.0.clone().unwrap();
        profile.diners[0].age_band = AgeBand::Adult;
        profile.diners[0].restrictions = RestrictionDeclaration::ConfirmedNone {confirmed_at:Instant(NOW.into())};
        profile.diners[0].portions.preset = PortionPresetName::Regular;
        profile.diners[0].portions.adjustment = PositiveRational::new(1,1).unwrap();
        profile.diners[0].portions.meal_overrides = BoundedVec(vec![]);
        profile.preferences.equipment = BoundedVec(vec![EquipmentCode::Hob]);
        profile.preferences.mobility.temperature = MobilityTemperature::Hot;
        snapshot.profile = Nullable(Some(profile.clone()));
        let mut week = snapshot.weeks[0].clone();
        week.day_enabled = BoundedVec(vec![true;7]);
        week.meal_type_enabled = MealTypeEnabled {breakfast:true,lunch:true,dinner:true};
        for cell in week.cells.iter_mut() { cell.choice = MealLocation::AtHome; }
        clock(NOW);
        (snapshot,profile,week)
    }
    fn real() -> RuntimeCatalogue {
        RuntimeCatalogue::from_records(&[known_packet(&porridge_packet_ref().unwrap()).unwrap().record().clone()]).unwrap()
    }
    fn selected(view: &RuntimeCatalogue, snapshot: &Snapshot, profile: &HouseholdProfile, week: &Week) -> Week {
        let choices = custom_choices(view,snapshot,profile,week,&week.cells[BREAKFAST].slot,ChoiceOptions::default()).unwrap();
        assert_eq!(choices.len(),1,"expected the exact personal recipe to derive successfully");
        let mut next = week.clone(); next.plan = choices[0].plan.clone().unwrap(); next
    }

    #[test]
    fn compiled_record_pins_and_manifest_are_exact_and_tamper_sensitive() {
        let reference = porridge_packet_ref().unwrap();
        let trusted = known_packet(&reference).unwrap();
        assert_eq!(reference.packet_digest.0,"edc4239b57366c96ef4b91198e4cc54de62db44d3d6346574e7318a852e2d726");
        assert_eq!(recipe_digest(trusted.recipe()).unwrap(),reference.recipe.digest);
        let mut record = trusted.record().clone();
        record.packet["operational_scope"]["scope_id"] = json!("unknown-scope");
        assert_eq!(known_record(&record).unwrap_err().code,ErrorCode::DigestMismatch);
        let mut record = trusted.record().clone(); record.evidence_documents[0]["actor"] = json!("human");
        assert_eq!(RuntimeCatalogue::from_records(&[record]).unwrap_err().code,ErrorCode::DigestMismatch);
        let mut wrong = reference.clone(); wrong.packet_digest = Digest("a".repeat(64));
        assert_eq!(known_packet(&wrong).unwrap_err().code,ErrorCode::DigestMismatch);
        let mut wrong = reference; wrong.recipe.revision = Revision(2);
        assert_eq!(known_packet(&wrong).unwrap_err().code,ErrorCode::MissingReference);
        let view = real();
        assert_eq!(view.pin().revision,Revision(2)); assert_eq!(view.pin().entries.len(),6);
        assert_eq!(view.pin(),real().pin());
        assert_eq!(view.resolve_recipe(&trusted.reference().recipe).unwrap(),trusted.recipe());
        assert_eq!(RuntimeCatalogue::from_records(&[trusted.record().clone(),trusted.record().clone()]).unwrap_err().code,ErrorCode::DuplicateKey);
        println!("real catalogue manifest {}",view.pin().manifest_digest.0);
    }

    #[test]
    fn explicit_legacy_view_reproduces_pre_pin_food_bytes() {
        let fixture: Value = serde_json::from_slice(include_bytes!("../tests/fixtures/legacy-engineering-receipts.json")).unwrap();
        let snapshot: Snapshot = parse_document(&canonical_bytes(&fixture["state"]["snapshot"]).unwrap()).unwrap();
        clock("2026-10-12T12:00:00Z");
        let view = RuntimeCatalogue::legacy().unwrap();
        assert_eq!(view.pin(),crate::catalogue::legacy_engineering_v1().unwrap().pin());
        let actual = derive(&view,&snapshot,snapshot.profile.0.as_ref().unwrap(),&snapshot.weeks[0]).unwrap().unwrap();
        assert_eq!(serde_json::to_value(actual.plan).unwrap(),fixture["derived"]);
        assert!(!view.has_real_entries());
    }

    #[test]
    fn admitted_real_choice_uses_exact_own_quantities_and_no_synthetic_evidence() {
        let (snapshot,profile,week) = demo(); let view = real();
        let next = selected(&view,&snapshot,&profile,&week);
        let reference = porridge_packet_ref().unwrap().recipe;
        assert_eq!(next.plan.selections[0].recipe,reference);
        let evidence = view.food_evidence(&[reference.clone()]).unwrap();
        assert_eq!(evidence.recipes[0].admitted.len(),11);
        assert!(evidence.recipes[0].admitted.iter().all(|pin|pin.evidence_id.0.starts_with("ev_800")));
        assert!(evidence.classes[0].components.is_empty());
        assert_eq!(evidence.methods.len(),1); assert_eq!(evidence.methods[0].kind,MethodKind::Production);
        assert!(evidence.essential_ingredient_ids.is_empty());
        let food = derive(&view,&snapshot,&profile,&next).unwrap().unwrap();
        let mut quantities = food.plan.demand.lines.iter().map(|line|(line.ingredient_id.clone(),line.quantity.amount,line.quantity.unit,line.quantity.form)).collect::<Vec<_>>();
        quantities.sort_by(|a,b|a.0.cmp(&b.0));
        assert_eq!(quantities,vec![(IngredientId("ig_80000000000000000000000000000001".into()),PositiveRational::new(45,1).unwrap(),Unit::G,IngredientForm::Dry),(IngredientId("ig_80000000000000000000000000000002".into()),PositiveRational::new(180,1).unwrap(),Unit::Ml,IngredientForm::Raw),(IngredientId("ig_80000000000000000000000000000003".into()),PositiveRational::new(180,1).unwrap(),Unit::Ml,IngredientForm::Raw)]);
        assert!(food.plan.demand.lines.iter().all(|line|!line.essential));
        assert_eq!(context(&view,&snapshot,&profile,&next).unwrap().stamp.catalogue_revision,Revision(2));
    }

    #[test]
    fn compensating_portions_and_extra_requests_do_not_bypass_choice_scope() {
        let (snapshot,mut profile,week) = demo(); let view = real();
        profile.diners[0].portions.adjustment = PositiveRational::new(2,1).unwrap();
        let options = ChoiceOptions {overrides:MealOverrides {portions:BoundedVec(vec![PersonPortionOverride {person:PersonRef::Diner {diner_id:profile.diners[0].diner_id.clone()},preset:PortionPresetName::Regular,adjustment:PositiveRational::new(1,2).unwrap()}]),eat_by:Nullable(None)},..Default::default()};
        assert!(custom_choices(&view,&snapshot,&profile,&week,&week.cells[BREAKFAST].slot,options).unwrap().is_empty());
        profile.diners[0].portions.adjustment = PositiveRational::new(1,1).unwrap();
        assert!(custom_choices(&view,&snapshot,&profile,&week,&week.cells[BREAKFAST].slot,ChoiceOptions {make_extra:true,..Default::default()}).unwrap().is_empty());
        assert!(custom_choices(&view,&snapshot,&profile,&week,&week.cells[5].slot,ChoiceOptions::default()).unwrap().is_empty());
    }

    #[test]
    fn incompatible_active_food_remains_readable_but_cannot_derive() {
        let (snapshot,mut profile,week) = demo(); let view = real();
        let next = selected(&view,&snapshot,&profile,&week);
        profile.diners[0].age_band = AgeBand::Teen;
        assert!(context(&view,&snapshot,&profile,&next).is_ok());
        assert!(custom_choices(&view,&snapshot,&profile,&next,&next.cells[BREAKFAST].slot,ChoiceOptions::default()).unwrap().is_empty());
        assert!(derive(&view,&snapshot,&profile,&next).is_err());
        assert!(!retained_conflicts(&view,&profile,&next).unwrap().is_empty());
    }

    #[test]
    fn excluded_real_selection_keeps_exact_food_and_reports_current_intrinsic_scope_conflicts() {
        let (snapshot,profile,week) = demo(); let view = real();
        let mut next = selected(&view,&snapshot,&profile,&week);
        next.day_enabled[1] = false; next.cells[BREAKFAST].choice = MealLocation::Skip;
        let bytes = canonical_bytes(&next).unwrap();
        assert!(retained_conflicts(&view,&profile,&next).unwrap().is_empty());
        assert!(derive(&view,&snapshot,&profile,&next).unwrap().unwrap().plan.demand.lines.is_empty());
        let mut changed = profile.clone(); changed.diners[0].age_band = AgeBand::Teen;
        assert!(!retained_conflicts(&view,&changed,&next).unwrap().is_empty());
        let mut changed = profile.clone(); changed.diners[0].portions.adjustment = PositiveRational::new(2,1).unwrap();
        assert!(!retained_conflicts(&view,&changed,&next).unwrap().is_empty());
        let mut changed = profile.clone(); changed.diners[0].restrictions = RestrictionDeclaration::Confirmed {confirmed_at:Instant(NOW.into()),requirements:BoundedVec(vec![HardRequirement::Diet {code:DietCode::Vegan}])};
        assert!(!retained_conflicts(&view,&changed,&next).unwrap().is_empty());
        assert_eq!(canonical_bytes(&next).unwrap(),bytes);
        let mut moved = next.clone(); moved.cells[BREAKFAST].choice = MealLocation::OnTheMove;
        assert!(!retained_conflicts(&view,&profile,&moved).unwrap().is_empty());
        let mut group = next; let mut changed = profile;
        let mut second = changed.diners[0].clone(); second.diner_id = DinerId("dn_00000000000000000000000000000007".into());
        group.cells[BREAKFAST].attendee_ids.0.push(second.diner_id.clone()); changed.diners.0.push(second);
        assert!(!retained_conflicts(&view,&changed,&group).unwrap().is_empty());
    }

    fn aglio_view(both: bool) -> RuntimeCatalogue {
        let mut records=vec![known_packet(&aglio_packet_ref().unwrap()).unwrap().record().clone()];
        if both {records.push(known_packet(&porridge_packet_ref().unwrap()).unwrap().record().clone());}
        RuntimeCatalogue::from_records(&records).unwrap()
    }
    #[test]
    fn aglio_exact_dinner_uses_own_policy_essentials_and_counts_process_water_once() {
        let(snapshot,profile,week)=demo();clock("2026-10-12T20:00:00Z");let view=aglio_view(false);
        let choices=custom_choices(&view,&snapshot,&profile,&week,&week.cells[5].slot,ChoiceOptions::default()).unwrap();
        assert_eq!(choices.len(),1);assert_eq!(choices[0].selection.recipe,aglio_packet_ref().unwrap().recipe);
        let mut next=week.clone();next.plan=choices[0].plan.clone().unwrap();
        let evidence=view.food_evidence(&[choices[0].selection.recipe.clone()]).unwrap();
        assert_eq!(evidence.methods[0].evidence_ids,vec![EvidenceId("ev_80000000000000000000000000000019".into())]);
        assert_eq!(evidence.essential_ingredient_ids,vec![IngredientId("ig_80000000000000000000000000000004".into()),IngredientId("ig_80000000000000000000000000000005".into())]);
        assert!(evidence.classes[0].components.is_empty());assert_eq!(evidence.recipes[0].admitted.len(),10);
        let food=derive(&view,&snapshot,&profile,&next).unwrap().unwrap();
        let mut values=food.plan.demand.lines.iter().map(|line|(line.ingredient_id.clone(),line.quantity.amount,line.quantity.unit,line.quantity.form,line.essential)).collect::<Vec<_>>();
        values.sort_by(|a,b|a.0.cmp(&b.0));
        assert_eq!(values,vec![(IngredientId("ig_80000000000000000000000000000002".into()),PositiveRational::new(1500,1).unwrap(),Unit::Ml,IngredientForm::Raw,false),
            (IngredientId("ig_80000000000000000000000000000004".into()),PositiveRational::new(125,1).unwrap(),Unit::G,IngredientForm::Dry,true),
            (IngredientId("ig_80000000000000000000000000000005".into()),PositiveRational::new(55,2).unwrap(),Unit::G,IngredientForm::Raw,true),
            (IngredientId("ig_80000000000000000000000000000006".into()),PositiveRational::new(2,1).unwrap(),Unit::Each,IngredientForm::Whole,false)]);
    }

    #[test]
    fn dual_catalogue_keeps_own_evidence_scope_aliases_and_porridge_pantry_semantics() {
        let view=aglio_view(true);assert_eq!(view.pin().revision,Revision(3));assert_eq!(view.recipes().len(),7);
        let reverse=RuntimeCatalogue::from_records(&[known_packet(&porridge_packet_ref().unwrap()).unwrap().record().clone(),known_packet(&aglio_packet_ref().unwrap()).unwrap().record().clone()]).unwrap();
        assert_eq!(view.pin(),reverse.pin());
        let old=view.food_evidence(&[porridge_packet_ref().unwrap().recipe]).unwrap();
        assert!(old.essential_ingredient_ids.is_empty());
        assert_eq!(old.methods[0].evidence_ids,vec![EvidenceId("ev_8000000000000000000000000000000b".into())]);
        let(snapshot,profile,week)=demo();clock("2026-10-12T20:00:00Z");
        let breakfast=custom_choices(&view,&snapshot,&profile,&week,&week.cells[3].slot,ChoiceOptions::default()).unwrap();
        assert_eq!(breakfast.len(),1);assert_eq!(breakfast[0].selection.recipe,porridge_packet_ref().unwrap().recipe);
        let dinner=custom_choices(&view,&snapshot,&profile,&week,&week.cells[5].slot,ChoiceOptions::default()).unwrap();
        assert_eq!(dinner.len(),1);assert_eq!(dinner[0].selection.recipe,aglio_packet_ref().unwrap().recipe);
        assert!(custom_choices(&view,&snapshot,&profile,&week,&week.cells[4].slot,ChoiceOptions::default()).unwrap().is_empty());
    }

    #[test]
    fn aglio_scope_rejects_compensating_dinner_portions_extras_and_excluded_current_age_changes() {
        let(snapshot,mut profile,week)=demo();clock("2026-10-12T20:00:00Z");let view=aglio_view(false);
        let mut offers=custom_choices(&view,&snapshot,&profile,&week,&week.cells[5].slot,ChoiceOptions::default()).unwrap();
        let mut next=week.clone();next.plan=offers.remove(0).plan.unwrap();
        profile.diners[0].portions.meal_overrides=BoundedVec(vec![MealPortionOverride{meal_type:MealType::Breakfast,preset:PortionPresetName::Larger,adjustment:PositiveRational::new(2,1).unwrap()}]);
        assert!(derive(&view,&snapshot,&profile,&next).is_ok(),"unrelated breakfast settings do not change Dinner scope");
        profile.diners[0].portions.adjustment=PositiveRational::new(2,1).unwrap();
        let options=ChoiceOptions{overrides:MealOverrides{portions:BoundedVec(vec![PersonPortionOverride{person:PersonRef::Diner{diner_id:profile.diners[0].diner_id.clone()},preset:PortionPresetName::Regular,adjustment:PositiveRational::new(1,2).unwrap()}]),eat_by:Nullable(None)},..Default::default()};
        assert!(custom_choices(&view,&snapshot,&profile,&week,&week.cells[5].slot,options).unwrap().is_empty());
        profile.diners[0].portions.adjustment=PositiveRational::new(1,1).unwrap();
        assert!(custom_choices(&view,&snapshot,&profile,&week,&week.cells[5].slot,ChoiceOptions{make_extra:true,..Default::default()}).unwrap().is_empty());
        next.meal_type_enabled.dinner=false;next.cells[5].choice=MealLocation::Skip;
        let bytes=canonical_bytes(&next).unwrap();profile.diners[0].age_band=AgeBand::Teen;
        assert!(!retained_conflicts(&view,&profile,&next).unwrap().is_empty());assert_eq!(canonical_bytes(&next).unwrap(),bytes);
    }
}
