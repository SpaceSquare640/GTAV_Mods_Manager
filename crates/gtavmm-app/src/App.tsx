import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import "./App.css";

type ModStatus = "Active" | "Disabled" | "Uninstalled";

interface InstalledMod {
  id: number;
  name: string;
  source_type: string;
  install_path: string;
  installed_at: string;
  status: ModStatus;
  notes: string | null;
  link: string | null;
}

type DetectGameResult =
  | { status: "found"; install_path: string; edition: string }
  | { status: "not_found" };

function App() {
  const [detected, setDetected] = useState<DetectGameResult | null>(null);
  const [mods, setMods] = useState<InstalledMod[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    // Real IPC round-trip to gtavmm-core, not mocked — see src-tauri/src/commands.rs.
    invoke<DetectGameResult>("detect_game")
      .then(setDetected)
      .catch((e) => setError(String(e)));
    invoke<InstalledMod[]>("list_mods")
      .then(setMods)
      .catch((e) => setError(String(e)));
  }, []);

  return (
    <main className="container">
      <h1>GTAV Mods Manager</h1>
      <p className="scaffold-note">
        Tauri + React skeleton — real IPC to the Rust core (<code>detect_game</code>,{" "}
        <code>list_mods</code>), not the design mockup. Most pages/features from the
        HTML mockup are not built here yet.
      </p>

      {error && <p className="error">Error: {error}</p>}

      <section>
        <h2>Game detection</h2>
        {detected === null && !error && <p>Detecting…</p>}
        {detected?.status === "found" && (
          <p>
            Found ({detected.edition}): <code>{detected.install_path}</code>
          </p>
        )}
        {detected?.status === "not_found" && <p>No GTA V installation detected.</p>}
      </section>

      <section>
        <h2>Installed mods</h2>
        {mods === null && !error && <p>Loading…</p>}
        {mods?.length === 0 && <p>(no mods installed yet)</p>}
        {mods && mods.length > 0 && (
          <ul>
            {mods.map((m) => (
              <li key={m.id}>
                #{m.id} {m.name} [{m.status}]
              </li>
            ))}
          </ul>
        )}
      </section>
    </main>
  );
}

export default App;
