// SPDX-License-Identifier: AGPL-3.0-only

//! SQLite connection management and schema migration. No cloud/network component —
//! this is a purely local file, per the project's offline-first, no-first-party-server
//! design decision.

pub mod models;

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::CoreResult;

const SCHEMA_SQL: &str = include_str!("schema.sql");
const PROFILE_SCHEMA_SQL: &str = include_str!("profile_schema.sql");
const CURRENT_SCHEMA_VERSION: i32 = 8;

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

/// Runs each version-gated migration step in order, so both a brand-new database
/// (`user_version` 0) and an existing pre-profile-system database (`user_version` 1)
/// end up fully migrated. Every step is written to be safe to (re-)run via
/// `CREATE TABLE IF NOT EXISTS`, except the one genuine `ALTER TABLE`, which is
/// best-effort (ignored if the column already exists — happens on a fresh database,
/// since `profile_schema.sql`'s tables already exist by the time it runs there).
fn run_migrations(conn: &Connection) -> CoreResult<()> {
    let user_version: i32 = conn.query_row("PRAGMA user_version", [], |row| row.get(0))?;
    if user_version < 1 {
        conn.execute_batch(SCHEMA_SQL)?;
    }
    if user_version < 2 {
        conn.execute_batch(PROFILE_SCHEMA_SQL)?;
        let _ = conn.execute(
            "ALTER TABLE user_settings ADD COLUMN active_profile_id INTEGER REFERENCES profile(id)",
            [],
        );
    }
    if user_version < 3 {
        // AI Assistant System (opt-in) settings. The API key itself is never stored
        // here — see `ai_assistant`, which keeps it in the OS-native credential store.
        let _ = conn.execute(
            "ALTER TABLE user_settings ADD COLUMN ai_enabled INTEGER NOT NULL DEFAULT 0",
            [],
        );
        let _ = conn.execute("ALTER TABLE user_settings ADD COLUMN ai_provider TEXT", []);
        let _ = conn.execute(
            "ALTER TABLE user_settings ADD COLUMN ai_ollama_model TEXT",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE user_settings ADD COLUMN ai_cloud_endpoint TEXT",
            [],
        );
        let _ = conn.execute(
            "ALTER TABLE user_settings ADD COLUMN ai_cloud_model TEXT",
            [],
        );
    }
    if user_version < 4 {
        // AI Workflow / Prompt template library: the user's own reusable prompt text,
        // independent of the AI Assistant's Action Schema (see `ai_assistant` module
        // docs) — this is just a CRUD store, no automated execution involved.
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS prompt_template (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                name       TEXT NOT NULL,
                content    TEXT NOT NULL,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
                updated_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            );",
        )?;
    }
    if user_version < 5 {
        // Records the original mod package's source path at install time — needed
        // for `reinstall_mod` (AI Action Schema, see `ai_assistant::action_schema`)
        // to know what to reinstall from. Nullable: rows from before this migration,
        // and any install path that's since moved/deleted on disk, simply can't be
        // reinstalled from — that's a real, disclosed limitation, not hidden.
        let _ = conn.execute("ALTER TABLE installed_mod ADD COLUMN source_path TEXT", []);
    }
    if user_version < 6 {
        // Low-risk action auto-approve whitelist (design doc §3.3, v0.8+): a
        // comma-separated list of Action Schema action kinds (e.g.
        // "disable_mod,enable_mod") the user has opted out of per-instance approval
        // for. Empty/NULL means nothing is whitelisted — every action still needs
        // explicit approval by default, matching the design doc's "預設仍是逐次確認".
        let _ = conn.execute(
            "ALTER TABLE user_settings ADD COLUMN auto_approve_action_kinds TEXT",
            [],
        );
    }
    if user_version < 7 {
        // Standalone mod-link bookmarks (design request, 2026-08-30): a user's own
        // saved list of mod page URLs (e.g. gta5-mods.com) they want to come back to
        // later — deliberately independent of `installed_mod`, since a bookmark is
        // useful before a mod is ever installed (or after it's uninstalled).
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS saved_mod_link (
                id         INTEGER PRIMARY KEY AUTOINCREMENT,
                name       TEXT NOT NULL,
                url        TEXT NOT NULL,
                notes      TEXT,
                created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
            );",
        )?;
    }
    if user_version < 8 {
        // Tags a saved link into a UI tab (see `saved_links` module doc) — `NULL` is
        // the user's own general bookmark list, unaffected by anything below.
        let _ = conn.execute("ALTER TABLE saved_mod_link ADD COLUMN category TEXT", []);

        // Seeds the built-in "模組 Setup 建議" tab (user-provided list, 2026-08-30) —
        // the small set of prerequisite/setup tools almost every Legacy SP install
        // needs before mods themselves go on. Only runs once, at the point a database
        // crosses this migration, so a user who deletes one of these afterward doesn't
        // get it silently reintroduced on the next app start.
        let mod_setup_links: &[(&str, &str, &str)] = &[
            (
                "Script Hook V",
                "http://www.dev-c.com/gtav/scripthookv/",
                "Legacy 版 ASI 模組的核心基礎元件，幾乎所有 .asi 腳本模組都依賴它。只支援 Legacy 版，Enhanced 版須改用 RAGE Plugin Hook；版本必須跟遊戲更新同步，否則遊戲會啟動失敗。",
            ),
            (
                "ScriptHookVDotNet",
                "https://github.com/scripthookvdotnet/scripthookvdotnet-nightly/releases",
                "在 Script Hook V 之上加一層 .NET 執行環境，讓 C#/VB.NET 寫的 .dll 模組能運作，是安裝任何 ScriptHookVDotNet 腳本模組的必要元件。",
            ),
            (
                "Menyoo 2.0",
                "https://www.gta5-mods.com/scripts/menyoo-2-0",
                "知名的萬用生成器/模式編輯器，許多其他模組（載具刷出、地圖擺放）都是以 Menyoo 的 .xml 格式發佈，是 SP 模組生態很基礎的工具。",
            ),
            (
                "Gameconfig for Legacy & Enhanced",
                "https://www.gta5-mods.com/misc/gta-5-gameconfig-300-cars",
                "修改遊戲內部各種物件池（載具、行人、物件等）的數量上限，避免安裝大量 Add-on 模組後遊戲當機，必須跟遊戲版本相符才能使用。",
            ),
            (
                "KRYST4LCLR's Gameconfig",
                "https://www.gta5-mods.com/misc/kryst4lclr-s-gameconfig-updated-regularly",
                "另一個更新頻率較高的 gameconfig，社群普遍認為維護比較即時、遊戲更新後通常會更快跟進——跟上面的 Gameconfig 是二選一，不要同時安裝。",
            ),
            (
                "HeapAdjuster",
                "https://www.gta5-mods.com/tools/heapadjuster",
                "提高遊戲執行時的記憶體堆積（heap）上限，避免大量高解析度貼圖/模組同時載入時因為記憶體不足而當機。",
            ),
            (
                "Packfile Limit Adjuster",
                "https://www.gta5-mods.com/tools/packfile-limit-adjuster",
                "提高遊戲可同時載入的封包檔（RPF）數量上限，安裝大量 Add-on 模組（尤其車輛/地圖）到一定數量後常見的「packfile limit」當機就是靠這個解決。",
            ),
        ];
        for (name, url, notes) in mod_setup_links {
            conn.execute(
                "INSERT INTO saved_mod_link (name, url, notes, category) VALUES (?1, ?2, ?3, 'mod_setup')",
                rusqlite::params![name, url, notes],
            )?;
        }
    }
    if user_version < CURRENT_SCHEMA_VERSION {
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
