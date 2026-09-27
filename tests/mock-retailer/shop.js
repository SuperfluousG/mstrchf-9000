// Same-origin loopback requests only; no remote resources or credentials.
const element = id => document.getElementById(id);
const show = (id, value) => { element(id).textContent = typeof value === 'string' ? value : JSON.stringify(value, null, 2); };
let observed;
async function api(path, body) {
  const response = await fetch(path, body === undefined ? {} : { method: 'POST', headers: { 'content-type': 'application/json' }, body: JSON.stringify(body) });
  const result = await response.json();
  if (!response.ok) throw new Error(`${response.status}: ${result.error}`);
  return result;
}
async function evidence() {
  show('oracle', await api('/api/admin'));
  show('ledger', (await api('/api/ledger')).events);
}
async function read() {
  observed = undefined;
  element('products').replaceChildren();
  show('basket', 'Reading; previous observation invalidated');
  try {
    const result = await api('/api/state'); observed = result;
    show('basket', { basketId: result.basketId, runId: result.runId, revision: result.revision,
      observedAt: result.observedAt, context: result.context, basket: result.basket });
    for (const product of result.products) {
      const article = document.createElement('article');
      const title = document.createElement('h3'); title.textContent = product.name;
      const details = document.createElement('pre'); details.textContent = JSON.stringify(product, null, 2);
      const form = document.createElement('form');
      const label = document.createElement('label'); label.textContent = `Quantity for ${product.sku} `;
      const quantity = document.createElement('input'); quantity.type = 'number'; quantity.min = '0'; quantity.max = '99'; quantity.value = '1'; label.append(quantity);
      const operation = document.createElement('select'); operation.setAttribute('aria-label', `Operation for ${product.sku}`);
      for (const name of ['add', 'set']) { const option = document.createElement('option'); option.value = name; option.textContent = name; operation.append(option); }
      const button = document.createElement('button'); button.textContent = `Apply ${product.sku} action`;
      form.append(label, operation, button);
      form.addEventListener('submit', event => {
        event.preventDefault(); const basketId = result.basketId;
        void perform(async () => {
          button.disabled = true;
          try { await api('/api/action', { basketId, actionId: crypto.randomUUID(), sku: product.sku, quantity: Number(quantity.value), operation: operation.value }); await read(); }
          finally { button.disabled = false; }
        });
      });
      article.append(title, details, form); element('products').append(article);
    }
  } catch (error) { show('basket', `No current shop observation: ${error.message}`); throw error; }
}
async function perform(action) {
  show('status', 'Synthetic test shop: request pending. Inspect independent evidence if acknowledgement is withheld.');
  try { await action(); show('status', 'Synthetic test shop: request completed.'); }
  catch (error) { show('status', `Synthetic test shop: ${error.message}. Mutation outcome may be uncertain. Read state; do not blindly repeat.`); }
  finally { try { await evidence(); } catch (error) { show('oracle', error.message); } }
}
function form(id, handler) { element(id).addEventListener('submit', event => { event.preventDefault(); void perform(() => handler(Object.fromEntries(new FormData(event.currentTarget)))); }); }
const control = body => api('/api/control', body);
element('refresh').onclick = () => perform(read);
element('evidence').onclick = () => perform(evidence);
form('fault', data => control(data));
form('reads', data => control({ readFailures: Number(data.readFailures) }));
form('product-control', data => control({ product: { ...data, pricePence: data.pricePence === '' ? null : Number(data.pricePence), inStock: data.inStock === 'true' } }));
form('manual', async data => {
  if (!observed) throw new Error('Read the shop state first');
  await api('/api/manual', { ...data, basketId: observed.basketId, quantity: Number(data.quantity) }); await read();
});
for (const key of ['release', 'discard']) for (const button of document.querySelectorAll(`[data-${key}]`)) button.onclick = () => perform(() => control({ [key]: button.dataset[key] }));
element('login').onclick = () => perform(() => control({ loggedIn: true }));
element('logout').onclick = () => perform(() => control({ loggedIn: false }));
element('reset').onclick = () => perform(async () => { await api('/api/reset', { confirm: 'RESET SYNTHETIC SHOP' }); await read(); });
void perform(read);
