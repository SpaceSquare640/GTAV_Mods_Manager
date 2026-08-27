// SPDX-License-Identifier: AGPL-3.0-only

//! `ModeProvider`: the abstraction that lets later modes (Enhanced SP, Legacy/Enhanced
//! LSPDFR, FiveM) be added without rewriting `mod_analyzer`'s dispatch logic. Each
//! mode's directory layout (where a `.dll` script goes, what `mods\` mirroring looks
//! like, etc.) lives entirely inside its own `ModeProvider` implementation — the
//! classifier only decides *what kind* of file something is and asks the active
//! provider *where it goes*.
//!
//! Implemented so far: [`LegacySpProvider`], [`EnhancedSpProvider`],
//! [`LegacyLspdfrProvider`], [`EnhancedLspdfrProvider`], [`FiveMClientProvider`]. The
//! FiveM server side (`resources\` dependency-order resolution) doesn't fit this
//! trait's per-file target-path shape at all — see `crate::fivem` instead.

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

/// Enhanced SP mode provider.
///
/// **Verified against a real Enhanced install** (inspected directly on this
/// machine): the executable/anti-cheat layout differs from Legacy (see
/// `protected_files` and `game_locator::classify_edition`), but no `mods\` folder
/// exists yet on that install to confirm loose-file-override conventions against.
///
/// **Assumption, not yet verified**: `.asi`/native+managed `.dll`/Menyoo XML/folder
/// mirroring are assumed to follow the same OpenIV-OpenRPF loose-override conventions
/// as Legacy, since Enhanced still ships ScriptHookV-compatible loaders as of this
/// writing. This is a reasonable low-risk default, not a confirmed fact.
///
/// **Correction (2026-08-27)**: an earlier version of this comment claimed Enhanced
/// uniquely ships both `update/update.rpf` and `update/update2.rpf`, "unlike Legacy's
/// single `update.rpf`". That claim was never actually verified and turned out to be
/// **false**: inspecting a real Legacy install's `update\` folder (and a real, heavily
/// modded Legacy install's mirrored `mods\update\`) on this machine shows Legacy *also*
/// has both `update.rpf` and `update2.rpf`. The Legacy-vs-Enhanced RPF split is not the
/// relevant unknown here — see below.
///
/// **Still genuinely uncertain, left unresolved**: which RPF an add-on pack's
/// `dlclist.xml` entry actually needs to reference internally is a fact about RPF
/// *contents*, not about which `.rpf` files exist on disk — confirming it needs
/// RPF-inspection tooling (OpenIV/CodeWalker), which isn't available in this
/// environment, on an Enhanced install specifically (the Legacy convention this
/// provider reuses is well-established in the community; Enhanced's equivalent is not
/// yet confirmed here). Treat Enhanced add-on-pack installs as unverified until
/// checked against a real install with RPF tooling.
pub struct EnhancedSpProvider {
    game_root: PathBuf,
}

impl EnhancedSpProvider {
    pub fn new(game_root: PathBuf) -> Self {
        Self { game_root }
    }
}

impl ModeProvider for EnhancedSpProvider {
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

/// The folder RAGE Plugin Hook (RPH) — the hook framework LSPDFR is built on, distinct
/// from ScriptHookV — loads its managed plugin DLLs from.
///
/// **Verified 2026-08-27** against a real, heavily-modded LSPDFR install (a full
/// "HUNTER" police modpack backup, inspected directly on this machine): a real
/// `RAGEPluginHook.exe`-launched install has a top-level `Plugins\` folder containing
/// both the core `LSPD First Response.dll` plugin itself and a mix of other RPH
/// plugins (EUP menu, body-cam, seatbelt, etc.) as flat `.dll` files — confirming this
/// constant. See [`resolve_lspdfr_managed_dll_target`] for the one nuance that
/// inspection also turned up.
pub(crate) const RPH_PLUGINS_SUBFOLDER: &str = "Plugins";

/// Shared LSPDFR target-path logic for both editions. LSPDFR's own mod ecosystem
/// (callouts, EUP uniform packs, vehicle packs) is treated generically here rather
/// than with dedicated per-content-type resolvers, consistent with the project's
/// decision not to build format-specific intelligence beyond what `mod_analyzer`
/// already classifies:
///
/// - Managed `.dll` → RPH's `Plugins\` folder (see [`RPH_PLUGINS_SUBFOLDER`]),
///   replacing SP mode's `scripts\` (ScriptHookVDotNet) target. **Verified against a
///   real install** for the general/majority case (most `Plugins\`-level `.dll`s in
///   the real modpack inspected were *not* callouts — the core plugin and other
///   generic RPH plugins). **One known refinement not implemented**: that same real
///   install also has a `Plugins\LSPDFR\` subfolder specifically holding callout
///   *expansion pack* DLLs (e.g. community callout packs), distinct from bare
///   `Plugins\`. `mod_analyzer` has no way to distinguish "a callout pack" from "a
///   generic RPH plugin" — both are just a managed `.dll` from a PE-inspection
///   standpoint — so this provider cannot route the two differently without a new,
///   more specific classification. Routing everything to bare `Plugins\` was checked
///   against the real sample and is not wrong, just occasionally coarser than the
///   community convention for callout-specific packs.
/// - Folder replacers (e.g. EUP/ped-pack file overrides) and add-on vehicle/map packs
///   reuse the same OpenIV-OpenRPF `mods\` mirroring and `dlclist.xml` registration as
///   the SP providers — **verified against the same real install**, which has its own
///   `mods\update\x64\dlcpacks\` full of real add-on vehicle packs laid out exactly
///   this way.
/// - `.asi`/native `.dll`/Menyoo are uncommon in the LSPDFR ecosystem (which doesn't
///   use ScriptHookV) but are still resolved to sane SP-equivalent defaults in case a
///   hybrid install mixes in a ScriptHookV-based mod alongside LSPDFR — and indeed the
///   real install inspected does mix in several ScriptHookV `.asi` mods (ELS, PLD,
///   Menyoo) alongside RPH/LSPDFR, confirming this coexistence is real, not just a
///   hypothetical edge case.
///
/// **Still explicitly out of scope / unverified**: EUP's actual per-faction/per-state
/// ped-model folder organization is not modeled — the real install's EUP menu plugin
/// keeps its own settings in `Plugins\EUP\` (a flat set of `.ini` files, not modeled
/// either), while the actual clothing/ped assets are installed as ordinary `mods\`
/// replacers. Treating EUP content packs as plain folder replacers is consistent with
/// what was observed, but this project has not attempted to install a real EUP content
/// pack end-to-end to confirm the boundary.
fn resolve_lspdfr_managed_dll_target(game_root: &Path, file_name: &OsStr) -> PathBuf {
    game_root.join(RPH_PLUGINS_SUBFOLDER).join(file_name)
}

pub struct LegacyLspdfrProvider {
    game_root: PathBuf,
}

impl LegacyLspdfrProvider {
    pub fn new(game_root: PathBuf) -> Self {
        Self { game_root }
    }
}

impl ModeProvider for LegacyLspdfrProvider {
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
        resolve_lspdfr_managed_dll_target(&self.game_root, file_name)
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

/// Enhanced LSPDFR provider. **Beta-grade even relative to `EnhancedSpProvider`'s
/// already-unverified add-on-pack path**: RPH/LSPDFR's Enhanced-edition support only
/// entered Public Preview in 2026-04 (per the competitive-analysis notes), meaning the
/// community conventions this provider assumes are for a moving target, not a settled
/// one. Reuses `EnhancedSpProvider`'s `mods\`/dlclist assumptions plus the same
/// RPH `Plugins\` convention as `LegacyLspdfrProvider` — treat every target this
/// resolves as more likely to need revision than any other provider in this module.
pub struct EnhancedLspdfrProvider {
    game_root: PathBuf,
}

impl EnhancedLspdfrProvider {
    pub fn new(game_root: PathBuf) -> Self {
        Self { game_root }
    }
}

impl ModeProvider for EnhancedLspdfrProvider {
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
        resolve_lspdfr_managed_dll_target(&self.game_root, file_name)
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

/// FiveM client-side asset mod provider. `game_root()` here is the FiveM client
/// installation folder, not a GTA V install — FiveM ships its own copy of the game
/// assets and is a fully separate application from `GTA5.exe`/`GTA5_Enhanced.exe`, so
/// unlike every other provider in this module there is no `game_locator` detection
/// path for it: callers must always supply the FiveM install folder explicitly.
///
/// **Unverified assumption**: no real FiveM client install exists on this machine to
/// confirm against (same caveat as the LSPDFR providers). FiveM's client-side asset
/// replacement (streamed textures/models/etc.) is widely documented as following the
/// same OpenIV-OpenRPF `mods\`-mirroring convention as regular SP mods, since FiveM's
/// client streams assets from a structure compatible with it — this provider reuses
/// that convention on that basis, not on direct verification.
pub struct FiveMClientProvider {
    game_root: PathBuf,
}

impl FiveMClientProvider {
    pub fn new(game_root: PathBuf) -> Self {
        Self { game_root }
    }
}

impl ModeProvider for FiveMClientProvider {
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

    #[test]
    fn legacy_lspdfr_routes_managed_dll_to_rph_plugins_folder() {
        let game_root = PathBuf::from("game");
        let provider = LegacyLspdfrProvider::new(game_root.clone());

        assert_eq!(
            provider.resolve_managed_dll_target(OsStr::new("SomeCallout.dll")),
            game_root.join("Plugins").join("SomeCallout.dll")
        );
        // Folder replacers and add-on packs are unaffected — same OpenIV mirroring
        // convention as the SP providers.
        assert_eq!(
            provider.resolve_add_on_pack_target("PoliceCar", Path::new("dlc.rpf")),
            game_root
                .join("mods")
                .join("update")
                .join("x64")
                .join("dlcpacks")
                .join("PoliceCar")
                .join("dlc.rpf")
        );
    }

    #[test]
    fn enhanced_lspdfr_routes_managed_dll_to_rph_plugins_folder() {
        let game_root = PathBuf::from("game");
        let provider = EnhancedLspdfrProvider::new(game_root.clone());

        assert_eq!(
            provider.resolve_managed_dll_target(OsStr::new("SomeCallout.dll")),
            game_root.join("Plugins").join("SomeCallout.dll")
        );
    }
}
