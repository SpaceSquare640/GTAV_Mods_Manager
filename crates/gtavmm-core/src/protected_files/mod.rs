// SPDX-License-Identifier: AGPL-3.0-only

//! The immutable allow/deny list guarding GTA V core binaries and anti-cheat components.
//!
//! No install/uninstall/enable/disable/recycle-bin-restore path may bypass
//! [`check_write`] — not even future automation (e.g. an AI-assistant "Plan" action).
//! This is the single source of truth; there is no override.

use std::collections::HashSet;
use std::path::Path;
use std::sync::LazyLock;

use crate::error::{CoreError, CoreResult};

/// Core executables and anti-cheat binaries that must never be written, overwritten,
/// or deleted, matched by file name only (case-insensitive).
static PROTECTED_FILE_NAMES: LazyLock<HashSet<&'static str>> = LazyLock::new(|| {
    [
        // Launchers / main executables (Legacy and Enhanced editions).
        "GTA5.exe",
        "GTA5_Enhanced.exe",
        "GTAV.exe",
        "PlayGTAV.exe",
        "GTAVLauncher.exe",
        "Launcher.exe",
        // Rockstar / online integrity binaries.
        "GTAVLanguageSelect.exe",
        "RockstarService.exe",
        "RockstarSteamHelper.exe",
        // Anti-cheat components.
        "BattlEye.exe",
        "Beclient.dll",
        "Beclient_x64.dll",
    ]
    .into_iter()
    .collect()
});

/// File extensions treated as core binaries; writing these into the game root is
/// blocked unless the caller explicitly classifies the payload as an allowed mod type
/// (e.g. a third-party `.asi`/`.dll` going into a plugin folder — that classification
/// happens in `mod_analyzer`, not here; this module only guards the write itself).
const PROTECTED_EXTENSIONS: &[&str] = &["exe"];

fn file_name_matches(path: &Path) -> bool {
    path.file_name()
        .and_then(|n| n.to_str())
        .is_some_and(|name| {
            PROTECTED_FILE_NAMES
                .iter()
                .any(|protected| protected.eq_ignore_ascii_case(name))
        })
}

fn extension_matches(path: &Path) -> bool {
    path.extension()
        .and_then(|e| e.to_str())
        .is_some_and(|ext| {
            PROTECTED_EXTENSIONS
                .iter()
                .any(|protected| protected.eq_ignore_ascii_case(ext))
        })
}

/// Returns `true` if `path` targets a protected file by name or extension.
pub fn is_protected(path: &Path) -> bool {
    file_name_matches(path) || extension_matches(path)
}

/// Guards a planned write. Call this before every file write/overwrite/delete in the
/// install, uninstall, enable/disable, and recycle-bin-restore pipelines.
pub fn check_write(path: &Path) -> CoreResult<()> {
    if is_protected(path) {
        return Err(CoreError::ProtectedFileViolation {
            path: path.to_path_buf(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn blocks_known_executables_case_insensitively() {
        for name in [
            "GTA5.exe",
            "gta5.exe",
            "GTA5_Enhanced.exe",
            "PlayGTAV.exe",
            "RockstarService.exe",
            "BattlEye.exe",
            "Beclient_x64.dll",
        ] {
            let path = PathBuf::from(format!(r"C:\Games\GTAV\{name}"));
            assert!(is_protected(&path), "{name} should be protected");
            assert!(check_write(&path).is_err());
        }
    }

    #[test]
    fn blocks_any_exe_extension() {
        let path = PathBuf::from(r"C:\Games\GTAV\some_random_tool.exe");
        assert!(is_protected(&path));
    }

    #[test]
    fn allows_ordinary_mod_files() {
        for name in [
            "my_mod.asi",
            "MyScript.dll",
            "menyoo_outfit.xml",
            "readme.txt",
        ] {
            let path = PathBuf::from(format!(r"C:\Games\GTAV\{name}"));
            assert!(!is_protected(&path), "{name} should NOT be protected");
            assert!(check_write(&path).is_ok());
        }
    }

    #[test]
    fn matches_regardless_of_directory() {
        let path = PathBuf::from(r"C:\Games\GTAV\scripts\subdir\GTA5.exe");
        assert!(is_protected(&path));
    }
}
