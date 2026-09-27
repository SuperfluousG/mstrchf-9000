# MSTRCHF 9000 project execution

Read workspace `CLAUDE.md` and shared-agent rules first. Product spec and reviewed implementation plan remain at `Cowork/Outputs/Weekly-Shop/`; the adopted product name is MSTRCHF 9000. Root orchestrator owns canonical execution plan `loop/001/plan.md`, `progress.md`, contract changes, integration and acceptance routing.

Work only within the explicit package/file ownership in the dispatch. Read-only access to shared code is allowed. Do not edit another worker's files, global dependency manifests, frozen contracts or migration numbering without root authorisation. Root integrates commits; a worker explicitly assigned its own isolated branch may commit only its authorised files there. Workers must report changed files, exact commands/counts/results, unresolved concerns and actual capabilities. Do not infer completion from a scaffold or mocks.

User explicitly requested new Codex chats for tasks and authorised the root orchestrator to coordinate them through completion. Workers do not create more user-owned chats. Root handles Asana and canonical loop updates; each chat has separate Codex session records. Workers may use bounded internal research helpers where useful, but no duplicate reviewer for the same package.

Production UI remains gated by the workspace design playbook. Design artefacts and disposable prototypes live under `loop/001/design/`; unseen approvals must never be inferred. Invisible backend/content work can proceed under root dispatch. Actual account basket/calendar mutations and publication await their specific authority. No checkout/payment automation.

Keep private loop/account/session evidence out of distribution. Runtime household data belongs in local app data, not this synced repository. No credentials or browser cookies in logs or fixtures. Use synthetic test records. No access to Legacy or active-excluded Freya. Never directly delete workspace files; honour workspace recovery rules.

Executable/app slug: `mstrchf-9000`; Rust packages: `mstrchf-*`; runtime directory: `%LOCALAPPDATA%/MSTRCHF9000`. Root adopted these names before code. Historical `weekly-shop-*` check examples in the reviewed plan map to the corresponding `mstrchf-*` packages.
