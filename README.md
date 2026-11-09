# MSTRCHF 9000

A HelloFresh-style weekly meal planner that is tuned to your taste and hands the shopping to Claude.

Tell it who's eating and what you like. Each week, type what you fancy ("something spicy, loads of peppers, maybe Thai"). It picks meals from 170 recipes, groups meals that share fresh ingredients so packs get finished, and writes a pack-rounded shopping list. Claude can then fill your Tesco basket in your browser, stopping before slot booking and checkout.

## Run it

```bash
npm install
npm run dev        # http://127.0.0.1:5190
npm test           # planner, shopping and date tests
npm run validate   # checks every recipe against the ingredient list
npm run build      # static site in dist/
```

Everything is stored in the browser (`localStorage`). Back it up from **Kitchen → Download backup**.

## How it fits together

| Path | What it is |
|---|---|
| `catalogue/ingredients.json` | 226 ingredients: aisle, unit, typical UK pack sizes and estimated prices, Tesco search phrase, allergens, diet kind, colour. |
| `catalogue/recipes/*.json` | 170 original recipes (126 dinners, 25 lunches, 19 breakfasts). Rules in `catalogue/AUTHORING.md`. |
| `src/domain/planner.ts` | Taste scoring (cuisine, protein, heat, time, favourites, ratings, recency, this week's craving) and greedy week selection that rewards shared perishables and penalises repeated cuisines and proteins. Also swaps and craving parsing. |
| `src/domain/shopping.ts` | Merges quantities, picks the cheapest covering packs, estimates waste and the saving from sharing ingredients. |
| `src/domain/basket.ts` | Basket file, plain list and the "Order with Claude" instructions. |
| `src/ui/` | Week, Menu, Shop and Kitchen screens. |
| `agent/tesco-basket/SKILL.md` | Claude skill for filling the Tesco basket from the basket file. |

Diet and allergen labels are derived from ingredients, never hand-labelled. Prices are rough estimates for planning, not live Tesco prices.

## Images

21 recipes have AI-generated photos in `public/recipes/`. The rest use a generated plate illustration built from the recipe's own ingredients until photos are added. Add a photo by dropping `<name>.webp` into `public/recipes/` and setting `"image": "<name>.webp"` on the recipe.

## History

v1 (September–October 2026) was a Rust/WASM native and ChatGPT Sites build. It is preserved on the `archive/codex-2026-10` branch. The Sites source snapshot is on the `snapshot-2026-10-30` branch of `sites/mstrchf-9000` (kept on disk, git-ignored here).
