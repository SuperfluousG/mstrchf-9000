import { type CSSProperties, useEffect, useRef, useState } from 'react';
import tableUrl from '../brand/hero-table.webp';
import { catalogue, catalogueFor } from '../data';
import { type Market, MARKET_ORDER, MARKETS, retailersFor } from '../domain/market';
import {
  ALLERGEN_LABELS, CUISINE_GROUPS, DEFAULT_PROFILE, PROTEIN_CHOICES, groupAffinity, marketOf, retailerOf, setGroupAffinity, setProteinAffinity,
  type Affinity, type Profile,
} from '../domain/profile';
import type { Allergen, Diet, MealType } from '../domain/types';
import { saveProfile, toggleFavourite, toggleHidden } from '../state/actions';
import { saveFile, useCanSaveFiles } from '../platform';
import { type FileStatus, useFileStatus } from '../state/fileSync';
import { exportBackup, importBackup, readBackup, resetAll, setState, useAppState } from '../state/store';
import { choiceKeys, Heat, Stepper } from './bits';

const AFFINITIES: [Affinity, string][] = [[0, 'Neutral'], [1, 'Like'], [2, 'Love'], [-1, 'Skip']];
const DRAFT_KEY = 'mstrchf9000:profile-draft';
function readDraft(initial: Profile): Profile {
  try {
    const draft = JSON.parse(localStorage.getItem(DRAFT_KEY) ?? 'null');
    return draft?.base === JSON.stringify(initial) && draft.profile ? draft.profile : initial;
  } catch { return initial; }
}
const HEAT_WORDS = ['None', 'Gentle', 'Spicy', 'Hot'];
const COUNTRY_TAB: Record<Market, string> = { uk: 'UK', us: 'US', ca: 'Canada' };

/** Taste profile editor, used for first-run setup and later edits. */
export function ProfileEditor({ initial, onSave, saveLabel, onDirtyChange }: { initial: Profile; onSave: (p: Profile) => void; saveLabel: string; onDirtyChange?: (dirty: boolean) => void }) {
  const [p, setP] = useState<Profile>(() => onDirtyChange ? readDraft(initial) : initial);
  const savedProfile = useRef(initial);
  const dirty = JSON.stringify(p) !== JSON.stringify(initial);
  useEffect(() => { if (savedProfile.current !== initial) { savedProfile.current = initial; setP(initial); } }, [initial]);
  useEffect(() => { onDirtyChange?.(dirty); }, [dirty, onDirtyChange]);
  useEffect(() => {
    if (!onDirtyChange) return;
    try { if (dirty) localStorage.setItem(DRAFT_KEY, JSON.stringify({ base: JSON.stringify(initial), profile: p })); else localStorage.removeItem(DRAFT_KEY); } catch { /* The visible save bar still preserves the draft during navigation. */ }
  }, [p, dirty, initial, onDirtyChange]);
  const [dislikeQ, setDislikeQ] = useState('');
  const editorCatalogue = catalogueFor(marketOf(p));
  const set = <K extends keyof Profile>(k: K, v: Profile[K]) => setP(prev => ({ ...prev, [k]: v }));
  const dislikeMatches = dislikeQ.length < 2 ? [] : [...editorCatalogue.ingredients.values()]
    .filter(i => !i.pantry && !p.dislikes.includes(i.id) && i.name.toLowerCase().includes(dislikeQ.toLowerCase())).slice(0, 6);

  const market = marketOf(p);
  const shop = retailerOf(p);

  return (
    <form className={`profile${onDirtyChange ? ' editing' : ' first-run'}`} onSubmit={e => { e.preventDefault(); onSave({ ...p, market, retailer: shop?.id ?? null }); }}>
      <details className="profile-section shop-settings"><summary>Where you shop <span>{COUNTRY_TAB[market]} · {shop?.name ?? 'Another shop'}</span></summary>
      <fieldset>
        <legend className="sr-only">Where you shop</legend>
        <div className="row">
          <span>Country</span>
          <div className="segmented" role="radiogroup" aria-label="Country" onKeyDown={choiceKeys}>
            {MARKET_ORDER.map(m => (
              <button key={m} type="button" role="radio" tabIndex={market === m ? 0 : -1} aria-checked={market === m} className={market === m ? 'on' : ''}
                onClick={() => setP(prev => ({ ...prev, market: m, retailer: undefined }))}>{COUNTRY_TAB[m]}</button>
            ))}
          </div>
        </div>
        <div className="row top">
          <span>Supermarket</span>
          <div className="chips">
            {retailersFor(market).map(r => (
              <button key={r.id} type="button" className={`chip${shop?.id === r.id ? ' on' : ''}`} aria-pressed={shop?.id === r.id} title={r.covers ? `Also ${r.covers}` : undefined}
                onClick={() => set('retailer', r.id)}>{r.name}</button>
            ))}
            <button type="button" className={`chip${!shop ? ' on' : ''}`} aria-pressed={!shop} onClick={() => set('retailer', null)}>Another shop</button>
          </div>
        </div>
        <p className="muted small">Ingredient names, measures and estimates for {MARKETS[market].short}. Listed shops have a retailer guide; only Tesco has been tested through a live basket fill. Other shops may need extra steps.</p>
      </fieldset>
      </details>

      <fieldset>
        <legend>Who's eating</legend>
        <div className="row"><span>People at each meal</span><Stepper label="people" value={p.people} min={1} max={8} onChange={n => set('people', n)} /></div>
        <div className="row">
          <span>Diet</span>
          <div className="segmented" role="radiogroup" aria-label="Diet" onKeyDown={choiceKeys}>
            {(['omnivore', 'pescatarian', 'vegetarian', 'vegan'] as Diet[]).map(d => (
              <button key={d} type="button" role="radio" tabIndex={p.diet === d ? 0 : -1} aria-checked={p.diet === d} className={p.diet === d ? 'on' : ''} onClick={() => set('diet', d)}>
                {d === 'omnivore' ? 'Everything' : d[0].toUpperCase() + d.slice(1)}
              </button>
            ))}
          </div>
        </div>
        <div className="row top">
          <span>Allergies and no-gos</span>
          <div className="chips">
            {(Object.keys(ALLERGEN_LABELS) as Allergen[]).map(a => (
              <button key={a} type="button" className={`chip${p.avoid.includes(a) ? ' on warn-on' : ''}`} aria-pressed={p.avoid.includes(a)}
                onClick={() => set('avoid', p.avoid.includes(a) ? p.avoid.filter(x => x !== a) : [...p.avoid, a])}>{ALLERGEN_LABELS[a]}</button>
            ))}
          </div>
        </div>
        <p className="muted small">Allergy filtering uses each ingredient's typical allergens. Always check the actual pack labels.</p>
        <div className="row top">
          <span>Ingredients you dislike</span>
          <div className="dislikes">
            <div className="chips">
              {p.dislikes.map(id => (
                <button key={id} type="button" className="chip on" onClick={() => set('dislikes', p.dislikes.filter(x => x !== id))} aria-label={`Remove ${editorCatalogue.ingredients.get(id)?.name}`}>
                  {editorCatalogue.ingredients.get(id)?.name} ×
                </button>
              ))}
            </div>
            <input value={dislikeQ} onChange={e => setDislikeQ(e.target.value)} placeholder="Type an ingredient, e.g. mushrooms" aria-label="Find an ingredient you dislike" />
            {dislikeMatches.length > 0 && (
              <div className="chips">
                {dislikeMatches.map(i => <button key={i.id} type="button" className="chip" onClick={() => { set('dislikes', [...p.dislikes, i.id]); setDislikeQ(''); }}>+ {i.name}</button>)}
              </div>
            )}
          </div>
        </div>
      </fieldset>

      <details className="profile-section"><summary>What you like <span>Heat, cuisines and proteins</span></summary><fieldset>
        <legend className="sr-only">What you like</legend>
        <div className="row">
          <span>Heat</span>
          <div className="segmented heat-pick" role="radiogroup" aria-label="Preferred heat" onKeyDown={choiceKeys}>
            {[0, 1, 2, 3].map(h => (
              <button key={h} type="button" role="radio" tabIndex={p.spice === h ? 0 : -1} aria-checked={p.spice === h} className={p.spice === h ? 'on' : ''} onClick={() => set('spice', h as Profile['spice'])}>
                <span aria-hidden="true"><Heat level={h} /></span> {HEAT_WORDS[h]}
              </button>
            ))}
          </div>
        </div>
        <div className="row top">
          <span>Cuisines</span>
          <div className="affinities">
            {CUISINE_GROUPS.map(g => {
              const a = groupAffinity(p, g.cuisines);
              return (
                <label key={g.label} className={`affinity aff-${a}`}><span>{g.label}</span><select aria-label={`${g.label} preference`} value={a} onChange={e => setP(prev => setGroupAffinity(prev, g.cuisines, Number(e.target.value) as Affinity))}>{AFFINITIES.map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label>
              );
            })}
          </div>
        </div>
        <div className="row top">
          <span>Proteins</span>
          <div className="affinities">
            {PROTEIN_CHOICES.map(c => {
              const a = (p.proteins[c.proteins[0]] ?? 0) as Affinity;
              return (
                <label key={c.label} className={`affinity aff-${a}`}><span>{c.label}</span><select aria-label={`${c.label} preference`} value={a} onChange={e => setP(prev => setProteinAffinity(prev, c.proteins, Number(e.target.value) as Affinity))}>{AFFINITIES.map(([value, label]) => <option key={value} value={value}>{label}</option>)}</select></label>
              );
            })}
          </div>
        </div>
      </fieldset>
      </details>

      <details className="profile-section"><summary>Your week <span>{p.meals.dinner} dinner{p.meals.dinner === 1 ? '' : 's'} · {p.meals.lunch} lunch{p.meals.lunch === 1 ? '' : 'es'} · {p.meals.breakfast} breakfast{p.meals.breakfast === 1 ? '' : 's'}</span></summary><fieldset>
        <legend className="sr-only">Your week</legend>
        {(['dinner', 'lunch', 'breakfast'] as MealType[]).map(m => (
          <div className="row" key={m}><span>{m === 'lunch' ? 'Lunches' : m === 'dinner' ? 'Dinners' : 'Breakfasts'} per week</span>
            <Stepper label={m === 'lunch' ? 'lunches' : `${m}s`} value={p.meals[m]} min={0} max={7} onChange={n => set('meals', { ...p.meals, [m]: n })} /></div>
        ))}
        <div className="row">
          <label htmlFor="maxTime">Weeknight cooking time</label>
          <span className="range"><input id="maxTime" type="range" min={15} max={75} step={5} value={p.maxTime} onChange={e => set('maxTime', Number(e.target.value))} /> <output>{p.maxTime} min</output></span>
        </div>
        <div className="row">
          <span>Budget</span>
          <div className="segmented" role="radiogroup" aria-label="Budget" onKeyDown={choiceKeys}>
            {(['relaxed', 'balanced', 'tight'] as const).map(b => (
              <button key={b} type="button" role="radio" tabIndex={p.budget === b ? 0 : -1} aria-checked={p.budget === b} className={p.budget === b ? 'on' : ''} onClick={() => set('budget', b)}>{b[0].toUpperCase() + b.slice(1)}</button>
            ))}
          </div>
        </div>
        <p className="muted small">A tighter budget leans harder on meals that share fresh ingredients.</p>
      </fieldset>
      </details>
      <div className={`profile-save${dirty ? ' dirty' : ''}`}>
        {onDirtyChange && dirty && <span role="status">Unsaved kitchen changes</span>}
        {onDirtyChange && dirty && <button type="button" className="btn ghost" onClick={() => setP(initial)}>Discard changes</button>}
        {!onDirtyChange && <span className="small">{p.people} people · {p.meals.dinner + p.meals.lunch + p.meals.breakfast} meals</span>}
        <button type="submit" className="btn primary big" disabled={onDirtyChange ? !dirty : p.meals.dinner + p.meals.lunch + p.meals.breakfast === 0}>{saveLabel}</button>
      </div>
    </form>
  );
}

export function Onboarding({ onDone }: { onDone: () => void }) {
  return (
    <div className="onboarding">
      <header className="mood-photo onboarding-head" style={{ '--photo': `url(${tableUrl})` } as CSSProperties}>
        <h1 className="display">Set up your kitchen</h1>
        <p className="lede">Who's eating? Start with the essentials; fine-tune your taste whenever you like. We'll pick the meals and work out the shopping.</p>
      </header>
      <ProfileEditor initial={DEFAULT_PROFILE} saveLabel="Plan my first week" onSave={p => { saveProfile(p); onDone(); }} />
    </div>
  );
}

export function KitchenView({ openRecipe, onDirtyChange }: { openRecipe: (id: string) => void; onDirtyChange: (dirty: boolean) => void }) {
  const s = useAppState();
  const fileStatus = useFileStatus();
  const file = useRef<HTMLInputElement>(null);
  const canSave = useCanSaveFiles();
  const [confirmReset, setConfirmReset] = useState(false);
  const [dataMsg, setDataMsg] = useState('');
  const [msg, setMsg] = useState('');
  const [restore, setRestore] = useState<{ name: string; text: string; people: number | null; weeks: number } | null>(null);
  if (!s.profile) return null;
  const listOf = (ids: string[]) => ids.map(id => catalogue.byId.get(id)).filter(Boolean);
  return (
    <div className="kitchen">
      <h1 className="display">Your kitchen</h1>
      {msg && <p className="notice" role="status">{msg}</p>}
      <ProfileEditor initial={s.profile} onDirtyChange={onDirtyChange} saveLabel="Save changes" onSave={p => { saveProfile(p); setMsg('Kitchen updated. Existing meals stay in place; re-plan a draft week to use your new preferences.'); }} />

      <section className="kitchen-lists">
        <div>
          <h2>Favourites</h2>
          {s.favourites.length === 0 ? <p className="muted small">Heart a recipe to see it more often.</p> : (
            <ul>{listOf(s.favourites).map(r => <li key={r!.id}><button type="button" className="link" onClick={() => openRecipe(r!.id)}>{r!.title}</button> <button type="button" className="link muted" onClick={() => toggleFavourite(r!.id)}>remove</button></li>)}</ul>
          )}
        </div>
        <div>
          <h2>Hidden</h2>
          {s.hidden.length === 0 ? <p className="muted small">Recipes you hide never get suggested.</p> : (
            <ul>{listOf(s.hidden).map(r => <li key={r!.id}>{r!.title} <button type="button" className="link" onClick={() => toggleHidden(r!.id)}>show again</button></li>)}</ul>
          )}
        </div>
        <div>
          <h2>In the cupboard</h2>
          {s.cupboard.length === 0 ? <p className="muted small">Tick long-life items on the shopping list and they stay off future lists.</p> : (
            <ul className="chips">{s.cupboard.map(id => (
              <li key={id}><button type="button" className="chip on" onClick={() => setState(st => ({ ...st, cupboard: st.cupboard.filter(x => x !== id) }))}>{catalogue.ingredients.get(id)?.name} ×</button></li>
            ))}</ul>
          )}
        </div>
      </section>

      <section className="data">
        <h2>Your data</h2>
        <DataNote status={fileStatus} />
        <div className="row-btns">
          {canSave && <button type="button" className="btn ghost" onClick={async () => {
            const r = await saveFile(`mstrchf-backup-${new Date().toISOString().slice(0, 10)}.json`, exportBackup());
            if (r === 'unavailable') setDataMsg('Saving files is not available here.');
            else if (r === 'busy') setDataMsg('Another save is waiting for you. Try again in a moment.');
          }}>Download backup</button>}
          <button type="button" className="btn ghost" onClick={() => file.current?.click()}>Restore backup</button>
          <input ref={file} type="file" accept="application/json" hidden onChange={async e => {
            const f = e.target.files?.[0]; if (!f) return;
            try {
              if (f.size > 4_500_000) throw new Error('That backup is too large to restore here. Your kitchen has not changed.');
              const text = await f.text(); const backup = readBackup(text);
              setRestore({ name: f.name, text, people: backup.profile?.people ?? null, weeks: Object.keys(backup.weeks).length }); setDataMsg('');
            } catch (err) { setDataMsg(err instanceof Error ? err.message : 'That file could not be read.'); }
            e.target.value = '';
          }} />
          {!confirmReset && <button type="button" className="btn ghost danger" onClick={() => setConfirmReset(true)}>Start over</button>}
        </div>
        {restore && <div className="confirm-reset" role="region" aria-label="Review backup"><p><strong>{restore.name}</strong> · {restore.weeks} saved {restore.weeks === 1 ? 'week' : 'weeks'}{restore.people ? ` · ${restore.people} people` : ' · no household'}</p><p>Restoring replaces this kitchen's plans, settings and favourites. Download your current backup first if you want to keep it.</p><div className="row-btns"><button type="button" className="btn primary" onClick={() => { importBackup(restore.text); setRestore(null); setDataMsg('Backup restored.'); }}>Restore this backup</button><button type="button" className="btn ghost" onClick={() => setRestore(null)}>Cancel restore</button></div></div>}
        {confirmReset && (
          <div className="confirm-reset" role="alert">
            <p>Delete all plans, settings, favourites and ratings{fileStatus.kind === 'saved' || fileStatus.kind === 'saving' ? `, here and in ${fileStatus.store === 'claude' ? 'your Claude account' : 'the kitchen file'}` : ''}? This cannot be undone{canSave ? ' unless you download a backup first' : ''}.</p>
            <div className="row-btns">
              <button type="button" className="btn primary" onClick={() => { setConfirmReset(false); resetAll(); }}>Yes, delete everything</button>
              <button type="button" className="btn ghost" onClick={() => setConfirmReset(false)}>Cancel</button>
            </div>
          </div>
        )}
        {dataMsg && <p className="notice" role="status">{dataMsg}</p>}
      </section>
    </div>
  );
}

function DataNote({ status }: { status: FileStatus }) {
  if (status.kind === 'error') return <p className="muted small">The latest changes are in this browser. Saving to {status.store === 'claude' ? 'your Claude account' : status.path} failed. Retry from the save notice above, or download a backup here.</p>;
  if (status.kind === 'saving') return <p className="muted small" role="status">Saving your kitchen… Changes remain in this browser while the save completes.</p>;
  if (status.kind === 'browser-only') {
    const why = status.reason === 'artifact-no-store'
      ? 'You can use this shared copy, but it cannot save to your Claude account, so your kitchen stays in this browser.'
      : status.reason === 'write-refused'
        ? 'Claude storage stopped accepting changes, so they stay in this browser for now.'
        : status.reason === 'read-failed'
          ? 'Your saved kitchen could not be loaded, so changes stay in this browser for now. Reload to try again.'
          : 'This copy of the app saves in this browser only.';
    return <p className="muted small">{why} Download a backup to keep it safe or move it to another device.</p>;
  }
  if (status.kind === 'saved' && status.store === 'claude') {
    return <p className="muted small">Saved to your Claude account with this planner, so it is there on any device where you open it. It is kept private to you.</p>;
  }
  return <p className="muted small">Your saved kitchen is on this computer in <code>{'path' in status ? status.path : 'kitchen.json'}</code>, as well as in this browser. Daily copies are kept in the <code>backups</code> folder for 14 days. Unsaved settings drafts stay in this browser until you save them.</p>;
}
