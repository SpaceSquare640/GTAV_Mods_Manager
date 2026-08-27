// SPDX-License-Identifier: AGPL-3.0-only

//! Classifies a mod package (file/folder/archive) into a plan of `(source, target)`
//! file mappings, per the format table in the MVP spec: `.asi`/`.dll` (native and
//! managed)/`.xml` (Menyoo)/folder replacers/`.zip`/`.7z` fully supported; simple
//! non-RPF `.oiv` supported via `assembly.xml` parsing; RPF-internal-edit `.oiv` and
//! `.rar` explicitly reported as unsupported (no silent partial handling).

pub mod dlc;
mod menyoo;
mod oiv;
mod pe;

use std::path::{Path, PathBuf};

use crate::error::{CoreError, CoreResult};

pub use menyoo::{MenyooCategory, MENYOO_ROOT_FOLDER};
pub use oiv::OivPlan;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModFormat {
    Asi,
    NativeDll,
    ManagedDll,
    MenyooXml,
    FolderReplacer,
    /// A standalone add-on content pack (add-on vehicle, add-on map, etc. — anything
    /// shipped as its own `dlc.rpf` rather than replacing an existing file). Carries
    /// the pack name that must be registered in `dlclist.xml` at install time and
    /// unregistered at uninstall time (see `dlc` module; not done by the classifier
    /// itself — the install pipeline, milestone 4, owns that side effect).
    AddOnPack {
        pack_name: String,
    },
    Zip,
    SevenZip,
    OivSimple,
    Unsupported(String),
}

#[derive(Debug, Clone)]
pub struct PlannedFile {
    pub source: PathBuf,
    pub target: PathBuf,
}

#[derive(Debug, Clone)]
pub struct ModPlan {
    pub format: ModFormat,
    pub files: Vec<PlannedFile>,
}

/// `scripts\` subfolder name for ScriptHookVDotNet managed assemblies. Re-exported
/// (`pub(crate)`) so `providers::LegacySpProvider` can share the same constant rather
/// than duplicating the literal.
pub(crate) const SCRIPTS_SUBFOLDER: &str = "scripts";
/// `mods\` subfolder name for folder-based replacer mods (per the OpenIV.asi/OpenRPF
/// loose-file-override convention, see the MVP spec's format table).
pub(crate) const MODS_SUBFOLDER: &str = "mods";

/// Classifies `input` (a file or folder path) and produces a `ModPlan`. Target paths
/// are resolved by delegating to `provider` — this function only decides *what kind*
/// of mod `input` is (by extension/content sniffing) and *what relative files it
/// contains*; it never hardcodes a mode's directory layout itself. That's what makes
/// adding a new mode (Enhanced SP, LSPDFR, FiveM) additive: implement a new
/// `ModeProvider`, and none of the dispatch logic here needs to change.
///
/// Archives (`.zip`/`.7z`) are extracted into a temp directory and then recursed into
/// as a folder; the returned plan's `source` paths point into that temp directory, so
/// callers must copy files out of it before it (or its `TempDir` guard) is dropped.
pub fn classify(
    input: &Path,
    provider: &dyn crate::providers::ModeProvider,
) -> CoreResult<ModPlan> {
    let extension = input
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase());

    match extension.as_deref() {
        _ if input.is_dir() => classify_folder(input, provider),
        Some("asi") => {
            let name = input.file_name().unwrap_or_default();
            Ok(single_file_plan(
                input,
                provider.resolve_asi_target(name),
                ModFormat::Asi,
            ))
        }
        Some("dll") => classify_dll(input, provider),
        Some("xml") => classify_menyoo_xml(input, provider),
        Some("zip") => classify_zip(input, provider),
        Some("7z") => classify_seven_zip(input, provider),
        Some("oiv") => classify_oiv(input, provider),
        Some("rar") => Ok(unsupported(
            "rar",
            "RAR archives are not supported (no pure-Rust decoder available). Please \
             extract this mod manually and point the manager at the extracted folder.",
        )),
        Some(other) => Ok(unsupported(
            other,
            &format!("Unrecognized mod file type: .{other}"),
        )),
        None => Err(CoreError::UnsupportedFormat {
            reason: "input has no file extension and is not a directory".to_string(),
        }),
    }
}

fn unsupported(tag: &str, message: &str) -> ModPlan {
    ModPlan {
        format: ModFormat::Unsupported(format!("{tag}: {message}")),
        files: Vec::new(),
    }
}

fn single_file_plan(source: &Path, target: PathBuf, format: ModFormat) -> ModPlan {
    ModPlan {
        format,
        files: vec![PlannedFile {
            source: source.to_path_buf(),
            target,
        }],
    }
}

fn classify_dll(
    input: &Path,
    provider: &dyn crate::providers::ModeProvider,
) -> CoreResult<ModPlan> {
    let name = input.file_name().unwrap_or_default();
    if pe::is_managed_assembly(input) {
        Ok(single_file_plan(
            input,
            provider.resolve_managed_dll_target(name),
            ModFormat::ManagedDll,
        ))
    } else {
        Ok(single_file_plan(
            input,
            provider.resolve_native_dll_target(name),
            ModFormat::NativeDll,
        ))
    }
}

fn classify_menyoo_xml(
    input: &Path,
    provider: &dyn crate::providers::ModeProvider,
) -> CoreResult<ModPlan> {
    let category = menyoo::detect_category(input);
    let name = input.file_name().unwrap_or_default();
    let target = provider.resolve_menyoo_target(category, name);
    Ok(single_file_plan(input, target, ModFormat::MenyooXml))
}

/// Mirrors a replacer-mod folder's structure via `provider.resolve_folder_replacer_target`
/// (the OpenIV.asi/OpenRPF loose-file-override convention — see the MVP spec;
/// verifying that prerequisite is out of scope for the classifier itself). Add-on
/// packs (folders containing a standalone `dlc.rpf`, per the `dlc` module) are
/// detected and routed to `resolve_add_on_pack_target` instead of a raw structural
/// mirror.
fn classify_folder(
    input: &Path,
    provider: &dyn crate::providers::ModeProvider,
) -> CoreResult<ModPlan> {
    if let Some(pack_dir) = dlc::find_dlc_pack_dir(input) {
        return classify_add_on_pack(&pack_dir, provider);
    }

    let files = walkdir::WalkDir::new(input)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|entry| {
            let relative = entry.path().strip_prefix(input).unwrap_or(entry.path());
            PlannedFile {
                source: entry.path().to_path_buf(),
                target: provider.resolve_folder_replacer_target(relative),
            }
        })
        .collect();

    Ok(ModPlan {
        format: ModFormat::FolderReplacer,
        files,
    })
}

fn classify_add_on_pack(
    pack_dir: &Path,
    provider: &dyn crate::providers::ModeProvider,
) -> CoreResult<ModPlan> {
    let name = dlc::pack_name(pack_dir)?;

    let files = walkdir::WalkDir::new(pack_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
        .map(|entry| {
            let relative = entry.path().strip_prefix(pack_dir).unwrap_or(entry.path());
            PlannedFile {
                source: entry.path().to_path_buf(),
                target: provider.resolve_add_on_pack_target(&name, relative),
            }
        })
        .collect();

    Ok(ModPlan {
        format: ModFormat::AddOnPack { pack_name: name },
        files,
    })
}

fn extract_zip(input: &Path) -> CoreResult<tempfile::TempDir> {
    let temp_dir = tempfile::tempdir()?;
    let file = std::fs::File::open(input)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| CoreError::UnsupportedFormat {
        reason: format!("not a valid zip archive: {e}"),
    })?;
    archive
        .extract(temp_dir.path())
        .map_err(|e| CoreError::UnsupportedFormat {
            reason: format!("failed to extract zip archive: {e}"),
        })?;
    Ok(temp_dir)
}

fn classify_zip(
    input: &Path,
    provider: &dyn crate::providers::ModeProvider,
) -> CoreResult<ModPlan> {
    let temp_dir = extract_zip(input)?;
    let mut plan = classify_folder(temp_dir.path(), provider)?;
    plan.format = ModFormat::Zip;
    // Deliberately leak the TempDir: its contents are the `source` paths in `plan`,
    // which the install pipeline (milestone 4) will copy from before this directory
    // would otherwise be cleaned up. Milestone 4 should own an explicit cleanup step
    // once files are copied out, rather than relying on TempDir's Drop here.
    std::mem::forget(temp_dir);
    Ok(plan)
}

fn classify_seven_zip(
    input: &Path,
    provider: &dyn crate::providers::ModeProvider,
) -> CoreResult<ModPlan> {
    let temp_dir = tempfile::tempdir()?;
    sevenz_rust::decompress_file(input, temp_dir.path()).map_err(|e| {
        CoreError::UnsupportedFormat {
            reason: format!("failed to extract 7z archive: {e}"),
        }
    })?;
    let mut plan = classify_folder(temp_dir.path(), provider)?;
    plan.format = ModFormat::SevenZip;
    std::mem::forget(temp_dir); // see classify_zip's note
    Ok(plan)
}

fn classify_oiv(
    input: &Path,
    provider: &dyn crate::providers::ModeProvider,
) -> CoreResult<ModPlan> {
    match oiv::analyze(input)? {
        OivPlan::Supported(entries) => {
            // Re-extract the .oiv (a zip container) so `input` paths inside the
            // manifest become real files we can copy from.
            let temp_dir = extract_zip(input)?;
            let files = entries
                .into_iter()
                .map(|entry| PlannedFile {
                    source: temp_dir.path().join(&entry.input),
                    target: provider.resolve_oiv_target(Path::new(&entry.output)),
                })
                .collect();
            std::mem::forget(temp_dir); // see classify_zip's note
            Ok(ModPlan {
                format: ModFormat::OivSimple,
                files,
            })
        }
        OivPlan::Unsupported => Ok(unsupported(
            "oiv",
            "This .oiv package requires RPF archive editing, which isn't supported \
             yet. Please install it with OpenIV, or wait for a future update.",
        )),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::providers::LegacySpProvider;

    fn provider(game_root: &Path) -> LegacySpProvider {
        LegacySpProvider::new(game_root.to_path_buf())
    }

    #[test]
    fn asi_targets_game_root() {
        let dir = tempfile::tempdir().unwrap();
        let game_root = dir.path().join("game");
        std::fs::create_dir_all(&game_root).unwrap();
        let mod_file = dir.path().join("cool_mod.asi");
        std::fs::write(&mod_file, b"payload").unwrap();

        let plan = classify(&mod_file, &provider(&game_root)).unwrap();
        assert_eq!(plan.format, ModFormat::Asi);
        assert_eq!(plan.files.len(), 1);
        assert_eq!(plan.files[0].target, game_root.join("cool_mod.asi"));
    }

    #[test]
    fn native_dll_targets_game_root() {
        let dir = tempfile::tempdir().unwrap();
        let game_root = dir.path().join("game");
        std::fs::create_dir_all(&game_root).unwrap();
        let mod_file = dir.path().join("native.dll");
        std::fs::write(&mod_file, b"not a real PE file").unwrap();

        let plan = classify(&mod_file, &provider(&game_root)).unwrap();
        assert_eq!(plan.format, ModFormat::NativeDll);
        assert_eq!(plan.files[0].target, game_root.join("native.dll"));
    }

    #[test]
    fn folder_mirrors_into_mods_subfolder() {
        let dir = tempfile::tempdir().unwrap();
        let game_root = dir.path().join("game");
        std::fs::create_dir_all(&game_root).unwrap();
        let mod_folder = dir.path().join("replacer_mod");
        std::fs::create_dir_all(mod_folder.join("update/x64/dlcpacks")).unwrap();
        std::fs::write(mod_folder.join("update/x64/dlcpacks/thing.rpf"), b"payload").unwrap();

        let plan = classify(&mod_folder, &provider(&game_root)).unwrap();
        assert_eq!(plan.format, ModFormat::FolderReplacer);
        assert_eq!(plan.files.len(), 1);
        assert_eq!(
            plan.files[0].target,
            game_root.join("mods/update/x64/dlcpacks/thing.rpf")
        );
    }

    #[test]
    fn rar_is_reported_unsupported_not_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let game_root = dir.path().join("game");
        let mod_file = dir.path().join("archive.rar");
        std::fs::write(&mod_file, b"fake rar").unwrap();

        let plan = classify(&mod_file, &provider(&game_root)).unwrap();
        assert!(matches!(plan.format, ModFormat::Unsupported(_)));
        assert!(plan.files.is_empty());
    }

    #[test]
    fn add_on_vehicle_pack_targets_dlcpacks_folder() {
        let dir = tempfile::tempdir().unwrap();
        let game_root = dir.path().join("game");
        let pack_folder = dir.path().join("MyAddonCar");
        std::fs::create_dir_all(&pack_folder).unwrap();
        std::fs::write(pack_folder.join("dlc.rpf"), b"payload").unwrap();

        let plan = classify(&pack_folder, &provider(&game_root)).unwrap();
        match plan.format {
            ModFormat::AddOnPack { pack_name } => assert_eq!(pack_name, "MyAddonCar"),
            other => panic!("expected AddOnPack, got {other:?}"),
        }
        assert_eq!(
            plan.files[0].target,
            game_root.join("mods/update/x64/dlcpacks/MyAddonCar/dlc.rpf")
        );
    }

    #[test]
    fn add_on_pack_one_level_down_is_still_detected() {
        let dir = tempfile::tempdir().unwrap();
        let game_root = dir.path().join("game");
        let outer = dir.path().join("DownloadedZipContents");
        let inner = outer.join("MyAddonCar");
        std::fs::create_dir_all(&inner).unwrap();
        std::fs::write(inner.join("dlc.rpf"), b"payload").unwrap();

        let plan = classify(&outer, &provider(&game_root)).unwrap();
        match plan.format {
            ModFormat::AddOnPack { pack_name } => assert_eq!(pack_name, "MyAddonCar"),
            other => panic!("expected AddOnPack, got {other:?}"),
        }
    }

    #[test]
    fn menyoo_xml_targets_detected_subfolder() {
        let dir = tempfile::tempdir().unwrap();
        let game_root = dir.path().join("game");
        let mod_file = dir.path().join("cool_outfit.xml");
        std::fs::write(&mod_file, b"<Outfit></Outfit>").unwrap();

        let plan = classify(&mod_file, &provider(&game_root)).unwrap();
        assert_eq!(plan.format, ModFormat::MenyooXml);
        assert_eq!(
            plan.files[0].target,
            game_root.join("menyooStuff/Outfits/cool_outfit.xml")
        );
    }
}
