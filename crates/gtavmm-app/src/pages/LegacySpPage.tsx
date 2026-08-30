import { Fragment, useCallback, useEffect, useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "../components/IconSprite";
import { InstallWizard } from "../components/InstallWizard";
import type { InstalledMod } from "../types";

export function LegacySpPage() {
  const { t } = useTranslation();
  const [mods, setMods] = useState<InstalledMod[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [wizardOpen, setWizardOpen] = useState(false);
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editNotes, setEditNotes] = useState("");
  const [editLink, setEditLink] = useState("");

  const loadMods = useCallback(() => {
    invoke<InstalledMod[]>("list_mods")
      .then(setMods)
      .catch((e) => setError(String(e)));
  }, []);

  function startEditDetails(m: InstalledMod) {
    setEditingId(m.id);
    setEditNotes(m.notes ?? "");
    setEditLink(m.link ?? "");
  }

  async function saveDetails(modId: number) {
    try {
      await invoke("update_mod_details", {
        modId,
        notes: editNotes.trim() || null,
        link: editLink.trim() || null,
      });
      setEditingId(null);
      loadMods();
    } catch (e) {
      setError(String(e));
    }
  }

  useEffect(() => {
    loadMods();
  }, [loadMods]);

  const active = mods?.filter((m) => m.status === "Active").length ?? 0;
  const disabled = mods?.filter((m) => m.status === "Disabled").length ?? 0;

  const statusLabel: Record<InstalledMod["status"], string> = {
    Active: t("legacySp.status_active"),
    Disabled: t("legacySp.status_disabled"),
    Uninstalled: t("legacySp.status_uninstalled"),
  };
  const statusClass: Record<InstalledMod["status"], string> = {
    Active: "active",
    Disabled: "disabled",
    Uninstalled: "disabled",
  };

  return (
    <section className="view" data-shown="true">
      <div className="page-head">
        <div>
          <h1 className="page-title">{t("legacySp.title")}</h1>
          <p className="page-sub">
            <Trans
              i18nKey="legacySp.subtitle"
              components={{ mono: <span className="mono" />, strong: <strong /> }}
            />
          </p>
        </div>
        <button className="btn-primary" type="button" onClick={() => setWizardOpen(true)}>
          {t("legacySp.install_mod_button")}
        </button>
      </div>

      <InstallWizard
        open={wizardOpen}
        onClose={() => setWizardOpen(false)}
        onInstalled={loadMods}
      />

      {error && <p className="error">{t("legacySp.loadError", { error })}</p>}

      <div className="stat-row">
        <div className="stat-card">
          <div className="eyebrow">{t("legacySp.stat_installed")}</div>
          <div className="value">{mods?.length ?? "—"}</div>
        </div>
        <div className="stat-card">
          <div className="eyebrow">{t("legacySp.stat_active")}</div>
          <div className="value">{mods ? active : "—"}</div>
        </div>
        <div className="stat-card">
          <div className="eyebrow">{t("legacySp.stat_disabled")}</div>
          <div className="value">{mods ? disabled : "—"}</div>
        </div>
      </div>

      <div className="panel">
        <div className="panel-head">
          <h2>{t("legacySp.panel_title")}</h2>
        </div>
        {mods === null && !error && (
          <p style={{ padding: "16px 20px" }}>{t("legacySp.loading")}</p>
        )}
        {mods && mods.length === 0 && (
          <div className="empty-state">
            <span className="glyph">
              <Icon name="folder" />
            </span>
            <h3>{t("legacySp.empty_title")}</h3>
            <p>{t("legacySp.empty_body")}</p>
          </div>
        )}
        {mods && mods.length > 0 && (
          <table>
            <thead>
              <tr>
                <th>{t("legacySp.col_mod")}</th>
                <th>{t("legacySp.col_status")}</th>
                <th>{t("legacySp.col_installed")}</th>
                <th>{t("legacySp.col_root")}</th>
                <th>{t("legacySp.col_link")}</th>
              </tr>
            </thead>
            <tbody>
              {mods.map((m) => (
                <Fragment key={m.id}>
                  <tr className="mod-row">
                    <td>
                      <div className="mod-name">{m.name}</div>
                      <div className="mod-type">.{m.source_type}</div>
                    </td>
                    <td>
                      <span className={`pill ${statusClass[m.status]}`}>
                        {statusLabel[m.status]}
                      </span>
                    </td>
                    <td className="mono">{m.installed_at.slice(0, 10)}</td>
                    <td className="path mono">{m.install_path}</td>
                    <td>
                      {m.link ? (
                        <a href={m.link} target="_blank" rel="noopener noreferrer" className="mono" style={{ fontSize: "11.5px" }}>
                          {t("legacySp.open_link")}
                        </a>
                      ) : (
                        <span className="path">—</span>
                      )}{" "}
                      <button className="icon-btn" type="button" onClick={() => startEditDetails(m)}>
                        {t("legacySp.edit_link_button")}
                      </button>
                    </td>
                  </tr>
                  {editingId === m.id && (
                    <tr>
                      <td colSpan={5} style={{ background: "var(--surface-2)" }}>
                        <div style={{ display: "flex", gap: 8, padding: "10px 4px", flexWrap: "wrap", alignItems: "center" }}>
                          <input
                            type="text"
                            value={editLink}
                            onChange={(e) => setEditLink(e.target.value)}
                            placeholder={t("legacySp.link_placeholder")}
                            style={{ flex: 1, minWidth: 200 }}
                          />
                          <input
                            type="text"
                            value={editNotes}
                            onChange={(e) => setEditNotes(e.target.value)}
                            placeholder={t("legacySp.notes_placeholder")}
                            style={{ flex: 1, minWidth: 200 }}
                          />
                          <button className="btn-primary" type="button" onClick={() => saveDetails(m.id)}>
                            {t("legacySp.save_link_button")}
                          </button>
                          <button className="icon-btn" type="button" onClick={() => setEditingId(null)}>
                            {t("legacySp.cancel_link_button")}
                          </button>
                        </div>
                      </td>
                    </tr>
                  )}
                </Fragment>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </section>
  );
}
