import { catalogue, selectMarket } from '../data';
import { buildBasket } from '../domain/basket';
import { EMPTY_FOCUS, exclusionReason, parseCraving, planWeek, type Focus, type PlanContext, type Rating } from '../domain/planner';
import { marketOf, retailerOf, type Profile } from '../domain/profile';
import { buildShoppingList } from '../domain/shopping';
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
  if (w.status === 'ordered') return;
  const seed = shuffle ? Math.floor(Math.random() * 1e6) : w.seed;
  const locked = w.picks.filter(p => p.locked).map(p => ({ recipeId: p.recipeId, meal: p.meal, day: p.day }));
  const ctx = contextFor(getState(), weekId, seed);
  // Re-planning means "show me something else": meals being replaced count as just eaten.
  for (const p of w.picks) if (!p.locked) ctx.recency.set(p.recipeId, 1);
  const planned = planWeek(ctx, locked);
  updateWeek(weekId, wk => ({
    ...wk,
    seed,
    picks: planned.map(p => ({ uid: w.picks.find(old => old.recipeId === p.recipeId && old.locked)?.uid ?? uid(), recipeId: p.recipeId, meal: p.meal, day: p.day, locked: p.locked, reasons: p.reasons })),
  }));
}

function editWeek(id: string, edit: (w: WeekPlan) => WeekPlan): void {
  updateWeek(id, w => w.status === 'ordered' ? w : edit(w));
}

export function weekExclusions(s: AppState, weekId: string): { pick: WeekPlan['picks'][number]; reason: string }[] {
  if (!s.profile) return [];
  const ctx = contextFor(s, weekId);
  return (s.weeks[weekId]?.picks ?? []).flatMap(pick => {
    const recipe = catalogue.byId.get(pick.recipeId);
    const reason = recipe ? exclusionReason(recipe, ctx) : 'Recipe no longer available';
    return reason ? [{ pick, reason }] : [];
  });
}

export function setCraving(weekId: string, craving: string): void {
  editWeek(weekId, w => ({ ...w, craving }));
}

export function toggleFocusTag(weekId: string, tag: string): void {
  editWeek(weekId, w => {
    const tags = w.focus.tags.includes(tag) ? w.focus.tags.filter(t => t !== tag) : [...w.focus.tags, tag];
    return { ...w, focus: { ...w.focus, tags } };
  });
}

export function swapPick(weekId: string, pickUid: string, recipeId: string, reasons: string[] = []): boolean {
  const s = getState();
  const w = s.weeks[weekId];
  const recipe = catalogue.byId.get(recipeId);
  const pick = w?.picks.find(p => p.uid === pickUid);
  if (!recipe || !pick || !s.profile || w.status === 'ordered' || recipe.meal !== pick.meal
    || w.picks.some(p => p.uid !== pickUid && p.recipeId === recipeId)
    || exclusionReason(recipe, contextFor(s, weekId))) return false;
  editWeek(weekId, w => ({ ...w, picks: w.picks.map(p => p.uid === pickUid ? { ...p, recipeId, reasons, locked: true } : p) }));
  return true;
}

export function toggleLock(weekId: string, pickUid: string): void {
  editWeek(weekId, w => ({ ...w, picks: w.picks.map(p => p.uid === pickUid ? { ...p, locked: !p.locked } : p) }));
}

export function removePick(weekId: string, pickUid: string): void {
  editWeek(weekId, w => ({ ...w, picks: w.picks.filter(p => p.uid !== pickUid) }));
}

export function addRecipe(weekId: string, recipeId: string, chosenDay?: number): { uid: string; day: number } | null {
  const s = getState();
  const r = catalogue.byId.get(recipeId);
  const w = s.weeks[weekId] ?? newWeek(weekId);
  if (!r || !s.profile || w.status === 'ordered' || w.picks.some(p => p.recipeId === recipeId)
    || exclusionReason(r, contextFor(s, weekId))) return null;
  const used = new Set(w.picks.filter(p => p.meal === r.meal).map(p => p.day));
  const day = chosenDay ?? [0, 1, 2, 3, 4, 5, 6].find(d => !used.has(d));
  if (day === undefined || !Number.isInteger(day) || day < 0 || day > 6 || used.has(day)) return null;
  const id = uid();
  editWeek(weekId, wk => ({ ...wk, picks: [...wk.picks, { uid: id, recipeId, meal: r.meal, day, locked: true, reasons: ['You chose this'] }] }));
  return { uid: id, day };
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
  if (getState().weeks[weekId]?.status === 'ordered') return;
  const ing = catalogue.ingredients.get(ingredientId);
  if (!ing) return;
  if (ing.perishable) {
    updateWeek(weekId, w => ({ ...w, have: w.have.includes(ingredientId) ? w.have.filter(x => x !== ingredientId) : [...w.have, ingredientId] }));
  } else {
    setState(s => ({ ...s, cupboard: s.cupboard.includes(ingredientId) ? s.cupboard.filter(x => x !== ingredientId) : [...s.cupboard, ingredientId] }));
  }
}

export function togglePantry(weekId: string, ingredientId: string): void {
  if (!catalogue.ingredients.get(ingredientId)?.pantry) return;
  editWeek(weekId, w => ({ ...w, pantryToBuy: (w.pantryToBuy ?? []).includes(ingredientId)
    ? w.pantryToBuy!.filter(id => id !== ingredientId) : [...w.pantryToBuy ?? [], ingredientId] }));
}

export function setStatus(weekId: string, status: WeekPlan['status']): boolean {
  const s = getState();
  const w = s.weeks[weekId];
  if (!w || !s.profile || w.status === status) return false;
  if (status === 'draft') { updateWeek(weekId, wk => ({ ...wk, status })); return true; }
  if (!w.picks.length || weekExclusions(s, weekId).length) return false;
  const recipes = w.picks.map(p => catalogue.byId.get(p.recipeId)!);
  const list = buildShoppingList({ recipes, people: s.profile.people, have: new Set([...w.have, ...s.cupboard]), pantryToBuy: new Set(w.pantryToBuy ?? []) }, catalogue);
  const orderedBasket = buildBasket(list, weekId, s.profile.people, recipes.map(r => r.title), catalogue.market, retailerOf(s.profile));
  updateWeek(weekId, wk => ({ ...wk, status, orderedBasket }));
  return true;
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
  // Switch country now, not on the next render, so a week planned straight after uses the right names and prices.
  selectMarket(marketOf(profile));
  setState(s => ({ ...s, profile }));
}
