import { useEffect, useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "../components/IconSprite";
import type { InstalledMod } from "../types";

function statusPill(status: InstalledMod["status"]) {
  const map = {
    Active: { cls: "active", label: "Active" },
    Disabled: { cls: "disabled", label: "Disabled" },
    Uninstalled: { cls: "disabled", label: "Uninstalled" },
  } as const;
  const { cls, label } = map[status];
  return <span className={`pill ${cls}`}>{label}</span>;
}

export function LegacyLspdfrPage() {
  const [mods, setMods] = useState<InstalledMod[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<InstalledMod[]>("list_mods")
      .then(setMods)
      .catch((e) => setError(String(e)));
  }, []);

  const active = mods?.filter((m) => m.status === "Active").length ?? 0;
  const disabled = mods?.filter((m) => m.status === "Disabled").length ?? 0;

  return (
    <section className="view" data-shown="true">
      <div className="page-head">
        <div>
          <h1 className="page-title">
            LSPDFR · Legacy{" "}
            <span className="lspdfr-badge">
              <Icon name="shield" /> RAGE Plugin Hook
            </span>
          </h1>
          <p className="page-sub">
            Managed through <span className="mono">RAGEPluginHook.exe</span>, not the game's
            own launcher. Callouts and plugins install to <span className="mono">Plugins\</span>
            . Same <span className="mono">list_mods</span> IPC as the other workspaces — no
            per-mode column in the database yet, so this shows every installed mod.
          </p>
        </div>
      </div>

      <div className="info-banner">
        <svg className="icon" style={{ fontSize: 15 }}>
          <use href="#i-info" />
        </svg>
        <span>
          <strong>Known limitation:</strong> callout packs and other RAGE Plugin Hook plugins
          both install as a managed <span className="mono">.dll</span>, so this app can't yet
          tell them apart automatically — everything lands in <span className="mono">Plugins\</span>{" "}
          rather than the community's <span className="mono">Plugins\LSPDFR\</span> subfolder
          for callouts specifically. Move a callout there yourself if a pack expects it.
          (Confirmed against a real LSPDFR install's backup — see the provider module docs;
          this specific callout-subfolder nuance is the one part not yet automated, not an
          unverified guess about the whole convention.)
        </span>
      </div>

      {error && <p className="error">Failed to load mods: {error}</p>}

      <div className="stat-row">
        <div className="stat-card">
          <div className="eyebrow">Installed</div>
          <div className="value">{mods?.length ?? "—"}</div>
        </div>
        <div className="stat-card">
          <div className="eyebrow">Active</div>
          <div className="value">{mods ? active : "—"}</div>
        </div>
        <div className="stat-card">
          <div className="eyebrow">Disabled</div>
          <div className="value">{mods ? disabled : "—"}</div>
        </div>
      </div>

      <div className="panel">
        <div className="panel-head">
          <h2>Installed mods</h2>
        </div>
        {mods === null && !error && <p style={{ padding: "16px 20px" }}>Loading…</p>}
        {mods && mods.length === 0 && (
          <div className="empty-state">
            <span className="glyph">
              <Icon name="folder" />
            </span>
            <h3>No mods installed yet</h3>
            <p>
              Install wizard isn't ported to this real app yet — use the CLI's `install`
              command for now.
            </p>
          </div>
        )}
        {mods && mods.length > 0 && (
          <table>
            <thead>
              <tr>
                <th>Mod</th>
                <th>Status</th>
                <th>Installed</th>
                <th>Install root</th>
              </tr>
            </thead>
            <tbody>
              {mods.map((m) => (
                <tr key={m.id} className="mod-row">
                  <td>
                    <div className="mod-name">{m.name}</div>
                    <div className="mod-type">.{m.source_type}</div>
                  </td>
                  <td>{statusPill(m.status)}</td>
                  <td className="mono">{m.installed_at.slice(0, 10)}</td>
                  <td className="path mono">{m.install_path}</td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </section>
  );
}
