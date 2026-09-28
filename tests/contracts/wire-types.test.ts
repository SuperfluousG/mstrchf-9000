import type {
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
