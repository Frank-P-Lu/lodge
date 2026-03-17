use crate::error::{LodgeError, Result};
use crate::schema::Collection;
use crate::types::FieldType;
use chrono::NaiveDate;
use rusqlite::Connection;
use serde_json::{json, Value};

fn validate_date_field(collection: &Collection, field: &str) -> Result<()> {
    let f = collection
        .fields
        .iter()
        .find(|f| f.name == field)
        .ok_or_else(|| {
            LodgeError::InvalidFieldsFormat(format!(
                "field '{}' not found in collection '{}'",
                field, collection.name
            ))
        })?;
    if f.field_type != FieldType::Date && f.field_type != FieldType::Datetime {
        return Err(LodgeError::WrongFieldType {
            field: field.to_string(),
            collection: collection.name.clone(),
            expected_type: "date or datetime".to_string(),
        });
    }
    Ok(())
}

fn validate_numeric_field(collection: &Collection, field: &str) -> Result<()> {
    let f = collection
        .fields
        .iter()
        .find(|f| f.name == field)
        .ok_or_else(|| {
            LodgeError::InvalidFieldsFormat(format!(
                "field '{}' not found in collection '{}'",
                field, collection.name
            ))
        })?;
    if f.field_type != FieldType::Int && f.field_type != FieldType::Real {
        return Err(LodgeError::WrongFieldType {
            field: field.to_string(),
            collection: collection.name.clone(),
            expected_type: "int or real".to_string(),
        });
    }
    Ok(())
}

pub fn streak(conn: &Connection, collection: &Collection, date_field: &str) -> Result<Value> {
    validate_date_field(collection, date_field)?;

    // Get distinct dates sorted ascending
    let sql = format!(
        "SELECT DISTINCT substr({date_field}, 1, 10) as d FROM \"{}\" WHERE {date_field} IS NOT NULL ORDER BY d ASC",
        collection.name
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| LodgeError::Sql(e.to_string()))?;
    let dates: Vec<NaiveDate> = stmt
        .query_map([], |row| {
            let s: String = row.get(0)?;
            Ok(s)
        })
        .map_err(|e| LodgeError::Sql(e.to_string()))?
        .filter_map(|r| r.ok())
        .filter_map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .collect();

    if dates.is_empty() {
        return Ok(json!({
            "current_streak": 0,
            "longest_streak": 0,
            "total_days_with_records": 0
        }));
    }

    let total_days = dates.len();

    // Compute streaks
    let mut streaks: Vec<(NaiveDate, NaiveDate, i64)> = Vec::new();
    let mut streak_start = dates[0];
    let mut streak_end = dates[0];

    for date in &dates[1..] {
        let diff = date.signed_duration_since(streak_end).num_days();
        if diff == 1 {
            streak_end = *date;
        } else {
            let len = streak_end.signed_duration_since(streak_start).num_days() + 1;
            streaks.push((streak_start, streak_end, len));
            streak_start = *date;
            streak_end = *date;
        }
    }
    // Push final streak
    let len = streak_end.signed_duration_since(streak_start).num_days() + 1;
    streaks.push((streak_start, streak_end, len));

    let longest = streaks.iter().max_by_key(|s| s.2).unwrap();
    let current = streaks.last().unwrap();

    Ok(json!({
        "current_streak": current.2,
        "current_streak_start": current.0.format("%Y-%m-%d").to_string(),
        "current_streak_end": current.1.format("%Y-%m-%d").to_string(),
        "longest_streak": longest.2,
        "longest_streak_start": longest.0.format("%Y-%m-%d").to_string(),
        "longest_streak_end": longest.1.format("%Y-%m-%d").to_string(),
        "total_days_with_records": total_days,
    }))
}

pub fn gaps(
    conn: &Connection,
    collection: &Collection,
    date_field: &str,
    threshold_days: i64,
) -> Result<Vec<Value>> {
    validate_date_field(collection, date_field)?;

    let sql = format!(
        "SELECT DISTINCT substr({date_field}, 1, 10) as d FROM \"{}\" WHERE {date_field} IS NOT NULL ORDER BY d ASC",
        collection.name
    );
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| LodgeError::Sql(e.to_string()))?;
    let dates: Vec<NaiveDate> = stmt
        .query_map([], |row| {
            let s: String = row.get(0)?;
            Ok(s)
        })
        .map_err(|e| LodgeError::Sql(e.to_string()))?
        .filter_map(|r| r.ok())
        .filter_map(|s| NaiveDate::parse_from_str(&s, "%Y-%m-%d").ok())
        .collect();

    let mut result = Vec::new();
    for i in 1..dates.len() {
        let days = dates[i].signed_duration_since(dates[i - 1]).num_days();
        if days > threshold_days {
            result.push(json!({
                "gap_start": dates[i-1].format("%Y-%m-%d").to_string(),
                "gap_end": dates[i].format("%Y-%m-%d").to_string(),
                "days": days,
            }));
        }
    }

    Ok(result)
}

pub fn rolling_average(
    conn: &Connection,
    collection: &Collection,
    value_field: &str,
    date_field: &str,
    window: i64,
) -> Result<Vec<Value>> {
    validate_date_field(collection, date_field)?;
    validate_numeric_field(collection, value_field)?;

    let sql = format!(
        "SELECT substr({date_field}, 1, 10) as date, {value_field} as value, \
         AVG(CAST({value_field} AS REAL)) OVER (ORDER BY {date_field} ROWS BETWEEN {preceding} PRECEDING AND CURRENT ROW) as rolling_avg \
         FROM \"{collection}\" WHERE {value_field} IS NOT NULL AND {date_field} IS NOT NULL ORDER BY {date_field}",
        collection = collection.name,
        preceding = window - 1,
    );

    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| LodgeError::Sql(e.to_string()))?;

    let rows = stmt
        .query_map([], |row| {
            let date: String = row.get(0)?;
            let value: f64 = row.get(1)?;
            let avg: f64 = row.get(2)?;
            Ok(json!({
                "date": date,
                "value": value,
                "rolling_avg": (avg * 100.0).round() / 100.0,
            }))
        })
        .map_err(|e| LodgeError::Sql(e.to_string()))?;

    let mut results = Vec::new();
    for row in rows {
        results.push(row.map_err(|e| LodgeError::Sql(e.to_string()))?);
    }
    Ok(results)
}
