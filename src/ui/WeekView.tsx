import { useMemo, useState } from 'react';
import { catalogue } from '../data';
import { amount, inlineName, money } from '../domain/format';
import { alternatives, FOCUS_TAGS } from '../domain/planner';
import { buildShoppingList } from '../domain/shopping';
import type { MealType, Recipe } from '../domain/types';
import { DAYS, dayDate, weekLabel } from '../domain/weeks';
import { contextFor, focusFor, moveDay, planWeekAction, removePick, setCraving, swapPick, toggleFocusTag, toggleLock } from '../state/actions';
import { newWeek, useAppState, type Pick } from '../state/store';
import { Meta } from './bits';
import { DishArt } from './DishArt';
import { Sheet } from './Sheet';

const MEAL_TITLES: Record<MealType, string> = { dinner: 'Dinners', lunch: 'Lunches', breakfast: 'Breakfasts' };

export function useWeekShopping(weekId: string) {
  const s = useAppState();
  const w = s.weeks[weekId] ?? newWeek(weekId);
  return useMemo(() => {
    const recipes = w.picks.map(p => catalogue.byId.get(p.recipeId)).filter((r): r is Recipe => !!r);
    return buildShoppingList({ recipes, people: s.profile?.people ?? 2, have: new Set([...w.have, ...s.cupboard]) }, catalogue);
  }, [w.picks, w.have, s.cupboard, s.profile?.people]);
}

export function WeekView({ weekId, openRecipe, go }: { weekId: string; openRecipe: (id: string) => void; go: (route: string) => void }) {
  const s = useAppState();
  const w = s.weeks[weekId] ?? newWeek(weekId);
  const [swapFor, setSwapFor] = useState<Pick | null>(null);
  const list = useWeekShopping(weekId);
  const focus = focusFor(w);
  const focusWords = [
    ...focus.tags.map(t => FOCUS_TAGS.find(f => f.id === t)?.label.toLowerCase() ?? t),
    ...focus.cuisines.map(c => c[0].toUpperCase() + c.slice(1)),
    ...focus.ingredients.map(i => inlineName(catalogue.ingredients.get(i)?.name ?? i)),
  ];
  const meals = (['dinner', 'lunch', 'breakfast'] as MealType[]).filter(m => w.picks.some(p => p.meal === m) || (s.profile?.meals[m] ?? 0) > 0);
  const empty = w.picks.length === 0;

  return (
    <div className="week">
      <section className="mood" aria-labelledby="mood-h">
        <p className="eyebrow">Week of {weekLabel(weekId)}{w.status === 'ordered' && <span className="badge">Ordered</span>}</p>
        <h1 id="mood-h" className="display">What are you in the mood for?</h1>
        <div className="mood-row">
          <label className="sr-only" htmlFor="craving">Describe what you fancy this week</label>
          <input
            id="craving"
            className="craving"
            value={w.craving}
            placeholder="Something spicy, loads of peppers, maybe Thai…"
            onChange={e => setCraving(weekId, e.target.value)}
            onKeyDown={e => { if (e.key === 'Enter') planWeekAction(weekId); }}
          />
          <button type="button" className="btn primary" onClick={() => planWeekAction(weekId)}>{empty ? 'Plan my week' : 'Re-plan'}</button>
          {!empty && <button type="button" className="btn ghost" onClick={() => planWeekAction(weekId, true)} title="Keep your kept meals, try different suggestions">Shuffle</button>}
        </div>
        <div className="chips" role="group" aria-label="Quick moods">
          {FOCUS_TAGS.map(t => (
            <button key={t.id} type="button" className={`chip${w.focus.tags.includes(t.id) ? ' on' : ''}`} aria-pressed={w.focus.tags.includes(t.id)} onClick={() => toggleFocusTag(weekId, t.id)}>{t.label}</button>
          ))}
        </div>
        {focusWords.length > 0 && <p className="focus-read">Planning around <strong>{focusWords.join(' · ')}</strong></p>}
      </section>

      {empty ? (
        <section className="empty-week">
          <p>Nothing planned for {weekLabel(weekId)} yet. Tell it what you fancy, or just press <strong>Plan my week</strong> and it will pick {describeCounts(s.profile?.meals)} that suit your kitchen.</p>
          <button type="button" className="link" onClick={() => go('menu')}>Or browse all {catalogue.recipes.length} recipes</button>
        </section>
      ) : (
        <>
          {list.shared.length > 0 && <Threads weekId={weekId} picks={w.picks} list={list} />}
          {meals.map(meal => {
            const picks = w.picks.filter(p => p.meal === meal).sort((a, b) => a.day - b.day);
            return (
              <section key={meal} className="meal-group" aria-labelledby={`h-${meal}`}>
                <div className="group-head">
                  <h2 id={`h-${meal}`}>{MEAL_TITLES[meal]}</h2>
                  <button type="button" className="link" onClick={() => go(`menu/${meal}`)}>Add {meal === 'lunch' ? 'a lunch' : `a ${meal}`}</button>
                </div>
                {picks.length === 0 && <p className="muted">No {MEAL_TITLES[meal].toLowerCase()} this week.</p>}
                <ol className={`cards ${meal}`}>
                  {picks.map(p => {
                    const r = catalogue.byId.get(p.recipeId);
                    if (!r) return null;
                    return (
                      <li key={p.uid} className={`card${p.locked ? ' kept' : ''}`}>
                        <button type="button" className="card-media" onClick={() => openRecipe(r.id)} aria-label={`Open ${r.title}`}>
                          <DishArt recipe={r} />
                        </button>
                        <div className="card-body">
                          <label className="day">
                            <span className="sr-only">Day for {r.title}</span>
                            <select value={p.day} onChange={e => moveDay(weekId, p.uid, Number(e.target.value))}>
                              {DAYS.map((d, i) => <option key={d} value={i}>{d} {dayDate(weekId, i)}</option>)}
                            </select>
                          </label>
                          <h3><button type="button" className="title-btn" onClick={() => openRecipe(r.id)}>{r.title}</button></h3>
                          <Meta recipe={r} />
                          {p.reasons[0] && <p className="why">{p.reasons[0]}</p>}
                          <div className="card-actions">
                            <button type="button" className="btn small" onClick={() => setSwapFor(p)}>Swap</button>
                            <button type="button" className={`btn small ghost${p.locked ? ' active' : ''}`} aria-pressed={!!p.locked} onClick={() => toggleLock(weekId, p.uid)} title="Kept meals stay when you re-plan">{p.locked ? 'Kept' : 'Keep'}</button>
                            <button type="button" className="btn small ghost" onClick={() => removePick(weekId, p.uid)} aria-label={`Remove ${r.title}`}>Remove</button>
                          </div>
                        </div>
                      </li>
                    );
                  })}
                </ol>
              </section>
            );
          })}
          <div className="summary-bar" role="region" aria-label="Week summary">
            <span><strong>{w.picks.length}</strong> meals</span>
            <span className="n-items"><strong>{list.lines.filter(l => l.status === 'buy').length}</strong> things to buy</span>
            <span>est. <strong>{money(list.total)}</strong></span>
            {list.groupingSaving > 0.05 && <span className="saving">sharing saves {money(list.groupingSaving)}</span>}
            <button type="button" className="btn primary" onClick={() => go('shop')}>Shopping list</button>
          </div>
        </>
      )}

      <Sheet open={!!swapFor} onClose={() => setSwapFor(null)} label="Swap a meal">
        {swapFor && <SwapList weekId={weekId} pick={swapFor} onPick={(id, reasons) => { swapPick(weekId, swapFor.uid, id, reasons); setSwapFor(null); }} onBrowse={() => { setSwapFor(null); go(`menu/${swapFor.meal}`); }} openRecipe={openRecipe} />}
      </Sheet>
    </div>
  );
}

function describeCounts(m?: Record<MealType, number>): string {
  if (!m) return 'meals';
  const parts = [m.dinner && `${m.dinner} dinners`, m.lunch && `${m.lunch} lunches`, m.breakfast && `${m.breakfast} breakfasts`].filter(Boolean);
  return parts.join(', ') || 'meals';
}

function Threads({ picks, list }: { weekId: string; picks: Pick[]; list: ReturnType<typeof buildShoppingList> }) {
  const dayOf = new Map(picks.map(p => [p.recipeId, p]));
  const threads = list.shared.slice().sort((a, b) => b.usedIn.length - a.usedIn.length).slice(0, 6);
  return (
    <section className="threads" aria-labelledby="threads-h">
      <div className="threads-head">
        <h2 id="threads-h">Running through your week</h2>
        <p className="muted small">Fresh ingredients shared between meals, so packs get finished{list.groupingSaving > 0.05 ? <> — about <strong>{money(list.groupingSaving)}</strong> less than buying for each meal separately</> : null}.</p>
      </div>
      <ul className="thread-list">
        {threads.map(l => {
          const days = new Set(l.usedIn.map(u => dayOf.get(u.recipeId)).filter(Boolean).map(p => `${p!.meal}:${p!.day}`));
          const used = Math.min(100, Math.round((l.need / Math.max(l.packs.bought, 1e-9)) * 100));
          return (
            <li key={l.ingredient.id} className="thread">
              <span className="swatch" style={{ background: l.ingredient.colour ?? '#999' }} aria-hidden="true" />
              <span className="thread-name">{l.ingredient.name}</span>
              <span className="thread-days" aria-label={`Used on ${[...days].length} meals`}>
                {DAYS.map((d, i) => {
                  const on = [...days].some(x => x.endsWith(`:${i}`));
                  const light = isLight(l.ingredient.colour ?? '#999999');
                  return <span key={d} className={`dot${on ? ' on' : ''}${on && light ? ' light' : ''}`} style={on ? { background: l.ingredient.colour } : undefined} title={on ? d : undefined}>{d[0]}</span>;
                })}
              </span>
              <span className="thread-use">{amount(l.need, l.ingredient.unit)} of {amount(l.packs.bought, l.ingredient.unit)} · {used}% used</span>
            </li>
          );
        })}
      </ul>
    </section>
  );
}

function isLight(hex: string): boolean {
  const n = parseInt(hex.slice(1), 16);
  const [r, g, b] = [(n >> 16) & 255, (n >> 8) & 255, n & 255];
  return 0.299 * r + 0.587 * g + 0.114 * b > 170;
}

function SwapList({ weekId, pick, onPick, onBrowse, openRecipe }: { weekId: string; pick: Pick; onPick: (id: string, reasons: string[]) => void; onBrowse: () => void; openRecipe: (id: string) => void }) {
  const s = useAppState();
  const w = s.weeks[weekId];
  const current = catalogue.byId.get(pick.recipeId);
  const alts = useMemo(() => {
    const index = w.picks.findIndex(p => p.uid === pick.uid);
    return alternatives(contextFor(s, weekId), w.picks, index, 5);
  }, [s, weekId, w.picks, pick.uid]);
  return (
    <div className="swap">
      <p className="eyebrow">Swap {DAYS[pick.day]}'s {pick.meal}</p>
      <h2 className="display small">Instead of {current?.title}…</h2>
      <ul className="alt-list">
        {alts.map(a => (
          <li key={a.recipe.id} className="alt">
            <button type="button" className="alt-media" onClick={() => openRecipe(a.recipe.id)} aria-label={`Open ${a.recipe.title}`}><DishArt recipe={a.recipe} /></button>
            <div>
              <h3>{a.recipe.title}</h3>
              <Meta recipe={a.recipe} />
              {a.reasons[0] && <p className="why">{a.reasons[0]}</p>}
            </div>
            <button type="button" className="btn small primary" onClick={() => onPick(a.recipe.id, a.reasons)}>Swap in</button>
          </li>
        ))}
        {alts.length === 0 && <li className="muted">No other recipes fit your kitchen settings for this meal.</li>}
      </ul>
      <button type="button" className="link" onClick={onBrowse}>Browse the full menu</button>
    </div>
  );
}
