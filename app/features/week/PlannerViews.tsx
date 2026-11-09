import {useEffect,useRef,useState} from 'react';
import type {Week,Cell,PersonRef,PortionPreference,PortionPresetName,AgeBand,FeedbackRating,FeedbackReason,EssentialsChoice,SlotId,PlanMode,HardRequirement,Snapshot,AppCommand} from '../../generated/contracts';
import {usePlanner} from '../../runtime/PlannerProvider';
import {isReviewedEntry,human,rational,slotKey,titleFor,type BoardOptions,type PlannerDetails} from './plannerTypes';
import {exportShoppingList,originText,quantityText} from './shoppingList';
import {adjustmentText,effectivePortion,parsePortionAdjustment,portionContextStamp,portionInputError,previewPortionOverride,PORTION_INPUT_HELP} from './portions';
const now=()=>new Date().toISOString().replace(/\.\d{3}Z$/,'Z') as never;
const personKey=(p:PersonRef)=>JSON.stringify(p);
const requirementKey=(r:HardRequirement)=>`${r.kind}/${r.kind==='ingredient_exclusion'?r.data.ingredient_id:r.data.code}`;
const diets=['vegetarian','vegan','gluten_free'] as const;
const allergens=['milk','eggs','peanuts','tree_nuts','cereals_containing_gluten','soybeans','sesame','fish','crustaceans','molluscs','celery','mustard','lupin','sulphur_dioxide_sulphites'] as const;

export function MealControls({week,cell,onOptions,portableAvailable=false}:{week:Week;cell:Cell;onOptions:(v:BoardOptions)=>void;portableAvailable?:boolean}){
 const {session,state}=usePlanner();const profile=state.snapshot!.profile!;
 const [people,setPeople]=useState(()=>structuredClone(cell));
 const contextBase=useRef(portionContextStamp([week,profile]));
 const currentWeek=state.snapshot!.weeks.find(value=>value.range.week_id===week.range.week_id);
 const stale=contextBase.current!==portionContextStamp([currentWeek,profile]);
 const [portions,setPortions]=useState<Record<string,PortionPresetName|''>>({});
 const [adjustments,setAdjustments]=useState<Record<string,string>>({});
 const [guestAdjustments,setGuestAdjustments]=useState<Record<string,string>>({});
 const [notice,setNotice]=useState('');
 const [extras,setExtras]=useState<string[]>([]);const [makeExtra,setMakeExtra]=useState(false);
 const [leftover,setLeftover]=useState('');const [time,setTime]=useState('');const [resolution,setResolution]=useState<'unambiguous'|'earlier'|'later'>('unambiguous');
 const [packing,setPacking]=useState(''),[departure,setDeparture]=useState('');
 const [packingResolution,setPackingResolution]=useState<'unambiguous'|'earlier'|'later'>('unambiguous');
 const [departureResolution,setDepartureResolution]=useState<'unambiguous'|'earlier'|'later'>('unambiguous');
 const [chillingConfirmed,setChillingConfirmed]=useState(false);
 const portable=portableAvailable&&cell.choice==='on_the_move'&&cell.slot.meal_type==='lunch'&&week.plan.selections.some(selection=>slotKey(selection.slot)===leftover&&selection.recipe.recipe_id==='rc_8000000000000000000000000000000d');
 const declarationContext=portionContextStamp([currentWeek,profile]);
 useEffect(()=>{setChillingConfirmed(false);},[declarationContext,portableAvailable]);
 const attendees:{person:PersonRef;label:string;portion:PortionPreference}[]=[...cell.attendee_ids.flatMap(id=>{const diner=profile.diners.find(d=>d.diner_id===id);return diner?[{person:{kind:'diner',data:{diner_id:id}} as PersonRef,label:diner.display_name??'Diner',portion:diner.portions}]:[];}),...(cell.occasion?.guests.map((g,i)=>({person:{kind:'guest',data:{occasion_id:cell.occasion!.occasion_id,guest_id:g.guest_id}} as PersonRef,label:`Guest ${i+1}`,portion:g.portion}))??[])];
 const guestRaw=(id:string,portion:PortionPreference)=>guestAdjustments[id]??adjustmentText(portion.adjustment);
 const guestError=people.occasion?.guests.map(g=>portionInputError(guestRaw(g.guest_id,g.portion))).find(Boolean);
 const previewError=attendees.map(a=>adjustments[personKey(a.person)]?.trim()?portionInputError(adjustments[personKey(a.person)]!):null).find(Boolean);
 const peopleChanged=portionContextStamp(people)!==portionContextStamp(cell)||people.occasion?.guests.some(g=>guestRaw(g.guest_id,g.portion)!==adjustmentText(g.portion.adjustment));
 const savePeople=()=>{if(!session.canMutate||stale||guestError)return;const edited=structuredClone(people);if(edited.occasion)edited.occasion.host_diner_ids=[...edited.attendee_ids];for(const guest of edited.occasion?.guests??[])guest.portion.adjustment=parsePortionAdjustment(guestRaw(guest.guest_id,guest.portion));return session.submit({kind:'save_week',data:{expected_week_revision:week.revision,week:{week_id:week.range.week_id,defaults_snapshot:week.defaults_snapshot,day_enabled:week.day_enabled,meal_type_enabled:week.meal_type_enabled,cells:week.cells.map(c=>slotKey(c.slot)===slotKey(cell.slot)?edited:c),plan:null}}});};
 const guestRequirements=(index:number,r:HardRequirement)=>setPeople(c=>{const n=structuredClone(c);const g=n.occasion!.guests[index]!;const current=g.restrictions.kind==='confirmed'?g.restrictions.data.requirements:[];const key=requirementKey(r);const requirements=current.some(x=>requirementKey(x)===key)?current.filter(x=>requirementKey(x)!==key):[...current,r];g.restrictions=requirements.length?{kind:'confirmed',data:{requirements,confirmed_at:now()}}:{kind:'unconfirmed',data:{}};return n;});
 const preview=()=>{if(!session.canMutate||stale||peopleChanged||previewError)return;try{const overrides=attendees.flatMap(a=>{const value=previewPortionOverride(a.person,a.portion,cell.slot.meal_type,portions[personKey(a.person)]??'',adjustments[personKey(a.person)]??'');return value?[value]:[];});onOptions({mealOverrides:{portions:overrides,eat_by:null},extraRecipients:attendees.filter(a=>extras.includes(personKey(a.person))).map(a=>a.person),makeExtra,leftoverSource:week.plan.selections.find(s=>slotKey(s.slot)===leftover)?.slot??null,eatByTime:time?`${time}:00`:null,timeResolution:resolution,...(portable&&packing&&departure&&chillingConfirmed?{portableCold:{packingCompleteTime:`${packing}:00`,packingResolution,departTime:`${departure}:00`,departureResolution,chillingConfirmed:true}}:{})});setNotice('Preview requested. A checked recipe must support these exact portions before it can be saved.');}catch(error){setNotice((error as Error).message);}};
 // Any edited value discards the old preview and its declaration. The checkbox
 // handler may then explicitly confirm the new values in this same event.
 return <details className="meal-controls" onChangeCapture={()=>{setChillingConfirmed(false);onOptions({});}}><summary>People, portions and leftovers for this meal</summary>
  <fieldset><legend>Diners at this meal</legend>{profile.diners.map(d=><label key={d.diner_id}><input type="checkbox" checked={people.attendee_ids.includes(d.diner_id)} onChange={e=>setPeople(c=>({...c,attendee_ids:e.target.checked?[...c.attendee_ids,d.diner_id].sort():c.attendee_ids.filter(id=>id!==d.diner_id)}))}/>{d.display_name??'Diner'}</label>)}
   {people.occasion?.guests.map((g,i)=><fieldset key={g.guest_id}><legend>Guest {i+1} · this meal only</legend>
    <label>Guest age<select value={g.age_band} onChange={e=>setPeople(c=>{const n=structuredClone(c);n.occasion!.guests[i]!.age_band=e.target.value as AgeBand;return n;})}>{['adult','teen','child_1_3','child_4_6','child_7_10','child_11_12'].map(v=><option key={v} value={v}>{human(v)}</option>)}</select></label>
    <label>Guest portion<select value={g.portion.preset} onChange={e=>setPeople(c=>{const n=structuredClone(c);n.occasion!.guests[i]!.portion.preset=e.target.value as PortionPresetName;return n;})}>{['smaller','regular','larger'].map(v=><option key={v}>{v}</option>)}</select></label>
    <label>Guest {i+1} usual portion multiplier<input type="text" maxLength={80} value={guestRaw(g.guest_id,g.portion)} aria-invalid={!!portionInputError(guestRaw(g.guest_id,g.portion))} onChange={e=>setGuestAdjustments(p=>({...p,[g.guest_id]:e.target.value}))}/></label>
    <p>{PORTION_INPUT_HELP}</p>
    {[...diets.map(code=>({label:code,r:{kind:'diet',data:{code}} as HardRequirement})),...allergens.map(code=>({label:`exclude ${human(code)}`,r:{kind:'allergen_exclusion',data:{code}} as HardRequirement}))].map(({label,r})=><label key={label}><input type="checkbox" checked={g.restrictions.kind==='confirmed'&&g.restrictions.data.requirements.some(x=>requirementKey(x)===requirementKey(r))} onChange={()=>guestRequirements(i,r)}/>{human(label)}</label>)}
    <button onClick={()=>setPeople(c=>{const n=structuredClone(c);n.occasion!.guests[i]!.restrictions={kind:'confirmed_none',data:{confirmed_at:now()}};return n;})} disabled={g.restrictions.kind==='confirmed'}>Confirm guest has no dietary requirements</button>
    <p>{g.restrictions.kind==='unconfirmed'?'Confirm guest requirements before saving.':human(g.restrictions.kind)}</p>
    <button onClick={()=>setPeople(c=>{const n=structuredClone(c);n.occasion!.guests.splice(i,1);return n;})}>Remove guest {i+1}</button>
   </fieldset>)}
   <button disabled={(people.occasion?.guests.length??0)>=64} onClick={()=>setPeople(c=>{const n=structuredClone(c);n.occasion??={occasion_id:`oc_${crypto.randomUUID().replaceAll('-','')}` as never,host_diner_ids:[],guests:[]};n.occasion.guests.push({guest_id:`gu_${crypto.randomUUID().replaceAll('-','')}` as never,age_band:'adult',restrictions:{kind:'unconfirmed',data:{}},portion:{preset:'regular',adjustment:{numerator:'1',denominator:'1'},meal_overrides:[]}});return n;})}>Add guest to this meal</button>
   {guestError&&<p role="alert">{guestError}</p>}
   <button disabled={!session.canMutate||stale||!!guestError||people.occasion?.guests.some(g=>g.restrictions.kind==='unconfirmed')} onClick={()=>void savePeople()}>Save meal diners and guests</button>
  </fieldset>
  <fieldset><legend>Preview portions and optional extras</legend>{attendees.map(a=>{const usual=effectivePortion(a.portion,cell.slot.meal_type);return <div key={personKey(a.person)}><label>{a.label} portion<select value={portions[personKey(a.person)]??''} onChange={e=>setPortions(p=>({...p,[personKey(a.person)]:e.target.value as PortionPresetName|''}))}><option value="">Usual portion</option>{['smaller','regular','larger'].map(v=><option key={v}>{v}</option>)}</select></label><label>{a.label} this-meal multiplier<input type="text" maxLength={80} placeholder={portions[personKey(a.person)]?'1 for chosen preset':'Use saved usual portion'} value={adjustments[personKey(a.person)]??''} aria-invalid={!!adjustments[personKey(a.person)]?.trim()&&!!portionInputError(adjustments[personKey(a.person)]!)} onChange={e=>setAdjustments(p=>({...p,[personKey(a.person)]:e.target.value}))}/></label><p>Saved usual for {cell.slot.meal_type}: {usual.preset} ×{adjustmentText(usual.adjustment)}. Both fields blank use this unchanged. An explicit multiplier replaces the saved multiplier on the selected preset.</p><label><input type="checkbox" checked={extras.includes(personKey(a.person))} onChange={e=>setExtras(p=>e.target.checked?[...p,personKey(a.person)]:p.filter(x=>x!==personKey(a.person)))}/>Optional extra for {a.label}</label></div>;})}
   <p>{PORTION_INPUT_HELP} Choose the preset and multiplier reviewed for this recipe; other adjustments may be refused. No nutrition or child portion calibration is claimed.</p>
   {peopleChanged&&<p role="status">Save diner and guest edits before previewing their saved portions.</p>}
   {previewError&&<p role="alert">{previewError}</p>}
   <label><input type="checkbox" checked={makeExtra} onChange={e=>setMakeExtra(e.target.checked)}/>Make a second set of portions for later</label>
   <label>Use a saved source batch<select value={leftover} onChange={e=>setLeftover(e.target.value)}><option value="">Cook a new batch</option>{week.plan.selections.filter(s=>s.batch_use.kind==='new_batch'&&slotKey(s.slot)!==slotKey(cell.slot)).map(s=><option key={slotKey(s.slot)} value={slotKey(s.slot)}>{s.slot.date} · {s.slot.meal_type}</option>)}</select></label>
   <p>Leftover choices appear only when the original batch has enough portions and the native storage and reheating checks pass.</p>
   <label>Eat-by override<input type="time" value={time} onChange={e=>setTime(e.target.value)}/></label><label>Clock-change resolution<select value={resolution} onChange={e=>setResolution(e.target.value as typeof resolution)}><option value="unambiguous">Ordinary local time</option><option value="earlier">Earlier repeated time</option><option value="later">Later repeated time</option></select></label>
   {portable&&<fieldset><legend>Pack this cold lunch</legend>
    <label>Finish packing at home<input type="time" value={packing} onChange={e=>setPacking(e.target.value)}/></label>
    <label>Leave home at<input type="time" value={departure} onChange={e=>setDeparture(e.target.value)}/></label>
    <p>Both times are on {cell.slot.date}, in {week.range.zone}. Packing needs five minutes after the original batch has cooled. Leave after packing and before serving lunch.</p>
    <details><summary>If the clocks change that morning</summary>
     <label>Packing clock-change resolution<select value={packingResolution} onChange={e=>setPackingResolution(e.target.value as typeof packingResolution)}><option value="unambiguous">Ordinary local time</option><option value="earlier">Earlier repeated time</option><option value="later">Later repeated time</option></select></label>
     <label>Departure clock-change resolution<select value={departureResolution} onChange={e=>setDepartureResolution(e.target.value as typeof departureResolution)}><option value="unambiguous">Ordinary local time</option><option value="earlier">Earlier repeated time</option><option value="later">Later repeated time</option></select></label>
    </details>
    <label><input type="checkbox" checked={chillingConfirmed} disabled={!packing||!departure||stale||!!peopleChanged} onChange={e=>setChillingConfirmed(e.target.checked)}/>I can keep this lunch at 0–5°C until I eat it, using an insulated bag with frozen ice packs and a fridge on arrival. I will check the temperature and eat it within the original cooking deadline.</label>
    <p>Confirm for these portions, source batch and times. If the chilling arrangements change or the food’s handling history is uncertain, choose another lunch.</p>
   </fieldset>}
   <button disabled={!session.canMutate||stale||!!peopleChanged||!!previewError} onClick={preview}>Preview meal options</button>
  </fieldset>
  {stale&&<p role="alert">The saved meal or household changed while these controls were open. Your edits are retained. Open another meal and return to compare the newer saved values before saving or previewing.</p>}
  {notice&&<p role="status">{notice}</p>}
 </details>;
}

export function SuggestControls({week,slot,horizonAvailable=false}:{week:Week;slot:SlotId;horizonAvailable?:boolean}){
 const {session,state}=usePlanner();const [repeat,setRepeat]=useState(false),[mode,setMode]=useState<PlanMode>('fill_open'),[scope,setScope]=useState<'meal'|'week'|'horizon'>('meal');
 const [busy,setBusy]=useState(false),[notice,setNotice]=useState('');
 type Summary={epoch:number;snapshot:Snapshot;commandId:string;weeks:{monday:string;changed:number;open:number}[]};
 const [summary,setSummary]=useState<Summary|null>(null),operation=useRef<object|null>(null);
 const settings=portionContextStamp([mode,scope,repeat]),household=state.snapshot?.household_draft.household_id;
 const lifecycle=useRef({session,household,weekId:week.range.week_id,slot:slotKey(slot),horizonAvailable,settings,epoch:0,alive:true});
 if(lifecycle.current.session!==session||lifecycle.current.household!==household||lifecycle.current.weekId!==week.range.week_id||lifecycle.current.slot!==slotKey(slot)||lifecycle.current.horizonAvailable!==horizonAvailable||lifecycle.current.settings!==settings)lifecycle.current={session,household,weekId:week.range.week_id,slot:slotKey(slot),horizonAvailable,settings,epoch:lifecycle.current.epoch+1,alive:lifecycle.current.alive};
 const epoch=lifecycle.current.epoch,active=(ownEpoch:number)=>lifecycle.current.alive&&lifecycle.current.session===session&&lifecycle.current.epoch===ownEpoch;
 useEffect(()=>{lifecycle.current.alive=true;return()=>{lifecycle.current.alive=false;lifecycle.current.epoch++;};},[]);
 useEffect(()=>{operation.current=null;setBusy(false);setSummary(null);setNotice('');},[epoch]);
 const weeks=[...(state.snapshot?.weeks??[])].sort((a,b)=>a.range.monday.localeCompare(b.range.monday));
 const fourAvailable=horizonAvailable&&weeks.length===4&&new Set(weeks.map(w=>w.range.week_id)).size===4;
 const four=mode!=='swap_named'&&scope==='horizon';
 const endDate=(monday:string)=>new Date(Date.parse(`${monday}T12:00:00Z`)+6*86400000).toISOString().slice(0,10);
 const dateLabel=(date:string)=>new Intl.DateTimeFormat('en-GB',{day:'numeric',month:'long',year:'numeric',timeZone:'UTC'}).format(new Date(`${date}T12:00:00Z`));
 const currentWeek=state.snapshot?.weeks.find(item=>item.range.week_id===week.range.week_id);
 const contextReady=!!currentWeek&&portionContextStamp(currentWeek)===portionContextStamp(week)&&currentWeek.cells.some(cell=>slotKey(cell.slot)===slotKey(slot));
 const save=async()=>{const before=session.getState();if(!active(epoch)||operation.current||!session.canMutate||!contextReady||before!==state||!before.snapshot||four&&!fourAvailable)return;
  const id={};operation.current=id;setBusy(true);setSummary(null);setNotice('');
  const command:AppCommand=four?{kind:'suggest_planning_horizon',data:{ordered_week_ids:weeks.map(item=>item.range.week_id),mode:mode==='replace_suggestions'?'replace_suggestions':'fill_open',allow_repeats:repeat}}:{kind:'suggest_meals',data:{week_id:week.range.week_id,mode,slots:mode==='swap_named'||scope==='meal'?[slot]:[],allow_repeats:repeat}};
  try{const submitting=session.submit(command),pending=session.getState().pending,commandId=pending?.command_id;await submitting;
   if(!active(epoch)||operation.current!==id||!four||!commandId)return;
   const after=session.getState(),receipt=after.lastReceipt,snapshot=after.snapshot;
   if(after.disposition!=='committed'||after.read!=='ready'||after.pending!==null||!receipt||receipt.command_id!==commandId||String(receipt.command_kind)!=='suggest_planning_horizon'||receipt.expected_snapshot_revision!==before.snapshot.snapshot_revision||!snapshot||snapshot.snapshot_revision!==receipt.receipt_snapshot_revision||snapshot.household_draft.household_id!==household)return;
   const afterWeeks=[...snapshot.weeks].sort((a,b)=>a.range.monday.localeCompare(b.range.monday));
   if(afterWeeks.length!==4||afterWeeks.some((item,index)=>item.range.week_id!==weeks[index].range.week_id||item.range.monday!==weeks[index].range.monday))return;
   const rows=afterWeeks.map((saved,index)=>{const previous=weeks[index],oldSelections=new Map(previous.plan.selections.map(item=>[slotKey(item.slot),item])),newSelections=new Map(saved.plan.selections.map(item=>[slotKey(item.slot),item]));
    const changed=[...new Set([...oldSelections.keys(),...newSelections.keys()])].filter(key=>portionContextStamp(oldSelections.get(key))!==portionContextStamp(newSelections.get(key))).length;
    const open=saved.cells.filter(cell=>{const day=(Date.parse(cell.slot.date)-Date.parse(saved.range.monday))/86400000;return Number.isInteger(day)&&day>=0&&day<7&&saved.day_enabled[day]&&saved.meal_type_enabled[cell.slot.meal_type]&&cell.choice!=='skip'&&!newSelections.has(slotKey(cell.slot));}).length;
    return {monday:saved.range.monday,changed,open};});
   setSummary({epoch,snapshot,commandId,weeks:rows});
  }catch{if(active(epoch)&&operation.current===id)setNotice('Suggestions could not be confirmed. Review the saved kitchen before trying again.');}
  finally{if(operation.current===id){operation.current=null;if(active(epoch))setBusy(false);}}
 };
 const shown=summary&&active(summary.epoch)&&state.snapshot===summary.snapshot&&state.disposition==='committed'&&state.read==='ready'&&state.pending===null&&state.lastReceipt?.command_id===summary.commandId?summary:null;
 return <details className="suggest-controls"><summary>Suggest meals locally</summary><p>Only checked choices in the current catalogue and recipe scope. Missing price and protein evidence stays unknown. Fixed meals are preserved unless you explicitly choose to swap this meal.</p>
  <label>Suggestion action<select value={mode} disabled={busy} onChange={e=>setMode(e.target.value as PlanMode)}><option value="fill_open">Fill open meals</option><option value="replace_suggestions">Replace previous suggestions</option><option value="swap_named">Swap this named meal</option></select></label>
  <label>Scope<select value={mode==='swap_named'?'meal':scope} onChange={e=>setScope(e.target.value as typeof scope)} disabled={busy||mode==='swap_named'}><option value="meal">This meal</option><option value="week">This week</option>{fourAvailable&&<option value="horizon">All four weeks</option>}</select></label>
  <label><input type="checkbox" checked={repeat} disabled={busy} onChange={e=>setRepeat(e.target.checked)}/>Allow repeats for this request</label>
  {four&&fourAvailable&&<p>All four weeks: {dateLabel(weeks[0].range.monday)} to {dateLabel(endDate(weeks[3].range.monday))}. Save local suggestions while keeping your fixed meals and availability choices. Baskets, orders and calendars are unchanged.</p>}
  {four&&!fourAvailable&&<p role="status">Open four current planning weeks before suggesting across the whole period.</p>}
  <button disabled={!session.canMutate||busy||!contextReady||four&&!fourAvailable} onClick={()=>void save()}>Save checked suggestions</button>
  {shown&&<section aria-label="Saved four-week suggestions"><h4>Suggestions saved locally across four weeks</h4><ul>{shown.weeks.map(item=><li key={item.monday}>{dateLabel(item.monday)} to {dateLabel(endDate(item.monday))}: {item.changed} meal{item.changed===1?'':'s'} changed; {item.open} active open meal{item.open===1?'':'s'} remaining.</li>)}</ul><p>Use the week tabs to review each week. Skipped and disabled meals are excluded from the remaining count.</p></section>}
  {notice&&active(epoch)&&<p role="status">{notice}</p>}
 </details>;
}

export function PrepAndEssentials({details}:{details:PlannerDetails}){
 const {session}=usePlanner();const d=details.derived;
 if(!d)return <section className="planner-panel"><h2>Prep and essentials</h2><p>{details.food_error?'Resolve the saved meal conflicts before preparation and quantities can be derived.':'Choose a meal to see its preparation and quantities.'}</p></section>;
 const confirm=(line:typeof d.demand.lines[number],choice:EssentialsChoice)=>session.submit({kind:'confirm_essentials',data:{confirmations:[{line_id:line.line_id,choice,displayed_quantity:line.quantity,demand_digest:d.demand.digest,confirmed_at:now()}]}});
 const prepSessions=[...d.prep_sessions].sort((a,b)=>Date.parse(a.start.instant)-Date.parse(b.start.instant)||Date.parse(a.end.instant)-Date.parse(b.end.instant)||(a.session_id<b.session_id?-1:a.session_id>b.session_id?1:0));
 return <section className="planner-panel"><h2>Prep and essentials</h2><p>Exact saved quantities for this week. Delivery and product availability remain unqualified.</p>
  <h3>Preparation</h3>{prepSessions.map(p=><article key={p.session_id}><h4>{p.start.local_date} · {p.start.local_time}–{p.end.local_time}</h4><p>{p.start.zone} · {p.linked_slots.map(s=>`${s.date} ${s.meal_type}`).join(', ')}</p>{p.tasks.map(t=>{const entry=details.library.find(e=>e.reference.recipe_id===t.recipe.recipe_id&&e.reference.revision===t.recipe.revision&&e.reference.digest===t.recipe.digest);const phase=entry?.recipe?.prep.find(v=>v.prep_id===t.prep_id);return <div key={t.task_id}><p>{titleFor(details,t.recipe)} · preparation {t.prep_id}</p>{isReviewedEntry(entry)&&phase&&<p>{phase.title}: {phase.active_minutes} minutes hands-on / {phase.elapsed_minutes} minutes elapsed. Completion and temperature are not established by the schedule.</p>}</div>;})}</article>)}
  <h3>Original batches</h3>{d.batches.map(b=><article key={b.batch_id}><h4>{titleFor(details,b.recipe)} · {b.source_slot.date} {b.source_slot.meal_type}</h4>{b.components.map(c=>{const at=c.original_production.kind==='cooked'?c.original_production.data.completed_at:c.original_production.data.preparation_started_at;return <p key={c.component_id}>Produced {rational(c.produced.amount)} {c.produced.unit} {human(c.produced.form)}; allocated {rational(c.allocated.amount)}; remaining {rational(c.unallocated.amount)}. {c.original_production.kind==='cooked'?'Planned cooking completion':'Planned first handling'} {at.local_date} {at.local_time}.</p>;})}</article>)}
  <h3>Ingredients and essentials</h3>{d.demand.lines.map(line=>{const ingredient=details.library.flatMap(v=>v.recipe?.ingredients??[]).find(i=>i.ingredient_id===line.ingredient_id);const saved=details.essentials.find(c=>c.line_id===line.line_id);const current=saved?.demand_digest===d.demand.digest&&saved.displayed_quantity.amount.numerator===line.quantity.amount.numerator&&saved.displayed_quantity.amount.denominator===line.quantity.amount.denominator&&saved.displayed_quantity.unit===line.quantity.unit&&saved.displayed_quantity.form===line.quantity.form;return <article key={line.line_id}><h4>{ingredient?.name??'Ingredient'} · {rational(line.quantity.amount)} {line.quantity.unit} {human(line.quantity.form)}</h4>{line.essential&&<><p>{current?human(saved!.choice):saved?'Saved pantry confirmation needs review; confirm this exact quantity again.':'Confirm against this exact quantity.'}</p><button disabled={!session.canMutate} onClick={()=>void confirm(line,'have_enough')}>Have enough</button><button disabled={!session.canMutate} onClick={()=>void confirm(line,'add_to_shop')}>Add to shop</button></>}</article>;})}
  {d.warnings.length>0&&<p role="status">{d.warnings.map(v=>human(v.code)).join('; ')}</p>}
  {details.shopping_list&&<section aria-label="Manual shopping list"><h3>Manual shopping list</h3>
   <p>{details.shopping_list.open_slots.length?`${details.shopping_list.open_slots.length} active meals remain open. This list covers saved active meals only.`:'All active meals have saved selections.'}</p>
   <p>Retailer stock, product suitability, packs, prices and delivery remain unqualified. This list does not confirm a basket or order.</p>
   <p>Ingredient forms describe recipe inputs, not retailer product specifications. Raw milk input form does not mean unpasteurised milk.</p>
   {details.shopping_list.contains_engineering&&<p role="status">Includes synthetic engineering ingredients. This is not a qualified production shop.</p>}
   <button disabled={!session.canMutate} onClick={()=>exportShoppingList(details.shopping_list!,'text')}>Export shopping list as text</button><button disabled={!session.canMutate} onClick={()=>exportShoppingList(details.shopping_list!,'json')}>Export shopping list as JSON</button>
   {details.shopping_list.lines.map(line=><article key={line.demand.line_id}><h4>{line.name} · {quantityText(line)}</h4><p>{line.action==='have_enough'?'Have enough: explicitly confirmed for this exact amount.':'Add to shop.'}</p>
    {line.confirmation_status==='stale'&&<p>Previous pantry confirmation is stale; the full amount is included again.</p>}
    {line.confirmation_status==='not_offered'&&<p>Pantry deduction is not supported for this ingredient; the full amount is included.</p>}
    <ul>{line.origins.map((origin,index)=><li key={index}>{originText(origin)}</li>)}</ul>
   </article>)}
  </section>}
 </section>;
}

export function SavedHistory({details,week}:{details:PlannerDetails;week:Week}){
 const {session}=usePlanner();const [rating,setRating]=useState<FeedbackRating>('okay');const [reason,setReason]=useState<FeedbackReason|''>('');const [hide,setHide]=useState(false);
 return <section className="planner-panel"><h2>Saved meals and feedback</h2><p>Selection history is saved locally. Feedback and hiding are optional.</p>{details.history.selected.map(v=><p key={slotKey(v.slot)}>{v.slot.date} · {v.slot.meal_type} · {titleFor(details,v.recipe)} · {v.servings.map(s=>rational(s.servings)).join(', ')} servings</p>)}
  <fieldset><legend>Optional feedback</legend><label>Rating<select value={rating} onChange={e=>setRating(e.target.value as FeedbackRating)}>{['liked','okay','not_for_me'].map(v=><option key={v} value={v}>{human(v)}</option>)}</select></label><label>Reason<select value={reason} onChange={e=>setReason(e.target.value as FeedbackReason| '')}><option value="">No reason</option>{['too_spicy','too_much','too_little','too_much_effort'].map(v=><option key={v} value={v}>{human(v)}</option>)}</select></label><label><input type="checkbox" checked={hide} onChange={e=>setHide(e.target.checked)}/>Hide recipe from suggestions</label>{week.plan.selections.map(v=><button key={slotKey(v.slot)} disabled={!session.canMutate} onClick={()=>void session.submit({kind:'record_feedback',data:{feedback:{slot:v.slot,recipe:v.recipe,rating,reasons:reason?[reason]:[],hide,portion_change:null,portion_change_accepted:false,recorded_at:now()}}})}>Save feedback for {v.slot.date} {v.slot.meal_type}</button>)}</fieldset>
  <h3>Hidden recipes</h3>{details.history.hidden_recipe_ids.map(id=><p key={id}>{details.library.find(v=>v.reference.recipe_id===id)?.recipe?.title??'Saved recipe'} <button disabled={!session.canMutate} onClick={()=>void session.submit({kind:'restore_hidden_recipe',data:{recipe_id:id}})}>Restore to suggestions</button></p>)}<p>{details.history.feedback.length} feedback records saved.</p>
 </section>;
}
