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
    sql: Option<String>,
}

struct ViewRow {
    name: String,
    collection: String,
    where_clause: Option<String>,
    sort: Option<String>,
    limit: Option<i64>,
    created_at: String,
    description: Option<String>,
    sql: Option<String>,
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
            sql: row.get(7)?,
        })
    }

    fn to_json(&self) -> Value {
        let collection_val = if self.collection.is_empty() {
            Value::Null
        } else {
            Value::String(self.collection.clone())
        };
        json!({
            "name": self.name,
            "collection": collection_val,
            "where": self.where_clause,
            "sort": self.sort,
            "limit": self.limit,
            "created_at": self.created_at,
            "description": self.description,
            "sql": self.sql,
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

fn validate_view_name(name: &str) -> Result<()> {
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_')
        || name.starts_with(|c: char| c.is_ascii_digit())
        || name.is_empty()
    {
        return Err(LodgeError::InvalidName(name.to_string()));
    }
    Ok(())
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
    validate_view_name(name)?;

    if !collection_exists(conn, collection)? {
        return Err(LodgeError::CollectionNotFound(collection.to_string()));
    }

    if view_exists(conn, name)? {
        return Err(LodgeError::ViewExists(name.to_string()));
    }

    let now = crate::types::now_timestamp();
    conn.execute(
        "INSERT INTO _lodge_views (name, collection, where_clause, sort, lim, created_at, description, sql) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, NULL)",
        rusqlite::params![name, collection, where_clause, sort, limit, now, description],
    )?;

    Ok(())
}

pub fn create_sql_view(
    conn: &Connection,
    name: &str,
    sql: &str,
    description: Option<&str>,
) -> Result<()> {
    validate_view_name(name)?;

    if view_exists(conn, name)? {
        return Err(LodgeError::ViewExists(name.to_string()));
    }

    // Validate SQL is read-only
    let trimmed = sql.trim_start();
    if !trimmed.starts_with("SELECT") && !trimmed.starts_with("select")
        && !trimmed.starts_with("Select")
        && !trimmed.starts_with("WITH") && !trimmed.starts_with("with")
        && !trimmed.starts_with("With")
    {
        return Err(LodgeError::InvalidInput(
            "SQL views must start with SELECT or WITH".to_string(),
        ));
    }

    // Syntax-check via prepare
    conn.prepare(sql)
        .map_err(|e| LodgeError::Sql(e.to_string()))?;

    let now = crate::types::now_timestamp();
    conn.execute(
        "INSERT INTO _lodge_views (name, collection, where_clause, sort, lim, created_at, description, sql) VALUES (?1, '', NULL, NULL, NULL, ?2, ?3, ?4)",
        rusqlite::params![name, now, description, sql],
    )?;

    Ok(())
}

pub fn list_views(conn: &Connection) -> Result<Vec<Value>> {
    let mut stmt = conn.prepare(
        "SELECT name, collection, where_clause, sort, lim, created_at, description, sql FROM _lodge_views ORDER BY name",
    )?;
    let rows = stmt.query_map([], ViewRow::from_row)?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row?.to_json());
    }
    Ok(results)
}

pub fn show_view(conn: &Connection, name: &str) -> Result<Value> {
    let view_row = conn
        .query_row(
            "SELECT name, collection, where_clause, sort, lim, created_at, description, sql FROM _lodge_views WHERE name = ?1",
            [name],
            ViewRow::from_row,
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
    sql: Option<&str>,
) -> Result<()> {
    if where_clause.is_none() && sort.is_none() && limit.is_none() && description.is_none() && sql.is_none() {
        return Err(LodgeError::InvalidInput(
            "Nothing to update — provide at least one of --where, --sort, --limit, --description, or --sql".to_string(),
        ));
    }

    // Load the existing view to check its type
    let view = load_view(conn, name)?;
    let is_sql_view = view.sql.is_some();

    // SQL views cannot have --where/--sort/--limit set
    if is_sql_view && (where_clause.is_some() || sort.is_some() || limit.is_some()) {
        return Err(LodgeError::InvalidInput(
            "Cannot use --where, --sort, or --limit on an SQL view".to_string(),
        ));
    }

    // Collection views cannot switch to --sql
    if !is_sql_view && sql.is_some() {
        return Err(LodgeError::InvalidInput(
            "Cannot use --sql on a collection view".to_string(),
        ));
    }

    // If updating SQL, validate it
    if let Some(new_sql) = sql {
        let trimmed = new_sql.trim_start();
        if !trimmed.starts_with("SELECT") && !trimmed.starts_with("select")
            && !trimmed.starts_with("Select")
            && !trimmed.starts_with("WITH") && !trimmed.starts_with("with")
            && !trimmed.starts_with("With")
        {
            return Err(LodgeError::InvalidInput(
                "SQL views must start with SELECT or WITH".to_string(),
            ));
        }
        conn.prepare(new_sql)
            .map_err(|e| LodgeError::Sql(e.to_string()))?;
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
    if let Some(s) = sql {
        sets.push(format!("sql = ?{}", params.len() + 1));
        params.push(Box::new(s.to_string()));
    }

    let update_sql = format!(
        "UPDATE _lodge_views SET {} WHERE name = ?{}",
        sets.join(", "),
        params.len() + 1
    );
    params.push(Box::new(name.to_string()));

    let params_refs: Vec<&dyn rusqlite::ToSql> = params.iter().map(|p| p.as_ref()).collect();
    let affected = conn.execute(&update_sql, params_refs.as_slice())?;
    if affected == 0 {
        return Err(LodgeError::ViewNotFound(name.to_string()));
    }
    Ok(())
}

pub fn run_view(conn: &Connection, name: &str) -> Result<(Option<String>, Vec<Value>)> {
    let view = load_view(conn, name)?;

    if let Some(ref sql) = view.sql {
        // SQL view: execute raw SQL
        let collections = schema::load_collections(conn)?;
        let results = record::execute_sql(conn, sql, &collections)?;
        Ok((None, results))
    } else {
        // Collection view: existing path
        let coll = schema::load_collection(conn, &view.collection)?
            .ok_or_else(|| LodgeError::CollectionNotFound(view.collection.clone()))?;

        let results = record::query_records(
            conn,
            &coll,
            view.where_clause.as_deref(),
            view.sort.as_deref(),
            view.limit,
        )?;

        Ok((Some(view.collection), results))
    }
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
        "SELECT name, collection, where_clause, sort, lim, created_at, sql FROM _lodge_views WHERE name = ?1",
        [name],
        |row| {
            Ok(View {
                collection: row.get(1)?,
                where_clause: row.get(2)?,
                sort: row.get(3)?,
                limit: row.get(4)?,
                sql: row.get(6)?,
            })
        },
    )
    .map_err(|_| LodgeError::ViewNotFound(name.to_string()))
}
