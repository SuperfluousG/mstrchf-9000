# Recipe authoring guide

The catalogue is original recipe writing in the style of a UK meal-kit service: weeknight-friendly, generous with flavour, clear about heat and timing. Every recipe is our own wording. Do not copy or closely paraphrase any published recipe (HelloFresh, BBC Good Food, etc.); common dishes are fine, but write the method fresh.

## File format

Each file in `catalogue/recipes/` is a JSON **array** of recipe objects:

```json
{
  "id": "smoky-chicken-fajita-traybake",
  "title": "Smoky chicken fajita traybake",
  "blurb": "Peppers and chicken roasted hot with smoked paprika, wrapped warm with lime yoghurt.",
  "meal": "dinner",
  "cuisine": "mexican",
  "protein": "chicken",
  "time": 35,
  "difficulty": 1,
  "spice": 2,
  "servings": 2,
  "tags": ["traybake", "family-friendly"],
  "ingredients": [
    { "id": "chicken-thigh", "qty": 300, "prep": "cut into strips" },
    { "id": "red-pepper", "qty": 1, "prep": "sliced" },
    { "id": "yellow-pepper", "qty": 1, "prep": "sliced" },
    { "id": "red-onion", "qty": 1, "prep": "cut into wedges" },
    { "id": "smoked-paprika", "qty": 5 },
    { "id": "ground-cumin", "qty": 2.5 },
    { "id": "tortilla", "qty": 4 },
    { "id": "greek-yoghurt", "qty": 100 },
    { "id": "lime", "qty": 1 },
    { "id": "coriander", "qty": 10, "prep": "roughly chopped" },
    { "id": "olive-oil", "qty": 15 },
    { "id": "salt", "qty": 2 },
    { "id": "black-pepper", "qty": 1 }
  ],
  "steps": [
    "Heat the oven to 220°C (200°C fan). Toss the chicken, peppers and onion on a large tray with the oil, smoked paprika, cumin and a good pinch of salt and pepper.",
    "Roast for 20–25 minutes, turning halfway, until the chicken is cooked through and the peppers are charred at the edges.",
    "Meanwhile, zest half the lime into the yoghurt, squeeze in a little juice and season. Cut the rest of the lime into wedges.",
    "Warm the tortillas in the oven for the last 2 minutes.",
    "Pile the traybake into the wraps with a spoonful of lime yoghurt and plenty of coriander."
  ],
  "source": "MSTRCHF original"
}
```

## Field rules

- `id`: kebab-case, unique across the whole catalogue. Make it descriptive (dish + key ingredient) to avoid clashes.
- `title` ≤ 60 characters, sentence case. `blurb` ≤ 140 characters, appetising, one sentence.
- `meal`: `dinner`, `lunch` or `breakfast`.
- `cuisine`: one of british, italian, french, spanish, greek, middle-eastern, north-african, indian, thai, chinese, japanese, korean, vietnamese, indonesian, mexican, american, caribbean, latin, european.
- `protein` (the star of the dish): chicken, beef, pork, lamb, turkey, fish, prawns, egg, cheese, beans, lentils, tofu, veg. Use `cheese`/`egg`/`beans`/`lentils`/`tofu`/`veg` only when there is no meat or fish.
- `time`: honest total minutes. Dinners mostly 20–45. `difficulty`: 1 easy, 2 medium, 3 involved (use 3 rarely).
- `spice`: 0 none, 1 gentle warmth, 2 properly spicy, 3 hot. Be honest; spice drives taste matching.
- `servings`: always 2. All quantities are for 2 people.
- `tags` (only these): quick (≤25 min), one-pot, traybake, family-friendly, comfort, light, high-protein, veg-packed, fakeaway, date-night, budget, no-cook, batch-friendly, lunchbox, meal-prep, fresh, hearty.
- `ingredients`: ids **must** exist in `catalogue/ingredients.json`. Quantities are in that ingredient's base unit (`g`, `ml` or `each`):
  - `each` items are counts (onions, peppers, garlic cloves, eggs, tins, tortillas, bread slices, stock cubes).
  - Spices are grams: 1 tsp ≈ 2.5 g, 1 tbsp ≈ 7 g. Oils/sauces are ml: 1 tsp = 5 ml, 1 tbsp = 15 ml.
  - Realistic portions for 2: meat/fish 250–320 g; dry rice 150 g; dry pasta 180–200 g; noodles 150–200 g; potatoes 500–600 g.
  - Include pantry items you use (salt, black-pepper, olive-oil, vegetable-oil, sugar) so the method is complete.
- `steps`: 4–8 steps, UK English, specific heats and times, each a full sentence or two.
- `source`: `"MSTRCHF original"`.
- Omit `image` (pictures are added separately) unless told otherwise.

## Make the planner's job easy

The weekly planner groups meals that share fresh ingredients so packs get used up (three peppers across three dinners, one bunch of coriander across two). Deliberately reuse common perishables across your recipes: peppers, spring onions, coriander, parsley, spinach, courgette, cherry tomatoes, lime, lemon, red chilli, feta, Greek yoghurt, mushrooms. Avoid one-off exotic ingredients.

## Diet coverage

Across each batch: at least 40% vegetarian (no meat or fish), at least 15% vegan (no meat, fish, dairy, egg or honey), some fish/prawn dishes, and a spread of spice levels (some 0, some 3). Vary cuisine and protein within your batch.

## Missing ingredients

Use only existing ids. If a dish genuinely needs something missing, swap in a close existing ingredient or pick another dish. Do not add new ingredient ids.

## Check your work

Run `node scripts/validate-catalogue.mjs <your-file.json>` from the project root and fix every error before finishing.
