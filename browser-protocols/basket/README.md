# Controlled-basket protocol candidate

This folder is isolated from the Site and browser Rust domain. Nothing here is
integrated or qualified to operate a retailer account. The tests use explicitly
synthetic host guards and observations. They establish coordination behaviour,
not live retailer evidence, basket truth, approval or worker-stop capability.

`exact.ts` supplies bounded integer/rational pack and price arithmetic.
`protocol.ts` supplies a finite, JSON-serialisable ledger and pure asynchronous
reducers using browser WebCrypto SHA-256. There are no effects, network calls,
credentials, UI, arbitrary scripts or URLs in either module. Effect results are
fixed product/variant quantity instructions. All returned values are frozen.

## Intent and arithmetic

`prepareIntent(input)` creates a reviewable immutable intent containing the
exact plan, demand, product-decision, price-quote, context and privacy bindings;
delivery binding; complete baseline; each exact SKU/variant; quantities/forms;
core match digest; explicit baseline credit; approved new pack count; applicable
price terms; expiry; and canonical digests. A submitted intent is rebuilt and
checked exactly when started. Extra keys, unsupported conversions, unknown
suitability, duplicate demand/SKU references and tampering are refused.

The owner's core matcher must produce required and pack-supply quantities in the
**same unit and form**. This module does no food scaling, density conversion or
ingredient inference. For example, 480 g drained demand needs two 240 g drained
packs; 400 g gross is refused as a substitute. A SKU cannot independently cover
two demand rows in this bounded version; the core matching owner must aggregate
or resolve that allocation before preparing an intent.

Counts are bounded by 10,000. Rational components and monetary minor units are
canonical uint64 decimal strings and use BigInt arithmetic. Money includes a
three-letter currency code and explicit minor-unit exponent (0–4). Currency or
exponent mismatches refuse; no FX, GBP assumption or floating point conversion
is performed. Actual currency/exponent validity is the qualified adapter's
responsibility; a syntactically valid code is not an ISO-currency assertion.

Only explicitly evidenced, applicable, repeatable **independent per-product**
offer bundles are supported. Unknown delivery applicability, ineligible loyalty
and delivery outside the offer range do not lower the quote. The optimiser
prices exactly the already approved pack count; it never adds a pack for a
discount. Cross-SKU/basket promotions, fees, tax, stock reservations and delivery
holds need separate qualified support. `purchase_quote` is new-pack spend only;
`total_checkout_spend` is deliberately null.

This candidate is additive: target quantity equals the protected baseline count
plus approved new packs. Existing/manual goods are never removed or replaced.
Baseline goods count towards plan coverage only through the explicit reviewed
`credit_existing_packs`. There is no automatic stock credit, negative delta,
rollback, clear-basket action or inferred ownership of existing goods. Later
adjustment/removal needs a separately reviewed contribution-ownership protocol.

## Required host authority

`createProtocol(guards)` requires all six functions. No permissive production
defaults exist. These callbacks are trusted owner code, never inbound JSON or
model fields:

| Guard | Owner obligation |
|---|---|
| `current_binding(key)` | Return current canonical plan/demand/product/price/context/privacy stamps, including the selected delivery and account identity epoch in context. |
| `approved(ref,intent)` | Verify genuine human approval of this exact immutable intent, exact core matching/suitability and allocations, current evidenced currency/offer rules, and expiry. A string or checksum is not approval. |
| `adapter_qualified(ref,key)` | Verify qualification and resolve the real canonical retailer/account/basket, not caller-chosen aliases. Require complete correlated observations, exact quantity semantics and conditional/manual-change-safe mutation. |
| `worker_allowed(worker,binding)` | Verify this exact live worker generation, permissions and privacy. AI revocation must fail this for AI workers. |
| `accepted_read(request,observation)` | Admit qualified complete read facts for the exact scope/time/key/delivery. A read digest establishes correlation, not truth or completeness. A null request is the qualified baseline admission. |
| `qualified_stop(fact,worker)` | Verify an actual qualified stop guarantee, exact intent and worker generation, evidence and qualification. Cancellation, timeout, browser closure or self-reported text is insufficient. |

Imported JSON does **not** gain authority from `auditLedger`. That is a bounded
shape/reference/intent-custody/invariant check, not a cryptographic proof of
historical transitions or external facts. Only the authenticated owner journal
may be used directly. A backup/import needs independently qualified history,
approval/read/stop evidence, account-context reconciliation and conservative
quarantine; an importer cannot merely accept self-signed journal claims. A host
must not use this folder's synthetic test guards in production.

## Durable execution and recovery

The sequence is:

1. Prepare and display intent; obtain genuine approval; `start` obtains the
   writer key `(retailer, account_ref, basket_ref)`. Delivery does not change it.
2. `intend` records the exact operation; `requestRead(preflight)` records its
   read scope. Persist each journal result before releasing its read request.
3. Admit a fresh complete preflight using `grant`. It verifies baseline and
   price scope and records a unique expiring grant. **It returns no effect.**
4. `claim` rechecks approval/context/privacy and records `executing`; this alone
   returns the fixed SetQuantity instruction. Persist that claim atomically
   before consuming the instruction **once**. A duplicate claim refuses.
5. A fresh correlated verification read resolves the exact target. A read at the
   old count while the worker may still execute is uncertain, not failure.
6. A fresh complete final read proves every approved target, coverage and all
   unrelated goods unchanged. The status is `reconciled`, never Booked, checked
   out, paid, delivery held or calendar committed.

`uncertain`, `cancel` and `suspend` produce no effects. Possible grants/executions
retain a quarantined writer. `afterRestart` is mandatory before releasing any
old grant/outbox from a cold owner journal; a possible effect becomes uncertain.
An uncertain claim commit acknowledgement is treated the same way. It is never
safe to replay the returned effect from memory or an exported outbox.

Recovery requires a qualified stop recorded after the possible action, followed
by a new scoped complete observation **strictly later** than the stop's recorded
time. The stop guard is checked again when requesting and accepting recovery,
including cold JSON round trips. Target already present resolves observed
success and produces no duplicate effect. Old quantity after proven stop
resolves observed failure; an unchanged current intent may then create a **new**
operation/grant under an incremented worker generation. Manual/price/unrelated
goods changes remain quarantined. A cancelled job may abandon unresolved
attribution after stop plus fresh observation; it leaves all goods untouched.
Recovery after stale context/approval/privacy remains read-only and requires
review before another effect.

## Storage and live-adapter integration seam

Root should compute reducers outside the IndexedDB transaction, then CAS against
both the planner owner generation/identity epoch and this ledger revision. The
same transaction must persist the immutable intent, approval reference, journal
and **central account/basket writer lock**. Release reads/effects only after
`transaction.oncomplete`, never from a transaction callback before commit. Two
computed claims from one revision are allowed; only one committed CAS may
release an effect. Commit failure releases nothing; uncertain commit outcome
quarantines. Recheck live account/context/approval/privacy immediately before
executing, and report suspension instead of acting after revocation.

The execution adapter must enforce grant/operation/generation consumption and
expiry, exact product variant and compare-before-set semantics. This reducer
cannot make a remote retailer update atomic with a browser CAS or prevent a
manual edit between preflight and a nonconditional retailer click. That gap
must be qualified against the actual adapter before live use. A local IndexedDB
lock also cannot prove exclusion across devices or other account clients; an
account-global lock/worker-stop facility is a remaining integration requirement.
Receipt/schema migration and mapping the shared contracts' currently narrower
retailer/currency wire types require explicit owner work; do not silently map
this protocol into those types.

Finite bounds: 32 retained jobs, 128 selected demand/SKU rows per intent, 512
complete basket rows, 256 operations and 512 read scopes per job, 16 independent
offers per product, and a 4 MB ledger/canonical hash bound. Exhaustion refuses;
no history is silently truncated. Retention/archive policy needs owner design.

## Isolated checks

From the repository root (no WASM/native build):

```powershell
node --experimental-transform-types --test browser-protocols/basket/protocol.test.mjs
node node_modules/typescript/bin/tsc --noEmit --strict --skipLibCheck --target ES2022 --module ESNext --moduleResolution bundler --allowImportingTsExtensions --lib ES2022,DOM browser-protocols/basket/exact.ts browser-protocols/basket/protocol.ts
```

The tests cover exact form/pack and currency arithmetic, applicable offer
thresholds, unchanged manual goods and explicit stock credit, immutable intent
tampering, all authority gates, account/basket writer exclusion, a synthetic CAS
claim harness, stale/privacy refusal, cold lost-ack recovery, qualified stop and
generation binding, fresh/correlated/replayed reads, no-effect recovery/retry,
manual conflicts, cancellation, complete reconciliation and explicit limits.
