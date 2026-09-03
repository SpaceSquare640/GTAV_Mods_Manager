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

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub enum Component {
    ScriptHookV,
    ScriptHookVDotNet,
    OpenIvOrOpenRpf,
    /// The LSPDFR framework layer, below.
    RagePluginHook,
    LspdFirstResponse,
    RageNativeUi,
}

/// Which panel a component belongs to.
///
/// The SP pages show what a script mod needs; the LSPDFR pages show the RPH
/// stack instead. Listing all six on both would ask an SP user about plugins
/// they have no reason to install.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComponentSet {
    /// ScriptHookV / SHVDN / OpenIV — the SP "Components" panel.
    ScriptMods,
    /// RPH / LSPDFR / RAGENativeUI — the LSPDFR "Framework" panel.
    LspdfrFramework,
}

impl Component {
    pub fn display_name(self) -> &'static str {
        match self {
            Component::ScriptHookV => "Script Hook V",
            Component::ScriptHookVDotNet => "Script Hook V .NET",
            Component::OpenIvOrOpenRpf => "OpenIV / OpenRPF",
            Component::RagePluginHook => "RAGE Plugin Hook",
            Component::LspdFirstResponse => "LSPD First Response",
            Component::RageNativeUi => "RAGENativeUI",
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
            // Each of these three was opened and confirmed to be the
            // maintainer's own page, not a mirror or a reupload.
            Component::RagePluginHook => "https://ragepluginhook.net/",
            Component::LspdFirstResponse => {
                "https://www.lcpdfr.com/downloads/gta5mods/scripts/7792-lspd-first-response/"
            }
            Component::RageNativeUi => "https://github.com/alexguirre/RAGENativeUI/releases",
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
            // RPH is its own launcher executable in the game root; the .dll is
            // what plugins link against and ships beside it.
            Component::RagePluginHook => &["RAGEPluginHook.exe", "RAGEPluginHook.dll"],
            // LSPDFR and its plugins live a level down, under Plugins\LSPDFR\.
            // Hence the relative paths — checking only the game root would
            // report every LSPDFR component missing on a working install.
            Component::LspdFirstResponse => &["Plugins/LSPDFR.dll", "Plugins/LSPDFR"],
            Component::RageNativeUi => &["RAGENativeUI.dll", "Plugins/RAGENativeUI.dll"],
        }
    }

    /// Which panel lists this component.
    pub fn set(self) -> ComponentSet {
        match self {
            Component::RagePluginHook | Component::LspdFirstResponse | Component::RageNativeUi => {
                ComponentSet::LspdfrFramework
            }
            _ => ComponentSet::ScriptMods,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct ComponentStatus {
    pub component: Component,
    pub is_installed: bool,
    /// Denormalized onto the row (rather than making the frontend re-derive them from
    /// `component`) since this struct only ever exists to be displayed.
    pub display_name: String,
    pub official_download_url: String,
}

/// An indicator may name a file or a directory, and may sit in a subfolder.
///
/// The original three all lived as files in the game root. LSPDFR does not:
/// its plugins are under `Plugins\`, and its own presence is best evidenced by
/// that folder existing. Both relaxations are needed for the same reason —
/// otherwise a working LSPDFR install reports every component missing.
fn is_present(game_root: &Path, component: Component) -> bool {
    component.indicator_file_names().iter().any(|name| {
        let path = name
            .split('/')
            .fold(game_root.to_path_buf(), |acc, part| acc.join(part));
        path.exists()
    })
}

/// Checks the script-mod components. Kept as-is so existing callers are unaffected.
pub fn check_all(game_root: &Path) -> Vec<ComponentStatus> {
    check_set(game_root, ComponentSet::ScriptMods)
}

/// Checks one panel's components against `game_root`, in a fixed, stable order
/// (suitable for direct display).
pub fn check_set(game_root: &Path, set: ComponentSet) -> Vec<ComponentStatus> {
    [
        Component::ScriptHookV,
        Component::ScriptHookVDotNet,
        Component::OpenIvOrOpenRpf,
        Component::RagePluginHook,
        Component::LspdFirstResponse,
        Component::RageNativeUi,
    ]
    .into_iter()
    .filter(|c| c.set() == set)
    .map(|component| ComponentStatus {
        component,
        is_installed: is_present(game_root, component),
        display_name: component.display_name().to_string(),
        official_download_url: component.official_download_url().to_string(),
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
    fn the_framework_panel_is_separate_from_the_script_mod_panel() {
        let dir = tempfile::tempdir().unwrap();
        let fw = check_set(dir.path(), ComponentSet::LspdfrFramework);
        assert_eq!(fw.len(), 3);
        assert_eq!(fw[0].component, Component::RagePluginHook);
        // check_all must keep listing exactly the original three, or the SP
        // pages would start asking about plugins they have no use for.
        assert_eq!(check_all(dir.path()).len(), 3);
    }

    #[test]
    fn detects_lspdfr_by_its_plugins_subfolder_not_the_game_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dir.path().join("Plugins").join("LSPDFR")).unwrap();
        let fw = check_set(dir.path(), ComponentSet::LspdfrFramework);
        let lspdfr = fw
            .iter()
            .find(|s| s.component == Component::LspdFirstResponse)
            .unwrap();
        assert!(
            lspdfr.is_installed,
            "a real install puts LSPDFR under a Plugins subfolder, not in the game root"
        );
    }

    #[test]
    fn detects_rage_plugin_hook_in_the_game_root() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("RAGEPluginHook.exe"), b"").unwrap();
        assert!(
            check_set(dir.path(), ComponentSet::LspdfrFramework)
                .iter()
                .find(|s| s.component == Component::RagePluginHook)
                .unwrap()
                .is_installed
        );
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
