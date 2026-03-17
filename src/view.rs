use crate::error::{LodgeError, Result};
use crate::output::Format;
use crate::record;
use crate::schema;
use rusqlite::Connection;
use serde_json::{json, Value};

struct View {
    collection: String,
    where_clause: Option<String>,
    sort: Option<String>,
    limit: Option<i64>,
}

pub fn create_view(
    conn: &Connection,
    name: &str,
    collection: &str,
    where_clause: Option<&str>,
    sort: Option<&str>,
    limit: Option<i64>,
) -> Result<()> {
    // Validate name (same rules as collection names)
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_')
        || name.starts_with(|c: char| c.is_ascii_digit())
        || name.is_empty()
    {
        return Err(LodgeError::InvalidFieldsFormat(format!(
            "invalid view name '{name}'"
        )));
    }

    // Check collection exists
    let coll_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM _lodge_meta WHERE collection = ?1",
        [collection],
        |row| row.get(0),
    )?;
    if !coll_exists {
        return Err(LodgeError::CollectionNotFound(collection.to_string()));
    }

    // Check view name not taken
    let view_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM _lodge_views WHERE name = ?1",
        [name],
        |row| row.get(0),
    )?;
    if view_exists {
        return Err(LodgeError::ViewExists(name.to_string()));
    }

    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();
    conn.execute(
        "INSERT INTO _lodge_views (name, collection, where_clause, sort, lim, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
        rusqlite::params![name, collection, where_clause, sort, limit, now],
    )?;

    Ok(())
}

pub fn list_views(conn: &Connection) -> Result<Vec<Value>> {
    let mut stmt = conn.prepare(
        "SELECT name, collection, where_clause, sort, lim, created_at FROM _lodge_views ORDER BY name",
    )?;
    let rows = stmt.query_map([], |row| {
        let name: String = row.get(0)?;
        let collection: String = row.get(1)?;
        let where_clause: Option<String> = row.get(2)?;
        let sort: Option<String> = row.get(3)?;
        let limit: Option<i64> = row.get(4)?;
        let created_at: String = row.get(5)?;
        Ok(json!({
            "name": name,
            "collection": collection,
            "where": where_clause,
            "sort": sort,
            "limit": limit,
            "created_at": created_at,
        }))
    })?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?);
    }
    Ok(results)
}

pub fn run_view(conn: &Connection, name: &str, format: &Format) -> Result<String> {
    let view = load_view(conn, name)?;
    let coll = schema::load_collection(conn, &view.collection)?
        .ok_or_else(|| LodgeError::CollectionNotFound(view.collection.clone()))?;

    let results = record::query_records(
        conn,
        &coll,
        view.where_clause.as_deref(),
        view.sort.as_deref(),
        view.limit,
    )?;

    Ok(crate::output::format_output(&results, format))
}

pub fn delete_view(conn: &Connection, name: &str) -> Result<()> {
    let affected = conn.execute("DELETE FROM _lodge_views WHERE name = ?1", [name])?;
    if affected == 0 {
        return Err(LodgeError::ViewNotFound(name.to_string()));
    }
    Ok(())
}

pub fn load_view_names(conn: &Connection) -> Result<Vec<String>> {
    let mut stmt = conn.prepare("SELECT name FROM _lodge_views ORDER BY name")?;
    let rows = stmt.query_map([], |row| row.get::<_, String>(0))?;
    let mut names = Vec::new();
    for row in rows {
        names.push(row?);
    }
    Ok(names)
}

fn load_view(conn: &Connection, name: &str) -> Result<View> {
    conn.query_row(
        "SELECT name, collection, where_clause, sort, lim, created_at FROM _lodge_views WHERE name = ?1",
        [name],
        |row| {
            Ok(View {
                collection: row.get(1)?,
                where_clause: row.get(2)?,
                sort: row.get(3)?,
                limit: row.get(4)?,
            })
        },
    )
    .map_err(|_| LodgeError::ViewNotFound(name.to_string()))
}
