// SPDX-License-Identifier: AGPL-3.0-only

//! Windows detection chain: Registry → Steam (`libraryfolders.vdf`) → Epic (manifest
//! scan) → Rockstar Games Launcher default path. Best-effort — exact registry key
//! layouts and manifest formats are undocumented/proprietary and vary across installs;
//! anything missed here is always recoverable via `validate_manual_path`.

use std::path::PathBuf;

use crate::db::models::DetectedVia;
use crate::error::CoreResult;

use super::{to_detect_result, DetectResult};

pub fn detect() -> CoreResult<DetectResult> {
    let mut unsupported_fallback: Option<DetectResult> = None;

    for (candidate, via) in candidates() {
        match to_detect_result(candidate, via) {
            found @ DetectResult::Found(_) => return Ok(found),
            unsupported @ DetectResult::FoundUnsupportedEdition { .. } => {
                unsupported_fallback.get_or_insert(unsupported);
            }
            DetectResult::NotFound => {}
        }
    }

    Ok(unsupported_fallback.unwrap_or(DetectResult::NotFound))
}

/// Every directory worth checking, in priority order, paired with how it was found.
fn candidates() -> Vec<(PathBuf, DetectedVia)> {
    let mut out = Vec::new();

    if let Some(path) = registry_install_folder() {
        out.push((path, DetectedVia::Registry));
    }
    out.extend(
        steam_candidates()
            .into_iter()
            .map(|p| (p, DetectedVia::Steam)),
    );
    out.extend(
        epic_candidates()
            .into_iter()
            .map(|p| (p, DetectedVia::Epic)),
    );
    if let Some(path) = rockstar_launcher_default_path() {
        out.push((path, DetectedVia::Rockstar));
    }

    out
}

fn registry_install_folder() -> Option<PathBuf> {
    use winreg::enums::HKEY_LOCAL_MACHINE;
    use winreg::RegKey;

    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    for subkey in [
        r"SOFTWARE\WOW6432Node\Rockstar Games\Grand Theft Auto V",
        r"SOFTWARE\Rockstar Games\Grand Theft Auto V",
    ] {
        if let Ok(key) = hklm.open_subkey(subkey) {
            if let Ok(install_folder) = key.get_value::<String, _>("InstallFolder") {
                let path = PathBuf::from(install_folder);
                if path.is_dir() {
                    return Some(path);
                }
            }
        }
    }
    None
}

fn steam_install_path() -> Option<PathBuf> {
    use winreg::enums::HKEY_CURRENT_USER;
    use winreg::RegKey;

    let hkcu = RegKey::predef(HKEY_CURRENT_USER);
    let key = hkcu.open_subkey(r"Software\Valve\Steam").ok()?;
    let steam_path: String = key.get_value("SteamPath").ok()?;
    Some(PathBuf::from(steam_path))
}

fn steam_candidates() -> Vec<PathBuf> {
    let Some(steam_path) = steam_install_path() else {
        return Vec::new();
    };

    let mut library_roots = vec![steam_path.clone()];
    let vdf_path = steam_path.join("steamapps").join("libraryfolders.vdf");
    if let Ok(contents) = std::fs::read_to_string(&vdf_path) {
        library_roots.extend(parse_vdf_library_paths(&contents));
    }

    library_roots
        .into_iter()
        .map(|root| {
            root.join("steamapps")
                .join("common")
                .join("Grand Theft Auto V")
        })
        .filter(|p| p.is_dir())
        .collect()
}

/// Extracts `"path"  "D:\\SteamLibrary"`-style entries from `libraryfolders.vdf`.
/// This is a deliberately simple line scanner, not a full VDF parser — Valve's VDF
/// format is simple enough (quoted key/value pairs, one per line) that this covers
/// every real-world file without pulling in a parser dependency for one field.
fn parse_vdf_library_paths(contents: &str) -> Vec<PathBuf> {
    contents
        .lines()
        .filter_map(|line| {
            let line = line.trim();
            if !line.starts_with("\"path\"") {
                return None;
            }
            // Expect: "path"    "D:\\SteamLibrary" — split on '"' and take the 4th
            // quoted segment (index 3): ["", path, "", D:\\SteamLibrary, ""].
            let segments: Vec<&str> = line.split('"').collect();
            segments
                .get(3)
                .map(|raw| PathBuf::from(raw.replace("\\\\", "\\")))
        })
        .collect()
}

fn epic_manifest_dir() -> PathBuf {
    PathBuf::from(r"C:\ProgramData\Epic\EpicGamesLauncher\Data\Manifests")
}

fn epic_candidates() -> Vec<PathBuf> {
    let dir = epic_manifest_dir();
    let Ok(entries) = std::fs::read_dir(&dir) else {
        return Vec::new();
    };

    entries
        .filter_map(|entry| entry.ok())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == "item"))
        .filter_map(|entry| std::fs::read_to_string(entry.path()).ok())
        .filter_map(|contents| serde_json::from_str::<serde_json::Value>(&contents).ok())
        .filter(|manifest| {
            manifest
                .get("DisplayName")
                .and_then(|v| v.as_str())
                .is_some_and(|name| name.contains("Grand Theft Auto V"))
        })
        .filter_map(|manifest| {
            manifest
                .get("InstallLocation")
                .and_then(|v| v.as_str())
                .map(PathBuf::from)
        })
        .filter(|path| path.is_dir())
        .collect()
}

fn rockstar_launcher_default_path() -> Option<PathBuf> {
    for program_files in [
        std::env::var_os("ProgramFiles").map(PathBuf::from),
        std::env::var_os("ProgramFiles(x86)").map(PathBuf::from),
    ]
    .into_iter()
    .flatten()
    {
        let candidate = program_files
            .join("Rockstar Games")
            .join("Grand Theft Auto V");
        if candidate.is_dir() {
            return Some(candidate);
        }
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_simple_vdf_library_path() {
        let vdf = r#"
"libraryfolders"
{
	"0"
	{
		"path"		"D:\\SteamLibrary"
		"label"		""
	}
}
"#;
        let paths = parse_vdf_library_paths(vdf);
        assert_eq!(paths, vec![PathBuf::from(r"D:\SteamLibrary")]);
    }

    #[test]
    fn ignores_non_path_lines() {
        let vdf = "\"label\"\t\t\"something\"\n\"contentid\"\t\t\"12345\"";
        assert!(parse_vdf_library_paths(vdf).is_empty());
    }
}
