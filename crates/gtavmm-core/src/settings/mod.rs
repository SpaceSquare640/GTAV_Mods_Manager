// SPDX-License-Identifier: AGPL-3.0-only

//! Loads/saves the singleton `user_settings` row.

use rusqlite::Connection;

use crate::db::models::UserSettings;
use crate::error::CoreResult;

pub fn load(conn: &Connection) -> CoreResult<UserSettings> {
    let settings = conn.query_row(
        "SELECT language, default_auto_backup, game_install_path_override \
         FROM user_settings WHERE id = 1",
        [],
        |row| {
            Ok(UserSettings {
                language: row.get(0)?,
                default_auto_backup: row.get::<_, i64>(1)? != 0,
                game_install_path_override: row.get(2)?,
            })
        },
    )?;
    Ok(settings)
}

pub fn save(conn: &Connection, settings: &UserSettings) -> CoreResult<()> {
    conn.execute(
        "UPDATE user_settings \
         SET language = ?1, default_auto_backup = ?2, game_install_path_override = ?3 \
         WHERE id = 1",
        rusqlite::params![
            settings.language,
            settings.default_auto_backup as i64,
            settings.game_install_path_override,
        ],
    )?;
    Ok(())
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
        };
        save(&conn, &updated).unwrap();
        assert_eq!(load(&conn).unwrap(), updated);
    }
}
