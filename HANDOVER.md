# MSTRCHF 9000 handover: Claude Code to Codex

## ChatGPT final review changes — 11 November 2026

Owner: Codex, branch `feat/chatgpt-final-review`. Gregg requested matching the current promotional Site red, using the Countercut M for sidebar pinning, retailer-aware shopping copy and a direct **Order with ChatGPT** handoff.

- Desktop edition **0.1.3** uses Site tokens `#741F29` (warm oxblood), `#932431` (red) and the same weathered-header blend. The texture file is byte-identical to the promotional Site asset. Shared React source is updated; the Claude artifact/package has not been rebuilt or republished.
- The MCP server now advertises the M icon in `serverInfo.icons`. This is the documented sidebar fallback for a local server, separate from the existing package `logo`/`composerIcon` declarations. The actual MCP handshake returns the bundled SVG. Actual sidebar pin rendering remains unverified.
- The desktop shopping button sends the reviewed retailer, week, people, search phrases and pack counts through MCP Apps `ui/message` to its associated conversation. It guards double clicks and acknowledges only accepted messages. On rejection it displays a failure message and offers copying as a fallback. The phone action uses the same handoff. Browser/Claude copies retain their manual handoff.
- The handoff and desktop grocery skill tell ChatGPT to open the selected retailer first, reuse an existing grocery tab where available, request user login only if needed, then resume. Chrome is not required. Delivery-slot confirmation and user checkout remain in place. Tesco, Morrisons, Ocado and an unspecified retailer are covered by the handoff test; the actual UI change from Tesco to Morrisons and both messages were observed in the SDK host.
- Verification: TypeScript check, **72 tests in 9 files**, desktop build, packaged stdio smoke (10 tools, 8 planned meals), package integrity and `git diff --check` passed. SDK-host desktop and 375 × 812 checks observed successful and rejected handoffs with disposable data; no supermarket was accessed or basket filled. Windows sandbox temporary-file renames failed initially; tests and smoke passed outside that restriction with disposable kitchens.
- Saved private personal release: `plugins_6af381bae888819197337453ced425bc`, release `pluginrel_6af43d34bf7c819189370a269323b353`, version 0.1.3. Readback confirms the version, warm palette, icon and updated skill; scope USER and discoverability PRIVATE are preserved. The partial update also preserves account author/developer metadata, MCP configuration, other skills and unchanged assets. Full downloadable local package: `release/mstrchf-9000-chatgpt-0.1.3.zip`.
- The verified local marketplace installation `mstrchf-9000@mstrchf-local` is enabled at 0.1.3. The open native panel still reports the old palette after reopening via the existing tool connection; current-session tools remain attached to the old runtime. An attempted refresh via `created-by-me-remote` was rejected by automatic approval review because that marketplace could not be verified. A read-only catalogue check returned no entries, so it was not retried. No installed cache was edited.

Next action: load the new plugin runtime in a fresh/restarted desktop session, observe the sidebar M and exercise the real button-to-chat/browser journey with Gregg. SDK-host acceptance is not proof of a real ChatGPT browser run. No public release, Site update or live grocery order was performed in this review change.

## Claude install: one-step open — 11 November 2026

Owner: Claude Code. Gregg tested 1.3.0 on a second Claude account. "Open MSTRCHF 9000" found no artifact and stopped to ask. The skill also required uploading 401 files (about 40 MB) from the plugin folder, which a fresh Claude session must first copy to its scratchpad. The Artifact tool also requires Claude to read every file it did not write before publishing it, and a fresh session cannot read a 1 MB minified script and 394 photos. Plugin 1.3.1 (this branch) changes setup as follows. This supersedes "Set up MSTRCHF 9000 for me" in the section below.

**Source artifact for installs.** https://claude.ai/artifact/VjcD3g9juBh6jUBcD9wXvQ ("MSTRCHF 9000 Source", on Gregg's account) holds the 1.3.0 build's 401 files and a short page explaining itself. It has no capabilities and stores nothing.
- Gregg must set its sharing to "Anyone with the link" in the page's Share menu. Users outside his organisation cannot copy from it until he does, and setup then falls back to uploading from disk.
- It is additive. When a new Claude build ships, publish the new build's files to it, but do not remove old ones while older plugin versions are installed. Their `setup.json` still names the old paths.
- It is not a planner, and nobody should be sent to it.

**How setup works now.** `scripts/setup-map.mjs`, now part of `npm run plugin`, writes `plugin/skills/mstrchf-planner/app/setup.json`. It holds two `files` maps of 201 and 200 entries, each of the form `{"artifact": <source>, "path": <path>}`. When there is no planner, the skill:
1. copies and reads only `app/index.html` (471 bytes);
2. publishes it twice with those maps, so the server copies everything else;
3. does all this without asking first.

An update publishes only the missing paths and `null`s stale ones. If the source cannot be opened, the skill falls back to uploading from disk. The artifact listing now uses `limit: 200`. The README tells users to say "Open MSTRCHF 9000".

**Verified here:** copying files from one artifact into a new one, on Gregg's own account. **Not verified:**
- whether a user on another account can copy from the source once it is shared by link;
- whether that user's Claude still feels bound to read the copied files;
- the full install on a clean second account. Gregg's re-test settles these.

**Still slow, and the next fix.** Each install makes Claude write out about 400 copy entries, roughly 76 KB, which takes a minute or two. The fix is fewer files. The app could load photos from a few packed files instead of 394 separate ones, which brings setup to one publish of about ten entries. That change touches `src/data.ts`, where photos are a single `import.meta.glob`, and the artifact build. It needs browser checks at desktop and phone width. It is not started.

## Claude plugin directory repo — 10 November 2026

Owner: Claude Code. This section records the Claude edition's public release route. It updates the Claude statements in the section below.

**Public repo.** https://github.com/SuperfluousG/mstrchf-9000-claude is public, owned by `SuperfluousG`, branch `main`, first commit `abdfe1e`.
- It is a copy of `plugin/` at private `main` `616abcb` (PR #9), with `plugin/` as the repo root: 428 files, plus a `.gitignore`.
- It contains the built planner app, recipes, photos, the three skills, `LICENSE` and `README.md`.
- Source, scripts, tests and catalogue authoring files stay in this private repo.
- Gregg uses this repo to submit to Anthropic's plugin directory. It is also the "add by name" install route.

**Licence.** `plugin/LICENSE` (PR #9), © 2026 Bayes Ltd.
- Free to use for yourself.
- No redistribution, modified versions, or reuse of the recipes, photos or artwork without written permission.
- Allergen and price caveats, no warranty, law of England and Wales.
- Claude wrote it in plain English; no lawyer has reviewed it.
- GitHub reports it as `NOASSERTION`, because it is a custom licence.

This resolves the "Licence" item under "Open before a public release" below.

**Validation.** `claude plugin validate . --profile directory --strict`, run in the repo root with Claude Code 2.1.322, passed with nothing to report. The directory also runs a safety scan and a human policy review, which this command does not.

**Install routes.** These are documented in `plugin/README.md`; plans other than paid are out of scope:
1. Download `release/mstrchf-9000-1.3.0.plugin`, then Customize → Plugins → Add → Upload plugin.
2. Customize → Plugins → Add → Add marketplace, entering `SuperfluousG/mstrchf-9000-claude`.
3. In Claude Code: `claude plugin marketplace add SuperfluousG/mstrchf-9000-claude`, then `claude plugin install mstrchf-9000@mstrchf-9000`.

After any of these, the user says "Open MSTRCHF 9000" (from 1.3.1; see the section above). Claude then publishes that user's own planner artifact, which only its owner can save to. Never point users at Gregg's artifact, where outside viewers cannot save.

**Directory submission.** Gregg submits it himself at https://claude.ai/directory/manage → Submit new → Plugin bundle → `SuperfluousG/mstrchf-9000-claude`. Anthropic states no review time. Claude has not submitted anything.

**Action for the Site: rebuild the Claude download.**
- `release/mstrchf-9000-1.3.0.plugin` was repacked with `LICENSE` and the updated README. It is now 39,784,344 bytes, SHA-256 `52054d1c8788969a7ce43a55fd98458c6fcb091c8fe2c8d2d5ac09a84b7da756`.
- The deployed Site's `dist/downloads/claude.json` still lists the earlier file: 39,783,115 bytes, `41bab5fafda7a45f7497f0763ee24fa649c04e455953a3e699ee1fc911e4ec55`.
- To fix it, rerun `../MSTRCHF-9000-site/package-downloads.py`, check that the joined browser download matches the new hash, and redeploy.
- The app and skills inside are unchanged: version 1.3.0, the same build as Gregg's artifact version 4.

**Keeping the public repo current.** It is a manual copy, not a mirror. For each Claude release:
1. Rebuild with `npm run plugin`.
   - `main` now includes the desktop-edition changes to shared `src/` from PR #8.
   - The Claude artifact build in `plugin/skills/mstrchf-planner/app/` predates them.
   - So test the rebuilt artifact before release, and ask a Claude session to republish Gregg's artifact.
2. Bump `plugin/.claude-plugin/plugin.json`.
3. Replace the public repo's contents with `git archive <commit>:plugin`.
4. Run the directory-profile validation above.
5. Push as `SuperfluousG`. The directory re-checks each version.

**Not yet verified.** Two things have not been run on a second paid account:
- installing from the file or by adding the repo name;
- "Set up MSTRCHF 9000 for me" completing, including the two-part artifact publish.

## Active desktop plugin and site — 10 November 2026

Owner: Codex, branch `feat/codex-plugin-and-site`, integrated with Claude release main `04d6c36`.

Gregg accepted the UI and selected **Countercut**, the first logo direction. His subsequent instructions supersede the deferred Codex-planning stage below: build the desktop plugin and promotional Site now, present **Claude and ChatGPT desktop** equally, provide setup without human command-line use, and state plainly that this is a **free beta**, still being developed. It uses the intelligence in the user's existing AI subscription; there are no additional MSTRCHF fees. The user buys their own groceries. Desktop-only support is explicitly accepted.

Current implementation:

- `codex-plugin/` is the ChatGPT desktop edition, version **0.1.2**, beside the unchanged Claude `plugin/` version **1.3.0**. Internal Codex script/file names are retained for development. Both the portable manifest and compatibility overlay include the selected icon, desktop-execution requirement and beta description.
- The shared React planner is an MCP App, opened through a global `open_planner` entrypoint with fullscreen metadata. Ten tools provide the planner, kitchen reads, revision-guarded edits/restore, photo reads, bounded exports, planning, weeks, baskets and scaled recipes. No AI API call or paid service is introduced by the server.
- The local stdio server saves outside plugin caches, normally `%LOCALAPPDATA%/MSTRCHF9000/kitchen.json`, with atomic replacement, a file lock, optimistic revisions and 14 daily backups. Ordered records are protected. An explicit newer reset defeats stale browser recovery in this edition. Model tool results update the view without overwriting a pending UI save; teardown awaits pending/in-flight saves.
- Photos load lazily through MCP resources, with an app-only tool fallback. Fonts, styling and brand material are embedded in the MCP page. Normal browser/Claude paths keep their hashes, storage and host-specific download behaviour; the MCP SDK is loaded only in the desktop view. Release the UI with a new `DESKTOP_VERSION` and matching manifest version so its resource URI changes; the stdio smoke check enforces this.
- The desktop package is installed and enabled locally through marketplace `mstrchf-local`. A native private **personal** plugin is also saved: [MSTRCHF 9000](https://chatgpt.com/plugins/plugins_6af381bae888819197337453ced425bc), ID `plugins_6af381bae888819197337453ced425bc`, release `pluginrel_6af38877676c81919f3ed4b18b8f67a6`. This is not a public-directory listing or a cross-account installation proof.
- Downloadable files are `release/mstrchf-9000-chatgpt-0.1.2.zip` and Claude's existing `release/mstrchf-9000-1.3.0.plugin`. The developer marketplace bundle remains available separately. The Claude release was verified and reused, not rebuilt from this branch.
- The promotional source is the separate checkout `../MSTRCHF-9000-site`. Native Site ID: `appgprj_6af372f74bf881919c57df38a740dec7`; URL: https://mstrchf-9000-codex.gregg-bayes-brown.chatgpt.site . Preserve that ID and owner-private audience. The older `mstrchf-9000` Site is a separate retained project. The final dual-version/free-beta preview is deployed: source commit `5f2c0c77d2083cc7879bf9e54b80a74d322ec127`, deployment `appgdep_6af38aa62e6881919c75df4377c2492d`, status succeeded.
- Both desktop-app download links and documented `claude://` / `codex://` app links are included. The latter scheme is the ChatGPT desktop compatibility scheme, not customer-facing Codex branding. Setup links prefill a message; they do not send it or silently install anything. Claude uses custom-plugin upload. ChatGPT's guided setup takes the downloaded ZIP into a new local chat. No terminal commands appear on the Site.
- Site downloads are split into 20 MB delivery parts and joined/checked in the browser, producing one unchanged `.plugin` or ZIP. This avoids the Site source repository's 32 MiB per-object limit and Cloudflare's 25 MiB static-asset limit. Both browser downloads match their release hashes and pass ZIP integrity checks. `package-downloads.py` rebuilds the parts from verified release archives.
- Bayes website hosting was checked read-only: `Website/wrangler.jsonc` uses Cloudflare Workers Static Assets. No Bayes production source, hosting or Drive sharing was changed. The current Site download path works without a separate Drive or object-storage setup.

Verification so far:

- `npm run check`, catalogue validation, production build, **71 tests in 9 files**, packaged stdio smoke test, ZIP validation and dependency audit passed. The existing browser main-chunk warning remains; the MCP SDK is a separate, dormant browser chunk.
- The real bundled stdio server exposed ten tools, planned eight meals, returned the embedded app and a real WebP, and saved/exported only disposable data.
- SDK MCP Apps host checks used an owned temporary kitchen at port 5197: desktop/phone Week, Menu, Shop, Recipe and Kitchen; photos through resources and fallback tool; model-to-view updates; persistent three-person settings after reload; ordered-record recovery; and real backup/basket exports with no phone overflow. Fonts rendered as Space Grotesk with the declared data-font policy. This is SDK-bridge proof, not actual ChatGPT host acceptance.
- Native account creation and tool startup succeeded. The actual ChatGPT app opened, served photos and returned a correctly scaled recipe. A saved native week was observed after app setup; agent checks did not write test data to that kitchen. Version 0.1.2 adds explicit Freya font scoping and a versioned UI resource URI to avoid stale host pages. The latest saved release is verified by metadata; refresh/update acceptance in the active host and sidebar pinning are not yet established. Claude 1.3.0 install/update on a second account is also unverified, as Claude reported. Do not describe either as established acceptance.
- The final Site was checked at 1440 px and 375 px, including all four demo moods, both edition cards and the free/subscription/beta explanation. Both edition downloads were saved in a browser and matched their release hashes. ChatGPT 0.1.2: SHA-256 `abd01b26886c0073a99bac8ed02dd1f65094c487e7f8ae13bd5b6c3780e4f4c0`; Claude 1.3.0: `41bab5fafda7a45f7497f0763ee24fa649c04e455953a3e699ee1fc911e4ec55`.
- Real `data/kitchen.json` retains SHA-256 `36B1222C893B88760D1D3F5CE4AFF584C8E4E35C0A5A4DC28C215DD7575AD06A`. Port 5193 remains Gregg's requested hands-on test-data app; do not remove it until his review ends. Other owned QA servers/temporary kitchens should be stopped/removed after verification.

Commands: `npm run build:codex`, `node scripts/smoke-codex.mjs`, `npm run pack:codex`. For a browser SDK host, `node scripts/preview-codex.mjs` starts an owned temporary kitchen; `--test-fixture` copies `data/test/kitchen.json` into that temporary directory. It never points at the real household.

Integration: [PR #8](https://github.com/SuperfluousG/mstrchf-9000/pull/8) contains this desktop edition. Its CI passed the catalogue validation, production build, 71 tests, desktop bundle and stdio smoke check. Integrate only a passing head revision.

Next action: Gregg's hands-on review of the saved personal plugin and private Site, including updating/reopening the latest desktop view. Public directory submission is separate; OpenAI currently limits local-MCP submissions to allowlisted partners.

The UI refinement and original handover below are retained historical records. Their earlier provisional-logo, deferred-stage and release statements are superseded by this section and Claude's release update.

## UI refinement record — 10 November 2026

Owner: Codex, branch `feat/ui-ux-finalisation`, from merged main `0485543`.

Gregg authorised the walkthrough's six behaviour fixes, the agreed desktop/phone layout improvements, a weathered Freya 2.0 oxblood header and distinct logo alternatives. Preserve the current red/gold visual style and the Claude artifact build. Codex packaging/planning is deferred until the UI is settled.

Current checkpoint: implementation and local verification are complete. [Draft PR #6](https://github.com/SuperfluousG/mstrchf-9000/pull/6) includes the app, recipe changes and rebuilt Claude artifact files; its [Checks tab](https://github.com/SuperfluousG/mstrchf-9000/pull/6/checks) is the current CI record. Gregg's UI acceptance and final logo preference remain open; the live Claude artifact has not been republished.

Update from Claude Code, 10 November 2026, evening:

- **Accepted.** Gregg accepted the UI for release. Countercut is still the header mark.
- **Republished.** Gregg's artifact is now version 4, built from this branch at `8467a1e`; the build was identical to the one Codex committed.
- **Plugin 1.3.0.** The plugin is now version 1.3.0, packed at `release/mstrchf-9000-1.3.0.plugin` (37.9 MB).
- **Planner skill fixes.**
  - First-time setup now publishes the 401 files in two parts, because one publish is limited to 255 files.
  - A new "Update the user's copy" section brings an installed planner up to the current build without touching the user's kitchen.
- **Install instructions.** These are in `plugin/README.md`.
- **Not touched.** `package.json` and `package-lock.json` gained MCP app dependencies from parallel Codex work in this folder. Claude did not touch or commit them.

| Behaviour fix | Result and proof |
|---|---|
| Kept meals bypassed new exclusions | Re-plan rechecks kept meals; conflicts are named and ordering is blocked until repaired. A kept cashew meal was replaced after excluding tree nuts. |
| Ordered weeks could be edited from Menu | All planning entry points guard ordered weeks. A recorded basket freezes names, quantities, market, people and estimates; changing Kitchen does not alter it. Older ordered weeks without a record are labelled transparently. Moving dates remains allowed. |
| A recipe opened from Swap offered Add | Shortlist previews and full-menu browsing retain the exact replacement context and day. Replacing kept the same meal count; full-menu confirmation includes Undo. |
| Shuffle reset kept dates | Kept identities and days survive both re-plan and shuffle. A kept Sunday dinner stayed on Sunday. |
| Kitchen drafts disappeared on navigation | Drafts survive route changes and reload, with explicit Save/Discard and an unsaved header state. Remote save failures have a retry action and honest status. |
| Methods used base quantities | Method measures and ingredient counts scale with servings, while cooking times, temperatures, cut sizes and per-person amounts stay fixed. Cupboard ingredients have quantities. Recipe text no longer assumes two plates or a fixed number of sandwiches or egg wells. |

Other refinements: compact planned-week controls, meals before ingredient-sharing detail, phone week selector, contextual add feedback, folded phone filters, larger touch targets, keyboard-operable radio/tab groups, full-width phone sheets with one active dialog and locked background scroll, shorter onboarding, and a phone Shop layout that puts ordering before the receipt. Pantry staples can be explicitly included. Backup restore validates and previews before replacement; invalid files leave the kitchen intact, and an empty restore carries explicit reset authority.

Verification on throwaway data at port 5193:

- `npm run check`, `npm test` and `npm run build` passed; **62 tests in 8 files**. `npm run validate`: **260 ingredients, 390 recipes, 17 files**, no errors. The existing large JavaScript chunk warning remains (about 1,025 KB, 238 KB gzip).
- Browser checks at **1440 × 1000** and **375 × 812** covered onboarding, empty/planned/ordered Week, swap shortlist and full-menu replacement, Menu filters and empty results, Recipe, draft/recorded Shop, Order with Claude and Kitchen. Keyboard arrows, Escape/focus return, background scroll, hard-exclusion recovery, draft reload, backup preview/cancel/restore and download were exercised. No horizontal overflow was found at these widths.
- UK/Tesco, US/Walmart and Canada/PC Express handoffs were checked. An ordered US estimate of **USD 194.81 for four** stayed unchanged after switching Kitchen to Canada/two and then the UK. These were fixtures, not real orders.
- In the compiled app, a stopped server and then a read-only test file caused a clear save failure. Restoring access and pressing **Try saving again** succeeded; four-person settings survived reload. Misleading secondary “saved” messages were removed.
- Phone first-meal position improved from about 1,550 px to 375 px; the order button moved from about 6,000 px to 540 px. These are layout observations, not performance measurements.
- `node scripts/build-artifact.mjs` produced **402 files / 38.2 MB**; the manifest references were checked. Calendar recipe exports were regenerated for all three countries. No release ZIP was replaced. Actual Claude-host runtime/republication still needs a Claude session after integration; local compatibility checks do not establish live-host acceptance.
- Real `data/kitchen.json` SHA-256 remained `36B1222C893B88760D1D3F5CE4AFF584C8E4E35C0A5A4DC28C215DD7575AD06A`. Browser test kitchens were reset through the app, temporary viewport overrides were reset and review tabs were closed. Test servers were stopped and `data/test` was deleted after preserving the evidence.

Review screenshots: [desktop Week](design/evidence/week-desktop.jpg), [phone Shop](design/evidence/shop-phone.jpg), [save failure](design/evidence/save-failure-phone.jpg), [recovered save](design/evidence/save-recovered-phone.jpg), [logo directions](design/evidence/logo-directions.jpg). The full local capture set and downloaded test backup are outside the repo at `C:/Users/Gregg/.codex/visualizations/2026/11/10/01a1c664-ece5-71e1-a686-f3a9c57f8a2a/mstrchf-refinement/`.

Header material: the actual Freya 2.0 weathered oxblood source; provenance is in `src/brand/BRAND-NOTES.md`. Three native SVG directions are in `src/brand/logo-directions/`, compared by `design/logo-directions.html` (serve through the dev server). Countercut is provisional in the app; Gregg has not selected a final logo. Prior bowl assets are retained. No Freya source was changed.

Next action: review the UI and logo directions with Gregg, then integrate through the PR and arrange Claude artifact republication. Plan the Codex version only once the UI is settled. Recipe cooking validation and a full assistive-technology audit remain outside this UI pass; the original handover's limitations still apply.

The existing record below is the original Claude handover; its release statements describe that handover, not acceptance of this branch.

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

- **Licence.** Resolved 10 November 2026: `plugin/LICENSE`, free use and no redistribution. See "Claude plugin directory repo" at the top.
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
