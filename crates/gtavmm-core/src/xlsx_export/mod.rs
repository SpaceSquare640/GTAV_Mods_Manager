// SPDX-License-Identifier: AGPL-3.0-only

//! Exports installed-mod records to a styled `.xlsx` workbook — the "Excel Export"
//! feature validated by the old prototype: two sheets ("Installed Mods" for
//! active/disabled mods, "Uninstalled Mods" for historical ones), bold headers,
//! sensible column widths, and a clickable hyperlink in the Link column when the mod
//! has one recorded.
//!
//! Read-only: this only reads from the database, it doesn't touch the filesystem
//! beyond writing the one output file the caller asked for.

use std::path::Path;

use rusqlite::Connection;
use rust_xlsxwriter::{Format, Workbook, Worksheet};

use crate::error::{CoreError, CoreResult};

struct ModRow {
    id: i64,
    name: String,
    source_type: String,
    installed_at: String,
    status: String,
    notes: Option<String>,
    link: Option<String>,
}

const COLUMN_HEADERS: [&str; 7] = [
    "ID",
    "Name",
    "Source Type",
    "Installed At",
    "Status",
    "Notes",
    "Link",
];

fn load_rows(conn: &Connection, uninstalled: bool) -> CoreResult<Vec<ModRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, source_type, installed_at, status, notes, link \
         FROM installed_mod \
         WHERE (status = 'uninstalled') = ?1 \
         ORDER BY id",
    )?;
    let rows = stmt.query_map([uninstalled], |row| {
        Ok(ModRow {
            id: row.get(0)?,
            name: row.get(1)?,
            source_type: row.get(2)?,
            installed_at: row.get(3)?,
            status: row.get(4)?,
            notes: row.get(5)?,
            link: row.get(6)?,
        })
    })?;
    rows.collect::<Result<Vec<_>, _>>().map_err(Into::into)
}

fn write_sheet(worksheet: &mut Worksheet, rows: &[ModRow]) -> CoreResult<()> {
    let xlsx_err = |e: rust_xlsxwriter::XlsxError| CoreError::UnsupportedFormat {
        reason: format!("writing xlsx sheet: {e}"),
    };

    let header_format = Format::new().set_bold();
    for (col, header) in COLUMN_HEADERS.iter().enumerate() {
        worksheet
            .write_string_with_format(0, col as u16, *header, &header_format)
            .map_err(xlsx_err)?;
    }

    for (index, row) in rows.iter().enumerate() {
        let excel_row = (index + 1) as u32;
        worksheet
            .write_number(excel_row, 0, row.id as f64)
            .map_err(xlsx_err)?;
        worksheet
            .write_string(excel_row, 1, &row.name)
            .map_err(xlsx_err)?;
        worksheet
            .write_string(excel_row, 2, &row.source_type)
            .map_err(xlsx_err)?;
        worksheet
            .write_string(excel_row, 3, &row.installed_at)
            .map_err(xlsx_err)?;
        worksheet
            .write_string(excel_row, 4, &row.status)
            .map_err(xlsx_err)?;
        worksheet
            .write_string(excel_row, 5, row.notes.as_deref().unwrap_or(""))
            .map_err(xlsx_err)?;

        match &row.link {
            Some(link) if !link.is_empty() => {
                worksheet
                    .write_url(excel_row, 6, link.as_str())
                    .map_err(xlsx_err)?;
            }
            _ => {
                worksheet.write_string(excel_row, 6, "").map_err(xlsx_err)?;
            }
        }
    }

    for (col, header) in COLUMN_HEADERS.iter().enumerate() {
        let width = (header.len() as f64).max(12.0);
        worksheet
            .set_column_width(col as u16, width)
            .map_err(xlsx_err)?;
    }

    Ok(())
}

/// Writes `output_path` as a two-sheet `.xlsx`: "Installed Mods" (active/disabled)
/// and "Uninstalled Mods" (historical). Overwrites `output_path` if it already exists.
pub fn export(conn: &Connection, output_path: &Path) -> CoreResult<()> {
    let installed = load_rows(conn, false)?;
    let uninstalled = load_rows(conn, true)?;

    let mut workbook = Workbook::new();

    let installed_sheet = workbook.add_worksheet();
    installed_sheet
        .set_name("Installed Mods")
        .map_err(|e| CoreError::UnsupportedFormat {
            reason: format!("naming xlsx sheet: {e}"),
        })?;
    write_sheet(installed_sheet, &installed)?;

    let uninstalled_sheet = workbook.add_worksheet();
    uninstalled_sheet
        .set_name("Uninstalled Mods")
        .map_err(|e| CoreError::UnsupportedFormat {
            reason: format!("naming xlsx sheet: {e}"),
        })?;
    write_sheet(uninstalled_sheet, &uninstalled)?;

    workbook
        .save(output_path)
        .map_err(|e| CoreError::UnsupportedFormat {
            reason: format!("saving xlsx workbook: {e}"),
        })?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn insert_mod(conn: &Connection, name: &str, status: &str, link: Option<&str>) {
        conn.execute(
            "INSERT INTO installed_mod (name, source_type, install_path, status, link) \
             VALUES (?1, 'asi', '', ?2, ?3)",
            rusqlite::params![name, status, link],
        )
        .unwrap();
    }

    #[test]
    fn export_creates_a_readable_workbook_with_both_sheets() {
        let conn = crate::db::open_in_memory().unwrap();
        insert_mod(
            &conn,
            "ActiveMod",
            "active",
            Some("https://example.com/mod"),
        );
        insert_mod(&conn, "DisabledMod", "disabled", None);
        insert_mod(&conn, "RemovedMod", "uninstalled", None);

        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("mods.xlsx");
        export(&conn, &output).unwrap();

        assert!(output.exists());
        assert!(std::fs::metadata(&output).unwrap().len() > 0);
    }

    #[test]
    fn separates_installed_from_uninstalled_rows() {
        let conn = crate::db::open_in_memory().unwrap();
        insert_mod(&conn, "Active", "active", None);
        insert_mod(&conn, "Gone", "uninstalled", None);

        let installed = load_rows(&conn, false).unwrap();
        let uninstalled = load_rows(&conn, true).unwrap();

        assert_eq!(installed.len(), 1);
        assert_eq!(installed[0].name, "Active");
        assert_eq!(uninstalled.len(), 1);
        assert_eq!(uninstalled[0].name, "Gone");
    }

    #[test]
    fn export_with_no_mods_still_produces_a_valid_file() {
        let conn = crate::db::open_in_memory().unwrap();
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("empty.xlsx");
        export(&conn, &output).unwrap();
        assert!(output.exists());
    }
}
