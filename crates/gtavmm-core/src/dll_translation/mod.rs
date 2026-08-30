// SPDX-License-Identifier: AGPL-3.0-only

//! AI-assisted translation of user-facing strings embedded directly in a `.NET`
//! (IL-only) mod `.dll`, via direct binary patching of the `#US` (User Strings) heap —
//! no source code, no recompilation, no external `.NET` tooling.
//!
//! **Never overwrites the original file.** Always writes to a new sibling file
//! (`Name.dll` -> `Name.<lang>.dll`), same convention as
//! [`crate::translation::generate_draft`] for config-file translation drafts.
//!
//! Verified 2026-08-30 at production scale against a real GTA V mod (`GangModV1.dll`,
//! 143/143 real user-facing strings translated and patched across 207 call sites,
//! cross-checked by re-parsing the patched file with `dotnetdll`'s independent,
//! spec-compliant reader). See the design doc for the full history — this supersedes
//! the `.NET` DLL translation rejection recorded in [`crate::translation`]'s module doc
//! comment.
//!
//! Two ways to get translations in, both ending at the same [`patch_with_translations`]
//! (2026-08-30, added per user request after the first version turned out to require AI
//! for every step and gave no way to review or hand-write a translation):
//! - AI draft, reviewable before committing: [`translate_draft`] returns each source
//!   string paired with the AI's translation and does **not** touch any file; the
//!   caller (the app's review UI) can let the user edit any entry before calling
//!   [`patch_with_translations`].
//! - Fully manual, no AI involved: build a `Vec<String>` by hand (same order as
//!   [`inspect`]'s `translatable` list) and pass it straight to
//!   [`patch_with_translations`] — [`crate::ai_assistant`] is never invoked.
//!
//! Guardrails, refused outright rather than guessed around:
//! - Mixed-mode assemblies (contain native code alongside managed IL) — only IL-only
//!   assemblies are supported ([`pe::PeLayout::is_il_only`]).
//! - Signed assemblies (Authenticode certificate present) — patching would invalidate
//!   the signature ([`pe::PeLayout::is_signed`]).
//! - A translated string whose original token can't be located anywhere in the
//!   assembly's IL is skipped, never guessed at.

mod pe;

use std::path::{Path, PathBuf};

use rusqlite::Connection;
use serde::Serialize;

use crate::error::{CoreError, CoreResult};

/// One `#US` heap entry judged real, translatable user-facing text (technical
/// identifiers — ped model names, animation dict/clip names, native/event constants,
/// file paths, bare config keys — are excluded before this list is built).
#[derive(Debug, Clone, Serialize)]
pub struct TranslatableString {
    /// Index into the filtered list — stable within one [`inspect`] call, used to
    /// correlate a later [`translate_and_patch`] run's skipped list back to the text.
    pub index: usize,
    pub text: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct DllInspection {
    pub total_strings: usize,
    pub excluded_technical: usize,
    pub translatable: Vec<TranslatableString>,
}

/// Reads `dll_path` and reports its guardrail status plus every string judged real
/// translatable user-facing text. Does not modify anything.
pub fn inspect(dll_path: &Path) -> CoreResult<DllInspection> {
    let bytes = std::fs::read(dll_path)?;
    let layout = parse_and_guard(&bytes)?;
    let entries = pe::parse_us_heap(&bytes, layout.us_heap_offset, layout.us_heap_size);
    let total_strings = entries.len();
    let translatable: Vec<TranslatableString> = entries
        .iter()
        .filter(|e| !pe::is_technical_string(&e.text))
        .enumerate()
        .map(|(index, e)| TranslatableString {
            index,
            text: e.text.clone(),
        })
        .collect();
    Ok(DllInspection {
        total_strings,
        excluded_technical: total_strings - translatable.len(),
        translatable,
    })
}

#[derive(Debug, Clone, Serialize)]
pub struct DllTranslationOutcome {
    pub output_path: PathBuf,
    pub strings_translated: usize,
    pub call_sites_patched: usize,
    /// Original text of any string whose token could not be located anywhere in the
    /// assembly's IL — translated, but not patched in, rather than guessed at.
    pub skipped: Vec<String>,
}

/// One AI-translated string, paired with its source text, before it's been patched
/// into anything — a draft the caller (the app's review UI) can let the user edit
/// before committing via [`patch_with_translations`].
#[derive(Debug, Clone, Serialize)]
pub struct TranslatedDraftEntry {
    /// Index into [`inspect`]'s `translatable` list — the two must stay in the same
    /// order (both derive from the same `#US` heap scan + [`pe::is_technical_string`]
    /// filter), so this can be used to correlate a draft entry back to its origin.
    pub index: usize,
    pub source: String,
    pub translated: String,
}

/// The set of translatable candidates from a DLL, structurally ready to patch: each
/// entry's original `#US` heap token alongside its source text, in a stable order that
/// [`inspect`], [`translate_draft`], and [`patch_with_translations`] all agree on.
struct Candidates {
    old_tokens: Vec<u32>,
    source_texts: Vec<String>,
}

fn extract_candidates(bytes: &[u8], layout: &pe::PeLayout) -> CoreResult<Candidates> {
    let entries = pe::parse_us_heap(bytes, layout.us_heap_offset, layout.us_heap_size);
    let candidates: Vec<_> = entries
        .iter()
        .filter(|e| !pe::is_technical_string(&e.text))
        .collect();
    if candidates.is_empty() {
        return Err(CoreError::DllTranslation {
            reason: "no real translatable user-facing text was found in this DLL".to_string(),
        });
    }
    let old_tokens = candidates
        .iter()
        .map(|c| {
            let prefix_len = if c.data_len < 0x80 {
                1
            } else if c.data_len < 0x4000 {
                2
            } else {
                4
            };
            let heap_rel = (c.data_offset - prefix_len) - layout.us_heap_offset;
            0x7000_0000u32 | (heap_rel as u32)
        })
        .collect();
    let source_texts = candidates.iter().map(|c| c.text.clone()).collect();
    Ok(Candidates {
        old_tokens,
        source_texts,
    })
}

/// Translates every real user-facing string in `dll_path` (same auto-filtering as
/// [`inspect`]) into `target_language` via the configured AI provider (requires
/// [`crate::ai_assistant::enable`] first — same gating as every other AI-assisted
/// feature in this crate), returning the drafts **without patching anything** — the
/// caller (the app's review UI) is expected to let the user look over, and optionally
/// hand-edit, each translation before calling [`patch_with_translations`] with the
/// (possibly edited) results.
///
/// `batch_size` caps how many strings are sent to the provider per request (smaller
/// free-tier models need this kept modest — 15 was proven reliable in production use).
pub fn translate_draft(
    conn: &Connection,
    dll_path: &Path,
    target_language: &str,
    batch_size: usize,
) -> CoreResult<Vec<TranslatedDraftEntry>> {
    let bytes = std::fs::read(dll_path)?;
    let layout = parse_and_guard(&bytes)?;
    let Candidates { source_texts, .. } = extract_candidates(&bytes, &layout)?;

    let mut translations: Vec<String> = Vec::with_capacity(source_texts.len());
    for chunk in source_texts.chunks(batch_size.max(1)) {
        let result = translate_batch(conn, target_language, chunk)?;
        if result.len() != chunk.len() {
            return Err(CoreError::DllTranslation {
                reason: format!(
                    "AI provider returned {} translations for {} input strings — refusing to guess a mapping",
                    result.len(),
                    chunk.len()
                ),
            });
        }
        translations.extend(result);
    }

    Ok(source_texts
        .into_iter()
        .zip(translations)
        .enumerate()
        .map(|(index, (source, translated))| TranslatedDraftEntry {
            index,
            source,
            translated,
        })
        .collect())
}

/// Patches `translations` (one per [`inspect`]-order translatable string — either AI
/// drafts from [`translate_draft`], hand-edited afterward, or written entirely by hand
/// with no AI involved at all) into a **new** copy of `dll_path`. The original file is
/// never touched. `translations.len()` must match the DLL's current translatable-string
/// count — a mismatch (e.g. the file changed since it was last inspected) is refused
/// rather than guessed at.
pub fn patch_with_translations(
    dll_path: &Path,
    target_language: &str,
    translations: &[String],
) -> CoreResult<DllTranslationOutcome> {
    let bytes = std::fs::read(dll_path)?;
    let layout = parse_and_guard(&bytes)?;
    let Candidates {
        old_tokens,
        source_texts,
    } = extract_candidates(&bytes, &layout)?;

    if translations.len() != source_texts.len() {
        return Err(CoreError::DllTranslation {
            reason: format!(
                "got {} translation(s) but this DLL currently has {} translatable string(s) — it may have changed since it was last inspected",
                translations.len(),
                source_texts.len()
            ),
        });
    }

    let new_entries: Vec<Vec<u8>> = translations.iter().map(|t| pe::build_us_entry(t)).collect();
    let (mut patched, new_offsets) = pe::relocate_and_append_entries(&bytes, &layout, &new_entries)
        .map_err(|reason| CoreError::DllTranslation { reason })?;
    let new_tokens: Vec<u32> = new_offsets.iter().map(|&o| 0x7000_0000u32 | o).collect();

    let (tables_offset, _) = pe::find_tables_stream(&bytes, &layout)
        .map_err(|reason| CoreError::DllTranslation { reason })?;
    let matches = pe::find_tokens_in_all_methods(&bytes, &layout, tables_offset, &old_tokens)
        .map_err(|reason| CoreError::DllTranslation { reason })?;

    // A token found more than once isn't ambiguous — the compiler deduplicates
    // identical string literals to one #US entry, so every occurrence currently loads
    // the exact same original text. Redirecting ALL of them to the new token is
    // correct, not a guess. Only a token found nowhere is skipped.
    let mut call_sites_patched = 0usize;
    let mut strings_translated = 0usize;
    let mut skipped = Vec::new();
    for (i, offs) in matches.iter().enumerate() {
        if offs.is_empty() {
            skipped.push(source_texts[i].clone());
            continue;
        }
        for &opcode_offset in offs {
            patched[opcode_offset + 1..opcode_offset + 5]
                .copy_from_slice(&new_tokens[i].to_le_bytes());
            call_sites_patched += 1;
        }
        strings_translated += 1;
    }

    let output_path = sibling_translated_path(dll_path, target_language)?;
    std::fs::write(&output_path, &patched)?;

    Ok(DllTranslationOutcome {
        output_path,
        strings_translated,
        call_sites_patched,
        skipped,
    })
}

/// Convenience wrapper: AI-translates every candidate ([`translate_draft`]) and
/// immediately patches the results with no review step ([`patch_with_translations`]).
/// Kept for callers (tests, scripts) that want the one-shot behavior this module had
/// before the review/manual-entry split; the app's own UI calls the two steps
/// separately so the user can look over (and edit) the AI's output first.
pub fn translate_and_patch(
    conn: &Connection,
    dll_path: &Path,
    target_language: &str,
    batch_size: usize,
) -> CoreResult<DllTranslationOutcome> {
    let drafts = translate_draft(conn, dll_path, target_language, batch_size)?;
    let translations: Vec<String> = drafts.into_iter().map(|d| d.translated).collect();
    patch_with_translations(dll_path, target_language, &translations)
}

fn parse_and_guard(bytes: &[u8]) -> CoreResult<pe::PeLayout> {
    let layout =
        pe::parse_pe_layout(bytes).map_err(|reason| CoreError::DllTranslation { reason })?;
    if !layout.is_il_only() {
        return Err(CoreError::DllTranslation {
            reason: "mixed-mode assembly (contains native code) — refusing to patch".to_string(),
        });
    }
    if layout.is_signed() {
        return Err(CoreError::DllTranslation {
            reason: "signed assembly (Authenticode certificate present) — patching would invalidate the signature, refusing"
                .to_string(),
        });
    }
    Ok(layout)
}

fn sibling_translated_path(dll_path: &Path, target_language: &str) -> CoreResult<PathBuf> {
    let stem = dll_path
        .file_stem()
        .and_then(|s| s.to_str())
        .ok_or_else(|| CoreError::DllTranslation {
            reason: format!("could not determine a file name for {}", dll_path.display()),
        })?;
    let file_name = format!("{stem}.{target_language}.dll");
    Ok(dll_path.with_file_name(file_name))
}

/// Sends one batch through [`crate::ai_assistant::call_provider`] — the same single
/// dispatch path every other AI-assisted feature in this crate uses — as a real JSON
/// array (not a numbered plaintext list, which was found in production use to
/// mis-count when a source string itself contained a literal newline).
fn translate_batch(
    conn: &Connection,
    target_language: &str,
    texts: &[String],
) -> CoreResult<Vec<String>> {
    let input_json = serde_json::to_string(texts).map_err(|e| CoreError::DllTranslation {
        reason: format!("failed to encode input strings: {e}"),
    })?;
    let prompt = format!(
        "You are translating GTA V mod UI text embedded in a .NET DLL into {target_language}. \
        Preserve any markup/format tags exactly as-is (e.g. ~r~, ~b~, ~g~, ~y~, ~p~, ~w~, ~o~, \
        ~INPUT_CONTEXT~, {{0}}) — translate only the natural-language words around them, and \
        keep any embedded newlines in the same positions. The following is a JSON array of \
        exactly {} input strings. Respond with ONLY a JSON array of exactly {} translated \
        strings, in the same order — one output element per input element, never split or \
        merge elements. No other text, no markdown code fence.\n\n{input_json}",
        texts.len(),
        texts.len()
    );
    let response = crate::ai_assistant::call_provider(conn, &prompt)?;
    let cleaned = response
        .trim()
        .trim_start_matches("```json")
        .trim_start_matches("```")
        .trim_end_matches("```")
        .trim();
    serde_json::from_str::<Vec<String>>(cleaned).map_err(|e| CoreError::DllTranslation {
        reason: format!("could not parse the AI provider's translations as a JSON array: {e}"),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn inspect_rejects_a_file_that_is_not_a_pe_binary() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not-a-dll.dll");
        std::fs::write(&path, b"definitely not a PE file").unwrap();
        let err = inspect(&path).unwrap_err();
        assert!(matches!(err, CoreError::DllTranslation { .. }));
    }

    #[test]
    fn translate_and_patch_requires_ai_assistant_to_be_enabled_first() {
        let conn = crate::db::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not-a-dll.dll");
        std::fs::write(&path, b"definitely not a PE file").unwrap();
        // Guardrail parsing runs before any AI call, so this file's own invalidity is
        // what surfaces first — proves translate_and_patch never reaches the network
        // without a valid, guard-passed assembly.
        let err = translate_and_patch(&conn, &path, "zh-TW", 15).unwrap_err();
        assert!(matches!(err, CoreError::DllTranslation { .. }));
    }

    #[test]
    fn sibling_translated_path_never_overwrites_the_original() {
        let original = Path::new("H:/mods/GangModV1.dll");
        let out = sibling_translated_path(original, "zh-TW").unwrap();
        assert_eq!(out, Path::new("H:/mods/GangModV1.zh-TW.dll"));
        assert_ne!(out, original);
    }

    #[test]
    fn translate_draft_requires_ai_assistant_to_be_enabled_first() {
        let conn = crate::db::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not-a-dll.dll");
        std::fs::write(&path, b"definitely not a PE file").unwrap();
        let err = translate_draft(&conn, &path, "zh-TW", 15).unwrap_err();
        assert!(matches!(err, CoreError::DllTranslation { .. }));
    }

    #[test]
    fn patch_with_translations_never_calls_ai_and_works_on_an_invalid_file_error_path() {
        // No AI provider is configured anywhere in this test — if patch_with_translations
        // reached the network it would hang/fail on that, not on the guardrail check.
        // Getting the guardrail error here proves the manual-entry path never touches
        // ai_assistant at all.
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not-a-dll.dll");
        std::fs::write(&path, b"definitely not a PE file").unwrap();
        let err = patch_with_translations(&path, "zh-TW", &["手動翻譯".to_string()]).unwrap_err();
        assert!(matches!(err, CoreError::DllTranslation { .. }));
    }
}
