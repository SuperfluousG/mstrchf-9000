// One-time independent fixture from the unchanged pre-pin module, never a host
// state mutation. Exclusive creation preserves the original receipt evidence.
import assert from 'node:assert/strict';
import {createHash} from 'node:crypto';
import {readFile, writeFile} from 'node:fs/promises';
const wasm = await readFile(new URL('../target/wasm32-unknown-unknown/release/mstrchf_browser.wasm', import.meta.url));
const wasm_sha256 = createHash('sha256').update(wasm).digest('hex');
assert.equal(wasm_sha256, '4bb97b7e2171f80c37ce2ccf75890895e9f7f6b7a57ea27032255607006d13b4');
const module = await WebAssembly.compile(wasm);
assert.deepEqual(WebAssembly.Module.imports(module), []);
const {exports:e} = await WebAssembly.instantiate(module, {});
const now = '2026-10-12T12:00:00Z', today = '2026-10-12';
function call(request) {
  const input = new TextEncoder().encode(JSON.stringify(request)), pointer = e.alloc(input.length);
  new Uint8Array(e.memory.buffer, pointer, input.length).set(input);
  const packed = e.run(pointer, input.length), out = Number(packed & 0xffffffffn), length = Number(packed >> 32n);
  const response = JSON.parse(new TextDecoder().decode(new Uint8Array(e.memory.buffer, out, length)));
  e.dealloc(pointer, input.length); e.dealloc(out, length);
  assert.equal(response.ok, true, JSON.stringify(response));
  return response.value;
}
let state = call({op:'initialize', household_id:'hh_00000000000000000000000000000005', diner_id:'dn_00000000000000000000000000000005', zone:'Europe/London', today, now, demo:true});
const acknowledgements = [];
function dispatch(command, n) {
  const envelope = {schema_version:1,command_id:`cm_${n.toString(16).padStart(32,'0')}`,expected_snapshot_revision:state.snapshot.snapshot_revision,command};
  const response = call({op:'dispatch', state, command:envelope, command_json:JSON.stringify(envelope), now,today});
  state=response.state; acknowledgements.push(response.receipt);
}
let week = state.snapshot.weeks[0];
let board = call({op:'board',state,week_id:week.range.week_id,slot:week.cells[2].slot,now});
dispatch({kind:'save_week',data:{expected_week_revision:week.revision,week:{week_id:week.range.week_id,defaults_snapshot:week.defaults_snapshot,day_enabled:week.day_enabled,meal_type_enabled:week.meal_type_enabled,cells:week.cells,plan:board.choices[0].plan}}},1);
board = call({op:'board',state,now});
const demand=board.details.derived.demand, line=demand.lines.find(line=>line.essential);
assert(line);
dispatch({kind:'confirm_essentials',data:{confirmations:[{line_id:line.line_id,choice:'have_enough',displayed_quantity:line.quantity,demand_digest:demand.digest,confirmed_at:now}]}},2);
dispatch({kind:'set_privacy',data:{policy:{...state.snapshot.privacy,use_ai_features:true},expected_policy_revision:state.snapshot.privacy.revision}},3);
week=state.snapshot.weeks[0];
const slot=week.cells[5].slot;
const prepared=call({op:'prepare_proposal',state,week_id:week.range.week_id,slots:[slot],mode:'fill_open',allow_repeats:false,nonce:'legacy-receipt-proof',now});
const request=prepared.request;
const result={proposal_id:request.proposal_id,context_digest:request.context_digest,selections:[{slot,recipe:request.slots[0].choices[0].recipe}]};
const returned=call({op:'return_proposal',state:prepared.state,result,now});
state=returned.state;
dispatch(returned.command,4);
board=call({op:'board',state,now});
assert.equal(state.receipts.length,4);
assert(state.receipts.every(record=>!Object.hasOwn(record,'food_catalogue_pin')));
call({op:'validate_state',state,now});
const fixture={wasm_sha256,now,today,state,acknowledgements,derived:board.details.derived,accepted_result:result};
await writeFile(new URL('./fixtures/legacy-engineering-receipts.json',import.meta.url),JSON.stringify(fixture),{flag:'wx'});
console.log(JSON.stringify({wasm_sha256,receipts:state.receipts.length,essentials:state.essentials.length,accepted_result:true}));
