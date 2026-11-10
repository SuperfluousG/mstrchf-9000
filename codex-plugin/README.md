# MSTRCHF 9000 for ChatGPT desktop

A free meal-planning beta for the ChatGPT desktop app. It uses your existing ChatGPT subscription, with no extra MSTRCHF fee or separate AI bill. You pay for your groceries. We intend to keep developing it; this is not a finished product.

## Get started

1. Install the [ChatGPT desktop app](https://chatgpt.com/download/) and sign in to your usual account.
2. Keep this plugin ZIP ready to attach to a new local ChatGPT chat.
3. Ask ChatGPT: “Install the MSTRCHF 9000 plugin from this ZIP, then open my meal planner.” Approve the installation prompts it shows. ChatGPT can handle local setup without you using a command line.
4. Once installed, ask “Open MSTRCHF 9000”. Use the Countercut icon to identify the planner and pin it in the sidebar for next time.

Your account and workspace must allow custom plugins. The local server uses Node.js 20 or newer; if the assistant reports that this is missing, ask it to help finish setup using the [official Node.js installer](https://nodejs.org/en/download). No API key or MSTRCHF account is needed. The beta’s first-time setup still needs testing across more accounts.

## Your kitchen

The plugin reports the actual path with `get_kitchen`. Defaults:

- Windows: `%LOCALAPPDATA%/MSTRCHF9000/kitchen.json`
- macOS: `~/Library/Application Support/MSTRCHF9000/kitchen.json`
- Linux: `${XDG_DATA_HOME:-~/.local/share}/mstrchf9000/kitchen.json`

One daily backup is retained for 14 days. Exports are written to `exports/` when the host does not offer downloads. The store survives plugin upgrades and uninstall. Keep a backup before deliberately removing its directory.

An existing kitchen is not imported automatically. Use Kitchen → Restore backup and review the file before replacing the new plugin kitchen. `MSTRCHF_DATA_DIR` may override the directory for an explicitly configured local run; development and tests must use throwaway data.

## What is included

390 original AI-written recipes and photos, three market catalogues, the planner and local MCP server, plus grocery-basket and meal-calendar skills. Prices are estimates. The recipes have not been cook-tested. Check actual packs and substitutions for allergens. Grocery and calendar workflows need the relevant browser or calendar connection; checkout stays with you.

The view includes a global sidebar entrypoint, the selected Countercut icon, fullscreen support, safe-area handling and lazy photos. The SDK preview and packaged stdio server are tested; actual ChatGPT host and cross-account installation acceptance remain separate checks. The browser, Claude artifact and ChatGPT desktop builds share the planner code.

Publisher: Bayes Ltd. This is a downloadable beta, not a public-directory listing.
