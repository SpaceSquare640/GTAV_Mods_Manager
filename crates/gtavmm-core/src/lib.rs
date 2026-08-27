// SPDX-License-Identifier: AGPL-3.0-only

//! Core business logic for GTAV Mods Manager: game detection, mod install/uninstall,
//! conflict detection, recycle bin, and settings. No UI or CLI dependencies.
//!
//! MVP scope: Legacy SP Mods only. See `providers` for the `ModeProvider` abstraction
//! that later modes (Enhanced SP, LSPDFR, FiveM) will implement additively.

pub mod components;
pub mod conflict;
pub mod db;
pub mod error;
pub mod full_backup;
pub mod game_locator;
pub mod history;
pub mod install;
pub mod mod_analyzer;
pub mod protected_files;
pub mod providers;
pub mod recycle_bin;
pub mod settings;
pub mod state;
pub mod uninstall;
pub mod util;

pub use error::CoreError;
