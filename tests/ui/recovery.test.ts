import { describe, it, expect, vi } from 'vitest';
import type { DomainError, AppCommand } from '../../app/generated/contracts';
import { PlannerSession, newCommandId } from '../../app/runtime/commandRecovery';
import { createPlannerClient } from '../../app/runtime/tauriClient';
import { snapshot } from '../fixtures/ui/snapshot';

const privacy: AppCommand = { kind: 'set_privacy', data: { policy: snapshot().privacy, expected_policy_revision: '1' } };
const error = (code: DomainError['code']): DomainError => ({code, issues: [{code,path:'/expected_snapshot_revision',entity_id:null}], retry: 'never', truncated: false});
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
    invoke.mockResolvedValueOnce(snapshot('2')).mockResolvedValueOnce(snapshot('3')); await session.recover();
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
    invoke.mockResolvedValueOnce(snapshot('2')).mockRejectedValueOnce('read unavailable'); await session.submit(privacy);
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
    expect(session.canMutate).toBe(false);
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
    invoke.mockResolvedValueOnce(snapshot('2')).mockResolvedValueOnce(snapshot('3'));
    await session.recover(); expect(invoke.mock.calls[2][1]).toEqual(sent);
    expect(session.state.snapshot?.snapshot_revision).toBe('3'); expect(session.canMutate).toBe(true);
  });
  it('retains intent after IPC loss and refresh cannot unlock it',async()=>{
    const {invoke,session}=harness(); invoke.mockResolvedValueOnce(snapshot()); await session.refresh();
    invoke.mockRejectedValueOnce(new Error('IPC lost')); await session.submit(privacy);
    const pending=session.state.pending; invoke.mockResolvedValueOnce(snapshot('2')); await session.refresh();
    expect(session.state.pending).toBe(pending); expect(session.canMutate).toBe(false);
  });
  it('does not regress cache to historical receipt; fresh read required before next action',async()=>{
    const {invoke,session}=harness(); invoke.mockResolvedValueOnce(snapshot('5')); await session.refresh();
    let current!: (v:unknown)=>void;
    invoke.mockResolvedValueOnce(snapshot('2')).mockImplementationOnce(()=>new Promise(r=>{current=r;}));
    const submit=session.submit(privacy); await vi.waitFor(()=>expect(invoke).toHaveBeenCalledTimes(3));
    expect(session.state.snapshot?.snapshot_revision).toBe('5'); expect(session.canMutate).toBe(false);
    current(snapshot('6')); await submit; expect(session.state.snapshot?.snapshot_revision).toBe('6'); expect(session.canMutate).toBe(true);
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
    invoke.mockResolvedValueOnce(snapshot('2')).mockResolvedValueOnce(snapshot('2')); await session.recover();
    expect(invoke.mock.calls[2][1]).toEqual(sent);
  });
  it('requires current read after acknowledgement and replays the same action if that read fails',async()=>{
    const {invoke,session}=harness(); invoke.mockResolvedValueOnce(snapshot()); await session.refresh();
    invoke.mockResolvedValueOnce(snapshot('2')).mockRejectedValueOnce(error('storage_failure')); await session.submit(privacy);
    expect(session.state.disposition).toBe('unknown'); expect(session.state.pending).not.toBeNull();
    const sent=invoke.mock.calls[1][1];
    invoke.mockResolvedValueOnce(snapshot('2')).mockResolvedValueOnce(snapshot('3')); await session.recover();
    expect(invoke.mock.calls[3][1]).toEqual(sent); expect(session.state.snapshot?.snapshot_revision).toBe('3');
  });
  it('treats equivalent equal-revision objects as the same state regardless of key order',async()=>{
    const {invoke,session}=harness(); const same=snapshot();
    invoke.mockResolvedValueOnce(same); await session.refresh();
    invoke.mockResolvedValueOnce(Object.fromEntries(Object.entries(same).reverse())); await session.refresh();
    expect(session.state.read).toBe('ready'); expect(session.canMutate).toBe(true);
  });
  it.each(['0','01','1.2','-1','1e4'])('rejects invalid snapshot revision %s',async rev=>{
    const {invoke,session}=harness(); invoke.mockResolvedValueOnce(snapshot(rev)); await session.refresh();
    expect(session.state.read).toBe('failed'); expect(session.canMutate).toBe(false);
  });
});
