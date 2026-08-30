// SPDX-License-Identifier: AGPL-3.0-only

//! Standalone bookmarks for mod page URLs (e.g. gta5-mods.com listings) — deliberately
//! independent of `installed_mod`: a link is worth saving before a mod is ever
//! installed (something to come back to and download later) or after it's been
//! uninstalled (in case the user wants to reinstall from the same source someday).
//! Pure CRUD, no network access — this never fetches or validates the URL itself.

use rusqlite::Connection;
use serde::Serialize;

use crate::error::{CoreError, CoreResult};

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SavedModLink {
    pub id: i64,
    pub name: String,
    pub url: String,
    pub notes: Option<String>,
    pub created_at: String,
}

/// Saves a new bookmark and returns its id.
pub fn add(conn: &Connection, name: &str, url: &str, notes: Option<&str>) -> CoreResult<i64> {
    conn.execute(
        "INSERT INTO saved_mod_link (name, url, notes) VALUES (?1, ?2, ?3)",
        rusqlite::params![name, url, notes],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Returns every saved link, most recently added first.
pub fn list(conn: &Connection) -> CoreResult<Vec<SavedModLink>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, url, notes, created_at FROM saved_mod_link ORDER BY id DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(SavedModLink {
            id: row.get(0)?,
            name: row.get(1)?,
            url: row.get(2)?,
            notes: row.get(3)?,
            created_at: row.get(4)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

/// Updates an existing bookmark's name/url/notes in place.
pub fn update(conn: &Connection, id: i64, name: &str, url: &str, notes: Option<&str>) -> CoreResult<()> {
    let rows = conn.execute(
        "UPDATE saved_mod_link SET name = ?2, url = ?3, notes = ?4 WHERE id = ?1",
        rusqlite::params![id, name, url, notes],
    )?;
    if rows == 0 {
        return Err(CoreError::UnsupportedFormat {
            reason: format!("no saved mod link with id {id}"),
        });
    }
    Ok(())
}

pub fn delete(conn: &Connection, id: i64) -> CoreResult<()> {
    let rows = conn.execute("DELETE FROM saved_mod_link WHERE id = ?1", [id])?;
    if rows == 0 {
        return Err(CoreError::UnsupportedFormat {
            reason: format!("no saved mod link with id {id}"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn add_list_update_delete_round_trip() {
        let conn = crate::db::open_in_memory().unwrap();
        assert!(list(&conn).unwrap().is_empty());

        let id = add(&conn, "Menyoo PC", "https://www.gta5-mods.com/scripts/menyoo-pc-sp", None).unwrap();
        let links = list(&conn).unwrap();
        assert_eq!(links.len(), 1);
        assert_eq!(links[0].id, id);
        assert_eq!(links[0].name, "Menyoo PC");
        assert!(links[0].notes.is_none());

        update(&conn, id, "Menyoo PC Trainer", "https://www.gta5-mods.com/scripts/menyoo-pc-sp", Some("great trainer")).unwrap();
        let links = list(&conn).unwrap();
        assert_eq!(links[0].name, "Menyoo PC Trainer");
        assert_eq!(links[0].notes.as_deref(), Some("great trainer"));

        delete(&conn, id).unwrap();
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn update_and_delete_of_unknown_id_error_cleanly() {
        let conn = crate::db::open_in_memory().unwrap();
        assert!(update(&conn, 999, "x", "y", None).is_err());
        assert!(delete(&conn, 999).is_err());
    }

    #[test]
    fn most_recently_added_link_comes_first() {
        let conn = crate::db::open_in_memory().unwrap();
        add(&conn, "First", "https://example.com/a", None).unwrap();
        add(&conn, "Second", "https://example.com/b", None).unwrap();
        let links = list(&conn).unwrap();
        assert_eq!(links[0].name, "Second");
        assert_eq!(links[1].name, "First");
    }
}
