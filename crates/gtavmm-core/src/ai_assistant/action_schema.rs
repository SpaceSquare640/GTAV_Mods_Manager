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
//! **Honesty note**: [`Action::ReinstallMod`] and [`Action::ReorderLoadOrder`] are part
//! of the schema (matching the design doc's Action Schema list) but have **no backing
//! implementation yet** — there is no `reinstall_mod` function anywhere in this crate,
//! and no mutation path exists for LSPDFR callout order or FiveM `ensure` order (FiveM's
//! [`crate::fivem::resolve_load_order`] is read-only, it only *suggests* an order). Both
//! variants exist so a Plan can be *displayed* faithfully, but [`execute_action`] refuses
//! to run either one rather than silently doing nothing.

use std::path::Path;

use rusqlite::Connection;
use serde::{Deserialize, Serialize};

use crate::error::{CoreError, CoreResult};

/// The closed set of actions an AI Plan may be built from. See the module docs for
/// which variants actually have a backing implementation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "action", rename_all = "snake_case")]
pub enum Action {
    DisableMod { mod_id: i64 },
    EnableMod { mod_id: i64 },
    UninstallMod { mod_id: i64 },
    /// No backing implementation yet — see module docs.
    ReinstallMod { mod_id: i64, version: String },
    /// No backing implementation yet — see module docs.
    ReorderLoadOrder { items: Vec<String> },
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
        Action::ReinstallMod { .. } => Err(CoreError::ActionSchema {
            reason: "reinstall_mod is not implemented in the core engine yet".to_string(),
        }),
        Action::ReorderLoadOrder { .. } => Err(CoreError::ActionSchema {
            reason: "reorder_load_order is not implemented in the core engine yet — no \
                     mutation path exists for LSPDFR callout order or FiveM ensure order"
                .to_string(),
        }),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn ctx(dir: &std::path::Path) -> (std::path::PathBuf, std::path::PathBuf, std::path::PathBuf) {
        (
            dir.join("game"),
            dir.join("staging"),
            dir.join("recycle"),
        )
    }

    #[test]
    fn execute_plan_runs_only_approved_items_and_reports_each_result() {
        let dir = tempfile::tempdir().unwrap();
        let (game_root, staging_root, recycle_bin_root) = ctx(dir.path());
        std::fs::create_dir_all(&game_root).unwrap();

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
                action: Action::ReorderLoadOrder { items: vec![] },
                reason: "unsupported action, should error not silently succeed".to_string(),
            },
        ];

        let exec_ctx = ExecutionContext {
            game_root: &game_root,
            staging_root: &staging_root,
            recycle_bin_root: &recycle_bin_root,
        };
        let results = execute_plan(&mut conn, &plan, &[0, 1], &exec_ctx);

        assert_eq!(results.len(), 2);
        assert_eq!(results[0].index, 0);
        assert!(results[0].result.is_ok());
        assert_eq!(results[1].index, 1);
        assert!(matches!(
            results[1].result,
            Err(CoreError::ActionSchema { .. })
        ));

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
        let (game_root, staging_root, recycle_bin_root) = ctx(dir.path());
        std::fs::create_dir_all(&game_root).unwrap();

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
    fn reinstall_and_reorder_are_schema_members_but_refuse_to_execute() {
        let dir = tempfile::tempdir().unwrap();
        let (game_root, staging_root, recycle_bin_root) = ctx(dir.path());
        let mut conn = crate::db::open_in_memory().unwrap();
        let exec_ctx = ExecutionContext {
            game_root: &game_root,
            staging_root: &staging_root,
            recycle_bin_root: &recycle_bin_root,
        };

        let err = execute_action(
            &mut conn,
            &Action::ReinstallMod {
                mod_id: 1,
                version: "1.0".to_string(),
            },
            &exec_ctx,
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::ActionSchema { .. }));

        let err = execute_action(
            &mut conn,
            &Action::ReorderLoadOrder { items: vec![] },
            &exec_ctx,
        )
        .unwrap_err();
        assert!(matches!(err, CoreError::ActionSchema { .. }));
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
