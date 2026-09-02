import { useState } from "react";
import { useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";

interface TermsGateProps {
  onAccepted: () => void;
}

/**
 * Shown before anything else until the current terms are accepted.
 *
 * Acceptance is stored as a version rather than a flag, so revising the text
 * can deliberately ask again instead of either passing on stale consent or
 * forcing everyone to re-accept with no way to tell the difference.
 *
 * The layout is the design's split pane: the section list on the left stays put
 * while the text fills the available height and scrolls on its own, rather than
 * being trapped in a fixed-height box.
 */
export function TermsGate({ onAccepted }: TermsGateProps) {
  const { t } = useTranslation();
  const [tab, setTab] = useState<"terms" | "privacy">("terms");
  const [agreed, setAgreed] = useState(false);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);

  // Terms and privacy are long prose rather than interface labels, so they live
  // as numbered clause keys instead of being threaded through <Trans>.
  const clauses = tab === "terms" ? [1, 2, 3, 4, 5, 6] : [1, 2, 3, 4, 5];

  async function accept() {
    setBusy(true);
    setError(null);
    try {
      await invoke("accept_terms");
      onAccepted();
    } catch (e) {
      setError(String(e));
      setBusy(false);
    }
  }

  return (
    <div className="fullscreen" data-shown="true">
      <div className="fullscreen-inner">
        <div className="page-head">
          <div>
            <h1 className="page-title">{t("terms.title")}</h1>
            <p className="page-sub">{t("terms.subtitle")}</p>
          </div>
        </div>

        {error && <p className="error">{error}</p>}

        <div className="split-shell" style={{ height: "min(430px, 56vh)" }}>
          <div className="split-side">
            <div className="page-tabs" style={{ border: "none", marginBottom: "var(--space-4)" }} role="tablist">
              <button
                className="page-tab"
                type="button"
                role="tab"
                data-active={String(tab === "terms")}
                aria-selected={tab === "terms"}
                onClick={() => setTab("terms")}
              >
                {t("terms.tab_terms")}
              </button>
              <button
                className="page-tab"
                type="button"
                role="tab"
                data-active={String(tab === "privacy")}
                aria-selected={tab === "privacy"}
                onClick={() => setTab("privacy")}
              >
                {t("terms.tab_privacy")}
              </button>
            </div>

            <div className="eyebrow" style={{ marginBottom: "var(--space-3)" }}>
              {t("terms.applies_to")}
            </div>
            <div style={{ display: "flex", flexDirection: "column", gap: "var(--space-2)" }}>
              {(["legacy", "enhanced", "fivem"] as const).map((m) => (
                <div className="mini-mode" data-mode={m} key={m}>
                  <span className="mode-dot" />
                  {t(`nav.${m}`)}
                </div>
              ))}
            </div>
          </div>

          <div className="split-body">
            <div className="split-scroll" data-shown="true">
              {clauses.map((n) => (
                <div key={`${tab}-${n}`}>
                  <h4>{t(`terms.${tab}_${n}_head`)}</h4>
                  <p>{t(`terms.${tab}_${n}_body`)}</p>
                </div>
              ))}
            </div>

            <div className="split-foot">
              <button
                className="agree-check"
                type="button"
                role="checkbox"
                aria-checked={agreed}
                aria-labelledby="agreeLabel"
                onClick={() => setAgreed((v) => !v)}
              >
                <svg className="icon" aria-hidden="true">
                  <use href="#i-check" />
                </svg>
              </button>
              <label className="agree-label" id="agreeLabel" onClick={() => setAgreed((v) => !v)}>
                {t("terms.agree_label")}
              </label>
              <button
                className="btn-primary"
                type="button"
                style={{ marginLeft: "auto" }}
                disabled={!agreed || busy}
                onClick={accept}
              >
                {busy ? t("terms.saving") : t("terms.continue")}
              </button>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
