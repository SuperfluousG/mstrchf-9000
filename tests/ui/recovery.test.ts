import { createHash } from 'node:crypto';
import { readFileSync } from 'node:fs';
import { describe, it, expect, vi } from 'vitest';
import type { DomainError, AppCommand } from '../../app/generated/contracts';
import { PlannerSession, newCommandId } from '../../app/runtime/commandRecovery';
import { createPlannerClient } from '../../app/runtime/tauriClient';
import { snapshot } from '../fixtures/ui/snapshot';

const privacy: AppCommand = { kind: 'set_privacy', data: { policy: snapshot().privacy, expected_policy_revision: '1' } };
const error = (code: DomainError['code']): DomainError => ({code, issues: [{code,path:'/expected_snapshot_revision',entity_id:null}], retry: 'never', truncated: false});
function ack(_name:string,args?:Record<string,unknown>) {
  const text=args!.commandJson as string; const command=JSON.parse(text);
  return {format_version:1,disposition:'replayed',receipt_profile:'B1Draft005',request_encoding:'current_json',
    request_digest:createHash('sha256').update(text).digest('hex'),command_id:command.command_id,
    command_kind:command.command.kind,expected_snapshot_revision:command.expected_snapshot_revision,
    household_id:'hh_00000000000000000000000000000001',receipt_snapshot_revision:(BigInt(command.expected_snapshot_revision)+1n).toString(),
    command_digest:'a'.repeat(64),response_digest:'b'.repeat(64)};
}
function harness() {
  const invoke = vi.fn();
  const session = new PlannerSession(createPlannerClient(invoke));
  return { invoke, session };
}
describe('exact live-session recovery (transport doubles)', () => {
  it('clears a resolved read failure without retaining a restart warning',async()=>{
    const {invoke,session}=harness(); invoke.mockRejectedValueOnce('read unavailable');
    await session.refresh(); expect(session.state.readError).toContain('cannot currently be read');
    invoke.mockResolvedValueOnce(snapshot()); await session.refresh();
    expect(session.state.readError).toBeNull(); expect(session.state.message).toBe('');
    expect(session.state.read).toBe('ready'); expect(session.canMutate).toBe(true);
  });
  it('preserves unknown action instructions and exact identity across failed then successful reads',async()=>{
    const {invoke,session}=harness(); invoke.mockResolvedValueOnce(snapshot()); await session.refresh();
    invoke.mockRejectedValueOnce('IPC lost'); await session.submit(privacy);
    const pending=session.state.pending; const message=session.state.message; const sent=invoke.mock.calls[1][1];
    invoke.mockRejectedValueOnce('read unavailable'); await session.refresh();
    expect(session.state.message).toBe(message); expect(session.state.readError).not.toBeNull();
    expect(session.state.pending).toBe(pending); expect(session.canMutate).toBe(false);
    invoke.mockResolvedValueOnce(snapshot('2')); await session.refresh();
    expect(session.state.readError).toBeNull(); expect(session.state.message).toBe(message);
    expect(session.state.pending).toBe(pending); expect(session.state.disposition).toBe('unknown');
    expect(session.canMutate).toBe(false); await expect(session.submit(privacy)).rejects.toThrow();
    invoke.mockImplementationOnce(ack).mockResolvedValueOnce(snapshot('3')); await session.recover();
    expect(invoke.mock.calls[4][1]).toEqual(sent); expect(session.canMutate).toBe(true);
  });
  it('preserves refusal instructions until explicit review after a recovered read',async()=>{
    const {invoke,session}=harness(); invoke.mockResolvedValueOnce(snapshot()); await session.refresh();
    invoke.mockRejectedValueOnce(error('context_stale')).mockRejectedValueOnce('read unavailable');
    await session.submit(privacy);
    expect(session.state.message).toContain('The action was refused');
    const message=session.state.message; expect(session.state.readError).not.toBeNull();
    session.reviewRefusal(); expect(session.state.disposition).toBe('refused'); expect(session.canMutate).toBe(false);
    invoke.mockResolvedValueOnce(snapshot('2')); await session.refresh();
    expect(session.state.readError).toBeNull(); expect(session.state.message).toBe(message);
    expect(session.state.pending).toBeNull(); expect(session.state.disposition).toBe('refused');
    expect(session.canMutate).toBe(false); session.reviewRefusal(); expect(session.canMutate).toBe(true);
  });
  it('keeps acknowledged action recovery required after a later manual read succeeds',async()=>{
    const {invoke,session}=harness(); invoke.mockResolvedValueOnce(snapshot()); await session.refresh();
    invoke.mockImplementationOnce(ack).mockRejectedValueOnce('read unavailable'); await session.submit(privacy);
    const pending=session.state.pending; const message=session.state.message;
    expect(message).toContain('follow-up verification failed');
    invoke.mockResolvedValueOnce(snapshot('3')); await session.refresh();
    expect(session.state.readError).toBeNull(); expect(session.state.message).toBe(message);
    expect(session.state.pending).toBe(pending); expect(session.state.disposition).toBe('unknown');
    expect(session.canMutate).toBe(false);
  });
  it('does not clear a newer read failure when an older read finishes successfully',async()=>{
    const {invoke,session}=harness(); let old!: (v:unknown)=>void;
    invoke.mockImplementationOnce(()=>new Promise(r=>{old=r;})).mockRejectedValueOnce('newer read failed');
    const first=session.refresh(); await session.refresh(); old(snapshot()); await first;
    expect(session.state.read).toBe('failed'); expect(session.state.readError).toContain('cannot currently be read');
    expect(session.canMutate).toBe(false); expect(session.state.snapshot).toBeNull();
  });
  it('generates nonzero native-format correlation ids', () => {
    expect(newCommandId()).toMatch(/^cm_(?!0{32}$)[0-9a-f]{32}$/);
  });
  it('discards stale queries using exact revisions above JS integer precision', async () => {
    const {invoke,session}=harness(); let old!: (v: unknown)=>void;
    invoke.mockImplementationOnce(()=>new Promise(r=>{old=r;})).mockResolvedValueOnce(snapshot('9007199254740993'));
    const first=session.refresh(); await session.refresh(); old(snapshot('9007199254740992')); await first;
    expect(session.state.snapshot?.snapshot_revision).toBe('9007199254740993');
  });
  it('rejects inconsistent equal revision state and blocks mutation', async () => {
    const {invoke,session}=harness(); invoke.mockResolvedValueOnce(snapshot()); await session.refresh();
    const inconsistent=snapshot(); inconsistent.privacy.use_ai_features=true; invoke.mockResolvedValueOnce(inconsistent);
    await session.refresh(); expect(session.state.read).toBe('failed'); expect(session.canMutate).toBe(false);
    expect(session.state.snapshot?.privacy.use_ai_features).toBe(false);
  });
  it.each(['storage_failure','malformed_json','replay_conflict','operation_uncertain'] as const)('retains exact bytes on unclassified %s even retry never',async code=>{
    const {invoke,session}=harness(); invoke.mockResolvedValueOnce(snapshot()); await session.refresh();
    invoke.mockRejectedValueOnce(error(code)); await session.submit(privacy);
    const sent=invoke.mock.calls[1][1]; expect(session.state.disposition).toBe('unknown');
    expect(session.canMutate).toBe(false); await expect(session.submit(privacy)).rejects.toThrow();
    invoke.mockImplementationOnce(ack).mockResolvedValueOnce(snapshot('3'));
    await session.recover(); expect(invoke.mock.calls[2][1]).toEqual(sent);
    expect(session.state.snapshot?.snapshot_revision).toBe('3'); expect(session.canMutate).toBe(true);
  });
  it('retains intent after IPC loss and refresh cannot unlock it',async()=>{
    const {invoke,session}=harness(); invoke.mockResolvedValueOnce(snapshot()); await session.refresh();
    invoke.mockRejectedValueOnce(new Error('IPC lost')); await session.submit(privacy);
    const pending=session.state.pending; invoke.mockResolvedValueOnce(snapshot('2')); await session.refresh();
    expect(session.state.pending).toBe(pending); expect(session.canMutate).toBe(false);
  });
  it('does not regress cache5 after a pending-at-1 action receives receipt2',async()=>{
    const {invoke,session}=harness(); invoke.mockResolvedValueOnce(snapshot()); await session.refresh();
    let reply!: (v:unknown)=>void; invoke.mockImplementationOnce(()=>new Promise(r=>{reply=r;}));
    const submit=session.submit(privacy); await vi.waitFor(()=>expect(invoke).toHaveBeenCalledTimes(2));
    invoke.mockResolvedValueOnce(snapshot('5')); await session.refresh();
    let current!: (v:unknown)=>void;
    invoke.mockImplementationOnce(()=>new Promise(r=>{current=r;})); reply(ack(...invoke.mock.calls[1] as [string,Record<string,unknown>]));
    await vi.waitFor(()=>expect(invoke).toHaveBeenCalledTimes(4));
    expect(session.state.snapshot?.snapshot_revision).toBe('5'); expect(session.canMutate).toBe(false);
    current(snapshot('4')); await submit; expect(session.state.snapshot?.snapshot_revision).toBe('5');
    expect(session.state.disposition).toBe('unknown'); expect(session.state.pending?.expected_snapshot_revision).toBe('1');
    invoke.mockImplementationOnce(ack).mockResolvedValueOnce(snapshot('6')); await session.recover(); expect(session.canMutate).toBe(true);
  });
  it('definite stale admission refusal refreshes and requires explicit review',async()=>{
    const {invoke,session}=harness(); invoke.mockResolvedValueOnce(snapshot()); await session.refresh();
    invoke.mockRejectedValueOnce(error('context_stale')).mockResolvedValueOnce(snapshot('2')); await session.submit(privacy);
    expect(session.state.disposition).toBe('refused'); expect(session.canMutate).toBe(false);
    session.reviewRefusal(); expect(session.canMutate).toBe(true);
  });
  it('does not classify an unfamiliar typed stale-error location as definite admission',async()=>{
    const {invoke,session}=harness(); invoke.mockResolvedValueOnce(snapshot()); await session.refresh();
    invoke.mockRejectedValueOnce({...error('context_stale'),issues:[{code:'context_stale',path:'/retained/receipt',entity_id:null}]});
    await session.submit(privacy); expect(session.state.disposition).toBe('unknown'); expect(session.canMutate).toBe(false);
  });
  it('failed recovery keeps intent and instructs restart without a request loop',async()=>{
    const {invoke,session}=harness(); invoke.mockResolvedValueOnce(snapshot()); await session.refresh();
    invoke.mockRejectedValue(error('storage_failure')); await session.submit(privacy); await session.recover();
    expect(invoke).toHaveBeenCalledTimes(3); expect(session.state.pending).not.toBeNull();
    expect(session.state.message).toContain('restart'); expect(session.canMutate).toBe(false);
  });
  it('renderer restart reads committed state without replaying lost intent',async()=>{
    const {invoke,session}=harness(); invoke.mockResolvedValueOnce(snapshot()); await session.refresh();
    invoke.mockRejectedValueOnce('lost'); await session.submit(privacy);
    const restarted=new PlannerSession(createPlannerClient(invoke)); invoke.mockResolvedValueOnce(snapshot('2')); await restarted.refresh();
    expect(restarted.state.pending).toBeNull(); expect(restarted.canMutate).toBe(true);
    expect(invoke.mock.calls.map(c=>c[0])).toEqual(['planner_snapshot','planner_dispatch','planner_snapshot']);
  });
  it('retains one immutable intent when the input object changes after submission',async()=>{
    const {invoke,session}=harness(); invoke.mockResolvedValueOnce(snapshot()); await session.refresh();
    const edit=structuredClone(privacy); invoke.mockRejectedValueOnce('lost'); await session.submit(edit);
    const sent=invoke.mock.calls[1][1];
    if(edit.kind==='set_privacy') edit.data.policy.use_ai_features=true;
    invoke.mockImplementationOnce(ack).mockResolvedValueOnce(snapshot('2')); await session.recover();
    expect(invoke.mock.calls[2][1]).toEqual(sent);
  });
  it('requires current read after acknowledgement and replays the same action if that read fails',async()=>{
    const {invoke,session}=harness(); invoke.mockResolvedValueOnce(snapshot()); await session.refresh();
    invoke.mockImplementationOnce(ack).mockRejectedValueOnce(error('storage_failure')); await session.submit(privacy);
    expect(session.state.disposition).toBe('unknown'); expect(session.state.pending).not.toBeNull();
    const sent=invoke.mock.calls[1][1];
    invoke.mockImplementationOnce(ack).mockResolvedValueOnce(snapshot('3')); await session.recover();
    expect(invoke.mock.calls[3][1]).toEqual(sent); expect(session.state.snapshot?.snapshot_revision).toBe('3');
  });
  it('treats equivalent equal-revision objects as the same state regardless of key order',async()=>{
    const {invoke,session}=harness(); const same=snapshot();
    invoke.mockResolvedValueOnce(same); await session.refresh();
    invoke.mockResolvedValueOnce(Object.fromEntries(Object.entries(same).reverse())); await session.refresh();
    expect(session.state.read).toBe('ready'); expect(session.canMutate).toBe(true);
  });
  it.each(['0','01','1.2','-1','1e4','18446744073709551616',1 as unknown as string])('rejects invalid snapshot revision %s',async rev=>{
    const {invoke,session}=harness(); invoke.mockResolvedValueOnce(snapshot(rev)); await session.refresh();
    expect(session.state.read).toBe('failed'); expect(session.canMutate).toBe(false);
  });
});
const raw=(group:string,name:string)=>readFileSync(`src-tauri/tests/fixtures/receipt/${group}/${name}`,'utf8');
const literal=(group:string,name:string)=>JSON.parse(raw(group,name));
function fixedId(lastByte:number) {
  vi.spyOn(crypto,'getRandomValues').mockImplementation((array:any)=>{array.fill(0);array[15]=lastByte;return array;});
}
function barrier<T>() { let resolve!:(value:T)=>void; const promise=new Promise<T>(r=>{resolve=r;}); return {resolve,promise}; }
const save:AppCommand=literal('r1','transport-current-save.txt').command;
const cancel:AppCommand=literal('r2','cancel-current-transport.txt').command;
describe('frozen receipt/current session obligations',()=>{
  it.each([['r1','current-initial-snapshot.json','transport-current-save.txt','current-applied-receipt.json','current-save-response.json',16],
    ['r2','cancel-current-initial-snapshot.json','cancel-current-transport.txt','cancel-current-applied-receipt.json','cancel-current-response.json',32]] as const)
  ('U-SA-10/36 keeps %s acknowledgement separate until its current read',async(group,initial,transport,receipt,current,id)=>{
    fixedId(id);const {invoke,session}=harness();const start=literal(group,initial);invoke.mockResolvedValueOnce(start);await session.refresh();
    const read=barrier<unknown>();invoke.mockResolvedValueOnce(literal(group,receipt)).mockReturnValueOnce(read.promise);
    const submit=session.submit(literal(group,transport).command);await vi.waitFor(()=>expect(invoke).toHaveBeenCalledTimes(3));
    expect(invoke.mock.calls[1][1]).toEqual({commandJson:raw(group,transport)});
    expect(session.state.snapshot).toEqual(start);expect(session.state.lastReceipt).toEqual(literal(group,receipt));expect(session.canMutate).toBe(false);
    read.resolve(literal(group,current));await submit;expect(session.state.snapshot).toEqual(literal(group,current));expect(session.state.pending).toBeNull();expect(session.canMutate).toBe(true);
  });
  it('U-SA-12/13 absence/read cannot unlock delayed intent; repeated recovery sends once and retains bytes',async()=>{
    fixedId(16);const invoke=vi.fn();const client=createPlannerClient(invoke);const session=new PlannerSession(client);
    invoke.mockResolvedValueOnce(literal('r1','current-initial-snapshot.json'));await session.refresh();
    invoke.mockRejectedValueOnce('transport lost while native write held');await session.submit(save);const pending=session.state.pending;
    invoke.mockRejectedValueOnce({...error('missing_reference'),issues:[{code:'missing_reference',path:'/command_id',entity_id:null}]});
    await expect(client.lookupReceipt(raw('r1','current-save-command.json'))).rejects.toBeTruthy();
    invoke.mockResolvedValueOnce(literal('r1','current-initial-snapshot.json'));await session.refresh();
    expect(session.state.pending).toBe(pending);expect(session.canMutate).toBe(false);await expect(session.submit(privacy)).rejects.toThrow();
    const reply=barrier<unknown>();invoke.mockReturnValueOnce(reply.promise).mockResolvedValueOnce(literal('r1','current-save-response.json'));
    const first=session.recover();await session.recover();await vi.waitFor(()=>expect(invoke).toHaveBeenCalledTimes(5));
    expect(invoke.mock.calls[4]).toEqual(invoke.mock.calls[1]);expect(session.state.pending).toBe(pending);
    reply.resolve(literal('r1','current-replayed-receipt.json'));await first;expect(session.canMutate).toBe(true);
    expect(invoke.mock.calls.filter(c=>c[0]==='planner_replay_receipt')).toHaveLength(1);
  });
  it('U-SA-14/36 cancel replay2 after current3 still waits for a separate current3 confirmation',async()=>{
    fixedId(32);const {invoke,session}=harness();invoke.mockResolvedValueOnce(literal('r2','cancel-current-initial-snapshot.json'));await session.refresh();
    invoke.mockResolvedValueOnce(literal('r2','cancel-current-applied-receipt.json')).mockRejectedValueOnce('read lost');await session.submit(cancel);
    const pending=session.state.pending;invoke.mockResolvedValueOnce(literal('r2','cancel-current-after-privacy-snapshot.json'));await session.refresh();
    expect(session.state.disposition).toBe('unknown');expect(session.state.pending).toBe(pending);expect(session.canMutate).toBe(false);
    const read=barrier<unknown>();invoke.mockResolvedValueOnce(literal('r2','cancel-current-replayed-receipt.json')).mockReturnValueOnce(read.promise);
    const recover=session.recover();await vi.waitFor(()=>expect(invoke).toHaveBeenCalledTimes(6));
    expect(session.state.lastReceipt?.receipt_snapshot_revision).toBe('2');expect(session.state.snapshot?.snapshot_revision).toBe('3');expect(session.state.pending).toBe(pending);
    read.resolve(literal('r2','cancel-current-after-privacy-snapshot.json'));await recover;expect(session.canMutate).toBe(true);
  });
  it.each(['changed-replayed-command_digest.json','changed-replayed-response_digest.json'])('U-SA-11 retains verified fingerprint after %s',async name=>{
    fixedId(16);const {invoke,session}=harness();invoke.mockResolvedValueOnce(literal('r1','current-initial-snapshot.json'));await session.refresh();
    invoke.mockResolvedValueOnce(literal('r1','current-applied-receipt.json')).mockRejectedValueOnce('read lost');await session.submit(save);
    const prior=session.state.lastReceipt,pending=session.state.pending;
    invoke.mockResolvedValueOnce(literal('r1',name));await session.recover();expect(session.state.lastReceipt).toBe(prior);expect(session.state.pending).toBe(pending);
    expect(session.state.disposition).toBe('unknown');expect(session.canMutate).toBe(false);expect(invoke).toHaveBeenCalledTimes(4);
  });
  it('U-SA-11 rejects a well-shaped wrong household receipt against captured household',async()=>{
    fixedId(16);const {invoke,session}=harness();invoke.mockResolvedValueOnce(literal('r1','current-initial-snapshot.json'));await session.refresh();
    invoke.mockResolvedValueOnce(literal('r1','invalid-receipt-wrong-household-id.json'));await session.submit(save);
    expect(session.state.lastReceipt).toBeNull();expect(session.state.pending).not.toBeNull();expect(session.canMutate).toBe(false);
  });
  it('U-SA-15 refuses a latest read older than its receipt without advancing cache',async()=>{
    fixedId(16);const {invoke,session}=harness();const initial=literal('r1','current-initial-snapshot.json');invoke.mockResolvedValueOnce(initial);await session.refresh();
    invoke.mockResolvedValueOnce(literal('r1','current-applied-receipt.json')).mockResolvedValueOnce(initial);await session.submit(save);
    expect(session.state.snapshot).toEqual(initial);expect(session.state.lastReceipt?.receipt_snapshot_revision).toBe('2');expect(session.state.disposition).toBe('unknown');expect(session.canMutate).toBe(false);
  });
  it.each(['1','2'])('U-SA-16 rejects changed household at revision%s',async rev=>{
    const {invoke,session}=harness();invoke.mockResolvedValueOnce(snapshot());await session.refresh();const foreign=snapshot(rev);foreign.household_draft.household_id='hh_00000000000000000000000000000002' as typeof foreign.household_draft.household_id;
    invoke.mockResolvedValueOnce(foreign);await session.refresh();expect(session.state.snapshot).toEqual(snapshot());expect(session.state.read).toBe('failed');
  });
  it('U-SA-17 discards an older query even if it has a higher revision than latest query',async()=>{
    const {invoke,session}=harness();const old=barrier<unknown>();invoke.mockReturnValueOnce(old.promise).mockResolvedValueOnce(snapshot('2'));
    const first=session.refresh();await session.refresh();old.resolve(snapshot('9'));await first;expect(session.state.snapshot).toEqual(snapshot('2'));expect(session.state.read).toBe('ready');
  });
  it('U-SA-17 receipt-bound read superseded by manual read cannot clear pending',async()=>{
    fixedId(16);const {invoke,session}=harness();invoke.mockResolvedValueOnce(literal('r1','current-initial-snapshot.json'));await session.refresh();
    const old=barrier<unknown>();invoke.mockResolvedValueOnce(literal('r1','current-applied-receipt.json')).mockReturnValueOnce(old.promise);
    const submit=session.submit(save);await vi.waitFor(()=>expect(invoke).toHaveBeenCalledTimes(3));
    invoke.mockResolvedValueOnce(literal('r1','current-after-privacy-snapshot.json'));await session.refresh();old.resolve(literal('r1','current-save-response.json'));await submit;
    expect(session.state.snapshot?.snapshot_revision).toBe('3');expect(session.state.pending).not.toBeNull();expect(session.canMutate).toBe(false);
  });
  it('U-SA-18 accepts maximum u64 current read but cannot acknowledge a wrapped increment',async()=>{
    const {invoke,session}=harness();invoke.mockResolvedValueOnce(snapshot('18446744073709551615'));await session.refresh();expect(session.state.read).toBe('ready');
    invoke.mockImplementationOnce(ack);await session.submit(privacy);expect(session.state.disposition).toBe('unknown');expect(session.canMutate).toBe(false);
  });
});
const refusalRows:[AppCommand,DomainError['code'],string][]=[
 [save,'context_stale','/expected_snapshot_revision'],[save,'context_stale','/revision'],[save,'context_stale','/connections/privacy'],
 [privacy,'context_stale','/command/data/policy/revision'],[save,'invalid_value','/command/data/restriction_issue_edits'],
 [cancel,'missing_reference','/command/data/job_id'],[{kind:'update_profile',data:{}} as AppCommand,'unsupported_capability','/command'],
 [privacy,'authority_denied','/command'],
];
const refusal=(code:DomainError['code'],path:string):DomainError=>({code,issues:[{code,path,entity_id:null}],retry:'never',truncated:false});
describe('U-SA-19 exact initial refusal and recovery matrix',()=>{
 it.each(refusalRows)('initial %j %s %s requires review',async(command,code,path)=>{
   const {invoke,session}=harness();invoke.mockResolvedValueOnce(snapshot());await session.refresh();const before=structuredClone(command);
   invoke.mockRejectedValueOnce(refusal(code,path)).mockResolvedValueOnce(snapshot('2'));await session.submit(command);
   expect(session.state.disposition).toBe('refused');expect(session.state.pending).toBeNull();expect(command).toEqual(before);expect(session.canMutate).toBe(false);
   session.reviewRefusal();expect(session.canMutate).toBe(true);
 });
 it.each(refusalRows)('recovery %j %s %s is always unknown',async(command,code,path)=>{
   const {invoke,session}=harness();invoke.mockResolvedValueOnce(snapshot());await session.refresh();invoke.mockRejectedValueOnce('lost');await session.submit(command);
   const pending=session.state.pending;invoke.mockRejectedValueOnce(refusal(code,path));await session.recover();
   expect(session.state.disposition).toBe('unknown');expect(session.state.pending).toBe(pending);expect(session.canMutate).toBe(false);
 });
 const malformed=[null,'lost',{}, {...refusal('context_stale','/expected_snapshot_revision'),extra:true},
   {...refusal('context_stale','/expected_snapshot_revision'),truncated:true},
   {...refusal('context_stale','/expected_snapshot_revision'),retry:'after_refresh'},
   {...refusal('context_stale','/expected_snapshot_revision'),issues:[]},
   {...refusal('context_stale','/expected_snapshot_revision'),issues:[{code:'context_stale',path:'/expected_snapshot_revision',entity_id:'hh_foreign'}]},
   {...refusal('context_stale','/expected_snapshot_revision'),issues:[{code:'context_stale',path:'/expected_snapshot_revision',entity_id:null,extra:true}]},
   {...refusal('context_stale','/expected_snapshot_revision'),issues:[null]},
   {...refusal('context_stale','/expected_snapshot_revision'),issues:[...refusal('context_stale','/expected_snapshot_revision').issues,...refusal('context_stale','/expected_snapshot_revision').issues]},
   {...refusal('context_stale','/expected_snapshot_revision'),code:'invalid_value'},
   refusal('missing_reference','/command_id'),refusal('context_stale','/command/data/draft/revision'),refusal('context_stale','/command/data/draft/connections/privacy'),
   refusal('unsupported_capability','/storage/backup/closure'),refusal('replay_conflict','/command_id'),
   refusal('context_stale','/revision'),refusal('context_stale','/connections/privacy'),refusal('invalid_value','/command/data/restriction_issue_edits'),refusal('missing_reference','/command/data/job_id'),refusal('unsupported_capability','/command')];
 it.each(malformed.map((value,index)=>[index,value] as const))('malformed/unlisted/wrong-kind rejection%s retains intent',async(_index,value)=>{
   const {invoke,session}=harness();invoke.mockResolvedValueOnce(snapshot());await session.refresh();invoke.mockRejectedValueOnce(value);await session.submit(privacy);
   expect(session.state.disposition).toBe('unknown');expect(session.state.pending?.command).toEqual(privacy);expect(session.canMutate).toBe(false);
 });
});
describe('immutable receipt recovery fingerprint',()=>{
 it.each(['profile','replayed-to-applied'] as const)('rejects %s changes on the same live action',async mutation=>{
  fixedId(16);const {invoke,session}=harness();invoke.mockResolvedValueOnce(literal('r1','current-initial-snapshot.json'));await session.refresh();
  const initial=literal('r1',mutation==='profile'?'current-applied-receipt.json':'current-replayed-receipt.json');
  invoke.mockResolvedValueOnce(initial).mockRejectedValueOnce('current read unavailable');await session.submit(save);
  const prior=session.state.lastReceipt,pending=session.state.pending;const changed=literal('r1','current-replayed-receipt.json');
  if(mutation==='profile')changed.receipt_profile='B1RetainedV1';else changed.disposition='applied';
  invoke.mockResolvedValueOnce(changed);await session.recover();expect(session.state.lastReceipt).toBe(prior);expect(session.state.pending).toBe(pending);
  expect(session.state.disposition).toBe('unknown');expect(session.canMutate).toBe(false);expect(invoke).toHaveBeenCalledTimes(4);
 });
});


describe('USA-J1-01 retained receipt floor on every current read',()=>{
  it.each(['failed follow-up','superseding manual'] as const)('rejects manual revision1 after receipt2: %s',async schedule=>{
    const {invoke,session}=harness();invoke.mockResolvedValueOnce(snapshot());await session.refresh();
    const cached=session.state.snapshot;const automatic=barrier<unknown>();
    invoke.mockImplementationOnce(ack);
    if(schedule==='failed follow-up')invoke.mockRejectedValueOnce('automatic read unavailable');
    else invoke.mockReturnValueOnce(automatic.promise);
    const submit=session.submit(privacy);
    await vi.waitFor(()=>expect(invoke).toHaveBeenCalledTimes(3));
    if(schedule==='failed follow-up')await submit;
    const pending=session.state.pending;const receipt=session.state.lastReceipt;const sent=invoke.mock.calls[1];
    expect(receipt?.receipt_snapshot_revision).toBe('2');
    invoke.mockResolvedValueOnce(snapshot('1'));expect(await session.refresh()).toBe(false);
    if(schedule==='superseding manual'){automatic.resolve(snapshot('2'));await submit;}
    expect(session.state.snapshot).toBe(cached);expect(session.state.read).toBe('failed');
    expect(session.state.readError).toContain('cannot currently be read');expect(session.state.lastReceipt).toBe(receipt);
    expect(session.state.pending).toBe(pending);expect(session.state.disposition).toBe('unknown');expect(session.canMutate).toBe(false);
    invoke.mockImplementationOnce(ack).mockResolvedValueOnce(snapshot('2'));await session.recover();
    expect(invoke.mock.calls[4]).toEqual(sent);expect(session.state.pending).toBeNull();expect(session.canMutate).toBe(true);
  });
  it.each([
    ['failed follow-up','2'],['failed follow-up','3'],['superseding manual','2'],['superseding manual','3'],
  ] as const)('valid manual read preserves exact intent: %s revision%s',async(schedule,rev)=>{
    const {invoke,session}=harness();invoke.mockResolvedValueOnce(snapshot());await session.refresh();
    const automatic=barrier<unknown>();invoke.mockImplementationOnce(ack);
    if(schedule==='failed follow-up')invoke.mockRejectedValueOnce('automatic read unavailable');
    else invoke.mockReturnValueOnce(automatic.promise);
    const submit=session.submit(privacy);await vi.waitFor(()=>expect(invoke).toHaveBeenCalledTimes(3));
    if(schedule==='failed follow-up')await submit;
    const pending=session.state.pending;const receipt=session.state.lastReceipt;const sent=invoke.mock.calls[1];
    invoke.mockResolvedValueOnce(snapshot(rev));expect(await session.refresh()).toBe(true);
    if(schedule==='superseding manual'){automatic.resolve(snapshot('2'));await submit;}
    expect(session.state.snapshot?.snapshot_revision).toBe(rev);expect(session.state.read).toBe('ready');
    expect(session.state.readError).toBeNull();expect(session.state.lastReceipt).toBe(receipt);
    expect(session.state.pending).toBe(pending);expect(session.state.disposition).toBe('unknown');
    expect(session.state.message).toContain('follow-up verification failed');expect(session.canMutate).toBe(false);
    await expect(session.submit(privacy)).rejects.toThrow();
    invoke.mockImplementationOnce(ack).mockResolvedValueOnce(snapshot(rev));await session.recover();
    expect(invoke.mock.calls[4]).toEqual(sent);expect(session.state.pending).toBeNull();expect(session.canMutate).toBe(true);
  });
});
