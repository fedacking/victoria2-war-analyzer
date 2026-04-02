export type WarKind = "active" | "previous";
export type BattleWinner = "attacker" | "defender" | "unknown";

export type ParsedSavefileView = {
  path: string;
  topLevelStatementCount: number;
  countryTags: string[];
  activeWars: WarView[];
  previousWars: WarView[];
};

export type CountryCatalogView = {
  countries: Record<string, CountryView>;
  warnings: string[];
};

export type CountryView = {
  tag: string;
  name: string;
  flagDataUrl: string | null;
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
  winner: BattleWinner;
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
