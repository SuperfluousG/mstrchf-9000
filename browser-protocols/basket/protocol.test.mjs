// Synthetic host-guard observations exercise coordination only; no retailer
// state, adapter qualification, credentials or account effect is represented.
import test from 'node:test';
import assert from 'node:assert/strict';
import {minimumPacks,packQuote,quotePacks,formatMoney,addMoney,BasketProtocolError} from './exact.ts';
import {prepareIntent,createProtocol,emptyLedger,digest,auditLedger,writerKey} from './protocol.ts';
const copy=v=>structuredClone(v);
const t=n=>`2026-10-12T20:${String(Math.floor(n/60)).padStart(2,'0')}:${String(n%60).padStart(2,'0')}Z`;
const q=(n,form='drained',d='1')=>({amount:{numerator:String(n),denominator:d},unit:'g',form});
const money=(n,code='USD',exponent=2)=>({currency:{code,minor_exponent:exponent},minor_units:String(n)});
const product={product_id:'beans',variant_id:'drained-240g'};
const other={product_id:'manual-soap',variant_id:null};
const key={retailer:'synthetic-retailer',account_ref:'opaque-account',basket_ref:'basket-a'};
const who={worker_ref:'synthetic-worker-1',generation:'1',uses_ai:true};
const rejects=(promise,code)=>assert.rejects(promise,e=>e instanceof BasketProtocolError&&e.code===code);
const throws=(fn,code)=>assert.throws(fn,e=>e instanceof BasketProtocolError&&e.code===code);
const terms=()=>({unit_price:money(123),offers:[],loyalty_eligible:false,delivery_at:t(300),observed_at:t(0),valid_until:t(600)});
async function fixture({baselineCount=5,credit=0,newPacks=2,keyOverride=key,id='intent-1'}={}){
 const prices=terms(),priceHash=await digest(prices),delivery=await digest('delivery-a'),binding={plan_digest:await digest('plan-a'),demand_digest:await digest('demand-a'),product_decisions_digest:await digest('decision-a'),price_quote_digest:await digest('prices-a'),context_digest:await digest('context-a'),privacy_revision:'1'};
 const lines=[{product:other,quantity:3,price_quote_digest:null}];if(baselineCount)lines.push({product,quantity:baselineCount,price_quote_digest:priceHash});
 const raw={intent_id:id,key:copy(keyOverride),delivery_digest:delivery,binding,adapter_ref:'synthetic-qualified-adapter',baseline:{key:copy(keyOverride),delivery_digest:delivery,read_id:'baseline-'+id,read_scope_digest:await digest('baseline-scope'),observed_at:t(0),lines,evidence_ref:'synthetic-baseline'},selections:[{demand_line_id:'beans-demand',ingredient_id:'beans-ingredient',product,required:q(480),supply:q(240),match_digest:await digest('core-matching-witness'),suitability:'suitable',terms:prices,credit_existing_packs:credit,new_packs:newPacks}],created_at:t(0),valid_until:t(600)};
 const flags={current:copy(binding),approval:true,adapter:true,worker:true,read:true,stop:true};
 const guards={current_binding:()=>copy(flags.current),approved:(ref,intent)=>flags.approval&&ref==='explicit-owner-approval'&&intent.digest.length===64,adapter_qualified:()=>flags.adapter,worker_allowed:()=>flags.worker,accepted_read:(_request,observation)=>flags.read&&observation.evidence_ref.startsWith('synthetic-'),qualified_stop:(fact)=>flags.stop&&fact.evidence_ref==='synthetic-stop'};
 return {raw,intent:await prepareIntent(raw),flags,guards,protocol:createProtocol(guards)};
}
async function started(f){return (await f.protocol.start(emptyLedger(),f.intent,'explicit-owner-approval',who,t(1))).state;}
const obs=(state,request,at,lines=state.jobs[0].last_observation.lines)=>({...copy(state.jobs[0].last_observation),read_id:request.read_id,read_scope_digest:request.digest,observed_at:t(at),lines:copy(lines),evidence_ref:'synthetic-read-'+request.read_id});
async function granted(f){let state=await started(f);let result=await f.protocol.intend(state,f.intent.intent_id,'op-1',product,t(1));assert.equal(result.effect,null);state=result.state;const read=await f.protocol.requestRead(state,f.intent.intent_id,'preflight-1','preflight','op-1',t(2),t(30));state=read.state;result=await f.protocol.grant(state,f.intent.intent_id,'op-1',read.request.read_id,obs(state,read.request,3),'grant-1',t(4),t(25));assert.equal(result.effect,null);return result.state;}
async function claimed(f){const state=await granted(f),result=await f.protocol.claim(state,f.intent.intent_id,'op-1',t(5));assert.equal(result.effect.kind,'set_quantity');return result;}
function targetLines(state){const lines=copy(state.jobs[0].last_observation.lines),op=state.jobs[0].operations.at(-1),line=lines.find(l=>JSON.stringify(l.product)===JSON.stringify(op.product));if(line){line.quantity=op.target_quantity;line.price_quote_digest=op.target_price_digest;}else lines.push({product:op.product,quantity:op.target_quantity,price_quote_digest:op.target_price_digest});return lines;}
async function quarantined(f){const result=await claimed(f);return (await f.protocol.uncertain(result.state,f.intent.intent_id,'op-1','Synthetic lost acknowledgement',t(6))).state;}
async function stopped(f,state){return (await f.protocol.stop(state,f.intent.intent_id,{intent_id:f.intent.intent_id,worker:copy(state.jobs[0].worker),qualification_ref:'synthetic-stop-qualification',evidence_ref:'synthetic-stop',stopped_at:t(7),recorded_at:t(8)},t(8))).state;}
async function recovery(f,state,readId='recover-1'){return f.protocol.requestRead(state,f.intent.intent_id,readId,'recovery','op-1',t(9),t(40));}

test('exact drained pack arithmetic refuses gross/cooked substitution and computes fractional ceilings',()=>{
 assert.equal(minimumPacks(q(480),q(240)),2);assert.equal(minimumPacks(q(481),q(240)),3);assert.equal(minimumPacks(q('3','drained','2'),q('2','drained','3')),3);
 throws(()=>minimumPacks(q(480),q(400,'whole')),'unsupported_conversion');throws(()=>minimumPacks(q(480),q(240,'cooked')),'unsupported_conversion');throws(()=>minimumPacks(q(480),q(0)),'noncanonical_value');
 const quote=packQuote(q(481),q(240),terms(),0,3,t(1));assert.deepEqual(quote.surplus,{numerator:'239',denominator:'1'});throws(()=>packQuote(q(480),q(240),terms(),0,1,t(1)),'insufficient_quantity');
});
test('currency/minor units remain exact including 0/3 decimal places and values above Number precision',()=>{
 assert.equal(formatMoney(money('9007199254740993')),'USD 90071992547409.93');assert.equal(formatMoney(money(123,'JPY',0)),'JPY 123');assert.equal(formatMoney(money(123,'TND',3)),'TND 0.123');
 assert.equal(addMoney([money('9007199254740993'),money(7)],money(0).currency).minor_units,'9007199254741000');throws(()=>addMoney([money(1,'USD',2),money(1,'USD',3)],money(0).currency),'currency_mismatch');throws(()=>quotePacks({...terms(),unit_price:money('18446744073709551615')},2,t(1)),'arithmetic_overflow');
});
test('only exact eligible repeated per-product offer bundles lower price; no extra pack is invented',()=>{
 const p=terms();p.loyalty_eligible=true;p.offers=[{offer_id:'two-for-150',packs:2,total:money(150),valid_from:t(100),valid_until:t(400),loyalty_required:true,delivery_applicable:'yes'}];
 assert.equal(quotePacks(p,1,t(1)).minor_units,'123');assert.equal(quotePacks(p,3,t(1)).minor_units,'273');assert.equal(quotePacks(p,4,t(1)).minor_units,'300');
 for(const update of [{loyalty_eligible:false},{delivery_at:t(401)}])assert.equal(quotePacks({...p,...update},2,t(1)).minor_units,'246');for(const applicable of ['unknown','no'])assert.equal(quotePacks({...p,offers:[{...p.offers[0],delivery_applicable:applicable}]},2,t(1)).minor_units,'246');
 throws(()=>quotePacks(p,2,t(601)),'price_stale');
});
test('reviewable intent is frozen, exactly hashed, and names only new-pack spend',async()=>{
 const f=await fixture();assert.ok(Object.isFrozen(f.intent.selections[0].terms));assert.equal(f.intent.purchase_quote.minor_units,'246');assert.equal(f.intent.total_checkout_spend,null);
 const changed=copy(f.intent);changed.selections[0].new_packs=3;await rejects(f.protocol.start(emptyLedger(),changed,'explicit-owner-approval',who,t(1)),'digest_mismatch');await rejects(prepareIntent({...f.raw,unexpected_actor:'owner'}),'invalid_shape');
 const unsafe=copy(f.raw);unsafe.selections[0].suitability='unknown';await rejects(prepareIntent(unsafe),'product_unresolved');
});
test('preserve all baseline goods and never infer stock credit from basket quantity',async()=>{
 const f=await fixture(),result=await claimed(f);assert.equal(result.effect.expected_quantity,5);assert.equal(result.effect.target_quantity,7);assert.deepEqual(result.state.jobs[0].intent.baseline.lines.find(l=>l.product.product_id==='manual-soap'),{product:other,quantity:3,price_quote_digest:null});
 const credited=await fixture({credit:2,newPacks:0});assert.equal(credited.intent.purchase_quote.minor_units,'0');await rejects(credited.protocol.intend(await started(credited),credited.intent.intent_id,'op-zero',product,t(2)),'already_satisfied');
 const invalid=copy(credited.raw);invalid.selections[0].credit_existing_packs=6;await rejects(prepareIntent(invalid),'invalid_value');
});
test('self-signed checksums cannot replace owner approval, qualified adapter, read proof or worker permission',async()=>{
 for(const gate of ['approval','adapter','worker','read']){const f=await fixture();f.flags[gate]=false;await rejects(f.protocol.start(emptyLedger(),f.intent,'explicit-owner-approval',who,t(1)),'authority_denied');}
 throws(()=>createProtocol({}),'authority_denied');const f=await fixture();await rejects(f.protocol.start(emptyLedger(),f.intent,'model-said-approved',who,t(1)),'authority_denied');
});
test('writer key excludes delivery; same account/basket cannot acquire a second writer',async()=>{
 const f=await fixture(),state=await started(f),otherIntent=copy(f.raw);otherIntent.intent_id='intent-2';otherIntent.delivery_digest=await digest('other-delivery');otherIntent.baseline.delivery_digest=otherIntent.delivery_digest;otherIntent.baseline.read_id='baseline-2';const second=await prepareIntent(otherIntent);
 assert.equal(writerKey(second.key),writerKey(f.intent.key));await rejects(f.protocol.start(state,second,'explicit-owner-approval',{...who,worker_ref:'second-worker'},t(1)),'writer_conflict');
 const alternate=await fixture({id:'intent-other-basket',keyOverride:{...key,basket_ref:'basket-b'}});assert.equal((await alternate.protocol.start(state,alternate.intent,'explicit-owner-approval',{...who,worker_ref:'worker-b'},t(1))).state.jobs.length,2);
});
test('grant is inert; once-only claimed effect requires host CAS commit and duplicate claim refuses',async()=>{
 const f=await fixture(),state=await granted(f),[a,b]=await Promise.all([f.protocol.claim(state,f.intent.intent_id,'op-1',t(5)),f.protocol.claim(state,f.intent.intent_id,'op-1',t(5))]);let saved=state;const effects=[];
 const commit=(result,baseRevision,simulateFailure=false)=>{if(simulateFailure||saved.revision!==baseRevision)return false;saved=result.state;if(result.effect)effects.push(result.effect);return true;};
 assert.equal(commit(a,state.revision,true),false);assert.equal(effects.length,0);assert.equal(commit(a,state.revision),true);assert.equal(commit(b,state.revision),false);assert.equal(effects.length,1);await rejects(f.protocol.claim(saved,f.intent.intent_id,'op-1',t(6)),'scope_violation');await auditLedger(saved);
 assert.deepEqual(Object.keys(effects[0]).sort(),['kind','intent_id','intent_digest','operation_id','key','product','expected_quantity','target_quantity','baseline_digest','price_quote_digest','grant_token','worker','expires_at'].sort());
});
test('stale plan or revoked AI permission refuses a previously granted claim',async()=>{
 const f=await fixture(),state=await granted(f);f.flags.current.plan_digest=await digest('edited-plan');await rejects(f.protocol.claim(state,f.intent.intent_id,'op-1',t(5)),'context_stale');f.flags.current=copy(f.intent.binding);f.flags.worker=false;await rejects(f.protocol.claim(state,f.intent.intent_id,'op-1',t(5)),'authority_denied');
 const suspended=await f.protocol.suspend(state,f.intent.intent_id,'Owner context changed',t(5));assert.equal(suspended.effect,null);assert.equal(suspended.state.jobs[0].status,'quarantined');
});
test('preflight refuses manual baseline or selected price change and never emits an effect',async()=>{
 const f=await fixture();let state=await started(f);state=(await f.protocol.intend(state,f.intent.intent_id,'op-1',product,t(1))).state;const read=await f.protocol.requestRead(state,f.intent.intent_id,'preflight-a','preflight','op-1',t(2),t(30));const observed=obs(read.state,read.request,3);observed.lines.find(l=>l.product.product_id==='manual-soap').quantity=4;
 await rejects(f.protocol.grant(read.state,f.intent.intent_id,'op-1',read.request.read_id,observed,'grant-a',t(4),t(25)),'manual_conflict');
 const bad=copy(f.raw);bad.baseline.lines.find(l=>l.product.product_id==='beans').price_quote_digest=await digest('old-price');const priceIntent=await prepareIntent(bad);let priceState=(await f.protocol.start(emptyLedger(),priceIntent,'explicit-owner-approval',who,t(1))).state;priceState=(await f.protocol.intend(priceState,priceIntent.intent_id,'op-price',product,t(1))).state;const priceRead=await f.protocol.requestRead(priceState,priceIntent.intent_id,'price-read','preflight','op-price',t(2),t(30));await rejects(f.protocol.grant(priceRead.state,priceIntent.intent_id,'op-price',priceRead.request.read_id,obs(priceRead.state,priceRead.request,3),'price-grant',t(4),t(25)),'price_changed');
});
test('cold granted or claimed journal quarantines; timeout/cancellation cannot substitute for stop',async()=>{
 const f=await fixture();for(const state of [await granted(f),(await claimed(f)).state]){const cold=JSON.parse(JSON.stringify(state)),result=await f.protocol.afterRestart(cold,t(6));assert.equal(result.effect,null);assert.equal(result.state.jobs[0].status,'quarantined');await rejects(f.protocol.intend(result.state,f.intent.intent_id,'op-retry',product,t(7)),'operation_uncertain');await rejects(recovery(f,result.state),'worker_not_stopped');}
 const cancelled=await f.protocol.cancel((await claimed(f)).state,f.intent.intent_id,t(6));assert.equal(cancelled.state.jobs[0].status,'quarantined');assert.equal(cancelled.effect,null);await rejects(recovery(f,cancelled.state),'worker_not_stopped');
});
test('stop requires exact intent and worker generation; stored stop is requalified after cold restart',async()=>{
 const f=await fixture(),state=await quarantined(f),good={intent_id:f.intent.intent_id,worker:who,qualification_ref:'synthetic-stop-qualification',evidence_ref:'synthetic-stop',stopped_at:t(7),recorded_at:t(8)};
 for(const bad of [{...good,intent_id:'previous-intent'},{...good,worker:{...who,generation:'2'}},{...good,stopped_at:t(4)}])await rejects(f.protocol.stop(state,f.intent.intent_id,bad,t(8)),'worker_not_stopped');
 f.flags.stop=false;await rejects(f.protocol.stop(state,f.intent.intent_id,good,t(8)),'worker_not_stopped');f.flags.stop=true;const stoppedState=await stopped(f,state),read=await recovery(f,stoppedState);f.flags.stop=false;await rejects(f.protocol.observe(JSON.parse(JSON.stringify(read.state)),f.intent.intent_id,read.request.read_id,obs(read.state,read.request,10),t(11)),'worker_not_stopped');
});
test('recovery read must be strictly after qualified stop and correctly correlated',async()=>{
 const f=await fixture(),state=await stopped(f,await quarantined(f)),read=await f.protocol.requestRead(state,f.intent.intent_id,'recovery-equal','recovery','op-1',t(8),t(40));await rejects(f.protocol.observe(read.state,f.intent.intent_id,read.request.read_id,obs(read.state,read.request,8),t(11)),'reconciliation_required');
 const fresh=await recovery(f,state),foreign=obs(fresh.state,fresh.request,10);foreign.key.basket_ref='other-basket';await rejects(f.protocol.observe(fresh.state,f.intent.intent_id,fresh.request.read_id,foreign,t(11)),'identity_mismatch');const replay=obs(fresh.state,fresh.request,10);replay.read_scope_digest=await digest('other-scope');await rejects(f.protocol.observe(fresh.state,f.intent.intent_id,fresh.request.read_id,replay,t(11)),'identity_mismatch');
});
test('lost-ack recovery at target is observed success with no repeated effect; duplicate read refuses',async()=>{
 const f=await fixture(),state=await stopped(f,await quarantined(f)),read=await recovery(f,state),observed=obs(read.state,read.request,10,targetLines(read.state)),result=await f.protocol.observe(read.state,f.intent.intent_id,read.request.read_id,observed,t(11));assert.equal(result.effect,null);assert.equal(result.state.jobs[0].operations[0].phase,'observed_success');assert.equal(result.state.jobs[0].worker.generation,'2');await rejects(f.protocol.intend(result.state,f.intent.intent_id,'op-duplicate',product,t(12)),'already_satisfied');await rejects(f.protocol.observe(result.state,f.intent.intent_id,read.request.read_id,observed,t(12)),'replay_conflict');await auditLedger(result.state);
});
test('unchanged basket after proven stop permits a new operation and worker generation, never replays old grant',async()=>{
 const f=await fixture({baselineCount:0}),state=await stopped(f,await quarantined(f)),read=await recovery(f,state),result=await f.protocol.observe(read.state,f.intent.intent_id,read.request.read_id,obs(read.state,read.request,10),t(11));assert.equal(result.effect,null);assert.equal(result.state.jobs[0].operations[0].phase,'observed_failure');assert.equal(result.state.jobs[0].status,'active');
 await rejects(f.protocol.claim(result.state,f.intent.intent_id,'op-1',t(12)),'scope_violation');const retry=await f.protocol.intend(result.state,f.intent.intent_id,'op-new',product,t(12));assert.equal(retry.state.jobs[0].operations[1].worker.generation,'2');await auditLedger(retry.state);
});
test('old quantity while worker may still execute quarantines instead of proving failure',async()=>{
 const f=await fixture(),claimedState=(await claimed(f)).state,read=await f.protocol.requestRead(claimedState,f.intent.intent_id,'verification-old','verification','op-1',t(6),t(30)),result=await f.protocol.observe(read.state,f.intent.intent_id,read.request.read_id,obs(read.state,read.request,7),t(8));assert.equal(result.effect,null);assert.equal(result.state.jobs[0].status,'quarantined');assert.equal(result.state.jobs[0].operations[0].phase,'uncertain');await auditLedger(result.state);
});
test('manual changes during recovery stay quarantined; cancellation after stop preserves changed goods',async()=>{
 const f=await fixture(),cancelled=(await f.protocol.cancel(await quarantined(f),f.intent.intent_id,t(6))).state,stopState=await stopped(f,cancelled),read=await recovery(f,stopState),observed=obs(read.state,read.request,10,targetLines(read.state));observed.lines.find(l=>l.product.product_id==='manual-soap').quantity=8;
 const unresolved=await f.protocol.observe(read.state,f.intent.intent_id,read.request.read_id,observed,t(11));assert.equal(unresolved.state.jobs[0].status,'quarantined');assert.equal(unresolved.effect,null);
 const fresh=await f.protocol.requestRead(unresolved.state,f.intent.intent_id,'cancel-recovery','recovery','op-1',t(12),t(40)),abandoned=await f.protocol.abandonAfterRecovery(fresh.state,f.intent.intent_id,fresh.request.read_id,obs(fresh.state,fresh.request,13,observed.lines),t(14));assert.equal(abandoned.state.jobs[0].status,'cancelled');assert.equal(abandoned.effect,null);assert.equal(abandoned.state.jobs[0].last_observation.lines.find(l=>l.product.product_id==='manual-soap').quantity,8);assert.match(abandoned.state.jobs[0].issue,/no goods removed/);await auditLedger(abandoned.state);
});
test('AI revocation after uncertain effect permits read-only recovery but no new authorised effect',async()=>{
 const f=await fixture(),read=await recovery(f,await stopped(f,await quarantined(f)));f.flags.worker=false;const result=await f.protocol.observe(read.state,f.intent.intent_id,read.request.read_id,obs(read.state,read.request,10),t(11));assert.equal(result.effect,null);assert.equal(result.state.jobs[0].status,'review_required');await rejects(f.protocol.intend(result.state,f.intent.intent_id,'op-after-revoke',product,t(12)),'authority_denied');
});
test('complete fresh final reconciliation checks all targets and unrelated goods; never means Booked',async()=>{
 const f=await fixture(),claimedState=(await claimed(f)).state,verification=await f.protocol.requestRead(claimedState,f.intent.intent_id,'verify-success','verification','op-1',t(6),t(30)),success=await f.protocol.observe(verification.state,f.intent.intent_id,verification.request.read_id,obs(verification.state,verification.request,7,targetLines(verification.state)),t(8));
 const final=await f.protocol.requestRead(success.state,f.intent.intent_id,'final-basket','final',null,t(9),t(30)),bad=obs(final.state,final.request,10);bad.lines=bad.lines.filter(l=>l.product.product_id!=='manual-soap');await rejects(f.protocol.observe(final.state,f.intent.intent_id,final.request.read_id,bad,t(11)),'reconciliation_required');
 const result=await f.protocol.observe(final.state,f.intent.intent_id,final.request.read_id,obs(final.state,final.request,10),t(11));assert.equal(result.state.jobs[0].status,'reconciled');assert.equal(result.effect,null);assert.equal(JSON.stringify(result).includes('Booked'),false);await auditLedger(JSON.parse(JSON.stringify(result.state)));
});
test('a final basket at unchanged protected baseline cannot satisfy unpurchased demand',async()=>{
 const f=await fixture(),state=await started(f),final=await f.protocol.requestRead(state,f.intent.intent_id,'premature-final','final',null,t(2),t(30));await rejects(f.protocol.observe(final.state,f.intent.intent_id,final.request.read_id,obs(final.state,final.request,3),t(4)),'insufficient_quantity');
});
test('wire bounds, exact keys, invalid revisions, unknown read scopes and forged target tampering refuse',async()=>{
 const f=await fixture(),raw=copy(f.raw);raw.selections.push(copy(raw.selections[0]));await rejects(prepareIntent(raw),'duplicate_key');const tooMany=copy(f.raw);tooMany.baseline.lines=Array.from({length:513},(_,i)=>({product:{product_id:'p'+i,variant_id:null},quantity:1,price_quote_digest:null}));await rejects(prepareIntent(tooMany),'limit_exceeded');
 const state=await granted(f),tampered=copy(state);tampered.jobs[0].operations[0].target_quantity=1;await rejects(auditLedger(tampered),'scope_violation');const forged=copy(state);forged.jobs[0].operations[0].actor='owner';await rejects(auditLedger(forged),'invalid_shape');await rejects(f.protocol.requestRead(state,f.intent.intent_id,'unknown-purpose','checkout',null,t(5),t(20)),'scope_violation');
 const overflow=copy(emptyLedger());overflow.revision='18446744073709551615';await rejects(f.protocol.start(overflow,f.intent,'explicit-owner-approval',who,t(1)),'arithmetic_overflow');const negative=copy(f.raw);negative.selections[0].new_packs=-0;await rejects(prepareIntent(negative),'limit_exceeded');
});
