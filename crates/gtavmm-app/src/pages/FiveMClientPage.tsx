import { useState } from "react";
import { Icon } from "../components/IconSprite";
import { pickFolder } from "../lib/pickers";

export function FiveMClientPage() {
  const [clientPath, setClientPath] = useState("");

  return (
    <section className="view" data-shown="true">
      <div className="page-head">
        <div>
          <h1 className="page-title">FiveM · Client</h1>
          <p className="page-sub">
            Client-side asset mods (textures, models) for multiplayer servers. FiveM installs
            per-user wherever <em>you</em> put it (typically under your own{" "}
            <span className="mono">AppData\Local</span>, not a shared Program Files path) —
            there's no reliable location to auto-detect, so this always needs to be set
            manually.
          </p>
        </div>
      </div>

      <div className="path-picker">
        <span className="path-icon">
          <Icon name="folder" />
        </span>
        <div className="path-text">
          <span className="label">FiveM client folder</span>
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
            placeholder="Not set — required before installing anything"
          />
        </div>
        <button
          className="btn-ghost"
          type="button"
          onClick={async () => {
            const picked = await pickFolder("Select your FiveM client folder");
            if (picked) setClientPath(picked);
          }}
        >
          Browse…
        </button>
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
          <strong>Unverified assumption:</strong> client asset mods are assumed to reuse the
          same OpenIV <span className="mono">mods\</span>-mirroring convention as SP mods,
          since no real FiveM client install has been available to confirm against. Treat this
          workspace as the least battle-tested of the six.
        </span>
      </div>

      <div className="panel">
        <div className="empty-state">
          <span className="glyph">
            <Icon name="folder" />
          </span>
          <h3>Mod list not wired yet</h3>
          <p>
            Unlike the Legacy/Enhanced pages (which reuse{" "}
            <span className="mono">list_mods</span> with a disclosed caveat), this page
            deliberately does <strong>not</strong> call it: <span className="mono">
              installed_mod
            </span>{" "}
            has no per-mode scoping, so showing those rows here would silently mix real GTA V
            game mods in with a FiveM client's own — actively misleading, not just imprecise.
            A real FiveM-client mod list needs either a schema change or a separate provider-
            scoped table before this can show real data.
          </p>
        </div>
      </div>
    </section>
  );
}
