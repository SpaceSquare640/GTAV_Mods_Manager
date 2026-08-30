import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { pickFile } from "../lib/pickers";
import type { ComponentStatus, PromptTemplate, RecycleBinEntry } from "../types";

/**
 * Four previously CLI-only tools, wired into the GUI for the first time:
 * - Component Checker (gtavmm_core::components) — presence-only detection of
 *   ScriptHookV/SHVDN/OpenIV against the auto-detected (or overridden) game root.
 * - Full Backup (gtavmm_core::full_backup) — whole-`mods\` zip snapshot, a coarser
 *   manual safety net distinct from install's per-mod backups.
 * - Recycle Bin (gtavmm_core::recycle_bin) — 15-day-retention restore point for
 *   uninstalled mods, written automatically by `uninstall`.
 * - Prompt Library (gtavmm_core::prompt_template) — the user's own reusable prompt
 *   text for pasting into an AI provider by hand; not part of the AI Assistant's
 *   Action Schema, never applied/executed automatically.
 * The first three default to auto-detecting the game install (gamePath = null, same
 * convention InstallWizard uses) rather than requiring the user to pick a folder
 * up front.
 */
export function ToolsPage() {
  const { t } = useTranslation();

  const [components, setComponents] = useState<ComponentStatus[] | null>(null);
  const [componentsError, setComponentsError] = useState<string | null>(null);

  const [backups, setBackups] = useState<string[] | null>(null);
  const [backupError, setBackupError] = useState<string | null>(null);
  const [backupBusy, setBackupBusy] = useState(false);
  const [backupStatus, setBackupStatus] = useState<string | null>(null);

  const [recycleEntries, setRecycleEntries] = useState<RecycleBinEntry[] | null>(null);
  const [recycleError, setRecycleError] = useState<string | null>(null);
  const [restoringId, setRestoringId] = useState<number | null>(null);

  const [prompts, setPrompts] = useState<PromptTemplate[] | null>(null);
  const [promptError, setPromptError] = useState<string | null>(null);
  const [newPromptName, setNewPromptName] = useState("");
  const [newPromptContent, setNewPromptContent] = useState("");
  const [editingPromptId, setEditingPromptId] = useState<number | null>(null);
  const [editPromptName, setEditPromptName] = useState("");
  const [editPromptContent, setEditPromptContent] = useState("");

  const loadComponents = useCallback(() => {
    invoke<ComponentStatus[]>("check_components", { gamePath: null })
      .then(setComponents)
      .catch((e) => setComponentsError(String(e)));
  }, []);

  const loadBackups = useCallback(() => {
    invoke<string[]>("list_full_backups")
      .then(setBackups)
      .catch((e) => setBackupError(String(e)));
  }, []);

  const loadRecycleBin = useCallback(() => {
    invoke<RecycleBinEntry[]>("list_recycle_bin")
      .then(setRecycleEntries)
      .catch((e) => setRecycleError(String(e)));
  }, []);

  const loadPrompts = useCallback(() => {
    invoke<PromptTemplate[]>("list_prompt_templates")
      .then(setPrompts)
      .catch((e) => setPromptError(String(e)));
  }, []);

  useEffect(() => {
    loadComponents();
    loadBackups();
    loadRecycleBin();
    loadPrompts();
    // Sweeping is opt-in-by-call, not a background daemon — this page is a
    // reasonable "app touched something recycle-bin-related" moment to do it.
    invoke("sweep_expired_recycle_bin")
      .then(() => loadRecycleBin())
      .catch(() => {});
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, []);

  async function addPrompt() {
    if (!newPromptName.trim() || !newPromptContent.trim()) return;
    try {
      await invoke("add_prompt_template", {
        name: newPromptName.trim(),
        content: newPromptContent.trim(),
      });
      setNewPromptName("");
      setNewPromptContent("");
      loadPrompts();
    } catch (e) {
      setPromptError(String(e));
    }
  }

  function startEditPrompt(p: PromptTemplate) {
    setEditingPromptId(p.id);
    setEditPromptName(p.name);
    setEditPromptContent(p.content);
  }

  async function saveEditPrompt() {
    if (editingPromptId === null || !editPromptName.trim() || !editPromptContent.trim()) return;
    try {
      await invoke("update_prompt_template", {
        id: editingPromptId,
        name: editPromptName.trim(),
        content: editPromptContent.trim(),
      });
      setEditingPromptId(null);
      loadPrompts();
    } catch (e) {
      setPromptError(String(e));
    }
  }

  async function deletePrompt(id: number) {
    try {
      await invoke("delete_prompt_template", { id });
      loadPrompts();
    } catch (e) {
      setPromptError(String(e));
    }
  }

  async function createBackup() {
    setBackupBusy(true);
    setBackupError(null);
    setBackupStatus(null);
    try {
      const path = await invoke<string>("create_full_backup", { gamePath: null });
      setBackupStatus(t("tools.backup_created", { path }));
      loadBackups();
    } catch (e) {
      setBackupError(String(e));
    } finally {
      setBackupBusy(false);
    }
  }

  async function restoreBackup(zipPath: string) {
    setBackupBusy(true);
    setBackupError(null);
    setBackupStatus(null);
    try {
      await invoke("restore_full_backup", { zipPath, gamePath: null });
      setBackupStatus(t("tools.backup_restored"));
    } catch (e) {
      setBackupError(String(e));
    } finally {
      setBackupBusy(false);
    }
  }

  async function restoreFromDisk() {
    const picked = await pickFile(["zip"], t("tools.backup_pick_title"));
    if (picked) await restoreBackup(picked);
  }

  async function restoreRecycleEntry(id: number) {
    setRestoringId(id);
    setRecycleError(null);
    try {
      await invoke("restore_recycle_bin_entry", { entryId: id, gamePath: null });
      loadRecycleBin();
    } catch (e) {
      setRecycleError(String(e));
    } finally {
      setRestoringId(null);
    }
  }

  return (
    <section className="view" data-shown="true">
      <div className="page-head">
        <div>
          <h1 className="page-title">{t("tools.title")}</h1>
          <p className="page-sub">{t("tools.subtitle")}</p>
        </div>
      </div>

      <div className="mini-row">
        <div className="mini-panel">
          <h3>
            {t("tools.components_title")}
            <button className="link-btn" type="button" onClick={loadComponents}>
              {t("tools.recheck_button")}
            </button>
          </h3>
          {componentsError && <p className="error">{componentsError}</p>}
          {components === null && !componentsError && <p>{t("tools.loading")}</p>}
          {components?.map((c) => (
            <div className="comp-row" key={c.component}>
              <span className="name">{c.display_name}</span>
              {c.is_installed ? (
                <span className="comp-ok">{t("tools.component_installed")}</span>
              ) : (
                <span className="comp-missing">{t("tools.component_missing")}</span>
              )}
            </div>
          ))}
          {components?.some((c) => !c.is_installed) && (
            <div className="comp-download">
              {components
                .filter((c) => !c.is_installed)
                .map((c) => (
                  <a href={c.official_download_url} target="_blank" rel="noopener noreferrer" key={c.component}>
                    {t("tools.download_link", { name: c.display_name })}
                  </a>
                ))}
              <span className="comp-caveat">{t("tools.component_caveat")}</span>
            </div>
          )}
        </div>

        <div className="mini-panel">
          <h3>{t("tools.backup_title")}</h3>
          <p className="backup-meta">{t("tools.backup_intro")}</p>
          <button className="btn-ghost" type="button" onClick={createBackup} disabled={backupBusy}>
            {backupBusy ? t("tools.backup_working") : t("tools.backup_create_button")}
          </button>{" "}
          <button className="btn-ghost" type="button" onClick={restoreFromDisk} disabled={backupBusy}>
            {t("tools.backup_restore_from_disk_button")}
          </button>
          {backupStatus && <p style={{ marginTop: 10, color: "var(--success)" }}>{backupStatus}</p>}
          {backupError && <p className="error" style={{ marginTop: 10 }}>{backupError}</p>}
          {backups && backups.length > 0 && (
            <div style={{ marginTop: 10 }}>
              {backups.map((path) => (
                <div className="recycle-item" key={path}>
                  <span className="name mono" style={{ fontSize: 11 }}>{path}</span>
                  <button className="icon-btn" type="button" onClick={() => restoreBackup(path)} disabled={backupBusy}>
                    {t("tools.backup_restore_button")}
                  </button>
                </div>
              ))}
            </div>
          )}
          {backups && backups.length === 0 && (
            <p className="backup-meta" style={{ marginTop: 10 }}>{t("tools.backup_empty")}</p>
          )}
        </div>

        <div className="mini-panel">
          <h3>{t("tools.recycle_title")}</h3>
          {recycleError && <p className="error">{recycleError}</p>}
          {recycleEntries === null && !recycleError && <p>{t("tools.loading")}</p>}
          {recycleEntries && recycleEntries.length === 0 && (
            <div className="rb-empty">
              <span className="glyph">🗑</span>
              {t("tools.recycle_empty")}
            </div>
          )}
          {recycleEntries?.map((entry) => (
            <div className="rb-row" key={entry.id}>
              <span className="name">#{entry.id}</span>
              <div className="meta">
                <span>{entry.deleted_at.slice(0, 10)}</span>
                <span className="expiry">{t("tools.recycle_expires", { date: entry.expires_at.slice(0, 10) })}</span>
              </div>
              <button
                className="btn-ghost"
                type="button"
                onClick={() => restoreRecycleEntry(entry.id)}
                disabled={restoringId === entry.id}
              >
                {restoringId === entry.id ? t("tools.recycle_restoring") : t("tools.recycle_restore_button")}
              </button>
            </div>
          ))}
        </div>
      </div>

      <div className="panel" style={{ padding: "18px 20px", marginTop: 16 }}>
        <div className="eyebrow" style={{ marginBottom: 10 }}>
          {t("tools.prompts_title")}
        </div>
        <p className="page-sub" style={{ marginBottom: 14 }}>{t("tools.prompts_intro")}</p>

        <div className="field-group" style={{ marginBottom: 8 }}>
          <label>{t("tools.prompts_name_label")}</label>
          <input type="text" value={newPromptName} onChange={(e) => setNewPromptName(e.target.value)} />
        </div>
        <div className="field-group" style={{ marginBottom: 8 }}>
          <label>{t("tools.prompts_content_label")}</label>
          <textarea rows={4} value={newPromptContent} onChange={(e) => setNewPromptContent(e.target.value)} />
        </div>
        <button className="btn-primary" type="button" onClick={addPrompt}>
          {t("tools.prompts_add_button")}
        </button>

        {promptError && <p className="error" style={{ marginTop: 10 }}>{promptError}</p>}

        {prompts && prompts.length === 0 && (
          <p className="page-sub" style={{ marginTop: 14 }}>{t("tools.prompts_empty")}</p>
        )}

        {prompts?.map((p) => (
          <div className="prompt-row" key={p.id}>
            {editingPromptId === p.id ? (
              <>
                <input
                  type="text"
                  value={editPromptName}
                  onChange={(e) => setEditPromptName(e.target.value)}
                  style={{ marginBottom: 6, width: "100%" }}
                />
                <textarea
                  rows={4}
                  value={editPromptContent}
                  onChange={(e) => setEditPromptContent(e.target.value)}
                  style={{ width: "100%", marginBottom: 8 }}
                />
                <div style={{ display: "flex", gap: 8 }}>
                  <button className="btn-primary" type="button" onClick={saveEditPrompt}>
                    {t("tools.prompts_save_button")}
                  </button>
                  <button className="icon-btn" type="button" onClick={() => setEditingPromptId(null)}>
                    {t("tools.prompts_cancel_button")}
                  </button>
                </div>
              </>
            ) : (
              <>
                <div className="top">
                  <span className="name">{p.name}</span>
                  <div className="cfg-actions">
                    <button className="icon-btn" type="button" onClick={() => startEditPrompt(p)}>
                      {t("tools.prompts_edit_button")}
                    </button>
                    <button className="icon-btn" type="button" onClick={() => deletePrompt(p.id)}>
                      {t("tools.prompts_delete_button")}
                    </button>
                  </div>
                </div>
                <div className="body-text">{p.content}</div>
              </>
            )}
          </div>
        ))}
      </div>
    </section>
  );
}
