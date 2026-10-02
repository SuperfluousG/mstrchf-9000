# `mstrchf-store` B1 boundary

This crate is the B1 local persistence boundary. Its public API is deliberately small:

- `Store::open(StoreConfig { app_data_root })` opens the per-household local store and starts its single owner thread.
- `Store::snapshot()` returns the current validated snapshot.
- `Store::dispatch_ui(AppCommandV1)` accepts commands from the local UI, checks their contract and caller authority, and returns the committed snapshot.
- `Store::import_result(&[u8])` parses a job result but currently rejects every well-formed result as `UnsupportedCapability`.

The owner thread receives requests on a bounded channel of 64 entries. Calls use non-blocking queue admission; a full or disconnected queue is reported as a storage failure. The SQLite connection stays with the owner. Admission preparation and mutation types, the database handle, receipt mechanics, and test controls are private. There is no public mutation trait, prevalidated flag, or test-only downstream hook. `tests/api_boundary.rs` checks a normal consumer against the public API, then checks that private admission and the `cfg(test)` module are inaccessible.

The implemented UI mutations are `SaveHouseholdDraft`, `SetPrivacy`, and `CancelJob`. The other nine command producers are deliberately unsupported in B1: `UpdateProfile`, `SaveWeek`, `ApplyPlanProposal`, `ConfirmEssentials`, `ApprovePlan`, `ConfirmOrder`, `RequestJob`, `RecordUserStop`, and `RequestCalendarPublication`. Qualified adapters are not acquired or run. B1 provides no adapter execution, result application, export, or restore API. `S21` coverage is local cancellation only; it does not imply that result import is implemented.

The boundary tests use the existing contract JSON fixtures as synthetic inputs. They adapt a draft fixture to the generated household identity of its temporary store. Mechanism tests that need persisted job/profile records seed synthetic rows directly inside the crate's `cfg(test)` module. Those tests exercise storage projections and refusal paths; they do not demonstrate a production job, profile, recipe, or asset producer.

`tests/fixtures/manifest.json` indexes B1 executable coverage by case ID and actual Rust test name. It is a coverage map, not a run report; executed counts belong to the implementation handoff. `tests/fixtures/required-case-ids.json` contains the applicable B1 IDs. `S06` and `S07` remain deferred recipe and asset producer mechanisms. `S13`, `S14`, and `S25` cover only the B1 record subset; they do not establish multiweek producer behaviour. B2, C1, C2 and `ST-R1-03` through `ST-R1-05` remain deferred, as does typed evidence-registry closure.

Migration-ledger and future-version refusal tests describe current executable behaviour. They do not freeze a later schema, approve migration numbering, or close the separate schema review.

The host supplies a trusted local app-data directory, never a renderer-selected path, synced workspace or network share. Tests use unique system temporary directories and retain failed-case evidence. Windows is the implemented filesystem target: initialisation records the app-created stage's volume/file identity, flushes it, and publishes using `MoveFileExW` without replacement. Unknown/empty existing live databases, ledger disagreement, future versions and upgrades requiring unavailable verified backup refuse. Process-kill tests establish process-crash recovery on the tested filesystem, not power-loss guarantees.

SQLite is pinned and bundled, with WAL, synchronous FULL, foreign keys on, trusted schema off and a 2000ms busy timeout. Migration001 is checksummed; recipes, assets, weeks, result/grant journals and portable formats are not allocated placeholder schemas. Command receipts retain the original canonical response and command bytes. Exact replay precedes current CAS and returns the original response even after later state changes. Uncertain storage outcomes reconnect under the retained owner lock and look up that binding; unreadable recovery blocks mutation.

Run from the repository root with the existing Windows toolchain:

```powershell
cargo +stable-x86_64-pc-windows-msvc test --locked -p mstrchf-store -- --nocapture
cargo +stable-x86_64-pc-windows-msvc clippy --locked -p mstrchf-store --all-targets -- -D warnings
```

Local cancellation retains terminal outcomes and quarantines live synthetic resource ownership. It supplies no worker-stop evidence and never releases a resource. There is no successful public job producer in B1.
