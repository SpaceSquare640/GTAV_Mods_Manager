// SPDX-License-Identifier: AGPL-3.0-only

use std::path::PathBuf;

/// Crate-wide error type. Kept in the public API (not `anyhow`) so callers — including
/// tests — can match on specific failure modes, especially around `ProtectedFiles`
/// rejections, which must never be silently swallowed or downgraded.
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("refused to write protected file: {path}")]
    ProtectedFileViolation { path: PathBuf },

    #[error("game installation not found (auto-detection failed and no manual override set)")]
    GameNotFound,

    #[error("unsupported mod format: {reason}")]
    UnsupportedFormat { reason: String },

    #[error("conflict with a different mod's file: {path} (owned by mod #{owner_mod_id})")]
    ForeignConflict { path: PathBuf, owner_mod_id: i64 },

    #[error("database error")]
    Database(#[from] rusqlite::Error),

    #[error("io error")]
    Io(#[from] std::io::Error),
}

pub type CoreResult<T> = Result<T, CoreError>;
