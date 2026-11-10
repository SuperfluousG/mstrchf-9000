import { describe, expect, it } from 'vitest';
import { catalogue } from '../src/data';
import { buildBasket, claudePrompt } from '../src/domain/basket';
import { amount, countedName, inlineName, packLabel, recipeQty } from '../src/domain/format';
import { retailerById } from '../src/domain/market';
import { EMPTY_FOCUS, eligibleFor, parseCraving, planWeek, alternatives, whyNot, type PlanContext } from '../src/domain/planner';
import { DEFAULT_PROFILE, type Profile } from '../src/domain/profile';
import { bestPacks, buildShoppingList } from '../src/domain/shopping';
import { addWeeks, mondayOf, weekLabel, weeksBetween } from '../src/domain/weeks';

function ctx(profile: Partial<Profile> = {}, extra: Partial<PlanContext> = {}): PlanContext {
  return {
    catalogue,
    profile: { ...DEFAULT_PROFILE, ...profile },
    recency: new Map(),
    favourites: new Set(),
    ratings: new Map(),
    hidden: new Set(),
    focus: EMPTY_FOCUS,
    seed: 7,
    ...extra,
  };
}

const recipesOf = (picks: { recipeId: string }[]) => picks.map(p => catalogue.byId.get(p.recipeId)!);

describe('catalogue', () => {
  it('loads a large catalogue whose ingredients all resolve', () => {
    expect(catalogue.recipes.length).toBeGreaterThanOrEqual(100);
    for (const r of catalogue.recipes) for (const i of r.ingredients) expect(catalogue.ingredients.has(i.id)).toBe(true);
  });

  it('derives diet from ingredients, not labels', () => {
    for (const r of catalogue.recipes) {
      const f = catalogue.facts.get(r.id)!;
      const kinds = r.ingredients.map(i => catalogue.ingredients.get(i.id)!.kind);
      expect(f.vegetarian).toBe(!kinds.some(k => ['meat', 'poultry', 'fish', 'shellfish'].includes(k)));
    }
  });
});

describe('pack maths', () => {
  it('picks the cheapest covering packs', () => {
    expect(bestPacks(2, [[1, 0.7], [3, 1.9]]).cost).toBeCloseTo(1.4);
    expect(bestPacks(3, [[1, 0.7], [3, 1.9]]).cost).toBeCloseTo(1.9);
    expect(bestPacks(4, [[1, 0.7], [3, 1.9]]).cost).toBeCloseTo(2.6);
    expect(bestPacks(450, [[400, 0.55]]).bought).toBe(800);
  });
});

describe('planner', () => {
  it('fills the requested number of meals without repeats', () => {
    const picks = planWeek(ctx({ meals: { dinner: 5, lunch: 3, breakfast: 2 } }));
    expect(picks.filter(p => p.meal === 'dinner')).toHaveLength(5);
    expect(picks.filter(p => p.meal === 'lunch')).toHaveLength(3);
    expect(picks.filter(p => p.meal === 'breakfast')).toHaveLength(2);
    expect(new Set(picks.map(p => p.recipeId)).size).toBe(picks.length);
  });

  it('never breaks diet, allergy or dislike rules', () => {
    const c = ctx({ diet: 'vegetarian', avoid: ['gluten', 'peanuts'], dislikes: ['mushroom'], meals: { dinner: 7, lunch: 0, breakfast: 0 } });
    for (const r of recipesOf(planWeek(c))) {
      const f = catalogue.facts.get(r.id)!;
      expect(f.vegetarian).toBe(true);
      expect(f.allergens.has('gluten')).toBe(false);
      expect(f.allergens.has('peanuts')).toBe(false);
      expect(f.ingredientIds.has('mushroom')).toBe(false);
      expect(whyNot(r, c)).toBeNull();
    }
  });

  it('builds a week around a craving', () => {
    const focus = parseCraving('something spicy with loads of peppers', catalogue);
    expect(focus.tags).toContain('spicy');
    expect(focus.ingredients).toContain('red-pepper');
    expect(focus.ingredients).not.toContain('black-pepper');
    const plain = recipesOf(planWeek(ctx()));
    const craved = recipesOf(planWeek(ctx({}, { focus })));
    const withPeppers = craved.filter(r => r.ingredients.some(i => /pepper/.test(i.id) && i.id !== 'black-pepper'));
    expect(withPeppers.length).toBeGreaterThanOrEqual(3);
    const avg = (rs: typeof plain) => rs.reduce((s, r) => s + r.spice, 0) / rs.length;
    expect(avg(craved)).toBeGreaterThan(avg(plain));
  });

  it('leans away from processed ingredients when whole foods are asked for', () => {
    expect(parseCraving('comfort food, spicy, whole foods', catalogue).tags).toEqual(expect.arrayContaining(['comfort', 'spicy', 'whole']));
    const count = (rs: ReturnType<typeof recipesOf>) => rs.reduce((s, r) => s + catalogue.facts.get(r.id)!.processed.length, 0);
    const meals = { dinner: 5, lunch: 3, breakfast: 0 };
    const plain = recipesOf(planWeek(ctx({ meals })));
    const whole = recipesOf(planWeek(ctx({ meals }, { focus: { tags: ['whole'], ingredients: [], cuisines: [] } })));
    expect(count(whole)).toBeLessThan(Math.max(1, count(plain)));
  });

  it('keeps comfort food to dinners', () => {
    const picks = recipesOf(planWeek(ctx({ meals: { dinner: 5, lunch: 3, breakfast: 0 } }, { focus: { tags: ['comfort'], ingredients: [], cuisines: [] } })));
    expect(picks.filter(r => r.meal === 'dinner' && (r.tags.includes('comfort') || r.tags.includes('hearty'))).length).toBeGreaterThanOrEqual(3);
  });

  it('recognises cuisines in a craving', () => {
    expect(parseCraving('maybe some Thai or Mexican', catalogue).cuisines).toEqual(expect.arrayContaining(['thai', 'mexican']));
  });

  it('avoids meals eaten last week when there is choice', () => {
    const first = planWeek(ctx());
    const recency = new Map(first.map(p => [p.recipeId, 1]));
    const second = planWeek(ctx({}, { recency, seed: 7 }));
    const repeats = second.filter(p => recency.has(p.recipeId));
    expect(repeats).toHaveLength(0);
  });

  it('wastes less fresh food than just taking the top-scoring meals', () => {
    for (const people of [1, 2]) {
      const c = ctx({ people, meals: { dinner: 5, lunch: 3, breakfast: 0 } });
      const planned = recipesOf(planWeek(c));
      const naive = [...eligibleFor('dinner', c, new Set()).slice(0, 5), ...eligibleFor('lunch', c, new Set()).slice(0, 3)].map(s => s.recipe);
      const waste = (rs: typeof planned) => buildShoppingList({ recipes: rs, people, have: new Set() }, catalogue).waste;
      expect(waste(planned)).toBeLessThan(waste(naive));
      expect(buildShoppingList({ recipes: planned, people, have: new Set() }, catalogue).shared.length).toBeGreaterThan(0);
    }
  });

  it('keeps locked meals and offers swaps that are not already in the week', () => {
    const locked = catalogue.recipes.find(r => r.meal === 'dinner')!;
    const picks = planWeek(ctx(), [{ recipeId: locked.id, meal: 'dinner' }]);
    expect(picks.some(p => p.recipeId === locked.id)).toBe(true);
    const alts = alternatives(ctx(), picks, 0, 4);
    expect(alts).toHaveLength(4);
    for (const a of alts) expect(picks.some(p => p.recipeId === a.recipe.id)).toBe(false);
  });
});

describe('shopping list', () => {
  it('merges quantities across meals and scales to the household', () => {
    const rs = catalogue.recipes.filter(r => r.ingredients.some(i => i.id === 'onion')).slice(0, 2);
    const expected = rs.reduce((s, r) => s + r.ingredients.find(i => i.id === 'onion')!.qty, 0) * 2; // 4 people = 2× base
    const list = buildShoppingList({ recipes: rs, people: 4, have: new Set() }, catalogue);
    const onion = list.lines.find(l => l.ingredient.id === 'onion')!;
    expect(onion.need).toBeCloseTo(expected);
    expect(onion.usedIn).toHaveLength(2);
  });

  it('keeps pantry staples and owned items out of the basket total', () => {
    const r = catalogue.recipes.find(x => x.ingredients.some(i => i.id === 'olive-oil'))!;
    const first = r.ingredients.find(i => !catalogue.ingredients.get(i.id)!.pantry)!;
    const list = buildShoppingList({ recipes: [r], people: 2, have: new Set([first.id]) }, catalogue);
    expect(list.lines.find(l => l.ingredient.id === 'olive-oil')!.status).toBe('pantry');
    expect(list.lines.find(l => l.ingredient.id === first.id)!.status).toBe('have');
    const buyTotal = list.lines.filter(l => l.status === 'buy').reduce((s, l) => s + l.packs.cost, 0);
    expect(list.total).toBeCloseTo(buyTotal, 1);
  });
});

describe('formatting and dates', () => {
  it('shows small amounts in spoons', () => {
    const cumin = catalogue.ingredients.get('ground-cumin')!;
    expect(recipeQty(5, cumin)).toBe('2 tsp');
    expect(recipeQty(7.5, cumin)).toBe('1 tbsp');
    expect(amount(1500, 'g')).toBe('1.5 kg');
  });

  it('pluralises counted ingredients without doubling up', () => {
    const ing = (id: string) => catalogue.ingredients.get(id)!;
    expect(countedName(ing('spring-onion'), 2)).toBe('Spring onions');
    expect(countedName(ing('tortilla'), 2)).toBe('Flour tortilla wraps');
    expect(countedName(ing('garlic'), 2)).toBe('Garlic cloves');
    expect(countedName(ing('tomato'), 2)).toBe('Salad tomatoes');
    expect(countedName(ing('egg'), 1)).toBe('Free-range egg');
  });

  it('labels packs and names in plain words', () => {
    const ing = (id: string) => catalogue.ingredients.get(id)!;
    expect(packLabel(1, 2, ing('onion'))).toBe('2 loose');
    expect(packLabel(1, 2, ing('chickpeas'))).toBe('2 tins');
    expect(packLabel(8, 1, ing('celery'))).toBe('1 × pack of 8');
    expect(packLabel(500, 1, ing('beef-mince'))).toBe('1 × 500 g');
    expect(inlineName('Greek-style yoghurt')).toBe('Greek-style yoghurt');
    expect(inlineName('Fresh coriander')).toBe('fresh coriander');
    expect(inlineName('Roasted red peppers (jar, drained)')).toBe('roasted red peppers');
  });

  it('writes a basket with only the things to buy', () => {
    const rs = recipesOf(planWeek(ctx()));
    const list = buildShoppingList({ recipes: rs, people: 2, have: new Set(['onion']) }, catalogue);
    const b = buildBasket(list, '2026-11-16', 2, rs.map(r => r.title), catalogue.market, retailerById('tesco'));
    expect(b.items.every(i => i.search && i.buy.length > 0)).toBe(true);
    expect(b.items.some(i => i.id === 'onion')).toBe(false);
    expect(b.items.some(i => i.id === 'salt')).toBe(false);
    expect(b).toMatchObject({ version: 2, market: 'uk', retailer: 'tesco', retailerName: 'Tesco', currency: 'GBP' });
    expect(claudePrompt(b)).toMatch(/fill my Tesco grocery basket/);
    expect(claudePrompt(b)).toMatch(/Do not sign in, enter payment details or check out/);
  });

  it('handles weeks across a month and year boundary', () => {
    expect(mondayOf(new Date(2026, 10, 12))).toBe('2026-11-09');
    expect(addWeeks('2026-12-28', 1)).toBe('2027-01-04');
    expect(weeksBetween('2026-12-28', '2027-01-11')).toBe(2);
    expect(weekLabel('2026-11-30')).toBe('30 Nov – 6 Dec');
  });
});
