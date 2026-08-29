import { useState } from "react";
import { Trans, useTranslation } from "react-i18next";
import { invoke } from "@tauri-apps/api/core";
import { Icon } from "../components/IconSprite";
import { pickFolder, pickFile } from "../lib/pickers";

export function FiveMServerPage() {
  const { t } = useTranslation();
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
      setApplyResult(t("fivemServer.apply_result", { count: result.length, path: serverCfgPath }));
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
          <h1 className="page-title">{t("fivemServer.title")}</h1>
          <p className="page-sub">
            <Trans i18nKey="fivemServer.subtitle" components={{ mono: <span className="mono" /> }} />
          </p>
        </div>
      </div>

      <div className="path-picker">
        <span className="path-icon">
          <Icon name="folder" />
        </span>
        <div className="path-text">
          <span className="label">{t("fivemServer.resources_label")}</span>
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
            placeholder={t("fivemServer.resources_placeholder")}
          />
        </div>
        <button
          className="btn-ghost"
          type="button"
          onClick={async () => {
            const picked = await pickFolder(t("fivemServer.resources_picker_title"));
            if (picked) setResourcesRoot(picked);
          }}
        >
          {t("fivemClient.browse")}
        </button>
        <button
          className="btn-primary"
          type="button"
          disabled={!resourcesRoot.trim() || resolving}
          onClick={resolve}
        >
          {resolving ? t("fivemServer.resolving") : t("fivemServer.resolve_button")}
        </button>
      </div>

      {error && <p className="error">{t("fivemServer.error_prefix", { error })}</p>}

      {order === null && !error && (
        <div className="panel">
          <div className="empty-state">
            <span className="glyph">
              <Icon name="bar-chart" />
            </span>
            <h3>{t("fivemServer.empty_title")}</h3>
            <p>
              <Trans i18nKey="fivemServer.empty_body" components={{ mono: <span className="mono" /> }} />
            </p>
          </div>
        </div>
      )}

      {order !== null && (
        <>
          <div className="stat-row">
            <div className="stat-card">
              <div className="eyebrow">{t("fivemServer.resources_found")}</div>
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
              {t("fivemServer.suggested_order")}
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
                {t("fivemServer.apply_title")}
              </h2>
            </div>
            <p className="diagnosis-disclaimer" style={{ margin: "-4px 0 10px" }}>
              <Trans i18nKey="fivemServer.apply_disclaimer" components={{ mono: <span className="mono" /> }} />
            </p>
            <div className="path-picker" style={{ margin: "0 0 10px" }}>
              <span className="path-icon">
                <Icon name="file-text" />
              </span>
              <div className="path-text">
                <span className="label">{t("fivemServer.cfg_label")}</span>
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
                  placeholder={t("fivemServer.cfg_placeholder")}
                />
              </div>
              <button
                className="btn-ghost"
                type="button"
                onClick={async () => {
                  const picked = await pickFile(["cfg"], t("fivemServer.cfg_picker_title"));
                  if (picked) setServerCfgPath(picked);
                }}
              >
                {t("fivemClient.browse")}
              </button>
              <button
                className="btn-primary"
                type="button"
                disabled={!serverCfgPath.trim() || applying}
                onClick={apply}
              >
                {applying ? t("fivemServer.applying") : t("fivemServer.apply_button")}
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
