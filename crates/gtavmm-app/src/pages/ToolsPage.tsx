import { useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { pickFile } from "../lib/pickers";
import { highlightLine, languageFor } from "../lib/highlight";
import type { FileHashes } from "../types";

type Tab = "editor" | "hash";

const NL = String.fromCharCode(10);
const SEP = /[\\/]/;

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

  /** One open file. `original` is what was read, so dirty is derived, not tracked. */
  interface OpenFile {
    path: string;
    content: string;
    original: string;
  }
  const [files, setFiles] = useState<OpenFile[]>([]);
  const [activeIndex, setActiveIndex] = useState(0);
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
    // Opening a file that is already open focuses it rather than loading a
    // second copy, which would let two tabs of the same file disagree.
    const existing = files.findIndex((f) => f.path === path);
    if (existing !== -1) {
      setActiveIndex(existing);
      return;
    }
    try {
      const text = await invoke<string>("read_text_file", { path });
      setFiles((prev) => [...prev, { path, content: text, original: text }]);
      setActiveIndex(files.length);
    } catch (e) {
      setOpenError(String(e));
    }
  }

  function closeFile(index: number) {
    setFiles((prev) => prev.filter((_, i) => i !== index));
    setActiveIndex((i) => (index < i || i >= files.length - 1 ? Math.max(0, i - 1) : i));
  }

  async function pickAndOpen() {
    const picked = await pickFile(["ini", "xml", "json"], t("tools.editor_pick_title"));
    if (picked) await openFile(picked);
  }

  async function saveFile() {
    const file = files[activeIndex];
    if (!file) return;
    setSaveError(null);
    setSaveStatus(null);
    try {
      await invoke("write_text_file", { path: file.path, content: file.content });
      // Saved content becomes the new baseline, which is what clears dirty.
      setFiles((prev) =>
        prev.map((f, i) => (i === activeIndex ? { ...f, original: f.content } : f))
      );
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

  const active = files[activeIndex] ?? null;
  const fileType = active?.path.split(".").pop()?.toUpperCase() ?? "";
  const language = active ? languageFor(active.path) : "text";
  const lines = active ? active.content.split(NL) : [];
  const baseName = (p: string) => p.split(SEP).pop() ?? p;

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
          <p className="diagnosis-disclaimer" style={{ margin: "10px 0 14px" }}>
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

          {files.length > 0 && (
            <div className="editor-shell">
              <div className="editor-files">
                <div className="editor-files-head">{t("tools.editor_open_files")}</div>
                {files.map((f, i) => (
                  <button
                    key={f.path}
                    className="editor-file"
                    type="button"
                    data-active={i === activeIndex}
                    onClick={() => setActiveIndex(i)}
                    title={f.path}
                  >
                    <svg className="icon" aria-hidden="true">
                      <use href="#i-file-text" />
                    </svg>
                    <span>{baseName(f.path)}</span>
                    {f.content !== f.original && (
                      <span className="dirty-dot" title={t("tools.editor_unsaved")} />
                    )}
                  </button>
                ))}
                <div className="editor-open-row">
                  <button
                    className="btn-ghost"
                    type="button"
                    onClick={pickAndOpen}
                    style={{ width: "100%" }}
                  >
                    {t("tools.editor_open_button")}
                  </button>
                </div>
              </div>

              <div className="editor-col">
                <div className="editor-tabs" role="tablist">
                  {files.map((f, i) => (
                    <button
                      key={f.path}
                      className="editor-tab"
                      type="button"
                      role="tab"
                      aria-selected={i === activeIndex}
                      data-active={i === activeIndex}
                      onClick={() => setActiveIndex(i)}
                    >
                      {baseName(f.path)}
                      {f.content !== f.original && " •"}
                      <span
                        role="button"
                        tabIndex={0}
                        aria-label={t("tools.editor_close_tab")}
                        style={{ marginLeft: 8, opacity: 0.6 }}
                        onClick={(e) => {
                          e.stopPropagation();
                          closeFile(i);
                        }}
                        onKeyDown={(e) => {
                          if (e.key === "Enter" || e.key === " ") {
                            e.stopPropagation();
                            closeFile(i);
                          }
                        }}
                      >
                        ×
                      </span>
                    </button>
                  ))}
                </div>

                {active && (
                  <>
                    <p className="page-sub mono" style={{ margin: "8px 0" }}>
                      {active.path}
                    </p>
                    {/* The gutter and the textarea are two elements sharing one
                        scroll position: a textarea cannot hold coloured spans,
                        and swapping it for a contenteditable would cost the
                        caret behaviour people expect from a text field. The
                        highlighted copy sits underneath, aligned, and the
                        textarea above it is transparent. */}
                    <div className="editor-area">
                      <div className="editor-gutter mono" aria-hidden="true">
                        {lines.map((_, i) => String(i + 1) + NL).join("")}
                      </div>
                      <div style={{ position: "relative", flex: 1, minWidth: 0 }}>
                        <pre
                          className="editor-code mono"
                          aria-hidden="true"
                          dangerouslySetInnerHTML={{
                            __html: lines
                              .map((l) => highlightLine(l, language) || "&nbsp;")
                              .join(NL),
                          }}
                        />
                        <textarea
                          className="editor-code mono"
                          spellCheck={false}
                          value={active.content}
                          onChange={(e) =>
                            setFiles((prev) =>
                              prev.map((f, i) =>
                                i === activeIndex ? { ...f, content: e.target.value } : f,
                              ),
                            )
                          }
                          onScroll={(e) => {
                            const pre = e.currentTarget
                              .previousElementSibling as HTMLElement | null;
                            const gutter = e.currentTarget.parentElement
                              ?.previousElementSibling as HTMLElement | null;
                            if (pre) pre.scrollTop = e.currentTarget.scrollTop;
                            if (gutter) gutter.scrollTop = e.currentTarget.scrollTop;
                          }}
                          style={{
                            position: "absolute",
                            inset: 0,
                            width: "100%",
                            height: "100%",
                            background: "transparent",
                            color: "transparent",
                            caretColor: "var(--text)",
                            border: "none",
                            outline: "none",
                            resize: "none",
                          }}
                        />
                      </div>
                    </div>

                    <div className="editor-status">
                      <span className="eyebrow">{fileType}</span>
                      <span className="mono">{t("tools.editor_lines", { count: lines.length })}</span>
                      {active.content !== active.original && (
                        <span className="dirty-dot" title={t("tools.editor_unsaved")} />
                      )}
                      <button
                        className="btn-primary"
                        type="button"
                        onClick={saveFile}
                        style={{ marginLeft: "auto" }}
                      >
                        {t("tools.editor_save_button")}
                      </button>
                    </div>
                    {saveStatus && <p style={{ marginTop: 8, color: "var(--success)" }}>{saveStatus}</p>}
                    {saveError && <p className="error" style={{ marginTop: 8 }}>{saveError}</p>}
                  </>
                )}
              </div>
            </div>
          )}
        </>
      )}

      {tab === "hash" && (
        <>
          <p className="diagnosis-disclaimer" style={{ margin: "10px 0 14px" }}>
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
