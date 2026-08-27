-- SPDX-License-Identifier: AGPL-3.0-only
--
-- Schema version 2: multi-profile support. Applied via the same PRAGMA user_version
-- gating as schema.sql (see db::mod::run_migrations) — CREATE TABLE IF NOT EXISTS
-- makes this safe to run both on a brand-new database and as an upgrade on top of an
-- existing version-1 database.

-- `name` is intentionally not UNIQUE: importing a shared profile export whose name
-- happens to match an existing local profile (e.g. re-importing your own export)
-- must not fail — profiles are identified by `id`, not by name.
CREATE TABLE IF NOT EXISTS profile (
    id         INTEGER PRIMARY KEY AUTOINCREMENT,
    name       TEXT NOT NULL,
    created_at TEXT NOT NULL DEFAULT (strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))
);

-- Which mods should be active when a given profile is selected. A mod not listed for
-- any profile is left untouched by profile::switch (profile membership is opt-in per
-- mod, not mandatory for every installed mod).
CREATE TABLE IF NOT EXISTS profile_mod (
    profile_id       INTEGER NOT NULL REFERENCES profile(id) ON DELETE CASCADE,
    installed_mod_id INTEGER NOT NULL REFERENCES installed_mod(id) ON DELETE CASCADE,
    PRIMARY KEY (profile_id, installed_mod_id)
);
CREATE INDEX IF NOT EXISTS idx_profile_mod_installed_mod_id ON profile_mod(installed_mod_id);
