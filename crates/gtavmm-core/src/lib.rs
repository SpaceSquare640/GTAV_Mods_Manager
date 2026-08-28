// SPDX-License-Identifier: AGPL-3.0-only

//! Core business logic for GTAV Mods Manager: game detection, mod install/uninstall,
//! conflict detection, recycle bin, and settings. No UI or CLI dependencies.
//!
//! MVP scope: Legacy SP Mods only. See `providers` for the `ModeProvider` abstraction
//! that later modes (Enhanced SP, LSPDFR, FiveM) will implement additively.

pub mod ai_assistant;
pub mod components;
pub mod conflict;
pub mod crash_report;
pub mod db;
pub mod error;
pub mod fivem;
pub mod full_backup;
pub mod game_locator;
pub mod history;
pub mod install;
pub mod malware_scan;
pub mod mod_analyzer;
pub mod profile;
pub mod prompt_template;
pub mod protected_files;
pub mod providers;
pub mod recycle_bin;
pub mod settings;
pub mod state;
pub mod uninstall;
pub mod update_check;
pub mod util;
pub mod xlsx_export;

pub use error::CoreError;
