import { StatusLabel, type StatusState } from '../../components/StatusLabel';

export interface SelectedMealView {
  slotId: string;
  heading: string;
  context: string;
  state: StatusState;
  statusLabel?: string;
  /** Already formatted by the future admitted adapter; no quantities are calculated here. */
  facts: readonly { label: string; value: string }[];
  notices?: readonly string[];
}

export function MealDetails({ meal }: { meal: SelectedMealView | null }) {
  if (!meal) return <><h3>Choose a meal</h3><p>Select a day slip to see its details here.</p></>;
  return <div className="meal-details" data-slot-id={meal.slotId}>
    <p className="meal-context">{meal.context}</p>
    <h3>{meal.heading}</h3>
    <StatusLabel state={meal.state} label={meal.statusLabel} />
    <dl className="meal-facts">{meal.facts.map((fact, index) => <div key={`${fact.label}-${index}`}>
      <dt>{fact.label}</dt><dd>{fact.value}</dd>
    </div>)}</dl>
    {meal.notices && meal.notices.length > 0 && <ul className="meal-notices" aria-label="Meal notices">
      {meal.notices.map((notice, index) => <li key={index}>{notice}</li>)}
    </ul>}
  </div>;
}
