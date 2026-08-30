// SPDX-License-Identifier: AGPL-3.0-only

//! Computes MD5/SHA-1/SHA-256 for an arbitrary local file — checking a downloaded mod
//! against a known-good hash the author published, or (later) feeding a known-fix rule
//! library. Deliberately separate from [`crate::util::hash_file`], which is
//! SHA-256-only and used internally for `InstalledModFile.file_hash` — this exists for
//! the user pointing at an arbitrary file, not for the install pipeline's own
//! bookkeeping.

use std::io::Read;
use std::path::Path;

use md5::Md5;
use serde::Serialize;
use sha1::Sha1;
use sha2::{Digest, Sha256};

use crate::error::CoreResult;

#[derive(Debug, Clone, Serialize)]
pub struct FileHashes {
    pub md5: String,
    pub sha1: String,
    pub sha256: String,
}

/// Computes all three digests in a single pass over the file.
pub fn compute(path: &Path) -> CoreResult<FileHashes> {
    let mut file = std::fs::File::open(path)?;
    let mut md5 = Md5::new();
    let mut sha1 = Sha1::new();
    let mut sha256 = Sha256::new();
    let mut buf = [0u8; 8192];
    loop {
        let n = file.read(&mut buf)?;
        if n == 0 {
            break;
        }
        md5.update(&buf[..n]);
        sha1.update(&buf[..n]);
        sha256.update(&buf[..n]);
    }
    Ok(FileHashes {
        md5: format!("{:x}", md5.finalize()),
        sha1: format!("{:x}", sha1.finalize()),
        sha256: format!("{:x}", sha256.finalize()),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn known_content_produces_known_digests() {
        // "hello" — verified against standard md5sum/sha1sum/sha256sum output.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("hello.txt");
        std::fs::write(&path, b"hello").unwrap();

        let hashes = compute(&path).unwrap();
        assert_eq!(hashes.md5, "5d41402abc4b2a76b9719d911017c592");
        assert_eq!(hashes.sha1, "aaf4c61ddcc5e8a2dabede0f3b482cd9aea9434d");
        assert_eq!(
            hashes.sha256,
            "2cf24dba5fb0a30e26e83b2ac5b9e29e1b161e5c1fa7425e73043362938b9824"
        );
    }

    #[test]
    fn different_content_produces_different_digests() {
        let dir = tempfile::tempdir().unwrap();
        let a = dir.path().join("a.txt");
        let b = dir.path().join("b.txt");
        std::fs::write(&a, b"hello").unwrap();
        std::fs::write(&b, b"world").unwrap();

        assert_ne!(compute(&a).unwrap().sha256, compute(&b).unwrap().sha256);
    }

    #[test]
    fn missing_file_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        assert!(compute(&dir.path().join("nope.txt")).is_err());
    }
}
