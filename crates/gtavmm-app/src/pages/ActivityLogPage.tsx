import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import type { EventType, InstallEvent, InstalledMod } from "../types";

type Tab = "activity" | "diagnostic";

const EVENT_TYPES: EventType[] = ["Install", "Uninstall", "Enable", "Disable", "Restore"];

/**
 * Two related but distinct logs, per the user's request (2026-08-30):
 * - "Activity" is a viewer for gtavmm_core::history — the mod install/uninstall/
 *   enable/disable/restore events already recorded in the database, previously only
 *   queryable from the CLI's `history` command.
 * - "Diagnostic Log" is gtavmm_core::app_log — a plain local log file that internal
 *   errors get written to (currently: install_mod, translate_dll_draft,
 *   patch_dll_translations), which is new — nothing in the app wrote to a log file
 *   before this.
 */
export function ActivityLogPage() {
  const { t } = useTranslation();
  const [tab, setTab] = useState<Tab>("activity");

  const [events, setEvents] = useState<InstallEvent[] | null>(null);
  const [mods, setMods] = useState<InstalledMod[] | null>(null);
  const [eventTypeFilter, setEventTypeFilter] = useState<EventType | "all">("all");
  const [error, setError] = useState<string | null>(null);

  const [logLines, setLogLines] = useState<string[] | null>(null);
  const [logPath, setLogPath] = useState<string | null>(null);
  const [logError, setLogError] = useState<string | null>(null);
  const [copyLabel, setCopyLabel] = useState<string | null>(null);

  const loadActivity = useCallback(() => {
    invoke<InstallEvent[]>("list_history", { modId: null })
      .then(setEvents)
      .catch((e) => setError(String(e)));
    invoke<InstalledMod[]>("list_mods")
      .then(setMods)
      .catch(() => {
        // Non-fatal — the activity table just falls back to showing raw mod ids.
      });
  }, []);

  async function exportLog() {
    if (!logLines || logLines.length === 0) return;
    try {
      await navigator.clipboard.writeText(logLines.join("\n"));
      setCopyLabel(t("activityLog.export_copied"));
    } catch (e) {
      setLogError(String(e));
    } finally {
      setTimeout(() => setCopyLabel(null), 1500);
    }
  }

  const loadDiagnosticLog = useCallback(() => {
    invoke<string[]>("read_app_log", { maxLines: 500 })
      .then(setLogLines)
      .catch((e) => setLogError(String(e)));
    invoke<string | null>("app_log_path")
      .then(setLogPath)
      .catch(() => {});
  }, []);

  useEffect(() => {
    loadActivity();
  }, [loadActivity]);

  useEffect(() => {
    if (tab === "diagnostic" && logLines === null) loadDiagnosticLog();
  }, [tab, logLines, loadDiagnosticLog]);

  const modNameById = new Map((mods ?? []).map((m) => [m.id, m.name]));
  const filteredEvents = (events ?? []).filter(
    (e) => eventTypeFilter === "all" || e.event_type === eventTypeFilter
  );

  return (
    <section className="view" data-shown="true">
      <div className="page-head">
        <div>
          <h1 className="page-title">{t("activityLog.title")}</h1>
          <p className="page-sub">{t("activityLog.subtitle")}</p>
        </div>
      </div>

      <div className="page-tabs">
        <button className="page-tab" type="button" data-active={String(tab === "activity")} onClick={() => setTab("activity")}>
          {t("activityLog.tab_activity")}
        </button>
        <button className="page-tab" type="button" data-active={String(tab === "diagnostic")} onClick={() => setTab("diagnostic")}>
          {t("activityLog.tab_diagnostic")}
        </button>
      </div>

      {tab === "activity" && (
        <>
          {error && <p className="error">{error}</p>}
          <div style={{ margin: "0 0 12px" }}>
            <label>
              {t("activityLog.filter_label")}{" "}
              <select
                value={eventTypeFilter}
                onChange={(e) => setEventTypeFilter(e.target.value as EventType | "all")}
              >
                <option value="all">{t("activityLog.filter_all")}</option>
                {EVENT_TYPES.map((et) => (
                  <option key={et} value={et}>
                    {et}
                  </option>
                ))}
              </select>
            </label>
          </div>
          <div className="panel">
            {events === null && !error && <p style={{ padding: "16px 20px" }}>{t("activityLog.loading")}</p>}
            {events && filteredEvents.length === 0 && (
              <p style={{ padding: "16px 20px" }}>{t("activityLog.empty")}</p>
            )}
            {events && filteredEvents.length > 0 && (
              <table>
                <thead>
                  <tr>
                    <th>{t("activityLog.col_event")}</th>
                    <th>{t("activityLog.col_mod")}</th>
                    <th>{t("activityLog.col_when")}</th>
                    <th>{t("activityLog.col_result")}</th>
                  </tr>
                </thead>
                <tbody>
                  {filteredEvents.map((ev) => (
                    <tr key={ev.id}>
                      <td>
                        <span className={`pill ${ev.success ? "active" : "conflict"}`}>{ev.event_type}</span>
                      </td>
                      <td>
                        {ev.installed_mod_id !== null
                          ? modNameById.get(ev.installed_mod_id) ?? `#${ev.installed_mod_id}`
                          : "—"}
                      </td>
                      <td className="mono">{ev.timestamp.slice(0, 19).replace("T", " ")}</td>
                      <td>
                        {ev.success ? (
                          <span className="comp-ok">{t("activityLog.result_success")}</span>
                        ) : (
                          <span className="comp-missing" title={ev.error_message ?? undefined}>
                            {t("activityLog.result_failed")}
                          </span>
                        )}
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            )}
          </div>
        </>
      )}

      {tab === "diagnostic" && (
        <>
          <p className="page-sub" style={{ marginBottom: 10 }}>
            {logPath ? t("activityLog.log_path", { path: logPath }) : ""}
          </p>
          {logError && <p className="error">{logError}</p>}
          <div style={{ marginBottom: 12, display: "flex", gap: 8 }}>
            <button className="btn-ghost" type="button" onClick={loadDiagnosticLog}>
              {t("activityLog.refresh_button")}
            </button>
            <button className="btn-ghost" type="button" onClick={exportLog} disabled={!logLines || logLines.length === 0}>
              {copyLabel ?? t("activityLog.export_button")}
            </button>
          </div>
          <div className="panel" style={{ padding: "14px 16px" }}>
            {logLines === null && !logError && <p>{t("activityLog.loading")}</p>}
            {logLines && logLines.length === 0 && <p>{t("activityLog.log_empty")}</p>}
            {logLines && logLines.length > 0 && (
              <pre
                className="mono"
                style={{
                  margin: 0,
                  whiteSpace: "pre-wrap",
                  wordBreak: "break-all",
                  maxHeight: 420,
                  overflowY: "auto",
                  fontSize: "11.5px",
                  lineHeight: 1.6,
                }}
              >
                {logLines.join("\n")}
              </pre>
            )}
          </div>
        </>
      )}
    </section>
  );
}
