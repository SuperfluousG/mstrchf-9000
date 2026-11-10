import { afterEach, describe, expect, it } from 'vitest';
import terms from '../catalogue/terms.json';
import { recipeForPeople, selectMarket } from '../src/data';
import { badPlaceholders, renderText, type Terms } from '../src/domain/localize';

afterEach(() => selectMarket('uk'));
const T = terms as Terms;
describe('method quantities', () => {
  it('scales ingredients and counts while keeping cooking times, temperatures and cuts unchanged', () => {
    const text = 'Heat to {oven 220}. Add {ml 150} water and {qty 1.5|lime|limes}. Simmer 15 minutes; cut into {cm 2} pieces.';
    expect(renderText(text, 'uk', T, 2)).toBe('Heat to 220°C (200°C fan). Add 300 ml water and 3 limes. Simmer 15 minutes; cut into 2 cm pieces.');
    expect(renderText('{qty 1|egg|eggs}', 'uk', T, .5)).toBe('½ egg');
    expect(renderText('{qty 1|egg|eggs}', 'uk', T, 2)).toBe('2 eggs');
    expect(badPlaceholders(text, T)).toEqual([]);
  });
  it('scales before conversion to US or Canadian measures, including explicit spoon equivalents', () => {
    for (const m of ['us', 'ca'] as const) {
      expect(renderText('{15 g|1 tbsp}', m, T, 2)).toBe('2 tbsp');
      expect(renderText('{ml 150}', m, T, 2)).toBe('1¼ cups');
    }
    expect(renderText('{15 g|1 tbsp}', 'uk', T, 2)).toBe('30 g');
  });
  it('uses scaled method counts for the cashew-free larb recipe in all three countries', () => {
    for (const market of ['uk', 'us', 'ca'] as const) {
      selectMarket(market);
      const recipe = recipeForPeople('thai-larb-style-pork-lettuce-cups', 4)!;
      expect(recipe.steps.join(' ')).toContain('juice of 3 limes');
      expect(recipe.steps.join(' ')).not.toContain('1½ limes');
    }
  });
  it('scales the total burger count while keeping the two patties per burger and cooking times fixed', () => {
    selectMarket('uk');
    const recipe = recipeForPeople('double-beef-burgers-paprika-wedges', 4)!;
    expect(recipe.steps.join(' ')).toContain('shape it into 8 thin balls');
    expect(recipe.steps.join(' ')).toContain('two patties each');
    expect(recipe.steps.join(' ')).toContain('cook for 2 minutes');
  });
});
