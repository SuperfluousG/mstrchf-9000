import type { BasketFile } from './basket';
import type { Focus, Rating } from './planner';
import type { Profile } from './profile';
import type { MealType } from './types';

export interface Pick {
  uid: string;
  recipeId: string;
  meal: MealType;
  day: number;
  locked?: boolean;
  reasons: string[];
}

export interface WeekPlan {
  id: string;
  craving: string;
  focus: Focus;
  picks: Pick[];
  status: 'draft' | 'ordered';
  /** The planning estimate recorded when the user marked this week ordered. Not a retailer receipt. */
  orderedBasket?: BasketFile;
  /** Perishables the user already has this week. */
  have: string[];
  /** Pantry staples explicitly requested for this shopping list. */
  pantryToBuy?: string[];
  seed: number;
}

export interface AppState {
  version: 2;
  profile: Profile | null;
  weeks: Record<string, WeekPlan>;
  favourites: string[];
  hidden: string[];
  ratings: Record<string, Rating>;
  /** Long-life ingredients the user keeps in stock (spices, rice, oils). */
  cupboard: string[];
  /** When this state was last changed (ms since epoch); used to pick the newer of browser and file copies. */
  savedAt?: number;
}

export const EMPTY_STATE: AppState = { version: 2, profile: null, weeks: {}, favourites: [], hidden: [], ratings: {}, cupboard: [] };
