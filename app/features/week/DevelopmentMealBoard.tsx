import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Week, MealSelection } from '../../generated/contracts';
import { WeekBoard, type SevenDays, type DaySlipView } from './WeekBoard';
import { usePlanner } from '../../runtime/PlannerProvider';
export type DevelopmentBoard = { week: Week; choices: { title: string; selection: MealSelection }[] };
const readBoard = () => invoke<DevelopmentBoard>('planner_development_board');

const weekdays = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];
function dateLabel(iso: string): string {
  return new Intl.DateTimeFormat('en-GB', { day: 'numeric', month: 'short', timeZone: 'UTC' }).format(new Date(`${iso}T12:00:00Z`));
}

export function DevelopmentMealBoard({ loadBoard = readBoard }: { loadBoard?: () => Promise<DevelopmentBoard> }) {
  const { session, state } = usePlanner();
  const [board, setBoard] = useState<DevelopmentBoard | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [boardRead, setBoardRead] = useState(0);
  const refresh = () => { setBoardRead(n => n + 1); void session.refresh(); };
  useEffect(() => {
    let active = true;
    void loadBoard().then(value => {
      if (active) { setBoard(value); setSelected(previous => previous ?? value.week.range.monday); setError(null); }
    }).catch(() => { if (active) setError('Meal choices could not be read. Refresh the saved kitchen to try again.'); });
    return () => { active = false; };
  }, [loadBoard, state.snapshot?.snapshot_revision, boardRead]);
  // Saved meal identity always comes from the accepted Snapshot. The board read supplies choices only.
  const week = state.snapshot?.weeks[0];
  if (!week || !board) return <main className="development-meal-board">{state.read === 'failed' || error
    ? <><p role="alert">{state.readError ?? error}</p><button onClick={refresh}>Refresh saved kitchen</button></>
    : <p role="status">Loading your kitchen board…</p>}</main>;
  const title = (meal: MealSelection | undefined) => meal && board.choices.find(c => c.selection.recipe.recipe_id === meal.recipe.recipe_id
    && c.selection.recipe.revision === meal.recipe.revision && c.selection.recipe.digest === meal.recipe.digest)?.title;
  const dinners = week.cells.filter(cell => cell.slot.meal_type === 'dinner');
  const days = dinners.map((cell, index): DaySlipView => ({
    slotId: cell.slot.date, weekday: weekdays[index]!, dateLabel: dateLabel(cell.slot.date), dateIso: cell.slot.date,
    mealLabel: 'Dinner', title: title(week.plan.selections.find(s => s.slot.date === cell.slot.date && s.slot.meal_type === 'dinner')) ?? 'Choose a meal',
    summary: 'One synthetic vegetarian diner', state: week.plan.selections.some(s => s.slot.date === cell.slot.date && s.slot.meal_type === 'dinner') ? 'booked' : 'empty',
    statusLabel: week.plan.selections.some(s => s.slot.date === cell.slot.date && s.slot.meal_type === 'dinner') ? 'Saved locally' : undefined,
  })) as unknown as SevenDays;
  const day = days.find(day => day.slotId === selected) ?? days[0];
  const meal = week.plan.selections.find(s => s.slot.date === day.dateIso && s.slot.meal_type === 'dinner');
  const savedTitle = title(meal);
  const mondayMeal = week.plan.selections.find(s => s.slot.date === week.range.monday && s.slot.meal_type === 'dinner');
  const choose = async (choice: DevelopmentBoard['choices'][number]) => {
    if (!session.canMutate) return;
    await session.submit({ kind: 'save_week', data: { expected_week_revision: week.revision,
      week: { week_id: week.range.week_id, defaults_snapshot: week.defaults_snapshot, day_enabled: week.day_enabled,
        meal_type_enabled: week.meal_type_enabled, cells: week.cells,
        plan: { ...week.plan, selections: [choice.selection] } } } });
  };
  const uncertain = state.disposition === 'unknown';
  const refused = state.disposition === 'refused';
  return <main className="development-meal-board">
    <p className="development-notice">Development kitchen · synthetic diner and recipes</p>
    {error && <p role="alert">{error}</p>}
    {(uncertain || refused || state.read === 'failed') && <p role="alert">{uncertain
      ? 'Meal saving could not be confirmed. Recover this save or restart to read the saved kitchen.'
      : refused ? 'The meal was not saved. Review the current saved kitchen before choosing again.' : state.readError}</p>}
    {state.disposition === 'pending' && <p role="status">Saving meal…</p>}
    {state.disposition === 'committed' && state.read === 'ready' && <p role="status">Meal saved locally.</p>}
    {uncertain && <button onClick={() => void session.recover()}>Recover meal save</button>}
    {refused && <button disabled={state.read !== 'ready'} onClick={() => session.reviewRefusal()}>Review saved kitchen</button>}
    {(state.read === 'failed' || error) && <button onClick={refresh}>Refresh saved kitchen</button>}
    <WeekBoard heading={`Your kitchen · ${dateLabel(days[0].dateIso)}–${dateLabel(days[6].dateIso)} ${days[6].dateIso.slice(0,4)}`}
      days={days} selectedSlotId={day.slotId} onSelectSlot={setSelected}
      selectedMeal={{ slotId: day.slotId, heading: savedTitle ?? (meal ? 'Saved development recipe' : 'An open dinner'), context: `${day.weekday} · Dinner`, state: meal ? 'booked' : 'empty', statusLabel: meal ? 'Saved locally' : undefined,
        facts: [{ label: 'Diners', value: '1 · vegetarian · synthetic development profile' }],
        notices: ['Development recipes only. Shopping, AI and calendar are unavailable.'] }}
      mealActions={day.dateIso === week.range.monday ? <div className="meal-choices" role="group" aria-label="Eligible Monday dinner choices">
        {board.choices.map(choice => <button key={choice.selection.recipe.recipe_id} disabled={!session.canMutate || !!error}
          onClick={() => void choose(choice)}>Choose {choice.title}</button>)}
      </div> : null}
      nextAction={{ title: mondayMeal ? 'Monday dinner is saved' : 'Choose Monday dinner',
        description: mondayMeal ? 'Your choice is held in this kitchen. Quit and reopen to check it.' : 'Choose an eligible development recipe from the meal panel.',
        action: { label: 'View Monday dinner', onAction: () => setSelected(week.range.monday) } }} />
  </main>;
}
