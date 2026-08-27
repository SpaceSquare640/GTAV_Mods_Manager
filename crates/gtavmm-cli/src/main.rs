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
    }
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
            let (game_root, edition) = require_game_root(&cli.game_path)?;
            let input_path = std::path::Path::new(&path);
            let provider = provider_for(game_root.clone(), &edition, cli.mode);
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
            let (game_root, edition) = require_game_root(&cli.game_path)?;
            let provider = provider_for(game_root, &edition, cli.mode);
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
    }

    Ok(())
}
