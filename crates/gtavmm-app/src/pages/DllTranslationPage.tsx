import { useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { pickFile, pickSaveFile } from "../lib/pickers";
import { VirtualList } from "../components/VirtualList";
import type { DllInspection, DllTranslationOutcome, TranslatedDraftEntry } from "../types";

/** `path` with its extension swapped for `Name.<lang>.dll` — the backend's own default
 *  output naming, mirrored here only so the UI can show it before patching happens. */
function defaultOutputPath(path: string, targetLanguage: string): string {
  const lastDot = path.lastIndexOf(".");
  const stem = lastDot === -1 ? path : path.slice(0, lastDot);
  return `${stem}.${targetLanguage}.dll`;
}

const REVIEW_LIST_HEIGHT = 360;
const REVIEW_ROW_HEIGHT = 76;

type Step = "pick" | "inspecting" | "review" | "done" | "error";

/**
 * Wizard UI for gtavmm_core::dll_translation — direct binary patching of user-facing
 * strings embedded in a .NET (IL-only) mod DLL's #US heap. Never overwrites the
 * original file: the backend always writes a new `Name.<lang>.dll` sibling.
 *
 * Every translation field is editable, and starts pre-filled with the original text
 * (not blank) so leaving a row untouched is a safe no-op rather than wiping it. Two
 * ways to fill them in, freely mixable:
 * - "Translate all with AI" calls translate_dll_draft and fills every field — the
 *   user can still edit any of them afterward before patching.
 * - Typing directly into a field needs no AI at all; patch_dll_translations never
 *   calls the AI assistant.
 */
export function DllTranslationPage() {
  const { t } = useTranslation();
  const [step, setStep] = useState<Step>("pick");
  const [path, setPath] = useState<string | null>(null);
  const [inspection, setInspection] = useState<DllInspection | null>(null);
  const [translations, setTranslations] = useState<string[]>([]);
  const [targetLanguage, setTargetLanguage] = useState("zh-TW");
  const [aiBusy, setAiBusy] = useState(false);
  const [patching, setPatching] = useState(false);
  const [outcome, setOutcome] = useState<DllTranslationOutcome | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [customOutputPath, setCustomOutputPath] = useState<string | null>(null);
  const [overwriteOriginal, setOverwriteOriginal] = useState(false);
  const [confirmingOverwrite, setConfirmingOverwrite] = useState(false);

  async function pickAndInspect() {
    try {
      const picked = await pickFile(["dll"], t("dllTranslation.pick_title"));
      if (!picked) return;
      setPath(picked);
      setStep("inspecting");
      setError(null);
      const result = await invoke<DllInspection>("inspect_dll", { dllPath: picked });
      setInspection(result);
      setTranslations(result.translatable.map((s) => s.text));
      setStep("review");
    } catch (e) {
      setError(String(e));
      setStep("error");
    }
  }

  async function aiTranslateAll() {
    if (!path) return;
    setAiBusy(true);
    setError(null);
    try {
      const drafts = await invoke<TranslatedDraftEntry[]>("translate_dll_draft", {
        dllPath: path,
        targetLanguage,
      });
      setTranslations((prev) => {
        const next = [...prev];
        for (const d of drafts) next[d.index] = d.translated;
        return next;
      });
    } catch (e) {
      setError(String(e));
    } finally {
      setAiBusy(false);
    }
  }

  function updateTranslation(index: number, value: string) {
    setTranslations((prev) => {
      const next = [...prev];
      next[index] = value;
      return next;
    });
  }

  async function chooseOutputPath() {
    if (!path) return;
    const picked = await pickSaveFile(defaultOutputPath(path, targetLanguage), ["dll"]);
    if (picked) setCustomOutputPath(picked);
  }

  async function runPatch() {
    if (!path) return;
    setPatching(true);
    setError(null);
    try {
      const result = await invoke<DllTranslationOutcome>("patch_dll_translations", {
        dllPath: path,
        targetLanguage,
        translations,
        outputPath: overwriteOriginal ? path : customOutputPath,
      });
      setOutcome(result);
      setStep("done");
    } catch (e) {
      setError(String(e));
      setStep("error");
    } finally {
      setPatching(false);
      setConfirmingOverwrite(false);
    }
  }

  function patchAll() {
    if (overwriteOriginal) {
      setConfirmingOverwrite(true);
      return;
    }
    void runPatch();
  }

  function reset() {
    setStep("pick");
    setPath(null);
    setInspection(null);
    setTranslations([]);
    setOutcome(null);
    setError(null);
    setCustomOutputPath(null);
    setOverwriteOriginal(false);
    setConfirmingOverwrite(false);
  }

  return (
    <section className="view" data-shown="true">
      <div className="page-head">
        <div>
          <h1 className="page-title">{t("dllTranslation.title")}</h1>
          <p className="page-sub">{t("dllTranslation.subtitle")}</p>
        </div>
      </div>

      <div className="panel" style={{ padding: "18px 20px" }}>
        {step === "pick" && (
          <>
            <p className="page-sub">{t("dllTranslation.intro")}</p>
            <button className="btn-primary" type="button" onClick={pickAndInspect}>
              {t("dllTranslation.pick_button")}
            </button>
          </>
        )}

        {step === "inspecting" && <p>{t("dllTranslation.inspecting")}</p>}

        {step === "review" && inspection && path && (
          <>
            <p className="page-sub">{path}</p>
            <p>
              {t("dllTranslation.review_summary", {
                translatable: inspection.translatable.length,
                total: inspection.total_strings,
                excluded: inspection.excluded_technical,
              })}
            </p>
            <div style={{ margin: "12px 0", display: "flex", gap: 10, alignItems: "center" }}>
              <label>
                {t("dllTranslation.target_language_label")}{" "}
                <input
                  type="text"
                  value={targetLanguage}
                  onChange={(e) => setTargetLanguage(e.target.value)}
                  style={{ width: 100 }}
                />
              </label>
              <button className="btn-ghost" type="button" onClick={aiTranslateAll} disabled={aiBusy}>
                {aiBusy ? t("dllTranslation.ai_translating") : t("dllTranslation.ai_translate_button")}
              </button>
            </div>
            <p className="page-sub">{t("dllTranslation.edit_fields_note")}</p>
            <VirtualList
              className="config-list"
              items={inspection.translatable}
              itemKey={(s) => s.index}
              rowHeight={REVIEW_ROW_HEIGHT}
              height={REVIEW_LIST_HEIGHT}
              renderItem={(s) => (
                <div
                  className="config-row"
                  style={{
                    flexDirection: "column",
                    alignItems: "stretch",
                    gap: 6,
                    height: "calc(100% - 8px)",
                    boxSizing: "border-box",
                  }}
                >
                  <span
                    className="cfg-name"
                    style={{
                      fontSize: "11.5px",
                      color: "var(--text-faint)",
                      overflow: "hidden",
                      textOverflow: "ellipsis",
                      whiteSpace: "nowrap",
                    }}
                    title={s.text}
                  >
                    {s.text}
                  </span>
                  <input
                    type="text"
                    value={translations[s.index] ?? ""}
                    onChange={(e) => updateTranslation(s.index, e.target.value)}
                    style={{ width: "100%" }}
                  />
                </div>
              )}
            />
            <div style={{ marginBottom: 12 }} />
            <p className="page-sub">{t("dllTranslation.original_untouched_note")}</p>

            <div className="override-row" style={{ marginTop: 0, marginBottom: 14 }}>
              <input
                type="checkbox"
                id="dllOverwriteOriginal"
                checked={overwriteOriginal}
                onChange={(e) => setOverwriteOriginal(e.target.checked)}
              />
              <label htmlFor="dllOverwriteOriginal">
                {t("dllTranslation.overwrite_original_label")}{" "}
                <span style={{ color: "var(--danger)", fontWeight: 600 }}>
                  {t("dllTranslation.overwrite_original_warning")}
                </span>
              </label>
            </div>

            <div className="path-picker" style={{ marginBottom: 16 }}>
              <svg className="icon path-icon">
                <use href="#i-folder" />
              </svg>
              <div className="path-text">
                <span className="label">{t("dllTranslation.output_file_label")}</span>
                <span className="value mono">
                  {overwriteOriginal
                    ? t("dllTranslation.output_overwriting", { path })
                    : customOutputPath ?? t("dllTranslation.output_default", { path: defaultOutputPath(path, targetLanguage) })}
                </span>
              </div>
              <button
                className="btn-ghost"
                type="button"
                onClick={chooseOutputPath}
                disabled={overwriteOriginal}
              >
                {t("dllTranslation.choose_output_button")}
              </button>
              {customOutputPath && !overwriteOriginal && (
                <button className="icon-btn" type="button" onClick={() => setCustomOutputPath(null)}>
                  {t("dllTranslation.reset_output_button")}
                </button>
              )}
            </div>

            <button className="btn-primary" type="button" onClick={patchAll} disabled={patching}>
              {patching ? t("dllTranslation.patching") : t("dllTranslation.patch_button")}
            </button>{" "}
            <button className="icon-btn" type="button" onClick={reset}>
              {t("dllTranslation.cancel_button")}
            </button>
            {error && <p className="error">{error}</p>}

            {confirmingOverwrite && (
              <div className="modal-backdrop" data-open="true">
                <div className="modal" style={{ width: 420 }}>
                  <div className="modal-head">
                    <h2>{t("dllTranslation.overwrite_confirm_title")}</h2>
                    <button
                      className="drawer-close"
                      type="button"
                      aria-label="Close"
                      onClick={() => setConfirmingOverwrite(false)}
                    >
                      ×
                    </button>
                  </div>
                  <div className="modal-body">
                    <p>{t("dllTranslation.overwrite_confirm_body", { path })}</p>
                  </div>
                  <div className="modal-foot">
                    <button
                      className="btn-ghost"
                      type="button"
                      onClick={() => setConfirmingOverwrite(false)}
                    >
                      {t("dllTranslation.overwrite_confirm_cancel")}
                    </button>
                    <button
                      className="btn-ghost btn-danger"
                      type="button"
                      onClick={() => void runPatch()}
                      disabled={patching}
                    >
                      {t("dllTranslation.overwrite_confirm_proceed")}
                    </button>
                  </div>
                </div>
              </div>
            )}
          </>
        )}

        {step === "done" && outcome && (
          <>
            <p>
              {t("dllTranslation.done_summary", {
                translated: outcome.strings_translated,
                callSites: outcome.call_sites_patched,
              })}
            </p>
            <p className="page-sub">{outcome.output_path}</p>
            {outcome.skipped.length > 0 && (
              <p className="page-sub">
                {t("dllTranslation.skipped_note", { count: outcome.skipped.length })}
              </p>
            )}
            <button className="btn-primary" type="button" onClick={reset}>
              {t("dllTranslation.translate_another_button")}
            </button>
          </>
        )}

        {step === "error" && (
          <>
            <p className="error">{error}</p>
            <button className="icon-btn" type="button" onClick={reset}>
              {t("dllTranslation.cancel_button")}
            </button>
          </>
        )}
      </div>
    </section>
  );
}
