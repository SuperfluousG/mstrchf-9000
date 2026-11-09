import ingredientList from '../catalogue/ingredients.json';
import { buildCatalogue } from './domain/catalogue';
import type { Ingredient, Recipe } from './domain/types';

const files = import.meta.glob<Recipe[]>('../catalogue/recipes/*.json', { eager: true, import: 'default' });
const recipes = Object.values(files).flat();

export const catalogue = buildCatalogue(ingredientList as Ingredient[], recipes);

// Recipe photos are src/photos/<recipe id>.webp (add them with scripts/import_photos.py). Recipes without one get a drawn plate.
const photos = import.meta.glob<string>('./photos/*.webp', { eager: true, query: '?url', import: 'default' });
const photoById = new Map(Object.entries(photos).map(([path, url]) => [path.slice(path.lastIndexOf('/') + 1, -'.webp'.length), url]));

export function imageUrl(r: Recipe): string | null {
  return photoById.get(r.id) ?? null;
}
