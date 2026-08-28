// SPDX-License-Identifier: AGPL-3.0-only

//! SP → FiveM Add-on/vehicle-pack converter (v0.7.x, scope confirmed to **vehicle/Add-on
//! packs only** — script mod conversion is a future, unscheduled extension).
//!
//! Reads a Single Player add-on vehicle mod's `dlc.rpf` directly (no external RPF
//! explorer needed — see [`rpf_archive`]) and reorganizes its contents into a FiveM
//! resource: `.meta` files under `data/`, streamed model/texture assets (found nested
//! inside the DLC's own `vehicles.rpf`, which [`rpf_archive::RpfFile::walk`] transparently
//! recurses into) flattened under `stream/`, plus a generated `fxmanifest.lua`.
//!
//! Deliberately does **not** try to be clever about which `.meta` files are "needed" —
//! it writes every one it finds (except `content.xml`/`setup2.xml`, which a real FiveM
//! resource has no use for) and the generated manifest uses glob patterns
//! (`data/**/*.meta`) so a missing file is simply not matched rather than a broken
//! explicit reference — this mirrors the community-standard "Add-on Car Globbing
//! Template" convention (glob-based `fxmanifest.lua`, unedited unless something's
//! genuinely missing), not something invented for this project.
//!
//! **Honesty note**: verified end-to-end against two real SP add-on vehicle packs'
//! `dlc.rpf` files during development (2026-08-28/29):
//! - A single-vehicle pack with a nested `vehicles.rpf` holding `.yft`/`.ytd` streamed
//!   assets, no `carcols.meta`.
//! - A second, larger (~390 MB) tuning-edition pack that **does** have `carcols.meta`
//!   (confirming that path is handled, not just assumed) and additionally has 12
//!   nested per-language `dlc.rpf` archives (`americandlc.rpf`, `chinesedlc.rpf`, …),
//!   each holding a `global.gxt2` localization file — [`rpf_archive`]'s recursion
//!   correctly walked into all of them, and `.gxt2` (not a `.meta` or a recognized
//!   streamed-asset extension) was correctly routed to `skipped_files` rather than
//!   misfiled or causing an error. GXT2 parsing/translation itself is **not**
//!   implemented — vehicle in-game display names from these files are simply not
//!   carried over.
//!
//! A third real mod (2026-08-29) added two more data points: its `dlc.rpf` sat at a
//! **non-ASCII path** (a Chinese-language mod's own folder naming) and round-tripped
//! correctly, and it had a `caraddoncontentunlocks.meta` file — a type not seen in the
//! first two mods, and not special-cased anywhere in this module. It was picked up
//! correctly anyway, confirming the "don't be clever about which `.meta` files are
//! needed, just take all of them" design decision (see above) actually pays off
//! against a real, previously-unanticipated file.
//!
//! Still not verified: a pack with **multiple vehicles in one DLC** (each mod tested
//! so far has exactly one), or one whose `vehicles.rpf` nests a *third* level of
//! archive.

use std::path::Path;

use rpf_archive::RpfFile;

use crate::error::{CoreError, CoreResult};

/// Files present in essentially every SP vehicle DLC's root that have no FiveM-side use.
const SKIP_FILES: &[&str] = &["content.xml", "setup2.xml"];

/// Streamed asset extensions FiveM will pick up from a resource's `stream/` folder
/// without any `data_file` declaration.
const STREAM_EXTENSIONS: &[&str] = &[
    "yft", "ytd", "ydr", "ydd", "yed", "ybn", "ynd", "ynv", "ycd", "yvr", "ymap", "ymt",
];

const FXMANIFEST_TEMPLATE: &str = "\
fx_version 'cerulean'
game { 'gta5' }

description 'Converted from an SP add-on vehicle pack by GTAV Mods Manager'

files {
    'data/**/*.meta'
}

data_file 'HANDLING_FILE' 'data/**/handling.meta'
data_file 'VEHICLE_LAYOUTS_FILE' 'data/**/vehiclelayouts.meta'
data_file 'VEHICLE_METADATA_FILE' 'data/**/vehicles.meta'
data_file 'CARCOLS_FILE' 'data/**/carcols.meta'
data_file 'VEHICLE_VARIATION_FILE' 'data/**/carvariations.meta'
";

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ConversionReport {
    pub data_files: Vec<String>,
    pub stream_files: Vec<String>,
    pub skipped_files: Vec<String>,
}

/// Converts a single SP add-on vehicle pack's `dlc.rpf` into a FiveM resource folder at
/// `output_dir` (created if it doesn't exist; existing contents are not cleared first —
/// caller's responsibility, matching this project's "never silently delete" stance).
pub fn convert_vehicle_pack(dlc_rpf: &Path, output_dir: &Path) -> CoreResult<ConversionReport> {
    let file = RpfFile::open(dlc_rpf, None).map_err(|e| CoreError::SpToFivem {
        reason: format!("could not open {}: {e}", dlc_rpf.display()),
    })?;

    let data_dir = output_dir.join("data");
    let stream_dir = output_dir.join("stream");
    std::fs::create_dir_all(&data_dir)?;
    std::fs::create_dir_all(&stream_dir)?;

    let mut report = ConversionReport::default();
    let mut walk_err: Option<CoreError> = None;

    file.walk(None, &mut |path, bytes| {
        if walk_err.is_some() {
            return;
        }
        let basename = path.rsplit('/').next().unwrap_or(path);
        let ext = basename.rsplit('.').next().unwrap_or("").to_lowercase();

        if SKIP_FILES.contains(&basename) {
            report.skipped_files.push(path.to_string());
            return;
        }

        let write_result = if ext == "meta" {
            let dest = data_dir.join(basename);
            std::fs::write(&dest, &bytes).map(|_| report.data_files.push(basename.to_string()))
        } else if STREAM_EXTENSIONS.contains(&ext.as_str()) {
            let dest = stream_dir.join(basename);
            std::fs::write(&dest, &bytes).map(|_| report.stream_files.push(basename.to_string()))
        } else {
            report.skipped_files.push(path.to_string());
            Ok(())
        };

        if let Err(e) = write_result {
            walk_err = Some(CoreError::Io(e));
        }
    })
    .map_err(|e| CoreError::SpToFivem {
        reason: format!("failed reading {}: {e}", dlc_rpf.display()),
    })?;

    if let Some(e) = walk_err {
        return Err(e);
    }

    std::fs::write(output_dir.join("fxmanifest.lua"), FXMANIFEST_TEMPLATE)?;

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rpf_archive::{RpfBuilder, RpfEncryption};

    /// Builds a minimal but realistic dlc.rpf: root-level meta files (as a real SP
    /// vehicle DLC has), plus a nested `vehicles.rpf` containing streamed assets —
    /// exercising the same nested-archive recursion this module relies on for real
    /// packs (confirmed separately against a real mod's dlc.rpf during development).
    fn build_test_dlc_rpf() -> Vec<u8> {
        let mut inner = RpfBuilder::new(RpfEncryption::None);
        inner.add_file("testcar.yft", b"fake-yft-bytes".to_vec());
        inner.add_file("testcar.ytd", b"fake-ytd-bytes".to_vec());
        let inner_bytes = inner.build(None).unwrap();

        let mut outer = RpfBuilder::new(RpfEncryption::None);
        outer.add_file("content.xml", b"<Item/>".to_vec());
        outer.add_file("setup2.xml", b"<Item/>".to_vec());
        outer.add_file("vehicles.meta", b"<CVehicleModelInfo__InitDataList/>".to_vec());
        outer.add_file("handling.meta", b"<CHandlingDataMgr/>".to_vec());
        outer.add_file("carvariations.meta", b"<CVehicleModelInfoVariation/>".to_vec());
        outer.add_file("vehicles.rpf", inner_bytes);
        outer.build(None).unwrap()
    }

    #[test]
    fn converts_meta_and_nested_stream_files_into_the_right_folders() {
        let dir = tempfile::tempdir().unwrap();
        let dlc_path = dir.path().join("dlc.rpf");
        std::fs::write(&dlc_path, build_test_dlc_rpf()).unwrap();
        let output_dir = dir.path().join("out");

        let report = convert_vehicle_pack(&dlc_path, &output_dir).unwrap();

        let mut data_files = report.data_files.clone();
        data_files.sort();
        assert_eq!(
            data_files,
            vec!["carvariations.meta", "handling.meta", "vehicles.meta"]
        );

        let mut stream_files = report.stream_files.clone();
        stream_files.sort();
        assert_eq!(stream_files, vec!["testcar.yft", "testcar.ytd"]);

        assert!(report.skipped_files.contains(&"content.xml".to_string()));
        assert!(report.skipped_files.contains(&"setup2.xml".to_string()));

        assert!(output_dir.join("data/vehicles.meta").exists());
        assert!(output_dir.join("data/handling.meta").exists());
        assert!(output_dir.join("data/carvariations.meta").exists());
        assert!(output_dir.join("stream/testcar.yft").exists());
        assert!(output_dir.join("stream/testcar.ytd").exists());
        assert!(!output_dir.join("data/content.xml").exists());
        assert!(output_dir.join("fxmanifest.lua").exists());

        let manifest = std::fs::read_to_string(output_dir.join("fxmanifest.lua")).unwrap();
        assert!(manifest.contains("fx_version 'cerulean'"));
        assert!(manifest.contains("data_file 'VEHICLE_METADATA_FILE' 'data/**/vehicles.meta'"));
    }

    #[test]
    fn stream_file_bytes_round_trip_unchanged() {
        let dir = tempfile::tempdir().unwrap();
        let dlc_path = dir.path().join("dlc.rpf");
        std::fs::write(&dlc_path, build_test_dlc_rpf()).unwrap();
        let output_dir = dir.path().join("out");

        convert_vehicle_pack(&dlc_path, &output_dir).unwrap();

        assert_eq!(
            std::fs::read(output_dir.join("stream/testcar.yft")).unwrap(),
            b"fake-yft-bytes"
        );
    }

    /// Mirrors the second real mod verified during development (2026-08-29): a
    /// `carcols.meta` at the root, plus 12 nested per-language `dlc.rpf` archives
    /// (here reduced to two) each holding an unsupported `.gxt2` localization file.
    #[test]
    fn handles_carcols_meta_and_nested_localization_archives_like_a_real_tuning_pack() {
        let mut american_lang_rpf = RpfBuilder::new(RpfEncryption::None);
        american_lang_rpf.add_file("global.gxt2", b"fake-gxt2-bytes".to_vec());
        let american_lang_bytes = american_lang_rpf.build(None).unwrap();

        let mut chinese_lang_rpf = RpfBuilder::new(RpfEncryption::None);
        chinese_lang_rpf.add_file("global.gxt2", b"fake-gxt2-bytes-zh".to_vec());
        let chinese_lang_bytes = chinese_lang_rpf.build(None).unwrap();

        let mut inner = RpfBuilder::new(RpfEncryption::None);
        inner.add_file("sv15.yft", b"fake-yft-bytes".to_vec());
        inner.add_file("sv15.ytd", b"fake-ytd-bytes".to_vec());
        let inner_bytes = inner.build(None).unwrap();

        let mut outer = RpfBuilder::new(RpfEncryption::None);
        outer.add_file("content.xml", b"<Item/>".to_vec());
        outer.add_file("vehicles.meta", b"<CVehicleModelInfo__InitDataList/>".to_vec());
        outer.add_file("handling.meta", b"<CHandlingDataMgr/>".to_vec());
        outer.add_file("carvariations.meta", b"<CVehicleModelInfoVariation/>".to_vec());
        outer.add_file("carcols.meta", b"<CVehicleModColors/>".to_vec());
        outer.add_file("vehicles.rpf", inner_bytes);
        outer.add_file("americandlc.rpf", american_lang_bytes);
        outer.add_file("chinesedlc.rpf", chinese_lang_bytes);
        let dlc_bytes = outer.build(None).unwrap();

        let dir = tempfile::tempdir().unwrap();
        let dlc_path = dir.path().join("dlc.rpf");
        std::fs::write(&dlc_path, dlc_bytes).unwrap();
        let output_dir = dir.path().join("out");

        let report = convert_vehicle_pack(&dlc_path, &output_dir).unwrap();

        let mut data_files = report.data_files.clone();
        data_files.sort();
        assert_eq!(
            data_files,
            vec!["carcols.meta", "carvariations.meta", "handling.meta", "vehicles.meta"]
        );
        assert!(output_dir.join("data/carcols.meta").exists());

        let mut stream_files = report.stream_files.clone();
        stream_files.sort();
        assert_eq!(stream_files, vec!["sv15.yft", "sv15.ytd"]);

        // .gxt2 isn't a supported extension — must be routed to skipped_files, not
        // silently dropped or misfiled into data/ or stream/.
        assert_eq!(
            report
                .skipped_files
                .iter()
                .filter(|f| f.ends_with("global.gxt2"))
                .count(),
            2,
            "both nested language archives' global.gxt2 must be seen and skipped: {:?}",
            report.skipped_files
        );
        assert!(!output_dir.join("data/global.gxt2").exists());
        assert!(!output_dir.join("stream/global.gxt2").exists());
    }

    /// Mirrors the third real mod verified during development (2026-08-29): a
    /// non-ASCII source path, and a previously-unseen `caraddoncontentunlocks.meta`
    /// file that isn't special-cased anywhere — must still be picked up correctly.
    #[test]
    fn handles_a_non_ascii_source_path_and_an_unrecognized_meta_file_type() {
        let mut inner = RpfBuilder::new(RpfEncryption::None);
        inner.add_file("nsczinger21c.yft", b"fake-yft-bytes".to_vec());
        inner.add_file("nsczinger21c.ytd", b"fake-ytd-bytes".to_vec());
        let inner_bytes = inner.build(None).unwrap();

        let mut outer = RpfBuilder::new(RpfEncryption::None);
        outer.add_file("vehicles.meta", b"<CVehicleModelInfo__InitDataList/>".to_vec());
        outer.add_file("handling.meta", b"<CHandlingDataMgr/>".to_vec());
        outer.add_file(
            "caraddoncontentunlocks.meta",
            b"<SContentUnlocks><listOfUnlocks><Item>CU_VEH_NSCZINGER21C</Item></listOfUnlocks></SContentUnlocks>".to_vec(),
        );
        outer.add_file("vehicles.rpf", inner_bytes);
        let dlc_bytes = outer.build(None).unwrap();

        // A Chinese-language folder name, matching the real mod's own path shape.
        let dir = tempfile::tempdir().unwrap();
        let source_dir = dir.path().join("添加式（Add-on）").join("nsczinger21c");
        std::fs::create_dir_all(&source_dir).unwrap();
        let dlc_path = source_dir.join("dlc.rpf");
        std::fs::write(&dlc_path, dlc_bytes).unwrap();
        let output_dir = dir.path().join("out");

        let report = convert_vehicle_pack(&dlc_path, &output_dir).unwrap();

        assert!(report.data_files.contains(&"caraddoncontentunlocks.meta".to_string()));
        assert!(output_dir.join("data/caraddoncontentunlocks.meta").exists());
        assert_eq!(
            std::fs::read_to_string(output_dir.join("data/caraddoncontentunlocks.meta")).unwrap(),
            "<SContentUnlocks><listOfUnlocks><Item>CU_VEH_NSCZINGER21C</Item></listOfUnlocks></SContentUnlocks>"
        );
    }

    #[test]
    fn errors_cleanly_on_a_missing_dlc_rpf() {
        let dir = tempfile::tempdir().unwrap();
        let err = convert_vehicle_pack(&dir.path().join("nope.rpf"), &dir.path().join("out"))
            .unwrap_err();
        assert!(matches!(err, CoreError::SpToFivem { .. }));
    }
}
