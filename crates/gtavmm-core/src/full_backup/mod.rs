// SPDX-License-Identifier: AGPL-3.0-only

//! Full backup/restore of the entire `mods\` folder as a single zip — the coarse,
//! whole-folder safety net validated by the old prototype (see
//! `docs/planning`/vault, not part of this repo). Distinct from the per-mod
//! backup/rollback in `install`: this is a manual, on-demand snapshot of everything
//! under `mods\` at once, useful before a risky batch of changes or as a periodic
//! checkpoint, not tied to any single mod's install/uninstall lifecycle.
//!
//! Restore is additive/overwrite — it extracts the zip on top of the existing
//! `mods\` folder, it never deletes files that aren't in the archive. That matches
//! the old prototype's behavior and keeps this a "bring back what was saved," not a
//! "make the folder exactly match the snapshot" operation.
//!
//! Real-world `mods\` folders run tens of gigabytes across a modest number of large
//! files (texture/vehicle replacements). Two consequences of that: (1) files are
//! streamed via `std::io::copy`, never buffered whole into memory; (2) entries are
//! stored **uncompressed** (`CompressionMethod::Stored`) rather than Deflated —
//! most of that content is already-compressed binary data (rpf archives, dds
//! textures), so Deflate mostly burns CPU time for negligible size savings, and this
//! is a safety-net snapshot, not long-term archival storage.

use std::path::{Path, PathBuf};

use zip::write::SimpleFileOptions;

use crate::error::{CoreError, CoreResult};

const MODS_SUBFOLDER: &str = "mods";

/// Creates `<backup_root>/full/mods_<timestamp>.zip` from `<game_root>/mods`.
/// Returns the created zip's path. Errors if the `mods\` folder doesn't exist (there
/// is nothing to back up yet).
pub fn create(game_root: &Path, backup_root: &Path) -> CoreResult<PathBuf> {
    let mods_dir = game_root.join(MODS_SUBFOLDER);
    if !mods_dir.is_dir() {
        return Err(CoreError::UnsupportedFormat {
            reason: format!(
                "{} does not exist yet — nothing to back up",
                mods_dir.display()
            ),
        });
    }

    let full_backup_dir = backup_root.join("full");
    std::fs::create_dir_all(&full_backup_dir)?;
    let zip_path = full_backup_dir.join(format!(
        "mods_{}.zip",
        chrono::Utc::now().format("%Y%m%d%H%M%S")
    ));

    let file = std::fs::File::create(&zip_path)?;
    let mut writer = zip::ZipWriter::new(file);
    let options = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Stored);

    for entry in walkdir::WalkDir::new(&mods_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_type().is_file())
    {
        let relative = entry.path().strip_prefix(&mods_dir).unwrap_or(entry.path());
        // Zip entry names are always forward-slash, even on Windows.
        let entry_name = relative.to_string_lossy().replace('\\', "/");

        writer
            .start_file(entry_name, options)
            .map_err(zip_err("writing zip entry"))?;
        let mut source = std::fs::File::open(entry.path())?;
        std::io::copy(&mut source, &mut writer)?;
    }

    writer.finish().map_err(zip_err("finalizing zip"))?;
    Ok(zip_path)
}

/// Lists existing full backups (newest first), as created by [`create`].
pub fn list(backup_root: &Path) -> CoreResult<Vec<PathBuf>> {
    let full_backup_dir = backup_root.join("full");
    if !full_backup_dir.is_dir() {
        return Ok(Vec::new());
    }
    let mut entries: Vec<PathBuf> = std::fs::read_dir(&full_backup_dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().is_some_and(|ext| ext == "zip"))
        .collect();
    entries.sort();
    entries.reverse();
    Ok(entries)
}

/// Extracts `zip_path` on top of `<game_root>/mods` — additive/overwrite, never
/// deletes files that aren't present in the archive.
pub fn restore(zip_path: &Path, game_root: &Path) -> CoreResult<()> {
    let mods_dir = game_root.join(MODS_SUBFOLDER);
    std::fs::create_dir_all(&mods_dir)?;

    let file = std::fs::File::open(zip_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(zip_err("reading backup zip"))?;
    archive
        .extract(&mods_dir)
        .map_err(zip_err("extracting backup zip"))?;
    Ok(())
}

fn zip_err(context: &'static str) -> impl Fn(zip::result::ZipError) -> CoreError {
    move |e| CoreError::UnsupportedFormat {
        reason: format!("{context}: {e}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_fails_when_mods_folder_does_not_exist() {
        let dir = tempfile::tempdir().unwrap();
        let game_root = dir.path().join("game");
        std::fs::create_dir_all(&game_root).unwrap();
        let err = create(&game_root, &dir.path().join("backups")).unwrap_err();
        assert!(matches!(err, CoreError::UnsupportedFormat { .. }));
    }

    #[test]
    fn create_then_list_then_restore_roundtrips() {
        let dir = tempfile::tempdir().unwrap();
        let game_root = dir.path().join("game");
        let mods_dir = game_root.join("mods");
        std::fs::create_dir_all(mods_dir.join("update/x64/dlcpacks/mymod")).unwrap();
        std::fs::write(
            mods_dir.join("update/x64/dlcpacks/mymod/dlc.rpf"),
            b"payload",
        )
        .unwrap();
        std::fs::write(mods_dir.join("root_file.asi"), b"asi payload").unwrap();
        let backup_root = dir.path().join("backups");

        let zip_path = create(&game_root, &backup_root).unwrap();
        assert!(zip_path.exists());

        let backups = list(&backup_root).unwrap();
        assert_eq!(backups, vec![zip_path.clone()]);

        // Simulate loss: delete the mods folder entirely, then restore.
        std::fs::remove_dir_all(&mods_dir).unwrap();
        assert!(!mods_dir.exists());

        restore(&zip_path, &game_root).unwrap();

        assert_eq!(
            std::fs::read(mods_dir.join("update/x64/dlcpacks/mymod/dlc.rpf")).unwrap(),
            b"payload"
        );
        assert_eq!(
            std::fs::read(mods_dir.join("root_file.asi")).unwrap(),
            b"asi payload"
        );
    }

    #[test]
    fn restore_is_additive_and_does_not_delete_existing_files() {
        let dir = tempfile::tempdir().unwrap();
        let game_root = dir.path().join("game");
        let mods_dir = game_root.join("mods");
        std::fs::create_dir_all(&mods_dir).unwrap();
        std::fs::write(mods_dir.join("a.asi"), b"a").unwrap();
        let backup_root = dir.path().join("backups");
        let zip_path = create(&game_root, &backup_root).unwrap();

        // A file added *after* the backup was taken should survive a restore.
        std::fs::write(mods_dir.join("b.asi"), b"b").unwrap();

        restore(&zip_path, &game_root).unwrap();

        assert!(mods_dir.join("a.asi").exists());
        assert!(
            mods_dir.join("b.asi").exists(),
            "restore must not delete files absent from the backup"
        );
    }

    #[test]
    fn list_is_empty_when_no_backups_exist() {
        let dir = tempfile::tempdir().unwrap();
        assert!(list(&dir.path().join("backups")).unwrap().is_empty());
    }
}
