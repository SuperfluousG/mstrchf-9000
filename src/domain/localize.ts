// Turns the British source catalogue into each country's version.
//
// Recipe text (title, blurb, prep notes, steps) may contain placeholders in braces:
//   {oven 220}          oven temperature in °C → "220°C (200°C fan)" or "425°F"
//   {temp 180}          any other temperature in °C → "180°C" or "350°F"
//   {ml 300}            a volume → "300 ml" or "1¼ cups"
//   {g 50}              a weight → "50 g" or "1¾ oz"
//   {cm 2}              a length → "2 cm" or "¾ inch"
//   {courgette}         a food or kitchen word looked up in catalogue/terms.json → "zucchini"
//   {grill|broil}       an explicit British|American pair, for words whose grammar changes
// British text is the source; the US and Canada share American wording unless a term says otherwise.

import type { Market } from './market';
import { MARKETS } from './market';
import type { Ingredient, Recipe } from './types';

export interface TermEntry {
  us: string;
  /** Canadian word, when it differs from the American one. */
  ca?: string;
  /** Regex source for British phrases that contain this word but must stay as they are (e.g. "ground coriander"). */
  keep?: string;
  /** Only a British-only word in recipes that use this ingredient id (plain "coriander" otherwise means the ground spice). */
  onlyWith?: string;
}
export type Terms = Record<string, TermEntry>;

export interface MarketIngredient { name?: string; search?: string; packs: [number, number][] }
export interface MarketFile { market: Market; currency: string; ingredients: Record<string, MarketIngredient> }

const FRACTIONS: [number, string][] = [[0.25, '¼'], [1 / 3, '⅓'], [0.5, '½'], [2 / 3, '⅔'], [0.75, '¾']];

/** 1.25 → "1¼", rounded to the nearest quarter or third; never shows 0 for a positive amount. */
export function nice(n: number): string {
  if (n <= 0) return '0';
  let whole = Math.floor(n + 1e-9);
  let rest = n - whole;
  if (rest > 0.88) { whole += 1; rest = 0; }
  if (rest < 0.12) return whole ? String(whole) : '¼';
  let best = FRACTIONS[0];
  for (const f of FRACTIONS) if (Math.abs(f[0] - rest) < Math.abs(best[0] - rest)) best = f;
  return whole ? `${whole}${best[1]}` : best[1];
}

export function toFahrenheit(c: number): number {
  return Math.round((c * 9 / 5 + 32) / 25) * 25;
}

/** Rounds to a multiple of `step` (¼ or ½) and shows it with fraction glyphs. */
function spoons(n: number, step: number): string {
  return nice(Math.max(step, Math.round(n / step) * step));
}

/** Kitchen volume in US measures: "1 tsp", "1½ tbsp", "⅓ cup", "1¼ cups". */
export function usVolume(ml: number): string {
  if (ml < 12.5) return `${spoons(ml / 4.93, 0.25)} tsp`;
  if (ml < 55) return `${spoons(ml / 14.79, 0.5)} tbsp`;
  const cups = ml / 236.6;
  const text = nice(cups);
  return `${text} ${cups > 1.12 ? 'cups' : 'cup'}`;
}

/** Kitchen weight in US measures: "½ oz", "10 oz", "1¼ lb". */
export function usWeight(g: number): string {
  const oz = g / 28.35;
  if (oz < 15.5) return `${oz < 3 ? nice(oz) : Math.round(oz)} oz`;
  return `${nice(Math.round(g / 453.6 * 4) / 4)} lb`;
}

/** Shop volume in US measures: "13.5 fl oz", "1 qt", "½ gal". */
export function usLiquidPack(ml: number): string {
  const floz = ml / 29.57;
  if (floz < 31) return `${floz < 10 ? Math.round(floz * 2) / 2 : Math.round(floz)} fl oz`;
  if (floz < 63) return `${nice(Math.round(floz / 32 * 2) / 2)} qt`;
  return `${nice(Math.round(floz / 128 * 4) / 4)} gal`;
}

export function usLength(cm: number): string {
  const inches = cm / 2.54;
  const text = nice(Math.max(0.25, inches));
  return `${text} ${inches > 1.12 ? 'inches' : 'inch'}`;
}

function matchCase(source: string, out: string): string {
  return /^[A-Z]/.test(source) ? out[0].toUpperCase() + out.slice(1) : out;
}

/** Replace every {placeholder} for the given country. Unknown placeholders lose their braces. */
export function renderText(text: string, market: Market, terms: Terms, factor = 1): string {
  const us = MARKETS[market].measures === 'us';
  return text.replace(/\{([^{}]+)\}/g, (_, raw: string) => {
    const inner = raw.trim();
    const count = /^qty (\d+(?:\.\d+)?)(?:\|([^|]+)\|([^|]+))?$/.exec(inner);
    if (count) {
      const n = Number(count[1]) * factor;
      const quantity = n > 0 && n < 0.2 ? '⅛' : nice(n);
      return count[2] ? `${quantity} ${n <= 1 ? count[2] : count[3]}` : quantity;
    }
    const m = /^(oven|temp|ml|g|cm) (\d+(?:\.\d+)?)$/.exec(inner);
    if (m) {
      const n = Number(m[2]) * (m[1] === 'ml' || m[1] === 'g' ? factor : 1);
      switch (m[1]) {
        case 'oven': return us ? `${toFahrenheit(n)}°F` : `${n}°C (${n - 20}°C fan)`;
        case 'temp': return us ? `${toFahrenheit(n)}°F` : `${n}°C`;
        case 'ml': return us ? usVolume(n) : `${n} ml`;
        case 'g': return us ? usWeight(n) : `${n} g`;
        case 'cm': return us ? usLength(n) : `${n} cm`;
      }
    }
    const pair = inner.split('|');
    if (pair.length === 2) {
      // Explicit spoon equivalents (e.g. {15 g|1 tbsp}) are quantities too.
      const side = market === 'uk' ? pair[0] : pair[1];
      const measure = /^(\d+(?:\.\d+)?(?:[¼½¾⅓⅔])?|[¼½¾⅓⅔]) (g|ml|tsp|tbsp|cup|cups)$/.exec(side);
      if (measure && factor !== 1) {
        const glyph: Record<string, number> = { '¼': .25, '½': .5, '¾': .75, '⅓': 1 / 3, '⅔': 2 / 3 };
        const number = Number(measure[1].replace(/[¼½¾⅓⅔]/, '')) + (glyph[measure[1].slice(-1)] ?? 0);
        const n = number * factor;
        return `${nice(n)} ${measure[2].startsWith('cup') ? (n > 1 ? 'cups' : 'cup') : measure[2]}`;
      }
      return side;
    }
    if (market === 'uk') return inner;
    const t = terms[inner.toLowerCase()];
    if (!t) return inner;
    return matchCase(inner, market === 'ca' ? (t.ca ?? t.us) : t.us);
  });
}

/** Placeholders in a piece of text that `renderText` cannot resolve. */
export function badPlaceholders(text: string, terms: Terms): string[] {
  const bad: string[] = [];
  for (const [, raw] of text.matchAll(/\{([^{}]*)\}/g)) {
    const inner = raw.trim();
    if (/^(oven|temp|ml|g|cm) \d+(?:\.\d+)?$/.test(inner) || /^qty \d+(?:\.\d+)?(?:\|[^|]+\|[^|]+)?$/.test(inner)) continue;
    const pair = inner.split('|');
    if (pair.length === 2 && pair[0].trim() && pair[1].trim()) continue;
    if (pair.length === 1 && terms[inner.toLowerCase()]) continue;
    bad.push(`{${raw}}`);
  }
  if (/[{}]/.test(text.replace(/\{[^{}]*\}/g, ''))) bad.push('unmatched brace');
  return bad;
}

export function localIngredient(ing: Ingredient, market: Market, files: Partial<Record<Market, MarketFile>>): Ingredient {
  if (market === 'uk') return ing;
  const us = files.us?.ingredients[ing.id];
  const own = files[market]?.ingredients[ing.id];
  if (!own) return ing;
  const ca = market === 'ca' ? own : undefined;
  return {
    ...ing,
    name: ca?.name ?? us?.name ?? ing.name,
    search: ca?.search ?? us?.search ?? (us?.name ? us.name.toLowerCase().replace(/\s*\(.*?\)/g, '') : ing.search),
    packs: own.packs,
  };
}

export function localRecipe(r: Recipe, market: Market, terms: Terms, people = r.servings): Recipe {
  const t = (s: string) => renderText(s, market, terms);
  return {
    ...r,
    title: t(r.title),
    blurb: t(r.blurb),
    steps: r.steps.map(s => renderText(s, market, terms, people / r.servings)),
    ingredients: r.ingredients.map(ri => (ri.prep ? { ...ri, prep: t(ri.prep) } : ri)),
  };
}
