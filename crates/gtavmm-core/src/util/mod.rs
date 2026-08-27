// SPDX-License-Identifier: AGPL-3.0-only

//! Small shared helpers: hashing, path utilities.

use std::io::Read;
use std::path::Path;

use sha2::{Digest, Sha256};

use crate::error::CoreResult;

#[cfg(test)]
mod move_file_tests {
    use super::*;

    #[test]
    fn moves_file_and_removes_source() {
        let dir = tempfile::tempdir().unwrap();
        let from = dir.path().join("a.txt");
        let to = dir.path().join("nested/b.txt");
        std::fs::write(&from, b"payload").unwrap();

        move_file(&from, &to).unwrap();

        assert!(!from.exists());
        assert_eq!(std::fs::read(&to).unwrap(), b"payload");
    }
}

/// Moves a file from `from` to `to`, working across drives/filesystems (unlike
/// `std::fs::rename`, which fails on Windows — and on Unix when crossing mount
/// points — with an OS-level "can't move across devices" error). Backups, staging
/// areas, and recycle-bin snapshots routinely live on a different drive than the
/// game install, so every cross-tree move in this crate must go through this
/// function rather than `std::fs::rename` directly.
pub fn move_file(from: &Path, to: &Path) -> CoreResult<()> {
    if let Some(parent) = to.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::copy(from, to)?;
    std::fs::remove_file(from)?;
    Ok(())
}

/// SHA-256 hex digest of a file's contents, used for `InstalledModFile.file_hash` and
/// the install pipeline's Verifying step.
pub fn hash_file(path: &Path) -> CoreResult<String> {
    let mut file = std::fs::File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        hasher.update(&buf[..n]);
    }
    Ok(format!("{:x}", hasher.finalize()))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hashes_are_stable_and_content_sensitive() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        std::fs::write(&a, b"hello").unwrap();
        std::fs::write(&b, b"world").unwrap();

        let hash_a1 = hash_file(&a).unwrap();
        let hash_a2 = hash_file(&a).unwrap();
        let hash_b = hash_file(&b).unwrap();

        assert_eq!(hash_a1, hash_a2);
        assert_ne!(hash_a1, hash_b);
    }
}
