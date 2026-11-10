import { useState } from 'react';
import { catalogue, recipeForPeople } from '../data';
import { countedName, recipeQty } from '../domain/format';
import { CUISINE_LABELS } from '../domain/profile';
import { addRecipe, contextFor, rate, swapPick, toggleFavourite, toggleHidden } from '../state/actions';
import { exclusionReason } from '../domain/planner';
import { DAYS, dayDate, weekLabel } from '../domain/weeks';
import { type Pick, useAppState } from '../state/store';
import { Heat } from './bits';
import { DishArt } from './DishArt';

const DIFFICULTY = ['', 'Easy', 'Medium', 'Involved'];

export function RecipeSheetContent({ recipeId, weekId, onDone, swapFor }: { recipeId: string; weekId: string; onDone: () => void; swapFor?: Pick }) {
  const s = useAppState();
  const w = s.weeks[weekId];
  const ordered = w?.status === 'ordered';
  const people = (ordered ? w.orderedBasket?.people : undefined) ?? s.profile?.people ?? 2;
  const r = recipeForPeople(recipeId, people);
  const available = [0, 1, 2, 3, 4, 5, 6].filter(day => !w?.picks.some(p => p.meal === r?.meal && p.day === day));
  const [day, setDay] = useState(available[0] ?? 0);
  const [message, setMessage] = useState('');
  if (!r) return <p>That recipe is no longer in the catalogue.</p>;
  const excluded = s.profile ? exclusionReason(r, contextFor(s, weekId)) : null;
  const factor = people / r.servings;
  const inWeek = s.weeks[weekId]?.picks.some(p => p.recipeId === r.id);
  const fav = s.favourites.includes(r.id);
  const hidden = s.hidden.includes(r.id);
  const rating = s.ratings[r.id];
  const facts = catalogue.facts.get(r.id)!;
  const ingredients = r.ingredients.map(ri => ({ ri, ing: catalogue.ingredients.get(ri.id)! }));
  const toBuy = ingredients.filter(x => !x.ing.pantry);
  const pantry = ingredients.filter(x => x.ing.pantry);

  return (
    <article className="recipe-sheet">
      <div className="recipe-hero"><DishArt recipe={r} /></div>
      <p className="recipe-origin">{CUISINE_LABELS[r.cuisine]} · {r.meal}</p>
      <h2 className="recipe-title">{r.title}</h2>
      <p className="blurb">{r.blurb}</p>
      <dl className="facts">
        <div><dt>Time</dt><dd>{r.time} min</dd></div>
        <div><dt>Effort</dt><dd>{DIFFICULTY[r.difficulty]}</dd></div>
        <div><dt>Heat</dt><dd><Heat level={r.spice} showNone /></dd></div>
        <div><dt>Serves</dt><dd>{people}</dd></div>
      </dl>
      {(facts.vegan || facts.vegetarian || facts.allergens.size > 0) && (
        <p className="allergens">
          {facts.vegan ? 'Vegan. ' : facts.vegetarian ? 'Vegetarian. ' : ''}
          {facts.allergens.size > 0 && <>Contains: {[...facts.allergens].map(a => a === 'milk' ? 'dairy' : a === 'crustaceans' ? 'shellfish' : a).join(', ')}. Check packs for traces.</>}
        </p>
      )}

      {excluded && <p className="notice warn" role="alert">Your kitchen excludes this recipe: {excluded.toLowerCase()}. Change your kitchen settings before adding it.</p>}
      <div className="sheet-actions">
        {swapFor ? <button type="button" className="btn primary" disabled={ordered || !!excluded || !!inWeek} onClick={() => {
          if (swapPick(weekId, swapFor.uid, r.id)) onDone(); else setMessage('That meal could not be replaced. Check the week and your kitchen settings.');
        }}>Replace {DAYS[swapFor.day]}'s {swapFor.meal}</button>
          : inWeek ? <span className="in-week">In the week of {weekLabel(weekId)}</span>
          : ordered ? <p className="muted small">This week is ordered. Reopen it from Shop to change meals.</p>
          : <div className="recipe-add">
            <label>Day <select aria-label="Day for this recipe" value={day} onChange={e => setDay(Number(e.target.value))} disabled={!available.length}>
              {available.map(d => <option key={d} value={d}>{DAYS[d]} {dayDate(weekId, d)}</option>)}
            </select></label>
            <button type="button" className="btn primary" disabled={!!excluded || !available.includes(day)} onClick={() => {
              if (addRecipe(weekId, r.id, day)) onDone(); else setMessage('That day is no longer available. Choose another day or swap a meal.');
            }}>Add {r.meal}</button>
            {!available.length && <p className="muted small">Every day already has a {r.meal}. Swap a meal from Week instead.</p>}
          </div>}
        <button type="button" className={`btn ghost${fav ? ' active' : ''}`} aria-pressed={fav} onClick={() => toggleFavourite(r.id)}>{fav ? '♥ Favourite' : '♡ Favourite'}</button>
      </div>
      {message && <p className="notice" role="status">{message}</p>}

      <div className="recipe-cols">
        <section>
          <h3>Ingredients <span className="muted">for {people}</span></h3>
          <ul className="ing-list">
            {toBuy.map(({ ri, ing }) => (
              <li key={ri.id}>
                <span className="qty">{recipeQty(ri.qty * factor, ing)}</span>
                <span>{ing.unit === 'each' ? countedName(ing, ri.qty * factor) : ing.name}{ri.prep && <span className="muted">, {ri.prep}</span>}</span>
              </li>
            ))}
          </ul>
          {pantry.length > 0 && <>
            <h4>From the cupboard</h4>
            <ul className="ing-list">{pantry.map(({ ri, ing }) => <li key={ri.id}><span className="qty">{recipeQty(ri.qty * factor, ing)}</span><span>{ing.name}</span></li>)}</ul>
          </>}
        </section>
        <section>
          <h3>Method</h3>
          <ol className="steps">{r.steps.map((st, i) => <li key={i}>{st}</li>)}</ol>
        </section>
      </div>

      <footer className="sheet-foot">
        <span className="muted small">Cooked it?</span>
        <div className="rating" role="group" aria-label="Rate this recipe">
          {([[1, 'Loved it'], [0, 'It was OK'], [-1, 'Not for me']] as const).map(([v, label]) => (
            <button key={v} type="button" className={`chip${rating === v ? ' on' : ''}`} aria-pressed={rating === v} onClick={() => rate(r.id, rating === v ? null : v)}>{label}</button>
          ))}
        </div>
        <button type="button" className="link" onClick={() => toggleHidden(r.id)}>{hidden ? 'Suggest this again' : "Don't suggest this"}</button>
      </footer>
    </article>
  );
}
