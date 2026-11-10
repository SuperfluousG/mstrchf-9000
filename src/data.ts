import ingredientList from '../catalogue/ingredients.json';
import terms from '../catalogue/terms.json';
import { buildCatalogue, type Catalogue } from './domain/catalogue';
import { localIngredient, localRecipe, type MarketFile, type Terms } from './domain/localize';
import { type Market, MARKETS, setActiveMarket } from './domain/market';
import type { Ingredient, Recipe } from './domain/types';

const files = import.meta.glob<Recipe[]>('../catalogue/recipes/*.json', { eager: true, import: 'default' });
/** British source recipes, placeholders unrendered. */
export const sourceRecipes = Object.values(files).flat();

const marketFiles = import.meta.glob<MarketFile>('../catalogue/markets/*.json', { eager: true, import: 'default' });
const byMarket: Partial<Record<Market, MarketFile>> = Object.fromEntries(Object.values(marketFiles).map(f => [f.market, f]));

const built = new Map<Market, Catalogue>();

/** The catalogue in one country's words, measures, packs and prices. Built once per country. */
export function catalogueFor(m: Market): Catalogue {
  let cat = built.get(m);
  if (!cat) {
    const market = m === 'uk' || byMarket[m] ? m : 'uk';
    const ingredients = (ingredientList as Ingredient[]).map(i => localIngredient(i, market, byMarket));
    cat = buildCatalogue(ingredients, sourceRecipes.map(r => localRecipe(r, market, terms as Terms)), MARKETS[market]);
    built.set(m, cat);
  }
  return cat;
}

/** The catalogue for the country being shown. Reassigned by `selectMarket`; importers always read the current one. */
export let catalogue: Catalogue = catalogueFor('uk');

/** Switch the whole app to a country. Cheap after the first call for that country. */
export function selectMarket(m: Market): void {
  setActiveMarket(m);
  catalogue = catalogueFor(m);
}

// Recipe photos are src/photos/<recipe id>.webp (add them with scripts/import_photos.py). Recipes without one get a drawn plate.
const photos = import.meta.glob<string>('./photos/*.webp', { eager: true, query: '?url', import: 'default' });
const photoById = new Map(Object.entries(photos).map(([path, url]) => [path.slice(path.lastIndexOf('/') + 1, -'.webp'.length), url]));

export function imageUrl(r: Recipe): string | null {
  return photoById.get(r.id) ?? null;
}
