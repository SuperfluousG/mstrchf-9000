// Writes plugin/skills/mstrchf-planner/app/setup.json: the `files` lists the planner skill passes to the
// Artifact tool, one list per publish. The first holds the page's script, stylesheet, fonts, icon and brand
// images, so the planner opens after one small publish; the recipe photo packs (scripts/pack_photos.py) follow.
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const MAX = 250; // one publish takes at most 255 files

const root = fileURLToPath(new URL('..', import.meta.url));
const app = join(root, 'plugin', 'skills', 'mstrchf-planner', 'app');
const { files } = JSON.parse(readFileSync(join(app, 'files.json'), 'utf8'));

// A built photo pack is assets/photos-<n>-<hash>.json.
const isPack = f => /^assets\/photos-\d+-[^/]+\.json$/.test(f);
const core = files.filter(f => !isPack(f));
const packs = files.filter(isPack);
if (core.length > MAX || packs.length > MAX) throw new Error(`setup.json: ${core.length} core files and ${packs.length} packs; each part must be at most ${MAX}.`);
const parts = packs.length ? [core, packs] : [core];

// One file per line, so the skill can pass each list to the Artifact tool exactly as written.
const list = p => `  [\n${p.map(path => `   {"path": ${JSON.stringify(path)}}`).join(',\n')}\n  ]`;
const text = `{\n "parts": [\n${parts.map(list).join(',\n')}\n ]\n}\n`;
JSON.parse(text);
writeFileSync(join(app, 'setup.json'), text);
console.log(`setup.json: ${core.length} core files, then ${packs.length} photo packs.`);
