import {readFileSync} from 'node:fs';
import {afterEach,it,expect,vi} from 'vitest';
import {cleanup,render,screen,fireEvent} from '@testing-library/react';
import {MealControls} from '../../app/features/week/PlannerViews';
import {slotKey} from '../../app/features/week/plannerTypes';
import type {HouseholdProfile,Week,MealSelection} from '../../app/generated/contracts';

const holder=vi.hoisted(()=>({current:{} as any}));
vi.mock('../../app/runtime/PlannerProvider',()=>({usePlanner:()=>holder.current}));
afterEach(cleanup);
function setup(){
 const profile=JSON.parse(readFileSync('tests/fixtures/households/p03ab/a1a2-profile.json','utf8')) as HouseholdProfile;
 const week=JSON.parse(readFileSync('tests/fixtures/households/p03ab/a1a2-week.json','utf8')) as Week;
 const source=JSON.parse(readFileSync('tests/fixtures/households/p03a3/ruling004-current--selected-week.json','utf8')).plan.selections[0] as MealSelection;
 source.slot=structuredClone(week.cells[2]!.slot);source.recipe.recipe_id='rc_8000000000000000000000000000000d' as never;
 source.batch_use={kind:'new_batch',data:{batch_id:'ba_00000000000000000000000000000001' as never,production_servings:{numerator:'4',denominator:'1'}}};
 week.plan.selections=[source];week.plan.batch_links=[];
 const cell=week.cells[4]!;cell.choice='on_the_move';cell.attendee_ids=[profile.diners[0]!.diner_id];cell.occasion=null;
 const onOptions=vi.fn();holder.current={session:{canMutate:true,submit:vi.fn()},state:{snapshot:{profile,weeks:[week]}}};
 const view=render(<MealControls week={week} cell={cell} onOptions={onOptions} portableAvailable/>);
 fireEvent.change(screen.getByLabelText('Use a saved source batch'),{target:{value:slotKey(source.slot)}});
 return {profile,week,cell,onOptions,view,source};
}
const confirm=()=>screen.getByRole('checkbox',{name:/I can keep this lunch/}) as HTMLInputElement;
function fill(){
 fireEvent.change(screen.getByLabelText('Finish packing at home'),{target:{value:'07:30'}});
 fireEvent.change(screen.getByLabelText('Leave home at'),{target:{value:'07:45'}});
}
it('requires a fresh declaration and discards the prior preview when any bound option changes',()=>{
 const {onOptions,source}=setup();expect(confirm().checked).toBe(false);expect(confirm().disabled).toBe(true);
 fill();expect(confirm().checked).toBe(false);expect(confirm().disabled).toBe(false);
 fireEvent.click(confirm());expect(confirm().checked).toBe(true);
 fireEvent.click(screen.getByRole('button',{name:'Preview meal options'}));
 expect(onOptions.mock.lastCall?.[0].portableCold).toEqual({packingCompleteTime:'07:30:00',packingResolution:'unambiguous',departTime:'07:45:00',departureResolution:'unambiguous',chillingConfirmed:true});
 fireEvent.change(screen.getByLabelText('Eat-by override'),{target:{value:'12:15'}});
 expect(confirm().checked).toBe(false);expect(onOptions.mock.lastCall?.[0]).toEqual({});
 fireEvent.change(screen.getByLabelText('Eat-by override'),{target:{value:''}});
 expect(confirm().checked).toBe(false);
 fireEvent.click(screen.getByRole('button',{name:'Preview meal options'}));
 expect(onOptions.mock.lastCall?.[0].portableCold).toBeUndefined();
 fireEvent.click(confirm());
 fireEvent.change(screen.getByLabelText('Use a saved source batch'),{target:{value:''}});
 expect(screen.queryByRole('checkbox',{name:/I can keep this lunch/})).toBeNull();
 fireEvent.change(screen.getByLabelText('Use a saved source batch'),{target:{value:slotKey(source.slot)}});
 expect(confirm().checked).toBe(false);
});
it('does not carry the confirmation across household changes or unavailable food',()=>{
 const {week,cell,onOptions,view,profile}=setup();fill();fireEvent.click(confirm());
 const next=structuredClone(profile);next.revision=(BigInt(profile.revision)+1n).toString();
 holder.current.state.snapshot.profile=next;
 view.rerender(<MealControls week={week} cell={cell} onOptions={onOptions} portableAvailable/>);
 expect(confirm().checked).toBe(false);expect(confirm().disabled).toBe(true);
 view.rerender(<MealControls key="fresh" week={week} cell={cell} onOptions={onOptions}/>);
 expect(screen.queryByRole('checkbox',{name:/I can keep this lunch/})).toBeNull();
});
