import { useCallback, useEffect, useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { pickFolder } from "../lib/pickers";

interface OnboardingGateProps {
  onDone: () => void;
}

type Detect =
  | { status: "found"; install_path: string; edition: string }
  | { status: "not_found" };

/**
 * First-run game path setup.
 *
 * Detection is best-effort: it can find a Steam, Epic or Rockstar install, but
 * a moved or unusual one has to be pointed at by hand, so both routes are
 * offered rather than only the automatic one. A chosen folder is validated
 * before it is accepted, so a wrong path is rejected here rather than surfacing
 * as a confusing failure at the first install.
 *
 * This is a separate gate from the terms rather than one flow, because someone
 * can accept the terms and quit before choosing paths, and should come back to
 * setup rather than to an empty workspace.
 */
export function OnboardingGate({ onDone }: OnboardingGateProps) {
  const { t } = useTranslation();
  const [detected, setDetected] = useState<Detect | null>(null);
  const [manual, setManual] = useState<string | null>(null);
  const [detecting, setDetecting] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);

  const detect = useCallback(() => {
    setDetecting(true);
    setError(null);
    invoke<Detect>("detect_game")
      .then(setDetected)
      .catch((e) => setError(String(e)))
      .finally(() => setDetecting(false));
  }, []);

  useEffect(detect, [detect]);

  async function browse() {
    const picked = await pickFolder(t("onboarding.browse_title"));
    if (!picked) return;
    setError(null);
    try {
      // Validate before accepting, so a wrong folder is caught at the point of
      // choosing rather than at the first install.
      const result = await invoke<Detect>("validate_game_path", { path: picked });
      if (result.status === "not_found") {
        setError(t("onboarding.not_a_game_folder"));
        return;
      }
      setManual(picked);
      setDetected(result);
    } catch (e) {
      setError(String(e));
    }
  }

  async function finish() {
    setBusy(true);
    setError(null);
    try {
      await invoke("complete_onboarding", { gameInstallPathOverride: manual });
      onDone();
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  }

  const found = detected?.status === "found" ? detected : null;

  return (
    <div className="fullscreen" data-shown="true">
      <div className="fullscreen-inner" style={{ maxWidth: 660 }}>
        <h1 className="page-title" style={{ justifyContent: "center", textAlign: "center" }}>
          {t("onboarding.title")}
        </h1>
        <p
          className="page-sub"
          style={{ textAlign: "center", margin: "var(--space-2) auto var(--space-6)" }}
        >
          {t("onboarding.subtitle")}
        </p>

        {error && <p className="error">{error}</p>}

        <div className="detect-card" data-state={found ? "found" : "manual"}>
          <span className="glyph">
            <svg className="icon icon-lg" aria-hidden="true">
              <use href={found ? "#i-check-circle" : "#i-folder"} />
            </svg>
          </span>
          <div className="body">
            <h3>{found ? t(`onboarding.edition_${found.edition.toLowerCase()}`, { defaultValue: found.edition }) : t("onboarding.not_detected")}</h3>
            {found ? (
              <>
                <p className="mono">{found.install_path}</p>
                <span className="src">
                  {manual ? t("onboarding.source_manual") : t("onboarding.source_detected")}
                </span>
              </>
            ) : (
              <p className="page-sub" style={{ margin: 0 }}>
                {t("onboarding.not_detected_help")}
              </p>
            )}
          </div>
          <button className="btn-ghost" type="button" onClick={browse}>
            {found ? t("onboarding.change") : t("onboarding.browse")}
          </button>
        </div>

        <div className="info-banner" style={{ marginTop: "var(--gap)" }}>
          <svg className="icon glyph" aria-hidden="true">
            <use href="#i-info" />
          </svg>
          <span>{t("onboarding.changeable_later")}</span>
        </div>

        <div
          style={{
            display: "flex",
            gap: "var(--space-3)",
            justifyContent: "center",
            marginTop: "var(--space-6)",
          }}
        >
          <button className="btn-ghost" type="button" onClick={detect} disabled={detecting}>
            <svg className="icon" aria-hidden="true">
              <use href="#i-refresh" />
            </svg>
            {detecting ? t("onboarding.detecting") : t("onboarding.detect_again")}
          </button>
          <button className="btn-primary" type="button" onClick={finish} disabled={busy}>
            {busy ? t("terms.saving") : t("onboarding.continue")}
          </button>
        </div>
      </div>
    </div>
  );
}
