export type Mode = "legacy" | "enhanced" | "fivem";
export type Sub = "mods" | "lspdfr" | "client" | "server" | "converter";

export type ModStatus = "Active" | "Disabled" | "Uninstalled";

export interface InstalledMod {
  id: number;
  name: string;
  source_type: string;
  install_path: string;
  installed_at: string;
  status: ModStatus;
  notes: string | null;
  link: string | null;
}

export type DetectGameResult =
  | { status: "found"; install_path: string; edition: string }
  | { status: "not_found" };

// --- Install wizard types — mirror gtavmm_core::mod_analyzer/conflict/install's real
// serde shapes exactly (see crates/gtavmm-app/src-tauri/src/commands.rs). Unit enum
// variants serialize as bare strings; struct/newtype variants as { VariantName: ... }.
export type ModFormat =
  | "Asi"
  | "NativeDll"
  | "ManagedDll"
  | "MenyooXml"
  | { AddOnPack: { pack_name: string } }
  | "Zip"
  | "SevenZip"
  | { Unsupported: string };

export interface PlannedFile {
  source: string;
  target: string;
}

export interface ModPlan {
  format: ModFormat;
  files: PlannedFile[];
}

export interface ProtectedHit {
  path: string;
}
export interface ForeignConflict {
  owner_mod_id: number;
  owner_name: string;
  path: string;
}
export interface SelfUpdateSuggestion {
  existing_mod_id: number;
  existing_name: string;
  overlap_ratio: number;
}
export interface ConflictReport {
  protected_hits: ProtectedHit[];
  foreign_conflicts: ForeignConflict[];
  self_update: SelfUpdateSuggestion | null;
}

export type InstallOutcome =
  | { Success: { installed_mod_id: number; files_written: number } }
  | { RequiresOverride: ConflictReport }
  | { ProtectedFileBlocked: string[] };

export function formatLabel(format: ModFormat): string {
  if (typeof format === "string") return format;
  if ("AddOnPack" in format) return `Add-on pack (${format.AddOnPack.pack_name})`;
  if ("Unsupported" in format) return `Unsupported: ${format.Unsupported}`;
  return "Unknown";
}
