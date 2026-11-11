import { build } from 'esbuild';
import { cp, lstat, mkdir, readFile, readdir, writeFile } from 'node:fs/promises';
import { resolve, join, relative, sep } from 'node:path';
import { fileURLToPath } from 'node:url';
import { spawnSync } from 'node:child_process';

const root = resolve(fileURLToPath(new URL('..', import.meta.url)));
const work = resolve(root, '.codex-build');
const out = resolve(root, 'codex-plugin');
if (!work.startsWith(root + sep) || relative(root, work) !== '.codex-build') throw new Error('Unsafe temporary build path.');
try { if ((await lstat(work)).isSymbolicLink()) throw new Error('Build directory is a link.'); } catch (e) { if (e.code !== 'ENOENT') throw e; }
const vite = spawnSync(process.execPath, [join(root, 'node_modules/vite/bin/vite.js'), 'build', '--outDir', work, '--emptyOutDir'], {
  cwd: root, env: { ...process.env, MSTRCHF_MCP_BUILD: '1' }, encoding: 'utf8', stdio: 'inherit',
});
if (vite.status !== 0) throw new Error('Codex app build failed.');
await mkdir(join(out, 'app', 'photos'), { recursive: true });
await mkdir(join(out, 'server'), { recursive: true });
await mkdir(join(out, 'assets'), { recursive: true });
const html = await readFile(join(work, 'index.html'), 'utf8');
const scripts = [...html.matchAll(/<script\b[^>]*src="([^"]+)"[^>]*><\/script>/g)].map(m => m[1]);
const styles = [...html.matchAll(/<link\b[^>]*href="([^"]+\.css)"[^>]*>/g)].map(m => m[1]);
if (scripts.length !== 1 || styles.length !== 1) throw new Error('Unexpected Vite entrypoints.');
const js = await readFile(resolve(work, scripts[0]), 'utf8');
if ((await readdir(join(work, 'assets'))).filter(name => name.endsWith('.js')).length !== 1) throw new Error('Codex app needs one self-contained script.');
let css = await readFile(resolve(work, styles[0]), 'utf8');
const fontUrls = new Map();
for (const name of await readdir(join(root, 'public/fonts'))) {
  if (!name.endsWith('.woff2')) continue;
  fontUrls.set(name, `data:font/woff2;base64,${(await readFile(join(root, 'public/fonts', name))).toString('base64')}`);
}
css = css.replace(/url\((['"]?)(.*?)\1\)/g, (whole, _quote, url) => fontUrls.has(url.split('/').pop()) ? `url("${fontUrls.get(url.split('/').pop())}")` : whole);
const page = html.replace(/<link\b[^>]*>/g, '').replace(/<script\b[^>]*><\/script>/g, '')
  .replace('</head>', () => `<style>${css}</style><script>window.__MSTRCHF_CODEX__=true;</script></head>`)
  .replace('</body>', () => `<script type="module">${js.replace(/<\/script/gi, '<\\/script')}</script></body>`);
const embedded = page.match(/<script type="module">([\s\S]*?)<\/script>/)?.[1];
if (!embedded || Buffer.byteLength(page) > 5_000_000) throw new Error('Invalid or oversized native app page.');
const syntax = spawnSync(process.execPath, ['--check', '--input-type=module'], { input: embedded, encoding: 'utf8' });
if (syntax.status !== 0) throw new Error(`Embedded app syntax failed: ${syntax.stderr}`);
if ([...css.matchAll(/url\((['"]?)(.*?)\1\)/g)].some(m => !m[2].startsWith('data:') && !m[2].startsWith('#'))) throw new Error('External CSS assets remain in the Codex app.');
await writeFile(join(out, 'app', 'planner.html'), page);
await cp(join(root, 'src/photos'), join(out, 'app/photos'), { recursive: true });
await cp(join(root, 'catalogue'), join(out, 'catalogue'), { recursive: true, filter: path => !/PHOTO-BRIEF|AUTHORING/.test(path) });
await build({ entryPoints: [join(root, 'server/codex.ts')], outfile: join(out, 'server/index.mjs'), bundle: true, platform: 'node', format: 'esm', target: 'node20', minify: true,
  banner: { js: 'import {createRequire} from "node:module";const require=createRequire(import.meta.url);' }, logLevel: 'info' });
const mark = '<svg xmlns="http://www.w3.org/2000/svg" viewBox="0 0 64 64"><path fill="#E2C779" d="M6 56V8h11l15 22L47 8h11v48H45V31L32 49 19 31v25Z"/><path fill="#741F29" d="m12 8 33 48h5L17 8Z"/><path fill="#EEEDE5" d="m40 18 7-10h11v7H47l-4 6Z"/></svg>';
await writeFile(join(out, 'assets/mark.svg'), mark);
await writeFile(join(out, 'assets/icon.svg'), mark.replace('viewBox="0 0 64 64"', 'viewBox="0 0 80 80"').replace('<path', '<rect width="80" height="80" rx="16" fill="#741F29"/><g transform="translate(8 8)"><path').replace('</svg>', '</g></svg>'));
console.log(`Codex app: ${(Buffer.byteLength(page) / 1024 / 1024).toFixed(1)} MB HTML; photos served lazily through MCP resources.`);
