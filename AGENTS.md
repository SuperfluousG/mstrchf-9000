# MSTRCHF 9000 — project guidance

## Current commission

10 November 2026: version 1.2.0 is live on `main` and in Gregg's artifact (version 3). The next stage moves to Codex: finalise the UI and UX, then prepare a Codex version alongside the Claude one. Read `HANDOVER.md` first; it records what is built, how to verify it and what is still open.

9 November 2026: Gregg commissioned a fresh rebuild ("take anything useful and start from scratch") on branch `v2`, since merged into `main`. Goal: a HelloFresh-style experience catered to his taste. He says what he fancies, it plans meals with lots of recipe choice, groups meals that share ingredients to cut cost and waste, works out the basket, and orders it.

Supersedes the September–October guidance (Codex Sites delivery, `loop/001/` records, AC-01–28, MC-01–13). That material is history only. It is kept on disk in `loop/` and `sites/`, which are git-ignored, and on the branch `archive/codex-2026-10`.

## Working rules

- The repo is https://github.com/SuperfluousG/mstrchf-9000 (private, owned by the `SuperfluousG` account). `main` is the default branch. Make changes on a branch and merge them through a pull request, so CI (`.github/workflows/ci.yml`: validate, build, test) runs first. Branch protection is off because the free plan doesn't offer it for private repos, so this is a convention, not enforced.
- Keep it simple: TypeScript + React + Vite, no native/Rust/WASM layer. The only server is the dev server's `/api/kitchen` file store (`server/kitchenFile.ts`).
- The same build runs as a Claude artifact (`npm run plugin`). Inside an artifact: plain `#name` hashes only, no direct downloads (use `src/platform.ts`), no `confirm()`, and the kitchen lives in the viewer's private `data/users/<id>/kitchen` document.
- Gregg's own planner artifact is https://claude.ai/artifact/65ZQJqG6jJTgia2XFXPw4a. Republish it with the Artifact tool from `plugin/skills/mstrchf-planner/app/index.html` (root that folder, files from `files.json`, capabilities `db`, `user`, `downloads`). Never add `db` rules that open `data/users`.
- Recipe photos are `src/photos/<id>.webp`, made by Codex from `catalogue/PHOTO-BRIEF.md` and imported with `scripts/import_photos.py`. Generate in fixed batches of one image per recipe; never loop or make variants (the v1 build ran unattended over a weekend and filled half a terabyte). Brand assets and their briefs are in `src/brand/`.
- The recipe catalogue is original writing. Do not bulk-copy HelloFresh or other publishers' recipe text or photos; their sites' terms and database rights make that unsafe. Personal import of a single recipe by URL is a possible later feature.
- Diet and allergen facts come from `catalogue/ingredients.json`. Run `npm run validate` after any catalogue change and `npm test` after logic changes.
- Ordering stays human-controlled. An agent may fill the basket at the user's chosen grocer in their own signed-in browser; only the Tesco guide has been tested live. Slot booking needs the user's confirmation in chat. Checkout and payment are always theirs. Never drive Amazon sites: Amazon's Conditions of Use bar AI agents.
- Verify UI changes in a browser at desktop and phone width before calling them done. Test against throwaway data (`MSTRCHF_DATA_DIR=data/test`, launch configuration `mstrchf-dev-test` on http://127.0.0.1:5193) and delete it afterwards. `data/kitchen.json` is Gregg's real household; do not write to it in tests.
- Use command-scoped `git -c safe.directory=*` if Git reports dubious ownership; the repo was created under a different Windows account.
