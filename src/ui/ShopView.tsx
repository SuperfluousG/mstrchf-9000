import { useState } from 'react';
import { catalogue } from '../data';
import { basketText, buildBasket, claudePrompt } from '../domain/basket';
import { amount, inlineName, money, packLabel } from '../domain/format';
import { AISLE_LABELS, type ShopLine } from '../domain/shopping';
import type { Recipe } from '../domain/types';
import { weekLabel } from '../domain/weeks';
import { copyText, saveFile, useCanSaveFiles } from '../platform';
import { setStatus, toggleHave } from '../state/actions';
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
  const canSave = useCanSaveFiles();
  const people = s.profile?.people ?? 2;
  const ordered = w.status === 'ordered';
  const meals = w.picks.map(p => catalogue.byId.get(p.recipeId)).filter((r): r is Recipe => !!r).map(r => r.title);
  const basket = buildBasket(list, weekId, people, meals);

  if (!w.picks.length) {
    return (
      <div className="shop">
        <h1 className="display">Shopping list</h1>
        <p className="empty-week">Plan some meals for {weekLabel(weekId)} first. <button type="button" className="link" onClick={() => go('week')}>Go to the week</button></p>
      </div>
    );
  }

  const buy = list.lines.filter(l => l.status === 'buy');
  const have = list.lines.filter(l => l.status === 'have');
  const pantry = list.lines.filter(l => l.status === 'pantry');
  const groups: [string, ShopLine[]][] = [];
  for (const l of buy) {
    const label = AISLE_LABELS[l.ingredient.aisle];
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

  return (
    <div className="shop">
      <header className="shop-head">
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
            <div className="aisle have">
              <h2>Already got</h2>
              <ul>{have.map(l => <Line key={l.ingredient.id} line={l} onToggle={() => toggleHave(weekId, l.ingredient.id)} />)}</ul>
            </div>
          )}
          {pantry.length > 0 && <p className="muted small pantry">Assumed in the cupboard: {pantry.map(l => inlineName(l.ingredient.name)).join(', ')}.</p>}
          {!ordered && <p className="muted small">Prices are rough supermarket estimates, not live Tesco prices. Tick “Got it” for anything you already have; long-life items stay ticked for future weeks.</p>}
          {!ordered && list.total > 0 && list.total < 50 && <p className="small warn">Tesco adds a £5 charge to baskets under £50. Adding a meal or a few cupboard staples may cost less than the charge.</p>}
        </section>

        {ordered ? (
          <aside className="order-panel">
            <h2>Ordered</h2>
            <p>This week's shopping is ordered. The list stays here so you can check the delivery against it.</p>
            <button type="button" className="btn ghost wide" onClick={() => copy('list', basketText(basket))}>{copied === 'list' ? 'Copied' : 'Copy list'}</button>
            {copyFallback}
            <hr />
            <button type="button" className="link" onClick={() => setStatus(weekId, 'draft')}>Mark as not ordered</button>
            <p className="muted small">Do this to change meals or shop again for this week.</p>
          </aside>
        ) : (
        <aside className="order-panel">
          <h2>Get it delivered</h2>
          <p>Hand the basket to Claude with browser access and it fills your Tesco basket, then stops for you to choose a slot and pay.</p>
          <button type="button" className="btn primary wide" onClick={() => setOrder(true)}>Order with Claude</button>
          <button type="button" className="btn ghost wide" onClick={() => copy('list', basketText(basket))}>{copied === 'list' ? 'Copied' : 'Copy list'}</button>
          {canSave && <button type="button" className="btn ghost wide" onClick={async () => {
            const r = await saveFile(`mstrchf-basket-${weekId}.json`, JSON.stringify(basket, null, 2));
            setFileMsg(r === 'unavailable' ? 'Saving files is not available here. Copy the list instead.' : r === 'busy' ? 'Another save is waiting for you. Try again in a moment.' : '');
          }}>Download basket file</button>}
          {fileMsg && <p className="warn small">{fileMsg}</p>}
          {!order && copyFallback}
          <hr />
          <button type="button" className="btn ghost wide" onClick={() => setStatus(weekId, 'ordered')}>Mark as ordered</button>
        </aside>
        )}
      </div>

      <Sheet open={order} onClose={() => setOrder(false)} label="Order with Claude">
        <div className="order-sheet">
          <p className="eyebrow">Tesco · {basket.items.length} items · est. {money(basket.estTotal)}</p>
          <h2 className="display small">Order with Claude</h2>
          <ol className="how">
            <li>Open Tesco in Chrome and sign in yourself.</li>
            <li>Copy the order instructions below and paste them to Claude (Claude in Chrome, or Claude Code with browser access). With the MSTRCHF 9000 plugin installed, Claude already knows the steps.</li>
            <li>Claude adds each item, keeps anything already in your basket, and reports substitutions.</li>
            <li>You choose the delivery slot and pay. Claude never checks out for you.</li>
          </ol>
          <button type="button" className="btn primary" onClick={() => copy('prompt', claudePrompt(basket))}>{copied === 'prompt' ? 'Copied — paste it to Claude' : 'Copy order instructions'}</button>
          {copyFallback}
          <details>
            <summary>See the instructions</summary>
            <pre className="prompt">{claudePrompt(basket)}</pre>
          </details>
        </div>
      </Sheet>
    </div>
  );
}

function Line({ line, onToggle }: { line: ShopLine; onToggle: () => void }) {
  const ing = line.ingredient;
  const meals = [...new Set(line.usedIn.map(u => u.title))];
  const packs = line.packs.buy.map(p => packLabel(p.size, p.count, ing)).join(' + ');
  const got = line.status === 'have';
  return (
    <li className={`line${got ? ' got' : ''}`}>
      <label className="got-it">
        <input type="checkbox" checked={got} onChange={onToggle} />
        <span className="sr-only">Got {ing.name}</span>
      </label>
      <div className="line-main">
        <span className="line-name">{ing.name}{meals.length > 1 && <span className="shared-tag">{meals.length} meals</span>}</span>
        <span className="muted small line-for" title={meals.join(', ')}>need {amount(line.need, ing.unit)} · {meals.slice(0, 2).join(', ')}{meals.length > 2 ? ` +${meals.length - 2}` : ''}</span>
      </div>
      <span className="mono line-buy">{packs}</span>
      <span className="mono line-price">{got ? '—' : money(line.packs.cost)}</span>
    </li>
  );
}
