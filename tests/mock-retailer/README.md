# Synthetic local test shop

Standalone P06/P08 test infrastructure, **not** an app component or retailer integration. Node 24; built-in HTTP/filesystem/test facilities only. No install, package manifest, remote resources, credentials, checkout or payment routes. Every page identifies itself as synthetic.

From the repository/worktree root:

```powershell
node tests/mock-retailer/run.mjs
node --test tests/mock-retailer/retailer.test.mjs
```

The launcher prints one JSON line containing its loopback URL, temporary data directory and ledger path. Open the exact `http://127.0.0.1:<port>/` URL. Port defaults to an available ephemeral port; `--port 8765` selects a fixed port. There is no host override. Stop with Ctrl+C. A process left running remains available to local clients; shut it down after qualification.

Restart with the **same printed temporary directory** to recover its basket and ledger:

```powershell
node tests/mock-retailer/run.mjs --data-dir 'C:\Users\YOUR_USER\AppData\Local\Temp\mstrchf-mock-PRINTED_SUFFIX'
```

Use the actual printed path, not that illustrative placeholder. Data directories must exist below the resolved system temp directory. Each default invocation creates a new run. No automatic ledger deletion occurs. Copy selected synthetic evidence into the owning loop before OS temp cleanup; never put real household data in this fixture. Only its temp lock file is removed on clean close or when the previous PID is demonstrably absent. A live/reused PID fails closed. One server owns a data directory at a time.

## What is visible

The shop exposes opaque basket/run IDs, exact integer basket quantities, observation time and revision. Baseline: one beans tin and two unrelated soap bars. Beans have 400g net and 240g drained weight; eggs are six discrete units. Invented prices, stock, ingredients/allergens, label evidence, offer dates, loyalty requirement and delivery context are visible as labelled JSON. The fixture does not calculate product suitability, grant authority, applicable offer pricing or planner contributions. Price/label/stock changes are observations for the future app to interpret.

Product controls submit either an additive quantity or an exact target quantity. Manual controls change the same SKU or unrelated goods without consuming a worker fault. Both actor types are labelled in the server ledger. A quantity of zero removes the contribution semantically but remains an explicit zero in the fixture basket. No whole-basket replacement exists except the explicit test reset.

## Fault controls

Arm a mode with the UI's **Next worker action / Arm one-shot fault**, or POST `/api/control`. It is consumed by one structurally valid worker action request, not manual edits. Controls are test-operator machinery; the later native worker should interact only with the shop/product/basket observations and granted actions. This is a workflow convention, not access isolation.

| `nextAction` | Actual mutation / response |
|---|---|
| `normal` | Commit and return quantity/revision. |
| `drop-after-commit` | Append/flush the mutation, begin an HTTP response, then truncate its JSON body. No usable acknowledgement. |
| `hold-after-commit` | Append/flush mutation; retain response at the `ack` gate until release/discard. Other reads remain available. |
| `never-commit` | Record receipt and non-commit, then truncate the response. Basket unchanged. |
| `gate-before-commit` | Record receipt and pause at `before`. Release rechecks current basket/login/stock and calculates from the current quantity. |
| `raw-drop-after-commit` | Optional transport-risk probe: commit then close before any response. A browser may transparently resend, causing multiple mutations. |

`{ "release": "before" }` / `{ "release": "ack" }` releases all pending entries at that stage in arrival order. `{ "discard": "before" }` drops pending uncommitted requests; `{ "discard": "ack" }` drops acknowledgements but retains committed quantities. Discard sends an incomplete response to avoid the tested browser's pre-response retry. No release happens on a timer. A disconnected client does **not** cancel a before gate: it stays visible and can still commit when released. Maximum 16 parked requests.

To gate a second action: perform the first, arm `gate-before-commit`, submit the second, and inspect independent evidence before release. To simulate interruption of the app or native worker, leave this shop running; stopping the shop itself is a different test. Pending sockets/gates are abandoned on shop restart, never replayed. The armed next-action mode resets to normal on restart; basket, observations, read-failure count and historical evidence survive.

Other controls:

```json
{ "readFailures": 2 }
{ "loggedIn": false }
{ "loggedIn": true }
{ "product": { "sku": "beans", "inStock": false, "pricePence": 120, "labelStatus": "unknown" } }
```

Read failures affect only `/api/state`, are consumed one per read and return 503. Simulated expired sign-in returns 401 for shop reads/mutations; no login fields or credentials exist. Stock blocks increases. Labels support `known`, `unknown`, `contradictory`; unknown has null evidence fields, contradictory includes explicit conflicting synthetic claims. Price can be an integer number of pence, zero or null (unknown). This fixture never turns unknown/contradictory labels into an app suitability verdict.

## HTTP boundary

POST requires exact `Content-Type: application/json`. Unknown fields, invalid numbers, unknown SKUs and unsupported operations are rejected. No cookies or authorisation headers are required or logged. Host must be the exact printed IPv4 loopback host/port; foreign Origin or `Sec-Fetch-Site: cross-site` is rejected. CORS is not enabled. Do not proxy, tunnel or expose it remotely.

| Method/path | Input or result |
|---|---|
| GET `/` | Synthetic shop and operator controls. |
| GET `/api/state` | Current shop observation; subject to read/login faults. |
| GET `/api/admin` | Independent current state and pending stages, including disconnected flag; bypasses read/login faults. |
| GET `/api/ledger` | Ordered server event ledger, including state snapshots. |
| POST `/api/action` | `{ "basketId": "<observed ID>", "actionId": "<1–80 ASCII letters/digits/_/->", "sku": "beans", "operation": "add", "quantity": 1 }` |
| POST `/api/manual` | `{ "basketId": "<observed ID>", "sku": "soap", "quantity": 3 }` |
| POST `/api/control` | Fault/configuration examples above; may combine non-conflicting keys. |
| POST `/api/reset` | `{ "confirm": "RESET SYNTHETIC SHOP" }`; new run/basket IDs, baseline restored, pending gates discarded, old ledger retained. |

`add` accepts 1–99; `set` and manual targets accept 0–99. Resulting quantities cannot exceed 99. Prices accept 0–100000 pence or null; read-failure count 0–100. Request bodies are limited to 16KiB, headers to 8KiB, connections to 64, request/header receipt to 10 seconds, journal to 10,000 events or 32MiB. Full journals refuse further writes; retain them and start a fresh temp directory. Static routes are an exact allowlist; arbitrary files and paths cannot be served.

## Durable evidence and limits

The append-only `ledger.jsonl` is the authoritative mock state store. Each mutation entry carries its complete next state and actual before/after quantity, server-generated request ID, caller action ID, actor, timestamp and sequence. Synchronous append with flush completes **before** in-memory state changes or a success response. Restarts replay snapshots only, never action receipts. Reset appends another snapshot without truncating evidence. An incomplete/corrupt tail causes startup to fail closed and preserves the file for investigation; no guessed repair or silent discard. This is process-restart evidence, not a power-loss/filesystem durability guarantee.

There is intentionally **no action-ID deduplication**. Duplicate requests can add twice. The browser smoke test found that a raw pre-response disconnect caused Chromium to resend the same action ID: two distinct server request IDs and two commits followed one UI action. The primary loss mode now truncates a started response and did not transparently retry on the tested in-app browser. That observation is not a guarantee for every client. The optional raw mode preserves the failure mechanism. Reconcile the actual observed basket; never infer one mutation from one click, request ID or worker statement.

The ledger is independent of worker self-report but local and unauthenticated. Any local client can use operator endpoints; any process with filesystem permission can alter the file. It is not tamper-proof, multi-tenant, a sandbox or app-owned authority. It does not simulate app grants, locks, privacy, worker quiescence, P06 file handshakes, checkout, delivery-slot holds, CAPTCHA or usage quotas.

HTTP tests and browser smoke establish **mock behaviour only**. Native-harness cancellation, browser/file alternation, app reconciliation, account operations and P06/P08 acceptance require later integrated qualification. The fixture cannot prove that an external worker stopped merely because no request arrived.
