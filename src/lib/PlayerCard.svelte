<script lang="ts">
  import { openUrl } from "@tauri-apps/plugin-opener";
  import type { UiPlayer } from "./types";

  let { player }: { player: UiPlayer } = $props();

  const POSITIONS: Record<string, string> = {
    top: "Top",
    jungle: "Jungle",
    middle: "Mid",
    bottom: "Bot",
    utility: "Support",
    "": "",
  };

  function rankClass(rank: string): string {
    return rank.toLowerCase().split(" ")[0] || "unranked";
  }

  function open() {
    if (player.opggUrl) openUrl(player.opggUrl);
  }
</script>

<div class="card" class:local={player.isLocal}>
  {#if player.position}
    <div class="pos">{POSITIONS[player.position] ?? player.position}</div>
  {/if}
  <div class="main">
    <div class="name-row">
      <span class="name">{player.riotId}</span>
      {#if player.isLocal}<span class="you">YOU</span>{/if}
    </div>
    <div class="sub">
      <span class="rank {rankClass(player.rank)}">{player.rank}</span>
      {#if player.lp > 0}<span class="games">{player.lp} LP</span>{/if}
      {#if player.recentGames > 0}
        <span
          class="wr"
          class:good={player.recentWinrate >= 50}
          class:bad={player.recentWinrate < 50}
        >
          {player.recentWinrate}% WR
        </span>
        <span class="games">last {player.recentGames}</span>
      {/if}
    </div>
  </div>
  {#if player.opggUrl}
    <button class="opgg" onclick={open} title="Open on OP.GG">OP.GG ↗</button>
  {/if}
</div>

<style>
  .card {
    display: flex;
    align-items: center;
    gap: 0.75rem;
    padding: 0.6rem 0.8rem;
    background: linear-gradient(180deg, #181e2c, #131825);
    border: 1px solid #232b3d;
    border-radius: 10px;
    transition:
      transform 0.08s ease,
      border-color 0.15s ease;
  }
  .card:hover {
    transform: translateY(-1px);
    border-color: #2f3950;
  }
  .card.local {
    border-color: #3b82f6;
    background: linear-gradient(180deg, #17223e, #141b30);
    box-shadow: 0 0 0 1px rgba(59, 130, 246, 0.25);
  }
  .pos {
    width: 64px;
    font-size: 0.72rem;
    letter-spacing: 0.05em;
    text-transform: uppercase;
    color: #8b94a7;
  }
  .main {
    flex: 1;
    min-width: 0;
  }
  .name-row {
    display: flex;
    align-items: center;
    gap: 0.5rem;
  }
  .name {
    font-weight: 600;
    font-size: 0.98rem;
    color: #eef1f7;
    white-space: nowrap;
    overflow: hidden;
    text-overflow: ellipsis;
  }
  .you {
    font-size: 0.6rem;
    font-weight: 700;
    color: #3b82f6;
    border: 1px solid #3b82f6;
    border-radius: 4px;
    padding: 1px 4px;
  }
  .sub {
    display: flex;
    flex-wrap: wrap;
    gap: 0.5rem;
    margin-top: 0.2rem;
    font-size: 0.78rem;
    color: #9aa3b5;
  }
  .rank {
    font-weight: 600;
    color: #c9d1e0;
  }
  .rank.iron { color: #6b6560; }
  .rank.bronze { color: #a9744f; }
  .rank.silver { color: #9fb0c3; }
  .rank.gold { color: #e6b94e; }
  .rank.platinum { color: #4ec9b0; }
  .rank.emerald { color: #41d18a; }
  .rank.diamond { color: #6aa6f0; }
  .rank.master { color: #c061f0; }
  .rank.grandmaster { color: #e0564b; }
  .rank.challenger { color: #5fd0ff; }
  .wr.good { color: #41d18a; font-weight: 600; }
  .wr.bad { color: #e0796b; font-weight: 600; }
  .opgg {
    background: transparent;
    border: 1px solid #2f3950;
    color: #9aa3b5;
    border-radius: 6px;
    padding: 4px 8px;
    font-size: 0.74rem;
    cursor: pointer;
  }
  .opgg:hover {
    border-color: #3b82f6;
    color: #cdd6e6;
  }
</style>
