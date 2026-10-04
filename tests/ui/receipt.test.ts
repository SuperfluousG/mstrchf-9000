import { readFileSync, readdirSync } from 'node:fs';
import { createHash } from 'node:crypto';
import { describe, it, expect, vi } from 'vitest';
import type { AppCommandV1 } from '../../app/generated/contracts';
import { createPlannerClient } from '../../app/runtime/tauriClient';
const root = 'src-tauri/tests/fixtures/receipt/';
const raw = (group:string, name:string) => readFileSync(`${root}${group}/${name}`, 'utf8');
const literal = (group:string, name:string) => JSON.parse(raw(group,name));
const digest = (text:string) => createHash('sha256').update(text).digest('hex');
describe('literal receipt binding (transport double; native owns canonical/body hashes)',()=>{
  it.each([
    ['r1','transport-current-save.txt','current-applied-receipt.json'],
    ['r1','transport-current-save.txt','current-replayed-receipt.json'],
    ['r2','cancel-current-transport.txt','cancel-current-applied-receipt.json'],
    ['r2','cancel-current-transport.txt','cancel-current-replayed-receipt.json'],
    ['r2','cancel-old-current-transport.txt','cancel-old-dispatch-receipt.json'],
  ])('decodes exact %s/%s -> %s',async(group,transport,receipt)=>{
    const text=raw(group,transport); const invoke=vi.fn().mockResolvedValue(literal(group,receipt));
    const result=await createPlannerClient(invoke).dispatch(JSON.parse(text));
    expect(invoke).toHaveBeenCalledWith('planner_dispatch',{commandJson:text});
    expect(result).toEqual(literal(group,receipt)); expect(result).not.toHaveProperty('household_draft');
  });
  it.each([['r1','old-save-command.json','old-replayed-receipt.json'],['r2','cancel-old-command.json','cancel-old-lookup-receipt.json']])('looks up exact historical %s/%s',async(group,command,receipt)=>{
    const invoke=vi.fn().mockResolvedValue(literal(group,receipt)); const text=raw(group,command);
    expect(await createPlannerClient(invoke).lookupReceipt(text)).toEqual(literal(group,receipt));
    expect(invoke).toHaveBeenCalledExactlyOnceWith('planner_replay_receipt',{originalCommandJson:text});
  });
  for(const name of readdirSync(`${root}r1`).filter(name=>name.startsWith('invalid-receipt-') && !name.includes('wrong-household'))) {
    it(`rejects literal ${name}`,async()=>{
      const invoke=vi.fn().mockResolvedValue(literal('r1',name));
      await expect(createPlannerClient(invoke).dispatch(literal('r1','transport-current-save.txt'))).rejects.toThrow();
    });
  }
  it.each(['disposition','request_digest','command_digest'])('rejects invalid lookup %s',async field=>{
    const receipt=literal('r1','old-replayed-receipt.json'); receipt[field]=field==='disposition'?'applied':'a'.repeat(64);
    await expect(createPlannerClient(vi.fn().mockResolvedValue(receipt)).lookupReceipt(raw('r1','old-save-command.json'))).rejects.toThrow();
  });
  it('retains the first transport and binding even if a direct caller mutates its object',async()=>{
    const text=raw('r1','transport-current-save.txt'); const command=JSON.parse(text) as AppCommandV1;
    const invoke=vi.fn().mockResolvedValue(literal('r1','current-applied-receipt.json')); const client=createPlannerClient(invoke);
    await client.dispatch(command); command.expected_snapshot_revision='99';
    await client.dispatch(command); expect(invoke.mock.calls[1][1]).toEqual({commandJson:text});
  });
  it('does not claim browser verification of absent canonical command/response bodies',async()=>{
    const receipt=literal('r1','current-applied-receipt.json'); receipt.command_digest='a'.repeat(64); receipt.response_digest='b'.repeat(64);
    expect(await createPlannerClient(vi.fn().mockResolvedValue(receipt)).dispatch(literal('r1','transport-current-save.txt'))).toEqual(receipt);
    expect(receipt.request_digest).toBe(digest(raw('r1','transport-current-save.txt')));
  });
  it.each(['{}','null','{','{"command_id":7}'])('malformed lookup cannot fall back to dispatch: %s',async text=>{
    const invoke=vi.fn(); await expect(createPlannerClient(invoke).lookupReceipt(text)).rejects.toThrow();
    expect(invoke).not.toHaveBeenCalled();
  });
});
