// Pure v2 coordination with explicitly synthetic guards/reads/stop evidence.
// No retailer/session, user approval, remote exclusivity or effects are proved.
import test from 'node:test';
import assert from 'node:assert/strict';
import {readFile} from 'node:fs/promises';
import {createHash} from 'node:crypto';
import {observedSinglePackPrice,observedOnePackQuote,BasketProtocolError} from './exact.ts';
import {prepareObservedIntent,emptyObservedLedger,auditObservedLedger,createObservedProtocol,observedContentDigest,
 prepareIntent,emptyLedger,auditLedger,createProtocol,digest} from './protocol.ts';

const copy=value=>structuredClone(value),t=n=>`2026-10-13T18:${String(Math.floor(n/60)).padStart(2,'0')}:${String(n%60).padStart(2,'0')}Z`;
const q=(n,d=1,unit='g',form='dry')=>({amount:{numerator:String(n),denominator:String(d)},unit,form});
const unit={currency:{code:'GBP',minor_exponent:2},minor_units:'125'};
const sku={product_id:'synthetic-oats',variant_id:null},manual={product_id:'synthetic-manual-soap',variant_id:null};
const key={retailer:'synthetic-retailer',account_ref:'synthetic-account',basket_ref:'synthetic-basket'};
const who={worker_ref:'synthetic-retained-reader',generation:'1',uses_ai:true};
const price=()=>({kind:'observed_single_pack',unit_price:copy(unit),observed_at:t(0),retailer_price_guaranteed:false,offers:'excluded',delivery:'unknown',fees:'unknown'});
const rejects=(promise,code)=>assert.rejects(promise,error=>error instanceof BasketProtocolError&&(!code||error.code===code));
const throws=(fn,code)=>assert.throws(fn,error=>error instanceof BasketProtocolError&&(!code||error.code===code));
async function fixture({baselineCount=0,id='synthetic-observed-intent'}={}){
 const binding={plan_digest:'1'.repeat(64),demand_digest:'2'.repeat(64),product_decisions_digest:'3'.repeat(64),price_quote_digest:'4'.repeat(64),context_digest:'5'.repeat(64),privacy_revision:'1'},delivery=await digest({delivery_date:null,slot_ref:null,context_ref:'synthetic-local-context'});
 const lines=[{product:manual,quantity:3,unit_price:null}];if(baselineCount)lines.push({product:sku,quantity:baselineCount,unit_price:copy(unit)});
 const raw={format:'mstrchf-observed-basket-intent',version:2,intent_id:id,key:copy(key),delivery_context_digest:delivery,binding,adapter_ref:'synthetic-increment-one-procedure',baseline:{key:copy(key),delivery_context_digest:delivery,read_id:'synthetic-baseline',read_scope_digest:await digest('synthetic-initial-read'),observed_at:t(1),complete:true,lines,selected_price:{product:sku,unit_price:copy(unit),observed_at:t(0)},evidence_ref:'synthetic-baseline-read'},selections:[{demand_line_id:'synthetic-oats-demand',ingredient_id:'synthetic-dry-oats',product:sku,required:q(135,2),supply:q(1000),match_digest:await digest('synthetic-native-candidate-double'),suitability:'suitable',price:price(),credit_existing_packs:0,new_packs:1}],created_at:t(2),approval_deadline:t(120)};
 const intent=await prepareObservedIntent(raw),flags={current:copy(binding),approval:true,adapter:true,worker:true,read:true,stop:true,generation:'1'};
 const guards={current_binding:()=>copy(flags.current),approved:(ref,value)=>flags.approval&&ref==='synthetic-runtime-operator-review'&&value.digest===intent.digest,
  adapter_qualified:(ref,resource)=>flags.adapter&&ref===raw.adapter_ref&&JSON.stringify(resource)===JSON.stringify(key),
  worker_allowed:(worker)=>flags.worker&&worker.worker_ref===who.worker_ref&&worker.uses_ai===true&&worker.generation===flags.generation,
  accepted_read:(_request,value)=>flags.read&&value.evidence_ref.startsWith('synthetic-'),qualified_stop:fact=>flags.stop&&fact.evidence_ref==='synthetic-visible-stop'};
 return {raw,intent,flags,guards,protocol:createObservedProtocol(guards)};
}
async function started(f){return (await f.protocol.start(emptyObservedLedger(),f.intent,'synthetic-runtime-operator-review',who,t(2))).state;}
function observation(state,request,at,lines=state.jobs[0].last_observation.lines){return {...copy(state.jobs[0].last_observation),read_id:request.read_id,read_scope_digest:request.digest,observed_at:t(at),selected_price:{product:sku,unit_price:copy(unit),observed_at:t(at)},lines:copy(lines),evidence_ref:'synthetic-current-read'};}
async function granted(f){let state=await started(f);state=(await f.protocol.intend(state,f.intent.intent_id,'synthetic-op',sku,t(3))).state;const read=await f.protocol.requestRead(state,f.intent.intent_id,'synthetic-preflight','preflight','synthetic-op',t(4),t(30));const result=await f.protocol.grant(read.state,f.intent.intent_id,'synthetic-op',read.request.read_id,observation(read.state,read.request,5),'synthetic-grant',t(6),t(25));assert.equal(result.effect,null);return result.state;}
async function claimed(f){return f.protocol.claim(await granted(f),f.intent.intent_id,'synthetic-op',t(7));}
function targetLines(state){const lines=copy(state.jobs[0].last_observation.lines),operation=state.jobs[0].operations.at(-1),line=lines.find(line=>line.product.product_id===sku.product_id);if(line){line.quantity=operation.target_quantity;line.unit_price=copy(unit);}else lines.push({product:sku,quantity:operation.target_quantity,unit_price:copy(unit)});return lines;}
async function uncertain(f){const result=await claimed(f);return (await f.protocol.uncertain(result.state,f.intent.intent_id,'synthetic-op','Synthetic lost acknowledgement',t(8))).state;}
async function stopped(f,state){return (await f.protocol.stop(state,f.intent.intent_id,{intent_id:f.intent.intent_id,worker:copy(state.jobs[0].worker),qualification_ref:'synthetic-local-stop-procedure',evidence_ref:'synthetic-visible-stop',stopped_at:t(9),recorded_at:t(10)},t(10))).state;}
async function recovery(f,state,id='synthetic-recovery'){return f.protocol.requestRead(state,f.intent.intent_id,id,'recovery','synthetic-op',t(11),t(40));}

test('observed single-pack arithmetic has exact fractional coverage and unknown delivery/fees without retailer expiry',async()=>{
 const f=await fixture();assert.equal(f.intent.observed_new_pack_spend.minor_units,'125');assert.equal(f.intent.total_checkout_spend,null);
 assert.deepEqual(observedOnePackQuote(q(135,2),q(1000),price(),t(90)).surplus,{numerator:'1865',denominator:'2'});
 assert(Object.isFrozen(f.intent)&&Object.isFrozen(f.intent.selections[0].price.unit_price));
 assert(!JSON.stringify(f.intent).includes('delivery_at'));assert(!JSON.stringify(f.intent).includes('valid_until'));assert.equal(f.intent.selections[0].price.retailer_price_guaranteed,false);
 for(const patch of [{valid_until:t(120)},{delivery_at:t(60)},{offers:[]},{retailer_price_guaranteed:true}])throws(()=>observedSinglePackPrice({...price(),...patch}));
 throws(()=>observedOnePackQuote(q(1001),q(1000),price(),t(2)),'insufficient_quantity');throws(()=>observedOnePackQuote(q(45),q(1000,1,'ml'),price(),t(2)),'unsupported_conversion');
 throws(()=>observedOnePackQuote(q(45),q(1000),{...price(),unit_price:{...unit,minor_units:'1.25'}},t(2)),'noncanonical_value');
});

test('one selection/new pack and zero credit are literal bounds; absent SKU is not represented by a dummy basket row',async()=>{
 const f=await fixture();assert.equal(f.intent.baseline.lines.length,1);assert.equal(f.intent.baseline.lines[0].product.product_id,manual.product_id);assert.equal(f.intent.baseline.selected_price.product.product_id,sku.product_id);
 for(const mutate of [raw=>raw.selections[0].credit_existing_packs=1,raw=>raw.selections[0].new_packs=2,raw=>raw.selections[0].new_packs=0,raw=>raw.selections.push(copy(raw.selections[0])),raw=>raw.selections[0].suitability='unknown',raw=>raw.baseline.lines.push({product:sku,quantity:0,unit_price:copy(unit)}),raw=>raw.baseline.complete=false,raw=>raw.baseline.selected_price.unit_price=null,raw=>raw.baseline.selected_price.product=manual]){const raw=copy(f.raw);mutate(raw);await rejects(prepareObservedIntent(raw));}
 const future=copy(f.raw);future.selections[0].price.observed_at=t(3);await rejects(prepareObservedIntent(future),'invalid_value');
 await rejects(fixture({baselineCount:10000}),'limit_exceeded');
});

test('every runtime guard is required; self-signed intent and ledger pass no imported operator authority',async()=>{
 throws(()=>createObservedProtocol({}),'authority_denied');
 for(const name of ['approval','adapter','worker','read']){const f=await fixture();f.flags[name]=false;await rejects(f.protocol.start(emptyObservedLedger(),f.intent,'synthetic-runtime-operator-review',who,t(2)),'authority_denied');}
 const f=await fixture();await rejects(f.protocol.start(emptyObservedLedger(),f.intent,'model-said-approved',who,t(2)),'authority_denied');const raw=copy(f.intent);raw.observed_new_pack_spend.minor_units='0';await rejects(f.protocol.start(emptyObservedLedger(),raw,'synthetic-runtime-operator-review',who,t(2)),'digest_mismatch');
 const state=await started(f);await auditObservedLedger(JSON.parse(JSON.stringify(state)));f.flags.approval=false;await rejects(f.protocol.intend(state,f.intent.intent_id,'imported-op',sku,t(3)),'authority_denied');
});

test('same-economic fresh timestamps preserve content but both basket and selected price must be freshly read',async()=>{
 const f=await fixture(),state=await granted(f);assert.equal(await observedContentDigest(state.jobs[0].last_observation),f.intent.baseline_digest);assert.notEqual(state.jobs[0].last_observation.selected_price.observed_at,f.intent.baseline.selected_price.observed_at);assert.equal(state.jobs[0].intent.digest,f.intent.digest);
 for(const field of ['observed_at','selected_price']){let pending=await started(f);pending=(await f.protocol.intend(pending,f.intent.intent_id,'stale-read-op',sku,t(3))).state;const read=await f.protocol.requestRead(pending,f.intent.intent_id,'stale-read','preflight','stale-read-op',t(4),t(30)),raw=observation(read.state,read.request,5);if(field==='selected_price')raw.selected_price.observed_at=t(3);else{raw.observed_at=t(3);raw.selected_price.observed_at=t(3);}await rejects(f.protocol.grant(read.state,f.intent.intent_id,'stale-read-op',read.request.read_id,raw,'grant',t(6),t(25)),'read_stale');}
});

test('fresh preflight rejects manual goods, selected price unknown/change, wrong account/context or scope before any effect',async()=>{
 for(const attack of ['manual','unknown','price','account','delivery','scope']){
  const f=await fixture();let state=await started(f);state=(await f.protocol.intend(state,f.intent.intent_id,'preflight-op',sku,t(3))).state;const read=await f.protocol.requestRead(state,f.intent.intent_id,'preflight-read','preflight','preflight-op',t(4),t(30)),raw=observation(read.state,read.request,5);
  if(attack==='manual')raw.lines[0].quantity++;if(attack==='unknown')raw.selected_price.unit_price=null;if(attack==='price')raw.selected_price.unit_price.minor_units='150';if(attack==='account')raw.key.account_ref='another-account';if(attack==='delivery')raw.delivery_context_digest='f'.repeat(64);if(attack==='scope')raw.read_scope_digest='f'.repeat(64);
  await rejects(f.protocol.grant(read.state,f.intent.intent_id,'preflight-op',read.request.read_id,raw,'grant',t(6),t(25)));assert.equal(read.state.jobs[0].operations[0].phase,'intended');
 }
});

test('claim emits only one explicit increment; failed or competing host CAS releases no extra instruction',async()=>{
 const f=await fixture({baselineCount:5}),state=await granted(f),a=await f.protocol.claim(state,f.intent.intent_id,'synthetic-op',t(7)),b=await f.protocol.claim(state,f.intent.intent_id,'synthetic-op',t(7));let saved=state;const released=[];
 const commit=(result,revision,fail=false)=>{if(fail||saved.revision!==revision)return false;saved=result.state;released.push(result.effect);return true;};
 assert.equal(commit(a,state.revision,true),false);assert.equal(released.length,0);assert.equal(commit(a,state.revision),true);assert.equal(commit(b,state.revision),false);assert.equal(released.length,1);
 assert.equal(a.effect.kind,'increment_one');assert.equal(a.effect.packs,1);assert.equal(a.effect.expected_quantity,5);assert.equal(a.effect.target_quantity,6);assert(!JSON.stringify(a.effect).includes('set_quantity'));assert.deepEqual(a.state.jobs[0].intent.baseline.lines.find(line=>line.product.product_id===manual.product_id),f.intent.baseline.lines.find(line=>line.product.product_id===manual.product_id));
 await rejects(f.protocol.claim(saved,f.intent.intent_id,'synthetic-op',t(8)),'scope_violation');await auditObservedLedger(saved);
});

test('current bindings, worker revocation and operational deadlines fence claim without claiming retail price expiry',async()=>{
 for(const change of ['context','worker','grant_expiry','approval_deadline']){const f=await fixture(),state=await granted(f);let now=t(7);if(change==='context')f.flags.current.context_digest='f'.repeat(64);if(change==='worker')f.flags.worker=false;if(change==='grant_expiry')now=t(25);if(change==='approval_deadline')now=t(120);await rejects(f.protocol.claim(state,f.intent.intent_id,'synthetic-op',now));assert.equal(state.jobs[0].operations[0].phase,'granted');}
 const f=await fixture();let state=await started(f);const second=copy(f.raw);second.intent_id='another-intent';second.delivery_context_digest=await digest('other-context');second.baseline.delivery_context_digest=second.delivery_context_digest;const intent=await prepareObservedIntent(second);await rejects(f.protocol.start(state,intent,'synthetic-runtime-operator-review',who,t(3)),'writer_conflict');
});

test('fresh verification and final read preserve every unrelated good and never claim checkout or Booked',async()=>{
 const f=await fixture({baselineCount:2}),claimedState=(await claimed(f)).state,read=await f.protocol.requestRead(claimedState,f.intent.intent_id,'verification','verification','synthetic-op',t(8),t(30));
 const success=await f.protocol.observe(read.state,f.intent.intent_id,read.request.read_id,observation(read.state,read.request,9,targetLines(read.state)),t(10));assert.equal(success.effect,null);assert.equal(success.state.jobs[0].operations[0].phase,'observed_success');
 const final=await f.protocol.requestRead(success.state,f.intent.intent_id,'final','final',null,t(11),t(30)),bad=observation(final.state,final.request,12);bad.lines=bad.lines.filter(line=>line.product.product_id!==manual.product_id);await rejects(f.protocol.observe(final.state,f.intent.intent_id,final.request.read_id,bad,t(13)),'reconciliation_required');
 const result=await f.protocol.observe(final.state,f.intent.intent_id,final.request.read_id,observation(final.state,final.request,12),t(13));assert.equal(result.state.jobs[0].status,'reconciled');assert.equal(result.effect,null);assert.equal(result.state.jobs[0].last_observation.lines.find(line=>line.product.product_id===manual.product_id).quantity,3);assert.equal(result.state.jobs[0].intent.total_checkout_spend,null);assert(!JSON.stringify(result).includes('Booked'));await auditObservedLedger(JSON.parse(JSON.stringify(result.state)));
});

test('old quantity and unknown or changed price after Add stay uncertain and cannot cause an automatic retry',async()=>{
 for(const attack of ['old','unknown','manual']){const f=await fixture(),state=(await claimed(f)).state,read=await f.protocol.requestRead(state,f.intent.intent_id,'uncertain-verification','verification','synthetic-op',t(8),t(30)),raw=observation(read.state,read.request,9,attack==='old'?read.state.jobs[0].last_observation.lines:targetLines(read.state));if(attack==='unknown')raw.selected_price.unit_price=null;if(attack==='manual')raw.lines.find(line=>line.product.product_id===manual.product_id).quantity=9;
  const result=await f.protocol.observe(read.state,f.intent.intent_id,read.request.read_id,raw,t(10));assert.equal(result.effect,null);assert.equal(result.state.jobs[0].status,'quarantined');assert.equal(result.state.jobs[0].operations[0].phase,'uncertain');await rejects(f.protocol.intend(result.state,f.intent.intent_id,'blind-retry',sku,t(11)),'operation_uncertain');await rejects(recovery(f,result.state),'worker_not_stopped');
 }
});

test('cold/lost acknowledgement recovery requires actual stop and later correlated reads; target already present causes no second increment',async()=>{
 const f=await fixture(),claimedState=(await claimed(f)).state,cold=await f.protocol.afterRestart(JSON.parse(JSON.stringify(claimedState)),t(8));assert.equal(cold.effect,null);assert.equal(cold.state.jobs[0].status,'quarantined');await rejects(recovery(f,cold.state),'worker_not_stopped');
 const stop=await stopped(f,cold.state),read=await recovery(f,stop);f.flags.stop=false;await rejects(f.protocol.observe(read.state,f.intent.intent_id,read.request.read_id,observation(read.state,read.request,12,targetLines(read.state)),t(13)),'worker_not_stopped');f.flags.stop=true;f.flags.generation='2';
 const result=await f.protocol.observe(read.state,f.intent.intent_id,read.request.read_id,observation(read.state,read.request,12,targetLines(read.state)),t(13));assert.equal(result.effect,null);assert.equal(result.state.jobs[0].operations[0].phase,'observed_success');await rejects(f.protocol.intend(result.state,f.intent.intent_id,'duplicate-add',sku,t(14)),'already_satisfied');await rejects(f.protocol.observe(result.state,f.intent.intent_id,read.request.read_id,observation(read.state,read.request,12,targetLines(read.state)),t(14)),'replay_conflict');await auditObservedLedger(result.state);
});

test('proven failure permits only a fresh operation/generation; cancellation preserves manually changed goods after stop',async()=>{
 const f=await fixture(),state=await stopped(f,await uncertain(f)),read=await recovery(f,state);f.flags.generation='2';const failure=await f.protocol.observe(read.state,f.intent.intent_id,read.request.read_id,observation(read.state,read.request,12),t(13));assert.equal(failure.effect,null);assert.equal(failure.state.jobs[0].operations[0].phase,'observed_failure');await rejects(f.protocol.claim(failure.state,f.intent.intent_id,'synthetic-op',t(14)),'scope_violation');const retry=await f.protocol.intend(failure.state,f.intent.intent_id,'fresh-operation',sku,t(14));assert.equal(retry.effect,null);assert.equal(retry.state.jobs[0].operations[1].worker.generation,'2');await auditObservedLedger(retry.state);
 const g=await fixture(),cancel=(await g.protocol.cancel(await uncertain(g),g.intent.intent_id,t(9))).state,stop=await stopped(g,cancel),fresh=await recovery(g,stop),actual=observation(fresh.state,fresh.request,12,targetLines(fresh.state));actual.lines.find(line=>line.product.product_id===manual.product_id).quantity=8;
 const retained=await g.protocol.abandonAfterRecovery(fresh.state,g.intent.intent_id,fresh.request.read_id,actual,t(13));assert.equal(retained.effect,null);assert.equal(retained.state.jobs[0].status,'cancelled');assert.equal(retained.state.jobs[0].last_observation.lines.find(line=>line.product.product_id===manual.product_id).quantity,8);await auditObservedLedger(retained.state);
});

test('strict stop chronology/read correlation and forged target/effect fields refuse',async()=>{
 const f=await fixture(),state=await stopped(f,await uncertain(f)),equal=await f.protocol.requestRead(state,f.intent.intent_id,'equal-stop','recovery','synthetic-op',t(10),t(30));await rejects(f.protocol.observe(equal.state,f.intent.intent_id,equal.request.read_id,observation(equal.state,equal.request,10),t(11)),'reconciliation_required');
 const read=await recovery(f,state),raw=observation(read.state,read.request,12);raw.read_scope_digest='f'.repeat(64);await rejects(f.protocol.observe(read.state,f.intent.intent_id,read.request.read_id,raw,t(13)),'identity_mismatch');
 const grantedState=await granted(f),target=copy(grantedState);target.jobs[0].operations[0].target_quantity=2;await rejects(auditObservedLedger(target),'scope_violation');const setter=copy(grantedState);setter.jobs[0].operations[0].kind='set_quantity';await rejects(auditObservedLedger(setter),'invalid_shape');await rejects(auditLedger(grantedState),'unsupported_version');await rejects(auditObservedLedger(emptyLedger()),'unsupported_version');
});

test('literal frozen v1 intent/data pins and original absolute effect remain unchanged beside v2',async()=>{
 // The literal pins were derived from the unchanged v30 shared v1 projection,
 // not from v2 or an altered expectation. Legacy is exercised independently.
 const source=await readFile(new URL('../../sites/mstrchf-9000/shared/browser-protocols/basket/protocol.ts',import.meta.url),'utf8');
 const prefix=(await readFile(new URL('./protocol.ts',import.meta.url),'utf8')).split('// Observational v2 is deliberately separate:')[0];
 assert.equal(createHash('sha256').update(prefix.replaceAll('\r\n','\n').trimEnd()).digest('hex'),createHash('sha256').update(source.split('// Observational v2 is deliberately separate:')[0].replaceAll('\r\n','\n').trimEnd()).digest('hex'));
 const legacyKey={retailer:'synthetic-legacy-retailer',account_ref:'synthetic-account',basket_ref:'synthetic-basket'},product={product_id:'legacy-oats',variant_id:null},terms={unit_price:copy(unit),offers:[],loyalty_eligible:false,delivery_at:t(90),observed_at:t(0),valid_until:t(120)},delivery=await digest('synthetic-legacy-delivery'),binding={plan_digest:'1'.repeat(64),demand_digest:'2'.repeat(64),product_decisions_digest:'3'.repeat(64),price_quote_digest:'4'.repeat(64),context_digest:'5'.repeat(64),privacy_revision:'1'};
 const raw={intent_id:'synthetic-v1-pin',key:legacyKey,delivery_digest:delivery,binding,adapter_ref:'synthetic-v1-adapter',baseline:{key:legacyKey,delivery_digest:delivery,read_id:'synthetic-v1-baseline',read_scope_digest:await digest('synthetic-v1-scope'),observed_at:t(0),lines:[{product,quantity:2,price_quote_digest:await digest(terms)}],evidence_ref:'synthetic-v1-read'},selections:[{demand_line_id:'synthetic-demand',ingredient_id:'synthetic-ingredient',product,required:q(45),supply:q(1000),match_digest:await digest('synthetic-v1-match'),suitability:'suitable',terms,credit_existing_packs:0,new_packs:1}],created_at:t(1),valid_until:t(120)};
 const intent=await prepareIntent(raw);assert.equal(intent.digest,'15d0d5145629419a7324d51be65b17a595b17ab77c9c443a7cff706c3f2e7f13');assert.equal(await digest(JSON.stringify(intent)),'5705cf38d86beab8b0c649cb9c15589386dc5c0406ce60fe16985bb7306a4418');
 const protocol=createProtocol({current_binding:()=>binding,approved:()=>true,adapter_qualified:()=>true,worker_allowed:()=>true,accepted_read:()=>true,qualified_stop:()=>false});let state=(await protocol.start(emptyLedger(),intent,'synthetic-v1-review',who,t(2))).state;state=(await protocol.intend(state,intent.intent_id,'v1-op',product,t(3))).state;const read=await protocol.requestRead(state,intent.intent_id,'v1-preflight','preflight','v1-op',t(4),t(30));const observed={...copy(intent.baseline),read_id:read.request.read_id,read_scope_digest:read.request.digest,observed_at:t(5)};state=(await protocol.grant(read.state,intent.intent_id,'v1-op',read.request.read_id,observed,'v1-grant',t(6),t(25))).state;const result=await protocol.claim(state,intent.intent_id,'v1-op',t(7));assert.equal(result.effect.kind,'set_quantity');assert.equal(result.effect.target_quantity,3);await auditLedger(result.state);
});
