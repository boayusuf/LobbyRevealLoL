# LobbyReveal

Reveal your hidden League of Legends ranked lobby. A lightweight desktop app
(Tauri + Rust + SvelteKit) that connects to the local League Client API (LCU)
and shows who's actually in your champ select — names, ranks, win rates — plus
quality-of-life automation.

> ⚠️ **Use at your own risk.** This talks to Riot's *local* client API. Reading
> lobby data is low-risk, but **automating champ select (auto-pick / auto-ban /
> auto-dodge) can violate the Riot Terms of Service** and may put your account
> at risk. Automation is opt-in and off by default.

## Features

**Phase 1 — Reveal**
- Detects the running League Client automatically (no setup).
- Reveals every ally in champ select: Riot ID, level, solo/duo rank, LP, and
  win rate — even while names are visually hidden.
- One-click **OP.GG** lookup per player.

**Phase 2 — Automation**
- **Auto-accept** ready checks.
- **Last-second auto-dodge** — dodges during finalization with a configurable
  threshold (e.g. 1.5s left), plus a manual **Dodge now** button.
- **Auto-pick** — locks in your chosen champion when it's your turn.
- **Auto-ban** — bans your chosen champion when it's your turn.

## How it works

The League Client (`LeagueClientUx.exe`) runs a local REST API secured with a
self-signed cert and basic auth. The port and auth token are passed to the
process as command-line args, so the app discovers them by inspecting the
running process — no lockfile hunting. A 1-second polling loop in Rust tracks
the gameflow phase, reads the champ-select session, resolves each ally's
`puuid` to a Riot ID + ranked stats, and performs any enabled automation by
`PATCH`ing champ-select actions.

## Develop

Prerequisites: [Rust](https://rustup.rs), Node 18+, and the MSVC build tools.

```bash
npm install
npm run tauri dev      # run the app with hot reload
npm run tauri build    # produce a release installer
```

## Project layout

```
src/                     SvelteKit frontend (UI, settings, event handling)
  routes/+page.svelte    main window
  lib/PlayerCard.svelte  one revealed player row
  lib/types.ts           shared TS types
src-tauri/src/
  lcu/                   LCU client: connection, models, api, champions
  engine.rs              polling loop (reveal + automation)
  commands.rs            Tauri commands (settings, dodge, champion list)
  state.rs               shared app state
```

## Roadmap

- Inline champion icons + mastery / recent games
- Rule-based auto-dodge (e.g. dodge on low-winrate teammate)
- WebSocket subscription instead of polling
- Signed auto-updating releases via GitHub Actions
