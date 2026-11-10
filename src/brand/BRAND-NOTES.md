# MSTRCHF 9000

## UI review additions — 10 November 2026

The active UI refinement uses the original Freya 2.0 weathered oxblood material in the header, with an oxblood veil for text contrast. Source: `Code/.worktrees/freya-v2-production/v2/assets/materials/oxblood.png`; SHA-256 `faa1bf0588072ac9eac4ecbfb28dc5259e251759ed20b9a86adf3cadaa008f28`. `weathered-oxblood.png` is an exact local copy; `weathered-oxblood.webp` is its 169 KB web derivative. Freya source was not edited.

Gregg selected **Countercut**, the first logo direction, on 10 November 2026. It is the app header mark, browser favicon, Codex plugin icon and promotional site identity. `logo-directions/` retains the three editable, dark-ground SVG directions: Countercut, Service docket and Fork current. Compare them at `design/logo-directions.html`. These are native vector designs using Space Grotesk and the existing palette; no generated bitmap logos are involved. Prior bowl marks and lockups are retained below for reference.

The following notes describe the original brand assets, retained for reference.

A stoneware bowl from above; a red machine eye at a glance. The gold lip, dark glaze and single off-centre reflection do the work. Do not add a pupil, face, chef hat, steam, shadows or extra decoration.

## Files and sizes

Minimum sizes below are on-screen display sizes in CSS pixels; keep the original aspect ratio.

| File | Use | Minimum display size |
|---|---|---|
| `logo-mark.svg` | Standalone mark; transparent, square 64-unit viewBox | 16 × 16; prefer 24 × 24 or larger |
| `logo-lockup-dark.svg` | Transparent horizontal lockup for oxblood; chalk wordmark and bright-gold 9000 | 205 × 40; prefer 246 × 48 |
| `logo-lockup-light.svg` | Transparent horizontal lockup for chalk; ink wordmark and deep-gold 9000 | 205 × 40; prefer 246 × 48 |
| `favicon.svg` | Browser icon; thicker lip and simpler glaze | 16 × 16; suitable through 32 × 32 |
| `app-icon.svg` | Editable 1024 × 1024 oxblood app tile; centred mark and 8-unit gold bottom rule | 64 × 64; use the favicon below this |
| `app-icon.png` | 1024 × 1024 PNG rendered from the app-icon SVG | 64 × 64 |
| `hero-table.png` | 1536 × 864, 16:9; five finished dishes, with calm walnut on the left for copy | 640 × 360 |
| `empty-plate.png` | 1536 × 1024, 3:2; empty-day state, cream plate, fork and folded brick-red napkin | 240 × 160 |
| `market-bag.png` | 1536 × 1024, 3:2; shopping-list image, produce and unwrapped loaf | 240 × 160 |
| `BRAND-NOTES.md` | This usage reference | Not applicable |

Use the standalone mark when a full lockup would be too small. Do not stretch, recolour, rotate or crop logo artwork. The app tile has a full square background; let the operating system apply its own corner mask.

## Clear space

Leave at least one quarter of the visible bowl diameter around the mark or the entire lockup, measured from the outermost visible artwork. At the mark's native scale the bowl is 56 units across, so clear space is 14 units. The SVG's four-unit outer padding counts towards this space; add the remainder in the surrounding layout. The gap between bowl and wordmark is already built in. The app-icon tile has its own fixed spacing.

## Colour and type

Use only the Freya v2 palette for vector artwork: oxblood `#4A1F2B`, cardinal `#9B1B26`, gold `#D7B461`, bright gold `#E2C779`, deep gold `#8A641F`, chalk `#EEEDE5`, ink `#2A1517`. Gold and bright gold are accents on dark grounds; deep gold is the text accent on chalk. The glaze is the only gradient. No blue or green.

Space Grotesk: 400 for body copy, 500 for supporting emphasis, 600 for headings and the main wordmark. The SVG lockups retain live `<text font-family="Space Grotesk">`; ensure the rendering context loads the font. The smaller 9000 uses weight 500. Write the name as MSTRCHF 9000; use sentence case elsewhere.

## Photography

All three photos were generated with the built-in image tool, one attempt each. Shared direction: real home-cooked texture, the same dark warm walnut, soft window light from the left, handmade matte cream/oatmeal/oxblood ceramics where relevant, and brick-red or ochre linen. No text, people, hands, branded packaging or steam effects. Natural wood, paper, food and linen retain their material colours rather than being reduced to flat vector swatches.

The hero prompt specified curry with rice, tomato pasta, a cheese toastie, bean chilli and a red-leaf salad, arranged on the right with the left third clear. The empty-state prompt specified only an empty cream plate, one fork and a folded brick-red napkin. The shopping prompt specified a plain brown bag, red peppers, onions, red chillies, lemons, burgundy-leaf herbs and a loaf at a 45-degree camera angle. Red-leaf salad and burgundy herbs keep the supplied subjects without introducing green. The hero was exported to exact 16:9 dimensions; no photographs were regenerated.

Preserve the hero's left-side space when placing copy; use chalk text there and check legibility in the final layout. Preserve the complete empty plate and the bag's produce when cropping the other photographs.
