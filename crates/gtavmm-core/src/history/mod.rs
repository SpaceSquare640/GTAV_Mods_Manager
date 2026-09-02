// SPDX-License-Identifier: AGPL-3.0-only

//! Read-only queries over `install_event`, used by the CLI's `history` command.

use rusqlite::Connection;

use crate::db::models::{EventType, InstallEvent};
use crate::error::CoreResult;

fn parse_event_type(s: &str) -> EventType {
    match s {
        "install" => EventType::Install,
        "uninstall" => EventType::Uninstall,
        "enable" => EventType::Enable,
        "disable" => EventType::Disable,
        "restore" => EventType::Restore,
        other => unreachable!("unknown event_type in db: {other}"),
    }
}

/// Returns all events, most recent first. `mod_id` optionally filters to one mod.
pub fn list(conn: &Connection, mod_id: Option<i64>) -> CoreResult<Vec<InstallEvent>> {
    list_filtered(conn, mod_id, None)
}

/// As [`list`], with an optional filter to one workspace page.
///
/// The page lives on `installed_mod`, not on the event, so this joins. Events
/// whose mod row is gone keep `installed_mod_id` NULL (the FK is
/// `ON DELETE SET NULL`) and so belong to no page — the page filter drops them,
/// which is right for a per-page history and wrong for the global one. Hence
/// two entry points rather than one with a nullable argument nobody reads.
pub fn list_filtered(
    conn: &Connection,
    mod_id: Option<i64>,
    mode: Option<&str>,
) -> CoreResult<Vec<InstallEvent>> {
    let mut stmt = conn.prepare(
        "SELECT e.id, e.installed_mod_id, e.event_type, e.timestamp, e.success, e.error_message \
         FROM install_event e \
         LEFT JOIN installed_mod m ON m.id = e.installed_mod_id \
         WHERE (?1 IS NULL OR e.installed_mod_id = ?1) \
           AND (?2 IS NULL OR m.mode = ?2) \
         ORDER BY e.timestamp DESC",
    )?;
    let rows = stmt.query_map(rusqlite::params![mod_id, mode], |row| {
        let event_type: String = row.get(2)?;
        Ok(InstallEvent {
            id: row.get(0)?,
            installed_mod_id: row.get(1)?,
            event_type: parse_event_type(&event_type),
            timestamp: row.get(3)?,
            success: row.get::<_, i64>(4)? != 0,
            error_message: row.get(5)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn list_is_empty_on_fresh_db() {
        let conn = crate::db::open_in_memory().unwrap();
        assert!(list(&conn, None).unwrap().is_empty());
    }

    #[test]
    fn filtering_by_page_returns_only_that_pages_events() {
        let conn = crate::db::open_in_memory().unwrap();
        for (name, mode) in [("Sp mod", "legacy-sp"), ("Lspdfr mod", "legacy-lspdfr")] {
            conn.execute(
                "INSERT INTO installed_mod (name, source_type, install_path, status, mode)
                 VALUES (?1, 'folder', '/x', 'active', ?2)",
                rusqlite::params![name, mode],
            )
            .unwrap();
            let id = conn.last_insert_rowid();
            conn.execute(
                "INSERT INTO install_event (installed_mod_id, event_type, success)
                 VALUES (?1, 'install', 1)",
                [id],
            )
            .unwrap();
        }
        // An event whose mod row was deleted: it belongs to no page.
        conn.execute(
            "INSERT INTO install_event (installed_mod_id, event_type, success)
             VALUES (NULL, 'uninstall', 1)",
            [],
        )
        .unwrap();

        assert_eq!(
            list(&conn, None).unwrap().len(),
            3,
            "the global view keeps all three"
        );
        assert_eq!(
            list_filtered(&conn, None, Some("legacy-sp")).unwrap().len(),
            1
        );
        assert_eq!(
            list_filtered(&conn, None, Some("legacy-lspdfr"))
                .unwrap()
                .len(),
            1
        );
    }
}
