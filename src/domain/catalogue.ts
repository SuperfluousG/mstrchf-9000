import type { Allergen, Diet, Ingredient, Recipe } from './types';

export interface RecipeFacts {
  vegetarian: boolean;
  vegan: boolean;
  pescatarian: boolean;
  allergens: Set<Allergen>;
  /** Ingredient ids, for quick lookups. */
  ingredientIds: Set<string>;
}

export interface Catalogue {
  ingredients: Map<string, Ingredient>;
  recipes: Recipe[];
  byId: Map<string, Recipe>;
  facts: Map<string, RecipeFacts>;
}

const ANIMAL_FLESH = new Set(['meat', 'poultry', 'fish', 'shellfish']);
const LAND_MEAT = new Set(['meat', 'poultry']);
const ANIMAL_ANY = new Set(['meat', 'poultry', 'fish', 'shellfish', 'dairy', 'egg', 'honey']);

export function factsFor(recipe: Recipe, ingredients: Map<string, Ingredient>): RecipeFacts {
  let flesh = false, landMeat = false, animal = false;
  const allergens = new Set<Allergen>();
  const ids = new Set<string>();
  for (const ri of recipe.ingredients) {
    const def = ingredients.get(ri.id);
    if (!def) continue;
    ids.add(ri.id);
    if (ANIMAL_FLESH.has(def.kind)) flesh = true;
    if (LAND_MEAT.has(def.kind)) landMeat = true;
    if (ANIMAL_ANY.has(def.kind)) animal = true;
    for (const a of def.allergens ?? []) allergens.add(a);
  }
  return { vegetarian: !flesh, vegan: !animal, pescatarian: !landMeat, allergens, ingredientIds: ids };
}

export function buildCatalogue(ingredientList: Ingredient[], recipes: Recipe[]): Catalogue {
  const ingredients = new Map(ingredientList.map(i => [i.id, i]));
  const valid = recipes.filter(r => r.ingredients.every(ri => ingredients.has(ri.id)));
  const byId = new Map(valid.map(r => [r.id, r]));
  const facts = new Map(valid.map(r => [r.id, factsFor(r, ingredients)]));
  return { ingredients, recipes: valid, byId, facts };
}

export function suitsDiet(f: RecipeFacts, diet: Diet): boolean {
  switch (diet) {
    case 'vegan': return f.vegan;
    case 'vegetarian': return f.vegetarian;
    case 'pescatarian': return f.pescatarian;
    default: return true;
  }
}

export function dietLabel(f: RecipeFacts): string | null {
  if (f.vegan) return 'Vegan';
  if (f.vegetarian) return 'Veggie';
  return null;
}
