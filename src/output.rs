use serde_json::Value;

pub enum Format {
    Json,
    Table,
    Csv,
}

impl Format {
    pub fn from_str(s: &str) -> Option<Self> {
        match s.to_lowercase().as_str() {
            "json" => Some(Format::Json),
            "table" => Some(Format::Table),
            "csv" => Some(Format::Csv),
            _ => None,
        }
    }
}

pub fn format_output(records: &[Value], format: &Format) -> String {
    match format {
        Format::Json => format_json(records),
        Format::Table => format_table(records),
        Format::Csv => format_csv(records),
    }
}

pub fn format_single(record: &Value, format: &Format) -> String {
    match format {
        Format::Json => serde_json::to_string_pretty(record).unwrap_or_default(),
        Format::Table => format_table(&[record.clone()]),
        Format::Csv => format_csv(&[record.clone()]),
    }
}

fn format_json(records: &[Value]) -> String {
    serde_json::to_string_pretty(records).unwrap_or_default()
}

fn format_table(records: &[Value]) -> String {
    if records.is_empty() {
        return "(no records)".to_string();
    }

    // Get column names from the first record
    let columns: Vec<String> = if let Some(Value::Object(map)) = records.first() {
        map.keys().cloned().collect()
    } else {
        return "(no records)".to_string();
    };

    // Calculate column widths
    let mut widths: Vec<usize> = columns.iter().map(|c| c.len()).collect();
    for record in records {
        if let Value::Object(map) = record {
            for (i, col) in columns.iter().enumerate() {
                let val_str = value_to_display(map.get(col).unwrap_or(&Value::Null));
                widths[i] = widths[i].max(val_str.len());
            }
        }
    }

    let mut output = String::new();

    // Header
    let header: Vec<String> = columns
        .iter()
        .enumerate()
        .map(|(i, c)| format!("{:<width$}", c, width = widths[i]))
        .collect();
    output.push_str(&header.join("  "));
    output.push('\n');

    // Separator
    let sep: Vec<String> = widths.iter().map(|w| "-".repeat(*w)).collect();
    output.push_str(&sep.join("  "));
    output.push('\n');

    // Rows
    for record in records {
        if let Value::Object(map) = record {
            let row: Vec<String> = columns
                .iter()
                .enumerate()
                .map(|(i, col)| {
                    let val_str = value_to_display(map.get(col).unwrap_or(&Value::Null));
                    format!("{:<width$}", val_str, width = widths[i])
                })
                .collect();
            output.push_str(&row.join("  "));
            output.push('\n');
        }
    }

    output.trim_end().to_string()
}

fn format_csv(records: &[Value]) -> String {
    if records.is_empty() {
        return String::new();
    }

    let columns: Vec<String> = if let Some(Value::Object(map)) = records.first() {
        map.keys().cloned().collect()
    } else {
        return String::new();
    };

    let mut output = String::new();
    output.push_str(&columns.join(","));
    output.push('\n');

    for record in records {
        if let Value::Object(map) = record {
            let row: Vec<String> = columns
                .iter()
                .map(|col| {
                    let val = value_to_display(map.get(col).unwrap_or(&Value::Null));
                    if val.contains(',') || val.contains('"') || val.contains('\n') {
                        format!("\"{}\"", val.replace('"', "\"\""))
                    } else {
                        val
                    }
                })
                .collect();
            output.push_str(&row.join(","));
            output.push('\n');
        }
    }

    output.trim_end().to_string()
}

fn value_to_display(val: &Value) -> String {
    match val {
        Value::Null => "null".to_string(),
        Value::String(s) => s.clone(),
        Value::Number(n) => n.to_string(),
        Value::Bool(b) => b.to_string(),
        other => other.to_string(),
    }
}
