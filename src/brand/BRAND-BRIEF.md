# MSTRCHF 9000 brand kit

MSTRCHF 9000 is a personal meal planner: it plans a week of dinners and lunches to your taste, writes the shopping list, and hands the shopping to an AI assistant. The name is MasterChef with the vowels taken out, plus "9000" as in a slightly sinister robot chef (think HAL 9000). The app currently looks generic and boxy. This kit gives it a recognisable identity. The attached screenshot shows the current app.

Write everything into `out/brand/`. Nothing else.

## Fixed palette (Freya v2; use only these)

| Name | Hex | Use |
|---|---|---|
| Oxblood | #4A1F2B | Dark ground, header band |
| Cardinal | #9B1B26 | Primary red, buttons |
| Gold | #D7B461 | Accent on dark |
| Bright gold | #E2C779 | Highlights on oxblood |
| Deep gold | #8A641F | Gold that reads on light grounds |
| Chalk | #EEEDE5 | Light ground |
| Ink | #2A1517 | Text |

No blue and no green anywhere in the brand assets. No capitals-only text except the name itself, which is always written MSTRCHF 9000. Typeface: Space Grotesk (weights 400, 500, 600).

## 1. The mark: `logo-mark.svg`

A stoneware bowl seen from directly above: a gold rim (the bowl's lip), an oxblood glaze inside it that deepens towards the centre, and one small soft highlight off centre, like light catching the glaze. Seen small, it should also read as a single red machine eye with a gold ring: that double reading is the whole idea, so keep it subtle, not a cartoon eye. Flat vector shapes; one radial gradient in the glaze at most. Must stay recognisable at 16 px. Square viewBox, transparent background.

## 2. Lockups

- `logo-lockup-dark.svg`: the mark, then "MSTRCHF" in Space Grotesk 600 in chalk with "9000" set smaller in bright gold, for use on oxblood.
- `logo-lockup-light.svg`: the same for chalk backgrounds (ink wordmark, deep-gold 9000).
- Use `<text>` with `font-family="Space Grotesk"`; the app loads the font.

## 3. Icons

- `favicon.svg`: the mark simplified for 16 to 32 px.
- `app-icon.svg`: 1024×1024, the mark centred on an oxblood square with a thin gold line along the bottom edge.
- If you can render SVG to PNG on this machine, also save `app-icon.png` (1024×1024). If not, skip it and say so.

## 4. Photographs (use your image generation tool; one image each)

Same house style as the recipe photos (`BRIEF.md`): soft window light from the left, handmade matte stoneware in cream, oatmeal or oxblood glaze, the same dark walnut table, linen in brick red or ochre, real texture, no text, no hands, no people, no packaging.

- `hero-table.png`, wide 16:9: overhead view of the walnut table with four or five different finished dishes from a week of home cooking (a curry with rice, a pasta, a toastie, a bowl of chilli, a salad), linen and a couple of loose ingredients, with calm empty table on the left third for text.
- `empty-plate.png`, 3:2: an empty cream stoneware plate with a fork and folded brick-red napkin, overhead, lots of walnut around it. Used when a day has no meal planned.
- `market-bag.png`, 3:2: a brown paper grocery bag on the walnut table with fresh produce spilling out (red peppers, onions, chillies, lemons, a bunch of herbs, a loaf), 45 degrees. Used on the shopping list.

## 5. `BRAND-NOTES.md`

A short note: each file, what it is for, minimum sizes, and the clear space around the mark.

## Rules

- One attempt per photograph; retry only if generation fails.
- Never: chef hats, robots, steam swirls, cartoon faces, gradients other than the glaze, drop shadows on the logo.
- When done, reply with the list of files written.
