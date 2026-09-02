import { useState } from "react";
import { useTranslation } from "react-i18next";
import { ModDetailDrawer } from "./ModDetailDrawer";
import type { InstalledMod } from "../types";

interface ModTableProps {
  mods: InstalledMod[];
  onChanged: () => void;
  /** Forwarded to the drawer's lifecycle calls; null lets the backend detect. */
  gamePath?: string | null;
  /** Which provider a reinstall from this page should use. */
  mode?: string;
}

/**
 * The mod table shared by every workspace page that lists `installed_mod` rows
 * (Legacy/Enhanced SP, Legacy/Enhanced LSPDFR — FiveM Client deliberately does
 * not use this, see its own page for why).
 *
 * A row opens the detail drawer rather than expanding an editor inline. The
 * inline row could only edit notes and the source link; the drawer is also
 * where disable, enable, reinstall and uninstall live, so there is one place to
 * act on a mod instead of an editor here and actions elsewhere.
 */
export function ModTable({ mods, onChanged, gamePath = null, mode = "sp" }: ModTableProps) {
  const { t } = useTranslation();
  const [openId, setOpenId] = useState<number | null>(null);

  const statusLabel: Record<InstalledMod["status"], string> = {
    Active: t("legacySp.status_active"),
    Disabled: t("legacySp.status_disabled"),
    Uninstalled: t("legacySp.status_uninstalled"),
  };
  // "pill--off" rather than "pill disabled": the design renamed it so a mod the
  // user switched off is not described with the same word as a control that
  // cannot be used. The value is a complete class list, not a modifier appended
  // to "pill", which is why "active" keeps its leading "pill ".
  const statusClass: Record<InstalledMod["status"], string> = {
    Active: "pill active",
    Disabled: "pill pill--off",
    Uninstalled: "pill pill--off",
  };

  // Read from the freshest list rather than holding a copy, so the drawer shows
  // the new status straight after an action instead of the pre-action snapshot.
  const openMod = mods.find((m) => m.id === openId) ?? null;

  return (
    <>
      <table>
        <thead>
          <tr>
            <th>{t("legacySp.col_mod")}</th>
            <th>{t("legacySp.col_status")}</th>
            <th>{t("legacySp.col_installed")}</th>
            <th>{t("legacySp.col_root")}</th>
            <th>{t("legacySp.col_link")}</th>
            <th />
          </tr>
        </thead>
        <tbody>
          {mods.map((m) => (
            <tr className="mod-row" key={m.id}>
              <td>
                <div className="mod-name">{m.name}</div>
                <div className="mod-type">.{m.source_type}</div>
              </td>
              <td>
                <span className={statusClass[m.status]}>{statusLabel[m.status]}</span>
              </td>
              <td className="mono">{m.installed_at.slice(0, 10)}</td>
              <td className="path mono">{m.install_path}</td>
              <td>
                {m.link ? (
                  <a
                    href={m.link}
                    target="_blank"
                    rel="noopener noreferrer"
                    className="mono"
                    style={{ fontSize: "var(--fs-xs)" }}
                  >
                    {t("legacySp.open_link")}
                  </a>
                ) : (
                  <span className="path">—</span>
                )}
              </td>
              <td style={{ textAlign: "right" }}>
                <button className="icon-btn" type="button" onClick={() => setOpenId(m.id)}>
                  {t("drawer.details")}
                </button>
              </td>
            </tr>
          ))}
        </tbody>
      </table>

      <ModDetailDrawer
        mod={openMod}
        gamePath={gamePath}
        mode={mode}
        onClose={() => setOpenId(null)}
        onChanged={onChanged}
      />
    </>
  );
}
