// SPDX-License-Identifier: AGPL-3.0-only

//! Compares a planned install's target files against existing `InstalledModFile` rows
//! (the "who owns which file" ownership tracking that lets the core distinguish, e.g.,
//! a Legacy SP mod's own file from a same-path file owned by a different mod/provider).
//!
//! Classifies each overlap per the MVP spec's three-tier policy:
//! - **Protected hit** — target matches `protected_files` → always fatal, no override,
//!   checked independently of everything else below.
//! - **Self-update** — the planned targets overlap mostly with one *existing active*
//!   mod's own files (overlap ratio ≥ [`SELF_UPDATE_OVERLAP_THRESHOLD`]) → suggested,
//!   not forced; caller may proceed without extra confirmation ("warn, one-click
//!   continue").
//! - **Foreign conflict** — a planned target matches a file owned by a *different*
//!   active mod → requires explicit override from the caller before `install`
//!   proceeds.

use std::collections::HashMap;
use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::CoreResult;
use crate::protected_files;

/// Overlap ratio (matching files / existing mod's total files) at or above which we
/// suggest "this looks like an update to an existing mod" rather than treating it as
/// an unrelated new install.
///
/// This is a suggestion only (see the MVP spec, section 4.2) — a human confirms it
/// before anything happens. One known ambiguity this doesn't try to resolve: if an
/// *unrelated* existing mod happens to own exactly one file and the new install's
/// only target coincidentally matches it, the ratio is 1.0 and this would suggest
/// "update" for a mod that isn't actually related. That's an acceptable tradeoff
/// precisely because the suggestion always goes through human review — it never
/// silently reclassifies a foreign conflict as safe.
pub const SELF_UPDATE_OVERLAP_THRESHOLD: f64 = 0.5;

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ProtectedHit {
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct ForeignConflict {
    pub owner_mod_id: i64,
    pub owner_name: String,
    pub path: PathBuf,
}

#[derive(Debug, Clone, PartialEq, serde::Serialize)]
pub struct SelfUpdateSuggestion {
    pub existing_mod_id: i64,
    pub existing_name: String,
    pub overlap_ratio: f64,
}

#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct ConflictReport {
    pub protected_hits: Vec<ProtectedHit>,
    pub foreign_conflicts: Vec<ForeignConflict>,
    pub self_update: Option<SelfUpdateSuggestion>,
}

impl ConflictReport {
    /// `true` if `install` must refuse to proceed without the caller supplying an
    /// explicit override (protected hits are never overridable and aren't included in
    /// this check — see [`ConflictReport::has_protected_hits`]).
    pub fn requires_explicit_override(&self) -> bool {
        !self.foreign_conflicts.is_empty()
    }

    pub fn has_protected_hits(&self) -> bool {
        !self.protected_hits.is_empty()
    }
}

/// One row of `installed_mod_file` joined with its owning mod, for active mods only.
struct ExistingFile {
    mod_id: i64,
    mod_name: String,
    target_path: String,
}

fn load_active_files(conn: &Connection) -> CoreResult<Vec<ExistingFile>> {
    let mut stmt = conn.prepare(
        "SELECT m.id, m.name, f.target_path \
         FROM installed_mod_file f \
         JOIN installed_mod m ON m.id = f.installed_mod_id \
         WHERE m.status = 'active'",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(ExistingFile {
            mod_id: row.get(0)?,
            mod_name: row.get(1)?,
            target_path: row.get(2)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

/// Analyzes `planned_targets` (absolute paths this install would write) against the
/// database's current active-mod file ownership.
pub fn analyze(conn: &Connection, planned_targets: &[PathBuf]) -> CoreResult<ConflictReport> {
    let mut report = ConflictReport::default();

    for target in planned_targets {
        if protected_files::is_protected(target) {
            report.protected_hits.push(ProtectedHit {
                path: target.clone(),
            });
        }
    }

    let existing = load_active_files(conn)?;
    let planned_set: std::collections::HashSet<&Path> =
        planned_targets.iter().map(PathBuf::as_path).collect();

    // Group existing files by owning mod so we can compute a per-mod overlap ratio.
    let mut by_mod: HashMap<i64, (String, Vec<String>)> = HashMap::new();
    for file in &existing {
        by_mod
            .entry(file.mod_id)
            .or_insert_with(|| (file.mod_name.clone(), Vec::new()))
            .1
            .push(file.target_path.clone());
    }

    let mut best_suggestion: Option<SelfUpdateSuggestion> = None;
    for (mod_id, (name, paths)) in &by_mod {
        if paths.is_empty() {
            continue;
        }
        let overlap_count = paths
            .iter()
            .filter(|p| planned_set.contains(Path::new(p.as_str())))
            .count();
        let ratio = overlap_count as f64 / paths.len() as f64;
        if ratio >= SELF_UPDATE_OVERLAP_THRESHOLD {
            let is_better = best_suggestion
                .as_ref()
                .is_none_or(|current| ratio > current.overlap_ratio);
            if is_better {
                best_suggestion = Some(SelfUpdateSuggestion {
                    existing_mod_id: *mod_id,
                    existing_name: name.clone(),
                    overlap_ratio: ratio,
                });
            }
        }
    }
    report.self_update = best_suggestion;

    let self_update_mod_id = report.self_update.as_ref().map(|s| s.existing_mod_id);
    for file in &existing {
        if Some(file.mod_id) == self_update_mod_id {
            continue; // this overlap is the accepted self-update, not a foreign conflict
        }
        if planned_set.contains(Path::new(file.target_path.as_str())) {
            report.foreign_conflicts.push(ForeignConflict {
                owner_mod_id: file.mod_id,
                owner_name: file.mod_name.clone(),
                path: PathBuf::from(&file.target_path),
            });
        }
    }

    Ok(report)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_active_mod(conn: &Connection, name: &str, files: &[&str]) -> i64 {
        conn.execute(
            "INSERT INTO installed_mod (name, source_type, install_path, status) \
             VALUES (?1, 'asi', '', 'active')",
            [name],
        )
        .unwrap();
        let mod_id = conn.last_insert_rowid();
        for file in files {
            conn.execute(
                "INSERT INTO installed_mod_file (installed_mod_id, target_path, file_hash) \
                 VALUES (?1, ?2, 'deadbeef')",
                rusqlite::params![mod_id, file],
            )
            .unwrap();
        }
        mod_id
    }

    #[test]
    fn no_conflict_on_fresh_targets() {
        let conn = crate::db::open_in_memory().unwrap();
        let report = analyze(&conn, &[PathBuf::from("/game/new_mod.asi")]).unwrap();
        assert!(report.protected_hits.is_empty());
        assert!(report.foreign_conflicts.is_empty());
        assert!(report.self_update.is_none());
    }

    #[test]
    fn protected_file_hit_is_always_flagged() {
        // Built with `.join`, not a hand-formatted backslash string — see
        // `protected_files::tests::game_path`'s comment for why that matters on Linux.
        let conn = crate::db::open_in_memory().unwrap();
        let path = PathBuf::from("game").join("GTA5.exe");
        let report = analyze(&conn, &[path]).unwrap();
        assert_eq!(report.protected_hits.len(), 1);
    }

    #[test]
    fn high_overlap_with_existing_mod_suggests_self_update() {
        let conn = crate::db::open_in_memory().unwrap();
        insert_active_mod(&conn, "CoolMod", &["/game/a.asi", "/game/b.dll"]);

        let planned = vec![PathBuf::from("/game/a.asi"), PathBuf::from("/game/b.dll")];
        let report = analyze(&conn, &planned).unwrap();

        let suggestion = report
            .self_update
            .clone()
            .expect("expected a self-update suggestion");
        assert_eq!(suggestion.existing_name, "CoolMod");
        assert_eq!(suggestion.overlap_ratio, 1.0);
        assert!(
            report.foreign_conflicts.is_empty(),
            "self-update overlap must not also be a foreign conflict"
        );
        assert!(!report.requires_explicit_override());
    }

    #[test]
    fn low_overlap_with_different_mod_is_foreign_conflict() {
        let conn = crate::db::open_in_memory().unwrap();
        insert_active_mod(
            &conn,
            "OtherMod",
            &[
                "/game/shared.dll",
                "/game/other1.asi",
                "/game/other2.asi",
                "/game/other3.asi",
            ],
        );

        // Only one of OtherMod's four files overlaps -> ratio 0.25, below threshold.
        let planned = vec![
            PathBuf::from("/game/shared.dll"),
            PathBuf::from("/game/new.asi"),
        ];
        let report = analyze(&conn, &planned).unwrap();

        assert!(report.self_update.is_none());
        assert_eq!(report.foreign_conflicts.len(), 1);
        assert_eq!(report.foreign_conflicts[0].owner_name, "OtherMod");
        assert!(report.requires_explicit_override());
    }

    #[test]
    fn disabled_mods_do_not_count_as_active_conflicts() {
        let conn = crate::db::open_in_memory().unwrap();
        insert_active_mod(&conn, "WillBeDisabled", &["/game/a.asi"]);
        conn.execute("UPDATE installed_mod SET status = 'disabled'", [])
            .unwrap();

        let report = analyze(&conn, &[PathBuf::from("/game/a.asi")]).unwrap();
        assert!(report.foreign_conflicts.is_empty());
        assert!(report.self_update.is_none());
    }
}
