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
    let mut stmt = conn.prepare(
        "SELECT id, installed_mod_id, event_type, timestamp, success, error_message \
         FROM install_event \
         WHERE ?1 IS NULL OR installed_mod_id = ?1 \
         ORDER BY timestamp DESC",
    )?;
    let rows = stmt.query_map([mod_id], |row| {
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
}
