<script lang="ts">
  import { onMount } from "svelte";
  import { invoke } from "@tauri-apps/api/core";
  import { listen } from "@tauri-apps/api/event";
  import { openUrl } from "@tauri-apps/plugin-opener";
  import PlayerCard from "$lib/PlayerCard.svelte";
  import {
    defaultSettings,
    type Champion,
    type Settings,
    type UiState,
  } from "$lib/types";

  const REGIONS = ["euw", "eune", "na", "kr", "br", "jp", "lan", "las", "oce", "tr", "ru"];
  const STORE_KEY = "lobbyreveal.settings";

  let ui = $state<UiState>({
    connected: false,
    phase: "",
    inChampSelect: false,
    champPhase: "",
    timeLeftMs: 0,
    players: [],
    message: "Starting…",
  });

  let settings = $state<Settings>({ ...defaultSettings });
  let champions = $state<Champion[]>([]);
  let pickName = $state("");
  let banName = $state("");
  let ready = false; // gate the settings $effect until initial load is done

  const byName = $derived(
    new Map(champions.map((c) => [c.name.toLowerCase(), c.id])),
  );
  const idToName = $derived(new Map(champions.map((c) => [c.id, c.name])));

  function nameToId(name: string): number {
    return byName.get(name.trim().toLowerCase()) ?? 0;
  }

  onMount(async () => {
    // Restore persisted settings.
    try {
      const raw = localStorage.getItem(STORE_KEY);
      if (raw) settings = { ...defaultSettings, ...JSON.parse(raw) };
    } catch {}

    try {
      champions = await invoke<Champion[]>("get_champions");
      pickName = idToName.get(settings.autoPickChampionId) ?? "";
      banName = idToName.get(settings.autoBanChampionId) ?? "";
    } catch (e) {
      console.error("champion list failed", e);
    }

    await invoke("set_settings", { settings });
    ready = true;

    await listen<UiState>("lcu-update", (event) => {
      ui = event.payload;
    });
  });

  // Persist + push settings whenever they change (after initial load).
  $effect(() => {
    const snapshot = $state.snapshot(settings);
    if (!ready) return;
    localStorage.setItem(STORE_KEY, JSON.stringify(snapshot));
    invoke("set_settings", { settings: snapshot });
  });

  function onPickInput() {
    settings.autoPickChampionId = nameToId(pickName);
  }
  function onBanInput() {
    settings.autoBanChampionId = nameToId(banName);
  }
  let dodgeMsg = $state("");
  let dodgeTimer: ReturnType<typeof setTimeout> | undefined;

  async function dodgeNow() {
    clearTimeout(dodgeTimer);
    dodgeMsg = "Dodging…";
    try {
      await invoke("dodge_now");
      dodgeMsg = "✓ Dodge sent — leaving champ select (client stays open).";
    } catch (e) {
      dodgeMsg = `✗ ${e}`;
    }
    dodgeTimer = setTimeout(() => (dodgeMsg = ""), 5000);
  }

  const seconds = $derived(Math.max(0, Math.ceil(ui.timeLeftMs / 1000)));

  // Players we actually resolved a Riot ID for.
  const revealed = $derived(
    ui.players.filter((p) => p.riotId && p.riotId !== "Hidden"),
  );

  // Open every revealed player in one OP.GG multi-search tab.
  function searchAll() {
    if (revealed.length === 0) return;
    const list = revealed.map((p) => p.riotId).join(",");
    openUrl(
      `https://www.op.gg/multisearch/${settings.region}?summoners=${encodeURIComponent(list)}`,
    );
  }
</script>

<datalist id="champ-list">
  {#each champions as c (c.id)}
    <option value={c.name}></option>
  {/each}
</datalist>

<main>
  <header>
    <div class="brand">
      <svg class="logo" viewBox="0 0 16 16" aria-hidden="true">
        <g shape-rendering="crispEdges">
          <path fill="#5fd0ff" d="M4 2h4v1H4zM3 3h6v1H3zM2 4h2v4H2zM8 4h2v4H8zM3 8h2v1H3zM7 8h2v1H7zM4 9h4v1H4z" />
          <path fill="#2a6cf0" d="M4 4h4v4H4zM4 4h4v1H4zM3 5h6v3H3z" />
          <rect x="5" y="3" width="1" height="1" fill="#fff" />
          <path fill="#5fd0ff" d="M9 9h2v2H9zM10 10h2v2h-2zM11 11h2v2h-2z" />
        </g>
      </svg>
      <h1>Lobby<span>Reveal</span></h1>
    </div>
    <div class="status" class:on={ui.connected}>
      <span class="dot"></span>
      {ui.connected ? "Connected" : "Disconnected"}
    </div>
  </header>

  <section class="lobby">
    {#if ui.inChampSelect}
      <div class="phase-bar">
        <span class="champ-phase">{ui.champPhase || "CHAMP SELECT"}</span>
        {#if seconds > 0}<span class="timer">{seconds}s</span>{/if}
      </div>
      {#if ui.players.length === 0}
        <p class="empty">Revealing lobby…</p>
      {:else}
        <div class="players">
          {#each ui.players as p (p.cellId)}
            <PlayerCard player={p} />
          {/each}
        </div>
      {/if}
      <div class="actions">
        <button
          class="search"
          onclick={searchAll}
          disabled={revealed.length === 0}
        >
          🔍 Search all on OP.GG
        </button>
        <button class="dodge" onclick={dodgeNow}>⏏ Dodge</button>
      </div>
      {#if dodgeMsg}
        <p class="dodge-msg" class:err={dodgeMsg.startsWith("✗")}>{dodgeMsg}</p>
      {/if}
    {:else}
      <p class="empty">{ui.message || "Waiting for champ select…"}</p>
    {/if}
  </section>

  <section class="settings">
    <h2>Automation</h2>

    <label class="row">
      <input type="checkbox" bind:checked={settings.autoAccept} />
      <span class="label">Auto-accept matches</span>
    </label>

    <label class="row">
      <input type="checkbox" bind:checked={settings.autoDodge} />
      <span class="label">Last-second auto-dodge</span>
      {#if settings.autoDodge}
        <span class="inline">
          at
          <input
            class="num"
            type="number"
            min="200"
            max="10000"
            step="100"
            bind:value={settings.dodgeThresholdMs}
          /> ms left
        </span>
      {/if}
    </label>

    <label class="row">
      <input type="checkbox" bind:checked={settings.autoPick} />
      <span class="label">Auto-pick</span>
      {#if settings.autoPick}
        <input
          class="champ"
          list="champ-list"
          placeholder="Champion…"
          bind:value={pickName}
          oninput={onPickInput}
        />
      {/if}
    </label>

    <label class="row">
      <input type="checkbox" bind:checked={settings.autoBan} />
      <span class="label">Auto-ban</span>
      {#if settings.autoBan}
        <input
          class="champ"
          list="champ-list"
          placeholder="Champion…"
          bind:value={banName}
          oninput={onBanInput}
        />
      {/if}
    </label>

    <label class="row">
      <span class="label">OP.GG region</span>
      <select bind:value={settings.region}>
        {#each REGIONS as r}
          <option value={r}>{r.toUpperCase()}</option>
        {/each}
      </select>
    </label>
  </section>

  <footer>
    Read-only client API use. Use at your own risk — automating champ select may
    violate the Riot Terms of Service.
  </footer>
</main>

<style>
  :global(body) {
    margin: 0;
    background:
      radial-gradient(1100px 460px at 50% -120px, #1a2944 0%, transparent 60%),
      #0a0d15;
    color: #e6e9f0;
    font-family: "Segoe UI", system-ui, sans-serif;
  }
  main {
    max-width: 560px;
    margin: 0 auto;
    padding: 1rem 1.1rem 2rem;
  }
  header {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 1.1rem;
  }
  .brand {
    display: flex;
    align-items: center;
    gap: 0.55rem;
  }
  .logo {
    width: 30px;
    height: 30px;
    filter: drop-shadow(0 0 6px rgba(95, 208, 255, 0.45));
  }
  h1 {
    font-size: 1.3rem;
    margin: 0;
    font-weight: 800;
    letter-spacing: -0.01em;
  }
  h1 span {
    background: linear-gradient(90deg, #3b82f6, #5fd0ff);
    -webkit-background-clip: text;
    background-clip: text;
    -webkit-text-fill-color: transparent;
  }
  .status {
    display: flex;
    align-items: center;
    gap: 0.4rem;
    font-size: 0.8rem;
    color: #8b94a7;
  }
  .status .dot {
    width: 8px;
    height: 8px;
    border-radius: 50%;
    background: #e0564b;
  }
  .status.on .dot {
    background: #41d18a;
    box-shadow: 0 0 0 0 rgba(65, 209, 138, 0.6);
    animation: pulse 2s infinite;
  }
  @keyframes pulse {
    0% { box-shadow: 0 0 0 0 rgba(65, 209, 138, 0.5); }
    70% { box-shadow: 0 0 0 6px rgba(65, 209, 138, 0); }
    100% { box-shadow: 0 0 0 0 rgba(65, 209, 138, 0); }
  }
  .lobby {
    min-height: 120px;
    margin-bottom: 1.25rem;
  }
  .phase-bar {
    display: flex;
    align-items: center;
    justify-content: space-between;
    margin-bottom: 0.6rem;
  }
  .champ-phase {
    font-size: 0.72rem;
    letter-spacing: 0.08em;
    color: #8b94a7;
  }
  .timer {
    font-variant-numeric: tabular-nums;
    font-weight: 700;
    color: #e6b94e;
  }
  .players {
    display: flex;
    flex-direction: column;
    gap: 0.5rem;
  }
  .empty {
    color: #6b7387;
    text-align: center;
    padding: 2rem 0;
    font-size: 0.9rem;
  }
  .actions {
    display: flex;
    gap: 0.5rem;
    margin-top: 0.85rem;
  }
  .search,
  .dodge {
    padding: 0.62rem 0.8rem;
    border-radius: 9px;
    font-weight: 700;
    font-size: 0.9rem;
    cursor: pointer;
    transition:
      transform 0.05s ease,
      filter 0.15s ease;
  }
  .search:active,
  .dodge:active {
    transform: translateY(1px);
  }
  .search {
    flex: 1;
    color: #061018;
    border: none;
    background: linear-gradient(90deg, #3b82f6, #5fd0ff);
  }
  .search:hover:not(:disabled) {
    filter: brightness(1.08);
  }
  .search:disabled {
    opacity: 0.4;
    cursor: not-allowed;
  }
  .dodge {
    background: #2a1417;
    color: #ef6f63;
    border: 1px solid #5a2a2a;
  }
  .dodge:hover {
    background: #3a1a1d;
  }
  .dodge-msg {
    margin: 0.5rem 0 0;
    font-size: 0.8rem;
    color: #41d18a;
    text-align: center;
  }
  .dodge-msg.err {
    color: #e0796b;
  }
  .settings {
    background: #11161f;
    border: 1px solid #1e2636;
    border-radius: 12px;
    padding: 0.6rem 1rem 1rem;
  }
  .settings h2 {
    font-size: 0.78rem;
    letter-spacing: 0.08em;
    text-transform: uppercase;
    color: #8b94a7;
    margin: 0.6rem 0;
  }
  .row {
    display: flex;
    align-items: center;
    gap: 0.6rem;
    padding: 0.5rem 0;
    border-top: 1px solid #1a2230;
  }
  .row .label {
    flex: 1;
    font-size: 0.92rem;
  }
  .inline {
    font-size: 0.82rem;
    color: #9aa3b5;
    display: flex;
    align-items: center;
    gap: 0.3rem;
  }
  input[type="checkbox"] {
    width: 17px;
    height: 17px;
    accent-color: #3b82f6;
  }
  .num {
    width: 64px;
  }
  .champ,
  .num,
  select {
    background: #0d111b;
    border: 1px solid #2a3346;
    color: #e6e9f0;
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 0.85rem;
  }
  .champ {
    width: 150px;
  }
  footer {
    margin-top: 1rem;
    font-size: 0.72rem;
    color: #5a6276;
    text-align: center;
    line-height: 1.5;
  }
</style>
