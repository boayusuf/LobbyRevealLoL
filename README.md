# LobbyReveal

A small Windows app that reveals your hidden League of Legends ranked lobby —
see who's in your champ select with their names, ranks, and recent win rates,
plus a few optional quick-actions.

## Features

- **Reveal teammates** in champ select — Riot ID, rank, LP, and last-10-games win rate
- **OP.GG lookup** — open one player, or the whole lobby at once
- **Auto-accept** matches
- **Auto-pick** and **auto-ban** your chosen champion
- **Auto-dodge** (last-second) + a manual dodge button

## ⚠️ Use at your own risk

This works through Riot's local client API. Reading lobby info is low-risk, but
the automation (auto pick / ban / dodge) can break Riot's Terms of Service and
may put your account at risk. Everything is **off by default** — if you enable
it, you accept all risk.

## Download

1. Go to the [Releases](https://github.com/boayusuf/LobbyRevealLoL/releases) page.
2. Download the latest `LobbyReveal_x.x.x_x64-setup.exe`.
3. Run it, start League, then open LobbyReveal.

Windows only.

## Note

Built against the current League patch. Riot changes the client API often, so it
may stop working — or only partly work — on patches newer than the last tested
one.
