//! Native adapter for the labelled engineering planner. No production food admission.
use mstrchf_contracts::*;
use mstrchf_core::{food::*, recipes::*, selection::*};

fn error(code:ErrorCode,path:&str)->DomainError {DomainError::new(code,path)}
fn moment(utc:chrono::DateTime<chrono::FixedOffset>,zone:&ZoneId)->Result<ZonedMoment,DomainError>{
    let tz:chrono_tz::Tz=zone.0.parse().map_err(crate::records::storage)?;
    let local=utc.with_timezone(&tz);
    let instant=Instant(utc.with_timezone(&chrono::Utc).to_rfc3339_opts(chrono::SecondsFormat::Secs,true));
    for resolution in [Resolution::Unambiguous,Resolution::Earlier,Resolution::Later] {
        if let Ok(value)=ZonedMoment::new(LocalDate(local.format("%Y-%m-%d").to_string()),LocalTime(local.format("%H:%M:%S").to_string()),zone.clone(),resolution,instant.clone()){return Ok(value);}
    }
    Err(error(ErrorCode::InvalidValue,"/schedule/time"))
}
pub(crate) fn food_error(e:FoodFailure)->DomainError {match e {FoodFailure::Invalid(e)=>e,FoodFailure::Blocked{conflicts,..}=>error(conflicts.first().map_or(ErrorCode::DomainConflict,|c|c.code),"/planner/food")}}
pub fn catalogue()->Result<Vec<RecipeRevision>,DomainError>{
    static CATALOGUE:std::sync::OnceLock<Result<Vec<RecipeRevision>,DomainError>>=std::sync::OnceLock::new();
    CATALOGUE.get_or_init(build_catalogue).clone()
}
fn build_catalogue()->Result<Vec<RecipeRevision>,DomainError>{
    let mut recipes=crate::development::catalogue()?;
    // New immutable recipe IDs. The three M1 revisions are retained byte-for-byte.
    for id in 4..=5 {
        let mut r=recipes[0].clone();r.recipe_id=RecipeId(format!("rc_{id:032x}"));
        r.title=Title(if id==4 {"Synthetic advance-prep bowl"} else {"Synthetic quick bowl"}.into());
        if let Some(image)=&mut r.image.0 {image.asset_id=AssetId(format!("as_{id:032x}"));image.digest=AssetDigest(bytes_digest(crate::planner_state::PLACEHOLDER).0);image.alt_text=Text("Engineering placeholder illustration; no food photograph".into());image.rights.attribution=Text("MSTRCHF host-owned engineering fixture".into());}
        r.meal_types=BoundedVec(vec![MealType::Breakfast,MealType::Lunch,MealType::Dinner]);
        r.scaling=ScalingPolicy::Linear{minimum:PositiveRational::new(1,12)?,maximum:PositiveRational::new(32,1)?};
        r.portion_presets.0.clear();
        // Explicit recipe-owned engineering premises; these are not nutritional advice.
        for age in [AgeBand::Adult,AgeBand::Teen,AgeBand::Child1To3,AgeBand::Child4To6,AgeBand::Child7To10,AgeBand::Child11To12] {
            let basis=match age {AgeBand::Child1To3=>PositiveRational::new(1,3)?,AgeBand::Child4To6=>PositiveRational::new(1,2)?,AgeBand::Child7To10=>PositiveRational::new(2,3)?,AgeBand::Child11To12=>PositiveRational::new(3,4)?,_=>PositiveRational::new(1,1)?};
            for meal_type in [MealType::Breakfast,MealType::Lunch,MealType::Dinner] {
                for (preset,multiplier) in [(PortionPresetName::Smaller,PositiveRational::new(1,2)?),(PortionPresetName::Regular,PositiveRational::new(1,1)?),(PortionPresetName::Larger,PositiveRational::new(3,2)?)] {
                    r.portion_presets.0.push(PortionPreset{preset,age_band:age,meal_type,servings:PositiveRational(basis.0.checked_mul(&multiplier.0)?),evidence_ids:BoundedVec(vec![EvidenceId(format!("ev_{:032x}",5))])});
                }
            }
        }
        r.r#yield.basis=Text("Recipe-specific synthetic engineering serving premises; not production food evidence.".into());
        r.portability=PortabilityEvidence{cold_suitable:true,reheating_required:false,cold_storage_required:true,evidence_ids:BoundedVec(vec![EvidenceId(format!("ev_{:032x}",8))])};
        if id==4 {r.prep[0].earliest_offset_seconds=-43200;r.prep[0].latest_offset_seconds=-43200;r.prep[0].title=Title("Synthetic advance preparation".into());}
        if id==5 {
            let component=ComponentId(format!("co_{:032x}",2));
            r.components[0].component_id=component.clone();r.ingredients[0].component_id=component.clone();r.r#yield.components[0].component_id=component.clone();r.steps[0].component_ids=BoundedVec(vec![component.clone()]);r.storage[0].component_id=component;
        }
        recipes.push(parse_document(&canonical_bytes(&r)?)?);
    }
    let extra=crate::development::recipe_reference(&recipes[4])?;
    recipes[3].extras=BoundedVec(vec![ExtraSpec{extra_id:ExtraId(format!("ex_{:032x}",1)),title:Title("Synthetic optional extra".into()),recipe:extra,eligible_person_requirements:BoundedVec(vec![]),supported_with:BoundedVec(vec![recipes[3].components[0].component_id.clone()])}]);
    Ok(recipes)
}

fn material()->Result<&'static (Vec<RecipeRevision>,Vec<(EvidenceBasis,ComponentClassView)>),DomainError>{
    static VALUE:std::sync::OnceLock<Result<(Vec<RecipeRevision>,Vec<(EvidenceBasis,ComponentClassView)>),DomainError>>=std::sync::OnceLock::new();
    VALUE.get_or_init(||{let recipes=catalogue()?;let inputs=recipes.iter().map(crate::development::evidence_inputs).collect::<Result<Vec<_>,_>>()?;Ok((recipes,inputs))}).as_ref().map_err(Clone::clone)
}
pub(crate) fn reviewed()->Result<&'static [ReviewedRecipe<'static>],DomainError>{
    static VALUE:std::sync::OnceLock<Result<Vec<ReviewedRecipe<'static>>,DomainError>>=std::sync::OnceLock::new();
    let material=material()?;
    VALUE.get_or_init(||material.0.iter().zip(&material.1).map(|(r,(b,k))|assess_revision(r,b,k)).collect()).as_ref().map(Vec::as_slice).map_err(Clone::clone)
}
pub(crate) fn retained_conflicts(p:&HouseholdProfile,w:&Week)->Result<Vec<Conflict>,DomainError>{
    let recipes=reviewed()?;let mut conflicts=Vec::new();
    for selected in &w.plan.selections.0 {
        let mut checks=vec![(selected.recipe.clone(),None)];
        for extra in &selected.extras.0 {checks.push((extra.recipe.clone(),Some(extra.recipients.iter().map(|v|v.person.clone()).collect::<Vec<_>>())));}
        for (reference,recipients) in checks {
            let result=(||{let r=recipes.iter().find(|r|crate::development::recipe_reference(r.recipe()).as_ref()==Ok(&reference)).ok_or_else(||error(ErrorCode::MissingReference,"/retained/recipe"))?;let requirements=mstrchf_core::availability::retained_hard_requirements(p,w,&selected.slot,recipients.as_deref())?;mstrchf_core::eligibility::eligible_retained(r,&selected.slot,&requirements)})();
            match result {Ok(mstrchf_core::eligibility::Eligibility::Eligible)=>{},Ok(mstrchf_core::eligibility::Eligibility::Ineligible(v)|mstrchf_core::eligibility::Eligibility::Unresolved(v))=>conflicts.extend(v),Err(e)=>conflicts.push(Conflict{code:e.code,week_ids:BoundedVec(vec![w.range.week_id.clone()]),slot_ids:BoundedVec(vec![selected.slot.clone()]),recipe_refs:BoundedVec(vec![reference]),evidence_ids:BoundedVec(vec![]),blocking:true})}
        }
    }
    Ok(conflicts)
}

pub(crate) fn stable_id(prefix:&str,material:&impl serde::Serialize)->Result<String,DomainError>{
    let digest=canonical_digest(&serde_json::json!({"domain":prefix,"key":material}))?;
    Ok(format!("{prefix}_{}",&digest.0[..32]))
}
pub(crate) fn context(s:&Snapshot,p:&HouseholdProfile,w:&Week,recipes:Vec<RecipeRevision>)->Result<PlanningContext,DomainError>{crate::development::context(s,p,w,recipes)}

pub(crate) fn derive(s:&Snapshot,p:&HouseholdProfile,w:&Week)->Result<Option<FoodDerivation>,DomainError>{
    if w.plan.selections.is_empty(){
        if !w.plan.batch_links.is_empty(){return Err(error(ErrorCode::MissingReference,"/plan/batch_links"));}
        return Ok(None);
    }
    let c=context(s,p,w,catalogue()?)?;
    let inputs=&material()?.1;
    let reviewed=reviewed()?;
    let active=mstrchf_core::availability::eligible_slots(w)?;
    for selected in w.plan.selections.iter().filter(|v|active.contains(&v.slot)) {
        let choice=ChoiceSpec{portable_cold: selected.portable_cold.clone(), recipe:selected.recipe.clone(),extras:selected.extras.iter().map(|e|ExtraChoice{extra_id:e.extra_id.clone(),recipe:e.recipe.clone(),recipients:e.recipients.iter().map(|v|v.person.clone()).collect()}).collect(),meal_overrides:selected.meal_overrides.clone(),batch_use:selected.batch_use.clone()};
        match assess_choice(&c,&selected.slot,&choice,selected.origin,selected.repeat_authorised,&reviewed)? {
            ChoiceAssessment::Suitable(actual) if actual==*selected=>{},
            ChoiceAssessment::Suitable(_)=>return Err(error(ErrorCode::IdentityMismatch,"/plan/servings")),
            ChoiceAssessment::Ineligible(conflicts)|ChoiceAssessment::Unresolved(conflicts)=>return Err(error(conflicts.first().map_or(ErrorCode::DomainConflict,|v|v.code),"/plan/restrictions")),
        }
    }
    let mut refs=w.plan.selections.iter().filter(|v|active.contains(&v.slot)).flat_map(|v|std::iter::once(v.recipe.clone()).chain(v.extras.iter().map(|e|e.recipe.clone()))).collect::<Vec<_>>();
    refs.sort_by_key(|r|(r.recipe_id.0.clone(),r.revision,r.digest.0.clone()));refs.dedup();
    let mut evidence=FoodEvidence{catalogue_revision:Revision(1),catalogue_digest:PayloadDigest("0".repeat(64)),policy_version:Revision(1),recipes:vec![],classes:vec![],methods:vec![],resources:vec![],essential_ingredient_ids:if refs.is_empty(){vec![]}else{vec![IngredientId(format!("ig_{:032x}",1))]}};
    let ev=vec![EvidenceId(format!("ev_{:032x}",8))];
    for rf in &refs {
        let index=c.recipes.iter().position(|r|crate::development::recipe_reference(r).as_ref()==Ok(rf)).ok_or_else(||error(ErrorCode::MissingReference,"/recipes"))?;
        let r=&c.recipes[index];evidence.recipes.push(inputs[index].0.clone());evidence.classes.push(inputs[index].1.clone());
        evidence.methods.push(MethodPhases{portable_cold: None, recipe:rf.clone(),kind:MethodKind::Production,prep_ids:r.prep.iter().map(|v|v.prep_id).collect(),direct_reuse_counts:vec![],evidence_ids:ev.clone()});
        evidence.methods.push(MethodPhases{portable_cold: None, recipe:rf.clone(),kind:MethodKind::Reuse,prep_ids:r.prep.iter().map(|v|v.prep_id).collect(),direct_reuse_counts:r.components.iter().map(|v|ComponentReheatCount{recipe:rf.clone(),component_id:v.component_id.clone(),planned_reheats:1,evidence_ids:ev.clone()}).collect(),evidence_ids:ev.clone()});
        for prep in &r.prep.0 {evidence.resources.push(PhaseResources{recipe:rf.clone(),prep_id:prep.prep_id,equipment:r.equipment.0.clone(),evidence_ids:ev.clone()});}
    }
    evidence.catalogue_digest=food_evidence_digest(&evidence)?;
    let phases=mstrchf_core::prep::required_phase_keys(&w.plan,&c,&evidence).map_err(food_error)?;
    let mut layout=ScheduleLayout{routes:vec![],fixed_starts:vec![],attachments:vec![]};
    let mut bounds=Vec::new();
    for phase in phases {
        let r=c.recipes.iter().find(|r|crate::development::recipe_reference(r).as_ref()==Ok(&phase.occurrence.recipe)).ok_or_else(||error(ErrorCode::MissingReference,"/phase/recipe"))?;
        let prep=r.prep.iter().find(|v|v.prep_id==phase.prep_id).ok_or_else(||error(ErrorCode::MissingReference,"/phase/prep"))?;
        let selected=w.plan.selections.iter().find(|v|v.slot==phase.occurrence.slot).ok_or_else(||error(ErrorCode::MissingReference,"/phase/slot"))?;
        let meal=c.requirements.iter().find(|v|v.slot==selected.slot).ok_or_else(||error(ErrorCode::MissingReference,"/requirements"))?;
        let end=selected.meal_overrides.eat_by.0.as_ref().or(meal.eat_by.0.as_ref()).ok_or_else(||error(ErrorCode::MissingReference,"/eat_by"))?;
        let utc=chrono::DateTime::parse_from_rfc3339(&end.instant.0).map_err(crate::records::storage)?+chrono::Duration::seconds(i64::from(prep.latest_offset_seconds));
        let start=moment(utc,&end.zone)?;
        let finish=moment(utc+chrono::Duration::seconds(i64::from(prep.elapsed_minutes)*60),&end.zone)?;
        bounds.push(start.clone());bounds.push(finish);
        layout.routes.push(PhaseRoute{phase:phase.clone(),session:SessionKey{anchor_slot:selected.slot.clone(),role:if prep.latest_offset_seconds < -3600 {SessionRole::AdvancePrep}else{SessionRole::Cooking},ordinal:if phase.occurrence.extra_id.is_some(){2}else{1}}});
        layout.fixed_starts.push(PhaseStartConstraint{phase,start});
    }
    bounds.sort_by(|a,b|a.instant.0.cmp(&b.instant.0));
    let mut occupied=OccupancySnapshot{query:bounds.first().zip(bounds.last()).map(|(start,end)|OccupancyQuery{horizon:TimeInterval{start:start.clone(),end:end.clone()},excluded_subject:None}),occupancy_epoch:Revision(1),owners:vec![],digest:PayloadDigest("0".repeat(64))};occupied.digest=occupancy_snapshot_digest(&occupied)?;
    let schedule=ScheduleInputs{layout,occupied};
    let keys=required_identity_keys(&w.plan,&c,&evidence,&schedule).map_err(food_error)?;
    // IDs bind semantic keys, not list positions, and remain stable across edits/reopen.
    let mut identities=FoodIdentities{revision:Revision(1),digest:PayloadDigest("0".repeat(64)),allocations:vec![],sessions:vec![],demand_lines:vec![]};
    for key in keys.allocations {let id=AllocationId(stable_id("al",&serde_json::json!([key.batch_id,key.component_id,key.target_slot,key.person]))?);identities.allocations.push((key,id));}
    for key in keys.sessions {let id=PrepSessionId(stable_id("ps",&serde_json::json!([key.anchor_slot,match key.role {SessionRole::Cooking=>"cooking",SessionRole::AdvancePrep=>"advance_prep"},key.ordinal]))?);identities.sessions.push((key,id));}
    for key in keys.demand_lines {let id=DemandLineId(stable_id("dl",&serde_json::json!([w.range.week_id,key.ingredient_id,key.unit,key.form]))?);identities.demand_lines.push((key,id));}
    identities.digest=food_identities_digest(&identities)?;
    let mut products=ProductUseSnapshot{revision:Revision(1),digest:PayloadDigest("0".repeat(64)),valid_until:None,evidence:vec![],bindings:vec![],facts:vec![]};products.digest=digest_product_uses(&products)?;
    let mut feasibility=FeasibilityInputs{revision:Revision(1),digest:PayloadDigest("0".repeat(64)),delivery:DeliveryInput::Unselected,products,confirmations:vec![],evaluated_at:crate::development::now()?};feasibility.digest=digest_feasibility_inputs(&feasibility)?;
    derive_plan(&w.plan,&c,&FoodInputs{evidence:&evidence,identities:&identities,schedule:&schedule,feasibility:&feasibility}).map(Some).map_err(food_error)
}

#[derive(Clone,Debug)]
pub struct ChoiceOptions {pub overrides:MealOverrides,pub extra_recipients:Vec<PersonRef>,pub make_extra:bool,pub leftover_source:Option<SlotId>,pub eat_by_time:Option<LocalTime>,pub time_resolution:Resolution}
impl Default for ChoiceOptions {fn default()->Self{Self{overrides:MealOverrides{portions:BoundedVec(vec![]),eat_by:Nullable(None)},extra_recipients:vec![],make_extra:false,leftover_source:None,eat_by_time:None,time_resolution:Resolution::Unambiguous}}}
pub(crate) fn choices(s:&Snapshot,p:&HouseholdProfile,w:&Week,slot:&SlotId,overrides:MealOverrides)->Result<Vec<crate::development::DevelopmentChoice>,DomainError>{custom_choices(s,p,w,slot,ChoiceOptions{overrides,..Default::default()})}
pub(crate) fn custom_choices(s:&Snapshot,p:&HouseholdProfile,w:&Week,slot:&SlotId,mut options:ChoiceOptions)->Result<Vec<crate::development::DevelopmentChoice>,DomainError>{
    if let Some(time)=options.eat_by_time {use chrono::TimeZone; let tz:chrono_tz::Tz=w.range.zone.0.parse().map_err(crate::records::storage)?;let local=chrono::NaiveDateTime::parse_from_str(&format!("{} {}",slot.date.0,time.0),"%Y-%m-%d %H:%M:%S").map_err(crate::records::storage)?;let value=match (tz.from_local_datetime(&local),options.time_resolution){(chrono::LocalResult::Single(v),Resolution::Unambiguous)=>v,(chrono::LocalResult::Ambiguous(a,b),Resolution::Earlier)=>a.min(b),(chrono::LocalResult::Ambiguous(a,b),Resolution::Later)=>a.max(b),_=>return Err(error(ErrorCode::InvalidValue,"/choice/eat_by_resolution"))};options.overrides.eat_by=Nullable(Some(moment(value.fixed_offset(),&w.range.zone)?));}
    if options.extra_recipients.len()>64{return Err(error(ErrorCode::LimitExceeded,"/choice/extras"));}
    options.overrides.validate_shape().map_err(|_|error(ErrorCode::InvalidValue,"/choice/overrides"))?;
    let c=context(s,p,w,catalogue()?)?;
    if !mstrchf_core::availability::eligible_slots(w)?.contains(slot){return Ok(vec![]);}
    let reviewed=reviewed()?;
    let mut choices=Vec::new();
    for r in &c.recipes.0 {
        let rf=crate::development::recipe_reference(r)?;
        let source=options.leftover_source.as_ref().map(|source|w.plan.selections.iter().find(|v|&v.slot==source&&v.recipe==rf&&matches!(v.batch_use,BatchUse::NewBatch{..}))).flatten();
        if options.leftover_source.is_some()&&source.is_none(){continue;}
        if !options.extra_recipients.is_empty() && r.extras.is_empty(){continue;}
        let extras=if options.extra_recipients.is_empty(){vec![]}else{let e=&r.extras[0];vec![ExtraChoice{extra_id:e.extra_id.clone(),recipe:e.recipe.clone(),recipients:options.extra_recipients.clone()}]};
        let mut spec=ChoiceSpec{portable_cold: None, recipe:rf.clone(),extras,meal_overrides:options.overrides.clone(),batch_use:BatchUse::NewBatch{batch_id:BatchId(stable_id("ba",&serde_json::json!([slot,rf]))?),production_servings:PositiveRational::new(1,1)?}};
        if let Some(source)=source {if let BatchUse::NewBatch{batch_id,..}=&source.batch_use{spec.batch_use=BatchUse::Leftover{batch_id:batch_id.clone(),component_ids:BoundedVec(r.components.iter().map(|v|v.component_id.clone()).collect())};}}
        if let ChoiceAssessment::Suitable(mut selection)=assess_choice(&c,slot,&spec,SelectionOrigin::Fixed,false,&reviewed)? {
            let mut total=selection.servings.iter().try_fold(Rational::new(0,1)?,|acc,v|acc.checked_add(&v.servings.0))?;
            if options.make_extra&&source.is_none(){total=total.checked_mul(&Rational::new(2,1)?)?;}
            let rr=reviewed.iter().find(|v|v.recipe().recipe_id==r.recipe_id).ok_or_else(||error(ErrorCode::MissingReference,"/reviewed"))?;
            let Ok(scaled)=mstrchf_core::portions::scale_recipe(rr,PositiveRational(total)) else {continue;};
            if let BatchUse::NewBatch{production_servings,..}=&mut spec.batch_use{*production_servings=scaled.produced_servings;}
            selection.batch_use=spec.batch_use;
            let mut candidate=w.clone();candidate.plan.selections.0.retain(|v|v.slot!=*slot);candidate.plan.selections.0.push(selection.clone());candidate.plan.batch_links.0.retain(|v|v.target_slot!=*slot);
            if let Some(source)=source {if let BatchUse::NewBatch{batch_id,..}=&source.batch_use {candidate.plan.batch_links.0.push(BatchLink{batch_id:batch_id.clone(),source_slot:source.slot.clone(),target_slot:slot.clone(),components:BoundedVec(scaled.components.iter().map(|v|ComponentAllocation{component_id:v.component_id.clone(),quantity:v.quantity.clone()}).collect())});}}
            if derive(s,p,&candidate).is_ok(){choices.push(crate::development::DevelopmentChoice{title:r.title.0.clone(),selection,plan:Some(candidate.plan)});}
        }
    }
    Ok(choices)
}

pub(crate) fn suggest(s:&Snapshot,p:&HouseholdProfile,w:&Week,h:&mut PlanningHistory,mode:PlanMode,given:&[SlotId],allow_repeats:bool)->Result<Week,DomainError>{
    if mode==PlanMode::SwapNamed && given.is_empty(){return Err(error(ErrorCode::MissingField,"/suggest/slots"));}
    let active=mstrchf_core::availability::eligible_slots(w)?;
    if given.iter().any(|slot|!active.contains(slot)){return Err(error(ErrorCode::ForeignReference,"/suggest/slots"));}
    let targets=if given.is_empty(){active}else{given.to_vec()};let mut candidate=w.clone();let mut changed=false;
    for slot in targets {
        let old=candidate.plan.selections.iter().find(|v|v.slot==slot);
        if (mode==PlanMode::FillOpen&&old.is_some())||(mode==PlanMode::ReplaceSuggestions&&!old.is_some_and(|v|v.origin==SelectionOrigin::Suggested)){continue;}
        let available=choices(s,p,&candidate,&slot,MealOverrides{portions:BoundedVec(vec![]),eat_by:Nullable(None)})?;
        if available.is_empty(){continue;}
        let mut c=context(s,p,&candidate,catalogue()?)?;c.history=h.clone();c.stamp.history_revision=h.revision;
        let reviewed=reviewed()?;
        let specs=available.iter().map(|v|ChoiceSpec{portable_cold: None, recipe:v.selection.recipe.clone(),extras:vec![],meal_overrides:v.selection.meal_overrides.clone(),batch_use:v.selection.batch_use.clone()}).collect::<Vec<_>>();
        let intent=SelectionIntent{mode,slots:vec![slot.clone()],origin:SelectionOrigin::Suggested,repeat_permissions:vec![]};
        let mut ranking=mstrchf_core::ranking::RankingEvidence{revision:Revision(1),digest:Digest("0".repeat(64)),catalogue_revision:Revision(1),categories:None,recipes:vec![],purchase:vec![]};
        ranking.digest=canonical_digest(&mstrchf_core::ranking::ranking_evidence_material(&ranking)?)?;
        let rank=if allow_repeats {mstrchf_core::ranking::rank_candidates_with_repeat_authorisation}else{mstrchf_core::ranking::rank_candidates};
        let ranked=rank(&c,&slot,&intent,&specs,&reviewed,&ranking,&crate::development::now()?)?;
        for choice in ranked.ranked {
            let mut intent=intent.clone();
            if allow_repeats {intent.repeat_permissions=vec![RepeatPermission{slot:slot.clone(),recipe:choice.selection.recipe.clone()}];}
            let proposal=PlanProposal{changes:BoundedVec(vec![WeekChange{week_id:candidate.range.week_id.clone(),base_revision:candidate.revision,replace_slots:BoundedVec(vec![ProposedSelection{slot:slot.clone(),selection:Nullable(Some(choice.selection))}])}])};
            let patch=mstrchf_core::selection::prepare_selection_patch(&c,&intent,&proposal,&reviewed,&[]).map_err(|v|match v {SelectionFailure::Invalid(e)=>e,SelectionFailure::Blocked(v)=>error(v.first().map_or(ErrorCode::DomainConflict,|v|v.code),"/suggest/choice")})?;
            let next=patch.context.weeks[0].clone();
            if derive(s,p,&next).is_ok(){candidate=next;changed=true;break;}
        }
    }
    if !changed{return Err(error(ErrorCode::DomainConflict,"/suggest/no_fit_or_permission"));}
    Ok(candidate)
}
