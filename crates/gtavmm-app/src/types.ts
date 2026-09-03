export type Mode = "legacy" | "enhanced" | "fivem";
export type Sub = "mods" | "lspdfr" | "client" | "server" | "converter";

export type ModStatus = "Active" | "Disabled" | "Uninstalled";

/**
 * Which workspace page a mod belongs to.
 *
 * Distinct from the provider mode ("sp"/"lspdfr"/"fivem-client") the install
 * path uses: Legacy SP and Enhanced SP share a provider but are separate pages
 * with separate mod lists.
 */
export type PageMode =
  | "legacy-sp"
  | "legacy-lspdfr"
  | "enhanced-sp"
  | "enhanced-lspdfr"
  | "fivem-client";

export interface InstalledMod {
  id: number;
  name: string;
  source_type: string;
  install_path: string;
  installed_at: string;
  status: ModStatus;
  notes: string | null;
  link: string | null;
  mode: PageMode | null;
  /** True when `mode` was guessed from the install path, not recorded at install. */
  mode_inferred: boolean;
  category: string | null;
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

/** Result of importing a profile. Mods not installed here are named, never fetched. */
export interface ImportOutcome {
  profile_id: number;
  matched: string[];
  not_found_locally: string[];
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

export type Component = "ScriptHookV" | "ScriptHookVDotNet" | "OpenIvOrOpenRpf";

export interface ComponentStatus {
  component: Component;
  is_installed: boolean;
  display_name: string;
  official_download_url: string;
}

export interface RecycleBinEntry {
  id: number;
  original_installed_mod_id: number | null;
  deleted_at: string;
  expires_at: string;
}

export interface ModSearchResult {
  id: number;
  name: string;
  status: string;
  notes: string | null;
  link: string | null;
}

export interface UpdateCheckResult {
  current_version: string;
  latest_version: string;
  update_available: boolean;
  release_url: string;
  platform_download_url: string | null;
}

export type ScanOutcome =
  | "Clean"
  | { ThreatDetected: { details: string | null } }
  | { Unavailable: { reason: string } };

export interface FileHashes {
  md5: string;
  sha1: string;
  sha256: string;
}

export interface PromptTemplate {
  id: number;
  name: string;
  content: string;
  created_at: string;
  updated_at: string;
}

export type AiProviderKind = "ollama" | "cloud";

export interface AiSettings {
  enabled: boolean;
  provider: AiProviderKind | null;
  ollama_model: string | null;
  cloud_endpoint: string | null;
  cloud_model: string | null;
}

export function formatLabel(format: ModFormat): string {
  if (typeof format === "string") return format;
  if ("AddOnPack" in format) return `Add-on pack (${format.AddOnPack.pack_name})`;
  if ("Unsupported" in format) return `Unsupported: ${format.Unsupported}`;
  return "Unknown";
}
