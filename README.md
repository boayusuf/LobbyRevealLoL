# LobbyReveal

A small Windows app that reveals your hidden League of Legends ranked lobby. See
who's in your champ select with their names, ranks and recent win rates, plus a
few optional quick actions.

## Features

- Reveal teammates in champ select (Riot ID, rank, LP, recent win rate over the last 10 games)
- OP.GG lookup for one player or the whole lobby at once
- Auto accept matches
- Auto pick and auto ban your chosen champion
- Auto dodge (last second) plus a manual dodge button

## Use at your own risk

This works through Riot's local client API. Reading lobby info is low risk, but
the automation (auto pick, ban, dodge) can break Riot's Terms of Service and may
put your account at risk. Everything is off by default. If you enable it, you
accept all risk.

## Download

Get the installer from the [latest release](https://github.com/boayusuf/LobbyRevealLoL/releases/latest):

1. Download `LobbyReveal_0.1.0_x64-setup.exe`.
2. Run it (Windows only, no admin needed, installs to your user profile).
3. Start League and log in, then open LobbyReveal.

## Build from source (optional)

You need Git, Node.js and Rust installed (Rust needs the Visual Studio C++ build
tools). Then in PowerShell:

```powershell
git clone https://github.com/boayusuf/LobbyRevealLoL.git
cd LobbyRevealLoL
npm install
npm run tauri build
```

Or just run it without building an installer:

```powershell
npm run tauri dev
```

## Note

Built against the current League patch. Riot changes the client API often, so it
may stop working, or only partly work, on patches newer than the last tested one.
