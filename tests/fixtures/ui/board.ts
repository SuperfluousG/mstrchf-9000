import type { DaySlipView, SevenDays } from '../../../app/features/week/WeekBoard';
import type { SelectedMealView } from '../../../app/features/meal/MealDetails';

// Presentation doubles only: these dates, names and details are not domain output.
const names = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];
export const boardFixture = names.map((weekday, index): DaySlipView => ({
  slotId: `synthetic-slot-${index}`,
  weekday,
  dateLabel: `${index + 1} June`,
  dateIso: `2026-06-0${index + 1}`,
  mealLabel: 'Dinner',
  title: `Synthetic meal ${index + 1}`,
  summary: 'Presentation example — no food or portion authority',
  state: index === 6 ? 'empty' : 'in-progress',
})) as unknown as SevenDays;

export function mealFixture(index: number): SelectedMealView {
  return {
    slotId: boardFixture[index]!.slotId,
    heading: boardFixture[index]!.title,
    context: `${boardFixture[index]!.weekday} · Dinner`,
    state: boardFixture[index]!.state,
    facts: [{ label: 'Source', value: 'Isolated presentation test fixture' }],
    notices: ['No recipe, serving or suitability claim is made.'],
  };
}
