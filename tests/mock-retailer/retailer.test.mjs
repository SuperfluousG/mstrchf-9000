import { test } from 'node:test';
import { request } from 'node:http';
import { appendFileSync, mkdtempSync, readFileSync } from 'node:fs';
import { start } from './server.mjs';

function call(shop, path, body, extra = {}) {
  return new Promise((resolve, reject) => {
    const payload = body === undefined ? undefined : JSON.stringify(body);
    const req = request(shop.url + path, { method: payload ? 'POST' : 'GET',
      headers: { ...(payload ? { 'content-type': 'application/json' } : {}), ...extra } }, res => {
      let text = '';
      res.on('aborted', () => reject(new Error('response aborted')));
      res.on('data', b => { text += b; });
      res.on('end', () => { let data; try { data = JSON.parse(text); } catch { data = text; }
        resolve({ status: res.statusCode, data }); });
    });
    req.on('error', reject);
    req.setTimeout(2500, () => req.destroy(new Error('request timed out')));
    req.end(payload);
  });
}
async function fixture(t) {
  const dir = mkdtempSync(`${process.env.TEMP || process.env.TMPDIR || '/tmp'}/mstrchf-mock-test-`);
  const shop = await start({ dataDir: dir });
  t.after(() => shop.close());
  return shop;
}
const state = async shop => (await call(shop, '/api/state')).data;
const admin = async shop => (await call(shop, '/api/admin')).data;
const ledger = async shop => (await call(shop, '/api/ledger')).data.events;
const control = (shop, body) => call(shop, '/api/control', body);
const add = (shop, basketId, actionId, more = {}) => call(shop, '/api/action',
  { basketId, actionId, sku: 'beans', operation: 'add', quantity: 1, ...more });
async function waitFor(shop, predicate) {
  for (let n = 0; n < 100; n++) {
    const value = await admin(shop);
    if (predicate(value)) return value;
    await new Promise(resolve => setTimeout(resolve, 10));
  }
  throw new Error('gate did not reach expected state');
}

test('shop exposes exact synthetic product evidence and unrelated baseline goods', async t => {
  const shop = await fixture(t); const result = await call(shop, '/api/state');
  t.assert.equal(result.status, 200);
  t.assert.equal(result.data.synthetic, true);
  t.assert.match(result.data.basketId, /^[a-f0-9-]{36}$/);
  t.assert.deepEqual(result.data.basket, { beans: 1, soap: 2 });
  const beans = result.data.products.find(p => p.sku === 'beans');
  t.assert.equal(beans.netGrams, 400); t.assert.equal(beans.drainedGrams, 240);
  t.assert.equal(beans.pricePence, 80); t.assert.equal(beans.offer.requiresLoyalty, true);
  t.assert.equal(beans.label.status, 'known');
  t.assert.match((await call(shop, '/')).data, /Synthetic test shop/);
});

test('add, set and manual edits preserve unrelated and same-SKU contributions with ordered ledger', async t => {
  const shop = await fixture(t); const { basketId } = await state(shop);
  t.assert.equal((await add(shop, basketId, 'first')).status, 200);
  t.assert.equal((await call(shop, '/api/manual', { basketId, sku: 'beans', quantity: 5 })).status, 200);
  await call(shop, '/api/manual', { basketId, sku: 'soap', quantity: 3 });
  await add(shop, basketId, 'second');
  t.assert.deepEqual((await state(shop)).basket, { beans: 6, soap: 3 });
  await add(shop, basketId, 'set', { operation: 'set', quantity: 0 });
  t.assert.deepEqual((await state(shop)).basket, { beans: 0, soap: 3 });
  const events = await ledger(shop);
  t.assert.deepEqual(events.filter(e => e.type === 'mutation').map(e => [e.actor, e.sku, e.before, e.after]),
    [['worker', 'beans', 1, 2], ['manual', 'beans', 2, 5], ['manual', 'soap', 2, 3], ['worker', 'beans', 5, 6], ['worker', 'beans', 6, 0]]);
  t.assert.deepEqual(events.map(e => e.seq), events.map((_, i) => i + 1));
});

test('lost acknowledgement commits once per request and ledger survives server restart', async t => {
  const shop = await fixture(t); const { basketId } = await state(shop);
  await control(shop, { nextAction: 'drop-after-commit' });
  await t.assert.rejects(add(shop, basketId, 'lost'));
  t.assert.equal((await state(shop)).basket.beans, 2);
  const disk = readFileSync(`${shop.dataDir}/ledger.jsonl`, 'utf8');
  t.assert.match(disk, /"type":"mutation"/);
  await shop.close();
  const restarted = await start({ dataDir: shop.dataDir }); t.after(() => restarted.close());
  t.assert.equal((await state(restarted)).basketId, basketId);
  t.assert.equal((await state(restarted)).basket.beans, 2);
  t.assert.equal((await ledger(restarted)).filter(e => e.type === 'mutation').length, 1);
  // The fixture intentionally does NOT deduplicate client action IDs.
  await add(restarted, basketId, 'lost');
  t.assert.equal((await state(restarted)).basket.beans, 3);
});

test('never-commit loses response without mutation; next action is normal', async t => {
  const shop = await fixture(t); const { basketId } = await state(shop);
  await control(shop, { nextAction: 'never-commit' });
  await t.assert.rejects(add(shop, basketId, 'absent'));
  t.assert.equal((await state(shop)).basket.beans, 1);
  t.assert.equal((await ledger(shop)).filter(e => e.type === 'mutation').length, 0);
  t.assert.equal((await add(shop, basketId, 'normal')).status, 200);
  t.assert.equal((await state(shop)).basket.beans, 2);
});

test('withheld acknowledgement is controllable while committed state remains readable', async t => {
  const shop = await fixture(t); const { basketId } = await state(shop);
  await control(shop, { nextAction: 'hold-after-commit' });
  let settled = false;
  const pending = add(shop, basketId, 'held').then(x => { settled = true; return x; });
  await waitFor(shop, x => x.pending.some(p => p.stage === 'ack'));
  t.assert.equal(settled, false); t.assert.equal((await state(shop)).basket.beans, 2);
  await control(shop, { release: 'ack' });
  t.assert.equal((await pending).status, 200);
  t.assert.equal((await ledger(shop)).filter(e => e.type === 'mutation').length, 1);
});

test('second action waits before commit, then sees manual edit at release', async t => {
  const shop = await fixture(t); const { basketId } = await state(shop);
  await add(shop, basketId, 'first');
  await control(shop, { nextAction: 'gate-before-commit' });
  const pending = add(shop, basketId, 'second');
  await waitFor(shop, x => x.pending.some(p => p.stage === 'before'));
  t.assert.equal((await state(shop)).basket.beans, 2);
  await call(shop, '/api/manual', { basketId, sku: 'beans', quantity: 7 });
  await control(shop, { release: 'before' });
  t.assert.equal((await pending).status, 200);
  t.assert.equal((await state(shop)).basket.beans, 8);
  t.assert.deepEqual((await ledger(shop)).filter(e => e.type === 'mutation').map(e => e.actor), ['worker', 'manual', 'worker']);
});

test('gate abandonment and reset never replay a queued action', async t => {
  const shop = await fixture(t); const old = await state(shop);
  await control(shop, { nextAction: 'gate-before-commit' });
  const pending = add(shop, old.basketId, 'queued').catch(() => ({ status: 0 }));
  await waitFor(shop, x => x.pending.length === 1);
  await call(shop, '/api/reset', { confirm: 'RESET SYNTHETIC SHOP' });
  await pending;
  t.assert.notEqual((await state(shop)).basketId, old.basketId);
  await control(shop, { release: 'before' });
  t.assert.equal((await state(shop)).basket.beans, 1);
  t.assert.equal((await add(shop, old.basketId, 'stale')).status, 409);
  t.assert.equal((await ledger(shop)).filter(e => e.type === 'mutation').length, 0);
});

test('restart preserves a held commit but abandons a before-commit gate', async t => {
  const shop = await fixture(t); const { basketId } = await state(shop);
  await control(shop, { nextAction: 'hold-after-commit' });
  const first = add(shop, basketId, 'held').catch(() => undefined);
  await waitFor(shop, x => x.pending.length === 1);
  await control(shop, { nextAction: 'gate-before-commit' });
  const second = add(shop, basketId, 'queued').catch(() => undefined);
  await waitFor(shop, x => x.pending.length === 2);
  await shop.close(); await Promise.all([first, second]);
  const next = await start({ dataDir: shop.dataDir }); t.after(() => next.close());
  t.assert.equal((await state(next)).basket.beans, 2);
  t.assert.deepEqual((await admin(next)).pending, []);
  await control(next, { release: 'before' });
  t.assert.equal((await state(next)).basket.beans, 2);
});

test('read failures are counted and do not hide independent admin evidence', async t => {
  const shop = await fixture(t);
  await control(shop, { readFailures: 2 });
  t.assert.equal((await call(shop, '/api/state')).status, 503);
  t.assert.equal((await admin(shop)).state.basket.beans, 1);
  t.assert.equal((await call(shop, '/api/state')).status, 503);
  t.assert.equal((await call(shop, '/api/state')).status, 200);
  t.assert.equal((await ledger(shop)).filter(e => e.type === 'read-failure').length, 2);
});

test('simulated login, stock, label and price changes are observable and block unavailable adds', async t => {
  const shop = await fixture(t); const { basketId } = await state(shop);
  await control(shop, { loggedIn: false });
  t.assert.equal((await call(shop, '/api/state')).status, 401);
  t.assert.equal((await add(shop, basketId, 'denied')).status, 401);
  await control(shop, { loggedIn: true, product: { sku: 'beans', inStock: false, pricePence: 120, labelStatus: 'unknown' } });
  const beans = (await state(shop)).products.find(p => p.sku === 'beans');
  t.assert.equal(beans.pricePence, 120); t.assert.equal(beans.label.status, 'unknown');
  t.assert.equal((await add(shop, basketId, 'stock')).status, 409);
  await control(shop, { product: { sku: 'beans', inStock: true, labelStatus: 'contradictory', pricePence: null } });
  t.assert.equal((await state(shop)).products.find(p => p.sku === 'beans').pricePence, null);
  t.assert.equal((await state(shop)).products.find(p => p.sku === 'beans').label.status, 'contradictory');
  t.assert.equal((await ledger(shop)).filter(e => e.type === 'mutation').length, 0);
});

test('gated action rechecks login and stock after release', async t => {
  const shop = await fixture(t); const { basketId } = await state(shop);
  await control(shop, { nextAction: 'gate-before-commit' });
  const pending = add(shop, basketId, 'gate');
  await waitFor(shop, x => x.pending.length === 1);
  await control(shop, { loggedIn: false }); await control(shop, { release: 'before' });
  t.assert.equal((await pending).status, 401);
  t.assert.equal((await admin(shop)).state.basket.beans, 1);
});

test('validation, request bounds and cross-origin rejection have no mutation side effects', async t => {
  const shop = await fixture(t); const { basketId } = await state(shop);
  for (const more of [{ quantity: -1 }, { quantity: 1.5 }, { quantity: 1000 }, { sku: '__proto__' }, { operation: 'checkout' }, { extra: 'ignored?' }]) {
    t.assert.equal((await add(shop, basketId, 'invalid', more)).status, 400);
  }
  t.assert.equal((await control(shop, { readFailures: -1 })).status, 400);
  t.assert.equal((await control(shop, { product: { sku: 'beans', pricePence: -1 } })).status, 400);
  t.assert.equal((await call(shop, '/api/action', { basketId, actionId: 'x'.repeat(17000) })).status, 413);
  t.assert.equal((await call(shop, '/api/reset', { confirm: 'wrong' })).status, 400);
  t.assert.equal((await call(shop, '/api/reset', { confirm: 'RESET SYNTHETIC SHOP' }, { origin: 'https://example.invalid' })).status, 403);
  t.assert.equal((await call(shop, '/api/state', undefined, { host: 'evil.invalid' })).status, 403);
  t.assert.equal((await call(shop, '/api/state', undefined, { 'sec-fetch-site': 'cross-site' })).status, 403);
  t.assert.equal((await call(shop, '/checkout')).status, 404);
  t.assert.deepEqual((await state(shop)).basket, { beans: 1, soap: 2 });
});

test('discard before commit leaves no mutation; discard acknowledgement retains committed quantity', async t => {
  const shop = await fixture(t); const { basketId } = await state(shop);
  for (const [mode, stage, expected] of [['gate-before-commit', 'before', 1], ['hold-after-commit', 'ack', 2]]) {
    await control(shop, { nextAction: mode });
    const pending = add(shop, basketId, stage).then(() => 'ack', () => 'lost');
    await waitFor(shop, x => x.pending.some(p => p.stage === stage));
    await control(shop, { discard: stage });
    t.assert.equal(await pending, 'lost');
    await control(shop, { release: stage });
    t.assert.equal((await state(shop)).basket.beans, expected);
  }
  const events = await ledger(shop);
  t.assert.equal(events.filter(e => e.type === 'gate-discard').length, 2);
  t.assert.equal(events.filter(e => e.type === 'mutation').length, 1);
});

test('client disconnect does not cancel a before gate or silently prove worker stop', async t => {
  const shop = await fixture(t); const { basketId } = await state(shop);
  await control(shop, { nextAction: 'gate-before-commit' });
  const req = request(shop.url + '/api/action', { method: 'POST', headers: { 'content-type': 'application/json' } });
  req.on('error', () => {});
  req.end(JSON.stringify({ basketId, actionId: 'disconnect', sku: 'beans', operation: 'add', quantity: 1 }));
  await waitFor(shop, x => x.pending.length === 1);
  req.destroy();
  await waitFor(shop, x => x.pending[0]?.disconnected);
  t.assert.equal((await state(shop)).basket.beans, 1);
  await control(shop, { release: 'before' });
  t.assert.equal((await state(shop)).basket.beans, 2);
});

test('exclusive data-directory lock rejects a second server; incomplete ledger fails closed', async t => {
  const shop = await fixture(t);
  await t.assert.rejects(start({ dataDir: shop.dataDir }), /already in use/);
  const { basketId } = await state(shop); await add(shop, basketId, 'persisted');
  await shop.close();
  appendFileSync(`${shop.dataDir}/ledger.jsonl`, '{"torn":', { flush: true });
  await t.assert.rejects(start({ dataDir: shop.dataDir }), /Incomplete ledger tail/);
  t.assert.ok(readFileSync(`${shop.dataDir}/ledger.jsonl`, 'utf8').endsWith('{"torn":'));
});

test('operator product changes preserve SKU-specific known labels', async t => {
  const shop = await fixture(t);
  await control(shop, { product: { sku: 'eggs', labelStatus: 'unknown' } });
  await control(shop, { product: { sku: 'eggs', labelStatus: 'known' } });
  const eggs = (await state(shop)).products.find(p => p.sku === 'eggs');
  t.assert.deepEqual(eggs.label.allergens, ['egg']);
  t.assert.deepEqual(eggs.label.ingredients, ['Synthetic egg']);
});

test('loss faults start an incomplete HTTP response to prevent transparent pre-response browser retries', async t => {
  const shop = await fixture(t); const { basketId } = await state(shop);
  for (const mode of ['drop-after-commit', 'never-commit', 'hold-after-commit', 'gate-before-commit']) {
    await control(shop, { nextAction: mode });
    const response = new Promise(resolve => {
      let responseSeen = false;
      const req = request(shop.url + '/api/action', { method: 'POST', headers: { 'content-type': 'application/json' } }, res => {
        responseSeen = true;
        res.resume();
        res.on('aborted', () => resolve({ responseSeen, complete: res.complete }));
        res.on('end', () => resolve({ responseSeen, complete: res.complete }));
      });
      req.on('error', () => resolve({ responseSeen, complete: false }));
      req.end(JSON.stringify({ basketId, actionId: 'browser-loss', sku: 'beans', quantity: 1, operation: 'add' }));
    });
    if (mode === 'hold-after-commit' || mode === 'gate-before-commit') {
      await waitFor(shop, x => x.pending.length === 1);
      await control(shop, { discard: mode === 'hold-after-commit' ? 'ack' : 'before' });
    }
    const result = await response;
    t.assert.equal(result.responseSeen, true);
    t.assert.equal(result.complete, false);
  }
  t.assert.equal((await state(shop)).basket.beans, 3);
});

test('optional raw disconnect closes before response and exposes repeatable client replay risk', async t => {
  const shop = await fixture(t); const { basketId } = await state(shop);
  t.assert.equal((await control(shop, { nextAction: 'raw-drop-after-commit' })).status, 200);
  await t.assert.rejects(add(shop, basketId, 'raw'));
  t.assert.equal((await state(shop)).basket.beans, 2);
  await add(shop, basketId, 'raw');
  t.assert.equal((await state(shop)).basket.beans, 3);
  t.assert.equal((await ledger(shop)).filter(e => e.type === 'mutation').length, 2);
});
