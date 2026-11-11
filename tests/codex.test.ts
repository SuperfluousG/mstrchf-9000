import { cp, mkdir, mkdtemp, readFile, rm, writeFile } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve, sep } from 'node:path';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { InMemoryTransport } from '@modelcontextprotocol/sdk/inMemory.js';
import { afterAll, beforeAll, describe, expect, it } from 'vitest';
import { createCodexServer, PLANNER_URI } from '../server/codexServer';
import { buildBasket } from '../src/domain/basket';
import { EMPTY_STATE, type AppState } from '../src/domain/kitchen';
import { DEFAULT_PROFILE, retailerOf } from '../src/domain/profile';
import { buildShoppingList } from '../src/domain/shopping';

const WEEK = '2026-11-16';
describe('local Codex plugin', () => {
  let folder: string; let client: Client; let built: Awaited<ReturnType<typeof createCodexServer>>;
  beforeAll(async () => {
    folder = await mkdtemp(join(tmpdir(), 'mstrchf-codex-test-'));
    await cp(resolve('catalogue'), join(folder, 'catalogue'), { recursive: true });
    await mkdir(join(folder, 'app', 'photos'), { recursive: true });
    await cp(resolve('codex-plugin/assets'), join(folder, 'assets'), { recursive: true });
    await writeFile(join(folder, 'app', 'planner.html'), '<!doctype html><html><body>Planner fixture</body></html>');
    await cp(resolve('src/photos/thai-larb-style-pork-lettuce-cups.webp'), join(folder, 'app', 'photos', 'thai-larb-style-pork-lettuce-cups.webp'));
    built = await createCodexServer(folder, join(folder, 'data'));
    const [c, s] = InMemoryTransport.createLinkedPair();
    client = new Client({ name: 'MSTRCHF test', version: '1.0.0' });
    await built.server.connect(s); await client.connect(c);
  });
  afterAll(async () => {
    await client?.close(); await built?.server.close();
    if (folder && resolve(folder).startsWith(resolve(tmpdir()) + sep) && folder.includes('mstrchf-codex-test-')) await rm(folder, { recursive: true, force: true });
  });
  const call = async (name: string, args: Record<string, unknown> = {}) => client.callTool({ name, arguments: args });
  async function seed() {
    return built.kitchen.update(() => ({ ...structuredClone(EMPTY_STATE), profile: { ...structuredClone(DEFAULT_PROFILE), market: 'uk', retailer: 'tesco', people: 4 } }), undefined, true);
  }
  it('discovers a zero-argument global planner entrypoint and its fullscreen app resource', async () => {
    const icon = client.getServerVersion()!.icons![0];
    expect(icon.mimeType).toBe('image/svg+xml');
    const svg = Buffer.from(icon.src.split(',')[1], 'base64').toString();
    expect(svg).toContain('M6 56V8h11l15 22');
    expect(icon.sizes).toEqual(['80x80']);
    const tools = (await client.listTools()).tools;
    const opener = tools.find(t => t.name === 'open_planner')!;
    expect(opener._meta).toMatchObject({ ui: { resourceUri: PLANNER_URI }, 'openai/ui': { entrypoints: [{ type: 'global' }] } });
    expect(opener.inputSchema.required ?? []).toEqual([]);
    const page = await client.readResource({ uri: PLANNER_URI });
    expect(page.contents[0].mimeType).toBe('text/html;profile=mcp-app');
    expect(page.contents[0]._meta).toMatchObject({ 'openai/ui': { preferredDisplayMode: 'fullscreen' } });
  });
  it('persists a kitchen and rejects a stale writer without losing newer edits', async () => {
    const first = await seed();
    await built.kitchen.update(s => ({ ...s, profile: { ...s.profile!, people: 3 } }), first.revision);
    const stale = await call('save_kitchen', { expectedRevision: first.revision, stateJson: JSON.stringify(first.state) });
    expect(stale.isError).toBe(true);
    expect((await built.kitchen.read()).state!.profile!.people).toBe(3);
  });
  it('plans through MCP, keeps a Sunday meal identity, and removes a kept meal after an allergy change', async () => {
    await seed(); await call('plan_week', { weekId: WEEK, craving: 'Thai, spicy' });
    const snap = await built.kitchen.read(); const first = snap.state!.weeks[WEEK].picks[0];
    await built.kitchen.update(s => ({ ...s, weeks: { ...s.weeks, [WEEK]: { ...s.weeks[WEEK], picks: s.weeks[WEEK].picks.map(p => p.uid === first.uid ? { ...p, locked: true, day: 6 } : p) } } }));
    await call('plan_week', { weekId: WEEK });
    expect((await built.kitchen.read()).state!.weeks[WEEK].picks.find(p => p.uid === first.uid)?.day).toBe(6);
    const cat = built.library.forMarket('uk');
    const nutty = cat.recipes.find(r => r.meal === 'dinner' && cat.facts.get(r.id)!.allergens.has('nuts'))!;
    await built.kitchen.update(s => ({ ...s, profile: { ...s.profile!, avoid: ['nuts'] }, weeks: { ...s.weeks, [WEEK]: { ...s.weeks[WEEK], picks: [{ uid: 'nutty', recipeId: nutty.id, meal: 'dinner', day: 6, locked: true, reasons: [] }] } } }));
    await call('plan_week', { weekId: WEEK });
    expect((await built.kitchen.read()).state!.weeks[WEEK].picks.some(p => p.recipeId === nutty.id)).toBe(false);
  });
  it('freezes an ordered basket across market/household changes and rejects a mutation', async () => {
    await seed(); await call('plan_week', { weekId: WEEK });
    await built.kitchen.update(s => {
      const w = s.weeks[WEEK]; const cat = built.library.forMarket('uk'); const recipes = w.picks.map(p => cat.byId.get(p.recipeId)!);
      const list = buildShoppingList({ recipes, people: 4, have: new Set() }, cat);
      return { ...s, weeks: { ...s.weeks, [WEEK]: { ...w, status: 'ordered', orderedBasket: buildBasket(list, WEEK, 4, recipes.map(r => r.title), cat.market, retailerOf(s.profile)) } } };
    });
    const before = (await built.kitchen.read()).state!.weeks[WEEK].orderedBasket;
    await built.kitchen.update(s => ({ ...s, profile: { ...s.profile!, market: 'ca', people: 2 } }));
    const basket = await call('get_basket', { weekId: WEEK });
    expect(basket.structuredContent).toMatchObject({ basket: before, ordered: true });
    await expect(built.kitchen.update(s => ({ ...s, weeks: { ...s.weeks, [WEEK]: { ...s.weeks[WEEK], picks: [] } } }))).rejects.toThrow(/ordered/);
    expect((await call('plan_week', { weekId: WEEK })).isError).toBe(true);
  });
  it('returns scaled methods and cupboard quantities through the recipe tool', async () => {
    await seed();
    const r = await call('get_recipe', { recipeId: 'thai-larb-style-pork-lettuce-cups', people: 4 });
    const data = r.structuredContent as { recipe: { steps: string[]; ingredients: { cupboard: boolean; amount: string }[] } };
    expect(data.recipe.steps.join(' ')).toContain('juice of 3 limes');
    expect(data.recipe.ingredients.filter(i => i.cupboard).every(i => !!i.amount)).toBe(true);
  });
  it('only serves known recipe photo resources', async () => {
    const photo = await client.readResource({ uri: 'recipe-photo://library/thai-larb-style-pork-lettuce-cups' });
    expect(photo.contents[0].mimeType).toBe('image/webp');
    expect('blob' in photo.contents[0] && Buffer.from(photo.contents[0].blob as string, 'base64').subarray(0, 4).toString()).toBe('RIFF');
    await expect(client.readResource({ uri: 'recipe-photo://library/%2e%2e%2fsecret' })).rejects.toThrow();
    const fallback = await call('get_photo', { recipeId: 'thai-larb-style-pork-lettuce-cups' });
    expect(fallback.structuredContent).toMatchObject({ mimeType: 'image/webp' });
    expect((await call('get_photo', { recipeId: '../secret' })).isError).toBe(true);
  });
  it('exports a real JSON file inside its own folder and rejects arbitrary paths', async () => {
    const snap = await seed();
    const text = JSON.stringify({ app: 'MSTRCHF 9000', state: snap.state });
    const saved = await call('export_file', { filename: 'mstrchf-backup.json', text });
    const path = (saved.structuredContent as { path: string }).path;
    expect(path.startsWith(join(folder, 'data', 'exports') + sep)).toBe(true);
    expect(JSON.parse(await readFile(path, 'utf8')).state.profile.people).toBe(4);
    expect((await call('export_file', { filename: '../kitchen.json', text })).isError).toBe(true);
  });
  it('requires explicit reset/restore to clear an existing kitchen and persists that reset', async () => {
    const before = await seed();
    expect((await call('save_kitchen', { expectedRevision: before.revision, stateJson: JSON.stringify(EMPTY_STATE) })).isError).toBe(true);
    expect((await call('restore_kitchen', { expectedRevision: before.revision, stateJson: JSON.stringify(EMPTY_STATE) })).isError).not.toBe(true);
    expect((await built.kitchen.read()).state).toMatchObject({ profile: null, weeks: {} } satisfies Partial<AppState>);
  });
});
