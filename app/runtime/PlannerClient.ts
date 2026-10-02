import type { AppCommandV1, Snapshot } from '../generated/contracts';
import type { ShellFeatures } from '../generated/shell';
export interface PlannerClient {
  loadSnapshot(): Promise<Snapshot>;
  dispatch(command: AppCommandV1): Promise<Snapshot>;
  loadFeatures(): Promise<ShellFeatures>;
}
