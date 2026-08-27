// SPDX-License-Identifier: AGPL-3.0-only

//! Linux detection: scans common Steam library locations for GTA V's Proton
//! `compatdata` prefix (Steam App ID 271590) and checks the expected install path
//! inside the Windows-format prefix. Only the most common Steam install locations are
//! checked — distro/Flatpak/custom-prefix variability is intentionally not chased
//! exhaustively; `validate_manual_path` covers the rest.

use std::path::PathBuf;

use crate::db::models::DetectedVia;
use crate::error::CoreResult;

use super::{to_detect_result, DetectResult};

/// GTA V's Steam App ID, used to locate its Proton compatdata prefix.
const GTA_V_STEAM_APP_ID: &str = "271590";

pub fn detect() -> CoreResult<DetectResult> {
    let mut unsupported_fallback: Option<DetectResult> = None;

    for path in candidates() {
        match to_detect_result(path, DetectedVia::Manual) {
            found @ DetectResult::Found(_) => return Ok(found),
            unsupported @ DetectResult::FoundUnsupportedEdition { .. } => {
                unsupported_fallback.get_or_insert(unsupported);
            }
            DetectResult::NotFound => {}
        }
    }

    Ok(unsupported_fallback.unwrap_or(DetectResult::NotFound))
}

fn candidates() -> Vec<PathBuf> {
    let Some(home) = dirs_home() else {
        return Vec::new();
    };

    let mut steam_roots = vec![
        home.join(".steam/steam"),
        home.join(".local/share/Steam"),
        home.join(".var/app/com.valvesoftware.Steam/.local/share/Steam"), // Flatpak
    ];
    steam_roots.retain(|p| p.is_dir());

    steam_roots
        .into_iter()
        .map(|root| {
            root.join("steamapps")
                .join("compatdata")
                .join(GTA_V_STEAM_APP_ID)
                .join("pfx")
                .join("drive_c")
                .join("Program Files")
                .join("Rockstar Games")
                .join("Grand Theft Auto V")
        })
        .filter(|p| p.is_dir())
        .collect()
}

fn dirs_home() -> Option<PathBuf> {
    std::env::var_os("HOME").map(PathBuf::from)
}
