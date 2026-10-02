import { spawnSync } from 'node:child_process';
import assert from 'node:assert/strict';
import { readFileSync, statSync } from 'node:fs';
import { resolve, sep } from 'node:path';
import { fileURLToPath } from 'node:url';

const projectRoot = resolve(fileURLToPath(new URL('..', import.meta.url)));
const fixtureRoot = resolve(projectRoot, 'tests/fixtures/contracts');
const stages = new Set(['wire','semantic','raw','binding','privacy','context_compare','read_binding','resume','operation_approval','operation_shape','arithmetic','conversion','essentials','command','canonical','primitive']);
const expectedRoots = [
  'household-draft', 'household-profile', 'planning-context', 'meal-requirements',
  'recipe-candidate', 'recipe-revision', 'week', 'week-plan', 'derived-plan',
  'job-request', 'job-result', 'operation-request', 'operation-grant',
  'operation-observation', 'worker-stop-evidence', 'app-command', 'snapshot',
  'event-batch', 'read-observation',
];

function run(command, args) {
  const result = spawnSync(command, args, { cwd: projectRoot, stdio: 'inherit', env: process.env });
  if (result.error) throw result.error;
  if (result.status !== 0) process.exit(result.status ?? 1);
}

export function validateFixtureInventory(manifest, sizeOf = file => statSync(file).size) {
  if (!manifest || !Array.isArray(manifest.cases) || manifest.cases.length === 0) {
    throw new Error('fixture manifest has no cases');
  }
  const roots = new Set(expectedRoots);
  const ids = new Set();
  for (const entry of manifest.cases) {
    if (!entry || typeof entry.id !== 'string' || !entry.id ||
        typeof entry.root !== 'string' || !roots.has(entry.root) ||
        typeof entry.stage !== 'string' || !stages.has(entry.stage) ||
        typeof entry.file !== 'string' || !entry.file ||
        typeof entry.expected !== 'string' || !entry.expected) {
      throw new Error('fixture manifest has an incomplete case');
    }
    const key = `${entry.id}:${entry.subcase ?? ''}`;
    if (ids.has(key)) throw new Error(`duplicate fixture case ${key}`);
    ids.add(key);
    const file = resolve(fixtureRoot, entry.file);
    if (!file.startsWith(`${fixtureRoot}${sep}`)) throw new Error(`fixture path escapes inventory: ${entry.file}`);
    if (sizeOf(file) === 0 && entry.id !== 'empty_input') {
      throw new Error(`empty fixture: ${entry.file}`);
    }
  }
  return { cases: manifest.cases.length, uniqueIds: ids.size };
}

function checkFixtures(readManifest = () => readFileSync(resolve(fixtureRoot, 'manifest.json'), 'utf8'), sizeOf) {
  const result = validateFixtureInventory(JSON.parse(readManifest()), sizeOf);
  console.log(`Fixture inventory: ${result.cases} cases, ${result.uniqueIds} unique IDs`);
}

function selfTest() {
  const caseOne = { id: 'valid', root: 'week-plan', stage: 'wire', file: 'week-plan.json', expected: 'ok' };
  assert.throws(() => checkFixtures(() => { throw new Error('missing manifest'); }), /missing manifest/);
  assert.throws(() => validateFixtureInventory({ cases: [] }), /no cases/);
  assert.throws(() => validateFixtureInventory({ cases: [caseOne] }, () => 0), /empty fixture/);
  assert.deepEqual(validateFixtureInventory({ cases: [caseOne] }, () => 1), { cases: 1, uniqueIds: 1 });
  console.log('Fixture inventory rejection checks passed.');
}

try {
  if (process.argv.includes('--self-test')) {
    selfTest();
    process.exit(0);
  }
  checkFixtures();
  if (Number(process.versions.node.split('.')[0]) !== 24) throw new Error('Node 24 is required');
  const required = JSON.parse(readFileSync(resolve(fixtureRoot, 'required-case-ids.json'), 'utf8'));
  const manifest = JSON.parse(readFileSync(resolve(fixtureRoot, 'manifest.json'), 'utf8'));
  const ids = new Set(manifest.cases.map(c => c.id));
  if (required.length !== 90 || required.some(id => !ids.has(id))) throw new Error(`missing required fixture rows: ${required.filter(id => !ids.has(id)).join(', ')}`);
  const rustc = spawnSync('rustc', ['+stable-x86_64-pc-windows-msvc', '--version'], { cwd: projectRoot, encoding: 'utf8', env: process.env });
  if (rustc.error || rustc.status !== 0 || !rustc.stdout.startsWith('rustc 1.94.0 ')) {
    throw new Error('Rust 1.94.0 is required for deterministic generation');
  }
  run('cargo', ['+stable-x86_64-pc-windows-msvc', 'run', '--locked', '-p', 'mstrchf-contracts', '--bin', 'generate-contracts', '--', '--check']);
  const tsc = resolve(projectRoot, 'node_modules/typescript/bin/tsc');
  run(process.execPath, [tsc, '--project', 'tests/contracts/tsconfig.json', '--noEmit']);
  console.log('Generated contracts and TypeScript parity verified.');
} catch (error) {
  console.error(`Contract check failed: ${error.message}`);
  process.exit(1);
}
