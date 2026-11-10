import type { Catalogue } from './catalogue';
import { activeMarket } from './market';
import type { Aisle, Ingredient, Recipe } from './types';

export type Demand = Map<string, number>;

export interface PackChoice { size: number; count: number; price: number }

export interface PackResult { buy: PackChoice[]; cost: number; bought: number }

/** Cheapest combination of pack sizes covering `need` (ties broken by least surplus). */
export function bestPacks(need: number, packs: [number, number][]): PackResult {
  if (need <= 0) return { buy: [], cost: 0, bought: 0 };
  const sorted = [...packs].sort((a, b) => b[0] - a[0]);
  let best: PackResult | null = null;
  const consider = (counts: number[]) => {
    let bought = 0, cost = 0;
    counts.forEach((c, i) => { bought += c * sorted[i][0]; cost += c * sorted[i][1]; });
    if (bought + 1e-9 < need) return;
    if (!best || cost < best.cost - 1e-9 || (Math.abs(cost - best.cost) < 1e-9 && bought < best.bought)) {
      best = {
        buy: counts.map((c, i) => ({ size: sorted[i][0], count: c, price: sorted[i][1] })).filter(p => p.count > 0),
        cost: Math.round(cost * 100) / 100,
        bought,
      };
    }
  };
  const max = sorted.map(([size]) => Math.ceil(need / size));
  if (sorted.length === 1) consider([max[0]]);
  else {
    for (let a = 0; a <= max[0]; a++) {
      for (let b = 0; b <= max[1]; b++) {
        const rest = need - a * sorted[0][0] - b * sorted[1][0];
        if (sorted.length === 2) consider([a, b]);
        else {
          const tail = sorted.slice(2).map(() => 0);
          if (rest > 0) tail[tail.length - 1] = Math.ceil(rest / sorted[sorted.length - 1][0]);
          consider([a, b, ...tail]);
        }
      }
    }
  }
  return best ?? { buy: [], cost: 0, bought: 0 };
}

/** Ingredient demand for one recipe scaled to `people` servings. */
export function recipeDemand(recipe: Recipe, people: number): Demand {
  const factor = people / recipe.servings;
  const d: Demand = new Map();
  for (const ri of recipe.ingredients) d.set(ri.id, (d.get(ri.id) ?? 0) + ri.qty * factor);
  return d;
}

export function mergeDemand(...ds: Demand[]): Demand {
  const out: Demand = new Map();
  for (const d of ds) for (const [k, v] of d) out.set(k, (out.get(k) ?? 0) + v);
  return out;
}

/** Cost of buying the perishable part of a demand. Used to reward shared fresh ingredients. */
export function perishableCost(d: Demand, cat: Catalogue): number {
  let total = 0;
  for (const [id, qty] of d) {
    const ing = cat.ingredients.get(id);
    if (!ing || !ing.perishable || ing.pantry) continue;
    total += bestPacks(qty, ing.packs).cost;
  }
  return total;
}

/** Leftover meat and fish usually go in the freezer, so their surplus counts for less. */
const FREEZABLE_WASTE_FACTOR = 0.35;

/** Estimated value of fresh food bought but likely to be thrown away (pack surplus). */
export function perishableWaste(d: Demand, cat: Catalogue): number {
  let total = 0;
  for (const [id, qty] of d) {
    const ing = cat.ingredients.get(id);
    if (!ing || !ing.perishable || ing.pantry) continue;
    const p = bestPacks(qty, ing.packs);
    const factor = ing.aisle === 'meat' || ing.aisle === 'fish' ? FREEZABLE_WASTE_FACTOR : 1;
    if (p.bought > 0) total += factor * p.cost * Math.max(0, p.bought - qty) / p.bought;
  }
  return total;
}

export type LineStatus = 'buy' | 'have' | 'pantry';

export interface ShopLine {
  ingredient: Ingredient;
  need: number;
  usedIn: { recipeId: string; title: string; qty: number }[];
  packs: PackResult;
  leftover: number;
  /** Estimated value of perishable surplus that is likely to be thrown away. */
  wasteCost: number;
  status: LineStatus;
}

export interface ShoppingList {
  lines: ShopLine[];
  /** Estimated cost of the lines with status "buy". */
  total: number;
  waste: number;
  /** Perishable lines used by two or more meals. */
  shared: ShopLine[];
  /** How much buying the meals together saves compared with buying each meal's fresh food separately. */
  groupingSaving: number;
}

export const AISLE_ORDER: Aisle[] = ['veg', 'fruit', 'herbs', 'meat', 'fish', 'dairy', 'eggs', 'bakery', 'grains', 'tins', 'world', 'sauces', 'spices', 'nuts', 'frozen', 'cupboard'];

export const AISLE_LABELS: Record<Aisle, string> = {
  veg: 'Fruit & veg', fruit: 'Fruit & veg', herbs: 'Fresh herbs', meat: 'Meat', fish: 'Fish',
  dairy: 'Dairy & chilled', eggs: 'Dairy & chilled', bakery: 'Bakery', grains: 'Rice, pasta & grains',
  tins: 'Tins & jars', world: 'World foods', sauces: 'Sauces & condiments', spices: 'Herbs & spices',
  nuts: 'Nuts & seeds', frozen: 'Frozen', cupboard: 'Cupboard',
};

/** Aisle names as North American grocers label them. */
const AISLE_LABELS_NA: Record<Aisle, string> = {
  ...AISLE_LABELS,
  veg: 'Produce', fruit: 'Produce', fish: 'Seafood', dairy: 'Dairy & eggs', eggs: 'Dairy & eggs',
  tins: 'Canned goods & jars', world: 'International', sauces: 'Condiments & sauces', spices: 'Spices & seasonings',
  cupboard: 'Pantry',
};

export function aisleLabel(aisle: Aisle): string {
  return (activeMarket().id === 'uk' ? AISLE_LABELS : AISLE_LABELS_NA)[aisle];
}

export interface ShoppingInput {
  recipes: Recipe[];
  people: number;
  /** Ingredient ids the user already has (cupboard or this week). */
  have: Set<string>;
  /** Include pantry staples (salt, oil) in the basket. */
  includePantry?: boolean;
  pantryToBuy?: Set<string>;
}

export function buildShoppingList(input: ShoppingInput, cat: Catalogue): ShoppingList {
  const lines = new Map<string, ShopLine>();
  const perRecipe: Demand[] = [];
  for (const recipe of input.recipes) {
    const d = recipeDemand(recipe, input.people);
    perRecipe.push(d);
    for (const [id, qty] of d) {
      const ingredient = cat.ingredients.get(id);
      if (!ingredient) continue;
      let line = lines.get(id);
      if (!line) {
        line = { ingredient, need: 0, usedIn: [], packs: { buy: [], cost: 0, bought: 0 }, leftover: 0, wasteCost: 0, status: 'buy' };
        lines.set(id, line);
      }
      line.need += qty;
      line.usedIn.push({ recipeId: recipe.id, title: recipe.title, qty });
    }
  }
  let total = 0, waste = 0;
  for (const line of lines.values()) {
    const ing = line.ingredient;
    line.status = input.have.has(ing.id) ? 'have' : ing.pantry && !input.includePantry && !input.pantryToBuy?.has(ing.id) ? 'pantry' : 'buy';
    line.packs = bestPacks(line.need, ing.packs);
    line.leftover = Math.max(0, line.packs.bought - line.need);
    line.wasteCost = ing.perishable && line.packs.bought > 0 ? (line.leftover / line.packs.bought) * line.packs.cost : 0;
    if (line.status === 'buy') { total += line.packs.cost; waste += line.wasteCost; }
  }
  const sorted = [...lines.values()].sort((a, b) =>
    AISLE_ORDER.indexOf(a.ingredient.aisle) - AISLE_ORDER.indexOf(b.ingredient.aisle) || a.ingredient.name.localeCompare(b.ingredient.name));
  const separate = perRecipe.reduce((s, d) => s + perishableCost(d, cat), 0);
  const combined = perishableCost(mergeDemand(...perRecipe), cat);
  return {
    lines: sorted,
    total: Math.round(total * 100) / 100,
    waste: Math.round(waste * 100) / 100,
    shared: sorted.filter(l => l.ingredient.perishable && new Set(l.usedIn.map(u => u.recipeId)).size > 1),
    groupingSaving: Math.max(0, Math.round((separate - combined) * 100) / 100),
  };
}
