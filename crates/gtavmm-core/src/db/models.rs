// SPDX-License-Identifier: AGPL-3.0-only

//! Typed row structs mirroring `schema.sql`. CRUD lives in the modules that own each
//! table's lifecycle (`install`, `uninstall`, `state`, `recycle_bin`, `settings`), not
//! here — this module is just the shared shape + connection/migration plumbing.

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Platform {
    Windows,
    Linux,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum DetectedVia {
    Registry,
    Steam,
    Epic,
    Rockstar,
    Manual,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct GameInstallation {
    pub id: i64,
    pub platform: Platform,
    pub install_path: String,
    /// MVP only supports "legacy"; kept as a string field so Enhanced/other modes can
    /// be recognized (and reported as unsupported) without a schema migration later.
    pub edition: String,
    pub detected_via: DetectedVia,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ModStatus {
    Active,
    Disabled,
    Uninstalled,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledMod {
    pub id: i64,
    pub name: String,
    pub source_type: String,
    pub install_path: String,
    pub installed_at: String,
    pub status: ModStatus,
    pub notes: Option<String>,
    pub link: Option<String>,
    /// Which page this mod belongs to (`legacy-sp`, `legacy-lspdfr`,
    /// `enhanced-sp`, `enhanced-lspdfr`, `fivem-client`). `None` only for rows
    /// written before schema v11 introduced the column.
    pub mode: Option<String>,
    /// True when `mode` was guessed from the install path rather than recorded
    /// at install time, so the interface can say so instead of presenting a
    /// guess as a fact.
    pub mode_inferred: bool,
    /// LSPDFR category. `None` outside the LSPDFR pages.
    pub category: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstalledModFile {
    pub id: i64,
    pub installed_mod_id: i64,
    pub target_path: String,
    pub backup_path: Option<String>,
    pub file_hash: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum EventType {
    Install,
    Uninstall,
    Enable,
    Disable,
    Restore,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InstallEvent {
    pub id: i64,
    pub installed_mod_id: Option<i64>,
    pub event_type: EventType,
    pub timestamp: String,
    pub success: bool,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct UserSettings {
    pub language: String,
    pub default_auto_backup: bool,
    pub game_install_path_override: Option<String>,
    /// The user's *choice* — "system", "dark" or "light" — not the palette that
    /// choice currently resolves to. Storing the choice keeps it meaningful when
    /// the OS preference changes later. `None` means never chosen.
    pub theme: Option<String>,
    /// Which version of the terms was accepted. A version rather than a flag, so
    /// revising the terms can ask again deliberately instead of either passing
    /// silently or forcing everyone to re-accept with no way to tell.
    pub terms_accepted_version: Option<String>,
    /// Whether first-run setup finished. Separate from the terms, because
    /// someone can accept and then quit before choosing game paths.
    pub onboarding_completed: bool,
    /// Where full backups go when the default app-data location will not do.
    pub backup_root_override: Option<String>,
}

impl Default for UserSettings {
    fn default() -> Self {
        Self {
            language: "en".to_string(),
            default_auto_backup: true,
            game_install_path_override: None,
            theme: None,
            terms_accepted_version: None,
            onboarding_completed: false,
            backup_root_override: None,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct RecycleBinEntry {
    pub id: i64,
    pub original_installed_mod_id: Option<i64>,
    pub mod_package_snapshot_path: String,
    pub deleted_at: String,
    /// `deleted_at + 15 days`; entries past this are swept on startup. See
    /// `recycle_bin::sweep_expired`.
    pub expires_at: String,
}
