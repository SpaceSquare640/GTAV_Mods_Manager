import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import type { PromptTemplate } from "../types";

interface PromptLibraryModalProps {
  open: boolean;
  onClose: () => void;
}

/** The AI Workflow / Prompt library — a modal per the design, opened from Settings'
 *  AI Assistant section. Plain CRUD over the user's own reusable prompt text; never
 *  applied or executed automatically (not part of the AI Assistant's Action Schema). */
export function PromptLibraryModal({ open, onClose }: PromptLibraryModalProps) {
  const { t } = useTranslation();
  const [prompts, setPrompts] = useState<PromptTemplate[] | null>(null);
  const [error, setError] = useState<string | null>(null);

  const [newName, setNewName] = useState("");
  const [newContent, setNewContent] = useState("");
  const [editingId, setEditingId] = useState<number | null>(null);
  const [editName, setEditName] = useState("");
  const [editContent, setEditContent] = useState("");

  const loadPrompts = useCallback(() => {
    invoke<PromptTemplate[]>("list_prompt_templates")
      .then(setPrompts)
      .catch((e) => setError(String(e)));
  }, []);

  useEffect(() => {
    if (open) loadPrompts();
  }, [open, loadPrompts]);

  if (!open) return null;

  async function addPrompt() {
    if (!newName.trim() || !newContent.trim()) return;
    try {
      await invoke("add_prompt_template", { name: newName.trim(), content: newContent.trim() });
      setNewName("");
      setNewContent("");
      loadPrompts();
    } catch (e) {
      setError(String(e));
    }
  }

  function startEdit(p: PromptTemplate) {
    setEditingId(p.id);
    setEditName(p.name);
    setEditContent(p.content);
  }

  async function saveEdit() {
    if (editingId === null || !editName.trim() || !editContent.trim()) return;
    try {
      await invoke("update_prompt_template", { id: editingId, name: editName.trim(), content: editContent.trim() });
      setEditingId(null);
      loadPrompts();
    } catch (e) {
      setError(String(e));
    }
  }

  async function deletePrompt(id: number) {
    try {
      await invoke("delete_prompt_template", { id });
      loadPrompts();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <div className="modal-backdrop" data-open="true" onClick={(e) => e.target === e.currentTarget && onClose()}>
      <div className="modal" style={{ width: 520 }}>
        <div className="modal-head">
          <h2>{t("promptLibrary.title")}</h2>
          <button className="drawer-close" type="button" aria-label="Close" onClick={onClose}>
            ×
          </button>
        </div>
        <div className="modal-body">
          <p style={{ marginTop: -4 }}>{t("promptLibrary.intro")}</p>

          {error && <p className="error">{error}</p>}

          {prompts?.map((p) =>
            editingId === p.id ? (
              <div className="prompt-row" key={p.id}>
                <input type="text" value={editName} onChange={(e) => setEditName(e.target.value)} style={{ width: "100%", marginBottom: 6 }} />
                <textarea rows={4} value={editContent} onChange={(e) => setEditContent(e.target.value)} style={{ width: "100%", marginBottom: 8 }} />
                <div style={{ display: "flex", gap: 8 }}>
                  <button className="btn-primary" type="button" onClick={saveEdit}>
                    {t("promptLibrary.save_button")}
                  </button>
                  <button className="icon-btn" type="button" onClick={() => setEditingId(null)}>
                    {t("promptLibrary.cancel_button")}
                  </button>
                </div>
              </div>
            ) : (
              <div className="prompt-row" key={p.id}>
                <div className="top">
                  <span className="name">{p.name}</span>
                  <div className="cfg-actions">
                    <button className="icon-btn" type="button" onClick={() => startEdit(p)}>
                      {t("promptLibrary.edit_button")}
                    </button>
                    <button className="icon-btn" type="button" onClick={() => deletePrompt(p.id)}>
                      {t("promptLibrary.delete_button")}
                    </button>
                  </div>
                </div>
                <div className="body-text">{p.content}</div>
              </div>
            )
          )}
          {prompts && prompts.length === 0 && <p className="page-sub">{t("promptLibrary.empty")}</p>}

          <div style={{ marginTop: 16, paddingTop: 16, borderTop: "1px solid var(--border)" }}>
            <div className="field-group" style={{ marginBottom: 8 }}>
              <label>{t("promptLibrary.name_label")}</label>
              <input type="text" value={newName} onChange={(e) => setNewName(e.target.value)} />
            </div>
            <div className="field-group" style={{ marginBottom: 8 }}>
              <label>{t("promptLibrary.content_label")}</label>
              <textarea rows={3} value={newContent} onChange={(e) => setNewContent(e.target.value)} />
            </div>
            <button className="btn-primary" type="button" onClick={addPrompt}>
              {t("promptLibrary.add_button")}
            </button>
          </div>
        </div>
        <div className="modal-foot">
          <button className="btn-primary" type="button" onClick={onClose}>
            {t("promptLibrary.close_button")}
          </button>
        </div>
      </div>
    </div>
  );
}
