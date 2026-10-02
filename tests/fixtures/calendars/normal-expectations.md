# C01 full synthetic normal-history oracle

Frozen before normal.rs construction. Source: reviewed calendar preparation C01
and worked layout, with newly explicit complete synthetic instruction/storage
premises. This does not reuse the renderer's incomplete method wording as food
admission evidence.

- Confirmed delivery Monday 12 October 2026 16:00–17:00 Europe/London.
- Dinner 18:00–18:10: base 2 whole bread slices and 60 g prepared grated cheese,
  two diners with one serving each. Diner 2 alone receives 100 g raw tomatoes.
- Base preparation is active 18:00–18:03, then grill unattended 18:03–18:10.
  Tomato extra is active 18:03–18:05. Breakfast preparation is active
  18:05–18:08 inside that SAME session, linked to Tuesday breakfast's occurrence.
- Breakfast Diner 1 only: 40 g dry oats and 100 ml prepared milk. Finish Tuesday
  13 October 07:55–08:00 in a separate Cooking session. No attached_to link.
- Exactly three calendar events, two sessions, five phase tasks, three exact
  recipe refs, five ingredient demand lines, two source batches and three
  first-service allocations. No leftover/reuse links or extra prep event.
- Dinner session links Monday dinner and Tuesday breakfast. Breakfast phase 2
  depends on exact phase 1 in the previous dinner session; its content references
  that earlier prep without repeating it as a new execution.
- Original oats first-handling/storage clock is Monday 18:05, not Tuesday's
  finishing phase. Storage ceiling 86,400 seconds; cold first service at Tuesday
  08:00 is 50,100 seconds after that clock. No reheating is claimed.
- Synthetic product bindings and known use-by 14 October cover every actual
  demand use. Complete current continuation is expected to have no warnings or
  blockers. Approval 11 October 11:00Z; explicit synthetic confirmation12:00Z;
  evaluation12:01Z, all before ingredient use. These are fictional premises.

Literal base preparation: Put 2 whole bread slices on a clean grill tray. Spread
the 60 g prepared grated cheese evenly over the two slices. Place the tray under
the preheated grill at the end of this three-minute preparation phase.

Literal grill: Grill the two cheese-topped bread slices unattended for seven
minutes until the cheese has melted and the bread is toasted. At 18:10 serve one
slice to Diner 1 and one slice to Diner 2. This synthetic method assumes that this
exact grill setting and unattended interval have been admitted; discard uneaten
toast after service because no storage method is admitted.

Literal extra: Wash 100 g cherry tomatoes under running water. Halve them on a
clean board during the two-minute phase and put the entire 100 g extra on Diner
2's plate for service at 18:10. Diner 1 receives no tomato extra. Discard uneaten
tomatoes after the meal.

Literal oats prep: Measure 40 g dry oats and 100 ml prepared milk into a clean
covered container. Stir thoroughly during the three-minute preparation phase,
cover and refrigerate immediately at 5 C or below by 18:08. Keep the container
chilled overnight for Diner 1's Tuesday breakfast.

Literal oats finish: At 07:55 remove Diner 1's covered oats container from the
refrigerator. Stir the prepared oats and milk thoroughly, transfer the full
portion to a clean bowl and serve cold by 08:00. Add no new ingredients and do
not repeat Monday's preparation. Discard any remainder after breakfast.

Literal oats storage: Cover and refrigerate the prepared oats and milk at 5 C
or below immediately after mixing, no later than Monday 18:08. Retain the
original first-handling clock Monday 18:05 and use within 24 hours of that clock.
Keep chilled until Tuesday 07:55; serve cold by 08:00. Discard any remainder.

Literal oats reheating rule: Serve this fixture's oats cold. No reheating method
is admitted and the maximum reheats is zero.

All quantities, timing-domain applicability, equipment and complete text are
explicit synthetic reviewed assumptions. No real recipe, source review, actual
checkout, observed cooking or client import is represented.

## Bounded grill-instruction correction

Appended before changing the two source constants. Root's text review found
that the original preparation omitted the preheating action and the grill step
referred to an unnamed setting. The original oracle is preserved in main
`loop/001/calendar-implementation/c01-grill-text-correction/normal-expectations-before.md`,
SHA256 `0ed317419a44c75928f7b9cc4d2773f993b6f9f3462cf17f8eb34434d365c318`.
Only the following two instruction literals supersede the corresponding
paragraphs above; quantities, timings, resources and other expectations remain.

Corrected literal base preparation: At 18:00 turn the fictional electric oven
grill to medium (setting 2 of 3). While it preheats, put 2 whole bread slices on
a clean grill tray and spread the 60 g prepared grated cheese evenly over the
two slices. Place the tray under the grill at 18:03, after the three-minute
preheating and assembly phase.

Corrected literal grill: With the fictional electric oven grill still at medium
(setting 2 of 3), grill the two cheese-topped bread slices unattended for seven
minutes until the cheese has melted and the bread is toasted. At 18:10 serve one
slice to Diner 1 and one slice to Diner 2. Discard uneaten toast after service
because no storage method is admitted.

These are explicit fictional appliance premises, not evidence of cooker safety
or a real recipe cook test. Exact recipe digests regenerate through the existing
builder; no timing or resource changes are made.

## C03 exact standalone-prep variant

Appended before constructing `normal_standalone()`. This is a separate complete
synthetic history derived from C01's saved basis; C01 remains unchanged.

Exactly four calendar events are expected, ordered by their actual starts:

1. Saturday 10 October 2026 delivery, 16:00–17:00 Europe/London.
2. Sunday 11 October standalone oats Prep, 19:00–19:03 Europe/London.
3. Monday 12 October breakfast Cooking, 07:55–08:00 Europe/London.
4. Monday 12 October dinner Cooking, 18:00–18:10 Europe/London, retaining the
   two bread slices / 60 g cheese base for both diners and the 100 g tomato extra
   for Diner 2 only. Dinner contains no oats-preparation execution.

The oats occurrence now belongs to Monday breakfast. Its exact Production
method retains two phases: Sunday prep and Monday finish. The Sunday phase has
its own AdvancePrep session, no `attached_to`, and the finish keeps the exact
cross-session prerequisite. The source/storage clock is Sunday 19:00; the
three-minute mixing/refrigeration phase finishes by 19:03. Monday 08:00 first
service is 46,800 seconds after that clock, within the explicitly supported
86,400-second synthetic storage rule. No reheating is admitted.

Three sessions, five phase tasks, five demand lines, two source batches and
three first-service allocations remain expected. The dinner has three tasks;
the prep and breakfast sessions have one task each. Current source pins cover
both Monday occurrences and all three recipes, including the exact new oats
revision. Complete synthetic product-use bindings with use-by 14 October cover
the changed uses; expected current warnings and blockers remain zero.

Synthetic approval is Saturday 10 October 11:00Z, confirmation12:00Z and
evaluation12:01Z, before delivery and all food handling. Product dates and
source metadata do not claim observed freshness or actual checkout.

Literal C03 oats prep: On Sunday at 19:00 measure 40 g dry oats and 100 ml
prepared milk into a clean covered container. Stir thoroughly during the
three-minute preparation phase, cover and refrigerate immediately at 5 C or
below by 19:03. Keep the container chilled overnight for Diner 1's Monday
breakfast.

Literal C03 oats finish: On Monday at 07:55 remove Diner 1's covered oats
container from the refrigerator. Stir the prepared oats and milk thoroughly,
transfer the full portion to a clean bowl and serve cold by 08:00. Add no new
ingredients and do not repeat Sunday's preparation. Discard any remainder
after breakfast.

Literal C03 oats storage: Cover and refrigerate the prepared oats and milk at
5 C or below immediately after mixing, no later than Sunday 19:03. Retain the
original first-handling clock Sunday 19:00 and use within 24 hours of that
clock. Keep chilled until Monday 07:55; serve cold by 08:00. Discard any
remainder.

C01's explicit cold-only reheating rule remains verbatim. The fictional source
domain is deliberately rebound to the exact changed recipe revision and fixed
phase window; no generic freshness exception or relaxed core constraint is
introduced. Source semantic admission and storage truth remain synthetic
premises, not production recipe or cooker-safety evidence.
