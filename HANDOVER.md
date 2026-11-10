# MSTRCHF 9000 handover: Claude Code to Codex

10 November 2026. Written by Claude Code at the end of the v2 rebuild, for Codex and Gregg. Read it with `AGENTS.md` (working rules) and `README.md` (layout and commands).

## Where it stands

- `main` is version 1.2.0 at commit `10a55ed`. Pull requests 1–4 are merged and CI (validate, build, test) passed on the last one.
- Gregg's planner artifact is https://claude.ai/artifact/65ZQJqG6jJTgia2XFXPw4a, on version 3, built from this commit. His kitchen data is in the artifact's private storage and was not changed by the release.
- The Claude plugin is `release/mstrchf-9000-1.2.0.plugin` (about 38 MB; `release/` is git-ignored). Rebuild it with `npm run plugin`.
- Checks at handover: `npm run validate` reports 260 ingredients and 390 recipes in 17 files with no errors; `npm test` passes 46 tests in 5 files.

## What was built (9–10 November 2026)

1. **The rebuild.** A React 19, TypeScript and Vite 8 app replaced the v1 Rust/WASM and ChatGPT Sites build, which is preserved on `archive/codex-2026-10`. Screens: onboarding, Week, Menu, Shop, Kitchen and the recipe sheet.
2. **Planning and shopping.** The planner scores recipes against the household's taste and this week's craving, then picks a week that shares fresh ingredients so packs get finished. The shopping list merges quantities, buys the cheapest covering packs and estimates waste and the saving from sharing (`src/domain/planner.ts`, `shopping.ts`).
3. **Catalogue.** 390 original recipes across 23 cuisines (294 dinners, 54 lunches, 42 breakfasts) and 260 ingredients. Diet and allergen labels are derived from ingredients. 220 recipes were added in this run, using BBC Good Food and HelloFresh only for dish ideas, with no text copied. 23 near-duplicates were removed. 19 recipes were upgraded with ingredients Gregg can stock: sage, okra, split peas, curry leaves, wine and blue cheese.
4. **Photos and brand.** Every recipe has a photo (`src/photos/<id>.webp`). Codex made them from `catalogue/PHOTO-BRIEF.md` in fixed batches, and `scripts/import_photos.py` imported them. The bowl mark and the Freya v2 red-and-gold styling are in `src/brand/` and `src/styles.css`.
5. **Three countries.** The UK, US and Canada:
   - Recipes are written once in British English, with placeholders for words and measures that change (`{oven 220}`, `{ml 15}`, `{courgette}`, `{grill|broiler}`). The rules are in `catalogue/AUTHORING.md`.
   - `catalogue/markets/us.json` and `ca.json` hold local names, packs and prices for each country.
   - Kitchen → Where you shop sets the country and supermarket. Before that, the country is guessed from the browser language.
6. **Ordering.** Shop → Order with Claude produces copyable instructions and a basket file (version 2, which names the country and shop). The `grocery-basket` plugin skill has one guide per shop: Tesco, Sainsbury's, Asda, Ocado and Morrisons in the UK; Walmart, Kroger, Target and Albertsons in the US; PC Express, Walmart Canada, Voilà and Metro in Canada. It also has a method for any other shop. Amazon Fresh is deliberately excluded.
7. **Claude plugin 1.2.0.** Three skills:
   - `mstrchf-planner` publishes the user's own artifact copy;
   - `grocery-basket` fills the basket;
   - `meal-calendar` books meals into a calendar, using per-country recipe exports.
8. **Repository.** https://github.com/SuperfluousG/mstrchf-9000 is private, with CI in `.github/workflows/ci.yml`.

## Running and checking

```bash
npm install
npm run dev        # http://127.0.0.1:5190, uses Gregg's data/kitchen.json
npm test
npm run validate
npm run plugin     # rebuilds dist/, the artifact app and release/*.plugin
```

- For UI work, run the `mstrchf-dev-test` launch configuration (`.claude/launch.json`), which serves http://127.0.0.1:5193 with `MSTRCHF_DATA_DIR=data/test`. Delete `data/test` afterwards.
- `mstrchf-artifact-static` (port 5192) serves the built artifact folder as it ships.
- Check every UI change at desktop width and at phone width (375 px).
- Copy and logic changes must work in all three countries. `tests/markets.test.ts` fails if metric measures or British food words leak into American text.

## Next stage 1: finalise the UI and UX

Gregg considers the product close to releasable. This stage settles the interface before any public release. The scope is every screen: onboarding, Week, Menu, the recipe sheet, Shop and its order sheet, and Kitchen.

| Screen | File |
|---|---|
| Shell, navigation, week tabs, save status | `src/App.tsx` |
| Onboarding and Kitchen (household, taste, country and shop, backup) | `src/ui/KitchenView.tsx` |
| Week plan, craving, swaps | `src/ui/WeekView.tsx` |
| Recipe browser | `src/ui/MenuView.tsx`, `src/ui/RecipeSheet.tsx`, `src/ui/DishArt.tsx` |
| Shopping list and Order with Claude | `src/ui/ShopView.tsx` |
| Shared pieces | `src/ui/Sheet.tsx`, `src/ui/bits.tsx`, `src/ui/Mark.tsx` |
| Tokens and all styles | `src/styles.css` (398 lines) |

**Constraints:**

- **Visual system.** It is Freya v2 in red and gold with Space Grotesk; usage is in `src/brand/BRAND-NOTES.md`.
  - Do not edit the Freya source in `Code/Freya/`.
  - Apply the workspace app-design playbook (`C:\Users\Gregg\Bayes Ltd\.bayes\app-design-playbook.md`) in proportion to the size of the change.
- **The artifact build.** The same build runs as a Claude artifact, so it must keep:
  - plain `#name` hash routes;
  - no `confirm()`;
  - downloads and copying through `src/platform.ts`.
- **Copy.** British English, sentence case.

**Facts found in this run that bear on the work:**

- **Photo weight.** The 390 photos are 38 MB of the 40 MB artifact app. The artifact folder holds 402 files, and an artifact version allows 511. Roughly 100 more recipes would need the photos stored another way, for example in the artifact's asset store. Page weight on a phone is also worth measuring.
- **French Canada.** Browsers set to `fr-CA` get the Canadian catalogue, but there is no French text.
- **Prices.** They are planning estimates, and the Shop screen says so.
- **Accessibility.** The repo has no accessibility audit record.

**Republishing.** Codex cannot publish the artifact. After UI changes land on `main`, run `npm run plugin` and ask a Claude session to republish Gregg's artifact, as described in `AGENTS.md`.

## Next stage 2: a Codex version

**Goal.** Codex users get MSTRCHF 9000 the way Claude users get it from the plugin: plan the week, fill the basket and book the calendar.

**Format.** I have not checked Codex's current plugin or skill format. Confirm it from Codex's own documentation before designing the package.

**What carries over and what is tied to Claude:**

| Carries over as is | Tied to Claude |
|---|---|
| `src/domain/*`: planner, shopping, countries, localisation, basket file | `src/platform.ts`: downloads through `window.claude.use('downloads')` |
| `catalogue/*` and `scripts/validate-catalogue.mjs` | `src/state/fileSync.ts`: the Claude-storage branch (the local `/api/kitchen` file branch is host-neutral) |
| `scripts/export-recipes.mjs` and the per-country recipe JSON | `scripts/build-artifact.mjs` and artifact publishing |
| Most of `src/ui/` | Copy that names Claude: `ShopView.tsx` (Order with Claude), `KitchenView.tsx` storage messages, `App.tsx` save status |
| `tests/` | `claudePrompt()` in `src/domain/basket.ts` |
| The retailer guides in `plugin/skills/grocery-basket/retailers/*.md` (plain Markdown) | `plugin/.claude-plugin/plugin.json` and the three `SKILL.md` files, which name Claude tools: the Artifact tool, Claude in Chrome and calendar connectors |

**Questions to settle first:**

1. **Hosting.** Where does the planner run for a Codex user, with no artifacts? The local server with its `data/kitchen.json` store already exists. A static hosted copy would keep data in the browser only.
2. **Storage.** Where is the kitchen saved and backed up?
3. **Browser.** Can Codex drive a browser the user is signed in to, so it can fill a basket? The hard rules in `plugin/skills/grocery-basket/SKILL.md` must carry over unchanged: no passwords, payment or loyalty-card details; no checkout; logins and CAPTCHAs handed to the user; a slot booked only once the user confirms it; and the run stopped on any block.
4. **Calendar.** Which calendar can Codex write to?
5. **Wording.** Should "Order with Claude" become neutral wording, or be set per build?

**Recommendation.** Keep one codebase:
- add a build-time host setting (for example `VITE_HOST=claude|codex`) that switches the platform layer and the copy;
- package the Codex version beside `plugin/`, not as a fork.

## Open before a public release

- **Licence.** There is no LICENSE file, and no licensing decision for the recipes and photos.
- **Versions.** The version numbers disagree: `package.json` says 2.0.0 and the plugin says 1.2.0.
- **Shops.** Only the Tesco guide is tested live (two runs, including a real order Gregg completed himself).
  - The other guides come from a read-only study on 10 November 2026.
  - Albertsons and Metro blocked that study with 403 pages, and Amazon showed an "Access restricted" page that names AI agents.
- **Prices.** US and Canadian prices are written estimates, not looked up.
- **Recipes.** They are AI-written and have not been cooked.
- **Plugin size.** The file is about 38 MB, over the 30 MB limit for files Claude can send to the phone and web apps. It needs a distribution route.
- **Repository.** It is private. Branch protection is off (the free plan), so the PR-and-CI rule is a convention only.

## Housekeeping left alone

These need Gregg's word before anyone acts on them:

- **Merged remote branches.** `feat/markets`, `feat/more-recipes`, `claude/friendly-bhaskara-db32cd` and `bb/ci-setup` are merged but still on GitHub.
- **Local v1 branches.** The `task/*` branches and `build/001` are v1 history.
- **Photo originals.** The 220 full-size PNGs for this run's recipes (about 590 MB) are in Claude's temporary session folder: `C:\Users\Gregg\AppData\Local\Temp\claude\ec9a40aeab176133\e7713860-801c-4141-93e5-2a8e4582be8c\scratchpad\codex-photos\out\recipes`.
  - The WebP copies are already imported.
  - The batch tooling used to make them (`BRIEF.md`, `make-batches.mjs`, `run-batches.sh`) is in the parent folder, not in the repo. Move it into `scripts/` if more recipe photos are planned.
- **Pushing.** The GitHub CLI on this machine has three accounts, and `gregg-cx` is the active one. Only `SuperfluousG` can push to the repo, and git uses whichever account is active.
