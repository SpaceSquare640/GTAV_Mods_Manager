import { useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "../components/IconSprite";
import { pickFolder, pickFile } from "../lib/pickers";

interface ConversionReport {
  data_files: string[];
  stream_files: string[];
  skipped_files: string[];
}

export function FiveMConverterPage() {
  const { t } = useTranslation();
  const [dlcRpf, setDlcRpf] = useState("");
  const [outputDir, setOutputDir] = useState("");
  const [report, setReport] = useState<ConversionReport | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [converting, setConverting] = useState(false);

  async function convert() {
    setError(null);
    setConverting(true);
    try {
      const result = await invoke<ConversionReport>("convert_vehicle_pack", {
        dlcRpf,
        outputDir,
      });
      setReport(result);
    } catch (e) {
      setError(String(e));
      setReport(null);
    } finally {
      setConverting(false);
    }
  }

  return (
    <section className="view" data-shown="true">
      <div className="page-head">
        <div>
          <h1 className="page-title">{t("fivemConverter.title")}</h1>
          <p className="page-sub">
            <Trans
              i18nKey="fivemConverter.subtitle"
              components={{ mono: <span className="mono" />, strong: <strong /> }}
            />
          </p>
        </div>
      </div>

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
            i18nKey="fivemConverter.unverified_assumption"
            components={{ mono: <span className="mono" />, strong: <strong /> }}
          />
        </span>
      </div>

      <div className="field-group">
        <label>{t("fivemConverter.source_label")}</label>
        <div className="path-picker" style={{ margin: 0 }}>
          <span className="path-icon">
            <Icon name="folder" />
          </span>
          <div className="path-text">
            <span className="label">{t("fivemConverter.source_sublabel")}</span>
            <input
              className="mono"
              style={{
                background: "transparent",
                border: "none",
                color: "var(--text)",
                width: "100%",
                outline: "none",
              }}
              value={dlcRpf}
              onChange={(e) => setDlcRpf(e.target.value)}
              placeholder={t("fivemConverter.source_placeholder")}
            />
          </div>
          <button
            className="btn-ghost"
            type="button"
            onClick={async () => {
              const picked = await pickFile(["rpf"], t("fivemConverter.source_picker_title"));
              if (picked) setDlcRpf(picked);
            }}
          >
            {t("fivemClient.browse")}
          </button>
        </div>
      </div>
      <div className="field-group">
        <label>{t("fivemConverter.output_label")}</label>
        <div className="path-picker" style={{ margin: 0 }}>
          <span className="path-icon">
            <Icon name="folder" />
          </span>
          <div className="path-text">
            <span className="label">{t("fivemConverter.output_sublabel")}</span>
            <input
              className="mono"
              style={{
                background: "transparent",
                border: "none",
                color: "var(--text)",
                width: "100%",
                outline: "none",
              }}
              value={outputDir}
              onChange={(e) => setOutputDir(e.target.value)}
              placeholder={t("fivemConverter.output_placeholder")}
            />
          </div>
          <button
            className="btn-ghost"
            type="button"
            onClick={async () => {
              const picked = await pickFolder(t("fivemConverter.output_picker_title"));
              if (picked) setOutputDir(picked);
            }}
          >
            {t("fivemClient.browse")}
          </button>
        </div>
      </div>
      <button
        className="btn-primary"
        type="button"
        style={{ marginTop: 6 }}
        disabled={!dlcRpf.trim() || !outputDir.trim() || converting}
        onClick={convert}
      >
        {converting ? t("fivemConverter.converting") : t("fivemConverter.convert_button")}
      </button>

      {error && <p className="error">{t("fivemServer.error_prefix", { error })}</p>}

      {report && (
        <div style={{ marginTop: 18 }}>
          <div className="panel" style={{ padding: "18px 20px" }}>
            <div className="eyebrow" style={{ marginBottom: 6 }}>
              {t("fivemConverter.written_to")}
            </div>
            <p className="mono" style={{ fontSize: 11.5, margin: "0 0 14px", color: "var(--text-muted)" }}>
              {outputDir}
            </p>
            <div className="eyebrow" style={{ marginBottom: 6 }}>
              {t("fivemConverter.data_files", { count: report.data_files.length })}
            </div>
            <p className="mono" style={{ fontSize: 11.5, margin: "0 0 14px", color: "var(--text-muted)" }}>
              {report.data_files.join(", ") || t("fivemConverter.none")}
            </p>
            <div className="eyebrow" style={{ marginBottom: 6 }}>
              {t("fivemConverter.stream_files", { count: report.stream_files.length })}
            </div>
            <p className="mono" style={{ fontSize: 11.5, margin: "0 0 14px", color: "var(--text-muted)" }}>
              {report.stream_files.join(", ") || t("fivemConverter.none")}
            </p>
            <div className="eyebrow" style={{ marginBottom: 6 }}>
              fxmanifest.lua
            </div>
            <p style={{ fontSize: 11.5, margin: "0 0 14px", display: "flex", alignItems: "center", gap: 6 }}>
              <svg className="icon" style={{ color: "var(--success)" }}>
                <use href="#i-check-circle" />
              </svg>{" "}
              {t("fivemConverter.manifest_generated")}
            </p>
            <div className="eyebrow" style={{ marginBottom: 6 }}>
              {t("fivemConverter.skipped_files", { count: report.skipped_files.length })}
            </div>
            <p className="mono" style={{ fontSize: 11.5, margin: 0, color: "var(--text-faint)" }}>
              {report.skipped_files.join(", ") || t("fivemConverter.none")}
            </p>
          </div>
        </div>
      )}
    </section>
  );
}
