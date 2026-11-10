// Core data model for MSTRCHF 9000 v2.
// Quantities are always stored in an ingredient's base unit (g, ml or each)
// for the recipe's base servings (2). Display conversions happen at render time.

export type Unit = 'g' | 'ml' | 'each';

export type Aisle =
  | 'veg' | 'fruit' | 'herbs' | 'meat' | 'fish' | 'dairy' | 'eggs' | 'bakery'
  | 'grains' | 'tins' | 'sauces' | 'spices' | 'world' | 'nuts' | 'frozen' | 'cupboard';

/** What an ingredient is, for diet rules. */
export type Kind = 'plant' | 'meat' | 'poultry' | 'fish' | 'shellfish' | 'dairy' | 'egg' | 'honey';

export type Allergen =
  | 'gluten' | 'milk' | 'egg' | 'fish' | 'crustaceans' | 'molluscs' | 'nuts'
  | 'peanuts' | 'soy' | 'sesame' | 'mustard' | 'celery' | 'sulphites';

export interface Ingredient {
  id: string;
  name: string;
  aisle: Aisle;
  unit: Unit;
  kind: Kind;
  allergens?: Allergen[];
  /** Typical pack sizes as [size in base unit, estimated GBP price]. */
  packs: [number, number][];
  /** Retailer search phrase. */
  search: string;
  /** Spoils within about a week once bought; leftovers count as waste. */
  perishable?: boolean;
  /** Assumed to be in the cupboard (salt, oil). Shown separately, not bought by default. */
  pantry?: boolean;
  /** Grams or ml per teaspoon, for friendly display of small amounts. */
  tsp?: number;
  /** Display colour used by generated dish art. */
  colour?: string;
  /** Ready-made or heavily processed (jarred pastes, stock cubes, wraps, cured meats). */
  processed?: boolean;
}

export type MealType = 'dinner' | 'lunch' | 'breakfast';

export type Cuisine =
  | 'british' | 'italian' | 'french' | 'spanish' | 'greek' | 'middle-eastern' | 'north-african'
  | 'indian' | 'thai' | 'chinese' | 'japanese' | 'korean' | 'vietnamese' | 'indonesian'
  | 'mexican' | 'american' | 'caribbean' | 'latin' | 'european'
  | 'german' | 'nordic' | 'lebanese' | 'canadian';

export type Protein =
  | 'chicken' | 'beef' | 'pork' | 'lamb' | 'turkey' | 'fish' | 'prawns' | 'egg'
  | 'cheese' | 'beans' | 'lentils' | 'tofu' | 'veg';

export interface RecipeIngredient {
  id: string;
  /** Quantity in the ingredient's base unit for the recipe's base servings. */
  qty: number;
  /** Optional preparation note, e.g. "finely sliced". */
  prep?: string;
}

export interface Recipe {
  id: string;
  title: string;
  /** Short appetising line, HelloFresh style. */
  blurb: string;
  meal: MealType;
  cuisine: Cuisine;
  protein: Protein;
  /** Total minutes, hands-on plus cooking. */
  time: number;
  difficulty: 1 | 2 | 3;
  /** 0 none, 1 mild, 2 medium, 3 hot. */
  spice: 0 | 1 | 2 | 3;
  servings: number;
  tags: string[];
  ingredients: RecipeIngredient[];
  steps: string[];
  source: string;
}

export type Diet = 'omnivore' | 'pescatarian' | 'vegetarian' | 'vegan';
