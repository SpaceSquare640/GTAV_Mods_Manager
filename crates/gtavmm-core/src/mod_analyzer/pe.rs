// SPDX-License-Identifier: AGPL-3.0-only

//! Minimal PE (Portable Executable) header inspection — just enough to distinguish a
//! *managed* .NET assembly (ScriptHookVDotNet script) from a *native* DLL (a
//! ScriptHookV plugin), by checking whether the COM Descriptor data directory (the
//! CLR header) is present. Deliberately not a full PE parser / no new dependency —
//! this one fact is all `mod_analyzer` needs.

use std::io::{Read, Seek, SeekFrom};

/// `true` if `path` is a managed (.NET) PE image; `false` for native or anything that
/// doesn't parse as a well-formed PE (native is the safer default on parse failure,
/// since `.asi`/native-plugin handling is simpler and more permissive than the
/// `scripts\` managed-assembly path).
pub fn is_managed_assembly(path: &std::path::Path) -> bool {
    try_is_managed_assembly(path).unwrap_or(false)
}

fn try_is_managed_assembly(path: &std::path::Path) -> std::io::Result<bool> {
    let mut file = std::fs::File::open(path)?;

    // DOS header: "MZ" magic, e_lfanew (offset to PE header) at 0x3C.
    let mut dos_header = [0u8; 0x40];
    file.read_exact(&mut dos_header)?;
    if &dos_header[0..2] != b"MZ" {
        return Ok(false);
    }
    let pe_offset = u32::from_le_bytes(dos_header[0x3C..0x40].try_into().unwrap()) as u64;

    file.seek(SeekFrom::Start(pe_offset))?;
    let mut pe_signature = [0u8; 4];
    file.read_exact(&mut pe_signature)?;
    if &pe_signature != b"PE\0\0" {
        return Ok(false);
    }

    // COFF File Header is 20 bytes; we only need to skip past it to the Optional Header.
    file.seek(SeekFrom::Current(20))?;

    let mut magic = [0u8; 2];
    file.read_exact(&mut magic)?;
    let magic = u16::from_le_bytes(magic);
    // PE32 = 0x10B, PE32+ (64-bit) = 0x20B. The offset from the start of the Optional
    // Header to the Data Directories array differs between the two.
    let data_directories_offset: i64 = match magic {
        0x10b => 96,
        0x20b => 112,
        _ => return Ok(false),
    };

    // We've already read 2 bytes of the Optional Header (the magic); seek the rest.
    file.seek(SeekFrom::Current(data_directories_offset - 2))?;

    // Data Directories: 16 entries of (RVA: u32, Size: u32). The COM Descriptor
    // (CLR header) is entry index 14.
    const COM_DESCRIPTOR_INDEX: u64 = 14;
    file.seek(SeekFrom::Current((COM_DESCRIPTOR_INDEX * 8) as i64))?;
    let mut entry = [0u8; 8];
    file.read_exact(&mut entry)?;
    let rva = u32::from_le_bytes(entry[0..4].try_into().unwrap());
    let size = u32::from_le_bytes(entry[4..8].try_into().unwrap());

    Ok(rva != 0 && size != 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn non_pe_file_is_not_managed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("not_a_dll.dll");
        std::fs::write(&path, b"this is not a PE file at all").unwrap();
        assert!(!is_managed_assembly(&path));
    }

    #[test]
    fn empty_file_is_not_managed() {
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("empty.dll");
        std::fs::write(&path, b"").unwrap();
        assert!(!is_managed_assembly(&path));
    }
}
