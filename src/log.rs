use crate::error::Result;
use rusqlite::Connection;
use serde_json::Value;

fn build_summary(
    operation: &str,
    collection: &str,
    record_id: Option<i64>,
    success: bool,
    error: Option<&str>,
    before_data: Option<&str>,
    after_data: Option<&str>,
) -> String {
    if !success {
        let err_snippet = error
            .unwrap_or("unknown error")
            .chars()
            .take(80)
            .collect::<String>();
        return format!("failed {operation} on {collection}: {err_snippet}");
    }

    match operation {
        "add" => {
            let label = after_data
                .and_then(|s| serde_json::from_str::<Value>(s).ok())
                .and_then(|v| first_text_value(&v));
            match label {
                Some(text) => format!("added {collection}: {text}"),
                None => match record_id {
                    Some(id) => format!("added {collection} #{id}"),
                    None => format!("added {collection}"),
                },
            }
        }
        "update" => {
            let id_str = record_id
                .map(|id| format!(" #{id}"))
                .unwrap_or_default();
            let changes = diff_fields(before_data, after_data);
            if changes.is_empty() {
                format!("updated {collection}{id_str}")
            } else {
                format!("updated {collection}{id_str}: {changes}")
            }
        }
        "delete" => {
            let id_str = record_id
                .map(|id| format!(" #{id}"))
                .unwrap_or_default();
            let label = before_data
                .and_then(|s| serde_json::from_str::<Value>(s).ok())
                .and_then(|v| first_text_value(&v));
            match label {
                Some(text) => format!("deleted {collection}{id_str}: {text}"),
                None => format!("deleted {collection}{id_str}"),
            }
        }
        _ => format!("{operation} {collection}"),
    }
}

fn first_text_value(v: &Value) -> Option<String> {
    let obj = v.as_object()?;
    // Skip auto-managed columns, find first string value
    for (key, val) in obj {
        if matches!(key.as_str(), "id" | "created_at" | "updated_at") {
            continue;
        }
        if let Some(s) = val.as_str() {
            // Skip date/datetime-looking values
            if s.len() == 10 && s.chars().nth(4) == Some('-') {
                continue;
            }
            if s.contains('T') && s.contains(':') {
                continue;
            }
            return Some(s.to_string());
        }
    }
    None
}

fn diff_fields(before_data: Option<&str>, after_data: Option<&str>) -> String {
    let before: Value = before_data
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(Value::Null);
    let after: Value = after_data
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(Value::Null);

    let before_obj = before.as_object();
    let after_obj = after.as_object();

    if before_obj.is_none() || after_obj.is_none() {
        return String::new();
    }
    let before_obj = before_obj.unwrap();
    let after_obj = after_obj.unwrap();

    let mut changes = Vec::new();
    for (key, new_val) in after_obj {
        if matches!(key.as_str(), "id" | "created_at" | "updated_at") {
            continue;
        }
        if let Some(old_val) = before_obj.get(key) {
            if old_val != new_val {
                let old_display = format_val(old_val);
                let new_display = format_val(new_val);
                changes.push(format!("{key} {old_display}\u{2192}{new_display}"));
            }
        }
        if changes.len() >= 3 {
            break;
        }
    }

    changes.join(", ")
}

fn format_val(v: &Value) -> String {
    match v {
        Value::String(s) => s.clone(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::Null => "null".to_string(),
        _ => v.to_string(),
    }
}

pub fn query_log(
    conn: &Connection,
    collection: Option<&str>,
    limit: i64,
    verbose: bool,
    since: Option<&str>,
) -> Result<Vec<Value>> {
    let mut conditions = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::types::ToSql>> = Vec::new();
    let mut idx = 1;

    if let Some(c) = collection {
        conditions.push(format!("collection = ?{idx}"));
        params.push(Box::new(c.to_string()));
        idx += 1;
    }

    if let Some(s) = since {
        // Expand date-only to datetime for inclusive comparison
        let since_ts = if s.contains('T') {
            s.to_string()
        } else {
            format!("{s}T00:00:00")
        };
        conditions.push(format!("timestamp >= ?{idx}"));
        params.push(Box::new(since_ts));
        idx += 1;
    }

    let where_clause = if conditions.is_empty() {
        String::new()
    } else {
        format!(" WHERE {}", conditions.join(" AND "))
    };

    let sql = format!(
        "SELECT id, timestamp, collection, operation, record_id, success, error, before_data, after_data \
         FROM _lodge_log{where_clause} ORDER BY id DESC LIMIT ?{idx}"
    );
    params.push(Box::new(limit));
    let _ = idx;

    let param_refs: Vec<&dyn rusqlite::types::ToSql> = params.iter().map(|p| p.as_ref()).collect();

    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map(param_refs.as_slice(), |row| {
        let id: i64 = row.get(0)?;
        let timestamp: String = row.get(1)?;
        let collection: String = row.get(2)?;
        let operation: String = row.get(3)?;
        let record_id: Option<i64> = row.get(4)?;
        let success: i64 = row.get(5)?;
        let error: Option<String> = row.get(6)?;
        let before_data: Option<String> = row.get(7)?;
        let after_data: Option<String> = row.get(8)?;

        let summary = build_summary(
            &operation,
            &collection,
            record_id,
            success == 1,
            error.as_deref(),
            before_data.as_deref(),
            after_data.as_deref(),
        );

        let mut obj = serde_json::Map::new();
        obj.insert("id".to_string(), serde_json::json!(id));
        obj.insert("timestamp".to_string(), serde_json::json!(timestamp));
        obj.insert("collection".to_string(), serde_json::json!(collection));
        obj.insert("operation".to_string(), serde_json::json!(operation));
        obj.insert(
            "record_id".to_string(),
            match record_id {
                Some(rid) => serde_json::json!(rid),
                None => Value::Null,
            },
        );
        obj.insert("success".to_string(), serde_json::json!(success == 1));
        obj.insert("summary".to_string(), serde_json::json!(summary));

        if let Some(err) = error {
            obj.insert("error".to_string(), serde_json::json!(err));
        }

        if verbose {
            obj.insert(
                "before".to_string(),
                match before_data {
                    Some(ref s) => serde_json::from_str(s).unwrap_or(Value::Null),
                    None => Value::Null,
                },
            );
            obj.insert(
                "after".to_string(),
                match after_data {
                    Some(ref s) => serde_json::from_str(s).unwrap_or(Value::Null),
                    None => Value::Null,
                },
            );
        }

        Ok(Value::Object(obj))
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}
