use crate::error::Result;
use rusqlite::Connection;

pub struct TrackResult {
    pub call_count: i64,
    pub newly_suggested: bool,
}

/// Track a query execution. Returns the new call count and whether this crossing the threshold
/// for the first time (only for query_type == "query").
pub fn track_query(
    conn: &Connection,
    query_type: &str,
    collection: &str,
    fingerprint: &str,
    threshold: i64,
) -> Result<TrackResult> {
    let now = chrono::Utc::now().format("%Y-%m-%dT%H:%M:%S").to_string();

    conn.execute(
        "INSERT INTO _lodge_query_log (query_type, collection, fingerprint, call_count, last_used)
         VALUES (?1, ?2, ?3, 1, ?4)
         ON CONFLICT(fingerprint) DO UPDATE SET
           call_count = call_count + 1,
           last_used = ?4",
        rusqlite::params![query_type, collection, fingerprint, now],
    )?;

    let (call_count, suggested): (i64, i64) = conn.query_row(
        "SELECT call_count, suggested FROM _lodge_query_log WHERE fingerprint = ?1",
        [fingerprint],
        |row| Ok((row.get(0)?, row.get(1)?)),
    )?;

    // Only suggest for "query" type, when crossing threshold for the first time
    let newly_suggested = query_type == "query" && call_count >= threshold && suggested == 0;

    if newly_suggested {
        conn.execute(
            "UPDATE _lodge_query_log SET suggested = 1 WHERE fingerprint = ?1",
            [fingerprint],
        )?;
    }

    Ok(TrackResult {
        call_count,
        newly_suggested,
    })
}

pub fn build_query_fingerprint(
    collection: &str,
    where_clause: Option<&str>,
    sort: Option<&str>,
    limit: Option<i64>,
    fields: Option<&str>,
) -> String {
    format!(
        "query:{}|w:{}|s:{}|l:{}|f:{}",
        collection,
        where_clause.unwrap_or(""),
        sort.unwrap_or(""),
        limit.map(|l| l.to_string()).unwrap_or_default(),
        fields.unwrap_or(""),
    )
}

pub fn build_search_fingerprint(collection: &str, query_text: &str, limit: Option<i64>) -> String {
    format!(
        "search:{}|q:{}|l:{}",
        collection,
        query_text,
        limit.map(|l| l.to_string()).unwrap_or_default(),
    )
}

pub fn build_view_run_fingerprint(view_name: &str) -> String {
    format!("view_run:{}", view_name)
}

/// Build a ready-to-paste `lodge view create` command for a suggested view.
pub fn build_suggestion_command(
    collection: &str,
    where_clause: Option<&str>,
    sort: Option<&str>,
    limit: Option<i64>,
) -> String {
    let name = format!("{}_view", collection);
    let mut parts = vec![
        "lodge".to_string(),
        "view".to_string(),
        "create".to_string(),
        name,
        "--collection".to_string(),
        collection.to_string(),
    ];
    if let Some(w) = where_clause {
        parts.push("--where".to_string());
        parts.push(format!("\"{}\"", w));
    }
    if let Some(s) = sort {
        parts.push("--sort".to_string());
        parts.push(format!("\"{}\"", s));
    }
    if let Some(l) = limit {
        parts.push("--limit".to_string());
        parts.push(l.to_string());
    }
    parts.join(" ")
}
