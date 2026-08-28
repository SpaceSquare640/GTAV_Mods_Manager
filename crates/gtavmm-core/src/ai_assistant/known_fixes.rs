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
//! **Design note — why rules match by name pattern, not a fixed mod id**: an earlier
//! version of this file held a demo rule with a hardcoded `mod_id: 0`, which exposed a
//! real problem — `installed_mod.id` is a per-database autoincrement value with no
//! stable meaning across different users' installs, so a rule authored once and shipped
//! to everyone can never reference a specific id. [`RuleMatch`] resolves against each
//! *caller's own* `installed_mod` table by name pattern instead, so the same bundled
//! rule works (or correctly doesn't apply) regardless of what a given install actually
//! looks like.
//!
//! **Honesty note**: the bundled [`KNOWN_FIXES_JSON`] currently holds two rules — see
//! each rule's own `description` field for how it's sourced. One repeats documented
//! community guidance (not independently verified by this project); the other
//! (duplicate-named active mods) is a structural fact about the install itself, not a
//! claim about mod behavior, so it doesn't carry the same "unverified" caveat. This is
//! a starting point, not a comprehensive fix database.

use rusqlite::Connection;
use serde::Deserialize;

use crate::ai_assistant::action_schema::{Action, PlanItem};
use crate::error::{CoreError, CoreResult};

const KNOWN_FIXES_JSON: &str = include_str!("known_fixes.json");

#[derive(Debug, Clone, Deserialize)]
pub struct KnownFixRule {
    pub id: String,
    pub title: String,
    pub description: String,
    #[serde(rename = "match")]
    pub rule_match: RuleMatch,
}

/// How a rule's applicability — and the concrete [`Action`]s it expands to — is
/// resolved against a real `installed_mod` table. Tagged on the JSON `type` field so
/// new match kinds can be added later without breaking existing bundled rules.
#[derive(Debug, Clone, Deserialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum RuleMatch {
    /// Applies when 2+ currently-*active* mods have a name containing (case-
    /// insensitively) any of `name_contains_any`. Proposes disabling every match
    /// after the first (ordered by install id, i.e. install order) — never the AI's
    /// choice of *which* to keep, just a deterministic, inspectable rule.
    MultipleActiveDisableAllButFirst { name_contains_any: Vec<String> },
    /// Applies when 2+ currently-*active* mods share the exact same name
    /// (case-insensitively) — almost always a user reinstalling the same mod without
    /// removing the old copy first. Unlike the pattern-based match above, this one
    /// needs no `name_contains_any` list: it's a structural fact about the install
    /// (two rows claiming to be the same mod), not community folklore about specific
    /// mod names. Proposes disabling every duplicate except the **newest** (highest
    /// install id) — the newest copy is more likely the one the user actually wants.
    DuplicateActiveNamesDisableAllButNewest,
}

/// Parses the bundled rule library. Cheap enough (a handful of KB of JSON at most) to
/// call on every lookup rather than caching — no lock/lazy-static machinery needed.
pub fn load_known_fixes() -> CoreResult<Vec<KnownFixRule>> {
    serde_json::from_str(KNOWN_FIXES_JSON).map_err(|e| CoreError::ActionSchema {
        reason: format!(
            "bundled known_fixes.json is malformed (this is a bug, not a \
             user-facing condition): {e}"
        ),
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

/// Resolves a known-fix rule against `conn`'s real `installed_mod` table and expands
/// it into the [`PlanItem`]s a caller shows the user for approval — this does **not**
/// execute anything, matching the Plan → 同意 → 執行 flow. Returns
/// [`CoreError::ActionSchema`] if the rule genuinely doesn't apply to this install
/// (e.g. fewer than two matching active mods) rather than an empty, vacuous Plan.
pub fn build_plan_from_known_fix(conn: &Connection, rule_id: &str) -> CoreResult<Vec<PlanItem>> {
    let rule = find_rule(rule_id)?;
    match rule.rule_match {
        RuleMatch::MultipleActiveDisableAllButFirst { name_contains_any } => {
            let matches = find_active_mods_matching_any(conn, &name_contains_any)?;
            if matches.len() < 2 {
                return Err(CoreError::ActionSchema {
                    reason: format!(
                        "rule '{}' does not apply — found {} matching active mod(s), need 2+",
                        rule.id,
                        matches.len()
                    ),
                });
            }
            let (kept_id, kept_name) = &matches[0];
            Ok(matches[1..]
                .iter()
                .map(|(mod_id, name)| PlanItem {
                    action: Action::DisableMod { mod_id: *mod_id },
                    reason: format!(
                        "{}: {} — keeping '{kept_name}' (#{kept_id}) active, disabling '{name}' (#{mod_id}).",
                        rule.title, rule.description
                    ),
                })
                .collect())
        }
        RuleMatch::DuplicateActiveNamesDisableAllButNewest => {
            let groups = find_duplicate_active_name_groups(conn)?;
            if groups.is_empty() {
                return Err(CoreError::ActionSchema {
                    reason: format!(
                        "rule '{}' does not apply — no two active mods share the same name",
                        rule.id
                    ),
                });
            }
            let mut items = Vec::new();
            for mut group in groups {
                // Newest (highest id) kept; ORDER BY id ASC means it's last.
                let (kept_id, kept_name) = group.pop().expect("group has 2+ entries");
                for (mod_id, name) in group {
                    items.push(PlanItem {
                        action: Action::DisableMod { mod_id },
                        reason: format!(
                            "{}: {} — keeping the newest copy '{kept_name}' (#{kept_id}), \
                             disabling the older duplicate '{name}' (#{mod_id}).",
                            rule.title, rule.description
                        ),
                    });
                }
            }
            Ok(items)
        }
    }
}

/// Groups of active `installed_mod` rows sharing the exact same name
/// (case-insensitively), each group ordered by id ascending (oldest first, newest
/// last) — only groups with 2+ members are returned.
fn find_duplicate_active_name_groups(conn: &Connection) -> CoreResult<Vec<Vec<(i64, String)>>> {
    let mut stmt =
        conn.prepare("SELECT id, name FROM installed_mod WHERE status = 'active' ORDER BY id ASC")?;
    let rows: Vec<(i64, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    drop(stmt);

    let mut groups: std::collections::BTreeMap<String, Vec<(i64, String)>> = Default::default();
    for (id, name) in rows {
        groups.entry(name.to_lowercase()).or_default().push((id, name));
    }
    Ok(groups.into_values().filter(|g| g.len() >= 2).collect())
}

/// Active `installed_mod` rows whose name contains (case-insensitively) any of
/// `patterns`, ordered by id (install order) so results are deterministic.
fn find_active_mods_matching_any(
    conn: &Connection,
    patterns: &[String],
) -> CoreResult<Vec<(i64, String)>> {
    let mut stmt = conn.prepare(
        "SELECT id, name FROM installed_mod WHERE status = 'active' ORDER BY id ASC",
    )?;
    let rows: Vec<(i64, String)> = stmt
        .query_map([], |row| Ok((row.get(0)?, row.get(1)?)))?
        .collect::<Result<_, _>>()?;
    Ok(rows
        .into_iter()
        .filter(|(_, name)| {
            let lower = name.to_lowercase();
            patterns.iter().any(|p| lower.contains(&p.to_lowercase()))
        })
        .collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_mod(conn: &Connection, name: &str, status: &str) -> i64 {
        conn.execute(
            "INSERT INTO installed_mod (name, source_type, install_path, status) \
             VALUES (?1, 'asi', '', ?2)",
            rusqlite::params![name, status],
        )
        .unwrap();
        conn.last_insert_rowid()
    }

    #[test]
    fn bundled_known_fixes_json_parses() {
        let rules = load_known_fixes().unwrap();
        assert!(!rules.is_empty(), "expected at least the real rule");
    }

    #[test]
    fn multiple_trainers_rule_proposes_disabling_all_but_the_first_installed() {
        let conn = crate::db::open_in_memory().unwrap();
        let first = insert_mod(&conn, "Menyoo PC Trainer", "active");
        let second = insert_mod(&conn, "Simple Trainer for GTA V", "active");
        insert_mod(&conn, "Unrelated ASI Mod", "active"); // shouldn't match at all

        let plan =
            build_plan_from_known_fix(&conn, "multiple-trainers-active-keep-first").unwrap();

        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].action, Action::DisableMod { mod_id: second });
        assert!(plan[0].reason.contains("Menyoo"));
        let _ = first;
    }

    #[test]
    fn rule_does_not_apply_with_fewer_than_two_matching_active_mods() {
        let conn = crate::db::open_in_memory().unwrap();
        insert_mod(&conn, "Menyoo PC Trainer", "active");
        insert_mod(&conn, "Some Other Mod", "active");

        let err =
            build_plan_from_known_fix(&conn, "multiple-trainers-active-keep-first").unwrap_err();
        assert!(matches!(err, CoreError::ActionSchema { .. }));
    }

    #[test]
    fn disabled_trainers_are_not_counted_as_matches() {
        let conn = crate::db::open_in_memory().unwrap();
        insert_mod(&conn, "Menyoo PC Trainer", "active");
        insert_mod(&conn, "Simple Trainer for GTA V", "disabled");

        let err =
            build_plan_from_known_fix(&conn, "multiple-trainers-active-keep-first").unwrap_err();
        assert!(matches!(err, CoreError::ActionSchema { .. }));
    }

    #[test]
    fn duplicate_names_rule_proposes_disabling_all_but_the_newest() {
        let conn = crate::db::open_in_memory().unwrap();
        let older = insert_mod(&conn, "Realistic Vehicle Handling", "active");
        let newer = insert_mod(&conn, "Realistic Vehicle Handling", "active");
        insert_mod(&conn, "Unrelated Mod", "active");

        let plan =
            build_plan_from_known_fix(&conn, "duplicate-active-mod-names-keep-newest").unwrap();

        assert_eq!(plan.len(), 1);
        assert_eq!(plan[0].action, Action::DisableMod { mod_id: older });
        assert!(plan[0].reason.contains(&format!("#{newer}")));
    }

    #[test]
    fn duplicate_names_rule_matches_case_insensitively() {
        let conn = crate::db::open_in_memory().unwrap();
        insert_mod(&conn, "Menyoo PC Trainer", "active");
        let dup = insert_mod(&conn, "menyoo pc trainer", "active");

        let plan =
            build_plan_from_known_fix(&conn, "duplicate-active-mod-names-keep-newest").unwrap();
        assert_eq!(plan.len(), 1);
        assert_ne!(plan[0].action, Action::DisableMod { mod_id: dup }, "the newest (dup) must be kept, not disabled");
    }

    #[test]
    fn duplicate_names_rule_does_not_apply_with_all_unique_names() {
        let conn = crate::db::open_in_memory().unwrap();
        insert_mod(&conn, "Mod A", "active");
        insert_mod(&conn, "Mod B", "active");

        let err = build_plan_from_known_fix(&conn, "duplicate-active-mod-names-keep-newest")
            .unwrap_err();
        assert!(matches!(err, CoreError::ActionSchema { .. }));
    }

    #[test]
    fn unknown_rule_id_errors() {
        let conn = crate::db::open_in_memory().unwrap();
        let err = build_plan_from_known_fix(&conn, "does-not-exist").unwrap_err();
        assert!(matches!(err, CoreError::ActionSchema { .. }));
    }
}
