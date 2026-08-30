// SPDX-License-Identifier: AGPL-3.0-only

//! Hand-rolled PE + CLI metadata parsing, just enough to locate and patch the `#US`
//! (User Strings) heap directly. Ported from the standalone R&D spike
//! (`Resource/[DLL Editor 驗證程式]/non-dotnetdll`) after it was verified at production
//! scale (143/143 real strings translated and patched into a real GTA V mod DLL,
//! cross-checked against `dotnetdll`'s spec-compliant parser) — see the 2026-08-30
//! entry in the design doc for the full verification history and the two bugs found
//! and fixed during that run (both already applied here).
//!
//! This supersedes the `.NET` DLL rejection documented in [`crate::translation`]'s
//! module doc comment (2026-08-28 decision): a hand-rolled Rust PE/CLI parser was
//! judged too high a correctness risk at the time, before it had been built and
//! proven. It has since been built and proven, so that decision no longer stands for
//! this specific `#US`-heap-only scope.

pub struct UsStringEntry {
    /// Absolute file offset of the *data* bytes (after the compressed length prefix).
    pub data_offset: usize,
    /// Total data length in bytes, including the trailing "has special chars" flag byte.
    pub data_len: usize,
    pub text: String,
}

/// Everything a growth-patch needs: where the stream header's own Offset/Size fields
/// live (so we can repoint them), and enough of the PE section table / optional header
/// to safely grow the section holding metadata.
#[derive(Debug)]
pub struct PeLayout {
    pub metadata_root_offset: usize,
    pub metadata_root_rva: u32,
    pub us_stream_header_offset_field: usize, // file offset of the 4-byte Offset field
    pub us_stream_header_size_field: usize,   // file offset of the 4-byte Size field
    pub us_heap_offset: usize,
    pub us_heap_size: usize,
    pub section_alignment: u32,
    pub file_alignment: u32,
    pub size_of_image_field: usize,
    pub security_dir_rva: u32,
    pub security_dir_size: u32,
    /// IMAGE_COR20_HEADER.Flags — bit 0x1 (COMIMAGE_FLAGS_ILONLY) tells us whether this
    /// assembly is pure-managed or mixed-mode (contains native code). Mixed-mode
    /// assemblies aren't supported by any of the patching logic here and are refused,
    /// not guessed at.
    pub cor20_flags: u32,
    pub sections: Vec<SectionFull>,
}

impl PeLayout {
    pub fn is_il_only(&self) -> bool {
        self.cor20_flags & 0x1 != 0
    }
    pub fn is_signed(&self) -> bool {
        self.security_dir_rva != 0 || self.security_dir_size != 0
    }
}

#[derive(Debug)]
pub struct SectionFull {
    pub header_offset: usize, // file offset of this section header (40 bytes)
    pub virtual_address: u32,
    pub virtual_size: u32,
    pub size_of_raw_data: u32,
    pub pointer_to_raw_data: u32,
}

pub fn read_u16(b: &[u8], off: usize) -> u16 {
    u16::from_le_bytes([b[off], b[off + 1]])
}
pub fn read_u32(b: &[u8], off: usize) -> u32 {
    u32::from_le_bytes([b[off], b[off + 1], b[off + 2], b[off + 3]])
}

/// ECMA-335 II.24.2.4 compressed unsigned integer. Returns (value, bytes_consumed).
pub fn read_compressed_u32(b: &[u8], off: usize) -> (u32, usize) {
    let b0 = b[off];
    if b0 & 0x80 == 0 {
        (b0 as u32, 1)
    } else if b0 & 0xC0 == 0x80 {
        let v = (((b0 & 0x3F) as u32) << 8) | b[off + 1] as u32;
        (v, 2)
    } else {
        let v = (((b0 & 0x1F) as u32) << 24)
            | (b[off + 1] as u32) << 16
            | (b[off + 2] as u32) << 8
            | b[off + 3] as u32;
        (v, 4)
    }
}

/// Encodes a length as an ECMA-335 compressed unsigned integer. Values above the 2-byte
/// range (0x3FFF) use the 4-byte form for simplicity (skips the 1-byte form's range).
pub fn write_compressed_u32(v: u32) -> Vec<u8> {
    if v < 0x80 {
        vec![v as u8]
    } else if v < 0x4000 {
        let v = v | 0x8000;
        vec![(v >> 8) as u8, (v & 0xFF) as u8]
    } else {
        let v = v | 0xC000_0000;
        vec![(v >> 24) as u8, (v >> 16) as u8, (v >> 8) as u8, v as u8]
    }
}

struct Section {
    virtual_address: u32,
    size_of_raw_data: u32,
    pointer_to_raw_data: u32,
}

fn rva_to_offset(sections: &[Section], rva: u32) -> Option<usize> {
    for s in sections {
        if rva >= s.virtual_address && rva < s.virtual_address + s.size_of_raw_data.max(1) {
            return Some((s.pointer_to_raw_data + (rva - s.virtual_address)) as usize);
        }
    }
    None
}

/// Full PE/CLI layout parse. Requires at least the file's DOS/PE headers, CLR header,
/// and metadata root to be well-formed; returns a descriptive `Err` otherwise rather
/// than panicking, since this always runs against a real user-supplied file.
pub fn parse_pe_layout(bytes: &[u8]) -> Result<PeLayout, String> {
    if bytes.len() < 0x40 || &bytes[0..2] != b"MZ" {
        return Err("not a PE file (missing MZ signature)".into());
    }
    let pe_offset = read_u32(bytes, 0x3C) as usize;
    if bytes.len() < pe_offset + 4 || &bytes[pe_offset..pe_offset + 4] != b"PE\0\0" {
        return Err("missing PE\\0\\0 signature".into());
    }
    let coff_offset = pe_offset + 4;
    let num_sections = read_u16(bytes, coff_offset + 2) as usize;
    let opt_header_size = read_u16(bytes, coff_offset + 16) as usize;
    let opt_header_offset = coff_offset + 20;
    let magic = read_u16(bytes, opt_header_offset);
    let is_pe32_plus = magic == 0x20B;
    let section_alignment = read_u32(bytes, opt_header_offset + 32);
    let file_alignment = read_u32(bytes, opt_header_offset + 36);
    let size_of_image_field = opt_header_offset + 56;
    let data_dirs_offset = opt_header_offset + if is_pe32_plus { 112 } else { 96 };
    let com_descriptor_dir_offset = data_dirs_offset + 14 * 8;
    let com_descriptor_rva = read_u32(bytes, com_descriptor_dir_offset);
    if com_descriptor_rva == 0 {
        return Err("no CLR header directory entry — this isn't a managed .NET assembly".into());
    }
    // Directory 4 = Security (Authenticode certificate table). If present, it lives
    // appended past the end of the mapped sections and is NOT part of any section's
    // raw data — appending our own bytes "at EOF" would collide with it.
    let security_dir_offset = data_dirs_offset + 4 * 8;
    let security_dir_rva = read_u32(bytes, security_dir_offset);
    let security_dir_size = read_u32(bytes, security_dir_offset + 4);

    let section_table_offset = opt_header_offset + opt_header_size;
    let mut sections = Vec::with_capacity(num_sections);
    let mut sections_simple = Vec::with_capacity(num_sections);
    for i in 0..num_sections {
        let s = section_table_offset + i * 40;
        let full = SectionFull {
            header_offset: s,
            virtual_address: read_u32(bytes, s + 12),
            virtual_size: read_u32(bytes, s + 8),
            size_of_raw_data: read_u32(bytes, s + 16),
            pointer_to_raw_data: read_u32(bytes, s + 20),
        };
        sections_simple.push(Section {
            virtual_address: full.virtual_address,
            size_of_raw_data: full.size_of_raw_data,
            pointer_to_raw_data: full.pointer_to_raw_data,
        });
        sections.push(full);
    }

    let com_descriptor_offset = rva_to_offset(&sections_simple, com_descriptor_rva)
        .ok_or("could not map CLR header RVA to a file offset")?;
    let cor20_flags = read_u32(bytes, com_descriptor_offset + 16);
    let metadata_rva = read_u32(bytes, com_descriptor_offset + 8);
    let metadata_root_offset = rva_to_offset(&sections_simple, metadata_rva)
        .ok_or("could not map metadata RVA to a file offset")?;

    if read_u32(bytes, metadata_root_offset) != 0x424A5342 {
        return Err("metadata root signature (BSJB) not found".into());
    }
    let version_len = read_u32(bytes, metadata_root_offset + 12) as usize;
    let flags_offset = metadata_root_offset + 16 + version_len;
    let stream_count = read_u16(bytes, flags_offset + 2) as usize;
    let mut cursor = flags_offset + 4;
    for _ in 0..stream_count {
        let stream_header_offset_field = cursor;
        let stream_header_size_field = cursor + 4;
        let stream_offset = read_u32(bytes, cursor) as usize;
        let stream_size = read_u32(bytes, cursor + 4) as usize;
        let name_start = cursor + 8;
        let mut name_end = name_start;
        while bytes[name_end] != 0 {
            name_end += 1;
        }
        let name = String::from_utf8_lossy(&bytes[name_start..name_end]).to_string();
        let name_padded_len = (name_end - name_start + 1).div_ceil(4) * 4;
        cursor = name_start + name_padded_len;

        if name == "#US" {
            // The stream header's Offset is relative to the metadata root's RVA, not
            // to its file offset — matters once the stream lives in a different PE
            // section than the metadata root (the EOF-append growth path below).
            let us_heap_rva = metadata_rva + stream_offset as u32;
            let us_heap_offset = rva_to_offset(&sections_simple, us_heap_rva)
                .ok_or("could not map #US stream RVA to a file offset")?;
            return Ok(PeLayout {
                metadata_root_offset,
                metadata_root_rva: metadata_rva,
                us_stream_header_offset_field: stream_header_offset_field,
                us_stream_header_size_field: stream_header_size_field,
                us_heap_offset,
                us_heap_size: stream_size,
                section_alignment,
                file_alignment,
                size_of_image_field,
                security_dir_rva,
                security_dir_size,
                cor20_flags,
                sections,
            });
        }
    }
    Err("no #US stream found in this assembly (no string literals were ever embedded)".into())
}

pub fn round_up(v: u32, align: u32) -> u32 {
    if align == 0 {
        return v;
    }
    v.div_ceil(align) * align
}

// ---------------------------------------------------------------------------
// Structural ldstr locator — independently parses the `#~` metadata tables
// stream (just enough of it: tables 0-6 and the row count of table 8) to get
// each MethodDef row's RVA in file order, then decodes the real IL instruction
// stream of one specific method to find the exact file offset of its ldstr
// operand — no guessing, no risk of colliding with an unrelated byte sequence
// elsewhere in the file.
// ---------------------------------------------------------------------------

/// Metadata table numbers we need (ECMA-335 II.22).
const TBL_MODULE: usize = 0x00;
const TBL_TYPEREF: usize = 0x01;
const TBL_TYPEDEF: usize = 0x02;
const TBL_FIELDPTR: usize = 0x03;
const TBL_FIELD: usize = 0x04;
const TBL_METHODPTR: usize = 0x05;
const TBL_METHODDEF: usize = 0x06;
const TBL_PARAM: usize = 0x08;
const TBL_MODULEREF: usize = 0x1A;
const TBL_ASSEMBLYREF: usize = 0x23;
const TBL_TYPESPEC: usize = 0x1B;

fn coded_index_size(row_counts: &[u32; 64], tables: &[usize]) -> usize {
    let tag_bits = (tables.len() as f64).log2().ceil() as u32;
    let max_rows = tables.iter().map(|&t| row_counts[t]).max().unwrap_or(0);
    if max_rows < (1u32 << (16 - tag_bits)) {
        2
    } else {
        4
    }
}
fn simple_index_size(row_counts: &[u32; 64], table: usize) -> usize {
    if row_counts[table] < 0x10000 {
        2
    } else {
        4
    }
}

/// Finds the `#~` (or `#-`) metadata tables stream and returns (file_offset, size).
pub fn find_tables_stream(bytes: &[u8], layout: &PeLayout) -> Result<(usize, usize), String> {
    let version_len = read_u32(bytes, layout.metadata_root_offset + 12) as usize;
    let flags_offset = layout.metadata_root_offset + 16 + version_len;
    let stream_count = read_u16(bytes, flags_offset + 2) as usize;
    let mut cursor = flags_offset + 4;
    for _ in 0..stream_count {
        let stream_offset = read_u32(bytes, cursor) as usize;
        let stream_size = read_u32(bytes, cursor + 4) as usize;
        let name_start = cursor + 8;
        let mut name_end = name_start;
        while bytes[name_end] != 0 {
            name_end += 1;
        }
        let name = String::from_utf8_lossy(&bytes[name_start..name_end]).to_string();
        let name_padded_len = (name_end - name_start + 1).div_ceil(4) * 4;
        cursor = name_start + name_padded_len;
        if name == "#~" || name == "#-" {
            return Ok((layout.metadata_root_offset + stream_offset, stream_size));
        }
    }
    Err("no #~/#- metadata tables stream found".into())
}

/// Returns each MethodDef row's RVA, in table row order (== the same order a
/// spec-compliant reader's type/method enumeration emits methods, since ECMA-335
/// requires TypeDef.MethodList ranges to be contiguous and non-decreasing).
pub fn read_method_rvas_in_order(bytes: &[u8], tables_offset: usize) -> Result<Vec<u32>, String> {
    let heap_sizes = bytes[tables_offset + 6];
    let string_idx_size = if heap_sizes & 0x01 != 0 { 4 } else { 2 };
    let blob_idx_size = if heap_sizes & 0x04 != 0 { 4 } else { 2 };
    let guid_idx_size = if heap_sizes & 0x02 != 0 { 4 } else { 2 };

    let valid = u64::from_le_bytes(
        bytes[tables_offset + 8..tables_offset + 16]
            .try_into()
            .unwrap(),
    );
    let mut row_counts = [0u32; 64];
    let mut cursor = tables_offset + 24;
    #[allow(clippy::needless_range_loop)]
    // indexes row_counts conditionally on `valid`, not a plain map
    for t in 0..64 {
        if valid & (1u64 << t) != 0 {
            row_counts[t] = read_u32(bytes, cursor);
            cursor += 4;
        }
    }

    let res_scope_size = coded_index_size(
        &row_counts,
        &[TBL_MODULE, TBL_MODULEREF, TBL_ASSEMBLYREF, TBL_TYPEREF],
    );
    let type_def_or_ref_size =
        coded_index_size(&row_counts, &[TBL_TYPEDEF, TBL_TYPEREF, TBL_TYPESPEC]);
    let field_idx_size = simple_index_size(&row_counts, TBL_FIELD);
    let method_idx_size = simple_index_size(&row_counts, TBL_METHODDEF);
    let param_idx_size = simple_index_size(&row_counts, TBL_PARAM);

    let row_size = |table: usize| -> usize {
        match table {
            TBL_MODULE => 2 + string_idx_size + guid_idx_size * 3,
            TBL_TYPEREF => res_scope_size + string_idx_size * 2,
            TBL_TYPEDEF => {
                4 + string_idx_size * 2 + type_def_or_ref_size + field_idx_size + method_idx_size
            }
            TBL_FIELDPTR => field_idx_size,
            TBL_FIELD => 2 + string_idx_size + blob_idx_size,
            TBL_METHODPTR => method_idx_size,
            TBL_METHODDEF => 4 + 2 + 2 + string_idx_size + blob_idx_size + param_idx_size,
            _ => unreachable!("only tables 0-6 are walked"),
        }
    };

    for t in [
        TBL_MODULE,
        TBL_TYPEREF,
        TBL_TYPEDEF,
        TBL_FIELDPTR,
        TBL_FIELD,
        TBL_METHODPTR,
    ] {
        cursor += row_size(t) * row_counts[t] as usize;
    }

    let method_row_size = row_size(TBL_METHODDEF);
    let mut rvas = Vec::with_capacity(row_counts[TBL_METHODDEF] as usize);
    for i in 0..row_counts[TBL_METHODDEF] as usize {
        rvas.push(read_u32(bytes, cursor + i * method_row_size));
    }
    Ok(rvas)
}

#[derive(Debug)]
pub struct LdstrOccurrence {
    /// File offset of the opcode byte itself (0x72).
    pub opcode_offset: usize,
    pub token: u32,
}

/// Decodes one method's IL body (given its RVA) and returns every `ldstr`
/// instruction's exact file offset + token — a real per-instruction walk, not a
/// byte-pattern guess.
pub fn find_ldstr_in_method(
    bytes: &[u8],
    layout: &PeLayout,
    method_rva: u32,
) -> Result<Vec<LdstrOccurrence>, String> {
    let offset =
        rva_to_offset_pub(layout, method_rva).ok_or("method RVA does not map to any section")?;
    let header_byte = bytes[offset];
    let (code_start, code_size) = if header_byte & 0x3 == 0x2 {
        // Tiny format: top 6 bits of the single header byte are the code size.
        (offset + 1, (header_byte >> 2) as usize)
    } else if header_byte & 0x3 == 0x3 {
        // Fat format: 12-byte header (assuming the standard 3-dword size).
        let header_word = read_u16(bytes, offset);
        let size_in_dwords = (header_word >> 12) & 0xF;
        let header_len = size_in_dwords as usize * 4;
        let code_size = read_u32(bytes, offset + 4) as usize;
        (offset + header_len, code_size)
    } else {
        return Err(format!(
            "unrecognized method header byte {header_byte:#04x}"
        ));
    };

    let mut occurrences = Vec::new();
    let mut pos = code_start;
    let end = code_start + code_size;
    while pos < end {
        let b0 = bytes[pos];
        if b0 == 0xFE {
            let b1 = bytes[pos + 1];
            let operand_len = two_byte_operand_len(b1);
            pos += 2 + operand_len;
        } else if b0 == 0x72 {
            // ldstr — our target. Operand is always a 4-byte String token.
            occurrences.push(LdstrOccurrence {
                opcode_offset: pos,
                token: read_u32(bytes, pos + 1),
            });
            pos += 1 + 4;
        } else if b0 == 0x45 {
            // switch — variable-length: 4-byte count N, then N * 4-byte targets.
            let n = read_u32(bytes, pos + 1) as usize;
            pos += 1 + 4 + n * 4;
        } else {
            let operand_len = single_byte_operand_len(b0);
            pos += 1 + operand_len;
        }
    }
    if pos != end {
        return Err(format!("IL walk misaligned: ended at {pos:#x}, expected {end:#x} — operand-length table is missing an opcode used by this method"));
    }
    Ok(occurrences)
}

fn rva_to_offset_pub(layout: &PeLayout, rva: u32) -> Option<usize> {
    for s in &layout.sections {
        if rva >= s.virtual_address && rva < s.virtual_address + s.size_of_raw_data.max(1) {
            return Some((s.pointer_to_raw_data + (rva - s.virtual_address)) as usize);
        }
    }
    None
}

/// Operand byte length for single-byte opcodes (0x00-0xFF, excluding the 0xFE
/// two-byte prefix and 0x45 `switch`, which are handled separately by the caller).
/// Everything not listed here is `InlineNone` (0 operand bytes) — covers the common
/// C#-compiler-emitted opcode set; exotic/rare opcodes (calli, jmp, mkrefany,
/// refanyval, tail./unaligned. prefixes' rare siblings) are not expected in ordinary
/// game-mod IL and aren't included, matching the spec's public opcode table (ECMA-335
/// Partition III).
fn single_byte_operand_len(opcode: u8) -> usize {
    match opcode {
        // 1-byte operand (short-form var/int/branch)
        0x0E..=0x13 | 0x1F | 0x2B..=0x37 | 0xDE => 1,
        // 4-byte operand (int32, float32, branch offset, or any metadata token)
        0x20
        | 0x22
        | 0x27..=0x29
        | 0x38..=0x44
        | 0x6F..=0x75
        | 0x79
        | 0x7B..=0x81
        | 0x8C
        | 0x8D
        | 0x8F
        | 0xA3..=0xA5
        | 0xC2
        | 0xC6
        | 0xD0
        | 0xDD => 4,
        // 8-byte operand (int64, float64)
        0x21 | 0x23 => 8,
        _ => 0,
    }
}

/// Operand byte length for 0xFE-prefixed two-byte opcodes (indexed by the second
/// byte). Same "list only the non-zero ones" approach as above.
fn two_byte_operand_len(second_byte: u8) -> usize {
    match second_byte {
        0x06 | 0x07 | 0x15 | 0x16 | 0x1C => 4, // ldftn/ldvirtftn/initobj/constrained./sizeof (all take a token)
        0x09..=0x0E => 2,                      // long-form ldarg/ldarga/starg/ldloc/ldloca/stloc
        0x12 => 1,                             // unaligned.
        _ => 0,
    }
}

/// Builds one `#US` heap entry (compressed length prefix + UTF-16LE data + trailing
/// "has special chars" flag byte) for `text`.
pub fn build_us_entry(text: &str) -> Vec<u8> {
    let units: Vec<u16> = text.encode_utf16().collect();
    let data_len = units.len() * 2 + 1;
    let mut entry = write_compressed_u32(data_len as u32);
    for u in &units {
        entry.extend_from_slice(&u.to_le_bytes());
    }
    entry.push(0);
    entry
}

/// Relocates the `#US` heap to make room for `new_entries` (each already built via
/// [`build_us_entry`]), appending all of them together in ONE relocation. Uses a 3-tier
/// strategy: prefer the `#US` section's own tail slack, then the last section's tail
/// slack, then a true EOF append (only if the last section ends exactly at EOF and
/// there's no Authenticode signature).
///
/// Returns the patched bytes and each new entry's heap-relative offset (in the same
/// order as `new_entries`), which the caller turns into `0x7000_0000 | offset` tokens.
pub fn relocate_and_append_entries(
    bytes: &[u8],
    layout: &PeLayout,
    new_entries: &[Vec<u8>],
) -> Result<(Vec<u8>, Vec<u32>), String> {
    if layout.security_dir_rva != 0 || layout.security_dir_size != 0 {
        return Err("Authenticode signature present — refusing to append at EOF".into());
    }

    let old_heap_bytes = &bytes[layout.us_heap_offset..layout.us_heap_offset + layout.us_heap_size];
    let mut new_heap_bytes = old_heap_bytes.to_vec();
    let mut new_offsets = Vec::with_capacity(new_entries.len());
    for entry in new_entries {
        new_offsets.push(new_heap_bytes.len() as u32);
        new_heap_bytes.extend_from_slice(entry);
    }
    let needed = new_heap_bytes.len() as u32;

    let us_section_idx = layout
        .sections
        .iter()
        .position(|s| {
            let start = s.pointer_to_raw_data as usize;
            layout.us_heap_offset >= start
                && layout.us_heap_offset < start + s.size_of_raw_data as usize
        })
        .ok_or("us heap must live inside some section")?;
    let us_sec = &layout.sections[us_section_idx];
    let last_section_idx = (0..layout.sections.len())
        .max_by_key(|&i| {
            layout.sections[i].pointer_to_raw_data + layout.sections[i].size_of_raw_data
        })
        .ok_or("no sections")?;
    let last_sec = &layout.sections[last_section_idx];

    let mut patched = bytes.to_vec();
    let us_slack = us_sec.size_of_raw_data.saturating_sub(us_sec.virtual_size);
    let new_data_rva = if us_slack >= needed {
        let offset = (us_sec.pointer_to_raw_data + us_sec.virtual_size) as usize;
        patched[offset..offset + new_heap_bytes.len()].copy_from_slice(&new_heap_bytes);
        let nvs = us_sec.virtual_size + needed;
        patched[us_sec.header_offset + 8..us_sec.header_offset + 12]
            .copy_from_slice(&nvs.to_le_bytes());
        grow_size_of_image(&mut patched, layout, us_sec.virtual_address + nvs);
        us_sec.virtual_address + us_sec.virtual_size
    } else {
        let last_slack = last_sec
            .size_of_raw_data
            .saturating_sub(last_sec.virtual_size);
        if last_slack >= needed {
            let offset = (last_sec.pointer_to_raw_data + last_sec.virtual_size) as usize;
            patched[offset..offset + new_heap_bytes.len()].copy_from_slice(&new_heap_bytes);
            let nvs = last_sec.virtual_size + needed;
            patched[last_sec.header_offset + 8..last_sec.header_offset + 12]
                .copy_from_slice(&nvs.to_le_bytes());
            grow_size_of_image(&mut patched, layout, last_sec.virtual_address + nvs);
            last_sec.virtual_address + last_sec.virtual_size
        } else {
            if last_sec.pointer_to_raw_data as usize + last_sec.size_of_raw_data as usize
                != bytes.len()
            {
                return Err("last section doesn't end at EOF — can't safely append".into());
            }
            while !patched.len().is_multiple_of(layout.file_alignment as usize) {
                patched.push(0);
            }
            let append_offset = patched.len();
            patched.extend_from_slice(&new_heap_bytes);
            while !patched.len().is_multiple_of(layout.file_alignment as usize) {
                patched.push(0);
            }
            let new_srd = (patched.len() as u32) - last_sec.pointer_to_raw_data;
            let rva =
                last_sec.virtual_address + (append_offset as u32 - last_sec.pointer_to_raw_data);
            let nvs = (rva - last_sec.virtual_address) + needed;
            patched[last_sec.header_offset + 8..last_sec.header_offset + 12]
                .copy_from_slice(&nvs.to_le_bytes());
            patched[last_sec.header_offset + 16..last_sec.header_offset + 20]
                .copy_from_slice(&new_srd.to_le_bytes());
            grow_size_of_image(&mut patched, layout, last_sec.virtual_address + nvs);
            rva
        }
    };

    let new_stream_offset_value = new_data_rva - layout.metadata_root_rva;
    patched[layout.us_stream_header_offset_field..layout.us_stream_header_offset_field + 4]
        .copy_from_slice(&new_stream_offset_value.to_le_bytes());
    patched[layout.us_stream_header_size_field..layout.us_stream_header_size_field + 4]
        .copy_from_slice(&needed.to_le_bytes());

    Ok((patched, new_offsets))
}

fn grow_size_of_image(patched: &mut [u8], layout: &PeLayout, new_extent_rva: u32) {
    let new_soi = round_up(new_extent_rva, layout.section_alignment);
    let old_soi = read_u32(patched, layout.size_of_image_field);
    if new_soi > old_soi {
        patched[layout.size_of_image_field..layout.size_of_image_field + 4]
            .copy_from_slice(&new_soi.to_le_bytes());
    }
}

/// Structurally locates every occurrence of every token in `old_tokens` across the
/// WHOLE assembly (every MethodDef row's real IL, decoded instruction-by-instruction).
/// Returns, for each input token in the same order, the list of file offsets where its
/// `ldstr` opcode was found (an empty list means the caller should refuse to patch it
/// rather than guess).
pub fn find_tokens_in_all_methods(
    bytes: &[u8],
    layout: &PeLayout,
    tables_offset: usize,
    old_tokens: &[u32],
) -> Result<Vec<Vec<usize>>, String> {
    let method_rvas = read_method_rvas_in_order(bytes, tables_offset)?;
    let mut results: Vec<Vec<usize>> = vec![Vec::new(); old_tokens.len()];
    for &rva in &method_rvas {
        if rva == 0 {
            continue;
        }
        let Ok(occurrences) = find_ldstr_in_method(bytes, layout, rva) else {
            continue; // skip methods we can't walk, same as the proven spike
        };
        for occ in &occurrences {
            if let Some(idx) = old_tokens.iter().position(|&t| t == occ.token) {
                results[idx].push(occ.opcode_offset);
            }
        }
    }
    Ok(results)
}

/// Heuristic classifier for "is this #US heap entry real translatable prose, or a
/// technical identifier (ped model name, animation dict/clip name, native/event
/// constant, file path, bare config key)?" — used to auto-scope a translation job to
/// real user-facing text without a human having to hand-pick every string.
///
/// Deliberately conservative in one direction: a short allowlist of common English
/// words that plausibly ARE real UI values despite looking like bare identifiers
/// (e.g. a "none"/"nothing" menu choice) is exempted from the "single lowercase
/// token" exclusion rule.
pub fn is_technical_string(text: &str) -> bool {
    if text.is_empty() {
        return true;
    }
    if text.contains('@') || text.contains('/') || text.contains('\\') {
        return true;
    }
    if text.starts_with("G_M_Y_") || text.starts_with("G_F_Y_") || text.starts_with("g_m_y_") {
        return true;
    }
    let stripped = strip_us_tags(text);
    if stripped.trim().is_empty() {
        return true;
    }
    let has_underscore = stripped.contains('_');
    let has_lowercase = stripped.chars().any(|c| c.is_ascii_lowercase());
    if has_underscore && !has_lowercase {
        return true; // ALL_CAPS_WITH_UNDERSCORES native/event constant
    }
    if !stripped.contains(' ')
        && !has_lowercase
        && stripped.chars().any(|c| c.is_ascii_alphabetic())
    {
        return true; // bare single ALLCAPS word, e.g. "UNIT"
    }
    if !stripped.contains(' ') && stripped.contains('_') {
        return true; // single-token identifier with an underscore, any case
    }
    const ALLOW_LOWERCASE_SINGLE_WORD: &[&str] = &["nothing", "none"];
    if !stripped.contains(' ')
        && stripped
            .chars()
            .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
        && !ALLOW_LOWERCASE_SINGLE_WORD.contains(&stripped.as_str())
    {
        return true; // single all-lowercase token, e.g. "base", "commonmenu"
    }
    false
}

/// Strips RAGE Plugin Hook-style `~x~` color/format tags, for judging whether any
/// real content is left in a string — does not mutate the original text elsewhere.
pub fn strip_us_tags(text: &str) -> String {
    let mut out = String::new();
    let mut chars = text.chars().peekable();
    while let Some(c) = chars.next() {
        if c == '~' {
            while let Some(&n) = chars.peek() {
                chars.next();
                if n == '~' {
                    break;
                }
            }
        } else {
            out.push(c);
        }
    }
    out
}

pub fn parse_us_heap(bytes: &[u8], heap_offset: usize, heap_size: usize) -> Vec<UsStringEntry> {
    let mut entries = Vec::new();
    let mut pos = 1usize; // index 0 is always the reserved empty entry (a single 0x00 byte)
    while pos < heap_size {
        let abs = heap_offset + pos;
        let (len, prefix_bytes) = read_compressed_u32(bytes, abs);
        let len = len as usize;
        if len == 0 {
            pos += prefix_bytes;
            continue;
        }
        let data_offset = abs + prefix_bytes;
        let char_count = (len - 1) / 2;
        let mut units = Vec::with_capacity(char_count);
        for i in 0..char_count {
            units.push(read_u16(bytes, data_offset + i * 2));
        }
        let text = String::from_utf16_lossy(&units);
        entries.push(UsStringEntry {
            data_offset,
            data_len: len,
            text,
        });
        pos += prefix_bytes + len;
    }
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn not_a_pe_file_is_rejected_cleanly() {
        let err = parse_pe_layout(b"not a pe file at all").unwrap_err();
        assert!(err.contains("MZ signature"));
    }

    #[test]
    fn is_technical_string_excludes_identifiers_and_paths() {
        assert!(is_technical_string(""));
        assert!(is_technical_string("weapons@pistol_1h@gang"));
        assert!(is_technical_string("Scripts\\GangModV1.ini"));
        assert!(is_technical_string("G_M_Y_STRPUNK_01"));
        assert!(is_technical_string("SOME_NATIVE_EVENT"));
        assert!(is_technical_string("UNIT"));
        assert!(is_technical_string("base_key"));
        assert!(is_technical_string("commonmenu"));
    }

    #[test]
    fn is_technical_string_keeps_real_ui_text_and_allowlisted_words() {
        assert!(!is_technical_string("Vehicle"));
        assert!(!is_technical_string("~r~Back"));
        assert!(!is_technical_string("Gang member recruited!"));
        assert!(!is_technical_string("none"));
        assert!(!is_technical_string("nothing"));
    }

    #[test]
    fn compressed_u32_round_trips_across_all_three_encoding_widths() {
        for v in [0u32, 0x7F, 0x80, 0x3FFF, 0x4000, 0x1FFF_FFFF] {
            let encoded = write_compressed_u32(v);
            let (decoded, consumed) = read_compressed_u32(&encoded, 0);
            assert_eq!(decoded, v);
            assert_eq!(consumed, encoded.len());
        }
    }

    #[test]
    fn build_us_entry_encodes_length_prefix_utf16_and_trailing_flag_byte() {
        let entry = build_us_entry("Hi");
        // "Hi" -> 2 UTF-16 code units (4 bytes) + 1 trailing flag byte = 5, fits the
        // 1-byte compressed-length form.
        assert_eq!(entry, vec![5, b'H', 0, b'i', 0, 0]);
    }
}
