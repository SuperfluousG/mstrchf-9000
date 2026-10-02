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
  const invoke=vi.fn(async(name:string)=>{
    if(name==='planner_features') return features;
    if(name==='planner_dispatch') {
      if(outcome==='acknowledged') return snapshot('2');
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
