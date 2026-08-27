// SPDX-License-Identifier: AGPL-3.0-only

//! `.oiv` (OpenIV installer package) support — two-tier boundary per the MVP spec:
//! packages whose `assembly.xml` only describes plain filesystem copies are
//! supported; anything that references writing *inside* an `.rpf` archive is reported
//! as `Unsupported`, not attempted.
//!
//! `.oiv` is really a ZIP container (readable via the `zip` crate) plus an
//! `assembly.xml` manifest. **The exact `assembly.xml` schema used here is a
//! best-effort reconstruction from public documentation, not verified against real
//! `.oiv` samples** — per the project's own risk note, this needs hands-on validation
//! with real mod packages before being trusted for anything beyond the simplest
//! cases. The detection is intentionally conservative: anything that looks even
//! slightly like it touches an `.rpf` internally is classified `Unsupported` rather
//! than risking a wrong "supported" classification.

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
