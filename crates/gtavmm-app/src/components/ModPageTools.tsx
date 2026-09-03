import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { pickFile } from "../lib/pickers";
import { RecycleBinModal } from "./RecycleBinModal";
import type { ComponentStatus, RecycleBinEntry } from "../types";

/**
 * The three-panel `mini-row` (Component Checker / Full Backup / Recycle Bin) — per the
 * design, these live embedded at the bottom of every SP/LSPDFR mods page, not on a
 * separate "Tools" page. This component is shared across all four (Legacy/Enhanced ×
 * SP/LSPDFR) so the logic isn't duplicated four times.
 */
/**
 * The mini-panel row under a mod list.
 *
 * `framework` swaps the first panel from the script-mod components
 * (ScriptHookV/SHVDN/OpenIV) to the RPH stack, which is what the LSPDFR pages
 * need — an SP user has no reason to be asked about LSPDFR plugins, and vice
 * versa. Everything else in the row is the same on every page.
 */
export function ModPageTools({ variant = "components" }: { variant?: "components" | "framework" }) {
  const { t } = useTranslation();

  const [components, setComponents] = useState<ComponentStatus[] | null>(null);
  const [componentsError, setComponentsError] = useState<string | null>(null);

  const [backups, setBackups] = useState<string[] | null>(null);
  const [backupBusy, setBackupBusy] = useState(false);
  const [backupStatus, setBackupStatus] = useState<string | null>(null);
  const [backupError, setBackupError] = useState<string | null>(null);

  const [recycleEntries, setRecycleEntries] = useState<RecycleBinEntry[] | null>(null);
  const [recycleError, setRecycleError] = useState<string | null>(null);
  const [recycleModalOpen, setRecycleModalOpen] = useState(false);

  const loadComponents = useCallback(() => {
    invoke<ComponentStatus[]>(variant === "framework" ? "check_framework" : "check_components", {
      gamePath: null,
    })
      .then(setComponents)
      .catch((e) => setComponentsError(String(e)));
  }, [variant]);

  const loadBackups = useCallback(() => {
    invoke<string[]>("list_full_backups")
      .then(setBackups)
      .catch(() => {});
  }, []);

  const loadRecycleBin = useCallback(() => {
    invoke<RecycleBinEntry[]>("list_recycle_bin")
      .then(setRecycleEntries)
      .catch((e) => setRecycleError(String(e)));
  }, []);

  useEffect(() => {
    loadComponents();
    loadBackups();
    loadRecycleBin();
    invoke("sweep_expired_recycle_bin")
      .then(() => loadRecycleBin())
      .catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function createBackup() {
    setBackupBusy(true);
    setBackupError(null);
    setBackupStatus(null);
    try {
      const path = await invoke<string>("create_full_backup", { gamePath: null });
      setBackupStatus(t("modPageTools.backup_created", { path }));
      loadBackups();
    } catch (e) {
      setBackupError(String(e));
    } finally {
      setBackupBusy(false);
    }
  }

  async function restoreLatestBackup() {
    if (!backups || backups.length === 0) return;
    setBackupBusy(true);
    setBackupError(null);
    setBackupStatus(null);
    try {
      await invoke("restore_full_backup", { zipPath: backups[0], gamePath: null });
      setBackupStatus(t("modPageTools.backup_restored"));
    } catch (e) {
      setBackupError(String(e));
    } finally {
      setBackupBusy(false);
    }
  }

  async function restoreFromDisk() {
    const picked = await pickFile(["zip"], t("modPageTools.backup_pick_title"));
    if (!picked) return;
    setBackupBusy(true);
    setBackupError(null);
    setBackupStatus(null);
    try {
      await invoke("restore_full_backup", { zipPath: picked, gamePath: null });
      setBackupStatus(t("modPageTools.backup_restored"));
    } catch (e) {
      setBackupError(String(e));
    } finally {
      setBackupBusy(false);
    }
  }

  return (
    <div className="mini-row">
      <div className="mini-panel">
        <h3>
          {t(variant === "framework" ? "modPageTools.framework_title" : "modPageTools.components_title")}
          <button className="link-btn" type="button" onClick={loadComponents}>
            {t("modPageTools.recheck_button")}
          </button>
        </h3>
        {componentsError && <p className="error">{componentsError}</p>}
        {components?.map((c) => (
          <div className={variant === "framework" ? "fw-row" : "comp-row"} key={c.component}>
            <span className="comp-name name">{c.display_name}</span>
            {c.is_installed ? (
              <span className="comp-ok">
                {t(
                  variant === "framework"
                    ? "modPageTools.component_detected"
                    : "modPageTools.component_installed",
                )}
              </span>
            ) : (
              <span className="comp-missing">{t("modPageTools.component_missing")}</span>
            )}
          </div>
        ))}
        {variant === "framework" && (
          <p className="fw-caveat">{t("modPageTools.framework_caveat")}</p>
        )}
        {components?.some((c) => !c.is_installed) && (
          <div className="comp-download">
            {components
              .filter((c) => !c.is_installed)
              .map((c) => (
                <a href={c.official_download_url} target="_blank" rel="noopener noreferrer" key={c.component}>
                  {t("modPageTools.download_link", { name: c.display_name })}
                </a>
              ))}
          </div>
        )}
      </div>

      <div className="mini-panel">
        <h3>{t("modPageTools.backup_title")}</h3>
        <div className="backup-meta">
          {backups && backups.length > 0
            ? t("modPageTools.backup_last", { path: backups[0] })
            : t("modPageTools.backup_none_yet")}
        </div>
        <button className="btn-ghost" type="button" onClick={createBackup} disabled={backupBusy}>
          {backupBusy ? t("modPageTools.backup_working") : t("modPageTools.backup_create_button")}
        </button>
        {backups && backups.length > 0 && (
          <button className="btn-ghost" type="button" onClick={restoreLatestBackup} disabled={backupBusy} style={{ marginLeft: 6 }}>
            {t("modPageTools.backup_restore_latest_button")}
          </button>
        )}
        <button className="link-btn" type="button" onClick={restoreFromDisk} style={{ display: "block", marginTop: 8 }}>
          {t("modPageTools.backup_restore_from_disk_button")}
        </button>
        {backupStatus && <p style={{ marginTop: 8, color: "var(--success)", fontSize: 11.5 }}>{backupStatus}</p>}
        {backupError && <p className="error" style={{ marginTop: 8, fontSize: 11.5 }}>{backupError}</p>}
      </div>

      <div className="mini-panel">
        <h3>
          {t("modPageTools.recycle_title")}
          <button className="link-btn" type="button" onClick={() => setRecycleModalOpen(true)}>
            {t("modPageTools.recycle_view_all_button")}
          </button>
        </h3>
        {recycleError && <p className="error">{recycleError}</p>}
        {recycleEntries && recycleEntries.length === 0 && (
          <span style={{ color: "var(--text-faint)", fontSize: 12 }}>{t("modPageTools.recycle_empty")}</span>
        )}
        {recycleEntries?.slice(0, 2).map((entry) => (
          <div className="recycle-item" key={entry.id}>
            <span className="name">#{entry.id}</span>
            <span className="expiry">{t("modPageTools.recycle_expires", { date: entry.expires_at.slice(0, 10) })}</span>
          </div>
        ))}
      </div>

      <RecycleBinModal
        open={recycleModalOpen}
        onClose={() => setRecycleModalOpen(false)}
        entries={recycleEntries ?? []}
        onRestored={loadRecycleBin}
      />
    </div>
  );
}
