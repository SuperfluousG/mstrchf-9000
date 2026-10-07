import { useEffect, useState } from 'react';
import { invoke } from '@tauri-apps/api/core';
import type { Week } from '../../generated/contracts';
import { WeekBoard, type SevenDays, type DaySlipView } from './WeekBoard';

const weekdays = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];
function dateLabel(iso: string): string {
  return new Intl.DateTimeFormat('en-GB', { day: 'numeric', month: 'short', timeZone: 'UTC' }).format(new Date(`${iso}T12:00:00Z`));
}

export function DevelopmentMealBoard() {
  const [week, setWeek] = useState<Week | null>(null);
  const [selected, setSelected] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);
  useEffect(() => {
    let active = true;
    void invoke<Week>('planner_development_board').then(value => {
      if (active) { setWeek(value); setSelected(value.range.monday); }
    }).catch(() => { if (active) setError('The saved kitchen board could not be read. Restart the development app to try again.'); });
    return () => { active = false; };
  }, []);
  if (error) return <main className="development-meal-board"><p role="alert">{error}</p></main>;
  if (!week) return <main className="development-meal-board"><p role="status">Loading your kitchen board…</p></main>;
  const dinners = week.cells.filter(cell => cell.slot.meal_type === 'dinner');
  const days = dinners.map((cell, index): DaySlipView => ({
    slotId: cell.slot.date, weekday: weekdays[index]!, dateLabel: dateLabel(cell.slot.date), dateIso: cell.slot.date,
    mealLabel: 'Dinner', title: 'Choose a meal', summary: 'One synthetic vegetarian diner', state: 'empty',
  })) as unknown as SevenDays;
  const day = days.find(day => day.slotId === selected) ?? days[0];
  return <main className="development-meal-board">
    <p className="development-notice">Development kitchen · synthetic diner and recipes</p>
    <WeekBoard heading={`Your kitchen · ${dateLabel(days[0].dateIso)}–${dateLabel(days[6].dateIso)} ${days[6].dateIso.slice(0,4)}`}
      days={days} selectedSlotId={day.slotId} onSelectSlot={setSelected}
      selectedMeal={{ slotId: day.slotId, heading: 'An open dinner', context: `${day.weekday} · Dinner`, state: 'empty',
        facts: [{ label: 'Diners', value: '1 · vegetarian · synthetic development profile' }],
        notices: ['Development recipes only. Shopping, AI and calendar are unavailable.'] }}
      nextAction={{ title: 'Choose Monday dinner', description: 'Your seven-day kitchen board is ready.',
        blockingReason: 'Meal saving is being connected to local storage.' }} />
  </main>;
}
