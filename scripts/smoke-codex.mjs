import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';
import { mkdtemp, readFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { join, resolve, sep } from 'node:path';
import assert from 'node:assert/strict';

const dir = await mkdtemp(join(tmpdir(), 'mstrchf-package-smoke-'));
const client = new Client({ name: 'MSTRCHF package verification', version: '1.0.0' });
const transport = new StdioClientTransport({ command: process.execPath, args: [resolve('codex-plugin/server/index.mjs')], env: { ...process.env, MSTRCHF_DATA_DIR: dir }, stderr: 'pipe' });
let serverLog = ''; transport.stderr?.on('data', c => { serverLog += c.toString(); });
async function call(name, args = {}) {
  const r = await client.callTool({ name, arguments: args });
  if (r.isError) throw new Error(JSON.stringify(r.content));
  return r.structuredContent;
}
try {
  await client.connect(transport);
  const tools = (await client.listTools()).tools;
  assert(tools.some(t => t.name === 'open_planner'));
  const version = JSON.parse(await readFile('codex-plugin/plugin.json', 'utf8')).version;
  assert.equal(client.getServerVersion().version, version);
  const plannerUri = tools.find(t => t.name === 'open_planner')._meta.ui.resourceUri;
  assert(plannerUri.includes(version));
  const empty = await call('get_kitchen'); assert.equal(empty.state, null);
  const state = { version: 2, profile: { people: 2, diet: 'omnivore', avoid: ['nuts'], dislikes: [], spice: 2, cuisines: { thai: 2 }, proteins: {}, maxTime: 40, meals: { dinner: 5, lunch: 2, breakfast: 1 }, budget: 'balanced', market: 'uk', retailer: 'tesco' }, weeks: {}, favourites: [], hidden: [], ratings: {}, cupboard: [] };
  await call('save_kitchen', { expectedRevision: empty.revision, stateJson: JSON.stringify(state) });
  const plan = await call('plan_week', { weekId: '2026-11-16', craving: 'Spicy, lots of peppers' }); assert.equal(plan.meals.length, 8);
  const basket = await call('get_basket', { weekId: '2026-11-16' }); assert.equal(basket.basket.market, 'uk'); assert(basket.basket.items.length > 0);
  const recipe = await call('get_recipe', { recipeId: 'thai-larb-style-pork-lettuce-cups', people: 4 }); assert(recipe.recipe.steps.join(' ').includes('juice of 3 limes'));
  const page = await client.readResource({ uri: plannerUri }); const html = page.contents[0].text;
  assert(html.includes('window.__MSTRCHF_CODEX__=true')); assert(Buffer.byteLength(html) < 5_000_000);
  const photo = await client.readResource({ uri: 'recipe-photo://library/thai-larb-style-pork-lettuce-cups' }); assert.equal(Buffer.from(photo.contents[0].blob, 'base64').subarray(0, 4).toString(), 'RIFF');
  const exported = await call('export_file', { filename: 'mstrchf-test-backup.json', text: JSON.stringify({ app: 'MSTRCHF 9000', state: plan.state }) }); assert(exported.path.startsWith(join(dir, 'exports') + sep));
  console.log(JSON.stringify({ verified: true, transport: 'stdio', tools: tools.length, plannedMeals: plan.meals.length, appBytes: Buffer.byteLength(html), photoBytes: Buffer.from(photo.contents[0].blob, 'base64').length, data: 'throwaway only' }));
} catch (e) { console.error(serverLog); throw e; }
finally { await client.close(); if (resolve(dir).startsWith(resolve(tmpdir()) + sep) && dir.includes('mstrchf-package-smoke-')) await rm(dir, { recursive: true, force: true }); }
