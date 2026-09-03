// SPDX-License-Identifier: AGPL-3.0-only

//! Guessing which LSPDFR category a mod falls into, from the files it installs.
//!
//! This is a heuristic and is treated as one: the guess is stored, shown, and
//! correctable by hand. LSPDFR packs carry no manifest declaring what they are,
//! so the only evidence available is where their files land — which is usually
//! but not always enough.

use gtavmm_core::mod_analyzer::ModPlan;

/// Categories the LSPDFR pages filter by, plus `framework` for the RPH stack
/// itself, which is shown in the table but is not one of the filter chips.
pub fn infer(plan: &ModPlan) -> &'static str {
    let paths: Vec<String> = plan
        .files
        .iter()
        .map(|f| f.target.to_string_lossy().replace('\\', "/").to_lowercase())
        .collect();
    let any = |pred: &dyn Fn(&str) -> bool| paths.iter().any(|p| pred(p));

    // Checked before callouts: LSPDFR itself also drops a .dll into Plugins\,
    // so the framework would otherwise be filed as a callout pack.
    if any(&|p| {
        p.contains("lspdfr.dll") || p.contains("ragepluginhook") || p.contains("ragenativeui")
    }) {
        return "framework";
    }
    if any(&|p| p.contains("eup") || p.contains("/peds/") || p.contains("ped_")) {
        return "eup-peds";
    }
    if any(&|p| p.ends_with(".rpf") || p.contains("vehicles.meta") || p.contains("carcols.meta")) {
        return "vehicles";
    }
    // A managed assembly under Plugins\LSPDFR\ is the callout convention.
    if any(&|p| p.contains("plugins/lspdfr/") && p.ends_with(".dll")) {
        return "callouts";
    }
    "other"
}

#[cfg(test)]
mod tests {
    use super::*;
    use gtavmm_core::mod_analyzer::{ModFormat, ModPlan, PlannedFile};
    use std::path::PathBuf;

    fn plan(targets: &[&str]) -> ModPlan {
        ModPlan {
            format: ModFormat::FolderReplacer,
            files: targets
                .iter()
                .map(|t| PlannedFile {
                    source: PathBuf::from("src"),
                    target: PathBuf::from(t),
                })
                .collect(),
        }
    }

    #[test]
    fn a_managed_assembly_under_plugins_lspdfr_is_a_callout() {
        assert_eq!(
            infer(&plan([r"Plugins\LSPDFR\BetterChases.dll"].as_ref())),
            "callouts"
        );
    }

    #[test]
    fn the_framework_wins_over_callouts() {
        // LSPDFR itself also lands in Plugins\ as a .dll, so order matters.
        assert_eq!(
            infer(&plan(
                [r"Plugins\LSPDFR.dll", r"Plugins\LSPDFR\x.dll"].as_ref()
            )),
            "framework"
        );
    }

    #[test]
    fn vehicle_and_ped_packs_are_told_apart() {
        assert_eq!(infer(&plan([r"mods\x\vehicles.meta"].as_ref())), "vehicles");
        assert_eq!(infer(&plan([r"mods\EUP\uniform.ymt"].as_ref())), "eup-peds");
    }

    #[test]
    fn anything_unrecognised_falls_to_other_rather_than_a_confident_guess() {
        assert_eq!(infer(&plan([r"scripts\thing.ini"].as_ref())), "other");
    }
}
