import { usLiquidPack, usVolume, usWeight } from './localize';
import { activeMarket } from './market';
import type { Ingredient } from './types';

const FRACTIONS: [number, string][] = [[0.25, '¼'], [0.5, '½'], [0.75, '¾'], [1 / 3, '⅓'], [2 / 3, '⅔']];

/** 1.5 → "1½", 0.25 → "¼". Rounds to the nearest quarter or third. */
export function fraction(n: number): string {
  if (n <= 0) return '0';
  const whole = Math.floor(n + 1e-9);
  const rest = n - whole;
  if (rest < 0.08) return String(whole || (n > 0 ? '¼' : '0'));
  if (rest > 0.92) return String(whole + 1);
  let best = FRACTIONS[0];
  for (const f of FRACTIONS) if (Math.abs(f[0] - rest) < Math.abs(best[0] - rest)) best = f;
  return whole ? `${whole}${best[1]}` : best[1];
}

function round(n: number): number {
  if (n >= 100) return Math.round(n / 5) * 5;
  if (n >= 20) return Math.round(n);
  return Math.round(n * 2) / 2;
}

/** Friendly quantity for a recipe card: "2 tsp", "1½ tbsp", "250 g" or "9 oz", "1¼ cups", "2". */
export function recipeQty(qty: number, ing: Ingredient): string {
  if (ing.tsp && qty / ing.tsp <= 9) {
    const tsp = qty / ing.tsp;
    if (tsp >= 3 && Math.abs(tsp / 3 - Math.round(tsp / 3 * 2) / 2) < 0.2) return `${fraction(tsp / 3)} tbsp`;
    return `${fraction(tsp)} tsp`;
  }
  if (activeMarket().measures === 'us' && ing.unit !== 'each') return ing.unit === 'g' ? usWeight(qty) : usVolume(qty);
  return metricAmount(qty, ing.unit);
}

function metricAmount(qty: number, unit: Ingredient['unit']): string {
  if (unit === 'each') return fraction(qty);
  const ca = activeMarket().id === 'ca';
  const small = unit === 'ml' && ca ? 'mL' : unit;
  const big = unit === 'g' ? 'kg' : 'L';
  if (qty >= 1000) return `${(qty / 1000).toFixed(qty % 1000 === 0 ? 0 : 1)} ${big}`;
  return `${round(qty)} ${small}`;
}

/** Shopping amount in the country's pack units: "250 g", "1.2 kg", "300 ml", "1 lb", "32 fl oz", "3". */
export function amount(qty: number, unit: Ingredient['unit']): string {
  if (unit !== 'each' && activeMarket().packs === 'us') return unit === 'g' ? usWeight(qty) : usLiquidPack(qty);
  return metricAmount(qty, unit);
}

export function money(n: number): string {
  return `${activeMarket().symbol}${n.toFixed(2)}`;
}

/** "£5", "£4.50": no pence when the amount is whole. */
export function shortMoney(n: number): string {
  return `${activeMarket().symbol}${Number.isInteger(n) ? n : n.toFixed(2)}`;
}

const PROPER = /^(Greek|Italian|Thai|Chinese|Dijon|Tenderstem|Worcestershire|English|Puy|Parmesan)/;

/** Ingredient name for use mid-sentence: "fresh coriander", "Greek-style yoghurt", without pack notes. */
export function inlineName(name: string): string {
  const clean = name.replace(/\s*\(.*?\)\s*/g, ' ').trim();
  return PROPER.test(clean) ? clean : clean[0].toLowerCase() + clean.slice(1);
}

/** "2 loose", "2 tins", "1 × pack of 8", "1 × 500 g". */
export function packLabel(size: number, count: number, ing: Ingredient): string {
  if (ing.unit === 'each') {
    if (size > 1) return `${count} × pack of ${size}`;
    if (ing.aisle === 'tins') {
      const word = activeMarket().id === 'uk' ? 'tin' : 'can';
      return `${count} ${word}${count > 1 ? 's' : ''}`;
    }
    if (ing.aisle === 'veg' || ing.aisle === 'fruit') return `${count} loose`;
    return String(count);
  }
  return `${count} × ${amount(size, ing.unit)}`;
}

/** Name with simple pluralisation for counted items: "2 red peppers". */
export function countedName(ing: Ingredient, qty: number): string {
  if (ing.unit !== 'each' || qty <= 1) return ing.name;
  const n = ing.name;
  if (/\(.*\)$/.test(n)) return n; // "Chickpeas (400g tin)"
  if (/[^s]s$/i.test(n)) return n; // already plural: "Spring onions", "Flour tortilla wraps"
  if (/(ch|sh|ss|x)$/i.test(n)) return `${n}es`;
  if (/[^aeiou]y$/i.test(n)) return `${n.slice(0, -1)}ies`;
  if (/o$/i.test(n)) return `${n}es`;
  return `${n}s`;
}
