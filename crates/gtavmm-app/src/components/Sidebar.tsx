import { useTranslation } from "react-i18next";
import { Icon } from "./IconSprite";
import type { Mode, Sub } from "../types";

interface ModeGroupDef {
  mode: Mode;
  accentVar: string;
  softVar: string;
  subs: Sub[];
}

const MODE_GROUPS: ModeGroupDef[] = [
  { mode: "legacy", accentVar: "var(--accent-legacy)", softVar: "var(--accent-legacy-soft)", subs: ["mods", "lspdfr"] },
  { mode: "enhanced", accentVar: "var(--accent-enhanced)", softVar: "var(--accent-enhanced-soft)", subs: ["mods", "lspdfr"] },
  { mode: "fivem", accentVar: "var(--accent-fivem)", softVar: "var(--accent-fivem-soft)", subs: ["client", "server", "converter"] },
];

/** Nav labels reuse the `nav.*` translation keys; "mods" maps to "sp_mods" since the
 *  `Sub` type value doesn't match the key name directly. */
function subLabelKey(sub: Sub): string {
  return sub === "mods" ? "sp_mods" : sub;
}

interface SidebarProps {
  mode: Mode;
  sub: Sub;
  onSelect: (mode: Mode, sub: Sub) => void;
  onOpenSettings: () => void;
  onOpenProfiles: () => void;
}

export function Sidebar({ mode, sub, onSelect, onOpenSettings, onOpenProfiles }: SidebarProps) {
  const { t } = useTranslation();
  return (
    <aside className="sidebar">
      <div className="brand">
        <div className="brand-mark">V</div>
        <div className="brand-text">
          <span className="brand-name">{t("brand.name")}</span>
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
                onClick={() => onSelect(group.mode, group.subs[0])}
              >
                <span className="mode-dot"></span> {t(`nav.${group.mode}`)}
              </button>
              <div className="sub-tabs">
                {group.subs.map((s) => (
                  <button
                    key={s}
                    className={
                      group.mode === "legacy" && s === "lspdfr"
                        ? "sub-btn lspdfr-btn"
                        : "sub-btn"
                    }
                    type="button"
                    data-active={String(group.mode === mode && s === sub)}
                    onClick={() => onSelect(group.mode, s)}
                  >
                    {t(`nav.${subLabelKey(s)}`)}
                  </button>
                ))}
              </div>
            </div>
          ))}
        </nav>
      </div>

      <div className="sidebar-foot">
        <button className="settings-btn" type="button" onClick={onOpenProfiles}>
          <span className="gear">
            <Icon name="layers" />
          </span>{" "}
          {t("nav.profiles")}
        </button>
        <button className="settings-btn" type="button" onClick={onOpenSettings}>
          <span className="gear">
            <Icon name="settings" />
          </span>{" "}
          {t("nav.settings")}
        </button>
      </div>
    </aside>
  );
}
