// Turns the Vite build in dist/ into a Claude artifact page plus its supporting files.
// Output: plugin/skills/mstrchf-planner/app/ (index.html, assets/, fonts/, recipes/, files.json).
// The artifact page has no doctype/html/head/body: Claude wraps it. Every URL is relative.
import { cpSync, existsSync, mkdirSync, readdirSync, readFileSync, rmSync, statSync, writeFileSync } from 'node:fs';
import { join, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const root = fileURLToPath(new URL('..', import.meta.url));
const dist = join(root, 'dist');
const out = join(root, 'plugin', 'skills', 'mstrchf-planner', 'app');

if (!existsSync(join(dist, 'index.html'))) throw new Error('Run `vite build` first: dist/index.html is missing.');
const html = readFileSync(join(dist, 'index.html'), 'utf8');

const title = html.match(/<title>[^<]*<\/title>/)?.[0] ?? '<title>MSTRCHF 9000</title>';
const links = [...html.matchAll(/<link\b[^>]*>/g)].map(m => m[0]);
const scripts = [...html.matchAll(/<script\b[^>]*><\/script>/g)].map(m => m[0]);
const rootDiv = html.match(/<div id="root">[\s\S]*?<\/div>/)?.[0];
if (!scripts.length || !rootDiv) throw new Error('dist/index.html is missing its script or root element.');

const rel = tag => tag.replace(/(href|src)="\.\//g, '$1="').replace(/ crossorigin(?=[\s>])/g, '');
// Font preloads keep crossorigin (fonts are fetched in CORS mode); stylesheet and script are same-origin.
const fontPreload = tag => tag.replace(/(href)="\.\//g, '$1="');

const page = [
  title,
  ...links.map(l => (/rel="preload"/.test(l) ? fontPreload(l) : rel(l))),
  rootDiv,
  ...scripts.map(rel),
  '',
].join('\n');

rmSync(out, { recursive: true, force: true });
mkdirSync(out, { recursive: true });
for (const name of readdirSync(dist)) {
  if (name === 'index.html') continue;
  cpSync(join(dist, name), join(out, name), { recursive: true });
}
writeFileSync(join(out, 'index.html'), page);

const files = [];
const walk = dir => {
  for (const name of readdirSync(dir)) {
    const p = join(dir, name);
    if (statSync(p).isDirectory()) walk(p);
    else if (name !== 'index.html' && name !== 'files.json') files.push(relative(out, p).split(sep).join('/'));
  }
};
walk(out);
files.sort();
writeFileSync(join(out, 'files.json'), JSON.stringify({ page: 'index.html', files }, null, 2) + '\n');

const size = files.reduce((n, f) => n + statSync(join(out, f)).size, 0) + Buffer.byteLength(page);
console.log(`Artifact page written to ${relative(root, out)} (${files.length + 1} files, ${(size / 1024 / 1024).toFixed(1)} MB).`);
