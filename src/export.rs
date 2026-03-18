use crate::error::Result;
use crate::output::{self, Format};
use crate::record;
use crate::schema;
use rusqlite::Connection;
use serde_json::{json, Value};

pub fn export_collection(conn: &Connection, name: &str, format: &Format) -> Result<String> {
    let coll = schema::load_collection(conn, name)?
        .ok_or_else(|| crate::error::LodgeError::CollectionNotFound(name.to_string()))?;

    let records = record::query_records(conn, &coll, None, None, None)?;

    match format {
        Format::Csv => output::format_output(&records, format),
        _ => {
            let fields: Vec<Value> = coll
                .fields
                .iter()
                .map(|f| json!({"name": f.name, "type": f.field_type.as_str()}))
                .collect();
            let envelope = json!({
                "collection": name,
                "fields": fields,
                "records": records,
            });
            serde_json::to_string_pretty(&envelope).map_err(|e| {
                crate::error::LodgeError::Sql(format!("JSON serialization failed: {e}"))
            })
        }
    }
}

pub fn export_all(conn: &Connection) -> Result<String> {
    let collections = schema::load_collections(conn)?;
    let mut coll_exports = Vec::new();

    for coll in &collections {
        let records = record::query_records(conn, coll, None, None, None)?;
        let fields: Vec<Value> = coll
            .fields
            .iter()
            .map(|f| json!({"name": f.name, "type": f.field_type.as_str()}))
            .collect();
        coll_exports.push(json!({
            "collection": coll.name,
            "fields": fields,
            "records": records,
        }));
    }

    let envelope = json!({
        "lodge_export": true,
        "collections": coll_exports,
    });
    serde_json::to_string_pretty(&envelope)
        .map_err(|e| crate::error::LodgeError::Sql(format!("JSON serialization failed: {e}")))
}
