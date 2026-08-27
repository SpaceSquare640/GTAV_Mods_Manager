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
        // Anti-cheat components (Legacy). `Beclient_x64.dll` also case-insensitively
        // matches the real Enhanced install's `BEClient_x64.dll`.
        "BattlEye.exe",
        "Beclient.dll",
        "Beclient_x64.dll",
        // Anti-cheat components confirmed present in a real Enhanced install's
        // BattlEye\ folder (not shared with Legacy, so listed separately).
        "GTA5_Enhanced_BE.exe",
        "BEService_x64.exe",
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

    // Paths are built with `Path::join`, not hand-formatted backslash strings — a
    // literal `r"C:\Games\GTAV\x"` is a single opaque path component on Linux (`\`
    // isn't a separator there), which silently defeats `file_name()`/`extension()`
    // and would pass locally on Windows while failing in CI on Linux.
    fn game_path(name: &str) -> PathBuf {
        PathBuf::from("game").join(name)
    }

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
            "GTA5_Enhanced_BE.exe",
            "BEService_x64.exe",
        ] {
            let path = game_path(name);
            assert!(is_protected(&path), "{name} should be protected");
            assert!(check_write(&path).is_err());
        }
    }

    #[test]
    fn blocks_any_exe_extension() {
        let path = game_path("some_random_tool.exe");
        assert!(is_protected(&path));
    }

    #[test]
    fn blocks_rage_plugin_hook_launcher_via_the_blanket_exe_rule() {
        // RAGEPluginHook.exe (the launcher a real LSPDFR install is started through,
        // confirmed present at the root of a real modpack backup inspected on
        // 2026-08-27) isn't in PROTECTED_FILE_NAMES by name — it doesn't need to be,
        // since the blanket .exe extension rule already covers it. This test pins
        // that down explicitly rather than relying on it being an accident.
        let path = game_path("RAGEPluginHook.exe");
        assert!(is_protected(&path));
        assert!(check_write(&path).is_err());
    }

    #[test]
    fn allows_ordinary_mod_files() {
        for name in [
            "my_mod.asi",
            "MyScript.dll",
            "menyoo_outfit.xml",
            "readme.txt",
        ] {
            let path = game_path(name);
            assert!(!is_protected(&path), "{name} should NOT be protected");
            assert!(check_write(&path).is_ok());
        }
    }

    #[test]
    fn matches_regardless_of_directory() {
        let path = PathBuf::from("game")
            .join("scripts")
            .join("subdir")
            .join("GTA5.exe");
        assert!(is_protected(&path));
    }
}
