use crate::error::{LodgeError, Result};
use crate::schema::Collection;
use crate::types::FieldType;
use rusqlite::Connection;
use serde_json::{json, Value};

pub fn fix_bool_fields(record: &mut Value, collection: &Collection) {
    if let Value::Object(ref mut map) = record {
        for field in &collection.fields {
            if field.field_type == FieldType::Bool {
                if let Some(val) = map.get_mut(&field.name) {
                    match val {
                        Value::Number(n) if n.as_i64() == Some(0) => *val = Value::Bool(false),
                        Value::Number(n) if n.as_i64() == Some(1) => *val = Value::Bool(true),
                        _ => {}
                    }
                }
            }
        }
    }
}

pub fn add_record(
    conn: &Connection,
    collection: &Collection,
    values: &[(String, String)],
) -> Result<Value> {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    // Validate and collect field values
    let mut field_names = Vec::new();
    let mut field_values = Vec::new();

    for field in &collection.fields {
        if let Some((_, val)) = values.iter().find(|(k, _)| k == &field.name) {
            let validated = field.field_type.validate(val, &field.name)?;
            field_names.push(field.name.clone());
            field_values.push(validated);
        }
        // Fields without values are left NULL (optional)
    }

    field_names.push("created_at".to_string());
    field_values.push(now.clone());
    field_names.push("updated_at".to_string());
    field_values.push(now);

    let placeholders: Vec<String> = (1..=field_names.len()).map(|i| format!("?{i}")).collect();
    let sql = format!(
        "INSERT INTO \"{}\" ({}) VALUES ({})",
        collection.name,
        field_names.join(", "),
        placeholders.join(", ")
    );

    let params: Vec<&dyn rusqlite::types::ToSql> = field_values
        .iter()
        .map(|v| v as &dyn rusqlite::types::ToSql)
        .collect();

    conn.execute(&sql, params.as_slice())?;
    let id = conn.last_insert_rowid();

    // Read back the inserted row
    get_record_by_id(conn, collection, id)
}

pub fn query_records(
    conn: &Connection,
    collection: &Collection,
    where_clause: Option<&str>,
    sort: Option<&str>,
    limit: Option<i64>,
) -> Result<Vec<Value>> {
    let mut sql = format!("SELECT * FROM \"{}\"", collection.name);
    if let Some(w) = where_clause {
        sql.push_str(&format!(" WHERE {w}"));
    }
    if let Some(s) = sort {
        sql.push_str(&format!(" ORDER BY {s}"));
    }
    if let Some(l) = limit {
        sql.push_str(&format!(" LIMIT {l}"));
    }

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| LodgeError::Sql(e.to_string()))?;
    let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

    let rows = stmt
        .query_map([], |row| row_to_json(row, &column_names))
        .map_err(|e| LodgeError::Sql(e.to_string()))?;

    let mut results = Vec::new();
    for row in rows {
        let mut record = row.map_err(|e| LodgeError::Sql(e.to_string()))?;
        fix_bool_fields(&mut record, collection);
        results.push(record);
    }
    Ok(results)
}

pub fn update_record(
    conn: &Connection,
    collection: &Collection,
    id: i64,
    values: &[(String, String)],
) -> Result<Value> {
    // Check record exists
    let exists: bool = conn.query_row(
        &format!(
            "SELECT COUNT(*) > 0 FROM \"{}\" WHERE id = ?1",
            collection.name
        ),
        [id],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(LodgeError::RecordNotFound(id));
    }

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    let mut set_clauses = Vec::new();
    let mut params: Vec<String> = Vec::new();

    for (key, val) in values {
        let field = collection
            .fields
            .iter()
            .find(|f| f.name == *key)
            .ok_or_else(|| {
                LodgeError::InvalidFieldsFormat(format!(
                    "unknown field '{key}' in collection '{}'",
                    collection.name
                ))
            })?;
        let validated = field.field_type.validate(val, key)?;
        set_clauses.push(format!("{} = ?{}", key, params.len() + 1));
        params.push(validated);
    }

    set_clauses.push(format!("updated_at = ?{}", params.len() + 1));
    params.push(now);

    let id_param_idx = params.len() + 1;
    let id_str = id.to_string();

    let sql = format!(
        "UPDATE \"{}\" SET {} WHERE id = ?{id_param_idx}",
        collection.name,
        set_clauses.join(", ")
    );

    let mut all_params: Vec<&dyn rusqlite::types::ToSql> = params
        .iter()
        .map(|v| v as &dyn rusqlite::types::ToSql)
        .collect();
    all_params.push(&id_str as &dyn rusqlite::types::ToSql);

    conn.execute(&sql, all_params.as_slice())?;

    get_record_by_id(conn, collection, id)
}

pub fn delete_record(conn: &Connection, collection: &Collection, id: i64) -> Result<Value> {
    let record = get_record_by_id(conn, collection, id)?;

    let sql = format!("DELETE FROM \"{}\" WHERE id = ?1", collection.name);
    let affected = conn.execute(&sql, [id])?;
    if affected == 0 {
        return Err(LodgeError::RecordNotFound(id));
    }

    Ok(record)
}

pub fn execute_sql(conn: &Connection, sql: &str, collections: &[Collection]) -> Result<Vec<Value>> {
    use std::collections::HashSet;

    let mut stmt = conn
        .prepare(sql)
        .map_err(|e| LodgeError::Sql(e.to_string()))?;
    let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

    let rows = stmt
        .query_map([], |row| row_to_json(row, &column_names))
        .map_err(|e| LodgeError::Sql(e.to_string()))?;

    let bool_field_names: HashSet<String> = collections
        .iter()
        .flat_map(|c| c.fields.iter())
        .filter(|f| f.field_type == FieldType::Bool)
        .map(|f| f.name.clone())
        .collect();

    let mut results = Vec::new();
    for row in rows {
        let mut record = row.map_err(|e| LodgeError::Sql(e.to_string()))?;
        if !bool_field_names.is_empty() {
            if let Value::Object(ref mut map) = record {
                for field_name in &bool_field_names {
                    if let Some(val) = map.get_mut(field_name) {
                        match val {
                            Value::Number(n) if n.as_i64() == Some(0) => *val = Value::Bool(false),
                            Value::Number(n) if n.as_i64() == Some(1) => *val = Value::Bool(true),
                            _ => {}
                        }
                    }
                }
            }
        }
        results.push(record);
    }
    Ok(results)
}

fn get_record_by_id(conn: &Connection, collection: &Collection, id: i64) -> Result<Value> {
    let sql = format!("SELECT * FROM \"{}\" WHERE id = ?1", collection.name);
    let mut stmt = conn.prepare(&sql)?;
    let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();

    let mut record = stmt
        .query_row([id], |row| row_to_json(row, &column_names))
        .map_err(|_| LodgeError::RecordNotFound(id))?;
    fix_bool_fields(&mut record, collection);
    Ok(record)
}

pub fn sqlite_to_json(val: rusqlite::types::Value) -> Value {
    match val {
        rusqlite::types::Value::Null => Value::Null,
        rusqlite::types::Value::Integer(i) => json!(i),
        rusqlite::types::Value::Real(f) => json!(f),
        rusqlite::types::Value::Text(s) => json!(s),
        rusqlite::types::Value::Blob(b) => json!(format!("<blob {} bytes>", b.len())),
    }
}

/// Convert a SQLite row into a JSON object given column names.
pub fn row_to_json(row: &rusqlite::Row, column_names: &[String]) -> rusqlite::Result<Value> {
    let mut obj = serde_json::Map::new();
    for (i, col) in column_names.iter().enumerate() {
        let val: rusqlite::types::Value = row.get(i)?;
        obj.insert(col.clone(), sqlite_to_json(val));
    }
    Ok(Value::Object(obj))
}
