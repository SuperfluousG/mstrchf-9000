# Native calendar preparation

`prepare_events(&CalendarInputs)` returns complete planned event drafts and unchanged typed continuation reports, or an atomic refusal. `version_events` proposes scoped event revisions from explicit retained custody. `render_ics` returns the fixed snapshot profile as bytes. These functions have no clock, file, store, account or publication effect.

Callers supply exact retained `HistoricalApproval` and current `ContinuationInputs` for the subject and every contributing owner, explicit evaluation time, complete assembly membership, and admitted occurrence-specific presentation text. Public native structs are revalidated; no JSON import/admission route is exposed.

The implementation replays native historical integrity, current continuation and schedule checks, then projects production/extra quantities using public reviewed helpers. Direct reuse reproduces the adopted exact rational projection locally; it has no new source ingredients. Every out-of-method instruction prerequisite resolves to its earlier source component's production anchor and validated task. Text labels that prerequisite as scheduled work and retains the original clock; it does not report observed cooking.

Only roots reached from subject sessions and the subject delivery become drafts. Linked owner histories may contain other sessions; those sessions and the prior owner's delivery are not exported merely because their owner was loaded. Assembly membership is checked against the supplied full histories and attachment ancestry. Missing retained owner/member custody requires reconciliation. Scoped versioning checks supplied old/new contribution maps and retained event custody.

Presentation evidence binds exact owner, occurrence, method, scaled projection and complete canonical selected-step closure. It cannot establish semantic truth. Trusted producers must check numeric quantities in prose, cooking/cooling/storage meaning, evidence applicability, immutable timing domains, complete owner queries, real confirmation and stable identity. A digest or evidence pin supplies none of those facts by itself.

Limits are additional calendar restrictions: 64 linked owners plus subject, 4,352 aggregate sessions/contributors, 20,480 tasks/instruction occurrences, 256 output events, 256 selected steps per occurrence, and 65,536 UTF-8 bytes per complete body. Oversized content is refused without truncation.

All fixtures are synthetic and keep their explicit assumed admission premises. No real recipe, order, file export, calendar client, phone display, import/update or product acceptance is established here. The root's fresh-service admission ruling remains open at the producer boundary.

Extra projection recomputes the canonical hard-requirement union from actual extra recipients, matching the accepted core correction. Restrictions on an actual recipient, explicit ExtraSpec restrictions and base-meal restrictions remain enforced. The asymmetric fixture establishes this through admitted synthetic native history and current continuation before exercising the calendar.

## Scoped versioning

Call `version_events(&prepared, &update, &fresh_ids, &revised_at)` only with an explicit `CalendarUpdateProjection`. Each scoped key declares `NeverVersioned`, `Retained` or `Unavailable` custody and an Upsert/Remove disposition. Complete before/after contributor mappings are checked against the supplied records. Missing custody, removals, role changes, reparenting, splits and merges require reconciliation; there is no partial snapshot or retirement list.

Fresh app-owned IDs are required exactly for never-versioned upserts. Changed content or semantic membership increments the retained payload revision and sequence without replacing its ID. Unchanged content retains the entire event, sequence and revision time, including when evidence pins or continuation reports renew. Event payload hashing uses the unchanged contracts JCS domain; contributor provenance, sequence and revision time remain outside that digest.

The result contains only scoped upserts. The store must merge those into its retained ledger and preserve unrelated events. It must also establish truthful scope, complete old/new membership, global ID uniqueness and genuine never-versioned custody; native validation cannot detect an internally consistent lie. No persistence or real calendar-client replacement is proved here.

## Fixed ICS profile

`render_ics(&snapshot)` revalidates present event digests, native zoned moments, complete text, contribution pins, IDs, keys and revision metadata. It preserves blocking continuation reports and refuses malformed or oversized inputs atomically. It cannot reconstruct absent history or establish truthful producer claims.

Events sort by start instant then canonical logical key. UID uses the app-owned event ID; DTSTAMP uses the supplied retained revision time; SEQUENCE uses retained version metadata. Times encode in UTC with no ambient clock or zone lookup outside the pinned native validator. A zero-duration cooking/preparation event omits DTEND; delivery must have positive duration. Selected DST folds are preserved and gaps refused. The installed chrono-tz 0.10.4 data used by these checks is IANA 2025b.

The profile emits only VCALENDAR VERSION, PRODID and CALSCALE plus each VEVENT's UID, DTSTAMP, SEQUENCE, DTSTART, optional DTEND, SUMMARY and DESCRIPTION. TEXT escapes backslash, comma, semicolon and LF exactly once. UTF-8 scalar boundaries survive greedy 75-octet CRLF folding, including the continuation space. The result has final CRLF, no BOM and a 32 MiB total byte cap. No cancellation, alarm, attendee, organiser, recurrence or import/update protocol is implemented.
