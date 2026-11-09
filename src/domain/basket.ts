import { amount, money } from './format';
import { AISLE_LABELS, type ShoppingList } from './shopping';
import { weekLabel } from './weeks';

export interface BasketItem {
  id: string;
  name: string;
  search: string;
  aisle: string;
  need: string;
  /** Packs to buy, e.g. [{ size: "3", count: 1 }]. */
  buy: { size: string; count: number }[];
  estPrice: number;
  usedIn: string[];
}

export interface BasketFile {
  app: 'MSTRCHF 9000';
  format: 'basket';
  version: 1;
  retailer: 'tesco';
  weekId: string;
  week: string;
  people: number;
  meals: string[];
  items: BasketItem[];
  alreadyHave: string[];
  estTotal: number;
  createdAt: string;
}

export function buildBasket(list: ShoppingList, weekId: string, people: number, meals: string[]): BasketFile {
  const items = list.lines.filter(l => l.status === 'buy').map(l => ({
    id: l.ingredient.id,
    name: l.ingredient.name,
    search: l.ingredient.search,
    aisle: AISLE_LABELS[l.ingredient.aisle],
    need: amount(l.need, l.ingredient.unit),
    buy: l.packs.buy.map(p => ({ size: l.ingredient.unit === 'each' ? (p.size === 1 ? 'single' : `pack of ${p.size}`) : amount(p.size, l.ingredient.unit), count: p.count })),
    estPrice: l.packs.cost,
    usedIn: [...new Set(l.usedIn.map(u => u.title))],
  }));
  return {
    app: 'MSTRCHF 9000', format: 'basket', version: 1, retailer: 'tesco',
    weekId, week: weekLabel(weekId), people, meals,
    items,
    alreadyHave: list.lines.filter(l => l.status !== 'buy').map(l => l.ingredient.name),
    estTotal: list.total,
    createdAt: new Date().toISOString(),
  };
}

export function basketText(b: BasketFile): string {
  const lines = [`MSTRCHF 9000 — shopping for ${b.week} (${b.people} people)`, '', `Meals: ${b.meals.join(', ')}`, ''];
  let aisle = '';
  for (const i of b.items) {
    if (i.aisle !== aisle) { aisle = i.aisle; lines.push('', `${aisle}:`); }
    const packs = i.buy.map(p => p.size === 'single' ? `${p.count}` : `${p.count} × ${p.size}`).join(' + ');
    lines.push(`[ ] ${i.name} — need ${i.need}; buy ${packs} (est. ${money(i.estPrice)})`);
  }
  lines.push('', `Estimated total: ${money(b.estTotal)}`);
  return lines.join('\n');
}

/** Instructions a browser-capable Claude can follow to fill the Tesco basket. */
export function claudePrompt(b: BasketFile): string {
  return [
    'Please fill my Tesco groceries basket for this MSTRCHF 9000 meal plan, using the tesco-basket skill if you have it.',
    '',
    'Rules:',
    '- Use the Tesco tab I am already signed in to. Do not sign in, enter payment details or check out.',
    '- Leave anything already in my basket alone.',
    '- For each item, search the given phrase, pick the best-value product that matches the pack size (own-brand is fine), and add the stated count.',
    '- If an item is unavailable or only a very different size exists, pick the closest sensible substitute and tell me.',
    '- When done, show me the basket total and a list of what you added, substituted or skipped. Then ask me before booking a delivery slot.',
    '',
    'Items (JSON):',
    JSON.stringify(b.items.map(i => ({ name: i.name, search: i.search, buy: i.buy, need: i.need, estPrice: i.estPrice })), null, 1),
  ].join('\n');
}
