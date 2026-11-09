---
name: tesco-basket
description: Fill a Tesco groceries basket from a MasterChef 9000 basket file or pasted order instructions, using the user's own signed-in browser. Use when the user asks to order, shop or fill the basket for a MasterChef 9000 week. Stops before delivery-slot booking and checkout for the user to confirm.
---

# Tesco basket from a MasterChef 9000 plan

MasterChef 9000 plans the week and writes a basket: one entry per ingredient with a search phrase, the amount the recipes need, and the packs to buy. Your job is to put those items into the user's Tesco basket in their own browser, then hand back to them.

## Input

Either the pasted "Order with Claude" instructions, or a `mstrchf-basket-<week>.json` file:

```json
{ "retailer": "tesco", "week": "16–22 Nov", "people": 2,
  "items": [ { "name": "Red pepper", "search": "red peppers", "need": "3", "buy": [{ "size": "pack of 3", "count": 1 }], "estPrice": 1.9, "usedIn": ["Smoky chicken fajitas", "Chorizo butter bean stew"] } ] }
```

## Hard rules

- Use the Tesco tab the user is already signed in to. If Tesco asks for a login, a CAPTCHA or any security check, stop and ask the user to do it.
- Never type passwords, card details or Clubcard numbers. Never press Checkout, Pay or Confirm order.
- Never remove or change items that were already in the basket before you started. Note them in your report.
- Book a delivery slot only after the user confirms a specific slot in chat.
- If an add might have failed (page error, spinner, timeout), read the basket before retrying so nothing is added twice.

## Procedure

1. Open the basket and record what is already in it (product and quantity).
2. For each item, in aisle order:
   1. Search Tesco for the `search` phrase.
   2. Choose the product that best matches the `buy` pack size. Prefer the lowest price per kg/litre/each among sensible matches; Tesco own-brand is fine. Avoid prepared, marinated or ready-to-eat variants unless the item name says so (e.g. "Cooked puy lentils").
   3. Add the `count` for each pack in `buy`. "single" means one loose or individual item.
   4. If it is unavailable, choose the closest substitute (same ingredient, nearest size; or the nearest equivalent such as cod ↔ haddock) and note it. If nothing sensible exists, skip it and note it.
3. Read the full basket again and compare with the list: every planned item present at the right count, earlier items untouched.
4. Report back:
   - Added: product, quantity, price.
   - Substituted: what and why.
   - Skipped: what and why.
   - Already in the basket before you started.
   - Basket total, and the estimated total from MasterChef for comparison.
5. Offer the next three available delivery slots and ask which one to book. After booking, remind the user to review the basket and check out themselves.

## Notes

- Quantities in the file are already scaled to the household and rounded up to whole packs.
- `usedIn` says which meals need the item; use it to judge substitutes (e.g. a curry can take any chicken thighs; a salad needs the leaves it names).
- Prices in the file are estimates, not Tesco's live prices.
