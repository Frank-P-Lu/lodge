use crate::error::LodgeError;
use crate::record;
use crate::schema::{self, Collection};
use rusqlite::Connection;
use serde_json::Value;

pub fn import_collection(conn: &Connection, name: &str, data: &str) -> crate::error::Result<usize> {
    let coll = schema::load_collection(conn, name)?
        .ok_or_else(|| LodgeError::CollectionNotFound(name.to_string()))?;

    // Try parsing as JSON
    if let Ok(parsed) = serde_json::from_str::<Value>(data) {
        return import_json_records(conn, &coll, &parsed);
    }

    // Try CSV
    import_csv_records(conn, &coll, data)
}

pub fn import_full(conn: &Connection, data: &str) -> crate::error::Result<Vec<(String, usize)>> {
    let parsed: Value = serde_json::from_str(data)
        .map_err(|e| LodgeError::ImportError(e.to_string()))?;

    let collections = parsed
        .get("collections")
        .and_then(|c| c.as_array())
        .ok_or_else(|| {
            LodgeError::ImportError(
                "expected 'collections' array in export envelope".to_string(),
            )
        })?;

    let mut results = Vec::new();

    for coll_data in collections {
        let coll_name = coll_data
            .get("collection")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                LodgeError::ImportError("missing 'collection' name".to_string())
            })?;

        // Create collection if it doesn't exist
        let coll = if let Some(existing) = schema::load_collection(conn, coll_name)? {
            existing
        } else {
            let fields = coll_data
                .get("fields")
                .and_then(|f| f.as_array())
                .ok_or_else(|| {
                    LodgeError::ImportError(format!(
                        "missing 'fields' for collection '{coll_name}'"
                    ))
                })?;
            let fields_spec: Vec<String> = fields
                .iter()
                .filter_map(|f| {
                    let name = f.get("name")?.as_str()?;
                    let ftype = f.get("type")?.as_str()?;
                    Some(format!("{name}:{ftype}"))
                })
                .collect();
            if fields_spec.is_empty() {
                return Err(LodgeError::ImportError(format!(
                    "no valid fields for collection '{coll_name}'"
                )));
            }
            crate::collection::create_collection(conn, coll_name, &fields_spec.join(", "))?;
            schema::load_collection(conn, coll_name)?.ok_or_else(|| {
                LodgeError::ImportError(format!(
                    "failed to load collection '{coll_name}' after creation"
                ))
            })?
        };

        let records = coll_data
            .get("records")
            .and_then(|r| r.as_array())
            .ok_or_else(|| {
                LodgeError::ImportError(format!(
                    "missing 'records' for collection '{coll_name}'"
                ))
            })?;

        let count = import_json_array(conn, &coll, records)?;
        results.push((coll_name.to_string(), count));
    }

    Ok(results)
}

fn import_json_records(conn: &Connection, coll: &Collection, parsed: &Value) -> crate::error::Result<usize> {
    // Accept either an array directly or an envelope with "records" key
    let records = if let Some(arr) = parsed.as_array() {
        arr.clone()
    } else if let Some(arr) = parsed.get("records").and_then(|r| r.as_array()) {
        arr.clone()
    } else {
        return Err(LodgeError::ImportError(
            "expected JSON array or object with 'records' array".to_string(),
        ));
    };

    import_json_array(conn, coll, &records)
}

fn import_json_array(conn: &Connection, coll: &Collection, records: &[Value]) -> crate::error::Result<usize> {
    let mut count = 0;
    for record in records {
        let obj = record.as_object().ok_or_else(|| {
            LodgeError::ImportError("each record must be a JSON object".to_string())
        })?;

        let mut values = Vec::new();
        for field in &coll.fields {
            if let Some(val) = obj.get(&field.name) {
                if !val.is_null() {
                    let val_str = match val {
                        Value::String(s) => s.clone(),
                        other => other.to_string(),
                    };
                    values.push((field.name.clone(), val_str));
                }
            }
        }

        record::add_record(conn, coll, &values)?;
        count += 1;
    }
    Ok(count)
}

fn parse_csv_row(input: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut current = String::new();
    let mut in_quotes = false;
    let mut chars = input.chars().peekable();

    while let Some(ch) = chars.next() {
        if in_quotes {
            if ch == '"' {
                if chars.peek() == Some(&'"') {
                    // Escaped quote ("") -> literal "
                    chars.next();
                    current.push('"');
                } else {
                    // End of quoted field
                    in_quotes = false;
                }
            } else {
                current.push(ch);
            }
        } else {
            match ch {
                ',' => {
                    fields.push(current.trim().to_string());
                    current = String::new();
                }
                '"' => {
                    in_quotes = true;
                }
                _ => {
                    current.push(ch);
                }
            }
        }
    }
    fields.push(current.trim().to_string());
    fields
}

fn import_csv_records(conn: &Connection, coll: &Collection, data: &str) -> crate::error::Result<usize> {
    let mut lines = data.lines();
    let header_line = lines
        .next()
        .ok_or_else(|| LodgeError::ImportError("CSV file is empty".to_string()))?;
    let headers = parse_csv_row(header_line);

    let mut count = 0;
    for line in lines {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let values_raw = parse_csv_row(line);
        let mut values = Vec::new();
        for (i, header) in headers.iter().enumerate() {
            // Skip auto-managed columns
            if matches!(header.as_str(), "id" | "created_at" | "updated_at") {
                continue;
            }
            if let Some(val) = values_raw.get(i) {
                if !val.is_empty() {
                    values.push((header.to_string(), val.to_string()));
                }
            }
        }
        record::add_record(conn, coll, &values)?;
        count += 1;
    }
    Ok(count)
}
