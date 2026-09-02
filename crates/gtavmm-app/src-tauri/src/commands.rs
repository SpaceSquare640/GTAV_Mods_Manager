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
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(|e| e.to_string())
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
    let result = gtavmm_core::fivem::apply_load_order(
        std::path::Path::new(resources_root),
        std::path::Path::new(server_cfg_path),
    )
    .map_err(|e| e.to_string());
    if let Err(reason) = &result {
        let _ = gtavmm_core::app_log::error(&format!(
            "fivem_apply_load_order failed for '{resources_root}' -> '{server_cfg_path}': {reason}"
        ));
    }
    result
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
    let result = gtavmm_core::sp_to_fivem::convert_vehicle_pack(
        std::path::Path::new(dlc_rpf),
        std::path::Path::new(output_dir),
    )
    .map_err(|e| e.to_string());
    if let Err(reason) = &result {
        let _ = gtavmm_core::app_log::error(&format!(
            "convert_vehicle_pack failed for '{dlc_rpf}' -> '{output_dir}': {reason}"
        ));
    }
    result
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
        other => Err(format!(
            "unknown mode: {other} (expected sp/lspdfr/fivem-client)"
        )),
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
    let (_, provider) =
        gtavmm_core::providers::resolve(game_path.map(std::path::Path::new), core_mode)
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
    let result = gtavmm_core::install::install(
        conn,
        &name,
        &plan,
        &game_root,
        backup_root,
        options,
        input_path,
    )
    .map_err(|e| e.to_string());
    if let Err(reason) = &result {
        let _ = gtavmm_core::app_log::error(&format!("install_mod failed for '{name}': {reason}"));
    }
    result
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
    // Each install gets its own timestamped folder under backups/, so restoring
    // one mod never disturbs what another mod replaced.
    let backup_root =
        default_backup_root()?.join(chrono::Utc::now().format("%Y%m%d%H%M%S%3f").to_string());
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

pub fn profile_remove_mod_impl(
    conn: &Connection,
    profile_id: i64,
    mod_id: i64,
) -> Result<(), String> {
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
    let result =
        gtavmm_core::profile::switch(conn, profile_id, staging_root).map_err(|e| e.to_string());
    if let Err(reason) = &result {
        let _ = gtavmm_core::app_log::error(&format!(
            "profile_switch failed for profile #{profile_id}: {reason}"
        ));
    }
    result
}

#[tauri::command]
pub fn profile_switch(
    state: tauri::State<crate::AppState>,
    profile_id: i64,
) -> Result<gtavmm_core::profile::SwitchOutcome, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let staging_root = default_staging_root()?;
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
pub fn set_language(state: tauri::State<crate::AppState>, language: String) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    set_language_impl(&conn, &language)
}

pub fn inspect_dll_impl(
    dll_path: &str,
) -> Result<gtavmm_core::dll_translation::DllInspection, String> {
    gtavmm_core::dll_translation::inspect(std::path::Path::new(dll_path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn inspect_dll(
    dll_path: String,
) -> Result<gtavmm_core::dll_translation::DllInspection, String> {
    inspect_dll_impl(&dll_path)
}

pub fn translate_dll_draft_impl(
    conn: &Connection,
    dll_path: &str,
    target_language: &str,
) -> Result<Vec<gtavmm_core::dll_translation::TranslatedDraftEntry>, String> {
    let result = gtavmm_core::dll_translation::translate_draft(
        conn,
        std::path::Path::new(dll_path),
        target_language,
        15,
    )
    .map_err(|e| e.to_string());
    if let Err(reason) = &result {
        let _ = gtavmm_core::app_log::error(&format!(
            "translate_dll_draft failed for '{dll_path}': {reason}"
        ));
    }
    result
}

/// Step 1 of the review flow: AI-translates every candidate string but writes nothing
/// — the frontend shows these to the user (editable) before [`patch_dll_translations`]
/// commits anything.
#[tauri::command]
pub fn translate_dll_draft(
    state: tauri::State<crate::AppState>,
    dll_path: String,
    target_language: String,
) -> Result<Vec<gtavmm_core::dll_translation::TranslatedDraftEntry>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    translate_dll_draft_impl(&conn, &dll_path, &target_language)
}

pub fn patch_dll_translations_impl(
    dll_path: &str,
    target_language: &str,
    translations: Vec<String>,
    output_path: Option<&str>,
) -> Result<gtavmm_core::dll_translation::DllTranslationOutcome, String> {
    let output_path = output_path.map(std::path::Path::new);
    let result = gtavmm_core::dll_translation::patch_with_translations(
        std::path::Path::new(dll_path),
        target_language,
        &translations,
        output_path,
    )
    .map_err(|e| e.to_string());
    match &result {
        Ok(outcome) => {
            let _ = gtavmm_core::app_log::info(&format!(
                "patch_dll_translations wrote {} (strings_translated={}, call_sites_patched={})",
                outcome.output_path.display(),
                outcome.strings_translated,
                outcome.call_sites_patched
            ));
        }
        Err(reason) => {
            let _ = gtavmm_core::app_log::error(&format!(
                "patch_dll_translations failed for '{dll_path}': {reason}"
            ));
        }
    }
    result
}

/// Step 2 of the review flow (also the entire flow for a fully manual translation,
/// since this never calls the AI assistant itself): patches the given translations —
/// AI drafts the user may have edited, or text they typed in by hand — into a new copy
/// of the DLL.
#[tauri::command]
pub fn patch_dll_translations(
    dll_path: String,
    target_language: String,
    translations: Vec<String>,
    output_path: Option<String>,
) -> Result<gtavmm_core::dll_translation::DllTranslationOutcome, String> {
    patch_dll_translations_impl(
        &dll_path,
        &target_language,
        translations,
        output_path.as_deref(),
    )
}

// ---------------------------------------------------------------------------
// Activity log (gtavmm_core::history) — read-only viewer for install/uninstall/
// enable/disable/restore events, previously only queryable from the CLI.
// ---------------------------------------------------------------------------

pub fn list_history_impl(
    conn: &Connection,
    mod_id: Option<i64>,
) -> Result<Vec<gtavmm_core::db::models::InstallEvent>, String> {
    gtavmm_core::history::list(conn, mod_id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_history(
    state: tauri::State<crate::AppState>,
    mod_id: Option<i64>,
) -> Result<Vec<gtavmm_core::db::models::InstallEvent>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    list_history_impl(&conn, mod_id)
}

// ---------------------------------------------------------------------------
// Saved mod links (gtavmm_core::saved_links) — standalone bookmarks, independent
// of installed_mod.
// ---------------------------------------------------------------------------

pub fn list_saved_links_impl(
    conn: &Connection,
) -> Result<Vec<gtavmm_core::saved_links::SavedModLink>, String> {
    gtavmm_core::saved_links::list(conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_saved_links(
    state: tauri::State<crate::AppState>,
) -> Result<Vec<gtavmm_core::saved_links::SavedModLink>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    list_saved_links_impl(&conn)
}

pub fn add_saved_link_impl(
    conn: &Connection,
    name: &str,
    url: &str,
    notes: Option<&str>,
) -> Result<i64, String> {
    gtavmm_core::saved_links::add(conn, name, url, notes).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_saved_link(
    state: tauri::State<crate::AppState>,
    name: String,
    url: String,
    notes: Option<String>,
) -> Result<i64, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    add_saved_link_impl(&conn, &name, &url, notes.as_deref())
}

pub fn update_saved_link_impl(
    conn: &Connection,
    id: i64,
    name: &str,
    url: &str,
    notes: Option<&str>,
) -> Result<(), String> {
    gtavmm_core::saved_links::update(conn, id, name, url, notes).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_saved_link(
    state: tauri::State<crate::AppState>,
    id: i64,
    name: String,
    url: String,
    notes: Option<String>,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    update_saved_link_impl(&conn, id, &name, &url, notes.as_deref())
}

pub fn delete_saved_link_impl(conn: &Connection, id: i64) -> Result<(), String> {
    gtavmm_core::saved_links::delete(conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_saved_link(state: tauri::State<crate::AppState>, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    delete_saved_link_impl(&conn, id)
}

// ---------------------------------------------------------------------------
// Mod details (gtavmm_core::mod_details) — editing an installed mod's own
// notes/link fields, previously read-only.
// ---------------------------------------------------------------------------

pub fn update_mod_details_impl(
    conn: &Connection,
    mod_id: i64,
    notes: Option<&str>,
    link: Option<&str>,
) -> Result<(), String> {
    gtavmm_core::mod_details::update(conn, mod_id, notes, link).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_mod_details(
    state: tauri::State<crate::AppState>,
    mod_id: i64,
    notes: Option<String>,
    link: Option<String>,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    update_mod_details_impl(&conn, mod_id, notes.as_deref(), link.as_deref())
}

// ---------------------------------------------------------------------------
// App diagnostic log (gtavmm_core::app_log) — a plain local log file the user can
// look at (or attach to a bug report) without digging through the OS app-data
// directory themselves.
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn read_app_log(max_lines: usize) -> Result<Vec<String>, String> {
    gtavmm_core::app_log::read_recent(max_lines).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn app_log_path() -> Option<String> {
    gtavmm_core::app_log::log_path().map(|p| p.display().to_string())
}

/// Manual "Clear Log Now" — resets the same 3-day timer the automatic cleanup uses, so
/// clearing by hand doesn't leave the next automatic cleanup stacked right behind it.
#[tauri::command]
pub fn clear_app_log() -> Result<(), String> {
    gtavmm_core::app_log::clear().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn app_log_last_cleanup() -> Option<String> {
    gtavmm_core::app_log::last_cleanup().map(|dt| dt.to_rfc3339())
}

// ---------------------------------------------------------------------------
// AI Assistant (gtavmm_core::ai_assistant) — opt-in crash/error diagnosis. Was
// previously reachable only from the CLI (`gtavmm ai ...`) despite already being
// tested end-to-end against a real cloud provider; this wires it into the GUI.
// ---------------------------------------------------------------------------

fn parse_provider_kind(
    provider: &str,
) -> Result<gtavmm_core::ai_assistant::AiProviderKind, String> {
    match provider {
        "ollama" => Ok(gtavmm_core::ai_assistant::AiProviderKind::Ollama),
        "cloud" => Ok(gtavmm_core::ai_assistant::AiProviderKind::Cloud),
        other => Err(format!("unknown AI provider kind: {other}")),
    }
}

pub fn ai_load_settings_impl(
    conn: &Connection,
) -> Result<gtavmm_core::ai_assistant::AiSettings, String> {
    gtavmm_core::ai_assistant::load_settings(conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ai_load_settings(
    state: tauri::State<crate::AppState>,
) -> Result<gtavmm_core::ai_assistant::AiSettings, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    ai_load_settings_impl(&conn)
}

pub fn ai_enable_impl(
    conn: &Connection,
    provider: &str,
    model: Option<String>,
    cloud_endpoint: Option<String>,
) -> Result<(), String> {
    let provider = parse_provider_kind(provider)?;
    gtavmm_core::ai_assistant::enable(conn, provider, model, cloud_endpoint)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ai_enable(
    state: tauri::State<crate::AppState>,
    provider: String,
    model: Option<String>,
    cloud_endpoint: Option<String>,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    ai_enable_impl(&conn, &provider, model, cloud_endpoint)
}

pub fn ai_disable_impl(conn: &Connection) -> Result<(), String> {
    gtavmm_core::ai_assistant::disable(conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ai_disable(state: tauri::State<crate::AppState>) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    ai_disable_impl(&conn)
}

/// Stores the cloud API key in the OS-native credential store — never in the
/// database or any file this app writes.
#[tauri::command]
pub fn ai_set_cloud_api_key(key: String) -> Result<(), String> {
    gtavmm_core::ai_assistant::set_cloud_api_key(&key).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn ai_has_cloud_api_key() -> bool {
    gtavmm_core::ai_assistant::has_cloud_api_key()
}

#[tauri::command]
pub fn ai_ollama_available() -> bool {
    gtavmm_core::ai_assistant::ollama_available()
}

pub fn ai_diagnose_impl(conn: &Connection, raw_context: &str) -> Result<String, String> {
    let result = gtavmm_core::ai_assistant::diagnose(conn, raw_context).map_err(|e| e.to_string());
    if let Err(reason) = &result {
        let _ = gtavmm_core::app_log::error(&format!("ai_diagnose failed: {reason}"));
    }
    result
}

#[tauri::command]
pub fn ai_diagnose(
    state: tauri::State<crate::AppState>,
    raw_context: String,
) -> Result<String, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    ai_diagnose_impl(&conn, &raw_context)
}

// ---------------------------------------------------------------------------
// Tools: Component Checker (gtavmm_core::components), full mods\ folder backup
// (gtavmm_core::full_backup), and the uninstall recycle bin (gtavmm_core::
// recycle_bin) — all three were previously CLI-only despite being fully built
// and tested.
// ---------------------------------------------------------------------------

/// The application-data directory the database lives in. Everything the engine
/// writes outside the game folder — backups, staging, the recycle bin — hangs
/// off this, so resolving it in one place keeps those three from drifting apart
/// as more commands need them.
fn app_data_root() -> Result<std::path::PathBuf, String> {
    let db_path = gtavmm_core::db::default_db_path()
        .ok_or_else(|| "could not resolve an app-data directory on this OS".to_string())?;
    Ok(db_path
        .parent()
        .expect("db path always has a parent")
        .to_path_buf())
}

fn default_backup_root() -> Result<std::path::PathBuf, String> {
    Ok(app_data_root()?.join("backups"))
}

/// Where a disabled mod's files are parked. `state::disable` moves deployed
/// files here and `state::enable` moves them back, so both sides must agree on
/// this path or an enable would find nothing to restore.
fn default_staging_root() -> Result<std::path::PathBuf, String> {
    Ok(app_data_root()?.join("staging"))
}

/// Where uninstall snapshots a mod before deleting it, so the removal can be
/// undone for the 15 days the recycle bin keeps entries.
fn default_recycle_bin_root() -> Result<std::path::PathBuf, String> {
    Ok(app_data_root()?.join("recycle_bin"))
}

pub fn check_components_impl(
    game_path: Option<&str>,
) -> Result<Vec<gtavmm_core::components::ComponentStatus>, String> {
    let (game_root, _) =
        gtavmm_core::providers::resolve_game_root(game_path.map(std::path::Path::new))
            .map_err(|e| e.to_string())?;
    Ok(gtavmm_core::components::check_all(&game_root))
}

#[tauri::command]
pub fn check_components(
    game_path: Option<String>,
) -> Result<Vec<gtavmm_core::components::ComponentStatus>, String> {
    check_components_impl(game_path.as_deref())
}

pub fn create_full_backup_impl(game_path: Option<&str>) -> Result<String, String> {
    let (game_root, _) =
        gtavmm_core::providers::resolve_game_root(game_path.map(std::path::Path::new))
            .map_err(|e| e.to_string())?;
    let backup_root = default_backup_root()?;
    let result = gtavmm_core::full_backup::create(&game_root, &backup_root)
        .map(|p| p.display().to_string())
        .map_err(|e| e.to_string());
    if let Err(reason) = &result {
        let _ = gtavmm_core::app_log::error(&format!("create_full_backup failed: {reason}"));
    }
    result
}

#[tauri::command]
pub fn create_full_backup(game_path: Option<String>) -> Result<String, String> {
    create_full_backup_impl(game_path.as_deref())
}

pub fn list_full_backups_impl() -> Result<Vec<String>, String> {
    let backup_root = default_backup_root()?;
    gtavmm_core::full_backup::list(&backup_root)
        .map(|paths| paths.into_iter().map(|p| p.display().to_string()).collect())
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_full_backups() -> Result<Vec<String>, String> {
    list_full_backups_impl()
}

pub fn restore_full_backup_impl(zip_path: &str, game_path: Option<&str>) -> Result<(), String> {
    let (game_root, _) =
        gtavmm_core::providers::resolve_game_root(game_path.map(std::path::Path::new))
            .map_err(|e| e.to_string())?;
    let result = gtavmm_core::full_backup::restore(std::path::Path::new(zip_path), &game_root)
        .map_err(|e| e.to_string());
    if let Err(reason) = &result {
        let _ = gtavmm_core::app_log::error(&format!(
            "restore_full_backup failed for '{zip_path}': {reason}"
        ));
    }
    result
}

#[tauri::command]
pub fn restore_full_backup(zip_path: String, game_path: Option<String>) -> Result<(), String> {
    restore_full_backup_impl(&zip_path, game_path.as_deref())
}

pub fn list_recycle_bin_impl(
    conn: &Connection,
) -> Result<Vec<gtavmm_core::recycle_bin::RecycleBinEntry>, String> {
    gtavmm_core::recycle_bin::list(conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_recycle_bin(
    state: tauri::State<crate::AppState>,
) -> Result<Vec<gtavmm_core::recycle_bin::RecycleBinEntry>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    list_recycle_bin_impl(&conn)
}

pub fn restore_recycle_bin_entry_impl(
    conn: &mut Connection,
    entry_id: i64,
    game_path: Option<&str>,
) -> Result<(), String> {
    let (game_root, _) =
        gtavmm_core::providers::resolve_game_root(game_path.map(std::path::Path::new))
            .map_err(|e| e.to_string())?;
    let backup_root = default_backup_root()?;
    let result = gtavmm_core::recycle_bin::restore(conn, entry_id, &game_root, &backup_root)
        .map_err(|e| e.to_string());
    if let Err(reason) = &result {
        let _ = gtavmm_core::app_log::error(&format!(
            "restore_recycle_bin_entry failed for entry #{entry_id}: {reason}"
        ));
    }
    result
}

#[tauri::command]
pub fn restore_recycle_bin_entry(
    state: tauri::State<crate::AppState>,
    entry_id: i64,
    game_path: Option<String>,
) -> Result<(), String> {
    let mut conn = state.conn.lock().map_err(|e| e.to_string())?;
    restore_recycle_bin_entry_impl(&mut conn, entry_id, game_path.as_deref())
}

pub fn sweep_expired_recycle_bin_impl(conn: &Connection) -> Result<usize, String> {
    gtavmm_core::recycle_bin::sweep_expired(conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn sweep_expired_recycle_bin(state: tauri::State<crate::AppState>) -> Result<usize, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    sweep_expired_recycle_bin_impl(&conn)
}

// ---------------------------------------------------------------------------
// AI Workflow / Prompt template library (gtavmm_core::prompt_template) — the
// user's own reusable prompt text, plain CRUD, not part of the Action Schema.
// ---------------------------------------------------------------------------

pub fn list_prompt_templates_impl(
    conn: &Connection,
) -> Result<Vec<gtavmm_core::prompt_template::PromptTemplate>, String> {
    gtavmm_core::prompt_template::list(conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn list_prompt_templates(
    state: tauri::State<crate::AppState>,
) -> Result<Vec<gtavmm_core::prompt_template::PromptTemplate>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    list_prompt_templates_impl(&conn)
}

pub fn add_prompt_template_impl(
    conn: &Connection,
    name: &str,
    content: &str,
) -> Result<i64, String> {
    gtavmm_core::prompt_template::create(conn, name, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn add_prompt_template(
    state: tauri::State<crate::AppState>,
    name: String,
    content: String,
) -> Result<i64, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    add_prompt_template_impl(&conn, &name, &content)
}

pub fn update_prompt_template_impl(
    conn: &Connection,
    id: i64,
    name: &str,
    content: &str,
) -> Result<(), String> {
    gtavmm_core::prompt_template::update(conn, id, name, content).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn update_prompt_template(
    state: tauri::State<crate::AppState>,
    id: i64,
    name: String,
    content: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    update_prompt_template_impl(&conn, id, &name, &content)
}

pub fn delete_prompt_template_impl(conn: &Connection, id: i64) -> Result<(), String> {
    gtavmm_core::prompt_template::delete(conn, id).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn delete_prompt_template(state: tauri::State<crate::AppState>, id: i64) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    delete_prompt_template_impl(&conn, id)
}

// ---------------------------------------------------------------------------
// Export to Excel (gtavmm_core::xlsx_export) — was CLI-only (`gtavmm export`),
// missed in the first pass over unwired core modules; the design's SP Mods page
// has always had an "Export to Excel" button, but nothing backed it.
// ---------------------------------------------------------------------------

pub fn export_mods_to_xlsx_impl(conn: &Connection, output_path: &str) -> Result<(), String> {
    gtavmm_core::xlsx_export::export(conn, std::path::Path::new(output_path))
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub fn export_mods_to_xlsx(
    state: tauri::State<crate::AppState>,
    output_path: String,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    export_mods_to_xlsx_impl(&conn, &output_path)
}

// ---------------------------------------------------------------------------
// Tools page: File Editor (plain-text .ini/.xml/.json, no backup — matches the
// design's own disclaimer) and Hash Calculator (gtavmm_core::hash_calculator).
// ---------------------------------------------------------------------------

#[tauri::command]
pub fn read_text_file(path: String) -> Result<String, String> {
    std::fs::read_to_string(&path).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn write_text_file(path: String, content: String) -> Result<(), String> {
    std::fs::write(&path, content).map_err(|e| e.to_string())
}

pub fn compute_file_hashes_impl(
    path: &str,
) -> Result<gtavmm_core::hash_calculator::FileHashes, String> {
    gtavmm_core::hash_calculator::compute(std::path::Path::new(path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn compute_file_hashes(
    path: String,
) -> Result<gtavmm_core::hash_calculator::FileHashes, String> {
    compute_file_hashes_impl(&path)
}

// ---------------------------------------------------------------------------
// Malware scan (gtavmm_core::malware_scan) — shells out to whatever OS-native
// antivirus is already present; never a self-maintained scan engine.
// ---------------------------------------------------------------------------

pub fn scan_mod_path_impl(path: &str) -> Result<gtavmm_core::malware_scan::ScanOutcome, String> {
    gtavmm_core::malware_scan::scan_path(std::path::Path::new(path)).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn scan_mod_path(path: String) -> Result<gtavmm_core::malware_scan::ScanOutcome, String> {
    scan_mod_path_impl(&path)
}

// ---------------------------------------------------------------------------
// Update check (gtavmm_core::update_check) — checks GitHub Releases only when
// explicitly called; never runs on its own, per the project's offline-first
// default. Does not download or apply anything.
// ---------------------------------------------------------------------------

pub fn check_for_update_impl() -> Result<gtavmm_core::update_check::UpdateCheckResult, String> {
    gtavmm_core::update_check::check(env!("CARGO_PKG_VERSION")).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn check_for_update() -> Result<gtavmm_core::update_check::UpdateCheckResult, String> {
    check_for_update_impl()
}

// ---------------------------------------------------------------------------
// Mod library search (gtavmm_core::mod_search) — real, fully local, always-
// available keyword search (not natural-language understanding — see the
// module's own honesty note for why).
// ---------------------------------------------------------------------------

pub fn search_mods_impl(
    conn: &Connection,
    query: &str,
) -> Result<Vec<gtavmm_core::mod_search::ModSearchResult>, String> {
    gtavmm_core::mod_search::search_mods(conn, query).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn search_mods(
    state: tauri::State<crate::AppState>,
    query: String,
) -> Result<Vec<gtavmm_core::mod_search::ModSearchResult>, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    search_mods_impl(&conn, &query)
}

// ---------------------------------------------------------------------------
// Mod lifecycle: disable, enable, uninstall, reinstall.
//
// The engine has done all four since early on and they carry twenty tests
// between them, but none was ever exposed to the interface, so the application
// could install a mod and then had no way to turn it off or remove it again.
// These are deliberately thin: each resolves the paths the core function needs
// and calls straight into it, adding no logic of its own. Every guard that
// matters — status checks, backup restoration, recycle bin snapshots, add-on
// pack deregistration — already lives in the core and is tested there.
// ---------------------------------------------------------------------------

pub fn disable_mod_impl(
    conn: &Connection,
    mod_id: i64,
    staging_root: &std::path::Path,
) -> Result<(), String> {
    let result = gtavmm_core::state::disable(conn, mod_id, staging_root).map_err(|e| e.to_string());
    if let Err(reason) = &result {
        let _ =
            gtavmm_core::app_log::error(&format!("disable_mod failed for id {mod_id}: {reason}"));
    }
    result
}

#[tauri::command]
pub fn disable_mod(state: tauri::State<crate::AppState>, mod_id: i64) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let staging_root = default_staging_root()?;
    disable_mod_impl(&conn, mod_id, &staging_root)
}

pub fn enable_mod_impl(
    conn: &Connection,
    mod_id: i64,
    staging_root: &std::path::Path,
) -> Result<(), String> {
    let result = gtavmm_core::state::enable(conn, mod_id, staging_root).map_err(|e| e.to_string());
    if let Err(reason) = &result {
        let _ =
            gtavmm_core::app_log::error(&format!("enable_mod failed for id {mod_id}: {reason}"));
    }
    result
}

#[tauri::command]
pub fn enable_mod(state: tauri::State<crate::AppState>, mod_id: i64) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    let staging_root = default_staging_root()?;
    enable_mod_impl(&conn, mod_id, &staging_root)
}

pub fn uninstall_mod_impl(
    conn: &mut Connection,
    mod_id: i64,
    game_path: Option<&str>,
    recycle_bin_root: &std::path::Path,
) -> Result<(), String> {
    let (game_root, _) =
        gtavmm_core::providers::resolve_game_root(game_path.map(std::path::Path::new))
            .map_err(|e| e.to_string())?;
    let result = gtavmm_core::uninstall::uninstall(conn, mod_id, &game_root, recycle_bin_root)
        .map_err(|e| e.to_string());
    if let Err(reason) = &result {
        let _ =
            gtavmm_core::app_log::error(&format!("uninstall_mod failed for id {mod_id}: {reason}"));
    }
    result
}

#[tauri::command]
pub fn uninstall_mod(
    state: tauri::State<crate::AppState>,
    mod_id: i64,
    game_path: Option<String>,
) -> Result<(), String> {
    let mut conn = state.conn.lock().map_err(|e| e.to_string())?;
    let recycle_bin_root = default_recycle_bin_root()?;
    uninstall_mod_impl(&mut conn, mod_id, game_path.as_deref(), &recycle_bin_root)
}

/// Replaces a mod's files from a newer download. The caller supplies the source
/// path because this project never fetches a mod on the user's behalf.
///
/// Note the core deliberately does not roll back across the uninstall/install
/// boundary: if the install half fails, the mod is left uninstalled and
/// recoverable from the recycle bin rather than silently restored. A real
/// interruption surfaces as a real failure.
#[allow(clippy::too_many_arguments)]
pub fn reinstall_mod_impl(
    conn: &mut Connection,
    mod_id: i64,
    new_source_path: &str,
    version_label: &str,
    mode: &str,
    game_path: Option<&str>,
    backup_root: &std::path::Path,
    recycle_bin_root: &std::path::Path,
) -> Result<gtavmm_core::install::InstallOutcome, String> {
    let core_mode = mode_from_str(mode)?;
    let (game_root, provider) =
        gtavmm_core::providers::resolve(game_path.map(std::path::Path::new), core_mode)
            .map_err(|e| e.to_string())?;
    let options = gtavmm_core::install::InstallOptions {
        auto_backup: true,
        override_foreign_conflicts: false,
    };
    let result = gtavmm_core::install::reinstall(
        conn,
        mod_id,
        std::path::Path::new(new_source_path),
        version_label,
        provider.as_ref(),
        &game_root,
        backup_root,
        recycle_bin_root,
        options,
    )
    .map_err(|e| e.to_string());
    if let Err(reason) = &result {
        let _ =
            gtavmm_core::app_log::error(&format!("reinstall_mod failed for id {mod_id}: {reason}"));
    }
    result
}

#[tauri::command]
pub fn reinstall_mod(
    state: tauri::State<crate::AppState>,
    mod_id: i64,
    new_source_path: String,
    version_label: String,
    mode: String,
    game_path: Option<String>,
) -> Result<gtavmm_core::install::InstallOutcome, String> {
    let mut conn = state.conn.lock().map_err(|e| e.to_string())?;
    let backup_root =
        default_backup_root()?.join(chrono::Utc::now().format("%Y%m%d%H%M%S%3f").to_string());
    let recycle_bin_root = default_recycle_bin_root()?;
    reinstall_mod_impl(
        &mut conn,
        mod_id,
        &new_source_path,
        &version_label,
        &mode,
        game_path.as_deref(),
        &backup_root,
        &recycle_bin_root,
    )
}

// ---------------------------------------------------------------------------
// User settings.
//
// `settings::load`/`save` have existed and been tested since early on, but only
// language was ever reachable — the interface had no way to read or write the
// theme, the terms acceptance, the first-run state or the backup location, so
// none of those screens could exist. These expose the whole row.
// ---------------------------------------------------------------------------

/// The startup state the shell needs before it can decide what to show: the
/// terms gate, first-run setup, or the workspace.
#[derive(Debug, Clone, Serialize)]
pub struct StartupState {
    /// The user's choice — "system", "dark" or "light" — not what it resolves to.
    pub theme: String,
    pub terms_accepted: bool,
    pub onboarding_completed: bool,
    pub language: String,
}

pub fn load_startup_state_impl(conn: &Connection) -> Result<StartupState, String> {
    let s = gtavmm_core::settings::load(conn).map_err(|e| e.to_string())?;
    Ok(StartupState {
        // An unset theme means the user never chose, which is "follow the OS".
        theme: s.theme.clone().unwrap_or_else(|| "system".to_string()),
        terms_accepted: gtavmm_core::settings::has_accepted_current_terms(&s),
        onboarding_completed: s.onboarding_completed,
        language: s.language.clone(),
    })
}

#[tauri::command]
pub fn load_startup_state(state: tauri::State<crate::AppState>) -> Result<StartupState, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    load_startup_state_impl(&conn)
}

pub fn set_theme_impl(conn: &Connection, theme: &str) -> Result<(), String> {
    if !matches!(theme, "system" | "dark" | "light") {
        return Err(format!(
            "unknown theme: {theme} (expected system/dark/light)"
        ));
    }
    let mut settings = gtavmm_core::settings::load(conn).map_err(|e| e.to_string())?;
    settings.theme = Some(theme.to_string());
    gtavmm_core::settings::save(conn, &settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn set_theme(state: tauri::State<crate::AppState>, theme: String) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    set_theme_impl(&conn, &theme)
}

/// Records acceptance of the terms version the application is currently showing.
/// Storing the version rather than a flag is what lets a later revision ask
/// again instead of passing on stale consent.
pub fn accept_terms_impl(conn: &Connection) -> Result<(), String> {
    let mut settings = gtavmm_core::settings::load(conn).map_err(|e| e.to_string())?;
    settings.terms_accepted_version =
        Some(gtavmm_core::settings::CURRENT_TERMS_VERSION.to_string());
    gtavmm_core::settings::save(conn, &settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn accept_terms(state: tauri::State<crate::AppState>) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    accept_terms_impl(&conn)
}

pub fn complete_onboarding_impl(
    conn: &Connection,
    game_install_path_override: Option<&str>,
) -> Result<(), String> {
    let mut settings = gtavmm_core::settings::load(conn).map_err(|e| e.to_string())?;
    // Only overwrite the stored path when one was actually supplied; passing
    // nothing means "keep whatever detection or a previous run already set",
    // not "clear it".
    if let Some(path) = game_install_path_override {
        settings.game_install_path_override = Some(path.to_string());
    }
    settings.onboarding_completed = true;
    gtavmm_core::settings::save(conn, &settings).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn complete_onboarding(
    state: tauri::State<crate::AppState>,
    game_install_path_override: Option<String>,
) -> Result<(), String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    complete_onboarding_impl(&conn, game_install_path_override.as_deref())
}

/// The whole settings row, for the Backup and Game paths panels.
pub fn load_user_settings_impl(
    conn: &Connection,
) -> Result<gtavmm_core::db::models::UserSettings, String> {
    gtavmm_core::settings::load(conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_user_settings(
    state: tauri::State<crate::AppState>,
) -> Result<gtavmm_core::db::models::UserSettings, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    load_user_settings_impl(&conn)
}

/// How a panel wants one optional path changed.
///
/// Deliberately an explicit three-way rather than `Option<Option<String>>`:
/// serde reads a JSON `null` for a double option as the outer `None`, so
/// "clear this path" would arrive looking exactly like "leave it alone" and the
/// clear button would silently do nothing.
#[derive(Debug, Clone, Default, serde::Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum PathChange {
    /// Not mentioned by the caller — keep whatever is stored.
    #[default]
    Keep,
    /// Remove the stored value.
    Clear,
    /// Replace the stored value.
    Set { path: String },
}

impl PathChange {
    fn apply(self, target: &mut Option<String>) {
        match self {
            PathChange::Keep => {}
            PathChange::Clear => *target = None,
            PathChange::Set { path } => *target = Some(path),
        }
    }
}

/// Updates the settings the Backup and Game paths panels own. Deliberately not a
/// blanket "save this whole row": a panel that only shows two fields must not be
/// able to silently reset the terms acceptance or the onboarding state by
/// writing back a row it never fully loaded.
pub fn update_user_settings_impl(
    conn: &Connection,
    default_auto_backup: Option<bool>,
    game_install_path_override: PathChange,
    backup_root_override: PathChange,
) -> Result<gtavmm_core::db::models::UserSettings, String> {
    let mut settings = gtavmm_core::settings::load(conn).map_err(|e| e.to_string())?;
    if let Some(v) = default_auto_backup {
        settings.default_auto_backup = v;
    }
    game_install_path_override.apply(&mut settings.game_install_path_override);
    backup_root_override.apply(&mut settings.backup_root_override);
    gtavmm_core::settings::save(conn, &settings).map_err(|e| e.to_string())?;
    Ok(settings)
}

#[tauri::command]
pub fn update_user_settings(
    state: tauri::State<crate::AppState>,
    default_auto_backup: Option<bool>,
    game_install_path_override: Option<PathChange>,
    backup_root_override: Option<PathChange>,
) -> Result<gtavmm_core::db::models::UserSettings, String> {
    let conn = state.conn.lock().map_err(|e| e.to_string())?;
    update_user_settings_impl(
        &conn,
        default_auto_backup,
        game_install_path_override.unwrap_or_default(),
        backup_root_override.unwrap_or_default(),
    )
}

/// Checks a manually chosen game folder before it is saved, so a wrong path is
/// rejected at the point of choosing rather than at the next install.
pub fn validate_game_path_impl(path: &str) -> Result<DetectGameResult, String> {
    use gtavmm_core::game_locator::DetectResult;
    match gtavmm_core::game_locator::validate_manual_path(std::path::Path::new(path)) {
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
pub fn validate_game_path(path: String) -> Result<DetectGameResult, String> {
    validate_game_path_impl(&path)
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
        let err = inspect_mod_impl(
            Some(game_dir.path().to_str().unwrap()),
            "not-a-mode",
            "x.asi",
        )
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
        assert_eq!(
            profile_mod_ids_impl(&conn, profile_id).unwrap(),
            vec![mod_id]
        );

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

        fivem_apply_load_order_impl(dir.path().to_str().unwrap(), server_cfg.to_str().unwrap())
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

    #[test]
    fn inspect_dll_impl_surfaces_a_readable_error_for_a_non_pe_file() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not-a-dll.dll");
        std::fs::write(&path, b"not a real PE file").unwrap();
        let err = inspect_dll_impl(path.to_str().unwrap()).unwrap_err();
        assert!(err.contains("PE file"));
    }

    #[test]
    fn translate_dll_draft_impl_never_writes_when_the_source_file_is_invalid() {
        let conn = gtavmm_core::db::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not-a-dll.dll");
        std::fs::write(&path, b"not a real PE file").unwrap();
        assert!(translate_dll_draft_impl(&conn, path.to_str().unwrap(), "zh-TW").is_err());
        assert!(!dir.path().join("not-a-dll.zh-TW.dll").exists());
    }

    #[test]
    fn patch_dll_translations_impl_never_writes_when_the_source_file_is_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not-a-dll.dll");
        std::fs::write(&path, b"not a real PE file").unwrap();
        let result = patch_dll_translations_impl(
            path.to_str().unwrap(),
            "zh-TW",
            vec!["手動翻譯".to_string()],
            None,
        );
        assert!(result.is_err());
        assert!(!dir.path().join("not-a-dll.zh-TW.dll").exists());
    }

    #[test]
    fn patch_dll_translations_impl_never_writes_to_a_custom_output_when_source_is_invalid() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not-a-dll.dll");
        std::fs::write(&path, b"not a real PE file").unwrap();
        let custom_output = dir.path().join("elsewhere.dll");
        let result = patch_dll_translations_impl(
            path.to_str().unwrap(),
            "zh-TW",
            vec!["手動翻譯".to_string()],
            Some(custom_output.to_str().unwrap()),
        );
        assert!(result.is_err());
        assert!(!custom_output.exists());
    }

    // -----------------------------------------------------------------------
    // Mod lifecycle. The core's own suite already covers the mechanics in
    // depth; what these check is that the command layer resolves paths and
    // arguments correctly, because that is the part the core tests cannot see
    // and the part that was missing entirely until now.
    // -----------------------------------------------------------------------

    /// Installs one .asi into a fake game root and returns everything the
    /// lifecycle commands need to act on it.
    fn installed_fixture() -> (
        Connection,
        tempfile::TempDir,
        tempfile::TempDir,
        tempfile::TempDir,
        i64,
    ) {
        let game_dir = fake_game_root();
        let work = tempfile::tempdir().unwrap();
        let staging = tempfile::tempdir().unwrap();

        let src = work.path().join("LifecycleMod.asi");
        std::fs::write(&src, b"fake asi bytes").unwrap();

        let mut conn = gtavmm_core::db::open_in_memory().unwrap();
        install_mod_impl(
            &mut conn,
            Some(game_dir.path().to_str().unwrap()),
            "sp",
            src.to_str().unwrap(),
            Some("LifecycleMod"),
            false,
            &work.path().join("backups"),
        )
        .unwrap();

        let id: i64 = conn
            .query_row("SELECT id FROM installed_mod LIMIT 1", [], |r| r.get(0))
            .unwrap();
        (conn, game_dir, work, staging, id)
    }

    fn status_of(conn: &Connection, id: i64) -> String {
        conn.query_row(
            "SELECT status FROM installed_mod WHERE id = ?1",
            [id],
            |r| r.get(0),
        )
        .unwrap()
    }

    #[test]
    fn disable_then_enable_moves_the_file_out_and_back() {
        let (conn, game_dir, _work, staging, id) = installed_fixture();
        let deployed = game_dir.path().join("LifecycleMod.asi");
        assert!(deployed.exists(), "install should have written the file");

        disable_mod_impl(&conn, id, staging.path()).unwrap();
        assert_eq!(status_of(&conn, id), "disabled");
        assert!(
            !deployed.exists(),
            "a disabled mod must not leave its file in the game folder"
        );

        enable_mod_impl(&conn, id, staging.path()).unwrap();
        assert_eq!(status_of(&conn, id), "active");
        assert!(deployed.exists(), "enabling should put the file back");
    }

    #[test]
    fn disable_and_enable_refuse_when_the_mod_is_not_in_the_right_state() {
        let (conn, _game_dir, _work, staging, id) = installed_fixture();

        // Already active, so there is nothing to enable.
        let err = enable_mod_impl(&conn, id, staging.path()).unwrap_err();
        assert!(err.contains("not disabled"), "unexpected message: {err}");

        disable_mod_impl(&conn, id, staging.path()).unwrap();
        let err = disable_mod_impl(&conn, id, staging.path()).unwrap_err();
        assert!(err.contains("not active"), "unexpected message: {err}");
    }

    #[test]
    fn uninstall_removes_the_file_and_snapshots_it_into_the_recycle_bin() {
        let (mut conn, game_dir, work, _staging, id) = installed_fixture();
        let deployed = game_dir.path().join("LifecycleMod.asi");
        let recycle = work.path().join("recycle_bin");

        uninstall_mod_impl(
            &mut conn,
            id,
            Some(game_dir.path().to_str().unwrap()),
            &recycle,
        )
        .unwrap();

        assert_eq!(status_of(&conn, id), "uninstalled");
        assert!(
            !deployed.exists(),
            "uninstall should delete the deployed file"
        );

        let entries = gtavmm_core::recycle_bin::list(&conn).unwrap();
        assert_eq!(
            entries.len(),
            1,
            "uninstall must leave something restorable behind"
        );
    }

    #[test]
    fn uninstalling_an_unknown_id_errors_rather_than_silently_doing_nothing() {
        let (mut conn, game_dir, work, _staging, _id) = installed_fixture();
        let err = uninstall_mod_impl(
            &mut conn,
            9999,
            Some(game_dir.path().to_str().unwrap()),
            &work.path().join("recycle_bin"),
        )
        .unwrap_err();
        assert!(err.contains("9999"), "unexpected message: {err}");
    }

    #[test]
    fn reinstall_replaces_the_file_from_a_new_source() {
        let (mut conn, game_dir, work, _staging, id) = installed_fixture();
        let deployed = game_dir.path().join("LifecycleMod.asi");

        let newer = work.path().join("v2").join("LifecycleMod.asi");
        std::fs::create_dir_all(newer.parent().unwrap()).unwrap();
        std::fs::write(&newer, b"newer asi bytes").unwrap();

        reinstall_mod_impl(
            &mut conn,
            id,
            newer.to_str().unwrap(),
            "2.0",
            "sp",
            Some(game_dir.path().to_str().unwrap()),
            &work.path().join("backups-reinstall"),
            &work.path().join("recycle_bin"),
        )
        .unwrap();

        assert_eq!(
            std::fs::read(&deployed).unwrap(),
            b"newer asi bytes",
            "the deployed file should be the new source's contents"
        );
    }

    #[test]
    fn reinstall_rejects_an_unknown_mode_before_touching_anything() {
        let (mut conn, game_dir, work, _staging, id) = installed_fixture();
        let deployed = game_dir.path().join("LifecycleMod.asi");
        let before = std::fs::read(&deployed).unwrap();

        let err = reinstall_mod_impl(
            &mut conn,
            id,
            "irrelevant.asi",
            "2.0",
            "not-a-mode",
            Some(game_dir.path().to_str().unwrap()),
            &work.path().join("backups-reinstall"),
            &work.path().join("recycle_bin"),
        )
        .unwrap_err();

        assert!(err.contains("unknown mode"), "unexpected message: {err}");
        assert_eq!(
            std::fs::read(&deployed).unwrap(),
            before,
            "a rejected mode must not have disturbed the installed file"
        );
        assert_eq!(status_of(&conn, id), "active");
    }

    // -----------------------------------------------------------------------
    // Settings and startup state.
    // -----------------------------------------------------------------------

    #[test]
    fn a_fresh_install_starts_at_the_terms_gate_following_the_system_theme() {
        let conn = gtavmm_core::db::open_in_memory().unwrap();
        let state = load_startup_state_impl(&conn).unwrap();
        assert_eq!(state.theme, "system", "an unset theme means follow the OS");
        assert!(!state.terms_accepted);
        assert!(!state.onboarding_completed);
    }

    #[test]
    fn accepting_terms_then_finishing_setup_moves_startup_past_both_gates() {
        let conn = gtavmm_core::db::open_in_memory().unwrap();
        accept_terms_impl(&conn).unwrap();
        let state = load_startup_state_impl(&conn).unwrap();
        assert!(state.terms_accepted);
        assert!(
            !state.onboarding_completed,
            "accepting the terms must not also count as finishing setup"
        );

        complete_onboarding_impl(&conn, Some(r"D:\Games\GTAV")).unwrap();
        let state = load_startup_state_impl(&conn).unwrap();
        assert!(state.onboarding_completed);
        assert_eq!(
            load_user_settings_impl(&conn)
                .unwrap()
                .game_install_path_override
                .as_deref(),
            Some(r"D:\Games\GTAV")
        );
    }

    #[test]
    fn finishing_setup_without_a_path_keeps_the_one_already_stored() {
        let conn = gtavmm_core::db::open_in_memory().unwrap();
        complete_onboarding_impl(&conn, Some(r"D:\Games\GTAV")).unwrap();
        // Passing nothing means "leave it alone", not "clear it".
        complete_onboarding_impl(&conn, None).unwrap();
        assert_eq!(
            load_user_settings_impl(&conn)
                .unwrap()
                .game_install_path_override
                .as_deref(),
            Some(r"D:\Games\GTAV")
        );
    }

    #[test]
    fn set_theme_round_trips_and_rejects_anything_else() {
        let conn = gtavmm_core::db::open_in_memory().unwrap();
        set_theme_impl(&conn, "dark").unwrap();
        assert_eq!(load_startup_state_impl(&conn).unwrap().theme, "dark");
        set_theme_impl(&conn, "system").unwrap();
        assert_eq!(load_startup_state_impl(&conn).unwrap().theme, "system");

        let err = set_theme_impl(&conn, "solarized").unwrap_err();
        assert!(err.contains("unknown theme"), "unexpected message: {err}");
        assert_eq!(
            load_startup_state_impl(&conn).unwrap().theme,
            "system",
            "a rejected theme must not have been written"
        );
    }

    #[test]
    fn updating_backup_settings_leaves_the_gates_untouched() {
        // A panel showing two fields must not be able to reset the terms
        // acceptance or the first-run state by writing back a partial row.
        let conn = gtavmm_core::db::open_in_memory().unwrap();
        accept_terms_impl(&conn).unwrap();
        complete_onboarding_impl(&conn, None).unwrap();

        update_user_settings_impl(
            &conn,
            Some(false),
            PathChange::Keep,
            PathChange::Set {
                path: r"E:\Backups".to_string(),
            },
        )
        .unwrap();

        let s = load_user_settings_impl(&conn).unwrap();
        assert!(!s.default_auto_backup);
        assert_eq!(s.backup_root_override.as_deref(), Some(r"E:\Backups"));

        let state = load_startup_state_impl(&conn).unwrap();
        assert!(state.terms_accepted, "terms acceptance was clobbered");
        assert!(state.onboarding_completed, "onboarding state was clobbered");
    }

    #[test]
    fn validate_game_path_reports_not_found_for_a_folder_without_a_game() {
        let dir = tempfile::tempdir().unwrap();
        let result = validate_game_path_impl(dir.path().to_str().unwrap()).unwrap();
        assert!(matches!(result, DetectGameResult::NotFound));
    }

    #[test]
    fn validate_game_path_recognises_a_real_looking_install() {
        let dir = fake_game_root();
        let result = validate_game_path_impl(dir.path().to_str().unwrap()).unwrap();
        assert!(matches!(result, DetectGameResult::Found { .. }));
    }

    #[test]
    fn clearing_a_path_is_distinguishable_from_leaving_it_alone() {
        // The reason PathChange is an explicit three-way rather than a nested
        // Option: serde reads a JSON null for Option<Option<String>> as the
        // outer None, so "clear this" would arrive indistinguishable from
        // "leave it alone" and the clear button would silently do nothing.
        let conn = gtavmm_core::db::open_in_memory().unwrap();
        update_user_settings_impl(
            &conn,
            None,
            PathChange::Set {
                path: r"D:\Games\GTAV".to_string(),
            },
            PathChange::Keep,
        )
        .unwrap();

        // Keep leaves it standing.
        update_user_settings_impl(&conn, None, PathChange::Keep, PathChange::Keep).unwrap();
        assert_eq!(
            load_user_settings_impl(&conn)
                .unwrap()
                .game_install_path_override
                .as_deref(),
            Some(r"D:\Games\GTAV")
        );

        // Clear actually removes it.
        update_user_settings_impl(&conn, None, PathChange::Clear, PathChange::Keep).unwrap();
        assert!(load_user_settings_impl(&conn)
            .unwrap()
            .game_install_path_override
            .is_none());
    }
}
