import { fileURLToPath } from 'node:url';
import { defineConfig, type Plugin, type UserConfig } from 'vite';
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

// `vite build --mode artifact` (the Claude artifact build) takes recipe photos from the few packed files that
// scripts/pack_photos.py writes, instead of one file per photo, so an install publishes about 15 files.
function photoPacks(mode: string): Plugin {
  const packed = fileURLToPath(new URL('./src/photoSource.packed.ts', import.meta.url));
  return {
    name: 'mstrchf-photo-packs',
    enforce: 'pre',
    resolveId(source) {
      return mode === 'artifact' && source === './photoSource' ? packed : null;
    },
  };
}

export default defineConfig(({ mode }) => ({
  // Relative URLs so the same build runs from any folder, including as a Claude artifact.
  base: './',
  plugins: [react(), kitchenFile(), photoPacks(mode)],
  build: {
    assetsInlineLimit: process.env.MSTRCHF_MCP_BUILD === '1' ? file => !/[/\\]photos[/\\]/.test(file) : 4096,
    ...(process.env.MSTRCHF_MCP_BUILD === '1' ? { rolldownOptions: { output: { codeSplitting: false } } } : {}),
  },
  test: {
    environment: 'node',
    include: ['tests/**/*.test.ts'],
  },
}) as UserConfig);
