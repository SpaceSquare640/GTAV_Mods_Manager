import { useCallback, useEffect, useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "../components/IconSprite";
import { InstallWizard } from "../components/InstallWizard";
import { ModTable } from "../components/ModTable";
import { ModPageTools } from "../components/ModPageTools";
import { pickSaveFile } from "../lib/pickers";
import type { InstalledMod } from "../types";

export function LegacySpPage() {
  const { t } = useTranslation();
  const [mods, setMods] = useState<InstalledMod[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [wizardOpen, setWizardOpen] = useState(false);
  const [exportBusy, setExportBusy] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);

  async function exportToXlsx() {
    const picked = await pickSaveFile("gtavmm-mods.xlsx", ["xlsx"], t("legacySp.export_pick_title"));
    if (!picked) return;
    setExportBusy(true);
    setExportError(null);
    try {
      await invoke("export_mods_to_xlsx", { outputPath: picked });
    } catch (e) {
      setExportError(String(e));
    } finally {
      setExportBusy(false);
    }
  }

  const loadMods = useCallback(() => {
    invoke<InstalledMod[]>("list_mods")
      .then(setMods)
      .catch((e) => setError(String(e)));
  }, []);

  useEffect(() => {
    loadMods();
  }, [loadMods]);

  const active = mods?.filter((m) => m.status === "Active").length ?? 0;
  const disabled = mods?.filter((m) => m.status === "Disabled").length ?? 0;

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
        <div style={{ display: "flex", gap: 10 }}>
          <button className="btn-ghost" type="button" onClick={exportToXlsx} disabled={exportBusy}>
            <Icon name="download" /> {exportBusy ? t("legacySp.exporting") : t("legacySp.export_button")}
          </button>
          <button className="btn-primary" type="button" onClick={() => setWizardOpen(true)}>
            {t("legacySp.install_mod_button")}
          </button>
        </div>
      </div>

      <InstallWizard
        open={wizardOpen}
        onClose={() => setWizardOpen(false)}
        onInstalled={loadMods}
      />

      {error && <p className="error">{t("legacySp.loadError", { error })}</p>}
      {exportError && <p className="error">{t("legacySp.exportError", { error: exportError })}</p>}

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
        {mods && mods.length > 0 && <ModTable mods={mods} onChanged={loadMods} mode="sp" />}
      </div>

      <ModPageTools />
    </section>
  );
}
