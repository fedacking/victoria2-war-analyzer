import { useEffect, useRef, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";
import type {
  BattleSideView,
  BattleWinner,
  CountryCatalogView,
  CountryView,
  ParsedSavefileView,
  UnitBreakdownRowView,
  WarSectionKey,
} from "./types";

type WarSelection = {
  section: WarSectionKey;
  warIndex: number;
};

const DEFAULT_GAME_PATH =
  "C:\\Program Files (x86)\\Steam\\steamapps\\common\\Victoria 2";
const GAME_PATH_STORAGE_KEY = "victoria2-war-analyzer:gamePath";
const MOD_PATH_STORAGE_KEY = "victoria2-war-analyzer:modPath";

const WAR_SECTION_ORDER: WarSectionKey[] = ["activeWars", "previousWars"];

const SECTION_COPY: Record<
  WarSectionKey,
  { title: string; tone: string; empty: string }
> = {
  activeWars: {
    title: "Active Wars",
    tone: "Live theatre",
    empty: "No active wars were found in this save.",
  },
  previousWars: {
    title: "Previous Wars",
    tone: "Campaign archive",
    empty: "No previous wars were found in this save.",
  },
};

const integerFormatter = new Intl.NumberFormat(undefined, {
  maximumFractionDigits: 0,
});

function App() {
  const [gamePath, setGamePath] = useState<string>(
    () => readStoredPath(GAME_PATH_STORAGE_KEY) || DEFAULT_GAME_PATH,
  );
  const [modPath, setModPath] = useState<string>(() =>
    readStoredPath(MOD_PATH_STORAGE_KEY),
  );
  const [selectedPath, setSelectedPath] = useState<string>("");
  const [parsedSavefile, setParsedSavefile] =
    useState<ParsedSavefileView | null>(null);
  const [countryCatalog, setCountryCatalog] =
    useState<CountryCatalogView | null>(null);
  const [countryWarnings, setCountryWarnings] = useState<string[]>([]);
  const [isParsing, setIsParsing] = useState<boolean>(false);
  const [isResolvingCountries, setIsResolvingCountries] =
    useState<boolean>(false);
  const [error, setError] = useState<string>("");
  const [selectedWarSection, setSelectedWarSection] =
    useState<WarSectionKey | null>(null);
  const [selectedWarIndex, setSelectedWarIndex] = useState<number>(0);
  const [selectedBattleIndex, setSelectedBattleIndex] = useState<number>(0);
  const battleListScrollRef = useRef<HTMLDivElement | null>(null);

  function resetSelection() {
    setSelectedWarSection(null);
    setSelectedWarIndex(0);
    setSelectedBattleIndex(0);
  }

  function applyDefaultSelection(nextSavefile: ParsedSavefileView) {
    const selection = getDefaultSelection(nextSavefile);

    if (!selection) {
      resetSelection();
      return;
    }

    setSelectedWarSection(selection.section);
    setSelectedWarIndex(selection.warIndex);
    setSelectedBattleIndex(0);
  }

  async function pickSavefile() {
    try {
      const selected = await open({
        directory: false,
        multiple: false,
        title: "Select a Victoria 2 savefile",
        filters: [
          {
            name: "Victoria 2 Savefiles",
            extensions: ["v2", "v2save", "sav"],
          },
        ],
      });

      if (typeof selected !== "string") {
        return;
      }

      setError("");
      setSelectedPath(selected);
      setParsedSavefile(null);
      resetSelection();
      setIsParsing(true);

      try {
        const nextSavefile = await invoke<ParsedSavefileView>("parse_savefile", {
          path: selected,
        });

        setParsedSavefile(nextSavefile);
        applyDefaultSelection(nextSavefile);
      } catch (cause) {
        console.error("Failed to parse savefile.", {
          cause,
          formatted: formatError(cause),
        });

        setParsedSavefile(null);
        resetSelection();
        setError(`Failed to parse savefile: ${formatError(cause)}`);
      } finally {
        setIsParsing(false);
      }
    } catch (cause) {
      console.error("Failed to open savefile dialog.", {
        cause,
        formatted: formatError(cause),
      });

      setError(`Failed to open file picker: ${formatError(cause)}`);
    }
  }

  async function pickGameFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select the Victoria 2 game folder",
        defaultPath: gamePath || DEFAULT_GAME_PATH,
      });

      if (typeof selected === "string") {
        setGamePath(selected);
      }
    } catch (cause) {
      console.error("Failed to pick base game folder.", {
        cause,
        formatted: formatError(cause),
      });
      setError(`Failed to open folder picker: ${formatError(cause)}`);
    }
  }

  async function pickModFolder() {
    try {
      const selected = await open({
        directory: true,
        multiple: false,
        title: "Select a Victoria 2 mod folder",
        defaultPath: modPath || `${gamePath}\\mod`,
      });

      if (typeof selected === "string") {
        setModPath(selected);
      }
    } catch (cause) {
      console.error("Failed to pick mod folder.", {
        cause,
        formatted: formatError(cause),
      });
      setError(`Failed to open folder picker: ${formatError(cause)}`);
    }
  }

  function clearModFolder() {
    setModPath("");
  }

  useEffect(() => {
    writeStoredPath(GAME_PATH_STORAGE_KEY, gamePath);
  }, [gamePath]);

  useEffect(() => {
    writeStoredPath(MOD_PATH_STORAGE_KEY, modPath);
  }, [modPath]);

  useEffect(() => {
    if (!parsedSavefile) {
      setCountryCatalog(null);
      setCountryWarnings([]);
      setIsResolvingCountries(false);
      return;
    }

    let isCancelled = false;
    setIsResolvingCountries(true);

    invoke<CountryCatalogView>("resolve_country_catalog", {
      gamePath,
      modPath: modPath || null,
      countryTags: parsedSavefile.countryTags,
    })
      .then((nextCatalog) => {
        if (isCancelled) {
          return;
        }

        setCountryCatalog(nextCatalog);
        setCountryWarnings(nextCatalog.warnings);
      })
      .catch((cause) => {
        if (isCancelled) {
          return;
        }

        console.error("Failed to resolve country catalog.", {
          cause,
          formatted: formatError(cause),
        });

        setCountryCatalog(null);
        setCountryWarnings([
          `Failed to resolve country data: ${formatError(cause)}`,
        ]);
      })
      .finally(() => {
        if (!isCancelled) {
          setIsResolvingCountries(false);
        }
      });

    return () => {
      isCancelled = true;
    };
  }, [gamePath, modPath, parsedSavefile]);

  const selectedWar =
    parsedSavefile && selectedWarSection
      ? parsedSavefile[selectedWarSection][selectedWarIndex] ?? null
      : null;
  const selectedBattle = selectedWar?.battles[selectedBattleIndex] ?? null;
  const totalWarCount = parsedSavefile
    ? parsedSavefile.activeWars.length + parsedSavefile.previousWars.length
    : 0;
  const countryStatus = parsedSavefile
    ? isResolvingCountries
      ? "Refreshing"
      : countryWarnings.length
        ? "Loaded with warnings"
        : "Ready"
    : "Waiting";

  function handleWarSelection(section: WarSectionKey, warIndex: number) {
    setSelectedWarSection(section);
    setSelectedWarIndex(warIndex);
    setSelectedBattleIndex(0);
  }

  useEffect(() => {
    if (battleListScrollRef.current) {
      battleListScrollRef.current.scrollTop = 0;
    }
  }, [selectedWarSection, selectedWarIndex]);

  return (
    <main className="app-shell">
      <section className="hero-panel">
        <div className="hero-copy">
          <p className="eyebrow">Victoria 2 War Analyzer</p>
          <h1>War losses command table</h1>
          <p className="lead">
            Open a savefile to scan active and previous wars, drill into their
            battles, and resolve country names and flags from the selected game
            data sources.
          </p>
        </div>

        <div className="hero-actions">
          <div className="action-row">
            <button
              className="primary-button"
              disabled={isParsing}
              onClick={pickSavefile}
              type="button"
            >
              {isParsing
                ? "Parsing savefile..."
                : parsedSavefile
                  ? "Pick another savefile"
                  : "Pick savefile"}
            </button>
          </div>

          <div className="path-controls">
            <section className="path-card">
              <div className="path-card__header">
                <div>
                  <p className="status-label">Base game folder</p>
                  <p className="path-card__value">{gamePath}</p>
                </div>
              </div>
              <div className="button-row">
                <button
                  className="secondary-button"
                  onClick={pickGameFolder}
                  type="button"
                >
                  Change game folder
                </button>
              </div>
            </section>

            <section className="path-card">
              <div className="path-card__header">
                <div>
                  <p className="status-label">Mod folder</p>
                  <p className="path-card__value">
                    {modPath || "Base game only"}
                  </p>
                </div>
              </div>
              <div className="button-row">
                <button
                  className="secondary-button"
                  onClick={pickModFolder}
                  type="button"
                >
                  Pick mod folder
                </button>
                <button
                  className="ghost-button"
                  disabled={!modPath}
                  onClick={clearModFolder}
                  type="button"
                >
                  Base game only
                </button>
              </div>
            </section>
          </div>

          <div className="hero-stats">
            <div className="status-card status-card--wide">
              <p className="status-label">Selected file</p>
              <p className="path-value">
                {selectedPath || "No savefile selected yet."}
              </p>
            </div>

            <div className="status-card">
              <p className="status-label">Parse status</p>
              <p className="status-value">
                {isParsing
                  ? "Reading campaign"
                  : parsedSavefile
                    ? "Campaign loaded"
                    : "Waiting"}
              </p>
            </div>

            <div className="status-card">
              <p className="status-label">Country data</p>
              <p className="status-value">{countryStatus}</p>
            </div>

            <div className="status-card">
              <p className="status-label">Wars loaded</p>
              <p className="status-value">
                {parsedSavefile ? integerFormatter.format(totalWarCount) : "0"}
              </p>
            </div>

            <div className="status-card">
              <p className="status-label">Countries tracked</p>
              <p className="status-value">
                {parsedSavefile
                  ? integerFormatter.format(parsedSavefile.countryTags.length)
                  : "0"}
              </p>
            </div>
          </div>
        </div>
      </section>

      {error ? <p className="error-text error-banner">{error}</p> : null}

      {countryWarnings.length ? (
        <section className="warning-banner">
          <p className="warning-title">Country data warnings</p>
          <ul className="warning-list">
            {countryWarnings.map((warning) => (
              <li key={warning}>{warning}</li>
            ))}
          </ul>
        </section>
      ) : null}

      {parsedSavefile ? (
        <section className="workspace">
          <section className="panel panel--wars">
            <div className="panel-header">
              <div>
                <p className="panel-kicker">Step 1</p>
                <h2>Wars</h2>
              </div>
              <p className="panel-note">Choose a conflict to inspect.</p>
            </div>

            <div className="panel-body panel-body--list">
              <div className="panel-scroll panel-scroll--wars">
                {WAR_SECTION_ORDER.map((sectionKey) => {
                  const wars = parsedSavefile[sectionKey];
                  const isSelectedSection = selectedWarSection === sectionKey;

                  return (
                    <section className="section-block" key={sectionKey}>
                      <div className="section-heading">
                        <div>
                          <p className="section-kicker">
                            {SECTION_COPY[sectionKey].tone}
                          </p>
                          <h3>{SECTION_COPY[sectionKey].title}</h3>
                        </div>
                        <span className="count-pill">
                          {integerFormatter.format(wars.length)}
                        </span>
                      </div>

                      {wars.length ? (
                        <div className="list-stack">
                          {wars.map((war, warIndex) => {
                            const isSelected =
                              isSelectedSection && selectedWarIndex === warIndex;
                            const warDateRange = formatWarDateRange(
                              war.startDate,
                              war.endDate,
                            );

                            return (
                              <button
                                className={`list-card ${isSelected ? "is-selected" : ""}`}
                                key={`${sectionKey}-${war.name}-${warIndex}`}
                                onClick={() =>
                                  handleWarSelection(sectionKey, warIndex)
                                }
                                type="button"
                              >
                                <div className="list-card__header">
                                  <h4>{war.name}</h4>
                                  <span className="kind-badge">
                                    {war.kind === "active"
                                      ? "Active"
                                      : "Previous"}
                                  </span>
                                </div>

                                <div className="list-card__summary">
                                  <div className="country-line">
                                    <span className="country-line__label">
                                      Attackers
                                    </span>
                                    <CountryInlineList
                                      catalog={countryCatalog}
                                      emptyLabel="Unknown"
                                      tags={war.attackers}
                                    />
                                  </div>

                                  <div className="country-line">
                                    <span className="country-line__label">
                                      Defenders
                                    </span>
                                    <CountryInlineList
                                      catalog={countryCatalog}
                                      emptyLabel="Unknown"
                                      tags={war.defenders}
                                    />
                                  </div>
                                </div>

                                {warDateRange ? (
                                  <p className="list-card__summary list-card__summary--compact">
                                    <span>{warDateRange}</span>
                                  </p>
                                ) : null}

                                <div className="metric-row">
                                  <span>{formatBattleCount(war.battleCount)}</span>
                                  <span>
                                    {formatLosses(war.totalLosses)} total losses
                                  </span>
                                </div>
                                <div className="metric-row metric-row--detail">
                                  <span>
                                    Attackers lost{" "}
                                    {formatLosses(war.attackerTotalLosses)}
                                  </span>
                                  <span>
                                    Defenders lost{" "}
                                    {formatLosses(war.defenderTotalLosses)}
                                  </span>
                                </div>
                              </button>
                            );
                          })}
                        </div>
                      ) : (
                        <p className="empty-copy">{SECTION_COPY[sectionKey].empty}</p>
                      )}
                    </section>
                  );
                })}
              </div>
            </div>
          </section>

          <section className="panel panel--battles">
            <div className="panel-header">
              <div>
                <p className="panel-kicker">Step 2</p>
                <h2>Battles</h2>
              </div>
              <p className="panel-note">
                {selectedWar
                  ? `${selectedWar.name} contains ${formatBattleCount(selectedWar.battleCount).toLowerCase()}.`
                  : "Choose a war to populate the battle list."}
              </p>
            </div>

            <div className="panel-body panel-body--list">
              <div
                className="panel-scroll panel-scroll--battles"
                ref={battleListScrollRef}
              >
                {selectedWar ? (
                  selectedWar.battles.length ? (
                    <div className="list-stack">
                      {selectedWar.battles.map((battle, battleIndex) => (
                        <button
                          className={`list-card ${selectedBattleIndex === battleIndex ? "is-selected" : ""}`}
                          key={`${battle.name}-${battle.locationId}-${battleIndex}`}
                          onClick={() => setSelectedBattleIndex(battleIndex)}
                          type="button"
                        >
                          <div className="list-card__header">
                            <h4>{battle.name}</h4>
                            <div className="pill-stack">
                              <WinnerPill winner={battle.winner} />
                              <span className="loss-pill">
                                {formatLosses(battle.totalLosses)} losses
                              </span>
                            </div>
                          </div>
                          <div className="list-card__summary">
                            <span>{battle.locationLabel}</span>
                            <div className="country-versus">
                              <CountryLabel
                                catalog={countryCatalog}
                                emptyLabel="Unknown attacker"
                                tag={battle.attacker.country}
                              />
                              <span className="country-versus__separator">
                                vs
                              </span>
                              <CountryLabel
                                catalog={countryCatalog}
                                emptyLabel="Unknown defender"
                                tag={battle.defender.country}
                              />
                            </div>
                          </div>
                        </button>
                      ))}
                    </div>
                  ) : (
                    <EmptyPanel
                      title="No battles recorded"
                      copy="The selected war parsed correctly, but its history block did not include any battle entries."
                    />
                  )
                ) : (
                  <EmptyPanel
                    title="No war selected"
                    copy="Select a war from the left panel to inspect its battles."
                  />
                )}
              </div>
            </div>
          </section>

          <section className="panel panel--detail">
            <div className="panel-header">
              <div>
                <p className="panel-kicker">Step 3</p>
                <h2>Battle Breakdown</h2>
              </div>
              <p className="panel-note">
                {selectedBattle
                  ? "Compare both sides and inspect the unit composition."
                  : "Choose a battle to inspect its losses and unit counts."}
              </p>
            </div>

            <div className="panel-body panel-body--detail">
              <div className="panel-scroll panel-scroll--detail">
                {selectedWar && selectedBattle ? (
                  <div className="detail-stack">
                    <section className="battle-summary">
                      <div>
                        <p className="section-kicker">{selectedWar.name}</p>
                        <h3>{selectedBattle.name}</h3>
                      </div>
                      <div className="battle-summary__stats">
                        <span>{selectedBattle.locationLabel}</span>
                        <WinnerPill winner={selectedBattle.winner} />
                        <strong>
                          {formatLosses(selectedBattle.totalLosses)} total losses
                        </strong>
                      </div>
                    </section>

                    <div className="side-grid">
                      <SideCard
                        catalog={countryCatalog}
                        label="Attacker"
                        side={selectedBattle.attacker}
                      />
                      <SideCard
                        catalog={countryCatalog}
                        label="Defender"
                        side={selectedBattle.defender}
                      />
                    </div>

                    <section className="unit-breakdown">
                      <div className="section-heading section-heading--detail">
                        <div>
                          <p className="section-kicker">Force comparison</p>
                          <h3>Unit kinds in battle</h3>
                        </div>
                        <span className="count-pill">
                          {integerFormatter.format(
                            selectedBattle.unitBreakdown.length,
                          )}
                        </span>
                      </div>

                      {selectedBattle.unitBreakdown.length ? (
                        <UnitBreakdownTable rows={selectedBattle.unitBreakdown} />
                      ) : (
                        <p className="empty-copy">
                          No unit-kind counts were recorded for this battle.
                        </p>
                      )}
                    </section>
                  </div>
                ) : (
                  <EmptyPanel
                    title="No battle selected"
                    copy="Select a battle from the middle panel to see side losses and the unit-kind comparison."
                  />
                )}
              </div>
            </div>
          </section>
        </section>
      ) : (
        <section className="empty-stage">
          <div className="empty-stage__card">
            <p className="section-kicker">Ready room</p>
            <h2>Load a campaign to begin</h2>
            <p>
              This first version focuses on trustworthy battle data: total
              losses by war and battle, plus attacker-versus-defender unit
              counts for each recorded engagement.
            </p>
          </div>
        </section>
      )}
    </main>
  );
}

function SideCard({
  catalog,
  label,
  side,
}: {
  catalog: CountryCatalogView | null;
  label: string;
  side: BattleSideView;
}) {
  return (
    <section className="side-card">
      <div className="side-card__header">
        <p className="section-kicker">{label}</p>
        <h3>
          <CountryLabel
            catalog={catalog}
            emptyLabel="Unknown country"
            tag={side.country}
          />
        </h3>
      </div>

      <dl className="side-card__stats">
        <div>
          <dt>Leader</dt>
          <dd>{side.leader ?? "No leader recorded"}</dd>
        </div>
        <div>
          <dt>Losses</dt>
          <dd>{formatOptionalLosses(side.losses)}</dd>
        </div>
      </dl>
    </section>
  );
}

function CountryInlineList({
  catalog,
  emptyLabel,
  tags,
}: {
  catalog: CountryCatalogView | null;
  emptyLabel: string;
  tags: string[];
}) {
  if (!tags.length) {
    return <span>{emptyLabel}</span>;
  }

  return (
    <span className="country-list">
      {tags.map((tag, index) => (
        <CountryLabel
          catalog={catalog}
          className="country-label country-list__item"
          emptyLabel={emptyLabel}
          key={`${tag}-${index}`}
          tag={tag}
        />
      ))}
    </span>
  );
}

function CountryLabel({
  catalog,
  className = "country-label",
  emptyLabel,
  tag,
}: {
  catalog: CountryCatalogView | null;
  className?: string;
  emptyLabel: string;
  tag: string | null;
}) {
  if (!tag) {
    return <span className={className}>{emptyLabel}</span>;
  }

  const country = getCountry(catalog, tag);

  return (
    <span className={className}>
      {country.flagDataUrl ? (
        <img
          alt=""
          aria-hidden="true"
          className="country-flag"
          src={country.flagDataUrl}
        />
      ) : null}
      <span>{country.name}</span>
    </span>
  );
}

function WinnerPill({ winner }: { winner: BattleWinner }) {
  return (
    <span className={`winner-pill winner-pill--${winner}`}>
      {formatBattleWinner(winner)}
    </span>
  );
}

function UnitBreakdownTable({ rows }: { rows: UnitBreakdownRowView[] }) {
  const maxUnitCount = rows.reduce((max, row) => {
    return Math.max(max, row.attackerCount, row.defenderCount);
  }, 1);

  return (
    <div className="unit-table-shell">
      <table className="unit-table">
        <thead>
          <tr>
            <th scope="col">Attacker</th>
            <th scope="col">Unit kind</th>
            <th scope="col">Defender</th>
          </tr>
        </thead>
        <tbody>
          {rows.map((row) => (
            <tr key={row.unitKind}>
              <td>
                <div className="unit-value">
                  <span>{integerFormatter.format(row.attackerCount)}</span>
                  <div className="unit-meter unit-meter--attacker">
                    <span
                      style={{
                        width: `${(row.attackerCount / maxUnitCount) * 100}%`,
                      }}
                    />
                  </div>
                </div>
              </td>
              <th className="unit-name" scope="row">
                {formatUnitKind(row.unitKind)}
              </th>
              <td>
                <div className="unit-value unit-value--defender">
                  <span>{integerFormatter.format(row.defenderCount)}</span>
                  <div className="unit-meter unit-meter--defender">
                    <span
                      style={{
                        width: `${(row.defenderCount / maxUnitCount) * 100}%`,
                      }}
                    />
                  </div>
                </div>
              </td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

function EmptyPanel({ title, copy }: { title: string; copy: string }) {
  return (
    <div className="empty-panel">
      <h3>{title}</h3>
      <p>{copy}</p>
    </div>
  );
}

function getCountry(
  catalog: CountryCatalogView | null,
  tag: string,
): CountryView {
  return catalog?.countries[tag] ?? {
    tag,
    name: tag,
    flagDataUrl: null,
  };
}

function getDefaultSelection(
  savefile: ParsedSavefileView,
): WarSelection | null {
  for (const section of WAR_SECTION_ORDER) {
    if (savefile[section].length > 0) {
      return {
        section,
        warIndex: 0,
      };
    }
  }

  return null;
}

function readStoredPath(key: string): string {
  try {
    return window.localStorage.getItem(key) ?? "";
  } catch {
    return "";
  }
}

function writeStoredPath(key: string, value: string) {
  try {
    if (value) {
      window.localStorage.setItem(key, value);
    } else {
      window.localStorage.removeItem(key);
    }
  } catch {
    // Ignore storage failures and keep the app usable.
  }
}

function formatError(cause: unknown): string {
  if (cause instanceof Error) {
    return cause.message;
  }

  if (typeof cause === "string") {
    return cause;
  }

  try {
    return JSON.stringify(cause);
  } catch {
    return String(cause);
  }
}

function formatLosses(value: number): string {
  return integerFormatter.format(value);
}

function formatOptionalLosses(value: number | null): string {
  return value === null ? "Unknown" : `${formatLosses(value)} troops`;
}

function formatBattleCount(value: number): string {
  return `${integerFormatter.format(value)} ${value === 1 ? "battle" : "battles"}`;
}

function formatWarDateRange(
  startDate: string | null,
  endDate: string | null,
): string | null {
  if (!startDate && !endDate) {
    return null;
  }

  if (startDate && endDate) {
    return startDate === endDate
      ? `War date: ${startDate}`
      : `War dates: ${startDate} to ${endDate}`;
  }

  return `War date: ${startDate ?? endDate}`;
}

function formatBattleWinner(value: BattleWinner): string {
  switch (value) {
    case "attacker":
      return "Attacker victory";
    case "defender":
      return "Defender victory";
    case "unknown":
      return "Winner unknown";
  }
}

function formatUnitKind(value: string): string {
  return value
    .split("_")
    .map((segment) =>
      segment ? segment[0].toUpperCase() + segment.slice(1) : segment,
    )
    .join(" ");
}

export default App;
