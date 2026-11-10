import { createHash } from 'node:crypto';
import { copyFile, mkdir, open, readFile, readdir, rename, stat, unlink, writeFile } from 'node:fs/promises';
import { homedir } from 'node:os';
import { basename, join, resolve } from 'node:path';
import { readBackup } from '../src/domain/backup';
import { buildBasket } from '../src/domain/basket';
import { EMPTY_STATE, type AppState } from '../src/domain/kitchen';
import { exclusionReason, EMPTY_FOCUS, type PlanContext } from '../src/domain/planner';
import { marketOf, retailerOf } from '../src/domain/profile';
import { buildShoppingList } from '../src/domain/shopping';
import type { CodexLibrary } from './codexLibrary';

export function defaultKitchenDir(): string {
  if (process.env.MSTRCHF_DATA_DIR) return resolve(process.env.MSTRCHF_DATA_DIR);
  if (process.platform === 'win32') return join(process.env.LOCALAPPDATA ?? join(homedir(), 'AppData', 'Local'), 'MSTRCHF9000');
  if (process.platform === 'darwin') return join(homedir(), 'Library', 'Application Support', 'MSTRCHF9000');
  return join(process.env.XDG_DATA_HOME ?? join(homedir(), '.local', 'share'), 'mstrchf9000');
}
export interface KitchenSnapshot { state: AppState | null; savedAt: number; revision: string; path: string }
const sleep = (n: number) => new Promise(resolve => setTimeout(resolve, n));
const code = (e: unknown) => (e as NodeJS.ErrnoException).code;

/** A private per-user file store, shared by the app and chat tools. All writes use a revision and file lock. */
export class CodexKitchen {
  readonly file: string;
  constructor(readonly dir: string, readonly library: CodexLibrary) { this.file = join(dir, 'kitchen.json'); }
  async read(): Promise<KitchenSnapshot> {
    try {
      const raw = await readFile(this.file, 'utf8');
      const envelope = JSON.parse(raw) as { savedAt?: number };
      return { state: readBackup(raw), savedAt: envelope.savedAt ?? 0, revision: createHash('sha256').update(raw).digest('hex'), path: this.file };
    } catch (e) {
      if (code(e) === 'ENOENT') return { state: null, savedAt: 0, revision: 'empty', path: this.file };
      throw e;
    }
  }
  async update(change: (current: AppState) => AppState, expectedRevision?: string, replace = false): Promise<KitchenSnapshot> {
    await mkdir(this.dir, { recursive: true });
    const lock = join(this.dir, 'kitchen.lock');
    let handle;
    const deadline = Date.now() + 4000;
    while (!handle) {
      try { handle = await open(lock, 'wx'); await handle.writeFile(JSON.stringify({ pid: process.pid, time: Date.now() })); }
      catch (e) {
        if (code(e) !== 'EEXIST') throw e;
        try {
          const held = JSON.parse(await readFile(lock, 'utf8')) as { pid: number };
          if (Date.now() - (await stat(lock)).mtimeMs > 15_000) {
            try { process.kill(held.pid, 0); } catch (err) { if (code(err) === 'ESRCH') { await unlink(lock); continue; } }
          }
        } catch { /* A lock being created or removed is retried. */ }
        if (Date.now() > deadline) throw new Error('Your kitchen is busy in another view. Try again in a moment.');
        await sleep(40);
      }
    }
    try {
      const before = await this.read();
      if (expectedRevision !== undefined && before.revision !== expectedRevision) throw new Error('The kitchen changed in another view. Reload the planner before retrying; export a backup first to keep your current changes.');
      const next = readBackup(JSON.stringify(change(structuredClone(before.state ?? EMPTY_STATE))));
      if (!replace) this.checkTransition(before.state, next);
      const savedAt = Math.max(Date.now(), before.savedAt + 1);
      next.savedAt = savedAt;
      if (before.state) await this.backup();
      const temp = `${this.file}.${process.pid}.tmp`;
      await writeFile(temp, JSON.stringify({ app: 'MSTRCHF 9000', savedAt, state: next }, null, 2), 'utf8');
      await rename(temp, this.file);
      return await this.read();
    } finally { await handle.close(); await unlink(lock).catch(() => {}); }
  }
  checkTransition(before: AppState | null, next: AppState) {
    if (!next.profile && !Object.keys(next.weeks).length && before?.profile) throw new Error('Use the explicit restore or reset action to replace a saved kitchen with an empty one.');
    for (const [id, old] of Object.entries(before?.weeks ?? {})) {
      if (old.status !== 'ordered') continue;
      const w = next.weeks[id];
      if (!w) throw new Error('Reopen an ordered week before removing it.');
      if (w.status === 'draft') continue;
      const withoutDays = (p: typeof old) => ({ ...p, picks: p.picks.map(({ day: _day, ...pick }) => pick).sort((a, b) => a.uid.localeCompare(b.uid)) });
      if (JSON.stringify(withoutDays(old)) !== JSON.stringify(withoutDays(w))) throw new Error('This week is ordered. Reopen it from Shop before changing meals or its recorded basket.');
    }
    for (const [id, w] of Object.entries(next.weeks)) {
      if (w.status !== 'ordered' || before?.weeks[id]?.status === 'ordered') continue;
      if (!next.profile || !w.picks.length || !w.orderedBasket) throw new Error('Record an ordered basket from Shop after planning meals.');
      const cat = this.library.forMarket(marketOf(next.profile));
      const ctx: PlanContext = { catalogue: cat, profile: next.profile, recency: new Map(), favourites: new Set(), ratings: new Map(), hidden: new Set(), focus: EMPTY_FOCUS, seed: w.seed };
      const recipes = w.picks.map(p => cat.byId.get(p.recipeId));
      if (recipes.some(r => !r || exclusionReason(r, ctx))) throw new Error('Review excluded or unavailable meals before ordering.');
      const list = buildShoppingList({ recipes: recipes.filter(r => !!r), people: next.profile.people, have: new Set([...next.cupboard, ...w.have]), pantryToBuy: new Set(w.pantryToBuy ?? []) }, cat);
      const expected = buildBasket(list, id, next.profile.people, recipes.map(r => r!.title), cat.market, retailerOf(next.profile));
      const { createdAt: _expectedTime, ...a } = expected;
      const { createdAt: _providedTime, ...b } = w.orderedBasket;
      if (JSON.stringify(a) !== JSON.stringify(b)) throw new Error('The basket estimate changed. Reopen Shop and record the current plan.');
    }
  }
  private async backup() {
    const dir = join(this.dir, 'backups'); await mkdir(dir, { recursive: true });
    const name = `kitchen-${new Date().toISOString().slice(0, 10)}.json`;
    const files = (await readdir(dir)).filter(f => /^kitchen-\d{4}-\d{2}-\d{2}\.json$/.test(f)).sort();
    if (!files.includes(name)) { await copyFile(this.file, join(dir, name)); files.push(name); }
    for (const old of files.sort().slice(0, Math.max(0, files.length - 14))) await unlink(join(dir, old));
  }
  async exportFile(filename: string, text: string): Promise<string> {
    if (filename !== basename(filename) || !/^[a-zA-Z0-9][a-zA-Z0-9._-]{0,120}\.json$/.test(filename)) throw new Error('Choose a simple JSON filename.');
    if (Buffer.byteLength(text) > 4_500_000 || JSON.parse(text)?.app !== 'MSTRCHF 9000') throw new Error('Only MSTRCHF JSON backups and baskets can be exported.');
    const folder = join(this.dir, 'exports'); await mkdir(folder, { recursive: true });
    const name = filename.replace(/\.json$/, `-${Date.now()}.json`);
    const path = join(folder, name); await writeFile(path, text, { encoding: 'utf8', flag: 'wx' });
    return path;
  }
}
