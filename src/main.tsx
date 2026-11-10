import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { App } from './App';
import { flushSave } from './state/fileSync';
import { hydrate } from './state/store';
import './styles.css';
import { onCodexTeardown } from './codexHost';

// Load the kitchen file before the first render so an emptied browser never shows the setup screen by mistake.
hydrate().finally(() => {
  createRoot(document.getElementById('root')!).render(
    <StrictMode>
      <App />
    </StrictMode>,
  );
});

addEventListener('pagehide', flushSave);
onCodexTeardown(flushSave);
document.addEventListener('visibilitychange', () => { if (document.visibilityState === 'hidden') flushSave(); });
