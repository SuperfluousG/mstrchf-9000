// Read-only cross-check against the current pre-integration zero-import module.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile} from 'node:fs/promises';
const wasm = await readFile(new URL('../target/wasm32-unknown-unknown/release/mstrchf_browser.wasm', import.meta.url));
const module = await WebAssembly.compile(wasm);
assert.deepEqual(WebAssembly.Module.imports(module), []);
const {exports: e} = await WebAssembly.instantiate(module, {});
function call(request) {
  const input = new TextEncoder().encode(JSON.stringify(request)), pointer = e.alloc(input.length);
  new Uint8Array(e.memory.buffer, pointer, input.length).set(input);
  const packed = e.run(pointer, input.length), out = Number(packed & 0xffffffffn), length = Number(packed >> 32n);
  const response = JSON.parse(new TextDecoder().decode(new Uint8Array(e.memory.buffer, out, length)));
  e.dealloc(pointer, input.length); e.dealloc(out, length);
  assert.equal(response.ok, true, JSON.stringify(response));
  return response.value;
}
const now = '2026-10-12T12:00:00Z';
const state = call({op: 'initialize', household_id: 'hh_00000000000000000000000000000004', diner_id: 'dn_00000000000000000000000000000004', zone: 'Europe/London', today: '2026-10-12', now, demo: true});
const library = call({op: 'board', state, now}).details.library.filter(entry => entry.admission === 'engineering');
const folder = new URL('../assets/catalogue/engineering-v1/', import.meta.url);
const manifest = JSON.parse(await readFile(new URL('manifest.json', folder), 'utf8'));
assert.equal(library.length, 5);
for (const entry of library) {
  const pin = manifest.catalogue.entries.find(pin => pin.recipe.recipe_id === entry.reference.recipe_id && pin.recipe.revision === entry.reference.revision);
  assert.deepEqual(pin.recipe, entry.reference);
  const bytes = await readFile(new URL(`${entry.reference.recipe_id}-r${entry.reference.revision}.canonical.json`, folder));
  assert.equal(createHash('sha256').update(bytes).digest('hex'), entry.reference.digest);
  assert.deepEqual(JSON.parse(bytes), entry.recipe);
}
console.log(JSON.stringify({imports: 0, recipes: 5, exact_frozen_bodies_match_current_wasm: true, catalogue_manifest_digest: manifest.catalogue.manifest_digest, wasm_sha256: createHash('sha256').update(wasm).digest('hex')}));
