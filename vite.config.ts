import { fileURLToPath } from 'node:url';
import { defineConfig, type Plugin } from 'vite';
import react from '@vitejs/plugin-react';
import { kitchenFileMiddleware } from './server/kitchenFile.ts';

// Where kitchen.json lives. Defaults to ./data (git-ignored); set MSTRCHF_DATA_DIR to move it.
const dataDir = process.env.MSTRCHF_DATA_DIR ?? fileURLToPath(new URL('./data', import.meta.url));

function kitchenFile(): Plugin {
  const middleware = kitchenFileMiddleware(dataDir);
  return {
    name: 'mstrchf-kitchen-file',
    configureServer(server) { server.middlewares.use(middleware); },
    configurePreviewServer(server) { server.middlewares.use(middleware); },
  };
}

export default defineConfig({
  // Relative URLs so the same build runs from any folder, including as a Claude artifact.
  base: './',
  plugins: [react(), kitchenFile()],
  test: {
    environment: 'node',
    include: ['tests/**/*.test.ts'],
  },
} as Parameters<typeof defineConfig>[0]);
