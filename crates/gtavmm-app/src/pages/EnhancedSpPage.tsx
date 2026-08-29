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

export function EnhancedSpPage() {
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
          <h1 className="page-title">SP Mods · Enhanced</h1>
          <p className="page-sub">
            Real data from <span className="mono">list_mods</span> (Tauri IPC → gtavmm-core).{" "}
            <strong>Honesty note:</strong> same as Legacy/SP — the database doesn't yet tag
            which mode/edition a mod belongs to, so this is currently the exact same
            underlying list, just relabeled. A real per-edition split needs a schema change
            that hasn't been made yet.
          </p>
        </div>
      </div>

      <div
        className="info-banner"
        style={{
          borderColor: "color-mix(in srgb, var(--accent-enhanced) 40%, var(--border))",
          background: "color-mix(in srgb, var(--accent-enhanced) 8%, var(--surface))",
        }}
      >
        <svg className="icon" style={{ fontSize: 15 }}>
          <use href="#i-info" />
        </svg>
        <span>
          <strong>Unverified assumption:</strong> add-on vehicle/map packs are assumed to
          mirror Legacy's <span className="mono">mods\update\x64\dlcpacks\</span> layout.
          Which RPF a pack's <span className="mono">dlclist.xml</span> entry needs internally
          hasn't been confirmed against a real install with RPF-inspection tooling —
          installs of this type should be double-checked in game. (Carried over from the
          `EnhancedSpProvider` module doc comment, not invented for this page.)
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
            <p>Install wizard isn't ported to this real app yet — use the CLI's `install` command for now.</p>
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
