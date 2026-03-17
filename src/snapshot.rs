use crate::error::{LodgeError, Result};
use crate::schema;
use crate::types::FieldType;
use rusqlite::Connection;
use serde_json::{json, Value};
use std::path::{Path, PathBuf};

pub fn create_snapshot(conn: &Connection, lodge_dir: &Path, output_path: Option<&str>) -> Result<PathBuf> {
    let collections = schema::load_collections(conn)?;

    let mut colls_json = serde_json::Map::new();
    for coll in &collections {
        let fields_json: Vec<Value> = coll.fields.iter().enumerate().map(|(i, f)| {
            json!({"name": f.name, "type": f.field_type.as_str(), "order": i})
        }).collect();

        // Query all records
        let mut stmt = conn.prepare(&format!("SELECT * FROM \"{}\" ORDER BY id", coll.name))
            .map_err(|e| LodgeError::Snapshot(e.to_string()))?;
        let column_names: Vec<String> = stmt.column_names().iter().map(|s| s.to_string()).collect();
        let rows = stmt.query_map([], |row| {
            let mut obj = serde_json::Map::new();
            for (i, col) in column_names.iter().enumerate() {
                let val: rusqlite::types::Value = row.get(i)?;
                let json_val = match val {
                    rusqlite::types::Value::Null => Value::Null,
                    rusqlite::types::Value::Integer(n) => json!(n),
                    rusqlite::types::Value::Real(f) => json!(f),
                    rusqlite::types::Value::Text(s) => json!(s),
                    rusqlite::types::Value::Blob(b) => json!(format!("<blob {} bytes>", b.len())),
                };
                obj.insert(col.clone(), json_val);
            }
            Ok(Value::Object(obj))
        }).map_err(|e| LodgeError::Snapshot(e.to_string()))?;

        let mut records = Vec::new();
        for row in rows {
            records.push(row.map_err(|e| LodgeError::Snapshot(e.to_string()))?);
        }

        // Check for FTS fields
        let fts_fields = load_fts_fields_for_snapshot(conn, &coll.name);

        let mut coll_obj = serde_json::Map::new();
        coll_obj.insert("fields".to_string(), json!(fields_json));
        coll_obj.insert("records".to_string(), json!(records));
        if !fts_fields.is_empty() {
            coll_obj.insert("fts_fields".to_string(), json!(fts_fields));
        }

        colls_json.insert(coll.name.clone(), Value::Object(coll_obj));
    }

    let snapshot = json!({
        "lodge_version": 1,
        "created_at": chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string(),
        "collections": colls_json,
    });

    let dest = if let Some(p) = output_path {
        PathBuf::from(p)
    } else {
        let snapshots_dir = lodge_dir.join("snapshots");
        std::fs::create_dir_all(&snapshots_dir)?;
        let ts = chrono::Utc::now().format("%Y%m%d_%H%M%S").to_string();
        snapshots_dir.join(format!("{ts}.json"))
    };

    std::fs::write(&dest, serde_json::to_string_pretty(&snapshot).map_err(|e| LodgeError::Snapshot(e.to_string()))?)?;
    Ok(dest)
}

pub fn restore_snapshot(conn: &Connection, path: &str) -> Result<()> {
    let data = std::fs::read_to_string(path)
        .map_err(|e| LodgeError::Snapshot(format!("cannot read snapshot file: {e}")))?;
    let snapshot: Value = serde_json::from_str(&data)
        .map_err(|e| LodgeError::InvalidSnapshot(format!("invalid JSON: {e}")))?;

    let collections = snapshot.get("collections")
        .and_then(|v| v.as_object())
        .ok_or_else(|| LodgeError::InvalidSnapshot("missing 'collections' object".to_string()))?;

    // Validate structure before making any changes
    for (name, coll_data) in collections {
        let fields = coll_data.get("fields")
            .and_then(|v| v.as_array())
            .ok_or_else(|| LodgeError::InvalidSnapshot(format!("collection '{name}' missing 'fields' array")))?;
        for field in fields {
            field.get("name").and_then(|v| v.as_str())
                .ok_or_else(|| LodgeError::InvalidSnapshot(format!("field missing 'name' in collection '{name}'")))?;
            let type_str = field.get("type").and_then(|v| v.as_str())
                .ok_or_else(|| LodgeError::InvalidSnapshot(format!("field missing 'type' in collection '{name}'")))?;
            FieldType::from_str(type_str)?;
        }
        coll_data.get("records")
            .and_then(|v| v.as_array())
            .ok_or_else(|| LodgeError::InvalidSnapshot(format!("collection '{name}' missing 'records' array")))?;
    }

    // Drop existing collections and meta rows inside a transaction
    conn.execute_batch("BEGIN TRANSACTION;")?;

    // Get existing collection names
    let existing: Vec<String> = {
        let mut stmt = conn.prepare("SELECT DISTINCT collection FROM _lodge_meta")?;
        let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
        let mut names = Vec::new();
        for row in rows { names.push(row?); }
        names
    };

    for name in &existing {
        // Drop FTS artifacts if they exist
        let _ = conn.execute_batch(&format!(
            "DROP TRIGGER IF EXISTS \"{name}_fts_ai\";
             DROP TRIGGER IF EXISTS \"{name}_fts_ad\";
             DROP TRIGGER IF EXISTS \"{name}_fts_au\";
             DROP TABLE IF EXISTS \"{name}_fts\";"
        ));
        conn.execute_batch(&format!("DROP TABLE IF EXISTS \"{name}\";"))?;
    }
    conn.execute_batch("DELETE FROM _lodge_meta;")?;
    // Clear FTS meta if it exists
    let _ = conn.execute_batch("DELETE FROM _lodge_fts_meta;");

    // Recreate collections
    for (name, coll_data) in collections {
        let fields = coll_data["fields"].as_array().unwrap();
        let records = coll_data["records"].as_array().unwrap();

        // Build CREATE TABLE
        let mut col_defs = vec!["id INTEGER PRIMARY KEY AUTOINCREMENT".to_string()];
        for field in fields {
            let fname = field["name"].as_str().unwrap();
            let ftype = FieldType::from_str(field["type"].as_str().unwrap())?;
            col_defs.push(format!("{fname} {}", ftype.sql_type()));
        }
        col_defs.push("created_at TEXT NOT NULL".to_string());
        col_defs.push("updated_at TEXT NOT NULL".to_string());

        conn.execute_batch(&format!("CREATE TABLE \"{}\" ({});", name, col_defs.join(", ")))?;

        // Insert meta rows
        for field in fields {
            let fname = field["name"].as_str().unwrap();
            let ftype = field["type"].as_str().unwrap();
            let order = field.get("order").and_then(|v| v.as_i64()).unwrap_or(0);
            conn.execute(
                "INSERT INTO _lodge_meta (collection, field_name, field_type, field_order) VALUES (?1, ?2, ?3, ?4)",
                rusqlite::params![name, fname, ftype, order as i32],
            )?;
        }

        // Insert records
        for record in records {
            let obj = record.as_object()
                .ok_or_else(|| LodgeError::InvalidSnapshot("record is not an object".to_string()))?;

            // Collect field names (excluding id which is auto-assigned)
            let mut ins_names = Vec::new();
            let mut ins_values: Vec<String> = Vec::new();

            for field in fields {
                let fname = field["name"].as_str().unwrap();
                if let Some(val) = obj.get(fname) {
                    ins_names.push(fname.to_string());
                    ins_values.push(json_value_to_sql_string(val));
                }
            }
            // Include auto-managed columns
            if let Some(val) = obj.get("created_at") {
                ins_names.push("created_at".to_string());
                ins_values.push(json_value_to_sql_string(val));
            }
            if let Some(val) = obj.get("updated_at") {
                ins_names.push("updated_at".to_string());
                ins_values.push(json_value_to_sql_string(val));
            }

            if !ins_names.is_empty() {
                let placeholders: Vec<String> = (1..=ins_names.len()).map(|i| format!("?{i}")).collect();
                let sql = format!(
                    "INSERT INTO \"{}\" ({}) VALUES ({})",
                    name, ins_names.join(", "), placeholders.join(", ")
                );
                let params: Vec<&dyn rusqlite::types::ToSql> = ins_values
                    .iter()
                    .map(|v| v as &dyn rusqlite::types::ToSql)
                    .collect();
                conn.execute(&sql, params.as_slice())?;
            }
        }

        // Restore FTS if snapshot includes fts_fields
        if let Some(fts_fields) = coll_data.get("fts_fields").and_then(|v| v.as_array()) {
            let field_names: Vec<String> = fts_fields.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_string()))
                .collect();
            if !field_names.is_empty() {
                crate::fts::create_fts_table(conn, name, &field_names)?;
            }
        }
    }

    conn.execute_batch("COMMIT;")?;
    Ok(())
}

fn json_value_to_sql_string(val: &Value) -> String {
    match val {
        Value::Null => "".to_string(), // shouldn't happen for non-null fields
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => if *b { "1".to_string() } else { "0".to_string() },
        other => other.to_string(),
    }
}

fn load_fts_fields_for_snapshot(conn: &Connection, collection: &str) -> Vec<String> {
    let mut stmt = match conn.prepare(
        "SELECT field_name FROM _lodge_fts_meta WHERE collection = ?1 ORDER BY field_name"
    ) {
        Ok(s) => s,
        Err(_) => return Vec::new(),
    };
    let rows = match stmt.query_map([collection], |row| row.get::<_, String>(0)) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };
    rows.filter_map(|r| r.ok()).collect()
}
