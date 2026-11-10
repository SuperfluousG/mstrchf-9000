import { useState } from 'react';
import bagUrl from '../brand/market-bag.webp';
import plateUrl from '../brand/empty-plate.webp';
import { catalogue } from '../data';
import { basketText, buildBasket, claudePrompt, type BasketFile } from '../domain/basket';
import { amount, money, packLabel, shortMoney } from '../domain/format';
import { retailerOf } from '../domain/profile';
import { MARKETS } from '../domain/market';
import { aisleLabel, type ShopLine } from '../domain/shopping';
import type { Recipe } from '../domain/types';
import { weekLabel } from '../domain/weeks';
import { assistantName, copyText, saveFile, saveFileLabel, useCanSaveFiles } from '../platform';
import { setStatus, toggleHave, togglePantry, weekExclusions } from '../state/actions';
import { newWeek, useAppState } from '../state/store';
import { Sheet } from './Sheet';
import { useWeekShopping } from './WeekView';

export function ShopView({ weekId, go }: { weekId: string; go: (r: string) => void }) {
  const s = useAppState();
  const w = s.weeks[weekId] ?? newWeek(weekId);
  const list = useWeekShopping(weekId);
  const [order, setOrder] = useState(false);
  const [copied, setCopied] = useState('');
  const [fallback, setFallback] = useState('');
  const [fileMsg, setFileMsg] = useState('');
  const [toolsOpen, setToolsOpen] = useState(false);
  const canSave = useCanSaveFiles();
  const people = s.profile?.people ?? 2;
  const ordered = w.status === 'ordered';
  const meals = w.picks.map(p => catalogue.byId.get(p.recipeId)).filter((r): r is Recipe => !!r).map(r => r.title);
  const retailer = retailerOf(s.profile);
  const shop = retailer?.name ?? 'your supermarket';
  const word = catalogue.market.basket;
  const basket = (ordered ? w.orderedBasket : undefined) ?? buildBasket(list, weekId, people, meals, catalogue.market, retailer);
  const conflicts = weekExclusions(s, weekId);

  if (!w.picks.length) {
    return (
      <div className="shop">
        <h1 className="display">Shopping list</h1>
        <section className="empty-week">
          <img className="empty-photo" src={plateUrl} alt="" />
          <p>Plan some meals for {weekLabel(weekId)} first. <button type="button" className="link" onClick={() => go('week')}>Go to the week</button></p>
        </section>
      </div>
    );
  }

  const buy = list.lines.filter(l => l.status === 'buy');
  const have = list.lines.filter(l => l.status === 'have');
  const pantry = list.lines.filter(l => l.status === 'pantry');
  const groups: [string, ShopLine[]][] = [];
  for (const l of buy) {
    const label = aisleLabel(l.ingredient.aisle);
    const g = groups.find(x => x[0] === label);
    if (g) g[1].push(l); else groups.push([label, [l]]);
  }

  const copy = async (what: string, text: string) => {
    if (await copyText(text)) { setFallback(''); setCopied(what); setTimeout(() => setCopied(''), 2500); }
    else { setCopied(''); setFallback(text); }
  };
  const copyFallback = fallback && (
    <>
      <p className="warn small">Copying is blocked here. Select the text below and copy it yourself.</p>
      <textarea className="copy-fallback" readOnly value={fallback} onFocus={e => e.currentTarget.select()} aria-label="Text to copy" />
    </>
  );

  if (ordered) return <RecordedOrder basket={basket} legacy={!w.orderedBasket} conflicts={conflicts.length} copied={copied === 'list'} onCopy={() => copy('list', basketText(basket))} onReopen={() => setStatus(weekId, 'draft')}>{copyFallback}</RecordedOrder>;

  return (
    <div className="shop">
      <header className="shop-head">
        <img className="shop-photo" src={bagUrl} alt="" />
        <div>
          <p className="eyebrow">Week of {weekLabel(weekId)} · {meals.length} meals for {people}{ordered && <span className="badge">Ordered</span>}</p>
          <h1 className="display">{ordered ? 'What you ordered' : 'Shopping list'}</h1>
        </div>
        <div className="shop-total">
          <span className="muted small">{ordered ? 'Estimated at planning' : 'Estimated basket'}</span>
          <strong className="mono big">{money(list.total)}</strong>
          {list.groupingSaving > 0.05 && <span className="saving small">sharing ingredients saves {money(list.groupingSaving)}</span>}
        </div>
      </header>

      <div className="shop-cols">
        <section className="receipt" aria-label="Things to buy">
          {groups.map(([label, lines]) => (
            <div key={label} className="aisle">
              <h2>{label}</h2>
              <ul>
                {lines.map(l => <Line key={l.ingredient.id} line={l} onToggle={() => toggleHave(weekId, l.ingredient.id)} />)}
              </ul>
            </div>
          ))}
          {have.length > 0 && (
            <details className="aisle have">
              <summary>Already got · {have.length} items</summary>
              <ul>{have.map(l => <Line key={l.ingredient.id} line={l} onToggle={() => toggleHave(weekId, l.ingredient.id)} />)}</ul>
            </details>
          )}
          {list.lines.some(l => l.ingredient.pantry) && <details className="pantry"><summary>Check cupboard staples · {pantry.length} assumed in stock</summary><p className="muted small">Missing something? Include it in this week's basket.</p><ul className="pantry-checks">{list.lines.filter(l => l.ingredient.pantry).map(l => <li key={l.ingredient.id}><label><input type="checkbox" aria-label={`Buy ${l.ingredient.name}`} checked={(w.pantryToBuy ?? []).includes(l.ingredient.id)} onChange={() => togglePantry(weekId, l.ingredient.id)} /><span>{l.ingredient.name} · {amount(l.need, l.ingredient.unit)}</span></label></li>)}</ul></details>}
          {!ordered && <p className="muted small">Prices are rough supermarket estimates, not live {retailer ? `${retailer.name} ` : ''}prices. Tick “Got it” for anything you already have; long-life items stay ticked for future weeks.</p>}
          {!ordered && retailer?.minBasket && list.total > 0 && list.total < retailer.minBasket.under && <p className="small warn">{retailer.name} adds a {shortMoney(retailer.minBasket.charge)} charge to {word}s under {shortMoney(retailer.minBasket.under)}. Adding a meal or a few cupboard staples may cost less than the charge.</p>}
        </section>

        <aside className="order-panel">
          <h2>Get it delivered</h2>
          <p>{assistantName} can fill your {retailer ? `${retailer.name} ` : 'online '}{word}. You review it, confirm any delivery slot and pay.</p>
          <p className="muted small">Shopping at {retailer ? retailer.name : 'another shop'} · <button type="button" className="link" onClick={() => go('kitchen')}>change</button></p>
          {conflicts.length > 0 && <p className="notice warn" role="alert">{conflicts.length} {conflicts.length === 1 ? 'meal conflicts' : 'meals conflict'} with your kitchen exclusions. Review and re-plan from Week before ordering.</p>}
          {!basket.items.length && <p className="notice">Nothing to buy. Review the items you already have and the cupboard staples below.</p>}
          <button type="button" className="btn primary wide" disabled={!!conflicts.length || !basket.items.length} onClick={() => setOrder(true)}>Order with {assistantName}</button>
          <button type="button" className="btn ghost shopping-tools-toggle" aria-expanded={toolsOpen} aria-controls="shopping-tools" onClick={() => setToolsOpen(!toolsOpen)}>List and order options</button>
          <div id="shopping-tools" className={`shopping-tools${toolsOpen ? ' expanded' : ''}`}>
          <button type="button" className="btn ghost wide" onClick={() => copy('list', basketText(basket))}>{copied === 'list' ? 'Copied' : 'Copy list'}</button>
          {canSave && <button type="button" className="btn ghost wide" onClick={async () => {
            setFileMsg('');
            const r = await saveFile(`mstrchf-basket-${weekId}.json`, JSON.stringify(basket, null, 2), path => setFileMsg(`Basket saved to ${path}`));
            if (r !== 'saved') setFileMsg(r === 'unavailable' ? 'Saving files is not available here. Copy the list instead.' : r === 'busy' ? 'Another save is waiting for you. Try again in a moment.' : '');
          }}>{saveFileLabel} basket file</button>}
          {fileMsg && <p className="warn small">{fileMsg}</p>}
          {!order && copyFallback}
          <hr />
          <button type="button" className="btn ghost wide" disabled={!!conflicts.length} onClick={() => setStatus(weekId, 'ordered')}>I've ordered this</button>
          <p className="muted small">Records this plan and its estimate. This button does not place an order.</p>
          </div>
        </aside>
      </div>
      {!!basket.items.length && <div className="shop-mobile-action" aria-label="Shopping actions"><span><strong>{money(basket.estTotal)}</strong> <span className="small">estimated</span></span><button type="button" className="btn primary" disabled={!!conflicts.length} onClick={() => setOrder(true)}>Review basket</button></div>}

      <Sheet open={order} onClose={() => setOrder(false)} label={`Order with ${assistantName}`}>
        <div className="order-sheet">
          <p className="eyebrow">{retailer?.name ?? 'Any shop'} · {basket.items.length} items · est. {money(basket.estTotal)}</p>
          <h2 className="display small">Order with {assistantName}</h2>
          <ol className="how">
            <li><strong>Open {shop}.</strong> Sign in yourself in Chrome.</li>
            <li><strong>Hand over the list.</strong> Copy these instructions to {assistantName} with browser access. The MSTRCHF 9000 plugin supplies the retailer guide.</li>
            <li><strong>Review and finish.</strong> {assistantName} reports additions and substitutions. You confirm any delivery slot and complete checkout.</li>
          </ol>
          <button type="button" className="btn primary" onClick={() => copy('prompt', claudePrompt(basket, assistantName))}>{copied === 'prompt' ? `Copied — paste it to ${assistantName}` : 'Copy order instructions'}</button>
          {copyFallback}
          <details>
            <summary>See the instructions</summary>
            <pre className="prompt">{claudePrompt(basket, assistantName)}</pre>
          </details>
        </div>
      </Sheet>
    </div>
  );
}

function RecordedOrder({ basket: b, legacy, conflicts, copied, onCopy, onReopen, children }: {
  basket: BasketFile; legacy: boolean; conflicts: number; copied: boolean; onCopy: () => void; onReopen: () => void; children: React.ReactNode;
}) {
  const price = (n: number) => money(n, MARKETS[b.market]);
  const groups = [...new Set(b.items.map(i => i.aisle))];
  return <div className="shop recorded-order">
    <header className="shop-head">
      <img className="shop-photo" src={bagUrl} alt="" />
      <div><p className="week-context">{b.week} · {b.meals.length} meals for {b.people} · {b.retailerName ?? MARKETS[b.market].label}</p><h1 className="display">{legacy ? 'Your ordered week' : 'What you ordered'}</h1></div>
      <div className="shop-total"><span className="muted small">{legacy ? 'Current plan estimate' : 'Recorded estimate'}</span><strong className="mono big">{price(b.estTotal)}</strong><span className="muted small">{b.currency}</span></div>
    </header>
    {legacy ? <p className="notice">This older week has no saved basket record. The list below reflects your current kitchen settings and prices.</p>
      : <p className="notice">Saved from your plan when you marked it ordered. Prices are estimates; your supermarket's order is the receipt.</p>}
    {!!conflicts && <p className="notice warn" role="alert">Your kitchen exclusions have changed. {conflicts} {conflicts === 1 ? 'meal needs' : 'meals need'} review. Reopen the week to change the plan.</p>}
    <div className="shop-cols">
      <section className="receipt frozen" aria-label="Recorded shopping list">{groups.map(aisle => <div className="aisle" key={aisle}><h2>{aisle}</h2><ul>{b.items.filter(i => i.aisle === aisle).map(i => <li className="line" key={i.id}>
        <div className="line-main"><span className="line-name">{i.name}</span><span className="muted small line-for">need {i.need} · {i.usedIn.join(', ')}</span></div>
        <span className="line-buy">{i.buy.map(p => p.size === 'single' ? `${p.count} loose` : `${p.count} × ${p.size}`).join(' + ')}</span><span className="line-price">{price(i.estPrice)}</span>
      </li>)}</ul></div>)}
      {!!b.alreadyHave.length && <details><summary>Already had or assumed in the cupboard</summary><p className="small muted">{b.alreadyHave.join(', ')}</p></details>}
      </section>
      <aside className="order-panel"><h2>Shopping recorded</h2><p>{b.meals.length} meals for {b.people}. You can still move meals between days.</p>
        <button type="button" className="btn ghost wide" onClick={onCopy}>{copied ? 'Copied' : 'Copy recorded list'}</button>{children}<hr />
        <button type="button" className="btn ghost wide" onClick={onReopen}>Reopen week for changes</button><p className="muted small">Changes here do not amend or cancel your supermarket order.</p>
      </aside>
    </div>
  </div>;
}

function Line({ line, onToggle }: { line: ShopLine; onToggle: () => void }) {
  const ing = line.ingredient;
  const meals = [...new Set(line.usedIn.map(u => u.title))];
  const packs = line.packs.buy.map(p => packLabel(p.size, p.count, ing)).join(' + ');
  const got = line.status === 'have';
  return (
    <li><label className={`line${got ? ' got' : ''}`}>
      <input className="got-it" type="checkbox" aria-label={`Got ${ing.name}`} checked={got} onChange={onToggle} />
      <div className="line-main">
        <span className="line-name">{ing.name}{meals.length > 1 && <span className="shared-tag">{meals.length} meals</span>}</span>
        <span className="muted small line-for" title={meals.join(', ')}>need {amount(line.need, ing.unit)} · {meals.slice(0, 2).join(', ')}{meals.length > 2 ? ` +${meals.length - 2}` : ''}</span>
      </div>
      <span className="mono line-buy">{packs}</span>
      <span className="mono line-price">{got ? '—' : money(line.packs.cost)}</span>
    </label></li>
  );
}
