import { useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { SUPPORTED_LANGUAGES } from "../i18n";

const LANGUAGE_LABELS: Record<string, string> = {
  en: "English",
  "zh-TW": "繁體中文",
};

export function SettingsPage() {
  const { t, i18n } = useTranslation();
  const [saveState, setSaveState] = useState<"idle" | "saving" | "saved" | "error">(
    "idle"
  );

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
    </section>
  );
}
