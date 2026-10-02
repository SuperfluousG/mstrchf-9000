import type { Snapshot, HouseholdId } from '../../../app/generated/contracts';
// Isolated transport/component double. It does not prove native integration.
export function snapshot(revision = '1'): Snapshot {
  return {
    schema_version: 1, snapshot_revision: revision,
    privacy: { revision, use_ai_features: false, include_household_requirements: false, confirmed_at: null },
    household_draft: { schema_version: 1, household_id: 'hh_00000000000000000000000000000001' as HouseholdId,
      revision: '1', completed_stages: [], diners: [], preferences: null, usual_week: null, connections: null },
    profile: null, weeks: [], jobs: [], conflicts: [],
  };
}
