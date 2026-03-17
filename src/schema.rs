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

/// Load a single collection by name.
pub fn load_collection(conn: &Connection, name: &str) -> Result<Option<Collection>> {
    let mut stmt = conn.prepare(
        "SELECT field_name, field_type, field_order
         FROM _lodge_meta
         WHERE collection = ?1
         ORDER BY field_order",
    )?;

    let rows = stmt.query_map([name], |row| {
        Ok((
            row.get::<_, String>(0)?,
            row.get::<_, String>(1)?,
        ))
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
