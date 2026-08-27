// SPDX-License-Identifier: AGPL-3.0-only

//! `ModeProvider`: the abstraction that lets later modes (Enhanced SP, Legacy/Enhanced
//! LSPDFR, FiveM) be added without rewriting the core install/conflict/recycle-bin
//! logic. MVP implements only [`LegacySpProvider`].
//!
//! Not yet implemented — milestone 3 (mod_analyzer) will flesh this out.

use std::path::Path;

/// Resolves mode-specific target paths and format-classification rules.
pub trait ModeProvider {
    /// The detected game installation root this provider operates against.
    fn game_root(&self) -> &Path;
}

pub struct LegacySpProvider {
    game_root: std::path::PathBuf,
}

impl LegacySpProvider {
    pub fn new(game_root: std::path::PathBuf) -> Self {
        Self { game_root }
    }
}

impl ModeProvider for LegacySpProvider {
    fn game_root(&self) -> &Path {
        &self.game_root
    }
}
