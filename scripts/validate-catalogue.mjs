#!/usr/bin/env node
// Validates catalogue/ingredients.json, catalogue/terms.json, catalogue/markets/*.json and every catalogue/recipes/*.json file.
// Usage: node scripts/validate-catalogue.mjs [recipes-file ...]
// With file arguments only those recipe files are checked (cross-file id clashes are still checked).
import { readFileSync, readdirSync } from 'node:fs';
import { join, dirname, basename } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = join(dirname(fileURLToPath(import.meta.url)), '..');
const catDir = join(root, 'catalogue');
const recipeDir = join(catDir, 'recipes');

const UNITS = ['g', 'ml', 'each'];
const AISLES = ['veg', 'fruit', 'herbs', 'meat', 'fish', 'dairy', 'eggs', 'bakery', 'grains', 'tins', 'sauces', 'spices', 'world', 'nuts', 'frozen', 'cupboard'];
const KINDS = ['plant', 'meat', 'poultry', 'fish', 'shellfish', 'dairy', 'egg', 'honey'];
const ALLERGENS = ['gluten', 'milk', 'egg', 'fish', 'crustaceans', 'molluscs', 'nuts', 'peanuts', 'soy', 'sesame', 'mustard', 'celery', 'sulphites'];
const MEALS = ['dinner', 'lunch', 'breakfast'];
const CUISINES = ['british', 'italian', 'french', 'spanish', 'greek', 'middle-eastern', 'north-african', 'indian', 'thai', 'chinese', 'japanese', 'korean', 'vietnamese', 'indonesian', 'mexican', 'american', 'caribbean', 'latin', 'european', 'german', 'nordic', 'lebanese', 'canadian'];
const PROTEINS = ['chicken', 'beef', 'pork', 'lamb', 'turkey', 'fish', 'prawns', 'egg', 'cheese', 'beans', 'lentils', 'tofu', 'veg'];
export const TAGS = ['quick', 'one-pot', 'traybake', 'family-friendly', 'comfort', 'light', 'high-protein', 'veg-packed', 'fakeaway', 'date-night', 'budget', 'no-cook', 'batch-friendly', 'lunchbox', 'meal-prep', 'fresh', 'hearty'];
const KEBAB = /^[a-z0-9]+(-[a-z0-9]+)*$/;

const errors = [];
const warn = [];
const err = (where, msg) => errors.push(`${where}: ${msg}`);

function load(path) {
  try { return JSON.parse(readFileSync(path, 'utf8')); }
  catch (e) { err(basename(path), `invalid JSON (${e.message})`); return null; }
}

// ---- ingredients ----
const ingredients = load(join(catDir, 'ingredients.json')) ?? [];
const ing = new Map();
for (const [i, x] of ingredients.entries()) {
  const w = `ingredients[${i}] ${x?.id ?? '?'}`;
  if (!x || typeof x !== 'object') { err(w, 'not an object'); continue; }
  if (!KEBAB.test(x.id ?? '')) err(w, 'id must be kebab-case');
  if (ing.has(x.id)) err(w, 'duplicate id');
  if (!x.name) err(w, 'missing name');
  if (!AISLES.includes(x.aisle)) err(w, `bad aisle ${x.aisle}`);
  if (!UNITS.includes(x.unit)) err(w, `bad unit ${x.unit}`);
  if (!KINDS.includes(x.kind)) err(w, `bad kind ${x.kind}`);
  for (const a of x.allergens ?? []) if (!ALLERGENS.includes(a)) err(w, `bad allergen ${a}`);
  if (!Array.isArray(x.packs) || !x.packs.length || x.packs.some(p => !Array.isArray(p) || p.length !== 2 || !(p[0] > 0) || !(p[1] >= 0))) err(w, 'packs must be [[size, price], ...]');
  if (!x.search) err(w, 'missing search');
  ing.set(x.id, x);
}

// ---- country data ----
const terms = load(join(catDir, 'terms.json')) ?? {};
for (const [k, v] of Object.entries(terms)) {
  if (k !== k.toLowerCase()) err(`terms.json "${k}"`, 'keys must be lower case');
  if (!v?.us) err(`terms.json "${k}"`, 'needs a "us" word');
}
const marketDir = join(catDir, 'markets');
for (const f of readdirSync(marketDir).filter(f => f.endsWith('.json')).sort()) {
  const m = load(join(marketDir, f));
  if (!m) continue;
  if (!['us', 'ca'].includes(m.market) || `${m.market}.json` !== f) err(f, 'market must be "us" or "ca" and match the file name');
  if (!m.currency) err(f, 'missing currency');
  for (const id of ing.keys()) if (!m.ingredients?.[id]) err(f, `no packs for ingredient "${id}"`);
  for (const [id, x] of Object.entries(m.ingredients ?? {})) {
    if (!ing.has(id)) err(f, `unknown ingredient id "${id}"`);
    if (!Array.isArray(x.packs) || !x.packs.length || x.packs.some(p => !Array.isArray(p) || p.length !== 2 || !(p[0] > 0) || !(p[1] >= 0))) err(`${f} ${id}`, 'packs must be [[size, price], ...]');
    for (const key of ['name', 'search']) if (x[key] !== undefined && (typeof x[key] !== 'string' || !x[key].trim())) err(`${f} ${id}`, `${key} must be a non-empty string`);
  }
}

// Recipe text is British and may use {placeholders}; see catalogue/AUTHORING.md and src/domain/localize.ts.
const MEASURE = /^(oven|temp|ml|g|cm) \d+(?:\.\d+)?$/;
function textProblems(text, recipe) {
  const out = [];
  for (const [, raw] of text.matchAll(/\{([^{}]*)\}/g)) {
    const inner = raw.trim();
    const pair = inner.split('|');
    if (MEASURE.test(inner) || /^qty \d+(?:\.\d+)?(?:\|[^|]+\|[^|]+)?$/.test(inner) || (pair.length === 2 && pair[0].trim() && pair[1].trim()) || (pair.length === 1 && terms[inner.toLowerCase()])) continue;
    out.push(`unknown placeholder {${raw}}`);
  }
  const plain = text.replace(/\{[^{}]*\}/g, ' ');
  if (/[{}]/.test(plain)) out.push('unmatched brace');
  if (/\d\s*°C/.test(plain)) out.push('write oven temperatures as {oven 220} (or {temp 180})');
  if (/\d\s*(ml|g|cm)\b/.test(plain)) out.push('write measures as {ml 300}, {g 50} or {cm 2}');
  for (const [word, def] of Object.entries(terms)) {
    if (def.onlyWith && !recipe.ingredients?.some(i => i.id === def.onlyWith)) continue;
    const scan = def.keep ? plain.replace(new RegExp(def.keep, 'gi'), ' ') : plain;
    if (new RegExp(`\\b${word}\\b`, 'i').test(scan)) out.push(`put "${word}" in braces: {${word}}`);
  }
  return out;
}
/** The British reading of a text, placeholders resolved roughly, for length checks. */
const british = text => text.replace(/\{([^{}]*)\}/g, (_, raw) => raw.split('|')[0].replace(/^(oven|temp) (\d+)$/, '$2°C').replace(/^(ml|g|cm) (\S+)$/, '$2 $1'));

// ---- recipes ----
const args = process.argv.slice(2);
const allFiles = readdirSync(recipeDir).filter(f => f.endsWith('.json')).sort();
const checkFiles = args.length ? args.map(a => basename(a)) : allFiles;
const seenIds = new Map();
const seenTitles = new Map();
let count = 0;
const tally = { meal: {}, cuisine: {}, protein: {}, veg: 0, vegan: 0 };

for (const f of allFiles) {
  const data = load(join(recipeDir, f));
  if (!Array.isArray(data)) { if (data) err(f, 'file must be a JSON array of recipes'); continue; }
  const checking = checkFiles.includes(f);
  for (const [i, r] of data.entries()) {
    const w = `${f}[${i}] ${r?.id ?? '?'}`;
    if (seenIds.has(r?.id)) err(w, `duplicate recipe id (also in ${seenIds.get(r.id)})`);
    seenIds.set(r?.id, f);
    const t = (r?.title ?? '').toLowerCase();
    if (seenTitles.has(t)) err(w, `duplicate title (also in ${seenTitles.get(t)})`);
    seenTitles.set(t, f);
    if (!checking) continue;
    count++;
    if (!KEBAB.test(r.id ?? '')) err(w, 'id must be kebab-case');
    if (!r.title || british(r.title).length > 60) err(w, 'title missing or over 60 chars');
    if (!r.blurb || british(r.blurb).length > 140) err(w, 'blurb missing or over 140 chars');
    const texts = [['title', r.title], ['blurb', r.blurb], ...(r.steps ?? []).map((s, n) => [`step ${n + 1}`, s]), ...(r.ingredients ?? []).filter(x => typeof x.prep === 'string').map(x => [`prep ${x.id}`, x.prep])];
    for (const [where, s] of texts) if (typeof s === 'string') for (const problem of textProblems(s, r)) err(w, `${where}: ${problem}`);
    if (!MEALS.includes(r.meal)) err(w, `bad meal ${r.meal}`);
    if (!CUISINES.includes(r.cuisine)) err(w, `bad cuisine ${r.cuisine}`);
    if (!PROTEINS.includes(r.protein)) err(w, `bad protein ${r.protein}`);
    if (!Number.isInteger(r.time) || r.time < 5 || r.time > 120) err(w, 'time must be an integer 5–120');
    if (![1, 2, 3].includes(r.difficulty)) err(w, 'difficulty must be 1, 2 or 3');
    if (![0, 1, 2, 3].includes(r.spice)) err(w, 'spice must be 0–3');
    if (r.servings !== 2) err(w, 'servings must be 2');
    if (!Array.isArray(r.tags)) err(w, 'tags must be an array');
    for (const tag of r.tags ?? []) if (!TAGS.includes(tag)) err(w, `unknown tag "${tag}" (allowed: ${TAGS.join(', ')})`);
    if (!Array.isArray(r.ingredients) || r.ingredients.length < 4) err(w, 'needs at least 4 ingredients');
    const used = new Set();
    let hasMeat = false, hasFish = false, animal = false;
    for (const x of r.ingredients ?? []) {
      const def = ing.get(x.id);
      if (!def) { err(w, `unknown ingredient id "${x.id}"`); continue; }
      if (used.has(x.id)) err(w, `ingredient "${x.id}" listed twice`);
      used.add(x.id);
      if (!(typeof x.qty === 'number' && x.qty > 0)) err(w, `ingredient "${x.id}" needs qty > 0`);
      if (def.unit === 'each' && x.qty > 30) warn.push(`${w}: ${x.id} qty ${x.qty} each looks high`);
      if (def.unit !== 'each' && x.qty > 2000) warn.push(`${w}: ${x.id} qty ${x.qty}${def.unit} looks high`);
      if (x.prep !== undefined && typeof x.prep !== 'string') err(w, `ingredient "${x.id}" prep must be a string`);
      if (['meat', 'poultry'].includes(def.kind)) hasMeat = true;
      if (['fish', 'shellfish'].includes(def.kind)) hasFish = true;
      if (['meat', 'poultry', 'fish', 'shellfish', 'dairy', 'egg', 'honey'].includes(def.kind)) animal = true;
    }
    if (!Array.isArray(r.steps) || r.steps.length < 3 || r.steps.some(s => typeof s !== 'string' || s.length < 15)) err(w, 'needs at least 3 steps of real text');
    if (r.image !== undefined) err(w, 'image is no longer used: photos are src/photos/<id>.webp');
    if (!r.source) err(w, 'missing source');
    if (['veg', 'beans', 'lentils', 'tofu', 'cheese', 'egg'].includes(r.protein) && (hasMeat || hasFish)) err(w, `protein "${r.protein}" but contains meat/fish`);
    tally.meal[r.meal] = (tally.meal[r.meal] ?? 0) + 1;
    tally.cuisine[r.cuisine] = (tally.cuisine[r.cuisine] ?? 0) + 1;
    tally.protein[r.protein] = (tally.protein[r.protein] ?? 0) + 1;
    if (!hasMeat && !hasFish) tally.veg++;
    if (!animal) tally.vegan++;
  }
}

for (const w of warn) console.warn(`warn  ${w}`);
if (errors.length) {
  for (const e of errors) console.error(`ERROR ${e}`);
  console.error(`\n${errors.length} error(s) across ${count} checked recipe(s).`);
  process.exit(1);
}
console.log(`OK: ${ing.size} ingredients, ${count} recipe(s) checked in ${checkFiles.length} file(s).`);
console.log(JSON.stringify(tally));
