// SPDX-License-Identifier: AGPL-3.0-only

//! The known-fix rule library `apply_known_fix` (design doc §3.2) draws from. Per the
//! design discussion this ships alongside: rules are **bundled into the binary at
//! compile time** (`include_str!`'d from [`KNOWN_FIXES_JSON`]) and grown via reviewed
//! GitHub PRs, exactly like the rest of this codebase — never fetched from a remote
//! server (this project has no first-party server, and the AI itself never invents a
//! fix outside this closed set — see [`crate::ai_assistant::action_schema`]'s module
//! docs for why that matters). A future version may add a user-local override/extension
//! file for advanced users, but that isn't built yet.
//!
//! **Honesty note**: the bundled [`KNOWN_FIXES_JSON`] currently holds exactly one
//! example rule that proves the `rule_id -> Plan` expansion path end-to-end. It is not
//! a curated knowledge base of real, verified fixes — see the rule's own `description`
//! field.

use serde::Deserialize;

use crate::ai_assistant::action_schema::{Action, PlanItem};
use crate::error::{CoreError, CoreResult};

const KNOWN_FIXES_JSON: &str = include_str!("known_fixes.json");

#[derive(Debug, Clone, Deserialize)]
pub struct KnownFixRule {
    pub id: String,
    pub title: String,
    pub description: String,
    pub actions: Vec<Action>,
}

/// Parses the bundled rule library. Cheap enough (a handful of KB of JSON at most) to
/// call on every lookup rather than caching — no lock/lazy-static machinery needed.
pub fn load_known_fixes() -> CoreResult<Vec<KnownFixRule>> {
    serde_json::from_str(KNOWN_FIXES_JSON).map_err(|e| CoreError::ActionSchema {
        reason: format!("bundled known_fixes.json is malformed (this is a bug, not a \
                          user-facing condition): {e}"),
    })
}

fn find_rule(rule_id: &str) -> CoreResult<KnownFixRule> {
    load_known_fixes()?
        .into_iter()
        .find(|r| r.id == rule_id)
        .ok_or_else(|| CoreError::ActionSchema {
            reason: format!("no known-fix rule with id '{rule_id}'"),
        })
}

/// Expands a known-fix rule into the [`PlanItem`]s a caller shows the user for
/// approval — this does **not** execute anything, matching the Plan → 同意 → 執行 flow.
pub fn build_plan_from_known_fix(rule_id: &str) -> CoreResult<Vec<PlanItem>> {
    let rule = find_rule(rule_id)?;
    Ok(rule
        .actions
        .into_iter()
        .map(|action| PlanItem {
            action,
            reason: format!("{}: {}", rule.title, rule.description),
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn bundled_known_fixes_json_parses() {
        let rules = load_known_fixes().unwrap();
        assert!(!rules.is_empty(), "expected at least the example rule");
    }

    #[test]
    fn build_plan_from_known_fix_expands_the_example_rule() {
        let plan = build_plan_from_known_fix("duplicate-scripthookv-disable-older").unwrap();
        assert_eq!(plan.len(), 1);
        assert!(matches!(plan[0].action, Action::DisableMod { .. }));
        assert!(plan[0].reason.contains("ScriptHookV"));
    }

    #[test]
    fn unknown_rule_id_errors() {
        let err = build_plan_from_known_fix("does-not-exist").unwrap_err();
        assert!(matches!(err, CoreError::ActionSchema { .. }));
    }
}
