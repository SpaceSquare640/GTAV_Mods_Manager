// SPDX-License-Identifier: AGPL-3.0-only

//! Uninstall: delete each `installed_mod_file.target_path` (guarded by
//! `protected_files::check_write`, which should never actually trigger here since
//! `install` already prevents ever deploying to a protected path — kept as
//! defense-in-depth, validated for *every* file before any deletion begins so a
//! surprise hit aborts with zero partial effect rather than leaving the mod
//! half-removed), restore each `backup_path`, write an `install_event` row, then hand
//! a snapshot of the deployed files to the recycle bin (snapshot-at-uninstall-time,
//! not at-install-time — see the MVP spec's recycle-bin tradeoff writeup) instead of
//! purging immediately. Add-on packs (detected by an `update/x64/dlcpacks/<name>/`
//! target path segment — see `mod_analyzer::dlc`) also get their `dlclist.xml` entry
//! removed.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::{CoreError, CoreResult};
use crate::mod_analyzer::dlc;
use crate::protected_files;
use crate::recycle_bin::RETENTION_DAYS;

struct ModFileRow {
    target_path: String,
    backup_path: Option<String>,
}

fn load_mod_files(conn: &Connection, mod_id: i64) -> CoreResult<Vec<ModFileRow>> {
    let mut stmt = conn.prepare(
        "SELECT target_path, backup_path FROM installed_mod_file WHERE installed_mod_id = ?1",
    )?;
    let rows = stmt.query_map([mod_id], |row| {
        Ok(ModFileRow {
            target_path: row.get(0)?,
            backup_path: row.get(1)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn mod_status(conn: &Connection, mod_id: i64) -> CoreResult<Option<String>> {
    conn.query_row(
        "SELECT status FROM installed_mod WHERE id = ?1",
        [mod_id],
        |row| row.get(0),
    )
    .map(Some)
    .or_else(|e| {
        if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
            Ok(None)
        } else {
            Err(e.into())
        }
    })
}

/// Extracts the add-on pack name from a target path if it sits under
/// `.../dlcpacks/<name>/...`, mirroring how `mod_analyzer::classify_add_on_pack`
/// constructs the target in the first place. We don't persist a dedicated "is this an
/// add-on" column — it's fully derivable from the path, so we derive it here rather
/// than adding schema just for this.
fn dlc_pack_name_from_target(path: &Path) -> Option<String> {
    let mut components = path.components().peekable();
    while let Some(component) = components.next() {
        if component.as_os_str().eq_ignore_ascii_case("dlcpacks") {
            return components
                .peek()
                .map(|c| c.as_os_str().to_string_lossy().into_owned());
        }
    }
    None
}

/// Removes `installed_mod_id`: deletes its deployed files (restoring anything they
/// overwrote), snapshots them into the recycle bin, deregisters any add-on pack
/// entries, and records the event. `game_root` is needed to locate `dlclist.xml`.
pub fn uninstall(
    conn: &mut Connection,
    installed_mod_id: i64,
    game_root: &Path,
    recycle_bin_root: &Path,
) -> CoreResult<()> {
    match mod_status(conn, installed_mod_id)? {
        None => {
            return Err(CoreError::UnsupportedFormat {
                reason: format!("no installed mod with id {installed_mod_id}"),
            })
        }
        Some(status) if status == "uninstalled" => {
            return Err(CoreError::UnsupportedFormat {
                reason: format!("mod {installed_mod_id} is already uninstalled"),
            })
        }
        Some(_) => {}
    }

    let files = load_mod_files(conn, installed_mod_id)?;

    // Pass 1: validate every target against protected_files before mutating anything.
    for file in &files {
        protected_files::check_write(Path::new(&file.target_path))?;
    }

    // Pass 2: snapshot deployed files into the recycle bin, then delete/restore.
    let entry_dir_name = format!(
        "{installed_mod_id}-{}",
        chrono::Utc::now().format("%Y%m%d%H%M%S%3f")
    );
    let entry_dir = recycle_bin_root.join(&entry_dir_name);
    std::fs::create_dir_all(&entry_dir)?;

    let mut pack_names = std::collections::HashSet::new();

    for file in &files {
        let target = PathBuf::from(&file.target_path);
        if let Some(name) = dlc_pack_name_from_target(&target) {
            pack_names.insert(name);
        }

        if target.exists() {
            let relative = target.strip_prefix(game_root).unwrap_or(&target);
            let snapshot_path = entry_dir.join(relative);
            if let Some(parent) = snapshot_path.parent() {
                std::fs::create_dir_all(parent)?;
            }
            std::fs::copy(&target, &snapshot_path)?;
            std::fs::remove_file(&target)?;
        }

        if let Some(backup) = &file.backup_path {
            let backup_path = PathBuf::from(backup);
            if backup_path.exists() {
                crate::util::move_file(&backup_path, &target)?;
            }
        }
    }

    for pack_name in &pack_names {
        // Best-effort: an uninstall shouldn't hard-fail just because dlclist.xml was
        // since deleted or already lacks the entry — the files are already gone,
        // which is the primary thing that matters.
        let _ = dlc::remove_entry(&dlc::dlclist_path(game_root), pack_name);
    }

    let deleted_at = chrono::Utc::now();
    let expires_at = deleted_at + chrono::Duration::days(RETENTION_DAYS);

    let db_tx = conn.transaction()?;
    db_tx.execute(
        "INSERT INTO recycle_bin_entry (original_installed_mod_id, mod_package_snapshot_path, deleted_at, expires_at) \
         VALUES (?1, ?2, ?3, ?4)",
        rusqlite::params![
            installed_mod_id,
            entry_dir.to_string_lossy(),
            deleted_at.to_rfc3339(),
            expires_at.to_rfc3339(),
        ],
    )?;
    db_tx.execute(
        "UPDATE installed_mod SET status = 'uninstalled' WHERE id = ?1",
        [installed_mod_id],
    )?;
    db_tx.execute(
        "INSERT INTO install_event (installed_mod_id, event_type, success) VALUES (?1, 'uninstall', 1)",
        [installed_mod_id],
    )?;
    db_tx.commit()?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_installed_mod(
        conn: &Connection,
        game_root: &Path,
        target_name: &str,
        backup: Option<&Path>,
    ) -> i64 {
        conn.execute(
            "INSERT INTO installed_mod (name, source_type, install_path, status) \
             VALUES ('TestMod', 'asi', '', 'active')",
            [],
        )
        .unwrap();
        let mod_id = conn.last_insert_rowid();
        let target = game_root.join(target_name);
        conn.execute(
            "INSERT INTO installed_mod_file (installed_mod_id, target_path, backup_path, file_hash) \
             VALUES (?1, ?2, ?3, 'hash')",
            rusqlite::params![
                mod_id,
                target.to_string_lossy(),
                backup.map(|p| p.to_string_lossy().into_owned()),
            ],
        )
        .unwrap();
        mod_id
    }

    #[test]
    fn uninstall_deletes_file_and_marks_status() {
        let dir = tempfile::tempdir().unwrap();
        let game_root = dir.path().join("game");
        std::fs::create_dir_all(&game_root).unwrap();
        std::fs::write(game_root.join("mod.asi"), b"payload").unwrap();

        let mut conn = crate::db::open_in_memory().unwrap();
        let mod_id = setup_installed_mod(&conn, &game_root, "mod.asi", None);

        uninstall(&mut conn, mod_id, &game_root, &dir.path().join("recycle")).unwrap();

        assert!(!game_root.join("mod.asi").exists());
        let status: String = conn
            .query_row(
                "SELECT status FROM installed_mod WHERE id = ?1",
                [mod_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "uninstalled");
    }

    #[test]
    fn uninstall_restores_backed_up_original() {
        let dir = tempfile::tempdir().unwrap();
        let game_root = dir.path().join("game");
        std::fs::create_dir_all(&game_root).unwrap();
        let backup_path = dir.path().join("backup_of_original.dll");
        std::fs::write(&backup_path, b"original content").unwrap();
        std::fs::write(game_root.join("shared.dll"), b"mod's overwritten content").unwrap();

        let mut conn = crate::db::open_in_memory().unwrap();
        let mod_id = setup_installed_mod(&conn, &game_root, "shared.dll", Some(&backup_path));

        uninstall(&mut conn, mod_id, &game_root, &dir.path().join("recycle")).unwrap();

        assert_eq!(
            std::fs::read(game_root.join("shared.dll")).unwrap(),
            b"original content"
        );
    }

    #[test]
    fn uninstall_snapshots_into_recycle_bin_with_expiry() {
        let dir = tempfile::tempdir().unwrap();
        let game_root = dir.path().join("game");
        std::fs::create_dir_all(&game_root).unwrap();
        std::fs::write(game_root.join("mod.asi"), b"payload").unwrap();
        let recycle_root = dir.path().join("recycle");

        let mut conn = crate::db::open_in_memory().unwrap();
        let mod_id = setup_installed_mod(&conn, &game_root, "mod.asi", None);

        uninstall(&mut conn, mod_id, &game_root, &recycle_root).unwrap();

        let (snapshot_path, deleted_at, expires_at): (String, String, String) = conn
            .query_row(
                "SELECT mod_package_snapshot_path, deleted_at, expires_at \
                 FROM recycle_bin_entry WHERE original_installed_mod_id = ?1",
                [mod_id],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
            )
            .unwrap();

        let snapshot_file = PathBuf::from(&snapshot_path).join("mod.asi");
        assert!(snapshot_file.exists(), "snapshot should preserve the file");
        assert_eq!(std::fs::read(&snapshot_file).unwrap(), b"payload");

        let deleted_at = chrono::DateTime::parse_from_rfc3339(&deleted_at).unwrap();
        let expires_at = chrono::DateTime::parse_from_rfc3339(&expires_at).unwrap();
        assert_eq!((expires_at - deleted_at).num_days(), RETENTION_DAYS);
    }

    #[test]
    fn uninstalling_add_on_pack_removes_dlclist_entry() {
        let dir = tempfile::tempdir().unwrap();
        let game_root = dir.path().join("game");
        let dlclist_path = dlc::dlclist_path(&game_root);
        std::fs::create_dir_all(dlclist_path.parent().unwrap()).unwrap();
        std::fs::write(
            &dlclist_path,
            "<SMandatoryPacksData>\n  <Paths>\n    <Item>dlcpacks:\\MyAddonCar\\</Item>\n  </Paths>\n</SMandatoryPacksData>\n",
        )
        .unwrap();
        let pack_target = game_root.join("mods/update/x64/dlcpacks/MyAddonCar/dlc.rpf");
        std::fs::create_dir_all(pack_target.parent().unwrap()).unwrap();
        std::fs::write(&pack_target, b"payload").unwrap();

        let mut conn = crate::db::open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO installed_mod (name, source_type, install_path, status) \
             VALUES ('My Addon Car', 'folder', '', 'active')",
            [],
        )
        .unwrap();
        let mod_id = conn.last_insert_rowid();
        conn.execute(
            "INSERT INTO installed_mod_file (installed_mod_id, target_path, file_hash) VALUES (?1, ?2, 'hash')",
            rusqlite::params![mod_id, pack_target.to_string_lossy()],
        )
        .unwrap();

        uninstall(&mut conn, mod_id, &game_root, &dir.path().join("recycle")).unwrap();

        assert!(!dlc::has_entry(&dlclist_path, "MyAddonCar").unwrap());
    }

    #[test]
    fn uninstalling_unknown_mod_id_errors() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = crate::db::open_in_memory().unwrap();
        let err = uninstall(&mut conn, 999, dir.path(), &dir.path().join("recycle")).unwrap_err();
        assert!(matches!(err, CoreError::UnsupportedFormat { .. }));
    }

    #[test]
    fn uninstalling_twice_errors_on_second_attempt() {
        let dir = tempfile::tempdir().unwrap();
        let game_root = dir.path().join("game");
        std::fs::create_dir_all(&game_root).unwrap();
        std::fs::write(game_root.join("mod.asi"), b"payload").unwrap();

        let mut conn = crate::db::open_in_memory().unwrap();
        let mod_id = setup_installed_mod(&conn, &game_root, "mod.asi", None);

        uninstall(&mut conn, mod_id, &game_root, &dir.path().join("recycle")).unwrap();
        let err =
            uninstall(&mut conn, mod_id, &game_root, &dir.path().join("recycle")).unwrap_err();
        assert!(matches!(err, CoreError::UnsupportedFormat { .. }));
    }
}
