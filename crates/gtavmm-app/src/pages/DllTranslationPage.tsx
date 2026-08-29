import { useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { pickFile } from "../lib/pickers";
import type { DllInspection, DllTranslationOutcome } from "../types";

type Step = "pick" | "inspecting" | "review" | "translating" | "done" | "error";

/**
 * Wizard UI for gtavmm_core::dll_translation — real AI-assisted patching of user-
 * facing strings embedded directly in a .NET (IL-only) mod DLL's #US heap. Requires
 * the AI assistant to already be enabled with a provider (Settings page) — same
 * gating as every other AI-assisted feature. Never overwrites the original file: the
 * backend always writes a new `Name.<lang>.dll` sibling.
 */
export function DllTranslationPage() {
  const { t } = useTranslation();
  const [step, setStep] = useState<Step>("pick");
  const [path, setPath] = useState<string | null>(null);
  const [inspection, setInspection] = useState<DllInspection | null>(null);
  const [targetLanguage, setTargetLanguage] = useState("zh-TW");
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
      setStep("review");
    } catch (e) {
      setError(String(e));
      setStep("error");
    }
  }

  async function runTranslation() {
    if (!path) return;
    setStep("translating");
    setError(null);
    try {
      const result = await invoke<DllTranslationOutcome>("translate_dll", {
        dllPath: path,
        targetLanguage,
      });
      setOutcome(result);
      setStep("done");
    } catch (e) {
      setError(String(e));
      setStep("error");
    }
  }

  function reset() {
    setStep("pick");
    setPath(null);
    setInspection(null);
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
            <div style={{ margin: "12px 0" }}>
              <label>
                {t("dllTranslation.target_language_label")}{" "}
                <input
                  type="text"
                  value={targetLanguage}
                  onChange={(e) => setTargetLanguage(e.target.value)}
                  style={{ width: 100 }}
                />
              </label>
            </div>
            <div
              className="config-list"
              style={{ maxHeight: 240, overflowY: "auto", marginBottom: 12 }}
            >
              {inspection.translatable.slice(0, 20).map((s) => (
                <div className="config-row" key={s.index}>
                  <span className="cfg-name">{s.text}</span>
                </div>
              ))}
              {inspection.translatable.length > 20 && (
                <p className="page-sub">
                  {t("dllTranslation.more_strings", {
                    count: inspection.translatable.length - 20,
                  })}
                </p>
              )}
            </div>
            <p className="page-sub">{t("dllTranslation.original_untouched_note")}</p>
            <button className="btn-primary" type="button" onClick={runTranslation}>
              {t("dllTranslation.translate_button")}
            </button>{" "}
            <button className="icon-btn" type="button" onClick={reset}>
              {t("dllTranslation.cancel_button")}
            </button>
          </>
        )}

        {step === "translating" && <p>{t("dllTranslation.translating")}</p>}

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
