import { afterEach, describe, expect, it } from 'vitest';
import terms from '../catalogue/terms.json';
import { catalogue, catalogueFor, selectMarket, sourceRecipes } from '../src/data';
import { buildBasket, claudePrompt } from '../src/domain/basket';
import { amount, money, packLabel, recipeQty } from '../src/domain/format';
import { badPlaceholders, renderText, type Terms } from '../src/domain/localize';
import { detectMarket, type Market, retailerById } from '../src/domain/market';
import { EMPTY_FOCUS, planWeek } from '../src/domain/planner';
import { DEFAULT_PROFILE, retailerOf } from '../src/domain/profile';
import { buildShoppingList } from '../src/domain/shopping';

const T = terms as Terms;

afterEach(() => selectMarket('uk'));

describe('placeholders', () => {
  it('renders measures for each country', () => {
    const text = 'Heat the oven to {oven 220}. Fry in {ml 15} of oil, add {ml 300} of water and {g 50} of feta, cut into {cm 2} chunks.';
    expect(renderText(text, 'uk', T)).toBe('Heat the oven to 220°C (200°C fan). Fry in 15 ml of oil, add 300 ml of water and 50 g of feta, cut into 2 cm chunks.');
    expect(renderText(text, 'us', T)).toBe('Heat the oven to 425°F. Fry in 1 tbsp of oil, add 1¼ cups of water and 1¾ oz of feta, cut into ¾ inch chunks.');
    expect(renderText('{oven 200}', 'ca', T)).toBe('400°F');
    expect(renderText('{oven 180}', 'us', T)).toBe('350°F');
  });

  it('swaps British food words and keeps capitals', () => {
    expect(renderText('{Courgette} and {spring onions} with {coriander}', 'us', T)).toBe('Zucchini and green onions with cilantro');
    expect(renderText('{Courgette} and {spring onions} with {coriander}', 'uk', T)).toBe('Courgette and spring onions with coriander');
    expect(renderText('{chilli flakes}', 'us', T)).toBe('red pepper flakes');
    expect(renderText('{chilli flakes}', 'ca', T)).toBe('chili flakes');
    expect(renderText('Heat the {grill|broiler}', 'us', T)).toBe('Heat the broiler');
    expect(renderText('Heat the {grill|broiler}', 'uk', T)).toBe('Heat the grill');
  });

  it('reports placeholders it cannot render', () => {
    expect(badPlaceholders('{courgette} {oven 220} {grill|broil}', T)).toEqual([]);
    expect(badPlaceholders('{zorbs} {oven hot} {a|}', T)).toEqual(['{zorbs}', '{oven hot}', '{a|}']);
    expect(badPlaceholders('an {unclosed', T)).toEqual(['unmatched brace']);
  });

  it('every recipe renders cleanly in every country', () => {
    for (const r of sourceRecipes) {
      for (const text of [r.title, r.blurb, ...r.steps, ...r.ingredients.map(i => i.prep ?? '')]) {
        expect(badPlaceholders(text, T), `${r.id}: ${text}`).toEqual([]);
      }
    }
    for (const m of ['uk', 'us', 'ca'] as Market[]) {
      for (const r of catalogueFor(m).recipes) {
        const all = [r.title, r.blurb, ...r.steps, ...r.ingredients.map(i => i.prep ?? '')].join(' ');
        expect(all, r.id).not.toMatch(/[{}]/);
      }
    }
  });

  it('leaves no metric measures or British-only food words in the American text', () => {
    const offenders: string[] = [];
    for (const r of catalogueFor('us').recipes) {
      let text = [r.title, r.blurb, ...r.steps, ...r.ingredients.map(i => i.prep ?? '')].join(' ');
      if (/°C|\d\s?(ml|g|cm)\b/.test(text)) offenders.push(`${r.id}: metric`);
      // American words can contain British ones ("red pepper flakes"); take them out before looking.
      for (const t of Object.values(T)) text = text.replace(new RegExp(`\\b${t.us}\\b`, 'gi'), '');
      for (const [word, t] of Object.entries(T)) {
        if (t.onlyWith && !r.ingredients.some(i => i.id === t.onlyWith)) continue;
        const cleaned = t.keep ? text.replace(new RegExp(t.keep, 'gi'), '') : text;
        if (new RegExp(`\\b${word}\\b`, 'i').test(cleaned)) offenders.push(`${r.id}: ${word}`);
      }
    }
    expect(offenders).toEqual([]);
  });
});

describe('countries', () => {
  it('guesses the country from the browser language', () => {
    expect(detectMarket('en-US')).toBe('us');
    expect(detectMarket('en-CA')).toBe('ca');
    expect(detectMarket('fr-CA')).toBe('ca');
    expect(detectMarket('en-GB')).toBe('uk');
    expect(detectMarket('fr-FR')).toBe('uk');
  });

  it('has US and Canadian packs for every ingredient', () => {
    const uk = catalogueFor('uk');
    for (const m of ['us', 'ca'] as Market[]) {
      const cat = catalogueFor(m);
      expect(cat.ingredients.size).toBe(uk.ingredients.size);
      for (const [id, ing] of cat.ingredients) expect(ing.packs, `${m} ${id}`).not.toBe(uk.ingredients.get(id)!.packs);
    }
  });

  it('shows American names, measures and dollars in the US', () => {
    selectMarket('us');
    expect(catalogue.market.id).toBe('us');
    const ing = (id: string) => catalogue.ingredients.get(id)!;
    expect(ing('courgette').name).toBe('Zucchini');
    expect(ing('coriander').name).toBe('Cilantro');
    expect(money(3.5)).toBe('$3.50');
    expect(amount(454, 'g')).toBe('1 lb');
    expect(amount(946, 'ml')).toBe('1 qt');
    expect(recipeQty(300, ing('chicken-thigh'))).toBe('11 oz');
    expect(packLabel(1, 2, ing('chickpeas'))).toBe('2 cans');
  });

  it('uses metric packs, American words and Canadian dollars in Canada', () => {
    selectMarket('ca');
    const ing = (id: string) => catalogue.ingredients.get(id)!;
    expect(ing('courgette').name).toBe('Zucchini');
    expect(amount(540, 'ml')).toBe('540 mL');
    expect(money(4)).toBe('$4.00');
    expect(catalogue.market.currency).toBe('CAD');
  });

  it('plans a week and writes a basket in each country', () => {
    for (const m of ['uk', 'us', 'ca'] as Market[]) {
      selectMarket(m);
      const ctx = { catalogue, profile: { ...DEFAULT_PROFILE, market: m }, recency: new Map(), favourites: new Set<string>(), ratings: new Map(), hidden: new Set<string>(), focus: EMPTY_FOCUS, seed: 3 };
      const picks = planWeek(ctx);
      expect(picks.filter(p => p.meal === 'dinner')).toHaveLength(5);
      const recipes = picks.map(p => catalogue.byId.get(p.recipeId)!);
      const list = buildShoppingList({ recipes, people: 2, have: new Set() }, catalogue);
      expect(list.total).toBeGreaterThan(5);
      const shop = retailerOf({ ...DEFAULT_PROFILE, market: m });
      const b = buildBasket(list, '2026-11-16', 2, recipes.map(r => r.title), catalogue.market, shop);
      expect(b.market).toBe(m);
      expect(b.retailer).toBe(shop?.id);
      expect(claudePrompt(b)).toContain(`fill my ${shop?.name} grocery ${catalogue.market.basket}`);
    }
  });

  it('defaults to the first listed shop and allows another shop', () => {
    expect(retailerOf({ ...DEFAULT_PROFILE, market: 'uk' })?.id).toBe('tesco');
    expect(retailerOf({ ...DEFAULT_PROFILE, market: 'us' })?.id).toBe('walmart');
    expect(retailerOf({ ...DEFAULT_PROFILE, market: 'us', retailer: 'kroger' })?.id).toBe('kroger');
    expect(retailerOf({ ...DEFAULT_PROFILE, market: 'us', retailer: null })).toBeUndefined();
    expect(retailerOf({ ...DEFAULT_PROFILE, market: 'us', retailer: 'tesco' })).toBeUndefined();
    expect(retailerById('voila')?.market).toBe('ca');
  });
});
