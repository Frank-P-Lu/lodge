use crate::collection::collection_exists;
use crate::error::{LodgeError, Result};
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

struct ViewRow {
    name: String,
    collection: String,
    where_clause: Option<String>,
    sort: Option<String>,
    limit: Option<i64>,
    created_at: String,
    description: Option<String>,
}

impl ViewRow {
    fn from_row(row: &rusqlite::Row) -> rusqlite::Result<Self> {
        Ok(Self {
            name: row.get(0)?,
            collection: row.get(1)?,
            where_clause: row.get(2)?,
            sort: row.get(3)?,
            limit: row.get(4)?,
            created_at: row.get(5)?,
            description: row.get(6)?,
        })
    }

    fn to_json(&self) -> Value {
        json!({
            "name": self.name,
            "collection": self.collection,
            "where": self.where_clause,
            "sort": self.sort,
            "limit": self.limit,
            "created_at": self.created_at,
            "description": self.description,
        })
    }
}

fn view_exists(conn: &Connection, name: &str) -> Result<bool> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM _lodge_views WHERE name = ?1",
        [name],
        |row| row.get(0),
    )?;
    Ok(exists)
}

pub fn create_view(
    conn: &Connection,
    name: &str,
    collection: &str,
    where_clause: Option<&str>,
    sort: Option<&str>,
    limit: Option<i64>,
    description: Option<&str>,
) -> Result<()> {
    // Validate name (same rules as collection names)
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_')
        || name.starts_with(|c: char| c.is_ascii_digit())
        || name.is_empty()
    {
        return Err(LodgeError::InvalidName(name.to_string()));
    }

    if !collection_exists(conn, collection)? {
        return Err(LodgeError::CollectionNotFound(collection.to_string()));
    }

    if view_exists(conn, name)? {
        return Err(LodgeError::ViewExists(name.to_string()));
    }

    let now = crate::types::now_timestamp();
    conn.execute(
        "INSERT INTO _lodge_views (name, collection, where_clause, sort, lim, created_at, description) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
        rusqlite::params![name, collection, where_clause, sort, limit, now, description],
    )?;

    Ok(())
}

pub fn list_views(conn: &Connection) -> Result<Vec<Value>> {
    let mut stmt = conn.prepare(
        "SELECT name, collection, where_clause, sort, lim, created_at, description FROM _lodge_views ORDER BY name",
    )?;
    let rows = stmt.query_map([], |row| ViewRow::from_row(row))?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?.to_json());
    }
    Ok(results)
}

pub fn show_view(conn: &Connection, name: &str) -> Result<Value> {
    let view_row = conn
        .query_row(
            "SELECT name, collection, where_clause, sort, lim, created_at, description FROM _lodge_views WHERE name = ?1",
            [name],
            |row| ViewRow::from_row(row),
        )
        .map_err(|_| LodgeError::ViewNotFound(name.to_string()))?;
    Ok(view_row.to_json())
}

pub fn update_view(
    conn: &Connection,
    name: &str,
    where_clause: Option<&str>,
    sort: Option<&str>,
    limit: Option<i64>,
    description: Option<&str>,
) -> Result<()> {
    if where_clause.is_none() && sort.is_none() && limit.is_none() && description.is_none() {
        return Err(LodgeError::InvalidInput(
            "Nothing to update — provide at least one of --where, --sort, --limit, or --description".to_string(),
        ));
    }

    let mut sets = Vec::new();
    let mut params: Vec<Box<dyn rusqlite::ToSql>> = Vec::new();

    if let Some(w) = where_clause {
        sets.push(format!("where_clause = ?{}", params.len() + 1));
        params.push(Box::new(w.to_string()));
    }
    if let Some(s) = sort {
        sets.push(format!("sort = ?{}", params.len() + 1));
        params.push(Box::new(s.to_string()));
    }
    if let Some(l) = limit {
        sets.push(format!("lim = ?{}", params.len() + 1));
        params.push(Box::new(l));
    }
    if let Some(d) = description {
        sets.push(format!("description = ?{}", params.len() + 1));
        params.push(Box::new(d.to_string()));
    }

    let sql = format!(
        "UPDATE _lodge_views SET {} WHERE name = ?{}",
        sets.join(", "),
        params.len() + 1
    );
    params.push(Box::new(name.to_string()));

    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let affected = conn.execute(&sql, params_refs.as_slice())?;
    if affected == 0 {
        return Err(LodgeError::ViewNotFound(name.to_string()));
    }
    Ok(())
}

pub fn run_view(conn: &Connection, name: &str) -> Result<(String, Vec<Value>)> {
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

    Ok((view.collection, results))
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
