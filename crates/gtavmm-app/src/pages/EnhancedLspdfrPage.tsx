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

export function EnhancedLspdfrPage() {
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
            LSPDFR · Enhanced{" "}
            <span className="lspdfr-badge">
              <Icon name="shield" /> RAGE Plugin Hook
            </span>{" "}
            <span className="beta-chip">
              <Icon name="alert-triangle" /> Public Preview
            </span>
          </h1>
          <p className="page-sub">
            RPH's Enhanced-edition support is new (Public Preview since 2026-04) — treat
            everything here as more likely to need a manual fix than the Legacy equivalent.
            Same <span className="mono">list_mods</span> IPC as every other workspace — no
            per-mode column yet, so this shows every installed mod.
          </p>
        </div>
      </div>

      <div className="info-banner warn">
        <svg className="icon" style={{ fontSize: 15 }}>
          <use href="#i-alert-triangle" />
        </svg>
        <span>
          <strong>Beta-grade support:</strong> this app's Enhanced LSPDFR conventions (the{" "}
          <span className="mono">Plugins\</span> folder, add-on-pack mirroring) are carried
          over from the Legacy provider on the assumption they still hold — they haven't been
          individually re-verified against a real Enhanced RPH install. Expect rougher edges
          than every other page in this app; check installs in-game before trusting them.
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
