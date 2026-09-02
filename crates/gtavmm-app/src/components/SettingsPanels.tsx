import { useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { SettingsModal } from "./SettingsModal";
import { applyTheme, type Theme } from "../lib/theme";
import { pickFolder } from "../lib/pickers";

/** The whole `user_settings` row, as `load_user_settings` returns it. */
interface UserSettings {
  language: string;
  default_auto_backup: boolean;
  game_install_path_override: string | null;
  theme: string | null;
  terms_accepted_version: string | null;
  onboarding_completed: boolean;
  backup_root_override: string | null;
}

type DetectResult =
  | { status: "found"; install_path: string; edition: string }
  | { status: "not_found" };

/**
 * Theme.
 *
 * Applies the moment it is picked so the choice can be judged rather than
 * imagined, and persists the preference rather than the resolved palette — so
 * "System" keeps following the OS instead of freezing today's answer.
 */
export function ThemePanel({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation();
  const [choice, setChoice] = useState<Theme>("system");
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<{ theme: string }>("load_startup_state")
      .then((s) => setChoice((s.theme ?? "system") as Theme))
      .catch(() => {
        // No Tauri runtime — leave the control on its default rather than
        // pretending a stored value was read.
      });
  }, []);

  async function choose(next: Theme) {
    setChoice(next);
    applyTheme(next);
    setError(null);
    try {
      await invoke("set_theme", { theme: next });
    } catch (e) {
      setError(String(e));
    }
  }

  const options: { value: Theme; icon: string }[] = [
    { value: "system", icon: "#i-monitor" },
    { value: "dark", icon: "#i-moon" },
    { value: "light", icon: "#i-sun" },
  ];

  return (
    <SettingsModal title={t("settingsCards.theme_title")} wide onClose={onClose}>
      {error && <p className="error">{error}</p>}
      <div className="radio-row">
        {options.map((o) => (
          <button
            key={o.value}
            className="radio-card"
            type="button"
            aria-pressed={choice === o.value}
            onClick={() => choose(o.value)}
          >
            <svg className="icon icon-lg" aria-hidden="true">
              <use href={o.icon} />
            </svg>
            <span className="rc-title">{t(`settingsCards.theme_${o.value}`)}</span>
            <span className="rc-desc">{t(`settingsCards.theme_${o.value}_desc`)}</span>
          </button>
        ))}
      </div>
      <p className="page-sub">{t("settingsCards.theme_help")}</p>
    </SettingsModal>
  );
}

/**
 * Game paths.
 *
 * This exists so moving an install is something the application can be told
 * about. Without it the stored path would go stale silently and mods would keep
 * being written to the old location.
 */
export function GamePathsPanel({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation();
  const [settings, setSettings] = useState<UserSettings | null>(null);
  const [detected, setDetected] = useState<DetectResult | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [saved, setSaved] = useState(false);

  useEffect(() => {
    invoke<UserSettings>("load_user_settings").then(setSettings).catch((e) => setError(String(e)));
    invoke<DetectResult>("detect_game").then(setDetected).catch(() => setDetected(null));
  }, []);

  async function browse() {
    const picked = await pickFolder(t("onboarding.browse_title"));
    if (!picked) return;
    setError(null);
    try {
      const result = await invoke<DetectResult>("validate_game_path", { path: picked });
      if (result.status === "not_found") {
        setError(t("onboarding.not_a_game_folder"));
        return;
      }
      setBusy(true);
      const updated = await invoke<UserSettings>("update_user_settings", {
        gameInstallPathOverride: { action: "set", path: picked },
      });
      setSettings(updated);
      setSaved(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function clearOverride() {
    setBusy(true);
    setError(null);
    try {
      // An explicit "clear" action rather than a null: a null would be read as
      // "leave it alone" and this button would appear to do nothing.
      const updated = await invoke<UserSettings>("update_user_settings", {
        gameInstallPathOverride: { action: "clear" },
      });
      setSettings(updated);
      setSaved(true);
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  const override = settings?.game_install_path_override ?? null;
  const auto = detected?.status === "found" ? detected : null;

  return (
    <SettingsModal title={t("settingsCards.paths_title")} wide onClose={onClose}>
      {error && <p className="error">{error}</p>}

      <div className="info-banner warn" style={{ marginBottom: "var(--gap)" }}>
        <svg className="icon glyph" aria-hidden="true">
          <use href="#i-alert-triangle" />
        </svg>
        <span>{t("settingsCards.paths_move_warning")}</span>
      </div>

      <div className="field-group" style={{ marginBottom: "var(--gap)" }}>
        <label>{t("settingsCards.paths_detected")}</label>
        <div className="path-picker">
          <svg className="icon path-icon" aria-hidden="true">
            <use href="#i-folder" />
          </svg>
          <div className="path-text">
            {auto ? (
              <span className="value mono">{auto.install_path}</span>
            ) : (
              <span className="value mono empty">{t("settingsCards.paths_none_detected")}</span>
            )}
          </div>
        </div>
      </div>

      <div className="field-group" style={{ marginBottom: "var(--gap)" }}>
        <label>{t("settingsCards.paths_override")}</label>
        <div className="path-picker">
          <svg className="icon path-icon" aria-hidden="true">
            <use href="#i-folder" />
          </svg>
          <div className="path-text">
            {override ? (
              <span className="value mono">{override}</span>
            ) : (
              <span className="value mono empty">{t("settingsCards.paths_no_override")}</span>
            )}
          </div>
          <button className="btn-ghost" type="button" onClick={browse} disabled={busy}>
            {t("onboarding.change")}
          </button>
          {override && (
            <button className="icon-btn" type="button" onClick={clearOverride} disabled={busy}>
              {t("settingsCards.paths_clear")}
            </button>
          )}
        </div>
      </div>

      <p className="page-sub">{t("settingsCards.paths_override_help")}</p>
      {saved && <p style={{ color: "var(--success)" }}>{t("settings.saved")}</p>}
    </SettingsModal>
  );
}

/** Backup defaults and where full backups are written. */
export function BackupPanel({ onClose }: { onClose: () => void }) {
  const { t } = useTranslation();
  const [settings, setSettings] = useState<UserSettings | null>(null);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    invoke<UserSettings>("load_user_settings").then(setSettings).catch((e) => setError(String(e)));
  }, []);

  async function update(patch: Record<string, unknown>) {
    setBusy(true);
    setError(null);
    try {
      setSettings(await invoke<UserSettings>("update_user_settings", patch));
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(false);
    }
  }

  async function chooseLocation() {
    const picked = await pickFolder(t("settingsCards.backup_pick_title"));
    if (!picked) return;
    await update({ backupRootOverride: { action: "set", path: picked } });
  }

  return (
    <SettingsModal title={t("settingsCards.backup_title")} wide onClose={onClose}>
      {error && <p className="error">{error}</p>}

      <div
        className="override-row"
        style={{ borderColor: "var(--border)", background: "var(--surface-2)", marginBottom: "var(--gap)" }}
      >
        <input
          type="checkbox"
          id="autoBackup"
          checked={settings?.default_auto_backup ?? true}
          disabled={busy || !settings}
          onChange={(e) => update({ defaultAutoBackup: e.target.checked })}
        />
        <label htmlFor="autoBackup">{t("settingsCards.backup_auto_label")}</label>
      </div>

      <div className="field-group">
        <label>{t("settingsCards.backup_location")}</label>
        <div className="path-picker">
          <svg className="icon path-icon" aria-hidden="true">
            <use href="#i-folder" />
          </svg>
          <div className="path-text">
            {settings?.backup_root_override ? (
              <span className="value mono">{settings.backup_root_override}</span>
            ) : (
              <span className="value mono empty">{t("settingsCards.backup_default_location")}</span>
            )}
          </div>
          <button className="btn-ghost" type="button" onClick={chooseLocation} disabled={busy}>
            {t("onboarding.change")}
          </button>
          {settings?.backup_root_override && (
            <button
              className="icon-btn"
              type="button"
              onClick={() => update({ backupRootOverride: { action: "clear" } })}
              disabled={busy}
            >
              {t("settingsCards.paths_clear")}
            </button>
          )}
        </div>
      </div>

      <p className="page-sub">{t("settingsCards.backup_help")}</p>
    </SettingsModal>
  );
}
