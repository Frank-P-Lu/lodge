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
    Ok(())
}

/// Get the lodge directory path, searching up from `start`.
pub fn lodge_dir(start: &Path) -> Result<PathBuf> {
    find_lodge_dir(start).ok_or(LodgeError::NotInitialized)
}

/// Open an existing lodge database, searching up from `start`.
pub fn open(start: &Path) -> Result<Connection> {
    let lodge_dir = find_lodge_dir(start).ok_or(LodgeError::NotInitialized)?;
    let db_path = lodge_dir.join(DB_FILE);
    let conn = Connection::open(db_path)?;
    // Migrate: ensure _lodge_views exists for older databases
    create_views_table(&conn)?;
    // Migrate: ensure _lodge_fts_meta exists for older databases
    create_fts_meta_table(&conn)?;
    Ok(conn)
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

fn create_views_table(conn: &Connection) -> Result<()> {
    conn.execute_batch(
        "CREATE TABLE IF NOT EXISTS _lodge_views (
            name       TEXT PRIMARY KEY,
            collection TEXT NOT NULL,
            where_clause TEXT,
            sort       TEXT,
            lim        INTEGER,
            created_at TEXT NOT NULL
        );",
    )?;
    Ok(())
}
