// SPDX-License-Identifier: AGPL-3.0-only

//! AI Workflow / Prompt template library — the user's own reusable prompt text,
//! stored and edited via plain CRUD. **Not** part of the AI Assistant's Action Schema
//! (see `crate::ai_assistant` module docs): a template is just text the user pastes to
//! an AI provider by hand, never applied or executed automatically by this project.

use rusqlite::Connection;

use crate::error::{CoreError, CoreResult};

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PromptTemplate {
    pub id: i64,
    pub name: String,
    pub content: String,
    pub created_at: String,
    pub updated_at: String,
}

/// Creates a new template and returns its id.
pub fn create(conn: &Connection, name: &str, content: &str) -> CoreResult<i64> {
    conn.execute(
        "INSERT INTO prompt_template (name, content) VALUES (?1, ?2)",
        rusqlite::params![name, content],
    )?;
    Ok(conn.last_insert_rowid())
}

/// Lists all templates, most recently updated first.
pub fn list(conn: &Connection) -> CoreResult<Vec<PromptTemplate>> {
    let mut stmt = conn.prepare(
        "SELECT id, name, content, created_at, updated_at \
         FROM prompt_template ORDER BY updated_at DESC",
    )?;
    let rows = stmt
        .query_map([], |row| {
            Ok(PromptTemplate {
                id: row.get(0)?,
                name: row.get(1)?,
                content: row.get(2)?,
                created_at: row.get(3)?,
                updated_at: row.get(4)?,
            })
        })?
        .collect::<Result<_, _>>()?;
    Ok(rows)
}

/// Updates an existing template's name and content, bumping `updated_at`.
pub fn update(conn: &Connection, id: i64, name: &str, content: &str) -> CoreResult<()> {
    let affected = conn.execute(
        "UPDATE prompt_template \
         SET name = ?1, content = ?2, updated_at = strftime('%Y-%m-%dT%H:%M:%fZ', 'now') \
         WHERE id = ?3",
        rusqlite::params![name, content, id],
    )?;
    if affected == 0 {
        return Err(CoreError::PromptTemplate {
            reason: format!("no prompt template with id {id}"),
        });
    }
    Ok(())
}

/// Deletes a template by id.
pub fn delete(conn: &Connection, id: i64) -> CoreResult<()> {
    let affected = conn.execute("DELETE FROM prompt_template WHERE id = ?1", [id])?;
    if affected == 0 {
        return Err(CoreError::PromptTemplate {
            reason: format!("no prompt template with id {id}"),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn create_then_list_round_trips() {
        let conn = crate::db::open_in_memory().unwrap();
        let id = create(&conn, "Crash triage", "Diagnose this crash log:").unwrap();

        let templates = list(&conn).unwrap();
        assert_eq!(templates.len(), 1);
        assert_eq!(templates[0].id, id);
        assert_eq!(templates[0].name, "Crash triage");
        assert_eq!(templates[0].content, "Diagnose this crash log:");
    }

    #[test]
    fn list_orders_most_recently_updated_first() {
        let conn = crate::db::open_in_memory().unwrap();
        let first = create(&conn, "First", "a").unwrap();
        let second = create(&conn, "Second", "b").unwrap();

        update(&conn, first, "First (edited)", "a-edited").unwrap();

        let templates = list(&conn).unwrap();
        assert_eq!(templates[0].id, first);
        assert_eq!(templates[0].name, "First (edited)");
        assert_eq!(templates[1].id, second);
    }

    #[test]
    fn update_unknown_id_errors() {
        let conn = crate::db::open_in_memory().unwrap();
        let err = update(&conn, 999, "x", "y").unwrap_err();
        assert!(matches!(err, CoreError::PromptTemplate { .. }));
    }

    #[test]
    fn delete_removes_the_row() {
        let conn = crate::db::open_in_memory().unwrap();
        let id = create(&conn, "Temp", "content").unwrap();
        delete(&conn, id).unwrap();
        assert!(list(&conn).unwrap().is_empty());
    }

    #[test]
    fn delete_unknown_id_errors() {
        let conn = crate::db::open_in_memory().unwrap();
        let err = delete(&conn, 999).unwrap_err();
        assert!(matches!(err, CoreError::PromptTemplate { .. }));
    }
}
