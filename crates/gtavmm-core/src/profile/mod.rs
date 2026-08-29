// SPDX-License-Identifier: AGPL-3.0-only

//! Multi-profile support: a Profile is a named set of mods that should be active
//! together (e.g. a "Roleplay" profile vs. a "Graphics Showcase" profile on the same
//! game install). Switching profiles disables the mods other profiles want active
//! that this profile doesn't, then enables this profile's mods that are currently
//! disabled — reusing the existing `state::enable`/`disable` machinery unchanged, not
//! a parallel deploy mechanism.
//!
//! Profile membership is **opt-in per mod** (via [`add_mod`]): a mod never assigned
//! to any profile is left untouched by [`switch`]. This keeps the feature additive
//! on top of the MVP's existing behavior rather than forcing every installed mod
//! into a profile up front.

use std::collections::HashSet;
use std::path::Path;

use rusqlite::{Connection, OptionalExtension};
use serde::{Deserialize, Serialize};

use crate::error::CoreResult;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct Profile {
    pub id: i64,
    pub name: String,
    pub created_at: String,
    pub is_active: bool,
}

pub fn create(conn: &Connection, name: &str) -> CoreResult<i64> {
    conn.execute("INSERT INTO profile (name) VALUES (?1)", [name])?;
    Ok(conn.last_insert_rowid())
}

/// Deletes a profile (does not uninstall or otherwise touch its mods). If `profile_id`
/// is the currently active profile, clears `active_profile_id` first — otherwise the
/// delete would fail its foreign key constraint (a real bug found via testing: `switch`
/// followed by `delete` on that same profile used to error with an opaque "database
/// error" instead of either succeeding or failing clearly).
pub fn delete(conn: &Connection, profile_id: i64) -> CoreResult<()> {
    conn.execute(
        "UPDATE user_settings SET active_profile_id = NULL \
         WHERE id = 1 AND active_profile_id = ?1",
        [profile_id],
    )?;
    conn.execute("DELETE FROM profile WHERE id = ?1", [profile_id])?;
    Ok(())
}

fn active_profile_id(conn: &Connection) -> CoreResult<Option<i64>> {
    conn.query_row(
        "SELECT active_profile_id FROM user_settings WHERE id = 1",
        [],
        |row| row.get(0),
    )
    .map_err(Into::into)
}

pub fn list(conn: &Connection) -> CoreResult<Vec<Profile>> {
    let active_id = active_profile_id(conn)?;
    let mut stmt = conn.prepare("SELECT id, name, created_at FROM profile ORDER BY id")?;
    let rows = stmt.query_map([], |row| {
        let id: i64 = row.get(0)?;
        Ok(Profile {
            id,
            name: row.get(1)?,
            created_at: row.get(2)?,
            is_active: Some(id) == active_id,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

pub fn add_mod(conn: &Connection, profile_id: i64, installed_mod_id: i64) -> CoreResult<()> {
    conn.execute(
        "INSERT OR IGNORE INTO profile_mod (profile_id, installed_mod_id) VALUES (?1, ?2)",
        [profile_id, installed_mod_id],
    )?;
    Ok(())
}

pub fn remove_mod(conn: &Connection, profile_id: i64, installed_mod_id: i64) -> CoreResult<()> {
    conn.execute(
        "DELETE FROM profile_mod WHERE profile_id = ?1 AND installed_mod_id = ?2",
        [profile_id, installed_mod_id],
    )?;
    Ok(())
}

/// IDs of every mod currently assigned to `profile_id`, in no particular order —
/// exposed so a UI can render membership (e.g. checkboxes) without needing its own
/// query against `profile_mod`.
pub fn mod_ids_in_profile(conn: &Connection, profile_id: i64) -> CoreResult<Vec<i64>> {
    let mut stmt =
        conn.prepare("SELECT installed_mod_id FROM profile_mod WHERE profile_id = ?1")?;
    let rows = stmt.query_map([profile_id], |row| row.get(0))?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

#[derive(Debug, Default, Clone, PartialEq, Eq, Serialize)]
pub struct SwitchOutcome {
    pub enabled: Vec<i64>,
    pub disabled: Vec<i64>,
}

/// Switches the active profile to `profile_id`. Disables any active mod that belongs
/// to a *different* profile and isn't also wanted by this one, then enables this
/// profile's mods that are currently disabled. A mod belonging to no profile, or to
/// both the outgoing and incoming profile, is left exactly as it is.
pub fn switch(
    conn: &Connection,
    profile_id: i64,
    staging_root: &Path,
) -> CoreResult<SwitchOutcome> {
    let target_mods: HashSet<i64> = mod_ids_in_profile(conn, profile_id)?.into_iter().collect();

    let mut stmt = conn.prepare(
        "SELECT DISTINCT pm.installed_mod_id \
         FROM profile_mod pm \
         JOIN installed_mod m ON m.id = pm.installed_mod_id \
         WHERE pm.profile_id != ?1 AND m.status = 'active'",
    )?;
    let other_active: Vec<i64> = stmt
        .query_map([profile_id], |row| row.get(0))?
        .collect::<Result<Vec<_>, _>>()?;
    drop(stmt);

    let mut outcome = SwitchOutcome::default();
    for mod_id in other_active {
        if target_mods.contains(&mod_id) {
            continue;
        }
        crate::state::disable(conn, mod_id, staging_root)?;
        outcome.disabled.push(mod_id);
    }

    for mod_id in target_mods {
        let status: String = conn.query_row(
            "SELECT status FROM installed_mod WHERE id = ?1",
            [mod_id],
            |row| row.get(0),
        )?;
        if status == "disabled" {
            crate::state::enable(conn, mod_id, staging_root)?;
            outcome.enabled.push(mod_id);
        }
    }

    conn.execute(
        "UPDATE user_settings SET active_profile_id = ?1 WHERE id = 1",
        [profile_id],
    )?;

    Ok(outcome)
}

/// A portable description of a profile: its name and the *names* of the mods it
/// contains — not the mod files themselves, which this project never redistributes
/// (see the licensing/non-commercial decisions in the project docs).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProfileExport {
    pub name: String,
    pub mod_names: Vec<String>,
}

pub fn export(conn: &Connection, profile_id: i64) -> CoreResult<ProfileExport> {
    let name: String = conn.query_row(
        "SELECT name FROM profile WHERE id = ?1",
        [profile_id],
        |row| row.get(0),
    )?;
    let mut stmt = conn.prepare(
        "SELECT m.name FROM profile_mod pm \
         JOIN installed_mod m ON m.id = pm.installed_mod_id \
         WHERE pm.profile_id = ?1 ORDER BY m.name",
    )?;
    let mod_names = stmt
        .query_map([profile_id], |row| row.get(0))?
        .collect::<Result<Vec<String>, _>>()?;
    Ok(ProfileExport { name, mod_names })
}

#[derive(Debug, Default, Clone, PartialEq, Eq)]
pub struct ImportOutcome {
    pub profile_id: i64,
    pub matched: Vec<String>,
    pub not_found_locally: Vec<String>,
}

/// Imports a [`ProfileExport`] as a new profile, matching each mod name against mods
/// already installed on *this* machine (by exact name, excluding uninstalled ones).
/// This project has no source integration yet (see the roadmap) and never auto-
/// downloads mods, so a name that doesn't match anything installed locally is
/// reported back rather than fetched — the user installs it themselves first.
pub fn import(conn: &Connection, export: &ProfileExport) -> CoreResult<ImportOutcome> {
    let profile_id = create(conn, &export.name)?;
    let mut outcome = ImportOutcome {
        profile_id,
        ..Default::default()
    };
    for name in &export.mod_names {
        let existing: Option<i64> = conn
            .query_row(
                "SELECT id FROM installed_mod WHERE name = ?1 AND status != 'uninstalled'",
                [name],
                |row| row.get(0),
            )
            .optional()?;
        match existing {
            Some(mod_id) => {
                add_mod(conn, profile_id, mod_id)?;
                outcome.matched.push(name.clone());
            }
            None => outcome.not_found_locally.push(name.clone()),
        }
    }
    Ok(outcome)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_mod(conn: &Connection, name: &str, status: &str) -> i64 {
        conn.execute(
            "INSERT INTO installed_mod (name, source_type, install_path, status) \
             VALUES (?1, 'asi', '', ?2)",
            rusqlite::params![name, status],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn create_list_and_delete_roundtrip() {
        let conn = crate::db::open_in_memory().unwrap();
        let id = create(&conn, "Roleplay").unwrap();
        let profiles = list(&conn).unwrap();
        assert_eq!(profiles.len(), 1);
        assert_eq!(profiles[0].id, id);
        assert_eq!(profiles[0].name, "Roleplay");
        assert!(!profiles[0].is_active);

        delete(&conn, id).unwrap();
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn switch_disables_other_profile_mods_and_enables_target_mods() {
        let dir = tempfile::tempdir().unwrap();
        let game_root = dir.path().join("game");
        std::fs::create_dir_all(&game_root).unwrap();
        let staging_root = dir.path().join("staging");

        let conn = crate::db::open_in_memory().unwrap();

        // A mod belonging to profile A, currently active.
        let mod_a = insert_mod(&conn, "ModA", "active");
        conn.execute(
            "INSERT INTO installed_mod_file (installed_mod_id, target_path, file_hash) \
             VALUES (?1, ?2, 'hash')",
            rusqlite::params![mod_a, game_root.join("a.asi").to_string_lossy()],
        )
        .unwrap();
        std::fs::write(game_root.join("a.asi"), b"a").unwrap();

        // A mod belonging to profile B, currently disabled.
        let mod_b = insert_mod(&conn, "ModB", "disabled");
        conn.execute(
            "INSERT INTO installed_mod_file (installed_mod_id, target_path, file_hash) \
             VALUES (?1, ?2, 'hash')",
            rusqlite::params![mod_b, game_root.join("b.asi").to_string_lossy()],
        )
        .unwrap();
        std::fs::create_dir_all(staging_root.join(mod_b.to_string())).unwrap();
        std::fs::write(staging_root.join(mod_b.to_string()).join("0"), b"b").unwrap();

        let profile_a = create(&conn, "A").unwrap();
        add_mod(&conn, profile_a, mod_a).unwrap();
        let profile_b = create(&conn, "B").unwrap();
        add_mod(&conn, profile_b, mod_b).unwrap();

        let outcome = switch(&conn, profile_b, &staging_root).unwrap();
        assert_eq!(outcome.disabled, vec![mod_a]);
        assert_eq!(outcome.enabled, vec![mod_b]);
        assert!(!game_root.join("a.asi").exists());
        assert!(game_root.join("b.asi").exists());

        let profiles = list(&conn).unwrap();
        let b = profiles.iter().find(|p| p.id == profile_b).unwrap();
        assert!(b.is_active);
    }

    #[test]
    fn deleting_the_currently_active_profile_succeeds() {
        let dir = tempfile::tempdir().unwrap();
        let staging_root = dir.path().join("staging");
        let conn = crate::db::open_in_memory().unwrap();

        let profile_id = create(&conn, "Roleplay").unwrap();
        switch(&conn, profile_id, &staging_root).unwrap();
        assert!(list(&conn).unwrap()[0].is_active);

        // Before the fix, this failed its foreign key constraint (active_profile_id
        // still pointed at the row being deleted) instead of succeeding or erroring
        // clearly.
        delete(&conn, profile_id).unwrap();
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn export_then_import_matches_locally_installed_mods_by_name() {
        let conn = crate::db::open_in_memory().unwrap();
        let mod_a = insert_mod(&conn, "ModA", "active");
        let profile_id = create(&conn, "Original").unwrap();
        add_mod(&conn, profile_id, mod_a).unwrap();

        let exported = export(&conn, profile_id).unwrap();
        assert_eq!(exported.name, "Original");
        assert_eq!(exported.mod_names, vec!["ModA".to_string()]);

        // Simulate importing on a machine that also has ModA installed, but not ModB.
        let mut for_import = exported.clone();
        for_import.mod_names.push("ModB".to_string());
        let outcome = import(&conn, &for_import).unwrap();
        assert_eq!(outcome.matched, vec!["ModA".to_string()]);
        assert_eq!(outcome.not_found_locally, vec!["ModB".to_string()]);

        let imported_mods = mod_ids_in_profile(&conn, outcome.profile_id).unwrap();
        assert_eq!(imported_mods, vec![mod_a]);
    }
}
