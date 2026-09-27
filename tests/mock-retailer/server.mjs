import { createServer } from 'node:http';
import { appendFileSync, existsSync, lstatSync, mkdtempSync, readFileSync, realpathSync,
  statSync, unlinkSync, writeFileSync } from 'node:fs';

const MAX_BODY = 16384;
const MAX_LEDGER = 32 * 1024 * 1024;
const MODES = ['normal', 'drop-after-commit', 'hold-after-commit', 'never-commit', 'gate-before-commit', 'raw-drop-after-commit'];
const clone = x => structuredClone(x);
const fail = (status, message) => { throw Object.assign(new Error(message), { status }); };
const integer = (x, min, max) => Number.isSafeInteger(x) && x >= min && x <= max;
function shape(value, allowed, required = []) {
  if (!value || typeof value !== 'object' || Array.isArray(value) ||
      Object.keys(value).some(k => !allowed.includes(k)) || required.some(k => !Object.hasOwn(value, k)))
    fail(400, 'Invalid or unexpected fields');
}
function initial() {
  const label = { status: 'known', ingredients: ['Synthetic beans', 'Water'], allergens: [],
    evidence: 'Synthetic fixture label; no real suitability claim' };
  return { synthetic: true, runId: crypto.randomUUID(), basketId: crypto.randomUUID(),
    loggedIn: true, revision: 0, readFailures: 0, nextAction: 'normal',
    context: { currency: 'GBP', deliveryDate: '2030-01-07', loyaltyMember: false },
    basket: { beans: 1, soap: 2 }, products: [
      { sku: 'beans', name: 'Synthetic beans tin', variant: 'in water', packCount: 1,
        netGrams: 400, drainedGrams: 240, unit: 'g', pricePence: 80, inStock: true, label,
        offer: { packs: 3, totalPence: 200, requiresLoyalty: true, starts: '2030-01-01', ends: '2030-01-31' } },
      { sku: 'eggs', name: 'Synthetic eggs', variant: 'six pack', packCount: 6,
        netGrams: null, drainedGrams: null, unit: 'each', pricePence: 190, inStock: true,
        label: { status: 'known', ingredients: ['Synthetic egg'], allergens: ['egg'], evidence: 'Synthetic fixture' }, offer: null },
      { sku: 'soap', name: 'Synthetic soap — unrelated basket item', variant: 'single bar', packCount: 1,
        netGrams: 100, drainedGrams: null, unit: 'g', pricePence: 125, inStock: true,
        label: { status: 'not-food', ingredients: [], allergens: [], evidence: 'Not food' }, offer: null }
    ] };
}
async function body(req) {
  if (req.headers['content-type'] !== 'application/json') fail(415, 'Use application/json');
  if (Number(req.headers['content-length']) > MAX_BODY) fail(413, 'Request too large');
  let bytes = 0; const chunks = [];
  for await (const chunk of req) {
    bytes += chunk.length;
    if (bytes > MAX_BODY) fail(413, 'Request too large');
    chunks.push(chunk);
  }
  try { return JSON.parse(Buffer.concat(chunks).toString('utf8')); }
  catch { fail(400, 'Invalid JSON'); }
}

export async function start({ dataDir, port = 0 } = {}) {
  if (!integer(port, 0, 65535)) throw new Error('Invalid port');
  const temp = realpathSync(process.env.TEMP || process.env.TMPDIR || '/tmp');
  dataDir = realpathSync(dataDir || mkdtempSync(`${temp}/mstrchf-mock-`));
  const normal = p => p.replaceAll('\\', '/').toLowerCase();
  if (!normal(dataDir).startsWith(`${normal(temp)}/`)) throw new Error('Data directory must be a child of system temp');
  const lock = `${dataDir}/server.lock`;
  const file = `${dataDir}/ledger.jsonl`;
  for (const name of [lock, file]) if (existsSync(name) && lstatSync(name).isSymbolicLink()) throw new Error('Symlink refused');
  if (existsSync(lock)) {
    const pid = Number(readFileSync(lock, 'utf8'));
    if (!integer(pid, 1, 2147483647)) throw new Error('Invalid lock; retain evidence and inspect');
    try { process.kill(pid, 0); throw new Error('Data directory is already in use'); }
    catch (error) { if (error.code !== 'ESRCH') throw error; }
    unlinkSync(lock); // Only this system-temp lock is removed, never ledger evidence.
  }
  writeFileSync(lock, String(process.pid), { flag: 'wx', flush: true });
  let events = [], current, ledgerBytes = 0, poisoned = false;
  const pending = new Map();
  function record(type, detail = {}, next) {
    if (poisoned) fail(503, 'Ledger failed; stop and inspect retained evidence');
    const event = { seq: events.length + 1, at: new Date().toISOString(),
      runId: (next || current).runId, type, ...detail, ...(next ? { snapshot: next } : {}) };
    const line = JSON.stringify(event) + '\n';
    if (events.length >= 10000 || ledgerBytes + Buffer.byteLength(line) > MAX_LEDGER)
      fail(507, 'Ledger capacity reached; retain this run and start a new temp directory');
    try { appendFileSync(file, line, { encoding: 'utf8', flush: true }); }
    catch (error) { poisoned = true; throw error; }
    ledgerBytes += Buffer.byteLength(line);
    events.push(event);
    if (next) current = next;
    return event;
  }
  try {
    if (existsSync(file)) {
      if (statSync(file).size > MAX_LEDGER) throw new Error('Ledger exceeds size limit');
      const text = readFileSync(file, 'utf8'); ledgerBytes = Buffer.byteLength(text);
      if (text && !text.endsWith('\n')) throw new Error('Incomplete ledger tail; preserve evidence, no automatic repair');
      events = text.trim() ? text.trimEnd().split('\n').map(line => JSON.parse(line)) : [];
      if (events.length > 10000) throw new Error('Ledger exceeds event limit');
      for (const [i, event] of events.entries()) {
        if (event.seq !== i + 1 || !event.type) throw new Error('Invalid ledger sequence');
        if (event.snapshot) current = event.snapshot;
      }
      if (events.length && (!current?.synthetic || !current.basketId)) throw new Error('Invalid ledger state');
    }
    if (!current) record('reset', {}, initial());
    // Pending sockets are not durable commands. A restart never executes them.
    const next = clone(current); next.nextAction = 'normal';
    record('server-start', { pendingPolicy: 'abandon; never replay' }, next);
  } catch (error) { unlinkSync(lock); throw error; }

  function send(res, status, value) {
    if (res.destroyed) return;
    res.writeHead(status, { 'content-type': 'application/json; charset=utf-8' });
    res.end(JSON.stringify({ synthetic: true, ...value }));
  }
  function loseResponse(res) {
    if (res.destroyed) return;
    // An empty connection close can cause Chromium to transparently repeat a
    // POST. Start the response, then truncate its body: no usable outcome and
    // no pre-response transport retry on the tested browser.
    res.writeHead(200, { 'content-type': 'application/json', 'content-length': '128' });
    res.write('{"synthetic":true,', () => res.destroy());
  }
  const snapshot = () => ({ ...clone(current), observedAt: new Date().toISOString(), ledgerSeq: events.length });
  const product = sku => current.products.find(p => p.sku === sku);
  function actionCheck(input, manual = false) {
    const fields = manual ? ['basketId', 'sku', 'quantity'] : ['basketId', 'sku', 'quantity', 'operation', 'actionId'];
    shape(input, fields, fields);
    if (typeof input.basketId !== 'string' || !product(input.sku) || !integer(input.quantity, 0, 99)) fail(400, 'Invalid basket, SKU or quantity');
    if (!manual && (!['add', 'set'].includes(input.operation) ||
        typeof input.actionId !== 'string' || !/^[A-Za-z0-9_-]{1,80}$/.test(input.actionId) ||
        (input.operation === 'add' && input.quantity === 0))) fail(400, 'Invalid action');
  }
  function commit(input, res, requestId, manual = false) {
    if (input.basketId !== current.basketId) fail(409, 'Basket changed');
    if (!current.loggedIn) fail(401, 'Simulated sign-in required; use test controls');
    const before = current.basket[input.sku] || 0;
    const after = manual || input.operation === 'set' ? input.quantity : before + input.quantity;
    if (after > 99) fail(400, 'Quantity exceeds fixture limit');
    if (after > before && !product(input.sku).inStock) fail(409, 'Simulated out of stock');
    const next = clone(current); next.basket[input.sku] = after; next.revision++;
    record('mutation', { requestId, actionId: input.actionId ?? null, actor: manual ? 'manual' : 'worker',
      sku: input.sku, before, after }, next);
    return { requestId, basketId: current.basketId, quantity: after, revision: current.revision };
  }
  function park(stage, requestId, res, resume) {
    if (pending.size >= 16) fail(429, 'Too many pending actions');
    pending.set(requestId, { stage, res, resume });
    record('gate-wait', { requestId, stage });
    // A disconnected browser does not reliably cancel a retailer request. Keep a
    // before gate executable until explicit discard/reset/restart; expose it.
  }
  function runAction(input, res, requestId, mode) {
    try {
      if (mode === 'never-commit') { record('not-committed', { requestId }); loseResponse(res); return; }
      const result = commit(input, res, requestId);
      if (mode === 'raw-drop-after-commit') { record('ack-dropped', { requestId, transport: 'raw-pre-response-close' }); res.destroy(); }
      else if (mode === 'drop-after-commit') { record('ack-dropped', { requestId, transport: 'truncated-response' }); loseResponse(res); }
      else if (mode === 'hold-after-commit') park('ack', requestId, res, () => send(res, 200, result));
      else send(res, 200, result);
    } catch (error) { send(res, error.status || 500, { error: error.status ? error.message : 'Ledger failure; inspect run' }); }
  }
  function applyControl(input) {
    shape(input, ['nextAction', 'readFailures', 'loggedIn', 'product', 'release', 'discard']);
    if (!Object.keys(input).length) fail(400, 'Empty control');
    const next = clone(current);
    if ('nextAction' in input) { if (!MODES.includes(input.nextAction)) fail(400, 'Invalid mode'); next.nextAction = input.nextAction; }
    if ('readFailures' in input) { if (!integer(input.readFailures, 0, 100)) fail(400, 'Invalid failure count'); next.readFailures = input.readFailures; }
    if ('loggedIn' in input) { if (typeof input.loggedIn !== 'boolean') fail(400, 'Invalid login state'); next.loggedIn = input.loggedIn; }
    if ('product' in input) {
      shape(input.product, ['sku', 'inStock', 'pricePence', 'labelStatus'], ['sku']);
      const p = next.products.find(p => p.sku === input.product.sku);
      if (!p) fail(400, 'Unknown SKU');
      if ('inStock' in input.product) { if (typeof input.product.inStock !== 'boolean') fail(400, 'Invalid stock'); p.inStock = input.product.inStock; }
      if ('pricePence' in input.product) { if (input.product.pricePence !== null && !integer(input.product.pricePence, 0, 100000)) fail(400, 'Invalid price'); p.pricePence = input.product.pricePence; }
      if ('labelStatus' in input.product) {
        if (!['known', 'unknown', 'contradictory'].includes(input.product.labelStatus)) fail(400, 'Invalid label');
        p.label = input.product.labelStatus === 'known' ? initial().products.find(original => original.sku === p.sku).label :
        { status: input.product.labelStatus, ingredients: input.product.labelStatus === 'unknown' ? null : ['Synthetic wheat'],
          allergens: input.product.labelStatus === 'unknown' ? null : input.product.labelStatus === 'contradictory' ? ['wheat'] : [],
          evidence: input.product.labelStatus === 'contradictory' ? 'Synthetic front: gluten-free; synthetic ingredients: wheat' : 'Synthetic fixture label' };
      }
    }
    for (const key of ['release', 'discard']) if (key in input && !['before', 'ack'].includes(input[key])) fail(400, 'Invalid gate');
    if ('release' in input && 'discard' in input) fail(400, 'Choose release or discard');
    record('control', { control: input }, next);
    for (const [id, item] of pending) {
      if (item.stage === input.release || item.stage === input.discard) {
        record(input.discard ? 'gate-discard' : 'gate-release', { requestId: id, stage: item.stage });
        pending.delete(id);
        if (input.discard) loseResponse(item.res); else item.resume();
      }
    }
  }
  const assets = new Map([['/', ['text/html; charset=utf-8', new URL('./shop.html', import.meta.url)]],
    ['/shop.js', ['text/javascript; charset=utf-8', new URL('./shop.js', import.meta.url)]]]);
  const server = createServer({ maxHeaderSize: 8192, requestTimeout: 10000, headersTimeout: 10000 }, async (req, res) => {
    res.setHeader('cache-control', 'no-store');
    res.setHeader('x-content-type-options', 'nosniff');
    res.setHeader('content-security-policy', "default-src 'none'; script-src 'self'; connect-src 'self'; style-src 'unsafe-inline'; frame-ancestors 'none'; base-uri 'none'; form-action 'none'");
    try {
      const host = `127.0.0.1:${server.address().port}`;
      if (req.headers.host !== host || (req.headers.origin && req.headers.origin !== `http://${host}`) ||
          req.headers['sec-fetch-site'] === 'cross-site') fail(403, 'Loopback same-origin requests only');
      if (req.method === 'GET') {
        if (assets.has(req.url)) { const [type, path] = assets.get(req.url); res.writeHead(200, { 'content-type': type }); res.end(readFileSync(path)); return; }
        if (req.url === '/api/admin') return send(res, 200, { state: snapshot(), pending: [...pending].map(([requestId, p]) => ({ requestId, stage: p.stage, disconnected: p.res.destroyed })) });
        if (req.url === '/api/ledger') return send(res, 200, { events });
        if (req.url === '/api/state') {
          if (current.readFailures) { const next = clone(current); next.readFailures--; record('read-failure', {}, next); fail(503, 'Simulated read failure'); }
          if (!current.loggedIn) fail(401, 'Simulated sign-in required; no credentials accepted');
          return send(res, 200, snapshot());
        }
        fail(404, 'Synthetic test shop: route not found');
      }
      if (req.method !== 'POST') fail(405, 'Method not supported');
      const input = await body(req);
      if (req.url === '/api/control') { applyControl(input); return send(res, 200, { state: snapshot() }); }
      if (req.url === '/api/reset') {
        shape(input, ['confirm'], ['confirm']);
        if (input.confirm !== 'RESET SYNTHETIC SHOP') fail(400, 'Explicit reset confirmation required');
        record('reset', { previousRunId: current.runId }, initial());
        for (const item of pending.values()) item.res.destroy(); pending.clear();
        return send(res, 200, { state: snapshot() });
      }
      if (req.url === '/api/manual' || req.url === '/api/action') {
        const manual = req.url === '/api/manual'; actionCheck(input, manual);
        if (pending.size >= 16) fail(429, 'Too many pending actions');
        const requestId = crypto.randomUUID();
        const mode = manual ? 'normal' : current.nextAction;
        const next = clone(current); if (!manual) next.nextAction = 'normal';
        record('action-received', { requestId, actionId: input.actionId ?? null, mode, actor: manual ? 'manual' : 'worker', input }, next);
        if (manual) return send(res, 200, commit(input, res, requestId, true));
        if (mode === 'gate-before-commit') park('before', requestId, res, () => runAction(input, res, requestId, 'normal'));
        else runAction(input, res, requestId, mode);
        return;
      }
      fail(404, 'Synthetic test shop: route not found');
    } catch (error) { send(res, error.status || 500, { error: error.status ? error.message : 'Fixture failure; inspect retained ledger' }); }
  });
  server.maxConnections = 64;
  server.on('clientError', (_error, socket) => socket.destroy());
  try { await new Promise((resolve, reject) => { server.once('error', reject); server.listen(port, '127.0.0.1', resolve); }); }
  catch (error) { unlinkSync(lock); throw error; }
  let closing;
  return { url: `http://127.0.0.1:${server.address().port}`, dataDir,
    close() {
      if (!closing) closing = new Promise(resolve => {
        server.close(() => { unlinkSync(lock); resolve(); });
        for (const item of pending.values()) item.res.destroy(); pending.clear();
        server.closeAllConnections();
      });
      return closing;
    } };
}
