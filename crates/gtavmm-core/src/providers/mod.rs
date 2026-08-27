// SPDX-License-Identifier: AGPL-3.0-only

//! `ModeProvider`: the abstraction that lets later modes (Enhanced SP, Legacy/Enhanced
//! LSPDFR, FiveM) be added without rewriting `mod_analyzer`'s dispatch logic. Each
//! mode's directory layout (where a `.dll` script goes, what `mods\` mirroring looks
//! like, etc.) lives entirely inside its own `ModeProvider` implementation — the
//! classifier only decides *what kind* of file something is and asks the active
//! provider *where it goes*.
//!
//! MVP implements only [`LegacySpProvider`]. Adding `EnhancedSpProvider`,
//! `LspdfrProvider`, or a `FiveMProvider` later means writing a new impl of this
//! trait, not touching `mod_analyzer`'s classification branches.

use std::ffi::OsStr;
use std::path::{Path, PathBuf};

use crate::mod_analyzer::{MenyooCategory, MENYOO_ROOT_FOLDER, MODS_SUBFOLDER, SCRIPTS_SUBFOLDER};

/// Resolves mode-specific target paths for each kind of file `mod_analyzer` can
/// classify. Every method takes already-known, mode-agnostic information (a file
/// name, a category, a path relative to some mod's own root) and returns an absolute
/// target path under this provider's `game_root()`.
pub trait ModeProvider {
    /// The detected game/resource installation root this provider operates against.
    fn game_root(&self) -> &Path;

    fn resolve_asi_target(&self, file_name: &OsStr) -> PathBuf;
    fn resolve_native_dll_target(&self, file_name: &OsStr) -> PathBuf;
    fn resolve_managed_dll_target(&self, file_name: &OsStr) -> PathBuf;
    fn resolve_menyoo_target(&self, category: MenyooCategory, file_name: &OsStr) -> PathBuf;
    /// `relative` is the file's path relative to the root of the mod folder being
    /// installed (i.e. already stripped of the source folder's own prefix).
    fn resolve_folder_replacer_target(&self, relative: &Path) -> PathBuf;
    fn resolve_add_on_pack_target(&self, pack_name: &str, relative: &Path) -> PathBuf;
    /// `relative_output` is the `output` path from an `.oiv` package's `assembly.xml`
    /// (already validated as not referencing an archive-internal path — see
    /// `mod_analyzer::oiv`), interpreted relative to `game_root()`.
    fn resolve_oiv_target(&self, relative_output: &Path) -> PathBuf;
}

pub struct LegacySpProvider {
    game_root: PathBuf,
}

impl LegacySpProvider {
    pub fn new(game_root: PathBuf) -> Self {
        Self { game_root }
    }
}

impl ModeProvider for LegacySpProvider {
    fn game_root(&self) -> &Path {
        &self.game_root
    }

    fn resolve_asi_target(&self, file_name: &OsStr) -> PathBuf {
        self.game_root.join(file_name)
    }

    fn resolve_native_dll_target(&self, file_name: &OsStr) -> PathBuf {
        self.game_root.join(file_name)
    }

    fn resolve_managed_dll_target(&self, file_name: &OsStr) -> PathBuf {
        self.game_root.join(SCRIPTS_SUBFOLDER).join(file_name)
    }

    fn resolve_menyoo_target(&self, category: MenyooCategory, file_name: &OsStr) -> PathBuf {
        let mut dir = self.game_root.join(MENYOO_ROOT_FOLDER);
        if let Some(subfolder) = category.subfolder() {
            dir = dir.join(subfolder);
        }
        dir.join(file_name)
    }

    fn resolve_folder_replacer_target(&self, relative: &Path) -> PathBuf {
        self.game_root.join(MODS_SUBFOLDER).join(relative)
    }

    fn resolve_add_on_pack_target(&self, pack_name: &str, relative: &Path) -> PathBuf {
        self.game_root
            .join(MODS_SUBFOLDER)
            .join("update")
            .join("x64")
            .join("dlcpacks")
            .join(pack_name)
            .join(relative)
    }

    fn resolve_oiv_target(&self, relative_output: &Path) -> PathBuf {
        self.game_root.join(relative_output)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn resolves_expected_legacy_sp_layout() {
        let game_root = PathBuf::from("game");
        let provider = LegacySpProvider::new(game_root.clone());

        assert_eq!(
            provider.resolve_asi_target(OsStr::new("mod.asi")),
            game_root.join("mod.asi")
        );
        assert_eq!(
            provider.resolve_managed_dll_target(OsStr::new("Script.dll")),
            game_root.join("scripts").join("Script.dll")
        );
        assert_eq!(
            provider.resolve_menyoo_target(MenyooCategory::Outfit, OsStr::new("fit.xml")),
            game_root
                .join("menyooStuff")
                .join("Outfits")
                .join("fit.xml")
        );
        assert_eq!(
            provider.resolve_add_on_pack_target("MyCar", Path::new("dlc.rpf")),
            game_root
                .join("mods")
                .join("update")
                .join("x64")
                .join("dlcpacks")
                .join("MyCar")
                .join("dlc.rpf")
        );
    }
}
