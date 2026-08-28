// SPDX-License-Identifier: AGPL-3.0-only

//! Plan → 同意 → 執行 safety model (v0.7.x): the **limited, closed** set of actions an
//! AI-generated Plan is allowed to be built from (design doc §3.2). Deliberately does
//! **not** introduce a new execution path — [`execute_action`] is a thin dispatcher onto
//! the exact same already-vetted functions the CLI/UI call directly ([`crate::state`],
//! [`crate::uninstall`], [`crate::profile`]), so a Plan can never bypass those modules'
//! own protected-file/conflict checks. The "double validation" the design doc describes
//! is this: (1) the Action enum itself is closed — an AI response can only select from
//! these seven variants, never free-form code or file paths, and (2) executing an
//! approved action still goes through the untouched safety checks already inside
//! `state`/`uninstall`/`profile`.
//!
//! **Honesty note**: [`Action::ReorderLoadOrder`] is **FiveM-only** — there is no
//! LSPDFR equivalent, and not because it isn't implemented yet: RAGE Plugin Hook has
//! no user-facing load-order file at all to write to (unlike FiveM's `server.cfg`),
//! so "LSPDFR callout order" in the design doc's Action Schema list turned out not to
//! correspond to anything that exists in the real LSPDFR/RPH ecosystem. For FiveM,
//! this dispatches to [`crate::fivem::apply_load_order`], which always **recomputes**
//! the order itself via [`crate::fivem::resolve_load_order`] rather than trusting an
//! externally-supplied list — the variant deliberately carries no `items` field, so an
//! AI-generated Plan can request *that* the order be resolved and applied, never
//! dictate *what* the order should be.
//!
//! [`Action::ReinstallMod`] had a related problem — the design doc's
//! `reinstall_mod(mod_id, version)` signature has no source-path field, but this crate
//! has no way to look one up on its own (it never downloads mods). Fixed by (1) adding
//! `source_path` to the [`Action`] variant itself, matching what [`crate::install::reinstall`]
//! actually needs, and (2) recording each install's source path in
//! `installed_mod.source_path` (schema v5) so a *same-version* reinstall from the
//! original file/folder is possible without asking again — a *different* version still
//! requires the caller to supply where that version's package lives, since this project
//! never fetches one on the user's behalf.

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::CoreResult;

/// The closed set of actions an AI Plan may be built from. See the module docs for
/// [`Action::ReorderLoadOrder`]'s FiveM-only scope.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    DisableMod { mod_id: i64 },
    EnableMod { mod_id: i64 },
    UninstallMod { mod_id: i64 },
    ReinstallMod {
        mod_id: i64,
        source_path: PathBuf,
        version: String,
    },
    /// FiveM only — see module docs for why LSPDFR has no equivalent. No `items`
    /// field: the order is always recomputed from `resources_root`, never trusted
    /// from an external source.
    ReorderLoadOrder {
        resources_root: PathBuf,
        server_cfg_path: PathBuf,
    },
    SwitchProfile { profile_id: i64 },
}

/// One line item in a Plan: the action plus the (human-readable) reason it was
/// proposed, so a reviewing user always sees *why*, never just *what*.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct PlanItem {
    pub action: Action,
    pub reason: String,
}

/// The result of attempting one approved [`PlanItem`], keeping the original index so a
/// caller can line results back up against the Plan it displayed.
#[derive(Debug)]
pub struct PlanItemResult {
    pub index: usize,
    pub result: CoreResult<()>,
}

/// Paths the executor needs to reach the same functions the CLI/UI already call
/// directly — nothing here is new, it's exactly what `state`/`uninstall`/`profile`
/// already require.
pub struct ExecutionContext<'a> {
    pub game_root: &'a Path,
    pub staging_root: &'a Path,
    pub recycle_bin_root: &'a Path,
    /// Needed only for [`Action::ReinstallMod`] — where overwritten files get backed
    /// up during the reinstall's `install` half (same role as the CLI/UI's own
    /// per-install-attempt backup folder).
    pub backup_root: &'a Path,
    /// Needed only for [`Action::ReinstallMod`] — [`crate::mod_analyzer::classify`]
    /// requires a provider to know this mode's file-layout conventions.
    pub provider: &'a dyn crate::providers::ModeProvider,
}

/// Runs every `approved_indices`-selected item from `plan`, in order, against a real
/// database connection — continuing past a failed item rather than aborting the whole
/// batch, and reporting each item's own result (per design doc §3.3: "逐項勾選同意" +
/// per-item result reporting, not all-or-nothing).
pub fn execute_plan(
    conn: &mut Connection,
    plan: &[PlanItem],
    approved_indices: &[usize],
    ctx: &ExecutionContext,
) -> Vec<PlanItemResult> {
    approved_indices
        .iter()
        .filter_map(|&index| plan.get(index).map(|item| (index, item)))
        .map(|(index, item)| PlanItemResult {
            index,
            result: execute_action(conn, &item.action, ctx),
        })
        .collect()
}

/// Executes a single action by dispatching to the corresponding already-vetted
/// function. See the module docs for the two variants that always return an error.
pub fn execute_action(
    conn: &mut Connection,
    action: &Action,
    ctx: &ExecutionContext,
) -> CoreResult<()> {
    match action {
        Action::DisableMod { mod_id } => crate::state::disable(conn, *mod_id, ctx.staging_root),
        Action::EnableMod { mod_id } => crate::state::enable(conn, *mod_id, ctx.staging_root),
        Action::UninstallMod { mod_id } => crate::uninstall::uninstall(
            conn,
            *mod_id,
            ctx.game_root,
            ctx.recycle_bin_root,
        ),
        Action::SwitchProfile { profile_id } => {
            crate::profile::switch(conn, *profile_id, ctx.staging_root).map(|_| ())
        }
        Action::ReinstallMod {
            mod_id,
            source_path,
            version,
        } => crate::install::reinstall(
            conn,
            *mod_id,
            source_path,
            version,
            ctx.provider,
            ctx.game_root,
            ctx.backup_root,
            ctx.recycle_bin_root,
            crate::install::InstallOptions::default(),
        )
        .map(|_| ()),
        Action::ReorderLoadOrder {
            resources_root,
            server_cfg_path,
        } => crate::fivem::apply_load_order(resources_root, server_cfg_path).map(|_| ()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(
        dir: &std::path::Path,
    ) -> (
        std::path::PathBuf,
        std::path::PathBuf,
        std::path::PathBuf,
        std::path::PathBuf,
    ) {
        (
            dir.join("game"),
            dir.join("staging"),
            dir.join("recycle"),
            dir.join("backups"),
        )
    }

    #[test]
    fn execute_plan_runs_only_approved_items_and_reports_each_result() {
        let dir = tempfile::tempdir().unwrap();
        let (game_root, staging_root, recycle_bin_root, backup_root) = ctx(dir.path());
        std::fs::create_dir_all(&game_root).unwrap();
        let provider = crate::providers::LegacySpProvider::new(game_root.clone());

        let mut conn = crate::db::open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO installed_mod (name, source_type, install_path, status) \
             VALUES ('ModA', 'asi', '', 'active')",
            [],
        )
        .unwrap();
        let mod_id = conn.last_insert_rowid();

        let plan = vec![
            PlanItem {
                action: Action::DisableMod { mod_id },
                reason: "test disable".to_string(),
            },
            PlanItem {
                action: Action::UninstallMod { mod_id: 9999 },
                reason: "unknown mod id, should error not silently succeed".to_string(),
            },
        ];

        let exec_ctx = ExecutionContext {
            game_root: &game_root,
            staging_root: &staging_root,
            recycle_bin_root: &recycle_bin_root,
            backup_root: &backup_root,
            provider: &provider,
        };
        let results = execute_plan(&mut conn, &plan, &[0, 1], &exec_ctx);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].index, 0);
        assert!(results[0].result.is_ok());
        assert_eq!(results[1].index, 1);
        assert!(results[1].result.is_err(), "unknown mod id must fail, not silently succeed");

        let status: String = conn
            .query_row(
                "SELECT status FROM installed_mod WHERE id = ?1",
                [mod_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "disabled");
    }

    #[test]
    fn execute_plan_skips_unapproved_items() {
        let dir = tempfile::tempdir().unwrap();
        let (game_root, staging_root, recycle_bin_root, backup_root) = ctx(dir.path());
        std::fs::create_dir_all(&game_root).unwrap();
        let provider = crate::providers::LegacySpProvider::new(game_root.clone());

        let mut conn = crate::db::open_in_memory().unwrap();
        conn.execute(
            "INSERT INTO installed_mod (name, source_type, install_path, status) \
             VALUES ('ModA', 'asi', '', 'active')",
            [],
        )
        .unwrap();
        let mod_id = conn.last_insert_rowid();

        let plan = vec![PlanItem {
            action: Action::DisableMod { mod_id },
            reason: "not approved".to_string(),
        }];

        let exec_ctx = ExecutionContext {
            game_root: &game_root,
            staging_root: &staging_root,
            recycle_bin_root: &recycle_bin_root,
            backup_root: &backup_root,
            provider: &provider,
        };
        let results = execute_plan(&mut conn, &plan, &[], &exec_ctx);

        assert!(results.is_empty());
        let status: String = conn
            .query_row(
                "SELECT status FROM installed_mod WHERE id = ?1",
                [mod_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(status, "active", "unapproved item must not have run");
    }

    #[test]
    fn reorder_load_order_action_actually_writes_server_cfg_via_the_real_pipeline() {
        let dir = tempfile::tempdir().unwrap();
        let (game_root, staging_root, recycle_bin_root, backup_root) = ctx(dir.path());
        let provider = crate::providers::LegacySpProvider::new(game_root.clone());
        let mut conn = crate::db::open_in_memory().unwrap();

        let resources_root = dir.path().join("resources");
        let resource_dir = resources_root.join("core-lib");
        std::fs::create_dir_all(&resource_dir).unwrap();
        std::fs::write(
            resource_dir.join("fxmanifest.lua"),
            "fx_version 'cerulean'\n",
        )
        .unwrap();
        let server_cfg_path = dir.path().join("server.cfg");
        std::fs::write(&server_cfg_path, "sv_hostname \"Test\"\n").unwrap();

        let exec_ctx = ExecutionContext {
            game_root: &game_root,
            staging_root: &staging_root,
            recycle_bin_root: &recycle_bin_root,
            backup_root: &backup_root,
            provider: &provider,
        };

        execute_action(
            &mut conn,
            &Action::ReorderLoadOrder {
                resources_root,
                server_cfg_path: server_cfg_path.clone(),
            },
            &exec_ctx,
        )
        .unwrap();

        let contents = std::fs::read_to_string(&server_cfg_path).unwrap();
        assert!(contents.contains("sv_hostname \"Test\""));
        assert!(contents.contains("ensure core-lib"));
    }

    #[test]
    fn reinstall_mod_action_actually_reinstalls_via_the_real_pipeline() {
        let dir = tempfile::tempdir().unwrap();
        let (game_root, staging_root, recycle_bin_root, backup_root) = ctx(dir.path());
        std::fs::create_dir_all(&game_root).unwrap();
        let provider = crate::providers::LegacySpProvider::new(game_root.clone());

        let old_source = dir.path().join("cool_mod.asi");
        std::fs::write(&old_source, b"v1").unwrap();
        let plan = crate::mod_analyzer::classify(&old_source, &provider).unwrap();
        let mut conn = crate::db::open_in_memory().unwrap();
        let outcome = crate::install::install(
            &mut conn,
            "Cool Mod",
            &plan,
            &game_root,
            &backup_root,
            crate::install::InstallOptions::default(),
            &old_source,
        )
        .unwrap();
        let crate::install::InstallOutcome::Success { installed_mod_id: old_id, .. } = outcome
        else {
            panic!("expected Success");
        };

        let new_source = dir.path().join("cool_mod_v2.asi");
        std::fs::write(&new_source, b"v2").unwrap();

        let exec_ctx = ExecutionContext {
            game_root: &game_root,
            staging_root: &staging_root,
            recycle_bin_root: &recycle_bin_root,
            backup_root: &backup_root,
            provider: &provider,
        };
        let result = execute_action(
            &mut conn,
            &Action::ReinstallMod {
                mod_id: old_id,
                source_path: new_source,
                version: "1.1".to_string(),
            },
            &exec_ctx,
        );
        assert!(result.is_ok(), "{result:?}");

        let old_status: String = conn
            .query_row(
                "SELECT status FROM installed_mod WHERE id = ?1",
                [old_id],
                |r| r.get(0),
            )
            .unwrap();
        assert_eq!(old_status, "uninstalled");
    }

    #[test]
    fn action_serializes_with_a_tagged_shape_suitable_for_the_known_fixes_json() {
        let action = Action::DisableMod { mod_id: 42 };
        let json = serde_json::to_string(&action).unwrap();
        assert_eq!(json, r#"{"action":"disable_mod","mod_id":42}"#);
        let round_tripped: Action = serde_json::from_str(&json).unwrap();
        assert_eq!(round_tripped, action);
    }
}
