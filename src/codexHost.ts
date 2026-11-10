import type { App } from '@modelcontextprotocol/ext-apps';
import { DESKTOP_VERSION } from './domain/desktopEdition';

declare global { interface Window { __MSTRCHF_CODEX__?: boolean } }
export const isCodexApp = typeof window !== 'undefined' && window.__MSTRCHF_CODEX__ === true;
let connection: Promise<App> | null = null;
let teardown: (() => Promise<void>) | null = null;
export interface CodexSnapshot { state: unknown; savedAt: number; revision: string; selectedWeekId?: string }
let latest: CodexSnapshot | null = null;
const snapshotListeners = new Set<(snapshot: CodexSnapshot) => void>();
export const latestCodexSnapshot = () => latest;
export function onCodexSnapshot(handler: (snapshot: CodexSnapshot) => void) { snapshotListeners.add(handler); return () => { snapshotListeners.delete(handler); }; }

export function onCodexTeardown(handler: () => Promise<void>) { teardown = handler; }
export function codexApp(): Promise<App> {
  if (!isCodexApp) return Promise.reject(new Error('This is not a Codex app view.'));
  return connection ??= (async () => {
    const { App } = await import('@modelcontextprotocol/ext-apps');
    const app = new App({ name: 'MSTRCHF 9000', version: DESKTOP_VERSION }, { availableDisplayModes: ['fullscreen'] });
    app.ontoolresult = result => {
      const data = result.structuredContent as unknown as CodexSnapshot | undefined;
      if (!result.isError && data && typeof data.revision === 'string' && typeof data.savedAt === 'number' && 'state' in data) {
        latest = data; snapshotListeners.forEach(fn => fn(data));
      }
    };
    app.onerror = e => console.error('MSTRCHF connection:', e.message);
    app.onteardown = async () => { await teardown?.(); return {}; };
    const applySafeArea = (insets?: { top: number; right: number; bottom: number; left: number }) => {
      for (const edge of ['top', 'right', 'bottom', 'left'] as const) document.documentElement.style.setProperty(`--safe-${edge}`, `${insets?.[edge] ?? 0}px`);
    };
    app.onhostcontextchanged = context => applySafeArea(context.safeAreaInsets);
    await app.connect();
    applySafeArea(app.getHostContext()?.safeAreaInsets);
    return app;
  })();
}

export async function codexCall<T>(name: string, args: Record<string, unknown> = {}): Promise<T> {
  const result = await (await codexApp()).callServerTool({ name, arguments: args });
  if (result.isError) {
    const detail = result.content?.filter(c => c.type === 'text').map(c => c.text).join(' ');
    throw new Error(detail || 'The local kitchen could not complete that action.');
  }
  if (!result.structuredContent) throw new Error('The local kitchen returned no data.');
  return result.structuredContent as T;
}

const photoCache = new Map<string, Promise<string>>();
export function codexPhoto(id: string): Promise<string> {
  let photo = photoCache.get(id);
  if (!photo) {
    photo = (async () => {
      const app = await codexApp();
      const item = app.getHostCapabilities()?.serverResources
        ? (await app.readServerResource({ uri: `recipe-photo://library/${encodeURIComponent(id)}` })).contents.find(c => 'blob' in c)
        : await codexCall<{ mimeType: string; blob: string }>('get_photo', { recipeId: id });
      if (!item || !('blob' in item) || item.mimeType !== 'image/webp') throw new Error('Recipe photo unavailable.');
      return `data:image/webp;base64,${item.blob}`;
    })();
    photoCache.set(id, photo);
    void photo.catch(() => photoCache.delete(id));
  }
  return photo;
}
