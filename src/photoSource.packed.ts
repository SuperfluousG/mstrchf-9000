// Claude artifact build only (vite.config.ts swaps it in for photoSource.ts). Recipe photos are packed into a few
// JSON files by scripts/pack_photos.py, so a Claude install publishes about 15 files instead of about 400.
// A pack is fetched the first time one of its photos is wanted. If it is not there yet (an install still uploading),
// the dish shows its drawn plate and the next view retries.
type PackIndex = { packs: number; ids: Record<string, number> };

const packUrls = import.meta.glob<string>('./photo-packs/photos-*.json', { eager: true, query: '?url', import: 'default' });
const index: PackIndex = Object.values(import.meta.glob<PackIndex>('./photo-packs/index.json', { eager: true, import: 'default' }))[0]
  ?? { packs: 0, ids: {} };

const ready = new Map<string, string>();
const loading = new Map<number, Promise<void>>();

/** True when photos come from packed files and must be loaded before use (the Claude artifact build). */
export const photosPacked: boolean = true;

/** The photo's URL if it is ready now; null when the recipe has no photo or, when packed, it has not loaded yet. */
export function photoUrl(id: string): string | null {
  return ready.get(id) ?? null;
}

function loadPack(n: number, url: string): Promise<void> {
  let pending = loading.get(n);
  if (!pending) {
    pending = fetch(url)
      .then(res => {
        if (!res.ok) throw new Error(`Photo pack ${n}: ${res.status}`);
        return res.json() as Promise<Record<string, string>>;
      })
      .then(pack => {
        for (const [id, data] of Object.entries(pack)) ready.set(id, `data:image/webp;base64,${data}`);
      })
      .catch(err => {
        loading.delete(n);
        throw err;
      });
    loading.set(n, pending);
  }
  return pending;
}

/** Resolves to the photo's URL, loading its pack first when photos are packed; null when there is none. */
export async function loadPhoto(id: string): Promise<string | null> {
  const n = index.ids[id];
  const url = n === undefined ? undefined : packUrls[`./photo-packs/photos-${n}.json`];
  if (n === undefined || !url) return null;
  if (!ready.has(id)) await loadPack(n, url);
  return ready.get(id) ?? null;
}
