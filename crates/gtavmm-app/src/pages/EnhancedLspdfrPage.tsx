import { useEffect, useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "../components/IconSprite";
import type { InstalledMod } from "../types";

export function EnhancedLspdfrPage() {
  const { t } = useTranslation();
  const [mods, setMods] = useState<InstalledMod[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<InstalledMod[]>("list_mods")
      .then(setMods)
      .catch((e) => setError(String(e)));
  }, []);

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
          <h1 className="page-title">
            {t("enhancedLspdfr.title")}{" "}
            <span className="lspdfr-badge">
              <Icon name="shield" /> {t("legacyLspdfr.badge")}
            </span>{" "}
            <span className="beta-chip">
              <Icon name="alert-triangle" /> {t("enhancedLspdfr.preview_badge")}
            </span>
          </h1>
          <p className="page-sub">
            <Trans i18nKey="enhancedLspdfr.subtitle" components={{ mono: <span className="mono" /> }} />
          </p>
        </div>
      </div>

      <div className="info-banner warn">
        <svg className="icon" style={{ fontSize: 15 }}>
          <use href="#i-alert-triangle" />
        </svg>
        <span>
          <Trans
            i18nKey="enhancedLspdfr.beta_support"
            components={{ mono: <span className="mono" />, strong: <strong /> }}
          />
        </span>
      </div>

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
            <p>{t("enhancedSp.empty_body")}</p>
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
              </tr>
            </thead>
            <tbody>
              {mods.map((m) => (
                <tr key={m.id} className="mod-row">
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
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </section>
  );
}
