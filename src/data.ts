import ingredientList from '../catalogue/ingredients.json';
import { buildCatalogue } from './domain/catalogue';
import type { Ingredient, Recipe } from './domain/types';

const files = import.meta.glob<Recipe[]>('../catalogue/recipes/*.json', { eager: true, import: 'default' });
const recipes = Object.values(files).flat();

export const catalogue = buildCatalogue(ingredientList as Ingredient[], recipes);

export function imageUrl(r: Recipe): string | null {
  return r.image ? `${import.meta.env.BASE_URL}recipes/${r.image}` : null;
}
