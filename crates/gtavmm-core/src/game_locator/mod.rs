// SPDX-License-Identifier: AGPL-3.0-only

//! Game installation detection: Windows Registry → Steam `libraryfolders.vdf` → Epic
//! manifest scan → Rockstar launcher key (Windows), Proton `compatdata` scan (Linux),
//! with a manual-path override always available as a fallback.
//!
//! Per the project's design principle, this does **not** try to achieve 100% coverage
//! across every distro/launcher/library-location combination — it detects the most
//! common cases and always leaves `validate_manual_path` as an equally-supported
//! fallback for anything auto-detection misses.

#[cfg(target_os = "windows")]
mod windows;

#[cfg(target_os = "linux")]
mod linux;

use std::path::{Path, PathBuf};

use crate::db::models::{DetectedVia, GameInstallation, Platform};
use crate::error::CoreResult;

/// Which edition an install directory was classified as. Both `Legacy` and
/// `Enhanced` are supported (see `providers::EnhancedSpProvider`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameEdition {
    Legacy,
    Enhanced,
    Unknown,
}

/// Outcome of a detection attempt against one candidate directory or of the overall
/// auto-detection sweep.
#[derive(Debug, Clone)]
pub enum DetectResult {
    Found(GameInstallation),
    /// Reserved for a recognized-but-unsupported edition. Currently unused — both
    /// editions this project targets (Legacy, Enhanced) are supported — but kept so
    /// the CLI/UI can distinguish "found something we don't support" from `NotFound`
    /// if a future edition needs this.
    FoundUnsupportedEdition {
        path: PathBuf,
        edition: GameEdition,
    },
    NotFound,
}

/// Classifies a directory by which recognized executable it contains. Does not touch
/// the filesystem beyond `Path::exists` checks.
///
/// **Enhanced must be checked first**: a real Enhanced install (confirmed by
/// inspecting one directly) also ships `PlayGTAV.exe` alongside `GTA5_Enhanced.exe` —
/// presumably a legacy-compatibility shim. An earlier version of this function
/// checked Legacy's signals first and silently misclassified a real Enhanced
/// installation as Legacy as a result; checking `GTA5_Enhanced.exe` first avoids that.
pub fn classify_edition(dir: &Path) -> GameEdition {
    if dir.join("GTA5_Enhanced.exe").exists() {
        GameEdition::Enhanced
    } else if dir.join("GTA5.exe").exists() || dir.join("PlayGTAV.exe").exists() {
        GameEdition::Legacy
    } else {
        GameEdition::Unknown
    }
}

fn current_platform() -> Platform {
    #[cfg(target_os = "windows")]
    {
        Platform::Windows
    }
    #[cfg(target_os = "linux")]
    {
        Platform::Linux
    }
    #[cfg(not(any(target_os = "windows", target_os = "linux")))]
    {
        compile_error!("gtavmm-core only targets Windows and Linux");
    }
}

fn to_detect_result(dir: PathBuf, via: DetectedVia) -> DetectResult {
    match classify_edition(&dir) {
        GameEdition::Legacy => DetectResult::Found(GameInstallation {
            id: 0, // assigned by the DB on insert; not yet persisted here
            platform: current_platform(),
            install_path: dir.to_string_lossy().into_owned(),
            edition: "legacy".to_string(),
            detected_via: via,
        }),
        GameEdition::Enhanced => DetectResult::Found(GameInstallation {
            id: 0,
            platform: current_platform(),
            install_path: dir.to_string_lossy().into_owned(),
            edition: "enhanced".to_string(),
            detected_via: via,
        }),
        GameEdition::Unknown => DetectResult::NotFound,
    }
}

/// Attempts auto-detection in priority order (Registry → Steam → Epic → Rockstar on
/// Windows; Proton `compatdata` scan on Linux). Returns `DetectResult::NotFound` (not
/// an error) when nothing is found, so callers can fall back to prompting for a
/// manual path.
pub fn detect() -> CoreResult<DetectResult> {
    #[cfg(target_os = "windows")]
    {
        windows::detect()
    }
    #[cfg(target_os = "linux")]
    {
        linux::detect()
    }
}

/// Validates a manually-provided path by checking for a recognized executable.
pub fn validate_manual_path(path: &Path) -> CoreResult<DetectResult> {
    Ok(to_detect_result(path.to_path_buf(), DetectedVia::Manual))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classifies_legacy_by_exe_presence() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("PlayGTAV.exe"), b"").unwrap();
        assert_eq!(classify_edition(dir.path()), GameEdition::Legacy);
    }

    #[test]
    fn classifies_enhanced_by_exe_presence() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("GTA5_Enhanced.exe"), b"").unwrap();
        assert_eq!(classify_edition(dir.path()), GameEdition::Enhanced);
    }

    #[test]
    fn enhanced_takes_priority_when_both_exes_are_present() {
        // Regression test for a real bug found by inspecting an actual Enhanced
        // install: it ships PlayGTAV.exe alongside GTA5_Enhanced.exe, which an
        // earlier version of this function misclassified as Legacy.
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("PlayGTAV.exe"), b"").unwrap();
        std::fs::write(dir.path().join("GTA5_Enhanced.exe"), b"").unwrap();
        assert_eq!(classify_edition(dir.path()), GameEdition::Enhanced);
    }

    #[test]
    fn classifies_unknown_when_no_recognized_exe() {
        let dir = tempfile::tempdir().unwrap();
        assert_eq!(classify_edition(dir.path()), GameEdition::Unknown);
    }

    #[test]
    fn manual_path_validation_recognizes_legacy_install() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("GTA5.exe"), b"").unwrap();
        match validate_manual_path(dir.path()).unwrap() {
            DetectResult::Found(installation) => {
                assert_eq!(installation.edition, "legacy");
                assert_eq!(installation.detected_via, DetectedVia::Manual);
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn manual_path_validation_recognizes_enhanced_as_supported() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("GTA5_Enhanced.exe"), b"").unwrap();
        match validate_manual_path(dir.path()).unwrap() {
            DetectResult::Found(installation) => {
                assert_eq!(installation.edition, "enhanced");
                assert_eq!(installation.detected_via, DetectedVia::Manual);
            }
            other => panic!("expected Found, got {other:?}"),
        }
    }

    #[test]
    fn manual_path_validation_reports_not_found_for_empty_dir() {
        let dir = tempfile::tempdir().unwrap();
        assert!(matches!(
            validate_manual_path(dir.path()).unwrap(),
            DetectResult::NotFound
        ));
    }
}
