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

#[cfg(test)]
mod tests {
    use super::*;

    fn write_manifest(dir: &std::path::Path, name: &str, body: &str) {
        let resource_dir = dir.join(name);
        std::fs::create_dir_all(&resource_dir).unwrap();
        std::fs::write(resource_dir.join("fxmanifest.lua"), body).unwrap();
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
