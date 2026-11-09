# MSTRCHF 9000 for Claude

A meal planner tuned to your taste, and the skills that do the shopping and the scheduling.

| Skill | What it does |
|---|---|
| `mstrchf-planner` | Sets up your own copy of the planner as a Claude artifact, opens it, and reads your week. Your kitchen is saved privately to your Claude account. |
| `tesco-basket` | Fills your Tesco basket from the planner's "Order with Claude" instructions in your own signed-in browser. Stops before the delivery slot and checkout for you. |
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
- The basket: a browser Claude can use (Claude in Chrome, or the desktop app's browser) with you signed in to Tesco.
- The calendar: a calendar connector such as Google Calendar.

## Safety

Claude never types passwords or card details, never checks out, and books a delivery slot only after you confirm one. If Tesco shows a security check, Claude stops and hands back to you.

Recipes are original. Prices are estimates for planning, not live Tesco prices.
