import { useCallback, useEffect, useState, type ReactNode } from "react";
import { Trans, useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "./IconSprite";
import { InstallWizard } from "./InstallWizard";
import { ModTable } from "./ModTable";
import { ModPageTools } from "./ModPageTools";
import { pickSaveFile } from "../lib/pickers";
import type { InstallEvent, InstalledMod, PageMode } from "../types";

/** What a stat card can count. */
export interface StatContext {
  mods: InstalledMod[];
  /** Mods with at least one failed install event on this page. */
  failedModIds: Set<number>;
}

/** A stat card. `value` runs once the page's data has loaded. */
export interface StatSpec {
  labelKey: string;
  tone?: "accent" | "warn";
  value: (ctx: StatContext) => number | string;
}

export interface ModWorkspaceProps {
  /** Which page this is. Everything shown is filtered to it. */
  pageMode: PageMode;
  titleKey: string;
  subtitleKey: string;
  /** LSPDFR shield and/or public-preview chip beside the title. */
  badges?: ("lspdfr" | "beta")[];
  banner?: { tone: "info" | "warn"; icon: string; key: string };
  /** Legacy SP alone offers the spreadsheet export. */
  showExcelExport?: boolean;
  stats: StatSpec[];
  /** LSPDFR category chips, in chip order. Omit on the SP pages. */
  categories?: string[];
  /** LSPDFR pages show the RPH framework panel in place of Components. */
  toolsVariant?: "components" | "framework";
  /** Rendered under the mods panel — extra content above the mini-panel row. */
  tools?: ReactNode;
}

/**
 * The workspace shared by the four mod pages.
 *
 * These four were near-identical copies of one another: same loader, same stat
 * markup byte for byte, same panel, same empty state. Worse, they all called
 * `list_mods` with no argument and so all showed the same mods — the page you
 * were on made no difference to what you saw.
 *
 * Both problems have the same root, so they are fixed together: the page is now
 * a parameter, passed to the query, and the parts that genuinely differ between
 * pages (title, banner, badges, which stat cards) are props.
 */
export function ModWorkspace({
  pageMode,
  titleKey,
  subtitleKey,
  badges = [],
  banner,
  showExcelExport = false,
  stats,
  categories,
  toolsVariant = "components",
  tools,
}: ModWorkspaceProps) {
  const { t } = useTranslation();
  const [mods, setMods] = useState<InstalledMod[] | null>(null);
  const [events, setEvents] = useState<InstallEvent[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [wizardOpen, setWizardOpen] = useState(false);
  const [tab, setTab] = useState<"mods" | "history">("mods");
  const [category, setCategory] = useState<string>("all");
  const [exportBusy, setExportBusy] = useState(false);
  const [exportError, setExportError] = useState<string | null>(null);

  const loadMods = useCallback(() => {
    setError(null);
    invoke<InstalledMod[]>("list_mods", { mode: pageMode })
      .then(setMods)
      .catch((e) => setError(String(e)));
    // Loaded here rather than inside the History tab because a stat card needs
    // it too, and fetching the same rows twice would let the two disagree.
    invoke<InstallEvent[]>("list_history", { modId: null, mode: pageMode })
      .then(setEvents)
      .catch(() => setEvents([]));
  }, [pageMode]);

  useEffect(() => {
    loadMods();
  }, [loadMods]);

  // Mods whose install left a failed event behind. This is what the design's
  // "Needs review" card counts — an approximation, not a core concept: the
  // engine has no notion of a mod needing attention, and a failed event is the
  // closest thing it actually records.
  const failedModIds = new Set(
    (events ?? [])
      .filter((e) => !e.success && e.installed_mod_id !== null)
      .map((e) => e.installed_mod_id as number)
  );

  // Chips filter what the table shows but not what the stat cards count: the
  // counts describe the page, and a chip is a view of it, not a change to it.
  const visible = (mods ?? []).filter((m) => category === "all" || m.category === category);

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

  return (
    <section className="view" data-shown="true">
      <div className="page-head">
        <div>
          <h1 className="page-title">
            {t(titleKey)}
            {badges.includes("lspdfr") && (
              <>
                {" "}
                <span className="lspdfr-badge">
                  <Icon name="shield" /> {t("legacyLspdfr.badge")}
                </span>
              </>
            )}
            {badges.includes("beta") && (
              <>
                {" "}
                <span className="beta-chip">
                  <Icon name="alert-triangle" /> {t("enhancedLspdfr.preview_badge")}
                </span>
              </>
            )}
          </h1>
          <p className="page-sub">
            <Trans
              i18nKey={subtitleKey}
              components={{ mono: <span className="mono" />, strong: <strong /> }}
            />
          </p>
        </div>
        <div className="head-actions">
          {showExcelExport && (
            <button className="btn-ghost" type="button" onClick={exportToXlsx} disabled={exportBusy}>
              <Icon name="download" />{" "}
              {exportBusy ? t("legacySp.exporting") : t("legacySp.export_button")}
            </button>
          )}
          <button className="btn-primary" type="button" onClick={() => setWizardOpen(true)}>
            {t("legacySp.install_mod_button")}
          </button>
        </div>
      </div>

      <InstallWizard
        open={wizardOpen}
        onClose={() => setWizardOpen(false)}
        onInstalled={loadMods}
        mode={pageMode}
      />

      {error && <p className="error">{t("legacySp.loadError", { error })}</p>}
      {exportError && <p className="error">{t("legacySp.exportError", { error: exportError })}</p>}

      <div className="page-tabs" role="tablist">
        <button
          className="page-tab"
          type="button"
          role="tab"
          aria-selected={tab === "mods"}
          data-active={tab === "mods"}
          onClick={() => setTab("mods")}
        >
          {t("modWorkspace.tab_mods")}
        </button>
        <button
          className="page-tab"
          type="button"
          role="tab"
          aria-selected={tab === "history"}
          data-active={tab === "history"}
          onClick={() => setTab("history")}
        >
          {t("modWorkspace.tab_history")}
        </button>
      </div>

      {tab === "mods" ? (
        <>
          {banner && (
            <div className={banner.tone === "warn" ? "info-banner warn" : "info-banner"}>
              <svg className="icon glyph" aria-hidden="true">
                <use href={`#i-${banner.icon}`} />
              </svg>
              <span>
                <Trans
                  i18nKey={banner.key}
                  components={{ mono: <span className="mono" />, strong: <strong /> }}
                />
              </span>
            </div>
          )}

          {categories && mods && (
            <div className="cat-tabs" role="group" aria-label={t("modWorkspace.filter_by_category")}>
              {["all", ...categories].map((c) => (
                <button
                  key={c}
                  className="cat-tab"
                  type="button"
                  data-active={category === c}
                  onClick={() => setCategory(c)}
                >
                  {t(`modWorkspace.cat_${c}`)}
                  <span className="cat-chip">
                    {c === "all" ? mods.length : mods.filter((m) => m.category === c).length}
                  </span>
                </button>
              ))}
            </div>
          )}

          {stats.length > 0 && (
            <div className="stat-row">
              {stats.map((s) => (
                <div className="stat-card" key={s.labelKey} data-tone={s.tone}>
                  <div className="eyebrow">{t(s.labelKey)}</div>
                  {/* An em dash until the data is in: a 0 nobody measured reads
                      as a measurement. */}
                  <div className="value">
                    {mods && events ? s.value({ mods, failedModIds }) : "—"}
                  </div>
                </div>
              ))}
            </div>
          )}

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
                <p>{t("modWorkspace.empty_body")}</p>
              </div>
            )}
            {mods && mods.length > 0 && (
              <ModTable
                mods={visible}
                onChanged={loadMods}
                mode={pageMode}
                showCategory={Boolean(categories)}
              />
            )}
          </div>

          {tools}
          <ModPageTools variant={toolsVariant} />
        </>
      ) : (
        <HistoryPanel events={events} mods={mods} titleKey={titleKey} />
      )}
    </section>
  );
}

/** This page's install events, newest first. */
function HistoryPanel({
  events,
  mods,
  titleKey,
}: {
  events: InstallEvent[] | null;
  mods: InstalledMod[] | null;
  titleKey: string;
}) {
  const { t } = useTranslation();
  // Events store a mod id, not a name. Names come from the mod list so a
  // renamed mod reads correctly in its own history rather than under whatever
  // it was called at the time.
  const names: Record<number, string> = Object.fromEntries(
    (mods ?? []).map((m) => [m.id, m.name])
  );
  const error = null;

  return (
    <div className="panel">
      <div className="panel-head">
        <h2>
          {t("modWorkspace.history_title")} · {t(titleKey)}
        </h2>
      </div>
      {error && <p className="error">{error}</p>}
      {events === null && !error && <p style={{ padding: "16px 20px" }}>{t("legacySp.loading")}</p>}
      {events && events.length === 0 && (
        <div className="empty-state">
          <span className="glyph">
            <Icon name="bar-chart" />
          </span>
          <h3>{t("modWorkspace.history_empty")}</h3>
        </div>
      )}
      {events && events.length > 0 && (
        <table className="data">
          <thead>
            <tr>
              <th>{t("modWorkspace.col_event")}</th>
              <th>{t("modWorkspace.col_mod")}</th>
              <th>{t("modWorkspace.col_when")}</th>
              <th>{t("modWorkspace.col_result")}</th>
            </tr>
          </thead>
          <tbody>
            {events.map((e) => (
              <tr key={e.id}>
                <td>{t(`activityLog.event_${e.event_type.toLowerCase()}`, e.event_type)}</td>
                <td className="mod-name">
                  {e.installed_mod_id === null
                    ? t("modWorkspace.mod_removed")
                    : (names[e.installed_mod_id] ?? `#${e.installed_mod_id}`)}
                </td>
                <td className="mono">{e.timestamp}</td>
                <td>
                  {e.success ? (
                    <span className="comp-ok">
                      <Icon name="check" /> {t("modWorkspace.result_success")}
                    </span>
                  ) : (
                    <span className="comp-missing">
                      <Icon name="x" /> {e.error_message ?? t("modWorkspace.result_failed")}
                    </span>
                  )}
                </td>
              </tr>
            ))}
          </tbody>
        </table>
      )}
    </div>
  );
}
