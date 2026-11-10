import { type CSSProperties, useMemo, useState } from 'react';
import plateUrl from '../brand/empty-plate.webp';
import tableUrl from '../brand/hero-table.webp';
import { catalogue } from '../data';
import { amount, inlineName, money } from '../domain/format';
import { alternatives, FOCUS_TAGS, scoreRecipe, sharedReason } from '../domain/planner';
import { buildShoppingList } from '../domain/shopping';
import type { MealType, Recipe } from '../domain/types';
import { DAYS, dayDate, daysSinceMonday, weekLabel } from '../domain/weeks';
import { contextFor, focusFor, moveDay, planWeekAction, removePick, setCraving, swapPick, toggleFocusTag, toggleLock, weekExclusions } from '../state/actions';
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
    return buildShoppingList({ recipes, people: s.profile?.people ?? 2, have: new Set([...w.have, ...s.cupboard]), pantryToBuy: new Set(w.pantryToBuy ?? []) }, catalogue);
  }, [w.picks, w.have, w.pantryToBuy, s.cupboard, s.profile?.people, catalogue.market.id]);
}

export function WeekView({ weekId, openRecipe, go, browseSwap }: { weekId: string; openRecipe: (id: string, swapFor?: Pick) => void; go: (route: string) => void; browseSwap: (pick: Pick) => void }) {
  const s = useAppState();
  const w = s.weeks[weekId] ?? newWeek(weekId);
  const [swapFor, setSwapFor] = useState<Pick | null>(null);
  const [adjusting, setAdjusting] = useState(false);
  const plan = (shuffle = false) => { planWeekAction(weekId, shuffle); setAdjusting(false); };
  const conflicts = weekExclusions(s, weekId);
  const list = useWeekShopping(weekId);
  const focus = focusFor(w);
  const focusWords = [
    ...focus.tags.map(t => FOCUS_TAGS.find(f => f.id === t)?.label.toLowerCase() ?? t),
    ...focus.cuisines.map(c => c[0].toUpperCase() + c.slice(1)),
    ...focus.ingredients.map(i => inlineName(catalogue.ingredients.get(i)?.name ?? i)),
  ];
  const meals = (['dinner', 'lunch', 'breakfast'] as MealType[]).filter(m => w.picks.some(p => p.meal === m) || (s.profile?.meals[m] ?? 0) > 0);
  const empty = w.picks.length === 0;
  const ordered = w.status === 'ordered' && !empty;
  const recipes = w.picks.map(p => catalogue.byId.get(p.recipeId)).filter((r): r is Recipe => !!r);
  const ctx = contextFor(s, weekId);

  return (
    <div className="week">
      {ordered ? <OrderedHeader weekId={weekId} picks={w.picks} people={w.orderedBasket?.people} openRecipe={openRecipe} /> : <>
      {!empty && <header className="week-heading"><div><p className="week-context">{weekLabel(weekId)} · {w.picks.length} meals</p><h1 className="display">Your week</h1></div><div className="row-btns"><button type="button" className="btn ghost" aria-expanded={adjusting} aria-controls="planning-controls" onClick={() => setAdjusting(!adjusting)}>Adjust this week</button><button type="button" className="btn ghost" onClick={() => plan(true)}>Shuffle</button></div></header>}
      {(empty || adjusting) && (
      <section id="planning-controls" className="mood mood-photo" aria-labelledby="mood-h" style={{ '--photo': `url(${tableUrl})` } as CSSProperties}>
        <p className="eyebrow">Week of {weekLabel(weekId)}{w.status === 'ordered' && <span className="badge">Ordered</span>}</p>
        {empty ? <h1 id="mood-h" className="display">What are you in the mood for?</h1> : <h2 id="mood-h" className="display small">What do you fancy?</h2>}
        <div className="mood-row">
          <label className="sr-only" htmlFor="craving">Describe what you fancy this week</label>
          <input
            id="craving"
            className="craving"
            value={w.craving}
            placeholder="Something spicy, loads of peppers, maybe Thai…"
            onChange={e => setCraving(weekId, e.target.value)}
            onKeyDown={e => { if (e.key === 'Enter') plan(); }}
          />
          <button type="button" className="btn primary" onClick={() => plan()}>{empty ? 'Plan my week' : 'Re-plan'}</button>
        </div>
        <div className="chips" role="group" aria-label="Quick moods">
          {FOCUS_TAGS.map(t => (
            <button key={t.id} type="button" className={`chip${w.focus.tags.includes(t.id) ? ' on' : ''}`} aria-pressed={w.focus.tags.includes(t.id)} onClick={() => toggleFocusTag(weekId, t.id)}>{t.label}</button>
          ))}
        </div>
        {focusWords.length > 0 && <p className="focus-read">Your next plan will favour <strong>{focusWords.join(' · ')}</strong></p>}
      </section>
      )}</>}

      {!!conflicts.length && <section className="notice warn" role="alert"><strong>Your kitchen settings have changed.</strong><p>{conflicts.length} {conflicts.length === 1 ? 'meal needs' : 'meals need'} review before shopping.</p><ul>{conflicts.map(({ pick, reason }) => <li key={pick.uid}><button type="button" className="link" onClick={() => openRecipe(pick.recipeId)}>{catalogue.byId.get(pick.recipeId)?.title ?? 'Unavailable recipe'}</button> — {reason}</li>)}</ul>{!ordered && <button type="button" className="btn primary" onClick={() => plan()}>Replace excluded meals</button>}</section>}

      {empty ? (
        <section className="empty-week">
          <img className="empty-photo" src={plateUrl} alt="" />
          <div>
            <p>Nothing planned for {weekLabel(weekId)} yet. Tell it what you fancy, or just press <strong>Plan my week</strong> and it will pick {describeCounts(s.profile?.meals)} that suit your kitchen.</p>
            <button type="button" className="link" onClick={() => go('menu')}>Or browse all {catalogue.recipes.length} recipes</button>
          </div>
        </section>
      ) : (
        <>
          {meals.map(meal => {
            const picks = w.picks.filter(p => p.meal === meal).sort((a, b) => a.day - b.day);
            return (
              <section key={meal} className="meal-group" aria-labelledby={`h-${meal}`}>
                <div className="group-head">
                  <h2 id={`h-${meal}`}>{MEAL_TITLES[meal]}</h2>
                  {!ordered && <button type="button" className="link" onClick={() => go(`menu/${meal}`)}>Add {meal === 'lunch' ? 'a lunch' : `a ${meal}`}</button>}
                </div>
                {picks.length === 0 && <p className="muted">No {MEAL_TITLES[meal].toLowerCase()} this week.</p>}
                <ol className={`cards ${meal}`}>
                  {picks.map(p => {
                    const r = catalogue.byId.get(p.recipeId);
                    if (!r) return null;
                    const reason = !ordered && (p.reasons.includes('You chose this') ? 'You chose this'
                      : sharedReason(r, recipes.filter(other => other.id !== r.id), ctx) ?? scoreRecipe(r, ctx).reasons[0]);
                    return (
                      <li key={p.uid} className={`card${p.locked && !ordered ? ' kept' : ''}`}>
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
                          {reason && <p className="why">{reason}</p>}
                          {!ordered && <div className="card-actions">
                            <button type="button" className="btn small" onClick={() => setSwapFor(p)}>Swap</button>
                            <button type="button" className={`btn small ghost${p.locked ? ' active' : ''}`} aria-pressed={!!p.locked} onClick={() => toggleLock(weekId, p.uid)} title="Kept meals stay when you re-plan">{p.locked ? 'Kept' : 'Keep'}</button>
                            <button type="button" className="btn small ghost" onClick={() => removePick(weekId, p.uid)} aria-label={`Remove ${r.title}`}>Remove</button>
                          </div>}
                        </div>
                      </li>
                    );
                  })}
                </ol>
              </section>
            );
          })}
          {!ordered && list.shared.length > 0 && <details className="sharing-details"><summary>Shared ingredients <span className="muted small">{list.groupingSaving > .05 ? `· about ${money(list.groupingSaving)} saved` : '· less food left over'}</span></summary><Threads weekId={weekId} picks={w.picks} list={list} /></details>}
          {!ordered && <div className="summary-bar" role="region" aria-label="Week summary">
            <span><strong>{w.picks.length}</strong> meals</span>
            <span className="n-items"><strong>{list.lines.filter(l => l.status === 'buy').length}</strong> things to buy</span>
            <span>est. <strong>{money(list.total)}</strong></span>
            {list.groupingSaving > 0.05 && <span className="saving">sharing saves {money(list.groupingSaving)}</span>}
            <button type="button" className="btn primary" onClick={() => go('shop')}>Shopping list</button>
          </div>}
        </>
      )}

      <Sheet open={!!swapFor} onClose={() => setSwapFor(null)} label="Swap a meal">
        {swapFor && <SwapList weekId={weekId} pick={swapFor} onPick={(id, reasons) => { swapPick(weekId, swapFor.uid, id, reasons); setSwapFor(null); }} onBrowse={() => { const pick = swapFor; setSwapFor(null); browseSwap(pick); }} openRecipe={id => { const pick = swapFor; setSwapFor(null); openRecipe(id, pick); }} />}
      </Sheet>
    </div>
  );
}

const MEAL_ORDER: MealType[] = ['breakfast', 'lunch', 'dinner'];

const FULL_DAYS = ['Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday', 'Sunday'];

/** Shown instead of the planning controls once the shopping is ordered: the next dinner, large. */
function OrderedHeader({ weekId, picks, people, openRecipe }: { weekId: string; picks: Pick[]; people?: number; openRecipe: (id: string) => void }) {
  const today = daysSinceMonday(weekId);
  const sorted = [...picks].sort((a, b) => a.day - b.day || MEAL_ORDER.indexOf(a.meal) - MEAL_ORDER.indexOf(b.meal));
  const nextDinner = sorted.find(p => p.day >= today && p.meal === 'dinner');
  const next = sorted.find(p => p.day >= today);
  const feature = nextDinner ?? next;
  const recipe = feature ? catalogue.byId.get(feature.recipeId) : undefined;
  const sooner = next && feature && next !== feature ? next : undefined;
  const soonerRecipe = sooner ? catalogue.byId.get(sooner.recipeId) : undefined;
  const when = (p: Pick) => p.day === today
    ? (p.meal === 'dinner' ? 'Tonight' : `Today's ${p.meal}`)
    : p.day === today + 1 ? `Tomorrow's ${p.meal}` : `${FULL_DAYS[p.day]}'s ${p.meal}`;

  if (!feature || !recipe) {
    return (
      <section className="mood ordered" aria-labelledby="mood-h">
        <p className="eyebrow">Week of {weekLabel(weekId)}<span className="badge">Ordered</span></p>
        <h1 id="mood-h" className="display">That's the week eaten.</h1>
        <p className="muted">All of this week's meals are behind you. Plan next week from the tabs above.</p>
      </section>
    );
  }
  return (
    <section className="hero" aria-labelledby="mood-h">
      <div className="hero-text">
        <p className="eyebrow">Week of {weekLabel(weekId)}<span className="badge">Ordered</span></p>
        <p className="hero-when">{when(feature)}{feature.day > today + 1 ? `, ${dayDate(weekId, feature.day)}` : ''}</p>
        <h1 id="mood-h" className="hero-title"><button type="button" className="title-btn" onClick={() => openRecipe(recipe.id)}>{recipe.title}</button></h1>
        <p className="hero-blurb">{recipe.blurb}</p>
        <Meta recipe={recipe} />
        <div className="hero-actions">
          <button type="button" className="btn primary" onClick={() => openRecipe(recipe.id)}>Open the recipe</button>
        </div>
        {sooner && soonerRecipe && (
          <p className="hero-sooner">Before that: {when(sooner).replace(/^(Today|Tomorrow|Tonight)/, w => w.toLowerCase())} is <button type="button" className="link" onClick={() => openRecipe(soonerRecipe.id)}>{soonerRecipe.title}.</button></p>
        )}
        <p className="muted small">{people ? `This week was planned for ${people}. ` : ''}Moving meals between days is fine. Reopen the week from Shop to change meals or quantities.</p>
      </div>
      <button type="button" className="hero-media" onClick={() => openRecipe(recipe.id)} aria-label={`Open ${recipe.title}`}>
        <DishArt recipe={recipe} />
      </button>
    </section>
  );
}

function describeCounts(m?: Record<MealType, number>): string {
  if (!m) return 'meals';
  const parts = [m.dinner && `${m.dinner} dinner${m.dinner === 1 ? '' : 's'}`, m.lunch && `${m.lunch} lunch${m.lunch === 1 ? '' : 'es'}`, m.breakfast && `${m.breakfast} breakfast${m.breakfast === 1 ? '' : 's'}`].filter(Boolean);
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
              <span className="thread-name">{l.ingredient.name}</span>
              <span className="thread-days" aria-label={`Used on ${[...days].length} meals`}>
                {DAYS.map((d, i) => {
                  const on = [...days].some(x => x.endsWith(`:${i}`));
                  return <span key={d} className={`dot${on ? ' on' : ''}`} title={on ? d : undefined}>{d[0]}</span>;
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
