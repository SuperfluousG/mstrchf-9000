import { readFile, readdir } from 'node:fs/promises';
import { join } from 'node:path';
import ingredients from '../catalogue/ingredients.json';
import terms from '../catalogue/terms.json';
import { buildCatalogue, type Catalogue } from '../src/domain/catalogue';
import { localIngredient, localRecipe, type MarketFile, type Terms } from '../src/domain/localize';
import { MARKETS, setActiveMarket, type Market } from '../src/domain/market';
import type { Ingredient, Recipe } from '../src/domain/types';

export async function loadCodexLibrary(catalogueDir: string) {
  const files = (await readdir(join(catalogueDir, 'recipes'))).filter(f => f.endsWith('.json')).sort();
  const sourceRecipes = (await Promise.all(files.map(async f => JSON.parse(await readFile(join(catalogueDir, 'recipes', f), 'utf8')) as Recipe[]))).flat();
  const byId = new Map(sourceRecipes.map(r => [r.id, r]));
  const markets: Partial<Record<Market, MarketFile>> = {};
  for (const id of ['us', 'ca'] as const) markets[id] = JSON.parse(await readFile(join(catalogueDir, 'markets', `${id}.json`), 'utf8'));
  const cache = new Map<Market, Catalogue>();
  function forMarket(id: Market): Catalogue {
    let cat = cache.get(id);
    if (!cat) {
      cat = buildCatalogue((ingredients as Ingredient[]).map(i => localIngredient(i, id, markets)), sourceRecipes.map(r => localRecipe(r, id, terms as Terms)), MARKETS[id]);
      cache.set(id, cat);
    }
    setActiveMarket(id);
    return cat;
  }
  return {
    forMarket, byId,
    recipe(id: string, people: number, market: Market) {
      const r = byId.get(id); return r ? localRecipe(r, market, terms as Terms, people) : undefined;
    },
  };
}
export type CodexLibrary = Awaited<ReturnType<typeof loadCodexLibrary>>;
