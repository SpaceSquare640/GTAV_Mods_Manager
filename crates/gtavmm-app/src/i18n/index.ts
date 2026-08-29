// i18n setup (v0.8+, partial rollout — see project notes for exactly which pages are
// translated so far). Language choice persists via the real `get_language`/
// `set_language` Tauri commands (user_settings.language in the database), not
// localStorage — so it's the same setting the CLI/other tools would eventually read.
import i18n from "i18next";
import { initReactI18next } from "react-i18next";
import en from "./locales/en.json";
import zhTW from "./locales/zh-TW.json";

export const SUPPORTED_LANGUAGES = ["en", "zh-TW"] as const;
export type SupportedLanguage = (typeof SUPPORTED_LANGUAGES)[number];

i18n.use(initReactI18next).init({
  resources: {
    en: { translation: en },
    "zh-TW": { translation: zhTW },
  },
  lng: "en",
  fallbackLng: "en",
  interpolation: { escapeValue: false },
});

export default i18n;
