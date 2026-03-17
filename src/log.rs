use crate::error::Result;
use rusqlite::Connection;
use serde_json::Value;

pub fn query_log(
    conn: &Connection,
    collection: Option<&str>,
    limit: i64,
) -> Result<Vec<Value>> {
    let (sql, params): (String, Vec<Box<dyn rusqlite::types::ToSql>>) = match collection {
        Some(c) => (
            "SELECT id, timestamp, collection, operation, record_id, success, error, before_data, after_data \
             FROM _lodge_log WHERE collection = ?1 ORDER BY id DESC LIMIT ?2"
                .to_string(),
            vec![Box::new(c.to_string()) as Box<dyn rusqlite::types::ToSql>, Box::new(limit)],
        ),
        None => (
            "SELECT id, timestamp, collection, operation, record_id, success, error, before_data, after_data \
             FROM _lodge_log ORDER BY id DESC LIMIT ?1"
                .to_string(),
            vec![Box::new(limit) as Box<dyn rusqlite::types::ToSql>],
        ),
    };

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

        if let Some(err) = error {
            obj.insert("error".to_string(), serde_json::json!(err));
        }

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

        Ok(Value::Object(obj))
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}
