# mstrchf-contracts

P02a defines the version 1 wire boundary. It contains no store, shell, provider integration, food engine, UI or account access.

Use `parse_document::<T>(bytes)` for untrusted JSON. It checks byte/depth limits and duplicate keys before typed shape/reference validation. Plain `serde_json::from_*` is not the complete admission path. Required nullable values use `Nullable<T>`; arrays use `BoundedVec<T, MIN, MAX>`. Public Rust fields permit locally constructed records, so call the applicable validator before treating a constructed value as admitted.

`validate_context`, `validate_result_binding`, `validate_outbound`, `validate_read_binding`, `authorize_command` and the operation/recovery predicates are pure checks. Caller classes and recovery facts must come from the trusted application boundary. Passing these functions does not establish real-world truth, provider qualification, persisted generation, worker termination, approval provenance or transactionality. Imported capability and evidence claims are not trusted decisions.

An outbound artefact contains one strict JobPayload fragment. It has the same variant as its containing request, stays within its scope, is independently subject to privacy rules, and must match the declared ID, byte length and SHA-256. Every declaration and outgoing fragment participates in exactly one match. Source-text provenance and the actual preview/launch boundary belong to P06.

The Rust crate generates 19 JSON Schema 2020-12 roots and `app/generated/contracts.ts`. These files are checked in. TypeScript represents brands, required nullable fields and tagged unions; it cannot enforce runtime bounds, uniqueness, reduced rational arithmetic, contextual references, digests or authority. JSON Schema covers representable shape constraints; Rust supplies semantic checks. The explicit ts-rs warnings about ignored Serde attributes are retained; fixture parity tests verify the actual generated shapes.

With Rust 1.94.0 and Node 24, use:

```powershell
cargo +stable-x86_64-pc-windows-msvc test --locked -p mstrchf-contracts
cargo +stable-x86_64-pc-windows-msvc run --locked -p mstrchf-contracts --bin generate-contracts
npm.cmd run check:contracts
```

Generator without arguments (or with `--write`) writes outputs; `--check` compares exact bytes and file set without rewriting. `tests/fixtures/contracts/manifest.json` names executable fixtures; `required-case-ids.json` pins all 90 foundation rows. Runtime transactions, backup/restore, planning/food decisions, external launch/stop, retail operations and calendar publication remain the separately assigned P02b/c, P03, P04, P06, P08 and P09 obligations.
