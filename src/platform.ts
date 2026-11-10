// What the page can do where it is running: a normal browser tab, or inside a Claude artifact.
// Inside an artifact, `window.claude.use(name)` hands out capabilities (storage, downloads); direct downloads are blocked.
import { useEffect, useState } from 'react';
import { codexApp, codexCall, isCodexApp } from './codexHost';

interface ClaudeHost { use(name: string): Promise<unknown> }

export const claudeHost: ClaudeHost | null =
  typeof window !== 'undefined' && typeof (window as { claude?: { use?: unknown } }).claude?.use === 'function'
    ? (window as unknown as { claude: ClaudeHost }).claude
    : null;
export const assistantName = claudeHost ? 'Claude' : 'ChatGPT';
export const saveFileLabel = isCodexApp ? 'Save' : 'Download';

interface Downloads { save(req: { filename: string; data: string }): Promise<{ status: string }> }

let downloads: Promise<Downloads | null> | null = null;
function getDownloads(): Promise<Downloads | null> {
  return downloads ??= claudeHost
    ? (claudeHost.use('downloads') as Promise<Downloads | null>).catch(() => null)
    : Promise.resolve(null);
}

export type SaveFileResult = 'saved' | 'declined' | 'busy' | 'unavailable';

/** Offers a text file to the user. In an artifact Claude asks them to confirm first. */
export async function saveFile(filename: string, text: string, onSaved?: (path: string) => void): Promise<SaveFileResult> {
  if (isCodexApp) {
    try {
      const app = await codexApp();
      if (app.getHostCapabilities()?.downloadFile) {
        const result = await app.downloadFile({ contents: [{ type: 'resource', resource: { uri: `file:///${encodeURIComponent(filename)}`, mimeType: 'application/json', text } }] });
        return result.isError ? 'declined' : 'saved';
      }
      const result = await codexCall<{ path: string }>('export_file', { filename, text });
      onSaved?.(result.path);
      return 'saved';
    } catch { return 'unavailable'; }
  }
  if (!claudeHost) {
    const url = URL.createObjectURL(new Blob([text], { type: 'application/json' }));
    const a = document.createElement('a');
    a.href = url; a.download = filename; a.click();
    setTimeout(() => URL.revokeObjectURL(url), 1000);
    return 'saved';
  }
  const d = await getDownloads();
  if (!d) return 'unavailable';
  try {
    await d.save({ filename, data: text });
    return 'saved';
  } catch (e) {
    const code = (e as { code?: string })?.code;
    return code === 'declined' ? 'declined' : code === 'rate_limited' ? 'busy' : 'unavailable';
  }
}

/** False while unknown and when this view cannot save files (an artifact without the downloads capability). */
export function useCanSaveFiles(): boolean {
  const [can, setCan] = useState(!claudeHost);
  useEffect(() => {
    if (!claudeHost) return;
    let live = true;
    void getDownloads().then(d => { if (live) setCan(!!d); });
    return () => { live = false; };
  }, []);
  return can;
}

/** Copies text, falling back to the older copy command where the clipboard API is blocked (as in some frames). */
export async function copyText(text: string): Promise<boolean> {
  try { await navigator.clipboard.writeText(text); return true; } catch { /* try the fallback */ }
  try {
    const ta = document.createElement('textarea');
    ta.value = text;
    ta.setAttribute('readonly', '');
    ta.style.position = 'fixed';
    ta.style.opacity = '0';
    document.body.appendChild(ta);
    ta.select();
    const ok = document.execCommand('copy');
    ta.remove();
    return ok;
  } catch {
    return false;
  }
}
