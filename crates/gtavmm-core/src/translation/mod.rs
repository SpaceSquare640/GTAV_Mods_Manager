// SPDX-License-Identifier: AGPL-3.0-only

//! Translation draft generation (design doc §一-3), scoped to external config files
//! (`.ini`/`.xml`). `.NET` DLL-embedded string translation is handled separately by
//! [`crate::dll_translation`] — direct binary patching, not a draft file — after the
//! 2026-08-28 rejection of a hand-rolled Rust PE/CIL parser (on correctness-risk
//! grounds) was superseded once that parser was actually built and verified at
//! production scale (2026-08-30, see that module's doc comment).
//!
//! Uses [`crate::ai_assistant::call_provider`] — the exact same provider-calling code
//! path as [`crate::ai_assistant::diagnose`], so there is only one place in this crate
//! that ever talks to Ollama/cloud endpoints. A draft is always written to a **new
//! sibling file**, never overwriting the original — the design doc is explicit that a
//! draft needs human/community proofreading before it's a real translation, and this
//! project never silently overwrites a mod's own files outside the guarded
//! `install`/`uninstall` write path.

use std::path::{Path, PathBuf};

use rusqlite::Connection;

use crate::error::{CoreError, CoreResult};

const SUPPORTED_EXTENSIONS: &[&str] = &["ini", "xml"];

const TRANSLATION_PROMPT_PREFIX_TEMPLATE: &str = "You are translating the human-readable \
    text inside a GTA V mod's configuration file into {target_language}. Preserve every \
    key, tag, attribute name, and structural character exactly as-is — translate only the \
    human-readable text values/labels. Output only the translated file content, nothing \
    else (no commentary, no code fences).\n\nOriginal file content:\n\n";

/// Generates a translation draft for `source_path` (must be `.ini` or `.xml`) into
/// `target_language`, writing it to a new sibling file
/// (`name.ext` -> `name.<target_language>.ext`) and returning that path. The original
/// file is never touched. Requires [`crate::ai_assistant::enable`] to have been called
/// first (same provider gating as [`crate::ai_assistant::diagnose`]).
pub fn generate_draft(
    conn: &Connection,
    source_path: &Path,
    target_language: &str,
) -> CoreResult<PathBuf> {
    let ext = source_path
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.to_lowercase())
        .unwrap_or_default();
    if !SUPPORTED_EXTENSIONS.contains(&ext.as_str()) {
        return Err(CoreError::AiAssistant {
            reason: format!(
                "translation draft generation only supports .ini/.xml files, got: {}",
                source_path.display()
            ),
        });
    }

    let content = std::fs::read_to_string(source_path)?;
    let prompt = format!(
        "{}{content}",
        TRANSLATION_PROMPT_PREFIX_TEMPLATE.replace("{target_language}", target_language)
    );

    let draft = crate::ai_assistant::call_provider(conn, &prompt)?;

    let output_path = sibling_draft_path(source_path, target_language)?;
    std::fs::write(&output_path, draft)?;
    Ok(output_path)
}

fn sibling_draft_path(source_path: &Path, target_language: &str) -> CoreResult<PathBuf> {
    let stem = source_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CoreError::AiAssistant {
            reason: format!("could not determine a file name for {}", source_path.display()),
        })?;
    let ext = source_path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let file_name = format!("{stem}.{target_language}.{ext}");
    Ok(source_path.with_file_name(file_name))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn refuses_unsupported_extensions_before_touching_the_ai_provider() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mod.dll");
        std::fs::write(&path, b"not a config file").unwrap();

        let conn = crate::db::open_in_memory().unwrap();
        let err = generate_draft(&conn, &path, "zh-TW").unwrap_err();
        assert!(matches!(err, CoreError::AiAssistant { .. }));
    }

    #[test]
    fn refuses_when_ai_assistant_is_disabled() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mod.ini");
        std::fs::write(&path, b"[Settings]\nLabel=Hello").unwrap();

        let conn = crate::db::open_in_memory().unwrap();
        let err = generate_draft(&conn, &path, "zh-TW").unwrap_err();
        assert!(matches!(err, CoreError::AiAssistant { .. }));
    }

    #[test]
    fn sibling_draft_path_inserts_the_target_language_before_the_extension() {
        let path = Path::new("C:/mods/outfit.xml");
        let draft = sibling_draft_path(path, "zh-TW").unwrap();
        assert_eq!(draft, Path::new("C:/mods/outfit.zh-TW.xml"));
    }

    #[test]
    fn never_writes_to_the_original_path() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("mod.ini");
        std::fs::write(&path, b"[Settings]\nLabel=Hello").unwrap();
        let draft = sibling_draft_path(&path, "zh-TW").unwrap();
        assert_ne!(draft, path);
    }
}
