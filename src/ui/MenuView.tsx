import { useMemo, useState } from 'react';
import { catalogue } from '../data';
import { scoreRecipe, whyNot } from '../domain/planner';
import { CUISINE_GROUPS } from '../domain/profile';
import type { MealType } from '../domain/types';
import { weekLabel } from '../domain/weeks';
import { addRecipe, contextFor } from '../state/actions';
import { newWeek, useAppState } from '../state/store';
import { Meta } from './bits';
import { DishArt } from './DishArt';

const MEALS: MealType[] = ['dinner', 'lunch', 'breakfast'];

export function MenuView({ weekId, initialMeal, openRecipe }: { weekId: string; initialMeal?: MealType; openRecipe: (id: string) => void }) {
  const s = useAppState();
  const w = s.weeks[weekId] ?? newWeek(weekId);
  const [meal, setMeal] = useState<MealType>(initialMeal ?? 'dinner');
  const [q, setQ] = useState('');
  const [group, setGroup] = useState('');
  const [time, setTime] = useState(0);
  const [heat, setHeat] = useState('');
  const [suitable, setSuitable] = useState(true);
  const [sort, setSort] = useState<'best' | 'quick'>('best');
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
        <p className="muted">{catalogue.recipes.length} recipes. Adding one puts it into the week of {weekLabel(weekId)}.</p>
      </header>
      <div className="tabs" role="tablist" aria-label="Meal">
        {MEALS.map(m => (
          <button key={m} role="tab" type="button" aria-selected={meal === m} className={`tab${meal === m ? ' on' : ''}`} onClick={() => setMeal(m)}>
            {m[0].toUpperCase() + m.slice(1)}s
          </button>
        ))}
      </div>
      <div className="filters">
        <label className="search">
          <span className="sr-only">Search recipes</span>
          <input type="search" value={q} onChange={e => setQ(e.target.value)} placeholder="Search: halloumi, noodles, pepper…" />
        </label>
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
        <label className="toggle">
          <input type="checkbox" checked={suitable} onChange={e => setSuitable(e.target.checked)} />
          <span>Only what suits my kitchen</span>
        </label>
      </div>
      <p className="muted small" aria-live="polite">{results.length} {results.length === 1 ? 'recipe' : 'recipes'}</p>
      <ul className="grid">
        {results.map(({ r, why }) => (
          <li key={r.id} className={`tile${why ? ' unsuitable' : ''}`}>
            <button type="button" className="tile-media" onClick={() => openRecipe(r.id)} aria-label={`Open ${r.title}`}><DishArt recipe={r} /></button>
            <div className="tile-body">
              <h3><button type="button" className="title-btn" onClick={() => openRecipe(r.id)}>{r.title}</button></h3>
              <Meta recipe={r} />
              {why && <p className="why warn">{why}</p>}
            </div>
            {inWeek.has(r.id)
              ? <span className="added" aria-label="In this week">✓</span>
              : <button type="button" className="add" onClick={() => addRecipe(weekId, r.id)} aria-label={`Add ${r.title} to this week`}>+</button>}
          </li>
        ))}
      </ul>
      {results.length === 0 && <p className="empty-week">Nothing matches. Clear a filter or try another word.</p>}
    </div>
  );
}
