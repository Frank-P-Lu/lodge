use crate::error::{LodgeError, Result};
use crate::types::parse_fields;
use rusqlite::Connection;

const RESERVED_NAMES: &[&str] = &[
    "init", "create", "alter", "drop", "sql", "help", "guide", "view", "export", "import",
    "snapshot", "restore", "run", "list", "log", "set",
];

/// Check whether a collection exists in the database.
pub fn collection_exists(conn: &Connection, name: &str) -> Result<bool> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM _lodge_meta WHERE collection = ?1",
        [name],
        |row| row.get(0),
    )?;
    Ok(exists)
}

pub fn create_collection(conn: &Connection, name: &str, fields_spec: &str) -> Result<()> {
    // Validate name
    if RESERVED_NAMES.contains(&name) {
        return Err(LodgeError::ReservedName(name.to_string()));
    }
    if !name.chars().all(|c| c.is_alphanumeric() || c == '_')
        || name.starts_with(|c: char| c.is_ascii_digit())
        || name.is_empty()
    {
        return Err(LodgeError::InvalidName(name.to_string()));
    }

    if collection_exists(conn, name)? {
        return Err(LodgeError::CollectionExists(name.to_string()));
    }

    let fields = parse_fields(fields_spec)?;

    // Build CREATE TABLE
    let mut col_defs = vec!["id INTEGER PRIMARY KEY AUTOINCREMENT".to_string()];
    for (field_name, field_type) in &fields {
        col_defs.push(format!("{} {}", field_name, field_type.sql_type()));
    }
    col_defs.push("created_at TEXT NOT NULL".to_string());
    col_defs.push("updated_at TEXT NOT NULL".to_string());

    let create_sql = format!("CREATE TABLE \"{}\" ({})", name, col_defs.join(", "));
    conn.execute(&create_sql, [])?;

    // Insert metadata
    for (i, (field_name, field_type)) in fields.iter().enumerate() {
        conn.execute(
            "INSERT INTO _lodge_meta (collection, field_name, field_type, field_order) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![name, field_name, field_type.as_str(), i as i32],
        )?;
    }

    Ok(())
}

const PROTECTED_FIELDS: &[&str] = &["id", "created_at", "updated_at"];

pub fn alter_collection(conn: &Connection, name: &str, add_fields_spec: &str) -> Result<()> {
    validate_collection_exists(conn, name)?;

    let new_fields = parse_fields(add_fields_spec)?;

    // Get current max order
    let max_order: i32 = conn.query_row(
        "SELECT COALESCE(MAX(field_order), -1) FROM _lodge_meta WHERE collection = ?1",
        [name],
        |row| row.get(0),
    )?;

    for (i, (field_name, field_type)) in new_fields.iter().enumerate() {
        // Check field doesn't already exist
        let field_exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM _lodge_meta WHERE collection = ?1 AND field_name = ?2",
            rusqlite::params![name, field_name],
            |row| row.get(0),
        )?;
        if field_exists {
            return Err(LodgeError::InvalidFieldsFormat(format!(
                "field '{field_name}' already exists in collection '{name}'"
            )));
        }

        // ALTER TABLE
        let alter_sql = format!(
            "ALTER TABLE \"{}\" ADD COLUMN {} {}",
            name,
            field_name,
            field_type.sql_type()
        );
        conn.execute(&alter_sql, [])?;

        // Insert metadata
        conn.execute(
            "INSERT INTO _lodge_meta (collection, field_name, field_type, field_order) VALUES (?1, ?2, ?3, ?4)",
            rusqlite::params![name, field_name, field_type.as_str(), max_order + 1 + i as i32],
        )?;
    }

    Ok(())
}

fn validate_collection_exists(conn: &Connection, name: &str) -> Result<()> {
    if !collection_exists(conn, name)? {
        return Err(LodgeError::CollectionNotFound(name.to_string()));
    }
    Ok(())
}

fn validate_field_exists(conn: &Connection, collection: &str, field: &str) -> Result<()> {
    let exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM _lodge_meta WHERE collection = ?1 AND field_name = ?2",
        rusqlite::params![collection, field],
        |row| row.get(0),
    )?;
    if !exists {
        return Err(LodgeError::FieldNotFound {
            field: field.to_string(),
            collection: collection.to_string(),
        });
    }
    Ok(())
}

fn validate_not_protected(field: &str) -> Result<()> {
    if PROTECTED_FIELDS.contains(&field) {
        return Err(LodgeError::ProtectedField(field.to_string()));
    }
    Ok(())
}

pub fn rename_field(
    conn: &Connection,
    collection: &str,
    old_name: &str,
    new_name: &str,
) -> Result<()> {
    validate_collection_exists(conn, collection)?;
    validate_not_protected(old_name)?;
    validate_field_exists(conn, collection, old_name)?;

    // Validate new name is a valid identifier
    if !new_name.chars().all(|c| c.is_alphanumeric() || c == '_')
        || new_name.starts_with(|c: char| c.is_ascii_digit())
        || new_name.is_empty()
    {
        return Err(LodgeError::InvalidName(new_name.to_string()));
    }

    // Check new name doesn't already exist (including auto-managed columns)
    if PROTECTED_FIELDS.contains(&new_name) {
        return Err(LodgeError::InvalidFieldsFormat(format!(
            "field '{}' already exists in collection '{}'",
            new_name, collection
        )));
    }
    let new_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM _lodge_meta WHERE collection = ?1 AND field_name = ?2",
        rusqlite::params![collection, new_name],
        |row| row.get(0),
    )?;
    if new_exists {
        return Err(LodgeError::InvalidFieldsFormat(format!(
            "field '{}' already exists in collection '{}'",
            new_name, collection
        )));
    }

    conn.execute(
        &format!(
            "ALTER TABLE \"{}\" RENAME COLUMN \"{}\" TO \"{}\"",
            collection, old_name, new_name
        ),
        [],
    )?;
    conn.execute(
        "UPDATE _lodge_meta SET field_name = ?1 WHERE collection = ?2 AND field_name = ?3",
        rusqlite::params![new_name, collection, old_name],
    )?;
    // Update FTS meta if it exists
    let fts_meta_exists: bool = conn.query_row(
        "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='_lodge_fts_meta'",
        [],
        |row| row.get(0),
    )?;
    if fts_meta_exists {
        conn.execute(
            "UPDATE _lodge_fts_meta SET field_name = ?1 WHERE collection = ?2 AND field_name = ?3",
            rusqlite::params![new_name, collection, old_name],
        )?;
    }

    Ok(())
}

pub fn drop_collection(conn: &Connection, name: &str) -> Result<()> {
    validate_collection_exists(conn, name)?;

    // Remove metadata
    conn.execute(
        "DELETE FROM _lodge_meta WHERE collection = ?1",
        rusqlite::params![name],
    )?;

    // Remove FTS table if it exists
    crate::fts::drop_fts_table(conn, name)?;

    // Remove views for this collection
    conn.execute(
        "DELETE FROM _lodge_views WHERE collection = ?1",
        rusqlite::params![name],
    )?;

    // Remove query log entries
    conn.execute(
        "DELETE FROM _lodge_query_log WHERE collection = ?1",
        rusqlite::params![name],
    )?;

    // Drop the collection table (mutation log is preserved)
    conn.execute(&format!("DROP TABLE \"{}\"", name), [])?;

    Ok(())
}

pub fn drop_fields(conn: &Connection, collection: &str, field_names: &[String]) -> Result<()> {
    validate_collection_exists(conn, collection)?;

    for field in field_names {
        validate_not_protected(field)?;
        validate_field_exists(conn, collection, field)?;
    }

    for field in field_names {
        conn.execute(
            &format!("ALTER TABLE \"{}\" DROP COLUMN \"{}\"", collection, field),
            [],
        )?;
        conn.execute(
            "DELETE FROM _lodge_meta WHERE collection = ?1 AND field_name = ?2",
            rusqlite::params![collection, field],
        )?;
        // Clean up FTS meta if it exists
        let fts_meta_exists: bool = conn.query_row(
            "SELECT COUNT(*) > 0 FROM sqlite_master WHERE type='table' AND name='_lodge_fts_meta'",
            [],
            |row| row.get(0),
        )?;
        if fts_meta_exists {
            conn.execute(
                "DELETE FROM _lodge_fts_meta WHERE collection = ?1 AND field_name = ?2",
                rusqlite::params![collection, field],
            )?;
        }
    }

    Ok(())
}
