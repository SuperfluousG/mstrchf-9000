import { amount, money } from './format';
import { type Market, type MarketInfo, MARKETS, type Retailer } from './market';
import { aisleLabel, type ShoppingList } from './shopping';
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

/** Version 2 adds the country and shop. Names, amounts and prices are already in that country's words and units. */
export interface BasketFile {
  app: 'MSTRCHF 9000';
  format: 'basket';
  version: 2;
  market: Market;
  /** Shop id from RETAILERS, or null when the user shops somewhere else. */
  retailer: string | null;
  retailerName: string | null;
  currency: MarketInfo['currency'];
  weekId: string;
  week: string;
  people: number;
  meals: string[];
  items: BasketItem[];
  alreadyHave: string[];
  estTotal: number;
  createdAt: string;
}

export function buildBasket(list: ShoppingList, weekId: string, people: number, meals: string[], market: MarketInfo, retailer?: Retailer): BasketFile {
  const items = list.lines.filter(l => l.status === 'buy').map(l => ({
    id: l.ingredient.id,
    name: l.ingredient.name,
    search: l.ingredient.search,
    aisle: aisleLabel(l.ingredient.aisle),
    need: amount(l.need, l.ingredient.unit),
    buy: l.packs.buy.map(p => ({ size: l.ingredient.unit === 'each' ? (p.size === 1 ? 'single' : `pack of ${p.size}`) : amount(p.size, l.ingredient.unit), count: p.count })),
    estPrice: l.packs.cost,
    usedIn: [...new Set(l.usedIn.map(u => u.title))],
  }));
  return {
    app: 'MSTRCHF 9000', format: 'basket', version: 2,
    market: market.id, retailer: retailer?.id ?? null, retailerName: retailer?.name ?? null, currency: market.currency,
    weekId, week: weekLabel(weekId), people, meals,
    items,
    alreadyHave: list.lines.filter(l => l.status !== 'buy').map(l => l.ingredient.name),
    estTotal: list.total,
    createdAt: new Date().toISOString(),
  };
}

export function basketText(b: BasketFile): string {
  const price = (n: number) => money(n, MARKETS[b.market]);
  const lines = [`MSTRCHF 9000 — shopping for ${b.week} (${b.people} people)`, '', `Meals: ${b.meals.join(', ')}`, ''];
  let aisle = '';
  for (const i of b.items) {
    if (i.aisle !== aisle) { aisle = i.aisle; lines.push('', `${aisle}:`); }
    const packs = i.buy.map(p => p.size === 'single' ? `${p.count}` : `${p.count} × ${p.size}`).join(' + ');
    lines.push(`[ ] ${i.name} — need ${i.need}; buy ${packs} (est. ${price(i.estPrice)})`);
  }
  lines.push('', `Estimated total: ${price(b.estTotal)}`);
  return lines.join('\n');
}

/** A self-contained shopping handoff, using the exact list reviewed in the planner. */
export function claudePrompt(b: BasketFile, assistant = 'Claude'): string {
  const word = MARKETS[b.market]?.basket ?? 'basket';
  const shop = b.retailerName ?? 'online supermarket';
  const browserRules = assistant === 'ChatGPT' ? [
    "- Use ChatGPT’s built-in browser inside the desktop app (@Browser).",
    b.retailerName
      ? `- Open the ${b.retailerName} online grocery site in a visible browser tab beside this chat. Reuse an existing ${b.retailerName} grocery tab only if it is in that built-in browser.`
      : '- Ask me which online supermarket to use, then open its grocery site in a visible browser tab beside this chat. Reuse an existing grocery tab only if it is in that built-in browser.',
    '- Do not use Chrome, Edge, an external browser, or a browser extension, even if already connected or signed in.',
    '- If sign-in is needed, show the retailer’s sign-in page in the built-in browser and ask me to sign in there myself. Keep the tab open and resume after I hand control back.',
    '- If the built-in browser is unavailable, explain that and stop rather than switching browsers.',
  ] : [
    b.retailerName
      ? `- Open your browser and navigate to the ${b.retailerName} online grocery site. Reuse my existing ${b.retailerName} grocery tab if available.`
      : '- Ask me which online supermarket to use, then open your browser and navigate to its grocery site.',
    '- If login is needed, open the site first and ask me to sign in there. Continue filling the basket after I hand control back. Do not require Chrome or a separate browser setup.',
  ];
  return [
    `Please fill my ${shop} grocery ${word} for this MSTRCHF 9000 meal plan, using the grocery-basket skill if you have it in ${assistant}.`,
    '',
    'Rules:',
    ...browserRules,
    '- Do not sign in, enter payment details or check out. Never enter passwords or create accounts on my behalf.',
    '- Work in one tab, one item at a time. If the site shows a security check or CAPTCHA, stop and hand back to me.',
    `- Leave anything already in my ${word} alone.`,
    "- For each item, search the given phrase, pick the best-value product that matches the pack size (the shop's own brand is fine), and add the stated count.",
    '- If an item is unavailable or only a very different size exists, pick the closest sensible substitute and tell me.',
    `- When done, show me the ${word} total and a list of what you added, substituted or skipped. Then ask me before booking a delivery slot.`,
    '',
    `Basket (${b.market.toUpperCase()}, prices in ${b.currency}, JSON):`,
    JSON.stringify({ weekId: b.weekId, people: b.people, market: b.market, retailer: b.retailer, retailerName: b.retailerName, items: b.items.map(i => ({ name: i.name, search: i.search, buy: i.buy, need: i.need, estPrice: i.estPrice })) }, null, 1),
  ].join('\n');
}
