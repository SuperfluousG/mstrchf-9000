// Countries the planner supports. The app is British first; the US and Canada get their own
// ingredient names, measures, pack sizes and prices so the same recipes work there.

export type Market = 'uk' | 'us' | 'ca';

export interface MarketInfo {
  id: Market;
  label: string;
  /** For use mid-sentence: "the UK". */
  short: string;
  currency: 'GBP' | 'USD' | 'CAD';
  symbol: string;
  /** Recipe amounts and oven temperatures: grams, ml and °C, or cups, oz and °F. */
  measures: 'metric' | 'us';
  /** Shop pack sizes: metric (500 g, 1 L) or US (1 lb, 32 fl oz). */
  packs: 'metric' | 'us';
  /** What shoppers call it online: basket in the UK, cart in North America. */
  basket: 'basket' | 'cart';
  /** Local money per pound sterling, roughly. The planner's waste and sharing weights are tuned in pounds. */
  perGbp: number;
}

export const MARKETS: Record<Market, MarketInfo> = {
  uk: { id: 'uk', label: 'United Kingdom', short: 'the UK', currency: 'GBP', symbol: '£', measures: 'metric', packs: 'metric', basket: 'basket', perGbp: 1 },
  us: { id: 'us', label: 'United States', short: 'the US', currency: 'USD', symbol: '$', measures: 'us', packs: 'us', basket: 'cart', perGbp: 1.3 },
  ca: { id: 'ca', label: 'Canada', short: 'Canada', currency: 'CAD', symbol: '$', measures: 'us', packs: 'metric', basket: 'cart', perGbp: 1.8 },
};

export const MARKET_ORDER: Market[] = ['uk', 'us', 'ca'];

export interface Retailer {
  id: string;
  name: string;
  market: Market;
  /** Other banners the same online shop covers. */
  covers?: string;
  /** Delivery charge on small baskets, as the shop states it. */
  minBasket?: { under: number; charge: number };
}

/**
 * Leading online grocers per country. Ids match the guides in the plugin's grocery-basket skill.
 * Amazon Fresh is left out on purpose: Amazon's Conditions of Use bar AI agents, so Claude won't fill its basket.
 */
export const RETAILERS: Retailer[] = [
  { id: 'tesco', name: 'Tesco', market: 'uk', minBasket: { under: 50, charge: 5 } },
  { id: 'sainsburys', name: "Sainsbury's", market: 'uk' },
  { id: 'asda', name: 'Asda', market: 'uk' },
  { id: 'ocado', name: 'Ocado', market: 'uk' },
  { id: 'morrisons', name: 'Morrisons', market: 'uk' },
  { id: 'walmart', name: 'Walmart', market: 'us' },
  { id: 'kroger', name: 'Kroger', market: 'us' },
  { id: 'target', name: 'Target', market: 'us' },
  { id: 'albertsons', name: 'Albertsons', market: 'us', covers: 'Safeway, Vons, Jewel-Osco' },
  { id: 'pc-express', name: 'PC Express', market: 'ca', covers: 'Loblaws, Real Canadian Superstore, No Frills' },
  { id: 'walmart-ca', name: 'Walmart Canada', market: 'ca' },
  { id: 'voila', name: 'Voilà', market: 'ca', covers: 'Sobeys, Safeway, IGA' },
  { id: 'metro', name: 'Metro', market: 'ca' },
];

export function retailersFor(m: Market): Retailer[] {
  return RETAILERS.filter(r => r.market === m);
}

export function retailerById(id: string | null | undefined): Retailer | undefined {
  return id ? RETAILERS.find(r => r.id === id) : undefined;
}

/** Best guess from the browser language: en-US → US, en-CA or fr-CA → Canada, anything else → UK. */
export function detectMarket(lang?: string): Market {
  const l = (lang ?? (typeof navigator !== 'undefined' ? navigator.language : '') ?? '').toLowerCase();
  if (/-us\b/.test(l)) return 'us';
  if (/-ca\b/.test(l)) return 'ca';
  return 'uk';
}

let active: Market = 'uk';

/** The country the app is currently showing. Set by `selectMarket` in data.ts. */
export function activeMarket(): MarketInfo {
  return MARKETS[active];
}

export function setActiveMarket(m: Market): void {
  active = MARKETS[m] ? m : 'uk';
}
