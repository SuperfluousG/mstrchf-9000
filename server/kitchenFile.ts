// Saves the kitchen (profile, plans, favourites) to a JSON file on this computer.
// Mounted on the Vite dev and preview servers at /api/kitchen; both listen on 127.0.0.1 only.
import { copyFile, mkdir, readdir, readFile, rename, unlink, writeFile } from 'node:fs/promises';
import type { IncomingMessage, ServerResponse } from 'node:http';
import { join } from 'node:path';

export const ROUTE = '/api/kitchen';
export const MAX_BYTES = 5_000_000;
export const KEEP_DAILY_BACKUPS = 14;

export interface KitchenDoc {
  app: 'MasterChef 9000';
  savedAt: number;
  state: { version: 2; profile: unknown; weeks: Record<string, unknown> } & Record<string, unknown>;
}

function isKitchen(doc: unknown): doc is Pick<KitchenDoc, 'savedAt' | 'state'> {
  const d = doc as KitchenDoc | null;
  return !!d && typeof d.savedAt === 'number' && !!d.state && d.state.version === 2
    && typeof d.state.weeks === 'object' && d.state.weeks !== null && !Array.isArray(d.state.weeks);
}

function isEmpty(state: KitchenDoc['state']): boolean {
  return !state.profile && Object.keys(state.weeks).length === 0;
}

function readBody(req: IncomingMessage): Promise<string> {
  return new Promise((resolve, reject) => {
    let size = 0;
    const chunks: Buffer[] = [];
    req.on('data', (c: Buffer) => {
      size += c.length;
      if (size > MAX_BYTES) { reject(new Error('Kitchen file is too large')); req.destroy(); return; }
      chunks.push(c);
    });
    req.on('end', () => resolve(Buffer.concat(chunks).toString('utf8')));
    req.on('error', reject);
  });
}

export function kitchenFileMiddleware(dataDir: string) {
  const file = join(dataDir, 'kitchen.json');
  const backups = join(dataDir, 'backups');

  async function readDoc(): Promise<KitchenDoc | null> {
    try {
      return JSON.parse(await readFile(file, 'utf8')) as KitchenDoc;
    } catch (e) {
      if ((e as NodeJS.ErrnoException).code === 'ENOENT') return null;
      throw e;
    }
  }

  /** Keeps one copy per day of the file as it was before that day's first save. */
  async function dailyBackup(): Promise<void> {
    await mkdir(backups, { recursive: true });
    const name = `kitchen-${new Date().toISOString().slice(0, 10)}.json`;
    const existing = (await readdir(backups)).filter(f => /^kitchen-\d{4}-\d{2}-\d{2}\.json$/.test(f)).sort();
    if (!existing.includes(name)) {
      try { await copyFile(file, join(backups, name)); existing.push(name); } catch { /* nothing saved yet */ }
    }
    for (const old of existing.sort().slice(0, Math.max(0, existing.length - KEEP_DAILY_BACKUPS))) {
      await unlink(join(backups, old)).catch(() => undefined);
    }
  }

  return async (req: IncomingMessage, res: ServerResponse, next: () => void): Promise<void> => {
    if (!req.url || req.url.split('?')[0] !== ROUTE) return next();
    const send = (code: number, body: unknown) => {
      res.statusCode = code;
      res.setHeader('content-type', 'application/json; charset=utf-8');
      res.setHeader('cache-control', 'no-store');
      res.end(JSON.stringify(body));
    };
    try {
      if (req.method === 'GET') {
        const doc = await readDoc();
        return send(200, { state: doc?.state ?? null, savedAt: doc?.savedAt ?? 0, path: file });
      }
      if (req.method === 'PUT') {
        let doc: unknown;
        try { doc = JSON.parse(await readBody(req)); } catch (e) { return send(400, { error: e instanceof Error ? e.message : 'Unreadable kitchen' }); }
        if (!isKitchen(doc)) return send(400, { error: 'That is not a MasterChef 9000 kitchen.' });
        const current = await readDoc();
        // Never let an empty browser silently overwrite a saved kitchen; "Start over" sends this header.
        if (current && isKitchen(current) && !isEmpty(current.state) && isEmpty(doc.state) && req.headers['x-confirm-reset'] !== '1') {
          return send(409, { error: 'Refusing to replace a saved kitchen with an empty one.' });
        }
        await mkdir(dataDir, { recursive: true });
        if (current) await dailyBackup();
        const tmp = `${file}.tmp`;
        await writeFile(tmp, JSON.stringify({ app: 'MasterChef 9000', savedAt: doc.savedAt, state: doc.state }, null, 2), 'utf8');
        await rename(tmp, file);
        return send(200, { ok: true, savedAt: doc.savedAt, path: file });
      }
      res.setHeader('allow', 'GET, PUT');
      return send(405, { error: 'Method not allowed' });
    } catch (e) {
      return send(500, { error: e instanceof Error ? e.message : 'Could not use the kitchen file' });
    }
  };
}
