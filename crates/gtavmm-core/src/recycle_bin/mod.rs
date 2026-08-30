// SPDX-License-Identifier: AGPL-3.0-only

//! Recycle bin: snapshot-at-uninstall-time storage of deployed files (written by
//! `uninstall`), no count limit, 15-day retention (`expires_at = deleted_at + 15
//! days`), swept on app/CLI startup — there is no background daemon (per the
//! project's "no always-on service" decision), so sweeping only happens when
//! something actually calls [`sweep_expired`].
//!
//! Restore replays files back through the guarded write path (still subject to
//! `protected_files`), and — since `uninstall` already consumed each file's original
//! `backup_path` by restoring it in place — re-establishes a fresh backup of whatever
//! is currently at each target before overwriting it, so a later re-uninstall can
//! still correctly restore the pre-mod state.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;

use crate::error::{CoreError, CoreResult};
use crate::protected_files;
use crate::util::hash_file;

pub const RETENTION_DAYS: i64 = 15;

#[derive(Debug, Clone, Serialize)]
pub struct RecycleBinEntry {
    pub id: i64,
    pub original_installed_mod_id: Option<i64>,
    pub deleted_at: String,
    pub expires_at: String,
}

/// Every recycle bin entry, most recently deleted first — the CLI's own `recycle-bin
/// list` used to run this query inline; factored out here so the GUI can show the
/// same thing without duplicating it.
pub fn list(conn: &Connection) -> CoreResult<Vec<RecycleBinEntry>> {
    let mut stmt = conn.prepare(
        "SELECT id, original_installed_mod_id, deleted_at, expires_at \
         FROM recycle_bin_entry ORDER BY deleted_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(RecycleBinEntry {
            id: row.get(0)?,
            original_installed_mod_id: row.get(1)?,
            deleted_at: row.get(2)?,
            expires_at: row.get(3)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

struct RecycleBinEntryRow {
    original_installed_mod_id: Option<i64>,
    mod_package_snapshot_path: String,
}

fn load_entry(conn: &Connection, entry_id: i64) -> CoreResult<RecycleBinEntryRow> {
    conn.query_row(
        "SELECT original_installed_mod_id, mod_package_snapshot_path \
         FROM recycle_bin_entry WHERE id = ?1",
        [entry_id],
        |row| {
            Ok(RecycleBinEntryRow {
                original_installed_mod_id: row.get(0)?,
                mod_package_snapshot_path: row.get(1)?,
            })
        },
    )
    .map_err(|e| {
        if matches!(e, rusqlite::Error::QueryReturnedNoRows) {
            CoreError::UnsupportedFormat {
                reason: format!("no recycle bin entry with id {entry_id}"),
            }
        } else {
            e.into()
        }
    })
}

/// Restores a recycle-bin entry: copies its snapshotted files back to their original
/// target paths, reinstates the owning mod as `active`, and removes the entry (its
/// job is done — the 15-day window doesn't apply to something already restored).
pub fn restore(
    conn: &mut Connection,
    entry_id: i64,
    game_root: &Path,
    backup_root: &Path,
) -> CoreResult<()> {
    let entry = load_entry(conn, entry_id)?;
    let mod_id = entry
        .original_installed_mod_id
        .ok_or_else(|| CoreError::UnsupportedFormat {
            reason: format!("recycle bin entry {entry_id} has no associated mod to restore"),
        })?;
    let snapshot_root = PathBuf::from(&entry.mod_package_snapshot_path);

    let mut stmt =
        conn.prepare("SELECT id, target_path FROM installed_mod_file WHERE installed_mod_id = ?1")?;
    let files: Vec<(i64, String)> = stmt
        .query_map([mod_id], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    drop(stmt);

    for (file_id, target_path) in &files {
        let target = PathBuf::from(target_path);
        let relative = target.strip_prefix(game_root).unwrap_or(&target);
        let snapshot_file = snapshot_root.join(relative);
        if !snapshot_file.exists() {
            continue; // nothing was snapshotted for this row (file didn't exist at uninstall time)
        }

        protected_files::check_write(&target)?;
        if let Some(parent) = target.parent() {
            std::fs::create_dir_all(parent)?;
        }

        // The file's original `backup_path` was already consumed by `uninstall`
        // (moved back onto `target`). Re-backup whatever is there now — the restored
        // pre-mod original, or nothing — before overwriting it with the mod's file,
        // so a future re-uninstall can restore correctly again.
        let new_backup_path = if target.exists() {
            std::fs::create_dir_all(backup_root)?;
            let backup_path = backup_root.join(format!("restore-{file_id}.bak"));
            std::fs::copy(&target, &backup_path)?;
            Some(backup_path)
        } else {
            None
        };

        std::fs::copy(&snapshot_file, &target)?;
        let hash = hash_file(&target)?;

        conn.execute(
            "UPDATE installed_mod_file SET backup_path = ?1, file_hash = ?2 WHERE id = ?3",
            rusqlite::params![
                new_backup_path.map(|p| p.to_string_lossy().into_owned()),
                hash,
                file_id,
            ],
        )?;
    }

    let db_tx = conn.transaction()?;
    db_tx.execute(
        "UPDATE installed_mod SET status = 'active' WHERE id = ?1",
        [mod_id],
    )?;
    db_tx.execute(
        "INSERT INTO install_event (installed_mod_id, event_type, success) VALUES (?1, 'restore', 1)",
        [mod_id],
    )?;
    db_tx.execute("DELETE FROM recycle_bin_entry WHERE id = ?1", [entry_id])?;
    db_tx.commit()?;

    if snapshot_root.exists() {
        let _ = std::fs::remove_dir_all(&snapshot_root); // best-effort cleanup
    }

    Ok(())
}

/// Deletes any `recycle_bin_entry` rows (and their snapshot storage) past
/// `expires_at`. Called once at app/CLI startup and on demand — there is no
/// background scheduler.
pub fn sweep_expired(conn: &Connection) -> CoreResult<usize> {
    let now = chrono::Utc::now().to_rfc3339();

    let mut stmt = conn.prepare(
        "SELECT id, mod_package_snapshot_path FROM recycle_bin_entry WHERE expires_at < ?1",
    )?;
    let expired: Vec<(i64, String)> = stmt
        .query_map([&now], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    drop(stmt);

    for (id, snapshot_path) in &expired {
        let path = PathBuf::from(snapshot_path);
        if path.exists() {
            std::fs::remove_dir_all(&path)?;
        }
        conn.execute("DELETE FROM recycle_bin_entry WHERE id = ?1", [id])?;
    }

    Ok(expired.len())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn setup_uninstalled_mod_with_recycle_entry(
        conn: &Connection,
        game_root: &Path,
        recycle_root: &Path,
        target_name: &str,
        expires_offset_days: i64,
    ) -> (i64, i64) {
        conn.execute(
            "INSERT INTO installed_mod (name, source_type, install_path, status) \
             VALUES ('TestMod', 'asi', '', 'uninstalled')",
            [],
        )
        .unwrap();
        let mod_id = conn.last_insert_rowid();
        let target = game_root.join(target_name);
        conn.execute(
            "INSERT INTO installed_mod_file (installed_mod_id, target_path, file_hash) \
             VALUES (?1, ?2, 'hash')",
            rusqlite::params![mod_id, target.to_string_lossy()],
        )
        .unwrap();

        let entry_dir = recycle_root.join(format!("{mod_id}-entry"));
        std::fs::create_dir_all(&entry_dir).unwrap();
        std::fs::write(entry_dir.join(target_name), b"snapshotted content").unwrap();

        let deleted_at = chrono::Utc::now();
        let expires_at = deleted_at + chrono::Duration::days(expires_offset_days);
        conn.execute(
            "INSERT INTO recycle_bin_entry (original_installed_mod_id, mod_package_snapshot_path, deleted_at, expires_at) \
             VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![mod_id, entry_dir.to_string_lossy(), deleted_at.to_rfc3339(), expires_at.to_rfc3339()],
        )
        .unwrap();
        let entry_id = conn.last_insert_rowid();

        (mod_id, entry_id)
    }

    #[test]
    fn restore_copies_file_back_and_reactivates_mod() {
        let dir = tempfile::tempdir().unwrap();
        let game_root = dir.path().join("game");
        std::fs::create_dir_all(&game_root).unwrap();
        let recycle_root = dir.path().join("recycle");
        let backup_root = dir.path().join("backups");

        let mut conn = crate::db::open_in_memory().unwrap();
        let (mod_id, entry_id) = setup_uninstalled_mod_with_recycle_entry(
            &conn,
            &game_root,
            &recycle_root,
            "mod.asi",
            15,
        );

        restore(&mut conn, entry_id, &game_root, &backup_root).unwrap();

        assert_eq!(
            std::fs::read(game_root.join("mod.asi")).unwrap(),
            b"snapshotted content"
        );
        let status: String = conn
            .query_row(
                "SELECT status FROM installed_mod WHERE id = ?1",
                [mod_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "active");

        let remaining: i64 = conn
            .query_row("SELECT COUNT(*) FROM recycle_bin_entry", [], |r| r.get(0))
            .unwrap();
        assert_eq!(
            remaining, 0,
            "restored entry should be removed from the recycle bin"
        );
    }

    #[test]
    fn restore_backs_up_whatever_currently_occupies_the_target() {
        let dir = tempfile::tempdir().unwrap();
        let game_root = dir.path().join("game");
        std::fs::create_dir_all(&game_root).unwrap();
        // Simulate the restored pre-mod original sitting at the target (as left by
        // `uninstall`'s backup-restore step).
        std::fs::write(game_root.join("mod.asi"), b"pre-mod original").unwrap();
        let recycle_root = dir.path().join("recycle");
        let backup_root = dir.path().join("backups");

        let mut conn = crate::db::open_in_memory().unwrap();
        let (_, entry_id) = setup_uninstalled_mod_with_recycle_entry(
            &conn,
            &game_root,
            &recycle_root,
            "mod.asi",
            15,
        );

        restore(&mut conn, entry_id, &game_root, &backup_root).unwrap();

        let backup_path: Option<String> = conn
            .query_row(
                "SELECT backup_path FROM installed_mod_file WHERE target_path = ?1",
                [game_root.join("mod.asi").to_string_lossy()],
                |r| r.get(0),
            )
            .unwrap();
        let backup_path =
            PathBuf::from(backup_path.expect("a fresh backup should have been recorded"));
        assert_eq!(std::fs::read(&backup_path).unwrap(), b"pre-mod original");
    }

    #[test]
    fn sweep_expired_removes_only_past_due_entries() {
        let dir = tempfile::tempdir().unwrap();
        let game_root = dir.path().join("game");
        std::fs::create_dir_all(&game_root).unwrap();
        let recycle_root = dir.path().join("recycle");

        let conn = crate::db::open_in_memory().unwrap();
        let (_, expired_entry) = setup_uninstalled_mod_with_recycle_entry(
            &conn,
            &game_root,
            &recycle_root,
            "old.asi",
            -1, // already expired
        );
        let (_, fresh_entry) = setup_uninstalled_mod_with_recycle_entry(
            &conn,
            &game_root,
            &recycle_root,
            "new.asi",
            15, // not expired
        );

        let removed = sweep_expired(&conn).unwrap();

        assert_eq!(removed, 1);
        let remaining_id: i64 = conn
            .query_row("SELECT id FROM recycle_bin_entry", [], |r| r.get(0))
            .unwrap();
        assert_eq!(remaining_id, fresh_entry);
        assert_ne!(remaining_id, expired_entry);
    }

    #[test]
    fn restore_unknown_entry_id_errors() {
        let dir = tempfile::tempdir().unwrap();
        let mut conn = crate::db::open_in_memory().unwrap();
        let err = restore(&mut conn, 999, dir.path(), &dir.path().join("backups")).unwrap_err();
        assert!(matches!(err, CoreError::UnsupportedFormat { .. }));
    }

    #[test]
    fn list_returns_entries_most_recently_deleted_first() {
        let dir = tempfile::tempdir().unwrap();
        let game_root = dir.path().join("game");
        let recycle_root = dir.path().join("recycle");
        std::fs::create_dir_all(&game_root).unwrap();
        let conn = crate::db::open_in_memory().unwrap();
        assert!(list(&conn).unwrap().is_empty());

        let (_, first_id) =
            setup_uninstalled_mod_with_recycle_entry(&conn, &game_root, &recycle_root, "a.asi", 15);
        std::thread::sleep(std::time::Duration::from_millis(2));
        let (_, second_id) =
            setup_uninstalled_mod_with_recycle_entry(&conn, &game_root, &recycle_root, "b.asi", 15);

        let entries = list(&conn).unwrap();
        assert_eq!(entries.len(), 2);
        assert_eq!(entries[0].id, second_id);
        assert_eq!(entries[1].id, first_id);
    }
}
