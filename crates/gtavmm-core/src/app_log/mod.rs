// SPDX-License-Identifier: AGPL-3.0-only

//! A plain local diagnostic log file — separate from [`crate::history`] (which
//! records mod install/uninstall/enable/disable *events* for the user to browse) and
//! from [`crate::crash_report`] (which drafts a de-identified GitHub Issue, never
//! writes anything to disk). This is the thing actually missing before now: nowhere
//! in the app wrote its own internal errors/warnings/diagnostics to a file the user
//! could attach when reporting a problem.
//!
//! Deliberately minimal — appends plain lines to one file under the app-data
//! directory, no external logging crate (`log`/`tracing`) pulled in for what's a
//! handful of call sites so far. One size-based rotation: if the file has grown past
//! [`MAX_LOG_BYTES`], it's renamed to `app.log.1` (overwriting any previous one) before
//! the new line is appended, so this can never grow unbounded.
//!
//! On top of that size-based rotation, the log is also cleared on a 3-day timer
//! ([`AUTO_CLEANUP_INTERVAL`]) — [`maybe_auto_cleanup`] is checked from [`log`] itself
//! so this self-enforces without a separate startup hook, plus from the diagnostic log
//! viewer so opening that page also catches a cleanup that's overdue even if nothing
//! happened to log an error in the meantime. The last-cleanup timestamp (whether from
//! the timer or a manual [`clear`]) lives in a sibling marker file, not the database —
//! this module intentionally never needs a `Connection` to do its job. The very first
//! time this runs (no marker yet, e.g. right after upgrading to a build with this
//! feature), it only records a baseline timestamp instead of immediately wiping
//! whatever log content already existed — silently destroying pre-existing diagnostic
//! history the moment this feature ships would be a surprising, not "silently
//! destructive by default" outcome per this project's own operating rules.

use std::fmt;
use std::io::Write;
use std::path::PathBuf;

use chrono::{DateTime, Utc};

use crate::error::CoreResult;

const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;
const AUTO_CLEANUP_INTERVAL: chrono::Duration = chrono::Duration::days(3);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LogLevel {
    Info,
    Warn,
    Error,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            LogLevel::Info => "INFO",
            LogLevel::Warn => "WARN",
            LogLevel::Error => "ERROR",
        })
    }
}

/// Resolves the log file location under the same app-data directory as the database
/// (`<app-data>/logs/app.log`), independent of any particular database being open.
pub fn log_path() -> Option<PathBuf> {
    directories::ProjectDirs::from("", "SpaceSquare", "GTAVModsManager")
        .map(|dirs| dirs.data_dir().join("logs").join("app.log"))
}

/// Appends one line (`TIMESTAMP [LEVEL] message`) to the log file, rotating first if
/// it's grown past [`MAX_LOG_BYTES`]. Silently does nothing if the app-data directory
/// can't be resolved (matches [`log_path`] returning `None`) — logging a diagnostic
/// message should never itself be a fatal error for the caller.
pub fn log(level: LogLevel, message: &str) -> CoreResult<()> {
    maybe_auto_cleanup()?;
    let Some(path) = log_path() else {
        return Ok(());
    };
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    if let Ok(meta) = std::fs::metadata(&path) {
        if meta.len() > MAX_LOG_BYTES {
            let rotated = path.with_extension("log.1");
            let _ = std::fs::rename(&path, rotated);
        }
    }
    let timestamp = chrono::Utc::now().to_rfc3339();
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    writeln!(file, "{timestamp} [{level}] {message}")?;
    Ok(())
}

/// Sibling marker file recording when the log was last cleared (by the 3-day timer or
/// a manual [`clear`]) — deliberately not in the database so this module never needs a
/// `Connection`.
fn last_cleanup_marker_path() -> Option<PathBuf> {
    log_path().map(|p| p.with_file_name(".last_cleanup"))
}

/// Returns when the log was last cleared, or `None` if it never has been (including:
/// this feature was never active before now).
pub fn last_cleanup() -> Option<DateTime<Utc>> {
    let path = last_cleanup_marker_path()?;
    let content = std::fs::read_to_string(path).ok()?;
    DateTime::parse_from_rfc3339(content.trim())
        .ok()
        .map(|dt| dt.with_timezone(&Utc))
}

fn record_cleanup_timestamp() -> CoreResult<()> {
    let Some(marker) = last_cleanup_marker_path() else {
        return Ok(());
    };
    if let Some(parent) = marker.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(marker, Utc::now().to_rfc3339())?;
    Ok(())
}

/// Manually clears the current log contents (both `app.log` and any rotated
/// `app.log.1`) and resets the 3-day auto-cleanup timer, so the next automatic
/// cleanup is 3 days from now rather than stacking on top of a timer the user just
/// satisfied by hand. Used by the "Clear Log Now" UI action and by
/// [`maybe_auto_cleanup`] when the timer is actually due.
pub fn clear() -> CoreResult<()> {
    if let Some(path) = log_path() {
        let _ = std::fs::remove_file(&path);
        let _ = std::fs::remove_file(path.with_extension("log.1"));
    }
    record_cleanup_timestamp()
}

/// Pure decision logic behind [`maybe_auto_cleanup`], split out so it's testable
/// without touching the real on-disk marker file.
fn is_cleanup_due(last: Option<DateTime<Utc>>, now: DateTime<Utc>) -> bool {
    match last {
        None => false,
        Some(last) => now.signed_duration_since(last) >= AUTO_CLEANUP_INTERVAL,
    }
}

/// Runs the 3-day forced auto-cleanup if it's due, and returns whether it actually ran.
/// The very first time this is called with no marker on disk yet, it only records a
/// baseline timestamp rather than clearing — see the module doc comment for why.
pub fn maybe_auto_cleanup() -> CoreResult<bool> {
    let last = last_cleanup();
    if last.is_none() {
        record_cleanup_timestamp()?;
        return Ok(false);
    }
    if is_cleanup_due(last, Utc::now()) {
        clear()?;
        return Ok(true);
    }
    Ok(false)
}

pub fn info(message: &str) -> CoreResult<()> {
    log(LogLevel::Info, message)
}
pub fn warn(message: &str) -> CoreResult<()> {
    log(LogLevel::Warn, message)
}
pub fn error(message: &str) -> CoreResult<()> {
    log(LogLevel::Error, message)
}

/// Returns the last `max_lines` lines of the current log file (oldest of that window
/// first), or an empty list if there's no log file yet. Reads the whole file into
/// memory — fine at the size this is capped to ([`MAX_LOG_BYTES`]).
pub fn read_recent(max_lines: usize) -> CoreResult<Vec<String>> {
    maybe_auto_cleanup()?;
    let Some(path) = log_path() else {
        return Ok(Vec::new());
    };
    let Ok(content) = std::fs::read_to_string(&path) else {
        return Ok(Vec::new());
    };
    let lines: Vec<String> = content.lines().map(str::to_string).collect();
    let start = lines.len().saturating_sub(max_lines);
    Ok(lines[start..].to_vec())
}

#[cfg(test)]
mod tests {
    use super::*;

    // These tests exercise the pure line-formatting/windowing logic without touching
    // the real app-data directory (log_path() is a fixed, real OS path — writing to it
    // from a test would pollute the developer's actual machine, so the file-based
    // functions themselves aren't exercised here; that's covered by manual/App-level
    // verification instead).

    #[test]
    fn log_level_display_matches_expected_short_codes() {
        assert_eq!(LogLevel::Info.to_string(), "INFO");
        assert_eq!(LogLevel::Warn.to_string(), "WARN");
        assert_eq!(LogLevel::Error.to_string(), "ERROR");
    }

    #[test]
    fn cleanup_is_not_due_with_no_prior_record() {
        assert!(!is_cleanup_due(None, Utc::now()));
    }

    #[test]
    fn cleanup_is_not_due_before_three_days_have_passed() {
        let now = Utc::now();
        let last = now - chrono::Duration::days(2);
        assert!(!is_cleanup_due(Some(last), now));
    }

    #[test]
    fn cleanup_is_due_at_exactly_three_days() {
        let now = Utc::now();
        let last = now - chrono::Duration::days(3);
        assert!(is_cleanup_due(Some(last), now));
    }

    #[test]
    fn cleanup_is_due_after_three_days() {
        let now = Utc::now();
        let last = now - chrono::Duration::days(10);
        assert!(is_cleanup_due(Some(last), now));
    }

    #[test]
    fn read_recent_windows_from_the_end_of_a_line_list() {
        let lines: Vec<String> = (1..=10).map(|i| format!("line {i}")).collect();
        let content = lines.join("\n");
        let all_lines: Vec<String> = content.lines().map(str::to_string).collect();
        let start = all_lines.len().saturating_sub(3);
        let windowed = &all_lines[start..];
        assert_eq!(windowed, &["line 8", "line 9", "line 10"]);
    }
}
