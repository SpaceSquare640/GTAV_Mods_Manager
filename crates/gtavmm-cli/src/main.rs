// SPDX-License-Identifier: AGPL-3.0-only

//! Development/testing CLI for gtavmm-core, and the eventual power-user CLI mode.
//! Per the project's "core first, UI last" decision, this is the primary way to
//! exercise the engine until the Tauri/React app exists.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use gtavmm_core::db;
use gtavmm_core::game_locator::{self, DetectResult};

/// Which mode's `ModeProvider` to use against the detected/overridden game install.
/// Orthogonal to the Legacy/Enhanced edition: this picks the mod-management
/// convention (plain SP mods vs. LSPDFR's RAGE Plugin Hook plugin layout), while
/// edition picks Legacy vs. Enhanced within that mode.
#[derive(Copy, Clone, Debug, ValueEnum)]
enum Mode {
    /// Plain Single Player mods (ScriptHookV/SHVDN/Menyoo/OpenIV conventions).
    Sp,
    /// LSPDFR (RAGE Plugin Hook plugin/callout conventions). See
    /// `gtavmm_core::providers::LegacyLspdfrProvider`'s doc comment for the
    /// unverified assumptions this currently relies on.
    Lspdfr,
    /// FiveM client-side asset mods. `--game-path` must point at the FiveM client
    /// install (not a GTA V install) — there is no auto-detection for it. Ignores
    /// the Legacy/Enhanced edition entirely.
    FivemClient,
}

#[derive(Parser)]
#[command(
    name = "gtavmm",
    version,
    about = "GTAV Mods Manager — core engine CLI"
)]
struct Cli {
    /// Override auto-detection with a specific GTA V install folder.
    #[arg(long, global = true)]
    game_path: Option<PathBuf>,

    /// Which mode's directory conventions to use for install/inspect. Defaults to
    /// plain SP mods; pass `--mode lspdfr` when managing an LSPDFR install.
    #[arg(long, global = true, value_enum, default_value_t = Mode::Sp)]
    mode: Mode,

    #[command(subcommand)]
    command: Command,
}

#[derive(Subcommand)]
enum Command {
    /// Detect the GTA V (Legacy) installation.
    DetectGame,
    /// List installed mods.
    ListMods,
    /// Install a mod from a file, folder, or archive.
    Install {
        path: String,
        /// Name to record this mod under (defaults to the file/folder name).
        #[arg(long)]
        name: Option<String>,
        /// Proceed even if this collides with a different mod's files (required
        /// after reviewing a `RequiresOverride` refusal).
        #[arg(long)]
        yes: bool,
        /// Skip backing up files this install would overwrite. Not recommended —
        /// see the MVP spec's default-on backup warning.
        #[arg(long)]
        no_backup: bool,
    },
    /// Uninstall a mod by id.
    Uninstall { id: i64 },
    /// Enable a disabled mod.
    Enable { id: i64 },
    /// Disable an active mod.
    Disable { id: i64 },
    /// Recycle bin operations.
    RecycleBin {
        #[command(subcommand)]
        action: RecycleBinAction,
    },
    /// Show install/uninstall/enable/disable history.
    History {
        #[arg(long)]
        mod_id: Option<i64>,
    },
    /// Check for ScriptHookV / ScriptHookVDotNet / OpenIV / OpenRPF.
    CheckComponents,
    /// Read-only preview of what installing a mod would do — no files are written,
    /// nothing is recorded. The "Install Helper" from the MVP spec's validated
    /// feature list.
    Inspect { path: String },
    /// Whole-`mods\`-folder backup/restore (a coarser, manual safety net distinct
    /// from install's per-mod backups).
    FullBackup {
        #[command(subcommand)]
        action: FullBackupAction,
    },
    /// Check GitHub Releases for a newer version of this tool. Does not download or
    /// apply anything — see the "Auto Update" note in the project docs for why
    /// applying updates automatically waits on the Tauri UI shell.
    CheckUpdate,
    /// Generate a pre-filled, de-identified GitHub Issue draft for reporting a
    /// problem (opt-in — this only prints a URL; nothing is sent until you submit it
    /// yourself on GitHub).
    ReportCrash {
        /// Describe what happened. This is included verbatim (after de-identifying
        /// any home-directory paths) in the issue draft.
        description: String,
    },
    /// Export installed/uninstalled mods to a styled .xlsx workbook.
    Export {
        /// Output file path (e.g. `mods.xlsx`).
        output: PathBuf,
    },
    /// Scan a mod file/folder using the OS's native antivirus (Windows Defender /
    /// clamscan) before installing it. We don't maintain our own scan engine — see
    /// the module docs for why. Reports plainly if no scanner is available.
    Scan { path: PathBuf },
    /// Resolve a correct load order for a FiveM server's `resources\` folder from
    /// each resource's declared `fxmanifest.lua` dependencies — unlike txAdmin's
    /// manual `ensure`-order CFG editor, this is computed automatically.
    FivemResourceOrder {
        /// Path to the server's `resources\` folder.
        resources_dir: PathBuf,
    },
    /// Multi-profile operations: named sets of mods that should be active together.
    Profile {
        #[command(subcommand)]
        action: ProfileAction,
    },
    /// AI Assistant System — currently just opt-in crash/error log diagnosis
    /// (read-only advice, no automated fixes yet). Disabled by default.
    Ai {
        #[command(subcommand)]
        action: AiAction,
    },
    /// AI Workflow / Prompt template library: the user's own reusable prompt text,
    /// stored for copy-paste reuse. Not automated — no Action Schema, nothing is
    /// executed on the user's behalf.
    Prompt {
        #[command(subcommand)]
        action: PromptAction,
    },
    /// SP → FiveM Add-on vehicle-pack converter (v0.7.x, scope confirmed to vehicle
    /// packs only — script mods are a future, unscheduled extension). Reads a SP
    /// add-on vehicle mod's `dlc.rpf` directly and writes a ready-to-use FiveM
    /// resource folder (data/, stream/, fxmanifest.lua).
    ConvertVehicle {
        /// Path to the SP add-on vehicle mod's dlc.rpf.
        dlc_rpf: PathBuf,
        /// Output folder for the generated FiveM resource (created if missing).
        output_dir: PathBuf,
    },
}

#[derive(Subcommand)]
enum PromptAction {
    /// Create a new prompt template.
    Create { name: String, content: String },
    /// List all prompt templates, most recently updated first.
    List,
    /// Update an existing prompt template's name and content.
    Update { id: i64, name: String, content: String },
    /// Delete a prompt template.
    Delete { id: i64 },
}

#[derive(Subcommand)]
enum AiAction {
    /// Enable the AI assistant with a provider.
    Enable {
        #[arg(value_enum)]
        provider: AiProviderArg,
        /// Model name override (Ollama: e.g. "llama3.1"; cloud: e.g. "gpt-4o-mini").
        #[arg(long)]
        model: Option<String>,
        /// Cloud provider endpoint override (defaults to the OpenAI chat-completions
        /// endpoint). Ignored for `ollama`.
        #[arg(long)]
        endpoint: Option<String>,
    },
    /// Disable the AI assistant.
    Disable,
    /// Show current AI assistant settings and provider availability.
    Status,
    /// Set the cloud provider API key. Reads the key from stdin (not as a command
    /// argument) so it doesn't end up in shell history or a process list.
    SetApiKey,
    /// Send a crash/error log or free-text description to the configured provider
    /// for a read-only diagnosis. Requires `ai enable` first.
    Diagnose {
        /// Path to a log file. If omitted, reads from stdin.
        #[arg(long)]
        file: Option<PathBuf>,
    },
    /// List the bundled known-fix rules (v0.7.x Action Schema — see the module docs
    /// for why this is currently one example rule, not a curated fix database).
    ListKnownFixes,
    /// Show the Plan a known-fix rule expands to, without executing anything.
    PlanKnownFix { rule_id: String },
    /// Expand and execute every action in a known-fix rule's Plan. `--yes` is
    /// required — this is the "同意" step; without it this only re-prints the Plan
    /// (equivalent to `plan-known-fix`) and executes nothing.
    ApplyKnownFix {
        rule_id: String,
        #[arg(long)]
        yes: bool,
    },
}

#[derive(Copy, Clone, Debug, clap::ValueEnum)]
enum AiProviderArg {
    Ollama,
    Cloud,
}

#[derive(Subcommand)]
enum ProfileAction {
    /// Create a new (initially empty) profile.
    Create { name: String },
    /// List all profiles, marking the currently active one.
    List,
    /// Delete a profile (does not uninstall or otherwise touch its mods).
    Delete { id: i64 },
    /// Assign a mod to a profile (opt-in — a mod not assigned to any profile is
    /// never touched by `switch`).
    AddMod { profile_id: i64, mod_id: i64 },
    /// Unassign a mod from a profile.
    RemoveMod { profile_id: i64, mod_id: i64 },
    /// Switch to a profile: disables other profiles' active-only mods, enables this
    /// profile's disabled mods.
    Switch { id: i64 },
    /// Export a profile (its name and mod names, not the mod files themselves) to a
    /// JSON file for sharing.
    Export { id: i64, output: PathBuf },
    /// Import a profile export as a new profile, matching mod names against what's
    /// already installed locally (this project never auto-downloads mods).
    Import { path: PathBuf },
}

#[derive(Subcommand)]
enum RecycleBinAction {
    List,
    Restore { id: i64 },
}

#[derive(Subcommand)]
enum FullBackupAction {
    Create,
    List,
    Restore { path: PathBuf },
}

/// Prints a known-fix rule's expanded Plan (index, reason, action) and returns it, so
/// both `plan-known-fix` (preview only) and `apply-known-fix` (preview + execute) share
/// one code path — the Plan a user approves is always exactly the Plan that was shown.
fn print_known_fix_plan(
    rule_id: &str,
) -> Result<Vec<gtavmm_core::ai_assistant::action_schema::PlanItem>> {
    let plan = gtavmm_core::ai_assistant::known_fixes::build_plan_from_known_fix(rule_id)?;
    println!("Plan for known-fix rule '{rule_id}':");
    for (i, item) in plan.iter().enumerate() {
        println!("  [{i}] {:?}", item.action);
        println!("       reason: {}", item.reason);
    }
    Ok(plan)
}

/// Returns the detected game root along with its edition (`"legacy"`/`"enhanced"`),
/// so call sites can pick the matching `ModeProvider` via [`provider_for`].
fn require_game_root(override_path: &Option<PathBuf>) -> Result<(PathBuf, String)> {
    if let Some(path) = override_path {
        return match game_locator::validate_manual_path(path)? {
            DetectResult::Found(installation) => Ok((
                PathBuf::from(installation.install_path),
                installation.edition,
            )),
            DetectResult::FoundUnsupportedEdition { path, edition } => Err(anyhow::anyhow!(
                "{} is a {:?} GTA V install, which is not supported yet.",
                path.display(),
                edition
            )),
            DetectResult::NotFound => Err(anyhow::anyhow!(
                "{} does not look like a supported GTA V install (no recognized executable found).",
                path.display()
            )),
        };
    }
    match game_locator::detect()? {
        DetectResult::Found(installation) => {
            Ok((PathBuf::from(installation.install_path), installation.edition))
        }
        DetectResult::FoundUnsupportedEdition { path, edition } => Err(anyhow::anyhow!(
            "Found a {:?} GTA V install at {}, but that edition is not supported yet.",
            edition,
            path.display()
        )),
        DetectResult::NotFound => Err(anyhow::anyhow!(
            "No supported GTA V installation detected. Pass --game-path <folder> to specify it manually."
        )),
    }
}

/// Picks the `ModeProvider` matching a detected edition string (`"legacy"`/`"enhanced"`,
/// as produced by `game_locator`/`require_game_root`) and the CLI's `--mode` flag.
fn provider_for(
    game_root: PathBuf,
    edition: &str,
    mode: Mode,
) -> Box<dyn gtavmm_core::providers::ModeProvider> {
    match (mode, edition) {
        (Mode::Sp, "enhanced") => {
            Box::new(gtavmm_core::providers::EnhancedSpProvider::new(game_root))
        }
        (Mode::Sp, _) => Box::new(gtavmm_core::providers::LegacySpProvider::new(game_root)),
        (Mode::Lspdfr, "enhanced") => Box::new(
            gtavmm_core::providers::EnhancedLspdfrProvider::new(game_root),
        ),
        (Mode::Lspdfr, _) => Box::new(gtavmm_core::providers::LegacyLspdfrProvider::new(game_root)),
        (Mode::FivemClient, _) => {
            Box::new(gtavmm_core::providers::FiveMClientProvider::new(game_root))
        }
    }
}

/// Resolves the `ModeProvider` to use for `Install`/`Inspect`, handling `--mode
/// fivem-client` specially: FiveM has no `game_locator` auto-detection path (it isn't
/// a GTA V install), so it requires an explicit `--game-path` pointing at the FiveM
/// client folder instead of going through `require_game_root`.
fn resolve_provider(
    game_path: &Option<PathBuf>,
    mode: Mode,
) -> Result<(PathBuf, Box<dyn gtavmm_core::providers::ModeProvider>)> {
    if matches!(mode, Mode::FivemClient) {
        let path = game_path.clone().ok_or_else(|| {
            anyhow::anyhow!(
                "--mode fivem-client requires --game-path pointing at the FiveM client \
                 install folder (there is no auto-detection for it)."
            )
        })?;
        let provider = gtavmm_core::providers::FiveMClientProvider::new(path.clone());
        return Ok((path, Box::new(provider)));
    }
    let (game_root, edition) = require_game_root(game_path)?;
    let provider = provider_for(game_root.clone(), &edition, mode);
    Ok((game_root, provider))
}

fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(cli) {
        eprintln!("Error: {err:#}");

        let draft = gtavmm_core::crash_report::build(
            env!("CARGO_PKG_VERSION"),
            std::env::consts::OS,
            &format!("{err:#}"),
        );
        eprintln!(
            "\nIf you'd like to report this (opt-in — nothing is sent unless you \
             submit it yourself), a pre-filled, de-identified issue draft is ready:\n{}",
            draft.github_issue_url
        );

        std::process::exit(1);
    }
}

fn run(cli: Cli) -> Result<()> {
    let db_path = db::default_db_path()
        .ok_or_else(|| anyhow::anyhow!("could not resolve an app-data directory on this OS"))?;
    let mut conn = db::open(&db_path)?;
    gtavmm_core::recycle_bin::sweep_expired(&conn)?;

    match cli.command {
        Command::DetectGame => {
            let result = match &cli.game_path {
                Some(path) => game_locator::validate_manual_path(path)?,
                None => game_locator::detect()?,
            };
            match result {
                DetectResult::Found(installation) => {
                    println!(
                        "Found {} GTA V at: {} (via {:?})",
                        installation.edition, installation.install_path, installation.detected_via
                    );
                }
                DetectResult::FoundUnsupportedEdition { path, edition } => {
                    println!(
                        "Found a {edition:?} GTA V install at {}, but that edition is not \
                         supported yet.",
                        path.display()
                    );
                }
                DetectResult::NotFound => {
                    println!(
                        "Could not auto-detect a GTA V installation. Pass --game-path <folder> \
                         to specify it manually."
                    );
                }
            }
        }
        Command::ListMods => {
            let mut stmt = conn
                .prepare("SELECT id, name, status, installed_at FROM installed_mod ORDER BY id")?;
            let rows = stmt.query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            })?;
            let mut any = false;
            for row in rows {
                let (id, name, status, installed_at) = row?;
                println!("#{id}  {name}  [{status}]  installed {installed_at}");
                any = true;
            }
            if !any {
                println!("(no mods installed yet)");
            }
        }
        Command::Install {
            path,
            name,
            yes,
            no_backup,
        } => {
            let (game_root, provider) = resolve_provider(&cli.game_path, cli.mode)?;
            let input_path = std::path::Path::new(&path);
            let plan = gtavmm_core::mod_analyzer::classify(input_path, provider.as_ref())?;
            let name = name.unwrap_or_else(|| {
                input_path
                    .file_name()
                    .map(|n| n.to_string_lossy().into_owned())
                    .unwrap_or_else(|| "unnamed mod".to_string())
            });

            let backup_root = db_path
                .parent()
                .expect("db path always has a parent")
                .join("backups")
                .join(chrono::Utc::now().format("%Y%m%d%H%M%S%3f").to_string());

            let options = gtavmm_core::install::InstallOptions {
                auto_backup: !no_backup,
                override_foreign_conflicts: yes,
            };

            match gtavmm_core::install::install(
                &mut conn,
                &name,
                &plan,
                &game_root,
                &backup_root,
                options,
            )? {
                gtavmm_core::install::InstallOutcome::Success {
                    installed_mod_id,
                    files_written,
                } => {
                    println!("Installed '{name}' (#{installed_mod_id}), {files_written} file(s) written.");
                }
                gtavmm_core::install::InstallOutcome::ProtectedFileBlocked(paths) => {
                    println!("Refused: this mod would write to protected core file(s):");
                    for p in paths {
                        println!("  {}", p.display());
                    }
                }
                gtavmm_core::install::InstallOutcome::RequiresOverride(report) => {
                    println!("This install collides with existing mod file(s):");
                    for conflict in &report.foreign_conflicts {
                        println!(
                            "  {} (owned by '{}')",
                            conflict.path.display(),
                            conflict.owner_name
                        );
                    }
                    if let Some(suggestion) = &report.self_update {
                        println!(
                            "(note: this also looks {:.0}% like an update to '{}' — if that's what you intended, this override still applies)",
                            suggestion.overlap_ratio * 100.0,
                            suggestion.existing_name
                        );
                    }
                    println!("Re-run with --yes to overwrite anyway.");
                }
            }
        }
        Command::Uninstall { id } => {
            let (game_root, _) = require_game_root(&cli.game_path)?;
            let recycle_root = db_path.parent().unwrap().join("recycle_bin");
            gtavmm_core::uninstall::uninstall(&mut conn, id, &game_root, &recycle_root)?;
            println!("Uninstalled mod #{id} (recoverable from the recycle bin for 15 days).");
        }
        Command::Enable { id } => {
            let staging_root = db_path.parent().unwrap().join("staging");
            gtavmm_core::state::enable(&conn, id, &staging_root)?;
            println!("Enabled mod #{id}.");
        }
        Command::Disable { id } => {
            let staging_root = db_path.parent().unwrap().join("staging");
            gtavmm_core::state::disable(&conn, id, &staging_root)?;
            println!("Disabled mod #{id}.");
        }
        Command::RecycleBin { action } => match action {
            RecycleBinAction::List => {
                let mut stmt = conn.prepare(
                    "SELECT id, original_installed_mod_id, deleted_at, expires_at \
                     FROM recycle_bin_entry ORDER BY deleted_at DESC",
                )?;
                let rows = stmt.query_map([], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, Option<i64>>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })?;
                let mut any = false;
                for row in rows {
                    let (id, mod_id, deleted_at, expires_at) = row?;
                    println!("#{id}  mod={mod_id:?}  deleted {deleted_at}  expires {expires_at}");
                    any = true;
                }
                if !any {
                    println!("(recycle bin is empty)");
                }
            }
            RecycleBinAction::Restore { id } => {
                let (game_root, _) = require_game_root(&cli.game_path)?;
                let backup_root = db_path.parent().unwrap().join("backups");
                gtavmm_core::recycle_bin::restore(&mut conn, id, &game_root, &backup_root)?;
                println!("Restored recycle bin entry #{id}.");
            }
        },
        Command::History { mod_id } => {
            let events = gtavmm_core::history::list(&conn, mod_id)?;
            if events.is_empty() {
                println!("(no history yet)");
            } else {
                for event in events {
                    println!(
                        "[{}] {:?} mod={:?} success={} {}",
                        event.timestamp,
                        event.event_type,
                        event.installed_mod_id,
                        event.success,
                        event.error_message.unwrap_or_default(),
                    );
                }
            }
        }
        Command::CheckComponents => {
            let (game_root, _) = require_game_root(&cli.game_path)?;
            for status in gtavmm_core::components::check_all(&game_root) {
                let mark = if status.is_installed { "OK" } else { "MISSING" };
                println!("[{mark}] {}", status.component.display_name());
                if !status.is_installed {
                    println!(
                        "        download: {}",
                        status.component.official_download_url()
                    );
                }
            }
        }
        Command::Inspect { path } => {
            let (_game_root, provider) = resolve_provider(&cli.game_path, cli.mode)?;
            let plan = gtavmm_core::mod_analyzer::classify(
                std::path::Path::new(&path),
                provider.as_ref(),
            )?;
            println!("Format: {:?}", plan.format);
            println!("Would write {} file(s):", plan.files.len());
            for file in &plan.files {
                println!("  {}", file.target.display());
            }
            println!("\n(read-only preview — nothing was written or recorded)");
        }
        Command::FullBackup { action } => {
            let backup_root = db_path.parent().unwrap().join("backups");
            match action {
                FullBackupAction::Create => {
                    let (game_root, _) = require_game_root(&cli.game_path)?;
                    let zip_path = gtavmm_core::full_backup::create(&game_root, &backup_root)?;
                    println!("Created full backup: {}", zip_path.display());
                }
                FullBackupAction::List => {
                    let backups = gtavmm_core::full_backup::list(&backup_root)?;
                    if backups.is_empty() {
                        println!("(no full backups yet)");
                    } else {
                        for path in backups {
                            println!("{}", path.display());
                        }
                    }
                }
                FullBackupAction::Restore { path } => {
                    let (game_root, _) = require_game_root(&cli.game_path)?;
                    gtavmm_core::full_backup::restore(&path, &game_root)?;
                    println!(
                        "Restored {} into {}\\mods",
                        path.display(),
                        game_root.display()
                    );
                }
            }
        }
        Command::CheckUpdate => {
            let current_version = env!("CARGO_PKG_VERSION");
            match gtavmm_core::update_check::check(current_version) {
                Ok(result) => {
                    if result.update_available {
                        println!(
                            "Update available: {} -> {}",
                            result.current_version, result.latest_version
                        );
                        match result.platform_download_url {
                            Some(url) => println!("Download: {url}"),
                            None => println!("Release page: {}", result.release_url),
                        }
                        println!(
                            "(This only downloads — this build doesn't apply updates \
                             automatically yet; that needs the desktop app, not the CLI.)"
                        );
                    } else {
                        println!("You're up to date (v{current_version}).");
                    }
                }
                Err(e) => println!("Could not check for updates: {e}"),
            }
        }
        Command::ReportCrash { description } => {
            let draft = gtavmm_core::crash_report::build(
                env!("CARGO_PKG_VERSION"),
                std::env::consts::OS,
                &description,
            );
            println!("{}", draft.github_issue_url);
        }
        Command::Export { output } => {
            gtavmm_core::xlsx_export::export(&conn, &output)?;
            println!("Exported to {}", output.display());
        }
        Command::Scan { path } => match gtavmm_core::malware_scan::scan_path(&path)? {
            gtavmm_core::malware_scan::ScanOutcome::Clean => println!("Clean — no threats found."),
            gtavmm_core::malware_scan::ScanOutcome::ThreatDetected { details } => {
                println!("THREAT DETECTED in {}", path.display());
                if let Some(details) = details {
                    println!("{details}");
                }
                println!("Do not install this file.");
            }
            gtavmm_core::malware_scan::ScanOutcome::Unavailable { reason } => {
                println!("Could not scan: {reason}");
            }
        },
        Command::FivemResourceOrder { resources_dir } => {
            match gtavmm_core::fivem::resolve_load_order(&resources_dir) {
                Ok(order) => {
                    if order.is_empty() {
                        println!("(no resources found under {})", resources_dir.display());
                    } else {
                        println!("Suggested load order (ensure lines, in this order):");
                        for name in order {
                            println!("ensure {name}");
                        }
                    }
                }
                Err(gtavmm_core::CoreError::DependencyGraph { reason }) => {
                    println!("Could not compute a load order: {reason}");
                }
                Err(err) => return Err(err.into()),
            }
        }
        Command::Profile { action } => match action {
            ProfileAction::Create { name } => {
                let id = gtavmm_core::profile::create(&conn, &name)?;
                println!("Created profile '{name}' (#{id}).");
            }
            ProfileAction::List => {
                let profiles = gtavmm_core::profile::list(&conn)?;
                if profiles.is_empty() {
                    println!("(no profiles yet)");
                } else {
                    for p in profiles {
                        let marker = if p.is_active { "*" } else { " " };
                        println!("{marker} #{}  {}  (created {})", p.id, p.name, p.created_at);
                    }
                }
            }
            ProfileAction::Delete { id } => {
                gtavmm_core::profile::delete(&conn, id)?;
                println!("Deleted profile #{id} (its mods were left untouched).");
            }
            ProfileAction::AddMod { profile_id, mod_id } => {
                gtavmm_core::profile::add_mod(&conn, profile_id, mod_id)?;
                println!("Added mod #{mod_id} to profile #{profile_id}.");
            }
            ProfileAction::RemoveMod { profile_id, mod_id } => {
                gtavmm_core::profile::remove_mod(&conn, profile_id, mod_id)?;
                println!("Removed mod #{mod_id} from profile #{profile_id}.");
            }
            ProfileAction::Switch { id } => {
                let staging_root = db_path.parent().unwrap().join("staging");
                let outcome = gtavmm_core::profile::switch(&conn, id, &staging_root)?;
                println!(
                    "Switched to profile #{id}: enabled {:?}, disabled {:?}.",
                    outcome.enabled, outcome.disabled
                );
            }
            ProfileAction::Export { id, output } => {
                let export = gtavmm_core::profile::export(&conn, id)?;
                let json = serde_json::to_string_pretty(&export)
                    .map_err(|e| anyhow::anyhow!("failed to serialize profile export: {e}"))?;
                std::fs::write(&output, json)?;
                println!(
                    "Exported profile #{id} ('{}', {} mod(s)) to {}.",
                    export.name,
                    export.mod_names.len(),
                    output.display()
                );
            }
            ProfileAction::Import { path } => {
                let json = std::fs::read_to_string(&path)?;
                let export: gtavmm_core::profile::ProfileExport = serde_json::from_str(&json)
                    .map_err(|e| anyhow::anyhow!("failed to parse profile export: {e}"))?;
                let outcome = gtavmm_core::profile::import(&conn, &export)?;
                println!(
                    "Imported '{}' as profile #{}: matched {:?}.",
                    export.name, outcome.profile_id, outcome.matched
                );
                if !outcome.not_found_locally.is_empty() {
                    println!(
                        "Not found locally (install these yourself first, then `profile add-mod`): {:?}",
                        outcome.not_found_locally
                    );
                }
            }
        },
        Command::Ai { action } => match action {
            AiAction::Enable {
                provider,
                model,
                endpoint,
            } => {
                let provider = match provider {
                    AiProviderArg::Ollama => gtavmm_core::ai_assistant::AiProviderKind::Ollama,
                    AiProviderArg::Cloud => gtavmm_core::ai_assistant::AiProviderKind::Cloud,
                };
                gtavmm_core::ai_assistant::enable(&conn, provider, model, endpoint)?;
                println!(
                    "AI assistant enabled (provider: {:?}). It stays opt-in and read-only — \
                     no automated fixes are applied.",
                    provider
                );
                if matches!(provider, gtavmm_core::ai_assistant::AiProviderKind::Cloud)
                    && !gtavmm_core::ai_assistant::has_cloud_api_key()
                {
                    println!(
                        "No cloud API key is set yet — run `ai set-api-key` before `ai diagnose`."
                    );
                }
            }
            AiAction::Disable => {
                gtavmm_core::ai_assistant::disable(&conn)?;
                println!("AI assistant disabled.");
            }
            AiAction::Status => {
                let settings = gtavmm_core::ai_assistant::load_settings(&conn)?;
                println!("Enabled: {}", settings.enabled);
                println!("Provider: {:?}", settings.provider);
                println!(
                    "Ollama reachable at localhost:11434: {}",
                    gtavmm_core::ai_assistant::ollama_available()
                );
                println!(
                    "Cloud API key set: {}",
                    gtavmm_core::ai_assistant::has_cloud_api_key()
                );
            }
            AiAction::SetApiKey => {
                use std::io::BufRead;
                println!("Paste the API key and press Enter (input is not displayed):");
                let mut key = String::new();
                std::io::stdin().lock().read_line(&mut key)?;
                let key = key.trim();
                if key.is_empty() {
                    return Err(anyhow::anyhow!("no key entered"));
                }
                gtavmm_core::ai_assistant::set_cloud_api_key(key)?;
                println!("API key saved to the OS credential store.");
            }
            AiAction::Diagnose { file } => {
                let context = match file {
                    Some(path) => std::fs::read_to_string(&path)?,
                    None => {
                        use std::io::Read;
                        let mut buf = String::new();
                        std::io::stdin().lock().read_to_string(&mut buf)?;
                        buf
                    }
                };
                let diagnosis = gtavmm_core::ai_assistant::diagnose(&conn, &context)?;
                println!("{diagnosis}");
            }
            AiAction::ListKnownFixes => {
                let rules = gtavmm_core::ai_assistant::known_fixes::load_known_fixes()?;
                for r in rules {
                    println!("{}  {}", r.id, r.title);
                }
            }
            AiAction::PlanKnownFix { rule_id } => {
                print_known_fix_plan(&rule_id)?;
            }
            AiAction::ApplyKnownFix { rule_id, yes } => {
                let plan = print_known_fix_plan(&rule_id)?;
                if !yes {
                    println!("\nNot applying — pass --yes to execute this Plan (this is the \"同意\" step).");
                    return Ok(());
                }
                let (game_root, _) = require_game_root(&cli.game_path)?;
                let staging_root = db_path.parent().unwrap().join("staging");
                let recycle_bin_root = db_path.parent().unwrap().join("recycle_bin");
                let exec_ctx = gtavmm_core::ai_assistant::action_schema::ExecutionContext {
                    game_root: &game_root,
                    staging_root: &staging_root,
                    recycle_bin_root: &recycle_bin_root,
                };
                let approved: Vec<usize> = (0..plan.len()).collect();
                let results = gtavmm_core::ai_assistant::action_schema::execute_plan(
                    &mut conn, &plan, &approved, &exec_ctx,
                );
                println!();
                for r in results {
                    match r.result {
                        Ok(()) => println!("  [{}] ok", r.index),
                        Err(e) => println!("  [{}] failed: {e}", r.index),
                    }
                }
            }
        },
        Command::Prompt { action } => match action {
            PromptAction::Create { name, content } => {
                let id = gtavmm_core::prompt_template::create(&conn, &name, &content)?;
                println!("Created prompt template '{name}' (#{id}).");
            }
            PromptAction::List => {
                let templates = gtavmm_core::prompt_template::list(&conn)?;
                if templates.is_empty() {
                    println!("(no prompt templates yet)");
                } else {
                    for t in templates {
                        println!("#{}  {}  (updated {})", t.id, t.name, t.updated_at);
                    }
                }
            }
            PromptAction::Update { id, name, content } => {
                gtavmm_core::prompt_template::update(&conn, id, &name, &content)?;
                println!("Updated prompt template #{id}.");
            }
            PromptAction::Delete { id } => {
                gtavmm_core::prompt_template::delete(&conn, id)?;
                println!("Deleted prompt template #{id}.");
            }
        },
        Command::ConvertVehicle {
            dlc_rpf,
            output_dir,
        } => {
            let report = gtavmm_core::sp_to_fivem::convert_vehicle_pack(&dlc_rpf, &output_dir)?;
            println!(
                "Converted {} -> {}",
                dlc_rpf.display(),
                output_dir.display()
            );
            println!("  data/:   {:?}", report.data_files);
            println!("  stream/: {:?}", report.stream_files);
            if !report.skipped_files.is_empty() {
                println!("  skipped: {:?}", report.skipped_files);
            }
        }
    }

    Ok(())
}
