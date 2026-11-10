import { useSyncExternalStore } from 'react';
import { EMPTY_STATE, type AppState, type WeekPlan } from '../domain/kitchen';
import { readBackup } from '../domain/backup';
import { isCodexApp } from '../codexHost';
export { EMPTY_STATE, type AppState, type WeekPlan, type Pick } from '../domain/kitchen';
export { readBackup } from '../domain/backup';
import { EMPTY_FOCUS } from '../domain/planner';
import { weeksBetween } from '../domain/weeks';
import { chooseState, readRemote, scheduleSave } from './fileSync';

const KEY = 'mstrchf9000:v2';


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
export function adoptSnapshot(remote: unknown, savedAt: number): void {
  const next = readBackup(JSON.stringify(remote ?? EMPTY_STATE));
  if (JSON.stringify(next.profile) === JSON.stringify(state.profile)) next.profile = state.profile;
  state = { ...next, savedAt };
  writeBrowser(); listeners.forEach(l => l());
}
export async function loadLatestKitchen(): Promise<void> {
  const latest = await readRemote<AppState>();
  if (latest) adoptSnapshot(latest.state, latest.savedAt);
}

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
  const choice = chooseState<AppState>({ state, savedAt: state.savedAt ?? 0 }, file, isEmptyState, { allowEmptyFile: isCodexApp });
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

export function importBackup(text: string): void {
  const next = readBackup(text);
  // A reviewed empty backup is an explicit reset, including in the external store.
  setState(() => next, { reset: true });
}

export function retrySave(): void {
  if (!isEmptyState(state)) scheduleSave(state, state.savedAt ?? Date.now());
}

export function resetAll(): void {
  setState(() => EMPTY_STATE, { reset: true });
}
