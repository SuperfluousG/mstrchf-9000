import { beforeEach, describe, expect, it } from 'vitest';
import { catalogue, selectMarket } from '../src/data';
import { basketText } from '../src/domain/basket';
import { DEFAULT_PROFILE } from '../src/domain/profile';
import { addRecipe, moveDay, planWeekAction, removePick, saveProfile, setCraving, setStatus, swapPick, toggleFocusTag, toggleHave, toggleLock, togglePantry, weekExclusions } from '../src/state/actions';
import { EMPTY_STATE, getState, setState } from '../src/state/store';

const WEEK = '2026-11-16';
beforeEach(() => {
  selectMarket('uk');
  setState(() => ({ ...structuredClone(EMPTY_STATE), profile: structuredClone(DEFAULT_PROFILE) }));
});

describe('planning edits', () => {
  it('rechecks kept meals against changed allergies and replaces excluded meals', () => {
    const nutty = catalogue.recipes.find(r => r.meal === 'dinner' && catalogue.facts.get(r.id)!.allergens.has('nuts'))!;
    addRecipe(WEEK, nutty.id);
    saveProfile({ ...DEFAULT_PROFILE, avoid: ['nuts'] });
    expect(weekExclusions(getState(), WEEK)).toHaveLength(1);
    expect(setStatus(WEEK, 'ordered')).toBe(false);
    planWeekAction(WEEK);
    const w = getState().weeks[WEEK];
    expect(w.picks).toHaveLength(5);
    expect(w.picks.some(p => p.recipeId === nutty.id)).toBe(false);
    expect(weekExclusions(getState(), WEEK)).toEqual([]);
    expect(addRecipe(WEEK, nutty.id)).toBeNull();
  });

  it('preserves a chosen day and meal identity through replanning and shuffling', () => {
    const r = catalogue.recipes.find(r => r.meal === 'dinner')!;
    const pick = addRecipe(WEEK, r.id, 6)!;
    for (const shuffle of [false, true]) {
      planWeekAction(WEEK, shuffle);
      expect(getState().weeks[WEEK].picks.find(p => p.recipeId === r.id)).toMatchObject({ uid: pick.uid, day: 6, locked: true });
      const dinners = getState().weeks[WEEK].picks.filter(p => p.meal === 'dinner');
      expect(new Set(dinners.map(p => p.day)).size).toBe(dinners.length);
    }
  });

  it('swaps a specific meal without adding another or changing its day', () => {
    const [a, b] = catalogue.recipes.filter(r => r.meal === 'dinner');
    const pick = addRecipe(WEEK, a.id, 4)!;
    expect(swapPick(WEEK, pick.uid, b.id)).toBe(true);
    expect(getState().weeks[WEEK].picks).toMatchObject([{ uid: pick.uid, recipeId: b.id, day: 4, locked: true }]);
  });

  it('does not silently put a second meal into an occupied day or overflow the week', () => {
    const dinners = catalogue.recipes.filter(r => r.meal === 'dinner').slice(0, 8);
    dinners.slice(0, 7).forEach((r, day) => expect(addRecipe(WEEK, r.id, day)).not.toBeNull());
    expect(addRecipe(WEEK, dinners[7].id)).toBeNull();
    expect(addRecipe(WEEK, dinners[7].id, 3)).toBeNull();
    expect(getState().weeks[WEEK].picks).toHaveLength(7);
  });
});

describe('ordered weeks', () => {
  it('records an explicitly requested cupboard staple and cannot change it after ordering', () => {
    const recipe = catalogue.recipes.find(r => r.meal === 'dinner' && r.ingredients.some(i => i.id === 'olive-oil'))!;
    addRecipe(WEEK, recipe.id); togglePantry(WEEK, 'olive-oil'); setStatus(WEEK, 'ordered');
    expect(getState().weeks[WEEK].orderedBasket!.items.some(i => i.id === 'olive-oil')).toBe(true);
    togglePantry(WEEK, 'olive-oil');
    expect(getState().weeks[WEEK].pantryToBuy).toEqual(['olive-oil']);
  });
  it('freezes the recorded basket and guards edits from every planning entry point', () => {
    planWeekAction(WEEK);
    expect(setStatus(WEEK, 'ordered')).toBe(true);
    const before = structuredClone(getState().weeks[WEEK]);
    const first = before.picks[0];
    const other = catalogue.recipes.find(r => r.meal === first.meal && !before.picks.some(p => p.recipeId === r.id))!;
    expect(addRecipe(WEEK, other.id)).toBeNull();
    expect(swapPick(WEEK, first.uid, other.id)).toBe(false);
    removePick(WEEK, first.uid); toggleLock(WEEK, first.uid);
    setCraving(WEEK, 'different'); toggleFocusTag(WEEK, 'spicy');
    toggleHave(WEEK, 'soy-sauce'); planWeekAction(WEEK, true);
    expect(getState().weeks[WEEK]).toEqual(before);
    expect(getState().cupboard).toEqual([]);
    moveDay(WEEK, first.uid, 6);
    expect(getState().weeks[WEEK].picks.find(p => p.uid === first.uid)?.day).toBe(6);
    expect(getState().weeks[WEEK].orderedBasket).toEqual(before.orderedBasket);
  });

  it('retains names, quantities, currency and people when the kitchen changes', () => {
    planWeekAction(WEEK); setStatus(WEEK, 'ordered');
    const basket = structuredClone(getState().weeks[WEEK].orderedBasket!);
    saveProfile({ ...DEFAULT_PROFILE, market: 'us', people: 4 });
    expect(getState().weeks[WEEK].orderedBasket).toEqual(basket);
    expect(basketText(getState().weeks[WEEK].orderedBasket!)).toContain(`Estimated total: £${basket.estTotal.toFixed(2)}`);
    expect(setStatus(WEEK, 'draft')).toBe(true);
    planWeekAction(WEEK);
    expect(setStatus(WEEK, 'ordered')).toBe(true);
    expect(getState().weeks[WEEK].orderedBasket).toMatchObject({ market: 'us', currency: 'USD', people: 4 });
  });
});
