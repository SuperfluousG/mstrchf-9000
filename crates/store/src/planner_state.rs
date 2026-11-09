//! Persisted M2 engineering library, current selected index and explicit essentials.
use mstrchf_contracts::*;
use rusqlite::{Connection,params};
use crate::records::storage;

pub(crate) const SQL:&str="CREATE TABLE recipe_library(recipe_id TEXT NOT NULL COLLATE BINARY,revision TEXT NOT NULL,digest TEXT NOT NULL,body BLOB NOT NULL,admission TEXT NOT NULL CHECK(admission IN ('engineering','personal_candidate')),PRIMARY KEY(recipe_id,revision)) STRICT;
CREATE TABLE planner_history(singleton INTEGER PRIMARY KEY CHECK(singleton=1),body BLOB NOT NULL) STRICT;
CREATE TABLE essentials(line_id TEXT PRIMARY KEY NOT NULL COLLATE BINARY,week_id TEXT NOT NULL REFERENCES weeks(week_id),body BLOB NOT NULL) STRICT;
CREATE TABLE local_assets(digest TEXT PRIMARY KEY NOT NULL COLLATE BINARY,payload BLOB NOT NULL) STRICT;";
pub(crate) const PLACEHOLDER:&[u8]=include_bytes!("../assets/engineering-placeholder.png");
fn error(code:ErrorCode,path:&str)->DomainError {DomainError::new(code,path)}
fn empty_history()->PlanningHistory {PlanningHistory{revision:Revision(1),selected:BoundedVec(vec![]),ordered:BoundedVec(vec![]),feedback:BoundedVec(vec![]),hidden_recipe_ids:BoundedVec(vec![])}}
pub(crate) fn present(c:&Connection)->Result<bool,DomainError>{c.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_schema WHERE name='planner_history')",[],|r|r.get(0)).map_err(storage)}
pub(crate) fn history(c:&Connection)->Result<PlanningHistory,DomainError>{if !present(c)?{return Ok(empty_history());}let bytes:Vec<u8>=c.query_row("SELECT body FROM planner_history WHERE singleton=1",[],|r|r.get(0)).map_err(storage)?;parse_document(&bytes)}
fn write_history(c:&Connection,h:&PlanningHistory)->Result<(),DomainError>{let _:PlanningHistory=parse_document(&canonical_bytes(h)?)?;c.execute("UPDATE planner_history SET body=?1 WHERE singleton=1",[canonical_bytes(h)?]).map_err(storage)?;Ok(())}

#[derive(Clone,serde::Serialize)]
pub struct LibraryEntry {pub recipe:Option<RecipeRevision>,pub candidate:Option<RecipeCandidate>,pub reference:RecipeRef,pub admission:String,pub notices:Vec<String>}
#[derive(serde::Serialize)]
pub struct PlannerDetails {pub library:Vec<LibraryEntry>,pub history:PlanningHistory,pub derived:Option<DerivedPlan>,pub essentials:Vec<EssentialsConfirmation>,pub food_error:Option<ErrorCode>}
pub(crate) fn board(c:&mut Connection,week_id:Option<WeekId>,slot:Option<SlotId>,options:crate::planner::ChoiceOptions)->Result<crate::development::DevelopmentBoard,DomainError>{
    let tx=c.transaction().map_err(storage)?;
    let s=crate::snapshot::load(&tx)?;let mut board=crate::development::board_for(&s,week_id.as_ref(),slot.as_ref())?;
    let p=s.profile.0.as_ref().ok_or_else(||error(ErrorCode::MissingReference,"/profile"))?;
    let slot=slot.unwrap_or_else(||board.week.cells[2].slot.clone());
    if !options.extra_recipients.is_empty()||!options.overrides.portions.is_empty()||options.overrides.eat_by.0.is_some()||options.make_extra||options.leftover_source.is_some()||options.eat_by_time.is_some() {
        board.choices=crate::planner::custom_choices(&s,p,&board.week,&slot,options)?;
    }
    let (derived,food_error)=match crate::planner::derive(&s,p,&board.week){Ok(v)=>(v.map(|v|v.plan),None),Err(v)=>(None,Some(v.code))};
    board.details=Some(PlannerDetails{library:library(&tx)?,history:history(&tx)?,derived,essentials:confirmations(&tx)?,food_error});
    tx.commit().map_err(storage)?;Ok(board)
}
pub(crate) fn library(c:&Connection)->Result<Vec<LibraryEntry>,DomainError>{
    let mut q=c.prepare("SELECT body,admission FROM recipe_library ORDER BY recipe_id,CAST(revision AS INTEGER)").map_err(storage)?;
    let rows=q.query_map([],|r|Ok((r.get::<_,Vec<u8>>(0)?,r.get::<_,String>(1)?))).map_err(storage)?;
    rows.map(|v|{let (body,admission)=v.map_err(storage)?;
        let (recipe,candidate,reference)=if admission=="engineering" {let recipe:RecipeRevision=parse_document(&body)?;let reference=crate::development::recipe_reference(&recipe)?;(Some(recipe),None,reference)}else{let candidate:RecipeCandidate=parse_document(&body)?;let reference=RecipeRef{recipe_id:candidate.candidate_id.clone(),revision:candidate.proposed_revision,digest:RecipeDigest(canonical_digest(&candidate)?.0)};(None,Some(candidate),reference)};
        let notices=if admission=="engineering" {vec!["Synthetic engineering premises. Production cooking, food and rights qualification are not established.".into()]}else{vec!["Personal candidate. Quantity, dietary, portion, storage, rights and cooking evidence require review before planning.".into()]};
        Ok(LibraryEntry{recipe,candidate,reference,admission,notices})}).collect()
}
pub(crate) fn confirmations(c:&Connection)->Result<Vec<EssentialsConfirmation>,DomainError>{
    let mut q=c.prepare("SELECT body FROM essentials ORDER BY line_id").map_err(storage)?;
    q.query_map([],|r|r.get::<_,Vec<u8>>(0)).map_err(storage)?.map(|v|parse_document(&v.map_err(storage)?)).collect()
}
pub(crate) fn initialise(c:&Connection)->Result<(),DomainError>{
    c.execute_batch(SQL).map_err(storage)?;
    c.execute("INSERT INTO planner_history VALUES(1,?1)",[canonical_bytes(&empty_history())?]).map_err(storage)?;
    c.execute("INSERT INTO local_assets VALUES(?1,?2)",params![bytes_digest(PLACEHOLDER).0,PLACEHOLDER]).map_err(storage)?;
    for r in crate::planner::catalogue()? {let rf=crate::development::recipe_reference(&r)?;c.execute("INSERT INTO recipe_library VALUES(?1,?2,?3,?4,'engineering')",params![r.recipe_id.0,r.revision.0.to_string(),rf.digest.0,canonical_bytes(&r)?]).map_err(storage)?;}
    let at=crate::development::now()?;let mut h=empty_history();
    for w in crate::current::all_weeks(c)? {let mut empty=w.clone();empty.plan.selections.0.clear();let delta=mstrchf_core::history::selected_index_delta(&[empty],&[w],&at)?;h.selected.0.extend(delta.upsert);}
    h.selected.0.sort_by_key(|v|(v.slot.week_id.0.clone(),v.slot.date.0.clone(),canonical_bytes(&v.slot.meal_type).unwrap_or_default()));write_history(c,&h)
}
pub(crate) fn record_week(c:&Connection,s:&Snapshot,w:&Week)->Result<(),DomainError>{
    if !present(c)? {return Ok(());}
    let mut h=history(c)?;
    let mut old=s.weeks.iter().find(|v|v.range.week_id==w.range.week_id).cloned().unwrap_or_else(||w.clone());
    if !s.weeks.iter().any(|v|v.range.week_id==w.range.week_id){old.plan.selections.0.clear();}
    let delta=mstrchf_core::history::selected_index_delta(&[old],&[w.clone()],&crate::development::now()?)?;
    if !delta.remove_slots.is_empty()||!delta.upsert.is_empty(){
        h.selected.0.retain(|v|!delta.remove_slots.contains(&v.slot)&&!delta.upsert.iter().any(|n|n.slot==v.slot));h.selected.0.extend(delta.upsert);h.selected.0.sort_by_key(|v|(v.slot.week_id.0.clone(),v.slot.date.0.clone(),canonical_bytes(&v.slot.meal_type).unwrap_or_default()));h.revision=h.revision.checked_increment()?;write_history(c,&h)?;
    }
    Ok(())
}
pub(crate) enum Mutation {Weeks(Vec<Week>),Recipe(RecipeCandidate),History(PlanningHistory),Essentials(Vec<(WeekId,EssentialsConfirmation)>)}
pub(crate) fn prepare(c:&Connection,s:&Snapshot,command:&AppCommand)->Result<Mutation,DomainError>{
    if !present(c)? {return Err(crate::records::unsupported());}
    match command {
        AppCommand::SavePersonalRecipe{candidate,previous}=>{
            let entries=library(c)?;if entries.len()>=512{return Err(error(ErrorCode::LimitExceeded,"/library"));}
            let mut r=candidate.clone();
            if let Some(previous)=&previous.0 {
                let old=entries.iter().filter(|v|v.reference.recipe_id==previous.recipe_id).max_by_key(|v|v.reference.revision).ok_or_else(||error(ErrorCode::MissingReference,"/recipe/previous"))?;
                if old.admission!="personal_candidate"||old.reference!=*previous{return Err(error(ErrorCode::ContextStale,"/recipe/previous"));}
                r.candidate_id=old.reference.recipe_id.clone();r.proposed_revision=old.reference.revision.checked_increment()?;
            }else{r.candidate_id=RecipeId(format!("rc_{}",crate::records::random_id()?));r.proposed_revision=Revision(1);}
            // Imported claims and model instructions do not grant food/rights authority.
            r.image=Nullable(None);
            let r:RecipeCandidate=parse_document(&canonical_bytes(&r)?)?;
            let validation=mstrchf_core::recipes::validate_recipe(&r)?;
            let mut r=r;r.unresolved=validation.capabilities.blocking_issues;
            Ok(Mutation::Recipe(r))
        }
        AppCommand::RecordFeedback{feedback}=>{
            let weeks=crate::current::all_weeks(c)?;
            let selected=weeks.iter().flat_map(|v|v.plan.selections.iter()).find(|v|v.slot==feedback.slot&&v.recipe==feedback.recipe).ok_or_else(||error(ErrorCode::ForeignReference,"/feedback/selection"))?;
            if feedback.portion_change.0.is_some()||feedback.portion_change_accepted{return Err(error(ErrorCode::UnsupportedCapability,"/feedback/portion_change"));}
            let _=selected;let mut h=history(c)?;if h.feedback.len()>=4096{return Err(error(ErrorCode::LimitExceeded,"/feedback"));}
            let mut feedback=feedback.clone();feedback.recorded_at=crate::development::now()?;
            if feedback.hide&&!h.hidden_recipe_ids.contains(&feedback.recipe.recipe_id){h.hidden_recipe_ids.0.push(feedback.recipe.recipe_id.clone());h.hidden_recipe_ids.0.sort();}
            h.feedback.0.push(feedback);h.revision=h.revision.checked_increment()?;Ok(Mutation::History(h))
        }
        AppCommand::RestoreHiddenRecipe{recipe_id}=>{let mut h=history(c)?;h.hidden_recipe_ids.0.retain(|v|v!=recipe_id);h.revision=h.revision.checked_increment()?;Ok(Mutation::History(h))}
        AppCommand::SuggestMeals{week_id,mode,slots,allow_repeats}=>{
            let p=s.profile.0.as_ref().ok_or_else(||error(ErrorCode::MissingReference,"/profile"))?;
            let w=s.weeks.iter().find(|v|&v.range.week_id==week_id).ok_or_else(||error(ErrorCode::ForeignReference,"/week_id"))?;
            let mut h=history(c)?;
            let candidate=crate::planner::suggest(s,p,w,&mut h,*mode,&slots.0,*allow_repeats)?;
            Ok(Mutation::Weeks(vec![candidate]))
        }
        AppCommand::ConfirmEssentials{confirmations:given}=>{
            let p=s.profile.0.as_ref().ok_or_else(||error(ErrorCode::MissingReference,"/profile"))?;
            let mut out=Vec::new();let mut seen=std::collections::BTreeSet::new();
            for confirmation in &given.0 {
                if !seen.insert(&confirmation.line_id){return Err(error(ErrorCode::DuplicateKey,"/confirmations"));}
                let mut matched=None;
                for w in &s.weeks.0 {if let Some(food)=crate::planner::derive(s,p,w)? {if food.plan.demand.lines.iter().any(|v|v.line_id==confirmation.line_id){validate_essentials_binding(confirmation,&food.plan.demand)?;matched=Some(w.range.week_id.clone());break;}}}
                let week=matched.ok_or_else(||error(ErrorCode::ForeignReference,"/confirmations/line_id"))?;
                let mut confirmation=confirmation.clone();confirmation.confirmed_at=crate::development::now()?;out.push((week,confirmation));
            }
            Ok(Mutation::Essentials(out))
        }
        _=>Err(crate::records::unsupported()),
    }
}
pub(crate) fn commit(c:&Connection,s:&Snapshot,mutation:Mutation)->Result<(),DomainError>{
    match mutation {
        Mutation::Weeks(weeks)=>for mut w in weeks {record_week(c,s,&w)?;w.revision=w.revision.checked_increment()?;w.plan.revision=w.plan.revision.checked_increment()?;w.approved_plan=Nullable(None);w.confirmed_order=Nullable(None);c.execute("UPDATE weeks SET revision=?1,body=?2 WHERE week_id=?3",params![w.revision.0.to_string(),canonical_bytes(&w)?,w.range.week_id.0]).map_err(storage)?;},
        Mutation::Recipe(r)=>{let digest=canonical_digest(&r)?;c.execute("INSERT INTO recipe_library VALUES(?1,?2,?3,?4,'personal_candidate')",params![r.candidate_id.0,r.proposed_revision.0.to_string(),digest.0,canonical_bytes(&r)?]).map_err(storage)?;},
        Mutation::History(h)=>write_history(c,&h)?,
        Mutation::Essentials(rows)=>for (week,confirmation) in rows {c.execute("INSERT INTO essentials VALUES(?1,?2,?3) ON CONFLICT(line_id) DO UPDATE SET week_id=excluded.week_id,body=excluded.body",params![confirmation.line_id.0,week.0,canonical_bytes(&confirmation)?]).map_err(storage)?;},
    }
    Ok(())
}
pub(crate) fn audit(c:&Connection)->Result<(),DomainError>{
    let entries=library(c)?;if entries.len()>512{return Err(error(ErrorCode::LimitExceeded,"/library"));}
    let compiled=crate::planner::catalogue()?;
    if entries.iter().filter(|v|v.admission=="engineering").count()!=compiled.len(){return Err(storage("engineering library count"));}
    let mut q=c.prepare("SELECT recipe_id,revision,digest,body,admission FROM recipe_library ORDER BY recipe_id,CAST(revision AS INTEGER)").map_err(storage)?;
    for row in q.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,r.get::<_,Vec<u8>>(3)?,r.get::<_,String>(4)?))).map_err(storage)? {
        let (id,revision,digest,body,admission)=row.map_err(storage)?;
        if admission=="engineering" {let r:RecipeRevision=parse_document(&body)?;if r.recipe_id.0!=id||r.revision.0.to_string()!=revision||recipe_digest(&r)?.0!=digest||canonical_bytes(&r)?!=body||!compiled.contains(&r){return Err(storage("engineering library binding"));}}
        else {let r:RecipeCandidate=parse_document(&body)?;let validation=mstrchf_core::recipes::validate_recipe(&r)?;if r.candidate_id.0!=id||r.proposed_revision.0.to_string()!=revision||canonical_digest(&r)?.0!=digest||canonical_bytes(&r)?!=body||r.image.0.is_some()||r.unresolved!=validation.capabilities.blocking_issues{return Err(storage("personal candidate binding"));}}
    }
    let h=history(c)?;let retained=crate::current::all_weeks(c)?;
    let mut expected=Vec::new();for w in &retained {let active=mstrchf_core::availability::eligible_slots(w)?;expected.extend(w.plan.selections.iter().filter(|v|active.contains(&v.slot)));}
    if h.selected.len()!=expected.len()||h.selected.iter().any(|v|!expected.iter().any(|s|s.slot==v.slot&&s.recipe==v.recipe&&s.servings==v.servings&&s.extras==v.extras&&s.repeat_authorised==v.repeat_authorised)){return Err(storage("selected index projection"));}
    for confirmation in confirmations(c)? {let _:EssentialsConfirmation=parse_document(&canonical_bytes(&confirmation)?)?;}
    let mut q=c.prepare("SELECT digest,payload FROM local_assets ORDER BY digest").map_err(storage)?;
    for row in q.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Vec<u8>>(1)?))).map_err(storage)? {let (digest,bytes)=row.map_err(storage)?;if bytes!=PLACEHOLDER||bytes_digest(&bytes).0!=digest{return Err(storage("local asset bytes"));}}
    Ok(())
}
