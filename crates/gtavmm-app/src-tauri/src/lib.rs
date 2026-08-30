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
            commands::inspect_dll,
            commands::translate_dll_draft,
            commands::patch_dll_translations,
            commands::profile_list,
            commands::profile_create,
            commands::profile_delete,
            commands::profile_mod_ids,
            commands::profile_add_mod,
            commands::profile_remove_mod,
            commands::profile_switch,
            commands::fivem_resolve_load_order,
            commands::fivem_apply_load_order,
            commands::convert_vehicle_pack,
            commands::get_language,
            commands::set_language,
            commands::list_history,
            commands::list_saved_links,
            commands::add_saved_link,
            commands::update_saved_link,
            commands::delete_saved_link,
            commands::update_mod_details,
            commands::read_app_log,
            commands::app_log_path,
            commands::clear_app_log,
            commands::app_log_last_cleanup,
            commands::ai_load_settings,
            commands::ai_enable,
            commands::ai_disable,
            commands::ai_set_cloud_api_key,
            commands::ai_has_cloud_api_key,
            commands::ai_ollama_available,
            commands::ai_diagnose,
            commands::check_components,
            commands::create_full_backup,
            commands::list_full_backups,
            commands::restore_full_backup,
            commands::list_recycle_bin,
            commands::restore_recycle_bin_entry,
            commands::sweep_expired_recycle_bin,
            commands::list_prompt_templates,
            commands::add_prompt_template,
            commands::update_prompt_template,
            commands::delete_prompt_template,
            commands::export_mods_to_xlsx,
            commands::read_text_file,
            commands::write_text_file,
            commands::compute_file_hashes,
            commands::scan_mod_path,
            commands::check_for_update,
            commands::search_mods,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
