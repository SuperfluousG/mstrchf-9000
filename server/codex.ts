import { fileURLToPath } from 'node:url';
import { StdioServerTransport } from '@modelcontextprotocol/sdk/server/stdio.js';
import { createCodexServer } from './codexServer';

const root = fileURLToPath(new URL('../', import.meta.url));
const { server } = await createCodexServer(root);
await server.connect(new StdioServerTransport());
