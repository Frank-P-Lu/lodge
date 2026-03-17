use crate::error::{LodgeError, Result};
use crate::record::{fix_bool_fields, row_to_json};
use crate::schema::Collection;
use rusqlite::Connection;
use serde_json::Value;

pub fn create_fts_table(conn: &Connection, collection: &str, text_fields: &[String]) -> Result<()> {
    let fields_csv = text_fields.join(", ");

    // Create FTS5 virtual table
    let fts_sql = format!(
        "CREATE VIRTUAL TABLE \"{collection}_fts\" USING fts5({fields_csv}, content=\"{collection}\", content_rowid=\"id\", tokenize='trigram')"
    );
    conn.execute_batch(&fts_sql)
        .map_err(|e| LodgeError::Fts(format!("failed to create FTS table: {e}")))?;

    // Create triggers for auto-sync
    // INSERT trigger
    let insert_cols: Vec<String> = text_fields.iter().map(|f| format!("new.{f}")).collect();
    conn.execute_batch(&format!(
        "CREATE TRIGGER \"{collection}_fts_ai\" AFTER INSERT ON \"{collection}\" BEGIN
            INSERT INTO \"{collection}_fts\"(rowid, {fields_csv}) VALUES (new.id, {});
        END;",
        insert_cols.join(", ")
    ))
    .map_err(|e| LodgeError::Fts(format!("failed to create insert trigger: {e}")))?;

    // DELETE trigger
    let old_cols: Vec<String> = text_fields.iter().map(|f| format!("old.{f}")).collect();
    conn.execute_batch(&format!(
        "CREATE TRIGGER \"{collection}_fts_ad\" AFTER DELETE ON \"{collection}\" BEGIN
            INSERT INTO \"{collection}_fts\"(\"{collection}_fts\", rowid, {fields_csv}) VALUES('delete', old.id, {});
        END;",
        old_cols.join(", ")
    )).map_err(|e| LodgeError::Fts(format!("failed to create delete trigger: {e}")))?;

    // UPDATE trigger
    let new_cols: Vec<String> = text_fields.iter().map(|f| format!("new.{f}")).collect();
    conn.execute_batch(&format!(
        "CREATE TRIGGER \"{collection}_fts_au\" AFTER UPDATE ON \"{collection}\" BEGIN
            INSERT INTO \"{collection}_fts\"(\"{collection}_fts\", rowid, {fields_csv}) VALUES('delete', old.id, {old});
            INSERT INTO \"{collection}_fts\"(rowid, {fields_csv}) VALUES (new.id, {new});
        END;",
        old = old_cols.join(", "),
        new = new_cols.join(", "),
    )).map_err(|e| LodgeError::Fts(format!("failed to create update trigger: {e}")))?;

    // Populate FTS index from existing data
    conn.execute_batch(&format!(
        "INSERT INTO \"{collection}_fts\"(\"{collection}_fts\") VALUES('rebuild')"
    ))
    .map_err(|e| LodgeError::Fts(format!("failed to rebuild FTS index: {e}")))?;

    // Record in _lodge_fts_meta
    for field in text_fields {
        conn.execute(
            "INSERT INTO _lodge_fts_meta (collection, field_name) VALUES (?1, ?2)",
            rusqlite::params![collection, field],
        )?;
    }

    Ok(())
}

pub fn search_records(
    conn: &Connection,
    collection: &Collection,
    query: &str,
    limit: Option<i64>,
) -> Result<Vec<Value>> {
    let name = &collection.name;
    if !has_fts(conn, name)? {
        return Err(LodgeError::FtsNotEnabled(name.to_string()));
    }

    // Trigram tokenizer requires at least 3 characters
    if query.trim().len() < 3 {
        return Ok(Vec::new());
    }

    // Escape for FTS5 literal matching: wrap in double quotes, double any internal quotes
    let escaped = format!("\"{}\"", query.replace('"', "\"\""));

    let mut sql = format!(
        "SELECT t.* FROM \"{name}\" t JOIN \"{name}_fts\" fts ON t.id = fts.rowid WHERE \"{name}_fts\" MATCH ?1 ORDER BY fts.rank"
    );
    if let Some(l) = limit {
        sql.push_str(&format!(" LIMIT {l}"));
    }

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| LodgeError::Fts(format!("search query failed: {e}")))?;
    let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

    let rows = stmt
        .query_map([&escaped], |row| row_to_json(row, &column_names))
        .map_err(|e| LodgeError::Fts(format!("search query failed: {e}")))?;

    let mut results = Vec::new();
    for row in rows {
        let mut record = row.map_err(|e| LodgeError::Fts(e.to_string()))?;
        fix_bool_fields(&mut record, collection);
        results.push(record);
    }
    Ok(results)
}

pub fn drop_fts_table(conn: &Connection, collection: &str) -> Result<()> {
    conn.execute_batch(&format!(
        "DROP TABLE IF EXISTS \"{collection}_fts\";
         DROP TRIGGER IF EXISTS \"{collection}_fts_ai\";
         DROP TRIGGER IF EXISTS \"{collection}_fts_ad\";
         DROP TRIGGER IF EXISTS \"{collection}_fts_au\";"
    ))
    .map_err(|e| LodgeError::Fts(format!("failed to drop FTS table: {e}")))?;
    conn.execute(
        "DELETE FROM _lodge_fts_meta WHERE collection = ?1",
        [collection],
    )?;
    Ok(())
}

pub fn has_fts(conn: &Connection, collection: &str) -> Result<bool> {
    // Check if _lodge_fts_meta exists first
    let table_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='_lodge_fts_meta'",
        [],
        |row| row.get(0),
    )?;
    if !table_exists {
        return Ok(false);
    }
    let count: i64 = conn.query_row(
        "SELECT COUNT(*) FROM _lodge_fts_meta WHERE collection = ?1",
        [collection],
        |row| row.get(0),
    )?;
    Ok(count > 0)
}
