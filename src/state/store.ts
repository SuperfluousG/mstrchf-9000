import { useSyncExternalStore } from 'react';
import { EMPTY_FOCUS, type Focus, type Rating } from '../domain/planner';
import type { Profile } from '../domain/profile';
import type { MealType } from '../domain/types';
import { weeksBetween } from '../domain/weeks';

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
  /** Perishables the user already has this week. */
  have: string[];
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
}

const KEY = 'mstrchf9000:v2';

export const EMPTY_STATE: AppState = { version: 2, profile: null, weeks: {}, favourites: [], hidden: [], ratings: {}, cupboard: [] };

function load(): AppState {
  try {
    const raw = localStorage.getItem(KEY);
    if (!raw) return EMPTY_STATE;
    const parsed = JSON.parse(raw) as AppState;
    return parsed?.version === 2 ? { ...EMPTY_STATE, ...parsed } : EMPTY_STATE;
  } catch {
    return EMPTY_STATE;
  }
}

let state: AppState = typeof localStorage === 'undefined' ? EMPTY_STATE : load();
let saveError: string | null = null;
const listeners = new Set<() => void>();

export function getState(): AppState { return state; }
export function getSaveError(): string | null { return saveError; }

export function setState(update: (s: AppState) => AppState): void {
  state = update(state);
  try {
    localStorage.setItem(KEY, JSON.stringify(state));
    saveError = null;
  } catch (e) {
    saveError = e instanceof Error ? e.message : 'Could not save';
  }
  listeners.forEach(l => l());
}

function subscribe(l: () => void) { listeners.add(l); return () => listeners.delete(l); }

export function useAppState(): AppState {
  return useSyncExternalStore(subscribe, getState, getState);
}

export function newWeek(id: string): WeekPlan {
  return { id, craving: '', focus: EMPTY_FOCUS, picks: [], status: 'draft', have: [], seed: Math.floor(Math.random() * 1e6) };
}

export function updateWeek(id: string, fn: (w: WeekPlan) => WeekPlan): void {
  setState(s => ({ ...s, weeks: { ...s.weeks, [id]: fn(s.weeks[id] ?? newWeek(id)) } }));
}

export function uid(): string {
  return Math.random().toString(36).slice(2, 10);
}

/** For a target week: recipeId → weeks to the nearest other week (within 8) that uses it. */
export function recencyFor(s: AppState, weekId: string): Map<string, number> {
  const out = new Map<string, number>();
  for (const w of Object.values(s.weeks)) {
    if (w.id === weekId) continue;
    const d = Math.abs(weeksBetween(w.id, weekId));
    if (d > 8) continue;
    for (const p of w.picks) out.set(p.recipeId, Math.min(out.get(p.recipeId) ?? 99, d));
  }
  return out;
}

export function exportBackup(): string {
  return JSON.stringify({ app: 'MasterChef 9000', exportedAt: new Date().toISOString(), state }, null, 2);
}

export function importBackup(text: string): void {
  const parsed = JSON.parse(text);
  const next = parsed?.state ?? parsed;
  if (next?.version !== 2) throw new Error('This file is not a MasterChef 9000 backup.');
  setState(() => ({ ...EMPTY_STATE, ...next }));
}

export function resetAll(): void {
  setState(() => EMPTY_STATE);
}
