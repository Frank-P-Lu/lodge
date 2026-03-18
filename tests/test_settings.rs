mod common;

use predicates::prelude::*;

#[test]
fn init_creates_default_settings_file() {
    let dir = common::setup();
    let settings_path = dir.path().join(".lodge").join("settings.json");
    assert!(
        settings_path.exists(),
        "settings.json should be created by init"
    );
    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&settings_path).unwrap()).unwrap();
    assert_eq!(content["default_format"], "json");
    assert_eq!(content["distinct_threshold"], 15);
}

#[test]
fn default_format_without_settings_file() {
    let dir = common::setup();
    // Create a collection so list has something to show
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();
    // Default format is JSON
    let output = common::lodge_cmd(&dir).args(["list"]).output().unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(json.is_array());
}

#[test]
fn set_and_read_default_format() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();
    // Set default format to table
    common::lodge_cmd(&dir)
        .args(["set", "default_format", "table"])
        .assert()
        .success();
    // Now list should output table format (not JSON)
    let output = common::lodge_cmd(&dir).args(["list"]).output().unwrap();
    let stdout = String::from_utf8_lossy(&output.stdout);
    // Table format has dashes separator line
    assert!(
        stdout.contains("---"),
        "Expected table format, got: {stdout}"
    );
    // Should NOT be valid JSON
    assert!(serde_json::from_str::<serde_json::Value>(&stdout).is_err());
}

#[test]
fn set_distinct_threshold() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "items", "--fields", "status:text"])
        .assert()
        .success();
    // Add 3 distinct values
    for val in &["open", "closed", "pending"] {
        common::lodge_cmd(&dir)
            .args(["items", "add", "--status", val])
            .assert()
            .success();
    }
    // Set threshold to 2 (below distinct count of 3)
    common::lodge_cmd(&dir)
        .args(["set", "distinct_threshold", "2"])
        .assert()
        .success();
    // Schema should NOT show values (3 > threshold 2)
    let output = common::lodge_cmd(&dir)
        .args(["items", "schema"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let fields = json["fields"].as_array().unwrap();
    let status_field = fields.iter().find(|f| f["name"] == "status").unwrap();
    assert!(
        status_field.get("values").is_none(),
        "values should be hidden when above threshold"
    );
}

#[test]
fn cli_format_flag_overrides_setting() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();
    // Set default format to table
    common::lodge_cmd(&dir)
        .args(["set", "default_format", "table"])
        .assert()
        .success();
    // But pass --format json explicitly
    let output = common::lodge_cmd(&dir)
        .args(["list", "--format", "json"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert!(
        json.is_array(),
        "Explicit --format json should override setting"
    );
}

#[test]
fn set_invalid_key_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["set", "nonexistent_key", "value"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Unknown setting"));
}

#[test]
fn settings_file_regenerated_on_read_if_missing() {
    let dir = common::setup();
    let settings_path = dir.path().join(".lodge").join("settings.json");
    // Delete the settings file created by init
    std::fs::remove_file(&settings_path).unwrap();
    assert!(!settings_path.exists());
    // Any command that reads settings should regenerate it
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();
    common::lodge_cmd(&dir).args(["list"]).assert().success();
    assert!(
        settings_path.exists(),
        "settings.json should be regenerated on read"
    );
    let content: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&settings_path).unwrap()).unwrap();
    assert_eq!(content["default_format"], "json");
    assert_eq!(content["distinct_threshold"], 15);
}

#[test]
fn set_invalid_format_value_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["set", "default_format", "xml"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid value"));
}
