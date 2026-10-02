import { MealDetails, type SelectedMealView } from './MealDetails';

export function SelectedMealRegion({ id, meal }: { id: string; meal: SelectedMealView | null }) {
  return <section id={id} className="selected-meal-region" aria-label="Selected meal details">
    <MealDetails meal={meal} />
  </section>;
}
