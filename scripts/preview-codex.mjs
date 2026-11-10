// Development-only SDK host. The shipped plugin uses stdio directly in Codex.
import { createServer } from 'node:http';
import { randomBytes } from 'node:crypto';
import { mkdtemp, readFile, writeFile, rm } from 'node:fs/promises';
import { tmpdir } from 'node:os';
import { resolve, join, sep } from 'node:path';
import { Client } from '@modelcontextprotocol/sdk/client/index.js';
import { StdioClientTransport } from '@modelcontextprotocol/sdk/client/stdio.js';
import { build } from 'esbuild';

const root = resolve(import.meta.dirname, '..');
const dataDir = await mkdtemp(join(tmpdir(), 'mstrchf-codex-preview-'));
const fixture = resolve(root, 'data/test/kitchen.json');
if (process.argv.includes('--test-fixture')) await writeFile(join(dataDir, 'kitchen.json'), await readFile(fixture));
const token = randomBytes(24).toString('hex');
const port = 5197;
const origin = `http://127.0.0.1:${port}`;
const bundled = await build({ entryPoints: [join(root, 'scripts/codex-preview.ts')], bundle: true, platform: 'browser', format: 'esm', write: false, minify: true, logLevel: 'silent' });
const transport = new StdioClientTransport({ command: process.execPath, args: [join(root, 'codex-plugin/server/index.mjs')], env: { ...process.env, MSTRCHF_DATA_DIR: dataDir }, stderr: 'inherit' });
const client = new Client({ name: 'MSTRCHF preview host', version: '0.1.0' });
await client.connect(transport);
const plannerUri = (await client.listTools()).tools.find(t => t.name === 'open_planner')._meta.ui.resourceUri;
const server = createServer(async (req, res) => {
  try {
    if (req.headers.host !== `127.0.0.1:${port}`) { res.writeHead(403).end(); return; }
    if (req.method === 'GET' && (req.url === '/' || req.url?.startsWith('/?'))) {
      res.setHeader('content-type', 'text/html; charset=utf-8');
      res.end(`<!doctype html><html lang="en-GB"><meta name="viewport" content="width=device-width,initial-scale=1"><meta name="preview-token" content="${token}"><title>MSTRCHF — MCP Apps test host</title><style>html,body{margin:0;height:100%;font:12px system-ui;background:#eeede5}header{height:36px;display:flex;align-items:center;justify-content:space-between;padding:0 10px;box-sizing:border-box;background:#2a1517;color:#eeede5;gap:8px}button{font:inherit}iframe{width:100%;height:calc(100% - 36px);border:0;display:block}</style><header><span id="status">Connecting to test kitchen…</span><button id="replan">Model re-plan</button></header><iframe id="planner" title="MSTRCHF planner test view"></iframe><script type="module" src="/bridge.js"></script></html>`);
    } else if (req.method === 'GET' && req.url === '/bridge.js') {
      res.setHeader('content-type', 'text/javascript'); res.end(bundled.outputFiles[0].text);
    } else if (req.method === 'GET' && req.url === '/planner') {
      res.setHeader('content-type', 'text/html; charset=utf-8');
      res.setHeader('Content-Security-Policy', "default-src 'none'; script-src 'unsafe-inline'; style-src 'unsafe-inline'; img-src data:; font-src data:; connect-src 'none'; frame-ancestors 'self'");
      const page = await client.readResource({ uri: plannerUri }); res.end(page.contents[0].text);
    } else if (req.method === 'POST' && req.url === '/rpc' && req.headers.origin === origin && req.headers['x-preview-token'] === token) {
      let body = ''; for await (const part of req) { body += part; if (Buffer.byteLength(body) > 4_600_000) throw new Error('Request too large'); }
      const { method, params } = JSON.parse(body);
      const answer = method === 'tools/call' ? await client.callTool(params) : method === 'resources/read' ? await client.readResource(params) : null;
      if (!answer) { res.writeHead(400).end(); return; }
      res.setHeader('content-type', 'application/json'); res.end(JSON.stringify(answer));
    } else res.writeHead(404).end();
  } catch (e) { res.writeHead(500).end(e.message); }
});
let stopping = false;
async function stop() {
  if (stopping) return; stopping = true; server.close(); await client.close();
  if (resolve(dataDir).startsWith(resolve(tmpdir()) + sep) && dataDir.includes('mstrchf-codex-preview-')) await rm(dataDir, { recursive: true, force: true });
  process.exit();
}
process.on('SIGINT', stop); process.on('SIGTERM', stop);
server.listen(port, '127.0.0.1', () => console.log(`MCP Apps test host: ${origin}; disposable data: ${dataDir}`));
