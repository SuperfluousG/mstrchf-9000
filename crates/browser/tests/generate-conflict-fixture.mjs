// Synthetic QA evidence generated only through the real zero-import WASM API.
// No saved selection, receipt, digest or projection is fabricated afterwards.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile, writeFile} from 'node:fs/promises';
import {pathToFileURL} from 'node:url';

const [firstDate = '2026-10-05', laterDate = '2026-10-12', filename = 'current-device-conflicts-110.json', wasmPath] = process.argv.slice(2);
assert.match(firstDate, /^\d{4}-\d{2}-\d{2}$/);
assert.match(laterDate, /^\d{4}-\d{2}-\d{2}$/);
assert.match(filename, /^[a-z0-9-]+\.json$/);
assert.notEqual(filename, 'archived-conflicts-110.json', 'the preserved original fixture must never be overwritten');
const first = `${firstDate}T12:00:00Z`, later = `${laterDate}T12:00:00Z`;
assert.equal(new Date(first).toISOString(), `${firstDate}T12:00:00.000Z`);
assert.equal(new Date(later).toISOString(), `${laterDate}T12:00:00.000Z`);
assert.equal(Date.parse(later) - Date.parse(first), 7 * 24 * 60 * 60 * 1000);
assert.ok(Date.parse(later) <= Date.now(), 'the final simulated receipt clock must not be in the device future');
const preservedUrl = new URL('./fixtures/archived-conflicts-110.json', import.meta.url);
const preserved = await readFile(preservedUrl);
const wasm = await readFile(wasmPath ? pathToFileURL(wasmPath) : new URL('../target/wasm32-unknown-unknown/release/mstrchf_browser.wasm', import.meta.url));
const module = await WebAssembly.compile(wasm);
assert.deepEqual(WebAssembly.Module.imports(module), []);
function api(instance) {
  const e = instance.exports;
  return request => {
    const bytes = new TextEncoder().encode(JSON.stringify(request)), ptr = e.alloc(bytes.length);
    new Uint8Array(e.memory.buffer, ptr, bytes.length).set(bytes);
    const packed = e.run(ptr, bytes.length), out = Number(packed & 0xffffffffn), len = Number(packed >> 32n);
    const response = JSON.parse(new TextDecoder().decode(new Uint8Array(e.memory.buffer, out, len)));
    e.dealloc(ptr, bytes.length); e.dealloc(out, len);
    assert.equal(response.ok, true, JSON.stringify(response));
    return response.value;
  };
}
const call = api(await WebAssembly.instantiate(module, {}));
let number = 1;
function command(state, body, now) {
  const envelope = {schema_version: 1, command_id: `cm_${(number++).toString(16).padStart(32, '0')}`, expected_snapshot_revision: state.snapshot.snapshot_revision, command: body};
  return call({op: 'dispatch', state, command: envelope, command_json: JSON.stringify(envelope), now}).state;
}
function weekEdit(w, plan) {
  return {kind: 'save_week', data: {expected_week_revision: w.revision, week: {week_id: w.range.week_id, defaults_snapshot: w.defaults_snapshot, day_enabled: Array(7).fill(true), meal_type_enabled: {breakfast: true, lunch: true, dinner: true}, cells: w.cells, plan}}};
}
function fill(state, weekId, now) {
  console.log(JSON.stringify({stage: 'admitting_week', week_id: weekId, command: number, now}));
  let w = state.snapshot.weeks.find(w => w.range.week_id === weekId);
  state = command(state, weekEdit(w, null), now);
  w = state.snapshot.weeks.find(w => w.range.week_id === weekId);
  const plan = structuredClone(w.plan);
  plan.selections = w.cells.map(cell => {
    const board = call({op: 'board', state, week_id: weekId, slot: cell.slot, now});
    const choice = board.choices.find(choice => choice.selection.recipe.recipe_id === 'rc_00000000000000000000000000000005');
    assert.ok(choice, 'the synthetic general-meal engineering recipe must be actually eligible');
    return choice.selection;
  });
  assert.equal(plan.selections.length, 21);
  state = command(state, weekEdit(w, plan), now);
  console.log(JSON.stringify({stage: 'week_admitted', week_id: weekId, revision: state.snapshot.snapshot_revision}));
  return state;
}
const start = performance.now();
let state = call({op: 'initialize', household_id: 'hh_00000000000000000000000000000003', diner_id: 'dn_00000000000000000000000000000003', zone: 'Europe/London', today: firstDate, now: first, demo: true});
let profile = structuredClone(state.snapshot.profile);
profile.usual_week.enabled_meal_types = ['breakfast', 'dinner', 'lunch'];
state = command(state, {kind: 'update_profile', data: {expected_profile_revision: profile.revision, profile}}, first);
const originalIds = state.snapshot.weeks.map(w => w.range.week_id);
for (const id of originalIds) state = fill(state, id, first);
state = command(state, {kind: 'open_planning_horizon', data: {}}, later);
const newId = state.snapshot.weeks.find(w => !originalIds.includes(w.range.week_id)).range.week_id;
state = fill(state, newId, later);
profile = structuredClone(state.snapshot.profile);
profile.diners[0].restrictions = {kind: 'confirmed', data: {confirmed_at: later, requirements: [{kind: 'diet', data: {code: 'vegan'}}]}};
state = command(state, {kind: 'update_profile', data: {expected_profile_revision: profile.revision, profile}}, later);
assert.equal(state.retained_weeks.length, 5);
assert.equal(state.history.selected.length, 105);
assert.equal(state.receipts.length, 13);
assert.equal(state.snapshot.conflicts.length, 100);
assert.ok(!state.snapshot.weeks.some(w => w.range.week_id === originalIds[0]));
assert.ok(state.retained_weeks.some(w => w.range.week_id === originalIds[0]));

// A fresh WASM instance proves cold restore; it cannot inherit successful audits.
const fresh = api(await WebAssembly.instantiate(module, {}));
const actualNow = new Date().toISOString().replace(/\.\d{3}Z$/, 'Z');
const cold = performance.now();
assert.deepEqual(fresh({op: 'validate_state', state, now: actualNow}), state);
const page1 = fresh({op: 'conflicts', state, offset: 0, limit: 100, now: actualNow});
const page2 = fresh({op: 'conflicts', state, offset: 100, limit: 100, now: actualNow});
assert.equal(page1.total, 110); assert.equal(page2.total, 110);
assert.equal(page1.conflicts.length, 100); assert.equal(page2.conflicts.length, 10);
assert.equal(page1.next_offset, 100); assert.equal(page2.next_offset, null);
assert.ok([...page1.conflicts, ...page2.conflicts].some(c => c.week_ids.includes(originalIds[0])));
assert.ok(state.receipts.every(r => Date.parse(r.committed_at) <= Date.parse(actualNow)));
assert.deepEqual(await readFile(preservedUrl), preserved, 'the preserved original fixture must be byte-identical');
const output = Buffer.from(JSON.stringify(state));
await writeFile(new URL(`./fixtures/${filename}`, import.meta.url), output, {flag: 'wx'});
// Same immutable engine state in the owner's existing reviewed-restore format.
// This is a synthetic QA package, not an export of anyone's current household.
const backupFilename = filename.replace(/\.json$/, '-backup.json');
const backup = Buffer.from(JSON.stringify({format: 'mstrchf-device-backup', version: 1, exported_at: actualNow, state}));
await writeFile(new URL(`./fixtures/${backupFilename}`, import.meta.url), backup, {flag: 'wx'});
console.log(JSON.stringify({stage: 'complete', fixture: filename, backup: backupFilename, first_simulated_clock: first, final_simulated_clock: later, cold_restore_actual_device_clock: actualNow, receipts: 13, selected_meals: 105, retained_weeks: 5, conflicts: 110, pages: [100, 10], archived_week_id: originalIds[0], bytes: output.length, sha256: createHash('sha256').update(output).digest('hex'), backup_bytes: backup.length, backup_sha256: createHash('sha256').update(backup).digest('hex'), wasm_sha256: createHash('sha256').update(wasm).digest('hex'), original_fixture_unchanged: true, cold_validation_and_pages_ms: Math.round(performance.now() - cold), duration_ms: Math.round(performance.now() - start)}));
