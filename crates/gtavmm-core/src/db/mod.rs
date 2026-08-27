// SPDX-License-Identifier: AGPL-3.0-only

//! SQLite connection management and schema migration. No cloud/network component —
//! this is a purely local file, per the project's offline-first, no-first-party-server
//! design decision.

pub mod models;

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::CoreResult;

const SCHEMA_SQL: &str = include_str!("schema.sql");
const CURRENT_SCHEMA_VERSION: i32 = 1;

/// Resolves the default database file location under the OS-appropriate app-data
/// directory (via the `directories` crate), e.g.
/// `%APPDATA%/GTAVModsManager/gtavmm.sqlite3` on Windows or
/// `~/.local/share/gtavmm/gtavmm.sqlite3` on Linux.
pub fn default_db_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "SpaceSquare", "GTAVModsManager")
        .map(|dirs| dirs.data_dir().join("gtavmm.sqlite3"))
}

/// Opens (creating if necessary) the database at `path` and applies pending migrations.
pub fn open(path: &Path) -> CoreResult<Connection> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let conn = Connection::open(path)?;
    conn.pragma_update(None, "foreign_keys", true)?;
    run_migrations(&conn)?;
    Ok(conn)
}

/// Opens an in-memory database with the schema applied — used by tests and by any
/// future integration test that wants a fake game install without touching real
/// app-data on disk.
pub fn open_in_memory() -> CoreResult<Connection> {
    let conn = Connection::open_in_memory()?;
    conn.pragma_update(None, "foreign_keys", true)?;
    run_migrations(&conn)?;
    Ok(conn)
}

fn run_migrations(conn: &Connection) -> CoreResult<()> {
    let user_version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if user_version < CURRENT_SCHEMA_VERSION {
        conn.execute_batch(SCHEMA_SQL)?;
        conn.pragma_update(None, "user_version", CURRENT_SCHEMA_VERSION)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn in_memory_db_applies_schema_and_seeds_singleton_settings_row() {
        let conn = open_in_memory().expect("schema should apply cleanly");
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM user_settings", [], |row| row.get(0))
            .unwrap();
        assert_eq!(count, 1, "user_settings singleton row should be seeded");
    }

    #[test]
    fn migrations_are_idempotent() {
        let conn = open_in_memory().unwrap();
        // Re-running should not error (CREATE TABLE IF NOT EXISTS + version gate).
        run_migrations(&conn).unwrap();
    }
}
