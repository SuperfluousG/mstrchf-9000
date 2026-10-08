import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Week, MealSelection,MealType,SlotId } from '../../generated/contracts';
import { WeekBoard, type SevenDays, type DaySlipView } from './WeekBoard';
import { usePlanner } from '../../runtime/PlannerProvider';
import {Settings} from '../settings/Settings';
import {AvailabilityGrid} from './AvailabilityGrid';
export type DevelopmentBoard = { week: Week; choices: { title: string; selection: MealSelection }[] };
const readBoard = (weekId?:string,slot?:SlotId) => invoke<DevelopmentBoard>('planner_development_board',{weekId:weekId??null,slot:slot??null});

const weekdays = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];
function dateLabel(iso: string): string {
  return new Intl.DateTimeFormat('en-GB', { day: 'numeric', month: 'short', timeZone: 'UTC' }).format(new Date(`${iso}T12:00:00Z`));
}

export function DevelopmentMealBoard({ loadBoard = readBoard }: { loadBoard?: (weekId?:string,slot?:SlotId) => Promise<DevelopmentBoard> }) {
  const { session, state } = usePlanner();
  const [board, setBoard] = useState<DevelopmentBoard | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [boardRead, setBoardRead] = useState(0);
  const [weekId,setWeekId]=useState<string|null>(null);
  const [mealType,setMealType]=useState<MealType>('dinner');
  const [panel,setPanel]=useState<'board'|'availability'|'settings'|'setup'>('board');
  const currentWeek=state.snapshot?.weeks.find(w=>w.range.week_id===weekId)??state.snapshot?.weeks[0];
  const refresh = () => { setBoardRead(n => n + 1); void session.refresh(); };
  useEffect(() => {
    let active = true;
    const target=currentWeek?.cells.find(c=>c.slot.date===(selected??currentWeek?.range.monday)&&c.slot.meal_type===mealType)?.slot;
    void loadBoard(currentWeek?.range.week_id,target).then(value => {
      if (active) { setBoard(value); setSelected(previous => previous ?? value.week.range.monday); setError(null); }
    }).catch(() => { if (active) setError('Meal choices could not be read. Refresh the saved kitchen to try again.'); });
    return () => { active = false; };
  }, [loadBoard, state.snapshot?.snapshot_revision, boardRead,weekId,selected,mealType]);
  // Saved meal identity always comes from the accepted Snapshot. The board read supplies choices only.
  const week = currentWeek;
  if (!week || !board) return <main className="development-meal-board">{state.read === 'failed' || error
    ? <><p role="alert">{state.readError ?? error}</p><button onClick={refresh}>Refresh saved kitchen</button></>
    : <p role="status">Loading your kitchen board…</p>}</main>;
  const title = (meal: MealSelection | undefined) => meal && board.choices.find(c => c.selection.recipe.recipe_id === meal.recipe.recipe_id
    && c.selection.recipe.revision === meal.recipe.revision && c.selection.recipe.digest === meal.recipe.digest)?.title;
  const dinners = week.cells.filter(cell => cell.slot.meal_type === mealType);
  const days = dinners.map((cell, index): DaySlipView => ({
    slotId: cell.slot.date, weekday: weekdays[index]!, dateLabel: dateLabel(cell.slot.date), dateIso: cell.slot.date,
    mealLabel: mealType[0]!.toUpperCase()+mealType.slice(1), title: title(week.plan.selections.find(s => s.slot.date === cell.slot.date && s.slot.meal_type === mealType)) ?? (week.plan.selections.some(s=>s.slot.date===cell.slot.date&&s.slot.meal_type===mealType)?'Saved recipe':'Choose a meal'),
    summary: `${cell.attendee_ids.length} household diner${cell.attendee_ids.length===1?'':'s'} · ${cell.choice.replaceAll('_',' ')}`, state: !week.day_enabled[index]||!week.meal_type_enabled[mealType]||cell.choice==='skip'?'disabled':week.plan.selections.some(s => s.slot.date === cell.slot.date && s.slot.meal_type === mealType) ? 'booked' : 'empty',
    statusLabel: week.plan.selections.some(s => s.slot.date === cell.slot.date && s.slot.meal_type === mealType) ? 'Saved locally' : undefined,
  })) as unknown as SevenDays;
  const day = days.find(day => day.slotId === selected) ?? days[0];
  const meal = week.plan.selections.find(s => s.slot.date === day.dateIso && s.slot.meal_type === mealType);
  const savedTitle = title(meal);
  const mondayMeal = week.plan.selections.find(s => s.slot.date === week.range.monday && s.slot.meal_type === 'dinner');
  const choose = async (choice: DevelopmentBoard['choices'][number]) => {
    if (!session.canMutate) return;
    await session.submit({ kind: 'save_week', data: { expected_week_revision: week.revision,
      week: { week_id: week.range.week_id, defaults_snapshot: week.defaults_snapshot, day_enabled: week.day_enabled,
        meal_type_enabled: week.meal_type_enabled, cells: week.cells,
        plan: { ...week.plan, selections: [...week.plan.selections.filter(s=>!(s.slot.date===choice.selection.slot.date&&s.slot.meal_type===choice.selection.slot.meal_type)),choice.selection] } } } });
  };
  const uncertain = state.disposition === 'unknown';
  const refused = state.disposition === 'refused';
  return <main className="development-meal-board">
    <p className="development-notice">Development kitchen · synthetic diner and recipes</p>
    <nav className="planner-toolbar" aria-label="Kitchen controls"><button disabled={!session.canMutate} onClick={()=>void session.submit({kind:'open_planning_horizon',data:{}})}>Open four upcoming weeks</button><button onClick={()=>setPanel('availability')}>Availability</button><button onClick={()=>setPanel('settings')}>Settings</button><button onClick={()=>setPanel('setup')}>Household setup</button></nav>
    <nav className="week-tabs" role="tablist" aria-label="Saved weeks">{state.snapshot?.weeks.map(w=><button role="tab" key={w.range.week_id} aria-selected={week.range.week_id===w.range.week_id} onClick={()=>{setWeekId(w.range.week_id);setSelected(w.range.monday);setPanel('board');}}>{dateLabel(w.range.monday)} · {w.confirmed_order?'Booked':w.plan.selections.length?'In progress':'Not started'}</button>)}</nav>
    <nav className="meal-view-switch" aria-label="Meal view">{(['breakfast','lunch','dinner'] as const).map(m=><button key={m} aria-pressed={mealType===m} onClick={()=>setMealType(m)}>{m[0].toUpperCase()+m.slice(1)}</button>)}</nav>
    {!!state.snapshot?.conflicts.length&&<p role="alert">Some saved meals need review against the current household or preparation requirements. {state.snapshot.conflicts.map(c=>c.code.replaceAll('_',' ')).join('; ')}</p>}
    {error && <p role="alert">{error}</p>}
    {(uncertain || refused || state.read === 'failed') && <p role="alert">{uncertain
      ? 'Meal saving could not be confirmed. Recover this save or restart to read the saved kitchen.'
      : refused ? 'The meal was not saved. Review the current saved kitchen before choosing again.' : state.readError}</p>}
    {state.disposition === 'pending' && <p role="status">Saving meal…</p>}
    {state.disposition === 'committed' && state.read === 'ready' && <p role="status">Meal saved locally.</p>}
    {uncertain && <button onClick={() => void session.recover()}>Recover meal save</button>}
    {refused && <button disabled={state.read !== 'ready'} onClick={() => session.reviewRefusal()}>Review saved kitchen</button>}
    {(state.read === 'failed' || error) && <button onClick={refresh}>Refresh saved kitchen</button>}
    {panel==='availability'&&<><button onClick={()=>setPanel('board')}>Back to kitchen</button><AvailabilityGrid week={week}/></>}
    {(panel==='settings'||panel==='setup')&&state.snapshot?.profile&&<Settings key={`${panel}-${state.snapshot.profile.household_id}`} onboarding={panel==='setup'} onClose={()=>setPanel('board')}/>}
    {panel==='board'&&<WeekBoard heading={`Your kitchen · ${dateLabel(days[0].dateIso)}–${dateLabel(days[6].dateIso)} ${days[6].dateIso.slice(0,4)}`}
      days={days} selectedSlotId={day.slotId} onSelectSlot={setSelected}
      selectedMeal={{ slotId: day.slotId, heading: savedTitle ?? (meal ? 'Saved development recipe' : `An open ${mealType}`), context: `${day.weekday} · ${mealType}`, state: meal ? 'booked' : 'empty', statusLabel: meal ? 'Saved locally' : undefined,
        facts: [{ label: 'Diners', value: String(state.snapshot?.profile?.diners.length??1) }],
        notices: ['Development recipes only. Shopping, AI and calendar are unavailable.'] }}
      mealActions={<div className="meal-choices" role="group" aria-label={`Eligible ${day.weekday} ${mealType} choices`}>
        {board.choices.map(choice => <button key={choice.selection.recipe.recipe_id} disabled={!session.canMutate || !!error}
          onClick={() => void choose(choice)}>Choose {choice.title}</button>)}
        {board.choices.length===0&&<p>No checked recipe fits this meal. Review its availability and household requirements.</p>}
      </div>}
      nextAction={{ title: mondayMeal ? 'Monday dinner is saved' : 'Choose Monday dinner',
        description: mondayMeal ? 'Your choice is held in this kitchen. Quit and reopen to check it.' : 'Choose an eligible development recipe from the meal panel.',
        action: { label: 'View Monday dinner', onAction: () => {setMealType('dinner');setSelected(week.range.monday);} } }} />}
  </main>;
}
