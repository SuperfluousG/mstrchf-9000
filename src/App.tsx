import { useEffect, useState } from 'react';
import { selectMarket } from './data';
import { marketOf } from './domain/profile';
import type { MealType } from './domain/types';
import { addWeeks, mondayOf, weekLabel } from './domain/weeks';
import { planWeekAction } from './state/actions';
import { acceptCodexRevision, useFileStatus } from './state/fileSync';
import { adoptSnapshot, getSaveError, getState, loadLatestKitchen, newWeek, type Pick, retrySave, useAppState } from './state/store';
import { latestCodexSnapshot, onCodexSnapshot } from './codexHost';
import { KitchenView, Onboarding } from './ui/KitchenView';
import { Mark } from './ui/Mark';
import { MenuView } from './ui/MenuView';
import { RecipeSheetContent } from './ui/RecipeSheet';
import { Sheet } from './ui/Sheet';
import { ShopView } from './ui/ShopView';
import { WeekView } from './ui/WeekView';
import { choiceKeys } from './ui/bits';

type Route = { view: 'week' | 'menu' | 'shop' | 'kitchen'; meal?: MealType };

// Plain hashes (#week, #menu-dinner) so the app also works inside a Claude artifact; older #/menu/dinner links still work.
function readRoute(): Route {
  const [view, meal] = location.hash.replace(/^#\/?/, '').split(/[/-]/);
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
  return <a className="wordmark" href="#week" aria-label="MSTRCHF 9000, your week"><Mark className="mark" /><span className="wordmark-name">MSTRCHF<span className="wordmark-edition">9000</span></span></a>;
}

function fileName(path: string): string {
  return path.split(/[\\/]/).pop() ?? path;
}

/** Small "Saved to kitchen.json" note so it's clear the plan is safe outside the browser. */
function SaveStatus({ dirty }: { dirty: boolean }) {
  const f = useFileStatus();
  const text = dirty ? 'Kitchen changes not saved' : f.kind === 'saved' ? (f.store === 'claude' ? 'Saved to Claude' : `Saved to ${fileName(f.path)}`)
    : f.kind === 'saving' ? 'Saving…'
    : f.kind === 'browser-only' ? 'Saved in this browser only'
    : f.kind === 'error' ? (f.store === 'claude' ? 'Not saved to Claude' : 'Not saved to file') : '';
  if (!text) return null;
  return <span className={`save-status ${dirty ? 'dirty' : f.kind}`} role="status" aria-label={text} title={text}><span className="save-long" aria-hidden="true">{text}</span><span className="save-short" aria-hidden="true">{dirty ? 'Unsaved' : f.kind === 'saved' ? 'Saved' : f.kind === 'saving' ? 'Saving…' : f.kind === 'error' ? 'Save failed' : 'Browser only'}</span></span>;
}

function FileError() {
  const f = useFileStatus();
  if (f.kind !== 'error') return null;
  const conflict = f.message.includes('changed in another view');
  return <div className="notice warn" role="alert"><p>The latest changes are saved in this browser, but have not reached {f.store === 'claude' ? 'your Claude account' : 'the kitchen file'}. {conflict ? <><span>Another view has changed this kitchen. Export a backup to keep these edits, then </span><button type="button" className="link" onClick={() => void loadLatestKitchen()}>Load latest kitchen</button>.</> : <button type="button" className="link" onClick={retrySave}>Try saving again</button>}</p><details><summary>Save details</summary><p>{f.path}: {f.message}</p></details></div>;
}

export function App() {
  const s = useAppState();
  // Every view reads the catalogue for this country, so switch it before anything below renders.
  const market = marketOf(s.profile);
  selectMarket(market);
  const [route, setRoute] = useState<Route>(readRoute);
  const thisWeek = mondayOf(new Date());
  const [weekId, setWeekId] = useState(latestCodexSnapshot()?.selectedWeekId ?? thisWeek);
  const weeks = [...new Set([...([0, 1, 2, 3].map(n => addWeeks(thisWeek, n))), weekId])];
  const [recipeView, setRecipeView] = useState<{ id: string; swapFor?: Pick } | null>(null);
  const [kitchenDirty, setKitchenDirty] = useState(false);
  const [browseSwap, setBrowseSwap] = useState<Pick | undefined>();
  const openRecipe = (id: string, swapFor?: Pick) => setRecipeView({ id, swapFor });

  useEffect(() => {
    const on = () => setRoute(readRoute());
    window.addEventListener('hashchange', on);
    return () => window.removeEventListener('hashchange', on);
  }, []);
  useEffect(() => onCodexSnapshot(snapshot => {
    if (snapshot.savedAt < (getState().savedAt ?? 0)) return;
    if (!acceptCodexRevision(snapshot.revision)) return;
    adoptSnapshot(snapshot.state, snapshot.savedAt);
    if (snapshot.selectedWeekId) { setWeekId(snapshot.selectedWeekId); location.hash = 'week'; }
  }), []);
  useEffect(() => { window.scrollTo({ top: 0 }); }, [route.view, weekId]);
  useEffect(() => { if (route.view !== 'menu') setBrowseSwap(undefined); }, [route.view]);

  const go = (r: string) => { location.hash = r.replace('/', '-'); };

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
        <SaveStatus dirty={kitchenDirty} />
        <nav aria-label="Main">
          {NAV.map(n => (
            <a key={n.view} href={`#${n.view}`} className={route.view === n.view ? 'on' : ''} aria-current={route.view === n.view ? 'page' : undefined}>{n.label}</a>
          ))}
        </nav>
      </header>
      {route.view !== 'kitchen' && (
        <div className="weekbar" role="tablist" aria-label="Choose a week" onKeyDown={choiceKeys}>
          {weeks.map((id, i) => {
            const w = s.weeks[id] ?? newWeek(id);
            const state = w.status === 'ordered' ? 'Ordered' : w.picks.length ? `${w.picks.length} meals` : 'Not started';
            return (
              <button key={id} role="tab" type="button" tabIndex={id === weekId ? 0 : -1} aria-selected={id === weekId} className={`weektab${id === weekId ? ' on' : ''}`} onClick={() => { setWeekId(id); setBrowseSwap(undefined); }}>
                <span className="wk-label">{i === 0 ? 'This week' : i === 1 ? 'Next week' : weekLabel(id)}</span>
                <span className="wk-state">{i <= 1 ? `${weekLabel(id)} · ` : ''}{state}</span>
              </button>
            );
          })}
        </div>
      )}
      {route.view !== 'kitchen' && <div className="mobile-week"><label><span className="sr-only">Choose a week</span><select value={weekId} onChange={e => { setWeekId(e.target.value); setBrowseSwap(undefined); }}>{weeks.map((id, i) => <option key={id} value={id}>{i === 0 ? 'This week' : i === 1 ? 'Next week' : 'Week'} · {weekLabel(id)}{s.weeks[id]?.status === 'ordered' ? ' · Ordered' : ''}</option>)}</select></label></div>}
      </div>
      {/* Kitchen stays mounted so navigating cannot discard a settings draft. */}
      <main className="shell" id="main">
        {saveError && <p className="notice warn" role="alert">Changes could not be saved in this browser ({saveError}). Download a backup from Kitchen.</p>}
        <FileError />
        {route.view === 'week' && <WeekView weekId={weekId} openRecipe={openRecipe} go={go} browseSwap={pick => { setBrowseSwap(pick); go(`menu/${pick.meal}`); }} />}
        {route.view === 'menu' && <MenuView key={`${market}-${route.meal ?? 'all'}-${weekId}`} weekId={weekId} initialMeal={route.meal} openRecipe={openRecipe} swapFor={browseSwap} onSwapDone={() => setBrowseSwap(undefined)} />}
        {route.view === 'shop' && <ShopView weekId={weekId} go={go} />}
        <div hidden={route.view !== 'kitchen'}><KitchenView openRecipe={openRecipe} onDirtyChange={setKitchenDirty} /></div>
      </main>
      <Sheet open={!!recipeView} onClose={() => setRecipeView(null)} label="Recipe" wide>
        {recipeView && <RecipeSheetContent key={`${market}-${recipeView.id}`} recipeId={recipeView.id} weekId={weekId} swapFor={recipeView.swapFor} onDone={() => { setRecipeView(null); if (recipeView.swapFor) setBrowseSwap(undefined); }} />}
      </Sheet>
    </>
  );
}
