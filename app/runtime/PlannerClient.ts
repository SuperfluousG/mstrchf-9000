import type { AppCommandV1, Snapshot } from '../generated/contracts';
import type { ShellFeatures, ShellCommandReceipt } from '../generated/shell';
export interface PlannerClient {
  loadSnapshot(): Promise<Snapshot>;
  dispatch(command: AppCommandV1): Promise<ShellCommandReceipt>;
  lookupReceipt(originalCommandJson: string): Promise<ShellCommandReceipt>;
  loadFeatures(): Promise<ShellFeatures>;
}
