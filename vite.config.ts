import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { resolve } from 'node:path';

export default defineConfig(({ mode }) => ({
  plugins: [react()],
  root: mode === 'development-shell' ? 'app/dev' : 'app',
  server: { port: 1420, strictPort: true, host: '127.0.0.1' },
  build: { outDir: resolve(import.meta.dirname, 'dist', mode === 'development-shell' ? 'development' : 'release'), emptyOutDir: false },
}));
