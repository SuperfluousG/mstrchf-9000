import {useMemo,useState,type ReactNode} from 'react';
import type {RecipeCandidate,RecipeRef,SourceRecord} from '../../generated/contracts';
import {usePlanner} from '../../runtime/PlannerProvider';
import {isReviewedEntry,human,rational,type LibraryEntry,type PlannerDetails} from './plannerTypes';
export function personalRecipeSource(url:string,recordedAt:SourceRecord['retrieved_at'],previous?:{source:SourceRecord;reference:RecipeRef}):SourceRecord{
 const reference=url.trim()||null;
 if(previous&&reference===previous.source.url)return {...previous.source,authors:[...previous.source.authors]};
 return {url:reference as SourceRecord['url'],source_revision:null,retrieved_at:recordedAt,authors:[],
  adaptation_notice:previous
   ?`Unreviewed personal adaptation of ${previous.reference.recipe_id}, revision ${previous.reference.revision}. The source reference was ${reference?'replaced':'cleared'}. This timestamp records that edit, not retrieval of a source. Earlier source records and credits remain with the immutable recipe revisions. Edited quantities and methods have not been validated.`
   :`Personal pasted or entered recipe; no source or cooking review established. This timestamp records the entry${reference?', not retrieval of the supplied source reference':''}.`,
  origin:previous||reference?'adapted':'user_authored'};
}
const sourceKey=(source:SourceRecord)=>JSON.stringify([source.url,source.source_revision,source.retrieved_at,source.authors,source.adaptation_notice,source.origin]);
export function earlierCandidateSources(entry:LibraryEntry,library:LibraryEntry[]){
 const current=entry.candidate?.source;if(!current)return [];
 const seen=new Set([sourceKey(current)]);
 return library.filter(value=>value.reference.recipe_id===entry.reference.recipe_id&&BigInt(value.reference.revision)<BigInt(entry.reference.revision))
  .sort((a,b)=>BigInt(a.reference.revision)<BigInt(b.reference.revision)?-1:1)
  .flatMap(value=>{const source=value.candidate?.source??value.recipe?.source;if(!source||seen.has(sourceKey(source)))return [];seen.add(sourceKey(source));return [{revision:value.reference.revision,source}];});
}
function CandidateSource({source}:{source:SourceRecord}){
 return <div style={{overflowWrap:'anywhere'}}><p>Recorded reference: {source.url??'No source URL recorded'}</p>
  <p>Source authors: {source.authors.length?source.authors.join('; '):'Not recorded'}</p>
  <p>Source revision: {source.source_revision??'Not recorded'} · Source record timestamp: {source.retrieved_at}</p>
  {source.adaptation_notice&&<p style={{whiteSpace:'pre-wrap',overflowWrap:'anywhere'}}>{source.adaptation_notice}</p>}
 </div>;
}
function EarlierSourceRecords({entry,library}:{entry:LibraryEntry;library:LibraryEntry[]}){
 const [open,setOpen]=useState(false);
 const earlier=useMemo(()=>open?earlierCandidateSources(entry,library):[],[open,entry,library]);
 return <details open={open} onToggle={event=>setOpen(event.currentTarget.open)}><summary>Earlier source records and credits</summary>
  {open&&(earlier.length?earlier.map(value=><section key={value.revision}><p>Retained from revision {value.revision}</p><CandidateSource source={value.source}/></section>):<p>No distinct earlier source records. Earlier revisions use the source record shown above.</p>)}
 </details>;
}
export function RecipeLibrary({details,renderImage,allowPersonalEditing=true}:{details:PlannerDetails;allowPersonalEditing?:boolean;renderImage?:(entry:LibraryEntry)=>ReactNode}){
 const {session}=usePlanner();const [editing,setEditing]=useState<LibraryEntry|null>(null);
 const [query,setQuery]=useState('');
 const [title,setTitle]=useState('');const [source,setSource]=useState('');const [ingredients,setIngredients]=useState('');const [instructions,setInstructions]=useState('');const [notice,setNotice]=useState('');
 const revisionBounds=useMemo(()=>{const bounds=new Map<string,{first:bigint;last:bigint}>();for(const entry of details.library){const revision=BigInt(entry.reference.revision),prior=bounds.get(entry.reference.recipe_id);bounds.set(entry.reference.recipe_id,{first:prior&&prior.first<revision?prior.first:revision,last:prior&&prior.last>revision?prior.last:revision});}return bounds;},[details.library]);
 const edit=(entry:LibraryEntry|null)=>{setEditing(entry);const c=entry?.candidate;setTitle(c?.title??'');setSource(c?.source.url??'');setIngredients(c?.ingredients.map(i=>i.source_text).join('\n')??'');setInstructions(c?.steps.map(s=>s.text).join('\n')??'');setNotice('');};
 const save=async()=>{
  const lines=ingredients.split('\n').map(s=>s.trim()).filter(Boolean);const steps=instructions.split('\n').map(s=>s.trim()).filter(Boolean);
  if(!title.trim()||!lines.length||!steps.length||lines.length>256||steps.length>256){setNotice('Enter a title, ingredients and method, with at most 256 lines in each.');return;}
  const candidateId:RecipeCandidate['candidate_id']=editing?.candidate?.candidate_id??`rc_${crypto.randomUUID().replaceAll('-','')}` as never;
  const candidate:RecipeCandidate={schema_version:1,candidate_id:candidateId,proposed_revision:'1',title:title.trim() as never,source:personalRecipeSource(source,new Date().toISOString().replace(/\.\d{3}Z$/,'Z') as never,editing?.candidate?{source:editing.candidate.source,reference:editing.reference}:undefined),ingredients:lines.map(line=>({ingredient_id:null,component_id:null,name:line as never,source_text:line as never,quantity:null})),yield:null,steps:steps.map((line,i)=>({step_id:i+1,text:line as never,ingredient_ids:[],component_ids:[],active_minutes:0,elapsed_minutes:0,depends_on:[]})),unresolved:[],image:null};
  await session.submit({kind:'save_personal_recipe',data:{candidate,previous:editing?.reference??null}});
  if(session.state.disposition==='committed'&&session.state.read==='ready'){edit(null);setNotice('Personal candidate saved. Quantities and evidence remain unresolved.');}else setNotice('The candidate was not confirmed saved. Review the saved library or recover the request.');
 };
 const matches=(entry:LibraryEntry)=>(entry.recipe?.title??entry.candidate?.title??'').toLocaleLowerCase('en-GB').includes(query.trim().toLocaleLowerCase('en-GB'));
 const reviewed=details.library.filter(entry=>isReviewedEntry(entry)&&entry.recipe&&matches(entry));
 const candidates=details.library.filter(entry=>entry.candidate&&!(isReviewedEntry(entry)&&entry.recipe)&&matches(entry));
 const fixtures=details.library.filter(entry=>!entry.candidate&&!(isReviewedEntry(entry)&&entry.recipe)&&matches(entry));
 const renderDetails=(entry:LibraryEntry,showImage=true)=>{const r=entry.recipe;const c=entry.candidate;const bounds=revisionBounds.get(entry.reference.recipe_id)!,revision=BigInt(entry.reference.revision),latest=bounds.last===revision;return <details className="recipe-library-details" key={`${entry.reference.recipe_id}/${entry.reference.revision}`}><summary>{r?.title??c?.title} · revision {entry.reference.revision} · {human(entry.admission)}</summary><p>{entry.notices.join(' ')}</p>{showImage&&(renderImage?.(entry)??<div className="recipe-placeholder">{r?.image?'Engineering illustration placeholder':'No qualified recipe image'}</div>)}{c?<><p>Source notes describe the retained source, including any published claims; they do not validate edited quantities or methods.</p><CandidateSource source={c.source}/>{bounds.first<revision&&<EarlierSourceRecords entry={entry} library={details.library}/>}</>:<p>{r?.source.url??'Locally entered source'}</p>}{r&&<p>Source batch: {r.yield.basis}. Equipment: {r.equipment.map(human).join(', ')||'no powered equipment'}. {r.active_minutes} min hands-on · {r.elapsed_minutes} min elapsed (estimates).</p>}{r?.ingredients.map(i=><p key={i.ingredient_id}>{i.quantity?`${rational(i.quantity.amount)} ${i.quantity.unit} · ${i.name} · ${human(i.quantity.form)}`:i.name}</p>)}{c?.ingredients.map((i,n)=><p key={n}>{i.source_text} · quantity {i.quantity?'supplied, unqualified':'unresolved'}</p>)}{(r?.steps??c?.steps??[]).map(s=><p key={s.step_id}>{s.step_id}. {s.text}</p>)}{r&&<p>{r.text_rights.attribution} {r.text_rights.licence_id}</p>}{c&&allowPersonalEditing&&<><p>{c.unresolved.map(i=>human(i.code)).join('; ')}</p><button disabled={!session.canMutate||!latest} onClick={()=>edit(entry)}>Edit personal recipe</button></>}</details>;};
 return <section className="planner-panel recipe-library"><header className="recipe-library-heading"><h2>Recipe library</h2><p>Good things to make, all in one place. Each reviewed recipe keeps its specific use limits.</p></header>
  <label className="recipe-library-search">Find a recipe<input type="search" value={query} onChange={event=>setQuery(event.target.value)} placeholder="Search by name"/></label>
  <div className="recipe-library-grid">{reviewed.map(entry=>{const recipe=entry.recipe!;return <article className="recipe-library-card" key={`${entry.reference.recipe_id}/${entry.reference.revision}`}>
   {renderImage?.(entry)??<div className="recipe-placeholder">No qualified recipe image</div>}
   <div className="recipe-library-card-body"><p className="recipe-library-card-meta">{recipe.meal_types.map(human).join(' · ')}{recipe.elapsed_minutes>0?` · ${recipe.elapsed_minutes} min`:''}</p><h3>{recipe.title}</h3><p className="recipe-library-card-scope">Reviewed recipe · revision {entry.reference.revision}</p>{renderDetails(entry,false)}</div>
  </article>;})}</div>
  {!reviewed.length&&<p className="recipe-library-empty">{query.trim()?'No reviewed recipes match this name.':'Your reviewed recipes will appear here.'}</p>}
  <details className="recipe-library-secondary"><summary>Personal candidates · {candidates.length}</summary><p>Personal candidates remain outside planning. Source notes, edits and every saved revision are retained.</p>{candidates.map(entry=>renderDetails(entry))}</details>
  <details className="recipe-library-secondary"><summary>Engineering fixtures · {fixtures.length}</summary><p>Retained test recipes. These are not qualified for production cooking or shopping.</p>{fixtures.map(entry=>renderDetails(entry))}</details>
  {allowPersonalEditing&&<fieldset><legend>{editing?'New revision of personal recipe':'Enter or paste a personal recipe'}</legend><label>Recipe title<input maxLength={240} value={title} onChange={e=>setTitle(e.target.value)}/></label><label>Source URL (optional)<input type="url" maxLength={2048} value={source} onChange={e=>setSource(e.target.value)}/></label><label>Ingredients (one per line; include the source wording)<textarea rows={5} maxLength={24000} value={ingredients} onChange={e=>setIngredients(e.target.value)}/></label><label>Method (one step per line)<textarea rows={6} maxLength={24000} value={instructions} onChange={e=>setInstructions(e.target.value)}/></label><p>Pasted wording is kept as text. Ingredient identities, quantities, yield, dietary, portion, storage and rights evidence remain unresolved; this does not create a planning recipe.</p><button disabled={!session.canMutate} onClick={()=>void save()}>{editing?'Save new personal revision':'Save personal candidate'}</button>{editing&&<button onClick={()=>edit(null)}>Cancel edit</button>}{notice&&<p role="status">{notice}</p>}</fieldset>}
 </section>;
}
