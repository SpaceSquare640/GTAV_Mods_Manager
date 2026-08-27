// SPDX-License-Identifier: AGPL-3.0-only

//! Component Checker: detects whether the essential third-party dependencies
//! (ScriptHookV, ScriptHookVDotNet, OpenIV/OpenRPF) are present in a Legacy GTA V
//! installation, with official download links. This is the "known dependency rule
//! library v1" from the roadmap and the `mod_analyzer`-adjacent prerequisite checks
//! referenced by `classify_dll`/`classify_folder`'s doc comments — surfaced here as
//! its own read-only query rather than baked into the classifier, since it's a
//! healthcheck concern, not a file-classification one.
//!
//! We only detect *presence* (file exists), not version numbers — verifying that a
//! present file is a *working, compatible* version would require either running it
//! or parsing its own version resource, neither of which is worth the complexity for
//! a "is this even here" check. Official links are the trusted source of truth for
//! "is this current."

use std::path::Path;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Component {
    ScriptHookV,
    ScriptHookVDotNet,
    OpenIvOrOpenRpf,
}

impl Component {
    pub fn display_name(self) -> &'static str {
        match self {
            Component::ScriptHookV => "Script Hook V",
            Component::ScriptHookVDotNet => "Script Hook V .NET",
            Component::OpenIvOrOpenRpf => "OpenIV / OpenRPF",
        }
    }

    /// Official (author/maintainer) download page — not a mirror. Several
    /// lookalike domains exist for OpenIV in particular (`.co`, `.app`, `.org`
    /// clones of the real `openiv.com`); only the verified official source is
    /// linked here.
    pub fn official_download_url(self) -> &'static str {
        match self {
            Component::ScriptHookV => "https://dev-c.com/gtav/scripthookv",
            Component::ScriptHookVDotNet => {
                "https://github.com/scripthookvdotnet/scripthookvdotnet/releases"
            }
            Component::OpenIvOrOpenRpf => "https://openiv.com/",
        }
    }

    fn indicator_file_names(self) -> &'static [&'static str] {
        match self {
            // ScriptHookV ships as ScriptHookV.dll plus the dinput8.dll ASI-loader
            // proxy; either file's presence is a reasonable signal something from
            // this pair was installed.
            Component::ScriptHookV => &["ScriptHookV.dll", "dinput8.dll"],
            // SHVDN v2 uses ScriptHookVDotNet.asi + ScriptHookVDotNet2.dll; v3 uses
            // ScriptHookVDotNet3.dll. Any of these being present counts.
            Component::ScriptHookVDotNet => &[
                "ScriptHookVDotNet.asi",
                "ScriptHookVDotNet2.dll",
                "ScriptHookVDotNet3.dll",
            ],
            Component::OpenIvOrOpenRpf => &["OpenIV.asi"],
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ComponentStatus {
    pub component: Component,
    pub is_installed: bool,
}

fn is_present(game_root: &Path, component: Component) -> bool {
    component
        .indicator_file_names()
        .iter()
        .any(|name| game_root.join(name).is_file())
}

/// Checks every tracked component against `game_root` and returns their status, in a
/// fixed, stable order (suitable for direct display).
pub fn check_all(game_root: &Path) -> Vec<ComponentStatus> {
    [
        Component::ScriptHookV,
        Component::ScriptHookVDotNet,
        Component::OpenIvOrOpenRpf,
    ]
    .into_iter()
    .map(|component| ComponentStatus {
        component,
        is_installed: is_present(game_root, component),
    })
    .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_missing_when_no_indicator_files_present() {
        let dir = tempfile::tempdir().unwrap();
        let statuses = check_all(dir.path());
        assert_eq!(statuses.len(), 3);
        assert!(statuses.iter().all(|s| !s.is_installed));
    }

    #[test]
    fn detects_scripthookv_via_either_indicator_file() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ScriptHookV.dll"), b"").unwrap();
        let statuses = check_all(dir.path());
        let shv = statuses
            .iter()
            .find(|s| s.component == Component::ScriptHookV)
            .unwrap();
        assert!(shv.is_installed);
    }

    #[test]
    fn detects_scripthookvdotnet_v3_variant() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("ScriptHookVDotNet3.dll"), b"").unwrap();
        let statuses = check_all(dir.path());
        let shvdn = statuses
            .iter()
            .find(|s| s.component == Component::ScriptHookVDotNet)
            .unwrap();
        assert!(shvdn.is_installed);
    }

    #[test]
    fn detects_openiv_asi() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("OpenIV.asi"), b"").unwrap();
        let statuses = check_all(dir.path());
        let openiv = statuses
            .iter()
            .find(|s| s.component == Component::OpenIvOrOpenRpf)
            .unwrap();
        assert!(openiv.is_installed);
    }

    #[test]
    fn official_urls_point_to_the_real_source_not_lookalike_mirrors() {
        // Regression guard: OpenIV in particular has several clone domains
        // (.co/.app/.org) impersonating the real openiv.com — make sure a future
        // edit doesn't accidentally swap in one of those.
        assert_eq!(
            Component::OpenIvOrOpenRpf.official_download_url(),
            "https://openiv.com/"
        );
    }
}
