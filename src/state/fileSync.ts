// Keeps the kitchen in a JSON file on this computer as well as in the browser.
// The file is served by the local MSTRCHF server at /api/kitchen; a static copy of the app has no file and stays browser-only.
import { useSyncExternalStore } from 'react';

const ROUTE = '/api/kitchen';
const DEBOUNCE_MS = 400;

export type FileStatus =
  | { kind: 'checking' }
  | { kind: 'saved'; path: string; savedAt: number }
  | { kind: 'saving'; path: string }
  | { kind: 'browser-only' }
  | { kind: 'error'; path: string; message: string };

export interface Snapshot<S> { state: S | null; savedAt: number }

export interface Choice<S> {
  state: S | null;
  /** Where the chosen state came from. */
  from: 'file' | 'browser' | 'none';
  /** The browser copy is newer than the file, so the file needs updating. */
  pushToFile: boolean;
}

/** Picks the newer of the browser copy and the file copy. An empty side never beats a full one. */
export function chooseState<S>(browser: Snapshot<S> | null, file: Snapshot<S> | null, isEmpty: (s: S) => boolean): Choice<S> {
  const b = browser?.state && !isEmpty(browser.state) ? browser : null;
  const f = file?.state && !isEmpty(file.state) ? file : null;
  if (f && (!b || f.savedAt >= b.savedAt)) return { state: f.state, from: 'file', pushToFile: false };
  if (b) return { state: b.state, from: 'browser', pushToFile: true };
  return { state: null, from: 'none', pushToFile: false };
}

let status: FileStatus = { kind: 'checking' };
let path = '';
const listeners = new Set<() => void>();
function setStatus(next: FileStatus) { status = next; listeners.forEach(l => l()); }
export function getFileStatus(): FileStatus { return status; }
export function useFileStatus(): FileStatus {
  return useSyncExternalStore(l => { listeners.add(l); return () => listeners.delete(l); }, getFileStatus, getFileStatus);
}

/** Reads the kitchen file. Returns null when there is no local server (browser-only mode). */
export async function readKitchenFile<S>(): Promise<(Snapshot<S> & { path: string }) | null> {
  try {
    const res = await fetch(ROUTE, { cache: 'no-store', signal: AbortSignal.timeout(2500) });
    if (!res.ok || !(res.headers.get('content-type') ?? '').includes('application/json')) { setStatus({ kind: 'browser-only' }); return null; }
    const body = await res.json() as Snapshot<S> & { path: string };
    path = body.path;
    setStatus(body.state ? { kind: 'saved', path, savedAt: body.savedAt } : { kind: 'saving', path });
    return body;
  } catch {
    setStatus({ kind: 'browser-only' });
    return null;
  }
}

let timer: ReturnType<typeof setTimeout> | null = null;
let pending: { body: string; reset: boolean } | null = null;

async function put(body: string, reset: boolean, keepalive = false): Promise<void> {
  try {
    const res = await fetch(ROUTE, {
      method: 'PUT', body, keepalive,
      headers: { 'content-type': 'application/json', ...(reset ? { 'x-confirm-reset': '1' } : {}) },
    });
    const json = await res.json().catch(() => ({})) as { savedAt?: number; error?: string };
    if (!res.ok) throw new Error(json.error ?? `Save failed (${res.status})`);
    setStatus({ kind: 'saved', path, savedAt: json.savedAt ?? Date.now() });
  } catch (e) {
    setStatus({ kind: 'error', path, message: e instanceof Error ? e.message : 'Could not save to file' });
  }
}

/** Saves to the file shortly after the last change. */
export function scheduleSave<S>(state: S, savedAt: number, reset = false): void {
  if (status.kind === 'browser-only' || status.kind === 'checking') return;
  pending = { body: JSON.stringify({ savedAt, state }), reset: reset || (pending?.reset ?? false) };
  setStatus({ kind: 'saving', path });
  if (timer) clearTimeout(timer);
  timer = setTimeout(() => { const p = pending; pending = null; timer = null; if (p) void put(p.body, p.reset); }, DEBOUNCE_MS);
}

/** Sends any unsaved change straight away, e.g. when the tab is closing. */
export function flushSave(): void {
  if (!pending) return;
  const p = pending;
  pending = null;
  if (timer) { clearTimeout(timer); timer = null; }
  void put(p.body, p.reset, p.body.length < 60_000);
}
