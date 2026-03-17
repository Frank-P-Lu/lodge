mod common;

use predicates::prelude::*;

fn setup_with_collection() -> tempfile::TempDir {
    let dir = common::setup();
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text, priority:int"])
        .assert()
        .success();
    dir
}

#[test]
fn import_json_array() {
    let dir = setup_with_collection();
    let json_data = r#"[{"title": "Task A", "priority": 1}, {"title": "Task B", "priority": 2}]"#;
    let file_path = dir.path().join("data.json");
    std::fs::write(&file_path, json_data).unwrap();

    common::lodge_cmd(&dir)
        .args(["import", "tasks", "--file", file_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Imported 2 records"));

    // Verify data
    let output = common::lodge_cmd(&dir)
        .args(["tasks", "query"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 2);
}

#[test]
fn import_json_envelope() {
    let dir = setup_with_collection();
    let json_data = r#"{"collection": "tasks", "records": [{"title": "Task A", "priority": 1}]}"#;
    let file_path = dir.path().join("data.json");
    std::fs::write(&file_path, json_data).unwrap();

    common::lodge_cmd(&dir)
        .args(["import", "tasks", "--file", file_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Imported 1 records"));
}

#[test]
fn import_csv() {
    let dir = setup_with_collection();
    let csv_data = "title,priority\nTask A,1\nTask B,2\n";
    let file_path = dir.path().join("data.csv");
    std::fs::write(&file_path, csv_data).unwrap();

    common::lodge_cmd(&dir)
        .args(["import", "tasks", "--file", file_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Imported 2 records"));
}

#[test]
fn import_validates_types() {
    let dir = setup_with_collection();
    let json_data = r#"[{"title": "Task A", "priority": "not_a_number"}]"#;
    let file_path = dir.path().join("data.json");
    std::fs::write(&file_path, json_data).unwrap();

    common::lodge_cmd(&dir)
        .args(["import", "tasks", "--file", file_path.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid value"));
}

#[test]
fn import_nonexistent_collection_errors() {
    let dir = common::setup();
    let file_path = dir.path().join("data.json");
    std::fs::write(&file_path, "[]").unwrap();

    common::lodge_cmd(&dir)
        .args(["import", "nope", "--file", file_path.to_str().unwrap()])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

#[test]
fn import_full_export() {
    let dir = common::setup();
    // Create one collection, export all, then import into fresh db
    common::lodge_cmd(&dir)
        .args(["create", "tasks", "--fields", "title:text, priority:int"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "Alpha", "--priority", "1"])
        .assert()
        .success();

    let export_output = common::lodge_cmd(&dir)
        .args(["export", "--all"])
        .output()
        .unwrap();
    let export_data = String::from_utf8(export_output.stdout).unwrap();

    // Set up a fresh db in a new directory
    let dir2 = common::setup();
    let file_path = dir2.path().join("full_export.json");
    std::fs::write(&file_path, &export_data).unwrap();

    common::lodge_cmd(&dir2)
        .args(["import", "--all", "--file", file_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Imported 1 records into 'tasks'"));

    // Verify the imported data
    let output = common::lodge_cmd(&dir2)
        .args(["tasks", "query"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(json.as_array().unwrap()[0]["title"], "Alpha");
}

#[test]
fn import_round_trip() {
    let dir = setup_with_collection();
    // Add some records
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "Task 1", "--priority", "5"])
        .assert()
        .success();
    common::lodge_cmd(&dir)
        .args(["tasks", "add", "--title", "Task 2", "--priority", "10"])
        .assert()
        .success();

    // Export
    let export_output = common::lodge_cmd(&dir)
        .args(["export", "tasks"])
        .output()
        .unwrap();
    let export_data = String::from_utf8(export_output.stdout).unwrap();

    // Create fresh db + collection, import
    let dir2 = common::setup();
    common::lodge_cmd(&dir2)
        .args(["create", "tasks", "--fields", "title:text, priority:int"])
        .assert()
        .success();
    let file_path = dir2.path().join("export.json");
    std::fs::write(&file_path, &export_data).unwrap();

    common::lodge_cmd(&dir2)
        .args(["import", "tasks", "--file", file_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Imported 2 records"));

    // Verify
    let output = common::lodge_cmd(&dir2)
        .args(["tasks", "query", "--sort", "priority ASC"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    let arr = json.as_array().unwrap();
    assert_eq!(arr.len(), 2);
    assert_eq!(arr[0]["title"], "Task 1");
    assert_eq!(arr[1]["title"], "Task 2");
}

#[test]
fn import_full_creates_missing_collections() {
    let dir = common::setup();
    let full_export = r#"{
        "lodge_export": true,
        "collections": [
            {
                "collection": "books",
                "fields": [{"name": "title", "type": "text"}, {"name": "pages", "type": "int"}],
                "records": [{"title": "Rust in Action", "pages": 456}]
            }
        ]
    }"#;
    let file_path = dir.path().join("full.json");
    std::fs::write(&file_path, full_export).unwrap();

    common::lodge_cmd(&dir)
        .args(["import", "--all", "--file", file_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Imported 1 records into 'books'"));

    // Verify collection was created and data imported
    let output = common::lodge_cmd(&dir)
        .args(["books", "query"])
        .output()
        .unwrap();
    let json: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(json.as_array().unwrap().len(), 1);
    assert_eq!(json.as_array().unwrap()[0]["title"], "Rust in Action");
}

#[test]
fn test_import_single_collection_with_file_flag() {
    let dir = setup_with_collection();
    let json_data = r#"[{"title": "Flagged", "priority": 3}]"#;
    let file_path = dir.path().join("data.json");
    std::fs::write(&file_path, json_data).unwrap();

    common::lodge_cmd(&dir)
        .args(["import", "tasks", "--file", file_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Imported 1 records"));
}

#[test]
fn test_import_all_with_file_flag() {
    let dir = common::setup();
    let full_export = r#"{
        "lodge_export": true,
        "collections": [
            {
                "collection": "items",
                "fields": [{"name": "name", "type": "text"}],
                "records": [{"name": "Widget"}]
            }
        ]
    }"#;
    let file_path = dir.path().join("dump.json");
    std::fs::write(&file_path, full_export).unwrap();

    common::lodge_cmd(&dir)
        .args(["import", "--all", "--file", file_path.to_str().unwrap()])
        .assert()
        .success()
        .stdout(predicate::str::contains("Imported 1 records into 'items'"));
}

#[test]
fn test_import_no_args_error() {
    let dir = common::setup();
    common::lodge_cmd(&dir).args(["import"]).assert().failure();
}
