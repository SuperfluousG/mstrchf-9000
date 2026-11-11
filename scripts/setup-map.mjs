// Writes plugin/skills/mstrchf-planner/app/setup.json: the `files` maps the planner skill passes to the
// Artifact tool, so a user's Claude copies the build server side from the source artifact instead of
// uploading ~400 files from disk. Every path in files.json must already be published to SOURCE
// (see HANDOVER.md, "Source artifact for installs").
import { readFileSync, writeFileSync } from 'node:fs';
import { join } from 'node:path';
import { fileURLToPath } from 'node:url';

const SOURCE = 'https://claude.ai/artifact/VjcD3g9juBh6jUBcD9wXvQ';
const MAX = 250; // one publish takes at most 255 files

const app = fileURLToPath(new URL('../plugin/skills/mstrchf-planner/app/', import.meta.url));
const { files } = JSON.parse(readFileSync(join(app, 'files.json'), 'utf8'));

// The page's script, stylesheet, fonts and icon go in the first part, so the page works as soon as it lands.
const ordered = [...files.filter(f => !f.endsWith('.webp')), ...files.filter(f => f.endsWith('.webp'))];
const size = Math.ceil(ordered.length / Math.ceil(ordered.length / MAX));
const parts = [];
for (let i = 0; i < ordered.length; i += size) parts.push(ordered.slice(i, i + size));

// One file per line, so the skill can pass each part to the Artifact tool exactly as written.
const entry = path => `   ${JSON.stringify(path)}: ${JSON.stringify({ artifact: SOURCE, path })}`;
const body = parts.map(p => `  {\n${p.map(entry).join(',\n')}\n  }`).join(',\n');
const text = `{\n "source": ${JSON.stringify(SOURCE)},\n "parts": [\n${body}\n ]\n}\n`;
JSON.parse(text);
writeFileSync(join(app, 'setup.json'), text);
console.log(`setup.json: ${ordered.length} files from ${SOURCE} in ${parts.length} parts.`);
