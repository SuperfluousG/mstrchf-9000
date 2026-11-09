import { catalogue } from '../data';
import { EMPTY_FOCUS, parseCraving, planWeek, type Focus, type PlanContext, type Rating } from '../domain/planner';
import type { Profile } from '../domain/profile';
import { getState, newWeek, recencyFor, setState, uid, updateWeek, type AppState, type WeekPlan } from './store';

export function focusFor(w: WeekPlan): Focus {
  const parsed = w.craving.trim() ? parseCraving(w.craving, catalogue) : EMPTY_FOCUS;
  return {
    tags: [...new Set([...w.focus.tags, ...parsed.tags])],
    ingredients: [...new Set([...w.focus.ingredients, ...parsed.ingredients])],
    cuisines: [...new Set([...w.focus.cuisines, ...parsed.cuisines])],
  };
}

export function contextFor(s: AppState, weekId: string, seed?: number): PlanContext {
  const w = s.weeks[weekId] ?? newWeek(weekId);
  return {
    catalogue,
    profile: s.profile!,
    recency: recencyFor(s, weekId),
    favourites: new Set(s.favourites),
    ratings: new Map(Object.entries(s.ratings)),
    hidden: new Set(s.hidden),
    focus: focusFor(w),
    seed: seed ?? w.seed,
  };
}

/** Plans (or re-plans) a week, keeping locked meals. `shuffle` picks a fresh seed. */
export function planWeekAction(weekId: string, shuffle = false): void {
  const s = getState();
  if (!s.profile) return;
  const w = s.weeks[weekId] ?? newWeek(weekId);
  const seed = shuffle ? Math.floor(Math.random() * 1e6) : w.seed;
  const locked = w.picks.filter(p => p.locked).map(p => ({ recipeId: p.recipeId, meal: p.meal }));
  const ctx = contextFor(getState(), weekId, seed);
  // Re-planning means "show me something else": meals being replaced count as just eaten.
  for (const p of w.picks) if (!p.locked) ctx.recency.set(p.recipeId, 1);
  const planned = planWeek(ctx, locked);
  updateWeek(weekId, wk => ({
    ...wk,
    seed,
    picks: planned.map(p => ({ uid: uid(), recipeId: p.recipeId, meal: p.meal, day: p.day, locked: p.locked, reasons: p.reasons })),
  }));
}

export function setCraving(weekId: string, craving: string): void {
  updateWeek(weekId, w => ({ ...w, craving }));
}

export function toggleFocusTag(weekId: string, tag: string): void {
  updateWeek(weekId, w => {
    const tags = w.focus.tags.includes(tag) ? w.focus.tags.filter(t => t !== tag) : [...w.focus.tags, tag];
    return { ...w, focus: { ...w.focus, tags } };
  });
}

export function swapPick(weekId: string, pickUid: string, recipeId: string, reasons: string[] = []): void {
  updateWeek(weekId, w => ({ ...w, picks: w.picks.map(p => p.uid === pickUid ? { ...p, recipeId, reasons, locked: true } : p) }));
}

export function toggleLock(weekId: string, pickUid: string): void {
  updateWeek(weekId, w => ({ ...w, picks: w.picks.map(p => p.uid === pickUid ? { ...p, locked: !p.locked } : p) }));
}

export function removePick(weekId: string, pickUid: string): void {
  updateWeek(weekId, w => ({ ...w, picks: w.picks.filter(p => p.uid !== pickUid) }));
}

export function addRecipe(weekId: string, recipeId: string): void {
  const r = catalogue.byId.get(recipeId);
  if (!r) return;
  updateWeek(weekId, w => {
    if (w.picks.some(p => p.recipeId === recipeId)) return w;
    const used = new Set(w.picks.filter(p => p.meal === r.meal).map(p => p.day));
    const day = [0, 1, 2, 3, 4, 5, 6].find(d => !used.has(d)) ?? 6;
    return { ...w, picks: [...w.picks, { uid: uid(), recipeId, meal: r.meal, day, locked: true, reasons: ['You chose this'] }] };
  });
}

export function moveDay(weekId: string, pickUid: string, day: number): void {
  updateWeek(weekId, w => {
    const me = w.picks.find(p => p.uid === pickUid);
    if (!me) return w;
    return {
      ...w,
      picks: w.picks.map(p => p.uid === pickUid ? { ...p, day } : p.meal === me.meal && p.day === day ? { ...p, day: me.day } : p),
    };
  });
}

export function toggleHave(weekId: string, ingredientId: string): void {
  const ing = catalogue.ingredients.get(ingredientId);
  if (!ing) return;
  if (ing.perishable) {
    updateWeek(weekId, w => ({ ...w, have: w.have.includes(ingredientId) ? w.have.filter(x => x !== ingredientId) : [...w.have, ingredientId] }));
  } else {
    setState(s => ({ ...s, cupboard: s.cupboard.includes(ingredientId) ? s.cupboard.filter(x => x !== ingredientId) : [...s.cupboard, ingredientId] }));
  }
}

export function setStatus(weekId: string, status: WeekPlan['status']): void {
  updateWeek(weekId, w => ({ ...w, status }));
}

export function toggleFavourite(recipeId: string): void {
  setState(s => ({ ...s, favourites: s.favourites.includes(recipeId) ? s.favourites.filter(x => x !== recipeId) : [...s.favourites, recipeId] }));
}

export function rate(recipeId: string, rating: Rating | null): void {
  setState(s => {
    const ratings = { ...s.ratings };
    if (rating === null) delete ratings[recipeId]; else ratings[recipeId] = rating;
    return { ...s, ratings };
  });
}

export function toggleHidden(recipeId: string): void {
  setState(s => ({ ...s, hidden: s.hidden.includes(recipeId) ? s.hidden.filter(x => x !== recipeId) : [...s.hidden, recipeId] }));
}

export function saveProfile(profile: Profile): void {
  setState(s => ({ ...s, profile }));
}
