mod common;

use predicates::prelude::*;

fn setup_with_data() -> tempfile::TempDir {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text, priority:int"])
        .assert()
        .success();
    for (title, priority) in &[("Alpha", "3"), ("Beta", "1")] {
        common::lodge_cmd(&dir)
            .args(["tasks", "add", "--title", title, "--priority", priority])
            .assert()
            .success();
    }
    dir
}

#[test]
fn export_json_contains_schema_and_records() {
    let dir = setup_with_data();
    let output = common::lodge_cmd(&dir)
        .args(["export", "tasks"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["collection"], "tasks");
    assert!(json["fields"].as_array().unwrap().len() >= 2);
    assert_eq!(json["records"].as_array().unwrap().len(), 2);
}

#[test]
fn export_csv_format() {
    let dir = setup_with_data();
    common::lodge_cmd(&dir)
        .args(["export", "tasks", "--format", "csv"])
        .assert()
        .success()
        .stdout(predicate::str::contains("id"))
        .stdout(predicate::str::contains("Alpha"));
}

#[test]
fn export_all_collections() {
    let dir = setup_with_data();
    // Add another collection
    common::lodge_cmd(&dir)
        .args(["create", "notes", "--fields", "body:text"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["notes", "add", "--body", "hello"])
        .assert()
        .success();

    let output = common::lodge_cmd(&dir)
        .args(["export", "--all"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["lodge_export"], true);
    let collections = json["collections"].as_array().unwrap();
    assert_eq!(collections.len(), 2);
}

#[test]
fn export_empty_collection() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text"])
        .assert()
        .success();
    let output = common::lodge_cmd(&dir)
        .args(["export", "tasks"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json["records"].as_array().unwrap().len(), 0);
}

#[test]
fn export_nonexistent_collection_errors() {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["export", "nope"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn export_json_includes_field_types() {
    let dir = setup_with_data();
    let output = common::lodge_cmd(&dir)
        .args(["export", "tasks"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let fields = json["fields"].as_array().unwrap();
    let field_names: Vec<&str> = fields.iter().map(|f| f["name"].as_str().unwrap()).collect();
    assert!(field_names.contains(&"title"));
    assert!(field_names.contains(&"priority"));
    let title_field = fields.iter().find(|f| f["name"] == "title").unwrap();
    assert_eq!(title_field["type"], "text");
}
