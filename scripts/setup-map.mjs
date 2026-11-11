// Writes plugin/skills/mstrchf-planner/app/setup.json: the `files` lists the planner skill passes to the
// Artifact tool, one list per publish. The first holds the page's script, stylesheet, fonts, icon and brand
// images, so the planner opens after one small publish; the recipe photos follow in the later ones.
import { existsSync, readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const MAX = 250; // one publish takes at most 255 files

const root = fileURLToPath(new URL('..', import.meta.url));
const app = join(root, 'plugin', 'skills', 'mstrchf-planner', 'app');
const { files } = JSON.parse(readFileSync(join(app, 'files.json'), 'utf8'));

// A built photo is assets/<recipe id>-<8-character hash>.webp; it is a recipe photo when src/photos has that id.
const isRecipePhoto = f => /^assets\/.+\.webp$/.test(f) && existsSync(join(root, 'src', 'photos', `${f.slice(7, -14)}.webp`));
const core = files.filter(f => !isRecipePhoto(f));
const photos = files.filter(isRecipePhoto);

const parts = [core];
const n = Math.ceil(photos.length / MAX);
const size = Math.ceil(photos.length / n);
for (let i = 0; i < photos.length; i += size) parts.push(photos.slice(i, i + size));

// One file per line, so the skill can pass each list to the Artifact tool exactly as written.
const list = p => `  [\n${p.map(path => `   {"path": ${JSON.stringify(path)}}`).join(',\n')}\n  ]`;
const text = `{\n "parts": [\n${parts.map(list).join(',\n')}\n ]\n}\n`;
JSON.parse(text);
writeFileSync(join(app, 'setup.json'), text);
console.log(`setup.json: ${core.length} core files, then ${photos.length} recipe photos in ${parts.length - 1} parts (${parts.map(p => p.length).join(' + ')}).`);
