import type {
  Diner, DinerId, WeekId, JobPayload,
  DecimalU64, Revision, Instant, Unit, Resolution, Quantity,
  JobRequestV1, WeekPlanDocumentV1, MealRequirementsDocumentV1,
  EventBatchDocumentV1, Snapshot, PrivacyPolicy,
} from '../../app/generated/contracts';

type Assert<T extends true> = T;
type IsRequired<T, K extends keyof T> = {} extends Pick<T, K> ? false : true;

type _RequiredNullableWeek = Assert<IsRequired<JobRequestV1, 'week_id'>>;
type _WeekIncludesNull = Assert<null extends JobRequestV1['week_id'] ? true : false>;
type _WeekPlanEnvelope = Assert<IsRequired<WeekPlanDocumentV1, 'schema_version'>>;
type _RequirementsEnvelope = Assert<IsRequired<MealRequirementsDocumentV1, 'meal_requirements'>>;
type _EventBatchEnvelope = Assert<IsRequired<EventBatchDocumentV1, 'event_batch'>>;
type _SnapshotPolicyRequired = Assert<IsRequired<Snapshot, 'privacy'>>;
type _SnapshotPolicyNonNull = Assert<null extends Snapshot['privacy'] ? false : true>;
declare const snapshot: Snapshot;
declare const policy: PrivacyPolicy;
const { privacy: _privacy, ...withoutPrivacy } = snapshot;
// @ts-expect-error Snapshot policy cannot be omitted or inferred from Connections
const missingSnapshotPrivacy: Snapshot = withoutPrivacy;
// @ts-expect-error Snapshot policy is non-null
const nullSnapshotPrivacy: Snapshot = { ...snapshot, privacy: null };
const explicitSnapshotPrivacy: Snapshot = { ...snapshot, privacy: policy };
void [_privacy, missingSnapshotPrivacy, nullSnapshotPrivacy, explicitSnapshotPrivacy];

declare const amount: Quantity;
const acceptedUnit: Unit = amount.unit;
void acceptedUnit;

// @ts-expect-error revisions are decimal strings on the wire
const badRevision: Revision = 1;
// @ts-expect-error a decimal u64 never crosses the wire as a JS number
const badDecimal: DecimalU64 = 12;
// @ts-expect-error unit is a closed enum
const badUnit: Unit = 'kg';
// @ts-expect-error resolution is a closed enum
const badResolution: Resolution = 'fold';
// @ts-expect-error instant is a UTC string, not a Unix number
const badInstant: Instant = 1792891800;
void [badRevision, badDecimal, badUnit, badResolution, badInstant];

declare const probeWeek: WeekId;
declare const probeDiner: Diner;
// @ts-expect-error ID brands must remain distinct
const wrongDinerId: DinerId = probeWeek;
const { display_name: _displayName, ...probeWithoutName } = probeDiner;
// @ts-expect-error nullable is required
const missingDinerName: Diner = probeWithoutName;
// @ts-expect-error union kind is closed
const unknownJobPayload: JobPayload = { kind: 'arbitrary', data: {} };
// @ts-expect-error payload must match its discriminant
const mismatchedJobPayload: JobPayload = { kind: 'setup_check', data: { week_ids: [] } };
// @ts-expect-error quantity numerators remain decimal strings
const numericQuantity: Quantity = { amount: { numerator: 1, denominator: '1' }, unit: 'g', form: 'raw' };
const validSetupPayload: JobPayload = { kind: 'setup_check', data: { provider: 'codex_local', requested_capabilities: [] } };
void [wrongDinerId, _displayName, missingDinerName, unknownJobPayload, mismatchedJobPayload, numericQuantity, validSetupPayload];

import type { BatchUse, DerivedBatch, DerivedComponent, DerivedPlan, ComponentSpec,
  ComponentProductionSpec, ProductionMoment, RecipeRevision, ReusedComponentInput } from '../../app/generated/contracts';
type _RequiredProduction = Assert<IsRequired<Extract<BatchUse, {kind: 'new_batch'}>['data'], 'production_servings'>>;
type _RequiredClock = Assert<IsRequired<DerivedComponent, 'original_production'>>;
type _RequiredStorage = Assert<IsRequired<DerivedComponent, 'storage_rule'>>;
type _NullableStorage = Assert<null extends DerivedComponent['storage_rule'] ? true : false>;
type _RequiredAnchor = Assert<IsRequired<ComponentSpec, 'production'>>;
type _NullableAnchor = Assert<null extends ComponentSpec['production'] ? true : false>;
type _RequiredReuse = Assert<IsRequired<RecipeRevision, 'reused_inputs'>>;
type _RequiredFoodDigest = Assert<IsRequired<DerivedPlan, 'food_inputs_digest'>>;
type _RemovedClock = Assert<'original_cook' extends keyof DerivedBatch ? false : true>;
declare const component: DerivedComponent;
declare const spec: ComponentSpec;
declare const revision: RecipeRevision;
const freshComponent: DerivedComponent = {...component, storage_rule: null};
const unknownAnchor: ComponentSpec = {...spec, production: null};
const ordinaryRecipe: RecipeRevision = {...revision, reused_inputs: []};
// @ts-expect-error original production is a closed, required union
const nullClock: ProductionMoment = null;
// @ts-expect-error wrong anchor payload for the discriminant
const wrongAnchor: ComponentProductionSpec = {kind: 'cooked', data: {first_handling_prep_id: 1}};
// @ts-expect-error source recipe is required
const incompleteReuse: ReusedComponentInput = {source_component_id: component.component_id};
void [freshComponent, unknownAnchor, ordinaryRecipe, nullClock, wrongAnchor, incompleteReuse];
