use crate::error::{LodgeError, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum FieldType {
    Text,
    Int,
    Real,
    Bool,
    Date,
    Datetime,
}

impl FieldType {
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "text" => Ok(FieldType::Text),
            "int" | "integer" => Ok(FieldType::Int),
            "real" | "float" | "double" => Ok(FieldType::Real),
            "bool" | "boolean" => Ok(FieldType::Bool),
            "date" => Ok(FieldType::Date),
            "datetime" => Ok(FieldType::Datetime),
            other => Err(LodgeError::InvalidFieldType(other.to_string())),
        }
    }

    pub fn sql_type(&self) -> &'static str {
        match self {
            FieldType::Text => "TEXT",
            FieldType::Int => "INTEGER",
            FieldType::Real => "REAL",
            FieldType::Bool => "INTEGER",
            FieldType::Date => "TEXT",
            FieldType::Datetime => "TEXT",
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            FieldType::Text => "text",
            FieldType::Int => "int",
            FieldType::Real => "real",
            FieldType::Bool => "bool",
            FieldType::Date => "date",
            FieldType::Datetime => "datetime",
        }
    }

    pub fn validate(&self, value: &str, field_name: &str) -> Result<String> {
        match self {
            FieldType::Text => Ok(value.to_string()),
            FieldType::Int => {
                value
                    .parse::<i64>()
                    .map(|v| v.to_string())
                    .map_err(|_| LodgeError::InvalidValue {
                        field: field_name.to_string(),
                        field_type: "int".to_string(),
                        value: value.to_string(),
                    })
            }
            FieldType::Real => {
                value
                    .parse::<f64>()
                    .map(|v| v.to_string())
                    .map_err(|_| LodgeError::InvalidValue {
                        field: field_name.to_string(),
                        field_type: "real".to_string(),
                        value: value.to_string(),
                    })
            }
            FieldType::Bool => match value.to_lowercase().as_str() {
                "true" | "1" | "yes" => Ok("1".to_string()),
                "false" | "0" | "no" => Ok("0".to_string()),
                _ => Err(LodgeError::InvalidValue {
                    field: field_name.to_string(),
                    field_type: "bool".to_string(),
                    value: value.to_string(),
                }),
            },
            FieldType::Date => chrono::NaiveDate::parse_from_str(value, "%Y-%m-%d")
                .map(|d| d.format("%Y-%m-%d").to_string())
                .map_err(|_| LodgeError::InvalidValue {
                    field: field_name.to_string(),
                    field_type: "date".to_string(),
                    value: value.to_string(),
                }),
            FieldType::Datetime => {
                // Try ISO 8601 formats
                if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%dT%H:%M:%S") {
                    return Ok(dt.format("%Y-%m-%dT%H:%M:%S").to_string());
                }
                if let Ok(dt) = chrono::NaiveDateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S") {
                    return Ok(dt.format("%Y-%m-%dT%H:%M:%S").to_string());
                }
                // Try timezone-aware formats (convert to UTC)
                if let Ok(dt) = chrono::DateTime::parse_from_rfc3339(value) {
                    return Ok(dt.naive_utc().format("%Y-%m-%dT%H:%M:%S").to_string());
                }
                if let Ok(dt) = chrono::DateTime::parse_from_str(value, "%Y-%m-%d %H:%M:%S%z") {
                    return Ok(dt.naive_utc().format("%Y-%m-%dT%H:%M:%S").to_string());
                }
                Err(LodgeError::InvalidValue {
                    field: field_name.to_string(),
                    field_type: "datetime".to_string(),
                    value: value.to_string(),
                })
            }
        }
    }
}

/// Parse a fields spec like "title:text, priority:int" into Vec<(name, FieldType)>
pub fn parse_fields(spec: &str) -> Result<Vec<(String, FieldType)>> {
    let mut fields = Vec::new();
    for part in spec.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let mut iter = part.splitn(2, ':');
        let name = iter
            .next()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| LodgeError::InvalidFieldsFormat(part.to_string()))?;
        let type_str = iter
            .next()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .ok_or_else(|| LodgeError::InvalidFieldsFormat(part.to_string()))?;

        // Validate field name is a valid identifier
        if !name.chars().all(|c| c.is_alphanumeric() || c == '_')
            || name.starts_with(|c: char| c.is_ascii_digit())
        {
            return Err(LodgeError::InvalidFieldsFormat(format!(
                "invalid field name '{name}'"
            )));
        }

        // Reject auto-column names
        if matches!(name, "id" | "created_at" | "updated_at") {
            return Err(LodgeError::InvalidFieldsFormat(format!(
                "'{name}' is an auto-managed column"
            )));
        }

        fields.push((name.to_string(), FieldType::from_str(type_str)?));
    }
    if fields.is_empty() {
        return Err(LodgeError::InvalidFieldsFormat(
            "no fields specified".to_string(),
        ));
    }
    Ok(fields)
}
