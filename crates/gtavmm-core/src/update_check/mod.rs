// SPDX-License-Identifier: AGPL-3.0-only

//! Checks GitHub Releases for a newer published version than the running one.
//!
//! This is deliberately just the *check* — it reports whether an update exists and
//! where to get it, nothing more. It does **not** download or apply anything. The
//! old prototype had an equivalent "Update Check" service; this ports that
//! check-only behavior. Actually *applying* an update automatically is a distinct,
//! much larger feature (the Tauri Updater Plugin, per the CI/CD design docs) that
//! genuinely requires the Tauri app shell to exist — there is no such shell yet (see
//! `crates/gtavmm-app`), so that half is out of scope here by necessity, not by
//! oversight.
//!
//! No network call happens unless the caller invokes [`check`] — this respects the
//! project's offline-first default (see `PRIVACY.md`): nothing here runs on its own.

use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

const RELEASES_API_URL: &str =
    "https://api.github.com/repos/SpaceSquare640/GTAV_Mods_Manager/releases/latest";

#[derive(Debug, Deserialize)]
struct GitHubRelease {
    tag_name: String,
    html_url: String,
    #[serde(default)]
    assets: Vec<GitHubReleaseAsset>,
}

#[derive(Debug, Deserialize)]
struct GitHubReleaseAsset {
    name: String,
    browser_download_url: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct UpdateCheckResult {
    pub current_version: String,
    pub latest_version: String,
    pub update_available: bool,
    pub release_url: String,
    /// Direct download URL for the asset matching the current platform, if the
    /// release has one — see [`asset_name_for_platform`]. `None` if no matching
    /// asset was found (e.g. release predates that platform's build, or naming
    /// changed) — callers should fall back to `release_url` in that case.
    pub platform_download_url: Option<String>,
}

/// The release asset name this platform's build is published under (see
/// `.github/workflows/release.yml`). Matching against a real release requires one to
/// have actually been tagged and built — this is exercised by tests against a mocked
/// payload, not yet against a live release, since none has been tagged so far.
fn asset_name_for_platform(os: &str) -> Option<&'static str> {
    match os {
        "windows" => Some("gtavmm-windows-x64.exe"),
        "linux" => Some("gtavmm-linux-x64"),
        _ => None,
    }
}

/// Queries the GitHub Releases API (unauthenticated, no data about you is sent) for
/// the latest published release and compares it against `current_version` (typically
/// `env!("CARGO_PKG_VERSION")`). Network/parse failures are returned as errors, not
/// panics — a failed update check should never crash the caller.
pub fn check(current_version: &str) -> CoreResult<UpdateCheckResult> {
    let response = ureq::get(RELEASES_API_URL)
        .set("User-Agent", "gtavmm-update-check")
        .call()
        .map_err(|e| CoreError::Network {
            reason: format!("could not reach GitHub Releases: {e}"),
        })?;

    let release: GitHubRelease = response.into_json().map_err(|e| CoreError::Network {
        reason: format!("could not parse GitHub Releases response: {e}"),
    })?;

    let latest_version = release.tag_name.trim_start_matches('v').to_string();
    let update_available = is_newer(&latest_version, current_version);
    let platform_download_url = find_platform_asset(&release.assets, std::env::consts::OS);

    Ok(UpdateCheckResult {
        current_version: current_version.to_string(),
        latest_version,
        update_available,
        release_url: release.html_url,
        platform_download_url,
    })
}

fn find_platform_asset(assets: &[GitHubReleaseAsset], os: &str) -> Option<String> {
    let expected_name = asset_name_for_platform(os)?;
    assets
        .iter()
        .find(|asset| asset.name == expected_name)
        .map(|asset| asset.browser_download_url.clone())
}

/// `true` if `candidate` (e.g. "1.2.0") is a newer version than `baseline` (e.g.
/// "1.1.5"), comparing dot-separated numeric segments left to right. Deliberately not
/// a full SemVer implementation (no pre-release/build-metadata handling) — this
/// project's version tags are plain `MAJOR.MINOR.PATCH`, and pulling in a SemVer
/// crate for three-segment numeric comparison isn't worth the dependency.
fn is_newer(candidate: &str, baseline: &str) -> bool {
    let parse = |s: &str| -> Vec<u64> { s.split('.').map(|p| p.parse().unwrap_or(0)).collect() };
    let candidate_parts = parse(candidate);
    let baseline_parts = parse(baseline);
    let len = candidate_parts.len().max(baseline_parts.len());

    for i in 0..len {
        let c = candidate_parts.get(i).copied().unwrap_or(0);
        let b = baseline_parts.get(i).copied().unwrap_or(0);
        if c != b {
            return c > b;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn detects_newer_patch_version() {
        assert!(is_newer("0.1.1", "0.1.0"));
        assert!(!is_newer("0.1.0", "0.1.1"));
    }

    #[test]
    fn detects_newer_minor_and_major_versions() {
        assert!(is_newer("0.2.0", "0.1.9"));
        assert!(is_newer("1.0.0", "0.9.9"));
    }

    #[test]
    fn identical_versions_are_not_newer() {
        assert!(!is_newer("1.0.0", "1.0.0"));
    }

    #[test]
    fn handles_mismatched_segment_counts() {
        assert!(is_newer("1.0.0.1", "1.0.0"));
        assert!(!is_newer("1.0", "1.0.0"));
    }

    fn sample_assets() -> Vec<GitHubReleaseAsset> {
        vec![
            GitHubReleaseAsset {
                name: "gtavmm-windows-x64.exe".to_string(),
                browser_download_url: "https://example.com/gtavmm-windows-x64.exe".to_string(),
            },
            GitHubReleaseAsset {
                name: "gtavmm-linux-x64".to_string(),
                browser_download_url: "https://example.com/gtavmm-linux-x64".to_string(),
            },
        ]
    }

    #[test]
    fn finds_matching_asset_for_known_platforms() {
        assert_eq!(
            find_platform_asset(&sample_assets(), "windows"),
            Some("https://example.com/gtavmm-windows-x64.exe".to_string())
        );
        assert_eq!(
            find_platform_asset(&sample_assets(), "linux"),
            Some("https://example.com/gtavmm-linux-x64".to_string())
        );
    }

    #[test]
    fn returns_none_for_unrecognized_platform_or_missing_asset() {
        assert_eq!(find_platform_asset(&sample_assets(), "macos"), None);
        assert_eq!(find_platform_asset(&[], "windows"), None);
    }
}
