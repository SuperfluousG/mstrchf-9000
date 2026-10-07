import { readFileSync, readdirSync } from 'node:fs';
import { resolve, join } from 'node:path';
import assert from 'node:assert/strict';

// Run after both UI builds. Actual native ACL/release tests remain separate.
const root = resolve(import.meta.dirname, '..');
const walk = directory => readdirSync(directory, { withFileTypes: true }).flatMap(entry =>
  entry.isDirectory() ? walk(join(directory, entry.name)) : [join(directory, entry.name)]);
const releaseFiles = [...walk(join(root, 'dist/release')), ...walk(join(root, 'src-tauri/not-ready'))];
assert(releaseFiles.some(path => path.endsWith('index.html')), 'release UI build is missing');
for (const path of releaseFiles) {
  const bytes = readFileSync(path, 'utf8');
  for (const marker of ['planner_dispatch', 'planner_snapshot', 'planner_features', 'planner_replay_receipt',
    'planner_development_board', 'DevelopmentCurrent', 'loop001-m1',
    'ShellCommandReceipt', 'B1RetainedV1', 'B1Draft005', 'synthetic_development',
    'Save synthetic draft', 'Recover exact action', 'loop001-u1-contracts003', 'loop001-u1-contracts005']) {
    assert(!bytes.includes(marker), `development marker ${marker} in release asset ${path}`);
  }
}
const release = JSON.parse(readFileSync(join(root, 'src-tauri/tauri.conf.json'), 'utf8'));
assert.equal(release.build.frontendDist, 'not-ready');
assert.equal(release.bundle.active, false);
assert.deepEqual(release.app.security.capabilities, ['not-ready']);
const capability = JSON.parse(readFileSync(join(root, 'src-tauri/capabilities/not-ready.json'), 'utf8'));
assert.deepEqual(capability.permissions, []);
const development = JSON.parse(readFileSync(join(root, 'src-tauri/development/tauri.conf.json'), 'utf8'));
assert.equal(development.build.frontendDist, '../../dist/development');
assert.deepEqual(development.app.security.capabilities, ['development-main']);
const devCapability = JSON.parse(readFileSync(join(root, 'src-tauri/capabilities/development-main.json'), 'utf8'));
assert.deepEqual(devCapability.windows, ['development-main']);
assert.equal(devCapability.remote, undefined);
assert.deepEqual([...devCapability.permissions].sort(), ['allow-planner-development-board', 'allow-planner-dispatch', 'allow-planner-features', 'allow-planner-replay-receipt', 'allow-planner-snapshot']);
console.log(`Release asset separation checked across ${releaseFiles.length} files; explicit capability configuration checked. This is not native ACL or launch evidence.`);
