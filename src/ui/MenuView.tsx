import { useMemo, useState } from 'react';
import { catalogue } from '../data';
import { exclusionReason, scoreRecipe, whyNot } from '../domain/planner';
import { CUISINE_GROUPS } from '../domain/profile';
import type { MealType } from '../domain/types';
import { DAYS, dayDate, weekLabel } from '../domain/weeks';
import { addRecipe, contextFor, removePick, swapPick } from '../state/actions';
import { newWeek, type Pick, useAppState } from '../state/store';
import { choiceKeys, Meta } from './bits';
import { DishArt } from './DishArt';

const MEALS: MealType[] = ['dinner', 'lunch', 'breakfast'];

export function MenuView({ weekId, initialMeal, openRecipe, swapFor, onSwapDone }: { weekId: string; initialMeal?: MealType; openRecipe: (id: string, swapFor?: Pick) => void; swapFor?: Pick; onSwapDone: () => void }) {
  const s = useAppState();
  const w = s.weeks[weekId] ?? newWeek(weekId);
  const [meal, setMeal] = useState<MealType>(initialMeal ?? 'dinner');
  const [q, setQ] = useState('');
  const [group, setGroup] = useState('');
  const [time, setTime] = useState(0);
  const [heat, setHeat] = useState('');
  const [suitable, setSuitable] = useState(true);
  const [sort, setSort] = useState<'best' | 'quick'>('best');
  const [filtersOpen, setFiltersOpen] = useState(false);
  const [added, setAdded] = useState<{ title: string; uid: string; day: number; previous?: string } | null>(null);
  const ordered = w.status === 'ordered';
  const clearFilters = () => { setQ(''); setGroup(''); setTime(0); setHeat(''); setSort('best'); setSuitable(true); };
  const filtered = !!(q || group || time || heat || sort !== 'best' || !suitable);
  const add = (id: string, title: string) => {
    if (swapFor) {
      if (swapPick(weekId, swapFor.uid, id)) { setAdded({ title, uid: swapFor.uid, day: swapFor.day, previous: swapFor.recipeId }); onSwapDone(); }
    } else {
      const addedPick = addRecipe(weekId, id);
      if (addedPick) setAdded({ ...addedPick, title });
    }
  };
  const inWeek = new Set(w.picks.map(p => p.recipeId));

  const results = useMemo(() => {
    const ctx = contextFor(s, weekId);
    const groupCuisines = CUISINE_GROUPS.find(g => g.label === group)?.cuisines;
    const words = q.toLowerCase().split(/\s+/).filter(Boolean);
    return catalogue.recipes
      .filter(r => r.meal === meal)
      .filter(r => !groupCuisines || groupCuisines.includes(r.cuisine))
      .filter(r => !time || r.time <= time)
      .filter(r => !heat || (heat === 'mild' ? r.spice <= 1 : r.spice >= 2))
      .filter(r => {
        if (!words.length) return true;
        const hay = `${r.title} ${r.blurb} ${r.cuisine} ${r.tags.join(' ')} ${r.ingredients.map(i => catalogue.ingredients.get(i.id)?.name ?? '').join(' ')}`.toLowerCase();
        return words.every(word => hay.includes(word));
      })
      .map(r => ({ r, why: whyNot(r, ctx), score: scoreRecipe(r, ctx).score }))
      .filter(x => !suitable || x.why === null)
      .sort((a, b) => sort === 'quick' ? a.r.time - b.r.time : b.score - a.score);
  }, [s, weekId, meal, q, group, time, heat, suitable, sort]);

  return (
    <div className="menu">
      <header className="menu-head">
        <h1 className="display">The menu</h1>
        <p className="muted">{catalogue.recipes.length} recipes · your week of {weekLabel(weekId)}</p>
      </header>
      {ordered && <p className="notice">This week is ordered. <a className="link" href="#shop">Reopen it from Shop</a> to change meals.</p>}
      {swapFor && <p className="notice">Replacing {DAYS[swapFor.day]}'s {swapFor.meal}: <strong>{catalogue.byId.get(swapFor.recipeId)?.title}</strong>. <button type="button" className="link" onClick={onSwapDone}>Cancel replacement</button></p>}
      <div className="tabs" role="tablist" aria-label="Meal" onKeyDown={choiceKeys}>
        {MEALS.map(m => (
          <button key={m} id={`meal-${m}`} role="tab" type="button" tabIndex={meal === m ? 0 : -1} aria-controls="menu-results" aria-selected={meal === m} className={`tab${meal === m ? ' on' : ''}`} onClick={() => setMeal(m)}>
            {m === 'lunch' ? 'Lunches' : m === 'dinner' ? 'Dinners' : 'Breakfasts'}
          </button>
        ))}
      </div>
      <div className="filters">
        <label className="search">
          <span className="sr-only">Search recipes</span>
          <input type="search" value={q} onChange={e => setQ(e.target.value)} placeholder="Search: halloumi, noodles, pepper…" />
        </label>
        <button type="button" className="btn ghost filter-toggle" aria-expanded={filtersOpen} aria-controls="menu-filter-options" onClick={() => setFiltersOpen(!filtersOpen)}>Filters</button>
        {filtered && <button type="button" className="link clear-filters" onClick={clearFilters}>Clear filters</button>}
        <div className={`filter-options${filtersOpen ? ' expanded' : ''}`} id="menu-filter-options">
        <label><span className="sr-only">Cuisine</span>
          <select value={group} onChange={e => setGroup(e.target.value)}>
            <option value="">All cuisines</option>
            {CUISINE_GROUPS.map(g => <option key={g.label}>{g.label}</option>)}
          </select>
        </label>
        <label><span className="sr-only">Time</span>
          <select value={time} onChange={e => setTime(Number(e.target.value))}>
            <option value={0}>Any time</option>
            <option value={20}>20 min or less</option>
            <option value={30}>30 min or less</option>
            <option value={45}>45 min or less</option>
          </select>
        </label>
        <label><span className="sr-only">Heat</span>
          <select value={heat} onChange={e => setHeat(e.target.value)}>
            <option value="">Any heat</option>
            <option value="mild">Mild</option>
            <option value="spicy">Spicy</option>
          </select>
        </label>
        <label><span className="sr-only">Sort</span>
          <select value={sort} onChange={e => setSort(e.target.value as 'best' | 'quick')}>
            <option value="best">Best for you</option>
            <option value="quick">Quickest first</option>
          </select>
        </label>
        </div>
        <label className="toggle">
          <input type="checkbox" checked={suitable} onChange={e => setSuitable(e.target.checked)} />
          <span>Only what suits my kitchen</span>
        </label>
      </div>
      <p className="muted small" aria-live="polite">{results.length} {results.length === 1 ? 'recipe' : 'recipes'}</p>
      <ul className="grid" id="menu-results" role="tabpanel" aria-labelledby={`meal-${meal}`}>
        {results.map(({ r, why }) => (
          <li key={r.id} className={`tile${why ? ' unsuitable' : ''}`}>
            <button type="button" className="tile-media" onClick={() => openRecipe(r.id, swapFor)} aria-label={`Open ${r.title}`}><DishArt recipe={r} /></button>
            <div className="tile-body">
              <h3><button type="button" className="title-btn" onClick={() => openRecipe(r.id, swapFor)}>{r.title}</button></h3>
              <Meta recipe={r} />
              {why && <p className="why warn">{why}</p>}
            </div>
            {inWeek.has(r.id)
              ? <span className="added" aria-label="In this week">✓</span>
              : <button type="button" className="add" disabled={ordered || !!exclusionReason(r, contextFor(s, weekId)) || (swapFor ? r.meal !== swapFor.meal : w.picks.filter(p => p.meal === r.meal).length >= 7)} onClick={() => add(r.id, r.title)} aria-label={`${swapFor ? 'Replace with' : 'Add'} ${r.title}${swapFor ? '' : ' to this week'}`} title={ordered ? 'Reopen this week from Shop to change meals' : why ?? (swapFor ? 'Replace the selected meal' : 'Add to the next free day')}>{swapFor ? '↔' : '+'}</button>}
          </li>
        ))}
      </ul>
      {results.length === 0 && <div className="empty-search"><p>Nothing matches. Try another word or clear the filters.</p><button type="button" className="btn ghost" onClick={clearFilters}>Clear filters</button></div>}
      {added && <div className="toast" role="status"><span>{added.previous ? 'Replaced with' : 'Added'} <strong>{added.title}</strong> · {DAYS[added.day]} {dayDate(weekId, added.day)}</span><button type="button" className="btn ghost" onClick={() => { if (added.previous) swapPick(weekId, added.uid, added.previous); else removePick(weekId, added.uid); setAdded(null); }}>Undo</button><button type="button" className="toast-close" aria-label="Dismiss confirmation" onClick={() => setAdded(null)}>×</button></div>}
    </div>
  );
}
