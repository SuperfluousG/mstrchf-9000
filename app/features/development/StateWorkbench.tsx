import { useEffect, useState } from 'react';
import type { ShellFeatures } from '../../generated/shell';
import { usePlanner } from '../../runtime/PlannerProvider';
import { syntheticDraft, type SyntheticExample } from './syntheticExamples';
export function StateWorkbench() {
  const {session,client,state}=usePlanner();
  const [features,setFeatures]=useState<ShellFeatures|null>(null);
  const [featureError,setFeatureError]=useState(false);
  const [example,setExample]=useState<SyntheticExample>('empty');
  const [privacyEdit,setPrivacyEdit]=useState<{ai:boolean;household:boolean}|null>(null);
  useEffect(()=>{let active=true; client.loadFeatures().then(value=>{if(active)setFeatures(value);},()=>{if(active)setFeatureError(true);}); return()=>{active=false;};},[client]);
  const snapshot=state.snapshot;
  const ai=privacyEdit?.ai??snapshot?.privacy.use_ai_features??false;
  const household=privacyEdit?.household??snapshot?.privacy.include_household_requirements??false;
  const supported=(kind:ShellFeatures['supported_commands'][number])=>features?.supported_commands.includes(kind)===true;
  const ready=session.canMutate && features?.data_mode==='synthetic_development' && features.production_runtime_enabled===false;
  const saveDraft=()=>snapshot && session.submit({kind:'save_household_draft',data:{draft:syntheticDraft(example,snapshot),restriction_issue_edits:[]}});
  const savePrivacy=()=>snapshot && session.submit({kind:'set_privacy',data:{expected_policy_revision:snapshot.privacy.revision,
    policy:{...snapshot.privacy,use_ai_features:ai,include_household_requirements:household}}});
  return <main className="state-workbench">
    <header><p className="development-notice" role="note">Development — synthetic data</p><h1>MSTRCHF 9000</h1>
      <p>Native state workbench · contracts005</p></header>
    <section aria-labelledby="state-heading"><h2 id="state-heading">Saved synthetic state</h2>
      {snapshot?<><p>Saved revision {snapshot.snapshot_revision}</p><p>Draft preferences: {snapshot.household_draft.preferences?.quick_meals?'Quick meals preferences':snapshot.household_draft.preferences?'Other saved preferences':'No saved preferences'}</p>
      <p>Saved privacy: AI {snapshot.privacy.use_ai_features?'on':'off'}; household requirements {snapshot.privacy.include_household_requirements?'on':'off'}.</p></>:<p>{state.read==='failed'?'Saved state unavailable':'Reading saved state…'}</p>}
      {state.readError&&<p role="alert">{state.readError}</p>}
      <button type="button" disabled={state.disposition==='pending'} onClick={()=>void session.refresh()}>Refresh saved state</button>
    </section>
    <section aria-labelledby="edit-heading"><h2 id="edit-heading">Fixed synthetic examples</h2>
      <p>These choices are development fixtures. They do not complete household setup.</p>
      {snapshot&&<p>Unresolved restrictions: {snapshot.household_draft.unresolved_restrictions.length}. These examples do not edit them.</p>}
      <label>Synthetic example <select value={example} onChange={event=>setExample(event.target.value as SyntheticExample)}>
        <option value="empty">Empty synthetic preferences</option><option value="quick-meals">Quick meals synthetic preferences</option>
      </select></label>
      <button type="button" disabled={!ready||!supported('save_household_draft')} onClick={()=>void saveDraft()}>Save synthetic draft</button>
      <fieldset><legend>Unsubmitted privacy switches</legend>
        <label><input type="checkbox" checked={ai} onChange={event=>setPrivacyEdit({ai:event.target.checked,household})}/>Use AI features</label>
        <label><input type="checkbox" checked={household} onChange={event=>setPrivacyEdit({ai,household:event.target.checked})}/>Include household requirements</label>
      </fieldset>
      <p>These settings are stored locally. This shell cannot launch an AI or account action.</p>
      <button type="button" disabled={!ready||!supported('set_privacy')} onClick={()=>void savePrivacy()}>Save privacy switches</button>
    </section>
    <section aria-label="Action outcome" aria-live="polite">
      {state.message&&<p role={state.disposition==='unknown'?'alert':'status'}>{state.message}</p>}
      {state.pending&&<p>Retained action: {state.pending.command.kind}. Keep this window open to retain its exact identity.</p>}
      {state.disposition==='unknown'&&<button type="button" onClick={()=>void session.recover()}>Recover exact action</button>}
      {state.disposition==='refused'&&<button type="button" disabled={state.read!=='ready'} onClick={()=>session.reviewRefusal()}>I have reviewed the saved state</button>}
    </section>
    <section aria-labelledby="support-heading"><h2 id="support-heading">Current shell support</h2>
      {featureError?<p role="alert">Native support information is unavailable. Saving remains disabled.</p>:<p>{features?'Draft, privacy and job cancellation commands are implemented.':'Reading native support…'}</p>}
      <p>Support is not household eligibility or provider permission.</p>
      <p>Profile promotion is unavailable. Week, food and AI producers are unavailable in this checkpoint. Account, shopping and calendar actions are unavailable.</p>
      <p>No active job is created by this workbench.</p>
    </section>
  </main>;
}
