// SPDX-License-Identifier: AGPL-3.0-only

//! Tauri command layer: thin IPC wrappers over `gtavmm-core`, mirroring the CLI's own
//! role — no business logic lives here, only argument marshalling and error-to-string
//! conversion for the frontend. Query/mutation logic itself is factored into plain
//! functions (`commands::*`) so it's unit-testable without spinning up a Tauri app.

mod commands;

use std::sync::Mutex;

use rusqlite::Connection;
use tauri::Manager;

pub struct AppState {
    pub conn: Mutex<Connection>,
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .setup(|app| {
            let db_path = gtavmm_core::db::default_db_path()
                .expect("could not resolve the app-data directory for the database");
            let conn = gtavmm_core::db::open(&db_path)
                .expect("failed to open/migrate the GTAV Mods Manager database");
            app.manage(AppState {
                conn: Mutex::new(conn),
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            commands::list_mods,
            commands::detect_game,
            commands::inspect_mod,
            commands::install_mod,
            commands::fivem_resolve_load_order,
            commands::fivem_apply_load_order,
            commands::convert_vehicle_pack,
            commands::get_language,
            commands::set_language,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
