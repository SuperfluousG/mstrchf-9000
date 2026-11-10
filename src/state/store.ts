import { useSyncExternalStore } from 'react';
import type { BasketFile } from '../domain/basket';
import { MARKETS } from '../domain/market';
import { EMPTY_FOCUS, type Focus, type Rating } from '../domain/planner';
import type { Profile } from '../domain/profile';
import type { MealType } from '../domain/types';
import { weeksBetween } from '../domain/weeks';
import { chooseState, readRemote, scheduleSave } from './fileSync';

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

function writeBrowser(): void {
  try {
    localStorage.setItem(KEY, JSON.stringify(state));
    saveError = null;
  } catch (e) {
    saveError = e instanceof Error ? e.message : 'Could not save';
  }
}

export function setState(update: (s: AppState) => AppState, opts: { reset?: boolean } = {}): void {
  const next = update(state);
  if (next === state && !opts.reset) return;
  state = { ...next, savedAt: Date.now() };
  writeBrowser();
  // An empty kitchen is only ever written on purpose (Start over).
  if (opts.reset || !isEmptyState(state)) scheduleSave(state, state.savedAt!, opts.reset);
  listeners.forEach(l => l());
}

export function isEmptyState(s: AppState): boolean {
  return !s.profile && Object.keys(s.weeks).length === 0;
}

/**
 * Loads the stored kitchen (Claude storage in an artifact, or the local kitchen file) and keeps
 * whichever of the browser and stored copies is newer. Call once before the first render.
 */
export async function hydrate(): Promise<void> {
  const file = await readRemote<AppState>();
  if (!file) {
    if (!isEmptyState(state)) scheduleSave(state, state.savedAt ?? Date.now());
    return;
  }
  const choice = chooseState<AppState>({ state, savedAt: state.savedAt ?? 0 }, file, isEmptyState);
  if (choice.from === 'file' && choice.state) {
    state = { ...EMPTY_STATE, ...choice.state };
    writeBrowser();
    listeners.forEach(l => l());
  } else if (choice.pushToFile) {
    scheduleSave(state, state.savedAt ?? Date.now());
  }
}

function subscribe(l: () => void) { listeners.add(l); return () => listeners.delete(l); }

export function useAppState(): AppState {
  return useSyncExternalStore(subscribe, getState, getState);
}

export function newWeek(id: string): WeekPlan {
  return { id, craving: '', focus: EMPTY_FOCUS, picks: [], status: 'draft', have: [], seed: Math.floor(Math.random() * 1e6) };
}

export function updateWeek(id: string, fn: (w: WeekPlan) => WeekPlan): void {
  setState(s => {
    const before = s.weeks[id] ?? newWeek(id);
    const after = fn(before);
    return after === before ? s : { ...s, weeks: { ...s.weeks, [id]: after } };
  });
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
  return JSON.stringify({ app: 'MSTRCHF 9000', exportedAt: new Date().toISOString(), state }, null, 2);
}

export function readBackup(text: string): AppState {
  const parsed = JSON.parse(text);
  const next = (parsed?.state ?? parsed) as AppState;
  const strings = (x: unknown): x is string[] => Array.isArray(x) && x.every(v => typeof v === 'string');
  const object = (x: unknown) => !!x && typeof x === 'object' && !Array.isArray(x);
  const invalid = () => { throw new Error('This file is not a complete MSTRCHF 9000 backup. Your kitchen has not changed.'); };
  if (!object(next) || next.version !== 2 || !object(next.weeks) || !('profile' in next)) return invalid();
  if (parsed?.app && parsed.app !== 'MSTRCHF 9000') return invalid();
  const p = next.profile;
  if (p !== null && (!object(p) || !Number.isInteger(p.people) || p.people < 1 || p.people > 8
    || !['omnivore', 'vegetarian', 'vegan', 'pescatarian'].includes(p.diet)
    || !strings(p.avoid) || !strings(p.dislikes) || !object(p.meals) || !object(p.cuisines) || !object(p.proteins)
    || ![...Object.values(p.cuisines), ...Object.values(p.proteins)].every(n => [-1, 0, 1, 2].includes(n))
    || ![p.meals.dinner, p.meals.lunch, p.meals.breakfast].every(n => Number.isInteger(n) && n >= 0 && n <= 7)
    || ![0, 1, 2, 3].includes(p.spice) || !Number.isFinite(p.maxTime)
    || !['relaxed', 'balanced', 'tight'].includes(p.budget) || (p.market !== undefined && !Object.hasOwn(MARKETS, p.market)))) return invalid();
  if (![next.favourites, next.hidden, next.cupboard].every(x => x === undefined || strings(x))
    || (next.ratings !== undefined && (!object(next.ratings) || !Object.values(next.ratings).every(n => [-1, 0, 1].includes(n))))) return invalid();
  for (const [id, w] of Object.entries(next.weeks)) {
    if (!/^\d{4}-\d{2}-\d{2}$/.test(id) || !object(w) || w.id !== id || !Array.isArray(w.picks)
      || !['draft', 'ordered'].includes(w.status) || !strings(w.have) || typeof w.craving !== 'string' || !Number.isFinite(w.seed)
      || (w.pantryToBuy !== undefined && !strings(w.pantryToBuy))
      || !object(w.focus) || !strings(w.focus.tags) || !strings(w.focus.ingredients) || !strings(w.focus.cuisines)
      || !w.picks.every(pick => object(pick) && typeof pick.uid === 'string' && typeof pick.recipeId === 'string'
        && ['dinner', 'lunch', 'breakfast'].includes(pick.meal) && Number.isInteger(pick.day) && pick.day >= 0 && pick.day <= 6 && strings(pick.reasons)
        && (pick.locked === undefined || typeof pick.locked === 'boolean'))) return invalid();
    const b = w.orderedBasket;
    if (b && (b.app !== 'MSTRCHF 9000' || b.format !== 'basket' || b.version !== 2 || !Object.hasOwn(MARKETS, b.market)
      || b.currency !== MARKETS[b.market].currency || b.weekId !== id || typeof b.week !== 'string' || typeof b.createdAt !== 'string'
      || !Number.isFinite(b.estTotal) || !Number.isInteger(b.people) || b.people < 1 || b.people > 8
      || (b.retailerName !== null && typeof b.retailerName !== 'string')
      || !strings(b.meals) || !strings(b.alreadyHave) || !Array.isArray(b.items)
      || !b.items.every(i => object(i) && typeof i.id === 'string' && typeof i.name === 'string' && typeof i.aisle === 'string'
        && typeof i.need === 'string' && typeof i.search === 'string' && Number.isFinite(i.estPrice) && strings(i.usedIn)
        && Array.isArray(i.buy) && i.buy.every(pack => object(pack) && typeof pack.size === 'string' && Number.isInteger(pack.count) && pack.count > 0)))) return invalid();
  }
  return { ...EMPTY_STATE, ...next };
}

export function importBackup(text: string): void {
  const next = readBackup(text);
  // A reviewed empty backup is an explicit reset, including in the external store.
  setState(() => next, { reset: isEmptyState(next) });
}

export function retrySave(): void {
  if (!isEmptyState(state)) scheduleSave(state, state.savedAt ?? Date.now());
}

export function resetAll(): void {
  setState(() => EMPTY_STATE, { reset: true });
}
