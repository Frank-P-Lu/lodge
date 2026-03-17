use crate::error::{LodgeError, Result};
use rusqlite::Connection;
use serde_json::Value;

pub fn create_fts_table(conn: &Connection, collection: &str, text_fields: &[String]) -> Result<()> {
    let fields_csv = text_fields.join(", ");

    // Create FTS5 virtual table
    let fts_sql = format!(
        "CREATE VIRTUAL TABLE \"{collection}_fts\" USING fts5({fields_csv}, content=\"{collection}\", content_rowid=\"id\")"
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
    collection: &str,
    query: &str,
    limit: Option<i64>,
) -> Result<Vec<Value>> {
    if !has_fts(conn, collection)? {
        return Err(LodgeError::FtsNotEnabled(collection.to_string()));
    }

    let mut sql = format!(
        "SELECT t.* FROM \"{collection}\" t JOIN \"{collection}_fts\" fts ON t.id = fts.rowid WHERE \"{collection}_fts\" MATCH ?1 ORDER BY fts.rank"
    );
    if let Some(l) = limit {
        sql.push_str(&format!(" LIMIT {l}"));
    }

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| LodgeError::Fts(format!("search query failed: {e}")))?;
    let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

    let rows = stmt
        .query_map([query], |row| {
            let mut obj = serde_json::Map::new();
            for (i, col) in column_names.iter().enumerate() {
                let val: rusqlite::types::Value = row.get(i)?;
                let json_val = match val {
                    rusqlite::types::Value::Null => Value::Null,
                    rusqlite::types::Value::Integer(n) => serde_json::json!(n),
                    rusqlite::types::Value::Real(f) => serde_json::json!(f),
                    rusqlite::types::Value::Text(s) => serde_json::json!(s),
                    rusqlite::types::Value::Blob(b) => {
                        serde_json::json!(format!("<blob {} bytes>", b.len()))
                    }
                };
                obj.insert(col.clone(), json_val);
            }
            Ok(Value::Object(obj))
        })
        .map_err(|e| LodgeError::Fts(format!("search query failed: {e}")))?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| LodgeError::Fts(e.to_string()))?);
    }
    Ok(results)
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
