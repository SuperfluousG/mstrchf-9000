import {createHash} from 'node:crypto';
import {readFileSync} from 'node:fs';
import {afterEach,it,expect,vi} from 'vitest';
import {cleanup,render,screen,fireEvent,waitFor,within} from '@testing-library/react';
import {DevelopmentMealBoard} from '../../app/features/week/DevelopmentMealBoard';
import {PlannerProvider} from '../../app/runtime/PlannerProvider';
import {createPlannerClient} from '../../app/runtime/tauriClient';
import {snapshot} from '../fixtures/ui/snapshot';
import type {Week,MealSelection,Snapshot,BatchId} from '../../app/generated/contracts';
afterEach(cleanup);
// Complete transport doubles from contract fixtures; these tests make no native food claim.
function initial():Snapshot {
 const s=snapshot('3');
 const w=JSON.parse(readFileSync('tests/fixtures/households/p03ab/a1a2-week.json','utf8')) as Week;
 w.revision='1';w.plan.revision='1';w.plan.selections=[];w.plan.batch_links=[];
 s.weeks=[w];return s;
}
function choice(s:Snapshot):MealSelection {
 const w=JSON.parse(readFileSync('tests/fixtures/households/p03a3/ruling004-current--selected-week.json','utf8')) as Week;
 const c=w.plan.selections[0]!;c.slot=structuredClone(s.weeks[0]!.cells[2]!.slot);
 c.servings=c.servings.slice(0,1);c.servings[0]!.preset_ref.meal_type='dinner';
 c.batch_use={kind:'new_batch',data:{batch_id:'ba_00000000000000000000000000000001' as BatchId,production_servings:{numerator:'1',denominator:'1'}}};
 return c;
}

it('navigates four saved weeks without creating renderer dates or changing another plan',async()=>{
 const saved=initial();
 saved.weeks=Array.from({length:4},(_,i)=>{const w=structuredClone(saved.weeks[0]!);w.range.week_id=`wk_${(i+1).toString(16).padStart(32,'0')}` as Week['range']['week_id'];w.range.monday=['2026-12-28','2027-01-04','2027-01-11','2027-01-18'][i]!;return w;});
 const loadBoard=vi.fn(async()=>({week:saved.weeks[0]!,choices:[]}));
 render(<PlannerProvider client={createPlannerClient(async()=>saved)}><DevelopmentMealBoard loadBoard={loadBoard}/></PlannerProvider>);
 const tabs=await screen.findAllByRole('tab');
 expect(tabs).toHaveLength(4);
 expect(tabs[1]!.textContent).toContain('4 Jan');
 fireEvent.click(tabs[1]!);
 await waitFor(()=>expect(loadBoard.mock.calls.length).toBeGreaterThan(1));
 expect(tabs[1]!.getAttribute('aria-selected')).toBe('true');
});

it('exposes breakfast lunch dinner and retained three-by-seven availability',async()=>{
 const saved=initial();
 render(<PlannerProvider client={createPlannerClient(async()=>saved)}><DevelopmentMealBoard loadBoard={async()=>({week:saved.weeks[0]!,choices:[]})}/></PlannerProvider>);
 await screen.findByRole('button',{name:'Breakfast'});
 expect(screen.getByRole('button',{name:'Lunch'})).toBeTruthy();
 fireEvent.click(screen.getByRole('button',{name:'Availability'}));
 expect(screen.getAllByRole('combobox',{name:/^(Monday|Tuesday|Wednesday|Thursday|Friday|Saturday|Sunday) (breakfast|lunch|dinner)$/})).toHaveLength(21);
});
it.each(['saved','write-failed','reload-failed'] as const)('uses current saved state and reports %s honestly',async outcome=>{
 let saved=initial();let reads=0;
 const selected=choice(saved);
 let releaseRead:(s:Snapshot)=>void=()=>{};
 const invoke=vi.fn(async(name:string,args?:Record<string,unknown>)=>{
  if(name==='planner_dispatch') {
   if(outcome==='write-failed')throw {code:'storage_failure',issues:[],retry:'never',truncated:false};
   const text=args!.commandJson as string;const command=JSON.parse(text);
   expect(command.command.kind).toBe('save_week');
   expect(command.command.data.expected_week_revision).toBe('1');
   expect(command.command.data.week.plan.selections).toEqual([selected]);
   saved=structuredClone(saved);saved.snapshot_revision='4';saved.weeks[0]!.revision='2';saved.weeks[0]!.plan.revision='2';saved.weeks[0]!.plan.selections=[selected];
   return {format_version:1,disposition:'applied',receipt_profile:'DevelopmentCurrent',request_encoding:'current_json',request_digest:createHash('sha256').update(text).digest('hex'),command_id:command.command_id,command_kind:'save_week',expected_snapshot_revision:'3',household_id:saved.household_draft.household_id,receipt_snapshot_revision:'4',command_digest:'a'.repeat(64),response_digest:'b'.repeat(64)};
  }
  reads++;
  if(reads===1)return saved;
  if(outcome==='reload-failed')throw 'read unavailable';
  return new Promise<Snapshot>(resolve=>{releaseRead=resolve;});
 });
 const loadBoard=async()=>({week:saved.weeks[0]!,choices:[{title:'Synthetic vegetarian bowl A',selection:selected}]});
 render(<PlannerProvider client={createPlannerClient(invoke)}><DevelopmentMealBoard loadBoard={loadBoard}/></PlannerProvider>);
 const button=await screen.findByRole('button',{name:'Choose Synthetic vegetarian bowl A'});
 expect(screen.queryByText('Meal saved locally.')).toBeNull();
 fireEvent.click(button);
 if(outcome==='saved') {
  await waitFor(()=>expect(reads).toBe(2));
  expect(screen.queryByText('Meal saved locally.')).toBeNull();
  expect(within(screen.getByRole('region',{name:'Selected meal details'})).getByRole('heading').textContent).toBe('An open dinner');
  releaseRead(saved);
  await screen.findByText('Meal saved locally.');
  expect(within(screen.getByRole('region',{name:'Selected meal details'})).getByRole('heading').textContent).toBe('Synthetic vegetarian bowl A');
 } else {
  await screen.findByRole('alert');
  expect(screen.queryByText('Meal saved locally.')).toBeNull();
  expect(within(screen.getByRole('region',{name:'Selected meal details'})).getByRole('heading').textContent).toBe('An open dinner');
  expect((button as HTMLButtonElement).disabled).toBe(true);
  expect(screen.getByRole('button',{name:'Recover meal save'})).toBeTruthy();
 }
 expect(invoke.mock.calls.filter(c=>c[0]==='planner_dispatch')).toHaveLength(1);
});
it('keeps the saved Monday next action when viewing another day',async()=>{
 const saved=initial();const selected=choice(saved);saved.weeks[0]!.plan.selections=[selected];
 render(<PlannerProvider client={createPlannerClient(async()=>saved)}><DevelopmentMealBoard
  loadBoard={async()=>({week:saved.weeks[0]!,choices:[{title:'Synthetic vegetarian bowl A',selection:selected}]})}/></PlannerProvider>);
 await screen.findByText('Monday dinner is saved');
 fireEvent.click(screen.getByRole('button',{name:/Select Tuesday/}));
 expect(screen.getByText('Monday dinner is saved')).toBeTruthy();
 expect(screen.queryByText('Choose Monday dinner')).toBeNull();
 expect(within(screen.getByRole('list',{name:'Seven day meal board'})).getByText('Saved locally')).toBeTruthy();
});
it('retries a failed choices read when the saved revision is unchanged',async()=>{
 const saved=initial();const selected=choice(saved);
 const loadBoard=vi.fn().mockRejectedValue('choices unavailable');
 render(<PlannerProvider client={createPlannerClient(async()=>saved)}><DevelopmentMealBoard loadBoard={loadBoard}/></PlannerProvider>);
 await screen.findByRole('alert');
 loadBoard.mockResolvedValue({week:saved.weeks[0]!,choices:[{title:'Synthetic vegetarian bowl A',selection:selected}]});
 fireEvent.click(screen.getByRole('button',{name:'Refresh saved kitchen'}));
 await screen.findByRole('button',{name:'Choose Synthetic vegetarian bowl A'});
 expect(loadBoard.mock.calls.length).toBeGreaterThan(1);
 expect(screen.queryByRole('alert')).toBeNull();
});
