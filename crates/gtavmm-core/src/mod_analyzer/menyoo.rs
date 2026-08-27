// SPDX-License-Identifier: AGPL-3.0-only

//! Menyoo `.xml` target-subfolder detection. Menyoo stores different content types
//! (outfits, spooner placements, vehicles, weapon loadouts) as XML files that must
//! land in specific `menyooStuff\` subfolders. We detect by root element name;
//! anything unrecognized still lands under `menyooStuff\` (not rejected outright) —
//! consistent with the project's "don't chase 100% coverage, degrade gracefully"
//! principle, since a human can still move a misclassified file one level down.

use quick_xml::events::Event;
use quick_xml::Reader;

pub const MENYOO_ROOT_FOLDER: &str = "menyooStuff";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MenyooCategory {
    Outfit,
    Spooner,
    Vehicle,
    WeaponsLoadout,
    Unknown,
}

impl MenyooCategory {
    /// The subfolder under `menyooStuff\` this category deploys to, or `None` for
    /// `Unknown`, which deploys to `menyooStuff\` itself.
    pub fn subfolder(self) -> Option<&'static str> {
        match self {
            MenyooCategory::Outfit => Some("Outfits"),
            MenyooCategory::Spooner => Some("Spooner"),
            MenyooCategory::Vehicle => Some("Vehicles"),
            MenyooCategory::WeaponsLoadout => Some("Weapons"),
            MenyooCategory::Unknown => None,
        }
    }
}

/// Reads just far enough into the XML to find the root element's name, then
/// classifies. Never fails — a malformed/unreadable XML is classified `Unknown`
/// rather than rejecting the file, per the graceful-degradation principle above.
pub fn detect_category(xml_path: &std::path::Path) -> MenyooCategory {
    let Ok(contents) = std::fs::read_to_string(xml_path) else {
        return MenyooCategory::Unknown;
    };

    let mut reader = Reader::from_str(&contents);
    loop {
        match reader.read_event() {
            Ok(Event::Start(tag)) | Ok(Event::Empty(tag)) => {
                let name = tag.local_name();
                let name = String::from_utf8_lossy(name.as_ref()).to_lowercase();
                return classify_root_name(&name);
            }
            Ok(Event::Eof) => return MenyooCategory::Unknown,
            Err(_) => return MenyooCategory::Unknown,
            _ => continue,
        }
    }
}

fn classify_root_name(name: &str) -> MenyooCategory {
    if name.contains("outfit") {
        MenyooCategory::Outfit
    } else if name.contains("spooner") || name.contains("placement") || name.contains("map") {
        MenyooCategory::Spooner
    } else if name.contains("vehicle") {
        MenyooCategory::Vehicle
    } else if name.contains("weapon") || name.contains("loadout") {
        MenyooCategory::WeaponsLoadout
    } else {
        MenyooCategory::Unknown
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write_xml(dir: &std::path::Path, name: &str, root_tag: &str) -> std::path::PathBuf {
        let path = dir.join(name);
        std::fs::write(&path, format!("<{root_tag}></{root_tag}>")).unwrap();
        path
    }

    #[test]
    fn detects_outfit_by_root_tag() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_xml(dir.path(), "a.xml", "Outfit");
        assert_eq!(detect_category(&path), MenyooCategory::Outfit);
    }

    #[test]
    fn detects_spooner_by_root_tag() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_xml(dir.path(), "a.xml", "SpoonerPlacements");
        assert_eq!(detect_category(&path), MenyooCategory::Spooner);
    }

    #[test]
    fn falls_back_to_unknown_for_unrecognized_or_unreadable() {
        let dir = tempfile::tempdir().unwrap();
        let path = write_xml(dir.path(), "a.xml", "SomethingElseEntirely");
        assert_eq!(detect_category(&path), MenyooCategory::Unknown);
        assert_eq!(MenyooCategory::Unknown.subfolder(), None);

        let missing = dir.path().join("does_not_exist.xml");
        assert_eq!(detect_category(&missing), MenyooCategory::Unknown);
    }
}
