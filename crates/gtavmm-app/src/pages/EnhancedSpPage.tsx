import { ModWorkspace } from "../components/ModWorkspace";
import { countByStatus, needsReview } from "../lib/modStats";

export function EnhancedSpPage() {
  return (
    <ModWorkspace
      pageMode="enhanced-sp"
      titleKey="enhancedSp.title"
      subtitleKey="enhancedSp.subtitle"
      banner={{ tone: "warn", icon: "info", key: "enhancedSp.unverified_assumption" }}
      stats={[
        { labelKey: "legacySp.stat_installed", value: ({ mods }) => mods.length },
        { labelKey: "legacySp.stat_active", tone: "accent", value: countByStatus("Active") },
        { labelKey: "legacySp.stat_disabled", value: countByStatus("Disabled") },
        { labelKey: "modWorkspace.stat_unverified", tone: "warn", value: needsReview },
      ]}
    />
  );
}
