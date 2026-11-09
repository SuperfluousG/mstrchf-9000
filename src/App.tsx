import { useEffect, useState } from 'react';
import type { MealType } from './domain/types';
import { addWeeks, mondayOf, weekLabel } from './domain/weeks';
import { planWeekAction } from './state/actions';
import { useFileStatus } from './state/fileSync';
import { getSaveError, newWeek, useAppState } from './state/store';
import { KitchenView, Onboarding } from './ui/KitchenView';
import { MenuView } from './ui/MenuView';
import { RecipeSheetContent } from './ui/RecipeSheet';
import { Sheet } from './ui/Sheet';
import { ShopView } from './ui/ShopView';
import { WeekView } from './ui/WeekView';

type Route = { view: 'week' | 'menu' | 'shop' | 'kitchen'; meal?: MealType };

function readRoute(): Route {
  const [view, meal] = location.hash.replace(/^#\/?/, '').split('/');
  if (view === 'menu') return { view, meal: (['dinner', 'lunch', 'breakfast'].includes(meal) ? meal : undefined) as MealType | undefined };
  if (view === 'shop' || view === 'kitchen') return { view };
  return { view: 'week' };
}

const NAV: { view: Route['view']; label: string }[] = [
  { view: 'week', label: 'Week' },
  { view: 'menu', label: 'Menu' },
  { view: 'shop', label: 'Shop' },
  { view: 'kitchen', label: 'Kitchen' },
];

function Wordmark() {
  return <a className="wordmark" href="#/week" aria-label="MSTRCHF 9000, your week">MSTRCHF<sup>9000</sup></a>;
}

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/** Small "Saved to kitchen.json" note so it's clear the plan is safe outside the browser. */
function SaveStatus() {
  const f = useFileStatus();
  const text = f.kind === 'saved' ? `Saved to ${fileName(f.path)}`
    : f.kind === 'saving' ? 'Saving…'
    : f.kind === 'browser-only' ? 'Saved in this browser only'
    : f.kind === 'error' ? 'Not saved to file' : '';
  if (!text) return null;
  return <span className={`save-status ${f.kind}`} role="status" title={'path' in f ? f.path : undefined}>{text}</span>;
}

function FileError() {
  const f = useFileStatus();
  if (f.kind !== 'error') return null;
  return <p className="notice warn" role="alert">Your changes are saved in this browser but not to {f.path}: {f.message}</p>;
}

export function App() {
  const s = useAppState();
  const [route, setRoute] = useState<Route>(readRoute);
  const thisWeek = mondayOf(new Date());
  const weeks = [0, 1, 2, 3].map(n => addWeeks(thisWeek, n));
  const [weekId, setWeekId] = useState(thisWeek);
  const [recipeId, setRecipeId] = useState<string | null>(null);

  useEffect(() => {
    const on = () => setRoute(readRoute());
    window.addEventListener('hashchange', on);
    return () => window.removeEventListener('hashchange', on);
  }, []);
  useEffect(() => { window.scrollTo({ top: 0 }); }, [route.view, weekId]);

  const go = (r: string) => { location.hash = `/${r}`; };

  if (!s.profile) {
    return (
      <>
        <div className="band">
          <header className="topbar"><Wordmark /></header>
        </div>
        <main className="shell">
          <Onboarding onDone={() => { planWeekAction(thisWeek); go('week'); window.scrollTo({ top: 0 }); }} />
        </main>
      </>
    );
  }

  const saveError = getSaveError();
  return (
    <>
      <div className="band">
      <header className="topbar">
        <Wordmark />
        <SaveStatus />
        <nav aria-label="Main">
          {NAV.map(n => (
            <a key={n.view} href={`#/${n.view}`} className={route.view === n.view ? 'on' : ''} aria-current={route.view === n.view ? 'page' : undefined}>{n.label}</a>
          ))}
        </nav>
      </header>
      {route.view !== 'kitchen' && (
        <div className="weekbar" role="tablist" aria-label="Choose a week">
          {weeks.map((id, i) => {
            const w = s.weeks[id] ?? newWeek(id);
            const state = w.status === 'ordered' ? 'Ordered' : w.picks.length ? `${w.picks.length} meals` : 'Not started';
            return (
              <button key={id} role="tab" type="button" aria-selected={id === weekId} className={`weektab${id === weekId ? ' on' : ''}`} onClick={() => setWeekId(id)}>
                <span className="wk-label">{i === 0 ? 'This week' : i === 1 ? 'Next week' : weekLabel(id)}</span>
                <span className="wk-state">{i <= 1 ? `${weekLabel(id)} · ` : ''}{state}</span>
              </button>
            );
          })}
        </div>
      )}
      </div>
      {saveError &&<p className="notice warn" role="alert">Changes could not be saved in this browser ({saveError}). Download a backup from Kitchen.</p>}
      <FileError />
      <main className="shell">
        {route.view === 'week' && <WeekView weekId={weekId} openRecipe={setRecipeId} go={go} />}
        {route.view === 'menu' && <MenuView key={route.meal ?? 'all'} weekId={weekId} initialMeal={route.meal} openRecipe={setRecipeId} />}
        {route.view === 'shop' && <ShopView weekId={weekId} go={go} />}
        {route.view === 'kitchen' && <KitchenView openRecipe={setRecipeId} />}
      </main>
      <Sheet open={!!recipeId} onClose={() => setRecipeId(null)} label="Recipe" wide>
        {recipeId && <RecipeSheetContent recipeId={recipeId} weekId={weekId} onDone={() => setRecipeId(null)} />}
      </Sheet>
    </>
  );
}
