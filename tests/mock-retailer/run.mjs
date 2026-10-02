import { start } from './server.mjs';

const args = process.argv.slice(2);
const options = {};
for (let i = 0; i < args.length; i += 2) {
  if (!['--port', '--data-dir'].includes(args[i]) || !args[i + 1]) throw new Error('Usage: node run.mjs [--port 0] [--data-dir existing-system-temp-directory]');
  options[args[i] === '--port' ? 'port' : 'dataDir'] = args[i] === '--port' ? Number(args[i + 1]) : args[i + 1];
}
const shop = await start(options);
console.log(JSON.stringify({ synthetic: true, url: shop.url, dataDir: shop.dataDir, ledger: `${shop.dataDir}/ledger.jsonl` }));
for (const signal of ['SIGINT', 'SIGTERM']) process.once(signal, async () => { await shop.close(); process.exit(0); });
