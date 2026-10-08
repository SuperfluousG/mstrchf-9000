import type { ShellCommandReceipt, ShellRequestEncoding } from '../generated/shell';

export type ReceiptBinding = {
  request_encoding: ShellRequestEncoding;
  request_digest: string;
  command_id: string;
  command_kind: string;
  expected_snapshot_revision: string;
  household_id?: string;
};
const keys = ['format_version', 'disposition', 'receipt_profile', 'request_encoding', 'request_digest',
  'command_id', 'command_kind', 'expected_snapshot_revision', 'household_id', 'receipt_snapshot_revision',
  'command_digest', 'response_digest'];
export function revision(value: unknown): bigint {
  if (typeof value !== 'string' || !/^[1-9][0-9]*$/.test(value) || value.length > 20) throw new Error('Invalid revision');
  const result = BigInt(value);
  if (result > 18446744073709551615n) throw new Error('Revision exceeds u64');
  return result;
}
export function householdId(value: unknown): value is string {
  return typeof value === 'string' && /^hh_(?!0{32}$)[0-9a-f]{32}$/.test(value);
}
export async function requestDigest(text: string): Promise<string> {
  const bytes = await crypto.subtle.digest('SHA-256', new TextEncoder().encode(text));
  return Array.from(new Uint8Array(bytes), byte => byte.toString(16).padStart(2, '0')).join('');
}
// Header extraction binds an acknowledgement only. Native owns full envelope admission.
export function requestBinding(text: string, encoding: ShellRequestEncoding): Omit<ReceiptBinding, 'request_digest'> {
  const value = JSON.parse(text);
  if (!value || typeof value.command_id !== 'string' || !/^cm_(?!0{32}$)[0-9a-f]{32}$/.test(value.command_id)
    || !value.command || typeof value.command.kind !== 'string') throw new Error('Invalid request identity');
  revision(value.expected_snapshot_revision);
  return { request_encoding: encoding, command_id: value.command_id, command_kind: value.command.kind,
    expected_snapshot_revision: value.expected_snapshot_revision };
}
export function decodeReceipt(value: unknown, binding: ReceiptBinding, prior?: ShellCommandReceipt | null): ShellCommandReceipt {
  if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error('Invalid receipt');
  const row = value as Record<string, unknown>;
  if (Object.keys(row).length !== keys.length || keys.some(key => !Object.hasOwn(row,key))
    || row.format_version !== 1 || !['applied','replayed'].includes(row.disposition as string)
    || !['B1RetainedV1','B1Draft005','DevelopmentCurrent'].includes(row.receipt_profile as string)
    || !['current_json','canonical_receipt_json'].includes(row.request_encoding as string)
    || !['open_planning_horizon','save_household_draft','set_privacy','cancel_job','update_profile','save_week'].includes(row.command_kind as string)
    || typeof row.command_id !== 'string' || !/^cm_(?!0{32}$)[0-9a-f]{32}$/.test(row.command_id)
    || !householdId(row.household_id)) throw new Error('Invalid receipt shape');
  for (const key of ['request_digest','command_digest','response_digest']) {
    if (typeof row[key] !== 'string' || !/^[0-9a-f]{64}$/.test(row[key])) throw new Error('Invalid receipt digest');
  }
  for (const [key, expected] of Object.entries(binding)) {
    if (row[key] !== expected) throw new Error('Receipt does not match request');
  }
  if (revision(row.receipt_snapshot_revision) !== revision(row.expected_snapshot_revision) + 1n
    || (row.disposition === 'applied' && !['B1Draft005','DevelopmentCurrent'].includes(row.receipt_profile as string))
    || (row.request_encoding === 'canonical_receipt_json'
      && (row.disposition !== 'replayed' || row.request_digest !== row.command_digest))) throw new Error('Invalid receipt relationship');
  if (prior && (keys.some(key => key !== 'disposition' && row[key] !== prior[key as keyof ShellCommandReceipt])
    || (prior.disposition === 'replayed' && row.disposition !== 'replayed'))) throw new Error('Acknowledgement fingerprint changed');
  return Object.freeze({ ...row }) as ShellCommandReceipt;
}
