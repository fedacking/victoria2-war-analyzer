import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { open } from "@tauri-apps/plugin-dialog";

type ParseSummary = {
  path: string;
  topLevelStatementCount: number;
};

function App() {
  const [selectedPath, setSelectedPath] = useState<string>("");
  const [parseSummary, setParseSummary] = useState<ParseSummary | null>(null);
  const [isParsing, setIsParsing] = useState<boolean>(false);
  const [error, setError] = useState<string>("");

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

  async function pickSavefile() {
    setError("");
    setParseSummary(null);

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

      if (typeof selected === "string") {
        setSelectedPath(selected);
        setIsParsing(true);

        try {
          const summary = await invoke<ParseSummary>("parse_savefile", {
            path: selected,
          });

          setParseSummary(summary);
          setError("");
        } catch (cause) {
          console.error("Failed to parse savefile.", {
            cause,
            formatted: formatError(cause),
          });

          setParseSummary(null);
          setError(`Failed to parse savefile: ${formatError(cause)}`);
        } finally {
          setIsParsing(false);
        }

        return;
      }

      setSelectedPath("");
      setParseSummary(null);
    } catch (cause) {
      console.error("Failed to open savefile dialog.", {
        cause,
        formatted: formatError(cause),
      });

      setError(`Failed to open file picker: ${formatError(cause)}`);
    }
  }

  return (
    <main className="app-shell">
      <section className="panel">
        <p className="eyebrow">Victoria 2 War Analyzer</p>
        <h1>Open a savefile</h1>
        <p className="lead">
          Start by choosing a Victoria 2 save so we can inspect its countries,
          wars, and military data.
        </p>

        <button
          className="primary-button"
          disabled={isParsing}
          onClick={pickSavefile}
          type="button"
        >
          {isParsing ? "Parsing savefile..." : "Pick savefile"}
        </button>

        <div className="status-card">
          <h2>Selected file</h2>
          <p className="path-value">
            {selectedPath || "No savefile selected yet."}
          </p>
        </div>

        <div className="status-card">
          <h2>Parse status</h2>
          <p className="path-value">
            {isParsing
              ? "Rust is reading and parsing the selected savefile."
              : parseSummary
                ? `Parse succeeded with ${parseSummary.topLevelStatementCount} top-level statements.`
                : "No parse result yet."}
          </p>
          {parseSummary ? (
            <p className="success-text">
              Confirmed path: {parseSummary.path}
            </p>
          ) : null}
        </div>

        {error ? <p className="error-text">{error}</p> : null}
      </section>
    </main>
  );
}

export default App;
