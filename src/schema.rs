use crate::error::Result;
use crate::types::FieldType;
use rusqlite::Connection;

#[derive(Debug, Clone)]
pub struct Field {
    pub name: String,
    pub field_type: FieldType,
}

#[derive(Debug, Clone)]
pub struct Collection {
    pub name: String,
    pub fields: Vec<Field>,
}

/// Load all collections from the _lodge_meta table.
pub fn load_collections(conn: &Connection) -> Result<Vec<Collection>> {
    let mut stmt = conn.prepare(
        "SELECT collection, field_name, field_type, field_order
         FROM _lodge_meta
         ORDER BY collection, field_order",
    )?;

    let rows = stmt.query_map([], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
            row.get::<_, String>(2)?,
            row.get::<_, i32>(3)?,
        ))
    })?;

    let mut collections: Vec<Collection> = Vec::new();
    for row in rows {
        let (coll_name, field_name, field_type_str, _field_order) = row?;
        let field_type = FieldType::from_str(&field_type_str).unwrap_or(FieldType::Text); // fallback for unknown types

        let field = Field {
            name: field_name,
            field_type,
        };

        if let Some(coll) = collections.iter_mut().find(|c| c.name == coll_name) {
            coll.fields.push(field);
        } else {
            collections.push(Collection {
                name: coll_name,
                fields: vec![field],
            });
        }
    }

    Ok(collections)
}

/// Load distinct values for all text fields in a collection in one pass.
/// Returns a map from field name to distinct values.
/// A field's values are shown only when:
/// - the distinct count is <= `max_distinct`, AND
/// - the distinct count / total rows <= `ratio`
pub fn load_all_distinct_values(
    conn: &Connection,
    collection: &str,
    fields: &[Field],
    max_distinct: usize,
    ratio: f64,
) -> Result<std::collections::HashMap<String, Vec<String>>> {
    let text_fields: Vec<&Field> = fields
        .iter()
        .filter(|f| f.field_type == FieldType::Text)
        .collect();

    if text_fields.is_empty() {
        return Ok(std::collections::HashMap::new());
    }

    // Get total row count for ratio calculation
    let total_rows: usize = conn.query_row(
        &format!("SELECT COUNT(*) FROM \"{}\"", collection),
        [],
        |row| row.get(0),
    )?;

    if total_rows == 0 {
        return Ok(std::collections::HashMap::new());
    }

    // Build a compound query that returns (field_name, value) pairs.
    let subqueries: Vec<String> = text_fields
        .iter()
        .map(|f| {
            format!(
                "SELECT '{name}' AS _field, \"{name}\" AS _val FROM \"{coll}\" \
                 WHERE \"{name}\" IS NOT NULL GROUP BY \"{name}\" LIMIT {lim}",
                name = f.name,
                coll = collection,
                lim = max_distinct + 1,
            )
        })
        .collect();

    let sql = subqueries
        .iter()
        .map(|sq| format!("SELECT * FROM ({sq})"))
        .collect::<Vec<_>>()
        .join(" UNION ALL ");
    let mut stmt = conn.prepare(&sql)?;
    let rows = stmt.query_map([], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut map: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for row in rows {
        let (field_name, value) = row?;
        map.entry(field_name).or_default().push(value);
    }

    // Remove fields that exceed max or ratio, and sort values
    map.retain(|_, values| {
        if values.is_empty() || values.len() > max_distinct {
            return false;
        }
        let field_ratio = values.len() as f64 / total_rows as f64;
        field_ratio <= ratio
    });
    for values in map.values_mut() {
        values.sort();
    }

    Ok(map)
}

/// Load a single collection by name.
pub fn load_collection(conn: &Connection, name: &str) -> Result<Option<Collection>> {
    let mut stmt = conn.prepare(
        "SELECT field_name, field_type, field_order
         FROM _lodge_meta
         WHERE collection = ?1
         ORDER BY field_order",
    )?;

    let rows = stmt.query_map([name], |row| {
        Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
    })?;

    let mut fields = Vec::new();
    for row in rows {
        let (field_name, field_type_str) = row?;
        let field_type = FieldType::from_str(&field_type_str).unwrap_or(FieldType::Text);
        fields.push(Field {
            name: field_name,
            field_type,
        });
    }

    if fields.is_empty() {
        Ok(None)
    } else {
        Ok(Some(Collection {
            name: name.to_string(),
            fields,
        }))
    }
}
