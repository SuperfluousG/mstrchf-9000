import { suitsDiet, type Catalogue } from './catalogue';
import { inlineName } from './format';
import { CUISINE_LABELS, cuisineAffinity, type Profile } from './profile';
import { mergeDemand, perishableCost, perishableWaste, recipeDemand, type Demand } from './shopping';
import type { Cuisine, MealType, Recipe } from './types';

export type Rating = 1 | 0 | -1;

/** This week's cravings: "spicy", "peppers", "Thai". */
export interface Focus {
  tags: string[];
  ingredients: string[];
  cuisines: Cuisine[];
}

export const EMPTY_FOCUS: Focus = { tags: [], ingredients: [], cuisines: [] };

export const FOCUS_TAGS: { id: string; label: string }[] = [
  { id: 'spicy', label: 'Spicy' },
  { id: 'comfort', label: 'Comfort food' },
  { id: 'fresh', label: 'Fresh & light' },
  { id: 'quick', label: 'Quick' },
  { id: 'fakeaway', label: 'Fakeaway' },
  { id: 'veggie', label: 'More veggie' },
  { id: 'high-protein', label: 'High protein' },
  { id: 'whole', label: 'Whole foods' },
  { id: 'new', label: 'Something new' },
  { id: 'budget', label: 'Budget' },
];

export interface PlanContext {
  catalogue: Catalogue;
  profile: Profile;
  /** recipeId → distance in weeks to the nearest other week it appears in. */
  recency: Map<string, number>;
  favourites: Set<string>;
  ratings: Map<string, Rating>;
  hidden: Set<string>;
  focus: Focus;
  /** Changes the tie-breaking jitter so "shuffle" gives a different but still sensible week. */
  seed: number;
}

export interface Scored { recipe: Recipe; score: number; reasons: string[] }

export interface PlannedPick {
  recipeId: string;
  meal: MealType;
  day: number;
  locked?: boolean;
  reasons: string[];
}

// ---------- eligibility ----------

/** Dietary exclusions always apply, including to meals the user has kept. */
export function exclusionReason(r: Recipe, ctx: PlanContext): string | null {
  const f = ctx.catalogue.facts.get(r.id);
  if (!f) return 'Unknown recipe';
  const p = ctx.profile;
  if (!suitsDiet(f, p.diet)) return `Not ${p.diet}`;
  for (const a of p.avoid) if (f.allergens.has(a)) return `Contains ${a === 'milk' ? 'dairy' : a === 'nuts' ? 'tree nuts' : a}`;
  for (const d of p.dislikes) if (f.ingredientIds.has(d)) return `Contains ${inlineName(ctx.catalogue.ingredients.get(d)?.name ?? d)}`;
  return null;
}

export function whyNot(r: Recipe, ctx: PlanContext): string | null {
  const excluded = exclusionReason(r, ctx);
  if (excluded) return excluded;
  const p = ctx.profile;
  if (ctx.hidden.has(r.id)) return 'You hid this recipe';
  if (cuisineAffinity(p, r.cuisine) === -1 && !ctx.focus.cuisines.includes(r.cuisine)) return `You skip ${CUISINE_LABELS[r.cuisine]}`;
  if ((p.proteins[r.protein] ?? 0) === -1) return `You skip ${r.protein}`;
  return null;
}

// ---------- scoring ----------

function hash(s: string): number {
  let h = 2166136261;
  for (let i = 0; i < s.length; i++) { h ^= s.charCodeAt(i); h = Math.imul(h, 16777619); }
  return h >>> 0;
}

function jitter(seed: number, id: string): number {
  let t = (hash(id) ^ Math.imul(seed + 1, 2654435761)) >>> 0;
  t = Math.imul(t ^ (t >>> 15), t | 1);
  t ^= t + Math.imul(t ^ (t >>> 7), t | 61);
  return ((t ^ (t >>> 14)) >>> 0) / 4294967296;
}

function matchesFocusTag(tag: string, r: Recipe, ctx: PlanContext): boolean {
  const f = ctx.catalogue.facts.get(r.id)!;
  switch (tag) {
    case 'spicy': return r.spice >= 2;
    case 'veggie': return f.vegetarian;
    case 'quick': return r.time <= 25 || r.tags.includes('quick');
    case 'new': return !ctx.recency.has(r.id) && !ctx.ratings.has(r.id);
    case 'fresh': return r.tags.includes('fresh') || r.tags.includes('light');
    // Comfort is a dinner mood; lunches stay light.
    case 'comfort': return r.meal === 'dinner' && (r.tags.includes('comfort') || r.tags.includes('hearty'));
    case 'whole': return false; // scored separately
    default: return r.tags.includes(tag);
  }
}

const FOCUS_REASON: Record<string, string> = {
  spicy: 'Brings the heat you asked for', comfort: 'Comfort food, as requested', fresh: 'Fresh and light, as requested',
  quick: 'Quick, as requested', fakeaway: 'Fakeaway night', veggie: 'Veggie, as requested',
  'high-protein': 'High protein', new: 'Something new', budget: 'Budget-friendly',
};

export function scoreRecipe(r: Recipe, ctx: PlanContext): Scored {
  const p = ctx.profile;
  const reasons: [number, string][] = [];
  let s = 5;
  const add = (v: number, why?: string) => { s += v; if (why && v > 0) reasons.push([v, why]); };

  const ca = cuisineAffinity(p, r.cuisine);
  if (ca === 2) add(2, `You love ${CUISINE_LABELS[r.cuisine]} food`);
  else if (ca === 1) add(1, `${CUISINE_LABELS[r.cuisine]}, a style you like`);
  const pa = p.proteins[r.protein] ?? 0;
  if (pa === 2) add(1.5); else if (pa === 1) add(0.75);

  const diff = r.spice - p.spice;
  if (diff > 0) add(-1.6 * diff);
  else if (diff < 0) add(-0.4 * -diff);
  if (p.spice >= 2 && r.spice >= 2) add(1, r.spice === 3 ? 'Proper heat' : 'Nicely spicy');

  if (r.time > p.maxTime) add(-0.12 * (r.time - p.maxTime));
  else if (r.time <= 25) add(0.3, `Ready in ${r.time} minutes`);

  if (ctx.favourites.has(r.id)) add(1.5, 'One of your favourites');
  const rating = ctx.ratings.get(r.id);
  if (rating === 1) add(1, 'You loved this last time');
  if (rating === -1) add(-3);

  const weeks = ctx.recency.get(r.id);
  if (weeks === undefined) { if (rating === undefined) add(0.2, 'New to you'); }
  else if (weeks <= 1) add(-4);
  else if (weeks <= 2) add(-2);
  else if (weeks <= 4) add(-0.75);
  else add(0.3, 'Not had it for a while');

  for (const tag of ctx.focus.tags) if (matchesFocusTag(tag, r, ctx)) add(2, FOCUS_REASON[tag] ?? `Matches “${tag}”`);
  if (ctx.focus.cuisines.includes(r.cuisine)) add(2.5, `${CUISINE_LABELS[r.cuisine]}, as requested`);
  const f = ctx.catalogue.facts.get(r.id)!;
  const hits = ctx.focus.ingredients.filter(i => f.ingredientIds.has(i));
  if (hits.length) add(Math.min(4, 2 * hits.length), `Built around ${hits.map(i => inlineName(ctx.catalogue.ingredients.get(i)?.name ?? i)).join(' and ')}`);

  if (ctx.focus.tags.includes('whole')) {
    if (f.processed.length === 0) add(1.5, 'Whole foods, nothing ready-made');
    else add(-Math.min(3, 0.8 * f.processed.length));
  }
  if (r.meal === 'lunch' && (r.tags.includes('light') || r.tags.includes('fresh') || r.tags.includes('lunchbox'))) add(0.6);

  if (p.budget === 'tight' && r.tags.includes('budget')) add(1, 'Budget-friendly');
  add(jitter(ctx.seed, r.id) * 1.4);

  reasons.sort((a, b) => b[0] - a[0]);
  return { recipe: r, score: s, reasons: reasons.map(x => x[1]) };
}

// ---------- selection ----------

function synergyWeight(p: Profile): number {
  return p.budget === 'tight' ? 2.2 : p.budget === 'relaxed' ? 0.7 : 1.4;
}

function wasteWeight(p: Profile): number {
  return p.budget === 'tight' ? 1.2 : p.budget === 'relaxed' ? 0.4 : 0.9;
}

/** £ of fresh food saved by adding `d` to an existing basket rather than buying it separately. */
function saving(existing: Demand, d: Demand, ctx: PlanContext): number {
  return perishableCost(existing, ctx.catalogue) + perishableCost(d, ctx.catalogue) - perishableCost(mergeDemand(existing, d), ctx.catalogue);
}

/**
 * How well a recipe fits the basket so far: rewards finishing packs already being bought and
 * penalises fresh food that would be left over. Matters most for small households.
 */
function basketFit(existing: Demand, d: Demand, ctx: PlanContext): number {
  const extraWaste = perishableWaste(mergeDemand(existing, d), ctx.catalogue) - perishableWaste(existing, ctx.catalogue);
  // Weights are tuned in pounds; convert dollars so a US or Canadian basket is judged the same way.
  return (synergyWeight(ctx.profile) * saving(existing, d, ctx) - wasteWeight(ctx.profile) * extraWaste) / ctx.catalogue.market.perGbp;
}

export function sharedReason(r: Recipe, others: Recipe[], ctx: PlanContext): string | null {
  const mine = ctx.catalogue.facts.get(r.id)!.ingredientIds;
  for (const o of others) {
    const shared = [...ctx.catalogue.facts.get(o.id)!.ingredientIds].filter(id => {
      const ing = ctx.catalogue.ingredients.get(id);
      return mine.has(id) && ing?.perishable && !ing.pantry;
    });
    if (shared.length) {
      const names = shared.slice(0, 2).map(id => inlineName(ctx.catalogue.ingredients.get(id)!.name));
      return `Shares ${names.join(' and ')} with ${o.title}`;
    }
  }
  return null;
}

function diversityPenalty(r: Recipe, chosen: Recipe[], meal: MealType): number {
  const scale = meal === 'dinner' ? 1 : 0.5;
  let pen = 0;
  for (const c of chosen) {
    if (c.meal !== r.meal) continue;
    if (c.cuisine === r.cuisine) pen += 1.2;
    if (c.protein === r.protein) pen += r.protein === 'veg' ? 0.5 : 1.5;
  }
  return pen * scale;
}

const MEAL_ORDER: MealType[] = ['dinner', 'lunch', 'breakfast'];

const PERISHABILITY: Record<string, number> = { fish: 3, prawns: 3, chicken: 2, turkey: 2, beef: 2, pork: 2, lamb: 2 };

export function eligibleFor(meal: MealType, ctx: PlanContext, exclude: Set<string>): Scored[] {
  return ctx.catalogue.recipes
    .filter(r => r.meal === meal && !exclude.has(r.id) && whyNot(r, ctx) === null)
    .map(r => scoreRecipe(r, ctx))
    .sort((a, b) => b.score - a.score);
}

/**
 * Builds a week: keeps locked picks, then greedily fills each meal type with the
 * best-scoring recipe given what is already chosen, rewarding shared fresh ingredients
 * and penalising repeated cuisines and proteins.
 */
export function planWeek(ctx: PlanContext, locked: { recipeId: string; meal: MealType; day?: number }[] = []): PlannedPick[] {
  const cat = ctx.catalogue;
  const kept = locked.filter(l => {
    const r = cat.byId.get(l.recipeId);
    return r && whyNot(r, ctx) === null;
  });
  const chosen: Recipe[] = [...new Set(kept.map(l => l.recipeId))].map(id => cat.byId.get(id)!);
  const reasons = new Map<string, string[]>(chosen.map(r => [r.id, scoreRecipe(r, ctx).reasons]));
  let demand = mergeDemand(...chosen.map(r => recipeDemand(r, ctx.profile.people)));

  for (const meal of MEAL_ORDER) {
    const want = ctx.profile.meals[meal] - chosen.filter(r => r.meal === meal).length;
    if (want <= 0) continue;
    const pool = eligibleFor(meal, ctx, new Set(chosen.map(r => r.id))).slice(0, 45);
    for (let n = 0; n < want && pool.length; n++) {
      let bestI = -1, bestV = -Infinity;
      pool.forEach((c, i) => {
        const v = c.score + basketFit(demand, recipeDemand(c.recipe, ctx.profile.people), ctx) - diversityPenalty(c.recipe, chosen, meal);
        if (v > bestV) { bestV = v; bestI = i; }
      });
      const [pick] = pool.splice(bestI, 1);
      const shared = sharedReason(pick.recipe, chosen, ctx);
      reasons.set(pick.recipe.id, shared ? [shared, ...pick.reasons] : pick.reasons);
      chosen.push(pick.recipe);
      demand = mergeDemand(demand, recipeDemand(pick.recipe, ctx.profile.people));
    }
  }
  return assignDays(chosen, new Set(kept.map(l => l.recipeId)), reasons,
    new Map(kept.filter(l => l.day !== undefined).map(l => [l.recipeId, l.day!])));
}

/** Orders meals so the most perishable proteins are cooked first in the week. */
export function assignDays(recipes: Recipe[], locked: Set<string>, reasons: Map<string, string[]>, keptDays = new Map<string, number>()): PlannedPick[] {
  const out: PlannedPick[] = [];
  for (const meal of MEAL_ORDER) {
    const list = recipes.filter(r => r.meal === meal)
      .sort((a, b) => (PERISHABILITY[b.protein] ?? 1) - (PERISHABILITY[a.protein] ?? 1));
    const days = new Map<string, number>();
    const used = new Set<number>();
    for (const r of list) {
      const day = keptDays.get(r.id);
      if (locked.has(r.id) && day !== undefined && Number.isInteger(day) && day >= 0 && day < 7 && !used.has(day)) {
        days.set(r.id, day); used.add(day);
      }
    }
    for (const r of list) {
      const day = days.get(r.id) ?? [0, 1, 2, 3, 4, 5, 6].find(d => !used.has(d)) ?? 6;
      used.add(day);
      out.push({ recipeId: r.id, meal, day, locked: locked.has(r.id) || undefined, reasons: reasons.get(r.id) ?? [] });
    }
  }
  return out;
}

/** Best swaps for one pick, given the rest of the week. */
export function alternatives(ctx: PlanContext, picks: { recipeId: string; meal: MealType }[], index: number, n = 4): Scored[] {
  const cat = ctx.catalogue;
  const target = picks[index];
  const others = picks.filter((_, i) => i !== index).map(p => cat.byId.get(p.recipeId)).filter((r): r is Recipe => !!r);
  const demand = mergeDemand(...others.map(r => recipeDemand(r, ctx.profile.people)));
  return eligibleFor(target.meal, ctx, new Set(picks.map(p => p.recipeId))).slice(0, 40)
    .map(c => {
      const shared = sharedReason(c.recipe, others, ctx);
      return {
        recipe: c.recipe,
        score: c.score + basketFit(demand, recipeDemand(c.recipe, ctx.profile.people), ctx) - diversityPenalty(c.recipe, others, target.meal),
        reasons: shared ? [shared, ...c.reasons] : c.reasons,
      };
    })
    .sort((a, b) => b.score - a.score)
    .slice(0, n);
}

// ---------- cravings ----------

const TAG_WORDS: [RegExp, string][] = [
  [/\b(spic|hot|heat|fiery|chilli)/i, 'spicy'], [/\b(comfort|cosy|cozy|hearty|warming)/i, 'comfort'],
  [/\b(light|fresh|healthy|salad)/i, 'fresh'], [/\b(quick|fast|easy|lazy)/i, 'quick'],
  [/\b(fakeaway|takeaway)/i, 'fakeaway'], [/\b(veggie|vegetarian|meat-?free|plant)/i, 'veggie'],
  [/\b(protein|gym)/i, 'high-protein'], [/\b(whole ?foods?|unprocessed|real food|from scratch)/i, 'whole'], [/\b(new|different|adventur)/i, 'new'], [/\b(cheap|budget|thrift)/i, 'budget'],
];

/** Turns "something spicy with loads of peppers, maybe Thai" into a Focus. */
export function parseCraving(text: string, cat: Catalogue): Focus {
  const lower = text.toLowerCase();
  const tags = TAG_WORDS.filter(([re]) => re.test(lower)).map(([, t]) => t);
  const cuisines = (Object.entries(CUISINE_LABELS) as [Cuisine, string][])
    .filter(([, label]) => lower.includes(label.toLowerCase()))
    .map(([c]) => c);
  const ingredients: string[] = [];
  for (const ing of cat.ingredients.values()) {
    if (ing.pantry) continue;
    const base = ing.name.toLowerCase().replace(/\(.*?\)/g, '').replace(/^(fresh|baby|raw|smoked|tinned|cooked|boneless|free-range|ground|dried|chestnut|mature|italian hard|soft|flat-leaf|rose)\s+/g, '').trim();
    const word = base.split(' ').pop() ?? base;
    const forms = new Set([base, `${base}s`, `${base}es`, ing.id.replace(/-/g, ' ')]);
    if (word.length > 3) { forms.add(word); forms.add(`${word}s`); }
    if ([...forms].some(f => f.length > 2 && new RegExp(`\\b${f.replace(/[.*+?^${}()|[\]\\]/g, '\\$&')}\\b`).test(lower))) ingredients.push(ing.id);
  }
  // "peppers" should mean the fresh peppers, not black pepper.
  const filtered = ingredients.filter(id => !['black-pepper', 'chilli-powder', 'chilli-flakes'].includes(id));
  return { tags: [...new Set(tags)], cuisines: [...new Set(cuisines)], ingredients: [...new Set(filtered)].slice(0, 6) };
}
