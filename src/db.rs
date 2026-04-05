use crate::error::{LodgeError, Result};
use rusqlite::Connection;
use std::path::{Path, PathBuf};

const LODGE_DIR: &str = ".lodge";
const DB_FILE: &str = "lodge.db";

/// Walk up from `start` looking for a `.lodge/` directory.
pub fn find_lodge_dir(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join(LODGE_DIR);
        if candidate.is_dir() {
            return Some(candidate);
        }
        if !current.pop() {
            return None;
        }
    }
}

/// Initialize a new lodge database in the current directory.
pub fn init(dir: &Path) -> Result<()> {
    let lodge_dir = dir.join(LODGE_DIR);
    if lodge_dir.exists() {
        return Err(LodgeError::AlreadyInitialized);
    }
    std::fs::create_dir(&lodge_dir)?;
    let db_path = lodge_dir.join(DB_FILE);
    let conn = Connection::open(&db_path)?;
    create_meta_table(&conn)?;
    create_views_table(&conn)?;
    create_fts_meta_table(&conn)?;
    create_log_table(&conn)?;
    create_query_log_table(&conn)?;
    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

/// Get the lodge directory path, searching up from `start`.
pub fn lodge_dir(start: &Path) -> Result<PathBuf> {
    find_lodge_dir(start).ok_or(LodgeError::NotInitialized)
}

const SCHEMA_VERSION: i64 = 1;

/// Open an existing lodge database, searching up from `start`.
pub fn open(start: &Path) -> Result<Connection> {
    let lodge_dir = find_lodge_dir(start).ok_or(LodgeError::NotInitialized)?;
    let db_path = lodge_dir.join(DB_FILE);
    let conn = Connection::open(db_path)?;
    run_migrations(&conn)?;
    Ok(conn)
}

fn run_migrations(conn: &Connection) -> Result<()> {
    let version: i64 = conn
        .pragma_query_value(None, "user_version", |row| row.get(0))
        .unwrap_or(0);
    if version >= SCHEMA_VERSION {
        return Ok(());
    }
    create_views_table(conn)?;
    migrate_views_description(conn)?;
    migrate_views_sql_column(conn)?;
    create_fts_meta_table(conn)?;
    create_log_table(conn)?;
    create_query_log_table(conn)?;
    conn.pragma_update(None, "user_version", SCHEMA_VERSION)?;
    Ok(())
}

fn create_meta_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _lodge_meta (
            collection TEXT NOT NULL,
            field_name TEXT NOT NULL,
            field_type TEXT NOT NULL,
            field_order INTEGER NOT NULL,
            PRIMARY KEY (collection, field_name)
        );",
    )?;
    Ok(())
}

fn create_fts_meta_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _lodge_fts_meta (
            collection TEXT NOT NULL,
            field_name TEXT NOT NULL,
            PRIMARY KEY (collection, field_name)
        );",
    )?;
    Ok(())
}

fn create_log_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _lodge_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            timestamp TEXT NOT NULL,
            collection TEXT NOT NULL,
            operation TEXT NOT NULL,
            record_id INTEGER,
            success INTEGER NOT NULL DEFAULT 1,
            error TEXT,
            before_data TEXT,
            after_data TEXT
        );",
    )?;
    Ok(())
}

fn create_query_log_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _lodge_query_log (
            id INTEGER PRIMARY KEY AUTOINCREMENT,
            query_type TEXT NOT NULL,
            collection TEXT NOT NULL,
            fingerprint TEXT NOT NULL UNIQUE,
            call_count INTEGER NOT NULL DEFAULT 1,
            last_used TEXT NOT NULL,
            suggested INTEGER NOT NULL DEFAULT 0
        );",
    )?;
    Ok(())
}

fn create_views_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _lodge_views (
            name       TEXT PRIMARY KEY,
            collection TEXT NOT NULL,
            where_clause TEXT,
            sort       TEXT,
            lim        INTEGER,
            created_at TEXT NOT NULL,
            description TEXT,
            sql        TEXT
        );",
    )?;
    Ok(())
}

fn migrate_views_description(conn: &Connection) -> Result<()> {
    // Attempt to add description column; ignore error if it already exists
    let _ = conn.execute_batch("ALTER TABLE _lodge_views ADD COLUMN description TEXT");
    Ok(())
}

fn migrate_views_sql_column(conn: &Connection) -> Result<()> {
    // Attempt to add sql column; ignore error if it already exists
    let _ = conn.execute_batch("ALTER TABLE _lodge_views ADD COLUMN sql TEXT");
    Ok(())
}
