-- SPDX-License-Identifier: AGPL-3.0-only
--
-- Initial schema. Applied once via PRAGMA user_version gating in db::mod::run_migrations.
-- See docs/planning (Obsidian vault, not part of this repo) 15-MVP-Detailed-Spec for the
-- design rationale behind each table.

CREATE TABLE IF NOT EXISTS game_installation (
    id            INTEGER PRIMARY KEY AUTOINCREMENT,
    platform      TEXT NOT NULL CHECK (platform IN ('windows', 'linux')),
    install_path  TEXT NOT NULL,
    edition       TEXT NOT NULL CHECK (edition IN ('legacy', 'enhanced')),
    detected_via  TEXT NOT NULL CHECK (detected_via IN ('registry', 'steam', 'epic', 'rockstar', 'manual')),
    created_at    TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

CREATE TABLE IF NOT EXISTS installed_mod (
    id           INTEGER PRIMARY KEY AUTOINCREMENT,
    name         TEXT NOT NULL,
    source_type  TEXT NOT NULL CHECK (source_type IN ('oiv', 'zip', 'sevenzip', 'folder', 'asi', 'dll', 'menyoo_xml')),
    install_path TEXT NOT NULL,
    installed_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    status       TEXT NOT NULL CHECK (status IN ('active', 'disabled', 'uninstalled')) DEFAULT 'active',
    notes        TEXT,
    link         TEXT
);

CREATE TABLE IF NOT EXISTS installed_mod_file (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    installed_mod_id INTEGER NOT NULL REFERENCES installed_mod(id) ON DELETE CASCADE,
    target_path      TEXT NOT NULL,
    backup_path      TEXT,
    file_hash        TEXT NOT NULL
);
CREATE INDEX IF NOT EXISTS idx_installed_mod_file_mod_id ON installed_mod_file(installed_mod_id);
CREATE INDEX IF NOT EXISTS idx_installed_mod_file_target_path ON installed_mod_file(target_path);

CREATE TABLE IF NOT EXISTS install_event (
    id               INTEGER PRIMARY KEY AUTOINCREMENT,
    installed_mod_id INTEGER REFERENCES installed_mod(id) ON DELETE SET NULL,
    event_type       TEXT NOT NULL CHECK (event_type IN ('install', 'uninstall', 'enable', 'disable', 'restore')),
    timestamp        TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    success          INTEGER NOT NULL CHECK (success IN (0, 1)),
    error_message    TEXT
);
CREATE INDEX IF NOT EXISTS idx_install_event_mod_id ON install_event(installed_mod_id);

CREATE TABLE IF NOT EXISTS user_settings (
    id                        INTEGER PRIMARY KEY CHECK (id = 1), -- singleton row
    language                  TEXT NOT NULL DEFAULT 'en',
    default_auto_backup       INTEGER NOT NULL CHECK (default_auto_backup IN (0, 1)) DEFAULT 1,
    game_install_path_override TEXT
);
INSERT OR IGNORE INTO user_settings (id) VALUES (1);

CREATE TABLE IF NOT EXISTS recycle_bin_entry (
    id                       INTEGER PRIMARY KEY AUTOINCREMENT,
    original_installed_mod_id INTEGER,
    mod_package_snapshot_path TEXT NOT NULL,
    deleted_at               TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now')),
    expires_at               TEXT NOT NULL -- deleted_at + 15 days, computed by the recycle_bin module
);
CREATE INDEX IF NOT EXISTS idx_recycle_bin_expires_at ON recycle_bin_entry(expires_at);
