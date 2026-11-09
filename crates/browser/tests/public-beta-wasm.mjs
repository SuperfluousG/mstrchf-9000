import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';

const wasm=await readFile(process.env.MSTRCHF_WASM_PATH ?? new URL('../target/wasm32-unknown-unknown/release/mstrchf_browser.wasm',import.meta.url));
const module=await WebAssembly.compile(wasm);
assert.deepEqual(WebAssembly.Module.imports(module),[]);
const instance=await WebAssembly.instantiate(module,{});
const {memory,alloc,dealloc,run}=instance.exports;
const call=input=>{
  const bytes=new TextEncoder().encode(JSON.stringify(input));
  const pointer=alloc(bytes.length);
  new Uint8Array(memory.buffer,pointer,bytes.length).set(bytes);
  const packed=run(pointer,bytes.length);
  const output=Number(packed&0xffffffffn),length=Number(packed>>32n);
  const response=JSON.parse(new TextDecoder().decode(new Uint8Array(memory.buffer,output,length)));
  dealloc(pointer,bytes.length);dealloc(output,length);
  assert.equal(response.ok,true,JSON.stringify(response));
  return response.value;
};
const now='2026-12-18T12:00:00Z';
const inventory=call({op:'public_starter_catalogue'});
assert.equal(inventory.recipes.length,13);
assert.ok(inventory.recipes.every(entry=>entry.admission==='public_reviewed'&&entry.recipe.image.kind==='generated_illustration'));
let state=call({op:'initialize',household_id:'hh_a0000000000000000000000000000003',diner_id:'dn_a0000000000000000000000000000003',zone:'Europe/London',today:'2026-12-18',now,demo:true,public_beta:true});
assert.equal(state.engineering_demo,true);
assert.equal(state.public_catalogue_pin.entries.length,13);
assert.equal(state.snapshot.privacy.use_ai_features,false);
assert.equal(state.snapshot.privacy.include_household_requirements,false);
assert.equal(state.snapshot.weeks[0].range.monday,'2026-12-28');
assert.equal(state.snapshot.weeks[1].range.monday,'2027-01-04');
const other=structuredClone(state.snapshot.weeks[1]);
for(const [ordinal,cell] of [0,1,2].entries()){
  const week=state.snapshot.weeks[0],slot=week.cells[cell].slot;
  const board=call({op:'board',state,week_id:week.range.week_id,slot,now});
  assert.equal(board.details.test_kitchen,true);
  assert.ok(board.choices.length>0);
  const choice=board.choices[0];
  const command={schema_version:1,command_id:`cm_a${String(ordinal+1).padStart(31,'0')}`,expected_snapshot_revision:state.snapshot.snapshot_revision,command:{kind:'save_week',data:{expected_week_revision:week.revision,week:{week_id:week.range.week_id,defaults_snapshot:week.defaults_snapshot,day_enabled:week.day_enabled,meal_type_enabled:week.meal_type_enabled,cells:week.cells,plan:choice.plan}}}};
  state=call({op:'dispatch',state,command,now}).state;
  state=call({op:'validate_state',state,now});
  const saved=call({op:'board',state,week_id:week.range.week_id,slot,now});
  const quantities=saved.details.selected_recipe_quantities;
  assert.deepEqual(quantities.recipe_reference,choice.selection.recipe);
  assert.deepEqual(quantities.slot,slot);
  assert.deepEqual(quantities.source_servings,{numerator:'2',denominator:'1'});
  assert.deepEqual(quantities.requested_servings,{numerator:'1',denominator:'1'});
  assert.deepEqual(quantities.multiplier,{numerator:'1',denominator:'2'});
  assert.equal(saved.details.shopping_list.contains_engineering,false);
  if(ordinal===0){
    for(const ingredient of quantities.ingredients){
      assert.deepEqual(ingredient.quantity,saved.details.shopping_list.lines.find(line=>line.demand.ingredient_id===ingredient.ingredient_id).demand.quantity);
    }
  }
}
assert.deepEqual(state.snapshot.weeks[1],other);
const legacy=JSON.parse(await readFile(new URL('fixtures/legacy-engineering-receipts.json',import.meta.url),'utf8'));
assert.deepEqual(call({op:'validate_state',state:legacy.state,now}),legacy.state);
console.log(JSON.stringify({public_starters:13,imports:0,fictional_public_demo:true,saved_real_meals:3,exact_scaling_and_demand:'passed',independent_weeks:'passed',legacy_cold_restore:'passed',wasm_bytes:wasm.length}));
