import { useEffect, useRef, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { pickFile } from "../lib/pickers";
import type { InstalledMod, PageMode } from "../types";

/** Every page a mod can be moved to, in sidebar order. */
const PAGE_MODES: PageMode[] = [
  "legacy-sp",
  "legacy-lspdfr",
  "enhanced-sp",
  "enhanced-lspdfr",
  "fivem-client",
];

interface ModDetailDrawerProps {
  mod: InstalledMod | null;
  /** Passed through to the lifecycle commands; null lets the backend auto-detect. */
  gamePath: string | null;
  /** The page this drawer was opened from; reinstall installs back into it. */
  mode: PageMode;
  onClose: () => void;
  /** Called after any change so the owning page can reload its list. */
  onChanged: () => void;
}

type Busy = "disable" | "enable" | "uninstall" | "reinstall" | "save" | "page" | null;

/**
 * Details for one installed mod, and the only place its lifecycle can be driven.
 *
 * Until now the interface could install a mod and then had no way to turn it off
 * or remove it: the engine has done disable, enable, uninstall and reinstall
 * since early on, but none was exposed. The notes and source-link editing that
 * used to expand inline inside the table lives here too, so a row has one place
 * to open rather than two competing affordances.
 *
 * Uninstall asks first. It is recoverable — the engine snapshots into the
 * recycle bin and keeps it for fifteen days — but it deletes real files from a
 * real game folder, so it should not happen on a single stray click.
 */
export function ModDetailDrawer({
  mod,
  gamePath,
  mode,
  onClose,
  onChanged,
}: ModDetailDrawerProps) {
  const { t } = useTranslation();
  const [notes, setNotes] = useState("");
  const [link, setLink] = useState("");
  const [busy, setBusy] = useState<Busy>(null);
  const [error, setError] = useState<string | null>(null);
  const [confirmUninstall, setConfirmUninstall] = useState(false);
  const closeRef = useRef<HTMLButtonElement>(null);

  // Re-seed the fields whenever a different mod is opened, otherwise the
  // previous mod's notes would linger in the inputs.
  useEffect(() => {
    if (!mod) return;
    setNotes(mod.notes ?? "");
    setLink(mod.link ?? "");
    setError(null);
    setConfirmUninstall(false);
    closeRef.current?.focus();
  }, [mod]);

  useEffect(() => {
    if (!mod) return;
    function onKey(e: KeyboardEvent) {
      if (e.key !== "Escape") return;
      // Escape backs out of the confirmation first, so it cannot skip past a
      // pending destructive question and close the whole drawer in one press.
      if (confirmUninstall) setConfirmUninstall(false);
      else onClose();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [mod, confirmUninstall, onClose]);

  if (!mod) return null;

  async function run(kind: Exclude<Busy, null>, action: () => Promise<unknown>) {
    setBusy(kind);
    setError(null);
    try {
      await action();
      onChanged();
      if (kind === "uninstall") onClose();
    } catch (e) {
      setError(String(e));
    } finally {
      setBusy(null);
    }
  }

  const modId = mod.id;
  async function changePage(next: PageMode) {
    await run("page", () => invoke("set_mod_mode", { modId, mode: next }));
  }

  const isActive = mod.status === "Active";
  const isDisabled = mod.status === "Disabled";
  const isGone = mod.status === "Uninstalled";
  const anyBusy = busy !== null;

  return (
    <div className="drawer-backdrop" data-open="true" onClick={(e) => {
      if (e.target === e.currentTarget) onClose();
    }}>
      <aside className="drawer" role="dialog" aria-modal="true" aria-label={mod.name}>
        <div className="drawer-head">
          <div>
            <h2>{mod.name}</h2>
            <span className="mod-type">
              .{mod.source_type} · {mod.installed_at.slice(0, 10)}
            </span>
          </div>
          <button
            ref={closeRef}
            className="drawer-close"
            type="button"
            onClick={onClose}
            aria-label={t("drawer.close")}
          >
            <svg className="icon" aria-hidden="true">
              <use href="#i-x" />
            </svg>
          </button>
        </div>

        <div className="drawer-body">
          {error && <p className="error">{error}</p>}

          <div className="drawer-section">
            <div className="eyebrow">{t("drawer.status")}</div>
            <span className={isActive ? "pill active" : "pill pill--off"}>
              {t(
                isActive
                  ? "legacySp.status_active"
                  : isDisabled
                    ? "legacySp.status_disabled"
                    : "legacySp.status_uninstalled",
              )}
            </span>
          </div>

          <div className="drawer-section">
            <div className="eyebrow">{t("drawer.install_root")}</div>
            <span className="path mono">{mod.install_path}</span>
          </div>

          <div className="drawer-section">
            <div className="eyebrow">
              {t("drawer.page")}{" "}
              {/* Mods installed before the page was recorded had it guessed
                  from their install path. Saying so is the point: a guess the
                  interface presents as a fact is one nobody thinks to check. */}
              {mod.mode_inferred && <span className="pill pill--off">{t("drawer.page_guessed")}</span>}
            </div>
            <select
              value={mod.mode ?? "legacy-sp"}
              disabled={busy !== null}
              onChange={(e) => changePage(e.target.value as PageMode)}
            >
              {PAGE_MODES.map((m) => (
                <option key={m} value={m}>
                  {t(`drawer.page_${m}`)}
                </option>
              ))}
            </select>
            <p className="page-sub" style={{ margin: "6px 0 0" }}>
              {t("drawer.page_help")}
            </p>
          </div>

          <div className="drawer-section">
            <div className="eyebrow">{t("legacySp.col_link")}</div>
            <input
              type="text"
              value={link}
              onChange={(e) => setLink(e.target.value)}
              placeholder={t("legacySp.link_placeholder")}
              style={{ width: "100%" }}
            />
          </div>

          <div className="drawer-section">
            <div className="eyebrow">{t("drawer.notes")}</div>
            <textarea
              rows={3}
              value={notes}
              onChange={(e) => setNotes(e.target.value)}
              placeholder={t("legacySp.notes_placeholder")}
              style={{ width: "100%" }}
            />
            <button
              className="btn-ghost"
              type="button"
              style={{ marginTop: "var(--space-2)" }}
              disabled={anyBusy}
              onClick={() =>
                run("save", () =>
                  invoke("update_mod_details", {
                    modId: mod.id,
                    notes: notes.trim() || null,
                    link: link.trim() || null,
                  }),
                )
              }
            >
              {busy === "save" ? t("drawer.saving") : t("legacySp.save_link_button")}
            </button>
          </div>

          {!isGone && (
            <div className="drawer-section">
              <div className="eyebrow">{t("drawer.reinstall")}</div>
              <p className="page-sub" style={{ marginTop: "var(--space-2)" }}>
                {t("drawer.reinstall_help")}
              </p>
              <button
                className="btn-ghost"
                type="button"
                disabled={anyBusy}
                onClick={async () => {
                  // The formats the analyzer accepts. .rar is deliberately absent
                  // — it is not supported, and offering it here would only produce
                  // a refusal after the user had already chosen a file.
                  const picked = await pickFile(
                    ["zip", "7z", "asi", "dll", "xml"],
                    t("drawer.choose_new_source"),
                  );
                  if (!picked) return;
                  await run("reinstall", () =>
                    invoke("reinstall_mod", {
                      modId: mod.id,
                      newSourcePath: picked,
                      versionLabel: new Date().toISOString().slice(0, 10),
                      mode,
                      gamePath,
                    }),
                  );
                }}
              >
                {busy === "reinstall" ? t("drawer.working") : t("drawer.choose_new_source")}
              </button>
            </div>
          )}
        </div>

        {!isGone && (
          <div className="drawer-actions">
            {isActive && (
              <button
                className="btn-ghost"
                type="button"
                disabled={anyBusy}
                onClick={() => run("disable", () => invoke("disable_mod", { modId: mod.id }))}
              >
                {busy === "disable" ? t("drawer.working") : t("drawer.disable")}
              </button>
            )}
            {isDisabled && (
              <button
                className="btn-ghost"
                type="button"
                disabled={anyBusy}
                onClick={() => run("enable", () => invoke("enable_mod", { modId: mod.id }))}
              >
                {busy === "enable" ? t("drawer.working") : t("drawer.enable")}
              </button>
            )}
            <button
              className="btn-ghost btn-danger"
              type="button"
              disabled={anyBusy}
              onClick={() => setConfirmUninstall(true)}
            >
              <svg className="icon" aria-hidden="true">
                <use href="#i-trash" />
              </svg>
              {t("drawer.uninstall")}
            </button>
          </div>
        )}

        {confirmUninstall && (
          <div className="modal-backdrop" data-open="true">
            <div className="modal" role="dialog" aria-modal="true" aria-labelledby="uninstallTitle">
              <div className="modal-head">
                <h2 id="uninstallTitle">{t("drawer.uninstall_confirm_title", { name: mod.name })}</h2>
              </div>
              <div className="modal-body">
                <p style={{ marginTop: 0 }}>{t("drawer.uninstall_confirm_body")}</p>
                <div className="info-banner" style={{ marginTop: "var(--gap)" }}>
                  <svg className="icon glyph" aria-hidden="true">
                    <use href="#i-info" />
                  </svg>
                  <span>{t("drawer.uninstall_recoverable")}</span>
                </div>
              </div>
              <div className="modal-foot">
                <button className="btn-ghost" type="button" onClick={() => setConfirmUninstall(false)}>
                  {t("legacySp.cancel_link_button")}
                </button>
                <button
                  className="btn-ghost btn-danger"
                  type="button"
                  disabled={anyBusy}
                  onClick={() => {
                    setConfirmUninstall(false);
                    run("uninstall", () => invoke("uninstall_mod", { modId: mod.id, gamePath }));
                  }}
                >
                  {t("drawer.uninstall")}
                </button>
              </div>
            </div>
          </div>
        )}
      </aside>
    </div>
  );
}
