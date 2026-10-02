import type { AppCommandV1, Snapshot } from '../generated/contracts';
import type { ShellFeatures } from '../generated/shell';
import type { PlannerClient } from './PlannerClient';
export type Invoke = (command: string, args?: Record<string, unknown>) => Promise<unknown>;
export function createPlannerClient(invoke: Invoke): PlannerClient {
  // Live-session object-keyed bytes only, not a persistent command-id registry.
  const retained = new WeakMap<AppCommandV1, string>();
  return {
    loadSnapshot: () => invoke('planner_snapshot') as Promise<Snapshot>,
    loadFeatures: () => invoke('planner_features') as Promise<ShellFeatures>,
    dispatch(command) {
      let commandJson = retained.get(command);
      if (commandJson === undefined) {
        commandJson = JSON.stringify(command);
        retained.set(command, commandJson);
      }
      return invoke('planner_dispatch', { commandJson }) as Promise<Snapshot>;
    },
  };
}
