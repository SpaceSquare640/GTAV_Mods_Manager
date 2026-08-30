import { useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { pickFile } from "../lib/pickers";
import { VirtualList } from "../components/VirtualList";
import type { DllInspection, DllTranslationOutcome, TranslatedDraftEntry } from "../types";

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

  async function patchAll() {
    if (!path) return;
    setPatching(true);
    setError(null);
    try {
      const result = await invoke<DllTranslationOutcome>("patch_dll_translations", {
        dllPath: path,
        targetLanguage,
        translations,
      });
      setOutcome(result);
      setStep("done");
    } catch (e) {
      setError(String(e));
      setStep("error");
    } finally {
      setPatching(false);
    }
  }

  function reset() {
    setStep("pick");
    setPath(null);
    setInspection(null);
    setTranslations([]);
    setOutcome(null);
    setError(null);
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
            <button className="btn-primary" type="button" onClick={patchAll} disabled={patching}>
              {patching ? t("dllTranslation.patching") : t("dllTranslation.patch_button")}
            </button>{" "}
            <button className="icon-btn" type="button" onClick={reset}>
              {t("dllTranslation.cancel_button")}
            </button>
            {error && <p className="error">{error}</p>}
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
