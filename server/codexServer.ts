import { randomUUID } from 'node:crypto';
import { readFile } from 'node:fs/promises';
import { join } from 'node:path';
import { McpServer, ResourceTemplate } from '@modelcontextprotocol/sdk/server/mcp.js';
import { registerAppResource, registerAppTool, RESOURCE_MIME_TYPE } from '@modelcontextprotocol/ext-apps/server';
import { z } from 'zod';
import { buildBasket } from '../src/domain/basket';
import { recipeQty } from '../src/domain/format';
import type { AppState, WeekPlan } from '../src/domain/kitchen';
import { EMPTY_FOCUS, exclusionReason, parseCraving, planWeek, type PlanContext } from '../src/domain/planner';
import { marketOf, retailerOf } from '../src/domain/profile';
import { buildShoppingList } from '../src/domain/shopping';
import { mondayOf, weeksBetween } from '../src/domain/weeks';
import { readBackup } from '../src/domain/backup';
import { DESKTOP_VERSION, PLANNER_URI } from '../src/domain/desktopEdition';
import { loadCodexLibrary } from './codexLibrary';
import { CodexKitchen, defaultKitchenDir } from './codexStorage';

export { PLANNER_URI } from '../src/domain/desktopEdition';
const readOnly = { readOnlyHint: true, destructiveHint: false, openWorldHint: false };
const localWrite = { readOnlyHint: false, destructiveHint: false, openWorldHint: false };
const result = (data: object, text: string) => ({ content: [{ type: 'text' as const, text }], structuredContent: data as Record<string, unknown> });
const error = (e: unknown) => ({ isError: true, content: [{ type: 'text' as const, text: e instanceof Error ? e.message : 'The local kitchen could not complete the action.' }] });
function weekId(value?: string): string {
  const id = value ?? mondayOf(new Date());
  const date = new Date(`${id}T12:00:00`);
  if (!/^\d{4}-\d{2}-\d{2}$/.test(id) || !Number.isFinite(date.getTime()) || date.getDay() !== 1 || mondayOf(date) !== id) throw new Error('Choose a week by its Monday, YYYY-MM-DD.');
  return id;
}

export async function createCodexServer(root: string, dataDir = defaultKitchenDir()) {
  const library = await loadCodexLibrary(join(root, 'catalogue'));
  const kitchen = new CodexKitchen(dataDir, library);
  // Sidebar entrypoints use the MCP server icon, independently of the package listing icon.
  const icon = await readFile(join(root, 'assets', 'icon.svg'));
  const server = new McpServer({ name: 'mstrchf-9000', version: DESKTOP_VERSION,
    icons: [{ src: `data:image/svg+xml;base64,${icon.toString('base64')}`, mimeType: 'image/svg+xml', sizes: ['80x80'] }],
  });
  registerAppTool(server, 'open_planner', {
    title: 'MSTRCHF 9000', description: 'Open the interactive meal planner to set up a kitchen, browse recipes, plan meals and review shopping.', inputSchema: {}, annotations: readOnly,
    _meta: { ui: { resourceUri: PLANNER_URI }, 'openai/ui': { entrypoints: [{ type: 'global' }] } },
  }, async () => { try { return result(await kitchen.read(), 'MSTRCHF 9000 is open.'); } catch (e) { return error(e); } });
  registerAppResource(server, 'MSTRCHF 9000 planner', PLANNER_URI, {}, async () => ({ contents: [{ uri: PLANNER_URI, mimeType: RESOURCE_MIME_TYPE, text: await readFile(join(root, 'app', 'planner.html'), 'utf8'), _meta: {
    ui: { prefersBorder: false, csp: { connectDomains: [], resourceDomains: ['data:'] } },
    'openai/ui': { availableDisplayModes: ['fullscreen'], preferredDisplayMode: 'fullscreen' },
  } }] }));
  server.registerResource('Recipe photo', new ResourceTemplate('recipe-photo://library/{id}', { list: undefined }), { mimeType: 'image/webp' }, async (uri, { id }) => {
    if (typeof id !== 'string' || !library.byId.has(id)) throw new Error('Unknown recipe photo.');
    return { contents: [{ uri: uri.href, mimeType: 'image/webp', blob: (await readFile(join(root, 'app', 'photos', `${id}.webp`))).toString('base64') }] };
  });
  server.registerTool('get_kitchen', { description: 'Read the current local MSTRCHF kitchen, its save revision and file location.', inputSchema: {}, annotations: readOnly }, async () => {
    try { return result(await kitchen.read(), 'Read the local kitchen.'); } catch (e) { return error(e); }
  });
  const saveSchema = { expectedRevision: z.string().min(1).max(64), stateJson: z.string().max(4_500_000) };
  for (const replace of [false, true]) server.registerTool(replace ? 'restore_kitchen' : 'save_kitchen', {
    description: replace ? 'Replace the local kitchen after the user confirms a backup restore or Start over in the planner.' : 'Save planner edits with optimistic concurrency; ordered basket records remain immutable.', inputSchema: saveSchema,
    annotations: { ...localWrite, destructiveHint: replace }, _meta: { ui: { visibility: ['app'] } },
  }, async ({ expectedRevision, stateJson }) => {
    try { if (Buffer.byteLength(stateJson) > 4_500_000) throw new Error('This kitchen is too large to save.'); const state = readBackup(stateJson); return result(await kitchen.update(() => state, expectedRevision, replace), 'Kitchen saved.'); } catch (e) { return error(e); }
  });
  server.registerTool('get_photo', {
    description: 'Read one known recipe photo for a planner view when the host cannot forward resource reads.', inputSchema: { recipeId: z.string().max(150) }, annotations: readOnly, _meta: { ui: { visibility: ['app'] } },
  }, async ({ recipeId }) => {
    try {
      if (!library.byId.has(recipeId)) throw new Error('Unknown recipe photo.');
      return result({ mimeType: 'image/webp', blob: (await readFile(join(root, 'app', 'photos', `${recipeId}.webp`))).toString('base64') }, 'Recipe photo.');
    } catch (e) { return error(e); }
  });
  server.registerTool('export_file', {
    description: 'Save a MSTRCHF JSON backup or basket to this kitchen’s local exports folder. Returns its exact path.', inputSchema: { filename: z.string().max(128), text: z.string().max(4_500_000) }, annotations: localWrite, _meta: { ui: { visibility: ['app'] } },
  }, async ({ filename, text }) => {
    try { const path = await kitchen.exportFile(filename, text); return result({ path }, `Saved ${path}`); } catch (e) { return error(e); }
  });

  function context(s: AppState, id: string, w: WeekPlan): PlanContext {
    if (!s.profile) throw new Error('Set up your kitchen in the planner first.');
    const catalogue = library.forMarket(marketOf(s.profile));
    const recency = new Map<string, number>();
    for (const other of Object.values(s.weeks)) if (other.id !== id) {
      const distance = Math.abs(weeksBetween(other.id, id));
      if (distance <= 8) for (const p of other.picks) recency.set(p.recipeId, Math.min(recency.get(p.recipeId) ?? 99, distance));
    }
    const parsed = parseCraving(w.craving, catalogue);
    return { catalogue, profile: s.profile, recency, favourites: new Set(s.favourites), ratings: new Map(Object.entries(s.ratings)), hidden: new Set(s.hidden), seed: w.seed,
      focus: { tags: [...new Set([...w.focus.tags, ...parsed.tags])], ingredients: [...new Set([...w.focus.ingredients, ...parsed.ingredients])], cuisines: [...new Set([...w.focus.cuisines, ...parsed.cuisines])] } };
  }
  registerAppTool(server, 'plan_week', {
    description: 'Plan or re-plan a draft week using the saved kitchen. Keeps chosen meals and dates, rechecks exclusions and groups ingredients. Does not order groceries or create calendar events.',
    inputSchema: { weekId: z.string().optional(), craving: z.string().max(1000).optional(), shuffle: z.boolean().optional() }, annotations: localWrite, _meta: { ui: { resourceUri: PLANNER_URI } },
  }, async args => {
    try {
      const id = weekId(args.weekId);
      const snap = await kitchen.update(s => {
        const w = s.weeks[id] ?? { id, craving: '', focus: EMPTY_FOCUS, picks: [], status: 'draft', have: [], seed: Math.floor(Math.random() * 1e6) };
        if (w.status === 'ordered') throw new Error('This week is ordered. Reopen it from Shop before re-planning.');
        const updated = { ...w, craving: args.craving ?? w.craving, seed: args.shuffle === false ? w.seed : Math.floor(Math.random() * 1e6) };
        const ctx = context(s, id, updated);
        for (const p of w.picks) if (!p.locked) ctx.recency.set(p.recipeId, 1);
        const picks = planWeek(ctx, w.picks.filter(p => p.locked)).map(p => ({ ...p, uid: w.picks.find(old => old.locked && old.recipeId === p.recipeId)?.uid ?? randomUUID() }));
        return { ...s, weeks: { ...s.weeks, [id]: { ...updated, picks } } };
      });
      const w = snap.state!.weeks[id];
      const cat = library.forMarket(marketOf(snap.state!.profile));
      return result({ ...snap, selectedWeekId: id, meals: w.picks.map(p => ({ ...p, title: cat.byId.get(p.recipeId)?.title })) }, `Planned ${w.picks.length} meals for the week of ${id}.`);
    } catch (e) { return error(e); }
  });
  server.registerTool('get_week', { description: 'Read a planned week, its meal dates and titles. Uses the current week by default.', inputSchema: { weekId: z.string().optional() }, annotations: readOnly }, async args => {
    try {
      const snap = await kitchen.read(); const id = weekId(args.weekId); const w = snap.state?.weeks[id];
      if (!w || !snap.state?.profile) throw new Error('There are no meals planned for that week. Open the planner or plan_week first.');
      const cat = library.forMarket(marketOf(snap.state.profile));
      return result({ weekId: id, status: w.status, people: w.status === 'ordered' ? w.orderedBasket?.people ?? snap.state.profile.people : snap.state.profile.people,
        meals: w.picks.map(p => ({ ...p, title: cat.byId.get(p.recipeId)?.title })) }, `${w.picks.length} meals in this week.`);
    } catch (e) { return error(e); }
  });
  server.registerTool('get_basket', { description: 'Read the pack-rounded grocery basket for a planned week, or its frozen ordered record. Prices are estimates. This tool does not shop.', inputSchema: { weekId: z.string().optional() }, annotations: readOnly }, async args => {
    try {
      const snap = await kitchen.read(); const id = weekId(args.weekId); const s = snap.state; const w = s?.weeks[id];
      if (!s?.profile || !w?.picks.length) throw new Error('Plan meals for that week first.');
      const ctx = context(s, id, w); const cat = ctx.catalogue;
      const recipes = w.picks.map(p => cat.byId.get(p.recipeId));
      if (recipes.some(r => !r || exclusionReason(r, ctx))) throw new Error('This week includes excluded or unavailable recipes. Review and re-plan before shopping.');
      if (w.status === 'ordered' && w.orderedBasket) return result({ basket: w.orderedBasket, ordered: true }, 'Read the recorded ordered basket.');
      const list = buildShoppingList({ recipes: recipes.filter(r => !!r), people: s.profile.people, have: new Set([...s.cupboard, ...w.have]), pantryToBuy: new Set(w.pantryToBuy ?? []) }, cat);
      return result({ basket: buildBasket(list, id, s.profile.people, recipes.map(r => r!.title), cat.market, retailerOf(s.profile)), ordered: w.status === 'ordered', legacyEstimate: w.status === 'ordered' }, 'Read the current planning estimate.');
    } catch (e) { return error(e); }
  });
  server.registerTool('get_recipe', { description: 'Read an original recipe with ingredient and method quantities scaled for the selected household. Use an ordered week to retain its original servings.',
    inputSchema: { recipeId: z.string().max(150), people: z.number().int().min(1).max(8).optional(), weekId: z.string().optional() }, annotations: readOnly,
  }, async args => {
    try {
      const s = (await kitchen.read()).state; const w = args.weekId ? s?.weeks[weekId(args.weekId)] : undefined;
      const people = args.people ?? (w?.status === 'ordered' ? w.orderedBasket?.people : undefined) ?? s?.profile?.people ?? 2;
      const cat = library.forMarket(marketOf(s?.profile)); const recipe = library.recipe(args.recipeId, people, cat.market.id);
      if (!recipe) throw new Error('That recipe is not in the catalogue.');
      const scaled = recipe.ingredients.map(i => { const ing = cat.ingredients.get(i.id)!; return { name: ing.name, amount: recipeQty(i.qty * people / recipe.servings, ing), prep: i.prep, cupboard: !!ing.pantry }; });
      return result({ recipe: { id: recipe.id, title: recipe.title, blurb: recipe.blurb, minutes: recipe.time, people, ingredients: scaled, steps: recipe.steps }, allergens: [...cat.facts.get(recipe.id)!.allergens] }, recipe.title);
    } catch (e) { return error(e); }
  });
  return { server, kitchen, library };
}
