import type { AppCommand, AppCommandV1, CommandId, DomainError, Snapshot } from '../generated/contracts';
import type { PlannerClient } from './PlannerClient';
type Disposition = 'idle' | 'pending' | 'committed' | 'refused' | 'unknown';
export type SessionState = {
  snapshot: Snapshot | null;
  read: 'loading' | 'ready' | 'failed';
  readError: string | null;
  disposition: Disposition;
  pending: AppCommandV1 | null;
  message: string;
};
function revision(value: string): bigint {
  if (!/^[1-9][0-9]*$/.test(value)) throw new Error('Invalid snapshot revision');
  return BigInt(value);
}
function stable(value: unknown): string {
  if (Array.isArray(value)) return `[${value.map(stable).join(',')}]`;
  if (value !== null && typeof value === 'object') {
    return `{${Object.entries(value).sort(([a], [b]) => a < b ? -1 : a > b ? 1 : 0)
      .map(([key, val]) => `${JSON.stringify(key)}:${stable(val)}`).join(',')}}`;
  }
  return JSON.stringify(value);
}
function freeze<T>(value: T): T {
  if (value && typeof value === 'object') {
    for (const item of Object.values(value)) freeze(item);
    Object.freeze(value);
  }
  return value;
}
export function newCommandId(): CommandId {
  const bytes = new Uint8Array(16);
  do { crypto.getRandomValues(bytes); } while (bytes.every(byte => byte === 0));
  return `cm_${Array.from(bytes, byte => byte.toString(16).padStart(2, '0')).join('')}` as CommandId;
}
function definiteRefusal(value: unknown): boolean {
  if (!value || typeof value !== 'object') return false;
  const error = value as Partial<DomainError>;
  // Only audited admission codes on contracts003. Receipt/parse failures,
  // ReplayConflict and every unclassified failure remain unknown regardless of Retry.
  if (!Array.isArray(error.issues) || error.issues.length !== 1 || error.truncated !== false) return false;
  const issue = error.issues[0];
  if (issue.code !== error.code) return false;
  if (error.code === 'unsupported_capability' || error.code === 'authority_denied') return issue.path === '/command';
  return error.code === 'context_stale' && ['/expected_snapshot_revision', '/command/data/draft/revision',
    '/command/data/draft/connections/privacy', '/command/data/policy/revision'].includes(issue.path);
}
export class PlannerSession {
  private current: SessionState = { snapshot: null, read: 'loading', readError: null, disposition: 'idle', pending: null, message: '' };
  private listeners = new Set<() => void>();
  private query = 0;
  private busy = false;
  constructor(private readonly client: PlannerClient) {}
  get state(): SessionState { return this.current; }
  get canMutate(): boolean {
    return !this.busy && this.current.read === 'ready' && this.current.snapshot !== null
      && this.current.pending === null && ['idle', 'committed'].includes(this.current.disposition);
  }
  subscribe = (listener: () => void) => { this.listeners.add(listener); return () => { this.listeners.delete(listener); }; };
  getState = () => this.current;
  private set(change: Partial<SessionState>) {
    this.current = { ...this.current, ...change };
    this.listeners.forEach(listener => listener());
  }
  private accept(incoming: Snapshot): boolean {
    const next = revision(incoming.snapshot_revision);
    const saved = this.current.snapshot;
    if (saved) {
      const existing = revision(saved.snapshot_revision);
      if (next < existing) return false;
      if (next === existing && stable(incoming) !== stable(saved)) throw new Error('Inconsistent snapshot at the same revision');
    }
    this.set({ snapshot: freeze(structuredClone(incoming)) });
    return true;
  }
  async refresh(): Promise<boolean> {
    const request = ++this.query;
    this.set({ read: 'loading' });
    try {
      const incoming = await this.client.loadSnapshot();
      const accepted = this.accept(incoming);
      if (request !== this.query) return false;
      if (!accepted) throw new Error('Current saved state could not be confirmed');
      this.set({ read: 'ready', readError: null });
      return true;
    } catch {
      if (request === this.query) this.set({ read: 'failed', readError: 'Saved state cannot currently be read. A normal app restart may be required.' });
      return false;
    }
  }
  async submit(command: AppCommand): Promise<void> {
    if (!this.canMutate) throw new Error('Review or recover the current action before another mutation');
    const envelope = freeze(structuredClone({ schema_version: 1 as const, command_id: newCommandId(),
      expected_snapshot_revision: this.current.snapshot!.snapshot_revision, command }));
    this.set({ pending: envelope, disposition: 'pending', message: '' });
    await this.send(envelope, false);
  }
  async recover(): Promise<void> {
    if (this.busy || !this.current.pending) return;
    await this.send(this.current.pending, true);
  }
  reviewRefusal(): void {
    if (!this.busy && this.current.disposition === 'refused' && this.current.read === 'ready') {
      this.set({ disposition: 'idle', message: '' });
    }
  }
  private async send(envelope: AppCommandV1, recovery: boolean): Promise<void> {
    this.busy = true;
    this.set({ disposition: 'pending' });
    try {
      let receipt: Snapshot;
      try {
        receipt = await this.client.dispatch(envelope);
      } catch (error) {
        if (!recovery && definiteRefusal(error)) {
          this.set({ pending: null, disposition: 'refused', message: 'The action was refused. Your edit is retained; compare it with current saved state before trying again.' });
          await this.refresh();
        } else {
          this.set({ disposition: 'unknown', message: recovery
            ? 'Recovery is unavailable. Keep this edit for reference and restart the app normally to read saved state. The lost intent will not be replayed after restart.'
            : 'The outcome is unknown. Your edit and exact action are retained in this session. Recover that action before making another change.' });
        }
        return;
      }
      // Replay receipts can be historical. Never regress the current cache.
      this.accept(receipt);
      const refreshed = await this.refresh();
      if (refreshed) this.set({ pending: null, disposition: 'committed', message: 'Action acknowledged; current saved state loaded.' });
      else this.set({ disposition: 'unknown', message: 'The action was acknowledged, but follow-up verification failed. Recover the same action or restart the app normally.' });
    } catch {
      this.set({ disposition: 'unknown', message: 'The response could not be verified. Recover the exact action; a normal app restart may be required.' });
    } finally {
      this.busy = false;
      this.set({});
    }
  }
}
