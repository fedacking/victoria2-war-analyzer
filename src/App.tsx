import { useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";

function App() {
  const [selectedPath, setSelectedPath] = useState<string>("");
  const [error, setError] = useState<string>("");

  function formatDialogError(cause: unknown): string {
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
        return;
      }

      setSelectedPath("");
    } catch (cause) {
      console.error("Failed to open savefile dialog.", {
        cause,
        formatted: formatDialogError(cause),
      });

      setError(
        `Failed to open file picker: ${formatDialogError(cause)}`,
      );
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

        <button className="primary-button" onClick={pickSavefile} type="button">
          Pick savefile
        </button>

        <div className="status-card">
          <h2>Selected file</h2>
          <p className="path-value">
            {selectedPath || "No savefile selected yet."}
          </p>
        </div>

        {error ? <p className="error-text">{error}</p> : null}
      </section>
    </main>
  );
}

export default App;
