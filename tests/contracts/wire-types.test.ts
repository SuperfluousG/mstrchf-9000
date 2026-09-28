import type {
  Diner, DinerId, WeekId, JobPayload,
  DecimalU64, Revision, Instant, Unit, Resolution, Quantity,
  JobRequestV1, WeekPlanDocumentV1, MealRequirementsDocumentV1,
  EventBatchDocumentV1,
} from '../../app/generated/contracts';

type Assert<T extends true> = T;
type IsRequired<T, K extends keyof T> = {} extends Pick<T, K> ? false : true;

type _RequiredNullableWeek = Assert<IsRequired<JobRequestV1, 'week_id'>>;
type _WeekIncludesNull = Assert<null extends JobRequestV1['week_id'] ? true : false>;
type _WeekPlanEnvelope = Assert<IsRequired<WeekPlanDocumentV1, 'schema_version'>>;
type _RequirementsEnvelope = Assert<IsRequired<MealRequirementsDocumentV1, 'meal_requirements'>>;
type _EventBatchEnvelope = Assert<IsRequired<EventBatchDocumentV1, 'event_batch'>>;

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
