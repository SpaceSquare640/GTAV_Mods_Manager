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

export interface Profile {
  id: number;
  name: string;
  created_at: string;
  is_active: boolean;
}

export interface SwitchOutcome {
  enabled: number[];
  disabled: number[];
}

export interface TranslatableString {
  index: number;
  text: string;
}

export interface DllInspection {
  total_strings: number;
  excluded_technical: number;
  translatable: TranslatableString[];
}

export interface TranslatedDraftEntry {
  index: number;
  source: string;
  translated: string;
}

export interface DllTranslationOutcome {
  output_path: string;
  strings_translated: number;
  call_sites_patched: number;
  skipped: string[];
}

export type EventType = "Install" | "Uninstall" | "Enable" | "Disable" | "Restore";

export interface InstallEvent {
  id: number;
  installed_mod_id: number | null;
  event_type: EventType;
  timestamp: string;
  success: boolean;
  error_message: string | null;
}

export interface SavedModLink {
  id: number;
  name: string;
  url: string;
  notes: string | null;
  created_at: string;
  /** `null` = the user's own general bookmark; `"mod_setup"` = the built-in
   *  "模組 Setup 建議" tab seeded by the schema migration. */
  category: string | null;
}

export function formatLabel(format: ModFormat): string {
  if (typeof format === "string") return format;
  if ("AddOnPack" in format) return `Add-on pack (${format.AddOnPack.pack_name})`;
  if ("Unsupported" in format) return `Unsupported: ${format.Unsupported}`;
  return "Unknown";
}
