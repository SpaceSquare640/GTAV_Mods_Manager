// SPDX-License-Identifier: AGPL-3.0-only

//! Editing an installed mod's own `notes`/`link` fields — both columns already existed
//! on `installed_mod` (surfaced read-only via `list_mods`), but nothing let the user
//! actually set them from the app. Deliberately separate from [`crate::state`]
//! (enable/disable) and [`crate::install`]/[`crate::uninstall`]: this never touches a
//! file on disk, only the two free-text columns.

use rusqlite::Connection;

use crate::error::{CoreError, CoreResult};

/// Sets `notes`/`link` on an installed mod. Either may be `None` to clear that field.
pub fn update(
    conn: &Connection,
    mod_id: i64,
    notes: Option<&str>,
    link: Option<&str>,
) -> CoreResult<()> {
    let rows = conn.execute(
        "UPDATE installed_mod SET notes = ?2, link = ?3 WHERE id = ?1",
        rusqlite::params![mod_id, notes, link],
    )?;
    if rows == 0 {
        return Err(CoreError::UnsupportedFormat {
            reason: format!("no installed mod with id {mod_id}"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn seed_mod(conn: &Connection) -> i64 {
        conn.execute(
            "INSERT INTO installed_mod (name, source_type, install_path, status) \
             VALUES ('Menyoo PC Trainer', 'asi', '', 'active')",
            [],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn update_sets_notes_and_link() {
        let conn = crate::db::open_in_memory().unwrap();
        let mod_id = seed_mod(&conn);
        update(
            &conn,
            mod_id,
            Some("great trainer"),
            Some("https://example.com"),
        )
        .unwrap();
        let (notes, link): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT notes, link FROM installed_mod WHERE id = ?1",
                [mod_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert_eq!(notes.as_deref(), Some("great trainer"));
        assert_eq!(link.as_deref(), Some("https://example.com"));
    }

    #[test]
    fn update_can_clear_fields_back_to_null() {
        let conn = crate::db::open_in_memory().unwrap();
        let mod_id = seed_mod(&conn);
        update(&conn, mod_id, Some("temp"), None).unwrap();
        update(&conn, mod_id, None, None).unwrap();
        let (notes, link): (Option<String>, Option<String>) = conn
            .query_row(
                "SELECT notes, link FROM installed_mod WHERE id = ?1",
                [mod_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .unwrap();
        assert!(notes.is_none());
        assert!(link.is_none());
    }

    #[test]
    fn update_of_unknown_mod_id_errors() {
        let conn = crate::db::open_in_memory().unwrap();
        assert!(update(&conn, 999, Some("x"), None).is_err());
    }
}
