import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { useTranslation } from "react-i18next";
import { pickFile } from "../lib/pickers";
import { formatLabel, type InstallOutcome, type ModPlan } from "../types";

interface InstallWizardProps {
  open: boolean;
  onClose: () => void;
  /** Called after a successful install so the caller can refresh its mod list. */
  onInstalled: () => void;
  /** `sp` (default), `lspdfr`, or `fivem-client` — matches `gtavmm_core::providers::Mode`. */
  mode?: "sp" | "lspdfr" | "fivem-client";
  /** Required for `fivem-client` (no auto-detection); ignored otherwise. */
  gamePath?: string | null;
}

type Step = "pick" | "analyzing" | "review" | "installing" | "done" | "error";

/**
 * A real install wizard against the real `inspect_mod`/`install_mod` Tauri commands —
 * not a mockup. Deliberately only has the steps the backend actually supports:
 * `inspect_mod` (gtavmm_core::mod_analyzer::classify, format/target preview only — it
 * does NOT check conflicts, that only happens inside `install_mod`) and `install_mod`
 * (the real pipeline: conflict check, backup, write, record). There's no separate
 * "Scan" or "Backup" step here because the backend doesn't expose those as distinct,
 * confirmable phases — showing fake intermediate steps would misrepresent what's
 * actually happening.
 */
export function InstallWizard({
  open,
  onClose,
  onInstalled,
  mode = "sp",
  gamePath = null,
}: InstallWizardProps) {
  const { t } = useTranslation();
  const [step, setStep] = useState<Step>("pick");
  const [path, setPath] = useState<string | null>(null);
  const [plan, setPlan] = useState<ModPlan | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [outcome, setOutcome] = useState<InstallOutcome | null>(null);

  function reset() {
    setStep("pick");
    setPath(null);
    setPlan(null);
    setError(null);
    setOutcome(null);
  }

  function handleClose() {
    reset();
    onClose();
  }

  async function pickAndAnalyze() {
    try {
      const picked = await pickFile(["asi", "dll", "xml", "zip", "7z", "oiv"], t("installWizard.pick_title"));
      if (!picked) return;
      setPath(picked);
      setStep("analyzing");
      setError(null);
      const result = await invoke<ModPlan>("inspect_mod", { gamePath, mode, path: picked });
      setPlan(result);
      setStep("review");
    } catch (e) {
      setError(String(e));
      setStep("error");
    }
  }

  async function runInstall(overrideForeignConflicts: boolean) {
    if (!path) return;
    setStep("installing");
    setError(null);
    try {
      const result = await invoke<InstallOutcome>("install_mod", {
        gamePath,
        mode,
        path,
        name: null,
        overrideForeignConflicts,
      });
      setOutcome(result);
      setStep("done");
      if ("Success" in result) onInstalled();
    } catch (e) {
      setError(String(e));
      setStep("error");
    }
  }

  if (!open) return null;

  return (
    <div className="modal-backdrop" data-open="true">
      <div className="modal">
        <div className="modal-head">
          <h2>{t("installWizard.title")}</h2>
          <button className="drawer-close" type="button" aria-label="Close" onClick={handleClose}>
            ×
          </button>
        </div>

        <div className="modal-body">
          {step === "pick" && (
            <>
              <p>{t(`installWizard.pick_body_${mode}`)}</p>
              <button className="btn-primary" type="button" onClick={pickAndAnalyze}>
                {t("installWizard.pick_button")}
              </button>
            </>
          )}

          {step === "analyzing" && <p>{t("installWizard.analyzing")}</p>}

          {step === "review" && plan && (
            <>
              <div className="analyze-row">
                <span>{t("installWizard.row_format")}</span>
                <span className="mono">{formatLabel(plan.format)}</span>
              </div>
              <div className="analyze-row">
                <span>{t("installWizard.row_files")}</span>
                <span className="mono">{plan.files.length}</span>
              </div>
              {plan.files.slice(0, 5).map((f, i) => (
                <div className="analyze-row" key={i}>
                  <span className="mono" style={{ fontSize: 11 }}>
                    {f.source.split(/[\\/]/).pop()}
                  </span>
                  <span className="mono" style={{ fontSize: 11 }}>
                    {f.target}
                  </span>
                </div>
              ))}
              {plan.files.length > 5 && (
                <p style={{ marginTop: 8 }}>
                  {t("installWizard.more_files", { count: plan.files.length - 5 })}
                </p>
              )}
              <p style={{ marginTop: 12 }}>{t("installWizard.review_note")}</p>
            </>
          )}

          {step === "installing" && <p>{t("installWizard.installing")}</p>}

          {step === "done" && outcome && "Success" in outcome && (
            <>
              <p>
                {t("installWizard.success", { count: outcome.Success.files_written })}
              </p>
            </>
          )}

          {step === "done" && outcome && "ProtectedFileBlocked" in outcome && (
            <div className="conflict-box">
              <div className="head">{t("installWizard.protected_head")}</div>
              <p>{t("installWizard.protected_body")}</p>
              {outcome.ProtectedFileBlocked.map((p, i) => (
                <p key={i} className="mono" style={{ fontSize: 11 }}>
                  {p}
                </p>
              ))}
            </div>
          )}

          {step === "done" && outcome && "RequiresOverride" in outcome && (
            <div className="conflict-box">
              <div className="head">{t("installWizard.override_head")}</div>
              <p>{t("installWizard.override_body")}</p>
              {outcome.RequiresOverride.foreign_conflicts.map((c, i) => (
                <p key={i} className="mono" style={{ fontSize: 11 }}>
                  {c.path} — {t("installWizard.owned_by", { name: c.owner_name })}
                </p>
              ))}
            </div>
          )}

          {step === "error" && <p className="error">{error}</p>}
        </div>

        <div className="modal-foot">
          {step === "review" && (
            <>
              <button className="btn-ghost" type="button" onClick={handleClose}>
                {t("installWizard.cancel")}
              </button>
              <button className="btn-primary" type="button" onClick={() => runInstall(false)}>
                {t("installWizard.install")}
              </button>
            </>
          )}
          {step === "done" && outcome && "RequiresOverride" in outcome && (
            <>
              <button className="btn-ghost" type="button" onClick={handleClose}>
                {t("installWizard.cancel")}
              </button>
              <button className="btn-primary" type="button" onClick={() => runInstall(true)}>
                {t("installWizard.override_and_install")}
              </button>
            </>
          )}
          {(step === "done" || step === "error") &&
            !(outcome && "RequiresOverride" in outcome) && (
              <button className="btn-primary" type="button" onClick={handleClose}>
                {t("installWizard.close")}
              </button>
            )}
          {(step === "pick" || step === "analyzing" || step === "installing") && (
            <button className="btn-ghost" type="button" onClick={handleClose}>
              {t("installWizard.cancel")}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
