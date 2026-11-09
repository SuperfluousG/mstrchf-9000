---
name: tesco-basket
description: Fill a Tesco groceries basket from a MSTRCHF 9000 basket file or pasted order instructions, using the user's own signed-in browser. Use when the user asks to order, shop or fill the basket for a MSTRCHF 9000 week. Stops before delivery-slot booking and checkout for the user to confirm.
---

# Tesco basket from a MSTRCHF 9000 plan

MSTRCHF 9000 plans the week and writes a basket: one entry per ingredient with a search phrase, the amount the recipes need, and the packs to buy. Your job is to put those items into the user's Tesco basket in their own browser, then hand back to them.

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
   - Basket total, and the estimated total from MSTRCHF for comparison.
5. Offer the next three available delivery slots and ask which one to book. After booking, remind the user to review the basket and check out themselves.

## Lessons from the first live run (9 Nov 2026)

- Every Tesco Add button is labelled `add 1 <exact product name>`. Find it by that label, **scroll it into view, then click**. In the first run 4 of 40 off-screen clicks silently did nothing.
- After each add, confirm the product's basket line ("2 Tesco Limes Each for £0.46") shows the expected count. Second clicks on the same product were the least reliable; on the basket page, use that product's own `add 1 …` button.
- Always finish by reading the whole basket: line count, unit count and the guide price. The first run matched 40 lines and 45 units only after a retry pass.
- Loose produce (onions, ginger) is added by count in Quantity mode; leave the weight toggle alone.
- When an item is out of stock, broaden the search (`red peppers` → `peppers`). Make up the count with the nearest sensible alternative, e.g. a mixed 3-pack plus one single.
- If `need` is far below the planned pack (curry paste: need about 90 g, planned 283 g jar), buy the smaller pack when one exists.
- Baskets under £50 carry a £5 minimum-basket charge. Mention it if the total is close.
- **Work in one tab, one item at a time.** On the second run (9 Nov 2026), searching in six tabs at once got the browser blocked by Tesco's security checks after about 20 items. Tesco then returned HTTP 403 "your browser has failed some security checks" on every page, including the basket.
- A single tab with about 3 seconds between actions added 47 items without a block on the same day. Pattern per item: search page, read the top results, `find "add 1"` to get every Add button by product name, then scroll, click, wait and confirm the "N <product> for £x" basket line. When one results page shows several needed items (black beans and kidney beans, ground cumin and cumin seeds), add them all from that page.
- **If Tesco shows a security, CAPTCHA or "failed some security checks" page, stop immediately.** Do not retry, switch tabs or try to get round it. Report what was added so far (unverified if the basket cannot be read) and the remaining items, and hand back to the user.

## Notes

- Quantities in the file are already scaled to the household and rounded up to whole packs.
- `usedIn` says which meals need the item; use it to judge substitutes (e.g. a curry can take any chicken thighs; a salad needs the leaves it names).
- Prices in the file are estimates, not Tesco's live prices.
