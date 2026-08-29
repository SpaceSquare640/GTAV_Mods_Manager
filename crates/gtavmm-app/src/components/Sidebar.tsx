import { Icon } from "./IconSprite";
import type { Mode, Sub } from "../types";

interface ModeGroupDef {
  mode: Mode;
  label: string;
  accentVar: string;
  softVar: string;
  subs: { sub: Sub; label: string }[];
}

const MODE_GROUPS: ModeGroupDef[] = [
  {
    mode: "legacy",
    label: "Legacy",
    accentVar: "var(--accent-legacy)",
    softVar: "var(--accent-legacy-soft)",
    subs: [
      { sub: "mods", label: "SP Mods" },
      { sub: "lspdfr", label: "LSPDFR" },
    ],
  },
  {
    mode: "enhanced",
    label: "Enhanced",
    accentVar: "var(--accent-enhanced)",
    softVar: "var(--accent-enhanced-soft)",
    subs: [
      { sub: "mods", label: "SP Mods" },
      { sub: "lspdfr", label: "LSPDFR" },
    ],
  },
  {
    mode: "fivem",
    label: "FiveM",
    accentVar: "var(--accent-fivem)",
    softVar: "var(--accent-fivem-soft)",
    subs: [
      { sub: "client", label: "Client" },
      { sub: "server", label: "Server" },
      { sub: "converter", label: "Converter" },
    ],
  },
];

interface SidebarProps {
  mode: Mode;
  sub: Sub;
  onSelect: (mode: Mode, sub: Sub) => void;
  onOpenSettings: () => void;
}

export function Sidebar({ mode, sub, onSelect, onOpenSettings }: SidebarProps) {
  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-mark">V</div>
        <div className="brand-text">
          <span className="brand-name">GTAV Mods Manager</span>
          <span className="brand-version">
            v0.0.1 <span className="mono">· win-x64</span>
          </span>
        </div>
      </div>

      <div className="nav-wrap">
        <nav className="nav">
          {MODE_GROUPS.map((group) => (
            <div
              key={group.mode}
              className="mode-group"
              data-active={String(group.mode === mode)}
              style={
                {
                  "--mode-accent": group.accentVar,
                  "--mode-soft": group.softVar,
                } as React.CSSProperties
              }
            >
              <button
                className="mode-btn"
                type="button"
                onClick={() => onSelect(group.mode, group.subs[0].sub)}
              >
                <span className="mode-dot"></span> {group.label}
              </button>
              <div className="sub-tabs">
                {group.subs.map((s) => (
                  <button
                    key={s.sub}
                    className={
                      group.mode === "legacy" && s.sub === "lspdfr"
                        ? "sub-btn lspdfr-btn"
                        : "sub-btn"
                    }
                    type="button"
                    data-active={String(group.mode === mode && s.sub === sub)}
                    onClick={() => onSelect(group.mode, s.sub)}
                  >
                    {s.label}
                  </button>
                ))}
              </div>
            </div>
          ))}
        </nav>
      </div>

      <div className="sidebar-foot">
        <button className="settings-btn" type="button" onClick={onOpenSettings}>
          <span className="gear">
            <Icon name="settings" />
          </span>{" "}
          Settings
        </button>
      </div>
    </aside>
  );
}
