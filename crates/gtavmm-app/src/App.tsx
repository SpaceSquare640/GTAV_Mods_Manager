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
import type { Mode, ModSearchResult, Sub } from "./types";
import "./styles/mockup.css";
import "./App.css";

/* The accent is switched by setting data-mode on the app root and letting the
   stylesheet do the rest — mockup.css carries a [data-mode="…"] rule per mode
   that assigns --accent and --accent-soft. The previous approach wrote an
   inline style referencing --accent-legacy and friends, which silently broke
   when the design renamed those to --mode-legacy: var() resolved to nothing,
   so --accent was empty and every component reading it lost its colour. An
   attribute cannot fail that way — if the mode is wrong the rule simply does
   not match, which is visible immediately rather than degrading to blank. */

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

  const [searchQuery, setSearchQuery] = useState("");
  const [searchResults, setSearchResults] = useState<ModSearchResult[] | null>(null);
  const [searchOpen, setSearchOpen] = useState(false);

  // Real, fully local keyword search (gtavmm_core::mod_search) — not natural-language
  // understanding, see that module's own doc comment for why. Debounced by a plain
  // timeout since there's no dedicated search-hook infrastructure in this app yet.
  useEffect(() => {
    if (!searchQuery.trim()) {
      setSearchResults(null);
      return;
    }
    const handle = setTimeout(() => {
      invoke<ModSearchResult[]>("search_mods", { query: searchQuery })
        .then(setSearchResults)
        .catch(() => setSearchResults(null));
    }, 200);
    return () => clearTimeout(handle);
  }, [searchQuery]);

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

  // LSPDFR has no accent of its own: it is a sub-tab under both editions, so a
  // page beneath it keeps its parent edition's colour and carries its identity
  // through the gradient badge and shield icon instead.
  return (
    <div className="app" data-mode={mode}>
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
          <div className="dropdown" data-open={String(searchOpen)} style={{ marginLeft: "auto" }}>
            <div className="search">
              <Icon name="search" />
              <input
                type="text"
                value={searchQuery}
                onChange={(e) => setSearchQuery(e.target.value)}
                onFocus={() => setSearchOpen(true)}
                onBlur={() => setTimeout(() => setSearchOpen(false), 150)}
                placeholder={t("topbar.search_placeholder")}
                style={{
                  background: "none",
                  border: "none",
                  outline: "none",
                  color: "inherit",
                  font: "inherit",
                  width: "100%",
                }}
              />
            </div>
            {searchOpen && searchQuery.trim() && (
              <div className="dd-menu" style={{ width: 260, maxHeight: 280, overflowY: "auto" }}>
                {searchResults === null && <div className="dd-option">{t("topbar.search_loading")}</div>}
                {searchResults && searchResults.length === 0 && (
                  <div className="dd-option">{t("topbar.search_empty")}</div>
                )}
                {searchResults?.map((r) => (
                  <div className="dd-option" key={r.id} style={{ flexDirection: "column", alignItems: "stretch" }}>
                    <span style={{ fontWeight: 600 }}>{r.name}</span>
                    <span style={{ fontSize: 11, color: "var(--text-faint)" }}>{r.status}</span>
                  </div>
                ))}
              </div>
            )}
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
