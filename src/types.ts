export type WarKind = "active" | "previous";

export type ParsedSavefileView = {
  path: string;
  topLevelStatementCount: number;
  activeWars: WarView[];
  previousWars: WarView[];
};

export type WarView = {
  name: string;
  kind: WarKind;
  attackers: string[];
  defenders: string[];
  battleCount: number;
  totalLosses: number;
  attackerTotalLosses: number;
  defenderTotalLosses: number;
  startDate: string | null;
  endDate: string | null;
  battles: BattleView[];
};

export type BattleView = {
  name: string;
  locationId: number;
  locationLabel: string;
  totalLosses: number;
  attacker: BattleSideView;
  defender: BattleSideView;
  unitBreakdown: UnitBreakdownRowView[];
};

export type BattleSideView = {
  country: string | null;
  leader: string | null;
  losses: number | null;
};

export type UnitBreakdownRowView = {
  unitKind: string;
  attackerCount: number;
  defenderCount: number;
};

export type WarSectionKey = "activeWars" | "previousWars";
