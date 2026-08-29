// SPDX-License-Identifier: AGPL-3.0-only

//! Command implementations, factored as plain functions so they're unit-testable
//! without a running Tauri app — the `#[tauri::command]` wrappers below just marshal
//! `tauri::State`/error-to-string for the frontend.

use rusqlite::Connection;
use serde::Serialize;

use gtavmm_core::db::models::{InstalledMod, ModStatus};

/// Reads every `installed_mod` row (any status), ordered by id (install order).
pub fn list_mods_impl(conn: &Connection) -> Result<Vec<InstalledMod>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, source_type, install_path, installed_at, status, notes, link \
             FROM installed_mod ORDER BY id ASC",
        )
        .map_err(|e| e.to_string())?;
    let rows = stmt
        .query_map([], |row| {
            let status_str: String = row.get(5)?;
            let status = match status_str.as_str() {
                "active" => ModStatus::Active,
                "disabled" => ModStatus::Disabled,
                _ => ModStatus::Uninstalled,
            };
            Ok(InstalledMod {
                id: row.get(0)?,
                name: row.get(1)?,
                source_type: row.get(2)?,
                install_path: row.get(3)?,
                installed_at: row.get(4)?,
                status,
                notes: row.get(6)?,
                link: row.get(7)?,
            })
        })
        .map_err(|e| e.to_string())?;
    rows.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_mods(state: tauri::State<crate::AppState>) -> Result<Vec<InstalledMod>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    list_mods_impl(&conn)
}

/// Serializable summary of a `game_locator::detect()` outcome — `DetectResult`
/// itself isn't `Serialize`, and the frontend doesn't need the full enum shape.
#[derive(Debug, Clone, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum DetectGameResult {
    Found {
        install_path: String,
        edition: String,
    },
    NotFound,
}

pub fn detect_game_impl() -> Result<DetectGameResult, String> {
    use gtavmm_core::game_locator::DetectResult;
    match gtavmm_core::game_locator::detect() {
        Ok(DetectResult::Found(installation)) => Ok(DetectGameResult::Found {
            install_path: installation.install_path,
            edition: installation.edition,
        }),
        Ok(DetectResult::FoundUnsupportedEdition { .. }) | Ok(DetectResult::NotFound) => {
            Ok(DetectGameResult::NotFound)
        }
        Err(e) => Err(e.to_string()),
    }
}

#[tauri::command]
pub fn detect_game() -> Result<DetectGameResult, String> {
    detect_game_impl()
}

pub fn fivem_resolve_load_order_impl(resources_root: &str) -> Result<Vec<String>, String> {
    gtavmm_core::fivem::resolve_load_order(std::path::Path::new(resources_root))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fivem_resolve_load_order(resources_root: String) -> Result<Vec<String>, String> {
    fivem_resolve_load_order_impl(&resources_root)
}

pub fn fivem_apply_load_order_impl(
    resources_root: &str,
    server_cfg_path: &str,
) -> Result<Vec<String>, String> {
    gtavmm_core::fivem::apply_load_order(
        std::path::Path::new(resources_root),
        std::path::Path::new(server_cfg_path),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn fivem_apply_load_order(
    resources_root: String,
    server_cfg_path: String,
) -> Result<Vec<String>, String> {
    fivem_apply_load_order_impl(&resources_root, &server_cfg_path)
}

pub fn convert_vehicle_pack_impl(
    dlc_rpf: &str,
    output_dir: &str,
) -> Result<gtavmm_core::sp_to_fivem::ConversionReport, String> {
    gtavmm_core::sp_to_fivem::convert_vehicle_pack(
        std::path::Path::new(dlc_rpf),
        std::path::Path::new(output_dir),
    )
    .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn convert_vehicle_pack(
    dlc_rpf: String,
    output_dir: String,
) -> Result<gtavmm_core::sp_to_fivem::ConversionReport, String> {
    convert_vehicle_pack_impl(&dlc_rpf, &output_dir)
}

fn mode_from_str(mode: &str) -> Result<gtavmm_core::providers::Mode, String> {
    match mode {
        "sp" => Ok(gtavmm_core::providers::Mode::Sp),
        "lspdfr" => Ok(gtavmm_core::providers::Mode::Lspdfr),
        "fivem-client" => Ok(gtavmm_core::providers::Mode::FivemClient),
        other => Err(format!("unknown mode: {other} (expected sp/lspdfr/fivem-client)")),
    }
}

/// Read-only preview of what installing `path` would do — no files written, nothing
/// recorded in the database. Mirrors the CLI's `inspect` command.
pub fn inspect_mod_impl(
    game_path: Option<&str>,
    mode: &str,
    path: &str,
) -> Result<gtavmm_core::mod_analyzer::ModPlan, String> {
    let core_mode = mode_from_str(mode)?;
    let (_, provider) = gtavmm_core::providers::resolve(game_path.map(std::path::Path::new), core_mode)
        .map_err(|e| e.to_string())?;
    gtavmm_core::mod_analyzer::classify(std::path::Path::new(path), provider.as_ref())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn inspect_mod(
    game_path: Option<String>,
    mode: String,
    path: String,
) -> Result<gtavmm_core::mod_analyzer::ModPlan, String> {
    inspect_mod_impl(game_path.as_deref(), &mode, &path)
}

/// Installs `path` for real. Mirrors the CLI's `install` command: classify, then run
/// the full install pipeline (conflict check, backup, write, record). `backup_root` is
/// a parameter (not computed internally) so this stays unit-testable against a temp
/// directory — the `#[tauri::command]` wrapper below points it at the real app-data
/// directory.
pub fn install_mod_impl(
    conn: &mut Connection,
    game_path: Option<&str>,
    mode: &str,
    path: &str,
    name: Option<&str>,
    override_foreign_conflicts: bool,
    backup_root: &std::path::Path,
) -> Result<gtavmm_core::install::InstallOutcome, String> {
    let core_mode = mode_from_str(mode)?;
    let input_path = std::path::Path::new(path);
    let (game_root, provider) =
        gtavmm_core::providers::resolve(game_path.map(std::path::Path::new), core_mode)
            .map_err(|e| e.to_string())?;
    let plan = gtavmm_core::mod_analyzer::classify(input_path, provider.as_ref())
        .map_err(|e| e.to_string())?;
    let name = name.map(str::to_string).unwrap_or_else(|| {
        input_path
            .file_name()
            .map(|n| n.to_string_lossy().into_owned())
            .unwrap_or_else(|| "unnamed mod".to_string())
    });

    let options = gtavmm_core::install::InstallOptions {
        auto_backup: true,
        override_foreign_conflicts,
    };
    gtavmm_core::install::install(conn, &name, &plan, &game_root, backup_root, options, input_path)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn install_mod(
    state: tauri::State<crate::AppState>,
    game_path: Option<String>,
    mode: String,
    path: String,
    name: Option<String>,
    override_foreign_conflicts: bool,
) -> Result<gtavmm_core::install::InstallOutcome, String> {
    let mut conn = state.conn.lock().map_err(|e| e.to_string())?;
    let db_path = gtavmm_core::db::default_db_path()
        .ok_or_else(|| "could not resolve an app-data directory on this OS".to_string())?;
    let backup_root = db_path
        .parent()
        .expect("db path always has a parent")
        .join("backups")
        .join(chrono::Utc::now().format("%Y%m%d%H%M%S%3f").to_string());
    install_mod_impl(
        &mut conn,
        game_path.as_deref(),
        &mode,
        &path,
        name.as_deref(),
        override_foreign_conflicts,
        &backup_root,
    )
}

pub fn profile_list_impl(conn: &Connection) -> Result<Vec<gtavmm_core::profile::Profile>, String> {
    gtavmm_core::profile::list(conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn profile_list(
    state: tauri::State<crate::AppState>,
) -> Result<Vec<gtavmm_core::profile::Profile>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    profile_list_impl(&conn)
}

pub fn profile_create_impl(conn: &Connection, name: &str) -> Result<i64, String> {
    gtavmm_core::profile::create(conn, name).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn profile_create(state: tauri::State<crate::AppState>, name: String) -> Result<i64, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    profile_create_impl(&conn, &name)
}

pub fn profile_delete_impl(conn: &Connection, profile_id: i64) -> Result<(), String> {
    gtavmm_core::profile::delete(conn, profile_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn profile_delete(state: tauri::State<crate::AppState>, profile_id: i64) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    profile_delete_impl(&conn, profile_id)
}

pub fn profile_mod_ids_impl(conn: &Connection, profile_id: i64) -> Result<Vec<i64>, String> {
    gtavmm_core::profile::mod_ids_in_profile(conn, profile_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn profile_mod_ids(
    state: tauri::State<crate::AppState>,
    profile_id: i64,
) -> Result<Vec<i64>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    profile_mod_ids_impl(&conn, profile_id)
}

pub fn profile_add_mod_impl(conn: &Connection, profile_id: i64, mod_id: i64) -> Result<(), String> {
    gtavmm_core::profile::add_mod(conn, profile_id, mod_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn profile_add_mod(
    state: tauri::State<crate::AppState>,
    profile_id: i64,
    mod_id: i64,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    profile_add_mod_impl(&conn, profile_id, mod_id)
}

pub fn profile_remove_mod_impl(conn: &Connection, profile_id: i64, mod_id: i64) -> Result<(), String> {
    gtavmm_core::profile::remove_mod(conn, profile_id, mod_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn profile_remove_mod(
    state: tauri::State<crate::AppState>,
    profile_id: i64,
    mod_id: i64,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    profile_remove_mod_impl(&conn, profile_id, mod_id)
}

/// Switches the active profile. `staging_root` is a parameter (not computed
/// internally) for the same testability reason as `install_mod_impl`.
pub fn profile_switch_impl(
    conn: &Connection,
    profile_id: i64,
    staging_root: &std::path::Path,
) -> Result<gtavmm_core::profile::SwitchOutcome, String> {
    gtavmm_core::profile::switch(conn, profile_id, staging_root).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn profile_switch(
    state: tauri::State<crate::AppState>,
    profile_id: i64,
) -> Result<gtavmm_core::profile::SwitchOutcome, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let db_path = gtavmm_core::db::default_db_path()
        .ok_or_else(|| "could not resolve an app-data directory on this OS".to_string())?;
    let staging_root = db_path.parent().expect("db path always has a parent").join("staging");
    profile_switch_impl(&conn, profile_id, &staging_root)
}

pub fn get_language_impl(conn: &Connection) -> Result<String, String> {
    gtavmm_core::settings::load(conn)
        .map(|s| s.language)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn get_language(state: tauri::State<crate::AppState>) -> Result<String, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    get_language_impl(&conn)
}

pub fn set_language_impl(conn: &Connection, language: &str) -> Result<(), String> {
    let mut settings = gtavmm_core::settings::load(conn).map_err(|e| e.to_string())?;
    settings.language = language.to_string();
    gtavmm_core::settings::save(conn, &settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_language(
    state: tauri::State<crate::AppState>,
    language: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    set_language_impl(&conn, &language)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_manifest(dir: &std::path::Path, name: &str, body: &str) {
        let resource_dir = dir.join(name);
        std::fs::create_dir_all(&resource_dir).unwrap();
        std::fs::write(resource_dir.join("fxmanifest.lua"), body).unwrap();
    }

    /// A fake game install directory that `game_locator::classify_edition` will
    /// recognize as Legacy — enough for `providers::resolve` to succeed against it.
    fn fake_game_root() -> tempfile::TempDir {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("GTA5.exe"), b"").unwrap();
        dir
    }

    #[test]
    fn inspect_mod_impl_classifies_a_real_asi_file() {
        let game_dir = fake_game_root();
        let mod_dir = tempfile::tempdir().unwrap();
        let asi_path = mod_dir.path().join("SomeMod.asi");
        std::fs::write(&asi_path, b"fake asi bytes").unwrap();

        let plan = inspect_mod_impl(
            Some(game_dir.path().to_str().unwrap()),
            "sp",
            asi_path.to_str().unwrap(),
        )
        .unwrap();
        assert_eq!(plan.format, gtavmm_core::mod_analyzer::ModFormat::Asi);
        assert_eq!(plan.files.len(), 1);
        assert_eq!(plan.files[0].target, game_dir.path().join("SomeMod.asi"));
    }

    #[test]
    fn inspect_mod_impl_rejects_an_unknown_mode() {
        let game_dir = fake_game_root();
        let err = inspect_mod_impl(Some(game_dir.path().to_str().unwrap()), "not-a-mode", "x.asi")
            .unwrap_err();
        assert!(err.contains("unknown mode"));
    }

    #[test]
    fn install_mod_impl_writes_the_file_and_records_it() {
        let mut conn = gtavmm_core::db::open_in_memory().unwrap();
        let game_dir = fake_game_root();
        let mod_dir = tempfile::tempdir().unwrap();
        let backup_dir = tempfile::tempdir().unwrap();
        let asi_path = mod_dir.path().join("SomeMod.asi");
        std::fs::write(&asi_path, b"fake asi bytes").unwrap();

        let outcome = install_mod_impl(
            &mut conn,
            Some(game_dir.path().to_str().unwrap()),
            "sp",
            asi_path.to_str().unwrap(),
            None,
            false,
            backup_dir.path(),
        )
        .unwrap();

        match outcome {
            gtavmm_core::install::InstallOutcome::Success { files_written, .. } => {
                assert_eq!(files_written, 1);
            }
            other => panic!("expected Success, got {other:?}"),
        }
        assert!(game_dir.path().join("SomeMod.asi").exists());
        assert_eq!(list_mods_impl(&conn).unwrap().len(), 1);
    }

    #[test]
    fn install_mod_impl_rejects_an_unsupported_extension_before_touching_the_filesystem() {
        let mut conn = gtavmm_core::db::open_in_memory().unwrap();
        let game_dir = fake_game_root();
        let mod_dir = tempfile::tempdir().unwrap();
        let backup_dir = tempfile::tempdir().unwrap();
        let weird_path = mod_dir.path().join("mystery.bin");
        std::fs::write(&weird_path, b"???").unwrap();

        let err = install_mod_impl(
            &mut conn,
            Some(game_dir.path().to_str().unwrap()),
            "sp",
            weird_path.to_str().unwrap(),
            None,
            false,
            backup_dir.path(),
        )
        .unwrap_err();
        assert!(err.contains("unsupported"));
        assert!(list_mods_impl(&conn).unwrap().is_empty());
    }

    fn insert_mod_row(conn: &Connection, name: &str, status: &str) -> i64 {
        conn.execute(
            "INSERT INTO installed_mod (name, source_type, install_path, status) \
             VALUES (?1, 'asi', '', ?2)",
            rusqlite::params![name, status],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn profile_lifecycle_create_membership_switch() {
        let conn = gtavmm_core::db::open_in_memory().unwrap();
        let staging = tempfile::tempdir().unwrap();

        let profile_id = profile_create_impl(&conn, "Roleplay").unwrap();
        assert_eq!(profile_list_impl(&conn).unwrap().len(), 1);

        let mod_id = insert_mod_row(&conn, "SomeMod", "disabled");
        profile_add_mod_impl(&conn, profile_id, mod_id).unwrap();
        assert_eq!(profile_mod_ids_impl(&conn, profile_id).unwrap(), vec![mod_id]);

        // Switching should enable the disabled mod belonging to this profile. There
        // are no files to actually move for it (no installed_mod_file row), so this
        // exercises the bookkeeping path, not real file I/O — that's covered by
        // gtavmm_core::profile's own tests.
        let outcome = profile_switch_impl(&conn, profile_id, staging.path()).unwrap();
        assert_eq!(outcome.enabled, vec![mod_id]);

        profile_remove_mod_impl(&conn, profile_id, mod_id).unwrap();
        assert!(profile_mod_ids_impl(&conn, profile_id).unwrap().is_empty());

        profile_delete_impl(&conn, profile_id).unwrap();
        assert!(profile_list_impl(&conn).unwrap().is_empty());
    }

    #[test]
    fn convert_vehicle_pack_impl_errors_cleanly_on_a_missing_dlc_rpf() {
        let dir = tempfile::tempdir().unwrap();
        let err = convert_vehicle_pack_impl(
            dir.path().join("nope.rpf").to_str().unwrap(),
            dir.path().join("out").to_str().unwrap(),
        )
        .unwrap_err();
        assert!(err.contains("SP → FiveM"));
    }

    #[test]
    fn language_defaults_to_en_and_round_trips_through_set() {
        let conn = gtavmm_core::db::open_in_memory().unwrap();
        assert_eq!(get_language_impl(&conn).unwrap(), "en");

        set_language_impl(&conn, "zh-TW").unwrap();
        assert_eq!(get_language_impl(&conn).unwrap(), "zh-TW");
    }

    #[test]
    fn fivem_resolve_load_order_impl_orders_by_dependency() {
        let dir = tempfile::tempdir().unwrap();
        write_manifest(dir.path(), "core-lib", "fx_version 'cerulean'\n");
        write_manifest(dir.path(), "framework", "dependency 'core-lib'\n");

        let order = fivem_resolve_load_order_impl(dir.path().to_str().unwrap()).unwrap();
        let pos = |n: &str| order.iter().position(|x| x == n).unwrap();
        assert!(pos("core-lib") < pos("framework"));
    }

    #[test]
    fn fivem_apply_load_order_impl_writes_server_cfg() {
        let dir = tempfile::tempdir().unwrap();
        write_manifest(dir.path(), "core-lib", "fx_version 'cerulean'\n");
        let server_cfg = dir.path().join("server.cfg");
        std::fs::write(&server_cfg, "sv_hostname \"Test\"\n").unwrap();

        fivem_apply_load_order_impl(
            dir.path().to_str().unwrap(),
            server_cfg.to_str().unwrap(),
        )
        .unwrap();

        let contents = std::fs::read_to_string(&server_cfg).unwrap();
        assert!(contents.contains("sv_hostname \"Test\""));
        assert!(contents.contains("ensure core-lib"));
    }

    #[test]
    fn list_mods_impl_reads_real_rows_in_id_order() {
        let conn = gtavmm_core::db::open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO installed_mod (name, source_type, install_path, status) \
             VALUES ('Mod A', 'asi', '', 'active')",
            [],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO installed_mod (name, source_type, install_path, status) \
             VALUES ('Mod B', 'dll', '', 'disabled')",
            [],
        )
        .unwrap();

        let mods = list_mods_impl(&conn).unwrap();
        assert_eq!(mods.len(), 2);
        assert_eq!(mods[0].name, "Mod A");
        assert_eq!(mods[0].status, ModStatus::Active);
        assert_eq!(mods[1].name, "Mod B");
        assert_eq!(mods[1].status, ModStatus::Disabled);
    }

    #[test]
    fn list_mods_impl_empty_db_returns_empty_vec() {
        let conn = gtavmm_core::db::open_in_memory().unwrap();
        assert!(list_mods_impl(&conn).unwrap().is_empty());
    }

    #[test]
    fn detect_game_result_serializes_with_a_tagged_shape() {
        let found = DetectGameResult::Found {
            install_path: "C:/Games/GTAV".to_string(),
            edition: "legacy".to_string(),
        };
        let json = serde_json::to_string(&found).unwrap();
        assert_eq!(
            json,
            r#"{"status":"found","install_path":"C:/Games/GTAV","edition":"legacy"}"#
        );
    }
}
