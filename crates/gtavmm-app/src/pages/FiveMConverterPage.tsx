import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "../components/IconSprite";
import { pickFolder, pickFile } from "../lib/pickers";

interface ConversionReport {
  data_files: string[];
  stream_files: string[];
  skipped_files: string[];
}

export function FiveMConverterPage() {
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
          <h1 className="page-title">FiveM · Converter</h1>
          <p className="page-sub">
            Converts an SP add-on vehicle pack straight into a FiveM resource — reads the{" "}
            <span className="mono">dlc.rpf</span> directly (including any nested{" "}
            <span className="mono">vehicles.rpf</span>), no OpenIV/CodeWalker extraction step
            needed. Real IPC to <span className="mono">gtavmm-core</span>, not a demo snapshot.{" "}
            <strong>Vehicle/Add-on packs only</strong> — script mod conversion isn't built.
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
          <strong>Unverified assumption:</strong> verified end-to-end against four real SP
          add-on vehicle mods so far (see project notes) — including packs with{" "}
          <span className="mono">carcols.meta</span>, nested per-language localization
          archives, non-ASCII paths, and 16 vehicles declared in a single DLC. Still untested:
          a <span className="mono">vehicles.rpf</span> nesting a third level of archive.
        </span>
      </div>

      <div className="field-group">
        <label>SP add-on pack's dlc.rpf</label>
        <div className="path-picker" style={{ margin: 0 }}>
          <span className="path-icon">
            <Icon name="folder" />
          </span>
          <div className="path-text">
            <span className="label">Source</span>
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
              placeholder="e.g. H:/Mods/MyCarPack/dlc.rpf"
            />
          </div>
          <button
            className="btn-ghost"
            type="button"
            onClick={async () => {
              const picked = await pickFile(["rpf"], "Select dlc.rpf");
              if (picked) setDlcRpf(picked);
            }}
          >
            Browse…
          </button>
        </div>
      </div>
      <div className="field-group">
        <label>Output folder</label>
        <div className="path-picker" style={{ margin: 0 }}>
          <span className="path-icon">
            <Icon name="folder" />
          </span>
          <div className="path-text">
            <span className="label">Destination</span>
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
              placeholder="e.g. C:/FiveMServer/resources/mycarpack"
            />
          </div>
          <button
            className="btn-ghost"
            type="button"
            onClick={async () => {
              const picked = await pickFolder("Select output folder");
              if (picked) setOutputDir(picked);
            }}
          >
            Browse…
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
        {converting ? "Converting…" : "Convert"}
      </button>

      {error && <p className="error">Error: {error}</p>}

      {report && (
        <div style={{ marginTop: 18 }}>
          <div className="panel" style={{ padding: "18px 20px" }}>
            <div className="eyebrow" style={{ marginBottom: 6 }}>
              Written to
            </div>
            <p className="mono" style={{ fontSize: 11.5, margin: "0 0 14px", color: "var(--text-muted)" }}>
              {outputDir}
            </p>
            <div className="eyebrow" style={{ marginBottom: 6 }}>
              data/ ({report.data_files.length} files)
            </div>
            <p className="mono" style={{ fontSize: 11.5, margin: "0 0 14px", color: "var(--text-muted)" }}>
              {report.data_files.join(", ") || "(none)"}
            </p>
            <div className="eyebrow" style={{ marginBottom: 6 }}>
              stream/ ({report.stream_files.length} files)
            </div>
            <p className="mono" style={{ fontSize: 11.5, margin: "0 0 14px", color: "var(--text-muted)" }}>
              {report.stream_files.join(", ") || "(none)"}
            </p>
            <div className="eyebrow" style={{ marginBottom: 6 }}>
              fxmanifest.lua
            </div>
            <p style={{ fontSize: 11.5, margin: "0 0 14px", display: "flex", alignItems: "center", gap: 6 }}>
              <svg className="icon" style={{ color: "var(--success)" }}>
                <use href="#i-check-circle" />
              </svg>{" "}
              Generated (glob-based, matches the community-standard Add-on Car template)
            </p>
            <div className="eyebrow" style={{ marginBottom: 6 }}>
              Skipped ({report.skipped_files.length})
            </div>
            <p className="mono" style={{ fontSize: 11.5, margin: 0, color: "var(--text-faint)" }}>
              {report.skipped_files.join(", ") || "(none)"}
            </p>
          </div>
        </div>
      )}
    </section>
  );
}
