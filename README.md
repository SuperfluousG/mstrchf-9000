# MSTRCHF 9000

A weekly meal planner tuned to your taste, with Claude and ChatGPT desktop editions.

Tell it who's eating and what you like. Each week, type what you fancy ("something spicy, loads of peppers, maybe Thai"). It picks meals from 390 recipes, groups meals that share fresh ingredients so packs get finished, and writes a pack-rounded shopping list. Claude or Codex can then help fill your supermarket basket in your browser (UK, US or Canada), stopping before slot booking and checkout.

## Run it

```bash
npm install
npm run dev        # http://127.0.0.1:5190
npm test           # planner, shopping and date tests
npm run validate   # checks every recipe against the ingredient list
npm run build      # static site in dist/
npm run plugin     # artifact page + recipes for the plugin, packed to release/*.plugin
npm run build:codex # bundled MCP App and local stdio server in codex-plugin/
npm run pack:codex  # portable plugin ZIP and local installer bundle in release/
```

## ChatGPT desktop — developer setup

Build the Codex package, then register this checkout's local marketplace and install:

```sh
codex plugin marketplace add "path/to/MSTRCHF-9000"
codex plugin add mstrchf-9000@mstrchf-local
```

Open a new local ChatGPT or Codex chat and ask **Open my MSTRCHF 9000 meal planner**. The downloadable installer bundle has its own local marketplace; see `codex-plugin/README.md` for that installation, storage paths and restore instructions. No separate API key or MSTRCHF account is needed. Node.js 20 or newer is required.

`node scripts/smoke-codex.mjs` checks the packaged stdio server with temporary data. `node scripts/preview-codex.mjs` serves a development MCP Apps host on port 5197 with an owned temporary kitchen; `--test-fixture` copies `data/test/kitchen.json` into it. That SDK preview verifies the bridge and UI, and does not establish acceptance in the actual Codex desktop host.

## Where your kitchen is saved

| How it runs | Saved to |
|---|---|
| `npm run dev` on this computer | `data/kitchen.json` (git-ignored) with daily copies in `data/backups/`, plus the browser |
| As a Claude artifact | The artifact's storage at `data/users/<your id>/kitchen`, private to you, plus the browser |
| As the local Codex plugin | The user's MSTRCHF9000 application-data directory, outside the plugin installation, with daily backups; the view also keeps a browser copy |
| Any other static copy | The browser only |

Back up from **Kitchen → Download backup** wherever it runs.

## Sharing it

`plugin/` is a Claude plugin. `npm run plugin` rebuilds it and packs `release/mstrchf-9000-<version>.plugin`. It has three skills:

- `mstrchf-planner` publishes the user's own copy of the planner as a Claude artifact (from `plugin/skills/mstrchf-planner/app/`), opens it and reads their week.
- `grocery-basket` fills the basket at the user's online grocer (UK, US or Canada) from the planner's order instructions.
- `meal-calendar` books the week's meals into a calendar, using `plugin/skills/meal-calendar/recipes.json` (UK), `recipes-us.json` or `recipes-ca.json`.

Each person should publish their own copy: on someone else's artifact, people outside the owner's organisation can view but not save.

## How it fits together

| Path | What it is |
|---|---|
| `catalogue/ingredients.json` | 260 ingredients: aisle, unit, typical UK pack sizes and estimated prices, supermarket search phrase, allergens, diet kind, colour. |
| `catalogue/markets/us.json`, `ca.json` | The same ingredients for the US and Canada: local names where they differ, pack sizes and estimated prices in dollars. |
| `catalogue/terms.json` | British food and kitchen words in recipe text and their American equivalents (courgette → zucchini). |
| `catalogue/recipes/*.json` | 390 original recipes across 23 cuisines (294 dinners, 54 lunches, 42 breakfasts), written in British English with placeholders for the words and measures that change. Rules in `catalogue/AUTHORING.md`. |
| `src/domain/market.ts`, `src/domain/localize.ts` | Countries, supermarkets, and turning the British source into each country's words, measures, packs and prices. |
| `src/domain/planner.ts` | Taste scoring (cuisine, protein, heat, time, favourites, ratings, recency, this week's craving) and greedy week selection that rewards shared perishables and penalises repeated cuisines and proteins. Also swaps and craving parsing. |
| `src/domain/shopping.ts` | Merges quantities, picks the cheapest covering packs, estimates waste and the saving from sharing ingredients. |
| `src/domain/basket.ts` | Basket file, plain list and the "Order with Claude" instructions. |
| `src/ui/` | Week, Menu, Shop and Kitchen screens. |
| `src/state/fileSync.ts` | Saves to the host's kitchen store: Claude storage, local development file or Codex MCP. |
| `src/platform.ts`, `src/codexHost.ts` | Host-aware downloads, copying and MCP Apps connection. |
| `plugin/` | The Claude plugin: planner, grocery basket and calendar skills. |
| `codex-plugin/`, `server/codex*.ts` | The Codex package and local stdio MCP implementation. |
| `scripts/build-artifact.mjs`, `scripts/export-recipes.mjs` | Build the artifact page and the recipe export for the plugin. |

Diet and allergen labels are derived from ingredients, never hand-labelled. Prices are rough estimates for planning, not live supermarket prices.

## Countries and shops

The app is British first and also works in the US and Canada. Kitchen → Where you shop sets the country and supermarket; before that, the country is guessed from the browser language (en-US → US, en-CA or fr-CA → Canada, anything else → UK).

| | UK | US | Canada |
|---|---|---|---|
| Words | courgette, coriander, spring onions | zucchini, cilantro, green onions | as the US |
| Recipe measures | g, ml, °C with fan | oz, cups, °F | as the US |
| Shop packs and money | 500 g, £ | 1 lb, $ | 454 g, $ (CAD) |
| Listed shops | Tesco, Sainsbury's, Asda, Ocado, Morrisons | Walmart, Kroger, Target, Albertsons | PC Express, Walmart Canada, Voilà, Metro |

"Another shop" works anywhere: the basket goes to Claude without a shop-specific guide. US and Canadian prices are estimates written for planning, not looked up.

## Look and images

The selected identity is Countercut, the first direction in `src/brand/logo-directions/`. It appears in the app header, favicon, Codex plugin and promotional site. Prior bowl artwork is retained in `src/brand/`; usage and material provenance are in `BRAND-NOTES.md`. Colours, Space Grotesk and weathered oxblood are Freya 2.0.

Every recipe has a photo at `src/photos/<recipe id>.webp`, generated by Codex from `catalogue/PHOTO-BRIEF.md` in one house style: window light, handmade stoneware, the same walnut table. A recipe without a photo falls back to a plate drawn from its ingredients. To add photos for new recipes, have Codex follow the brief, then run `python scripts/import_photos.py <folder>`; it crops to 3:2, saves 960×640 WebP and skips existing photos unless `--force` is given.

## History

v1 (September–October 2026) was a Rust/WASM native and ChatGPT Sites build. It is preserved on the `archive/codex-2026-10` branch. The Sites source snapshot is on the `snapshot-2026-10-30` branch of `sites/mstrchf-9000` (kept on disk, git-ignored here).
