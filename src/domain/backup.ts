import { MARKETS } from './market';
import { EMPTY_STATE, type AppState } from './kitchen';

export function readBackup(text: string): AppState {
  const parsed = JSON.parse(text);
  const next = (parsed?.state ?? parsed) as AppState;
  const strings = (x: unknown): x is string[] => Array.isArray(x) && x.every(v => typeof v === 'string');
  const object = (x: unknown) => !!x && typeof x === 'object' && !Array.isArray(x);
  const invalid = () => { throw new Error('This file is not a complete MSTRCHF 9000 backup. Your kitchen has not changed.'); };
  if (!object(next) || next.version !== 2 || !object(next.weeks) || !('profile' in next)) return invalid();
  if (parsed?.app && parsed.app !== 'MSTRCHF 9000') return invalid();
  const p = next.profile;
  if (p !== null && (!object(p) || !Number.isInteger(p.people) || p.people < 1 || p.people > 8
    || !['omnivore', 'vegetarian', 'vegan', 'pescatarian'].includes(p.diet)
    || !strings(p.avoid) || !strings(p.dislikes) || !object(p.meals) || !object(p.cuisines) || !object(p.proteins)
    || ![...Object.values(p.cuisines), ...Object.values(p.proteins)].every(n => [-1, 0, 1, 2].includes(n))
    || ![p.meals.dinner, p.meals.lunch, p.meals.breakfast].every(n => Number.isInteger(n) && n >= 0 && n <= 7)
    || ![0, 1, 2, 3].includes(p.spice) || !Number.isFinite(p.maxTime)
    || !['relaxed', 'balanced', 'tight'].includes(p.budget) || (p.market !== undefined && !Object.hasOwn(MARKETS, p.market)))) return invalid();
  if (![next.favourites, next.hidden, next.cupboard].every(x => x === undefined || strings(x))
    || (next.ratings !== undefined && (!object(next.ratings) || !Object.values(next.ratings).every(n => [-1, 0, 1].includes(n))))) return invalid();
  for (const [id, w] of Object.entries(next.weeks)) {
    if (!/^\d{4}-\d{2}-\d{2}$/.test(id) || !object(w) || w.id !== id || !Array.isArray(w.picks)
      || !['draft', 'ordered'].includes(w.status) || !strings(w.have) || typeof w.craving !== 'string' || !Number.isFinite(w.seed)
      || (w.pantryToBuy !== undefined && !strings(w.pantryToBuy))
      || !object(w.focus) || !strings(w.focus.tags) || !strings(w.focus.ingredients) || !strings(w.focus.cuisines)
      || !w.picks.every(pick => object(pick) && typeof pick.uid === 'string' && typeof pick.recipeId === 'string'
        && ['dinner', 'lunch', 'breakfast'].includes(pick.meal) && Number.isInteger(pick.day) && pick.day >= 0 && pick.day <= 6 && strings(pick.reasons)
        && (pick.locked === undefined || typeof pick.locked === 'boolean'))) return invalid();
    const b = w.orderedBasket;
    if (b && (b.app !== 'MSTRCHF 9000' || b.format !== 'basket' || b.version !== 2 || !Object.hasOwn(MARKETS, b.market)
      || b.currency !== MARKETS[b.market].currency || b.weekId !== id || typeof b.week !== 'string' || typeof b.createdAt !== 'string'
      || !Number.isFinite(b.estTotal) || !Number.isInteger(b.people) || b.people < 1 || b.people > 8
      || (b.retailerName !== null && typeof b.retailerName !== 'string')
      || !strings(b.meals) || !strings(b.alreadyHave) || !Array.isArray(b.items)
      || !b.items.every(i => object(i) && typeof i.id === 'string' && typeof i.name === 'string' && typeof i.aisle === 'string'
        && typeof i.need === 'string' && typeof i.search === 'string' && Number.isFinite(i.estPrice) && strings(i.usedIn)
        && Array.isArray(i.buy) && i.buy.every(pack => object(pack) && typeof pack.size === 'string' && Number.isInteger(pack.count) && pack.count > 0)))) return invalid();
  }
  return { ...EMPTY_STATE, ...next };
}
