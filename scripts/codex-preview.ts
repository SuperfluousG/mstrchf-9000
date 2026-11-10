import { AppBridge, PostMessageTransport } from '@modelcontextprotocol/ext-apps/app-bridge';

const frame = document.querySelector<HTMLIFrameElement>('#planner')!;
const status = document.querySelector<HTMLElement>('#status')!;
const token = document.querySelector<HTMLMetaElement>('meta[name="preview-token"]')!.content;
async function rpc(method: string, params: unknown) {
  const res = await fetch('/rpc', { method: 'POST', headers: { 'content-type': 'application/json', 'x-preview-token': token }, body: JSON.stringify({ method, params }) });
  if (!res.ok) throw new Error(await res.text());
  return res.json();
}
const resources = !new URLSearchParams(location.search).has('tools-only');
const bridge = new AppBridge(null, { name: 'MSTRCHF test host', version: '0.1.0' }, {
  serverTools: {}, ...(resources ? { serverResources: {} } : {}), logging: {},
}, { hostContext: { displayMode: 'fullscreen', availableDisplayModes: ['fullscreen'], theme: 'light', locale: 'en-GB', safeAreaInsets: { top: 0, right: 0, bottom: 0, left: 0 } } });
bridge.oncalltool = params => rpc('tools/call', params);
bridge.onreadresource = params => rpc('resources/read', params);
bridge.oninitialized = async () => {
  await bridge.sendToolInput({ arguments: {} });
  await bridge.sendToolResult(await rpc('tools/call', { name: 'open_planner', arguments: {} }));
  status.textContent = `MCP Apps preview · throwaway kitchen · ${resources ? 'photo resources' : 'photo tool fallback'}`;
};
bridge.onerror = error => { status.textContent = error.message; };
await bridge.connect(new PostMessageTransport(frame.contentWindow!, frame.contentWindow!));
frame.src = '/planner';
document.querySelector('#replan')!.addEventListener('click', async () => {
  const date = new Date(); date.setDate(date.getDate() - ((date.getDay() + 6) % 7) + 7);
  const weekId = `${date.getFullYear()}-${String(date.getMonth()+1).padStart(2,'0')}-${String(date.getDate()).padStart(2,'0')}`;
  const result = await rpc('tools/call', { name: 'plan_week', arguments: { weekId, craving: 'Thai, spicy' } });
  await bridge.sendToolResult(result);
  status.textContent = result.isError ? 'Set up the test kitchen first.' : 'Model plan delivered through MCP Apps.';
});
