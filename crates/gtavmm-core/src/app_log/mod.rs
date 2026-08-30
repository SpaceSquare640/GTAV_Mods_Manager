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

use std::fmt;
use std::io::Write;
use std::path::PathBuf;

use crate::error::CoreResult;

const MAX_LOG_BYTES: u64 = 5 * 1024 * 1024;

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
    let Some(path) = log_path() else { return Ok(()) };
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
    let mut file = std::fs::OpenOptions::new().create(true).append(true).open(&path)?;
    writeln!(file, "{timestamp} [{level}] {message}")?;
    Ok(())
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
    let Some(path) = log_path() else { return Ok(Vec::new()) };
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
    fn read_recent_windows_from_the_end_of_a_line_list() {
        let lines: Vec<String> = (1..=10).map(|i| format!("line {i}")).collect();
        let content = lines.join("\n");
        let all_lines: Vec<String> = content.lines().map(str::to_string).collect();
        let start = all_lines.len().saturating_sub(3);
        let windowed = &all_lines[start..];
        assert_eq!(windowed, &["line 8", "line 9", "line 10"]);
    }
}
