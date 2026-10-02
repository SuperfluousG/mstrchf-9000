# Frozen checkpoint 1 synthetic fixture expectations

Frozen before implementing the fixture builders or calendar subject. These are synthetic assumptions, never production admission or observed cooking. Derivation follows reviewed C14 and R2-R01/R02; the fixture shifts the witness by one week to F11's Monday 5 October while retaining its literal 400 g / two-serving / 200 g ratio.

Direct cold and reheated reuse: one source recipe, two current occurrences, one batch, two 200 g allocations, two tasks and sessions, four source ingredient demand lines. Monday 5 October production is 18:00–18:10 Europe/London (17:00–17:10Z); source first service at 18:10. Tuesday 6 October use is 13:00 Europe/London (12:00Z). Cold phase 2 executes 12:59–13:00; reheated phase 2 executes 12:55–13:00. Original storage clock remains Monday 17:10Z. Source production phase 1 is an exact dependency of target phase 2. No target raw ingredient demand or target production step. Expected calendar event count: three (delivery, source cooking, target cooking). Target allocation is exactly 200 g, reheat count zero or one respectively.

Source ingredients are exactly 160 g drained beans, 120 g raw tomatoes, 110 g raw onion and 10 ml prepared oil on the declared two-serving 400 g cooked yield. Yield is a reviewed synthetic declaration, not inferred mass balance. Source serves one portion and retains one portion.

Literal source instruction: Combine 160 g drained beans, 120 g chopped tomatoes, 110 g chopped onion and 10 ml oil in the pan. Stir over medium heat for 10 minutes until the mixture is steaming throughout. Divide the declared 400 g cooked yield into two 200 g portions; serve one immediately.

Literal cooling/storage instruction: Transfer the remaining 200 g portion into a clean shallow covered container, cool promptly and refrigerate within one hour at 5 C or below. Keep chilled until Tuesday service and use within 48 hours of the original Monday 18:10 production completion. This fixture assumes that cooling and storage method was independently admitted for these exact quantities and timings; the calendar does not observe it happening.

Literal cold reuse instruction: Remove the reserved 200 g portion from the refrigerator immediately before service. Transfer it to a clean plate and serve cold within the one-minute preparation interval; discard any remainder after service. Do not cook the source batch again.

Literal reheated reuse instruction: Remove the reserved 200 g portion from the refrigerator. Heat in a saucepan over medium heat for five minutes, stirring continuously, until steaming throughout. Serve immediately at 13:00; discard any remainder. Reheat this portion once only.

Source prerequisite heading is exactly: Preparation scheduled earlier for this reuse.

Remix fixture: F11 declared 1200 g batch / four servings, source consumes 600 g and two later remixes each consume 300 g. Four source ingredient demands (480 g drained beans, 400 g raw tomatoes, 160 g raw onion, 20 ml prepared oil), plus one wrap and 250 g raw potato; six total demand lines, three sessions/tasks, four calendar events. Each remix's source allocation uses the original source clock and adds only its own new ingredient.

Literal remix source instruction: Combine 480 g drained beans, 400 g chopped tomatoes, 160 g chopped onion and 20 ml oil in the pan. Stir over medium heat for 10 minutes until steaming throughout. From the declared 1200 g cooked yield, serve 600 g immediately and reserve two separate 300 g portions.

Literal remix cooling instruction: Put each reserved 300 g portion in a separate clean shallow covered container. Cool promptly and refrigerate within one hour at 5 C or below. Keep chilled and use within 48 hours of Monday 18:00 production completion. These are assumed admitted synthetic storage premises, not observed actions.

Literal wrap instruction: Take one chilled 300 g source portion and one wrap. Heat the source portion in a saucepan for eight minutes, stirring until steaming throughout. Place it in the wrap, fold and serve during the remaining two minutes. Reheat this portion once only and discard any remainder.

Literal potato instruction: Wash and pierce one 250 g raw potato. Microwave for five minutes, turn it, and microwave for another three minutes until the centre is soft. Meanwhile heat the separate chilled 300 g source portion in a saucepan until steaming throughout. Split the potato, add the hot source portion and serve within the ten-minute phase. Reheat this portion once only and discard any remainder.

Additional isolated standalone-prep oracle (not the C03 renderer example): one Monday source occurrence with explicitly admitted chilled first-service timing. Production is Sunday 4 October 19:00–19:10 London; Monday use is 18:10, within the represented 48-hour storage limit. One AdvancePrep session, one 200 g allocation from the declared 400 g batch, two expected calendar events (Saturday delivery and Sunday prep). The unallocated 200 g remainder has no later use promised.

Literal standalone instruction: Combine 160 g drained beans, 120 g chopped tomatoes, 110 g chopped onion and 10 ml oil in the pan. Stir over medium heat for 10 minutes until steaming throughout. Divide the declared 400 g yield into two 200 g portions. Cool promptly in clean shallow covered containers and refrigerate within one hour at 5 C or below. Keep chilled; serve the allocated 200 g portion cold on Monday at 18:10. No later meal is assigned the other portion; discard it within 48 hours of Sunday 19:10 production completion.

Additional isolated extra oracle: retain Monday source only and add a 100 g raw tomato extra for its one named diner. Extra executes 17:59–18:00; base executes 18:00–18:10 in the same session. One session with two tasks, one source allocation, five demand lines; two expected calendar events. Base source yields 400 g and allocates 200 g; no reuse occurrence is included. The extra is admitted as fresh-only for the exact 11-minute first-handling-to-consumption domain. It has no storage/reuse support.

Literal extra instruction: Wash 100 g tomatoes under running water. Cut into bite-sized pieces on a clean board, put on the named diner's plate and serve alongside the main meal at 18:10. Complete the chopping during the one-minute phase; discard any uneaten extra after the meal.

Linked prior-owner oracle: prior week starts 28 September, with Sunday 4 October dinner at 19:15. Its own complete source history has active mixing 18:50–18:55, followed by unattended oven cooking 18:55–19:15; source consumes 200 g from 400 g immediately, with no storage support. The new week's Sunday standalone prep 19:00–19:10 attaches to that exact prior Cooking session, during unattended time and using a different resource (hob versus oven). Both full histories and both current continuations must validate. Expected calendar output is only subject delivery plus changed prior-root cooking event; the prior delivery stays outside this preparation scope. Root event has exactly two contributors and three executed tasks; full retained prior owner is required.

Literal prior mixing instruction: Mix 160 g drained beans, 120 g chopped tomatoes, 110 g chopped onion and 10 ml oil in an ovenproof dish during the five-minute active phase. Place the dish in the oven at 180 C.

Literal prior unattended instruction: Bake the dish unattended at 180 C for 20 minutes until steaming throughout. At 19:15 serve the allocated 200 g from the declared 400 g yield immediately. No storage method is admitted for this prior recipe; discard the unallocated 200 g after service.

The isolated extra's unused source portion is unallocated: its storage prose must say no later meal is assigned it and require discard within 48 hours of Monday 18:10. It must not claim a selected Tuesday reuse.

The potato remix explicitly requires hob plus microwave; those resources are present in its synthetic household, requirements, recipe and phase evidence.

Body-bound oracle, frozen before source mutations: add enough admitted zero-duration explanatory steps to a selected production phase that complete event text exceeds 65,536 UTF-8 bytes, while each source Text and selected-step count remains within its own bound. The full native history and current continuation must still validate before calendar invocation. Calendar must return LimitExceeded with no partial drafts or truncation. A 65,536-byte body is admitted at the body boundary if all other constraints pass. This does not promise the whole renderer fits its later aggregate byte bound.

Missing cooking surface: a core-valid occurrence with no executable phase, if expressible through the existing public admission boundary, must not become an invented 30-minute calendar meal. If native batch/production closure already refuses such a history, preserve that earlier refusal and do not fabricate a historical wrapper to claim calendar-only coverage.

Metadata chronology correction, commissioned after the initial native runs:
the F11 prototype inherited December source retrieval, review and household /
privacy confirmation instants while these fixtures plan October meals. All such
inherited December `retrieved_at`, `at` and `confirmed_at` metadata becomes exactly
`2026-10-01T00:00:00Z`, before the synthetic 3 October evaluation. Planned meal /
production / original-storage clocks and quantities stay unchanged. Recipe
digests and all exact source/dependent reference joins are rebuilt. Earlier
green results belong to the preceding bytes; these revised fixtures await rerun
through root's contained toolchain route.
