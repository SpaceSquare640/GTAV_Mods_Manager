import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { SUPPORTED_LANGUAGES } from "../i18n";
import { PromptLibraryModal } from "../components/PromptLibraryModal";
import type { AiProviderKind, AiSettings, UpdateCheckResult } from "../types";

const LANGUAGE_LABELS: Record<string, string> = {
  en: "English",
  "zh-TW": "繁體中文",
};

export function SettingsPage() {
  const { t, i18n } = useTranslation();
  const [saveState, setSaveState] = useState<"idle" | "saving" | "saved" | "error">(
    "idle"
  );

  const [promptLibraryOpen, setPromptLibraryOpen] = useState(false);
  const [aiSettings, setAiSettings] = useState<AiSettings | null>(null);
  const [aiHasKey, setAiHasKey] = useState(false);
  const [aiProviderChoice, setAiProviderChoice] = useState<AiProviderKind>("cloud");
  const [aiModel, setAiModel] = useState("");
  const [aiEndpoint, setAiEndpoint] = useState("");
  const [aiApiKey, setAiApiKey] = useState("");
  const [aiStatus, setAiStatus] = useState<string | null>(null);
  const [aiError, setAiError] = useState<string | null>(null);
  const [aiBusy, setAiBusy] = useState(false);

  const [diagnoseContext, setDiagnoseContext] = useState("");
  const [diagnoseResult, setDiagnoseResult] = useState<string | null>(null);
  const [diagnosing, setDiagnosing] = useState(false);
  const [diagnoseError, setDiagnoseError] = useState<string | null>(null);

  const [updateResult, setUpdateResult] = useState<UpdateCheckResult | null>(null);
  const [updateChecking, setUpdateChecking] = useState(false);
  const [updateError, setUpdateError] = useState<string | null>(null);

  async function checkForUpdate() {
    setUpdateChecking(true);
    setUpdateError(null);
    setUpdateResult(null);
    try {
      const result = await invoke<UpdateCheckResult>("check_for_update");
      setUpdateResult(result);
    } catch (e) {
      setUpdateError(String(e));
    } finally {
      setUpdateChecking(false);
    }
  }

  function loadAiSettings() {
    invoke<AiSettings>("ai_load_settings")
      .then((s) => {
        setAiSettings(s);
        if (s.provider) setAiProviderChoice(s.provider);
        setAiModel((s.provider === "ollama" ? s.ollama_model : s.cloud_model) ?? "");
        setAiEndpoint(s.cloud_endpoint ?? "");
      })
      .catch(() => {
        // No Tauri runtime (plain browser preview) — leave the AI panel in its
        // not-yet-loaded state rather than showing a scary error for something
        // that's expected outside the real app.
      });
    invoke<boolean>("ai_has_cloud_api_key")
      .then(setAiHasKey)
      .catch(() => {});
  }

  useEffect(() => {
    loadAiSettings();
  }, []);

  async function enableAi() {
    setAiBusy(true);
    setAiError(null);
    setAiStatus(null);
    try {
      await invoke("ai_enable", {
        provider: aiProviderChoice,
        model: aiModel.trim() || null,
        cloudEndpoint: aiProviderChoice === "cloud" ? aiEndpoint.trim() || null : null,
      });
      if (aiProviderChoice === "cloud" && aiApiKey.trim()) {
        await invoke("ai_set_cloud_api_key", { key: aiApiKey.trim() });
        setAiApiKey("");
      }
      setAiStatus(t("settings.ai_enabled_status"));
      loadAiSettings();
    } catch (e) {
      setAiError(String(e));
    } finally {
      setAiBusy(false);
    }
  }

  async function disableAi() {
    setAiBusy(true);
    setAiError(null);
    try {
      await invoke("ai_disable");
      loadAiSettings();
    } catch (e) {
      setAiError(String(e));
    } finally {
      setAiBusy(false);
    }
  }

  async function runDiagnose() {
    if (!diagnoseContext.trim()) return;
    setDiagnosing(true);
    setDiagnoseError(null);
    setDiagnoseResult(null);
    try {
      const result = await invoke<string>("ai_diagnose", { rawContext: diagnoseContext });
      setDiagnoseResult(result);
    } catch (e) {
      setDiagnoseError(String(e));
    } finally {
      setDiagnosing(false);
    }
  }

  async function changeLanguage(lang: string) {
    // Switch immediately (instant feedback, and lets this work in a plain browser
    // preview with no Tauri runtime) — persistence to user_settings.language happens
    // in the background afterward, separately from the visible language switch.
    await i18n.changeLanguage(lang);
    setSaveState("saving");
    try {
      // Real IPC — persists via the already-tested gtavmm_core::settings module, not
      // just localStorage.
      await invoke("set_language", { language: lang });
      setSaveState("saved");
      setTimeout(() => setSaveState("idle"), 1500);
    } catch (e) {
      setSaveState("error");
      console.error(e);
    }
  }

  return (
    <section className="view" data-shown="true">
      <div className="page-head">
        <div>
          <h1 className="page-title">{t("settings.title")}</h1>
        </div>
      </div>

      <div className="panel" style={{ padding: "18px 20px" }}>
        <div className="eyebrow" style={{ marginBottom: 10 }}>
          {t("settings.language_label")}
        </div>
        <div style={{ display: "flex", gap: 10 }}>
          {SUPPORTED_LANGUAGES.map((lang) => (
            <button
              key={lang}
              type="button"
              className="btn-ghost"
              data-active={String(i18n.language === lang)}
              style={
                i18n.language === lang
                  ? { borderColor: "var(--accent)", color: "var(--accent)" }
                  : undefined
              }
              onClick={() => changeLanguage(lang)}
            >
              {LANGUAGE_LABELS[lang] ?? lang}
            </button>
          ))}
        </div>
        {saveState === "saving" && <p style={{ marginTop: 10 }}>{t("settings.saving")}</p>}
        {saveState === "saved" && (
          <p style={{ marginTop: 10, color: "var(--success)" }}>{t("settings.saved")}</p>
        )}
        {saveState === "error" && (
          <p className="error" style={{ marginTop: 10 }}>
            Failed to save (no Tauri runtime in this preview?)
          </p>
        )}
      </div>

      <div className="panel" style={{ padding: "18px 20px", marginTop: 16 }}>
        <div className="eyebrow" style={{ marginBottom: 10 }}>
          {t("settings.ai_section_label")}
        </div>
        <p className="page-sub" style={{ marginBottom: 14 }}>{t("settings.ai_section_intro")}</p>

        <button className="btn-ghost" type="button" onClick={() => setPromptLibraryOpen(true)} style={{ marginBottom: 14 }}>
          {t("promptLibrary.open_button")}
        </button>

        {aiSettings?.enabled ? (
          <>
            <p>
              {t("settings.ai_currently_enabled", {
                provider: aiSettings.provider,
                model: aiSettings.provider === "ollama" ? aiSettings.ollama_model : aiSettings.cloud_model,
              })}
            </p>
            <button className="btn-ghost" type="button" onClick={disableAi} disabled={aiBusy}>
              {t("settings.ai_disable_button")}
            </button>
          </>
        ) : (
          <>
            <div className="radio-row" style={{ marginBottom: 14 }}>
              <div
                className="radio-card"
                data-active={String(aiProviderChoice === "ollama")}
                onClick={() => setAiProviderChoice("ollama")}
              >
                {t("settings.ai_provider_ollama")}
              </div>
              <div
                className="radio-card"
                data-active={String(aiProviderChoice === "cloud")}
                onClick={() => setAiProviderChoice("cloud")}
              >
                {t("settings.ai_provider_cloud")}
              </div>
            </div>

            <div className="field-group">
              <label>{t("settings.ai_model_label")}</label>
              <input
                type="text"
                value={aiModel}
                onChange={(e) => setAiModel(e.target.value)}
                placeholder={aiProviderChoice === "ollama" ? "llama3" : "e.g. liquid/lfm-2.5-2.6b:free"}
              />
            </div>

            {aiProviderChoice === "cloud" && (
              <>
                <div className="field-group">
                  <label>{t("settings.ai_endpoint_label")}</label>
                  <input
                    type="text"
                    value={aiEndpoint}
                    onChange={(e) => setAiEndpoint(e.target.value)}
                    placeholder="https://openrouter.ai/api/v1/chat/completions"
                  />
                </div>
                <div className="field-group">
                  <label>
                    {t("settings.ai_api_key_label")}{" "}
                    {aiHasKey && (
                      <span style={{ color: "var(--success)", fontWeight: 600 }}>
                        {t("settings.ai_api_key_already_set")}
                      </span>
                    )}
                  </label>
                  <input
                    type="password"
                    value={aiApiKey}
                    onChange={(e) => setAiApiKey(e.target.value)}
                    placeholder={t("settings.ai_api_key_placeholder")}
                  />
                </div>
              </>
            )}

            <button className="btn-primary" type="button" onClick={enableAi} disabled={aiBusy}>
              {aiBusy ? t("settings.ai_enabling") : t("settings.ai_enable_button")}
            </button>
          </>
        )}

        {aiStatus && (
          <p style={{ marginTop: 10, color: "var(--success)" }}>{aiStatus}</p>
        )}
        {aiError && <p className="error" style={{ marginTop: 10 }}>{aiError}</p>}
      </div>

      {aiSettings?.enabled && (
        <div className="panel" style={{ padding: "18px 20px", marginTop: 16 }}>
          <div className="eyebrow" style={{ marginBottom: 10 }}>
            {t("settings.ai_diagnose_label")}
          </div>
          <p className="page-sub" style={{ marginBottom: 10 }}>{t("settings.ai_diagnose_intro")}</p>
          <div className="field-group">
            <textarea
              rows={6}
              value={diagnoseContext}
              onChange={(e) => setDiagnoseContext(e.target.value)}
              placeholder={t("settings.ai_diagnose_placeholder")}
            />
          </div>
          <button className="btn-ai" type="button" onClick={runDiagnose} disabled={diagnosing}>
            {diagnosing ? t("settings.ai_diagnosing") : t("settings.ai_diagnose_button")}
          </button>
          {diagnoseError && <p className="error" style={{ marginTop: 10 }}>{diagnoseError}</p>}
          {diagnoseResult && (
            <div className="diagnosis-box" style={{ marginTop: 14 }}>
              <div className="src">{t("settings.ai_diagnosis_source")}</div>
              {diagnoseResult}
              <p className="diagnosis-disclaimer">{t("settings.ai_diagnosis_disclaimer")}</p>
            </div>
          )}
        </div>
      )}

      <div className="panel" style={{ padding: "18px 20px", marginTop: 16 }}>
        <div className="eyebrow" style={{ marginBottom: 10 }}>
          {t("settings.update_section_label")}
        </div>
        <p className="page-sub" style={{ marginBottom: 10 }}>{t("settings.update_section_intro")}</p>
        <button className="btn-ghost" type="button" onClick={checkForUpdate} disabled={updateChecking}>
          {updateChecking ? t("settings.update_checking") : t("settings.update_check_button")}
        </button>
        {updateError && <p className="error" style={{ marginTop: 10 }}>{updateError}</p>}
        {updateResult && (
          <p style={{ marginTop: 10 }}>
            {updateResult.update_available ? (
              <>
                {t("settings.update_available", { version: updateResult.latest_version })}{" "}
                <a href={updateResult.platform_download_url ?? updateResult.release_url} target="_blank" rel="noopener noreferrer">
                  {t("settings.update_download_link")}
                </a>
              </>
            ) : (
              t("settings.update_up_to_date", { version: updateResult.current_version })
            )}
          </p>
        )}
      </div>

      <PromptLibraryModal open={promptLibraryOpen} onClose={() => setPromptLibraryOpen(false)} />
    </section>
  );
}
