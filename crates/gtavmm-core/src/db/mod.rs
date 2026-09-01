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
const CURRENT_SCHEMA_VERSION: i32 = 9;

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
                "The core foundation almost every .asi script mod depends on for Legacy. Legacy-only — Enhanced needs RAGE Plugin Hook instead; the version must match your game build exactly or the game will fail to launch.",
            ),
            (
                "ScriptHookVDotNet",
                "https://github.com/scripthookvdotnet/scripthookvdotnet-nightly/releases",
                "Adds a .NET runtime layer on top of Script Hook V so C#/VB.NET .dll mods can run — required for any ScriptHookVDotNet script mod.",
            ),
            (
                "Menyoo 2.0",
                "https://www.gta5-mods.com/scripts/menyoo-2-0",
                "The well-known all-purpose trainer/spawner and map editor. Many other mods (vehicle spawn presets, map placements) are distributed as Menyoo .xml files, making this a foundational tool in the SP mod ecosystem.",
            ),
            (
                "Gameconfig for Legacy & Enhanced",
                "https://www.gta5-mods.com/misc/gta-5-gameconfig-300-cars",
                "Raises the internal object-pool limits (vehicles, peds, objects, etc.) to prevent crashes once you've installed a lot of add-on mods. Must match your game version to work correctly.",
            ),
            (
                "KRYST4LCLR's Gameconfig",
                "https://www.gta5-mods.com/misc/kryst4lclr-s-gameconfig-updated-regularly",
                "An alternative gameconfig that's updated more frequently — the community generally considers it more actively maintained and quicker to catch up after game updates. Pick one or the other, never both at once.",
            ),
            (
                "HeapAdjuster",
                "https://www.gta5-mods.com/tools/heapadjuster",
                "Raises the game's runtime memory heap limit, preventing crashes when many high-resolution textures/mods are loaded at once.",
            ),
            (
                "Packfile Limit Adjuster",
                "https://www.gta5-mods.com/tools/packfile-limit-adjuster",
                "Raises the limit on how many packfiles (RPFs) the game can load at once — the fix for the common \"packfile limit\" crash once you've installed enough add-on mods (especially vehicles/maps).",
            ),
        ];
        for (name, url, notes) in mod_setup_links {
            conn.execute(
                "INSERT INTO saved_mod_link (name, url, notes, category) VALUES (?1, ?2, ?3, 'mod_setup')",
                rusqlite::params![name, url, notes],
            )?;
        }
    }
    if user_version < 9 {
        // Seeds the built-in "LSPDFR Mods" tab (user-provided list, 2026-08-31, sourced
        // from lcpdfr.com — notes below were written after reading each mod's own page,
        // not guessed) — the small set of LSPDFR/EUP mods almost every police-roleplay
        // setup builds on. Same one-time-only seeding rule as the mod_setup_links above:
        // this only runs once, at the point a database crosses this migration.
        let lspdfr_links: &[(&str, &str, &str)] = &[
            (
                "LSPD First Response",
                "https://www.lcpdfr.com/downloads/gta5mods/g17media/7792-lspd-first-response/",
                "The core LSPDFR plugin that turns GTA V into a police roleplay game. Runs on RAGE Plugin Hook. Officially supported on Legacy Edition only — a separate Public Preview build exists for Enhanced Edition. Some antivirus software flags LSPDFR/RPH as a false positive due to its use of memory hooking.",
            ),
            (
                "Emergency uniforms pack - Law & Order",
                "https://www.lcpdfr.com/downloads/gta5mods/character/8151-emergency-uniforms-pack-law-order/",
                "The core Emergency Uniforms Pack (EUP) — an all-in-one player/ped clothing system with lore-friendly, California-inspired law-enforcement uniforms for male and female characters. Licensed CC BY-NC-SA 4.0. Pairs with EUP Serve & Rescue, EUP Menu, and the EUPFR configs below.",
            ),
            (
                "Emergency uniforms pack - Serve & Rescue",
                "https://www.lcpdfr.com/downloads/gta5mods/character/16256-emergency-uniforms-pack-serve-rescue/",
                "Companion pack to EUP Law & Order, adding non-law-enforcement agency outfits (fire, EMS, construction, security, etc). Requires the base EUP Law & Order pack to be installed first.",
            ),
            (
                "EUP Menu",
                "https://www.lcpdfr.com/downloads/gta5mods/scripts/13245-eup-menu/",
                "In-game RAGE Plugin Hook menu for browsing and applying EUP outfits to multiplayer peds, including a GTA Online-style character creator with save/share support. Requires RAGE Plugin Hook 0.37+, RAGENativeUI 1.8.1+, and EUP Law & Order 8.3+ (EUP Serve & Rescue is optional but recommended).",
            ),
            (
                "EUP Badges",
                "https://www.lcpdfr.com/downloads/gta5mods/misc/32225-eup-badges/",
                "Adds 15+ prop badges from EUP for LSPDFR's \"Flash Badge\" feature (0.4.2+). Packaged as an .oiv — install via OpenIV, then manually assign the badge model to each agency in your LSPDFR agency data files.",
            ),
            (
                "EUPFR - Ultimate Edition",
                "https://www.lcpdfr.com/downloads/gta5mods/datafile/32429-eupfr-ultimate-edition/",
                "LSPDFR agency data configs that let AI/backup peds wear EUP outfits, covering the full agency roster (RHPD, DPPD, LSPP, LSIA, BCSO, FIB). Install by dropping the \"custom\" folder into Grand Theft Auto V\\lspdfr\\data. Some agency variants expect specific third-party vehicle packs to match.",
            ),
            (
                "EUPFR Basic Configurations",
                "https://www.lcpdfr.com/downloads/gta5mods/datafile/22400-eupfr-basic-configurations/",
                "A lighter EUPFR config covering only the core agencies (LSPD, LSSD, SAHP, FIB, NOOSE, SASP, DOA, NYSP, SASPA, etc), without RHPD/DPPD/BCSO. Same install method as EUPFR Ultimate Edition, but relies only on the base EUP packs — no extra vehicle packs required.",
            ),
            (
                "Emergency Lighting System",
                "https://www.lcpdfr.com/downloads/gta5mods/scripts/13865-emergency-lighting-system/",
                "In-depth ELS lighting/siren control system (200+ patterns across 4 light groups) for emergency vehicles — the GTA V successor to the classic ELS-IV. Requires Script Hook V and vehicle models specifically built for ELS. Not compatible with multiplayer, and does not currently work on GTA V Enhanced Edition.",
            ),
        ];
        for (name, url, notes) in lspdfr_links {
            conn.execute(
                "INSERT INTO saved_mod_link (name, url, notes, category) VALUES (?1, ?2, ?3, 'lspdfr')",
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
