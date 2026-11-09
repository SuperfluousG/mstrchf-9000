import type { Allergen, Cuisine, Diet, MealType, Protein } from './types';

/** -1 avoid, 0 neutral, 1 like, 2 love. */
export type Affinity = -1 | 0 | 1 | 2;

export interface Profile {
  people: number;
  diet: Diet;
  avoid: Allergen[];
  dislikes: string[];
  /** Preferred heat: 0 none … 3 hot. */
  spice: 0 | 1 | 2 | 3;
  cuisines: Partial<Record<Cuisine, Affinity>>;
  proteins: Partial<Record<Protein, Affinity>>;
  /** Usual maximum minutes for a weeknight meal. */
  maxTime: number;
  /** Meals per week by type. */
  meals: Record<MealType, number>;
  budget: 'relaxed' | 'balanced' | 'tight';
}

export const DEFAULT_PROFILE: Profile = {
  people: 2,
  diet: 'omnivore',
  avoid: [],
  dislikes: [],
  spice: 1,
  cuisines: {},
  proteins: {},
  maxTime: 40,
  meals: { dinner: 5, lunch: 0, breakfast: 0 },
  budget: 'balanced',
};

export const CUISINE_GROUPS: { label: string; cuisines: Cuisine[] }[] = [
  { label: 'Italian', cuisines: ['italian'] },
  { label: 'Mediterranean', cuisines: ['greek', 'spanish'] },
  { label: 'British & European', cuisines: ['british', 'european', 'french'] },
  { label: 'Indian', cuisines: ['indian'] },
  { label: 'Middle Eastern', cuisines: ['middle-eastern', 'north-african'] },
  { label: 'Thai & Vietnamese', cuisines: ['thai', 'vietnamese', 'indonesian'] },
  { label: 'Chinese', cuisines: ['chinese'] },
  { label: 'Japanese & Korean', cuisines: ['japanese', 'korean'] },
  { label: 'Mexican & Latin', cuisines: ['mexican', 'latin'] },
  { label: 'American & Caribbean', cuisines: ['american', 'caribbean'] },
];

export const PROTEIN_CHOICES: { label: string; proteins: Protein[] }[] = [
  { label: 'Chicken', proteins: ['chicken', 'turkey'] },
  { label: 'Beef', proteins: ['beef'] },
  { label: 'Pork', proteins: ['pork'] },
  { label: 'Lamb', proteins: ['lamb'] },
  { label: 'Fish', proteins: ['fish'] },
  { label: 'Prawns', proteins: ['prawns'] },
  { label: 'Veggie', proteins: ['veg', 'beans', 'lentils', 'cheese', 'egg'] },
  { label: 'Tofu', proteins: ['tofu'] },
];

export const ALLERGEN_LABELS: Record<Allergen, string> = {
  gluten: 'Gluten', milk: 'Dairy', egg: 'Egg', fish: 'Fish', crustaceans: 'Shellfish',
  molluscs: 'Molluscs', nuts: 'Tree nuts', peanuts: 'Peanuts', soy: 'Soya', sesame: 'Sesame',
  mustard: 'Mustard', celery: 'Celery', sulphites: 'Sulphites',
};

export const CUISINE_LABELS: Record<Cuisine, string> = {
  british: 'British', italian: 'Italian', french: 'French', spanish: 'Spanish', greek: 'Greek',
  'middle-eastern': 'Middle Eastern', 'north-african': 'North African', indian: 'Indian', thai: 'Thai',
  chinese: 'Chinese', japanese: 'Japanese', korean: 'Korean', vietnamese: 'Vietnamese',
  indonesian: 'Indonesian', mexican: 'Mexican', american: 'American', caribbean: 'Caribbean',
  latin: 'Latin American', european: 'European',
};

export function groupAffinity(p: Profile, cuisines: Cuisine[]): Affinity {
  return (p.cuisines[cuisines[0]] ?? 0) as Affinity;
}

export function setGroupAffinity(p: Profile, cuisines: Cuisine[], a: Affinity): Profile {
  const next = { ...p.cuisines };
  for (const c of cuisines) next[c] = a;
  return { ...p, cuisines: next };
}

export function setProteinAffinity(p: Profile, proteins: Protein[], a: Affinity): Profile {
  const next = { ...p.proteins };
  for (const x of proteins) next[x] = a;
  return { ...p, proteins: next };
}
