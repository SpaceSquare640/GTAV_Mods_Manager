import { useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import { Icon } from "../components/IconSprite";
import { InstallWizard } from "../components/InstallWizard";
import { pickFolder } from "../lib/pickers";

export function FiveMClientPage() {
  const { t } = useTranslation();
  const [clientPath, setClientPath] = useState("");
  const [wizardOpen, setWizardOpen] = useState(false);

  return (
    <section className="view" data-shown="true">
      <div className="page-head">
        <div>
          <h1 className="page-title">{t("fivemClient.title")}</h1>
          <p className="page-sub">
            <Trans
              i18nKey="fivemClient.subtitle"
              components={{ mono: <span className="mono" />, em: <em /> }}
            />
          </p>
        </div>
      </div>

      <div className="path-picker">
        <span className="path-icon">
          <Icon name="folder" />
        </span>
        <div className="path-text">
          <span className="label">{t("fivemClient.folder_label")}</span>
          <input
            className="mono"
            style={{
              background: "transparent",
              border: "none",
              color: "var(--text)",
              width: "100%",
              outline: "none",
            }}
            value={clientPath}
            onChange={(e) => setClientPath(e.target.value)}
            placeholder={t("fivemClient.folder_placeholder")}
          />
        </div>
        <button
          className="btn-ghost"
          type="button"
          onClick={async () => {
            const picked = await pickFolder(t("fivemClient.picker_title"));
            if (picked) setClientPath(picked);
          }}
        >
          {t("fivemClient.browse")}
        </button>
        <button
          className="btn-primary"
          type="button"
          disabled={!clientPath.trim()}
          onClick={() => setWizardOpen(true)}
        >
          {t("legacySp.install_mod_button")}
        </button>
      </div>

      <InstallWizard
        open={wizardOpen}
        onClose={() => setWizardOpen(false)}
        onInstalled={() => {}}
        mode="fivem-client"
        gamePath={clientPath}
      />

      <div
        className="info-banner"
        style={{
          borderColor: "color-mix(in srgb, var(--accent-fivem) 40%, var(--border))",
          background: "color-mix(in srgb, var(--accent-fivem) 8%, var(--surface))",
        }}
      >
        <svg className="icon" style={{ fontSize: 15 }}>
          <use href="#i-info" />
        </svg>
        <span>
          <Trans
            i18nKey="fivemClient.unverified_assumption"
            components={{ mono: <span className="mono" />, strong: <strong /> }}
          />
        </span>
      </div>

      <div className="panel">
        <div className="empty-state">
          <span className="glyph">
            <Icon name="folder" />
          </span>
          <h3>{t("fivemClient.not_wired_title")}</h3>
          <p>
            <Trans
              i18nKey="fivemClient.not_wired_body"
              components={{ mono: <span className="mono" />, strong: <strong /> }}
            />
          </p>
        </div>
      </div>
    </section>
  );
}
