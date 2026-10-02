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

## Private backup foundation (awaiting independent source review)

Startup now requires a fixed local NTFS volume, retained no-delete ancestor handles,
no reparse points and single-link regular leaves. Root custody precedes lock-file
creation. Existing data is recognised either through sealed existing WAL/SHM or
through a new sealed inspection copy when **all** journal sidecars are absent.
Only that private copy uses SQLite immutable mode. Any rollback journal, partial
WAL/SHM pair or unreadable safe-reader state refuses without writable retry.
Ordinary clean closure and retained-sidecar process crashes have separate positive
fixtures. Recognition is followed by original-file logical revalidation.

`audit.rs` checks every B1 table/row, original receipt bytes, projections, ledger,
integrity and foreign keys. Production B1 has an explicitly empty asset closure.
`backup.rs` implements SQLite Online Backup with a pinned source read transaction,
bounded stepping, destination close/flush/seal and independent full audit. Its
private non-cloneable receipt retains protected database/manifest/asset handles;
persisted `verified.json` is evidence only and cannot recreate authority.

Production catalogue remains migration001. `upgrade_fixture.rs` and its literal
schema1→2 catalogue are compiled only for unit tests: a consumed verified receipt
precedes the single exclusive transaction, and uncertain commit compares exact old
or intended-new contents after guarded reopen. Neither a successful backup nor a
test migration enables a production producer, export, restore or migration005.
The retained old/005 reader boundary remains a separate dependency.

Limits: database/WAL 1GiB, SHM 32MiB, individual asset 32MiB, total assets 1GiB,
10,000 assets, manifest 2MiB, existing BLOB 8MiB; 64KiB streaming buffers, 128-page
backup steps, three 25ms busy retries and a cooperative 120-second phase budget.
Blocking OS calls are not hard-cancelled. Failed attempts and inspection copies
remain on disk; repeated starts can retain another source-sized inspection copy.
No pruning or retention optimisation is implemented. Asset tests use 21 literal
bytes named `.png` to prove path/copy mechanics, not media decoding or rights.

BU01–BU24, the 15 kill barriers and SR01–SR13 remain distinct requirements/groups,
not executed test counts. The implementation evidence and remaining gaps are in
the coordinating checkout's `loop/001/backup-upgrade-implementation/`. Process kills
do not establish power-cut, torn-write or disk-firmware guarantees. The observed
fresh-crash VFS path uses heap index reconstruction; read-only SHM mapping remains
a separately unobserved branch. Full loop001 and product acceptance remain open.
