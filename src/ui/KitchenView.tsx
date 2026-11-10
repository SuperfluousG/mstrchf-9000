import { type CSSProperties, useRef, useState } from 'react';
import tableUrl from '../brand/hero-table.webp';
import { catalogue } from '../data';
import { type Market, MARKET_ORDER, MARKETS, retailersFor } from '../domain/market';
import {
  ALLERGEN_LABELS, CUISINE_GROUPS, DEFAULT_PROFILE, PROTEIN_CHOICES, groupAffinity, marketOf, retailerOf, setGroupAffinity, setProteinAffinity,
  type Affinity, type Profile,
} from '../domain/profile';
import type { Allergen, Diet, MealType } from '../domain/types';
import { saveProfile, toggleFavourite, toggleHidden } from '../state/actions';
import { saveFile, useCanSaveFiles } from '../platform';
import { type FileStatus, useFileStatus } from '../state/fileSync';
import { exportBackup, importBackup, resetAll, setState, useAppState } from '../state/store';
import { Heat, Stepper } from './bits';

const AFFINITY_LABEL: Record<Affinity, string> = { [-1]: 'Skip', 0: '', 1: 'Like', 2: 'Love' } as Record<Affinity, string>;
const NEXT: Record<Affinity, Affinity> = { [-1]: 0, 0: 1, 1: 2, 2: -1 } as Record<Affinity, Affinity>;
const HEAT_WORDS = ['None', 'Gentle', 'Spicy', 'Hot'];
const COUNTRY_TAB: Record<Market, string> = { uk: 'UK', us: 'US', ca: 'Canada' };

/** Taste profile editor, used for first-run setup and later edits. */
export function ProfileEditor({ initial, onSave, saveLabel }: { initial: Profile; onSave: (p: Profile) => void; saveLabel: string }) {
  const [p, setP] = useState<Profile>(initial);
  const [dislikeQ, setDislikeQ] = useState('');
  const set = <K extends keyof Profile>(k: K, v: Profile[K]) => setP(prev => ({ ...prev, [k]: v }));
  const dislikeMatches = dislikeQ.length < 2 ? [] : [...catalogue.ingredients.values()]
    .filter(i => !i.pantry && !p.dislikes.includes(i.id) && i.name.toLowerCase().includes(dislikeQ.toLowerCase())).slice(0, 6);

  const market = marketOf(p);
  const shop = retailerOf(p);

  return (
    <form className="profile" onSubmit={e => { e.preventDefault(); onSave({ ...p, market, retailer: shop?.id ?? null }); }}>
      <fieldset>
        <legend>Where you shop</legend>
        <div className="row">
          <span>Country</span>
          <div className="segmented" role="radiogroup" aria-label="Country">
            {MARKET_ORDER.map(m => (
              <button key={m} type="button" role="radio" aria-checked={market === m} className={market === m ? 'on' : ''}
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
        <p className="muted small">Sets ingredient names, measures, pack sizes and price estimates for {MARKETS[market].short}, and which shop Claude fills the basket at. Any online grocer works; the listed ones have a step-by-step guide.</p>
      </fieldset>

      <fieldset>
        <legend>Who's eating</legend>
        <div className="row"><span>People at each meal</span><Stepper label="people" value={p.people} min={1} max={8} onChange={n => set('people', n)} /></div>
        <div className="row">
          <span>Diet</span>
          <div className="segmented" role="radiogroup" aria-label="Diet">
            {(['omnivore', 'pescatarian', 'vegetarian', 'vegan'] as Diet[]).map(d => (
              <button key={d} type="button" role="radio" aria-checked={p.diet === d} className={p.diet === d ? 'on' : ''} onClick={() => set('diet', d)}>
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
                <button key={id} type="button" className="chip on" onClick={() => set('dislikes', p.dislikes.filter(x => x !== id))} aria-label={`Remove ${catalogue.ingredients.get(id)?.name}`}>
                  {catalogue.ingredients.get(id)?.name} ×
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

      <fieldset>
        <legend>What you like</legend>
        <div className="row">
          <span>Heat</span>
          <div className="segmented heat-pick" role="radiogroup" aria-label="Preferred heat">
            {[0, 1, 2, 3].map(h => (
              <button key={h} type="button" role="radio" aria-checked={p.spice === h} className={p.spice === h ? 'on' : ''} onClick={() => set('spice', h as Profile['spice'])}>
                <Heat level={h} /> {HEAT_WORDS[h]}
              </button>
            ))}
          </div>
        </div>
        <div className="row top">
          <span>Cuisines <span className="muted small">tap to cycle like → love → skip</span></span>
          <div className="chips">
            {CUISINE_GROUPS.map(g => {
              const a = groupAffinity(p, g.cuisines);
              return (
                <button key={g.label} type="button" className={`chip aff-${a}`} onClick={() => setP(prev => setGroupAffinity(prev, g.cuisines, NEXT[a]))}>
                  {g.label}{AFFINITY_LABEL[a] && <em>{AFFINITY_LABEL[a]}</em>}
                </button>
              );
            })}
          </div>
        </div>
        <div className="row top">
          <span>Proteins</span>
          <div className="chips">
            {PROTEIN_CHOICES.map(c => {
              const a = (p.proteins[c.proteins[0]] ?? 0) as Affinity;
              return (
                <button key={c.label} type="button" className={`chip aff-${a}`} onClick={() => setP(prev => setProteinAffinity(prev, c.proteins, NEXT[a]))}>
                  {c.label}{AFFINITY_LABEL[a] && <em>{AFFINITY_LABEL[a]}</em>}
                </button>
              );
            })}
          </div>
        </div>
      </fieldset>

      <fieldset>
        <legend>Your week</legend>
        {(['dinner', 'lunch', 'breakfast'] as MealType[]).map(m => (
          <div className="row" key={m}><span>{m[0].toUpperCase() + m.slice(1)}s per week</span>
            <Stepper label={`${m}s`} value={p.meals[m]} min={0} max={7} onChange={n => set('meals', { ...p.meals, [m]: n })} /></div>
        ))}
        <div className="row">
          <label htmlFor="maxTime">Weeknight cooking time</label>
          <span className="range"><input id="maxTime" type="range" min={15} max={75} step={5} value={p.maxTime} onChange={e => set('maxTime', Number(e.target.value))} /> <output>{p.maxTime} min</output></span>
        </div>
        <div className="row">
          <span>Budget</span>
          <div className="segmented" role="radiogroup" aria-label="Budget">
            {(['relaxed', 'balanced', 'tight'] as const).map(b => (
              <button key={b} type="button" role="radio" aria-checked={p.budget === b} className={p.budget === b ? 'on' : ''} onClick={() => set('budget', b)}>{b[0].toUpperCase() + b.slice(1)}</button>
            ))}
          </div>
        </div>
        <p className="muted small">A tighter budget leans harder on meals that share fresh ingredients.</p>
      </fieldset>
      <button type="submit" className="btn primary big">{saveLabel}</button>
    </form>
  );
}

export function Onboarding({ onDone }: { onDone: () => void }) {
  return (
    <div className="onboarding">
      <header className="mood-photo onboarding-head" style={{ '--photo': `url(${tableUrl})` } as CSSProperties}>
        <p className="eyebrow">MSTRCHF 9000</p>
        <h1 className="display">Set up your kitchen</h1>
        <p className="lede">Tell it who's eating and what you like. It picks a week of meals from {catalogue.recipes.length} recipes, shares ingredients between them so less goes to waste, and writes the shopping list.</p>
      </header>
      <ProfileEditor initial={DEFAULT_PROFILE} saveLabel="Plan my first week" onSave={p => { saveProfile(p); onDone(); }} />
    </div>
  );
}

export function KitchenView({ openRecipe }: { openRecipe: (id: string) => void }) {
  const s = useAppState();
  const fileStatus = useFileStatus();
  const file = useRef<HTMLInputElement>(null);
  const canSave = useCanSaveFiles();
  const [confirmReset, setConfirmReset] = useState(false);
  const [dataMsg, setDataMsg] = useState('');
  const [msg, setMsg] = useState('');
  if (!s.profile) return null;
  const listOf = (ids: string[]) => ids.map(id => catalogue.byId.get(id)).filter(Boolean);
  return (
    <div className="kitchen">
      <h1 className="display">Your kitchen</h1>
      <ProfileEditor initial={s.profile} saveLabel="Save changes" onSave={p => { saveProfile(p); setMsg('Saved. Re-plan a week to use the new settings.'); }} />
      {msg && <p className="notice" role="status">{msg}</p>}

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
            try { importBackup(await f.text()); setDataMsg('Backup restored.'); } catch (err) { setDataMsg(err instanceof Error ? err.message : 'That file could not be read.'); }
            e.target.value = '';
          }} />
          {!confirmReset && <button type="button" className="btn ghost danger" onClick={() => setConfirmReset(true)}>Start over</button>}
        </div>
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
  if ((status.kind === 'saved' || status.kind === 'saving' || status.kind === 'error') && status.store === 'claude') {
    return <p className="muted small">Saved to your Claude account with this planner, so it is there on any device where you open it. It is kept private to you.</p>;
  }
  return <p className="muted small">Saved on this computer in <code>{'path' in status ? status.path : 'kitchen.json'}</code> as well as in this browser, so clearing the browser loses nothing. A dated copy is kept in the <code>backups</code> folder next to it each day (last 14 days).</p>;
}
