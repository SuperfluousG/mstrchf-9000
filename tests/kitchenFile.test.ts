import { mkdtemp, readdir, readFile, rm } from 'node:fs/promises';
import type { IncomingMessage, ServerResponse } from 'node:http';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { Readable } from 'node:stream';
import { afterEach, beforeEach, describe, expect, it } from 'vitest';
import { kitchenFileMiddleware, ROUTE } from '../server/kitchenFile';
import { chooseState } from '../src/state/fileSync';
import { daysSinceMonday } from '../src/domain/weeks';

type Res = { status: number; body: any; headers: Record<string, string> };

function call(mw: ReturnType<typeof kitchenFileMiddleware>, method: string, body?: unknown, headers: Record<string, string> = {}, url = ROUTE): Promise<Res | 'next'> {
  const req = Readable.from(body === undefined ? [] : [Buffer.from(typeof body === 'string' ? body : JSON.stringify(body))]) as unknown as IncomingMessage;
  Object.assign(req, { method, url, headers });
  return new Promise(resolve => {
    const out: Res = { status: 0, body: null, headers: {} };
    const res = {
      set statusCode(v: number) { out.status = v; },
      setHeader(k: string, v: string) { out.headers[k] = v; },
      end(text: string) { out.body = JSON.parse(text); resolve(out); },
    } as unknown as ServerResponse;
    void mw(req, res, () => resolve('next'));
  });
}

const kitchen = (savedAt: number, weeks: Record<string, unknown> = { '2026-11-09': {} }, profile: unknown = { people: 1 }) =>
  ({ savedAt, state: { version: 2, profile, weeks, favourites: [], hidden: [], ratings: {}, cupboard: [] } });

describe('kitchen file API', () => {
  let dir: string;
  let mw: ReturnType<typeof kitchenFileMiddleware>;
  beforeEach(async () => { dir = await mkdtemp(join(tmpdir(), 'mstrchf-')); mw = kitchenFileMiddleware(dir); });
  afterEach(async () => { await rm(dir, { recursive: true, force: true }); });

  it('ignores other routes', async () => {
    expect(await call(mw, 'GET', undefined, {}, '/src/main.tsx')).toBe('next');
  });

  it('reports no kitchen before the first save, then returns what was saved', async () => {
    const first = await call(mw, 'GET') as Res;
    expect(first.status).toBe(200);
    expect(first.body.state).toBeNull();
    expect((await call(mw, 'PUT', kitchen(100)) as Res).status).toBe(200);
    const again = await call(mw, 'GET') as Res;
    expect(again.body.savedAt).toBe(100);
    expect(again.body.state.profile).toEqual({ people: 1 });
    expect(JSON.parse(await readFile(join(dir, 'kitchen.json'), 'utf8')).app).toBe('MasterChef 9000');
  });

  it('rejects anything that is not a kitchen', async () => {
    expect((await call(mw, 'PUT', 'not json') as Res).status).toBe(400);
    expect((await call(mw, 'PUT', { savedAt: 1, state: { version: 1, weeks: {} } }) as Res).status).toBe(400);
  });

  it('refuses to replace a saved kitchen with an empty one unless told to start over', async () => {
    await call(mw, 'PUT', kitchen(100));
    const blocked = await call(mw, 'PUT', kitchen(200, {}, null)) as Res;
    expect(blocked.status).toBe(409);
    expect(((await call(mw, 'GET')) as Res).body.savedAt).toBe(100);
    const reset = await call(mw, 'PUT', kitchen(300, {}, null), { 'x-confirm-reset': '1' }) as Res;
    expect(reset.status).toBe(200);
  });

  it('keeps a dated copy of the previous file', async () => {
    await call(mw, 'PUT', kitchen(100));
    await call(mw, 'PUT', kitchen(200));
    const files = await readdir(join(dir, 'backups'));
    expect(files).toHaveLength(1);
    expect(JSON.parse(await readFile(join(dir, 'backups', files[0]), 'utf8')).savedAt).toBe(100);
  });
});

describe('choosing between the browser and the file', () => {
  const empty = (s: { n: number }) => s.n === 0;
  it('uses the file when the browser has been cleared', () => {
    expect(chooseState({ state: { n: 0 }, savedAt: 999 }, { state: { n: 5 }, savedAt: 1 }, empty)).toMatchObject({ from: 'file', state: { n: 5 } });
  });
  it('keeps the newer copy', () => {
    expect(chooseState({ state: { n: 1 }, savedAt: 2 }, { state: { n: 2 }, savedAt: 1 }, empty)).toMatchObject({ from: 'browser', pushToFile: true });
    expect(chooseState({ state: { n: 1 }, savedAt: 1 }, { state: { n: 2 }, savedAt: 2 }, empty)).toMatchObject({ from: 'file', pushToFile: false });
  });
  it('writes the browser copy when there is no file yet', () => {
    expect(chooseState({ state: { n: 1 }, savedAt: 0 }, { state: null, savedAt: 0 }, empty)).toMatchObject({ from: 'browser', pushToFile: true });
  });
});

describe('days since Monday', () => {
  it('counts across months', () => {
    expect(daysSinceMonday('2026-11-09', new Date(2026, 10, 11, 18))).toBe(2);
    expect(daysSinceMonday('2026-11-30', new Date(2026, 11, 6))).toBe(6);
    expect(daysSinceMonday('2026-11-16', new Date(2026, 10, 9))).toBe(-7);
  });
});
