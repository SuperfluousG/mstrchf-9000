import {useState} from 'react';
import type {HouseholdProfile,HouseholdDraft,HardRequirement,Diner,AgeBand,PortionPresetName,EquipmentCode,HouseholdStage,MealLocation} from '../../generated/contracts';
import type {LibraryEntry} from '../week/plannerTypes';
import {usePlanner} from '../../runtime/PlannerProvider';

const stages:HouseholdStage[]=['household','food_preferences','usual_week','connections'];
const labels=['Household','Food preferences','Usual week','Connections'];
const diets=['vegetarian','vegan','gluten_free'] as const;
const allergens=['milk','eggs','peanuts','tree_nuts','cereals_containing_gluten','soybeans','sesame','fish','crustaceans','molluscs','celery','mustard','lupin','sulphur_dioxide_sulphites'] as const;
const equipment:EquipmentCode[]=['hob','oven','microwave','fridge','kettle','toaster','blender','slow_cooker','pressure_cooker'];
const human=(s:string)=>s.replaceAll('_',' ');
export function Settings({onClose,onboarding=false,library=[]}:{onClose:()=>void;onboarding?:boolean;library?:LibraryEntry[]}){
 const {session,state}=usePlanner();
 const saved=state.snapshot!;
 const [profile,setProfile]=useState<HouseholdProfile>(()=>{
   const p=structuredClone(saved.profile!);const d=saved.household_draft;
   if(onboarding&&p){if(d.diners.length)p.diners=structuredClone(d.diners);if(d.preferences)p.preferences=structuredClone(d.preferences);if(d.usual_week)p.usual_week=structuredClone(d.usual_week);if(d.connections)p.connections=structuredClone(d.connections);}
   return p;
 });
 const [step,setStep]=useState(()=>onboarding?Math.min(saved.household_draft.completed_stages.length,3):0);
 const [notice,setNotice]=useState('');
 if(!profile)return <section><p>A saved household is required.</p><button onClick={onClose}>Back to kitchen</button></section>;
 const edit=(fn:(p:HouseholdProfile)=>void)=>setProfile(p=>{const next=structuredClone(p);fn(next);return next;});
 const requirements=(d:Diner)=>d.restrictions.kind==='confirmed'?d.restrictions.data.requirements:[];
 const toggle=(index:number,r:HardRequirement)=>edit(p=>{
   const diner=p.diners[index]!;const key=JSON.stringify(r);
   const list=requirements(diner);const exists=list.some(x=>JSON.stringify(x)===key);
   const next=exists?list.filter(x=>JSON.stringify(x)!==key):[...list,r];
   diner.restrictions=next.length?{kind:'confirmed',data:{confirmed_at:new Date().toISOString().replace(/\.\d{3}Z$/,'Z') as never,requirements:next}}:
     {kind:'unconfirmed',data:{}};
 });
 const save=async()=>{
   if(!session.canMutate)return;
   const current=state.snapshot!;const d=current.household_draft;
   const proposed=onboarding?{...profile,diners:d.diners,preferences:d.preferences!,usual_week:d.usual_week!,connections:d.connections!}:profile;
   await session.submit({kind:'update_profile',data:{profile:{...proposed,revision:current.profile!.revision,connections:{...proposed.connections,privacy:current.privacy}},expected_profile_revision:current.profile!.revision}});
   setNotice(session.state.disposition==='committed'&&session.state.read==='ready'?'Household saved. Existing weeks keep their choices.':'Household saving needs review or recovery.');
 };
 const saveStage=async()=>{
   if(!session.canMutate)return;
   const current=state.snapshot!.household_draft;
   const draft:HouseholdDraft={...structuredClone(current),diners:structuredClone(profile.diners),
     preferences:step>=1?structuredClone(profile.preferences):current.preferences,
     usual_week:step>=2?structuredClone(profile.usual_week):current.usual_week,
     connections:step>=3?{...structuredClone(profile.connections),privacy:state.snapshot!.privacy}:current.connections,
     completed_stages:[]};
   if(draft.diners.length&&draft.diners.every(d=>d.restrictions.kind!=='unconfirmed'))draft.completed_stages.push('household');
   if(draft.preferences)draft.completed_stages.push('food_preferences');
   if(draft.usual_week)draft.completed_stages.push('usual_week');
   if(draft.connections)draft.completed_stages.push('connections');
   draft.completed_stages.sort();
   await session.submit({kind:'save_household_draft',data:{draft,restriction_issue_edits:[]}});
   if(session.state.disposition==='committed'&&session.state.read==='ready'){setStep(n=>Math.min(n+1,3));setNotice('Setup progress saved.');}
 };
 return <section className="planner-panel settings-panel" aria-label={onboarding?'Household setup':'Settings'}>
  <header><h2>{onboarding?'Set up your kitchen':'Settings'}</h2><button onClick={onClose}>Back to kitchen</button></header>
  {notice&&<p role="status">{notice}</p>}
  {onboarding&&<nav aria-label="Setup stages">{labels.map((label,i)=><button key={label} aria-current={step===i?'step':undefined} onClick={()=>setStep(i)}>{i+1}. {label}</button>)}</nav>}
  {(!onboarding||step===0)&&<fieldset><legend>Household</legend>
   {profile.diners.map((d,i)=><fieldset key={d.diner_id}><legend>{d.display_name??`Diner ${i+1}`}</legend>
    <label>Name (optional)<input value={d.display_name??''} maxLength={80} onChange={e=>edit(p=>{p.diners[i]!.display_name=(e.target.value||null) as never;})}/></label>
    <label>Age band<select value={d.age_band} onChange={e=>edit(p=>{p.diners[i]!.age_band=e.target.value as AgeBand;})}>{['adult','teen','child_1_3','child_4_6','child_7_10','child_11_12'].map(x=><option key={x} value={x}>{human(x)}</option>)}</select></label>
    <label>Usual portion<select value={d.portions.preset} onChange={e=>edit(p=>{p.diners[i]!.portions.preset=e.target.value as PortionPresetName;})}>{['smaller','regular','larger'].map(x=><option key={x}>{x}</option>)}</select></label>
    <p>Portions use recipe-specific reviewed servings. An unsupported age or preset stays unresolved.</p>
    <details><summary>Portions by meal type</summary>{(['breakfast','lunch','dinner'] as const).map(m=><label key={m}>{m}<select value={d.portions.meal_overrides.find(o=>o.meal_type===m)?.preset??''} onChange={e=>edit(p=>{const portions=p.diners[i]!.portions;portions.meal_overrides=portions.meal_overrides.filter(o=>o.meal_type!==m);if(e.target.value)portions.meal_overrides.push({meal_type:m,preset:e.target.value as PortionPresetName,adjustment:{numerator:'1',denominator:'1'}});portions.meal_overrides.sort((a,b)=>a.meal_type.localeCompare(b.meal_type));})}><option value="">Usual portion</option>{['smaller','regular','larger'].map(x=><option key={x}>{x}</option>)}</select></label>)}</details>
    <details><summary>Ingredients to suggest less often</summary>{[...new Map(library.flatMap(v=>v.recipe?.ingredients??[]).map(v=>[v.ingredient_id,v])).values()].map(ingredient=><label key={ingredient.ingredient_id}><input type="checkbox" checked={d.dislikes.includes(ingredient.ingredient_id)} onChange={e=>edit(p=>{p.diners[i]!.dislikes=e.target.checked?[...p.diners[i]!.dislikes,ingredient.ingredient_id].sort():p.diners[i]!.dislikes.filter(id=>id!==ingredient.ingredient_id);})}/>{ingredient.name}</label>)}<p>Dislikes affect suggestions. Dietary exclusions remain separate.</p></details>
    <div className="requirements">{diets.map(code=><label key={code}><input type="checkbox" checked={requirements(d).some(r=>r.kind==='diet'&&r.data.code===code)} onChange={()=>toggle(i,{kind:'diet',data:{code}})}/>{human(code)}</label>)}</div>
    <details><summary>Allergens to exclude</summary>{allergens.map(code=><label key={code}><input type="checkbox" checked={requirements(d).some(r=>r.kind==='allergen_exclusion'&&r.data.code===code)} onChange={()=>toggle(i,{kind:'allergen_exclusion',data:{code}})}/>{human(code)}</label>)}</details>
    <button disabled={requirements(d).length>0} onClick={()=>edit(p=>{p.diners[i]!.restrictions={kind:'confirmed_none',data:{confirmed_at:new Date().toISOString().replace(/\.\d{3}Z$/,'Z') as never}};})}>Confirm no dietary requirements</button>
    <p>{d.restrictions.kind==='unconfirmed'?'Dietary requirements need confirmation.':d.restrictions.kind==='confirmed_none'?'Confirmed: no dietary requirements.':'Dietary requirements confirmed.'}</p>
    <button disabled={profile.diners.length===1} onClick={()=>edit(p=>{p.diners=p.diners.filter(x=>x.diner_id!==d.diner_id);for(const c of p.usual_week.cells)c.attendee_ids=c.attendee_ids.filter(id=>id!==d.diner_id);})}>Remove diner</button><p>A diner referenced by a saved week must remain until those references are resolved.</p>
   </fieldset>)}
   <button disabled={profile.diners.length>=64} onClick={()=>edit(p=>{const d=structuredClone(p.diners[0]!);d.diner_id=`dn_${crypto.randomUUID().replaceAll('-','')}` as never;d.display_name=null;d.restrictions={kind:'unconfirmed',data:{}};p.diners.push(d);for(const c of p.usual_week.cells)c.attendee_ids.push(d.diner_id);})}>Add diner</button>
  </fieldset>}
  {(!onboarding||step===1)&&<fieldset><legend>Food preferences</legend>
   <label>Spending style<select value={profile.preferences.spending_style} onChange={e=>edit(p=>{p.preferences.spending_style=e.target.value as never;})}>{['value','balanced','flexible'].map(x=><option key={x}>{x}</option>)}</select></label>
   <p>A preference; live prices are unavailable in this development kitchen.</p>
   {(['higher_protein','more_variety','quick_meals','whole_food_preference'] as const).map(key=><label key={key}><input type="checkbox" checked={profile.preferences[key]} onChange={e=>edit(p=>{p.preferences[key]=e.target.checked;})}/>{human(key)}</label>)}
   <label>Preferred hands-on minutes<input type="number" min={1} max={480} value={profile.preferences.preferred_hands_on_minutes} onChange={e=>edit(p=>{p.preferences.preferred_hands_on_minutes=Number(e.target.value);})}/></label>
   <fieldset><legend>Available equipment</legend>{equipment.map(item=><label key={item}><input type="checkbox" checked={profile.preferences.equipment.includes(item)} onChange={e=>edit(p=>{p.preferences.equipment=e.target.checked?[...p.preferences.equipment,item]:p.preferences.equipment.filter(x=>x!==item);p.preferences.equipment.sort();})}/>{human(item)}</label>)}</fieldset>
  </fieldset>}
  {(!onboarding||step===2)&&<fieldset><legend>Usual week</legend>
   <p>These defaults apply to new weeks. Existing weeks retain their availability.</p>
   <label>Home time zone<input value={profile.usual_week.zone} maxLength={80} onChange={e=>edit(p=>{p.usual_week.zone=e.target.value as never;})}/></label>
   <div className="availability-scroll"><table><thead><tr><th>Usual meal</th><th>Location</th><th>Diners</th></tr></thead><tbody>{profile.usual_week.cells.map((c,i)=><tr key={`${c.weekday}/${c.meal_type}`}><th>{human(c.weekday)} {c.meal_type}</th><td><select aria-label={`Usual ${human(c.weekday)} ${c.meal_type}`} value={c.choice} onChange={e=>edit(p=>{p.usual_week.cells[i]!.choice=e.target.value as MealLocation;})}>{['at_home','on_the_move','skip'].map(v=><option key={v} value={v}>{human(v)}</option>)}</select></td><td>{profile.diners.map(d=><label key={d.diner_id}><input type="checkbox" checked={c.attendee_ids.includes(d.diner_id)} onChange={e=>edit(p=>{const cell=p.usual_week.cells[i]!;cell.attendee_ids=e.target.checked?[...cell.attendee_ids,d.diner_id].sort():cell.attendee_ids.filter(id=>id!==d.diner_id);})}/>{d.display_name??'Diner'}</label>)}</td></tr>)}</tbody></table></div>
   {profile.preferences.meal_times.map((t,i)=><label key={t.meal_type}>{human(t.meal_type)} eat-by time<input type="time" value={t.time.slice(0,5)} onChange={e=>edit(p=>{p.preferences.meal_times[i]!.time=`${e.target.value}:00` as never;})}/></label>)}
   <label><input type="checkbox" checked={profile.preferences.mobility.reheating} onChange={e=>edit(p=>{p.preferences.mobility.reheating=e.target.checked;})}/>Reheating available on the move</label>
   <label><input type="checkbox" checked={profile.preferences.mobility.cold_storage} onChange={e=>edit(p=>{p.preferences.mobility.cold_storage=e.target.checked;})}/>Cold storage available on the move</label>
   <label>Food temperature on the move<select value={profile.preferences.mobility.temperature} onChange={e=>edit(p=>{p.preferences.mobility.temperature=e.target.value as never;})}>{['hot','cold','either'].map(x=><option key={x}>{x}</option>)}</select></label>
   <label><input type="checkbox" checked={profile.usual_week.suggest_leftovers} onChange={e=>edit(p=>{p.usual_week.suggest_leftovers=e.target.checked;})}/>Allow supported leftover suggestions</label>
  </fieldset>}
  {(!onboarding||step===3)&&<fieldset><legend>Connections</legend>
   <label>Preferred AI harness<select value={profile.connections.harness??''} onChange={e=>edit(p=>{p.connections.harness=(e.target.value||null) as never;})}><option value="">Choose later</option><option value="codex_local">Codex</option><option value="claude_cowork">Claude</option></select></label>
   <p>Choosing a harness does not connect an account. Shopping and calendar qualification are later stages.</p>
   <label><input type="checkbox" checked={saved.privacy.use_ai_features} disabled={!session.canMutate} onChange={e=>void session.submit({kind:'set_privacy',data:{policy:{...saved.privacy,use_ai_features:e.target.checked},expected_policy_revision:saved.privacy.revision}})}/>Use AI features</label>
   <label><input type="checkbox" checked={saved.privacy.include_household_requirements} disabled={!session.canMutate} onChange={e=>void session.submit({kind:'set_privacy',data:{policy:{...saved.privacy,include_household_requirements:e.target.checked},expected_policy_revision:saved.privacy.revision}})}/>Include household requirements in AI requests</label>
  </fieldset>}
  {onboarding&&<button className="button-primary" disabled={!session.canMutate} onClick={()=>void saveStage()}>Save {labels[step]}{step<3?' and continue':''}</button>}
  <button className="button-primary" disabled={!session.canMutate||(onboarding&&saved.household_draft.completed_stages.length!==4)} onClick={()=>void save()}>{onboarding?'Apply completed household':'Save settings'}</button>
 </section>;
}
