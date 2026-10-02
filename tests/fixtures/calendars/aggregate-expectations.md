# Calendar aggregate oracle

Frozen before writing aggregate.rs or invoking the calendar subject, 2 October 2026.
Owner: bounded aggregate fixture worker; checkpoint1, exact core baseline
06af08f0b457cede7630ed73f2503d60bd751ca7. Reference-only Asana task:
Nutritionist Agent: menu-picker feature (low priority), GID 1216779462407190.

| Case | Owners | Selected meals per owner | Phases per meal | Sessions per owner | Tasks per owner | Total sessions | Total tasks | Calendar result |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| R2-B01 | 18 | 2 | 128 | 256 | 256 | 4608 | 4608 | LimitExceeded, no partial output |
| R2-B02 native expansion | 8 | 21 | 128 | 21 | 2688 | 168 | 21504 | LimitExceeded, no partial output |

R2-B02 uses eight full weeks instead of the reduced witness's six owners with
4096 tasks each. It tests the same independent aggregate limit of 20480 tasks;
every owner remains within 4096 phases, 256 sessions, 256 tasks per session,
21 selections and 128 phases per recipe. R2-B01 exceeds 4352 sessions while
remaining below the task limit. Neither test duplicates a malformed wrapper.

Each synthetic recipe has 128 sequential one-minute phases, comprising one
ingredient-combining phase followed by 127 explicit stirring phases. The final
phase completes cooking and service occurs immediately. This artificial recipe
exists only to exercise the admitted food-domain boundary; its source admission
and scaled prose are explicit synthetic premises, not real recipe or observed
cooking evidence. All phases have nonzero duration and real source step IDs.
Every meal produces a distinct batch and allocates one portion. The complete
saved basis must pass derive_plan, validate_schedule, verify_historical and
assess_continuation with no blocking report before calendar invocation.

Owners have distinct week, approval, order, batch, allocation, demand-line and
session identities. They are independently admitted owner requests, not one
shared occupancy ledger: the aggregate rejection must happen before expansion
or interpretation of cross-owner occupancy. No account or store authority is
asserted. Calendar assertions are parent-owned; this file freezes only their
literal expected counts and failure class.

## Pre-subject amendment authorised by root

Before any calendar call, the initial source-only run exposed excessive core
validation cost at 2688 tasks per owner. Root authorised this bounded replacement:

| Final case | Owners | Meals per owner | Phases per meal | Sessions per owner | Tasks per owner | Total sessions | Total tasks | Calendar result |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| R2-B02 native expansion | 54 | 3 | 128 | 3 | 384 | 162 | 20736 | LimitExceeded, no partial output |

This row supersedes only the original R2-B02 count row; R2-B01 is unchanged.
The 54 owners are below the 65-owner limit and individually below food limits.
The 20736 total exceeds the same adopted 20480 aggregate task cap, with sessions
below 4352. This is an input-size optimisation, not a relaxation of validation.
Every owner still passes complete unchanged-core food checks before calendar.
The original oracle was preserved in calendar-implementation/aggregate-initial-expectations.md
with SHA256 9774e89e80543b5604b19e45c067534f31f9e3a7171005132f921ea7d114f52a.
The amended literal counts were written before changing the fixture constructor.

## Final pre-subject amendment: smaller real recipes

Root subsequently authorised reducing each recipe from 128 phases to 16 while
increasing actual selected meals. This removes expensive repeated large-recipe
schema scans. The following rows supersede both earlier count tables:

| Final case | Owners | Meals per owner | Phases per meal | Sessions per owner | Tasks per owner | Total sessions | Total tasks | Calendar result |
|---|---:|---:|---:|---:|---:|---:|---:|---|
| R2-B01 | 18 | 16 | 16 | 256 | 256 | 4608 | 4608 | LimitExceeded, no partial output |
| R2-B02 | 61 | 21 | 16 | 21 | 336 | 1281 | 20496 | LimitExceeded, no partial output |

Every recipe now has 16 sequential one-minute phases: ingredient combination,
14 intermediate stirring phases and the final stirring/production phase. Its
text and fixed offsets describe exactly 16 minutes, with immediate same-occasion
service and disposal of the second unallocated portion. No storage/reuse claim
is present. R2-B01 selects the first 16 chronological meals of the week and
R2-B02 selects all 21. The complete native checks remain unchanged; no fabricated
derived task or repeated malformed owner is introduced. Owners remain below 65.
This final literal amendment was saved before changing constructor source, and
before calendar invocation. All earlier running binaries remain separately
identified as evidence for their earlier source bytes.
