import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import type { InstalledMod, Profile, SwitchOutcome } from "../types";

/**
 * Real profile management against the real gtavmm-core `profile` module — create,
 * delete, switch, and per-mod membership (opt-in per mod, same as the CLI: a mod not
 * assigned to any profile is never touched by switch). Export/import aren't wired into
 * this page yet (still CLI-only) — see the Wiki for the full command list.
 */
export function ProfilesPage() {
  const { t } = useTranslation();
  const [profiles, setProfiles] = useState<Profile[] | null>(null);
  const [mods, setMods] = useState<InstalledMod[] | null>(null);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [memberIds, setMemberIds] = useState<Set<number>>(new Set());
  const [newName, setNewName] = useState("");
  const [error, setError] = useState<string | null>(null);
  const [switchResult, setSwitchResult] = useState<SwitchOutcome | null>(null);

  const loadProfiles = useCallback(() => {
    invoke<Profile[]>("profile_list")
      .then(setProfiles)
      .catch((e) => setError(String(e)));
  }, []);

  useEffect(() => {
    loadProfiles();
    invoke<InstalledMod[]>("list_mods")
      .then(setMods)
      .catch((e) => setError(String(e)));
  }, [loadProfiles]);

  useEffect(() => {
    if (selectedId === null) {
      setMemberIds(new Set());
      return;
    }
    invoke<number[]>("profile_mod_ids", { profileId: selectedId })
      .then((ids) => setMemberIds(new Set(ids)))
      .catch((e) => setError(String(e)));
  }, [selectedId]);

  async function createProfile() {
    if (!newName.trim()) return;
    try {
      await invoke<number>("profile_create", { name: newName.trim() });
      setNewName("");
      loadProfiles();
    } catch (e) {
      setError(String(e));
    }
  }

  async function deleteProfile(id: number) {
    try {
      await invoke("profile_delete", { profileId: id });
      if (selectedId === id) setSelectedId(null);
      loadProfiles();
    } catch (e) {
      setError(String(e));
    }
  }

  async function toggleMembership(modId: number) {
    if (selectedId === null) return;
    try {
      if (memberIds.has(modId)) {
        await invoke("profile_remove_mod", { profileId: selectedId, modId });
      } else {
        await invoke("profile_add_mod", { profileId: selectedId, modId });
      }
      const ids = await invoke<number[]>("profile_mod_ids", { profileId: selectedId });
      setMemberIds(new Set(ids));
    } catch (e) {
      setError(String(e));
    }
  }

  async function switchTo(id: number) {
    setError(null);
    setSwitchResult(null);
    try {
      const result = await invoke<SwitchOutcome>("profile_switch", { profileId: id });
      setSwitchResult(result);
      loadProfiles();
    } catch (e) {
      setError(String(e));
    }
  }

  return (
    <section className="view" data-shown="true">
      <div className="page-head">
        <div>
          <h1 className="page-title">{t("profiles.title")}</h1>
          <p className="page-sub">{t("profiles.subtitle")}</p>
        </div>
      </div>

      {error && <p className="error">{error}</p>}
      {switchResult && (
        <p className="page-sub">
          {t("profiles.switch_result", {
            enabled: switchResult.enabled.length,
            disabled: switchResult.disabled.length,
          })}
        </p>
      )}

      <div className="panel" style={{ padding: "18px 20px" }}>
        <div className="config-list">
          {profiles?.map((p) => (
            <div className="config-row" key={p.id} data-active={String(p.id === selectedId)}>
              <button
                type="button"
                style={{
                  all: "unset",
                  cursor: "pointer",
                  display: "flex",
                  alignItems: "center",
                  gap: 10,
                  flex: 1,
                  color: "inherit",
                  fontFamily: "inherit",
                }}
                onClick={() => setSelectedId(p.id)}
              >
                <span className="cfg-dot" />
                <span className="cfg-name">{p.name}</span>
                {p.is_active && (
                  <span className="badge-soon" style={{ marginLeft: 8 }}>
                    {t("profiles.active_badge")}
                  </span>
                )}
              </button>
              <div className="cfg-actions">
                {!p.is_active && (
                  <button className="icon-btn" type="button" onClick={() => switchTo(p.id)}>
                    {t("profiles.switch_button")}
                  </button>
                )}
                <button className="icon-btn" type="button" onClick={() => deleteProfile(p.id)}>
                  {t("profiles.delete_button")}
                </button>
              </div>
            </div>
          ))}
          {profiles && profiles.length === 0 && <p>{t("profiles.empty")}</p>}
        </div>
        <div className="new-config-row">
          <input
            type="text"
            placeholder={t("profiles.new_name_placeholder")}
            value={newName}
            onChange={(e) => setNewName(e.target.value)}
            onKeyDown={(e) => e.key === "Enter" && createProfile()}
          />
          <button className="btn-primary" type="button" onClick={createProfile}>
            {t("profiles.create_button")}
          </button>
        </div>
      </div>

      {selectedId !== null && (
        <div className="panel">
          <div className="panel-head">
            <h2>{t("profiles.membership_title")}</h2>
          </div>
          {mods === null && <p style={{ padding: "16px 20px" }}>{t("legacySp.loading")}</p>}
          {mods && mods.length === 0 && (
            <p style={{ padding: "16px 20px" }}>{t("profiles.no_mods_installed")}</p>
          )}
          {mods && mods.length > 0 && (
            <table>
              <thead>
                <tr>
                  <th>{t("legacySp.col_mod")}</th>
                  <th>{t("profiles.col_member")}</th>
                </tr>
              </thead>
              <tbody>
                {mods.map((m) => (
                  <tr key={m.id} className="mod-row">
                    <td>
                      <div className="mod-name">{m.name}</div>
                    </td>
                    <td>
                      <input
                        type="checkbox"
                        checked={memberIds.has(m.id)}
                        onChange={() => toggleMembership(m.id)}
                      />
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}
    </section>
  );
}
