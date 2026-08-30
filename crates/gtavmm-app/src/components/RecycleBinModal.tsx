import { useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import type { RecycleBinEntry } from "../types";

interface RecycleBinModalProps {
  open: boolean;
  onClose: () => void;
  entries: RecycleBinEntry[];
  onRestored: () => void;
}

/** The full recycle bin list + restore action, opened via each mods page's "View all"
 *  link — the mini-panel embedded in the page only shows a preview. */
export function RecycleBinModal({ open, onClose, entries, onRestored }: RecycleBinModalProps) {
  const { t } = useTranslation();
  const [restoringId, setRestoringId] = useState<number | null>(null);
  const [error, setError] = useState<string | null>(null);

  if (!open) return null;

  async function restore(id: number) {
    setRestoringId(id);
    setError(null);
    try {
      await invoke("restore_recycle_bin_entry", { entryId: id, gamePath: null });
      onRestored();
    } catch (e) {
      setError(String(e));
    } finally {
      setRestoringId(null);
    }
  }

  return (
    <div className="modal-backdrop" data-open="true" onClick={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal" style={{ width: 460 }}>
        <div className="modal-head">
          <h2>{t("recycleBinModal.title")}</h2>
          <button className="drawer-close" type="button" aria-label="Close" onClick={onClose}>
            ×
          </button>
        </div>
        <div className="modal-body">
          {error && <p className="error">{error}</p>}
          {entries.length === 0 && (
            <div className="rb-empty">
              <span className="glyph">🗑</span>
              {t("recycleBinModal.empty")}
            </div>
          )}
          {entries.map((entry) => (
            <div className="rb-row" key={entry.id}>
              <span className="name">#{entry.id}</span>
              <div className="meta">
                <span>{entry.deleted_at.slice(0, 10)}</span>
                <span className="expiry">{t("recycleBinModal.expires", { date: entry.expires_at.slice(0, 10) })}</span>
              </div>
              <button
                className="btn-ghost"
                type="button"
                onClick={() => restore(entry.id)}
                disabled={restoringId === entry.id}
              >
                {restoringId === entry.id ? t("recycleBinModal.restoring") : t("recycleBinModal.restore_button")}
              </button>
            </div>
          ))}
        </div>
        <div className="modal-foot">
          <button className="btn-primary" type="button" onClick={onClose}>
            {t("recycleBinModal.close")}
          </button>
        </div>
      </div>
    </div>
  );
}
