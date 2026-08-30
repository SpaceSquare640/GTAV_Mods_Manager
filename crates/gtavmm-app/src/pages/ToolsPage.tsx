import { useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { pickFile } from "../lib/pickers";
import type { FileHashes } from "../types";

type Tab = "editor" | "hash";

/**
 * The real Tools page, matching the design's actual content — general-purpose
 * utilities not tied to a specific mode: a plain-text file editor (.ini/.xml/.json,
 * no built-in backup — back up first if unsure) and a hash calculator. Component
 * Checker/Backup/Recycle Bin live embedded in each mods page instead (see
 * ModPageTools) — an earlier version of this page wrongly put them here, which
 * didn't match the design at all.
 */
export function ToolsPage() {
  const { t } = useTranslation();
  const [tab, setTab] = useState<Tab>("editor");

  const [filePath, setFilePath] = useState<string | null>(null);
  const [content, setContent] = useState("");
  const [dirty, setDirty] = useState(false);
  const [openError, setOpenError] = useState<string | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [saveStatus, setSaveStatus] = useState<string | null>(null);
  const [manualPath, setManualPath] = useState("");

  const [hashPath, setHashPath] = useState("");
  const [hashes, setHashes] = useState<FileHashes | null>(null);
  const [hashBusy, setHashBusy] = useState(false);
  const [hashError, setHashError] = useState<string | null>(null);

  async function openFile(path: string) {
    setOpenError(null);
    try {
      const text = await invoke<string>("read_text_file", { path });
      setFilePath(path);
      setContent(text);
      setDirty(false);
    } catch (e) {
      setOpenError(String(e));
    }
  }

  async function pickAndOpen() {
    const picked = await pickFile(["ini", "xml", "json"], t("tools.editor_pick_title"));
    if (picked) await openFile(picked);
  }

  async function saveFile() {
    if (!filePath) return;
    setSaveError(null);
    setSaveStatus(null);
    try {
      await invoke("write_text_file", { path: filePath, content });
      setDirty(false);
      setSaveStatus(t("tools.editor_saved"));
    } catch (e) {
      setSaveError(String(e));
    }
  }

  async function computeHashes() {
    if (!hashPath.trim()) return;
    setHashBusy(true);
    setHashError(null);
    setHashes(null);
    try {
      const result = await invoke<FileHashes>("compute_file_hashes", { path: hashPath.trim() });
      setHashes(result);
    } catch (e) {
      setHashError(String(e));
    } finally {
      setHashBusy(false);
    }
  }

  async function pickHashFile() {
    const picked = await pickFile([], t("tools.hash_pick_title"));
    if (picked) setHashPath(picked);
  }

  const fileType = filePath?.split(".").pop()?.toUpperCase() ?? "";

  return (
    <section className="view" data-shown="true">
      <div className="page-head">
        <div>
          <h1 className="page-title">{t("tools.title")}</h1>
          <p className="page-sub">{t("tools.subtitle")}</p>
        </div>
      </div>

      <div className="page-tabs">
        <button className="page-tab" type="button" data-active={String(tab === "editor")} onClick={() => setTab("editor")}>
          {t("tools.tab_editor")}
        </button>
        <button className="page-tab" type="button" data-active={String(tab === "hash")} onClick={() => setTab("hash")}>
          {t("tools.tab_hash")}
        </button>
      </div>

      {tab === "editor" && (
        <>
          <p className="diagnosis-disclaimer" style={{ margin: "-8px 0 14px" }}>
            {t("tools.editor_disclaimer")}
          </p>
          <div style={{ display: "flex", gap: 10, marginBottom: 12, alignItems: "center" }}>
            <button className="btn-primary" type="button" onClick={pickAndOpen}>
              {t("tools.editor_open_button")}
            </button>
            <input
              type="text"
              value={manualPath}
              onChange={(e) => setManualPath(e.target.value)}
              placeholder={t("tools.editor_path_placeholder")}
              style={{ flex: 1 }}
            />
            <button className="btn-ghost" type="button" onClick={() => manualPath.trim() && openFile(manualPath.trim())}>
              {t("tools.editor_open_path_button")}
            </button>
          </div>
          {openError && <p className="error">{openError}</p>}

          {filePath && (
            <>
              <p className="page-sub mono" style={{ marginBottom: 8 }}>{filePath}</p>
              <textarea
                className="mono"
                value={content}
                onChange={(e) => {
                  setContent(e.target.value);
                  setDirty(true);
                }}
                rows={18}
                style={{
                  width: "100%",
                  background: "var(--surface)",
                  border: "1px solid var(--border)",
                  borderRadius: "var(--radius)",
                  color: "var(--text)",
                  fontSize: 12.5,
                  padding: "12px 14px",
                  resize: "vertical",
                }}
              />
              <div style={{ display: "flex", alignItems: "center", gap: 10, marginTop: 10 }}>
                <span className="eyebrow">{fileType}</span>
                {dirty && <span className="dirty-dot" title={t("tools.editor_unsaved")} />}
                <button className="btn-primary" type="button" onClick={saveFile} style={{ marginLeft: "auto" }}>
                  {t("tools.editor_save_button")}
                </button>
              </div>
              {saveStatus && <p style={{ marginTop: 8, color: "var(--success)" }}>{saveStatus}</p>}
              {saveError && <p className="error" style={{ marginTop: 8 }}>{saveError}</p>}
            </>
          )}
        </>
      )}

      {tab === "hash" && (
        <>
          <p className="diagnosis-disclaimer" style={{ margin: "-8px 0 14px" }}>
            {t("tools.hash_disclaimer")}
          </p>
          <div className="panel" style={{ padding: "20px 22px", maxWidth: 640 }}>
            <div className="field-group" style={{ marginBottom: 6 }}>
              <label>{t("tools.hash_path_label")}</label>
              <div style={{ display: "flex", gap: 8 }}>
                <input
                  type="text"
                  value={hashPath}
                  onChange={(e) => setHashPath(e.target.value)}
                  placeholder={t("tools.hash_path_placeholder")}
                  style={{ flex: 1 }}
                />
                <button className="btn-ghost" type="button" onClick={pickHashFile}>
                  {t("tools.hash_browse_button")}
                </button>
                <button className="btn-primary" type="button" onClick={computeHashes} disabled={hashBusy}>
                  {hashBusy ? t("tools.hash_computing") : t("tools.hash_compute_button")}
                </button>
              </div>
            </div>
            {hashError && <p className="error" style={{ marginTop: 10 }}>{hashError}</p>}
            {hashes && (
              <div style={{ marginTop: 14 }}>
                <div className="hash-row">
                  <span className="eyebrow">MD5</span>
                  <span className="mono">{hashes.md5}</span>
                </div>
                <div className="hash-row">
                  <span className="eyebrow">SHA-1</span>
                  <span className="mono">{hashes.sha1}</span>
                </div>
                <div className="hash-row">
                  <span className="eyebrow">SHA-256</span>
                  <span className="mono">{hashes.sha256}</span>
                </div>
              </div>
            )}
          </div>
        </>
      )}
    </section>
  );
}
