import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import type { SavedModLink } from "../types";

/**
 * Standalone mod-link bookmarks (gtavmm_core::saved_links) — deliberately independent
 * of installed mods, so a link is worth saving before a mod is ever installed (or
 * after it's uninstalled). Pure CRUD; nothing here fetches or validates the URL.
 */
export function SavedLinksPage() {
  const { t } = useTranslation();
  const [links, setLinks] = useState<SavedModLink[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [newName, setNewName] = useState("");
  const [newUrl, setNewUrl] = useState("");
  const [newNotes, setNewNotes] = useState("");

  const [editingId, setEditingId] = useState<number | null>(null);
  const [editName, setEditName] = useState("");
  const [editUrl, setEditUrl] = useState("");
  const [editNotes, setEditNotes] = useState("");

  const loadLinks = useCallback(() => {
    invoke<SavedModLink[]>("list_saved_links")
      .then(setLinks)
      .catch((e) => setError(String(e)));
  }, []);

  useEffect(() => {
    loadLinks();
  }, [loadLinks]);

  async function addLink() {
    if (!newName.trim() || !newUrl.trim()) return;
    try {
      await invoke("add_saved_link", {
        name: newName.trim(),
        url: newUrl.trim(),
        notes: newNotes.trim() || null,
      });
      setNewName("");
      setNewUrl("");
      setNewNotes("");
      loadLinks();
    } catch (e) {
      setError(String(e));
    }
  }

  function startEdit(link: SavedModLink) {
    setEditingId(link.id);
    setEditName(link.name);
    setEditUrl(link.url);
    setEditNotes(link.notes ?? "");
  }

  function cancelEdit() {
    setEditingId(null);
  }

  async function saveEdit() {
    if (editingId === null || !editName.trim() || !editUrl.trim()) return;
    try {
      await invoke("update_saved_link", {
        id: editingId,
        name: editName.trim(),
        url: editUrl.trim(),
        notes: editNotes.trim() || null,
      });
      setEditingId(null);
      loadLinks();
    } catch (e) {
      setError(String(e));
    }
  }

  async function deleteLink(id: number) {
    try {
      await invoke("delete_saved_link", { id });
      loadLinks();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <section className="view" data-shown="true">
      <div className="page-head">
        <div>
          <h1 className="page-title">{t("savedLinks.title")}</h1>
          <p className="page-sub">{t("savedLinks.subtitle")}</p>
        </div>
      </div>

      {error && <p className="error">{error}</p>}

      <div className="panel" style={{ padding: "18px 20px", marginBottom: 16 }}>
        <div className="field-group" style={{ marginBottom: 8 }}>
          <label>{t("savedLinks.name_label")}</label>
          <input type="text" value={newName} onChange={(e) => setNewName(e.target.value)} placeholder={t("savedLinks.name_placeholder")} />
        </div>
        <div className="field-group" style={{ marginBottom: 8 }}>
          <label>{t("savedLinks.url_label")}</label>
          <input type="text" value={newUrl} onChange={(e) => setNewUrl(e.target.value)} placeholder="https://www.gta5-mods.com/..." />
        </div>
        <div className="field-group" style={{ marginBottom: 8 }}>
          <label>{t("savedLinks.notes_label")}</label>
          <input type="text" value={newNotes} onChange={(e) => setNewNotes(e.target.value)} />
        </div>
        <button className="btn-primary" type="button" onClick={addLink}>
          {t("savedLinks.add_button")}
        </button>
      </div>

      <div className="panel">
        {links === null && !error && <p style={{ padding: "16px 20px" }}>{t("savedLinks.loading")}</p>}
        {links && links.length === 0 && <p style={{ padding: "16px 20px" }}>{t("savedLinks.empty")}</p>}
        {links && links.length > 0 && (
          <div className="config-list" style={{ padding: "14px 16px 0" }}>
            {links.map((link) => (
              <div className="config-row" key={link.id} style={{ flexDirection: "column", alignItems: "stretch", gap: 6 }}>
                {editingId === link.id ? (
                  <>
                    <input type="text" value={editName} onChange={(e) => setEditName(e.target.value)} />
                    <input type="text" value={editUrl} onChange={(e) => setEditUrl(e.target.value)} />
                    <input type="text" value={editNotes} onChange={(e) => setEditNotes(e.target.value)} />
                    <div style={{ display: "flex", gap: 8 }}>
                      <button className="btn-primary" type="button" onClick={saveEdit}>
                        {t("savedLinks.save_button")}
                      </button>
                      <button className="icon-btn" type="button" onClick={cancelEdit}>
                        {t("savedLinks.cancel_button")}
                      </button>
                    </div>
                  </>
                ) : (
                  <>
                    <div style={{ display: "flex", alignItems: "center", gap: 10 }}>
                      <span className="cfg-name" style={{ flex: 1 }}>
                        {link.name}
                      </span>
                      <div className="cfg-actions">
                        <button className="icon-btn" type="button" onClick={() => startEdit(link)}>
                          {t("savedLinks.edit_button")}
                        </button>
                        <button className="icon-btn" type="button" onClick={() => deleteLink(link.id)}>
                          {t("savedLinks.delete_button")}
                        </button>
                      </div>
                    </div>
                    <a href={link.url} target="_blank" rel="noopener noreferrer" className="mono" style={{ fontSize: "12px" }}>
                      {link.url}
                    </a>
                    {link.notes && <span className="page-sub">{link.notes}</span>}
                  </>
                )}
              </div>
            ))}
          </div>
        )}
      </div>
    </section>
  );
}
