import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { IconSprite, Icon } from "./components/IconSprite";
import { Sidebar } from "./components/Sidebar";
import { LegacySpPage } from "./pages/LegacySpPage";
import { LegacyLspdfrPage } from "./pages/LegacyLspdfrPage";
import { EnhancedSpPage } from "./pages/EnhancedSpPage";
import { EnhancedLspdfrPage } from "./pages/EnhancedLspdfrPage";
import { FiveMClientPage } from "./pages/FiveMClientPage";
import { FiveMServerPage } from "./pages/FiveMServerPage";
import { FiveMConverterPage } from "./pages/FiveMConverterPage";
import { SettingsPage } from "./pages/SettingsPage";
import { ProfilesPage } from "./pages/ProfilesPage";
import { DllTranslationPage } from "./pages/DllTranslationPage";
import { ActivityLogPage } from "./pages/ActivityLogPage";
import { SavedLinksPage } from "./pages/SavedLinksPage";
import { ToolsPage } from "./pages/ToolsPage";
import { PlaceholderPage } from "./pages/PlaceholderPage";
import type { Mode, Sub } from "./types";
import "./styles/mockup.css";
import "./App.css";

const ACCENT_VAR: Record<Mode, string> = {
  legacy: "--accent-legacy",
  enhanced: "--accent-enhanced",
  fivem: "--accent-fivem",
};
const ACCENT_SOFT_VAR: Record<Mode, string> = {
  legacy: "--accent-legacy-soft",
  enhanced: "--accent-enhanced-soft",
  fivem: "--accent-fivem-soft",
};

function pageFor(mode: Mode, sub: Sub) {
  if (mode === "legacy" && sub === "mods") return <LegacySpPage />;
  if (mode === "legacy" && sub === "lspdfr") return <LegacyLspdfrPage />;
  if (mode === "enhanced" && sub === "mods") return <EnhancedSpPage />;
  if (mode === "enhanced" && sub === "lspdfr") return <EnhancedLspdfrPage />;
  if (mode === "fivem" && sub === "client") return <FiveMClientPage />;
  if (mode === "fivem" && sub === "server") return <FiveMServerPage />;
  if (mode === "fivem" && sub === "converter") return <FiveMConverterPage />;
  const titles: Record<string, string> = {
    "legacy-lspdfr": "LSPDFR · Legacy",
    "enhanced-mods": "SP Mods · Enhanced",
    "enhanced-lspdfr": "LSPDFR · Enhanced",
    "fivem-client": "FiveM · Client",
    "fivem-server": "FiveM · Server",
    "fivem-converter": "FiveM · Converter",
  };
  return <PlaceholderPage title={titles[`${mode}-${sub}`] ?? `${mode} / ${sub}`} />;
}

function App() {
  const { t, i18n } = useTranslation();
  const [mode, setMode] = useState<Mode>("legacy");
  const [sub, setSub] = useState<Sub>("mods");
  type Overlay =
    | "settings"
    | "profiles"
    | "dllTranslation"
    | "activityLog"
    | "savedLinks"
    | "tools"
    | null;
  const [overlay, setOverlay] = useState<Overlay>(null);

  // Load the persisted language (user_settings.language) on startup, not just
  // whatever i18next's own default is — same setting the CLI/other tools would read.
  useEffect(() => {
    invoke<string>("get_language")
      .then((lang) => {
        if (lang) i18n.changeLanguage(lang);
      })
      .catch(() => {
        // No Tauri runtime (plain browser preview) — stay on the default language.
      });
  }, [i18n]);

  const style = {
    "--accent": `var(${ACCENT_VAR[mode]})`,
    "--accent-soft": `var(${ACCENT_SOFT_VAR[mode]})`,
  } as React.CSSProperties;

  return (
    <div className="app" style={style}>
      <IconSprite />
      <Sidebar
        mode={mode}
        sub={sub}
        onSelect={(m, s) => {
          setMode(m);
          setSub(s);
          setOverlay(null);
        }}
        onOpenSettings={() => setOverlay("settings")}
        onOpenProfiles={() => setOverlay("profiles")}
        onOpenDllTranslation={() => setOverlay("dllTranslation")}
        onOpenActivityLog={() => setOverlay("activityLog")}
        onOpenSavedLinks={() => setOverlay("savedLinks")}
        onOpenTools={() => setOverlay("tools")}
      />
      <main className="main">
        <div className="topbar">
          <div className="crumb">
            {overlay === "settings" ? (
              <strong>{t("nav.settings")}</strong>
            ) : overlay === "profiles" ? (
              <strong>{t("nav.profiles")}</strong>
            ) : overlay === "dllTranslation" ? (
              <strong>{t("nav.dll_translation")}</strong>
            ) : overlay === "activityLog" ? (
              <strong>{t("nav.activity_log")}</strong>
            ) : overlay === "savedLinks" ? (
              <strong>{t("nav.saved_links")}</strong>
            ) : overlay === "tools" ? (
              <strong>{t("nav.tools")}</strong>
            ) : (
              <>
                <strong>{t(`nav.${mode}`)}</strong>
                <span className="sep">/</span>
                {t(`nav.${sub === "mods" ? "sp_mods" : sub}`)}
              </>
            )}
          </div>
          <div className="search">
            <Icon name="search" /> {t("topbar.search_placeholder")}
          </div>
        </div>
        <div className="content">
          {overlay === "settings" ? (
            <SettingsPage />
          ) : overlay === "profiles" ? (
            <ProfilesPage />
          ) : overlay === "dllTranslation" ? (
            <DllTranslationPage />
          ) : overlay === "activityLog" ? (
            <ActivityLogPage />
          ) : overlay === "savedLinks" ? (
            <SavedLinksPage />
          ) : overlay === "tools" ? (
            <ToolsPage />
          ) : (
            pageFor(mode, sub)
          )}
        </div>
      </main>
    </div>
  );
}

export default App;
