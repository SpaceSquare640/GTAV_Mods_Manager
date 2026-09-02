import { ModWorkspace } from "../components/ModWorkspace";
import { countByStatus } from "../lib/modStats";

export function LegacyLspdfrPage() {
  return (
    <ModWorkspace
      pageMode="legacy-lspdfr"
      titleKey="legacyLspdfr.title"
      subtitleKey="legacyLspdfr.subtitle"
      badges={["lspdfr"]}
      banner={{ tone: "info", icon: "info", key: "legacyLspdfr.known_limitation" }}
      stats={[
        { labelKey: "legacySp.stat_installed", value: ({ mods }) => mods.length },
        { labelKey: "legacySp.stat_active", tone: "accent", value: countByStatus("Active") },
        { labelKey: "legacySp.stat_disabled", value: countByStatus("Disabled") },
      ]}
    />
  );
}
