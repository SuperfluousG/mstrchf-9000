# MSTRCHF 9000 for Claude

A meal planner tuned to your taste, and the skills that do the shopping and the scheduling.

| Skill | What it does |
|---|---|
| `mstrchf-planner` | Sets up your own copy of the planner as a Claude artifact, opens it, and reads your week. Your kitchen is saved privately to your Claude account. |
| `grocery-basket` | Fills your basket at your online grocer from the planner's "Order with Claude" instructions, in your own signed-in browser. Has a guide for each leading shop in the UK (Tesco, Sainsbury's, Asda, Ocado, Morrisons), the US (Walmart, Kroger, Target, Albertsons) and Canada (PC Express, Walmart Canada, Voilà, Metro), and a method for any other online grocer. Stops before the delivery slot and checkout for you. |
| `meal-calendar` | Books each planned meal into your calendar with its ingredients and method. |

## Install

In the Claude desktop app, add the `.plugin` file under Settings → Plugins (or Cowork → Plugins). In Claude Code:

```
/plugin marketplace add <path to this folder>
/plugin install mstrchf-9000@mstrchf-9000
```

Then ask Claude: "Set up MSTRCHF 9000 for me."

## Needs

- The planner: the Claude desktop app or claude.ai (it publishes an artifact).
- The basket: a browser Claude can use (Claude in Chrome, or the desktop app's browser) with you signed in to your grocery shop.
- The calendar: a calendar connector such as Google Calendar.

## Safety

Claude never types passwords, card details or loyalty numbers, never creates accounts, never checks out, and books a delivery slot only after you confirm one. If the shop shows a login, CAPTCHA or security check, Claude stops and hands back to you.

Amazon's Conditions of Use bar unauthorised AI agents, so for Amazon Fresh Claude gives you the list to shop yourself instead of filling the cart.

Recipes are original. Prices are estimates for planning, not the shop's live prices.
