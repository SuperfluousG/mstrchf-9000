import { catalogue } from '../data';
import { countedName, inlineName, recipeQty } from '../domain/format';
import { CUISINE_LABELS } from '../domain/profile';
import { addRecipe, rate, toggleFavourite, toggleHidden } from '../state/actions';
import { useAppState } from '../state/store';
import { Heat } from './bits';
import { DishArt } from './DishArt';

const DIFFICULTY = ['', 'Easy', 'Medium', 'Involved'];

export function RecipeSheetContent({ recipeId, weekId, onDone }: { recipeId: string; weekId: string; onDone: () => void }) {
  const s = useAppState();
  const r = catalogue.byId.get(recipeId);
  if (!r) return <p>That recipe is no longer in the catalogue.</p>;
  const people = s.profile?.people ?? 2;
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
      <p className="eyebrow">{CUISINE_LABELS[r.cuisine]} · {r.meal}</p>
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

      <div className="sheet-actions">
        {inWeek
          ? <span className="in-week">In this week</span>
          : <button type="button" className="btn primary" onClick={() => { addRecipe(weekId, r.id); onDone(); }}>Add to this week</button>}
        <button type="button" className={`btn ghost${fav ? ' active' : ''}`} aria-pressed={fav} onClick={() => toggleFavourite(r.id)}>{fav ? '♥ Favourite' : '♡ Favourite'}</button>
      </div>

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
          {pantry.length > 0 && <p className="muted small">From the cupboard: {pantry.map(x => inlineName(x.ing.name)).join(', ')}.</p>}
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
