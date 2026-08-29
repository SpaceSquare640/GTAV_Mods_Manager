import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "../components/IconSprite";
import { pickFolder, pickFile } from "../lib/pickers";

export function FiveMServerPage() {
  const [resourcesRoot, setResourcesRoot] = useState("");
  const [serverCfgPath, setServerCfgPath] = useState("");
  const [order, setOrder] = useState<string[] | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [resolving, setResolving] = useState(false);
  const [applying, setApplying] = useState(false);
  const [applyResult, setApplyResult] = useState<string | null>(null);

  async function resolve() {
    setError(null);
    setApplyResult(null);
    setResolving(true);
    try {
      const result = await invoke<string[]>("fivem_resolve_load_order", {
        resourcesRoot,
      });
      setOrder(result);
    } catch (e) {
      setError(String(e));
      setOrder(null);
    } finally {
      setResolving(false);
    }
  }

  async function apply() {
    setError(null);
    setApplying(true);
    try {
      const result = await invoke<string[]>("fivem_apply_load_order", {
        resourcesRoot,
        serverCfgPath,
      });
      setOrder(result);
      setApplyResult(
        `Applied — wrote ${result.length} ensure line(s) to ${serverCfgPath}.`
      );
    } catch (e) {
      setError(String(e));
    } finally {
      setApplying(false);
    }
  }

  return (
    <section className="view" data-shown="true">
      <div className="page-head">
        <div>
          <h1 className="page-title">FiveM · Server</h1>
          <p className="page-sub">
            Resolves a correct load order for your <span className="mono">resources\</span>{" "}
            folder from each resource's declared <span className="mono">fxmanifest.lua</span>{" "}
            dependencies — real IPC to <span className="mono">gtavmm-core</span>, not a demo
            snapshot. Browse or type/paste a path below.
          </p>
        </div>
      </div>

      <div className="path-picker">
        <span className="path-icon">
          <Icon name="folder" />
        </span>
        <div className="path-text">
          <span className="label">resources\ folder</span>
          <input
            className="mono"
            style={{
              background: "transparent",
              border: "none",
              color: "var(--text)",
              width: "100%",
              outline: "none",
            }}
            value={resourcesRoot}
            onChange={(e) => setResourcesRoot(e.target.value)}
            placeholder="e.g. C:/FiveMServer/resources"
          />
        </div>
        <button
          className="btn-ghost"
          type="button"
          onClick={async () => {
            const picked = await pickFolder("Select resources\\ folder");
            if (picked) setResourcesRoot(picked);
          }}
        >
          Browse…
        </button>
        <button
          className="btn-primary"
          type="button"
          disabled={!resourcesRoot.trim() || resolving}
          onClick={resolve}
        >
          {resolving ? "Resolving…" : "Resolve order"}
        </button>
      </div>

      {error && <p className="error">Error: {error}</p>}

      {order === null && !error && (
        <div className="panel">
          <div className="empty-state">
            <span className="glyph">
              <Icon name="bar-chart" />
            </span>
            <h3>No resources folder set</h3>
            <p>
              Point this at your server's <span className="mono">resources\</span> folder,
              then click Resolve order.
            </p>
          </div>
        </div>
      )}

      {order !== null && (
        <>
          <div className="stat-row">
            <div className="stat-card">
              <div className="eyebrow">Resources found</div>
              <div className="value">{order.length}</div>
            </div>
          </div>

          <div className="panel" style={{ padding: "18px 20px", marginBottom: 16 }}>
            <h2
              style={{
                fontFamily: "'Rajdhani',sans-serif",
                fontSize: 15,
                margin: "0 0 12px",
              }}
            >
              Suggested load order
            </h2>
            <div className="order-list">
              {order.map((name, i) => (
                <div className="order-row" key={name}>
                  <span className="rank">{i + 1}</span>
                  <span className="name">{name}</span>
                </div>
              ))}
            </div>
          </div>

          <div className="panel" style={{ padding: "18px 20px" }}>
            <div
              style={{
                display: "flex",
                alignItems: "center",
                justifyContent: "space-between",
                marginBottom: 10,
              }}
            >
              <h2 style={{ fontFamily: "'Rajdhani',sans-serif", fontSize: 15, margin: 0 }}>
                Apply to server.cfg
              </h2>
            </div>
            <p className="diagnosis-disclaimer" style={{ margin: "-4px 0 10px" }}>
              Writes only a clearly-marked, idempotent block into{" "}
              <span className="mono">server.cfg</span> — every other setting (hostname,
              maxclients, unrelated ensures) is left untouched; re-running updates the block
              in place instead of duplicating it.
            </p>
            <div className="path-picker" style={{ margin: "0 0 10px" }}>
              <span className="path-icon">
                <Icon name="file-text" />
              </span>
              <div className="path-text">
                <span className="label">server.cfg path</span>
                <input
                  className="mono"
                  style={{
                    background: "transparent",
                    border: "none",
                    color: "var(--text)",
                    width: "100%",
                    outline: "none",
                  }}
                  value={serverCfgPath}
                  onChange={(e) => setServerCfgPath(e.target.value)}
                  placeholder="e.g. C:/FiveMServer/server.cfg"
                />
              </div>
              <button
                className="btn-ghost"
                type="button"
                onClick={async () => {
                  const picked = await pickFile(["cfg"], "Select server.cfg");
                  if (picked) setServerCfgPath(picked);
                }}
              >
                Browse…
              </button>
              <button
                className="btn-primary"
                type="button"
                disabled={!serverCfgPath.trim() || applying}
                onClick={apply}
              >
                {applying ? "Applying…" : "Apply directly"}
              </button>
            </div>
            {applyResult && (
              <p>
                <Icon name="check-circle" /> {applyResult}
              </p>
            )}
          </div>
        </>
      )}
    </section>
  );
}
