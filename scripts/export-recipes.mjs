// Exports every recipe with readable ingredient lines and method for the plugin's meal-calendar skill.
// Loads the app's own catalogue and quantity formatting through Vite, so the wording matches the app.
// Output: plugin/skills/meal-calendar/recipes.json
import { writeFileSync } from 'node:fs';
import { fileURLToPath } from 'node:url';
import { createServer } from 'vite';

const root = fileURLToPath(new URL('..', import.meta.url));
const out = fileURLToPath(new URL('../plugin/skills/meal-calendar/recipes.json', import.meta.url));

// No file watcher and no dependency scan: this only loads a few modules, and the repo holds large ignored folders.
const server = await createServer({
  root, configFile: false, logLevel: 'error', appType: 'custom',
  server: { middlewareMode: true, hmr: false, watch: null },
  optimizeDeps: { noDiscovery: true, include: [] },
});
try {
  const { catalogue } = await server.ssrLoadModule('/src/data.ts');
  const { recipeQty } = await server.ssrLoadModule('/src/domain/format.ts');

  const recipes = catalogue.recipes.map(r => {
    const lines = [];
    const cupboard = [];
    for (const ri of r.ingredients) {
      const ing = catalogue.ingredients.get(ri.id);
      if (!ing) throw new Error(`${r.id}: unknown ingredient ${ri.id}`);
      if (ing.pantry) { cupboard.push(ing.name.toLowerCase()); continue; }
      lines.push({ amount: recipeQty(ri.qty, ing), name: ing.name, ...(ri.prep ? { prep: ri.prep } : {}), qty: ri.qty, unit: ing.unit, ...(ing.tsp ? { gramsPerTsp: ing.tsp } : {}) });
    }
    return { id: r.id, title: r.title, blurb: r.blurb, meal: r.meal, minutes: r.time, serves: r.servings, ingredients: lines, cupboard, steps: r.steps };
  });

  writeFileSync(out, JSON.stringify({
    note: 'Quantities are for `serves` people. `qty` is in `unit` (g, ml or each); `amount` is the same quantity as the app shows it. `cupboard` items are assumed in stock.',
    recipes,
  }) + '\n');
  console.log(`Exported ${recipes.length} recipes to plugin/skills/meal-calendar/recipes.json`);
} finally {
  await server.close();
}
