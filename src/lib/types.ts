export interface UiPlayer {
  cellId: number;
  position: string;
  championId: number;
  riotId: string;
  level: number;
  rank: string;
  lp: number;
  wins: number;
  losses: number;
  winrate: number;
  isLocal: boolean;
  opggUrl: string;
}

export interface UiState {
  connected: boolean;
  phase: string;
  inChampSelect: boolean;
  champPhase: string;
  timeLeftMs: number;
  players: UiPlayer[];
  message: string;
  debug: string;
}

export interface Settings {
  autoAccept: boolean;
  autoDodge: boolean;
  dodgeThresholdMs: number;
  autoPick: boolean;
  autoPickChampionId: number;
  autoBan: boolean;
  autoBanChampionId: number;
  region: string;
}

export interface Champion {
  id: number;
  name: string;
}

export const defaultSettings: Settings = {
  autoAccept: false,
  autoDodge: false,
  dodgeThresholdMs: 1500,
  autoPick: false,
  autoPickChampionId: 0,
  autoBan: false,
  autoBanChampionId: 0,
  region: "euw",
};
