import { readFileSync, writeFileSync } from 'node:fs';
import { syntheticDraft } from '../app/features/development/syntheticExamples.ts';
import { snapshot } from '../tests/fixtures/ui/snapshot.ts';
import assert from 'node:assert/strict';
const path = new URL('../tests/fixtures/ui/synthetic-draft.json', import.meta.url);
const generated = `${JSON.stringify(syntheticDraft('quick-meals', snapshot()), null, 2)}\n`;
if (process.argv.includes('--write')) writeFileSync(path, generated);
else if (readFileSync(path, 'utf8').replace(/\r\n/g, '\n') !== generated) throw new Error('Native synthetic fixture differs from the actual development builder');
console.log('Synthetic draft fixture matches the development builder. Native admission is tested separately.');
// Independent immutable oracles, rather than builder-derived parity alone.
const literal = name => JSON.parse(readFileSync(new URL(`../src-tauri/tests/fixtures/receipt/r2/${name}`, import.meta.url), 'utf8'));
for (const [example, expected] of [['empty','rich-empty-overlay-draft.json'], ['quick-meals','rich-quick-overlay-draft.json']]) {
  const input = literal('rich-initial-snapshot.json'); const before = structuredClone(input);
  assert.deepEqual(syntheticDraft(example, input), literal(expected)); assert.deepEqual(input,before);
}
console.log('Both rich preference overlays match independently frozen whole-draft literals.');
