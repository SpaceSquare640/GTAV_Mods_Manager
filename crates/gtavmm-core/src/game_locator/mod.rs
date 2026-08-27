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

/// Which edition an install directory was classified as. MVP only supports `Legacy`;
/// `Enhanced` is still correctly *recognized* (not silently mistreated as Legacy).
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
    /// A GTA V install was found, but it's the Enhanced edition, which MVP does not
    /// support yet — surfaced distinctly from `NotFound` so the CLI/UI can give an
    /// accurate message instead of implying nothing was found at all.
    FoundUnsupportedEdition {
        path: PathBuf,
        edition: GameEdition,
    },
    NotFound,
}

/// Classifies a directory by which recognized executable it contains. Does not touch
/// the filesystem beyond `Path::exists` checks.
pub fn classify_edition(dir: &Path) -> GameEdition {
    if dir.join("GTA5.exe").exists() || dir.join("PlayGTAV.exe").exists() {
        GameEdition::Legacy
    } else if dir.join("GTA5_Enhanced.exe").exists() {
        GameEdition::Enhanced
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
        edition @ (GameEdition::Enhanced | GameEdition::Unknown) => {
            if edition == GameEdition::Enhanced {
                DetectResult::FoundUnsupportedEdition {
                    path: dir,
                    edition: GameEdition::Enhanced,
                }
            } else {
                DetectResult::NotFound
            }
        }
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
    fn manual_path_validation_flags_enhanced_as_unsupported() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("GTA5_Enhanced.exe"), b"").unwrap();
        match validate_manual_path(dir.path()).unwrap() {
            DetectResult::FoundUnsupportedEdition { edition, .. } => {
                assert_eq!(edition, GameEdition::Enhanced);
            }
            other => panic!("expected FoundUnsupportedEdition, got {other:?}"),
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
