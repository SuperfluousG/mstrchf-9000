import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
const bytes=await readFile(new URL('../target/wasm32-unknown-unknown/release/mstrchf_browser.wasm',import.meta.url));
const module=await WebAssembly.compile(bytes);assert.deepEqual(WebAssembly.Module.imports(module),[]);
async function boot(){const {exports:e}=await WebAssembly.instantiate(module,{});return request=>{
  const input=new TextEncoder().encode(JSON.stringify(request)),p=e.alloc(input.length);new Uint8Array(e.memory.buffer,p,input.length).set(input);
  const packed=e.run(p,input.length),outp=Number(packed&0xffffffffn),len=Number(packed>>32n);
  const response=JSON.parse(new TextDecoder().decode(new Uint8Array(e.memory.buffer,outp,len)));e.dealloc(p,input.length);e.dealloc(outp,len);return response;
};}
const now='2026-10-12T12:00:00Z',start=performance.now();
const good=(call,request)=>{const response=call(request);assert.equal(response.ok,true,JSON.stringify(response));return response.value;};
const envelope=(state,command,n)=>({schema_version:1,command_id:`cm_${n.toString(16).padStart(32,'0')}`,expected_snapshot_revision:state.snapshot.snapshot_revision,command});
let call=await boot();
let state=good(call,{op:'initialize',household_id:'hh_00000000000000000000000000000003',diner_id:'dn_00000000000000000000000000000003',zone:'Europe/London',today:'2026-10-12',now,demo:true});
let policy={...state.snapshot.privacy,use_ai_features:true,confirmed_at:now};
let command=envelope(state,{kind:'set_privacy',data:{policy,expected_policy_revision:policy.revision}},1);
state=good(call,{op:'dispatch',state,command,command_json:JSON.stringify(command),now}).state;
const week=state.snapshot.weeks[0],slot=week.cells[2].slot;
const prepared=good(call,{op:'prepare_proposal',state,week_id:week.range.week_id,slots:[slot],mode:'fill_open',allow_repeats:false,nonce:'e13445c5-14a4-4d69-bcbd-6b900f8b16ed',now});
for(const text of ['household','restrictions','dn_','servings','portions'])assert.ok(!JSON.stringify(prepared.request).includes(text));
const result={proposal_id:prepared.request.proposal_id,context_digest:prepared.request.context_digest,selections:[{slot,recipe:prepared.request.slots[0].choices[0].recipe}]};
const returned=good(call,{op:'return_proposal',state:prepared.state,result,now});
// Restores must cancel both awaiting-return and review-ready private scopes.
// A new instance exercises full admission instead of a prior success cache.
const restoreCall=await boot();
for(const saved of [prepared.state,returned.state]){
  const restored=good(restoreCall,{op:'validate_state',state:JSON.parse(JSON.stringify(saved)),now});
  const cleared=good(restoreCall,{op:'revoke_pending_proposal',state:restored,now});
  const expected=structuredClone(saved);expected.pending_proposal=null;assert.deepEqual(cleared,expected);
  assert.equal(good(restoreCall,{op:'pending_proposal',state:cleared,now}).status,'idle');
  assert.equal(restoreCall({op:'return_proposal',state:cleared,result,now}).error.code,'missing_reference');
}
const clearedReview=good(restoreCall,{op:'revoke_pending_proposal',state:returned.state,now});
const cancelledApply=envelope(clearedReview,returned.command,2);
assert.equal(restoreCall({op:'dispatch',state:clearedReview,command:cancelledApply,command_json:JSON.stringify(cancelledApply),now}).error.code,'missing_reference');
const reprepared=good(restoreCall,{op:'prepare_proposal',state:clearedReview,week_id:week.range.week_id,slots:[slot],mode:'fill_open',allow_repeats:false,nonce:'fd45feb2-f283-4874-baa7-1dde600fd01d',now});
assert.notEqual(reprepared.request.proposal_id,prepared.request.proposal_id);
assert.equal(reprepared.request.context_digest,prepared.request.context_digest);
assert.equal(restoreCall({op:'return_proposal',state:reprepared.state,result,now}).error.code,'identity_mismatch');
command=envelope(returned.state,returned.command,2);const originalCommandJson=JSON.stringify(command);
const applied=good(call,{op:'dispatch',state:returned.state,command,command_json:originalCommandJson,now});
const revision=applied.state.snapshot.snapshot_revision;
assert.ok(applied.state.receipts[1].accepted_proposal);
// A genuinely fresh module instance has no successful-state audit cache.
call=await boot();state=good(call,{op:'validate_state',state:JSON.parse(JSON.stringify(applied.state)),now});
const acceptedCleared=good(call,{op:'revoke_pending_proposal',state,now});assert.deepEqual(acceptedCleared,state);state=acceptedCleared;
const repeated=good(call,{op:'return_proposal',state,result,now});assert.deepEqual(repeated.state,state);assert.equal(repeated.command,null);assert.equal(repeated.status,'already_applied');
const changed=structuredClone(result);changed.selections[0].recipe=prepared.request.slots[0].choices[1].recipe;
assert.equal(call({op:'return_proposal',state,result:changed,now}).error.code,'replay_conflict');
policy={...state.snapshot.privacy,use_ai_features:false};let offCommand=envelope(state,{kind:'set_privacy',data:{policy,expected_policy_revision:policy.revision}},3);
state=good(call,{op:'dispatch',state,command:offCommand,command_json:JSON.stringify(offCommand),now}).state;
const offRepeated=good(call,{op:'return_proposal',state,result,now});assert.deepEqual(offRepeated.state,state);assert.equal(offRepeated.command,null);assert.equal(offRepeated.status,'already_applied');
const replay=good(call,{op:'dispatch',state,command,command_json:originalCommandJson,now});assert.equal(replay.receipt.disposition,'replayed');assert.deepEqual(replay.state,state);
console.log(JSON.stringify({imports:0,revision_after_apply:revision,restore_cancelled_awaiting_return:true,restore_cancelled_review_ready:true,cancelled_apply:'missing_reference',same_clock_new_nonce_changed_id:true,old_return_after_reprepare:'identity_mismatch',accepted_result_after_fresh_instance:repeated.status,changed_result:'replay_conflict',ai_off_repeat:offRepeated.status,original_command_replay:replay.receipt.disposition,unchanged_state:true,duration_ms:Math.round(performance.now()-start)}));
