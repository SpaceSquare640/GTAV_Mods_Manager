// SPDX-License-Identifier: AGPL-3.0-only

//! `.oiv` (OpenIV installer package) support — two-tier boundary per the MVP spec:
//! packages whose `assembly.xml` only describes plain filesystem copies are
//! supported; anything that references writing *inside* an `.rpf` archive is reported
//! as `Unsupported`, not attempted.
//!
//! `.oiv` is really a ZIP container (readable via the `zip` crate) plus an
//! `assembly.xml` manifest.
//!
//! **Corrected 2026-08-27 against a real `.oiv` sample** (a real EUP — Emergency
//! Uniforms Pack — package for LSPDFR, inspected directly on this machine). The
//! previous version of this parser was a best-effort reconstruction of the schema
//! from public documentation, never checked against a real package, and it was
//! **wrong** in a way that was actively dangerous: it looked for elements carrying
//! both an `input`/`source` *attribute* and an `output`/`target` *attribute*, but the
//! real OpenIV assembly.xml schema (`version="2.0"`) nests everything inside
//! `<archive path="..." type="RPF7">` blocks — an archive-modification operation
//! against a specific RPF (either an existing one being edited, or a brand new one
//! being created via `createIfNotExist="True"`) — using `<add source="...">target
//! (as element text, not an attribute)</add>`, `<delete>...</delete>`, and
//! `<text path="...">...</text>` children. None of that matched the old
//! attribute-pair search, so the real (genuinely RPF-touching, genuinely dangerous)
//! sample produced **zero matched entries** and was classified `Supported` with an
//! *empty* file list — silently reporting "nothing to do" for a package that actually
//! needed real RPF surgery. That is the exact silent-mishandling failure mode this
//! module's design was supposed to prevent.
//!
//! The fix: detect `<archive>` elements directly. Every real `.oiv` sample available
//! for testing uses `<archive>` for its entire payload — which makes sense, since a
//! package that never needs to touch an RPF wouldn't need OpenIV's archive-editing
//! format in the first place (a plain folder-replacer mod would do). So **any**
//! `<archive>` element found anywhere in `assembly.xml` now classifies the whole
//! package `Unsupported`, unconditionally — there is no plain-copy fallback that
//! still has any real evidence behind it. If a real `.oiv` package genuinely
//! containing zero `<archive>` elements ever turns up, this will need revisiting; none
//! has been seen yet.

use std::io::Read;

use quick_xml::events::Event;
use quick_xml::Reader;

use crate::error::{CoreError, CoreResult};

#[derive(Debug, Clone)]
pub struct OivCopyEntry {
    /// Path inside the `.oiv` zip container.
    pub input: String,
    /// Path relative to the game root this entry should be copied to.
    pub output: String,
}

#[derive(Debug, Clone)]
pub enum OivPlan {
    /// Every instruction is a plain filesystem copy; safe to install as a normal
    /// folder-replacer-style plan.
    Supported(Vec<OivCopyEntry>),
    /// At least one instruction references archive-internal (`.rpf`) content.
    Unsupported,
}

/// Reads `assembly.xml` out of the `.oiv` zip container and classifies it.
pub fn analyze(oiv_path: &std::path::Path) -> CoreResult<OivPlan> {
    let file = std::fs::File::open(oiv_path)?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| CoreError::UnsupportedFormat {
        reason: format!(".oiv is not a valid zip container: {e}"),
    })?;

    let assembly_index = (0..archive.len())
        .find(|&i| {
            archive
                .by_index(i)
                .ok()
                .is_some_and(|f| f.name().eq_ignore_ascii_case("assembly.xml"))
        })
        .ok_or_else(|| CoreError::UnsupportedFormat {
            reason: ".oiv package has no assembly.xml".to_string(),
        })?;

    let mut assembly_xml = String::new();
    archive
        .by_index(assembly_index)
        .map_err(|e| CoreError::UnsupportedFormat {
            reason: format!("failed to read assembly.xml from .oiv: {e}"),
        })?
        .read_to_string(&mut assembly_xml)?;

    Ok(parse_assembly_xml(&assembly_xml))
}

fn parse_assembly_xml(xml: &str) -> OivPlan {
    let mut reader = Reader::from_str(xml);
    let mut entries = Vec::new();

    loop {
        match reader.read_event() {
            Ok(Event::Start(tag)) | Ok(Event::Empty(tag)) => {
                // See this module's doc comment: an <archive> block is OpenIV's
                // mechanism for editing or creating an RPF archive's contents —
                // every real sample seen uses this for its entire payload, so its
                // mere presence means "touches an RPF", unconditionally.
                if tag.local_name().as_ref().eq_ignore_ascii_case(b"archive") {
                    return OivPlan::Unsupported;
                }

                let Some((input, output)) = extract_copy_attrs(&tag) else {
                    continue;
                };
                if references_archive_internal_path(&output)
                    || references_archive_internal_path(&input)
                {
                    return OivPlan::Unsupported;
                }
                entries.push(OivCopyEntry { input, output });
            }
            Ok(Event::Eof) => break,
            Err(_) => return OivPlan::Unsupported, // malformed XML: don't guess, refuse safely
            _ => continue,
        }
    }

    OivPlan::Supported(entries)
}

/// Looks for `input`/`output` (or `source`/`target`) attributes on any element,
/// case-insensitively — OpenIV's `<content>` elements use `input`/`output`, but we
/// accept the common alternative naming too rather than assuming one exact schema.
fn extract_copy_attrs(tag: &quick_xml::events::BytesStart) -> Option<(String, String)> {
    let mut input = None;
    let mut output = None;

    for attr in tag.attributes().flatten() {
        let key = String::from_utf8_lossy(attr.key.as_ref()).to_lowercase();
        let value = attr.unescape_value().ok()?.into_owned();
        match key.as_str() {
            "input" | "source" => input = Some(value),
            "output" | "target" => output = Some(value),
            _ => {}
        }
    }

    match (input, output) {
        (Some(i), Some(o)) => Some((i, o)),
        _ => None,
    }
}

/// `true` if any path component before the final segment ends in `.rpf` — i.e. the
/// path traverses *into* an archive rather than naming a plain file on disk.
fn references_archive_internal_path(path: &str) -> bool {
    let normalized = path.replace('\\', "/");
    let mut segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
    if !segments.is_empty() {
        segments.pop(); // the final segment is the file itself, not a traversal step
    }
    segments
        .iter()
        .any(|segment| segment.to_lowercase().ends_with(".rpf"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn plain_copy_instructions_are_supported() {
        let xml = r#"<Assembly>
            <ContentFiles>
                <Content input="data/foo.meta" output="common/data/foo.meta" />
                <Content input="data/bar.meta" output="common/data/bar.meta" />
            </ContentFiles>
        </Assembly>"#;
        match parse_assembly_xml(xml) {
            OivPlan::Supported(entries) => assert_eq!(entries.len(), 2),
            OivPlan::Unsupported => panic!("expected Supported"),
        }
    }

    #[test]
    fn archive_internal_target_is_unsupported() {
        let xml = r#"<Assembly>
            <ContentFiles>
                <Content input="data/foo.ytd" output="update/x64/dlcpacks/mymod/dlc.rpf/x64/textures/foo.ytd" />
            </ContentFiles>
        </Assembly>"#;
        assert!(matches!(parse_assembly_xml(xml), OivPlan::Unsupported));
    }

    /// Regression fixture: the real (trimmed) `assembly.xml` from a real EUP (LSPDFR
    /// uniforms pack) `.oiv`, inspected directly on 2026-08-27. This is the sample
    /// that caught the real bug this module's doc comment describes — the old
    /// attribute-pair-based parser found zero matches against this and classified it
    /// `Supported` with an empty file list, when it should have been `Unsupported`
    /// (it deletes/inserts content inside `update\update.rpf` and creates a new
    /// nested `dlc.rpf`).
    const REAL_EUP_ASSEMBLY_XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<package version="2.0" id="{64DE0490-6A4B-468E-B963-4B7DB6223FAA}" target="Five">
    <content>
        <archive path="update\update.rpf" createIfNotExist="False" type="RPF7">
            <delete>dlc_patch\mpimportexport\content.xml</delete>
            <text path="common\data\dlclist.xml" createIfNotExist="False">
                <delete condition="Mask">*\eup\*</delete>
                <insert where="Before" line="*&lt;/Paths&gt;*" condition="Mask">		&lt;Item&gt;dlcpacks:/eup/&lt;/Item&gt;</insert>
            </text>
        </archive>
        <archive path="update\x64\dlcpacks\eup\dlc.rpf" createIfNotExist="True" type="RPF7">
            <add source="content.xml">content.xml</add>
            <add source="x64\eup_componentpeds.rpf">x64\eup_componentpeds.rpf</add>
        </archive>
    </content>
</package>"#;

    #[test]
    fn real_eup_oiv_with_archive_edits_is_unsupported_not_silently_empty() {
        assert!(matches!(
            parse_assembly_xml(REAL_EUP_ASSEMBLY_XML),
            OivPlan::Unsupported
        ));
    }

    #[test]
    fn malformed_xml_is_conservatively_unsupported() {
        assert!(matches!(
            parse_assembly_xml("<not><valid"),
            OivPlan::Unsupported
        ));
    }

    #[test]
    fn detects_rpf_reference_regardless_of_slash_style() {
        assert!(references_archive_internal_path(
            r"update\x64\dlcpacks\mymod\dlc.rpf\x64\data\foo.meta"
        ));
        assert!(!references_archive_internal_path("common/data/foo.meta"));
    }
}
