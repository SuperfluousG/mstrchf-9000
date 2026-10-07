import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { syntheticDraft } from '../../app/features/development/syntheticExamples';
import { afterEach, it, expect, vi } from 'vitest';
import { cleanup, render, screen, fireEvent, waitFor, within } from '@testing-library/react';
import { StateWorkbench } from '../../app/features/development/StateWorkbench';
import { PlannerProvider } from '../../app/runtime/PlannerProvider';
import { createPlannerClient } from '../../app/runtime/tauriClient';
import { snapshot } from '../fixtures/ui/snapshot';
afterEach(cleanup);
const features = { format_version: 1, data_mode: 'synthetic_development', supported_commands: ['save_household_draft','set_privacy','cancel_job'], production_runtime_enabled: false };
it('removes a resolved initial read warning and enables saving',async()=>{
  let reads=0;
  const invoke=vi.fn(async(name:string)=>{
    if(name==='planner_features') return features;
    if(++reads===1) throw 'read unavailable';
    return snapshot('2');
  });
  render(<PlannerProvider client={createPlannerClient(invoke)}><StateWorkbench /></PlannerProvider>);
  await screen.findByText(/Saved state cannot currently be read/);
  expect((screen.getByRole('button',{name:'Save synthetic draft'}) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole('button',{name:'Refresh saved state'}));
  await screen.findByText('Saved revision 2');
  expect(screen.queryByText(/Saved state cannot currently be read/)).toBeNull();
  expect(screen.queryByText(/restart/)).toBeNull();
  expect((screen.getByRole('button',{name:'Save synthetic draft'}) as HTMLButtonElement).disabled).toBe(false);
});
it.each(['unknown','refused','acknowledged'] as const)('preserves %s action instructions and edits through failed then successful reads',async outcome=>{
  let reads=0;
  const invoke=vi.fn(async(name:string,args?:Record<string,unknown>)=>{
    if(name==='planner_features') return features;
    if(name==='planner_dispatch') {
      if(outcome==='acknowledged') {
        const text=args!.commandJson as string; const command=JSON.parse(text);
        return {format_version:1,disposition:'applied',receipt_profile:'B1Draft005',request_encoding:'current_json',
          request_digest:createHash('sha256').update(text).digest('hex'),command_id:command.command_id,
          command_kind:'save_household_draft',expected_snapshot_revision:'1',household_id:snapshot().household_draft.household_id,
          receipt_snapshot_revision:'2',command_digest:'a'.repeat(64),response_digest:'b'.repeat(64)};
      }
      if(outcome==='refused') throw {code:'context_stale',issues:[{code:'context_stale',path:'/expected_snapshot_revision',entity_id:null}],retry:'never',truncated:false};
      throw 'IPC lost';
    }
    reads++;
    if(reads===2) throw 'read unavailable';
    return snapshot(reads===1?'1':'3');
  });
  render(<PlannerProvider client={createPlannerClient(invoke)}><StateWorkbench /></PlannerProvider>);
  await screen.findByText('Saved revision 1');
  fireEvent.change(screen.getByLabelText('Synthetic example'),{target:{value:'quick-meals'}});
  fireEvent.click(screen.getByLabelText('Use AI features'));
  fireEvent.click(screen.getByRole('button',{name:'Save synthetic draft'}));
  const instruction=outcome==='refused'?/The action was refused/:outcome==='acknowledged'?/follow-up verification failed/:/The outcome is unknown/;
  await screen.findByText(instruction);
  if(outcome==='unknown') fireEvent.click(screen.getByRole('button',{name:'Refresh saved state'}));
  await screen.findByText(/Saved state cannot currently be read/);
  const actionMessage=screen.getByText(instruction).textContent;
  const savedSection=screen.getByRole('region',{name:'Saved synthetic state'});
  expect(within(savedSection).getByRole('alert').textContent).toContain('cannot currently be read');
  const retained=outcome==='refused'?null:screen.getByText(/Retained action:/).textContent;
  if(outcome==='refused') expect((screen.getByRole('button',{name:'I have reviewed the saved state'}) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole('button',{name:'Refresh saved state'}));
  await screen.findByText('Saved revision 3');
  expect(screen.queryByText(/Saved state cannot currently be read/)).toBeNull();
  expect(screen.getByText(instruction).textContent).toBe(actionMessage);
  expect((screen.getByLabelText('Synthetic example') as HTMLSelectElement).value).toBe('quick-meals');
  expect((screen.getByLabelText('Use AI features') as HTMLInputElement).checked).toBe(true);
  for(const name of ['Save synthetic draft','Save privacy switches']) expect((screen.getByRole('button',{name}) as HTMLButtonElement).disabled).toBe(true);
  expect(invoke.mock.calls.filter(c=>c[0]==='planner_dispatch')).toHaveLength(1);
  if(outcome==='refused') {
    expect(screen.queryByText(/Retained action:/)).toBeNull();
    fireEvent.click(screen.getByRole('button',{name:'I have reviewed the saved state'}));
    expect((screen.getByRole('button',{name:'Save synthetic draft'}) as HTMLButtonElement).disabled).toBe(false);
  } else {
    expect(screen.getByText(/Retained action:/).textContent).toBe(retained);
    expect(screen.getByRole('button',{name:'Recover exact action'})).toBeTruthy();
  }
});
it('labels synthetic state, offers fixed examples only and refuses later producers',async()=>{
  const invoke=vi.fn(async(name:string)=>name==='planner_features'?features:snapshot());
  render(<PlannerProvider client={createPlannerClient(invoke)}><StateWorkbench /></PlannerProvider>);
  await screen.findByText('Saved revision 1');
  expect(screen.getByText('Development — synthetic data')).toBeTruthy();
  expect(screen.queryByRole('textbox')).toBeNull();
  expect(screen.getByLabelText('Use AI features').getAttribute('type')).toBe('checkbox');
  expect((screen.getByLabelText('Use AI features') as HTMLInputElement).checked).toBe(false);
  expect(screen.getByText(/Week, food and AI producers are unavailable/)).toBeTruthy();
});
it('retains chosen example and privacy input while unknown, with only exact recovery enabled',async()=>{
  const invoke=vi.fn(async(name:string)=>{
    if(name==='planner_features') return features;
    if(name==='planner_dispatch') throw {code:'storage_failure',issues:[],retry:'never',truncated:false};
    return snapshot();
  });
  render(<PlannerProvider client={createPlannerClient(invoke)}><StateWorkbench /></PlannerProvider>);
  await screen.findByText('Saved revision 1');
  fireEvent.change(screen.getByLabelText('Synthetic example'),{target:{value:'quick-meals'}});
  fireEvent.click(screen.getByLabelText('Use AI features'));
  fireEvent.click(screen.getByRole('button',{name:'Save synthetic draft'}));
  await screen.findByText(/The outcome is unknown/);
  expect((screen.getByLabelText('Synthetic example') as HTMLSelectElement).value).toBe('quick-meals');
  expect((screen.getByLabelText('Use AI features') as HTMLInputElement).checked).toBe(true);
  expect((screen.getByRole('button',{name:'Save privacy switches'}) as HTMLButtonElement).disabled).toBe(true);
  fireEvent.click(screen.getByRole('button',{name:'Recover exact action'}));
  await waitFor(()=>expect(invoke.mock.calls.filter(c=>c[0]==='planner_dispatch')).toHaveLength(2));
});

const fixture=(group:string,name:string)=>JSON.parse(readFileSync(`src-tauri/tests/fixtures/receipt/${group}/${name}`, 'utf8'));
it.each(['empty','quick-meals'] as const)('preserves whole rich draft for %s preferences (U-SA-37/38)',example=>{
  const input=fixture('r2','rich-initial-snapshot.json'); const before=structuredClone(input);
  const expected=fixture('r2',example==='empty'?'rich-empty-overlay-draft.json':'rich-quick-overlay-draft.json');
  const result=syntheticDraft(example,input); expect(result).toEqual(expected); expect(input).toEqual(before);
  result.diners.length=0; expect(input).toEqual(before);
});
it('preserves sparse unresolved draft literally (U-SA-27)',()=>{
 const input=fixture('r1','new-issue-response.json');
 expect(syntheticDraft('empty',input)).toEqual(fixture('r1','issue-preserving-empty-draft.json'));
});
it.each(['empty','quick-meals'] as const)('U-SA-39 %s oracle rejects every reset/ownership mutation',example=>{
 const expected=fixture('r2',example==='empty'?'rich-empty-overlay-draft.json':'rich-quick-overlay-draft.json');
 const good=syntheticDraft(example,fixture('r2','rich-initial-snapshot.json'));expect(good).toEqual(expected);
 const mutations:((draft:any)=>void)[]=[d=>{d.usual_week=null;},d=>{d.connections=null;},
 d=>{d.completed_stages=d.completed_stages.filter((s:string)=>s!=='connections');},d=>{d.completed_stages=d.completed_stages.filter((s:string)=>s!=='usual_week');},
 d=>{d.unresolved_restrictions.reverse();},d=>{const a=d.unresolved_restrictions;[a[0].supplied_text,a[1].supplied_text]=[a[1].supplied_text,a[0].supplied_text];},
 d=>{d.diners[2].restrictions={kind:'unconfirmed',data:{}};},d=>{d.diners[2].restrictions.data.requirements=[];},d=>{d.diners=[];}];
 for(const mutate of mutations){const bad=structuredClone(good);mutate(bad);expect(bad).not.toEqual(expected);}
});
it.each(['empty','quick-meals'] as const)('U-SA-27/29/30 submits only %s preference overlay with explicit empty edits',async example=>{
 const initial=fixture('r2','rich-initial-snapshot.json');let sent:any;
 const invoke=vi.fn(async(name:string,args?:Record<string,unknown>)=>{
  if(name==='planner_features')return features;
  if(name==='planner_dispatch'){sent=JSON.parse(args!.commandJson as string);throw 'synthetic lost response';}
  return initial;
 });
 render(<PlannerProvider client={createPlannerClient(invoke)}><StateWorkbench /></PlannerProvider>);await screen.findByText('Saved revision 10');
 expect(screen.getByText(/Unresolved restrictions: 2/)).toBeTruthy();expect(screen.getByText(/These examples do not edit them/)).toBeTruthy();
 expect(screen.getAllByRole('option')).toHaveLength(2);expect(screen.getByRole('option',{name:'Empty synthetic preferences'})).toBeTruthy();
 expect(screen.getByRole('option',{name:'Quick meals synthetic preferences'})).toBeTruthy();expect(screen.queryByRole('textbox')).toBeNull();
 fireEvent.change(screen.getByLabelText('Synthetic example'),{target:{value:example}});fireEvent.click(screen.getByRole('button',{name:'Save synthetic draft'}));
 await screen.findByText(/The outcome is unknown/);
 expect(sent.command).toEqual({kind:'save_household_draft',data:{draft:fixture('r2',example==='empty'?'rich-empty-overlay-draft.json':'rich-quick-overlay-draft.json'),restriction_issue_edits:[]}});
 expect(initial).toEqual(fixture('r2','rich-initial-snapshot.json'));
});
it('renders preference status without implying household reset',async()=>{
 const invoke=vi.fn(async(name:string)=>name==='planner_features'?features:snapshot());
 render(<PlannerProvider client={createPlannerClient(invoke)}><StateWorkbench /></PlannerProvider>);await screen.findByText('Saved revision 1');
 expect(screen.getByText('Draft preferences: No saved preferences')).toBeTruthy();
});
