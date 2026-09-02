// SPDX-License-Identifier: AGPL-3.0-only

//! Loads/saves the singleton `user_settings` row.

use rusqlite::Connection;

use crate::db::models::UserSettings;
use crate::error::CoreResult;

pub fn load(conn: &Connection) -> CoreResult<UserSettings> {
    let settings = conn.query_row(
        "SELECT language, default_auto_backup, game_install_path_override, \
                theme, terms_accepted_version, onboarding_completed, backup_root_override \
         FROM user_settings WHERE id = 1",
        [],
        |row| {
            Ok(UserSettings {
                language: row.get(0)?,
                default_auto_backup: row.get::<_, i64>(1)? != 0,
                game_install_path_override: row.get(2)?,
                theme: row.get(3)?,
                terms_accepted_version: row.get(4)?,
                onboarding_completed: row.get::<_, i64>(5)? != 0,
                backup_root_override: row.get(6)?,
            })
        },
    )?;
    Ok(settings)
}

pub fn save(conn: &Connection, settings: &UserSettings) -> CoreResult<()> {
    conn.execute(
        "UPDATE user_settings \
         SET language = ?1, default_auto_backup = ?2, game_install_path_override = ?3, \
             theme = ?4, terms_accepted_version = ?5, onboarding_completed = ?6, \
             backup_root_override = ?7 \
         WHERE id = 1",
        rusqlite::params![
            settings.language,
            settings.default_auto_backup as i64,
            settings.game_install_path_override,
            settings.theme,
            settings.terms_accepted_version,
            settings.onboarding_completed as i64,
            settings.backup_root_override,
        ],
    )?;
    Ok(())
}

/// The terms version the application currently ships. Bumping this is what makes
/// the gate ask again after the text has materially changed; leaving it alone
/// means an existing acceptance still stands.
pub const CURRENT_TERMS_VERSION: &str = "1";

/// Whether the accepted version matches what the application is showing now.
pub fn has_accepted_current_terms(settings: &UserSettings) -> bool {
    settings.terms_accepted_version.as_deref() == Some(CURRENT_TERMS_VERSION)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn load_returns_seeded_defaults() {
        let conn = crate::db::open_in_memory().unwrap();
        let settings = load(&conn).unwrap();
        assert_eq!(settings, UserSettings::default());
    }

    #[test]
    fn save_then_load_roundtrips() {
        let conn = crate::db::open_in_memory().unwrap();
        let updated = UserSettings {
            language: "zh-TW".to_string(),
            default_auto_backup: false,
            game_install_path_override: Some(r"D:\Games\GTAV".to_string()),
            theme: Some("dark".to_string()),
            terms_accepted_version: Some(CURRENT_TERMS_VERSION.to_string()),
            onboarding_completed: true,
            backup_root_override: Some(r"E:\Backups".to_string()),
        };
        save(&conn, &updated).unwrap();
        assert_eq!(load(&conn).unwrap(), updated);
    }

    #[test]
    fn a_fresh_install_has_not_accepted_the_terms() {
        let conn = crate::db::open_in_memory().unwrap();
        assert!(!has_accepted_current_terms(&load(&conn).unwrap()));
    }

    #[test]
    fn accepting_an_older_terms_version_does_not_count_as_accepting_this_one() {
        // The point of storing a version rather than a flag: if the terms are
        // revised, a previous acceptance must stop counting so the gate asks
        // again, instead of passing silently on stale consent.
        let conn = crate::db::open_in_memory().unwrap();
        let mut settings = load(&conn).unwrap();
        settings.terms_accepted_version = Some("0".to_string());
        save(&conn, &settings).unwrap();

        let reloaded = load(&conn).unwrap();
        assert_eq!(reloaded.terms_accepted_version.as_deref(), Some("0"));
        assert!(!has_accepted_current_terms(&reloaded));

        settings.terms_accepted_version = Some(CURRENT_TERMS_VERSION.to_string());
        save(&conn, &settings).unwrap();
        assert!(has_accepted_current_terms(&load(&conn).unwrap()));
    }
}
