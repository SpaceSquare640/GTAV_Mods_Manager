import { useState } from "react";
import { IconSprite, Icon } from "./components/IconSprite";
import { Sidebar } from "./components/Sidebar";
import { LegacySpPage } from "./pages/LegacySpPage";
import { FiveMServerPage } from "./pages/FiveMServerPage";
import { PlaceholderPage } from "./pages/PlaceholderPage";
import type { Mode, Sub } from "./types";
import "./styles/mockup.css";
import "./App.css";

const CRUMB_LABEL: Record<Mode, string> = {
  legacy: "Legacy",
  enhanced: "Enhanced",
  fivem: "FiveM",
};

const SUB_LABEL: Record<Sub, string> = {
  mods: "SP Mods",
  lspdfr: "LSPDFR",
  client: "Client",
  server: "Server",
  converter: "Converter",
};

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
  if (mode === "fivem" && sub === "server") return <FiveMServerPage />;
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
  const [mode, setMode] = useState<Mode>("legacy");
  const [sub, setSub] = useState<Sub>("mods");
  const [showSettings, setShowSettings] = useState(false);

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
          setShowSettings(false);
        }}
        onOpenSettings={() => setShowSettings(true)}
      />
      <main className="main">
        <div className="topbar">
          <div className="crumb">
            <strong>{CRUMB_LABEL[mode]}</strong>
            <span className="sep">/</span>
            {SUB_LABEL[sub]}
          </div>
          <div className="search">
            <Icon name="search" /> Search installed mods… (not wired yet)
          </div>
        </div>
        <div className="content">
          {showSettings ? <PlaceholderPage title="Settings" /> : pageFor(mode, sub)}
        </div>
      </main>
    </div>
  );
}

export default App;
