// Recipe photos as separate files: src/photos/<recipe id>.webp (add them with scripts/import_photos.py).
// The Claude artifact build swaps this module for photoSource.packed.ts (see vite.config.ts); both export the same API.
const photos = import.meta.glob<string>('./photos/*.webp', { eager: true, query: '?url', import: 'default' });
const byId = new Map(Object.entries(photos).map(([path, url]) => [path.slice(path.lastIndexOf('/') + 1, -'.webp'.length), url]));

/** True when photos come from packed files and must be loaded before use (the Claude artifact build). */
export const photosPacked: boolean = false;

/** The photo's URL if it is ready now; null when the recipe has no photo or, when packed, it has not loaded yet. */
export function photoUrl(id: string): string | null {
  return byId.get(id) ?? null;
}

/** Resolves to the photo's URL, loading its pack first when photos are packed; null when there is none. */
export async function loadPhoto(id: string): Promise<string | null> {
  return photoUrl(id);
}
