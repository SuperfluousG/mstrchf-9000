import { MealDetails, type SelectedMealView } from './MealDetails';
import type { ReactNode } from 'react';

export function SelectedMealRegion({ id, meal, children }: { id: string; meal: SelectedMealView | null; children?: ReactNode }) {
  return <section id={id} className="selected-meal-region" aria-label="Selected meal details">
    <MealDetails meal={meal} />
    {children}
  </section>;
}
