import { afterEach, describe, expect, it, vi } from 'vitest';

// A stand-in for the artifact runtime: window.claude.use('db' | 'user').
function fakeClaude(opts: { canWrite?: boolean | null; doc?: Record<string, unknown> | null; id?: string | null } = {}) {
  const writes: { path: string; data: Record<string, unknown> }[] = [];
  let stored = opts.doc ?? null;
  const db = {
    doc(path: string) {
      return {
        async get() { return { exists: !!stored, data: () => (stored ? Object.freeze(structuredClone(stored)) : undefined) }; },
        async set(data: Record<string, unknown>) { writes.push({ path, data }); stored = data; },
      };
    },
  };
  const user = { id: async () => (opts.id === undefined ? 'u_test' : opts.id), can: async () => (opts.canWrite === undefined ? true : opts.canWrite) };
  return { claude: { use: async (name: string) => (name === 'db' ? db : name === 'user' ? user : null) }, writes };
}

async function load(fake: ReturnType<typeof fakeClaude>) {
  vi.resetModules();
  (globalThis as { window?: unknown }).window = { claude: fake.claude };
  const store = await import('../src/state/store');
  const sync = await import('../src/state/fileSync');
  return { store, sync };
}

const settle = () => new Promise(r => setTimeout(r, 500));
const kitchen = (people: number) => ({ version: 2, profile: { people }, weeks: {}, favourites: [], hidden: [], ratings: {}, cupboard: [] });

afterEach(() => { delete (globalThis as { window?: unknown }).window; });

describe('kitchen in Claude storage (artifact)', () => {
  it('loads the kitchen saved in the viewer\'s Claude storage', async () => {
    const fake = fakeClaude({ doc: { savedAt: 5, state: kitchen(3) } });
    const { store, sync } = await load(fake);
    await store.hydrate();
    expect(store.getState().profile).toEqual({ people: 3 });
    expect(sync.getFileStatus()).toMatchObject({ kind: 'saved', store: 'claude' });
    // The loaded state is the app's own copy, not the frozen snapshot.
    expect(Object.isFrozen(store.getState().weeks)).toBe(false);
  });

  it('saves changes to the viewer\'s private document', async () => {
    const fake = fakeClaude();
    const { store, sync } = await load(fake);
    await store.hydrate();
    store.setState(s => ({ ...s, profile: { people: 2 } as never }));
    await settle();
    expect(fake.writes).toHaveLength(1);
    expect(fake.writes[0].path).toBe('data/users/u_test/kitchen');
    expect((fake.writes[0].data.state as { profile: unknown }).profile).toEqual({ people: 2 });
    expect(sync.getFileStatus()).toMatchObject({ kind: 'saved', store: 'claude' });
  });

  it('stays browser-only for a viewer who cannot write', async () => {
    const fake = fakeClaude({ canWrite: false });
    const { store, sync } = await load(fake);
    await store.hydrate();
    store.setState(s => ({ ...s, profile: { people: 2 } as never }));
    await settle();
    expect(fake.writes).toHaveLength(0);
    expect(sync.getFileStatus()).toMatchObject({ kind: 'browser-only', reason: 'artifact-no-store' });
  });

  it('stays browser-only when the viewer has no id', async () => {
    const fake = fakeClaude({ id: null });
    const { store, sync } = await load(fake);
    await store.hydrate();
    expect(sync.getFileStatus()).toMatchObject({ kind: 'browser-only' });
  });

  it('never writes an empty kitchen except on Start over', async () => {
    const fake = fakeClaude();
    const { store } = await load(fake);
    await store.hydrate();
    store.setState(s => s);
    await settle();
    expect(fake.writes).toHaveLength(0);
    store.resetAll();
    await settle();
    expect(fake.writes).toHaveLength(1);
    expect((fake.writes[0].data.state as { profile: unknown }).profile).toBeNull();
  });
});
