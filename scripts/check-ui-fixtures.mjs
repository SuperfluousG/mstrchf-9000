import { readFileSync, writeFileSync } from 'node:fs';
import { syntheticDraft } from '../app/features/development/syntheticExamples.ts';
import { snapshot } from '../tests/fixtures/ui/snapshot.ts';
const path = new URL('../tests/fixtures/ui/synthetic-draft.json', import.meta.url);
const generated = `${JSON.stringify(syntheticDraft('quick-meals', snapshot()), null, 2)}\n`;
if (process.argv.includes('--write')) writeFileSync(path, generated);
else if (readFileSync(path, 'utf8').replace(/\r\n/g, '\n') !== generated) throw new Error('Native synthetic fixture differs from the actual development builder');
console.log('Synthetic draft fixture matches the development builder. Native admission is tested separately.');
