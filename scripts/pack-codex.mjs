import { spawnSync } from 'node:child_process';
import { fileURLToPath } from 'node:url';
const run = spawnSync('python', [fileURLToPath(new URL('./pack_codex.py', import.meta.url))], { stdio: 'inherit' });
if (run.status !== 0) process.exit(run.status ?? 1);
