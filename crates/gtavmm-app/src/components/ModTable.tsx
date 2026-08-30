import { Fragment, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import type { InstalledMod } from "../types";

interface ModTableProps {
  mods: InstalledMod[];
  onChanged: () => void;
}

/**
 * The mod/status/installed/root/link table shared by every workspace page that lists
 * `installed_mod` rows (Legacy/Enhanced SP, Legacy/Enhanced LSPDFR — FiveM Client
 * deliberately doesn't use this, see its own page's doc comment for why). Previously
 * each page duplicated this table inline; a link/notes edit affordance was added to
 * the Legacy SP copy first as a reference implementation, then rolled out to the rest
 * by extracting it here instead of copy-pasting the edit logic three more times.
 */
export function ModTable({ mods, onChanged }: ModTableProps) {
  const { t } = useTranslation();
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editNotes, setEditNotes] = useState("");
  const [editLink, setEditLink] = useState("");
  const [error, setError] = useState<string | null>(null);

  const statusLabel: Record<InstalledMod["status"], string> = {
    Active: t("legacySp.status_active"),
    Disabled: t("legacySp.status_disabled"),
    Uninstalled: t("legacySp.status_uninstalled"),
  };
  const statusClass: Record<InstalledMod["status"], string> = {
    Active: "active",
    Disabled: "disabled",
    Uninstalled: "disabled",
  };

  function startEdit(m: InstalledMod) {
    setEditingId(m.id);
    setEditNotes(m.notes ?? "");
    setEditLink(m.link ?? "");
  }

  async function saveEdit(modId: number) {
    try {
      await invoke("update_mod_details", {
        modId,
        notes: editNotes.trim() || null,
        link: editLink.trim() || null,
      });
      setEditingId(null);
      onChanged();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <>
      {error && <p className="error">{error}</p>}
      <table>
        <thead>
          <tr>
            <th>{t("legacySp.col_mod")}</th>
            <th>{t("legacySp.col_status")}</th>
            <th>{t("legacySp.col_installed")}</th>
            <th>{t("legacySp.col_root")}</th>
            <th>{t("legacySp.col_link")}</th>
          </tr>
        </thead>
        <tbody>
          {mods.map((m) => (
            <Fragment key={m.id}>
              <tr className="mod-row">
                <td>
                  <div className="mod-name">{m.name}</div>
                  <div className="mod-type">.{m.source_type}</div>
                </td>
                <td>
                  <span className={`pill ${statusClass[m.status]}`}>{statusLabel[m.status]}</span>
                </td>
                <td className="mono">{m.installed_at.slice(0, 10)}</td>
                <td className="path mono">{m.install_path}</td>
                <td>
                  {m.link ? (
                    <a href={m.link} target="_blank" rel="noopener noreferrer" className="mono" style={{ fontSize: "11.5px" }}>
                      {t("legacySp.open_link")}
                    </a>
                  ) : (
                    <span className="path">—</span>
                  )}{" "}
                  <button className="icon-btn" type="button" onClick={() => startEdit(m)}>
                    {t("legacySp.edit_link_button")}
                  </button>
                </td>
              </tr>
              {editingId === m.id && (
                <tr>
                  <td colSpan={5} style={{ background: "var(--surface-2)" }}>
                    <div style={{ display: "flex", gap: 8, padding: "10px 4px", flexWrap: "wrap", alignItems: "center" }}>
                      <input
                        type="text"
                        value={editLink}
                        onChange={(e) => setEditLink(e.target.value)}
                        placeholder={t("legacySp.link_placeholder")}
                        style={{ flex: 1, minWidth: 200 }}
                      />
                      <input
                        type="text"
                        value={editNotes}
                        onChange={(e) => setEditNotes(e.target.value)}
                        placeholder={t("legacySp.notes_placeholder")}
                        style={{ flex: 1, minWidth: 200 }}
                      />
                      <button className="btn-primary" type="button" onClick={() => saveEdit(m.id)}>
                        {t("legacySp.save_link_button")}
                      </button>
                      <button className="icon-btn" type="button" onClick={() => setEditingId(null)}>
                        {t("legacySp.cancel_link_button")}
                      </button>
                    </div>
                  </td>
                </tr>
              )}
            </Fragment>
          ))}
        </tbody>
      </table>
    </>
  );
}
