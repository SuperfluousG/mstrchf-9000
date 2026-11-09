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
  artwork?: { src: string; alt: string; caption: string };
}

export function MealDetails({ meal }: { meal: SelectedMealView | null }) {
  if (!meal) return <><h3>Choose a meal</h3><p>Choose a day to explore meals for your week.</p></>;
  return <div className="meal-details" data-slot-id={meal.slotId}>
    <header className="meal-details-header">
      <p className="meal-context">{meal.context}</p>
      <h3>{meal.heading}</h3>
      <StatusLabel state={meal.state} label={meal.statusLabel} />
    </header>
    {meal.artwork&&<figure className="selected-meal-artwork"><img src={meal.artwork.src} alt={meal.artwork.alt}/><figcaption>{meal.artwork.caption}</figcaption></figure>}
    <dl className="meal-facts">{meal.facts.map((fact, index) => <div key={`${fact.label}-${index}`}>
      <dt>{fact.label}</dt><dd>{fact.value}</dd>
    </div>)}</dl>
    {meal.notices && meal.notices.length > 0 && <details className="meal-use-limits"><summary>Recipe notes and use limits ({meal.notices.length})</summary><ul className="meal-notices" aria-label="Meal notices">
      {meal.notices.map((notice, index) => <li key={index}>{notice}</li>)}
    </ul></details>}
  </div>;
}
