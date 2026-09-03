import { ModWorkspace } from "../components/ModWorkspace";

export function EnhancedLspdfrPage() {
  return (
    <ModWorkspace
      pageMode="enhanced-lspdfr"
      titleKey="enhancedLspdfr.title"
      subtitleKey="enhancedLspdfr.subtitle"
      categories={["callouts", "other"]}
      toolsVariant="framework"
      badges={["lspdfr", "beta"]}
      banner={{ tone: "warn", icon: "alert-triangle", key: "enhancedLspdfr.beta_support" }}
      /* The design shows no stat cards here: with Enhanced LSPDFR unverified,
         a row of counts would lend it a confidence the page does not have. */
      stats={[]}
    />
  );
}
