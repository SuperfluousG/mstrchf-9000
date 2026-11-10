// Keeps the kitchen somewhere safer than the browser, as well as in it:
// - in Claude's storage when the app runs as a Claude artifact (each person's copy is private to them), or
// - in a JSON file on this computer when it runs from the local MSTRCHF server (/api/kitchen).
// A plain static copy has neither and stays browser-only.
import { useSyncExternalStore } from 'react';
import { claudeHost } from '../platform';
import { codexCall, isCodexApp } from '../codexHost';

const ROUTE = '/api/kitchen';
const DEBOUNCE_MS = 400;
const LOAD_TIMEOUT_MS = 9000;

export type Store = 'file' | 'claude';

export type FileStatus =
  | { kind: 'checking' }
  | { kind: 'saved'; store: Store; path: string; savedAt: number }
  | { kind: 'saving'; store: Store; path: string }
  | { kind: 'browser-only'; reason?: string }
  | { kind: 'error'; store: Store; path: string; message: string };

export interface Snapshot<S> { state: S | null; savedAt: number }

export interface Choice<S> {
  state: S | null;
  /** Where the chosen state came from. */
  from: 'file' | 'browser' | 'none';
  /** The browser copy is newer than the stored copy, so the store needs updating. */
  pushToFile: boolean;
}

/** Picks the newer of the browser copy and the stored copy. An empty side never beats a full one. */
export function chooseState<S>(browser: Snapshot<S> | null, file: Snapshot<S> | null, isEmpty: (s: S) => boolean, options: { allowEmptyFile?: boolean } = {}): Choice<S> {
  const b = browser?.state && !isEmpty(browser.state) ? browser : null;
  const f = file?.state && (!isEmpty(file.state) || options.allowEmptyFile) ? file : null;
  if (f && (!b || f.savedAt >= b.savedAt)) return { state: f.state, from: 'file', pushToFile: false };
  if (b) return { state: b.state, from: 'browser', pushToFile: true };
  return { state: null, from: 'none', pushToFile: false };
}

let status: FileStatus = { kind: 'checking' };
const listeners = new Set<() => void>();
function setStatus(next: FileStatus) { status = next; listeners.forEach(l => l()); }
export function getFileStatus(): FileStatus { return status; }
export function useFileStatus(): FileStatus {
  return useSyncExternalStore(l => { listeners.add(l); return () => listeners.delete(l); }, getFileStatus, getFileStatus);
}

interface Body { savedAt: number; state: unknown }

/** Where the kitchen is kept outside the browser. */
interface Backend {
  store: Store;
  path: string;
  /** Null when nothing is stored yet. Throws Unavailable when this backend cannot be used here. */
  read(): Promise<Snapshot<unknown> | null>;
  /** Resolves with the saved time. */
  write(body: Body, reset: boolean, keepalive: boolean): Promise<number>;
}

class Unavailable extends Error {}

function timeout<T>(p: Promise<T>, ms: number): Promise<T> {
  return Promise.race([p, new Promise<T>((_, reject) => setTimeout(() => reject(new Unavailable('timed out')), ms))]);
}

function fileBackend(): Backend {
  const b: Backend = {
    store: 'file',
    path: 'kitchen.json',
    async read() {
      const res = await fetch(ROUTE, { cache: 'no-store', signal: AbortSignal.timeout(2500) });
      if (!res.ok || !(res.headers.get('content-type') ?? '').includes('application/json')) throw new Unavailable('no local server');
      const body = await res.json() as Snapshot<unknown> & { path: string };
      b.path = body.path;
      return body.state ? body : null;
    },
    async write(body, reset, keepalive) {
      const text = JSON.stringify(body);
      const res = await fetch(ROUTE, {
        method: 'PUT', body: text, keepalive: keepalive && text.length < 60_000,
        headers: { 'content-type': 'application/json', ...(reset ? { 'x-confirm-reset': '1' } : {}) },
      });
      const json = await res.json().catch(() => ({})) as { savedAt?: number; error?: string };
      if (!res.ok) throw new Error(json.error ?? `Save failed (${res.status})`);
      return json.savedAt ?? Date.now();
    },
  };
  return b;
}

// The slice of the artifact `db` and `user` capabilities this file uses.
interface DocSnap { exists: boolean; data(): Record<string, unknown> | undefined }
interface DocRef { get(): Promise<DocSnap>; set(data: Record<string, unknown>): Promise<void> }
interface Db { doc(path: string): DocRef }
interface User { id(): Promise<string | null>; can?(what: string): Promise<boolean | null> }

const wait = (ms: number) => new Promise(r => setTimeout(r, ms));
const codeOf = (e: unknown) => (e as { code?: string })?.code;

/** The viewer's own private document in the artifact's storage, or null when this view cannot store anything. */
async function claudeBackend(): Promise<Backend | null> {
  if (!claudeHost) return null;
  const [db, user] = await Promise.all([claudeHost.use('db') as Promise<Db | null>, claudeHost.use('user') as Promise<User | null>]);
  const id = user ? await user.id() : null;
  if (!db || !id) return null;
  if ((await user?.can?.('data.write')) === false) return null;
  const ref = db.doc(`data/users/${id}/kitchen`);
  const retry = async <T>(fn: () => Promise<T>): Promise<T> => {
    try { return await fn(); } catch (e) {
      if (codeOf(e) !== 'unavailable') throw e;
      await wait(300 + Math.random() * 700);
      return fn();
    }
  };
  return {
    store: 'claude',
    path: 'your Claude account',
    async read() {
      const snap = await retry(() => ref.get());
      if (!snap.exists) return null;
      // Snapshots are frozen; the app needs its own copy.
      const d = JSON.parse(JSON.stringify(snap.data() ?? {})) as Partial<Body>;
      return d.state ? { state: d.state, savedAt: Number(d.savedAt) || 0 } : null;
    },
    async write(body) {
      await retry(() => ref.set(JSON.parse(JSON.stringify(body)) as Record<string, unknown>));
      return body.savedAt;
    },
  };
}

let backend: Backend | null = null;
let codexRevision = 'empty';
export function acceptCodexRevision(revision: string): boolean {
  if (!isCodexApp || pending || inFlight || status.kind !== 'saved') return false;
  codexRevision = revision;
  return true;
}
function codexBackend(): Backend {
  const b: Backend = {
    store: 'file', path: 'your local kitchen file',
    async read() {
      const snap = await codexCall<Snapshot<unknown> & { path: string; revision: string }>('get_kitchen');
      codexRevision = snap.revision; b.path = snap.path;
      return snap;
    },
    async write(body, reset) {
      const result = await codexCall<{ savedAt: number; revision: string }>(reset ? 'restore_kitchen' : 'save_kitchen', {
        expectedRevision: codexRevision, stateJson: JSON.stringify(body.state),
      });
      codexRevision = result.revision;
      return result.savedAt;
    },
  };
  return b;
}

/**
 * Finds where this copy of the app can keep the kitchen and reads it.
 * Returns null when there is no stored copy (or nowhere to store one).
 */
export async function readRemote<S>(): Promise<Snapshot<S> | null> {
  try {
    backend = claudeHost ? await timeout(claudeBackend(), LOAD_TIMEOUT_MS) : isCodexApp ? codexBackend() : fileBackend();
    if (!backend) {
      setStatus({ kind: 'browser-only', reason: claudeHost ? 'artifact-no-store' : undefined });
      return null;
    }
    const snap = await timeout(backend.read(), LOAD_TIMEOUT_MS);
    setStatus(snap ? { kind: 'saved', store: backend.store, path: backend.path, savedAt: snap.savedAt } : { kind: 'saving', store: backend.store, path: backend.path });
    return snap as Snapshot<S> | null;
  } catch (e) {
    const b = backend;
    backend = null;
    if (b && !(e instanceof Unavailable)) {
      setStatus({ kind: 'browser-only', reason: 'read-failed' });
    } else {
      setStatus({ kind: 'browser-only', reason: claudeHost ? 'artifact-no-store' : undefined });
    }
    return null;
  }
}

let timer: ReturnType<typeof setTimeout> | null = null;
let pending: { body: Body; reset: boolean } | null = null;
let inFlight: Promise<void> | null = null;

/** Writes the latest pending change, one write at a time. */
async function drain(keepalive: boolean): Promise<void> {
  while (inFlight) await inFlight;
  const p = pending;
  const b = backend;
  pending = null;
  if (!p || !b) return;
  inFlight = (async () => {
    try {
      const savedAt = await b.write(p.body, p.reset, keepalive);
      if (!pending) setStatus({ kind: 'saved', store: b.store, path: b.path, savedAt });
    } catch (e) {
      const code = codeOf(e);
      if (b.store === 'claude' && code && code !== 'quota_exceeded' && code !== 'resource_exhausted' && code !== 'unavailable') {
        // Refused or withdrawn (view-only access, revoked grant): keep working in this browser.
        backend = null;
        setStatus({ kind: 'browser-only', reason: 'write-refused' });
        return;
      }
      const message = code === 'quota_exceeded' ? 'Claude storage is full' : e instanceof Error ? e.message : (e as { message?: string })?.message ?? 'Could not save';
      setStatus({ kind: 'error', store: b.store, path: b.path, message });
    }
  })();
  await inFlight;
  inFlight = null;
}

/** Saves shortly after the last change. */
export function scheduleSave<S>(state: S, savedAt: number, reset = false): void {
  if (!backend || status.kind === 'browser-only' || status.kind === 'checking') return;
  pending = { body: { savedAt, state }, reset: reset || (pending?.reset ?? false) };
  setStatus({ kind: 'saving', store: backend.store, path: backend.path });
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => { timer = null; void drain(false); }, DEBOUNCE_MS);
}

/** Sends any unsaved change straight away, e.g. when the tab is closing. */
export async function flushSave(): Promise<void> {
  if (!pending) { await inFlight; return; }
  if (timer) { clearTimeout(timer); timer = null; }
  await drain(true);
}
