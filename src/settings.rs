use crate::error::{LodgeError, Result};
use std::path::Path;

const SETTINGS_FILE: &str = "settings.json";

const VALID_FORMATS: &[&str] = &["json", "table", "csv"];
const VALID_KEYS: &[&str] = &[
    "default_format",
    "distinct_max",
    "distinct_ratio",
    "view_suggest_threshold",
];

pub struct Settings {
    pub default_format: String,
    pub distinct_max: usize,
    pub distinct_ratio: f64,
    pub view_suggest_threshold: i64,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            default_format: "json".to_string(),
            distinct_max: 20,
            distinct_ratio: 0.5,
            view_suggest_threshold: 3,
        }
    }
}

/// Load settings from `.lodge/settings.json`. Creates the file with defaults if missing.
pub fn load_settings(lodge_dir: &Path) -> Settings {
    let path = lodge_dir.join(SETTINGS_FILE);
    let defaults = Settings::default();

    let content = match std::fs::read_to_string(&path) {
        Ok(c) => c,
        Err(_) => {
            // File doesn't exist — create it with defaults
            let _ = create_default_settings(lodge_dir);
            return defaults;
        }
    };

    let parsed: serde_json::Value = match serde_json::from_str(&content) {
        Ok(v) => v,
        Err(_) => return defaults,
    };

    Settings {
        default_format: parsed
            .get("default_format")
            .and_then(|v| v.as_str())
            .unwrap_or(&defaults.default_format)
            .to_string(),
        distinct_max: parsed
            .get("distinct_max")
            .and_then(|v| v.as_u64())
            .map(|v| v as usize)
            .unwrap_or(defaults.distinct_max),
        distinct_ratio: parsed
            .get("distinct_ratio")
            .and_then(|v| v.as_f64())
            .unwrap_or(defaults.distinct_ratio),
        view_suggest_threshold: parsed
            .get("view_suggest_threshold")
            .and_then(|v| v.as_i64())
            .unwrap_or(defaults.view_suggest_threshold),
    }
}

/// Create default settings file if it doesn't exist.
pub fn create_default_settings(lodge_dir: &Path) -> Result<()> {
    let path = lodge_dir.join(SETTINGS_FILE);
    if !path.exists() {
        let defaults = Settings::default();
        let json = serde_json::json!({
            "default_format": defaults.default_format,
            "distinct_max": defaults.distinct_max,
            "distinct_ratio": defaults.distinct_ratio,
            "view_suggest_threshold": defaults.view_suggest_threshold,
        });
        let content =
            serde_json::to_string_pretty(&json).map_err(|e| LodgeError::Serialization(e.to_string()))?;
        std::fs::write(&path, content)?;
    }
    Ok(())
}

/// Set a single setting value, validating key and value.
pub fn set_setting(lodge_dir: &Path, key: &str, value: &str) -> Result<()> {
    if !VALID_KEYS.contains(&key) {
        return Err(LodgeError::InvalidSetting(format!(
            "Unknown setting '{}'. Valid settings: {}",
            key,
            VALID_KEYS.join(", ")
        )));
    }

    // Validate value
    let json_value = match key {
        "default_format" => {
            if !VALID_FORMATS.contains(&value) {
                return Err(LodgeError::InvalidValue {
                    field: key.to_string(),
                    field_type: "format".to_string(),
                    value: format!("{value}. Valid values: {}", VALID_FORMATS.join(", ")),
                });
            }
            serde_json::Value::String(value.to_string())
        }
        "distinct_max" => {
            let n: usize = value.parse().map_err(|_| LodgeError::InvalidValue {
                field: key.to_string(),
                field_type: "int".to_string(),
                value: value.to_string(),
            })?;
            serde_json::Value::Number(serde_json::Number::from(n))
        }
        "distinct_ratio" => {
            let r: f64 = value.parse().map_err(|_| LodgeError::InvalidValue {
                field: key.to_string(),
                field_type: "real".to_string(),
                value: value.to_string(),
            })?;
            if !(0.0..=1.0).contains(&r) {
                return Err(LodgeError::InvalidValue {
                    field: key.to_string(),
                    field_type: "real".to_string(),
                    value: format!("{value}. Must be between 0.0 and 1.0"),
                });
            }
            serde_json::Value::Number(
                serde_json::Number::from_f64(r).ok_or_else(|| LodgeError::InvalidValue {
                    field: key.to_string(),
                    field_type: "real".to_string(),
                    value: value.to_string(),
                })?,
            )
        }
        "view_suggest_threshold" => {
            let n: i64 = value.parse().map_err(|_| LodgeError::InvalidValue {
                field: key.to_string(),
                field_type: "int".to_string(),
                value: value.to_string(),
            })?;
            if n < 1 {
                return Err(LodgeError::InvalidValue {
                    field: key.to_string(),
                    field_type: "int".to_string(),
                    value: format!("{value}. Must be a positive integer"),
                });
            }
            serde_json::Value::Number(serde_json::Number::from(n))
        }
        _ => unreachable!(),
    };

    // Read existing or start fresh
    let path = lodge_dir.join(SETTINGS_FILE);
    let mut obj = if path.exists() {
        let content = std::fs::read_to_string(&path)?;
        serde_json::from_str::<serde_json::Value>(&content).unwrap_or(serde_json::json!({}))
    } else {
        let defaults = Settings::default();
        serde_json::json!({
            "default_format": defaults.default_format,
            "distinct_max": defaults.distinct_max,
            "distinct_ratio": defaults.distinct_ratio,
            "view_suggest_threshold": defaults.view_suggest_threshold,
        })
    };

    obj[key] = json_value;
    let content = serde_json::to_string_pretty(&obj).map_err(|e| LodgeError::Serialization(e.to_string()))?;
    std::fs::write(&path, content)?;

    Ok(())
}
