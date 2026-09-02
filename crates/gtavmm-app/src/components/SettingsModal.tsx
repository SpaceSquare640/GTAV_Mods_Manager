import { useEffect, type ReactNode } from "react";
import { useTranslation } from "react-i18next";

interface SettingsModalProps {
  title: string;
  wide?: boolean;
  onClose: () => void;
  children: ReactNode;
  /** Optional footer; omit for panels that save as you go. */
  footer?: ReactNode;
}

/** The shell every Settings panel opens into. Escape and a scrim click close it. */
export function SettingsModal({ title, wide, onClose, children, footer }: SettingsModalProps) {
  const { t } = useTranslation();

  useEffect(() => {
    function onKey(e: KeyboardEvent) {
      if (e.key === "Escape") onClose();
    }
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  }, [onClose]);

  return (
    <div
      className="modal-backdrop"
      data-open="true"
      onClick={(e) => {
        if (e.target === e.currentTarget) onClose();
      }}
    >
      <div className={wide ? "modal modal-wide" : "modal"} role="dialog" aria-modal="true" aria-label={title}>
        <div className="modal-head">
          <h2>{title}</h2>
          <button className="drawer-close" type="button" onClick={onClose} aria-label={t("drawer.close")}>
            <svg className="icon" aria-hidden="true">
              <use href="#i-x" />
            </svg>
          </button>
        </div>
        <div className="modal-body">{children}</div>
        {footer && <div className="modal-foot">{footer}</div>}
      </div>
    </div>
  );
}
