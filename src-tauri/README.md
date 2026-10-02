# Native shell: Task1 receipt boundary

Normal/default runtime renders the static not-ready page, registers no planner handlers and opens no Store. Release build guards reject synthetic-development and configuration overrides. bundle.active remains false.

The debug development entry selects only local_data_dir()/MSTRCHF9000-Development/loop001-u1-contracts005. It never scans, opens or migrates the contracts003 sibling or normal household root. Tests use unique temporary synthetic roots.

The four IPC commands are planner_snapshot, planner_dispatch, planner_replay_receipt and planner_features. The current bounded parser and shell allowlist admit only SaveHouseholdDraft, SetPrivacy and CancelJob before Store execution. Feature information remains format_version 1 with those three producers and production_runtime_enabled false.

Dispatch returns ShellCommandReceipt, an outbound summary generated from Rust into app/generated/shell.ts. It verifies canonical command bytes, both original-byte digests, identity, revision arithmetic, profile and disposition against the actual Store outcome. Transport request_digest may differ from canonical command_digest. Historical lookup submits the original UTF-8 bytes unchanged to Store::replay_ui; it cannot execute a command and requires a replayed receipt with equal request/command digests. Original historical response graphs never cross IPC. Current state comes only from planner_snapshot.

All Store calls run in blocking tasks after wrapper window/origin checks. The AppManifest lists exactly four commands; the selected development-main local capability grants their four named permissions. Production not-ready capability remains empty. No remote origin, blanket permission, root selector, reader selector or fixture API is exposed.

## Verification route

Use installed direct Rust 1.94.0-x86_64-pc-windows-msvc cargo/rustc/rustdoc only, explicit CARGO/RUSTC/RUSTDOC, exact bin directory first on PATH, RUSTUP_TOOLCHAIN exact, RUSTUP_AUTO_INSTALL=0 and both primary compiler wrapper environment values explicitly empty. Use --locked --offline and an isolated target. Do not install or resolve through stable/proxies.

- shell-types with --no-default-features generates DTO declarations without Store/Tauri runtime; -- --check verifies parity.
- Shell --no-default-features --features store-boundary --lib tests use real synthetic Stores and frozen literal expectations.
- Shell --features native-tests --lib additionally tests actual AppManifest, capabilities and registered wrappers with MockRuntime. autotests=false; fixture and adapter modules are explicitly registered with cfg(test). Historical wrapper reopen checks use child test processes to release MockRuntime-held state normally.
- Store-owned commit-return and recovery-only faults remain private Store tests. Shell fixtures neither expose nor simulate those private hooks.

The exact Tauri family and domain pins are retained in the deliberately unioned lock. Package identity retention does not imply graph equivalence; checkpoint evidence records edges, features and declaration limits.

Task1 deliberately leaves the U1 client unchanged. Its dispatch return type, recovery handling, contracts003 examples and release-marker script require the root-owned Task2 checkpoint before joined UI checks can pass. Tests do not qualify actual WebView operation, accessibility, app launch, release or product acceptance. The previous desktop launch approval timeout remains unresolved.
