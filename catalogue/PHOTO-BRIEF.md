# MSTRCHF 9000 recipe photos

MSTRCHF 9000 is a meal-planning app. Every recipe card shows one photo. The current photos look generic and AI-made; these replace them with one consistent, editorial look across every recipe.

## The job

For each recipe in the batch file you are given, in order:

1. If `out/recipes/<id>.png` already exists, skip it.
2. Otherwise generate **one** image with your image generation tool: landscape 3:2 (1536×1024 if you can choose), using the house style below plus a subject line written from that recipe's `title`, `blurb` and `visible` ingredients.
3. Save it as `out/recipes/<id>.png`, then append `<id>` on its own line to `out/recipes/done.txt`.
4. If generation fails, try once more. If it fails again, append `<id> FAILED` to `out/recipes/done.txt` and move on.

Rules:

- Exactly one saved image per recipe. No variants, no extra files, no contact sheets.
- Write only inside `out/`. Do not touch anything else.
- When the batch is done, reply with the number saved, skipped and failed.

## House style (use for every image)

Editorial home-cooking photograph for a recipe card. Soft natural window light from the left, gentle shadows, warm and appetising, true-to-life colour. Camera at about 45 degrees; straight overhead only for flat food (toast, pizza, salads on a plate, tray bakes). One portion, served in or on a single handmade stoneware piece with a matte glaze in cream, oatmeal or deep oxblood red, on the same dark walnut wooden table every time, with a crumpled linen napkin in muted brick red or ochre. One or two honest props at most: a fork or spoon, a small scatter of one raw ingredient from the dish. Real cooked texture: a drip of sauce, uneven herbs, a few crumbs; not glossy, not perfectly symmetrical, no steam effects. The food fills about 60 percent of the frame, slightly off centre, with calm space around it.

Never: text, logos, labels, packaging, hands, people, cutlery sets, cluttered tables, white studio backgrounds, cartoon or illustration styles.

Vary the composition from image to image so a page of 20 cards does not look stamped out: move the napkin between left, right, top and bottom edges (or leave it out), swap spoon for fork or no cutlery, and alternate glaze colours. Keep the table, light and overall mood identical.

## Subject line

Write one or two sentences naming the dish and what is visibly on the plate, e.g. "Hot beef madras: tender beef strips in a deep red chilli tomato sauce, a mound of basmati rice beside it, a spoon of yoghurt and torn coriander on top."

- Show the ingredients from `visible` that would really be seen in the finished dish (sauces, bases, proteins, garnishes). Leave out things cooked into the sauce, like garlic or tomato puree.
- If `vegetarian` is true, there must be no meat or fish anywhere in the image.
- Breakfasts and lunches are smaller and simpler than dinners.
