# MSTRCHF 9000 — project guidance

## Current commission

9 November 2026: Gregg commissioned a fresh rebuild ("take anything useful and start from scratch") on branch `v2`. Goal: a HelloFresh-style experience catered to his taste. He says what he fancies, it plans meals with lots of recipe choice, groups meals that share ingredients to cut cost and waste, works out the basket, and orders it.

Supersedes the September–October guidance (Codex Sites delivery, `loop/001/` records, AC-01–28, MC-01–13). That material is history only. It is kept on disk in `loop/` and `sites/`, which are git-ignored, and on the branch `archive/codex-2026-10`.

## Working rules

- The repo is https://github.com/SuperfluousG/mstrchf-9000 (private, owned by the `SuperfluousG` account). `main` is the default branch. Make changes on a branch and merge them through a pull request, so CI (`.github/workflows/ci.yml`: validate, build, test) runs first. Branch protection is off because the free plan doesn't offer it for private repos, so this is a convention, not enforced.
- Keep it simple: TypeScript + React + Vite, no native/Rust/WASM layer. The only server is the dev server's `/api/kitchen` file store (`server/kitchenFile.ts`).
- The same build runs as a Claude artifact (`npm run plugin`). Inside an artifact: plain `#name` hashes only, no direct downloads (use `src/platform.ts`), no `confirm()`, and the kitchen lives in the viewer's private `data/users/<id>/kitchen` document.
- Gregg's own planner artifact is https://claude.ai/artifact/65ZQJqG6jJTgia2XFXPw4a. Republish it with the Artifact tool from `plugin/skills/mstrchf-planner/app/index.html` (root that folder, files from `files.json`, capabilities `db`, `user`, `downloads`). Never add `db` rules that open `data/users`.
- Recipe photos are `src/photos/<id>.webp`, made by Codex from `catalogue/PHOTO-BRIEF.md` and imported with `scripts/import_photos.py`. Generate in fixed batches of one image per recipe; never loop or make variants (the v1 build ran unattended over a weekend and filled half a terabyte). Brand assets and their briefs are in `src/brand/`.
- The recipe catalogue is original writing. Do not bulk-copy HelloFresh or other publishers' recipe text or photos; their sites' terms and database rights make that unsafe. Personal import of a single recipe by URL is a possible later feature.
- Diet and allergen facts come from `catalogue/ingredients.json`. Run `npm run validate` after any catalogue change and `npm test` after logic changes.
- Ordering stays human-controlled. Claude may fill the Tesco basket in Gregg's own signed-in browser. Slot booking needs his confirmation in chat. Checkout and payment are always his.
- Verify UI changes in a browser at desktop and phone width before calling them done.
- Use command-scoped `git -c safe.directory=*` if Git reports dubious ownership; the repo was created under a different Windows account.
